//! Metadata authority and deterministic display resolution. Provider transport
//! belongs in adapters; this module consumes already-bounded observations.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;

use chrono::Utc;
use collector_core::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

use crate::{
    AssetState, CollectionService, CommandContext, CreateAssetCommand, FrozenAcquisitionJob,
    FrozenMetadataResolution, JobClaim, MetadataAuthorityRecord, Principal, ProcessingPort,
    ProviderRequest, SnapshotQuery, deterministic_inventory_uuid,
};

/// Ephemeral provider thumbnail; never an archived Asset or verification evidence.
pub struct ArtworkPreview {
    pub bytes: Vec<u8>,
    pub media_type: String,
}
pub const MAX_ARTWORK_PREVIEW_BYTES: u64 = 1024 * 1024;
pub trait ArtworkPreviewPort: Send + Sync {
    fn preview(
        &self,
        release_id: &str,
        image_id: &str,
        cancelled: &dyn Fn() -> Result<bool>,
    ) -> Result<ArtworkPreview>;
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(tag = "change", rename_all = "snake_case", deny_unknown_fields)]
pub enum MetadataAuthorityChange {
    SelectCandidate {
        candidate: MetadataCandidateIdentity,
    },
    ClearSelectedCandidate,
    SetFieldOverride {
        field: MetadataField,
        value: String,
        language: Option<LanguageTag>,
    },
    ClearFieldOverride {
        field: MetadataField,
    },
    SetArtworkChoice {
        asset_id: AssetId,
    },
    ClearArtworkChoice,
    SetDomainOverrides {
        #[serde(deserialize_with = "deserialize_domain_override")]
        overrides: ModuleData,
    },
    ClearDomainOverrides,
}

