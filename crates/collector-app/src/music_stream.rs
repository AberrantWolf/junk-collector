//! Source selection and authorization for temporary, source-bound playback.
use crate::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StreamCodec {
    Flac,
    Opus(u16),
}
impl StreamCodec {
    pub fn validate(self) -> Result<Self> {
        match self {
            Self::Opus(rate) if !(6..=256).contains(&rate) => {
                Err(Error::Invalid("Opus bitrate must be 6–256 kbps".into()))
            }
            _ => Ok(self),
        }
    }

    pub fn media_type(self) -> &'static str {
        match self {
            Self::Flac => "audio/flac",
            Self::Opus(_) => "audio/ogg; codecs=opus",
        }
    }
}

#[derive(Clone, Copy)]
pub enum StreamCoordinate {
    Track(u16),
    File(usize),
}

#[derive(Clone, Copy)]
pub struct MusicStreamRequest {
    pub coordinate: StreamCoordinate,
    pub codec: StreamCodec,
    pub start_ms: u32,
}

pub enum StreamInput {
    Saved(MediaResource),
    File(MediaResource, &'static str),
    Pcm(Box<dyn junk_libs_pcm::IntegerPcmReader>, u64),
}

// A package lease checks every member because the CD reader can traverse
// several files. Single-file leases check only the consumed member.
#[derive(Clone, Copy)]
pub struct StreamAccess {
    item: ItemId,
    representation: RepresentationId,
    index: Option<usize>,
    generation: u64,
}
impl StreamAccess {
    /// A regular media coordinate for checking a browser's buffered playback.
    /// Disc package access has already been checked for every member.
    pub fn playback_check_path(self) -> String {
        format!(
            "/api/v1/items/{}/representations/{}/{}",
            self.item,
            self.representation,
            self.index.unwrap_or(0)
        )
    }
}

impl CollectionService {
    pub fn recheck_stream(&self, actor: &Principal, access: StreamAccess) -> Result<()> {
        let item = self.reader.item(access.item)?;
        let role = self.authorize(actor, item.collection_id, Action::Browse)?;
        if item.deleted || item.lifecycle.generation != access.generation {
            return Err(Error::NotFound);
        }
        let rep = item
            .captures
            .iter()
            .filter(|c| c.association.item_id == Some(item.id))
            .flat_map(|c| &c.representations)
            .find(|r| r.id == access.representation)
            .ok_or(Error::NotFound)?;
        let private = match access.index {
            Some(index) => {
                rep.files.get(index).ok_or(Error::NotFound)?.visibility == Visibility::Curators
            }
            None => rep
                .files
                .iter()
                .any(|f| f.visibility == Visibility::Curators),
        };
        if private && role == Role::Viewer {
            return Err(Error::Forbidden);
        }
        Ok(())
    }

