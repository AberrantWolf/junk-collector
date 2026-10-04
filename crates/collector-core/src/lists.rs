//! Portable ordered collection lists. Occurrence identities allow intentional repeats.
use crate::*;

pub const MAX_LIST_ENTRIES: usize = 5_000;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CollectionListKind {
    Items,
    MusicTracks,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ListTarget {
    Item {
        item_id: ItemId,
    },
    MusicTrack {
        item_id: ItemId,
        capture_id: CaptureId,
        source_representation_id: RepresentationId,
        track_position: u16,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CollectionListEntry {
    pub id: ListEntryId,
    pub target: ListTarget,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CollectionList {
    pub id: CollectionListId,
    pub collection_id: CollectionId,
    pub revision: Revision,
    pub name: String,
    pub kind: CollectionListKind,
    pub entries: Vec<CollectionListEntry>,
    pub deleted: bool,
}
impl CollectionList {
    pub fn validate(&self) -> Result<()> {
        if self.revision.0 == 0
            || self.name.trim().is_empty()
            || self.name.len() > 256
            || self.name.chars().any(char::is_control)
            || self.entries.len() > MAX_LIST_ENTRIES
        {
            return Err(Error::Invalid(
                "Give the list a name of at most 256 bytes; lists hold at most 5,000 entries."
                    .into(),
            ));
        }
        let mut ids = BTreeSet::new();
        for entry in &self.entries {
            if !ids.insert(entry.id)
                || !matches!(
                    (&self.kind, &entry.target),
                    (CollectionListKind::Items, ListTarget::Item { .. })
                        | (
                            CollectionListKind::MusicTracks,
                            ListTarget::MusicTrack {
                                track_position: 1..,
                                ..
                            }
                        )
                )
            {
                return Err(Error::Invalid(
                    "List entries must have unique occurrence IDs and match the list kind.".into(),
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn list_bounds_and_kind_allow_repeats_but_not_reused_occurrence_ids() {
        let target = ListTarget::Item {
            item_id: ItemId::new(),
        };
        let mut list = CollectionList {
            id: CollectionListId::new(),
            collection_id: CollectionId::new(),
            revision: Revision(1),
            name: "Reading list".into(),
            kind: CollectionListKind::Items,
            entries: vec![
                CollectionListEntry {
                    id: ListEntryId::new(),
                    target: target.clone(),
                },
                CollectionListEntry {
                    id: ListEntryId::new(),
                    target,
                },
            ],
            deleted: false,
        };
        list.validate().unwrap();
        list.entries[1].id = list.entries[0].id;
        assert!(list.validate().is_err());
        list.entries[1].id = ListEntryId::new();
        list.kind = CollectionListKind::MusicTracks;
        assert!(list.validate().is_err());
        list.kind = CollectionListKind::Items;
        list.entries = (0..MAX_LIST_ENTRIES)
            .map(|_| CollectionListEntry {
                id: ListEntryId::new(),
                target: ListTarget::Item {
                    item_id: ItemId::new(),
                },
            })
            .collect();
        list.validate().unwrap();
        list.entries.push(CollectionListEntry {
            id: ListEntryId::new(),
            target: ListTarget::Item {
                item_id: ItemId::new(),
            },
        });
        assert!(list.validate().is_err());
        list.entries.clear();
        list.name = " \n ".into();
        assert!(list.validate().is_err());
        list.name = "x".repeat(257);
        assert!(list.validate().is_err());
    }
}