// Internally tagged commands buffer their contents before variant decoding. Decode
// this submitted override through Value because serde's buffer cannot supply the
// RawValue token; immutable archived ModuleData still retains its raw JSON.
fn deserialize_domain_override<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<ModuleData, D::Error> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct SubmittedOverride {
        module: ModuleId,
        schema_version: u32,
        payload: serde_json::Value,
    }
    let value = SubmittedOverride::deserialize(deserializer)?;
    Ok(ModuleData {
        module: value.module,
        schema_version: value.schema_version,
        payload: RawJson::from_value(value.payload),
    })
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct UpdateMetadataAuthorityCommand {
    pub expected_revision: Revision,
    pub change: MetadataAuthorityChange,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RefreshMetadataCommand {
    pub expected_revision: Revision,
    pub observation: MetadataRefreshObservation,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ProviderArtworkLink {
    pub provider_record_id: String,
    pub asset_id: AssetId,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RefreshProviderMetadataCommand {
    pub expected_revision: Revision,
    pub snapshot_id: SnapshotId,
    /// Required for DAT-backed metadata so descriptive rows remain bound to the
    /// exact D04 catalog result that selected them.
    pub source_observation_id: Option<ObservationId>,
    #[serde(default)]
    pub artwork: Vec<ProviderArtworkLink>,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PublishProviderArtworkCommand {
    pub snapshot_id: SnapshotId,
}

#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
pub struct MetadataView {
    pub item_id: ItemId,
    pub revision: Revision,
    pub authority: MetadataAuthority,
    pub display: ResolvedMetadataDisplay,
    pub candidates: Vec<MetadataCandidateObservation>,
    pub current_candidates: Vec<MetadataCandidateIdentity>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum MetadataProviderAvailability {
    Available,
    Unconfigured,
    Inapplicable,
}

#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
pub struct MetadataProviderView {
    pub id: ModuleId,
    pub label: String,
    pub availability: MetadataProviderAvailability,
    pub can_acquire: bool,
    pub guidance: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum MetadataLookupKind {
    ReleaseId,
    DiscId,
    Search,
}

#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
pub struct MetadataLookupChoice {
    pub id: String,
    pub kind: MetadataLookupKind,
    pub label: String,
    pub value: String,
    pub recommended: bool,
}

#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
pub struct MetadataCandidateView {
    pub identity: MetadataCandidateIdentity,
    pub title: MetadataText,
    pub subtitle: Option<MetadataText>,
    pub artwork: Option<AssetId>,
    pub complete_identity: bool,
    pub strength: MatchStrength,
    pub current: bool,
    pub selected: bool,
    pub accepted_automatically: bool,
    pub music_layout: Option<MetadataMusicLayout>,
    pub unmatched: bool,
    pub provider_priority: u16,
    #[schema(value_type = Object)]
    pub details: Option<serde_json::Value>,
}

#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
pub struct MetadataMusicLayout {
    pub release_id: String,
    pub album_id: Option<String>,
    pub artist: Option<String>,
    pub edition: Option<String>,
    pub discs: Vec<MetadataMediumLayout>,
}
#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
pub struct MetadataMediumLayout {
    pub position: u16,
    pub disc_ids: Vec<String>,
    pub tracks: Vec<MetadataTrackTitle>,
}
#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
pub struct MetadataTrackTitle {
    pub position: u16,
    pub title: String,
}
fn music_layout(candidate: &MetadataCandidateObservation) -> Option<MetadataMusicLayout> {
    if candidate.identity.provider.0 != collector_modules::music::MUSICBRAINZ_PROVIDER_ID {
        return None;
    }
    let release: collector_modules::music::MusicBrainzReleaseObservation =
        candidate.observation.parse().ok()?;
    if release.validate().is_err() || release.release_id != candidate.identity.provider_record_id {
        return None;
    }
    Some(MetadataMusicLayout {
        release_id: release.release_id,
        album_id: release.release_group_id,
        artist: release.artist_credit,
        edition: release.disambiguation,
        discs: release
            .media
            .into_iter()
            .map(|medium| MetadataMediumLayout {
                position: medium.position,
                disc_ids: medium.disc_ids,
                tracks: medium
                    .tracks
                    .into_iter()
                    .map(|track| MetadataTrackTitle {
                        position: track.position,
                        title: track.title,
                    })
                    .collect(),
            })
            .collect(),
    })
}

#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
pub struct MetadataCatalogSourceView {
    pub observation_id: ObservationId,
    pub snapshot_id: SnapshotId,
    pub label: String,
    pub current: bool,
}

#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
pub struct MetadataPermissions {
    pub can_update: bool,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum MetadataReviewReason {
    MissingName,
    MissingArtwork,
    AlbumAmbiguity,
    IncompleteProviderInformation,
    NoMatch,
    LookupFailed,
    ManualConflict,
    GroupingProposal,
}

#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
pub struct MetadataReviewView {
    pub item_id: ItemId,
    pub revision: Revision,
    pub authority: MetadataAuthority,
    pub display: ResolvedMetadataDisplay,
    pub candidates: Vec<MetadataCandidateView>,
    pub review_reasons: Vec<MetadataReviewReason>,
    pub proposed_candidate: Option<MetadataCandidateIdentity>,
    pub preferred_language: LanguageTag,
    pub providers: Vec<MetadataProviderView>,
    pub lookup_choices: Vec<MetadataLookupChoice>,
    pub catalog_sources: Vec<MetadataCatalogSourceView>,
    pub permissions: MetadataPermissions,
    pub terms: collector_modules::MetadataTerms,
    pub recognized_systems: Vec<collector_modules::games::StandardGameSystem>,
    pub identification: Option<collector_modules::games::GameIdentification>,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(tag = "source", rename_all = "snake_case", deny_unknown_fields)]
pub enum AcquireMetadataCommand {
    MusicBrainzRelease {
        release_id: String,
        #[serde(default)]
        refresh: bool,
    },
    MusicBrainzDisc {
        disc_id: String,
        #[serde(default)]
        refresh: bool,
    },
    MusicBrainzSearch {
        query: String,
        #[serde(default)]
        refresh: bool,
    },
    CoverArtListing {
        candidate: MetadataCandidateIdentity,
    },
    CoverArtImage {
        candidate: MetadataCandidateIdentity,
        image_id: String,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ApplyMetadataSnapshotCommand {
    pub expected_revision: Revision,
    pub snapshot_id: SnapshotId,
    pub source_observation_id: Option<ObservationId>,
    #[serde(default)]
    pub artwork: Vec<ProviderArtworkLink>,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct MetadataArtworkOption {
    pub snapshot_id: SnapshotId,
    pub release_id: String,
    pub image_id: String,
    pub front: bool,
    pub back: bool,
    pub approved: bool,
    pub types: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CompleteMetadataAcquisitionCommand {
    pub job_id: JobId,
    pub expected_revision: Revision,
}

#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CompletedMetadataAcquisition {
    Metadata {
        review: Box<MetadataReviewView>,
        outcome: MetadataRefreshOutcome,
    },
    ArtworkOptions {
        options: Vec<MetadataArtworkOption>,
    },
    ArtworkPublished {
        asset: Box<crate::PresentedAsset>,
    },
}

struct LoadedMetadataAuthority {
    revision: Revision,
    authority: MetadataAuthority,
    legacy: Vec<MetadataRefreshObservation>,
    current: Option<Vec<MetadataCandidateIdentity>>,
}

impl CollectionService {
    pub(crate) fn effective_metadata_for_user(
        &self,
        item: &Item,
        user: UserId,
    ) -> Result<ResolvedMetadataDisplay> {
        let LoadedMetadataAuthority {
            authority,
            legacy,
            current,
            ..
        } = self.authority_or_inventory(item)?;
        let mut candidates = self
            .processing
            .metadata_candidates(item.collection_id, item.id)?;
        merge_metadata_candidates(
            &mut candidates,
            legacy.iter().flat_map(|value| &value.candidates),
        )?;
        resolve_metadata_display(
            item,
            &authority,
            &current_metadata_candidates(&authority, &candidates, current.as_deref())?,
            &self.metadata_language(user)?,
        )
    }

    pub fn enqueue_musicbrainz_metadata_acquisition(
        &self,
        actor: &Principal,
        item_id: ItemId,
        query: collector_modules::music::MusicBrainzQuery,
        context: &CommandContext,
    ) -> Result<JobId> {
        query.validate()?;
        let item = self.reader.item(item_id)?;
        let eligibility = self.content_write_eligibility(actor, item.collection_id)?;
        if item.deleted || eligibility.owner.0 != "org.junk.music" {
            return Err(Error::NotFound);
        }
        self.validate_owned_domain(&eligibility, &item.domain)?;
        let job = crate::deterministic_job_id(
            actor.user_id,
            "metadata.musicbrainz.acquire",
            context.key(),
        );
        self.execute_command(
            actor,
            context,
            "metadata.musicbrainz.acquire",
            &(item_id, &query),
            |effects| {
        if !self
            .streaming_providers
            .contains_key(collector_modules::music::MUSICBRAINZ_PROVIDER_ID)
            || !self
                .catalog_parsers
                .contains_key(collector_modules::music::MUSICBRAINZ_PROVIDER_ID)
        {
            return Err(Error::Unavailable(
                "MusicBrainz is not configured. Add providers.musicbrainz_user_agent to the server configuration and restart the server."
                    .into(),
            ));
        }

                let frozen = FrozenAcquisitionJob {
                    request: ProviderRequest {
                        collection_id: item.collection_id,
                        provider: ModuleId(
                            collector_modules::music::MUSICBRAINZ_PROVIDER_ID.into(),
                        ),
                        operation: OperationId::new(
                            collector_modules::music::MUSICBRAINZ_ACQUIRE_OPERATION,
                        )?,
                        source_fingerprint: None,
                        query: RawJson::from_value(
                            serde_json::to_value(&query)
                                .map_err(|error| Error::Invalid(error.to_string()))?,
                        ),
                        maximum_response_bytes: 8 * 1024 * 1024,
                    },
                    collection_binding: eligibility.binding,
                    item_id: Some(item_id),
                    snapshot_id: SnapshotId(job.0),
                    created_at: self.authentication.now(),
                };
                let spec = self.catalog_acquisition_spec(actor, &frozen, job)?;
                effects.enqueue_job(spec)
            },
        )
    }

    pub fn enqueue_cover_art_acquisition(
        &self,
        actor: &Principal,
        item_id: ItemId,
        query: collector_modules::music::CoverArtArchiveQuery,
        context: &CommandContext,
    ) -> Result<JobId> {
        query.validate()?;
        let item = self.reader.item(item_id)?;
        let eligibility = self.content_write_eligibility(actor, item.collection_id)?;
        if item.deleted || eligibility.owner.0 != "org.junk.music" {
            return Err(Error::NotFound);
        }
        self.validate_owned_domain(&eligibility, &item.domain)?;
        let maximum_response_bytes = if matches!(
            query,
            collector_modules::music::CoverArtArchiveQuery::Listing { .. }
        ) {
            8 * 1024 * 1024
        } else {
            20 * 1024 * 1024
        };
        let job =
            crate::deterministic_job_id(actor.user_id, "metadata.cover-art.acquire", context.key());
        self.execute_command(
            actor,
            context,
            "metadata.cover-art.acquire",
            &(item_id, &query),
            |effects| {
        if !self
            .streaming_providers
            .contains_key(collector_modules::music::COVER_ART_ARCHIVE_PROVIDER_ID)
            || !self
                .catalog_parsers
                .contains_key(collector_modules::music::COVER_ART_ARCHIVE_PROVIDER_ID)
        {
            return Err(Error::Unavailable(
                "Cover Art Archive is not configured. Configure MusicBrainz provider identification and restart the server."
                    .into(),
            ));
        }

                let frozen = FrozenAcquisitionJob {
                    request: ProviderRequest {
                        collection_id: item.collection_id,
                        provider: ModuleId(
                            collector_modules::music::COVER_ART_ARCHIVE_PROVIDER_ID.into(),
                        ),
                        operation: OperationId::new(
                            collector_modules::music::COVER_ART_ARCHIVE_ACQUIRE_OPERATION,
                        )?,
                        source_fingerprint: None,
                        query: RawJson::from_value(
                            serde_json::to_value(&query)
                                .map_err(|error| Error::Invalid(error.to_string()))?,
                        ),
                        maximum_response_bytes,
                    },
                    collection_binding: eligibility.binding,
                    item_id: Some(item_id),
                    snapshot_id: SnapshotId(job.0),
                    created_at: self.authentication.now(),
                };
                let spec = self.catalog_acquisition_spec(actor, &frozen, job)?;
                effects.enqueue_job(spec)
            },
        )
    }

    /// Publish a previously acquired CAA image through the existing authorized
    /// Asset command effect. Metadata candidates can cite the returned active
    /// item Asset only in a later refresh command.
    pub fn publish_provider_artwork(
        &self,
        actor: &Principal,
        item_id: ItemId,
        command: PublishProviderArtworkCommand,
        context: &CommandContext,
    ) -> Result<AssetState> {
        let item = self.reader.item(item_id)?;
        let eligibility = self.content_write_eligibility(actor, item.collection_id)?;
        if item.deleted || eligibility.owner.0 != "org.junk.music" {
            return Err(Error::NotFound);
        }
        self.validate_owned_domain(&eligibility, &item.domain)?;
        let asset_id = crate::deterministic_new_asset_id(actor.user_id, context.key());
        let request = (item_id, &command);
        self.execute_command(
            actor,
            context,
            "metadata.artwork.publish",
            &request,
            |effects| {
                self.processing.ensure_snapshot_current(item_id, command.snapshot_id)?;
                let snapshot = self.processing.snapshot(SnapshotQuery {
                    collection_id: item.collection_id,
                    snapshot_id: command.snapshot_id,
                })?;
                if snapshot.provider.0
                    != collector_modules::music::COVER_ART_ARCHIVE_PROVIDER_ID
                    || snapshot.bytes.0 > 20 * 1024 * 1024
                {
                    return Err(Error::Invalid(
                        "metadata artwork requires a bounded Cover Art Archive image snapshot"
                            .into(),
                    ));
                }
                let observation: collector_modules::music::CoverArtArchiveSnapshotObservation =
                    snapshot.parsed_observation.parse()?;
                observation.validate()?;
                let (release_id, image_id) = match &observation.query {
                    collector_modules::music::CoverArtArchiveQuery::Image {
                        release_id,
                        image_id,
                    } if matches!(
                        observation.outcome,
                        collector_modules::music::MetadataProviderOutcome::Candidates
                    ) => (release_id, image_id),
                    _ => {
                        return Err(Error::Invalid(
                            "Cover Art Archive snapshot contains no acquired image".into(),
                        ));
                    }
                };
                if observation.media_type.as_deref() != Some(snapshot.media_type.as_str()) {
                    return Err(Error::Invalid(
                        "Cover Art Archive image type disagrees with its observation".into(),
                    ));
                }
                let bytes = read_exact_snapshot_bytes(
                    self.processing.as_ref(),
                    &snapshot,
                    20 * 1024 * 1024,
                )?;
                effects.create_asset(
                    asset_id,
                    CreateAssetCommand {
                        collection_id: item.collection_id,
                        subject: AssetSubject::Item { item_id },
                        kind: AssetKind::Cover,
                        visibility: Visibility::Shared,
                        alt_text: format!(
                            "Cover Art Archive image {image_id} for MusicBrainz release {release_id}"
                        ),
                    },
                    &bytes,
                    crate::AssetWriteContext::Item(crate::ItemEffectContext { item_id, generation: item.lifecycle.generation, source: crate::ItemEffectSource::Snapshot(command.snapshot_id) }),
                )
            },
        )
    }

    /// Resolve an immutable acquired snapshot into a fresh item-bound observation,
    /// then share the existing revisioned metadata refresh publication effect.
    pub fn refresh_provider_metadata(
        &self,
        actor: &Principal,
        item_id: ItemId,
        command: RefreshProviderMetadataCommand,
        context: &CommandContext,
    ) -> Result<MetadataAuthorityRecord> {
        if command.artwork.len() > 128 {
            return Err(Error::Invalid("too many provider artwork links".into()));
        }
        let mut linked = BTreeSet::new();
        if command.artwork.iter().any(|value| {
            value.provider_record_id.is_empty()
                || value.provider_record_id.len() > 1024
                || value.provider_record_id.chars().any(char::is_control)
                || !linked.insert(value.provider_record_id.clone())
        }) {
            return Err(Error::Invalid("invalid provider artwork links".into()));
        }
        let item = self.reader.item(item_id)?;
        let eligibility = self.content_write_eligibility(actor, item.collection_id)?;
        if item.deleted {
            return Err(Error::NotFound);
        }
        self.validate_owned_domain(&eligibility, &item.domain)?;
        let request = (item_id, &command);
        self.execute_command(
            actor,
            context,
            "metadata.refresh.provider",
            &request,
            |effects| {
                self.processing
                    .ensure_snapshot_current(item_id, command.snapshot_id)?;
                let snapshot = self.processing.snapshot(SnapshotQuery {
                    collection_id: item.collection_id,
                    snapshot_id: command.snapshot_id,
                })?;
                let observation = self.provider_metadata_observation(
                    &item,
                    &eligibility.owner,
                    &command,
                    provider_observation_id(
                        actor.user_id,
                        item_id,
                        &snapshot.provider,
                        context.key(),
                    ),
                )?;
                let refresh = RefreshMetadataCommand {
                    expected_revision: command.expected_revision,
                    observation,
                };
                self.refresh_metadata_effect(
                    &item,
                    &eligibility.owner,
                    &refresh,
                    &self.metadata_language(actor.user_id)?,
                    None,
                    effects,
                )
            },
        )
    }

    fn provider_metadata_observation(
        &self,
        item: &Item,
        owner: &ModuleId,
        command: &RefreshProviderMetadataCommand,
        observation_id: ObservationId,
    ) -> Result<MetadataRefreshObservation> {
        self.provider_metadata_observation_controlled(
            item,
            owner,
            command,
            observation_id,
            &mut || Ok(false),
        )
    }

    fn provider_metadata_observation_controlled(
        &self,
        item: &Item,
        owner: &ModuleId,
        command: &RefreshProviderMetadataCommand,
        observation_id: ObservationId,
        cancelled: &mut dyn FnMut() -> Result<bool>,
    ) -> Result<MetadataRefreshObservation> {
        let snapshot_id = command.snapshot_id;
        let source_observation_id = command.source_observation_id;
        let artwork_links = &command.artwork;
        let snapshot = self.processing.snapshot(SnapshotQuery {
            collection_id: item.collection_id,
            snapshot_id,
        })?;
        let parser = self
            .catalog_parsers
            .get(&snapshot.provider.0)
            .ok_or_else(|| Error::Unavailable("metadata provider parser is unavailable".into()))?;
        let bytes = read_exact_snapshot_bytes_controlled(
            self.processing.as_ref(),
            &snapshot,
            if owner.0 == "org.junk.games" {
                512 * 1024 * 1024
            } else {
                8 * 1024 * 1024
            },
            cancelled,
        )?;
        let parsed =
            parser.parse_snapshot(&snapshot, &mut std::io::Cursor::new(&bytes), cancelled)?;
        let artwork = artwork_links
            .iter()
            .map(|value| (value.provider_record_id.clone(), value.asset_id))
            .collect::<BTreeMap<_, _>>();
        let (candidates, raw_observation) = if owner.0 == "org.junk.music"
            && snapshot.provider.0 == collector_modules::music::MUSICBRAINZ_PROVIDER_ID
        {
            if source_observation_id.is_some() {
                return Err(Error::Invalid(
                    "MusicBrainz metadata does not accept a DAT source observation".into(),
                ));
            }
            let observation: collector_modules::music::MusicBrainzSnapshotObservation =
                parsed.observation.parse()?;
            let mut candidates = collector_modules::music::musicbrainz_metadata_candidates(
                &snapshot.digest,
                &observation,
                &parsed.entries,
                &artwork,
            )?;
            if let collector_modules::music::MusicBrainzQuery::DiscId { disc_id, .. } =
                &observation.query
                && !self.has_computed_disc_id(item, disc_id)?
            {
                for candidate in &mut candidates {
                    candidate.complete_identity = false;
                    candidate.strength = MatchStrength::Name;
                    candidate.identity.provider_record_version = Some(format!(
                        "sha256:{}",
                        Digest::sha256(
                            &serde_json::to_vec(candidate)
                                .map_err(|e| Error::Storage(e.to_string()))?
                        )
                        .hex
                    ));
                }
            }
            let raw = json_or_snapshot_reference(&snapshot, bytes);
            (candidates, raw)
        } else if owner.0 == "org.junk.games"
            && snapshot.provider.0 == "org.junk.providers.local-dat"
        {
            if !artwork.is_empty() {
                return Err(Error::Invalid(
                    "the configured DAT provider does not supply artwork".into(),
                ));
            }
            let source_id = source_observation_id.ok_or_else(|| {
                Error::Invalid("DAT metadata requires its D04 catalog observation".into())
            })?;
            let source = self.processing.processing_record(source_id)?.observation;
            if source.collection_id != item.collection_id
                || source.snapshot_id != Some(snapshot.id)
                || source.typed_result.module != *owner
            {
                return Err(Error::Invalid(
                    "DAT metadata source does not match this item and snapshot".into(),
                ));
            }
            let representation = match source.source_fingerprint.source {
                PackageScope::Representation(id) => id,
                _ => {
                    return Err(Error::Invalid(
                        "DAT metadata requires committed Representation evidence".into(),
                    ));
                }
            };
            if !item.captures.iter().any(|capture| {
                capture
                    .representations
                    .iter()
                    .any(|value| value.id == representation)
            }) {
                return Err(Error::Invalid(
                    "DAT metadata evidence is not associated with this item".into(),
                ));
            }
            if self
                .processing
                .recorded_source(item.collection_id, source.source_fingerprint.source)?
                != source.source_fingerprint
            {
                return Err(Error::Conflict);
            }
            let catalog = if source.operation.as_str() == "org.junk.games.catalog-verification" {
                match source
                    .typed_result
                    .payload
                    .parse::<collector_modules::games::GameProcessingResult>()?
                {
                    collector_modules::games::GameProcessingResult::CatalogMatch { catalog } => {
                        catalog
                    }
                    _ => {
                        return Err(Error::Invalid(
                            "DAT metadata requires a catalog match result".into(),
                        ));
                    }
                }
            } else if source.operation.as_str() == "org.junk.games.import-analysis" {
                let value: collector_modules::games::GameImportAnalysisOutput =
                    source.typed_result.payload.parse()?;
                let raw = value.observations.get(1).ok_or_else(|| {
                    Error::Invalid("DAT Import analysis contains no catalog result".into())
                })?;
                match raw.parse::<collector_modules::games::GameProcessingResult>()? {
                    collector_modules::games::GameProcessingResult::CatalogMatch { catalog } => {
                        catalog
                    }
                    _ => {
                        return Err(Error::Invalid(
                            "DAT Import analysis has no catalog match".into(),
                        ));
                    }
                }
            } else {
                return Err(Error::Invalid(
                    "DAT metadata requires catalog or Import analysis evidence".into(),
                ));
            };
            let candidates = collector_modules::games::dat_metadata_candidates(
                snapshot.provider.clone(),
                &snapshot.digest,
                &parsed.entries,
                &catalog,
            )?;
            let raw = RawJson::from_value(serde_json::json!({
                "provider":"org.junk.providers.local-dat",
                "snapshot_id":snapshot.id,
                "snapshot_digest":snapshot.digest,
                "source_observation_id":source_id
            }));
            (candidates, raw)
        } else {
            return Err(Error::Invalid(
                "metadata provider is incompatible with the collection owner".into(),
            ));
        };
        let candidate_ids = candidates
            .iter()
            .map(|candidate| candidate.identity.provider_record_id.as_str())
            .collect::<BTreeSet<_>>();
        if artwork
            .keys()
            .any(|id| !candidate_ids.contains(id.as_str()))
        {
            return Err(Error::Invalid(
                "metadata artwork link does not identify an observed candidate".into(),
            ));
        }
        Ok(MetadataRefreshObservation {
            id: observation_id,
            outcome: if candidates.is_empty() {
                MetadataRefreshOutcome::NoMatch
            } else {
                MetadataRefreshOutcome::Candidates
            },
            candidates,
            raw_observation,
            observed_at: self.authentication.now(),
        })
    }

    /// Curator detail. Raw candidate observations are intentionally not part of
    /// the viewer item DTO.
    pub fn metadata_view(&self, actor: &Principal, item_id: ItemId) -> Result<MetadataView> {
        let item = self.reader.item(item_id)?;
        self.authorize(actor, item.collection_id, Action::Curate)?;
        let LoadedMetadataAuthority {
            revision,
            authority,
            legacy,
            current,
        } = self.authority_or_inventory(&item)?;
        let mut candidates = self
            .processing
            .metadata_candidates(item.collection_id, item.id)?;
        merge_metadata_candidates(
            &mut candidates,
            legacy.iter().flat_map(|value| &value.candidates),
        )?;
        let display = resolve_metadata_display(
            &item,
            &authority,
            &current_metadata_candidates(&authority, &candidates, current.as_deref())?,
            &self.metadata_language(actor.user_id)?,
        )?;
        let current_candidates = current.unwrap_or_else(|| {
            candidates
                .iter()
                .map(|candidate| candidate.identity.clone())
                .collect()
        });
        Ok(MetadataView {
            item_id,
            revision,
            authority,
            display,
            candidates,
            current_candidates,
        })
    }

    pub(crate) fn has_computed_disc_id(&self, item: &Item, disc_id: &str) -> Result<bool> {
        for representation in item
            .captures
            .iter()
            .flat_map(|capture| &capture.representations)
        {
            let source = PackageScope::Representation(representation.id);
            let fingerprint = self
                .processing
                .recorded_source(item.collection_id, source)?;
            let mut after = None;
            loop {
                let page = self.processing.observations(&crate::ProcessingQuery {
                    collection_id: item.collection_id,
                    source,
                    source_fingerprint: fingerprint.clone(),
                    operation: None,
                    after,
                    limit: 100,
                })?;
                for observation in &page.items {
                    if observation.source_fingerprint == fingerprint
                        && let Some(collector_modules::music::CdProcessingResult::Layout {
                            disc_ids: Some(ids),
                            ..
                        }) = recorded_music_layout(observation)?
                        && ids.musicbrainz == disc_id
                    {
                        return Ok(true);
                    }
                }
                let Some(cursor) = page.next_cursor else {
                    break;
                };
                after = Some(
                    cursor
                        .parse()
                        .map_err(|_| Error::Storage("invalid layout cursor".into()))?,
                );
            }
        }
        Ok(false)
    }

    pub(crate) fn effective_music_release(
        &self,
        item: &Item,
    ) -> Result<Option<collector_modules::music::MusicBrainzReleaseObservation>> {
        let LoadedMetadataAuthority {
            authority,
            legacy,
            current,
            ..
        } = self.authority_or_inventory(item)?;
        let mut candidates = self
            .processing
            .metadata_candidates(item.collection_id, item.id)?;
        merge_metadata_candidates(
            &mut candidates,
            legacy.iter().flat_map(|value| &value.candidates),
        )?;
        let current = current_metadata_candidates(&authority, &candidates, current.as_deref())?;
        let display = resolve_metadata_display(
            item,
            &authority,
            &current,
            authority
                .automatic_language
                .as_ref()
                .map_or("en", LanguageTag::as_str),
        )?;
        if display.resolution == MetadataResolutionKind::Tentative {
            return Ok(None);
        }
        let Some(identity) = display.candidate else {
            return Ok(None);
        };
        if identity.provider.0 != collector_modules::music::MUSICBRAINZ_PROVIDER_ID {
            return Ok(None);
        }
        current
            .iter()
            .find(|candidate| {
                candidate.identity == identity
                    && (candidate.complete_identity
                        || authority.selected_candidate.as_ref() == Some(&identity))
            })
            .map(|candidate| candidate.observation.parse())
            .transpose()
    }

    pub(crate) fn music_disc_assignments(
        &self,
        item: &Item,
    ) -> Result<Vec<collector_modules::music::MusicDiscAssignment>> {
        let authority = self.authority_or_inventory(item)?.authority;
        Ok(authority
            .domain_overrides
            .and_then(|value| {
                value
                    .payload
                    .parse::<collector_modules::music::MusicMetadataOverrides>()
                    .ok()
            })
            .map(|value| value.disc_assignments)
            .unwrap_or_default())
    }

    pub fn metadata_review(
        &self,
        actor: &Principal,
        item_id: ItemId,
    ) -> Result<MetadataReviewView> {
        let item = self.reader.item(item_id)?;
        self.authorize(actor, item.collection_id, Action::Curate)?;
        let view = self.metadata_view(actor, item_id)?;
        let current = view.current_candidates.iter().collect::<BTreeSet<_>>();
        let can_update = self
            .content_write_eligibility(actor, item.collection_id)
            .and_then(|eligibility| self.validate_owned_domain(&eligibility, &item.domain))
            .is_ok()
            && !item.deleted;
        let music = item.domain.module.0 == "org.junk.music";
        let games = item.domain.module.0 == "org.junk.games";
        let provider = |id: &str, label: &str, applicable: bool, guidance: &str| {
            let configured =
                self.streaming_providers.contains_key(id) && self.catalog_parsers.contains_key(id);
            MetadataProviderView {
                id: ModuleId(id.into()),
                label: label.into(),
                availability: if !applicable {
                    MetadataProviderAvailability::Inapplicable
                } else if configured {
                    MetadataProviderAvailability::Available
                } else {
                    MetadataProviderAvailability::Unconfigured
                },
                can_acquire: applicable && configured && can_update,
                guidance: if applicable && configured {
                    guidance.into()
                } else if applicable {
                    "Ask the server administrator to configure a meaningful MusicBrainz User-Agent, then restart the server. Inventory, preservation and existing artwork remain available."
                        .into()
                } else {
                    "This provider does not apply to this collection type.".into()
                },
            }
        };
        let mut lookup_choices = Vec::new();
        if music {
            for identity in &item.identifiers {
                let release = collector_modules::music::MusicBrainzQuery::ReleaseId {
                    release_id: identity.value.clone(),
                    refresh: false,
                };
                let disc = collector_modules::music::MusicBrainzQuery::DiscId {
                    disc_id: identity.value.clone(),
                    refresh: false,
                };
                let (kind, label) = if identity.provider.0.contains("musicbrainz")
                    && identity.provider.0.contains("disc")
                    && disc.validate().is_ok()
                {
                    (MetadataLookupKind::DiscId, "Recorded disc identifier")
                } else if identity.provider.0.contains("musicbrainz") && release.validate().is_ok()
                {
                    (
                        MetadataLookupKind::ReleaseId,
                        "Recorded MusicBrainz release",
                    )
                } else {
                    continue;
                };
                let digest = Digest::sha256(
                    format!("{}:{}", identity.provider.0, identity.value).as_bytes(),
                );
                lookup_choices.push(MetadataLookupChoice {
                    id: format!("recorded:{}", digest.hex),
                    kind,
                    label: format!("{label}: {}", identity.value),
                    value: identity.value.clone(),
                    recommended: false,
                });
            }
            for capture in &item.captures {
                for representation in &capture.representations {
                    let source = PackageScope::Representation(representation.id);
                    let fingerprint = self
                        .processing
                        .recorded_source(item.collection_id, source)?;
                    let mut after = None;
                    loop {
                        let page = self.processing.observations(&crate::ProcessingQuery {
                            collection_id: item.collection_id,
                            source,
                            source_fingerprint: fingerprint.clone(),
                            operation: None,
                            after,
                            limit: 100,
                        })?;
                        let next = page.items.last().map(|o| o.id);
                        let mut found = false;
                        for observation in page.items {
                            if observation.source_fingerprint != fingerprint {
                                continue;
                            }
                            if let Some(collector_modules::music::CdProcessingResult::Layout {
                                disc_ids: Some(ids),
                                ..
                            }) = recorded_music_layout(&observation)?
                            {
                                if !lookup_choices.iter().any(|choice| {
                                    choice.kind == MetadataLookupKind::DiscId
                                        && choice.value == ids.musicbrainz
                                }) {
                                    lookup_choices.push(MetadataLookupChoice {
                                        id: format!("layout:{}", observation.id),
                                        kind: MetadataLookupKind::DiscId,
                                        label: format!(
                                            "Disc identifier from recorded layout: {}",
                                            ids.musicbrainz
                                        ),
                                        value: ids.musicbrainz,
                                        recommended: false,
                                    });
                                }
                                found = true;
                                break;
                            }
                        }
                        if found || page.next_cursor.is_none() {
                            break;
                        }
                        after = next;
                    }
                }
            }
            let search_value = [view.display.title.value.as_str()]
                .into_iter()
                .chain(
                    view.display
                        .subtitle
                        .as_ref()
                        .map(|value| value.value.as_str()),
                )
                .collect::<Vec<_>>()
                .join(" ");
            let search_value = search_value
                .chars()
                .scan(0usize, |bytes, character| {
                    *bytes += character.len_utf8();
                    (*bytes <= 255).then_some(character)
                })
                .collect::<String>();
            lookup_choices.push(MetadataLookupChoice {
                id: format!("search:{}", Digest::sha256(search_value.as_bytes()).hex),
                kind: MetadataLookupKind::Search,
                label: format!("Search by current text: {search_value}"),
                value: search_value,
                recommended: false,
            });
            let recorded = lookup_choices
                .iter()
                .filter(|choice| choice.kind != MetadataLookupKind::Search)
                .count();
            if recorded == 1 {
                if let Some(choice) = lookup_choices
                    .iter_mut()
                    .find(|choice| choice.kind != MetadataLookupKind::Search)
                {
                    choice.recommended = true;
                }
            } else if recorded == 0
                && let Some(choice) = lookup_choices
                    .iter_mut()
                    .find(|choice| choice.kind == MetadataLookupKind::Search)
            {
                choice.recommended = true;
            }
        }
        let mut catalog_sources = Vec::new();
        if games {
            for capture in &item.captures {
                for representation in &capture.representations {
                    let source = PackageScope::Representation(representation.id);
                    let source_fingerprint = self
                        .processing
                        .recorded_source(item.collection_id, source)?;
                    let page = self.processing.observations(&crate::ProcessingQuery {
                        collection_id: item.collection_id,
                        source,
                        source_fingerprint: source_fingerprint.clone(),
                        operation: Some(OperationId::new("org.junk.games.catalog-verification")?),
                        after: None,
                        limit: 200,
                    })?;
                    for observation in page.items {
                        let Some(snapshot_id) = observation.snapshot_id else {
                            continue;
                        };
                        if observation.typed_result.module.0 != "org.junk.games"
                            || observation
                                .typed_result
                                .payload
                                .parse::<collector_modules::games::GameProcessingResult>()
                                .is_err()
                        {
                            continue;
                        }
                        catalog_sources.push(MetadataCatalogSourceView {
                            observation_id: observation.id,
                            snapshot_id,
                            label: format!(
                                "Recorded DAT result from {}",
                                observation.created_at.format("%Y-%m-%d %H:%M UTC")
                            ),
                            current: observation.source_fingerprint == source_fingerprint,
                        });
                    }
                }
            }
        }
        catalog_sources.sort_by(|left, right| {
            right
                .current
                .cmp(&left.current)
                .then_with(|| left.observation_id.cmp(&right.observation_id))
        });
        let preferred_language = LanguageTag::new(self.metadata_language(actor.user_id)?)?;
        let current_candidates = view
            .candidates
            .iter()
            .filter(|candidate| current.contains(&candidate.identity))
            .cloned()
            .collect::<Vec<_>>();
        let (proposed, uncertain) =
            automatic_metadata_candidate(&current_candidates, preferred_language.as_str());
        let mut review_reasons = Vec::new();
        if view.display.candidate.is_none() && view.authority.field_overrides.is_empty() {
            review_reasons.push(MetadataReviewReason::MissingName);
        }
        if view.display.artwork.is_none() {
            review_reasons.push(MetadataReviewReason::MissingArtwork);
        }
        if let Some(reason) = uncertain {
            review_reasons.push(reason);
        }
        if self
            .processing
            .metadata_authority(item.collection_id, item.id)?
            .is_some_and(|record| {
                record
                    .observations
                    .last()
                    .is_some_and(|value| value.outcome == MetadataRefreshOutcome::NoMatch)
            })
        {
            review_reasons.push(MetadataReviewReason::NoMatch);
        }
        if self
            .care
            .metadata_resolution_jobs(actor.user_id, item.id)?
            .first()
            .is_some_and(|job| job.state == JobState::Failed)
        {
            review_reasons.push(MetadataReviewReason::LookupFailed);
        }
        if let Some(proposed) = proposed
            && view
                .authority
                .field_overrides
                .iter()
                .any(|field| match field.field {
                    MetadataField::Title => field.value != proposed.title.value,
                    MetadataField::Subtitle => {
                        Some(field.value.as_str())
                            != proposed.subtitle.as_ref().map(|value| value.value.as_str())
                    }
                })
        {
            review_reasons.push(MetadataReviewReason::ManualConflict);
        }
        let proposed_candidate = proposed.map(|value| value.identity.clone());
        let candidates = view
            .candidates
            .into_iter()
            .map(|candidate| MetadataCandidateView {
                current: current.contains(&candidate.identity),
                selected: view.authority.selected_candidate.as_ref() == Some(&candidate.identity),
                unmatched: (view.authority.selected_candidate.as_ref()
                    == Some(&candidate.identity)
                    || view.authority.automatic_candidate.as_ref() == Some(&candidate.identity))
                    && !current.contains(&candidate.identity),
                provider_priority: candidate.provider_priority,
                details: safe_candidate_details(&candidate),
                music_layout: music_layout(&candidate),
                accepted_automatically: view.authority.selected_candidate.is_none()
                    && view.display.candidate.as_ref() == Some(&candidate.identity),
                identity: candidate.identity,
                title: candidate.title,
                subtitle: candidate.subtitle,
                artwork: candidate.artwork,
                complete_identity: candidate.complete_identity,
                strength: candidate.strength,
            })
            .collect();
        let mut identification = None;
        if games {
            for representation in item
                .captures
                .iter()
                .flat_map(|capture| &capture.representations)
            {
                let source = PackageScope::Representation(representation.id);
                let fingerprint = self
                    .processing
                    .recorded_source(item.collection_id, source)?;
                let mut after = None;
                loop {
                    let page = self.processing.observations(&crate::ProcessingQuery {
                        collection_id: item.collection_id,
                        source,
                        source_fingerprint: fingerprint.clone(),
                        operation: Some(OperationId::new("org.junk.games.identify")?),
                        after,
                        limit: 100,
                    })?;
                    if let Some(observation) = page
                        .items
                        .iter()
                        .find(|o| o.source_fingerprint == fingerprint)
                    {
                        identification = Some(observation.typed_result.payload.parse()?);
                        break;
                    }
                    after = page
                        .next_cursor
                        .map(|value| value.parse())
                        .transpose()
                        .map_err(|_| Error::Conflict)?;
                    if after.is_none() {
                        break;
                    }
                }
                if identification.is_some() {
                    break;
                }
            }
        }
        Ok(MetadataReviewView {
            terms: self
                .modules
                .descriptors()
                .into_iter()
                .find(|descriptor| descriptor.id == item.domain.module)
                .map_or_else(
                    || collector_modules::metadata_terms(&item.domain.module.0),
                    |descriptor| descriptor.metadata_terms,
                ),
            recognized_systems: if item.domain.module.0 == "org.junk.games" {
                collector_modules::games::standard_game_systems()
            } else {
                vec![]
            },
            identification,
            item_id,
            revision: view.revision,
            authority: view.authority,
            display: view.display,
            candidates,
            providers: vec![
                provider(
                    collector_modules::music::MUSICBRAINZ_PROVIDER_ID,
                    "MusicBrainz",
                    music,
                    "Search by disc ID or album name.",
                ),
                provider(
                    collector_modules::music::COVER_ART_ARCHIVE_PROVIDER_ID,
                    "Cover Art Archive",
                    music,
                    "Find cover art, save an image, then choose it as the cover.",
                ),
            ],
            lookup_choices,
            catalog_sources,
            permissions: MetadataPermissions { can_update },
            review_reasons,
            proposed_candidate,
            preferred_language,
        })
    }

    /// Dispatch one completed acquisition to its application-owned result action.
    /// Image publication does not require another network lookup or silently alter
    /// metadata authority: the published image is available for explicit selection.
    pub fn complete_metadata_acquisition(
        &self,
        actor: &Principal,
        item_id: ItemId,
        command: CompleteMetadataAcquisitionCommand,
        context: &CommandContext,
    ) -> Result<CompletedMetadataAcquisition> {
        let item = self.reader.item(item_id)?;
        self.authorize(actor, item.collection_id, Action::Curate)?;
        let job = self.job(actor, command.job_id)?;
        if job.item_id != Some(item_id) || job.collection_id != Some(item.collection_id) {
            return Err(Error::NotFound);
        }
        if job.state != crate::JobState::Completed {
            return Err(Error::Conflict);
        }
        let Some(crate::JobResultView::CatalogSnapshot { snapshot_id, .. }) = job.result else {
            return Err(Error::Invalid("job has no metadata snapshot".into()));
        };
        let snapshot = self.processing.snapshot(SnapshotQuery {
            collection_id: item.collection_id,
            snapshot_id,
        })?;
        if snapshot.provider.0 == collector_modules::music::MUSICBRAINZ_PROVIDER_ID {
            let record = self.refresh_provider_metadata(
                actor,
                item_id,
                RefreshProviderMetadataCommand {
                    expected_revision: command.expected_revision,
                    snapshot_id,
                    source_observation_id: None,
                    artwork: Vec::new(),
                },
                context,
            )?;
            let outcome = record
                .observations
                .last()
                .ok_or_else(|| Error::Invalid("metadata refresh has no observation".into()))?
                .outcome;
            return Ok(CompletedMetadataAcquisition::Metadata {
                review: Box::new(self.metadata_review(actor, item_id)?),
                outcome,
            });
        }
        if snapshot.provider.0 != collector_modules::music::COVER_ART_ARCHIVE_PROVIDER_ID {
            return Err(Error::Invalid("job is not a metadata acquisition".into()));
        }
        let observation: collector_modules::music::CoverArtArchiveSnapshotObservation =
            snapshot.parsed_observation.parse()?;
        observation.validate()?;
        match observation.query {
            collector_modules::music::CoverArtArchiveQuery::Listing { .. } => {
                Ok(CompletedMetadataAcquisition::ArtworkOptions {
                    options: self.metadata_artwork_options(actor, item_id, snapshot_id)?,
                })
            }
            collector_modules::music::CoverArtArchiveQuery::Image { .. } => {
                let asset = self.publish_provider_artwork(
                    actor,
                    item_id,
                    PublishProviderArtworkCommand { snapshot_id },
                    context,
                )?;
                Ok(CompletedMetadataAcquisition::ArtworkPublished {
                    asset: Box::new(self.asset_presented(
                        actor,
                        item.collection_id,
                        asset.asset.id,
                    )?),
                })
            }
        }
    }

    pub fn acquire_metadata(
        &self,
        actor: &Principal,
        item_id: ItemId,
        command: AcquireMetadataCommand,
        context: &CommandContext,
    ) -> Result<JobId> {
        match command {
            AcquireMetadataCommand::MusicBrainzRelease {
                release_id,
                refresh,
            } => self.enqueue_musicbrainz_metadata_acquisition(
                actor,
                item_id,
                collector_modules::music::MusicBrainzQuery::ReleaseId {
                    release_id,
                    refresh,
                },
                context,
            ),
            AcquireMetadataCommand::MusicBrainzDisc { disc_id, refresh } => self
                .enqueue_musicbrainz_metadata_acquisition(
                    actor,
                    item_id,
                    collector_modules::music::MusicBrainzQuery::DiscId { disc_id, refresh },
                    context,
                ),
            AcquireMetadataCommand::MusicBrainzSearch { query, refresh } => self
                .enqueue_musicbrainz_metadata_acquisition(
                    actor,
                    item_id,
                    collector_modules::music::MusicBrainzQuery::Search { query, refresh },
                    context,
                ),
            AcquireMetadataCommand::CoverArtListing { candidate } => {
                self.validate_musicbrainz_candidate(actor, item_id, &candidate)?;
                self.enqueue_cover_art_acquisition(
                    actor,
                    item_id,
                    collector_modules::music::CoverArtArchiveQuery::Listing {
                        release_id: candidate.provider_record_id,
                    },
                    context,
                )
            }
            AcquireMetadataCommand::CoverArtImage {
                candidate,
                image_id,
            } => {
                self.validate_musicbrainz_candidate(actor, item_id, &candidate)?;
                self.enqueue_cover_art_acquisition(
                    actor,
                    item_id,
                    collector_modules::music::CoverArtArchiveQuery::Image {
                        release_id: candidate.provider_record_id,
                        image_id,
                    },
                    context,
                )
            }
        }
    }

    pub fn apply_metadata_snapshot(
        &self,
        actor: &Principal,
        item_id: ItemId,
        command: ApplyMetadataSnapshotCommand,
        context: &CommandContext,
    ) -> Result<MetadataReviewView> {
        self.refresh_provider_metadata(
            actor,
            item_id,
            RefreshProviderMetadataCommand {
                expected_revision: command.expected_revision,
                snapshot_id: command.snapshot_id,
                source_observation_id: command.source_observation_id,
                artwork: command.artwork,
            },
            context,
        )?;
        self.metadata_review(actor, item_id)
    }

    pub fn with_artwork_preview(
        mut self,
        provider: std::sync::Arc<dyn ArtworkPreviewPort>,
    ) -> Self {
        self.artwork_preview = Some(provider);
        self
    }

    pub fn metadata_artwork_preview(
        &self,
        actor: &Principal,
        item_id: ItemId,
        snapshot_id: SnapshotId,
        image_id: &str,
        cancelled: &dyn Fn() -> Result<bool>,
    ) -> Result<ArtworkPreview> {
        // Reuse collection authorization and verified listing parsing. A caller
        // cannot turn this into an arbitrary URL/image proxy.
        let option = self
            .metadata_artwork_options(actor, item_id, snapshot_id)?
            .into_iter()
            .find(|option| option.image_id == image_id)
            .ok_or(Error::NotFound)?;
        if cancelled()? {
            return Err(Error::JobCancelled);
        }
        let preview = self
            .artwork_preview
            .as_ref()
            .ok_or_else(|| Error::Unavailable("Artwork previews are not configured.".into()))?
            .preview(&option.release_id, &option.image_id, cancelled)?;
        if cancelled()? {
            return Err(Error::JobCancelled);
        }
        if preview.bytes.is_empty()
            || preview.bytes.len() as u64 > MAX_ARTWORK_PREVIEW_BYTES
            || !matches!(
                preview.media_type.as_str(),
                "image/png" | "image/jpeg" | "image/webp"
            )
        {
            return Err(Error::Unavailable(
                "Invalid or oversized artwork preview.".into(),
            ));
        }
        Ok(preview)
    }

    pub fn metadata_artwork_options(
        &self,
        actor: &Principal,
        item_id: ItemId,
        snapshot_id: SnapshotId,
    ) -> Result<Vec<MetadataArtworkOption>> {
        let item = self.reader.item(item_id)?;
        self.authorize(actor, item.collection_id, Action::Curate)?;
        let snapshot = self.processing.snapshot(SnapshotQuery {
            collection_id: item.collection_id,
            snapshot_id,
        })?;
        if snapshot.provider.0 != collector_modules::music::COVER_ART_ARCHIVE_PROVIDER_ID {
            return Err(Error::Invalid(
                "artwork choices require a Cover Art Archive listing".into(),
            ));
        }
        let parser = self
            .catalog_parsers
            .get(&snapshot.provider.0)
            .ok_or_else(|| Error::Unavailable("artwork provider parser is unavailable".into()))?;
        let mut reader = self.processing.snapshot_bytes(SnapshotQuery {
            collection_id: item.collection_id,
            snapshot_id,
        })?;
        let parsed = parser.parse_snapshot(&snapshot, reader.as_mut(), &mut || Ok(false))?;
        let observation: collector_modules::music::CoverArtArchiveSnapshotObservation =
            parsed.observation.parse()?;
        if !matches!(
            observation.query,
            collector_modules::music::CoverArtArchiveQuery::Listing { .. }
        ) {
            return Err(Error::Invalid(
                "artwork choices require a Cover Art Archive listing".into(),
            ));
        }
        parsed
            .entries
            .into_iter()
            .map(|raw| {
                let option: collector_modules::music::CoverArtObservation = raw.parse()?;
                option.validate()?;
                Ok(MetadataArtworkOption {
                    snapshot_id,
                    release_id: option.release_id,
                    image_id: option.image_id,
                    front: option.front,
                    back: option.back,
                    approved: option.approved,
                    types: option.types,
                })
            })
            .collect()
    }

    fn validate_musicbrainz_candidate(
        &self,
        actor: &Principal,
        item_id: ItemId,
        candidate: &MetadataCandidateIdentity,
    ) -> Result<()> {
        if candidate.provider.0 != collector_modules::music::MUSICBRAINZ_PROVIDER_ID
            || !self
                .metadata_view(actor, item_id)?
                .candidates
                .iter()
                .any(|value| &value.identity == candidate)
        {
            return Err(Error::Invalid(
                "artwork lookup requires an observed MusicBrainz release candidate".into(),
            ));
        }
        Ok(())
    }

    pub fn update_metadata_authority(
        &self,
        actor: &Principal,
        item_id: ItemId,
        command: UpdateMetadataAuthorityCommand,
        context: &CommandContext,
    ) -> Result<MetadataAuthorityRecord> {
        let item = self.reader.item(item_id)?;
        let eligibility = self.content_write_eligibility(actor, item.collection_id)?;
        if item.deleted {
            return Err(Error::NotFound);
        }
        self.validate_owned_domain(&eligibility, &item.domain)?;
        let request = (item_id, &command);
        self.execute_command(
            actor,
            context,
            "metadata.authority.update",
            &request,
            |effects| {
                let LoadedMetadataAuthority {
                    revision,
                    mut authority,
                    legacy,
                    current,
                } = self.authority_or_inventory(&item)?;
                if revision != command.expected_revision {
                    return Err(Error::Conflict);
                }
                let previous_disc_assignments = authority
                    .domain_overrides
                    .as_ref()
                    .and_then(|overrides| {
                        overrides
                            .payload
                            .parse::<collector_modules::music::MusicMetadataOverrides>()
                            .ok()
                    })
                    .map(|overrides| overrides.disc_assignments)
                    .unwrap_or_default();
                let mut candidates = self
                    .processing
                    .metadata_candidates(item.collection_id, item.id)?;
                merge_metadata_candidates(
                    &mut candidates,
                    legacy.iter().flat_map(|value| &value.candidates),
                )?;
                apply_authority_change(&mut authority, &command.change, &candidates)?;
                self.modules
                    .validate_metadata_authority(&eligibility.owner, &authority)
                    .map_err(Error::from)?;
                if matches!(
                    &command.change,
                    MetadataAuthorityChange::SetDomainOverrides { .. }
                ) && item.domain.module.0 == "org.junk.music"
                {
                    validate_music_disc_assignment_change(
                        &item,
                        &authority,
                        &current_metadata_candidates(&authority, &candidates, current.as_deref())?,
                        &previous_disc_assignments,
                    )?;
                }
                if let MetadataAuthorityChange::SetArtworkChoice { asset_id } = &command.change
                    && !self
                        .content
                        .asset_states_for_item(item.id)?
                        .iter()
                        .any(|state| !state.deleted && state.asset.id == *asset_id)
                {
                    return Err(Error::Invalid(
                        "metadata artwork choice is not an active asset for this item".into(),
                    ));
                }
                authority.automatic_language =
                    Some(LanguageTag::new(self.metadata_language(actor.user_id)?)?);
                let record = MetadataAuthorityRecord {
                    resolution_policy: 3,
                    inventory: Some((&item).into()),
                    collection_id: item.collection_id,
                    item_id,
                    revision: Revision(revision.0.checked_add(1).ok_or(Error::Conflict)?),
                    display: Some(resolve_metadata_display(
                        &item,
                        &authority,
                        &current_metadata_candidates(&authority, &candidates, current.as_deref())?,
                        &self.metadata_language(actor.user_id)?,
                    )?),
                    authority,
                    observations: legacy,
                    current_candidates: current,
                    updated_at: Utc::now(),
                };
                record.validate()?;
                effects.publish_metadata_authority(TransactionId::new(), record.clone())?;
                Ok(record)
            },
        )
    }

    pub fn refresh_metadata(
        &self,
        actor: &Principal,
        item_id: ItemId,
        command: RefreshMetadataCommand,
        context: &CommandContext,
    ) -> Result<MetadataAuthorityRecord> {
        command.observation.validate()?;
        let item = self.reader.item(item_id)?;
        let eligibility = self.content_write_eligibility(actor, item.collection_id)?;
        if item.deleted {
            return Err(Error::NotFound);
        }
        self.validate_owned_domain(&eligibility, &item.domain)?;
        let request = (item_id, &command);
        self.execute_command(actor, context, "metadata.refresh", &request, |effects| {
            self.refresh_metadata_effect(
                &item,
                &eligibility.owner,
                &command,
                &self.metadata_language(actor.user_id)?,
                None,
                effects,
            )
        })
    }

    pub(crate) fn refresh_metadata_effect(
        &self,
        item: &Item,
        owner: &ModuleId,
        command: &RefreshMetadataCommand,
        preferred_language: &str,
        claim: Option<&JobClaim>,
        effects: &dyn ProcessingPort,
    ) -> Result<MetadataAuthorityRecord> {
        command.observation.validate()?;
        let LoadedMetadataAuthority {
            revision,
            mut authority,
            legacy: mut observations,
            current,
        } = self.authority_or_inventory(item)?;
        if revision != command.expected_revision {
            return Err(Error::Conflict);
        }
        let mut candidates = self
            .processing
            .metadata_candidates(item.collection_id, item.id)?;
        merge_metadata_candidates(
            &mut candidates,
            observations.iter().flat_map(|value| &value.candidates),
        )?;
        let assets = self.content.asset_states_for_item(item.id)?;
        if command
            .observation
            .candidates
            .iter()
            .filter_map(|candidate| candidate.artwork)
            .any(|id| {
                !assets
                    .iter()
                    .any(|state| !state.deleted && state.asset.id == id)
            })
        {
            return Err(Error::Invalid(
                "metadata artwork is not an active asset for this item".into(),
            ));
        }
        let previous_current = current.unwrap_or_else(|| {
            candidates
                .iter()
                .map(|candidate| candidate.identity.clone())
                .collect()
        });
        let previous_choice = authority
            .selected_candidate
            .as_ref()
            .and_then(|identity| candidates.iter().find(|value| &value.identity == identity))
            .cloned();
        let available = command.observation.outcome != MetadataRefreshOutcome::Unavailable;
        let mut current = if available {
            command
                .observation
                .candidates
                .iter()
                .map(|candidate| candidate.identity.clone())
                .collect::<Vec<_>>()
        } else {
            previous_current
        };
        current.sort();
        current.dedup();
        merge_metadata_candidates(&mut candidates, &command.observation.candidates)?;
        if let Some(previous) = previous_choice.filter(|_| available) {
            authority.selected_candidate = Some(
                command
                    .observation
                    .candidates
                    .iter()
                    .find(|incoming| metadata_candidates_equivalent(&previous, incoming))
                    .map(|incoming| incoming.identity.clone())
                    .unwrap_or(previous.identity),
            );
        }
        authority.automatic_language = Some(LanguageTag::new(preferred_language)?);
        if available && authority.selected_candidate.is_none() {
            let (automatic, _) =
                automatic_metadata_candidate(&command.observation.candidates, preferred_language);
            if let Some(automatic) = automatic {
                authority.automatic_candidate = Some(automatic.identity.clone());
            }
        }
        observations.push(command.observation.clone());
        self.modules
            .validate_metadata_authority(owner, &authority)
            .map_err(Error::from)?;
        let record = MetadataAuthorityRecord {
            resolution_policy: 3,
            inventory: Some(item.into()),
            collection_id: item.collection_id,
            item_id: item.id,
            revision: Revision(revision.0.checked_add(1).ok_or(Error::Conflict)?),
            display: Some(resolve_metadata_display(
                item,
                &authority,
                &current_metadata_candidates(&authority, &candidates, Some(&current))?,
                preferred_language,
            )?),
            authority,
            observations,
            current_candidates: Some(current),
            updated_at: self.authentication.now(),
        };
        record.validate()?;
        if let Some(old) = self
            .processing
            .metadata_authority(item.collection_id, item.id)?
        {
            let same_observation = match (old.observations.last(), record.observations.last()) {
                (Some(old), Some(new)) => {
                    let normalize =
                        |observation: &MetadataRefreshObservation| -> Result<serde_json::Value> {
                            let mut value = serde_json::to_value(observation)
                                .map_err(|e| Error::Invalid(e.to_string()))?;
                            if let Some(object) = value.as_object_mut() {
                                object.remove("id");
                                object.remove("observed_at");
                            }
                            // Snapshot references for empty/non-JSON responses describe
                            // content and interpretation, rather than an occurrence ID.
                            if let Some(raw) = value
                                .get_mut("raw_observation")
                                .and_then(|v| v.as_object_mut())
                                && raw.get("provider").and_then(|v| v.as_str())
                                    == Some(collector_modules::music::MUSICBRAINZ_PROVIDER_ID)
                                && raw.keys().all(|key| {
                                    matches!(
                                        key.as_str(),
                                        "provider"
                                            | "snapshot_id"
                                            | "snapshot_digest"
                                            | "outcome"
                                            | "observation"
                                    )
                                })
                                && let Some(id) = raw.get("snapshot_id").and_then(|v| {
                                    serde_json::from_value::<SnapshotId>(v.clone()).ok()
                                })
                                && let Ok(snapshot) = self.processing.snapshot(SnapshotQuery {
                                    collection_id: item.collection_id,
                                    snapshot_id: id,
                                })
                                && raw.get("snapshot_digest")
                                    == Some(
                                        &serde_json::to_value(&snapshot.digest)
                                            .map_err(|e| Error::Invalid(e.to_string()))?,
                                    )
                                && snapshot.provider.0
                                    == collector_modules::music::MUSICBRAINZ_PROVIDER_ID
                            {
                                let mut interpretation = snapshot.parsed_observation.value()?;
                                if let Some(query) = interpretation
                                    .get_mut("query")
                                    .and_then(|v| v.as_object_mut())
                                {
                                    query.remove("refresh");
                                }
                                raw.remove("snapshot_id");
                                raw.insert("observation".into(), interpretation);
                                raw.insert(
                                    "media_type".into(),
                                    serde_json::json!(snapshot.media_type),
                                );
                                raw.insert("bytes".into(), serde_json::json!(snapshot.bytes));
                            }
                            Ok(value)
                        };
                    normalize(old)? == normalize(new)?
                }
                _ => false,
            };
            if same_observation
                && serde_json::to_value(&old.inventory)
                    .map_err(|e| Error::Invalid(e.to_string()))?
                    == serde_json::to_value(&record.inventory)
                        .map_err(|e| Error::Invalid(e.to_string()))?
                && serde_json::to_value(&old.authority)
                    .map_err(|e| Error::Invalid(e.to_string()))?
                    == serde_json::to_value(&record.authority)
                        .map_err(|e| Error::Invalid(e.to_string()))?
                && old.display == record.display
                && old.current_candidates == record.current_candidates
                && old.resolution_policy == record.resolution_policy
            {
                return Ok(old);
            }
        }
        if let Some(claim) = claim {
            effects.publish_metadata_for_job(claim, record.clone())?;
        } else {
            effects.publish_metadata_authority(TransactionId::new(), record.clone())?;
        }
        Ok(record)
    }

    pub(crate) fn freeze_existing_dat_metadata(
        &self,
        user: UserId,
        item: &Item,
        job_id: collector_core::JobId,
    ) -> Result<crate::FrozenJobInput> {
        let eligibility = self.content_write_eligibility_for_user(user, item.collection_id)?;
        for representation in item.captures.iter().flat_map(|c| &c.representations) {
            let source = PackageScope::Representation(representation.id);
            let fingerprint = self
                .processing
                .recorded_source(item.collection_id, source)?;
            let mut after = None;
            loop {
                let page = self.processing.observations(&crate::ProcessingQuery {
                    collection_id: item.collection_id,
                    source,
                    source_fingerprint: fingerprint.clone(),
                    operation: None,
                    after,
                    limit: 100,
                })?;
                for observation in page.items {
                    if observation.source_fingerprint == fingerprint
                        && observation.typed_result.module.0 == "org.junk.games"
                        && matches!(
                            observation.operation.as_str(),
                            "org.junk.games.catalog-verification"
                                | "org.junk.games.import-analysis"
                        )
                        && let Some(snapshot_id) = observation.snapshot_id
                    {
                        self.processing
                            .ensure_snapshot_current(item.id, snapshot_id)?;
                        return Ok(crate::FrozenJobInput::ExistingMetadata(
                            crate::FrozenExistingMetadata {
                                job_id,
                                item_id: item.id,
                                collection_id: item.collection_id,
                                inventory_revision: item.revision,
                                generation: item.lifecycle.generation,
                                snapshot_id,
                                source_observation_id: observation.id,
                                source: fingerprint,
                                binding: eligibility.binding,
                                language: LanguageTag::new(self.metadata_language(user)?)?,
                            },
                        ));
                    }
                }
                let Some(next) = page.next_cursor else {
                    break;
                };
                after = Some(
                    next.parse()
                        .map_err(|_| Error::Storage("invalid DAT observation cursor".into()))?,
                );
            }
        }
        Err(Error::Unavailable("Analyze the original package with the configured DAT catalog before refreshing metadata.".into()))
    }
    pub(crate) fn run_existing_metadata(&self, claim: &JobClaim) -> Result<serde_json::Value> {
        let Some(crate::FrozenJobInput::ExistingMetadata(frozen)) = &claim.frozen_input else {
            return Err(Error::Conflict);
        };
        frozen.validate()?;
        let item = self.reader.item(frozen.item_id)?;
        let eligibility =
            self.content_write_eligibility_for_user(claim.user_id, item.collection_id)?;
        self.validate_frozen_binding(&eligibility, &frozen.binding)?;
        if item.deleted
            || item.revision != frozen.inventory_revision
            || item.lifecycle.generation != frozen.generation
            || self
                .processing
                .recorded_source(item.collection_id, frozen.source.source)?
                != frozen.source
        {
            return Err(Error::Conflict);
        }
        self.processing
            .ensure_snapshot_current(item.id, frozen.snapshot_id)?;
        let observation_id = ObservationId(claim.id.0);
        if !self.processing.metadata_refresh_observed(
            item.collection_id,
            item.id,
            observation_id,
        )? {
            let revision = self.authority_or_inventory(&item)?.revision;
            let observation = self.provider_metadata_observation_controlled(
                &item,
                &eligibility.owner,
                &RefreshProviderMetadataCommand {
                    expected_revision: revision,
                    snapshot_id: frozen.snapshot_id,
                    source_observation_id: Some(frozen.source_observation_id),
                    artwork: vec![],
                },
                observation_id,
                &mut || {
                    self.jobs
                        .cancellation_requested(claim.id, &claim.claim_token)
                },
            )?;
            self.refresh_metadata_effect(
                &item,
                &eligibility.owner,
                &RefreshMetadataCommand {
                    expected_revision: revision,
                    observation,
                },
                frozen.language.as_str(),
                Some(claim),
                self.processing.as_ref(),
            )?;
        }
        Ok(serde_json::json!({"item_id":item.id,"snapshot_id":frozen.snapshot_id}))
    }

    pub(crate) fn refresh_resolution_metadata(
        &self,
        claim: &JobClaim,
        frozen: &FrozenMetadataResolution,
        snapshot_id: SnapshotId,
        artwork: Vec<ProviderArtworkLink>,
        stage: &str,
    ) -> Result<MetadataAuthorityRecord> {
        let item = self.resolution_item(claim, frozen)?;
        let observation_id = ObservationId(deterministic_inventory_uuid(
            claim.user_id,
            stage,
            &claim.id.to_string(),
        ));
        if self
            .processing
            .metadata_refresh_observed(item.collection_id, item.id, observation_id)?
        {
            return self
                .processing
                .metadata_authority(item.collection_id, item.id)?
                .ok_or(Error::Conflict);
        }
        let revision = self.authority_or_inventory(&item)?.revision;
        let eligibility =
            self.content_write_eligibility_for_user(claim.user_id, item.collection_id)?;
        let observation = self.provider_metadata_observation_controlled(
            &item,
            &eligibility.owner,
            &RefreshProviderMetadataCommand {
                expected_revision: revision,
                snapshot_id,
                source_observation_id: None,
                artwork,
            },
            observation_id,
            &mut || {
                self.jobs
                    .cancellation_requested(claim.id, &claim.claim_token)
            },
        )?;
        self.refresh_metadata_effect(
            &item,
            &eligibility.owner,
            &RefreshMetadataCommand {
                expected_revision: revision,
                observation,
            },
            frozen.language.as_str(),
            Some(claim),
            self.processing.as_ref(),
        )
    }

    pub(crate) fn select_resolution_artwork(
        &self,
        claim: &JobClaim,
        frozen: &FrozenMetadataResolution,
        asset: AssetId,
    ) -> Result<()> {
        let item = self.resolution_item(claim, frozen)?;
        let LoadedMetadataAuthority {
            revision,
            mut authority,
            legacy: observations,
            current,
        } = self.authority_or_inventory(&item)?;
        if authority.artwork_choice == Some(asset) {
            return Ok(());
        }
        if revision != frozen.authority_revision {
            return Err(Error::Conflict);
        }
        let mut candidates = self
            .processing
            .metadata_candidates(item.collection_id, item.id)?;
        merge_metadata_candidates(
            &mut candidates,
            observations.iter().flat_map(|value| &value.candidates),
        )?;
        apply_authority_change(
            &mut authority,
            &MetadataAuthorityChange::SetArtworkChoice { asset_id: asset },
            &candidates,
        )?;
        let display = resolve_metadata_display(
            &item,
            &authority,
            &current_metadata_candidates(&authority, &candidates, current.as_deref())?,
            frozen.language.as_str(),
        )?;
        self.processing.publish_metadata_for_job(
            claim,
            MetadataAuthorityRecord {
                resolution_policy: 3,
                inventory: Some((&item).into()),
                collection_id: item.collection_id,
                item_id: item.id,
                revision: Revision(revision.0.checked_add(1).ok_or(Error::Conflict)?),
                authority,
                observations,
                current_candidates: current,
                display: Some(display),
                updated_at: self.authentication.now(),
            },
        )
    }

    fn authority_or_inventory(&self, item: &Item) -> Result<LoadedMetadataAuthority> {
        if let Some(record) = self
            .processing
            .metadata_authority(item.collection_id, item.id)?
        {
            let authority = metadata_authority_for_inventory(item, &record)?;
            return Ok(LoadedMetadataAuthority {
                revision: record.revision,
                authority,
                legacy: Vec::new(),
                current: if record.revision.0 <= item.lifecycle.automatic_metadata_through
                    && item.lifecycle.automatic_metadata_through > 0
                {
                    Some(vec![])
                } else {
                    record.current_candidates
                },
            });
        }
        let mut authority = MetadataAuthority {
            field_overrides: vec![MetadataFieldOverride {
                field: MetadataField::Title,
                value: item.title.clone(),
                language: None,
            }],
            ..Default::default()
        };
        if !item.subtitle.is_empty() {
            authority.field_overrides.push(MetadataFieldOverride {
                field: MetadataField::Subtitle,
                value: item.subtitle.clone(),
                language: None,
            });
        }
        for (field, value) in &item.metadata.overrides {
            authority
                .preserved_field_overrides
                .push(RawJson::from_value(
                    serde_json::json!({"field": field, "value": value}),
                ));
        }
        let mut legacy_candidates = item
            .metadata
            .candidates
            .iter()
            .map(|candidate| MetadataCandidateObservation {
                identity: MetadataCandidateIdentity {
                    provider: candidate.provider.clone(),
                    provider_record_id: candidate.id.clone(),
                    provider_record_version: None,
                },
                title: MetadataText {
                    value: candidate.title.clone(),
                    language: Some(candidate.language.clone()),
                },
                subtitle: (!candidate.subtitle.is_empty()).then(|| MetadataText {
                    value: candidate.subtitle.clone(),
                    language: Some(candidate.language.clone()),
                }),
                artwork: None,
                complete_identity: candidate.strength == MatchStrength::CompleteHashes,
                strength: candidate.strength,
                provider_priority: u16::MAX,
                observation: candidate.observation.clone(),
            })
            .collect::<Vec<_>>();
        legacy_candidates.sort_by(|left, right| left.identity.cmp(&right.identity));
        if let Some(selected) = &item.metadata.selected {
            let mut matches = legacy_candidates
                .iter()
                .filter(|candidate| candidate.identity.provider_record_id == *selected);
            if let Some(candidate) = matches.next()
                && matches.next().is_none()
            {
                authority.selected_candidate = Some(candidate.identity.clone());
            }
        }
        authority.validate()?;
        let observations = if legacy_candidates.is_empty() && item.metadata.overrides.is_empty() {
            Vec::new()
        } else {
            vec![MetadataRefreshObservation {
                id: legacy_observation_id(item.id),
                outcome: if legacy_candidates.is_empty() {
                    MetadataRefreshOutcome::NoMatch
                } else {
                    MetadataRefreshOutcome::Candidates
                },
                candidates: legacy_candidates,
                raw_observation: RawJson::from_value(
                    serde_json::to_value(&item.metadata)
                        .map_err(|error| Error::Invalid(error.to_string()))?,
                ),
                observed_at: item.updated_at,
            }]
        };
        Ok(LoadedMetadataAuthority {
            revision: Revision(0),
            authority,
            legacy: observations,
            current: None,
        })
    }
}

fn metadata_candidates_equivalent(
    left: &MetadataCandidateObservation,
    right: &MetadataCandidateObservation,
) -> bool {
    left.identity.provider == right.identity.provider
        && left.identity.provider_record_id == right.identity.provider_record_id
        && left.title == right.title
        && left.subtitle == right.subtitle
        && left.complete_identity == right.complete_identity
        && left.strength == right.strength
        && left.provider_priority == right.provider_priority
        && left.observation.parse::<serde_json::Value>().ok()
            == right.observation.parse::<serde_json::Value>().ok()
}

fn safe_candidate_details(candidate: &MetadataCandidateObservation) -> Option<serde_json::Value> {
    if candidate.identity.provider.0 == collector_modules::music::MUSICBRAINZ_PROVIDER_ID {
        let release = candidate
            .observation
            .parse::<collector_modules::music::MusicBrainzReleaseObservation>()
            .ok()?;
        release.validate().ok()?;
        if release.release_id != candidate.identity.provider_record_id {
            return None;
        }
        serde_json::to_value(release).ok()
    } else {
        let row = candidate
            .observation
            .parse::<collector_modules::games::DatCatalogEntryObservation>()
            .ok()?;
        if row.record_id != candidate.identity.provider_record_id
            && !candidate
                .identity
                .provider_record_id
                .ends_with(&format!(":{}", row.record_id))
        {
            return None;
        }
        serde_json::to_value(row).ok()
    }
}

fn validate_music_disc_assignment_change(
    item: &Item,
    authority: &MetadataAuthority,
    candidates: &[MetadataCandidateObservation],
    previous: &[collector_modules::music::MusicDiscAssignment],
) -> Result<()> {
    let Some(overrides) = &authority.domain_overrides else {
        return Ok(());
    };
    let overrides: collector_modules::music::MusicMetadataOverrides = overrides.payload.parse()?;
    let changed = overrides
        .disc_assignments
        .iter()
        .filter(|assignment| !previous.contains(assignment))
        .collect::<Vec<_>>();
    if changed.is_empty() {
        return Ok(());
    }
    let display = resolve_metadata_display(
        item,
        authority,
        candidates,
        authority
            .automatic_language
            .as_ref()
            .map_or("en", LanguageTag::as_str),
    )?;
    let release = display
        .candidate
        .as_ref()
        .filter(|identity| identity.provider.0 == collector_modules::music::MUSICBRAINZ_PROVIDER_ID)
        .and_then(|identity| {
            candidates
                .iter()
                .find(|candidate| &candidate.identity == identity)
        })
        .and_then(|candidate| {
            candidate
                .observation
                .parse::<collector_modules::music::MusicBrainzReleaseObservation>()
                .ok()
        })
        .ok_or_else(|| {
            Error::Invalid("select a MusicBrainz release before assigning discs".into())
        })?;
    for assignment in changed {
        if assignment.release_id != release.release_id
            || !release
                .media
                .iter()
                .any(|medium| medium.position == assignment.medium_position)
            || !item.captures.iter().any(|capture| {
                capture.association.item_id == Some(item.id)
                    && capture.representations.iter().any(|representation| {
                        representation.id == assignment.source_representation_id
                            && representation.role
                                == collector_core::RepresentationRole::PreservationMaster
                    })
            })
        {
            return Err(Error::Invalid(
                "disc assignment must reference a current original and release medium".into(),
            ));
        }
    }
    Ok(())
}

pub fn metadata_authority_for_inventory(
    item: &Item,
    record: &MetadataAuthorityRecord,
) -> Result<MetadataAuthority> {
    let mut authority = record.authority.clone();
    if record.revision.0 <= item.lifecycle.reset_fields_through
        && item.lifecycle.reset_fields_through > 0
    {
        authority.field_overrides.clear();
        authority.selected_candidate = None;
        authority.domain_overrides = None;
    }
    if record.revision.0 <= item.lifecycle.automatic_metadata_through
        && item.lifecycle.automatic_metadata_through > 0
    {
        authority.automatic_candidate = None;
    }
    if record.revision.0 <= item.lifecycle.reset_artwork_through
        && item.lifecycle.reset_artwork_through > 0
    {
        authority.artwork_choice = None;
    }

    if let Some(baseline) = &record.inventory {
        for (field, old, new) in [
            (MetadataField::Title, &baseline.title, &item.title),
            (MetadataField::Subtitle, &baseline.subtitle, &item.subtitle),
        ] {
            if old != new {
                apply_authority_change(
                    &mut authority,
                    &MetadataAuthorityChange::SetFieldOverride {
                        field,
                        value: new.clone(),
                        language: None,
                    },
                    &[],
                )?;
            }
        }
    } else if record.display.is_none() {
        // Contract-only historical authorities predate explicit clearing.
        // Preserve their existing overrides and seed remaining inventory text.
        for (field, value) in [
            (MetadataField::Title, &item.title),
            (MetadataField::Subtitle, &item.subtitle),
        ] {
            if !authority
                .field_overrides
                .iter()
                .any(|entry| entry.field == field)
            {
                apply_authority_change(
                    &mut authority,
                    &MetadataAuthorityChange::SetFieldOverride {
                        field,
                        value: value.clone(),
                        language: None,
                    },
                    &[],
                )?;
            }
        }
    }
    Ok(authority)
}
/// Shared policy for presentation and rebuildable search projection.
pub fn resolve_record_metadata(
    item: &Item,
    record: &MetadataAuthorityRecord,
    history: &[MetadataCandidateObservation],
) -> Result<ResolvedMetadataDisplay> {
    let authority = metadata_authority_for_inventory(item, record)?;
    let current = current_metadata_candidates(
        &authority,
        history,
        if record.revision.0 <= item.lifecycle.automatic_metadata_through
            && item.lifecycle.automatic_metadata_through > 0
        {
            Some(&[])
        } else {
            record.current_candidates.as_deref()
        },
    )?;
    resolve_metadata_display(
        item,
        &authority,
        &current,
        authority
            .automatic_language
            .as_ref()
            .map_or("en", LanguageTag::as_str),
    )
}

pub(crate) fn current_metadata_candidates(
    authority: &MetadataAuthority,
    history: &[MetadataCandidateObservation],
    current: Option<&[MetadataCandidateIdentity]>,
) -> Result<Vec<MetadataCandidateObservation>> {
    let Some(current) = current else {
        return Ok(history.to_vec());
    };
    if current.len() > 4096 {
        return Err(Error::Invalid("current metadata exceeds its bound".into()));
    }
    let by_identity = history
        .iter()
        .map(|candidate| (&candidate.identity, candidate))
        .collect::<BTreeMap<_, _>>();
    let mut identities = current.iter().collect::<BTreeSet<_>>();
    if identities.len() != current.len() {
        return Err(Error::Invalid("duplicate current metadata identity".into()));
    }
    for selected in authority
        .selected_candidate
        .iter()
        .chain(authority.automatic_candidate.iter())
    {
        identities.insert(selected);
    }
    identities
        .into_iter()
        .map(|identity| {
            by_identity
                .get(identity)
                .map(|candidate| (*candidate).clone())
                .ok_or_else(|| {
                    Error::Invalid("current metadata identity is absent from history".into())
                })
        })
        .collect()
}

pub fn resolve_metadata_display(
    item: &Item,
    authority: &MetadataAuthority,
    candidates: &[MetadataCandidateObservation],
    preferred_language: &str,
) -> Result<ResolvedMetadataDisplay> {
    let mut display = resolve_metadata_fields(
        &item.title,
        &item.subtitle,
        authority,
        candidates,
        preferred_language,
    )?;
    resolve_metadata_domain(&item.domain, authority, candidates, &mut display)?;
    Ok(display)
}

pub(crate) fn resolve_metadata_domain(
    domain: &ModuleData,
    authority: &MetadataAuthority,
    candidates: &[MetadataCandidateObservation],
    display: &mut ResolvedMetadataDisplay,
) -> Result<()> {
    if domain.module.0 == "org.junk.games" && domain.schema_version == 1 {
        let overrides = authority
            .domain_overrides
            .as_ref()
            .map(|data| {
                data.payload
                    .parse::<collector_modules::games::GameMetadataOverrides>()
            })
            .transpose()?;
        let automatic = overrides
            .as_ref()
            .is_some_and(|value| value.automatic_platform);
        let manual = overrides
            .as_ref()
            .and_then(|value| value.platform.clone())
            .or_else(|| {
                (!automatic)
                    .then(|| {
                        domain
                            .payload
                            .parse::<collector_modules::games::GameItemData>()
                            .ok()?
                            .platform
                    })
                    .flatten()
            })
            .filter(|value| !value.trim().is_empty());
        let detected = display
            .candidate
            .as_ref()
            .and_then(|identity| candidates.iter().find(|c| &c.identity == identity))
            .filter(|c| {
                c.complete_identity && display.resolution != MetadataResolutionKind::Tentative
            })
            .and_then(|c| {
                c.observation
                    .parse::<collector_modules::games::DatCatalogEntryObservation>()
                    .ok()?
                    .system
                    .map(|system| (system, c.identity.clone()))
            });
        if let Some(value) = manual
            .map(|value| ResolvedMetadataText {
                value,
                language: None,
                source: MetadataValueSource::Manual,
            })
            .or_else(|| {
                detected.map(|(value, identity)| ResolvedMetadataText {
                    value,
                    language: None,
                    source: MetadataValueSource::Candidate { identity },
                })
            })
        {
            display.domain_fields.insert("platform".into(), value);
        }
    }
    Ok(())
}

pub(crate) fn resolve_metadata_fields(
    inventory_title: &str,
    inventory_subtitle: &str,
    authority: &MetadataAuthority,
    candidates: &[MetadataCandidateObservation],
    preferred_language: &str,
) -> Result<ResolvedMetadataDisplay> {
    resolve_metadata_fields_with_policy(
        inventory_title,
        inventory_subtitle,
        authority,
        candidates,
        preferred_language,
        false,
    )
}

pub(crate) fn resolve_metadata_fields_with_policy(
    inventory_title: &str,
    inventory_subtitle: &str,
    authority: &MetadataAuthority,
    candidates: &[MetadataCandidateObservation],
    preferred_language: &str,
    historical: bool,
) -> Result<ResolvedMetadataDisplay> {
    authority.validate()?;
    let mut checked = Vec::new();
    merge_metadata_candidates(&mut checked, candidates)?;
    let selected = authority
        .selected_candidate
        .as_ref()
        .map(|identity| {
            candidates
                .iter()
                .find(|candidate| &candidate.identity == identity)
                .ok_or_else(|| Error::Invalid("selected metadata candidate is absent".into()))
        })
        .transpose()?;
    let (proposed, uncertain) = if historical {
        let best = candidates
            .iter()
            .max_by(|left, right| compare_candidate(left, right, preferred_language));
        let uncertain = best.is_some_and(|best| {
            candidates.iter().any(|other| {
                other.identity != best.identity
                    && policy_rank(other, preferred_language)
                        == policy_rank(best, preferred_language)
            })
        });
        (
            best,
            uncertain.then_some(MetadataReviewReason::AlbumAmbiguity),
        )
    } else {
        automatic_metadata_candidate(candidates, preferred_language)
    };
    let accepted = authority
        .automatic_candidate
        .as_ref()
        .and_then(|identity| candidates.iter().find(|value| &value.identity == identity));
    // Read language variants only within the accepted album. A candidate for a
    // different album never replaces a recorded automatic decision during reads.
    let automatic = accepted
        .map(|accepted| {
            candidates
                .iter()
                .filter(|value| {
                    value.complete_identity
                        && value.strength != MatchStrength::Name
                        && same_metadata_album(accepted, value)
                })
                .max_by(|left, right| compare_candidate(left, right, preferred_language))
                .unwrap_or(accepted)
        })
        .or(proposed);
    let candidate = selected.or(automatic);
    let ambiguous = selected.is_none() && accepted.is_none() && uncertain.is_some();
    let field = |kind: MetadataField| {
        authority
            .field_overrides
            .iter()
            .find(|value| value.field == kind)
    };
    let title = field(MetadataField::Title).map_or_else(
        || {
            candidate.map_or(
                ResolvedMetadataText {
                    value: inventory_title.to_owned(),
                    language: None,
                    source: MetadataValueSource::Provisional,
                },
                |candidate| ResolvedMetadataText {
                    value: candidate.title.value.clone(),
                    language: candidate.title.language.clone(),
                    source: MetadataValueSource::Candidate {
                        identity: candidate.identity.clone(),
                    },
                },
            )
        },
        |value| ResolvedMetadataText {
            value: value.value.clone(),
            language: value.language.clone(),
            source: MetadataValueSource::Manual,
        },
    );
    let subtitle = field(MetadataField::Subtitle).map_or_else(
        || {
            if candidate.is_none() && !inventory_subtitle.is_empty() {
                return Some(ResolvedMetadataText {
                    value: inventory_subtitle.to_owned(),
                    language: None,
                    source: MetadataValueSource::Provisional,
                });
            }
            candidate.and_then(|selected_candidate| {
                selected_candidate
                    .subtitle
                    .as_ref()
                    .map(|value| ResolvedMetadataText {
                        value: value.value.clone(),
                        language: value.language.clone(),
                        source: MetadataValueSource::Candidate {
                            identity: selected_candidate.identity.clone(),
                        },
                    })
            })
        },
        |value| {
            (!value.value.is_empty()).then(|| ResolvedMetadataText {
                value: value.value.clone(),
                language: value.language.clone(),
                source: MetadataValueSource::Manual,
            })
        },
    );
    let manual = candidate.is_none()
        || !authority.field_overrides.is_empty()
        || authority.artwork_choice.is_some()
        || authority.domain_overrides.as_ref().is_some_and(|data| {
            !(data.module.0 == "org.junk.games"
                && data.schema_version == 1
                && data
                    .payload
                    .parse::<collector_modules::games::GameMetadataOverrides>()
                    .is_ok_and(|value| value.automatic_platform && value.platform.is_none()))
        });
    Ok(ResolvedMetadataDisplay {
        domain_fields: BTreeMap::new(),
        title,
        subtitle,
        artwork: authority
            .artwork_choice
            .or_else(|| candidate.and_then(|value| value.artwork)),
        candidate: candidate.map(|value| value.identity.clone()),
        resolution: if ambiguous {
            MetadataResolutionKind::Tentative
        } else if manual {
            MetadataResolutionKind::Manual
        } else if selected.is_some() {
            MetadataResolutionKind::Selected
        } else {
            MetadataResolutionKind::Automatic
        },
    })
}

fn apply_authority_change(
    authority: &mut MetadataAuthority,
    change: &MetadataAuthorityChange,
    candidates: &[MetadataCandidateObservation],
) -> Result<()> {
    match change {
        MetadataAuthorityChange::SelectCandidate { candidate } => {
            if !candidates.iter().any(|value| &value.identity == candidate) {
                return Err(Error::Invalid(
                    "selected metadata candidate has not been observed".into(),
                ));
            }
            authority.selected_candidate = Some(candidate.clone());
        }
        MetadataAuthorityChange::ClearSelectedCandidate => authority.selected_candidate = None,
        MetadataAuthorityChange::SetFieldOverride {
            field,
            value,
            language,
        } => {
            authority
                .field_overrides
                .retain(|entry| entry.field != *field);
            authority.field_overrides.push(MetadataFieldOverride {
                field: *field,
                value: value.clone(),
                language: language.clone(),
            });
        }
        MetadataAuthorityChange::ClearFieldOverride { field } => {
            authority
                .field_overrides
                .retain(|entry| entry.field != *field);
        }
        MetadataAuthorityChange::SetArtworkChoice { asset_id } => {
            authority.artwork_choice = Some(*asset_id);
        }
        MetadataAuthorityChange::ClearArtworkChoice => authority.artwork_choice = None,
        MetadataAuthorityChange::SetDomainOverrides { overrides } => {
            authority.domain_overrides = Some(overrides.clone());
        }
        MetadataAuthorityChange::ClearDomainOverrides => authority.domain_overrides = None,
    }
    authority.field_overrides.sort_by_key(|value| value.field);
    authority.validate()
}

pub fn merge_metadata_candidates<'a>(
    target: &mut Vec<MetadataCandidateObservation>,
    incoming: impl IntoIterator<Item = &'a MetadataCandidateObservation>,
) -> Result<()> {
    let mut values = BTreeMap::new();
    for candidate in target.iter().cloned().chain(incoming.into_iter().cloned()) {
        candidate.validate()?;
        if let Some(existing) = values.get(&candidate.identity)
            && existing != &candidate
        {
            return Err(Error::Conflict);
        }
        values.insert(candidate.identity.clone(), candidate);
        if values.len() > 4096 {
            return Err(Error::Invalid(
                "metadata candidate history exceeds its bound".into(),
            ));
        }
    }
    *target = values.into_values().collect();
    Ok(())
}

/// Domain identity, not release language or provider array order, defines album
/// ambiguity. A missing release-group identity stays conservatively release-bound.
pub fn same_metadata_album(
    left: &MetadataCandidateObservation,
    right: &MetadataCandidateObservation,
) -> bool {
    if left.identity.provider != right.identity.provider {
        return false;
    }
    if left.identity.provider.0 == collector_modules::music::MUSICBRAINZ_PROVIDER_ID {
        let parse = |value: &MetadataCandidateObservation| {
            value
                .observation
                .parse::<collector_modules::music::MusicBrainzReleaseObservation>()
                .ok()
                .filter(|release| {
                    release.validate().is_ok()
                        && release.release_id == value.identity.provider_record_id
                })
        };
        if let (Some(left), Some(right)) = (parse(left), parse(right)) {
            return left.release_id == right.release_id
                || left.release_group_id.is_some()
                    && left.release_group_id == right.release_group_id;
        }
    }
    left.identity.provider_record_id == right.identity.provider_record_id
        || collector_modules::games::equivalent_game_candidates(left, right)
}

pub fn automatic_metadata_candidate<'a>(
    candidates: &'a [MetadataCandidateObservation],
    language: &str,
) -> (
    Option<&'a MetadataCandidateObservation>,
    Option<MetadataReviewReason>,
) {
    let eligible = candidates
        .iter()
        .filter(|value| value.complete_identity && value.strength != MatchStrength::Name)
        .collect::<Vec<_>>();
    let Some(best) = eligible
        .iter()
        .max_by(|left, right| compare_candidate(left, right, language))
        .copied()
    else {
        return (
            None,
            (!candidates.is_empty()).then_some(MetadataReviewReason::IncompleteProviderInformation),
        );
    };
    let lacks_album_identity = |candidate: &MetadataCandidateObservation| {
        candidate.identity.provider.0 == collector_modules::music::MUSICBRAINZ_PROVIDER_ID
            && candidate
                .observation
                .parse::<collector_modules::music::MusicBrainzReleaseObservation>()
                .ok()
                .is_none_or(|value| value.validate().is_err() || value.release_group_id.is_none())
    };
    if lacks_album_identity(best) {
        return (
            None,
            Some(MetadataReviewReason::IncompleteProviderInformation),
        );
    }
    let alternatives = eligible
        .iter()
        .filter(|other| {
            other.strength == best.strength
                && other.provider_priority == best.provider_priority
                && !same_metadata_album(best, other)
        })
        .collect::<Vec<_>>();
    if alternatives.is_empty() {
        return (Some(best), None);
    }
    let reason = if lacks_album_identity(best)
        || alternatives.iter().any(|other| lacks_album_identity(other))
    {
        MetadataReviewReason::IncompleteProviderInformation
    } else {
        MetadataReviewReason::AlbumAmbiguity
    };
    (None, Some(reason))
}

fn compare_candidate(
    left: &MetadataCandidateObservation,
    right: &MetadataCandidateObservation,
    preferred_language: &str,
) -> Ordering {
    policy_rank(left, preferred_language)
        .cmp(&policy_rank(right, preferred_language))
        .then_with(|| right.identity.cmp(&left.identity))
}

fn policy_rank(
    candidate: &MetadataCandidateObservation,
    preferred_language: &str,
) -> (bool, MatchStrength, bool, std::cmp::Reverse<u16>) {
    (
        candidate.complete_identity,
        candidate.strength,
        candidate
            .title
            .language
            .as_ref()
            .is_some_and(|language| language_matches(language.as_str(), preferred_language)),
        std::cmp::Reverse(candidate.provider_priority),
    )
}

fn language_matches(language: &str, preferred: &str) -> bool {
    language.eq_ignore_ascii_case(preferred)
        || language
            .split('-')
            .next()
            .zip(preferred.split('-').next())
            .is_some_and(|(language, preferred)| language.eq_ignore_ascii_case(preferred))
}

fn legacy_observation_id(item: ItemId) -> ObservationId {
    let digest = Sha256::digest(format!("junk-collector:legacy-metadata:{}", item).as_bytes());
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    ObservationId(uuid::Uuid::from_bytes(bytes))
}

fn provider_observation_id(
    user: UserId,
    item: ItemId,
    provider: &ModuleId,
    key: &str,
) -> ObservationId {
    let digest = Sha256::digest(
        format!(
            "junk-collector:metadata-provider-observation:{user}:{item}:{}:{key}",
            provider.0
        )
        .as_bytes(),
    );
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    bytes[6] = (bytes[6] & 0x0f) | 0x50;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    ObservationId(uuid::Uuid::from_bytes(bytes))
}

pub(crate) fn read_exact_snapshot_bytes(
    processing: &dyn crate::ProcessingPort,
    snapshot: &crate::SnapshotRecord,
    maximum: u64,
) -> Result<Vec<u8>> {
    read_exact_snapshot_bytes_controlled(processing, snapshot, maximum, &mut || Ok(false))
}

pub(crate) fn read_exact_snapshot_bytes_controlled(
    processing: &dyn crate::ProcessingPort,
    snapshot: &crate::SnapshotRecord,
    maximum: u64,
    cancelled: &mut dyn FnMut() -> Result<bool>,
) -> Result<Vec<u8>> {
    if snapshot.bytes.0 > maximum {
        return Err(Error::Invalid(
            "provider snapshot exceeds its resource bound".into(),
        ));
    }
    let mut reader = processing.snapshot_bytes(SnapshotQuery {
        collection_id: snapshot.collection_id,
        snapshot_id: snapshot.id,
    })?;
    let capacity = usize::try_from(snapshot.bytes.0)
        .map_err(|_| Error::Invalid("provider snapshot length does not fit memory".into()))?;
    let mut bytes = Vec::with_capacity(capacity);
    use sha2::{Digest as _, Sha256};
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 65536];
    loop {
        if cancelled()? {
            return Err(Error::Unavailable(
                "provider snapshot read cancelled".into(),
            ));
        }
        let allowed = (snapshot.bytes.0 + 1)
            .saturating_sub(bytes.len() as u64)
            .min(buffer.len() as u64) as usize;
        if allowed == 0 {
            break;
        }
        let length = reader
            .read(&mut buffer[..allowed])
            .map_err(|error| Error::Unavailable(format!("provider snapshot read: {error}")))?;
        if length == 0 {
            break;
        }
        hasher.update(&buffer[..length]);
        bytes.extend_from_slice(&buffer[..length]);
    }
    if cancelled()? {
        return Err(Error::Unavailable(
            "provider snapshot read cancelled".into(),
        ));
    }
    if bytes.len() as u64 != snapshot.bytes.0
        || hex::encode(hasher.finalize()) != snapshot.digest.hex
    {
        return Err(Error::Invalid(
            "provider snapshot length or digest mismatch".into(),
        ));
    }
    Ok(bytes)
}

fn json_or_snapshot_reference(snapshot: &crate::SnapshotRecord, bytes: Vec<u8>) -> RawJson {
    if bytes.is_empty() {
        return RawJson::from_value(serde_json::json!({
            "provider":snapshot.provider,
            "snapshot_id":snapshot.id,
            "snapshot_digest":snapshot.digest,
            "outcome":"no_match"
        }));
    }
    // A provider 404 may have an HTML/plain-text body. Its exact bytes remain
    // archived; the authority links that snapshot rather than inventing JSON.
    match String::from_utf8(bytes)
        .ok()
        .and_then(|json| RawJson::from_json(json).ok())
    {
        Some(raw) => raw,
        None => RawJson::from_value(serde_json::json!({
            "provider":snapshot.provider, "snapshot_id":snapshot.id,
            "snapshot_digest":snapshot.digest, "observation":snapshot.parsed_observation
        })),
    }
}

pub(crate) fn recorded_music_layout(
    observation: &collector_core::ProcessingObservation,
) -> Result<Option<collector_modules::music::CdProcessingResult>> {
    if observation.typed_result.module.0 != "org.junk.music"
        || !matches!(
            observation.support,
            collector_core::OperationSupport::Supported
        )
    {
        return Ok(None);
    }
    match observation.operation.as_str() {
        collector_modules::music::CD_STRUCTURE_OPERATION => {
            Ok(Some(observation.typed_result.payload.parse()?))
        }
        "org.junk.music.import-analysis" => {
            let output: collector_modules::music::MusicImportAnalysisOutput =
                observation.typed_result.payload.parse()?;
            output
                .observations
                .first()
                .map(|raw| raw.parse())
                .transpose()
        }
        _ => Ok(None),
    }
}

/// Album identity comes from an accepted provider observation, never title similarity.
pub fn accepted_album_group_key(
    item: &Item,
    record: &MetadataAuthorityRecord,
    candidates: &[MetadataCandidateObservation],
) -> Result<Option<String>> {
    let display = resolve_record_metadata(item, record, candidates)?;
    let Some(identity) = display.candidate else {
        return Ok(None);
    };
    if display.resolution == MetadataResolutionKind::Tentative {
        return Ok(None);
    }
    let Some(candidate) = candidates.iter().find(|v| v.identity == identity) else {
        return Ok(None);
    };
    if candidate.identity.provider.0 != collector_modules::music::MUSICBRAINZ_PROVIDER_ID {
        return Ok(None);
    }
    // Historical observations remain selectable even when they predate the
    // module's current release schema. Missing identity cannot propose a merge.
    Ok(candidate
        .observation
        .parse::<collector_modules::music::MusicBrainzReleaseObservation>()
        .ok()
        .filter(|release| release.validate().is_ok())
        .and_then(|release| release.release_group_id))
}

#[cfg(test)]
mod candidate_detail_tests {
    use super::*;

    #[test]
    fn details_show_tracks_without_exposing_untyped_provider_fields() {
        let mut candidate = MetadataCandidateObservation {
            identity: MetadataCandidateIdentity {
                provider: ModuleId(collector_modules::music::MUSICBRAINZ_PROVIDER_ID.into()),
                provider_record_id: "11111111-1111-1111-1111-111111111111".into(),
                provider_record_version: Some("one".into()),
            },
            title: MetadataText {
                value: "Album".into(),
                language: None,
            },
            subtitle: None,
            artwork: None,
            complete_identity: true,
            strength: MatchStrength::Identifier,
            provider_priority: 20,
            observation: RawJson::from_value(serde_json::json!({
                "release_id": "11111111-1111-1111-1111-111111111111",
                "title": "Album", "disambiguation": null, "language": "eng", "script": null,
                "artist_credit": "Artist", "artist_ids": [], "release_group_id": null,
                "disc_ids": [], "media": [{"position": 1, "disc_ids": [], "tracks": [{"position": 1, "title": "Opening"}]}]
            })),
        };
        assert_eq!(
            safe_candidate_details(&candidate).unwrap()["media"][0]["tracks"][0]["title"],
            "Opening"
        );
        candidate.observation =
            RawJson::from_value(serde_json::json!({"private_raw_marker": "secret"}));
        assert!(safe_candidate_details(&candidate).is_none());
    }
}
