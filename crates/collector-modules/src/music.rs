pub mod catalog;
use catalog::{DbarOutcome, DbarSnapshotObservation, ParsedDbarSnapshot};
use std::io::{Read, SeekFrom};

use collector_core::{
    ByteLength, CdSector, CdTrackKind, CdTrackLayout, Digest, HashAlgorithm, ModuleData,
    PackageFileId, PackageInventory, PcmFrame, RawJson, SampleFrame, StereoSampleFrameOffset,
};
use junk_libs_accuraterip::{
    BoundedDiscReader, DbarFile, DbarResponse, DiscPcmTrack, ExpectedChecksum,
    TrackVerificationStatus, VerificationOptions, VerificationStatus, VerificationSummary,
    verify_with_offset_reader,
};
use junk_libs_disc::TrackKind;
use serde::{Deserialize, Serialize};

use crate::{
    Availability, Capability, CapabilityId, CollectionModule, DomainFieldDescriptor,
    DomainFieldKind, ModuleDescriptor, ModuleError, ModulePresentation, ModuleResult,
    OperationEntity, OperationOutput, OperationRequest, OperationScope, PackageAccess, PatchValue,
    apply_patch_value, basic_presentation, descriptor, module_data_from_raw_object,
    raw_domain_object,
};

pub const CD_STRUCTURE_OPERATION: &str = "org.junk.music.cd-structure";
pub const ACCURATERIP_VERIFICATION_OPERATION: &str = "org.junk.music.accuraterip-verification";
const ACCURATERIP_PROVIDER_ID: &str = "org.junk.providers.accuraterip";
const MAX_CUE_BYTES: u64 = 1024 * 1024;

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
pub enum CdPackageKind {
    Cue,
    Redumper,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CdPackageSelection {
    pub package_kind: CdPackageKind,
    pub anchor_file_index: usize,
}

/// Music-owned mapping from one supported package timeline to independently
/// addressable derivative tracks. Coordinates are stereo PCM frames in the
/// returned reader, not CD sectors or AccurateRip sample offsets.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConversionPcmTrack {
    pub position: u16,
    pub start_frame: u64,
    pub frame_count: u64,
    /// Stored audio before this track's INDEX 01 which this mapping retains.
    /// D05's later INDEX 00 spans belong to the preceding track; only track 1
    /// can therefore have a nonzero leading count here (HTOA).
    pub leading_hidden_frames: u64,
}

