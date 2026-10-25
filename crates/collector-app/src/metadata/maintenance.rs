//! Derived edition fit and non-destructive consolidation of equivalent versions.
use super::*;

/// Prefer referenced identities, then a current identity. Never merge distinct
/// provider records or candidates with different artwork, ranking or observations.
pub(super) fn consolidate_matches(
    candidates: &[MetadataCandidateObservation],
    current: &[MetadataCandidateIdentity],
    preferred: &[&MetadataCandidateIdentity],
) -> Result<(
    Vec<MetadataCandidateObservation>,
    Vec<MetadataCandidateIdentity>,
)> {
    let current_set = current.iter().collect::<BTreeSet<_>>();
    let mut groups: BTreeMap<String, Vec<&MetadataCandidateObservation>> = BTreeMap::new();
    for candidate in candidates {
        let mut value =
            serde_json::to_value(candidate).map_err(|e| Error::Storage(e.to_string()))?;
        value["identity"]["provider_record_version"] = serde_json::Value::Null;
        let key = serde_json::to_string(&value).map_err(|e| Error::Storage(e.to_string()))?;
        groups.entry(key).or_default().push(candidate);
    }
    let mut result = Vec::new();
    let mut active = Vec::new();
    for group in groups.into_values() {
        let representative = group
            .iter()
            .min_by_key(|candidate| {
                (
                    preferred
                        .iter()
                        .position(|id| **id == candidate.identity)
                        .unwrap_or(usize::MAX),
                    !current_set.contains(&candidate.identity),
                    &candidate.identity,
                )
            })
            .expect("nonempty equivalence group");
        if group
            .iter()
            .any(|candidate| current_set.contains(&candidate.identity))
        {
            active.push(representative.identity.clone());
        }
        result.push((*representative).clone());
    }
    result.sort_by(|a, b| a.identity.cmp(&b.identity));
    active.sort();
    Ok((result, active))
}

#[derive(Default)]
struct DiscEvidence {
    sources: BTreeSet<RepresentationId>,
    ids: BTreeMap<RepresentationId, String>,
    assignments: Vec<collector_modules::music::MusicDiscAssignment>,
}

impl CollectionService {
    /// Only reads recorded, fingerprint-bound observations, never disc bytes.
    fn edition_evidence(&self, item: &Item, authority: &MetadataAuthority) -> Result<DiscEvidence> {
        let mut evidence = DiscEvidence::default();
        if let Some(overrides) = &authority.domain_overrides
            && overrides.module.0 == "org.junk.music"
            && overrides.schema_version == 1
        {
            evidence.assignments = overrides
                .payload
                .parse::<collector_modules::music::MusicMetadataOverrides>()?
                .disc_assignments;
        }
        for representation in item
            .captures
            .iter()
            .flat_map(|capture| &capture.representations)
        {
            evidence.sources.insert(representation.id);
            if let Some(collector_modules::music::CdProcessingResult::Layout {
                disc_ids: Some(ids),
                ..
            }) = self.recorded_music_layout(item.collection_id, representation.id)?
            {
                evidence.ids.insert(representation.id, ids.musicbrainz);
            }
        }
        Ok(evidence)
    }

    pub(super) fn reassess_music_edition(
        &self,
        item: &Item,
        authority: &mut MetadataAuthority,
        candidates: &[MetadataCandidateObservation],
        language: &str,
    ) -> Result<()> {
        if item.domain.module.0 != "org.junk.music"
            || item.domain.schema_version != 1
            || authority.selected_candidate.is_some()
        {
            return Ok(());
        }
        let evidence = self.edition_evidence(item, authority)?;
        let accepted = authority.automatic_candidate.as_ref().and_then(|id| {
            candidates
                .iter()
                .find(|candidate| &candidate.identity == id)
        });
        // An accepted album remains stable when another album appears in a later lookup.
        let anchor = accepted.or_else(|| automatic_metadata_candidate(candidates, language).0);
        let Some(anchor) = anchor else {
            return Ok(());
        };
        let best = candidates
            .iter()
            .filter(|candidate| {
                candidate.complete_identity
                    && candidate.strength != MatchStrength::Name
                    && same_metadata_album(anchor, candidate)
            })
            .max_by(|left, right| {
                edition_fit(left, &evidence)
                    .cmp(&edition_fit(right, &evidence))
                    .then_with(|| compare_candidate(left, right, language))
            });
        if let Some(best) = best {
            authority.automatic_candidate = Some(best.identity.clone());
        }
        Ok(())
    }

