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
        let mut previous = 0;
        for track in &self.tracks {
            if track.position <= previous
                || track.position > 10_000
                || !files.insert(track.file_id)
                || !matches!(track.audio.codec.as_str(), "flac" | "mp3" | "wav")
                || track.audio.sample_rate == 0
                || !matches!(track.audio.channels, 1 | 2)
                || track.audio.decoded_frames.is_none_or(|n| n == 0)
                || (track.wav_trailing_bytes != 0 && track.audio.codec != "wav")
            {
                return Err(ModuleError::Invalid("invalid file track manifest".into()));
            }
            previous = track.position;
        }
        Ok(())
    }
}
/// Explicit disc directories remain meaningful above or below encoding wrappers.
pub fn path_disc_number(name: &str) -> Option<u16> {
    name.rsplit_once('/')?
        .0
        .split('/')
        .rev()
        .find_map(super::discovery::disc_number)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct MusicImportTrack {
    pub file_name: String,
    pub disc_position: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_position: Option<u16>,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct MusicImportLayout {
    pub tracks: Vec<MusicImportTrack>,
    pub needs_attention: bool,
    pub notices: Vec<String>,
}

pub fn codec_priority(codec: &str) -> u8 {
    match codec {
        "flac" => 0,
        "wav" => 1,
        _ => 2,
    }
}

/// A bounded names-only proposal. Unfamiliar branches require explicit review.
pub fn import_layout(
    names: &[String],
    reviewed: Option<&[MusicImportTrack]>,
) -> ModuleResult<MusicImportLayout> {
    use std::collections::{BTreeMap, BTreeSet};
    let audio = names
        .iter()
        .filter(|name| is_audio_name(name))
        .collect::<Vec<_>>();
    let paths = audio
        .iter()
        .map(|name| name.split('/').collect::<Vec<_>>())
        .collect::<Vec<_>>();
    let mut common = paths.first().map_or(0, |parts| parts.len() - 1);
    for parts in &paths {
        common = (0..common.min(parts.len() - 1))
            .take_while(|index| parts[*index] == paths[0][*index])
            .count();
    }
    let mut counts = BTreeMap::<u16, BTreeMap<String, usize>>::new();
    let mut notices = BTreeSet::new();
    for (name, parts) in audio.iter().zip(&paths) {
        let codec = name.rsplit('.').next().unwrap().to_ascii_lowercase();
        *counts
            .entry(path_disc_number(name).unwrap_or(1))
            .or_default()
            .entry(codec)
            .or_default() += 1;
        if parts[common..parts.len() - 1].iter().any(|part| {
            super::discovery::disc_number(part).is_none()
                && !matches!(part.to_ascii_lowercase().as_str(), "flac" | "wav" | "mp3")
        }) {
            notices.insert("Named audio sections may contain overlapping editions or bonus tracks. Choose the main album files and disc assignments.".into());
        }
    }
    if counts
        .values()
        .any(|formats| formats.values().collect::<BTreeSet<_>>().len() > 1)
    {
        notices.insert(
            "Audio formats have different track counts. Review which files make up the album."
                .into(),
        );
    }
    if counts.keys().any(|disc| *disc > 1)
        && audio.iter().any(|name| path_disc_number(name).is_none())
    {
        notices.insert(
            "Some audio files lack a disc folder in a multi-disc layout. Assign their album discs."
                .into(),
        );
    }
    let tracks = if let Some(reviewed) = reviewed {
        let known = audio
            .iter()
            .map(|name| name.as_str())
            .collect::<BTreeSet<_>>();
        let mut selected = BTreeSet::new();
        if reviewed.is_empty()
            || reviewed.len() > audio.len()
            || reviewed.iter().any(|track| {
                !known.contains(track.file_name.as_str())
                    || !selected.insert(track.file_name.as_str())
                    || track.disc_position == Some(0)
                    || track.display_position == Some(0)
            })
        {
            return Err(ModuleError::Invalid(
                "Choose distinct audio files and positive disc and track numbers for album playback.".into(),
            ));
        }
        reviewed.to_vec()
    } else {
        let preferred = counts
            .iter()
            .map(|(disc, formats)| {
                (
                    *disc,
                    formats
                        .keys()
                        .min_by_key(|codec| codec_priority(codec))
                        .unwrap(),
                )
            })
            .collect::<BTreeMap<_, _>>();
        audio
            .into_iter()
            .filter(|name| {
                preferred[&path_disc_number(name).unwrap_or(1)]
                    .eq_ignore_ascii_case(name.rsplit('.').next().unwrap())
            })
            .map(|name| MusicImportTrack {
                file_name: name.clone(),
                disc_position: path_disc_number(name),
                display_position: None,
            })
            .collect()
    };
    Ok(MusicImportLayout {
        tracks,
        needs_attention: reviewed.is_none() && !notices.is_empty(),
        notices: notices.into_iter().collect(),
    })
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
pub(super) fn analyze_files(
    host: &dyn crate::ModuleHost,
    access: PackageAccess,
    inventory: &PackageInventory,
    reviewed: &[MusicImportAnalysisFile],
    album_selection: Option<&[Option<bool>]>,
) -> ModuleResult<(FileTrackManifest, Vec<String>)> {
    let names = inventory
        .files
        .iter()
        .map(|file| file.display_name.as_str().to_owned())
        .collect::<Vec<_>>();
    let defaults = import_layout(&names, None)?
        .tracks
        .into_iter()
        .map(|track| track.file_name)
        .collect::<std::collections::BTreeSet<_>>();
    let choices = reviewed
        .iter()
        .filter_map(|f| f.album_selected.map(|selected| (f.name.as_str(), selected)))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut tracks = Vec::new();
    let mut warnings = Vec::new();
    let mut skipped = 0;
    let mut tag_bytes = 0usize;
    for (ordinal, (index, file)) in inventory
        .files
        .iter()
        .enumerate()
        .filter(|(_, f)| is_audio_name(f.display_name.as_str()))
        .enumerate()
    {
        host.checkpoint()?;
        let name = file.display_name.as_str();
        let required = album_selection
            .and_then(|selection| selection[index])
            .or_else(|| choices.get(name).copied())
            .unwrap_or_else(|| defaults.contains(name));
        match analyze_file(host, access, file) {
            Ok((mut audio, wav_trailing_bytes)) => {
                tag_bytes += audio
                    .tags
                    .values()
                    .flatten()
                    .map(String::len)
                    .sum::<usize>();
                if tag_bytes > 8 * 1024 * 1024 {
                    return Err(ModuleError::Invalid("package tags exceed 8 MiB".into()));
                }
                let embedded_artwork = !audio.pictures.is_empty();
                audio.pictures.clear();
                tracks.push(FileTrack {
                    embedded_artwork,
                    position: u16::try_from(ordinal + 1)
                        .map_err(|_| ModuleError::Invalid("too many file tracks".into()))?,
                    file_index: index,
                    file_id: file.id,
                    audio,
                    wav_trailing_bytes,
                });
            }
            Err(ModuleError::FailedOperation(reason) | ModuleError::Invalid(reason)) => {
                let reason: String = reason
                    .chars()
                    .filter(|c| !c.is_control())
                    .take(256)
                    .collect();
                let detail = format!("{name}: {reason}");
                if required {
                    return Err(ModuleError::FailedOperation(detail));
                }
                skipped += 1;
                if warnings.len() < 12 {
                    warnings.push(format!(
                        "Alternate audio retained without track analysis: {detail}"
                    ));
                }
            }
            Err(error) => return Err(error),
        }
    }
    let manifest = FileTrackManifest { tracks };
    manifest.validate()?;
    if skipped > warnings.len() {
        warnings.push(format!(
            "{} additional alternate audio files were retained without track analysis.",
            skipped - warnings.len()
        ));
    }
    warnings.extend(limitations(&manifest));
    Ok((manifest, warnings))
}

fn analyze_file(
    host: &dyn crate::ModuleHost,
    access: PackageAccess,
    file: &collector_core::PackageInventoryFile,
) -> ModuleResult<(junk_libs_audio::FileAudioInfo, u64)> {
    let reader = host.package_file(access, file.id)?;
    let extension = file.display_name.as_str().rsplit('.').next();
    let mut wav_trailing_bytes = 0;
    let audio = if extension.is_some_and(|ext| ext.eq_ignore_ascii_case("wav")) {
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
        if wav.source_bits() == 32 {
            let mut block = vec![0; 4096 * format.frame_bytes()];
            loop {
                host.checkpoint()?;
                if wav
                    .read_pcm(&mut block)
                    .map_err(|e| ModuleError::FailedOperation(e.to_string()))?
                    == 0
                {
                    break;
                }
            }
        }
        junk_libs_audio::FileAudioInfo {
            codec: "wav".into(),
            sample_rate: format.sample_rate,
            channels: format.channels,
            bits_per_sample: Some(u32::from(wav.source_bits())),
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
    let expected = extension.unwrap().to_ascii_lowercase();
    if audio.codec != expected {
        return Err(ModuleError::Invalid(
            "audio contents disagree with selected file format".into(),
        ));
    }
    Ok((audio, wav_trailing_bytes))
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

#[cfg(test)]
mod layout_tests {
    use super::*;

    #[test]
    fn loose_alternate_formats_choose_one_complete_copy_without_tag_guesses() {
        let names = [
            "Album/01.flac",
            "Album/02.flac",
            "Album/a.mp3",
            "Album/b.mp3",
            "Album/a.wav",
            "Album/b.wav",
        ]
        .map(str::to_owned);
        let layout = import_layout(&names, None).unwrap();
        assert!(!layout.needs_attention);
        assert_eq!(
            layout
                .tracks
                .iter()
                .map(|track| track.file_name.as_str())
                .collect::<Vec<_>>(),
            ["Album/01.flac", "Album/02.flac"]
        );
    }

    #[test]
    fn both_disc_layouts_keep_numbers_and_choose_lossless_per_disc() {
        for names in [
            [
                "Album/FLAC/Disc 1/01.flac",
                "Album/WAV/Disc 1/01.wav",
                "Album/WAV/CD_2/01.wav",
                "Album/MP3/CD_2/01.mp3",
            ],
            [
                "Album/Disc 1/FLAC/01.flac",
                "Album/Disc 1/WAV/01.wav",
                "Album/CD_2/WAV/01.wav",
                "Album/CD_2/MP3/01.mp3",
            ],
        ] {
            let layout = import_layout(&names.map(str::to_owned), None).unwrap();
            assert!(!layout.needs_attention);
            assert_eq!(
                layout
                    .tracks
                    .iter()
                    .map(|track| (
                        track.file_name.rsplit_once('.').unwrap().1,
                        track.disc_position
                    ))
                    .collect::<Vec<_>>(),
                [("flac", Some(1)), ("wav", Some(2))]
            );
        }
    }

    #[test]
    fn overlapping_sections_and_unequal_counts_require_a_reviewed_selection() {
        let names = [
            "Album/All FLAC/01.flac",
            "Album/All FLAC/02.flac",
            "Album/DLC MP3/01.mp3",
        ]
        .map(str::to_owned);
        assert!(import_layout(&names, None).unwrap().needs_attention);
        let selected = [MusicImportTrack {
            file_name: names[0].clone(),
            disc_position: Some(3),
            display_position: Some(7),
        }];
        let layout = import_layout(&names, Some(&selected)).unwrap();
        assert!(!layout.needs_attention);
        assert_eq!(layout.tracks, selected);
        for tracks in [
            vec![],
            vec![selected[0].clone(), selected[0].clone()],
            vec![MusicImportTrack {
                file_name: "missing.flac".into(),
                disc_position: Some(1),
                display_position: None,
            }],
            vec![MusicImportTrack {
                file_name: names[0].clone(),
                disc_position: Some(0),
                display_position: None,
            }],
            vec![MusicImportTrack {
                display_position: Some(0),
                ..selected[0].clone()
            }],
        ] {
            assert!(import_layout(&names, Some(&tracks)).is_err());
        }
    }
}
