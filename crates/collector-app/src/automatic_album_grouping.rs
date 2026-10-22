//! Membership is independent of the chosen album metadata variant.
use crate::*;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub struct AutomaticAlbumGroupCandidate {
    pub item_id: ItemId,
    pub user_id: UserId,
}
const COMPONENT_LIMIT: usize = 200;
const INPUT_BYTE_LIMIT: usize = 32 * 1024 * 1024;
type GroupRecords = Vec<(Item, Option<MetadataAuthorityRecord>)>;

#[derive(Clone, Default)]
struct DiscOption {
    unproven: bool,
    positions: BTreeMap<RepresentationId, u16>,
    components: BTreeMap<ComponentId, BTreeSet<u16>>,
    registered: BTreeMap<String, u16>,
    candidates: BTreeSet<MetadataCandidateIdentity>,
}
struct Membership {
    item: Item,
    metadata: MetadataAuthorityRecord,
    options: BTreeMap<String, DiscOption>,
    accepted: bool,
    idle: bool,
}
fn placement_compatible(left: &DiscOption, right: &DiscOption) -> bool {
    !left.unproven
        && !right.unproven
        && !left
            .positions
            .values()
            .any(|position| right.positions.values().any(|other| position == other))
        && left.registered.iter().all(|(id, position)| {
            right
                .registered
                .get(id)
                .is_none_or(|other| other == position)
        })
}
fn membership_compatible(left: &Membership, right: &Membership) -> bool {
    left.options.keys().eq(right.options.keys())
        && left
            .options
            .iter()
            .all(|(group, option)| placement_compatible(option, &right.options[group]))
}
fn metadata_digest<T: Serialize>(value: &T) -> Result<Digest> {
    Ok(Digest::sha256(
        &serde_json::to_vec(value).map_err(|error| Error::Storage(error.to_string()))?,
    ))
}

impl CollectionService {
    pub fn reconcile_automatic_album_groups(&self) -> Result<()> {
        let mut after = None;
        let mut visited = BTreeSet::new();
        loop {
            let candidates = self.grouping.automatic_album_group_candidates(after, 100)?;
            if candidates.is_empty() {
                break;
            }
            after = candidates.last().map(|candidate| candidate.item_id);
            for candidate in candidates {
                if visited.contains(&candidate.item_id) {
                    continue;
                }
                match self.try_automatic_album_component(candidate, &mut visited) {
                    Ok(())
                    | Err(
                        Error::Conflict
                        | Error::NotFound
                        | Error::Forbidden
                        | Error::Invalid(_)
                        | Error::Unavailable(_),
                    ) => {}
                    Err(error) => return Err(error),
                }
            }
        }
        Ok(())
    }