    /// Revisit all pages periodically, including entries imported before this policy.
    /// Immutable history/source bytes remain untouched; only current choices change.
    pub fn reconcile_music_metadata(&self) -> Result<()> {
        let mut after = None;
        loop {
            let page = self
                .processing
                .metadata_maintenance_candidates(after, 100)?;
            if page.is_empty() {
                break;
            }
            after = page.last().map(|candidate| candidate.item_id);
            for candidate in page {
                match self.reconcile_music_entry(candidate) {
                    Ok(())
                    | Err(
                        Error::Conflict
                        | Error::NotFound
                        | Error::Forbidden
                        | Error::Unavailable(_)
                        | Error::Invalid(_),
                    ) => {}
                    Err(error) => return Err(error),
                }
            }
        }
        Ok(())
    }

    fn reconcile_music_entry(&self, candidate: crate::MetadataMaintenanceCandidate) -> Result<()> {
        let actor = Principal {
            user_id: candidate.user_id,
            administrator: false,
            device_id: None,
        };
        let item = self.reader.item(candidate.item_id)?;
        let eligibility = self.content_write_eligibility(&actor, item.collection_id)?;
        self.validate_owned_domain(&eligibility, &item.domain)?;
        if item.deleted || item.domain.module.0 != "org.junk.music" {
            return Ok(());
        }
        let Some(old) = self
            .processing
            .metadata_authority(item.collection_id, item.id)?
        else {
            return Ok(());
        };
        let mut authority = metadata_authority_for_inventory(&item, &old)?;
        let history = self
            .processing
            .metadata_candidates(item.collection_id, item.id)?;
        let current = record_current_candidates(&item, &old)
            .map(|ids| ids.to_vec())
            .unwrap_or_else(|| history.iter().map(|c| c.identity.clone()).collect());
        let active = current_metadata_candidates(&authority, &history, Some(&current))?;
        let language = authority
            .automatic_language
            .as_ref()
            .map_or("en", LanguageTag::as_str)
            .to_owned();
        self.reassess_music_edition(&item, &mut authority, &active, &language)?;
        let preferred = authority
            .selected_candidate
            .iter()
            .chain(authority.automatic_candidate.iter())
            .collect::<Vec<_>>();
        let (_, current) = consolidate_matches(&history, &current, &preferred)?;
        let display = resolve_metadata_display(
            &item,
            &authority,
            &current_metadata_candidates(&authority, &history, Some(&current))?,
            &language,
        )?;
        if old.resolution_policy == 4
            && old.current_candidates.as_ref() == Some(&current)
            && serde_json::to_value(&old.authority).map_err(|e| Error::Storage(e.to_string()))?
                == serde_json::to_value(&authority).map_err(|e| Error::Storage(e.to_string()))?
            && old.display.as_ref() == Some(&display)
        {
            return Ok(());
        }
        self.processing.publish_metadata_maintenance(
            actor.user_id,
            MetadataAuthorityRecord {
                resolution_policy: 4,
                current_candidates: Some(current),
                inventory: Some((&item).into()),
                collection_id: item.collection_id,
                item_id: item.id,
                revision: Revision(old.revision.0.checked_add(1).ok_or(Error::Conflict)?),
                authority,
                observations: vec![],
                display: Some(display),
                updated_at: self.authentication.now(),
            },
        )
    }
}

