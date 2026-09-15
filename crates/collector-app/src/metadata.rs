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
    AssetState, CollectionService, CommandContext, CommandEffects, CreateAssetCommand,
    FrozenAcquisitionJob, MetadataAuthorityRecord, Principal, ProviderRequest, SnapshotQuery,
};

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
        overrides: ModuleData,
    },
    ClearDomainOverrides,
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

#[derive(Clone, Debug, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Serialize, Deserialize)]
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

struct LoadedMetadataAuthority {
    revision: Revision,
    authority: MetadataAuthority,
    legacy: Vec<MetadataRefreshObservation>,
    current: Option<Vec<MetadataCandidateIdentity>>,
}

impl CollectionService {
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
                let snapshot = self.processing.snapshot(SnapshotQuery {
                    collection_id: item.collection_id,
                    snapshot_id: command.snapshot_id,
                })?;
                let parser = self
                    .catalog_parsers
                    .get(&snapshot.provider.0)
                    .ok_or_else(|| {
                        Error::Unavailable("metadata provider parser is unavailable".into())
                    })?;
                let mut reader = self.processing.snapshot_bytes(SnapshotQuery {
                    collection_id: item.collection_id,
                    snapshot_id: command.snapshot_id,
                })?;
                let parsed =
                    parser.parse_snapshot(&snapshot, reader.as_mut(), &mut || Ok(false))?;
                let artwork = command
                    .artwork
                    .iter()
                    .map(|value| (value.provider_record_id.clone(), value.asset_id))
                    .collect::<BTreeMap<_, _>>();
                let candidates;
                let raw_observation;
                if eligibility.owner.0 == "org.junk.music"
                    && snapshot.provider.0 == collector_modules::music::MUSICBRAINZ_PROVIDER_ID
                {
                    if command.source_observation_id.is_some() {
                        return Err(Error::Invalid(
                            "MusicBrainz metadata does not accept a DAT source observation".into(),
                        ));
                    }
                    let observation: collector_modules::music::MusicBrainzSnapshotObservation =
                        parsed.observation.parse()?;
                    candidates = collector_modules::music::musicbrainz_metadata_candidates(
                        &snapshot.digest,
                        &observation,
                        &parsed.entries,
                        &artwork,
                    )?;
                    raw_observation = exact_json_or_snapshot_reference(
                        self.processing.as_ref(),
                        &snapshot,
                        8 * 1024 * 1024,
                    )?;
                } else if eligibility.owner.0 == "org.junk.games"
                    && snapshot.provider.0 == "org.junk.providers.local-dat"
                {
                    if !artwork.is_empty() {
                        return Err(Error::Invalid(
                            "the configured DAT provider does not supply artwork".into(),
                        ));
                    }
                    let source_id = command.source_observation_id.ok_or_else(|| {
                        Error::Invalid("DAT metadata requires its D04 catalog observation".into())
                    })?;
                    let source = self.processing.processing_record(source_id)?.observation;
                    if source.collection_id != item.collection_id
                        || source.snapshot_id != Some(snapshot.id)
                        || source.operation.as_str() != "org.junk.games.catalog-verification"
                        || source.typed_result.module != eligibility.owner
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
                    let result: collector_modules::games::GameProcessingResult =
                        source.typed_result.payload.parse()?;
                    let catalog = match result {
                        collector_modules::games::GameProcessingResult::CatalogMatch {
                            catalog,
                        } => catalog,
                        _ => {
                            return Err(Error::Invalid(
                                "DAT metadata requires a catalog match result".into(),
                            ));
                        }
                    };
                    candidates = collector_modules::games::dat_metadata_candidates(
                        snapshot.provider.clone(),
                        &snapshot.digest,
                        &parsed.entries,
                        &catalog,
                    )?;
                    raw_observation = RawJson::from_value(serde_json::json!({
                        "provider":"org.junk.providers.local-dat",
                        "snapshot_id":snapshot.id,
                        "snapshot_digest":snapshot.digest,
                        "source_observation_id":source_id
                    }));
                } else {
                    return Err(Error::Invalid(
                        "metadata provider is incompatible with the collection owner".into(),
                    ));
                }
                let candidate_ids = candidates
                    .iter()
                    .map(|candidate| candidate.identity.provider_record_id.as_str())
                    .collect::<BTreeSet<_>>();
                if artwork
                    .keys()
                    .any(|record_id| !candidate_ids.contains(record_id.as_str()))
                {
                    return Err(Error::Invalid(
                        "metadata artwork link does not identify an observed candidate".into(),
                    ));
                }
                let refresh = RefreshMetadataCommand {
                    expected_revision: command.expected_revision,
                    observation: MetadataRefreshObservation {
                        id: provider_observation_id(
                            actor.user_id,
                            item_id,
                            &snapshot.provider,
                            context.key(),
                        ),
                        outcome: if candidates.is_empty() {
                            MetadataRefreshOutcome::NoMatch
                        } else {
                            MetadataRefreshOutcome::Candidates
                        },
                        candidates,
                        raw_observation,
                        observed_at: self.authentication.now(),
                    },
                };
                self.refresh_metadata_effect(&item, &eligibility.owner, &refresh, effects)
            },
        )
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
            "en",
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
                let record = MetadataAuthorityRecord {
                    inventory: Some((&item).into()),
                    collection_id: item.collection_id,
                    item_id,
                    revision: Revision(revision.0.checked_add(1).ok_or(Error::Conflict)?),
                    display: Some(resolve_metadata_display(
                        &item,
                        &authority,
                        &current_metadata_candidates(&authority, &candidates, current.as_deref())?,
                        "en",
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
            self.refresh_metadata_effect(&item, &eligibility.owner, &command, effects)
        })
    }

    fn refresh_metadata_effect(
        &self,
        item: &Item,
        owner: &ModuleId,
        command: &RefreshMetadataCommand,
        effects: &dyn CommandEffects,
    ) -> Result<MetadataAuthorityRecord> {
        command.observation.validate()?;
        let LoadedMetadataAuthority {
            revision,
            authority,
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
        let mut current = current.unwrap_or_else(|| {
            candidates
                .iter()
                .map(|candidate| candidate.identity.clone())
                .collect()
        });
        for candidate in &command.observation.candidates {
            current.retain(|identity| {
                identity.provider != candidate.identity.provider
                    || identity.provider_record_id != candidate.identity.provider_record_id
            });
            current.push(candidate.identity.clone());
        }
        current.sort();
        current.dedup();
        merge_metadata_candidates(&mut candidates, &command.observation.candidates)?;
        observations.push(command.observation.clone());
        self.modules
            .validate_metadata_authority(owner, &authority)
            .map_err(Error::from)?;
        let record = MetadataAuthorityRecord {
            inventory: Some(item.into()),
            collection_id: item.collection_id,
            item_id: item.id,
            revision: Revision(revision.0.checked_add(1).ok_or(Error::Conflict)?),
            display: Some(resolve_metadata_display(
                item,
                &authority,
                &current_metadata_candidates(&authority, &candidates, Some(&current))?,
                "en",
            )?),
            authority,
            observations,
            current_candidates: Some(current),
            updated_at: self.authentication.now(),
        };
        record.validate()?;
        effects.publish_metadata_authority(TransactionId::new(), record.clone())?;
        Ok(record)
    }

    fn authority_or_inventory(&self, item: &Item) -> Result<LoadedMetadataAuthority> {
        if let Some(record) = self
            .processing
            .metadata_authority(item.collection_id, item.id)?
        {
            let mut authority = record.authority;
            if let Some(baseline) = record.inventory {
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
            return Ok(LoadedMetadataAuthority {
                revision: record.revision,
                authority,
                legacy: Vec::new(),
                current: record.current_candidates,
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
    if let Some(selected) = &authority.selected_candidate {
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
    resolve_metadata_fields(
        &item.title,
        &item.subtitle,
        authority,
        candidates,
        preferred_language,
    )
}

pub(crate) fn resolve_metadata_fields(
    inventory_title: &str,
    inventory_subtitle: &str,
    authority: &MetadataAuthority,
    candidates: &[MetadataCandidateObservation],
    preferred_language: &str,
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
    let automatic = if selected.is_none() {
        candidates
            .iter()
            .max_by(|left, right| compare_candidate(left, right, preferred_language))
    } else {
        None
    };
    let candidate = selected.or(automatic);
    let ambiguous = automatic.is_some_and(|best| {
        candidates.iter().any(|other| {
            other.identity != best.identity && same_policy_rank(best, other, preferred_language)
        })
    });
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
                    source: MetadataValueSource::Manual,
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
                    source: MetadataValueSource::Manual,
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
        || authority.domain_overrides.is_some();
    Ok(ResolvedMetadataDisplay {
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

fn compare_candidate(
    left: &MetadataCandidateObservation,
    right: &MetadataCandidateObservation,
    preferred_language: &str,
) -> Ordering {
    policy_rank(left, preferred_language)
        .cmp(&policy_rank(right, preferred_language))
        .then_with(|| right.identity.cmp(&left.identity))
}

fn same_policy_rank(
    left: &MetadataCandidateObservation,
    right: &MetadataCandidateObservation,
    preferred_language: &str,
) -> bool {
    policy_rank(left, preferred_language) == policy_rank(right, preferred_language)
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
            .is_some_and(|value| value.eq_ignore_ascii_case(preferred))
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

fn read_exact_snapshot_bytes(
    processing: &dyn crate::ProcessingPort,
    snapshot: &crate::SnapshotRecord,
    maximum: u64,
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
    reader
        .by_ref()
        .take(snapshot.bytes.0.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|error| Error::Unavailable(format!("provider snapshot read: {error}")))?;
    if bytes.len() as u64 != snapshot.bytes.0 || Digest::sha256(&bytes) != snapshot.digest {
        return Err(Error::Invalid(
            "provider snapshot length or digest mismatch".into(),
        ));
    }
    Ok(bytes)
}

fn exact_json_or_snapshot_reference(
    processing: &dyn crate::ProcessingPort,
    snapshot: &crate::SnapshotRecord,
    maximum: u64,
) -> Result<RawJson> {
    let bytes = read_exact_snapshot_bytes(processing, snapshot, maximum)?;
    if bytes.is_empty() {
        return Ok(RawJson::from_value(serde_json::json!({
            "provider":snapshot.provider,
            "snapshot_id":snapshot.id,
            "snapshot_digest":snapshot.digest,
            "outcome":"no_match"
        })));
    }
    // A provider 404 may have an HTML/plain-text body. Its exact bytes remain
    // archived; the authority links that snapshot rather than inventing JSON.
    match String::from_utf8(bytes)
        .ok()
        .and_then(|json| RawJson::from_json(json).ok())
    {
        Some(raw) => Ok(raw),
        None => Ok(RawJson::from_value(serde_json::json!({
            "provider":snapshot.provider, "snapshot_id":snapshot.id,
            "snapshot_digest":snapshot.digest, "observation":snapshot.parsed_observation
        }))),
    }
}