    fn try_automatic_album_component(
        &self,
        candidate: AutomaticAlbumGroupCandidate,
        visited: &mut BTreeSet<ItemId>,
    ) -> Result<()> {
        let actor = Principal {
            user_id: candidate.user_id,
            administrator: false,
            device_id: None,
        };
        let seed = self.reader.item(candidate.item_id)?;
        if seed.deleted {
            return Ok(());
        }
        self.authorize(&actor, seed.collection_id, Action::Curate)?;
        let mut pending = VecDeque::from([seed.id]);
        let mut seen = BTreeSet::new();
        while let Some(id) = pending.pop_front() {
            if !seen.insert(id) {
                continue;
            }
            if seen.len() > COMPONENT_LIMIT {
                visited.extend(seen);
                return Ok(());
            }
            let peers =
                self.care
                    .album_group_proposals(actor.user_id, id, None, COMPONENT_LIMIT as u16)?;
            if peers.next_cursor.is_some() {
                visited.extend(seen);
                return Ok(());
            }
            pending.extend(
                peers
                    .items
                    .into_iter()
                    .map(|item| item.id)
                    .filter(|id| !seen.contains(id)),
            );
        }
        visited.extend(&seen);
        let mut guards = Vec::new();
        let mut groups = BTreeSet::new();
        let mut profiles = Vec::new();
        let mut bytes = 0;
        for id in seen {
            let item = self.reader.item(id)?;
            if item.collection_id != seed.collection_id {
                return Err(Error::Forbidden);
            }
            let metadata = self.processing.metadata_authority(item.collection_id, id)?;
            let history = self
                .processing
                .metadata_candidates(item.collection_id, id)?;
            bytes += serde_json::to_vec(&(&item, &metadata, &history))
                .map_err(|error| Error::Storage(error.to_string()))?
                .len();
            if bytes > INPUT_BYTE_LIMIT {
                return Ok(());
            }
            guards.push((id, metadata_digest(&item)?, metadata_digest(&metadata)?));
            if let Some(record) = &metadata {
                match music_membership_album_keys(&item, record, &history) {
                    Ok(keys) => groups.extend(keys),
                    Err(Error::Storage(error)) => return Err(Error::Storage(error)),
                    Err(_) => continue,
                }
            }
            match self.album_membership(&actor, item, metadata, &history) {
                Ok(Some(profile)) => profiles.push(profile),
                Err(Error::Storage(error)) => return Err(Error::Storage(error)),
                Ok(None) | Err(_) => {}
            }
        }
        // A resolved target may admit an unresolved sibling only when no other
        // established album is an equally compatible destination for that disc.
        let narrowing = profiles
            .iter()
            .map(|profile| {
                if profile.accepted {
                    return None;
                }
                let targets = profiles
                    .iter()
                    .filter(|other| other.accepted && other.item.id != profile.item.id)
                    .filter_map(|other| {
                        let (group, placement) = other.options.first_key_value()?;
                        profile
                            .options
                            .get(group)
                            .filter(|option| placement_compatible(option, placement))
                            .map(|_| group.clone())
                    })
                    .collect::<BTreeSet<_>>();
                Some(targets)
            })
            .collect::<Vec<_>>();
        for (profile, targets) in profiles.iter_mut().zip(narrowing) {
            if let Some(targets) = targets {
                if targets.len() > 1 {
                    profile.idle = false;
                } else if let Some(group) = targets.first() {
                    profile.options.retain(|key, _| key == group);
                }
            }
        }
        // Identical closed neighborhoods give safe cliques without choosing one
        // of several competing maximal subsets or letting an invalid peer veto all.
        // Establish a safe accepted subset before admitting unresolved siblings;
        // an unresolved duplicate must not compete with an already accepted disc.
        let mut ordered_classes = Vec::new();
        for accepted_only in [true, false] {
            let mut classes = BTreeMap::<Vec<usize>, Vec<usize>>::new();
            for (index, profile) in profiles
                .iter()
                .enumerate()
                .filter(|(_, profile)| profile.idle && (!accepted_only || profile.accepted))
            {
                let mut neighbors = vec![];
                for (other_index, other) in profiles
                    .iter()
                    .enumerate()
                    .filter(|(_, other)| other.idle && (!accepted_only || other.accepted))
                {
                    if index == other_index
                        || membership_compatible(profile, other)
                            && automatic_choices(
                                &[
                                    (&profile.item, Some(&profile.metadata)),
                                    (&other.item, Some(&other.metadata)),
                                ],
                                profile.item.id,
                            )?
                            .is_some()
                    {
                        neighbors.push(other_index);
                    }
                }
                classes.entry(neighbors).or_default().push(index);
            }
            let mut classes = classes
                .into_values()
                .filter(|class| class.len() >= 2)
                .collect::<Vec<_>>();
            classes.sort_by_key(|class| std::cmp::Reverse(class.len()));
            ordered_classes.extend(classes);
        }
        // One publication per component/pass. Remaining classes are reassessed
        // against fresh records next pass, including unresolved attachments.
        for class in ordered_classes {
            let mut selected = class
                .iter()
                .map(|index| &profiles[*index])
                .collect::<Vec<_>>();
            selected.sort_by_key(|profile| {
                (
                    std::cmp::Reverse(profile.item.captures.len()),
                    profile.item.id,
                )
            });
            selected.truncate(32);
            if self.publish_automatic_membership(&actor, &selected, &guards, &groups)? {
                break;
            }
        }
        Ok(())
    }

