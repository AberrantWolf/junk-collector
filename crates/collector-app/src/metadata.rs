//! Metadata authority and deterministic display resolution. Provider transport
//! belongs in adapters; this module consumes already-bounded observations.

use std::cmp::Ordering;
use std::collections::BTreeMap;

use chrono::Utc;
use collector_core::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

use crate::{CollectionService, CommandContext, MetadataAuthorityRecord, Principal};

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

#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
pub struct MetadataView {
    pub item_id: ItemId,
    pub revision: Revision,
    pub authority: MetadataAuthority,
    pub display: ResolvedMetadataDisplay,
    pub candidates: Vec<MetadataCandidateObservation>,
}

impl CollectionService {
    /// Curator detail. Raw candidate observations are intentionally not part of
    /// the viewer item DTO.
    pub fn metadata_view(&self, actor: &Principal, item_id: ItemId) -> Result<MetadataView> {
        let item = self.reader.item(item_id)?;
        self.authorize(actor, item.collection_id, Action::Curate)?;
        let (revision, authority, legacy) = self.authority_or_inventory(&item)?;
        let mut candidates = self
            .processing
            .metadata_candidates(item.collection_id, item.id)?;
        merge_metadata_candidates(
            &mut candidates,
            legacy.iter().flat_map(|value| &value.candidates),
        )?;
        let display = resolve_metadata_display(&item, &authority, &candidates, "en")?;
        Ok(MetadataView {
            item_id,
            revision,
            authority,
            display,
            candidates,
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
                let (revision, mut authority, legacy) = self.authority_or_inventory(&item)?;
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
                        &candidates,
                        "en",
                    )?),
                    authority,
                    observations: legacy,
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
            let (revision, authority, mut observations) = self.authority_or_inventory(&item)?;
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
            merge_metadata_candidates(&mut candidates, &command.observation.candidates)?;
            observations.push(command.observation.clone());
            self.modules
                .validate_metadata_authority(&eligibility.owner, &authority)
                .map_err(Error::from)?;
            let record = MetadataAuthorityRecord {
                inventory: Some((&item).into()),
                collection_id: item.collection_id,
                item_id,
                revision: Revision(revision.0.checked_add(1).ok_or(Error::Conflict)?),
                display: Some(resolve_metadata_display(
                    &item,
                    &authority,
                    &candidates,
                    "en",
                )?),
                authority,
                observations,
                updated_at: Utc::now(),
            };
            record.validate()?;
            effects.publish_metadata_authority(TransactionId::new(), record.clone())?;
            Ok(record)
        })
    }

    fn authority_or_inventory(
        &self,
        item: &Item,
    ) -> Result<(Revision, MetadataAuthority, Vec<MetadataRefreshObservation>)> {
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
            return Ok((record.revision, authority, Vec::new()));
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
        Ok((Revision(0), authority, observations))
    }
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
