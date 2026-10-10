//! Confirmed inventory grouping policy. Preview freezes every affected record;
//! the adapter publishes the resulting revisions as one archive commit.
use crate::*;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct GroupEntry {
    pub item_id: ItemId,
    pub expected_revision: Revision,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AlbumGroupChoices {
    pub title_from: Option<ItemId>,
    pub subtitle_from: Option<ItemId>,
    pub identity_from: Option<ItemId>,
    pub domain_from: Option<ItemId>,
    pub artwork_from: Option<ItemId>,
    #[serde(default)]
    pub assemblies: Vec<PhysicalSetAssembly>,
    #[serde(default)]
    pub disc_positions: Vec<GroupDiscPosition>,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PhysicalSetAssembly {
    pub destination: SpecimenId,
    pub sources: Vec<SpecimenId>,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct GroupDiscPosition {
    pub component_id: ComponentId,
    pub position: u16,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PreviewAlbumGroup {
    pub destination: ItemId,
    pub entries: Vec<GroupEntry>,
    #[serde(default)]
    pub choices: AlbumGroupChoices,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ConfirmAlbumGroup {
    pub preview: PreviewAlbumGroup,
    pub token: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum GroupConflictKind {
    ManualTitle,
    ManualSubtitle,
    IdentityLink,
    DomainInformation,
    Artwork,
    PhysicalInventory,
    ActiveLoan,
    Possession,
    ChangedRecord,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct GroupConflict {
    pub kind: GroupConflictKind,
    pub message: String,
}
#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
pub struct AlbumGroupPreview {
    pub token: String,
    pub destination: ItemId,
    pub entries: Vec<Item>,
    pub conflicts: Vec<GroupConflict>,
    pub can_confirm: bool,
    pub physical_copies: u32,
    pub proposed_disc_positions: Vec<GroupDiscPosition>,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AlbumGroupResult {
    pub group_id: TransactionId,
    pub destination: ItemId,
    pub sources: Vec<ItemId>,
}
#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
pub struct AlbumGroupUndo {
    pub group: AlbumGroupResult,
    pub can_undo: bool,
    pub reason: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GroupedItemRevision {
    pub before: Item,
    pub after: Item,
    pub metadata_before: Option<MetadataAuthorityRecord>,
    pub metadata_after: Option<MetadataAuthorityRecord>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AlbumGroupPublication {
    pub result: AlbumGroupResult,
    pub entries: Vec<GroupedItemRevision>,
    pub undo_of: Option<TransactionId>,
}
pub trait AlbumGroupingPort: Send + Sync {
    fn publish_album_group(
        &self,
        user: UserId,
        publication: AlbumGroupPublication,
    ) -> Result<AlbumGroupResult>;
    fn album_group_publication(&self, group: TransactionId) -> Result<AlbumGroupPublication>;
}

fn identical<T: Serialize>(left: &T, right: &T) -> Result<bool> {
    Ok(
        serde_json::to_vec(left).map_err(|error| Error::Invalid(error.to_string()))?
            == serde_json::to_vec(right).map_err(|error| Error::Invalid(error.to_string()))?,
    )
}

impl CollectionService {
    fn album_group_records(
        &self,
        actor: &Principal,
        command: &PreviewAlbumGroup,
    ) -> Result<Vec<(Item, Option<MetadataAuthorityRecord>)>> {
        if !(2..=32).contains(&command.entries.len())
            || command.choices.assemblies.len() > 32
            || command.choices.disc_positions.len() > 1024
        {
            return Err(Error::Invalid(
                "Choose 2–32 entries and at most 1,024 disc positions for one album group.".into(),
            ));
        }
        let mut ids = BTreeSet::new();
        let mut collection = None;
        let mut module = None;
        let mut records = Vec::new();
        for entry in &command.entries {
            if !ids.insert(entry.item_id) {
                return Err(Error::Invalid(
                    "An entry appears more than once in the group.".into(),
                ));
            }
            let item = self.reader.item(entry.item_id)?;
            self.authorize(actor, item.collection_id, Action::Curate)?;
            let eligibility = self.content_write_eligibility(actor, item.collection_id)?;
            self.validate_owned_domain(&eligibility, &item.domain)?;
            if item.deleted
                || item
                    .lifecycle
                    .reimport
                    .as_ref()
                    .is_some_and(|value| !value.completed)
                || item.revision != entry.expected_revision
            {
                return Err(Error::Conflict);
            }
            if collection.is_some_and(|id| id != item.collection_id)
                || module
                    .as_ref()
                    .is_some_and(|owner| owner != &item.domain.module)
            {
                return Err(Error::Invalid(
                    "Album grouping requires entries of one type in the same collection.".into(),
                ));
            }
            collection = Some(item.collection_id);
            module = Some(item.domain.module.clone());
            let metadata = self
                .processing
                .metadata_authority(item.collection_id, item.id)?;
            records.push((item, metadata));
        }
        if !ids.contains(&command.destination) {
            return Err(Error::Invalid(
                "The destination must be one of the reviewed entries.".into(),
            ));
        }
        for choice in [
            command.choices.title_from,
            command.choices.subtitle_from,
            command.choices.identity_from,
            command.choices.domain_from,
            command.choices.artwork_from,
        ]
        .into_iter()
        .flatten()
        {
            if !ids.contains(&choice) {
                return Err(Error::Invalid(
                    "A field choice names an entry outside this group.".into(),
                ));
            }
        }
        Ok(records)
    }
    fn automatic_group_positions(
        &self,
        actor: &Principal,
        records: &[(Item, Option<MetadataAuthorityRecord>)],
        command: &PreviewAlbumGroup,
    ) -> Result<Vec<GroupDiscPosition>> {
        let mut positions = BTreeMap::new();
        for (item, _) in records {
            if item.domain.module.0 != "org.junk.music" {
                continue;
            }
            let coverage = self.music_album_coverage(actor, item.id)?;
            for source in coverage
                .sources
                .into_iter()
                .filter(|s| s.match_state == crate::MusicDiscMatchState::DiscIdMatched)
            {
                let Some(position) = source.medium_position else {
                    continue;
                };
                let Some(component) = item
                    .captures
                    .iter()
                    .find(|c| c.id == source.capture_id)
                    .and_then(|c| c.association.component_id)
                else {
                    continue;
                };
                if positions
                    .insert(component, position)
                    .is_some_and(|old| old != position)
                {
                    return Err(Error::Invalid("Alternative dumps disagree on this physical disc's album position. Resolve their metadata before grouping.".into()));
                }
            }
        }
        for choice in &command.choices.disc_positions {
            positions.remove(&choice.component_id);
        }
        Ok(positions
            .into_iter()
            .map(|(component_id, position)| GroupDiscPosition {
                component_id,
                position,
            })
            .collect())
    }
    pub fn preview_album_group(
        &self,
        actor: &Principal,
        command: PreviewAlbumGroup,
    ) -> Result<AlbumGroupPreview> {
        let records = self.album_group_records(actor, &command)?;
        let proposed_disc_positions = self.automatic_group_positions(actor, &records, &command)?;
        let mut effective = command.clone();
        effective
            .choices
            .disc_positions
            .extend(proposed_disc_positions.clone());
        let (_, conflicts) =
            self.compose_album_group(&records, &effective, TransactionId::new())?;
        let token = Digest::sha256(
            &serde_json::to_vec(&(&command, &records, &proposed_disc_positions))
                .map_err(|error| Error::Invalid(error.to_string()))?,
        )
        .hex;
        Ok(AlbumGroupPreview {
            token,
            destination: command.destination,
            physical_copies: records
                .iter()
                .map(|(item, _)| item.specimens.len() as u32)
                .sum(),
            entries: records.into_iter().map(|(item, _)| item).collect(),
            proposed_disc_positions,
            can_confirm: conflicts.is_empty(),
            conflicts,
        })
    }
    pub fn confirm_album_group(
        &self,
        actor: &Principal,
        command: ConfirmAlbumGroup,
        context: &CommandContext,
    ) -> Result<AlbumGroupResult> {
        self.execute_command(actor, context, "album.group.confirm", &command, |effects| {
            let records = self.album_group_records(actor, &command.preview)?;
            let proposed_disc_positions =
                self.automatic_group_positions(actor, &records, &command.preview)?;
            let token = Digest::sha256(
                &serde_json::to_vec(&(&command.preview, &records, &proposed_disc_positions))
                    .map_err(|error| Error::Invalid(error.to_string()))?,
            )
            .hex;
            if command.token != token {
                return Err(Error::Conflict);
            }
            let group = TransactionId(deterministic_inventory_uuid(
                actor.user_id,
                "album.group",
                context.key(),
            ));
            let (publication, conflicts) = self.compose_album_group(
                &records,
                &{
                    let mut effective = command.preview.clone();
                    effective
                        .choices
                        .disc_positions
                        .extend(proposed_disc_positions);
                    effective
                },
                group,
            )?;
            if !conflicts.is_empty() {
                return Err(Error::Invalid(
                    "Resolve every grouping conflict before confirming.".into(),
                ));
            }
            effects.publish_album_group(actor.user_id, publication)
        })
    }
    fn compose_album_group(
        &self,
        records: &[(Item, Option<MetadataAuthorityRecord>)],
        command: &PreviewAlbumGroup,
        group: TransactionId,
    ) -> Result<(AlbumGroupPublication, Vec<GroupConflict>)> {
        let destination = records
            .iter()
            .find(|(item, _)| item.id == command.destination)
            .ok_or(Error::NotFound)?;
        let mut result = destination.0.clone();
        let mut conflicts = Vec::new();
        let conflict = |kind, message: &str| GroupConflict {
            kind,
            message: message.into(),
        };
        let chosen = |choice: Option<ItemId>| {
            records
                .iter()
                .find(|(item, _)| item.id == choice.unwrap_or(command.destination))
                .ok_or(Error::NotFound)
        };
        if command.choices.title_from.is_none()
            && records.iter().any(|(item, _)| item.title != result.title)
        {
            conflicts.push(conflict(
                GroupConflictKind::ManualTitle,
                "Choose the title to keep; the entries have different inventory titles.",
            ));
        }
        if command.choices.subtitle_from.is_none()
            && records
                .iter()
                .any(|(item, _)| item.subtitle != result.subtitle)
        {
            conflicts.push(conflict(
                GroupConflictKind::ManualSubtitle,
                "Choose the artist or edition text to keep.",
            ));
        }
        if command.choices.identity_from.is_none()
            && records
                .iter()
                .any(|(item, _)| item.identity_link != result.identity_link)
        {
            conflicts.push(conflict(
                GroupConflictKind::IdentityLink,
                "Choose the destination identity link.",
            ));
        }
        for (item, _) in records {
            if command.choices.domain_from.is_none() && !identical(&item.domain, &result.domain)? {
                conflicts.push(conflict(GroupConflictKind::DomainInformation,"Choose the album's descriptive module information; all original values remain in group history."));
                break;
            }
        }
        for (field, choice, kind) in [
            (
                MetadataField::Title,
                command.choices.title_from,
                GroupConflictKind::ManualTitle,
            ),
            (
                MetadataField::Subtitle,
                command.choices.subtitle_from,
                GroupConflictKind::ManualSubtitle,
            ),
        ] {
            let values = records
                .iter()
                .map(|(item, record)| {
                    let authority = record
                        .as_ref()
                        .map(|r| metadata_authority_for_inventory(item, r))
                        .transpose()?;
                    Ok(authority
                        .and_then(|a| a.field_overrides.into_iter().find(|v| v.field == field))
                        .map(|v| v.value))
                })
                .collect::<Result<BTreeSet<_>>>()?;
            if choice.is_none() && values.len() > 1 {
                conflicts.push(conflict(kind,"Choose which manual metadata value to keep; the entries have different saved values."));
            }
        }
        let mut album_ids = BTreeSet::new();
        for (source, record) in records {
            if let Some(record) = record {
                let candidates = self
                    .processing
                    .metadata_candidates(source.collection_id, source.id)?;
                if let Some(id) = crate::accepted_album_group_key(source, record, &candidates)? {
                    album_ids.insert(id);
                }
            }
        }
        if album_ids.len() > 1 && command.choices.identity_from.is_none() {
            conflicts.push(conflict(GroupConflictKind::IdentityLink,"These entries have different accepted album identities. Choose the album identity to keep."));
        }
        let artworks = records
            .iter()
            .filter_map(|(_, record)| {
                record
                    .as_ref()
                    .and_then(|value| value.authority.artwork_choice)
            })
            .collect::<BTreeSet<_>>();
        if artworks.len() > 1 && command.choices.artwork_from.is_none() {
            conflicts.push(conflict(
                GroupConflictKind::Artwork,
                "Choose the selected artwork to keep; every image is retained.",
            ));
        }
        result.title = chosen(command.choices.title_from)?.0.title.clone();
        result.subtitle = chosen(command.choices.subtitle_from)?.0.subtitle.clone();
        result.identity_link = chosen(command.choices.identity_from)?.0.identity_link;
        result.domain = chosen(command.choices.domain_from)?.0.domain.clone();
        result.specimens = records
            .iter()
            .flat_map(|(item, _)| item.specimens.clone())
            .collect();
        let mut positions = BTreeMap::new();
        for mapping in &command.choices.disc_positions {
            if mapping.position == 0
                || positions
                    .insert(mapping.component_id, mapping.position)
                    .is_some()
            {
                return Err(Error::Invalid(
                    "Disc positions must be positive and each disc mapped once.".into(),
                ));
            }
        }
        for specimen in &mut result.specimens {
            for component in &mut specimen.components {
                if let Some(position) = positions.remove(&component.id) {
                    component.position = position;
                }
            }
            specimen.components.sort_by_key(|value| value.position);
        }
        if !positions.is_empty() {
            return Err(Error::Invalid(
                "A disc mapping names a component outside the group.".into(),
            ));
        }
        let mut assembled = BTreeSet::new();
        let mut specimen_map = BTreeMap::new();
        for assembly in &command.choices.assemblies {
            let mut all = vec![assembly.destination];
            all.extend(assembly.sources.clone());
            if all.len() < 2 || all.iter().any(|id| !assembled.insert(*id)) {
                return Err(Error::Invalid(
                    "Map each physical copy to at most one complete set.".into(),
                ));
            }
            let members = all
                .iter()
                .map(|id| {
                    result
                        .specimens
                        .iter()
                        .find(|value| value.id == *id)
                        .cloned()
                        .ok_or(Error::Invalid(
                            "A copy mapping names a copy outside this group.".into(),
                        ))
                })
                .collect::<Result<Vec<_>>>()?;
            if members
                .iter()
                .any(|value| value.loans.iter().any(|loan| loan.returned_at.is_none()))
            {
                conflicts.push(conflict(
                    GroupConflictKind::ActiveLoan,
                    "Copies with active loans cannot be combined into one physical set.",
                ));
                continue;
            }
            if members.iter().any(|value| {
                !identical(&value.possession, &members[0].possession).unwrap_or(false)
                    || !identical(&value.ownership, &members[0].ownership).unwrap_or(false)
            }) {
                conflicts.push(conflict(
                    GroupConflictKind::Possession,
                    "Copies with contradictory possession or ownership cannot be combined.",
                ));
                continue;
            }
            if members.iter().any(|copy| copy.components.is_empty()) {
                conflicts.push(conflict(GroupConflictKind::PhysicalInventory,
                    "A physical set mapping needs recorded disc components. Add the missing discs under the entry’s Copies section, then review their positions here. Keep copies distinct until their physical inventory is known."));
                continue;
            }
            let mut merged = members[0].clone();
            merged.components = members
                .iter()
                .flat_map(|value| value.components.clone())
                .collect();
            merged.loans = members
                .iter()
                .flat_map(|value| value.loans.clone())
                .collect();
            let mut occupied = BTreeSet::new();
            if merged
                .components
                .iter()
                .map(|value| value.position)
                .any(|position| !occupied.insert(position))
            {
                conflicts.push(conflict(GroupConflictKind::PhysicalInventory,"Two discs occupy the same position in the proposed physical set; map their positions explicitly."));
                continue;
            }
            for id in &all {
                specimen_map.insert(*id, assembly.destination);
            }
            result.specimens.retain(|value| !all.contains(&value.id));
            result.specimens.push(merged);
        }
        result.captures = records
            .iter()
            .flat_map(|(item, _)| item.captures.clone())
            .map(|mut capture| {
                capture.association.item_id = Some(result.id);
                if let Some(id) = capture
                    .association
                    .specimen_id
                    .and_then(|id| specimen_map.get(&id))
                {
                    capture.association.specimen_id = Some(*id);
                }
                capture
            })
            .collect();
        result.assets = records
            .iter()
            .flat_map(|(item, _)| item.assets.clone())
            .map(|mut asset| {
                match &mut asset.subject {
                    AssetSubject::Item { item_id } => *item_id = result.id,
                    AssetSubject::Specimen { specimen_id } => {
                        if let Some(id) = specimen_map.get(specimen_id) {
                            *specimen_id = *id
                        }
                    }
                    _ => {}
                }
                asset
            })
            .collect();
        let destination_work = match result.identity_link {
            ItemIdentityLink::Work { work_id } => Some(work_id),
            ItemIdentityLink::Edition { edition_id } => self.reader.edition(edition_id)?.work_id,
            ItemIdentityLink::None => None,
        };
        result.assets.retain(|asset| match asset.subject {
            AssetSubject::Work { work_id } => Some(work_id) == destination_work,
            AssetSubject::Edition { edition_id } => {
                result.identity_link == ItemIdentityLink::Edition { edition_id }
            }
            _ => true,
        });
        let mut asset_ids = BTreeSet::new();
        result.assets.retain(|value| asset_ids.insert(value.id));
        result.lifecycle.historical_captures = records
            .iter()
            .flat_map(|(item, _)| item.lifecycle.historical_captures.clone())
            .collect();
        result.revision = Revision(result.revision.0.checked_add(1).ok_or(Error::Conflict)?);
        result.updated_at = self.authentication.now();
        let sources = records
            .iter()
            .filter(|(item, _)| item.id != result.id)
            .map(|(item, _)| item.id)
            .collect::<Vec<_>>();
        let group_metadata = self.compose_group_metadata(records, command, &result, group)?;
        let mut entries = Vec::new();
        for (before, metadata) in records {
            let mut after = if before.id == result.id {
                result.clone()
            } else {
                let mut item = before.clone();
                item.specimens.clear();
                item.captures.clear();
                item.assets.retain(|asset| {
                    matches!(
                        asset.subject,
                        AssetSubject::Work { .. } | AssetSubject::Edition { .. }
                    )
                });
                item.deleted = true;
                item.lifecycle.merged_into = Some(result.id);
                item.lifecycle.generation = item
                    .lifecycle
                    .generation
                    .checked_add(1)
                    .ok_or(Error::Conflict)?;
                item.revision = Revision(item.revision.0.checked_add(1).ok_or(Error::Conflict)?);
                item.updated_at = result.updated_at;
                item
            };
            after.lifecycle.merge_group = Some(group);
            if let Err(error) = after.validate() {
                conflicts.push(conflict(
                    GroupConflictKind::PhysicalInventory,
                    &error.to_string(),
                ));
            }
            entries.push(GroupedItemRevision {
                before: before.clone(),
                after,
                metadata_before: metadata.clone(),
                metadata_after: if before.id == result.id {
                    Some(group_metadata.clone())
                } else {
                    None
                },
            });
        }
        Ok((
            AlbumGroupPublication {
                result: AlbumGroupResult {
                    group_id: group,
                    destination: result.id,
                    sources,
                },
                entries,
                undo_of: None,
            },
            conflicts,
        ))
    }
    fn compose_group_metadata(
        &self,
        records: &[(Item, Option<MetadataAuthorityRecord>)],
        command: &PreviewAlbumGroup,
        item: &Item,
        group: TransactionId,
    ) -> Result<MetadataAuthorityRecord> {
        let chosen = |choice: Option<ItemId>| {
            records
                .iter()
                .find(|(item, _)| item.id == choice.unwrap_or(command.destination))
                .ok_or(Error::NotFound)
        };
        let destination = chosen(None)?;
        let mut authority = destination
            .1
            .as_ref()
            .map(|record| metadata_authority_for_inventory(&destination.0, record))
            .transpose()?
            .unwrap_or_default();
        for (field, choice) in [
            (MetadataField::Title, command.choices.title_from),
            (MetadataField::Subtitle, command.choices.subtitle_from),
        ] {
            let source = chosen(choice)?;
            let source_authority = source
                .1
                .as_ref()
                .map(|record| metadata_authority_for_inventory(&source.0, record))
                .transpose()?;
            let selected = source_authority
                .as_ref()
                .and_then(|record| {
                    record
                        .field_overrides
                        .iter()
                        .find(|value| value.field == field)
                })
                .cloned();
            authority
                .field_overrides
                .retain(|value| value.field != field);
            if let Some(selected) = selected {
                authority.field_overrides.push(selected);
            } else if source.1.is_none() {
                authority.field_overrides.push(MetadataFieldOverride {
                    field,
                    value: if field == MetadataField::Title {
                        source.0.title.clone()
                    } else {
                        source.0.subtitle.clone()
                    },
                    language: None,
                });
            }
        }
        if let Some(source) = command.choices.artwork_from {
            authority.artwork_choice = chosen(Some(source))?
                .1
                .as_ref()
                .and_then(|record| record.authority.artwork_choice);
        }
        if let Some(source) = command.choices.domain_from {
            authority.domain_overrides = chosen(Some(source))?
                .1
                .as_ref()
                .and_then(|record| record.authority.domain_overrides.clone());
        }
        if let Some(source) = command.choices.identity_from {
            let selected = chosen(Some(source))?.1.as_ref().map(|r| &r.authority);
            authority.selected_candidate = selected.and_then(|a| a.selected_candidate.clone());
            authority.automatic_candidate = selected.and_then(|a| a.automatic_candidate.clone());
            authority.automatic_language = selected.and_then(|a| a.automatic_language.clone());
        }
        let mut candidates = BTreeMap::new();
        for (source, _) in records {
            for candidate in self
                .processing
                .metadata_candidates(source.collection_id, source.id)?
            {
                if let Some(previous) =
                    candidates.insert(candidate.identity.clone(), candidate.clone())
                    && previous != candidate
                {
                    return Err(Error::Conflict);
                }
            }
        }
        if candidates.len() > 4096 {
            return Err(Error::Invalid(
                "Album group has too many retained metadata versions for one resolution.".into(),
            ));
        }
        let candidates = candidates.into_values().collect::<Vec<_>>();
        let current_candidates = Some(
            candidates
                .iter()
                .map(|value| value.identity.clone())
                .collect(),
        );
        let observations = vec![MetadataRefreshObservation {
            id: ObservationId(group.0),
            outcome: if candidates.is_empty() {
                MetadataRefreshOutcome::NoMatch
            } else {
                MetadataRefreshOutcome::Candidates
            },
            candidates: candidates.clone(),
            raw_observation: RawJson::from_value(
                serde_json::json!({"basis":"confirmed_album_group","sources":records.iter().map(|(item,record)|serde_json::json!({"item_id":item.id,"inventory_revision":item.revision,"metadata_revision":record.as_ref().map(|record|record.revision)})).collect::<Vec<_>>()}),
            ),
            observed_at: item.updated_at,
        }];
        let display = crate::metadata::resolve_metadata_display(
            item,
            &authority,
            &candidates,
            authority
                .automatic_language
                .as_ref()
                .map_or("en", LanguageTag::as_str),
        )?;
        let record = MetadataAuthorityRecord {
            resolution_policy: 3,
            current_candidates,
            inventory: Some(item.into()),
            collection_id: item.collection_id,
            item_id: item.id,
            revision: Revision(
                destination
                    .1
                    .as_ref()
                    .map_or(0, |record| record.revision.0)
                    .checked_add(1)
                    .ok_or(Error::Conflict)?,
            ),
            authority,
            observations,
            display: Some(display),
            updated_at: item.updated_at,
        };
        record.validate()?;
        record.validate_display(&candidates)?;
        Ok(record)
    }
    pub fn album_group_undo(
        &self,
        actor: &Principal,
        group: TransactionId,
    ) -> Result<AlbumGroupUndo> {
        let publication = self.grouping.album_group_publication(group)?;
        for entry in &publication.entries {
            self.authorize(actor, entry.after.collection_id, Action::Curate)?;
        }
        let mut reason = None;
        for entry in &publication.entries {
            let item = self.reader.item(entry.after.id)?;
            if !identical(&item, &entry.after)? {
                reason=Some("An affected entry, copy, capture or image changed after grouping; automatic undo is unavailable.".into());
                break;
            }
            let authority = self
                .processing
                .metadata_authority(item.collection_id, item.id)?;
            if !identical(
                &authority,
                &entry
                    .metadata_after
                    .as_ref()
                    .or(entry.metadata_before.as_ref())
                    .cloned(),
            )? {
                reason =
                    Some("Metadata changed after grouping; automatic undo is unavailable.".into());
                break;
            }
        }
        Ok(AlbumGroupUndo {
            group: publication.result,
            can_undo: reason.is_none(),
            reason,
        })
    }
    pub fn undo_album_group(
        &self,
        actor: &Principal,
        group: TransactionId,
        context: &CommandContext,
    ) -> Result<AlbumGroupResult> {
        self.execute_command(actor, context, "album.group.undo", &group, |effects| {
            if !self.album_group_undo(actor, group)?.can_undo {
                return Err(Error::Conflict);
            }
            let original = self.grouping.album_group_publication(group)?;
            let mut inverse = original.clone();
            inverse.undo_of = Some(group);
            inverse.result.group_id = TransactionId(deterministic_inventory_uuid(
                actor.user_id,
                "album.group.undo",
                context.key(),
            ));
            for entry in &mut inverse.entries {
                std::mem::swap(&mut entry.before, &mut entry.after);
                entry.after.revision = Revision(
                    entry
                        .before
                        .revision
                        .0
                        .checked_add(1)
                        .ok_or(Error::Conflict)?,
                );
                entry.after.updated_at = self.authentication.now();
                let current = entry
                    .metadata_after
                    .clone()
                    .or(entry.metadata_before.clone());
                let mut restored =
                    entry
                        .metadata_before
                        .clone()
                        .unwrap_or_else(|| MetadataAuthorityRecord {
                            resolution_policy: 3,
                            current_candidates: Some(vec![]),
                            inventory: Some((&entry.after).into()),
                            collection_id: entry.after.collection_id,
                            item_id: entry.after.id,
                            revision: Revision(0),
                            authority: MetadataAuthority::default(),
                            observations: vec![],
                            display: None,
                            updated_at: entry.after.updated_at,
                        });
                if current.is_some() {
                    restored.revision = Revision(
                        current
                            .as_ref()
                            .map_or(0, |value| value.revision.0)
                            .checked_add(1)
                            .ok_or(Error::Conflict)?,
                    );
                    restored.inventory = Some((&entry.after).into());
                    restored.observations.clear();
                    restored.updated_at = entry.after.updated_at;
                    let history = self
                        .processing
                        .metadata_candidates(restored.collection_id, restored.item_id)?;
                    restored.display =
                        Some(resolve_record_metadata(&entry.after, &restored, &history)?);
                    entry.metadata_after = Some(restored);
                } else {
                    entry.metadata_after = None;
                }
                entry.metadata_before = current;
            }
            effects.publish_album_group(actor.user_id, inverse)
        })
    }
}