    fn album_membership(
        &self,
        actor: &Principal,
        item: Item,
        metadata: Option<MetadataAuthorityRecord>,
        history: &[MetadataCandidateObservation],
    ) -> Result<Option<Membership>> {
        if item.deleted
            || item.lifecycle.automatic_grouping_disabled
            || item.domain.module.0 != "org.junk.music"
            || item
                .lifecycle
                .reimport
                .as_ref()
                .is_some_and(|reimport| !reimport.completed)
        {
            return Ok(None);
        }
        let Some(metadata) = metadata else {
            return Ok(None);
        };
        let eligibility = self.content_write_eligibility(actor, item.collection_id)?;
        self.validate_owned_domain(&eligibility, &item.domain)?;
        let assignments = self.music_disc_assignments(&item)?;
        let mut sources = Vec::new();
        for capture in &item.captures {
            for source in capture
                .representations
                .iter()
                .filter(|source| source.role == RepresentationRole::PreservationMaster)
            {
                let Some(collector_modules::music::CdProcessingResult::Layout {
                    tracks,
                    disc_ids: Some(ids),
                    ..
                }) = self.recorded_music_layout(item.collection_id, source.id)?
                else {
                    return Ok(None);
                };
                sources.push((
                    source.id,
                    capture.association.component_id,
                    ids.musicbrainz,
                    tracks
                        .iter()
                        .map(|track| u16::from(track.position))
                        .collect::<Vec<_>>(),
                ));
            }
        }
        if sources.is_empty() || sources.len() > 128 {
            return Ok(None);
        }
        let mut options = BTreeMap::<String, DiscOption>::new();
        let mut rejected = BTreeSet::new();
        for candidate in music_membership_candidates(&item, &metadata, history)? {
            if candidate.identity.provider.0 != collector_modules::music::MUSICBRAINZ_PROVIDER_ID {
                return Ok(None);
            }
            let release: collector_modules::music::MusicBrainzReleaseObservation =
                candidate.observation.parse()?;
            release.validate()?;
            if release.release_id != candidate.identity.provider_record_id {
                return Ok(None);
            }
            let Some(group) = &release.release_group_id else {
                return Ok(None);
            };
            let option = options.entry(group.clone()).or_default();
            option.candidates.insert(candidate.identity);
            for medium in &release.media {
                for id in &medium.disc_ids {
                    if option
                        .registered
                        .insert(id.clone(), medium.position)
                        .is_some_and(|position| position != medium.position)
                    {
                        rejected.insert(group.clone());
                    }
                }
            }
            for (id, component, disc, tracks) in &sources {
                let (medium, state) = crate::derivatives::resolve_music_medium(
                    Some(&release),
                    Some(disc),
                    None,
                    Some(tracks),
                );
                if state != MusicDiscMatchState::DiscIdMatched {
                    rejected.insert(group.clone());
                    continue;
                }
                let position = medium.unwrap().position;
                if assignments.iter().any(|assignment| {
                    assignment.source_representation_id == *id
                        && assignment.medium_position != position
                }) {
                    rejected.insert(group.clone());
                }
                if option
                    .positions
                    .insert(*id, position)
                    .is_some_and(|previous| previous != position)
                {
                    rejected.insert(group.clone());
                }
                if let Some(component) = component {
                    option
                        .components
                        .entry(*component)
                        .or_default()
                        .insert(position);
                }
            }
        }
        // Incomplete or conflicting coverage is not evidence against an album
        // option. Keep it unresolved until an established target narrows the
        // options, and require proof for every option used in a merge.
        for (group, option) in &mut options {
            option.unproven = rejected.contains(group);
        }
        if options.is_empty() {
            return Ok(None);
        }
        let display = resolve_record_metadata(&item, &metadata, history)?;
        let accepted =
            options.len() == 1 && display.resolution != MetadataResolutionKind::Tentative;
        let idle = self.grouping.automatic_album_group_idle(&[item.id])?;
        Ok(Some(Membership {
            item,
            metadata,
            options,
            accepted,
            idle,
        }))
    }

