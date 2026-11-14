//! Source selection and authorization for temporary, source-bound playback.
use crate::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StreamCodec {
    Original,
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

    pub fn media_type(self) -> Option<&'static str> {
        match self {
            Self::Original => None,
            Self::Flac => Some("audio/flac"),
            Self::Opus(_) => Some("audio/ogg; codecs=opus"),
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
    pub fn is_disc(self) -> bool {
        self.index.is_none()
    }

    /// Select an authenticated coordinate for checking buffered playback.
    pub fn playback_check_path(self, track: Option<u16>) -> String {
        if let (None, Some(track)) = (self.index, track) {
            return format!(
                "/api/v1/items/{}/music-stream/{}?mode=original&track={track}",
                self.item, self.representation,
            );
        }
        format!(
            "/api/v1/items/{}/representations/{}/{}",
            self.item,
            self.representation,
            self.index.unwrap_or(0)
        )
    }
}

impl CollectionService {
    fn playback_capture(&self, item: &Item, source: RepresentationId) -> Result<Capture> {
        if let Some(capture) = item.captures.iter().find(|capture| {
            capture.association.item_id == Some(item.id)
                && capture.representations.iter().any(|rep| rep.id == source)
        }) {
            return Ok(capture.clone());
        }
        for id in &item.lifecycle.historical_captures {
            match self.reader.capture(*id) {
                Ok(capture)
                    if capture.collection_id == item.collection_id
                        && capture.association.item_id == Some(item.id)
                        && capture.representations.iter().any(|rep| rep.id == source) =>
                {
                    return Ok(capture);
                }
                Ok(_) | Err(Error::NotFound) => (),
                Err(error) => return Err(error),
            }
        }
        Err(Error::NotFound)
    }

    pub fn recheck_stream(&self, actor: &Principal, access: StreamAccess) -> Result<()> {
        let item = self.reader.item(access.item)?;
        let role = self.authorize(actor, item.collection_id, Action::Browse)?;
        if item.deleted || item.lifecycle.generation != access.generation {
            return Err(Error::NotFound);
        }
        let capture = self.playback_capture(&item, access.representation)?;
        let rep = capture
            .representations
            .iter()
            .find(|rep| rep.id == access.representation)
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
        let capture = self.playback_capture(&item, source_id)?;
        let source = capture
            .representations
            .iter()
            .find(|rep| rep.id == source_id)
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
            let mut media = self.representation(actor, id, rep, index)?;
            let input = match media.media_type.as_str() {
                "audio/flac" | "audio/mpeg" if codec == StreamCodec::Flac => {
                    StreamInput::Saved(media)
                }
                "audio/flac" | "audio/mpeg" | "audio/wav" | "audio/ogg"
                    if codec == StreamCodec::Original =>
                {
                    StreamInput::Saved(media)
                }
                "audio/ogg" if exact_opus => {
                    media.media_type = "audio/ogg; codecs=opus".into();
                    StreamInput::Saved(media)
                }
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
        let profile = match codec {
            StreamCodec::Original => None,
            StreamCodec::Flac => Some(ConversionProfile::Flac),
            StreamCodec::Opus(bitrate_kbps) => Some(ConversionProfile::Opus { bitrate_kbps }),
        };
        if let Some(profile) = &profile {
            for rep in capture.representations.iter().filter(|rep| {
                rep.source == Some(source_id) && rep.profile.as_ref() == Some(profile)
            }) {
                if !rep.files.first().is_some_and(|file| {
                    file.visibility == Visibility::Shared || role != Role::Viewer
                }) {
                    continue;
                }
                match self.processing.recipe(RecipeId(rep.id.0)) {
                    Ok(recipe)
                        if recipe.source_fingerprint == fingerprint
                            && recipe.tracks.len() == 1
                            && recipe.tracks[0].position == position
                            && profile == &crate::derivatives::profile_for_recipe(&recipe) =>
                    {
                        match open_file(rep.id, 0, matches!(codec, StreamCodec::Opus(_))) {
                            Err(Error::PreparedMediaInvalid | Error::NotFound) => (),
                            result => return result,
                        }
                    }
                    Ok(_) | Err(Error::NotFound) => (),
                    Err(error) => return Err(error),
                }
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
                    let mut reader = junk_libs_pcm::CdReader(pcm.reader);
                    if codec == StreamCodec::Original {
                        let wave = junk_libs_pcm::wav_output::PcmWave::new(
                            Box::new(reader),
                            track.start_frame,
                            track.frame_count,
                        )
                        .map_err(|error| Error::Invalid(error.to_string()))?;
                        let identity = serde_json::to_vec(&(
                            "cd-track-wave-v1",
                            &fingerprint,
                            track.start_frame,
                            track.frame_count,
                        ))
                        .map_err(|error| Error::Storage(error.to_string()))?;
                        return Ok((
                            StreamInput::Saved(MediaResource {
                                visibility: Visibility::Curators,
                                length: wave.byte_length(),
                                reader: Box::new(wave),
                                media_type: "audio/wav".into(),
                                digest: Digest::sha256(&identity),
                                download_name: format!("Track {position:02}.wav"),
                            }),
                            lease,
                        ));
                    }
                    let start = pcm_offset(start_ms, 44_100);
                    let frames = track
                        .frame_count
                        .checked_sub(start)
                        .filter(|n| *n > 0)
                        .ok_or(Error::Invalid(
                            "Playback position is beyond the track".into(),
                        ))?;
                    use junk_libs_pcm::IntegerPcmReader;
                    reader
                        .seek_frame(track.start_frame + start)
                        .map_err(|e| Error::Unavailable(e.to_string()))?;
                    Ok((StreamInput::Pcm(Box::new(reader), frames), lease))
                })()
            };
        checkpoint()?;
        original
    }
}

fn pcm_offset(ms: u32, rate: u32) -> u64 {
    u64::from(ms) * u64::from(rate) / 1000
}
