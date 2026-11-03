//! File-track mapping and purchase-package interpretation. Decoding stays shared.
use super::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileTrack {
    /// Immutable package order; independent from editable printed disc/track labels.
    pub position: u16,
    pub file_index: usize,
    pub file_id: PackageFileId,
    pub embedded_artwork: bool,
    pub audio: junk_libs_audio::FileAudioInfo,
    /// Preserved original bytes outside a complete declared RIFF segment.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub wav_trailing_bytes: u64,
}
fn is_zero(value: &u64) -> bool {
    *value == 0
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileTrackManifest {
    pub tracks: Vec<FileTrack>,
}
impl FileTrackManifest {
    pub fn validate(&self) -> ModuleResult<()> {
        if self.tracks.is_empty() || self.tracks.len() > 10_000 {
            return Err(ModuleError::Invalid("invalid file track count".into()));
        }
        let mut files = std::collections::BTreeSet::new();
        for (i, track) in self.tracks.iter().enumerate() {
            if usize::from(track.position) != i + 1
                || !files.insert(track.file_id)
                || !matches!(track.audio.codec.as_str(), "flac" | "mp3" | "wav")
                || track.audio.sample_rate == 0
                || !matches!(track.audio.channels, 1 | 2)
                || track.audio.decoded_frames.is_none_or(|n| n == 0)
                || (track.wav_trailing_bytes != 0 && track.audio.codec != "wav")
            {
                return Err(ModuleError::Invalid("invalid file track manifest".into()));
            }
        }
        Ok(())
    }
}
pub fn is_audio_name(name: &str) -> bool {
    name.rsplit('.').next().is_some_and(|ext| {
        ext.eq_ignore_ascii_case("flac")
            || ext.eq_ignore_ascii_case("mp3")
            || ext.eq_ignore_ascii_case("wav")
    })
}
pub fn is_file_package(names: &[PackageFileName]) -> bool {
    !names.iter().any(|n| {
        n.as_str().to_ascii_lowercase().ends_with(".cue")
            || n.as_str().to_ascii_lowercase().ends_with(".toc")
    }) && names.iter().any(|n| is_audio_name(n.as_str()))
}
pub fn analyze_files(
    host: &dyn crate::ModuleHost,
    access: PackageAccess,
    inventory: &PackageInventory,
) -> ModuleResult<FileTrackManifest> {
    if inventory
        .files
        .iter()
        .filter(|file| is_audio_name(file.display_name.as_str()))
        .count()
        > 10_000
    {
        return Err(ModuleError::Invalid("too many file tracks".into()));
    }
    let mut tracks = Vec::new();
    let mut tag_bytes = 0usize;
    for (index, file) in inventory
        .files
        .iter()
        .enumerate()
        .filter(|(_, f)| is_audio_name(f.display_name.as_str()))
    {
        host.checkpoint()?;
        let reader = host.package_file(access, file.id)?;
        let extension = file.display_name.as_str().rsplit('.').next();
        let mut wav_trailing_bytes = 0;
        let mut audio = if extension.is_some_and(|ext| ext.eq_ignore_ascii_case("wav")) {
            use junk_libs_pcm::IntegerPcmReader;
            let mut checkpoint_error = None;
            let wav = junk_libs_pcm::WavReader::open(reader, &mut || {
                host.checkpoint().map_err(|error| {
                    checkpoint_error = Some(error.clone());
                    std::io::Error::other(error)
                })
            });
            if let Some(error) = checkpoint_error {
                return Err(error);
            }
            let mut wav = wav.map_err(|e| ModuleError::FailedOperation(e.to_string()))?;
            let format = wav.format();
            wav_trailing_bytes = wav.trailing_bytes();
            junk_libs_audio::FileAudioInfo {
                codec: "wav".into(),
                sample_rate: format.sample_rate,
                channels: format.channels,
                bits_per_sample: Some(u32::from(format.bits)),
                decoded_frames: Some(wav.total_frames()),
                tags: std::mem::take(&mut wav.metadata.tags),
                pictures: std::mem::take(&mut wav.metadata.pictures),
                ..Default::default()
            }
        } else {
            let mut checkpoint_error = None;
            let audio = junk_libs_audio::inspect_file_audio(
                junk_libs_audio::ByteSource::new(
                    reader,
                    junk_libs_audio::MediaHint::new(Some(file.bytes.0), extension, None),
                ),
                &mut || {
                    host.checkpoint().map_err(|error| {
                        checkpoint_error = Some(error);
                        junk_libs_audio::AudioDecodeError::Cancelled
                    })
                },
            );
            if let Some(error) = checkpoint_error {
                return Err(error);
            }
            audio.map_err(|error| ModuleError::FailedOperation(error.to_string()))?
        };
        tag_bytes += audio
            .tags
            .values()
            .flatten()
            .map(String::len)
            .sum::<usize>();
        if tag_bytes > 8 * 1024 * 1024 {
            return Err(ModuleError::Invalid("package tags exceed 8 MiB".into()));
        }
        let expected = extension.unwrap().to_ascii_lowercase();
        if audio.codec != expected {
            return Err(ModuleError::Invalid(
                "audio contents disagree with selected file format".into(),
            ));
        }
        let embedded_artwork = !audio.pictures.is_empty();
        audio.pictures.clear();
        tracks.push(FileTrack {
            embedded_artwork,
            position: u16::try_from(tracks.len() + 1)
                .map_err(|_| ModuleError::Invalid("too many file tracks".into()))?,
            file_index: index,
            file_id: file.id,
            audio,
            wav_trailing_bytes,
        });
    }
    let result = FileTrackManifest { tracks };
    result.validate()?;
    Ok(result)
}

pub fn limitations(manifest: &FileTrackManifest) -> Vec<String> {
    let affected = manifest
        .tracks
        .iter()
        .filter(|t| t.wav_trailing_bytes != 0)
        .count();
    if affected == 0 {
        return vec![];
    }
    vec![format!(
        "WAV warning: {affected} file(s) contain trailing bytes outside their declared RIFF segment. The complete originals are preserved; playback and preparation use only the declared audio. Trailing bytes are not interpreted as audio or metadata."
    )]
}

/// Source-local descriptive defaults. These never acquire CD/catalog identity strength.
pub fn metadata_candidate(
    manifest: &FileTrackManifest,
    source: RepresentationId,
    capture: collector_core::CaptureId,
    fingerprint: &collector_core::SourceFingerprint,
    shared_files: &std::collections::BTreeSet<usize>,
    artwork: Option<collector_core::AssetId>,
    fallback_title: &str,
) -> ModuleResult<collector_core::MetadataCandidateObservation> {
    use collector_core::*;
    let tracks = manifest
        .tracks
        .iter()
        .filter(|t| shared_files.contains(&t.file_index))
        .collect::<Vec<_>>();
    let consensus = |key: &str| -> Option<String> {
        let values = tracks
            .iter()
            .filter_map(|t| t.audio.tag(key))
            .collect::<std::collections::BTreeSet<_>>();
        (values.len() == 1).then(|| values.first().unwrap().to_string())
    };
    let title = consensus("album")
        .or_else(|| {
            (tracks.len() == 1)
                .then(|| tracks[0].audio.tag("title").map(str::to_owned))
                .flatten()
        })
        .unwrap_or_else(|| fallback_title.to_owned());
    let mut candidate = MetadataCandidateObservation {
        identity: MetadataCandidateIdentity {
            provider: ModuleId("org.junk.music.file-tags".into()),
            provider_record_id: source.to_string(),
            provider_record_version: None,
        },
        title: MetadataText {
            value: title,
            language: None,
        },
        subtitle: consensus("album_artist")
            .or_else(|| consensus("artist"))
            .map(|value| MetadataText {
                value,
                language: None,
            }),
        artwork,
        complete_identity: false,
        strength: MatchStrength::Name,
        provider_priority: 40,
        observation: RawJson::from_value(
            serde_json::json!({"source_representation_id": source, "capture_id": capture, "source_fingerprint": fingerprint, "tag_manifest_digest": Digest::sha256(&serde_json::to_vec(manifest).map_err(|e| ModuleError::Invalid(e.to_string()))?), "shared_file_indices": shared_files, "title_fallback": fallback_title}),
        ),
    };
    candidate.identity.provider_record_version = Some(
        Digest::sha256(
            &serde_json::to_vec(&candidate).map_err(|e| ModuleError::Invalid(e.to_string()))?,
        )
        .hex,
    );
    candidate
        .validate()
        .map_err(|e| ModuleError::Invalid(e.to_string()))?;
    Ok(candidate)
}