    fn publish_automatic_membership(
        &self,
        actor: &Principal,
        profiles: &[&Membership],
        guards: &[(ItemId, Digest, Digest)],
        groups: &BTreeSet<String>,
    ) -> Result<bool> {
        let records: GroupRecords = profiles
            .iter()
            .map(|profile| (profile.item.clone(), Some(profile.metadata.clone())))
            .collect();
        let Some(mut choices) = automatic_choices(
            &records
                .iter()
                .map(|(item, record)| (item, record.as_ref()))
                .collect::<Vec<_>>(),
            profiles[0].item.id,
        )?
        else {
            return Ok(false);
        };
        let shared = profiles[0].options.keys().cloned().collect::<BTreeSet<_>>();
        let mut positions = BTreeMap::<ComponentId, BTreeSet<u16>>::new();
        let mut allowed = BTreeSet::new();
        for profile in profiles {
            for group in &shared {
                let option = &profile.options[group];
                allowed.extend(option.candidates.iter().cloned());
                for (component, values) in &option.components {
                    positions.entry(*component).or_default().extend(values);
                }
            }
        }
        choices.disc_positions = positions
            .into_iter()
            .filter_map(|(component_id, values)| {
                (values.len() == 1).then(|| GroupDiscPosition {
                    component_id,
                    position: *values.first().unwrap(),
                })
            })
            .collect();
        let command = PreviewAlbumGroup {
            destination: profiles[0].item.id,
            entries: profiles
                .iter()
                .map(|profile| GroupEntry {
                    item_id: profile.item.id,
                    expected_revision: profile.item.revision,
                })
                .collect(),
            choices,
        };
        let records = self.album_group_records(actor, &command)?;
        let key = metadata_digest(&(&command, &records, &shared))?.hex;
        let group = TransactionId(deterministic_inventory_uuid(
            actor.user_id,
            "automatic-album-group",
            &key,
        ));
        let (mut publication, conflicts) = self.compose_album_group(&records, &command, group)?;
        if !conflicts.is_empty() {
            return Ok(false);
        }
        publication.result.automatic = true;
        publication.membership_guards = guards.to_vec();
        publication.membership_groups = groups.iter().cloned().collect();
        for entry in &mut publication.entries {
            if let Some(metadata) = &mut entry.metadata_after {
                metadata.current_candidates = Some(allowed.iter().cloned().collect());
                if metadata
                    .authority
                    .automatic_candidate
                    .as_ref()
                    .is_some_and(|id| !allowed.contains(id))
                {
                    metadata.authority.automatic_candidate = None;
                }
                let history = metadata
                    .observations
                    .iter()
                    .flat_map(|observation| observation.candidates.clone())
                    .collect::<Vec<_>>();
                metadata.display = Some(resolve_record_metadata(&entry.after, metadata, &history)?);
                for observation in &mut metadata.observations {
                    observation.raw_observation = RawJson::from_value(
                        serde_json::json!({"basis":"automatic_musicbrainz_disc_group","album_options":shared,
                      "sources":records.iter().map(|(item,record)|serde_json::json!({"item_id":item.id,"inventory_revision":item.revision,"metadata_revision":record.as_ref().map(|record|record.revision),"selected_candidate":record.as_ref().and_then(|record|record.authority.selected_candidate.clone())})).collect::<Vec<_>>() }),
                    );
                }
            }
        }
        self.grouping
            .publish_album_group(actor.user_id, publication)?;
        Ok(true)
    }
}

fn same<T: Serialize>(left: &T, right: &T) -> Result<bool> {
    Ok(
        serde_json::to_vec(left).map_err(|error| Error::Storage(error.to_string()))?
            == serde_json::to_vec(right).map_err(|error| Error::Storage(error.to_string()))?,
    )
}

