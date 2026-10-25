//! Collection-owned list policy; playback resolves current prepared outputs without reading bytes.
use crate::*;
use std::collections::{BTreeMap, BTreeSet};

pub use collector_api_types::{
    CollectionListChange, CollectionListSummary, CollectionListView, CollectionListsQuery,
    CreateCollectionList, ListEntriesQuery, ListEntryAvailability, PresentedListEntry,
    UpdateCollectionList,
};
#[derive(Clone, Debug)]
pub struct ListSourceRevision {
    pub item_id: ItemId,
    pub item_revision: Revision,
    pub metadata_revision: Revision,
    pub captures: Vec<(CaptureId, Revision)>,
}

pub trait CollectionListPort: Send + Sync {
    fn collection_lists(
        &self,
        collection: CollectionId,
        search: &str,
        cursor: Option<CollectionListId>,
        limit: u16,
    ) -> Result<Page<CollectionListSummary>>;
    fn collection_list(&self, id: CollectionListId) -> Result<CollectionList>;
    fn publish_collection_list(
        &self,
        user: UserId,
        list: CollectionList,
        expected: Revision,
        sources: Vec<ListSourceRevision>,
    ) -> Result<CollectionListSummary>;
}
fn summary(list: &CollectionList, can_edit: bool) -> CollectionListSummary {
    CollectionListSummary {
        id: list.id,
        collection_id: list.collection_id,
        revision: list.revision,
        name: list.name.clone(),
        kind: list.kind,
        entry_count: list.entries.len() as u32,
        can_edit,
    }
}
impl CollectionService {
    fn readable_list(
        &self,
        actor: &Principal,
        collection: CollectionId,
        id: CollectionListId,
    ) -> Result<CollectionList> {
        self.authorize(actor, collection, Action::Browse)?;
        let list = self.lists.collection_list(id)?;
        if list.collection_id != collection || list.deleted {
            return Err(Error::NotFound);
        }
        Ok(list)
    }
    pub fn collection_lists(
        &self,
        actor: &Principal,
        collection: CollectionId,
        query: CollectionListsQuery,
    ) -> Result<Page<CollectionListSummary>> {
        let role = self.authorize(actor, collection, Action::Browse)?;
        let search = query.search.unwrap_or_default();
        let limit = query.limit.unwrap_or(50);
        if search.len() > 256 || !(1..=100).contains(&limit) {
            return Err(Error::Invalid(
                "Use a search of at most 256 bytes and a page size of 1–100.".into(),
            ));
        }
        let mut page = self
            .lists
            .collection_lists(collection, &search, query.cursor, limit)?;
        for list in &mut page.items {
            list.can_edit = role.allows(Action::Curate);
        }
        Ok(page)
    }
    pub fn create_collection_list(
        &self,
        actor: &Principal,
        collection: CollectionId,
        command: CreateCollectionList,
        context: &CommandContext,
    ) -> Result<CollectionListSummary> {
        self.authorize(actor, collection, Action::Curate)?;
        self.execute_command(actor, context, "collection.list.create", &(collection, &command), |effects| {
            let binding = self.reader.collection(collection)?.binding;
            let default_kind = if matches!(binding, CollectionBinding::Assigned { ref module } if module.0 == "org.junk.music") { CollectionListKind::MusicTracks } else { CollectionListKind::Items };
            let kind = command.kind.unwrap_or(default_kind);
            if kind == CollectionListKind::MusicTracks && !matches!(&binding, CollectionBinding::Assigned { module } if self.modules.supports_listening(module)) {
                return Err(Error::Invalid("This collection does not support audio playlists.".into()));
            }
            let id = CollectionListId(deterministic_job_id(actor.user_id, "collection.list.create", context.key()).0);
            let list = CollectionList {id, collection_id: collection, revision: Revision(1), name: command.name.trim().into(), kind, entries: vec![], deleted: false};
            list.validate()?;
            effects.publish_collection_list(actor.user_id, list, Revision(0), vec![])
        })
    }
    pub fn update_collection_list(
        &self,
        actor: &Principal,
        collection: CollectionId,
        id: CollectionListId,
        command: UpdateCollectionList,
        context: &CommandContext,
    ) -> Result<CollectionListSummary> {
        self.authorize(actor, collection, Action::Curate)?;
        self.execute_command(actor, context, "collection.list.update", &(collection, id, &command), |effects| {
            let mut list = self.readable_list(actor, collection, id)?;
            if list.revision != command.expected_revision { return Err(Error::Conflict); }
            let mut sources = vec![];
            match &command.change {
                CollectionListChange::Rename {name} => list.name = name.trim().into(),
                CollectionListChange::Delete => list.deleted = true,
                CollectionListChange::Remove {entry_id} => {
                    let index = list.entries.iter().position(|e| e.id == *entry_id).ok_or(Error::Conflict)?;
                    list.entries.remove(index);
                }
                CollectionListChange::Move {entry_id, before} => {
                    if before == &Some(*entry_id) { return Err(Error::Invalid("Choose a different destination for this entry.".into())); }
                    let index = list.entries.iter().position(|e| e.id == *entry_id).ok_or(Error::Conflict)?;
                    let entry = list.entries.remove(index);
                    let to = before.map(|id| list.entries.iter().position(|e| e.id == id).ok_or(Error::Conflict)).transpose()?.unwrap_or(list.entries.len());
                    list.entries.insert(to, entry);
                }
                CollectionListChange::Append {item_id, source_representation_id, track_position} => {
                    let item = self.reader.item(*item_id)?;
                    if item.collection_id != collection { return Err(Error::Forbidden); }
                    if item.deleted { return Err(Error::NotFound); }
                    sources.push(ListSourceRevision {item_id: item.id, item_revision: item.revision, metadata_revision: self.processing.metadata_authority(item.collection_id,item.id)?.map_or(Revision(0),|record|record.revision), captures: item.captures.iter().map(|c|(c.id,c.revision)).collect()});
                    let targets = if list.kind == CollectionListKind::Items {
                        if source_representation_id.is_some() || track_position.is_some() { return Err(Error::Invalid("This list contains collection entries.".into())); }
                        vec![ListTarget::Item {item_id: item.id}]
                    } else { self.list_music_targets(actor, &item, *source_representation_id, *track_position)? };
                    if list.entries.len() + targets.len() > MAX_LIST_ENTRIES { return Err(Error::Invalid("This addition would exceed the list’s 5,000-entry limit. Remove entries or choose a smaller addition.".into())); }
                    list.entries.extend(targets.into_iter().map(|target| CollectionListEntry {id: ListEntryId::new(), target}));
                }
            }
            list.revision = Revision(list.revision.0.checked_add(1).ok_or(Error::Conflict)?);
            list.validate()?;
            effects.publish_collection_list(actor.user_id, list, command.expected_revision, sources)
        })
    }
    pub(super) fn list_music_targets(
        &self,
        actor: &Principal,
        item: &Item,
        source: Option<RepresentationId>,
        track: Option<u16>,
    ) -> Result<Vec<ListTarget>> {
        self.resolve_music_targets(actor, item, source, track, false)
    }