pub struct ConversionPcmSource {
    pub reader: Box<dyn junk_libs_disc::CdPcmReader + Send>,
    pub tracks: Vec<ConversionPcmTrack>,
    pub total_frames: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AccurateRipSelection {
    pub package_kind: CdPackageKind,
    pub anchor_file_index: usize,
    pub snapshot_id: collector_core::SnapshotId,
    #[serde(default = "default_maximum_offset")]
    pub maximum_offset: i32,
}

fn default_maximum_offset() -> i32 {
    junk_libs_accuraterip::DEFAULT_MAX_SAMPLE_SHIFT
}

#[derive(Deserialize)]
struct MusicImportAnalysisInput {
    #[serde(rename = "import_id")]
    _import_id: collector_core::ImportId,
    #[serde(rename = "representation_id")]
    _representation_id: collector_core::RepresentationId,
    #[serde(rename = "files")]
    _files: Vec<serde_json::Value>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct MusicImportAnalysisOutput {
    candidates: Vec<serde_json::Value>,
    limitations: Vec<String>,
    observations: Vec<RawJson>,
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
pub struct AudioChecksumEvidence {
    pub version: AudioChecksumVersion,
    pub checksum: Digest,
    pub confidence: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AudioSearchCoverage {
    /// Inclusive v1 search range; v2 candidate evaluation is listed separately.
    pub v1_evaluated_offset_start: StereoSampleFrameOffset,
    pub v1_evaluated_offset_end: StereoSampleFrameOffset,
    pub v2_evaluated_offsets: Vec<StereoSampleFrameOffset>,
    pub unavailable_leading_frames: PcmFrame,
    pub unavailable_trailing_frames: PcmFrame,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AudioTrackVerification {
    pub position: u8,
    pub status: AudioVerificationStatus,
    /// Offset at which both recorded checksums were computed, even without a disc-wide selection.
    pub computed_offset: StereoSampleFrameOffset,
    pub checksums: Vec<AudioChecksumEvidence>,
    pub coverage: AudioSearchCoverage,
    pub limitations: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AccurateRipDatabaseIdentity {
    pub track_count: u8,
    pub accuraterip_id1: u32,
    pub accuraterip_id2: u32,
    pub cddb_id: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AudioVerificationResult {
    pub status: AudioVerificationStatus,
    pub selected_offset: Option<StereoSampleFrameOffset>,
    pub ambiguous_offsets: Vec<StereoSampleFrameOffset>,
    pub algorithm_version: u32,
    pub database: AccurateRipDatabaseIdentity,
    pub database_response_count: u32,
    pub database_response_digest: Digest,
    pub input_digests: Vec<Digest>,
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
        self.database_response_digest.validate()?;
        if self.algorithm_version != 1
            || !(1..=99).contains(&self.database.track_count)
            || self.database_response_digest.algorithm != HashAlgorithm::Sha256
            || (self.status == AudioVerificationStatus::Verified
                && self.database_response_count == 0)
            || self.input_digests.is_empty()
            || self.input_digests.len() > 4096
            || self.input_digests.iter().any(|digest| {
                digest.validate().is_err() || digest.algorithm != HashAlgorithm::Sha256
            })
            || (matches!(
                self.status,
                AudioVerificationStatus::Verified
                    | AudioVerificationStatus::IncompleteSearch
                    | AudioVerificationStatus::Mismatched
                    | AudioVerificationStatus::AmbiguousOffsets
            ) && (self.database_response_count == 0
                || self.tracks.len() != usize::from(self.database.track_count)))
            || (self.status == AudioVerificationStatus::NoData
                && (self.database_response_count != 0 || !self.tracks.is_empty()))
            || self.ambiguous_offsets.len() > 5879
            || self
                .ambiguous_offsets
                .windows(2)
                .any(|pair| pair[0].0 >= pair[1].0)
        {
            return Err(collector_core::Error::Invalid(
                "invalid audio verification bounds".into(),
            ));
        }
        let mut previous = 0;
        for (index, track) in self.tracks.iter().enumerate() {
            if track.position <= previous
                || track.position > 99
                || usize::from(track.position) != index + 1
            {
                return Err(collector_core::Error::Invalid(
                    "audio tracks must have unique ordered positions".into(),
                ));
            }
            previous = track.position;
            validate_search_coverage(&track.coverage)?;
            if !track
                .coverage
                .v2_evaluated_offsets
                .contains(&track.computed_offset)
                || (track.status == AudioVerificationStatus::Mismatched
                    && !complete_audio_search(&track.coverage))
                || (self.status == AudioVerificationStatus::Mismatched
                    && !complete_audio_search(&track.coverage))
                || (self.status == AudioVerificationStatus::Verified
                    && self.selected_offset != Some(track.computed_offset))
            {
                return Err(collector_core::Error::Invalid(
                    "checksum offset or outcome disagrees with search coverage".into(),
                ));
            }
            if track.checksums.len() != 2
                || track.checksums[0].version != AudioChecksumVersion::AccurateRipV1
                || track.checksums[1].version != AudioChecksumVersion::AccurateRipV2
            {
                return Err(collector_core::Error::Invalid(
                    "audio tracks require ordered v1 and v2 checksums".into(),
                ));
            }
            for checksum in &track.checksums {
                checksum.checksum.validate()?;
                let expected = match checksum.version {
                    AudioChecksumVersion::AccurateRipV1 => HashAlgorithm::AccurateRipV1,
                    AudioChecksumVersion::AccurateRipV2 => HashAlgorithm::AccurateRipV2,
                };
                if checksum.checksum.algorithm != expected
                    || checksum
                        .confidence
                        .is_some_and(|confidence| confidence == 0 || confidence > 255)
                {
                    return Err(collector_core::Error::Invalid(
                        "invalid audio checksum evidence".into(),
                    ));
                }
            }
            let has_match = track
                .checksums
                .iter()
                .any(|checksum| checksum.confidence.is_some());
            if (track.status == AudioVerificationStatus::Verified && !has_match)
                || (!matches!(
                    track.status,
                    AudioVerificationStatus::Verified | AudioVerificationStatus::AmbiguousOffsets
                ) && has_match)
            {
                return Err(collector_core::Error::Invalid(
                    "track status does not match its full-checksum evidence".into(),
                ));
            }
            validate_limitations(&track.limitations)?;
        }
        if self.status == AudioVerificationStatus::Verified
            && (self.tracks.len() != usize::from(self.database.track_count)
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
        if self.selected_offset.is_some_and(|offset| {
            self.tracks.iter().any(|track| {
                offset.0 < track.coverage.v1_evaluated_offset_start.0
                    || offset.0 > track.coverage.v1_evaluated_offset_end.0
            })
        }) || self.ambiguous_offsets.iter().any(|offset| {
            self.tracks.iter().any(|track| {
                offset.0 < track.coverage.v1_evaluated_offset_start.0
                    || offset.0 > track.coverage.v1_evaluated_offset_end.0
            })
        }) {
            return Err(collector_core::Error::Invalid(
                "audio offset evidence falls outside recorded coverage".into(),
            ));
        }
        if self.status != AudioVerificationStatus::Verified && self.selected_offset.is_some()
            || (self.status == AudioVerificationStatus::AmbiguousOffsets)
                == self.ambiguous_offsets.is_empty()
        {
            return Err(collector_core::Error::Invalid(
                "audio offset evidence is inconsistent with its status".into(),
            ));
        }
        validate_limitations(&self.limitations)?;
        Ok(())
    }
}

fn validate_search_coverage(coverage: &AudioSearchCoverage) -> collector_core::Result<()> {
    if coverage.v1_evaluated_offset_start.0 > coverage.v1_evaluated_offset_end.0
        || coverage.v2_evaluated_offsets.len() > 5879
        || coverage
            .v2_evaluated_offsets
            .windows(2)
            .any(|pair| pair[0].0 >= pair[1].0)
        || coverage.v2_evaluated_offsets.iter().any(|offset| {
            offset.0 < coverage.v1_evaluated_offset_start.0
                || offset.0 > coverage.v1_evaluated_offset_end.0
        })
    {
        return Err(collector_core::Error::Invalid(
            "invalid per-version audio search coverage".into(),
        ));
    }
    Ok(())
}

fn complete_v2_search(coverage: &AudioSearchCoverage) -> bool {
    i64::from(coverage.v1_evaluated_offset_end.0) - i64::from(coverage.v1_evaluated_offset_start.0)
        + 1
        == coverage.v2_evaluated_offsets.len() as i64
}

fn complete_audio_search(coverage: &AudioSearchCoverage) -> bool {
    complete_v2_search(coverage)
        && coverage.unavailable_leading_frames.0 == 0
        && coverage.unavailable_trailing_frames.0 == 0
}

fn validate_limitations(limitations: &[String]) -> collector_core::Result<()> {
    if limitations.len() > 128
        || limitations
            .iter()
            .any(|value| value.len() > 4096 || value.chars().any(char::is_control))
    {
        return Err(collector_core::Error::Invalid(
            "audio limitations are oversized or invalid".into(),
        ));
    }
    Ok(())
}

pub const ACCURATERIP_INTERPRETATION_VERSION: u32 = 1;

pub struct InterpretAccurateRipInput {
    pub database: AccurateRipDatabaseIdentity,
    pub database_response_digest: Digest,
    pub input_digests: Vec<Digest>,
}

/// Convert the shared checksum search into the music module's evidence policy.
/// The shared `Verified` state means at least one track; this boundary alone may
/// promote it to whole-disc verification after checking every expected track.
pub fn interpret_accuraterip_summary(
    dbar: &DbarFile,
    summary: &VerificationSummary,
    input: InterpretAccurateRipInput,
) -> ModuleResult<AudioVerificationResult> {
    dbar.validate_request(
        input.database.track_count,
        input.database.accuraterip_id1,
        input.database.accuraterip_id2,
        input.database.cddb_id,
    )
    .map_err(|error| ModuleError::FailedOperation(error.to_string()))?;
    let coverage = AudioSearchCoverage {
        v1_evaluated_offset_start: StereoSampleFrameOffset(summary.evaluated_sample_shift_start),
        v1_evaluated_offset_end: StereoSampleFrameOffset(summary.evaluated_sample_shift_end),
        v2_evaluated_offsets: summary
            .v2_evaluated_sample_shifts
            .iter()
            .copied()
            .map(StereoSampleFrameOffset)
            .collect(),
        unavailable_leading_frames: PcmFrame(u64::from(summary.unavailable_leading_frames)),
        unavailable_trailing_frames: PcmFrame(u64::from(summary.unavailable_trailing_frames)),
    };
    validate_search_coverage(&coverage).map_err(|error| ModuleError::Invalid(error.to_string()))?;
    let complete_search = complete_audio_search(&coverage);
    let expected_tracks = usize::from(input.database.track_count);
    if !dbar.responses.is_empty()
        && (summary.tracks.len() != expected_tracks
            || summary
                .tracks
                .iter()
                .enumerate()
                .any(|(index, track)| usize::from(track.position) != index + 1))
    {
        return Err(ModuleError::Invalid(
            "AccurateRip result does not cover every expected database track".into(),
        ));
    }
    if summary.status == VerificationStatus::Verified && !summary.ambiguous_offsets.is_empty() {
        return Err(ModuleError::Invalid(
            "verified summary contains ambiguous offsets".into(),
        ));
    }
    for track in &summary.tracks {
        // The shared verifier retains zero-offset computed values when it clears
        // a selection for mismatch/ambiguity. Preserve that computation origin.
        let offset = track.sample_shift.unwrap_or(0);
        if !summary.v2_evaluated_sample_shifts.contains(&offset)
            || (summary.status == VerificationStatus::Verified
                && track.sample_shift != summary.chosen_sample_shift)
        {
            return Err(ModuleError::Invalid(
                "track computation offset disagrees with search evidence".into(),
            ));
        }
        for (matches, computed, version) in [
            (
                &track.v1_matches,
                track.computed.v1,
                junk_libs_accuraterip::ChecksumVersion::V1,
            ),
            (
                &track.v2_matches,
                track.computed.v2,
                junk_libs_accuraterip::ChecksumVersion::V2,
            ),
        ] {
            for matched in matches {
                let expected = dbar
                    .responses
                    .get(matched.pressing)
                    .and_then(|response| {
                        track
                            .position
                            .checked_sub(1)
                            .and_then(|position| response.tracks.get(usize::from(position)))
                    })
                    .ok_or_else(|| {
                        ModuleError::Invalid(
                            "checksum match references a missing database entry".into(),
                        )
                    })?;
                if matched.checksum != computed
                    || expected.checksum != computed
                    || matched.confidence != expected.confidence
                    || !(matched.version == version
                        || matched.version == junk_libs_accuraterip::ChecksumVersion::Both)
                {
                    return Err(ModuleError::Invalid(
                        "checksum match disagrees with the frozen database entry".into(),
                    ));
                }
            }
        }
    }
    let all_tracks_verified = summary.tracks.len() == expected_tracks
        && summary
            .tracks
            .iter()
            .all(|track| track.status == TrackVerificationStatus::Verified);
    let status = match summary.status {
        VerificationStatus::Verified
            if all_tracks_verified && summary.chosen_sample_shift.is_some() =>
        {
            AudioVerificationStatus::Verified
        }
        VerificationStatus::Verified | VerificationStatus::Mismatched if complete_search => {
            AudioVerificationStatus::Mismatched
        }
        VerificationStatus::Verified
        | VerificationStatus::IncompleteSearch
        | VerificationStatus::Mismatched => AudioVerificationStatus::IncompleteSearch,
        VerificationStatus::NoData => AudioVerificationStatus::NoData,
        VerificationStatus::AmbiguousOffsets => AudioVerificationStatus::AmbiguousOffsets,
    };
    let tracks = summary
        .tracks
        .iter()
        .map(|track| {
            let v1_confidence = track
                .v1_matches
                .iter()
                .map(|matched| u32::from(matched.confidence))
                .max();
            let v2_confidence = track
                .v2_matches
                .iter()
                .map(|matched| u32::from(matched.confidence))
                .max();
            let track_status = match track.status {
                TrackVerificationStatus::Verified => AudioVerificationStatus::Verified,
                TrackVerificationStatus::Ambiguous => AudioVerificationStatus::AmbiguousOffsets,
                TrackVerificationStatus::NoData => AudioVerificationStatus::NoData,
                TrackVerificationStatus::Mismatched if complete_search => {
                    AudioVerificationStatus::Mismatched
                }
                TrackVerificationStatus::Mismatched => AudioVerificationStatus::IncompleteSearch,
            };
            Ok(AudioTrackVerification {
                position: track.position,
                status: track_status,
                computed_offset: StereoSampleFrameOffset(track.sample_shift.unwrap_or(0)),
                checksums: vec![
                    AudioChecksumEvidence {
                        version: AudioChecksumVersion::AccurateRipV1,
                        checksum: Digest::new(
                            HashAlgorithm::AccurateRipV1,
                            format!("{:08x}", track.computed.v1),
                        )
                        .map_err(|error| ModuleError::Invalid(error.to_string()))?,
                        confidence: v1_confidence,
                    },
                    AudioChecksumEvidence {
                        version: AudioChecksumVersion::AccurateRipV2,
                        checksum: Digest::new(
                            HashAlgorithm::AccurateRipV2,
                            format!("{:08x}", track.computed.v2),
                        )
                        .map_err(|error| ModuleError::Invalid(error.to_string()))?,
                        confidence: v2_confidence,
                    },
                ],
                coverage: coverage.clone(),
                limitations: Vec::new(),
            })
        })
        .collect::<ModuleResult<Vec<_>>>()?;
    let mut limitations = Vec::new();
    if !complete_v2_search(&coverage)
        && !matches!(
            status,
            AudioVerificationStatus::Verified | AudioVerificationStatus::NoData
        )
    {
        limitations.push(
            "AccurateRip v2 was evaluated only at the recorded candidate offsets; the result is inconclusive".into(),
        );
    }
    if summary.tracks.iter().any(|track| track.frame_450_support)
        && !summary
            .tracks
            .iter()
            .any(|track| track.status == TrackVerificationStatus::Verified)
    {
        limitations
            .push("frame-450 evidence identified offset candidates but cannot verify audio".into());
    }
    let result = AudioVerificationResult {
        status,
        selected_offset: (status == AudioVerificationStatus::Verified)
            .then_some(summary.chosen_sample_shift)
            .flatten()
            .map(StereoSampleFrameOffset),
        ambiguous_offsets: if status == AudioVerificationStatus::AmbiguousOffsets {
            summary
                .ambiguous_offsets
                .iter()
                .map(|candidate| StereoSampleFrameOffset(candidate.sample_shift))
                .collect()
        } else {
            Vec::new()
        },
        algorithm_version: ACCURATERIP_INTERPRETATION_VERSION,
        database: AccurateRipDatabaseIdentity {
            track_count: input.database.track_count,
            accuraterip_id1: input.database.accuraterip_id1,
            accuraterip_id2: input.database.accuraterip_id2,
            cddb_id: input.database.cddb_id,
        },
        database_response_count: u32::try_from(dbar.responses.len())
            .map_err(|_| ModuleError::Invalid("too many AccurateRip database responses".into()))?,
        database_response_digest: input.database_response_digest,
        input_digests: input.input_digests,
        tracks,
        limitations,
    };
    result
        .validate()
        .map_err(|error| ModuleError::Invalid(error.to_string()))?;
    Ok(result)
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
    open_file: impl FnMut(PackageFileId) -> ModuleResult<Box<dyn crate::RepresentationReader>>,
) -> ModuleResult<(PackageCdPcmReader, Vec<DiscPcmTrack>)> {
    let (reader, tracks, _) =
        open_cue_pcm_package_with_origin(cue_text, cue_file_id, inventory, open_file, false)?;
    Ok((reader, tracks))
}

fn open_cue_pcm_package_with_origin(
    cue_text: &str,
    cue_file_id: PackageFileId,
    inventory: &PackageInventory,
    mut open_file: impl FnMut(PackageFileId) -> ModuleResult<Box<dyn crate::RepresentationReader>>,
    retain_htoa: bool,
) -> ModuleResult<(PackageCdPcmReader, Vec<DiscPcmTrack>, u64)> {
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
    let stored_start_sector = prepared
        .resolved
        .files
        .first()
        .map(|file| u64::from(file.file_start_disc_sector))
        .ok_or_else(|| ModuleError::FailedOperation("CUE has no source files".into()))?;
    let htoa_frames = if retain_htoa {
        prepared
            .first_index_01_absolute_sector
            .checked_sub(stored_start_sector)
            .and_then(|sectors| sectors.checked_mul(588))
            .ok_or_else(|| ModuleError::FailedOperation("CUE HTOA range overflow".into()))?
    } else {
        0
    };
    let absolute_start_frame = prepared
        .first_index_01_absolute_sector
        .checked_mul(588)
        .and_then(|start| start.checked_sub(htoa_frames))
        .ok_or_else(|| ModuleError::FailedOperation("CUE PCM origin overflow".into()))?;
    let absolute_end_frame = prepared.leadout_absolute_sector * 588;
    let frames = absolute_end_frame
        .checked_sub(absolute_start_frame)
        .ok_or_else(|| ModuleError::FailedOperation("CD PCM range underflow".into()))?;
    let mut windows = tracks
        .iter()
        .map(|track| {
            Ok(DiscPcmTrack {
                position: track.position,
                start_frame: track
                    .pcm_start
                    .ok_or_else(|| {
                        ModuleError::UnavailableCapability("track has no exact PCM range".into())
                    })?
                    .0
                    .checked_add(htoa_frames)
                    .ok_or_else(|| ModuleError::FailedOperation("track start overflow".into()))?,
                frame_count: track
                    .pcm_length
                    .ok_or_else(|| {
                        ModuleError::UnavailableCapability("track has no exact PCM range".into())
                    })?
                    .0,
            })
        })
        .collect::<ModuleResult<Vec<_>>>()?;
    if let Some(first) = windows.first_mut() {
        first.start_frame = 0;
        first.frame_count = first
            .frame_count
            .checked_add(htoa_frames)
            .ok_or_else(|| ModuleError::FailedOperation("track HTOA span overflow".into()))?;
    }
    Ok((
        PackageCdPcmReader::new(sources, absolute_start_frame, frames)
            .map_err(module_disc_error)?,
        windows,
        htoa_frames,
    ))
}

/// Reader-based CUE conversion entry point used by the application host after it
/// has authorized and frozen the package inventory.
pub fn open_cue_conversion_pcm_package(
    cue_text: &str,
    cue_file_id: PackageFileId,
    inventory: &PackageInventory,
    open_file: impl FnMut(PackageFileId) -> ModuleResult<Box<dyn crate::RepresentationReader>>,
) -> ModuleResult<ConversionPcmSource> {
    let (reader, windows, htoa_frames) =
        open_cue_pcm_package_with_origin(cue_text, cue_file_id, inventory, open_file, true)?;
    conversion_pcm_source(Box::new(reader), windows, htoa_frames)
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

fn operation_access(request: &OperationRequest) -> ModuleResult<PackageAccess> {
    let scope = match request.entity {
        OperationEntity::Import(id) => collector_core::PackageScope::Import(id),
        OperationEntity::Representation(id) => collector_core::PackageScope::Representation(id),
        _ => {
            return Err(ModuleError::Invalid(
                "CD processing requires an Import or Representation".into(),
            ));
        }
    };
    Ok(PackageAccess {
        collection_id: request.collection_id,
        scope,
    })
}

fn anchor_file(
    inventory: &PackageInventory,
    index: usize,
) -> ModuleResult<&collector_core::PackageInventoryFile> {
    inventory
        .files
        .get(index)
        .ok_or_else(|| ModuleError::Invalid("anchor file index is out of bounds".into()))
}

fn read_cue(
    host: &dyn crate::ModuleHost,
    access: PackageAccess,
    inventory: &PackageInventory,
    index: usize,
) -> ModuleResult<(PackageFileId, String)> {
    let file = anchor_file(inventory, index)?;
    if file.bytes.0 > MAX_CUE_BYTES {
        return Err(ModuleError::FailedOperation(
            "CUE sheet exceeds its one-MiB bound".into(),
        ));
    }
    let mut reader = host.package_file(access, file.id)?;
    let mut bytes = Vec::with_capacity(file.bytes.0 as usize);
    reader
        .by_ref()
        .take(MAX_CUE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| ModuleError::FailedOperation(error.to_string()))?;
    if bytes.len() as u64 != file.bytes.0 {
        return Err(ModuleError::FailedOperation(
            "CUE sheet length changed while reading".into(),
        ));
    }
    let text = String::from_utf8(bytes)
        .map_err(|_| ModuleError::FailedOperation("CUE sheet is not UTF-8".into()))?;
    Ok((file.id, text))
}

fn analyze_selection(
    host: &dyn crate::ModuleHost,
    access: PackageAccess,
    inventory: &PackageInventory,
    selection: &CdPackageSelection,
) -> ModuleResult<CdProcessingResult> {
    match selection.package_kind {
        CdPackageKind::Cue => {
            let (id, cue) = read_cue(host, access, inventory, selection.anchor_file_index)?;
            analyze_cue_package(&cue, id, inventory)
        }
        CdPackageKind::Redumper => {
            let file = anchor_file(inventory, selection.anchor_file_index)?;
            analyze_redumper_package(file.id, inventory, |id| host.package_file(access, id))
        }
    }
}

enum PackagePcmReader {
    Cue(PackageCdPcmReader),
    Redumper(junk_libs_disc::redumper::pcm::RawAudioReader),
}

impl junk_libs_disc::CdPcmReader for PackagePcmReader {
    fn total_frames(&self) -> u64 {
        match self {
            Self::Cue(reader) => junk_libs_disc::CdPcmReader::total_frames(reader),
            Self::Redumper(reader) => junk_libs_disc::CdPcmReader::total_frames(reader),
        }
    }

    fn seek_frame(&mut self, frame: u64) -> Result<(), junk_libs_core::AnalysisError> {
        match self {
            Self::Cue(reader) => junk_libs_disc::CdPcmReader::seek_frame(reader, frame),
            Self::Redumper(reader) => junk_libs_disc::CdPcmReader::seek_frame(reader, frame),
        }
    }

    fn read_frames(
        &mut self,
        output: &mut [[i16; 2]],
    ) -> Result<usize, junk_libs_core::AnalysisError> {
        match self {
            Self::Cue(reader) => junk_libs_disc::CdPcmReader::read_frames(reader, output),
            Self::Redumper(reader) => junk_libs_disc::CdPcmReader::read_frames(reader, output),
        }
    }
}

fn open_pcm_selection(
    host: &dyn crate::ModuleHost,
    access: PackageAccess,
    inventory: &PackageInventory,
    selection: &CdPackageSelection,
) -> ModuleResult<(PackagePcmReader, Vec<DiscPcmTrack>, CdProcessingResult)> {
    let layout = analyze_selection(host, access, inventory, selection)?;
    let (reader, tracks) = match selection.package_kind {
        CdPackageKind::Cue => {
            let (id, cue) = read_cue(host, access, inventory, selection.anchor_file_index)?;
            let (reader, tracks) =
                open_cue_pcm_package(&cue, id, inventory, |file| host.package_file(access, file))?;
            (PackagePcmReader::Cue(reader), tracks)
        }
        CdPackageKind::Redumper => {
            let file = anchor_file(inventory, selection.anchor_file_index)?;
            let (reader, tracks) = open_redumper_pcm_package(file.id, inventory, |file| {
                host.package_file(access, file)
            })?;
            (PackagePcmReader::Redumper(reader), tracks)
        }
    };
    Ok((reader, tracks, layout))
}

/// Open the exact PCM and music-owned track ranges used by derivative recipes.
/// This is deliberately independent of AccurateRip acquisition/results: a
/// structurally supported package remains convertible after no-data, mismatch or
/// provider outage.
pub fn open_conversion_pcm_selection(
    host: &dyn crate::ModuleHost,
    access: PackageAccess,
    selection: &CdPackageSelection,
) -> ModuleResult<ConversionPcmSource> {
    let inventory = host.package_inventory(access)?;
    match selection.package_kind {
        CdPackageKind::Cue => {
            let (id, cue) = read_cue(host, access, &inventory, selection.anchor_file_index)?;
            open_cue_conversion_pcm_package(&cue, id, &inventory, |file| {
                host.package_file(access, file)
            })
        }
        CdPackageKind::Redumper => {
            let file = anchor_file(&inventory, selection.anchor_file_index)?;
            let (reader, windows) = open_redumper_pcm_package(file.id, &inventory, |member| {
                host.package_file(access, member)
            })?;
            conversion_pcm_source(Box::new(reader), windows, 0)
        }
    }
}

fn conversion_pcm_source(
    reader: Box<dyn junk_libs_disc::CdPcmReader + Send>,
    windows: Vec<DiscPcmTrack>,
    htoa_frames: u64,
) -> ModuleResult<ConversionPcmSource> {
    let total_frames = reader.total_frames();
    let mut expected_start = 0_u64;
    let tracks = windows
        .into_iter()
        .enumerate()
        .map(|(index, track)| {
            if track.start_frame != expected_start || track.frame_count == 0 {
                return Err(ModuleError::FailedOperation(
                    "music conversion track ranges are not contiguous and nonempty".into(),
                ));
            }
            expected_start = expected_start
                .checked_add(track.frame_count)
                .ok_or_else(|| ModuleError::FailedOperation("track range overflow".into()))?;
            Ok(ConversionPcmTrack {
                position: u16::from(track.position),
                start_frame: track.start_frame,
                frame_count: track.frame_count,
                leading_hidden_frames: if index == 0 { htoa_frames } else { 0 },
            })
        })
        .collect::<ModuleResult<Vec<_>>>()?;
    if expected_start != total_frames {
        return Err(ModuleError::FailedOperation(
            "music conversion track ranges do not cover the exact PCM timeline".into(),
        ));
    }
    Ok(ConversionPcmSource {
        reader,
        tracks,
        total_frames,
    })
}

fn parse_hex_id(value: &str, label: &str) -> ModuleResult<u32> {
    u32::from_str_radix(value, 16)
        .map_err(|_| ModuleError::Invalid(format!("invalid {label} identifier")))
}

/// Domain policy for acquiring catalog data from a supported structural result.
pub fn accuraterip_query_from_structure(
    result: &CdProcessingResult,
) -> ModuleResult<catalog::DbarQuery> {
    let CdProcessingResult::Layout {
        tracks,
        disc_ids: Some(ids),
        limitations,
        ..
    } = result
    else {
        return Err(ModuleError::Invalid(
            "AccurateRip requires a complete audio CD layout".into(),
        ));
    };
    if !limitations.is_empty()
        || tracks.is_empty()
        || tracks.len() > 99
        || calculate_cd_ids(tracks)? != *ids
    {
        return Err(ModuleError::Invalid(
            "AccurateRip requires a supported complete audio CD layout".into(),
        ));
    }
    Ok(catalog::DbarQuery {
        track_count: tracks.len() as u8,
        accuraterip_id1: parse_hex_id(&ids.accuraterip_id1, "AccurateRip 1")?,
        accuraterip_id2: parse_hex_id(&ids.accuraterip_id2, "AccurateRip 2")?,
        cddb_id: parse_hex_id(&ids.freedb, "CDDB")?,
    })
}

fn parsed_dbar(
    snapshot: &crate::CatalogSnapshotInput,
) -> ModuleResult<(DbarFile, DbarSnapshotObservation)> {
    if snapshot.provider.0 != ACCURATERIP_PROVIDER_ID || snapshot.entries.len() != 1 {
        return Err(ModuleError::Invalid(
            "frozen snapshot is not an AccurateRip response".into(),
        ));
    }
    let observation: DbarSnapshotObservation = snapshot
        .observation
        .parse()
        .map_err(|error| ModuleError::Invalid(error.to_string()))?;
    let parsed: ParsedDbarSnapshot = snapshot.entries[0]
        .parse()
        .map_err(|error| ModuleError::Invalid(error.to_string()))?;
    if observation.schema_version != 1
        || parsed.responses.len() != observation.response_count as usize
        || matches!(observation.outcome, DbarOutcome::NoData) && !parsed.responses.is_empty()
        || matches!(observation.outcome, DbarOutcome::Data) && parsed.responses.is_empty()
    {
        return Err(ModuleError::Invalid(
            "AccurateRip snapshot observation disagrees with its verified bytes".into(),
        ));
    }
    let dbar = DbarFile {
        responses: parsed
            .responses
            .into_iter()
            .map(|response| DbarResponse {
                track_count: response.track_count,
                ar_id1: response.accuraterip_id1,
                ar_id2: response.accuraterip_id2,
                cddb_id: response.cddb_id,
                tracks: response
                    .tracks
                    .into_iter()
                    .map(|track| ExpectedChecksum {
                        confidence: track.confidence,
                        checksum: track.checksum,
                        checksum_450: track.checksum_450,
                    })
                    .collect(),
            })
            .collect(),
    };
    Ok((dbar, observation))
}

fn verify_selection(
    host: &dyn crate::ModuleHost,
    access: PackageAccess,
    inventory: &PackageInventory,
    selection: &AccurateRipSelection,
) -> ModuleResult<CdProcessingResult> {
    if !(0..=junk_libs_accuraterip::DEFAULT_MAX_SAMPLE_SHIFT).contains(&selection.maximum_offset) {
        return Err(ModuleError::Invalid(
            "maximum AccurateRip offset is out of bounds".into(),
        ));
    }
    let package = CdPackageSelection {
        package_kind: selection.package_kind,
        anchor_file_index: selection.anchor_file_index,
    };
    let (reader, tracks, layout) = open_pcm_selection(host, access, inventory, &package)?;
    let CdProcessingResult::Layout {
        disc_ids: Some(ids),
        tracks: layout_tracks,
        ..
    } = layout
    else {
        return Err(ModuleError::UnavailableCapability(
            "AccurateRip requires a supported complete audio layout".into(),
        ));
    };
    let snapshot = host.catalog_snapshot(selection.snapshot_id)?;
    let (dbar, observation) = parsed_dbar(&snapshot)?;
    let expected = AccurateRipDatabaseIdentity {
        track_count: u8::try_from(layout_tracks.len())
            .map_err(|_| ModuleError::Invalid("too many CD tracks".into()))?,
        accuraterip_id1: parse_hex_id(&ids.accuraterip_id1, "AccurateRip 1")?,
        accuraterip_id2: parse_hex_id(&ids.accuraterip_id2, "AccurateRip 2")?,
        cddb_id: parse_hex_id(&ids.freedb, "CDDB")?,
    };
    if observation.query.track_count != expected.track_count
        || observation.query.accuraterip_id1 != expected.accuraterip_id1
        || observation.query.accuraterip_id2 != expected.accuraterip_id2
        || observation.query.cddb_id != expected.cddb_id
    {
        return Err(ModuleError::Invalid(
            "AccurateRip snapshot query does not match the frozen package layout".into(),
        ));
    }
    let mut disc = BoundedDiscReader::new(reader, tracks)
        .map_err(|error| ModuleError::FailedOperation(error.to_string()))?;
    let summary = verify_with_offset_reader(
        &dbar,
        &mut disc,
        VerificationOptions {
            max_sample_shift: selection.maximum_offset,
        },
        || {
            host.checkpoint()
                .map_err(|error| junk_libs_core::AnalysisError::Other(error.to_string()))
        },
    )
    .map_err(|error| ModuleError::FailedOperation(error.to_string()))?;
    let fingerprint = host.source_fingerprint(access)?;
    let verification = interpret_accuraterip_summary(
        &dbar,
        &summary,
        InterpretAccurateRipInput {
            database: expected,
            database_response_digest: snapshot.digest,
            input_digests: fingerprint
                .files
                .into_iter()
                .map(|file| file.sha256)
                .collect(),
        },
    )?;
    Ok(CdProcessingResult::AudioVerification { verification })
}

/// Detect the one supported CD package anchor without opening any archive path.
/// Application read/command services use this to present and prepare the same
/// music-owned track mapping as processing jobs.
pub fn detect_package(inventory: &PackageInventory) -> ModuleResult<CdPackageSelection> {
    let anchors = inventory
        .files
        .iter()
        .enumerate()
        .filter_map(|(index, file)| {
            let name = file.display_name.as_str().to_ascii_lowercase();
            if name.ends_with(".cue") {
                Some((index, CdPackageKind::Cue))
            } else if name.ends_with(".toc") {
                Some((index, CdPackageKind::Redumper))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    let [(anchor_file_index, package_kind)] = anchors.as_slice() else {
        return Err(ModuleError::UnavailableCapability(
            "CD analysis needs exactly one CUE or redumper TOC anchor; review may continue unidentified"
                .into(),
        ));
    };
    Ok(CdPackageSelection {
        package_kind: *package_kind,
        anchor_file_index: *anchor_file_index,
    })
}

pub struct Music;

impl CollectionModule for Music {
    fn descriptor(&self) -> ModuleDescriptor {
        let mut descriptor = descriptor(
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
        );
        for capability in &mut descriptor.capabilities {
            if capability.id.as_str() == "org.junk.music.import-analysis" {
                capability.availability = Availability::Available;
            }
        }
        let capability = |id: &str, display_name: &str| Capability {
            id: CapabilityId::new(id).unwrap(),
            display_name: display_name.into(),
            availability: Availability::Available,
            scope: OperationScope::CollectionDomain,
        };
        descriptor.capabilities.extend([
            capability(CD_STRUCTURE_OPERATION, "Inspect CD structure"),
            capability(
                ACCURATERIP_VERIFICATION_OPERATION,
                "Verify audio with AccurateRip",
            ),
        ]);
        descriptor
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
        capability: &CapabilityId,
        result: &RawJson,
    ) -> ModuleResult<()> {
        if capability.as_str() == "org.junk.music.import-analysis" {
            let output: MusicImportAnalysisOutput = result
                .parse()
                .map_err(|error| ModuleError::Invalid(error.to_string()))?;
            if !output.candidates.is_empty()
                || output.observations.len() != 1
                || output.limitations.len() > 64
            {
                return Err(ModuleError::Invalid(
                    "invalid music Import analysis result".into(),
                ));
            }
            let observation: CdProcessingResult = output.observations[0]
                .parse()
                .map_err(|error| ModuleError::Invalid(error.to_string()))?;
            if !matches!(observation, CdProcessingResult::Layout { .. }) {
                return Err(ModuleError::Invalid(
                    "music Import analysis must contain structural evidence".into(),
                ));
            }
            return self.validate_operation_result(
                &CapabilityId::new(CD_STRUCTURE_OPERATION).unwrap(),
                &output.observations[0],
            );
        }
        let result: CdProcessingResult = result
            .parse()
            .map_err(|error| ModuleError::Invalid(error.to_string()))?;
        match result {
            CdProcessingResult::AudioVerification { verification } => {
                if capability.as_str() != ACCURATERIP_VERIFICATION_OPERATION {
                    return Err(ModuleError::Invalid(
                        "CD result does not match its operation".into(),
                    ));
                }
                verification
                    .validate()
                    .map_err(|error| ModuleError::Invalid(error.to_string()))?
            }
            CdProcessingResult::Layout {
                tracks,
                source_spans,
                disc_ids,
                limitations,
            } => {
                if capability.as_str() != CD_STRUCTURE_OPERATION {
                    return Err(ModuleError::Invalid(
                        "CD result does not match its operation".into(),
                    ));
                }
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
        request: OperationRequest,
        host: &dyn crate::ModuleHost,
    ) -> ModuleResult<OperationOutput> {
        let access = operation_access(&request)?;
        let inventory = host.package_inventory(access)?;
        let result = match request.capability.as_str() {
            CD_STRUCTURE_OPERATION => {
                let selection: CdPackageSelection = request
                    .input
                    .parse()
                    .map_err(|error| ModuleError::Invalid(error.to_string()))?;
                analyze_selection(host, access, &inventory, &selection)?
            }
            ACCURATERIP_VERIFICATION_OPERATION => {
                let selection: AccurateRipSelection = request
                    .input
                    .parse()
                    .map_err(|error| ModuleError::Invalid(error.to_string()))?;
                verify_selection(host, access, &inventory, &selection)?
            }
            "org.junk.music.import-analysis" => {
                let _: MusicImportAnalysisInput = request
                    .input
                    .parse()
                    .map_err(|error| ModuleError::Invalid(error.to_string()))?;
                let selection = detect_package(&inventory)?;
                let structure = analyze_selection(host, access, &inventory, &selection)?;
                let limitations = match &structure {
                    CdProcessingResult::Layout { limitations, .. } => limitations.clone(),
                    _ => unreachable!(),
                };
                return Ok(OperationOutput {
                    value: RawJson::from_value(
                        serde_json::to_value(MusicImportAnalysisOutput {
                            candidates: vec![],
                            limitations,
                            observations: vec![RawJson::from_value(
                                serde_json::to_value(structure)
                                    .map_err(|error| ModuleError::Invalid(error.to_string()))?,
                            )],
                        })
                        .map_err(|error| ModuleError::Invalid(error.to_string()))?,
                    ),
                });
            }
            _ => {
                return Err(ModuleError::UnknownCapability(
                    request.capability.to_string(),
                ));
            }
        };
        Ok(OperationOutput {
            value: RawJson::from_value(
                serde_json::to_value(result)
                    .map_err(|error| ModuleError::Invalid(error.to_string()))?,
            ),
        })
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