/// Preserve compatible explicit authority. Provisional filenames do not create a
/// conflict; independent inventory/unknown data and conflicting manual choices do.
fn automatic_choices(
    records: &[(&Item, Option<&MetadataAuthorityRecord>)],
    destination: ItemId,
) -> Result<Option<AlbumGroupChoices>> {
    let first = &records[0].0;
    for (item, _) in records {
        if item.wishlist != first.wishlist
            || item.identity != first.identity
            || item.identity_link != first.identity_link
            || !same(&item.domain, &first.domain)?
            || !same(&item.translations, &first.translations)?
            || !same(&item.identifiers, &first.identifiers)?
            || !same(&item.metadata, &first.metadata)?
        {
            return Ok(None);
        }
    }
    let authorities = records
        .iter()
        .map(|(item, record)| {
            record
                .as_ref()
                .map(|record| metadata_authority_for_inventory(item, record))
                .transpose()
        })
        .collect::<Result<Vec<_>>>()?;
    if authorities.iter().any(Option::is_none) {
        return Ok(None);
    }
    let authorities = authorities
        .into_iter()
        .map(Option::unwrap)
        .collect::<Vec<_>>();
    for authority in &authorities {
        if !same(
            &authority.retained_unknown_fields,
            &authorities[0].retained_unknown_fields,
        )? || !same(
            &authority.preserved_field_overrides,
            &authorities[0].preserved_field_overrides,
        )? {
            return Ok(None);
        }
    }
    let choose = |values: Vec<(ItemId, serde_json::Value)>| -> Option<ItemId> {
        let first = values.first()?;
        values
            .iter()
            .all(|(_, value)| value == &first.1)
            .then_some(first.0)
    };
    let mut choices = AlbumGroupChoices {
        title_from: Some(destination),
        subtitle_from: Some(destination),
        identity_from: Some(destination),
        domain_from: Some(destination),
        artwork_from: Some(destination),
        ..Default::default()
    };
    for field in [MetadataField::Title, MetadataField::Subtitle] {
        let values = authorities
            .iter()
            .zip(records)
            .filter_map(|(authority, (item, _))| {
                authority
                    .field_overrides
                    .iter()
                    .find(|value| value.field == field)
                    .map(|value| (item.id, serde_json::to_value(value)))
            })
            .map(|(id, value)| {
                value
                    .map(|value| (id, value))
                    .map_err(|error| Error::Storage(error.to_string()))
            })
            .collect::<Result<Vec<_>>>()?;
        if !values.is_empty() {
            let Some(source) = choose(values) else {
                return Ok(None);
            };
            if field == MetadataField::Title {
                choices.title_from = Some(source);
            } else {
                choices.subtitle_from = Some(source);
            }
        }
    }
    // Callers first prove common album options and compatible placement. Explicit
    // choices may name different releases or versions of that same album. Retain
    // the destination's manual choice when present, otherwise a stable manual
    // choice, then an established automatic identity rather than a tentative one.
    let preferred = |manual: bool| {
        authorities
            .iter()
            .zip(records)
            .filter(|(authority, _)| {
                if manual {
                    authority.selected_candidate.is_some()
                } else {
                    authority.automatic_candidate.is_some()
                }
            })
            .map(|(_, (item, _))| item.id)
            .min_by_key(|id| (*id != destination, *id))
    };
    choices.identity_from = preferred(true)
        .or_else(|| preferred(false))
        .or(Some(destination));
    let domains = authorities
        .iter()
        .zip(records)
        .filter_map(|(authority, (item, _))| {
            authority
                .domain_overrides
                .as_ref()
                .map(|value| (item.id, serde_json::to_value(value)))
        })
        .map(|(id, value)| {
            value
                .map(|value| (id, value))
                .map_err(|error| Error::Storage(error.to_string()))
        })
        .collect::<Result<Vec<_>>>()?;
    if !domains.is_empty() {
        let Some(source) = choose(domains) else {
            return Ok(None);
        };
        choices.domain_from = Some(source);
    }
    let mut artwork = Vec::new();
    for (authority, (item, _)) in authorities.iter().zip(records) {
        if let Some(id) = authority.artwork_choice {
            let Some(asset) = item.assets.iter().find(|asset| asset.id == id) else {
                return Ok(None);
            };
            artwork.push((
                item.id,
                serde_json::to_value((&asset.file.digest, asset.visibility))
                    .map_err(|error| Error::Storage(error.to_string()))?,
            ));
        }
    }
    if !artwork.is_empty() {
        let Some(source) = choose(artwork) else {
            return Ok(None);
        };
        choices.artwork_from = Some(source);
    }
    Ok(Some(choices))
}
