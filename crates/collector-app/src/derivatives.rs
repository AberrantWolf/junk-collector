use std::io::{Read, Seek, Write};

use collector_core::{
    ByteLength, ConversionProfile, ConversionRecipe, ConversionSettings, ConversionTrack, Digest,
    Error, PcmFrame, PcmSampleFormat, RecipeId, Result, SourceFingerprint,
};

use crate::ProcessingExecutionContext;

/// Resolve a user-readable preset to one immutable recipe after the music module
/// has established exact track ownership. Verification evidence is intentionally
/// not an input: conversion eligibility comes from the supported PCM layout.
pub fn build_music_conversion_recipe(
    id: RecipeId,
    source_fingerprint: SourceFingerprint,
    tracks: &[collector_modules::music::ConversionPcmTrack],
    selected_positions: &[u16],
    profile: ConversionProfile,
    encoder_version: String,
) -> Result<ConversionRecipe> {
    if selected_positions.is_empty() || selected_positions.windows(2).any(|pair| pair[0] >= pair[1])
    {
        return Err(Error::Invalid(
            "conversion track selection must be nonempty, unique and ordered".into(),
        ));
    }
    let selected = selected_positions
        .iter()
        .map(|position| {
            tracks
                .iter()
                .find(|track| track.position == *position)
                .map(|track| ConversionTrack {
                    position: track.position,
                    start: PcmFrame(track.start_frame),
                    length: PcmFrame(track.frame_count),
                })
                .ok_or_else(|| Error::Invalid(format!("unknown music track position {position}")))
        })
        .collect::<Result<Vec<_>>>()?;
    let (settings, encoder, output_format) = match profile {
        ConversionProfile::Flac => (
            ConversionSettings::Flac { level: 5 },
            "flac",
            PcmSampleFormat::CDDA_I16_LE_STEREO,
        ),
        ConversionProfile::Opus { bitrate_kbps } => (
            ConversionSettings::Opus {
                bitrate_kbps,
                frame_milliseconds: 20,
            },
            "opusenc",
            PcmSampleFormat::OPUS_DECODED_I16_STEREO,
        ),
    };
    ConversionRecipe {
        id,
        version: 1,
        settings,
        source_fingerprint,
        tracks: selected,
        encoder: encoder.into(),
        encoder_version,
        input_format: PcmSampleFormat::CDDA_I16_LE_STEREO,
        output_format,
        digest: Digest::sha256(b"unsealed conversion recipe"),
    }
    .seal()
}

/// Opaque scratch handle. Infrastructure owns concrete file creation/cleanup;
/// provider adapters never receive archive roots or package labels.
pub trait ConversionScratch: Read + Write + Seek + Send {
    fn discard_partial(&mut self) -> std::io::Result<()>;
}

