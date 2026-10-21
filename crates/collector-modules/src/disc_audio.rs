//! Source-bound CD listening contracts. Shared libraries own geometry and PCM reads.
use crate::{ModuleError, ModuleHost, ModuleResult, PackageAccess};
use collector_core::{PackageInventory, RepresentationId};
use serde::{Deserialize, Serialize};
use std::io::{Read, Seek, SeekFrom};

pub const GAME_CD_AUDIO_OPERATION: &str = "org.junk.games.cd-audio";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct TrackMetadataOverride {
    #[serde(default)]
    pub album_selected: Option<bool>,
    pub source_representation_id: RepresentationId,
    pub track_position: u16,
    pub title: Option<String>,
    pub disc_position: Option<u16>,
    pub display_position: Option<u16>,
}

/// Conversion coordinates are logical stereo frames; source coordinates remain
/// in the recorded format manifest. Only explicit retained HTOA has a leading count.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConversionPcmTrack {
    pub position: u16,
    pub start_frame: u64,
    pub frame_count: u64,
    pub leading_hidden_frames: u64,
}
pub struct ConversionPcmSource {
    pub reader: Box<dyn junk_libs_disc::CdPcmReader + Send>,
    pub tracks: Vec<ConversionPcmTrack>,
    pub total_frames: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct DiscAudioTrack {
    pub position: u16,
    pub start_frame: u64,
    pub frame_count: u64,
    pub source_start_frame: u64,
    pub source_end_frame: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct DiscAudioManifest {
    pub version: u32,
    pub anchor_file_index: usize,
    pub tracks: Vec<DiscAudioTrack>,
    pub data_tracks: Vec<u16>,
}
impl DiscAudioManifest {
    pub fn validate(&self) -> ModuleResult<()> {
        let invalid = || ModuleError::Invalid("Invalid source-bound CD audio manifest".into());
        if self.tracks.is_empty() && self.data_tracks.is_empty() {
            return Err(invalid());
        }
        if self.version != 1
            || self.anchor_file_index >= 10000
            || self.tracks.len() + self.data_tracks.len() > 99
        {
            return Err(invalid());
        }
        let mut next = 0u64;
        let mut prior_position = 0;
        let mut prior_end = 0;
        for track in &self.tracks {
            if !(1..=99).contains(&track.position)
                || track.position <= prior_position
                || track.start_frame != next
                || track.frame_count == 0
                || track.source_start_frame < prior_end
                || track.source_end_frame.checked_sub(track.source_start_frame)
                    != Some(track.frame_count)
            {
                return Err(invalid());
            }
            next = next.checked_add(track.frame_count).ok_or_else(invalid)?;
            prior_position = track.position;
            prior_end = track.source_end_frame;
        }
        if self.data_tracks.windows(2).any(|v| v[0] >= v[1])
            || self
                .data_tracks
                .iter()
                .any(|p| !(1..=99).contains(p) || self.tracks.iter().any(|t| t.position == *p))
        {
            return Err(invalid());
        }
        Ok(())
    }
    pub fn conversion_tracks(&self) -> Vec<ConversionPcmTrack> {
        self.tracks
            .iter()
            .map(|t| ConversionPcmTrack {
                position: t.position,
                start_frame: t.start_frame,
                frame_count: t.frame_count,
                leading_hidden_frames: 0,
            })
            .collect()
    }
}
fn error(e: impl std::fmt::Display) -> ModuleError {
    ModuleError::FailedOperation(e.to_string())
}

fn layout(
    host: &dyn ModuleHost,
    access: PackageAccess,
    inventory: &PackageInventory,
    anchor: usize,
) -> ModuleResult<junk_libs_disc::cue_audio::CueAudioDisc> {
    host.checkpoint()?;
    let (id, text) = crate::music::read_cue(host, access, inventory, anchor)?;
    junk_libs_disc::cue_audio::CueAudioDisc::from_text(&text, |name| {
        inventory
            .resolve_relative(id, name)
            .map(|f| f.bytes.0)
            .map_err(|e| junk_libs_core::AnalysisError::invalid_format(e.to_string()))
    })
    .map_err(error)
}
fn manifest(
    disc: &junk_libs_disc::cue_audio::CueAudioDisc,
    anchor: usize,
) -> ModuleResult<DiscAudioManifest> {
    let mut result = DiscAudioManifest {
        version: 1,
        anchor_file_index: anchor,
        tracks: vec![],
        data_tracks: vec![],
    };
    let mut next = 0u64;
    for track in &disc.tracks {
        if track.kind != junk_libs_disc::layout::TrackKind::Audio {
            result.data_tracks.push(u16::from(track.number));
            continue;
        }
        let frames = track
            .end_frame
            .checked_sub(track.start_frame)
            .ok_or_else(|| error("CD audio range underflow"))?;
        result.tracks.push(DiscAudioTrack {
            position: u16::from(track.number),
            start_frame: next,
            frame_count: frames,
            source_start_frame: track.start_frame,
            source_end_frame: track.end_frame,
        });
        next = next
            .checked_add(frames)
            .ok_or_else(|| error("CD audio range overflow"))?;
    }
    result.validate()?;
    Ok(result)
}
pub fn inspect(
    host: &dyn ModuleHost,
    access: PackageAccess,
    inventory: &PackageInventory,
    anchor: usize,
) -> ModuleResult<DiscAudioManifest> {
    manifest(&layout(host, access, inventory, anchor)?, anchor)
}
pub fn discover(
    host: &dyn ModuleHost,
    access: PackageAccess,
    inventory: &PackageInventory,
) -> ModuleResult<Option<DiscAudioManifest>> {
    let anchors = inventory
        .files
        .iter()
        .enumerate()
        .filter(|(_, f)| {
            f.display_name
                .as_str()
                .to_ascii_lowercase()
                .ends_with(".cue")
        })
        .map(|(i, _)| i)
        .collect::<Vec<_>>();
    match anchors.as_slice() {
        [] => Ok(None),
        [one] => inspect(host, access, inventory, *one).map(Some),
        _ => Err(ModuleError::UnavailableCapability(
            "Choose a CUE anchor before inspecting disc audio".into(),
        )),
    }
}
struct Reader(Box<dyn crate::RepresentationReader>);
impl Read for Reader {
    fn read(&mut self, b: &mut [u8]) -> std::io::Result<usize> {
        self.0.read(b)
    }
}
impl Seek for Reader {
    fn seek(&mut self, p: SeekFrom) -> std::io::Result<u64> {
        self.0.seek(p)
    }
}
pub fn open_conversion(
    host: &dyn ModuleHost,
    access: PackageAccess,
    inventory: &PackageInventory,
    anchor: usize,
) -> ModuleResult<ConversionPcmSource> {
    let disc = layout(host, access, inventory, anchor)?;
    let recorded = manifest(&disc, anchor)?;
    let id = inventory
        .files
        .get(anchor)
        .ok_or_else(|| error("CUE anchor absent"))?
        .id;
    let mut readers = vec![];
    for track in disc
        .tracks
        .iter()
        .filter(|t| t.kind == junk_libs_disc::layout::TrackKind::Audio)
    {
        host.checkpoint()?;
        let reader = disc
            .range_reader_with(track.start_frame, track.end_frame, |name| {
                let get = || -> ModuleResult<Box<dyn junk_libs_core::ReadSeek + Send>> {
                    host.checkpoint()?;
                    let name = name
                        .to_str()
                        .ok_or_else(|| error("CUE filename is not UTF-8"))?;
                    let member = inventory.resolve_relative(id, name).map_err(error)?;
                    Ok(Box::new(Reader(host.package_file(access, member.id)?)))
                };
                get().map_err(|e| junk_libs_core::AnalysisError::invalid_format(e.to_string()))
            })
            .map_err(error)?;
        readers.push(Box::new(reader) as Box<dyn junk_libs_disc::CdPcmReader + Send>);
    }
    let reader = junk_libs_disc::pcm::PcmSequenceReader::new(readers).map_err(error)?;
    let total_frames = junk_libs_disc::CdPcmReader::total_frames(&reader);
    Ok(ConversionPcmSource {
        reader: Box::new(reader),
        tracks: recorded.conversion_tracks(),
        total_frames,
    })
}

pub fn validate_track_overrides(edits: &[TrackMetadataOverride]) -> ModuleResult<()> {
    let mut sources = std::collections::BTreeSet::new();
    if edits.len() > 10000 {
        return Err(ModuleError::Invalid("Too many source track edits".into()));
    }
    for edit in edits {
        if edit.track_position == 0
            || edit.disc_position == Some(0)
            || edit.display_position == Some(0)
            || edit
                .title
                .as_ref()
                .is_some_and(|s| s.len() > 4096 || s.chars().any(char::is_control))
            || !sources.insert((edit.source_representation_id, edit.track_position))
        {
            return Err(ModuleError::Invalid("Invalid source track edit".into()));
        }
    }
    Ok(())
}