    pub(super) fn resolve_music_targets(
        &self,
        actor: &Principal,
        item: &Item,
        source: Option<RepresentationId>,
        track: Option<u16>,
        recorded_only: bool,
    ) -> Result<Vec<ListTarget>> {
        if track.is_some() && source.is_none() {
            return Err(Error::Invalid("Choose a source disc and track.".into()));
        }
        let rows = if recorded_only {
            self.music_track_availability_recorded(actor.user_id, item)?
        } else {
            self.music_track_availability_for_item(actor.user_id, item)?
        };
        let coverage = self.music_album_coverage(actor, item.id)?;
        // Availability can retain only prepared tracks when original bytes are
        // absent. A disc/album addition must not turn that partial view into a
        // successful-looking complete expansion. Recorded structure defines the
        // full audio scope; individual track additions remain deliberately narrow.
        if track.is_none() {
            if rows.iter().any(|r| {
                source.is_none_or(|id| id == r.source_representation_id)
                    && r.album_selection == collector_api_types::MusicAlbumSelection::NeedsReview
            }) {
                return Err(Error::Invalid("Choose which alternate encodings belong in the album using Edit track details. Individual originals remain playable.".into()));
            }
            for candidate in coverage.sources.iter().filter(|candidate| {
                source.is_none_or(|id| candidate.source_representation_id == id)
            }) {
                if let Some(manifest) = self
                    .recorded_file_tracks(item.collection_id, candidate.source_representation_id)?
                {
                    let available = rows
                        .iter()
                        .filter(|r| {
                            r.source_representation_id == candidate.source_representation_id
                        })
                        .map(|r| r.track_position)
                        .collect::<BTreeSet<_>>();
                    if manifest
                        .tracks
                        .iter()
                        .any(|t| !available.contains(&t.position))
                    {
                        return Err(Error::Unavailable("Some purchased tracks are unavailable; add individual available tracks instead.".into()));
                    }
                    continue;
                }
                if let Some(manifest) = self
                    .recorded_game_audio(item.collection_id, candidate.source_representation_id)?
                {
                    let recorded = manifest
                        .tracks
                        .iter()
                        .map(|t| t.position)
                        .collect::<BTreeSet<_>>();
                    let available = rows
                        .iter()
                        .filter(|r| {
                            r.primary_playback()
                                && r.source_representation_id == candidate.source_representation_id
                        })
                        .map(|r| r.track_position)
                        .collect::<BTreeSet<_>>();
                    if recorded != available {
                        return Err(Error::Unavailable(
                            "The complete game disc audio scope is unavailable.".into(),
                        ));
                    }
                    continue;
                }
                let Some(collector_modules::music::CdProcessingResult::Layout { tracks, .. }) =
                    self.recorded_music_layout(
                        item.collection_id,
                        candidate.source_representation_id,
                    )?
                else {
                    return Err(Error::Invalid("The complete disc track list is not recorded. Inspect the CD structure before adding the disc or album; individual available tracks can still be added.".into()));
                };
                let recorded = tracks
                    .iter()
                    .filter(|track| track.kind == CdTrackKind::Audio)
                    .map(|track| u16::from(track.position))
                    .collect::<BTreeSet<_>>();
                let available = rows
                    .iter()
                    .filter(|row| {
                        row.primary_playback()
                            && row.source_representation_id == candidate.source_representation_id
                    })
                    .map(|row| row.track_position)
                    .collect::<BTreeSet<_>>();
                if recorded != available {
                    return Err(Error::Invalid("The complete disc audio scope is unavailable. Restore or inspect its original files before adding the disc or album; individual available tracks can still be added.".into()));
                }
            }
        }
        let mut discs: BTreeMap<String, BTreeSet<RepresentationId>> = BTreeMap::new();
        for row in rows.iter().filter(|r| r.primary_playback()) {
            let medium = coverage
                .sources
                .iter()
                .find(|s| s.source_representation_id == row.source_representation_id)
                .and_then(|s| s.medium_position);
            let capture = item
                .captures
                .iter()
                .find(|c| c.id == row.capture_id)
                .ok_or(Error::Conflict)?;
            let key = medium.map(|p| format!("medium:{p:05}")).unwrap_or_else(|| {
                capture
                    .association
                    .carrier_id
                    .map(|id| format!("carrier:{id}"))
                    .unwrap_or_else(|| format!("capture:{}", capture.id))
            });
            discs
                .entry(key)
                .or_default()
                .insert(row.source_representation_id);
        }
        let selected = if let Some(source) = source {
            if !discs.values().any(|values| values.contains(&source)) {
                return Err(Error::Invalid("This original has no recorded supported audio tracks. Finish audio analysis first.".into()));
            }
            BTreeSet::from([source])
        } else {
            if discs.values().any(|values| values.len() != 1) {
                return Err(Error::Invalid("This album has alternative originals for a disc. Add each disc from the original you want to use.".into()));
            }
            if discs.len() > 1
                && coverage
                    .sources
                    .iter()
                    .filter(|s| {
                        discs
                            .values()
                            .any(|ids| ids.contains(&s.source_representation_id))
                    })
                    .any(|s| s.medium_position.is_none())
            {
                return Err(Error::Invalid("Assign each original to an album disc before adding the whole multi-disc album. You can add discs individually now.".into()));
            }
            discs.values().flat_map(|v| v.iter().copied()).collect()
        };
        let mut rows = rows
            .into_iter()
            .filter(|r| {
                r.primary_playback()
                    && selected.contains(&r.source_representation_id)
                    && track.is_none_or(|p| p == r.track_position)
                    && (track.is_some()
                        || r.album_selection == collector_api_types::MusicAlbumSelection::Included)
            })
            .collect::<Vec<_>>();
        rows.sort_by_key(|r| {
            (
                coverage
                    .sources
                    .iter()
                    .find(|s| s.source_representation_id == r.source_representation_id)
                    .and_then(|s| s.medium_position)
                    .unwrap_or(u16::MAX),
                r.disc_position.unwrap_or(u16::MAX),
                r.display_position.unwrap_or(r.track_position),
                r.track_position,
            )
        });
        if rows.is_empty() {
            return Err(Error::Invalid(
                "No recorded audio tracks are available. Finish audio analysis first.".into(),
            ));
        }
        Ok(rows
            .into_iter()
            .map(|r| ListTarget::MusicTrack {
                item_id: item.id,
                capture_id: r.capture_id,
                source_representation_id: r.source_representation_id,
                track_position: r.track_position,
            })
            .collect())
    }
    pub fn collection_list_view(
        &self,
        actor: &Principal,
        collection: CollectionId,
        id: CollectionListId,
        query: ListEntriesQuery,
    ) -> Result<CollectionListView> {
        let list = self.readable_list(actor, collection, id)?;
        let role = self.authorize(actor, collection, Action::Browse)?;
        let limit = query.limit.unwrap_or(100) as usize;
        if !(1..=100).contains(&limit) {
            return Err(Error::Invalid("List page size must be 1–100.".into()));
        }
        let start = if let Some(cursor) = query.cursor {
            let (revision, offset) = cursor.split_once(':').ok_or(Error::Conflict)?;
            if revision.parse::<u64>().ok() != Some(list.revision.0) {
                return Err(Error::Conflict);
            }
            offset.parse::<usize>().map_err(|_| Error::Conflict)?
        } else {
            0
        };
        if start > list.entries.len() {
            return Err(Error::Conflict);
        }
        let mut items = BTreeMap::<ItemId, PresentedItem>::new();
        let mut audio =
            BTreeMap::<ItemId, (Vec<MusicTrackAvailability>, MusicAlbumCoverage)>::new();
        let mut entries = vec![];
        for (index, entry) in list.entries.iter().enumerate().skip(start).take(limit) {
            let original_item = match entry.target {
                ListTarget::Item { item_id } | ListTarget::MusicTrack { item_id, .. } => item_id,
            };
            let item_id = match entry.target {
                ListTarget::MusicTrack { capture_id, .. } => {
                    match self.reader.capture(capture_id) {
                        Ok(capture) if capture.collection_id == collection => {
                            capture.association.item_id.unwrap_or(original_item)
                        }
                        Ok(_) | Err(Error::NotFound) => original_item,
                        Err(error) => return Err(error),
                    }
                }
                _ => original_item,
            };
            if let std::collections::btree_map::Entry::Vacant(slot) = items.entry(item_id) {
                match self.item_presented(actor, item_id) {
                    Ok(item) if item.collection_id == collection && !item.deleted => {
                        slot.insert(item);
                    }
                    Ok(_) | Err(Error::NotFound | Error::Forbidden) => {}
                    Err(error) => return Err(error),
                }
            }
            let mut view = PresentedListEntry {
                source: None,
                audio: None,
                id: entry.id,
                position: index as u32 + 1,
                item_id: None,
                title: "Removed or unavailable entry".into(),
                album: None,
                artist: None,
                disc_position: None,
                track_position: None,
                artwork: None,
                availability: ListEntryAvailability::EntryRemoved,
            };
            if let Some(item) = items.get(&item_id) {
                view.item_id = Some(item.id);
                view.source = Some(entry.target.clone());
                view.title = item.title.clone();
                view.artwork = item.primary_artwork;
                view.availability = ListEntryAvailability::Entry;
                if let ListTarget::MusicTrack {
                    capture_id,
                    source_representation_id,
                    track_position,
                    ..
                } = entry.target
                {
                    view.album = Some(item.title.clone());
                    view.artist = (!item.subtitle.is_empty()).then(|| item.subtitle.clone());
                    view.track_position = Some(track_position);
                    view.title = format!("Track {track_position:02}");
                    {
                        if let std::collections::btree_map::Entry::Vacant(slot) =
                            audio.entry(item.id)
                        {
                            slot.insert((
                                self.music_track_availability(actor, item.id)?,
                                self.music_album_coverage(actor, item.id)?,
                            ));
                        }
                        let (rows, coverage) = &audio[&item.id];
                        view.disc_position = coverage
                            .sources
                            .iter()
                            .find(|s| s.source_representation_id == source_representation_id)
                            .and_then(|s| s.medium_position);
                        let matches = rows
                            .iter()
                            .filter(|r| {
                                r.capture_id == capture_id
                                    && r.source_representation_id == source_representation_id
                                    && r.track_position == track_position
                            })
                            .collect::<Vec<_>>();
                        let preferred = matches
                            .iter()
                            .find(|r| {
                                r.primary_playback()
                                    && matches!(
                                        r.state,
                                        MusicTrackAvailabilityState::Available { .. }
                                    )
                            })
                            .or_else(|| {
                                matches.iter().find(|r| {
                                    matches!(r.state, MusicTrackAvailabilityState::Available { .. })
                                })
                            })
                            .or_else(|| matches.first());
                        view.availability = if let Some(row) = preferred {
                            view.title = row.title.clone();
                            view.audio = Some((*row).clone());
                            match &row.state {
                                MusicTrackAvailabilityState::Available {
                                    representation_id,
                                    file_index,
                                    media_type,
                                    ..
                                } => ListEntryAvailability::Available {
                                    representation_id: *representation_id,
                                    file_index: *file_index,
                                    media_type: media_type.clone(),
                                },
                                MusicTrackAvailabilityState::Preparing => {
                                    ListEntryAvailability::Preparing
                                }
                                MusicTrackAvailabilityState::Unavailable { .. } => {
                                    if self.modules.has_module(&item.domain.module) {
                                        ListEntryAvailability::NeedsPreparation
                                    } else {
                                        ListEntryAvailability::ModuleUnavailable
                                    }
                                }
                            }
                        } else {
                            if self.modules.has_module(&item.domain.module) {
                                ListEntryAvailability::SourceUnavailable
                            } else {
                                ListEntryAvailability::ModuleUnavailable
                            }
                        };
                    }
                }
            }
            entries.push(view);
        }
        let end = start + entries.len();
        Ok(CollectionListView {
            list: summary(&list, role.allows(Action::Curate)),
            entries: Page {
                items: entries,
                next_cursor: (end < list.entries.len())
                    .then(|| format!("{}:{end}", list.revision.0)),
            },
        })
    }
}