impl ConversionScratch for std::io::Cursor<Vec<u8>> {
    fn discard_partial(&mut self) -> std::io::Result<()> {
        self.get_mut().clear();
        self.set_position(0);
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EncoderAvailability {
    Available { encoder: String, version: String },
    Unavailable { reason: String },
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConversionValidation {
    pub media_type: String,
    pub bytes: ByteLength,
    pub digest: Digest,
    pub decoded_format: PcmSampleFormat,
    pub decoded_frames: u64,
    pub lossless_pcm_equal: bool,
    /// Opus pre-skip in its fixed 48-kHz decoded-frame coordinate space.
    pub encoder_delay_frames: u32,
    /// End padding derived from the final Ogg granule position.
    pub end_padding_frames: u32,
}

/// Optional encoder-process mechanism. Authorization, durable Job identity,
/// source currency and publication stay in application services; the adapter
/// receives only a sealed recipe, an authorized integer PCM reader and scratch.
pub trait AudioConversionPort: Send + Sync {
    /// Probe the currently configured server-owned tool for a readable preset.
    /// The returned complete report is authoritative recipe input; clients never
    /// supply executable paths or version claims.
    fn configured_availability(&self, profile: &ConversionProfile) -> EncoderAvailability {
        let encoder = match profile {
            ConversionProfile::Flac => "flac",
            ConversionProfile::Opus { .. } => "opusenc",
        };
        EncoderAvailability::Unavailable {
            reason: format!("configured {encoder} tool is unavailable"),
        }
    }

    fn availability(&self, recipe: &ConversionRecipe) -> EncoderAvailability;

    fn convert(
        &self,
        recipe: &ConversionRecipe,
        source: &mut (dyn junk_libs_disc::CdPcmReader + Send),
        scratch: &mut dyn ConversionScratch,
        execution: &mut dyn ProcessingExecutionContext,
    ) -> Result<ConversionValidation>;
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use collector_core::{
        ByteLength, Digest, PackageFileFingerprint, PackageFileId, PackageScope, RepresentationId,
    };

    use super::*;

    fn source() -> SourceFingerprint {
        SourceFingerprint::calculate(
            PackageScope::Representation(RepresentationId::new()),
            vec![PackageFileFingerprint {
                id: PackageFileId::new(),
                bytes: ByteLength(16),
                sha256: Digest::sha256(b"source"),
            }],
            None,
            BTreeMap::new(),
        )
        .unwrap()
    }

    #[test]
    fn track_names_require_the_right_disc_and_complete_positions() {
        let release: collector_modules::music::MusicBrainzReleaseObservation =
            serde_json::from_value(serde_json::json!({
                "release_id": "00000000-0000-0000-0000-000000000001",
                "title": "Album",
                "disambiguation": null,
                "language": null,
                "script": null,
                "artist_credit": null,
                "artist_ids": [],
                "release_group_id": null,
                "disc_ids": ["disc-one", "disc-two"],
                "media": [
                    {"position": 1, "disc_ids": ["disc-one"], "tracks": [{"position": 1, "title": "First"}]},
                    {"position": 2, "disc_ids": ["disc-two"], "tracks": [{"position": 1, "title": "Second"}]}
                ]
            }))
            .unwrap();
        release.validate().unwrap();
        let track = ConversionTrack {
            position: 1,
            start: PcmFrame(0),
            length: PcmFrame(588),
        };
        assert_eq!(
            matching_musicbrainz_medium(&release, Some("disc-one"), std::slice::from_ref(&track))
                .unwrap()
                .tracks[0]
                .title,
            "First"
        );
        assert_eq!(
            matching_musicbrainz_medium(&release, Some("disc-two"), std::slice::from_ref(&track))
                .unwrap()
                .tracks[0]
                .title,
            "Second"
        );
        assert!(
            matching_musicbrainz_medium(&release, None, std::slice::from_ref(&track)).is_none()
        );
        assert!(
            matching_musicbrainz_medium(&release, Some("other"), std::slice::from_ref(&track))
                .is_none()
        );
        assert!(
            matching_musicbrainz_medium(
                &release,
                Some("disc-one"),
                &[
                    track,
                    ConversionTrack {
                        position: 2,
                        start: PcmFrame(588),
                        length: PcmFrame(588),
                    }
                ],
            )
            .is_none()
        );
        let assignment = collector_modules::music::MusicDiscAssignment {
            source_representation_id: RepresentationId::new(),
            release_id: release.release_id.clone(),
            medium_position: 1,
        };
        assert_eq!(
            resolve_music_medium(Some(&release), None, Some(&assignment), Some(&[1]))
                .0
                .unwrap()
                .tracks[0]
                .title,
            "First"
        );
        assert_eq!(
            resolve_music_medium(
                Some(&release),
                Some("disc-two"),
                Some(&assignment),
                Some(&[1])
            )
            .1,
            MusicDiscMatchState::AssignmentConflictsWithDiscId
        );
        assert_eq!(
            resolve_music_medium(Some(&release), Some("other"), None, Some(&[1])).1,
            MusicDiscMatchState::DiscIdNotListed
        );
        assert_eq!(
            resolve_music_medium(Some(&release), Some("disc-two"), None, Some(&[1, 2])).1,
            MusicDiscMatchState::TrackLayoutMismatch
        );
    }

    #[test]
    fn presets_seal_exact_music_owned_ranges_and_encoder_identity() {
        let tracks = vec![
            collector_modules::music::ConversionPcmTrack {
                position: 1,
                start_frame: 0,
                frame_count: 588,
                leading_hidden_frames: 588,
            },
            collector_modules::music::ConversionPcmTrack {
                position: 2,
                start_frame: 588,
                frame_count: 1_176,
                leading_hidden_frames: 0,
            },
        ];
        let flac = build_music_conversion_recipe(
            RecipeId::new(),
            source(),
            &tracks,
            &[1, 2],
            ConversionProfile::Flac,
            "1.5.0".into(),
        )
        .unwrap();
        assert_eq!(flac.settings, ConversionSettings::Flac { level: 5 });
        assert_eq!(flac.tracks[0].length, PcmFrame(588));
        let opus = build_music_conversion_recipe(
            RecipeId::new(),
            source(),
            &tracks,
            &[2],
            ConversionProfile::Opus { bitrate_kbps: 128 },
            "0.2".into(),
        )
        .unwrap();
        assert_eq!(opus.tracks[0].start, PcmFrame(588));
        assert_eq!(opus.output_format.sample_rate, 48_000);
        assert_ne!(flac.digest, opus.digest);
        assert!(
            build_music_conversion_recipe(
                RecipeId::new(),
                source(),
                &tracks,
                &[2, 1],
                ConversionProfile::Flac,
                "1.5.0".into(),
            )
            .is_err()
        );
    }
    #[test]
    fn validation_envelope_distinguishes_opus_structure_from_lossless_equality() {
        let recipe = build_music_conversion_recipe(
            RecipeId::new(),
            source(),
            &[collector_modules::music::ConversionPcmTrack {
                position: 1,
                start_frame: 0,
                frame_count: 588,
                leading_hidden_frames: 0,
            }],
            &[1],
            ConversionProfile::Opus { bitrate_kbps: 128 },
            "opusenc test report".into(),
        )
        .unwrap();
        let mut validation = ConversionValidation {
            media_type: "audio/ogg; codecs=opus".into(),
            bytes: ByteLength(20),
            digest: Digest::sha256(b"test envelope only"),
            decoded_format: PcmSampleFormat::OPUS_DECODED_I16_STEREO,
            decoded_frames: 640,
            lossless_pcm_equal: false,
            encoder_delay_frames: 312,
            end_padding_frames: 8,
        };
        validation.validate_recipe(&recipe).unwrap();
        validation.lossless_pcm_equal = true;
        assert!(validation.validate_recipe(&recipe).is_err());
        validation.lossless_pcm_equal = false;
        validation.decoded_frames = 588;
        assert!(validation.validate_recipe(&recipe).is_err());
    }
}

use crate::{CollectionService, CommandContext, FrozenJobInput, JobClaim, JobSpec, Principal};
use collector_core::{
    CaptureId, CollectionBinding, CollectionId, JobId, PackageScope, RepresentationId,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PrepareMusicTrack {
    pub source_representation_id: RepresentationId,
    pub profile: ConversionProfile,
}

pub use collector_api_types::{
    MusicAlbumSelection, MusicTrackAvailability, MusicTrackAvailabilityState,
};

#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
pub struct MusicAlbumCoverage {
    pub release_id: Option<String>,
    pub release_title: Option<String>,
    pub media: Vec<MusicMediumCoverage>,
    pub sources: Vec<MusicDiscSourceCoverage>,
}

#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
pub struct MusicMediumCoverage {
    pub position: u16,
    pub track_count: usize,
    pub registered_disc_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
pub struct MusicDiscSourceCoverage {
    pub capture_id: CaptureId,
    pub captured_at: chrono::DateTime<chrono::Utc>,
    pub source_representation_id: RepresentationId,
    pub file_label: String,
    pub file_count: usize,
    pub recorded_disc_id: Option<String>,
    pub track_count: Option<usize>,
    pub assigned_medium_position: Option<u16>,
    pub medium_position: Option<u16>,
    pub match_state: MusicDiscMatchState,
    pub audio_verification: Option<collector_modules::music::AudioVerificationStatus>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum MusicDiscMatchState {
    NoRelease,
    NoLayout,
    DiscIdMatched,
    CuratorAssigned,
    SingleMedium,
    NeedsAssignment,
    DiscIdNotListed,
    AmbiguousDiscId,
    AssignmentConflictsWithDiscId,
    TrackLayoutMismatch,
}

#[derive(Clone, Debug)]
pub struct ActiveDerivativeJob {
    pub id: JobId,
    pub user_id: collector_core::UserId,
    pub input: FrozenDerivativeJob,
}

/// One request always means one independently playable track file.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrepareMusicDerivative {
    pub representation_id: RepresentationId,
    pub selection: collector_modules::music::CdPackageSelection,
    pub track_position: u16,
    pub profile: ConversionProfile,
    /// Full configured encoder probe report, checked by the conversion adapter.
    pub encoder_version: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrozenDerivativeJob {
    pub capture_id: CaptureId,
    pub collection_id: CollectionId,
    pub collection_binding: CollectionBinding,
    pub selection: collector_modules::music::CdPackageSelection,
    pub recipe: ConversionRecipe,
    pub representation_id: RepresentationId,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl FrozenDerivativeJob {
    pub fn validate(&self) -> Result<()> {
        self.recipe.validate()?;
        if self.collection_binding
            != (CollectionBinding::Assigned {
                module: collector_core::ModuleId("org.junk.music".into()),
            })
            || self.recipe.tracks.len() != 1
            || !matches!(
                self.recipe.source_fingerprint.source,
                PackageScope::Representation(_)
            )
            || self.representation_id.0 != self.recipe.id.0
            || self.representation_id.0 != derivative_identity(self.capture_id, &self.recipe.digest)
        {
            return Err(Error::Invalid("invalid frozen music derivative".into()));
        }
        Ok(())
    }
}

impl ConversionValidation {
    pub fn validate_recipe(&self, recipe: &ConversionRecipe) -> Result<()> {
        recipe.validate()?;
        self.digest.validate()?;
        let frames = recipe
            .tracks
            .iter()
            .try_fold(0_u64, |sum, track| sum.checked_add(track.length.0))
            .ok_or_else(|| Error::Invalid("conversion frame count overflow".into()))?;
        let valid = match recipe.settings {
            ConversionSettings::Flac { .. } => {
                self.media_type == "audio/flac"
                    && self.lossless_pcm_equal
                    && self.decoded_frames == frames
                    && self.encoder_delay_frames == 0
                    && self.end_padding_frames == 0
            }
            ConversionSettings::Opus { .. } => {
                self.media_type == "audio/ogg; codecs=opus"
                    && !self.lossless_pcm_equal
                    && self.decoded_frames
                        == frames
                            .checked_mul(160)
                            .and_then(|v| v.checked_add(146))
                            .map(|v| v / 147)
                            .unwrap_or(0)
            }
        };
        if !valid
            || self.bytes.0 == 0
            || self.bytes.0 > 8 * 1024 * 1024 * 1024
            || self.digest.algorithm != collector_core::HashAlgorithm::Sha256
            || self.decoded_format != recipe.output_format
        {
            return Err(Error::Invalid(
                "conversion validation disagrees with frozen recipe".into(),
            ));
        }
        Ok(())
    }
}

struct ConversionHost<'a> {
    port: &'a dyn crate::ProcessingPort,
    access: collector_modules::PackageAccess,
    claim: Option<(&'a dyn crate::JobPort, &'a JobClaim)>,
}
impl collector_modules::ModuleHost for ConversionHost<'_> {
    fn checkpoint(&self) -> collector_modules::ModuleResult<()> {
        if let Some((jobs, claim)) = self.claim
            && jobs
                .cancellation_requested(claim.id, &claim.claim_token)
                .map_err(crate::processing_module_error)?
        {
            return Err(crate::processing_module_error(Error::JobCancelled));
        }
        Ok(())
    }
    fn package_inventory(
        &self,
        access: collector_modules::PackageAccess,
    ) -> collector_modules::ModuleResult<collector_core::PackageInventory> {
        if access != self.access {
            return Err(collector_modules::ModuleError::AccessDenied);
        }
        self.port
            .package_inventory(access.collection_id, access.scope)
            .map_err(crate::processing_module_error)
    }
    fn package_file(
        &self,
        access: collector_modules::PackageAccess,
        id: collector_core::PackageFileId,
    ) -> collector_modules::ModuleResult<Box<dyn collector_modules::RepresentationReader>> {
        self.checkpoint()?;
        if access != self.access {
            return Err(collector_modules::ModuleError::AccessDenied);
        }
        Ok(Box::new(
            self.port
                .package_file(access.collection_id, access.scope, id)
                .map_err(crate::processing_module_error)?,
        ))
    }
    fn representation_access(
        &self,
        id: RepresentationId,
    ) -> collector_modules::ModuleResult<collector_modules::PackageAccess> {
        if self.access.scope != PackageScope::Representation(id) {
            return Err(collector_modules::ModuleError::AccessDenied);
        }
        Ok(self.access)
    }
    fn submit_job(
        &self,
        _: collector_modules::JobSubmission,
    ) -> collector_modules::ModuleResult<JobId> {
        Err(collector_modules::ModuleError::AccessDenied)
    }
}

impl CollectionService {
    pub fn with_audio_converter(
        mut self,
        converter: std::sync::Arc<dyn AudioConversionPort>,
    ) -> Self {
        self.audio_converter = Some(converter);
        self
    }

    pub fn prepare_music_derivative(
        &self,
        actor: &Principal,
        capture_id: CaptureId,
        command: PrepareMusicDerivative,
        context: &CommandContext,
    ) -> Result<JobId> {
        let capture = self.reader.capture(capture_id)?;
        let eligibility = self.content_write_eligibility(actor, capture.collection_id)?;
        self.content_write_eligibility_for_job(
            actor.user_id,
            Some(capture.collection_id),
            capture.association.item_id,
        )?;
        if eligibility.owner.0 != "org.junk.music" {
            return Err(Error::Invalid("music collection required".into()));
        }
        if !capture.representations.iter().any(|r| {
            r.id == command.representation_id
                && matches!(
                    r.role,
                    collector_core::RepresentationRole::PreservationMaster
                )
        }) {
            return Err(Error::NotFound);
        }
        // Replay happens before any tool or source access. Frozen inputs survive tool changes.
        self.execute_command(
            actor,
            context,
            "music.derivative.prepare",
            &(capture_id, &command),
            |effects| {
                let frozen =
                    self.freeze_music_derivative(&capture, eligibility.binding.clone(), &command)?;
                let id = crate::deterministic_job_id(
                    actor.user_id,
                    "music.derivative.prepare",
                    context.key(),
                );
                Self::enqueue_derivative(effects, actor.user_id, id, frozen)
            },
        )
    }

    pub(super) fn freeze_music_derivative(
        &self,
        capture: &collector_core::Capture,
        binding: CollectionBinding,
        command: &PrepareMusicDerivative,
    ) -> Result<FrozenDerivativeJob> {
        let capture_id = capture.id;
        let source = PackageScope::Representation(command.representation_id);
        let fingerprint = self
            .processing
            .recorded_source(capture.collection_id, source)?;
        let host = ConversionHost {
            port: self.processing.as_ref(),
            access: collector_modules::PackageAccess {
                collection_id: capture.collection_id,
                scope: source,
            },
            claim: None,
        };
        let pcm = collector_modules::music::open_conversion_pcm_selection(
            &host,
            host.access,
            &command.selection,
        )?;
        let mut recipe = build_music_conversion_recipe(
            RecipeId::new(),
            fingerprint,
            &pcm.tracks,
            &[command.track_position],
            command.profile.clone(),
            command.encoder_version.clone(),
        )?;
        // Identity excludes requester/key, so separate requests converge at publication.
        let identity = derivative_identity(capture_id, &recipe.digest);
        recipe.id = RecipeId(identity);
        let converter = self
            .audio_converter
            .as_ref()
            .ok_or_else(|| Error::Unavailable("audio encoder is not configured".into()))?;
        if let EncoderAvailability::Unavailable { reason } = converter.availability(&recipe) {
            return Err(Error::Unavailable(reason));
        }
        let frozen = FrozenDerivativeJob {
            capture_id,
            collection_id: capture.collection_id,
            collection_binding: binding,
            selection: command.selection.clone(),
            recipe,
            representation_id: RepresentationId(identity),
            created_at: self.authentication.now(),
        };
        frozen.validate()?;
        Ok(frozen)
    }

    fn enqueue_derivative(
        effects: &(impl crate::JobPort + ?Sized),
        user: collector_core::UserId,
        id: JobId,
        frozen: FrozenDerivativeJob,
    ) -> Result<JobId> {
        effects.enqueue_job(Self::derivative_job_spec(user, id, frozen))
    }

    pub(super) fn derivative_job_spec(
        user: collector_core::UserId,
        id: JobId,
        frozen: FrozenDerivativeJob,
    ) -> JobSpec {
        JobSpec {
            id,
            user_id: user,
            collection_id: Some(frozen.collection_id),
            item_id: None,
            kind: "music_derivative".into(),
            phase: "queued".into(),
            total_units: frozen.recipe.tracks[0].length.0,
            request: serde_json::json!({
                "capture_id": frozen.capture_id,
                "representation_id": frozen.representation_id,
            }),
            frozen_input: Some(FrozenJobInput::Derivative(frozen)),
        }
    }

    /// Prepare one track using only server-owned encoder configuration and the
    /// music module's package detection. This is the command used by HTTP clients.
    pub fn prepare_configured_music_track(
        &self,
        actor: &Principal,
        item_id: collector_core::ItemId,
        capture_id: CaptureId,
        command: PrepareMusicTrack,
        track_position: u16,
        context: &CommandContext,
    ) -> Result<JobId> {
        let item = self.reader.item(item_id)?;
        self.authorize(actor, item.collection_id, crate::Action::Curate)?;
        if item.deleted {
            return Err(Error::NotFound);
        }
        let capture = self.reader.capture(capture_id)?;
        if capture.collection_id != item.collection_id
            || capture.association.item_id != Some(item_id)
            || !capture.representations.iter().any(|representation| {
                representation.id == command.source_representation_id
                    && matches!(
                        representation.role,
                        collector_core::RepresentationRole::PreservationMaster
                    )
            })
        {
            return Err(Error::NotFound);
        }
        let eligibility = self.content_write_eligibility_for_job(
            actor.user_id,
            Some(item.collection_id),
            Some(item_id),
        )?;
        if eligibility.owner.0 != "org.junk.music" {
            return Err(Error::Invalid("music collection required".into()));
        }
        // The receipt binds the client request, not derived tool or source state.
        // Resolve those only on the first execution; never nest command receipts.
        self.execute_command(
            actor,
            context,
            "music.track.prepare",
            &(item_id, capture_id, track_position, &command),
            |effects| {
                let selection = self.detect_music_selection(
                    capture.collection_id,
                    command.source_representation_id,
                )?;
                let version = match self.encoder_availability(&command.profile) {
                    EncoderAvailability::Available { version, .. } => version,
                    EncoderAvailability::Unavailable { reason } => {
                        return Err(Error::Unavailable(reason));
                    }
                };
                let frozen = self.freeze_music_derivative(
                    &capture,
                    eligibility.binding.clone(),
                    &PrepareMusicDerivative {
                        representation_id: command.source_representation_id,
                        selection,
                        track_position,
                        profile: command.profile.clone(),
                        encoder_version: version,
                    },
                )?;
                // A user's identical track/profile requests converge even when
                // they arrive from an album batch, an import, or the track button.
                let id = crate::deterministic_job_id(
                    actor.user_id,
                    "music.derivative.output",
                    &frozen.representation_id.to_string(),
                );
                Self::enqueue_derivative(effects, actor.user_id, id, frozen)
            },
        )
    }

    /// Ordered, viewer-safe track/profile availability. Published outputs are
    /// described from immutable Capture/recipe records even when the music module
    /// or current encoder is absent.
    pub fn music_track_availability(
        &self,
        actor: &Principal,
        item_id: collector_core::ItemId,
    ) -> Result<Vec<MusicTrackAvailability>> {
        let item = self.reader.item(item_id)?;
        self.authorize(actor, item.collection_id, crate::Action::Browse)?;
        self.music_track_availability_for_item(actor.user_id, &item)
    }

    /// Native browsing resolves only committed observations and immutable recipes.
    pub fn recorded_music_track_availability(
        &self,
        actor: &Principal,
        item_id: collector_core::ItemId,
    ) -> Result<Vec<MusicTrackAvailability>> {
        let item = self.reader.item(item_id)?;
        self.authorize(actor, item.collection_id, crate::Action::Browse)?;
        self.music_track_availability_recorded(actor.user_id, &item)
    }

    pub fn music_album_coverage(
        &self,
        actor: &Principal,
        item_id: collector_core::ItemId,
    ) -> Result<MusicAlbumCoverage> {
        let item = self.reader.item(item_id)?;
        self.authorize(actor, item.collection_id, crate::Action::Browse)?;
        if item.deleted || item.domain.module.0 != "org.junk.music" {
            return Err(Error::NotFound);
        }
        let release = if self.modules.has_module(&item.domain.module)
            && self.modules.inspect(&item.domain).is_ok()
        {
            self.effective_music_release(&item)?
        } else {
            None
        };
        let assignments = self.music_disc_assignments(&item)?;
        let mut sources = Vec::new();
        for capture in item
            .captures
            .iter()
            .filter(|capture| capture.association.item_id == Some(item.id))
        {
            for source in capture.representations.iter().filter(|representation| {
                representation.role == collector_core::RepresentationRole::PreservationMaster
            }) {
                let layout = self.recorded_music_layout(item.collection_id, source.id)?;
                let (disc_id, positions) = match layout {
                    Some(collector_modules::music::CdProcessingResult::Layout {
                        tracks,
                        disc_ids,
                        ..
                    }) => (
                        disc_ids.map(|ids| ids.musicbrainz),
                        Some(
                            tracks
                                .iter()
                                .map(|track| u16::from(track.position))
                                .collect::<Vec<_>>(),
                        ),
                    ),
                    _ => (None, None),
                };
                let assignment = assignments.iter().find(|value| {
                    value.source_representation_id == source.id
                        && release
                            .as_ref()
                            .is_some_and(|release| value.release_id == release.release_id)
                });
                let (medium, match_state) = resolve_music_medium(
                    release.as_ref(),
                    disc_id.as_deref(),
                    assignment,
                    positions.as_deref(),
                );
                let audio_verification = match self.latest_music_result(
                    item.collection_id,
                    source.id,
                    collector_modules::music::ACCURATERIP_VERIFICATION_OPERATION,
                )? {
                    Some((
                        _,
                        collector_modules::music::CdProcessingResult::AudioVerification {
                            verification,
                        },
                    )) => Some(verification.status),
                    _ => None,
                };
                sources.push(MusicDiscSourceCoverage {
                    capture_id: capture.id,
                    captured_at: capture.created_at,
                    source_representation_id: source.id,
                    file_label: source
                        .files
                        .first()
                        .map(|file| file.name.clone())
                        .unwrap_or_else(|| "Original package".into()),
                    file_count: source.files.len(),
                    recorded_disc_id: disc_id,
                    track_count: positions.as_ref().map(Vec::len),
                    assigned_medium_position: assignment.map(|value| value.medium_position),
                    medium_position: medium.map(|value| value.position),
                    match_state,
                    audio_verification,
                });
            }
        }
        Ok(MusicAlbumCoverage {
            release_id: release.as_ref().map(|value| value.release_id.clone()),
            release_title: release.as_ref().map(|value| value.title.clone()),
            media: release
                .as_ref()
                .map(|value| {
                    value
                        .media
                        .iter()
                        .map(|medium| MusicMediumCoverage {
                            position: medium.position,
                            track_count: medium.tracks.len(),
                            registered_disc_ids: medium.disc_ids.clone(),
                        })
                        .collect()
                })
                .unwrap_or_default(),
            sources,
        })
    }

    pub(super) fn latest_music_result(
        &self,
        collection_id: CollectionId,
        source: RepresentationId,
        operation: &str,
    ) -> Result<
        Option<(
            collector_core::ObservationId,
            collector_modules::music::CdProcessingResult,
        )>,
    > {
        let scope = PackageScope::Representation(source);
        let fingerprint = match self.processing.recorded_source(collection_id, scope) {
            Ok(value) => value,
            Err(Error::NotFound) => return Ok(None),
            Err(error) => return Err(error),
        };
        let mut after = None;
        loop {
            let page = self.processing.observations(&crate::ProcessingQuery {
                collection_id,
                source: scope,
                source_fingerprint: fingerprint.clone(),
                operation: Some(collector_core::OperationId::new(operation)?),
                after,
                limit: 200,
            })?;
            for observation in &page.items {
                if observation.source_fingerprint != fingerprint {
                    continue;
                }
                if operation == "org.junk.music.import-analysis" {
                    if let Ok(output) = observation
                        .typed_result
                        .payload
                        .parse::<collector_modules::music::MusicImportAnalysisOutput>(
                    ) && let Some(result) = output.observations.iter().find_map(|value| {
                        value
                            .parse::<collector_modules::music::CdProcessingResult>()
                            .ok()
                    }) {
                        return Ok(Some((observation.id, result)));
                    }
                } else if let Ok(result) = observation
                    .typed_result
                    .payload
                    .parse::<collector_modules::music::CdProcessingResult>()
                {
                    return Ok(Some((observation.id, result)));
                }
            }
            if page.next_cursor.is_none() {
                return Ok(None);
            }
            after = page.items.last().map(|value| value.id);
        }
    }

    pub(super) fn recorded_music_layout(
        &self,
        collection_id: CollectionId,
        source: RepresentationId,
    ) -> Result<Option<collector_modules::music::CdProcessingResult>> {
        for operation in [
            collector_modules::music::CD_STRUCTURE_OPERATION,
            "org.junk.music.import-analysis",
        ] {
            if let Some((_, layout @ collector_modules::music::CdProcessingResult::Layout { .. })) =
                self.latest_music_result(collection_id, source, operation)?
            {
                return Ok(Some(layout));
            }
        }
        Ok(None)
    }

    pub(super) fn music_track_availability_for_item(
        &self,
        user_id: collector_core::UserId,
        item: &collector_core::Item,
    ) -> Result<Vec<MusicTrackAvailability>> {
        self.music_track_rows(user_id, item, false)
    }
    pub(super) fn music_track_availability_recorded(
        &self,
        user_id: collector_core::UserId,
        item: &collector_core::Item,
    ) -> Result<Vec<MusicTrackAvailability>> {
        self.music_track_rows(user_id, item, true)
    }
    fn music_track_rows(
        &self,
        user_id: collector_core::UserId,
        item: &collector_core::Item,
        recorded_only: bool,
    ) -> Result<Vec<MusicTrackAvailability>> {
        let item_id = item.id;
        if item.deleted {
            return Err(Error::NotFound);
        }
        let music_owner = item.domain.module.0 == "org.junk.music";
        let readable_music_schema = music_owner
            && self.modules.has_module(&item.domain.module)
            && self.modules.inspect(&item.domain).is_ok();
        let writable_music = music_owner
            && self
                .content_write_eligibility_for_job(user_id, Some(item.collection_id), Some(item_id))
                .is_ok();
        let titles = item
            .domain
            .payload
            .parse::<collector_modules::music::MusicItemData>()
            .ok()
            .filter(|_| readable_music_schema)
            .map(|music| music.tracks)
            .unwrap_or_default();
        let manual_title_for = |position: u16| crate::manual_music_track_title(&titles, position);
        let release = if readable_music_schema {
            self.effective_music_release(item)?
        } else {
            None
        };
        let assignments = self.music_disc_assignments(item)?;
        let unscoped_manual_titles = item.captures.len() == 1;
        let profiles = [
            ConversionProfile::Flac,
            ConversionProfile::Opus { bitrate_kbps: 96 },
            ConversionProfile::Opus { bitrate_kbps: 128 },
            ConversionProfile::Opus { bitrate_kbps: 192 },
        ];
        let mut rows = Vec::new();
        for capture in item
            .captures
            .iter()
            .filter(|capture| capture.association.item_id == Some(item_id))
        {
            let active = self.jobs.active_derivative_jobs(capture.id)?;
            for source in capture.representations.iter().filter(|representation| {
                matches!(
                    representation.role,
                    collector_core::RepresentationRole::PreservationMaster
                )
            }) {
                if let Some(manifest) = self.recorded_file_tracks(item.collection_id, source.id)? {
                    rows.extend(self.file_track_rows(item, capture, source, manifest, user_id)?);
                    continue;
                }
                let mut tracks = Vec::<ConversionTrack>::new();
                let mut source_readable = false;
                if !recorded_only
                    && readable_music_schema
                    && let Ok(selection) =
                        self.detect_music_selection(capture.collection_id, source.id)
                {
                    let host = ConversionHost {
                        port: self.processing.as_ref(),
                        access: collector_modules::PackageAccess {
                            collection_id: capture.collection_id,
                            scope: PackageScope::Representation(source.id),
                        },
                        claim: None,
                    };
                    if let Ok(pcm) = collector_modules::music::open_conversion_pcm_selection(
                        &host,
                        host.access,
                        &selection,
                    ) {
                        source_readable = true;
                        tracks.extend(pcm.tracks.into_iter().map(|track| ConversionTrack {
                            position: track.position,
                            start: PcmFrame(track.start_frame),
                            length: PcmFrame(track.frame_count),
                        }));
                    }
                }
                if recorded_only
                    && let Some(collector_modules::music::CdProcessingResult::Layout {
                        tracks: layout,
                        ..
                    }) = self.recorded_music_layout(item.collection_id, source.id)?
                {
                    for track in layout
                        .into_iter()
                        .filter(|t| t.kind == collector_core::CdTrackKind::Audio)
                    {
                        if let (Some(start), Some(length)) = (track.pcm_start, track.pcm_length) {
                            tracks.push(ConversionTrack {
                                position: u16::from(track.position),
                                start,
                                length,
                            });
                        }
                    }
                }
                let mut retained = Vec::new();
                for representation in capture.representations.iter().filter(|representation| {
                    representation.source == Some(source.id) && representation.profile.is_some()
                }) {
                    let recipe = match self.processing.recipe(RecipeId(representation.id.0)) {
                        Ok(recipe) => recipe,
                        // Imported derivatives need not have Collector-generated recipes.
                        Err(Error::NotFound) => continue,
                        Err(error) => return Err(error),
                    };
                    if recipe.tracks.len() == 1
                        && !tracks
                            .iter()
                            .any(|track| track.position == recipe.tracks[0].position)
                    {
                        tracks.push(recipe.tracks[0].clone());
                    }
                    retained.push((representation, recipe));
                }
                if recorded_only {
                    let mut positions = std::collections::BTreeSet::new();
                    for (representation, recipe) in &retained {
                        if representation.profile.as_ref() == Some(&ConversionProfile::Flac)
                            && recipe.tracks.len() == 1
                            && positions.insert(recipe.tracks[0].position)
                        {
                            tracks.retain(|v| v.position != recipe.tracks[0].position);
                            tracks.push(recipe.tracks[0].clone());
                        }
                    }
                }
                tracks.sort_by_key(|track| track.position);
                tracks.dedup_by_key(|track| track.position);
                let disc_id = if release.is_some() {
                    match self.recorded_music_layout(capture.collection_id, source.id)? {
                        Some(collector_modules::music::CdProcessingResult::Layout {
                            disc_ids: Some(ids),
                            ..
                        }) => Some(ids.musicbrainz),
                        _ => None,
                    }
                } else {
                    None
                };
                let assignment = assignments.iter().find(|value| {
                    value.source_representation_id == source.id
                        && release
                            .as_ref()
                            .is_some_and(|release| value.release_id == release.release_id)
                });
                let positions = tracks
                    .iter()
                    .map(|track| track.position)
                    .collect::<Vec<_>>();
                let medium = resolve_music_medium(
                    release.as_ref(),
                    disc_id.as_deref(),
                    assignment,
                    Some(&positions),
                )
                .0;
                for track in tracks {
                    for profile in profiles.iter().cloned() {
                        let published = retained
                            .iter()
                            .find(|(representation, recipe)| {
                                representation.profile.as_ref() == Some(&profile)
                                    && recipe.tracks.as_slice() == [track.clone()]
                            })
                            .map(|(representation, _)| *representation);
                        let encoder_ready = matches!(
                            self.encoder_availability(&profile),
                            EncoderAvailability::Available { .. }
                        );
                        let state = if let Some(representation) = published {
                            let file = representation.files.first().ok_or_else(|| {
                                Error::Storage("published derivative has no file".into())
                            })?;
                            let inventory = self.processing.package_inventory(
                                capture.collection_id,
                                PackageScope::Representation(representation.id),
                            )?;
                            let member = inventory.files.first().ok_or_else(|| {
                                Error::Storage("published derivative inventory is empty".into())
                            })?;
                            MusicTrackAvailabilityState::Available {
                                representation_id: representation.id,
                                file_id: member.id,
                                file_index: 0,
                                length: file.bytes,
                                media_type: match profile {
                                    ConversionProfile::Flac => "audio/flac",
                                    ConversionProfile::Opus { .. } => "audio/ogg; codecs=opus",
                                }
                                .into(),
                                digest: file.digest.clone(),
                            }
                        } else if active.iter().any(|job| {
                            job.input.recipe.source_fingerprint.source
                                == PackageScope::Representation(source.id)
                                && job.input.recipe.tracks.as_slice() == [track.clone()]
                                && profile_for_recipe(&job.input.recipe) == profile
                        }) {
                            MusicTrackAvailabilityState::Preparing
                        } else {
                            let reason = if recorded_only {
                                "Prepared audio is not available. Open Collector to inspect preparation.".into()
                            } else if !source_readable {
                                "The source layout or its music module is unavailable for preparation.".into()
                            } else {
                                match self.encoder_availability(&profile) {
                                    EncoderAvailability::Available { .. } => {
                                        "This track has not been prepared in this format.".into()
                                    }
                                    EncoderAvailability::Unavailable { reason } => reason,
                                }
                            };
                            MusicTrackAvailabilityState::Unavailable { reason }
                        };
                        rows.push(MusicTrackAvailability {
                            album_selection: collector_api_types::MusicAlbumSelection::Included,
                            display_position: None,
                            audio_format: None,
                            capture_id: capture.id,
                            source_label: source
                                .files
                                .first()
                                .map(|f| f.name.clone())
                                .unwrap_or_else(|| "Original".into()),
                            disc_position: medium.map(|m| m.position),
                            source_representation_id: source.id,
                            track_position: track.position,
                            start_frame: track.start,
                            frame_count: track.length,
                            title: unscoped_manual_titles
                                .then(|| manual_title_for(track.position))
                                .flatten()
                                .or_else(|| {
                                    medium.and_then(|medium| {
                                        medium
                                            .tracks
                                            .iter()
                                            .find(|named| named.position == track.position)
                                            .map(|named| named.title.clone())
                                    })
                                })
                                .unwrap_or_else(|| format!("Track {:02}", track.position)),
                            profile: Some(profile),
                            can_prepare: writable_music
                                && source_readable
                                && encoder_ready
                                && matches!(state, MusicTrackAvailabilityState::Unavailable { .. }),
                            state,
                        });
                    }
                }
            }
        }
        rows.sort_by_key(|row| {
            (
                row.capture_id,
                row.source_representation_id,
                row.track_position,
                row.profile.as_ref().map_or(0, profile_order),
            )
        });
        Ok(rows)
    }

    pub(super) fn detect_music_selection(
        &self,
        collection_id: CollectionId,
        source: RepresentationId,
    ) -> Result<collector_modules::music::CdPackageSelection> {
        let inventory = self
            .processing
            .package_inventory(collection_id, PackageScope::Representation(source))?;
        // Selection and availability are catalog reads, not integrity checks.
        // Rehashing here rereads the entire disc on every poll and once per
        // track during album admission. Workers validate actual bytes before
        // encoding and again before publication.
        let source_fingerprint = self
            .processing
            .recorded_source(collection_id, PackageScope::Representation(source))?;
        let observations = self.processing.observations(&crate::ProcessingQuery {
            collection_id,
            source: PackageScope::Representation(source),
            source_fingerprint,
            operation: Some(collector_core::OperationId::new(
                "org.junk.music.import-analysis",
            )?),
            after: None,
            limit: 1,
        })?;
        if let Some(selection) = observations
            .items
            .first()
            .and_then(|observation| {
                observation
                    .typed_result
                    .payload
                    .parse::<collector_modules::music::MusicImportAnalysisOutput>()
                    .ok()
            })
            .and_then(|output| output.selection)
        {
            return Ok(selection);
        }
        collector_modules::music::detect_package(&inventory).map_err(Into::into)
    }

    pub(crate) fn encoder_availability(&self, profile: &ConversionProfile) -> EncoderAvailability {
        self.audio_converter.as_ref().map_or_else(
            || EncoderAvailability::Unavailable {
                reason: match profile {
                    ConversionProfile::Flac => "FLAC encoder is not configured.",
                    ConversionProfile::Opus { .. } => "Opus encoder is not configured.",
                }
                .into(),
            },
            |converter| converter.configured_availability(profile),
        )
    }

    pub(crate) fn run_music_derivative(&self, claim: &JobClaim) -> Result<serde_json::Value> {
        let Some(FrozenJobInput::Derivative(frozen)) = claim.frozen_input.as_ref() else {
            return Err(Error::Storage("derivative job has no frozen input".into()));
        };
        frozen.validate()?;
        if claim.collection_id != Some(frozen.collection_id) || claim.item_id.is_some() {
            return Err(Error::Conflict);
        }
        let output = serde_json::json!({"representation_id":frozen.representation_id,"recipe_id":frozen.recipe.id});
        if self.processing.derivative_committed(frozen)? {
            return Ok(output);
        }
        let checkpoint = || {
            if self
                .jobs
                .cancellation_requested(claim.id, &claim.claim_token)?
            {
                return Err(Error::JobCancelled);
            }
            let current_capture = self.reader.capture(frozen.capture_id)?;
            if current_capture.collection_id != frozen.collection_id {
                return Err(Error::Conflict);
            }
            let eligibility = self.content_write_eligibility_for_job(
                claim.user_id,
                Some(frozen.collection_id),
                current_capture.association.item_id,
            )?;
            self.validate_frozen_binding(&eligibility, &frozen.collection_binding)?;
            if self.processing.recorded_source(
                frozen.collection_id,
                frozen.recipe.source_fingerprint.source,
            )? != frozen.recipe.source_fingerprint
            {
                return Err(Error::Conflict);
            }
            Ok(())
        };
        checkpoint()?;
        let inventory = self.processing.package_inventory(
            frozen.collection_id,
            frozen.recipe.source_fingerprint.source,
        )?;
        if self.fingerprint_inventory_checked(&inventory, &mut || checkpoint())?
            != frozen.recipe.source_fingerprint
        {
            return Err(Error::Conflict);
        }
        let host = ConversionHost {
            port: self.processing.as_ref(),
            access: collector_modules::PackageAccess {
                collection_id: frozen.collection_id,
                scope: frozen.recipe.source_fingerprint.source,
            },
            claim: Some((self.jobs.as_ref(), claim)),
        };
        let pcm = collector_modules::music::open_conversion_pcm_selection(
            &host,
            host.access,
            &frozen.selection,
        );
        checkpoint()?;
        let mut pcm = pcm?;
        let expected = build_music_conversion_recipe(
            frozen.recipe.id,
            frozen.recipe.source_fingerprint.clone(),
            &pcm.tracks,
            &[frozen.recipe.tracks[0].position],
            match frozen.recipe.settings {
                ConversionSettings::Flac { .. } => ConversionProfile::Flac,
                ConversionSettings::Opus { bitrate_kbps, .. } => {
                    ConversionProfile::Opus { bitrate_kbps }
                }
            },
            frozen.recipe.encoder_version.clone(),
        )?;
        if expected.digest != frozen.recipe.digest {
            return Err(Error::Conflict);
        }
        let converter = self
            .audio_converter
            .as_ref()
            .ok_or_else(|| Error::Unavailable("audio encoder is not configured".into()))?;
        let mut execution = crate::DurableProcessingContext {
            progress_base: 0,
            frozen: FrozenJobInput::Derivative(frozen.clone()),
            jobs: self.jobs.clone(),
            id: claim.id,
            claim_token: claim.claim_token.clone(),
        };
        let mut scratch = self.processing.conversion_scratch()?;
        let validation = converter.convert(
            &frozen.recipe,
            pcm.reader.as_mut(),
            scratch.as_mut(),
            &mut execution,
        );
        checkpoint()?;
        let validation = validation?;
        validation.validate_recipe(&frozen.recipe)?;
        if self.fingerprint_inventory_checked(&inventory, &mut || checkpoint())?
            != frozen.recipe.source_fingerprint
        {
            return Err(Error::Conflict);
        }
        self.processing.publish_derivative(
            claim,
            frozen,
            &validation,
            scratch.as_mut(),
            &mut || checkpoint(),
        )?;
        Ok(output)
    }
}

pub(super) fn profile_for_recipe(recipe: &ConversionRecipe) -> ConversionProfile {
    match recipe.settings {
        ConversionSettings::Flac { .. } => ConversionProfile::Flac,
        ConversionSettings::Opus { bitrate_kbps, .. } => ConversionProfile::Opus { bitrate_kbps },
    }
}

#[cfg(test)]
fn matching_musicbrainz_medium<'a>(
    release: &'a collector_modules::music::MusicBrainzReleaseObservation,
    disc_id: Option<&str>,
    tracks: &[ConversionTrack],
) -> Option<&'a collector_modules::music::MusicBrainzMediumObservation> {
    let positions = tracks
        .iter()
        .map(|track| track.position)
        .collect::<Vec<_>>();
    resolve_music_medium(Some(release), disc_id, None, Some(&positions)).0
}

fn resolve_music_medium<'a>(
    release: Option<&'a collector_modules::music::MusicBrainzReleaseObservation>,
    disc_id: Option<&str>,
    assignment: Option<&collector_modules::music::MusicDiscAssignment>,
    positions: Option<&[u16]>,
) -> (
    Option<&'a collector_modules::music::MusicBrainzMediumObservation>,
    MusicDiscMatchState,
) {
    let Some(release) = release else {
        return (None, MusicDiscMatchState::NoRelease);
    };
    let Some(positions) = positions.filter(|positions| !positions.is_empty()) else {
        return (None, MusicDiscMatchState::NoLayout);
    };
    let (medium, state) = if let Some(assignment) = assignment {
        let Some(medium) = release
            .media
            .iter()
            .find(|medium| medium.position == assignment.medium_position)
        else {
            return (None, MusicDiscMatchState::NeedsAssignment);
        };
        if disc_id.is_some_and(|id| !medium.disc_ids.iter().any(|value| value == id)) {
            return (None, MusicDiscMatchState::AssignmentConflictsWithDiscId);
        }
        (medium, MusicDiscMatchState::CuratorAssigned)
    } else if let Some(id) = disc_id {
        let mut matches = release
            .media
            .iter()
            .filter(|medium| medium.disc_ids.iter().any(|value| value == id));
        let Some(medium) = matches.next() else {
            return (None, MusicDiscMatchState::DiscIdNotListed);
        };
        if matches.next().is_some() {
            return (None, MusicDiscMatchState::AmbiguousDiscId);
        }
        (medium, MusicDiscMatchState::DiscIdMatched)
    } else if release.media.len() == 1 {
        (&release.media[0], MusicDiscMatchState::SingleMedium)
    } else {
        return (None, MusicDiscMatchState::NeedsAssignment);
    };
    if medium.tracks.len() != positions.len()
        || positions.iter().any(|position| {
            !medium
                .tracks
                .iter()
                .any(|track| track.position == *position)
        })
    {
        return (None, MusicDiscMatchState::TrackLayoutMismatch);
    }
    (Some(medium), state)
}

fn profile_order(profile: &ConversionProfile) -> u8 {
    match profile {
        ConversionProfile::Flac => 0,
        ConversionProfile::Opus { bitrate_kbps: 96 } => 1,
        ConversionProfile::Opus { bitrate_kbps: 128 } => 2,
        ConversionProfile::Opus { bitrate_kbps: 192 } => 3,
        ConversionProfile::Opus { .. } => 4,
    }
}

fn derivative_identity(capture: CaptureId, digest: &Digest) -> uuid::Uuid {
    use sha2::{Digest as _, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(b"collector music derivative identity v1");
    hasher.update(capture.0.as_bytes());
    hasher.update(digest.hex.as_bytes());
    let mut bytes: [u8; 16] = hasher.finalize()[..16].try_into().expect("SHA-256 prefix");
    bytes[6] = (bytes[6] & 0x0f) | 0x50;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    uuid::Uuid::from_bytes(bytes)
}
