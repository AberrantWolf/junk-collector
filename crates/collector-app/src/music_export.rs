//! Authorized metadata-aware downloads over immutable audio and temporary scratch.
use crate::*;
use std::{
    collections::BTreeMap,
    io::{Read, Seek, SeekFrom},
};
struct ExportReader(Box<dyn ConversionScratch>);
impl Read for ExportReader {
    fn read(&mut self, b: &mut [u8]) -> std::io::Result<usize> {
        self.0.read(b)
    }
}
impl Seek for ExportReader {
    fn seek(&mut self, p: SeekFrom) -> std::io::Result<u64> {
        self.0.seek(p)
    }
}
impl CollectionService {
    pub fn export_music_flac(
        &self,
        actor: &Principal,
        item_id: ItemId,
        source: RepresentationId,
        position: u16,
        checkpoint: &mut dyn FnMut() -> Result<()>,
    ) -> Result<MediaResource> {
        checkpoint()?;
        let item = self.item_presented(actor, item_id)?;
        let row = self
            .music_track_availability(actor, item_id)?
            .into_iter()
            .find(|r| {
                r.source_representation_id == source
                    && r.track_position == position
                    && r.profile == Some(ConversionProfile::Flac)
            })
            .ok_or(Error::NotFound)?;
        let MusicTrackAvailabilityState::Available {
            representation_id,
            file_index,
            ref media_type,
            ..
        } = row.state
        else {
            return Err(Error::Unavailable(
                "Prepare FLAC before downloading an album file".into(),
            ));
        };
        if media_type != "audio/flac" {
            return Err(Error::Invalid("This track is not FLAC".into()));
        }
        let mut original = self.representation(actor, item_id, representation_id, file_index)?;
        // Historical media remains readable without executable module/schema support.
        let core_item = self.reader.item(item_id)?;
        if core_item.domain.module.0 != "org.junk.music"
            || !self.modules.has_module(&core_item.domain.module)
            || self.modules.inspect(&core_item.domain).is_err()
        {
            return Ok(original);
        }
        let mut tags = BTreeMap::new();
        tags.insert("TITLE".into(), row.title.clone());
        tags.insert("ALBUM".into(), item.title.clone());
        let manual = self
            .processing
            .metadata_authority(item.collection_id, item_id)?;
        let artist_override = manual
            .as_ref()
            .map(|record| crate::music_search::music_search_artist_override(&core_item, record))
            .transpose()?
            .flatten();
        let manifest = self.recorded_file_tracks(item.collection_id, source)?;
        let source_audio = manifest
            .as_ref()
            .and_then(|m| m.tracks.iter().find(|t| t.position == position))
            .map(|t| &t.audio);
        let artist = artist_override
            .as_deref()
            .or_else(|| source_audio.and_then(|a| a.tag("artist")))
            .unwrap_or(&item.subtitle);
        if !artist.is_empty() {
            tags.insert("ARTIST".into(), artist.into());
        }
        let album_artist = artist_override.as_deref().unwrap_or(&item.subtitle);
        if !album_artist.is_empty() {
            tags.insert("ALBUMARTIST".into(), album_artist.into());
        }
        if let Some(release) = self.effective_music_release(&core_item)? {
            tags.insert("MUSICBRAINZ_ALBUMID".into(), release.release_id);
        }

        tags.insert(
            "TRACKNUMBER".into(),
            row.display_position
                .unwrap_or(row.track_position)
                .to_string(),
        );
        if let Some(disc) = row.disc_position {
            tags.insert("DISCNUMBER".into(), disc.to_string());
        }
        let mut picture = None;
        if let Some(asset) = item.primary_artwork {
            let mut cover = self.asset_resource(actor, item.collection_id, asset, true)?;
            if cover.length > 1024 * 1024 {
                return Err(Error::Invalid("Export cover preview exceeds 1 MiB".into()));
            }
            let mut bytes = Vec::new();
            let mut block = [0; 65536];
            loop {
                checkpoint()?;
                let n = cover
                    .reader
                    .read(&mut block)
                    .map_err(|e| Error::Unavailable(e.to_string()))?;
                if n == 0 {
                    break;
                }
                if bytes.len() + n > 1024 * 1024 {
                    return Err(Error::Invalid("Export artwork exceeded its bound".into()));
                }
                bytes.extend(&block[..n]);
            }
            if bytes.len() as u64 != cover.length || Digest::sha256(&bytes) != cover.digest {
                return Err(Error::Conflict);
            }
            picture = Some((cover.media_type, bytes));
        }
        let mut scratch = self.processing.conversion_scratch()?;
        let result = junk_libs_pcm::flac::repack(
            original.reader.as_mut(),
            scratch.as_mut(),
            &junk_libs_pcm::flac::ExportMetadata { tags, picture },
            &mut || checkpoint().map_err(std::io::Error::other),
        )
        .map_err(|e| Error::Unavailable(e.to_string()))?;
        checkpoint()?;
        if original.digest.algorithm != HashAlgorithm::Sha256
            || hex::encode(result.input_digest) != original.digest.hex
        {
            return Err(Error::PreparedMediaInvalid);
        }
        // Freeze one accepted metadata/artwork snapshot; a changed grant or item must be retried.
        let current = self.item_presented(actor, item_id)?;
        let current_core = self.reader.item(item_id)?;
        if current_core.lifecycle.generation != core_item.lifecycle.generation {
            return Err(Error::Conflict);
        }
        let _authorized_again =
            self.representation(actor, item_id, representation_id, file_index)?;
        if current.revision != item.revision
            || current.primary_artwork != item.primary_artwork
            || current.title != item.title
            || current.subtitle != item.subtitle
        {
            return Err(Error::Conflict);
        }
        scratch
            .seek(SeekFrom::Start(0))
            .map_err(|e| Error::Unavailable(e.to_string()))?;
        let coordinate = match row.disc_position {
            Some(disc) => format!(
                "{disc:02}-{:02}",
                row.display_position.unwrap_or(row.track_position)
            ),
            None => format!("{:02}", row.display_position.unwrap_or(row.track_position)),
        };
        let name = format!("{} - {} - {}", coordinate, item.title, row.title);
        let safe = name
            .chars()
            .map(|c| {
                if c.is_control()
                    || matches!(c, '/' | '\\' | ':' | '"' | '<' | '>' | '|' | '?' | '*')
                {
                    '_'
                } else {
                    c
                }
            })
            .scan(0usize, |bytes, c| {
                *bytes += c.len_utf8();
                Some((*bytes, c))
            })
            .take_while(|(bytes, _)| *bytes <= 180)
            .map(|(_, c)| c)
            .collect::<String>();
        let safe = format!("{safe}.flac");
        Ok(MediaResource {
            visibility: original.visibility,
            reader: Box::new(ExportReader(scratch)),
            length: result.bytes,
            media_type: "audio/flac".into(),
            digest: Digest {
                algorithm: HashAlgorithm::Sha256,
                hex: hex::encode(result.output_digest),
            },
            download_name: safe,
            verify_on_transfer: false,
        })
    }
}
