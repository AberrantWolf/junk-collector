use std::io::SeekFrom;

use collector_core::{
    ByteLength, CdSector, CdTrackKind, CdTrackLayout, ModuleData, PackageFileId, PackageInventory,
    PcmFrame, RawJson, SampleFrame, StereoSampleFrameOffset,
};
use junk_libs_accuraterip::DiscPcmTrack;
use junk_libs_disc::TrackKind;
use serde::{Deserialize, Serialize};

use crate::{
    CapabilityId, CollectionModule, DomainFieldDescriptor, DomainFieldKind, ModuleDescriptor,
    ModuleError, ModulePresentation, ModuleResult, OperationOutput, OperationRequest, PatchValue,
    apply_patch_value, basic_presentation, descriptor, module_data_from_raw_object,
    raw_domain_object,
};

#[derive(Clone, Debug, Default, Serialize, Deserialize, utoipa::ToSchema)]
pub struct MusicItemData {
    pub artist: Option<String>,
    pub label: Option<String>,
    pub release_year: Option<u16>,
    #[serde(default)]
    pub tracks: Vec<Track>,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Track {
    pub position: u16,
    pub title: String,
    pub start: SampleFrame,
    pub length: SampleFrame,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MusicItemPatch {
    pub artist: PatchValue<String>,
    pub label: PatchValue<String>,
    pub release_year: PatchValue<u16>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct MusicMetadataOverrides {
    pub artist: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AudioVerificationStatus {
    Verified,
    IncompleteSearch,
    Mismatched,
    NoData,
    AmbiguousOffsets,
    Unavailable,
    Unsupported,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AudioChecksumVersion {
    AccurateRipV1,
    AccurateRipV2,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AudioTrackVerification {
    pub position: u8,
    pub status: AudioVerificationStatus,
    pub checksum_version: Option<AudioChecksumVersion>,
    pub confidence: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AudioVerificationResult {
    pub status: AudioVerificationStatus,
    pub selected_offset: Option<StereoSampleFrameOffset>,
    /// Inclusive v1 search range; v2 candidate evaluation is listed separately.
    pub evaluated_offset_start: StereoSampleFrameOffset,
    pub evaluated_offset_end: StereoSampleFrameOffset,
    pub v2_evaluated_offsets: Vec<StereoSampleFrameOffset>,
    pub unavailable_leading_frames: PcmFrame,
    pub unavailable_trailing_frames: PcmFrame,
    pub tracks: Vec<AudioTrackVerification>,
    pub limitations: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CdDiscIds {
    pub musicbrainz: String,
    pub freedb: String,
    pub accuraterip_id1: String,
    pub accuraterip_id2: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CdSourceSpan {
    pub position: u8,
    pub source_file_id: PackageFileId,
    pub source_byte_offset: u64,
    pub source_bytes: ByteLength,
    pub stored_index_00_frames: u32,
    pub synthetic_pregap_frames: u32,
    pub synthetic_postgap_frames: u32,
}

impl AudioVerificationResult {
    pub fn validate(&self) -> collector_core::Result<()> {
        if self.evaluated_offset_start.0 > self.evaluated_offset_end.0
            || self.selected_offset.is_some_and(|offset| {
                offset.0 < self.evaluated_offset_start.0 || offset.0 > self.evaluated_offset_end.0
            })
            || self.v2_evaluated_offsets.len() > 5879
            || self
                .v2_evaluated_offsets
                .windows(2)
                .any(|pair| pair[0].0 >= pair[1].0)
            || self.v2_evaluated_offsets.iter().any(|offset| {
                offset.0 < self.evaluated_offset_start.0 || offset.0 > self.evaluated_offset_end.0
            })
            || self.tracks.len() > 99
        {
            return Err(collector_core::Error::Invalid(
                "invalid audio verification bounds".into(),
            ));
        }
        let mut previous = 0;
        for track in &self.tracks {
            if track.position <= previous || track.position > 99 {
                return Err(collector_core::Error::Invalid(
                    "audio tracks must have unique ordered positions".into(),
                ));
            }
            previous = track.position;
            if track.status == AudioVerificationStatus::Verified
                && (track.checksum_version.is_none()
                    || track.confidence.is_none_or(|value| value == 0))
            {
                return Err(collector_core::Error::Invalid(
                    "verified track needs checksum version and positive confidence".into(),
                ));
            }
        }
        if self.status == AudioVerificationStatus::Verified
            && (self.tracks.is_empty()
                || self.selected_offset.is_none()
                || self
                    .tracks
                    .iter()
                    .any(|track| track.status != AudioVerificationStatus::Verified))
        {
            return Err(collector_core::Error::Invalid(
                "whole-disc verification requires every track and a selected offset".into(),
            ));
        }
        if self.limitations.len() > 128
            || self
                .limitations
                .iter()
                .any(|value| value.len() > 4096 || value.chars().any(char::is_control))
        {
            return Err(collector_core::Error::Invalid(
                "audio limitations are oversized or invalid".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum CdProcessingResult {
    Layout {
        tracks: Vec<CdTrackLayout>,
        source_spans: Vec<CdSourceSpan>,
        disc_ids: Option<CdDiscIds>,
        limitations: Vec<String>,
    },
    AudioVerification {
        verification: AudioVerificationResult,
    },
}

struct DiscReaderAdapter(Box<dyn crate::RepresentationReader>);

impl std::io::Read for DiscReaderAdapter {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        self.0.read(buffer)
    }
}

impl std::io::Seek for DiscReaderAdapter {
    fn seek(&mut self, position: SeekFrom) -> std::io::Result<u64> {
        self.0.seek(position)
    }
}

/// Package identities and authorization remain in Collector; integer decoding and
/// source-boundary reads are shared mechanisms.
pub type PackageCdPcmReader = junk_libs_disc::pcm::SourcePcmReader;

struct PreparedCue {
    resolved: junk_libs_disc::CueResolvedLayout,
    result: CdProcessingResult,
    first_index_01_absolute_sector: u64,
    leadout_absolute_sector: u64,
}

fn module_disc_error(error: junk_libs_core::AnalysisError) -> ModuleError {
    match error {
        junk_libs_core::AnalysisError::UnsupportedVariant(message) => {
            ModuleError::UnavailableCapability(message)
        }
        error => ModuleError::FailedOperation(error.to_string()),
    }
}

fn validate_strict_cue(sheet: &junk_libs_disc::CueSheet) -> ModuleResult<()> {
    let mut previous_track = 0_u8;
    for file in &sheet.files {
        if !file.file_type.eq_ignore_ascii_case("BINARY") {
            return Err(ModuleError::UnavailableCapability(format!(
                "CD analysis does not support CUE {} sources",
                file.file_type
            )));
        }
        for track in &file.tracks {
            if track.number <= previous_track || track.number > 99 {
                return Err(ModuleError::FailedOperation(
                    "CUE track numbers must be unique and strictly increasing".into(),
                ));
            }
            previous_track = track.number;
            let mut previous_index = None;
            let mut previous_sector = None;
            let mut index_00 = 0_usize;
            let mut index_01 = 0_usize;
            for index in &track.indexes {
                let sector = index.to_sector_offset();
                if previous_index.is_some_and(|value| index.number <= value)
                    || previous_sector.is_some_and(|value| sector < value)
                {
                    return Err(ModuleError::FailedOperation(
                        "CUE indexes must be unique and strictly ordered".into(),
                    ));
                }
                previous_index = Some(index.number);
                previous_sector = Some(sector);
                index_00 += usize::from(index.number == 0);
                index_01 += usize::from(index.number == 1);
            }
            if index_00 > 1 || index_01 != 1 {
                return Err(ModuleError::FailedOperation(
                    "each CUE track needs exactly one INDEX 01 and at most one INDEX 00".into(),
                ));
            }
        }
    }
    if previous_track == 0 {
        return Err(ModuleError::FailedOperation("CUE has no tracks".into()));
    }
    Ok(())
}

fn validate_strict_cue_text(cue_text: &str) -> ModuleResult<()> {
    for (line_index, line) in cue_text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let fields = line.split_whitespace().collect::<Vec<_>>();
        let keyword = fields[0].to_ascii_uppercase();
        if keyword == "FLAGS"
            && fields
                .iter()
                .skip(1)
                .any(|flag| flag.eq_ignore_ascii_case("4CH"))
        {
            return Err(ModuleError::UnavailableCapability(
                "four-channel CD audio is not supported by the stereo PCM reader".into(),
            ));
        }
        let exact_fields = match keyword.as_str() {
            "TRACK" | "INDEX" => Some(3),
            "PREGAP" | "POSTGAP" => Some(2),
            _ => None,
        };
        if exact_fields.is_some_and(|expected| fields.len() != expected) {
            return Err(ModuleError::FailedOperation(format!(
                "CUE line {} has trailing or missing structural fields",
                line_index + 1
            )));
        }
        if !matches!(
            keyword.as_str(),
            "FILE"
                | "TRACK"
                | "INDEX"
                | "PREGAP"
                | "POSTGAP"
                | "REM"
                | "TITLE"
                | "PERFORMER"
                | "SONGWRITER"
                | "CATALOG"
                | "CDTEXTFILE"
                | "ISRC"
                | "FLAGS"
        ) {
            return Err(ModuleError::FailedOperation(format!(
                "unsupported CUE directive on line {}",
                line_index + 1
            )));
        }
    }
    Ok(())
}

fn prepare_cue(
    cue_text: &str,
    cue_file_id: PackageFileId,
    inventory: &PackageInventory,
) -> ModuleResult<PreparedCue> {
    validate_strict_cue_text(cue_text)?;
    if !junk_libs_disc::check_cue_compat(cue_text).is_standard() {
        return Err(ModuleError::UnavailableCapability(
            "CD analysis requires a standard BINARY CUE sheet".into(),
        ));
    }
    let sheet = junk_libs_disc::cue::parse_cue(cue_text).map_err(module_disc_error)?;
    validate_strict_cue(&sheet)?;
    let resolve = |name: &str| {
        inventory
            .resolve_relative(cue_file_id, name)
            .map_err(|error| junk_libs_core::AnalysisError::InvalidFormat(error.to_string()))
    };
    let mut source_ids = std::collections::BTreeSet::new();
    for file in &sheet.files {
        if !source_ids.insert(resolve(&file.filename).map_err(module_disc_error)?.id) {
            return Err(ModuleError::FailedOperation(
                "CUE repeats a package source file".into(),
            ));
        }
    }
    let resolved = junk_libs_disc::compute_cue_resolved_layout(&sheet, |name| {
        resolve(name).map(|file| file.bytes.0)
    })
    .map_err(module_disc_error)?;
    let spans = junk_libs_disc::compute_cue_track_spans(&sheet, |name| {
        resolve(name).map(|file| file.bytes.0)
    })
    .map_err(module_disc_error)?;
    for span in &spans {
        let track = sheet
            .files
            .iter()
            .flat_map(|file| &file.tracks)
            .find(|track| track.number == span.track_number)
            .expect("resolved track exists");
        let frame_bytes = junk_libs_disc::cue::checked_sector_size_for_mode(&track.mode)
            .map_err(module_disc_error)?;
        let end = span
            .byte_offset
            .checked_add(span.byte_len)
            .ok_or_else(|| ModuleError::FailedOperation("CUE span overflow".into()))?;
        for index in &track.indexes {
            let byte = index
                .to_sector_offset()
                .checked_mul(frame_bytes)
                .ok_or_else(|| ModuleError::FailedOperation("CUE index overflow".into()))?;
            if byte < span.byte_offset || byte >= end {
                return Err(ModuleError::FailedOperation(
                    "CUE index falls outside its stored track span".into(),
                ));
            }
        }
    }
    let first_index_01_absolute_sector = u64::from(resolved.tracks[0].absolute_offset);
    let leadout_absolute_sector = resolved
        .files
        .last()
        .map(|file| u64::from(file.file_end_disc_sector()))
        .ok_or_else(|| ModuleError::FailedOperation("CUE has no source files".into()))?;
    let all_audio = resolved
        .tracks
        .iter()
        .all(|track| track.kind == TrackKind::Audio);
    let has_synthetic_gaps = sheet
        .files
        .iter()
        .flat_map(|file| &file.tracks)
        .any(|track| track.pregap_frames != 0 || track.postgap_frames != 0);
    let declared_tracks = sheet
        .files
        .iter()
        .flat_map(|file| &file.tracks)
        .collect::<Vec<_>>();
    let tracks = resolved
        .tracks
        .iter()
        .zip(&declared_tracks)
        .map(|(layout, track)| {
            let source = sheet
                .files
                .iter()
                .zip(&resolved.files)
                .find(|(file, _)| file.tracks.iter().any(|entry| entry.number == track.number))
                .expect("resolved track retains its source file");
            let index_00 = track
                .indexes
                .iter()
                .find(|index| index.number == 0)
                .map(|index| {
                    i128::from(source.1.file_start_disc_sector)
                        + i128::from(index.to_sector_offset())
                        - i128::from(first_index_01_absolute_sector)
                })
                .map(|sector| {
                    i64::try_from(sector)
                        .map(CdSector)
                        .map_err(|_| ModuleError::FailedOperation("CUE index overflow".into()))
                })
                .transpose()?;
            let index_01 = i64::try_from(
                i128::from(layout.absolute_offset) - i128::from(first_index_01_absolute_sector),
            )
            .map(CdSector)
            .map_err(|_| ModuleError::FailedOperation("CUE index overflow".into()))?;
            let (pcm_start, pcm_length) = if all_audio && !has_synthetic_gaps {
                let start = u64::try_from(index_01.0)
                    .ok()
                    .and_then(|value| value.checked_mul(588))
                    .ok_or_else(|| {
                        ModuleError::UnavailableCapability(
                            "negative program audio cannot form whole-disc PCM".into(),
                        )
                    })?;
                (
                    Some(PcmFrame(start)),
                    Some(PcmFrame(u64::from(layout.length_sectors) * 588)),
                )
            } else {
                (None, None)
            };
            Ok(CdTrackLayout {
                position: layout.number,
                kind: match layout.kind {
                    TrackKind::Audio => CdTrackKind::Audio,
                    TrackKind::Data | TrackKind::Unknown => CdTrackKind::Data,
                },
                index_00,
                index_01,
                pcm_start,
                pcm_length,
            })
        })
        .collect::<ModuleResult<Vec<_>>>()?;
    let source_spans = spans
        .into_iter()
        .map(|span| {
            let file = resolve(&span.filename).map_err(module_disc_error)?;
            let track = declared_tracks
                .iter()
                .find(|track| track.number == span.track_number)
                .expect("span came from a declared track");
            let index_00 = track
                .indexes
                .iter()
                .find(|index| index.number == 0)
                .map(|index| index.to_sector_offset());
            let index_01 = track
                .indexes
                .iter()
                .find(|index| index.number == 1)
                .expect("strict validation requires INDEX 01")
                .to_sector_offset();
            Ok(CdSourceSpan {
                position: span.track_number,
                source_file_id: file.id,
                source_byte_offset: span.byte_offset,
                source_bytes: ByteLength(span.byte_len),
                stored_index_00_frames: index_00
                    .map(|index_00| u32::try_from(index_01.saturating_sub(index_00)))
                    .transpose()
                    .map_err(|_| ModuleError::FailedOperation("stored pregap overflow".into()))?
                    .unwrap_or(0),
                synthetic_pregap_frames: span.synthetic_pregap_frames,
                synthetic_postgap_frames: span.synthetic_postgap_frames,
            })
        })
        .collect::<ModuleResult<Vec<_>>>()?;
    let disc_ids = (all_audio && !has_synthetic_gaps)
        .then(|| calculate_cd_ids(&tracks))
        .transpose()?;
    let mut limitations = Vec::new();
    if !all_audio {
        limitations.push(
            "mixed-mode CUE layout is structural only; whole-disc audio is unsupported".into(),
        );
    }
    if has_synthetic_gaps {
        limitations.push(
            "synthetic CUE gaps have no physical PCM and cannot support exact verification".into(),
        );
    }
    Ok(PreparedCue {
        resolved,
        result: CdProcessingResult::Layout {
            tracks,
            source_spans,
            disc_ids,
            limitations,
        },
        first_index_01_absolute_sector,
        leadout_absolute_sector,
    })
}

pub fn analyze_cue_package(
    cue_text: &str,
    cue_file_id: PackageFileId,
    inventory: &PackageInventory,
) -> ModuleResult<CdProcessingResult> {
    Ok(prepare_cue(cue_text, cue_file_id, inventory)?.result)
}

pub fn calculate_cd_ids(tracks: &[CdTrackLayout]) -> ModuleResult<CdDiscIds> {
    if tracks.is_empty()
        || tracks.iter().any(|track| {
            track.kind != CdTrackKind::Audio
                || track.index_01.0 < 0
                || track
                    .pcm_length
                    .is_none_or(|length| !length.0.is_multiple_of(588))
        })
    {
        return Err(ModuleError::UnavailableCapability(
            "disc IDs require a complete audio-only sector layout".into(),
        ));
    }
    let offsets = tracks
        .iter()
        .map(|track| {
            u32::try_from(track.index_01.0)
                .ok()
                .and_then(|offset| offset.checked_add(150))
                .ok_or_else(|| ModuleError::FailedOperation("CD TOC offset overflow".into()))
        })
        .collect::<ModuleResult<Vec<_>>>()?;
    if offsets.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(ModuleError::FailedOperation(
            "CD TOC offsets are not strictly increasing".into(),
        ));
    }
    let last = tracks.last().unwrap();
    let leadout = u64::try_from(last.index_01.0)
        .ok()
        .and_then(|start| start.checked_add(last.pcm_length.unwrap().0 / 588))
        .and_then(|sector| sector.checked_add(150))
        .and_then(|sector| u32::try_from(sector).ok())
        .ok_or_else(|| ModuleError::FailedOperation("CD lead-out overflow".into()))?;
    let toc = junk_libs_disc_id::Toc {
        first_track: tracks[0].position,
        last_track: last.position,
        leadout_sector: leadout,
        track_offsets: offsets,
    };
    let ids = junk_libs_disc_id::calculate_disc_ids(&toc)
        .map_err(|error| ModuleError::FailedOperation(error.to_string()))?;
    Ok(CdDiscIds {
        musicbrainz: ids.musicbrainz,
        freedb: ids.freedb,
        accuraterip_id1: ids.accuraterip_id1,
        accuraterip_id2: ids.accuraterip_id2,
    })
}

pub fn open_cue_pcm_package(
    cue_text: &str,
    cue_file_id: PackageFileId,
    inventory: &PackageInventory,
    mut open_file: impl FnMut(PackageFileId) -> ModuleResult<Box<dyn crate::RepresentationReader>>,
) -> ModuleResult<(PackageCdPcmReader, Vec<DiscPcmTrack>)> {
    let prepared = prepare_cue(cue_text, cue_file_id, inventory)?;
    let CdProcessingResult::Layout {
        tracks,
        limitations,
        ..
    } = &prepared.result
    else {
        unreachable!()
    };
    if !limitations.is_empty() {
        return Err(ModuleError::UnavailableCapability(limitations.join("; ")));
    }
    let mut sources = Vec::with_capacity(prepared.resolved.files.len());
    for file in &prepared.resolved.files {
        let inventory_file = inventory
            .resolve_relative(cue_file_id, &file.filename)
            .map_err(|error| ModuleError::FailedOperation(error.to_string()))?;
        let mut reader = open_file(inventory_file.id)?;
        let actual = reader
            .seek(SeekFrom::End(0))
            .map_err(|error| ModuleError::FailedOperation(error.to_string()))?;
        if actual != inventory_file.bytes.0 {
            return Err(ModuleError::FailedOperation(format!(
                "package file {} length changed while opening CD PCM",
                inventory_file.id
            )));
        }
        reader
            .seek(SeekFrom::Start(0))
            .map_err(|error| ModuleError::FailedOperation(error.to_string()))?;
        sources.push(junk_libs_disc::pcm::PcmByteSource {
            absolute_start_frame: u64::from(file.file_start_disc_sector) * 588,
            frames: u64::from(file.file_sectors) * 588,
            reader: Box::new(DiscReaderAdapter(reader)),
        });
    }
    let absolute_start_frame = prepared.first_index_01_absolute_sector * 588;
    let absolute_end_frame = prepared.leadout_absolute_sector * 588;
    let frames = absolute_end_frame
        .checked_sub(absolute_start_frame)
        .ok_or_else(|| ModuleError::FailedOperation("CD PCM range underflow".into()))?;
    let windows = tracks
        .iter()
        .map(|track| {
            Ok(DiscPcmTrack {
                position: track.position,
                start_frame: track
                    .pcm_start
                    .ok_or_else(|| {
                        ModuleError::UnavailableCapability("track has no exact PCM range".into())
                    })?
                    .0,
                frame_count: track
                    .pcm_length
                    .ok_or_else(|| {
                        ModuleError::UnavailableCapability("track has no exact PCM range".into())
                    })?
                    .0,
            })
        })
        .collect::<ModuleResult<Vec<_>>>()?;
    Ok((
        PackageCdPcmReader::new(sources, absolute_start_frame, frames)
            .map_err(module_disc_error)?,
        windows,
    ))
}

const REDUMPER_ORIGIN_FRAMES: i64 = 45_150 * 588;

fn redumper_member<'a>(
    toc_file_id: PackageFileId,
    inventory: &'a PackageInventory,
    extension: &str,
) -> ModuleResult<Option<&'a collector_core::PackageInventoryFile>> {
    let toc = inventory
        .files
        .iter()
        .find(|file| file.id == toc_file_id)
        .ok_or(ModuleError::MissingPackageFile(toc_file_id))?;
    let name = toc.display_name.as_str();
    let stem = name
        .strip_suffix(".toc")
        .or_else(|| name.strip_suffix(".TOC"))
        .ok_or_else(|| ModuleError::Invalid("redumper anchor must be a .toc sidecar".into()))?;
    let wanted = format!("{stem}.{extension}");
    Ok(inventory
        .files
        .iter()
        .find(|file| file.display_name.as_str().eq_ignore_ascii_case(&wanted)))
}

fn required_redumper_member<'a>(
    toc_file_id: PackageFileId,
    inventory: &'a PackageInventory,
    extension: &str,
) -> ModuleResult<&'a collector_core::PackageInventoryFile> {
    redumper_member(toc_file_id, inventory, extension)?.ok_or_else(|| {
        ModuleError::FailedOperation(format!(
            "current redumper CD package is missing .{extension}"
        ))
    })
}

fn read_bounded_sidecar(
    file: &collector_core::PackageInventoryFile,
    mut open_file: impl FnMut(PackageFileId) -> ModuleResult<Box<dyn crate::RepresentationReader>>,
) -> ModuleResult<Vec<u8>> {
    const MAX_SCSI_RESPONSE: u64 = u16::MAX as u64 + 2;
    if file.bytes.0 > MAX_SCSI_RESPONSE {
        return Err(ModuleError::FailedOperation(format!(
            "sidecar {} exceeds the bounded SCSI response size",
            file.id
        )));
    }
    let mut reader = open_file(file.id)?;
    let actual = reader
        .seek(SeekFrom::End(0))
        .map_err(|error| ModuleError::FailedOperation(error.to_string()))?;
    if actual != file.bytes.0 {
        return Err(ModuleError::FailedOperation(format!(
            "sidecar {} length changed while reading",
            file.id
        )));
    }
    reader
        .seek(SeekFrom::Start(0))
        .map_err(|error| ModuleError::FailedOperation(error.to_string()))?;
    let mut bytes = vec![0_u8; file.bytes.0 as usize];
    reader
        .read_exact(&mut bytes)
        .map_err(|error| ModuleError::FailedOperation(error.to_string()))?;
    Ok(bytes)
}

fn prepare_redumper(
    toc_file_id: PackageFileId,
    inventory: &PackageInventory,
    mut open_file: impl FnMut(PackageFileId) -> ModuleResult<Box<dyn crate::RepresentationReader>>,
) -> ModuleResult<(
    CdProcessingResult,
    Vec<junk_libs_disc::redumper::pcm::RawAudioTrack>,
)> {
    for extension in ["scrap", "sdram", "sbram"] {
        if redumper_member(toc_file_id, inventory, extension)?.is_some() {
            return Err(ModuleError::UnavailableCapability(format!(
                "redumper .{extension} packages are not supported as CD audio"
            )));
        }
    }
    let scram = required_redumper_member(toc_file_id, inventory, "scram")?;
    let state = required_redumper_member(toc_file_id, inventory, "state")?;
    let subcode = required_redumper_member(toc_file_id, inventory, "subcode")?;
    let toc = required_redumper_member(toc_file_id, inventory, "toc")?;
    let toc_bytes = read_bounded_sidecar(toc, &mut open_file)?;
    let structure = junk_libs_disc::redumper::validate_current_cd_raw_parts(
        scram.bytes.0,
        state.bytes.0,
        subcode.bytes.0,
        &toc_bytes,
    )
    .map_err(module_disc_error)?;
    if let Some(fulltoc) = redumper_member(toc_file_id, inventory, "fulltoc")? {
        let bytes = read_bounded_sidecar(fulltoc, &mut open_file)?;
        junk_libs_disc::redumper::validate_full_toc_bytes(&bytes).map_err(module_disc_error)?;
    }
    let raw_tracks =
        junk_libs_disc::redumper::pcm::parse_audio_toc(&toc_bytes).map_err(module_disc_error)?;
    let absolute_end = raw_tracks
        .last()
        .and_then(|track| track.end_frame.checked_add(REDUMPER_ORIGIN_FRAMES))
        .and_then(|frame| u64::try_from(frame).ok())
        .ok_or_else(|| ModuleError::FailedOperation("redumper lead-out overflow".into()))?;
    if absolute_end > structure.sample_frames || absolute_end / 588 > structure.subcode_frames {
        return Err(ModuleError::FailedOperation(
            "redumper lead-out exceeds the raw timeline".into(),
        ));
    }
    let first_start = raw_tracks[0].start_frame;
    let tracks = raw_tracks
        .iter()
        .scan(0_u64, |pcm_start, track| {
            let frame_count = u64::try_from(track.end_frame - track.start_frame).ok()?;
            let result = CdTrackLayout {
                position: track.number,
                kind: CdTrackKind::Audio,
                index_00: None,
                index_01: CdSector(track.start_frame / 588),
                pcm_start: Some(PcmFrame(*pcm_start)),
                pcm_length: Some(PcmFrame(frame_count)),
            };
            *pcm_start += frame_count;
            Some(result)
        })
        .collect::<Vec<_>>();
    if tracks.len() != raw_tracks.len() {
        return Err(ModuleError::FailedOperation(
            "redumper track frame range overflow".into(),
        ));
    }
    let source_spans = raw_tracks
        .iter()
        .map(|track| {
            let absolute_start = track
                .start_frame
                .checked_add(REDUMPER_ORIGIN_FRAMES)
                .and_then(|frame| u64::try_from(frame).ok())
                .ok_or_else(|| ModuleError::FailedOperation("redumper span overflow".into()))?;
            Ok(CdSourceSpan {
                position: track.number,
                source_file_id: scram.id,
                source_byte_offset: absolute_start.checked_mul(4).ok_or_else(|| {
                    ModuleError::FailedOperation("redumper byte span overflow".into())
                })?,
                source_bytes: ByteLength(
                    u64::try_from(track.end_frame - track.start_frame)
                        .ok()
                        .and_then(|frames| frames.checked_mul(4))
                        .ok_or_else(|| {
                            ModuleError::FailedOperation("redumper byte span overflow".into())
                        })?,
                ),
                stored_index_00_frames: 0,
                synthetic_pregap_frames: 0,
                synthetic_postgap_frames: 0,
            })
        })
        .collect::<ModuleResult<Vec<_>>>()?;
    let mut limitations = Vec::new();
    let disc_ids = match calculate_cd_ids(&tracks) {
        Ok(ids) => Some(ids),
        Err(_) if first_start < 0 => {
            limitations.push(
                "negative first-track lead-in is retained; disc IDs are unavailable for this layout"
                    .into(),
            );
            None
        }
        Err(error) => return Err(error),
    };
    Ok((
        CdProcessingResult::Layout {
            tracks,
            source_spans,
            disc_ids,
            limitations,
        },
        raw_tracks,
    ))
}

pub fn analyze_redumper_package(
    toc_file_id: PackageFileId,
    inventory: &PackageInventory,
    open_file: impl FnMut(PackageFileId) -> ModuleResult<Box<dyn crate::RepresentationReader>>,
) -> ModuleResult<CdProcessingResult> {
    Ok(prepare_redumper(toc_file_id, inventory, open_file)?.0)
}

pub fn open_redumper_pcm_package(
    toc_file_id: PackageFileId,
    inventory: &PackageInventory,
    mut open_file: impl FnMut(PackageFileId) -> ModuleResult<Box<dyn crate::RepresentationReader>>,
) -> ModuleResult<(
    junk_libs_disc::redumper::pcm::RawAudioReader,
    Vec<DiscPcmTrack>,
)> {
    let (_, tracks) = prepare_redumper(toc_file_id, inventory, &mut open_file)?;
    let scram = required_redumper_member(toc_file_id, inventory, "scram")?;
    let state = required_redumper_member(toc_file_id, inventory, "state")?;
    let mut pcm = open_file(scram.id)?;
    let mut quality = open_file(state.id)?;
    let pcm_len = pcm
        .seek(SeekFrom::End(0))
        .map_err(|error| ModuleError::FailedOperation(error.to_string()))?;
    let state_len = quality
        .seek(SeekFrom::End(0))
        .map_err(|error| ModuleError::FailedOperation(error.to_string()))?;
    if pcm_len != scram.bytes.0 || state_len != state.bytes.0 {
        return Err(ModuleError::FailedOperation(
            "redumper PCM sidecar length changed while opening".into(),
        ));
    }
    let reader = junk_libs_disc::redumper::pcm::RawAudioReader::from_signed_sources(
        Box::new(DiscReaderAdapter(pcm)),
        Box::new(DiscReaderAdapter(quality)),
        tracks[0].start_frame,
        tracks.last().unwrap().end_frame,
        junk_libs_disc::redumper::pcm::InvalidSamplePolicy::Reject,
    )
    .map_err(module_disc_error)?;
    let windows = tracks
        .iter()
        .scan(0_u64, |start, track| {
            let frame_count = u64::try_from(track.end_frame - track.start_frame).ok()?;
            let window = DiscPcmTrack {
                position: track.number,
                start_frame: *start,
                frame_count,
            };
            *start += frame_count;
            Some(window)
        })
        .collect::<Vec<_>>();
    if windows.len() != tracks.len() {
        return Err(ModuleError::FailedOperation(
            "redumper track frame range overflow".into(),
        ));
    }
    Ok((reader, windows))
}

pub struct Music;

impl CollectionModule for Music {
    fn descriptor(&self) -> ModuleDescriptor {
        descriptor(
            "org.junk.music",
            "Music CDs",
            vec![
                text_field("artist", "Artist"),
                text_field("label", "Label"),
                DomainFieldDescriptor {
                    key: "release_year".into(),
                    label: "Release year".into(),
                    required: false,
                    editable: true,
                    kind: DomainFieldKind::Integer {
                        minimum: 0,
                        maximum: 9999,
                    },
                },
                DomainFieldDescriptor {
                    key: "tracks".into(),
                    label: "Tracks".into(),
                    required: false,
                    editable: false,
                    kind: DomainFieldKind::Structured,
                },
            ],
        )
    }

    fn validate_current(&self, data: &ModuleData) -> ModuleResult<()> {
        let music: MusicItemData = data
            .payload
            .parse()
            .map_err(|error| ModuleError::Invalid(error.to_string()))?;
        for track in music.tracks {
            track
                .start
                .0
                .checked_add(track.length.0)
                .ok_or_else(|| ModuleError::Invalid("track span overflow".into()))?;
        }
        validate_texts([&music.artist, &music.label])?;
        validate_year(music.release_year)
    }

    fn apply_edit(&self, current: &ModuleData, edit: &RawJson) -> ModuleResult<ModuleData> {
        let patch: MusicItemPatch = edit
            .parse()
            .map_err(|error| ModuleError::Invalid(error.to_string()))?;
        let mut object = raw_domain_object(current)?;
        apply_patch_value(&mut object, "artist", patch.artist)?;
        apply_patch_value(&mut object, "label", patch.label)?;
        apply_patch_value(&mut object, "release_year", patch.release_year)?;
        module_data_from_raw_object(current, object)
    }

    fn validate_metadata_overrides(&self, overrides: &ModuleData) -> ModuleResult<()> {
        let value: MusicMetadataOverrides = overrides
            .payload
            .parse()
            .map_err(|error| ModuleError::Invalid(error.to_string()))?;
        validate_texts([&value.artist])
    }

    fn validate_operation_result(
        &self,
        _capability: &CapabilityId,
        result: &RawJson,
    ) -> ModuleResult<()> {
        let result: CdProcessingResult = result
            .parse()
            .map_err(|error| ModuleError::Invalid(error.to_string()))?;
        match result {
            CdProcessingResult::AudioVerification { verification } => verification
                .validate()
                .map_err(|error| ModuleError::Invalid(error.to_string()))?,
            CdProcessingResult::Layout {
                tracks,
                source_spans,
                disc_ids,
                limitations,
            } => {
                validate_cd_layout(&tracks)?;
                validate_cd_source_spans(&source_spans)?;
                if let Some(ids) = disc_ids
                    && calculate_cd_ids(&tracks)? != ids
                {
                    return Err(ModuleError::Invalid(
                        "CD layout identifiers do not match its coordinates".into(),
                    ));
                }
                if limitations.len() > 64
                    || limitations.iter().any(|value| {
                        value.trim().is_empty()
                            || value.len() > 4096
                            || value.chars().any(char::is_control)
                    })
                {
                    return Err(ModuleError::Invalid("invalid CD layout limitations".into()));
                }
            }
        }
        Ok(())
    }

    fn present(&self, data: &ModuleData) -> ModuleResult<ModulePresentation> {
        self.validate_current(data)?;
        Ok(basic_presentation(data, self.descriptor()))
    }

    fn operate(
        &self,
        _request: OperationRequest,
        _host: &dyn crate::ModuleHost,
    ) -> ModuleResult<OperationOutput> {
        Err(ModuleError::UnavailableCapability(
            "music operation handler is not implemented".into(),
        ))
    }
}

fn text_field(key: &str, label: &str) -> DomainFieldDescriptor {
    DomainFieldDescriptor {
        key: key.into(),
        label: label.into(),
        required: false,
        editable: true,
        kind: DomainFieldKind::Text {
            maximum_bytes: 4096,
        },
    }
}

fn validate_texts<const N: usize>(values: [&Option<String>; N]) -> ModuleResult<()> {
    if values
        .into_iter()
        .flatten()
        .any(|value| value.len() > 4096 || value.chars().any(|character| character.is_control()))
    {
        return Err(ModuleError::Invalid(
            "music text is invalid or oversized".into(),
        ));
    }
    Ok(())
}

fn validate_year(value: Option<u16>) -> ModuleResult<()> {
    if value.is_some_and(|year| year > 9999) {
        return Err(ModuleError::Invalid("release year is out of bounds".into()));
    }
    Ok(())
}

fn validate_cd_layout(tracks: &[CdTrackLayout]) -> ModuleResult<()> {
    let mut previous = 0;
    if tracks.is_empty() || tracks.len() > 99 {
        return Err(ModuleError::Invalid("invalid CD track count".into()));
    }
    for track in tracks {
        if track.position <= previous
            || track.position > 99
            || track
                .index_00
                .is_some_and(|index| index.0 > track.index_01.0)
            || track.pcm_start.is_some() != track.pcm_length.is_some()
            || track
                .pcm_start
                .zip(track.pcm_length)
                .is_some_and(|(start, length)| start.0.checked_add(length.0).is_none())
        {
            return Err(ModuleError::Invalid("invalid CD track layout".into()));
        }
        previous = track.position;
    }
    Ok(())
}

fn validate_cd_source_spans(spans: &[CdSourceSpan]) -> ModuleResult<()> {
    if spans.is_empty() || spans.len() > 4096 {
        return Err(ModuleError::Invalid("invalid CD source span count".into()));
    }
    let mut previous = None;
    for span in spans {
        if span.position == 0
            || span.position > 99
            || span.source_bytes.0 == 0
            || span
                .source_byte_offset
                .checked_add(span.source_bytes.0)
                .is_none()
            || previous.is_some_and(|position| span.position <= position)
        {
            return Err(ModuleError::Invalid("invalid CD source span".into()));
        }
        previous = Some(span.position);
    }
    Ok(())
}