// Exact matching here refers to descriptive disc identity, not audio verification.
fn edition_fit(
    candidate: &MetadataCandidateObservation,
    evidence: &DiscEvidence,
) -> (bool, usize, std::cmp::Reverse<usize>) {
    let Some(layout) = music_layout(candidate) else {
        return (false, 0, std::cmp::Reverse(usize::MAX));
    };
    let mut matched = BTreeSet::new();
    let mut compatible = true;
    for (source, id) in &evidence.ids {
        let positions = layout
            .discs
            .iter()
            .filter(|disc| disc.disc_ids.contains(id))
            .collect::<Vec<_>>();
        if positions.len() != 1 {
            compatible = false;
            continue;
        }
        let position = positions[0].position;
        if evidence.assignments.iter().any(|assignment| {
            assignment.source_representation_id == *source
                && (assignment.release_id != layout.release_id
                    || assignment.medium_position != position)
        }) {
            compatible = false;
            continue;
        }
        matched.insert(position);
    }
    for assignment in evidence.assignments.iter().filter(|assignment| {
        evidence
            .sources
            .contains(&assignment.source_representation_id)
    }) {
        if assignment.release_id == layout.release_id
            && layout
                .discs
                .iter()
                .any(|disc| disc.position == assignment.medium_position)
        {
            if !evidence
                .ids
                .contains_key(&assignment.source_representation_id)
            {
                matched.insert(assignment.medium_position);
            }
        } else {
            compatible = false;
        }
    }
    // Unknown/unprocessed originals do not count as a missing or matching disc.
    let unrepresented = if evidence.ids.is_empty() && matched.is_empty() {
        0
    } else {
        layout.discs.len().saturating_sub(matched.len())
    };
    (compatible, matched.len(), std::cmp::Reverse(unrepresented))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate() -> MetadataCandidateObservation {
        MetadataCandidateObservation {
            identity: MetadataCandidateIdentity {
                provider: ModuleId(collector_modules::music::MUSICBRAINZ_PROVIDER_ID.into()),
                provider_record_id: "11111111-1111-4111-8111-111111111111".into(),
                provider_record_version: Some("one".into()),
            },
            title: MetadataText {
                value: "Album".into(),
                language: None,
            },
            subtitle: None,
            artwork: None,
            complete_identity: true,
            strength: MatchStrength::DiscLayout,
            provider_priority: 20,
            observation: RawJson::from_value(serde_json::json!({"title":"Album","tracks":["One"]})),
        }
    }

    #[test]
    fn consolidation_preserves_every_semantic_difference_and_referenced_identity() {
        let first = candidate();
        let mut duplicate = first.clone();
        duplicate.identity.provider_record_version = Some("two".into());
        duplicate.observation =
            RawJson::from_json("{ \"tracks\": [\"One\"], \"title\": \"Album\" }").unwrap();
        let mut differences = vec![];
        let mut art = first.clone();
        art.artwork = Some(AssetId::new());
        differences.push(art);
        let mut rank = first.clone();
        rank.provider_priority += 1;
        differences.push(rank);
        let mut text = first.clone();
        text.title.value = "Other title".into();
        differences.push(text);
        let mut evidence = first.clone();
        evidence.strength = MatchStrength::Name;
        differences.push(evidence);
        let mut incomplete = first.clone();
        incomplete.complete_identity = false;
        differences.push(incomplete);
        let mut observation = first.clone();
        observation.observation =
            RawJson::from_value(serde_json::json!({"title":"Album","tracks":["Changed"]}));
        differences.push(observation);
        let mut record = first.clone();
        record.identity.provider_record_id = "22222222-2222-4222-8222-222222222222".into();
        differences.push(record);
        let mut all = vec![first.clone(), duplicate.clone()];
        for (i, mut changed) in differences.into_iter().enumerate() {
            changed.identity.provider_record_version = Some(format!("different-{i}"));
            all.push(changed);
        }
        let (choices, current) = consolidate_matches(
            &all,
            std::slice::from_ref(&first.identity),
            &[&duplicate.identity],
        )
        .unwrap();
        assert_eq!(choices.len(), all.len() - 1);
        assert_eq!(current, vec![duplicate.identity.clone()]);
        assert!(choices.iter().any(|c| c.identity == duplicate.identity));
        assert!(!choices.iter().any(|c| c.identity == first.identity));
    }
}
