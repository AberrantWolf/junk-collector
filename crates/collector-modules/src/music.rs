use collector_core::{CdTrackLayout, ModuleData, RawJson, SampleFrame, StereoSampleFrameOffset};
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
    pub evaluated_offset_start: StereoSampleFrameOffset,
    pub evaluated_offset_end: StereoSampleFrameOffset,
    pub tracks: Vec<AudioTrackVerification>,
    pub limitations: Vec<String>,
}

impl AudioVerificationResult {
    pub fn validate(&self) -> collector_core::Result<()> {
        if self.evaluated_offset_start.0 > self.evaluated_offset_end.0
            || self.selected_offset.is_some_and(|offset| {
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
    },
    AudioVerification {
        verification: AudioVerificationResult,
    },
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
            CdProcessingResult::Layout { tracks } => validate_cd_layout(&tracks)?,
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