    pub fn music_stream(
        &self,
        actor: &Principal,
        id: ItemId,
        source_id: RepresentationId,
        request: MusicStreamRequest,
        checkpoint: &dyn Fn() -> Result<()>,
    ) -> Result<(StreamInput, StreamAccess)> {
        checkpoint()?;
        let MusicStreamRequest {
            coordinate,
            codec,
            start_ms,
        } = request;
        codec.validate()?;
        let item = self.reader.item(id)?;
        let role = self.authorize(actor, item.collection_id, Action::Browse)?;
        if item.deleted {
            return Err(Error::NotFound);
        }
        let (capture, source) = item
            .captures
            .iter()
            .filter(|c| c.association.item_id == Some(id))
            .find_map(|capture| {
                capture
                    .representations
                    .iter()
                    .find(|r| r.id == source_id)
                    .map(|r| (capture, r))
            })
            .ok_or(Error::NotFound)?;
        let access = |representation, index| StreamAccess {
            item: id,
            representation,
            index,
            generation: item.lifecycle.generation,
        };
        if source.role != RepresentationRole::PreservationMaster {
            // Older browser queues stored the prepared representation rather
            // than the source track. Resolve its recipe under the same grant.
            if let (StreamCoordinate::File(0), Some(original)) = (coordinate, source.source) {
                self.recheck_stream(actor, access(source_id, Some(0)))?;
                let recipe = self.processing.recipe(RecipeId(source_id.0))?;
                if recipe.source_fingerprint.source == PackageScope::Representation(original)
                    && let [track] = recipe.tracks.as_slice()
                {
                    return self.music_stream(
                        actor,
                        id,
                        original,
                        MusicStreamRequest {
                            coordinate: StreamCoordinate::Track(track.position),
                            ..request
                        },
                        checkpoint,
                    );
                }
            }
            return Err(Error::NotFound);
        }
        let open_file = |rep, index, exact_opus| -> Result<_> {
            let media = self.representation(actor, id, rep, index)?;
            let input = match media.media_type.as_str() {
                "audio/flac" if codec == StreamCodec::Flac && start_ms == 0 => {
                    StreamInput::Saved(media)
                }
                "audio/ogg" if exact_opus && start_ms == 0 => StreamInput::Saved(media),
                "audio/flac" => StreamInput::File(media, "flac"),
                "audio/mpeg" => StreamInput::File(media, "mp3"),
                "audio/ogg" => StreamInput::File(media, "ogg"),
                "audio/wav" => {
                    let mut pcm = junk_libs_pcm::WavReader::open(media.reader, &mut || {
                        checkpoint().map_err(std::io::Error::other)
                    })
                    .map_err(|e| Error::Invalid(e.to_string()))?;
                    use junk_libs_pcm::IntegerPcmReader;
                    let start = pcm_offset(start_ms, pcm.format().sample_rate);
                    let frames = pcm
                        .total_frames()
                        .checked_sub(start)
                        .filter(|n| *n > 0)
                        .ok_or(Error::Invalid(
                            "Playback position is beyond the track".into(),
                        ))?;
                    pcm.seek_frame(start)
                        .map_err(|e| Error::Unavailable(e.to_string()))?;
                    StreamInput::Pcm(Box::new(pcm), frames)
                }
                _ => {
                    return Err(Error::Invalid(
                        "This file is not supported for live audio".into(),
                    ));
                }
            };
            Ok((input, access(rep, Some(index))))
        };
        let StreamCoordinate::Track(position) = coordinate else {
            let StreamCoordinate::File(index) = coordinate else {
                unreachable!()
            };
            if let Some(track) = self
                .recorded_file_tracks(item.collection_id, source_id)?
                .and_then(|m| m.tracks.into_iter().find(|t| t.file_index == index))
            {
                return self.music_stream(
                    actor,
                    id,
                    source_id,
                    MusicStreamRequest {
                        coordinate: StreamCoordinate::Track(track.position),
                        ..request
                    },
                    checkpoint,
                );
            }
            return open_file(source_id, index, false);
        };
        if position == 0 {
            return Err(Error::Invalid("Track numbers start at one".into()));
        }
        let scope = PackageScope::Representation(source_id);
        let fingerprint = self.processing.recorded_source(item.collection_id, scope)?;
        let mut saved = Vec::new();
        for rep in capture
            .representations
            .iter()
            .filter(|r| r.source == Some(source_id))
        {
            let Some(profile) = &rep.profile else {
                continue;
            };
            if !rep
                .files
                .first()
                .is_some_and(|f| f.visibility == Visibility::Shared || role != Role::Viewer)
            {
                continue;
            }
            match self.processing.recipe(RecipeId(rep.id.0)) {
                Ok(recipe)
                    if recipe.source_fingerprint == fingerprint
                        && recipe.tracks.len() == 1
                        && recipe.tracks[0].position == position
                        && profile == &crate::derivatives::profile_for_recipe(&recipe) =>
                {
                    let quality = match profile {
                        ConversionProfile::Flac => u32::MAX,
                        ConversionProfile::Opus { bitrate_kbps } => u32::from(*bitrate_kbps),
                    };
                    saved.push((quality, rep.id));
                }
                Ok(_) | Err(Error::NotFound) => (),
                Err(e) => return Err(e),
            }
        }
        saved.sort_unstable_by_key(|value| std::cmp::Reverse(*value));
        for &(_, rep) in saved.iter().take_while(|(rank, _)| *rank == u32::MAX) {
            match open_file(rep, 0, false) {
                Err(Error::PreparedMediaInvalid | Error::NotFound) => (),
                result => return result,
            }
        }
        let original =
            if let Some(manifest) = self.recorded_file_tracks(item.collection_id, source_id)? {
                let track = manifest
                    .tracks
                    .iter()
                    .find(|t| t.position == position)
                    .ok_or(Error::NotFound)?;
                open_file(source_id, track.file_index, false)
            } else {
                let lease = access(source_id, None);
                (|| {
                    self.recheck_stream(actor, lease)?;
                    if !self.modules.supports_listening(&item.domain.module)
                        || self.modules.inspect(&item.domain).is_err()
                    {
                        return Err(Error::Unavailable(
                            "The original disc audio layout is unsupported".into(),
                        ));
                    }
                    let selection = self.detect_music_selection(item.collection_id, source_id)?;
                    let package = collector_modules::PackageAccess {
                        collection_id: item.collection_id,
                        scope,
                    };
                    let host = crate::derivatives::ConversionHost::for_read(
                        self.processing.as_ref(),
                        package,
                        checkpoint,
                    );
                    let pcm = collector_modules::music::open_conversion_pcm_selection(
                        &host, package, &selection,
                    )?;
                    let track = pcm
                        .tracks
                        .iter()
                        .find(|t| t.position == position)
                        .ok_or(Error::NotFound)?;
                    // Use the same track-local window as durable preparation, including
                    // the first track's captured pregap where the source provides it.
                    let start = pcm_offset(start_ms, 44_100);
                    let frames = track
                        .frame_count
                        .checked_sub(start)
                        .filter(|n| *n > 0)
                        .ok_or(Error::Invalid(
                            "Playback position is beyond the track".into(),
                        ))?;
                    let mut reader = junk_libs_pcm::CdReader(pcm.reader);
                    use junk_libs_pcm::IntegerPcmReader;
                    reader
                        .seek_frame(track.start_frame + start)
                        .map_err(|e| Error::Unavailable(e.to_string()))?;
                    Ok((StreamInput::Pcm(Box::new(reader), frames), lease))
                })()
            };
        checkpoint()?;
        match original {
            Ok(source) => Ok(source),
            Err(error) => {
                // Older archives can retain a playable derivative even if the
                // original package cannot be opened or isn't visible to this role.
                for (rate, rep) in saved.into_iter().filter(|(rate, _)| *rate != u32::MAX) {
                    match open_file(rep, 0, codec == StreamCodec::Opus(rate as u16)) {
                        Err(Error::PreparedMediaInvalid | Error::NotFound) => (),
                        result => return result,
                    }
                }
                Err(error)
            }
        }
    }
}

fn pcm_offset(ms: u32, rate: u32) -> u64 {
    u64::from(ms) * u64::from(rate) / 1000
}
