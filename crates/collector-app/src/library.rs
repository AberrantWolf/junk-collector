//! Compact, rebuildable Library presentation; full album reads stay in detail.
use crate::*;

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct LibraryArtwork {
    pub id: AssetId,
    pub revision: Revision,
    pub alt_text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct LibraryEntry {
    pub id: ItemId,
    pub collection_id: CollectionId,
    pub title: String,
    pub subtitle: String,
    pub context: String,
    pub module: ModuleId,
    pub identity: IdentityState,
    pub wishlist: bool,
    pub deleted: bool,
    pub can_edit: bool,
    pub artwork: Option<LibraryArtwork>,
    pub metadata_tentative: bool,
    pub import_in_progress: bool,
    pub album_playback: Option<collector_api_types::AlbumPlaybackReadiness>,
    pub digital_audio: bool,
    pub accuraterip: Option<Vec<PresentedAudioCheck>>,
}

/// Storage-selected identity and cache generation, never a full capture graph.
pub struct LibraryEntrySource {
    pub id: ItemId,
    pub collection_id: CollectionId,
    pub role: Role,
    pub generation: String,
    pub cached: Option<LibraryEntry>,
}

impl From<PresentedItem> for LibraryEntry {
    fn from(item: PresentedItem) -> Self {
        let artwork = item
            .assets
            .iter()
            .find(|asset| Some(asset.id) == item.primary_artwork)
            .map(|asset| LibraryArtwork {
                id: asset.id,
                revision: asset.revision,
                alt_text: asset.alt_text.clone(),
            });
        Self {
            id: item.id,
            collection_id: item.collection_id,
            title: item.title,
            subtitle: item.subtitle,
            context: item
                .presentation
                .fields
                .into_iter()
                .filter(|field| field.label != "Collection type")
                .take(2)
                .map(|field| field.value)
                .collect::<Vec<_>>()
                .join(" · "),
            module: item.domain.module,
            identity: item.identity,
            wishlist: item.wishlist,
            deleted: item.deleted,
            can_edit: item.content_permissions.can_edit,
            artwork,
            metadata_tentative: item.effective_metadata.resolution
                == MetadataResolutionKind::Tentative,
            // Operational progress is refreshed separately on every read.
            import_in_progress: false,
            album_playback: item.album_playback,
            digital_audio: item.digital_audio,
            accuraterip: item.accuraterip,
        }
    }
}

impl CollectionService {
    pub fn library_entries(
        &self,
        actor: &Principal,
        query: BrowseQuery,
    ) -> Result<Page<LibraryEntry>> {
        // A publication can invalidate a row while its cold value is being
        // calculated. Retry observation, never publish against a newer token.
        let mut retries = 0;
        loop {
            match self.library_entries_once(actor, query.clone()) {
                Err(Error::Conflict) if retries < 2 => retries += 1,
                result => return result,
            }
        }
    }

    fn library_entries_once(
        &self,
        actor: &Principal,
        query: BrowseQuery,
    ) -> Result<Page<LibraryEntry>> {
        if let Some(collection) = query.collection_id {
            self.authorize(
                actor,
                collection,
                if query.filter == BrowseFilter::Removed {
                    Action::Curate
                } else {
                    Action::Browse
                },
            )?;
        }
        let language = self.metadata_language(actor.user_id)?;
        let namespace = Digest::sha256(
            &serde_json::to_vec(&(
                "library-row-v1",
                env!("CARGO_PKG_VERSION"),
                self.modules.descriptors(),
            ))
            .map_err(|error| Error::Storage(error.to_string()))?,
        )
        .hex;
        let attention = query.filter == BrowseFilter::AlbumPlaybackNeedsAttention;
        let limit = usize::from(query.limit.unwrap_or(50).clamp(1, 200));
        let mut query = query;
        let mut items = Vec::new();
        let mut examined = 0;
        let next_cursor = loop {
            query.limit = Some((limit - items.len()).min(200 - examined) as u16);
            let page = self.care.library_entry_sources(
                actor.user_id,
                query.clone(),
                &namespace,
                &language,
            )?;
            examined += page.items.len();
            for source in page.items {
                let role = self.authorize(
                    actor,
                    source.collection_id,
                    if query.filter == BrowseFilter::Removed {
                        Action::Curate
                    } else {
                        Action::Browse
                    },
                )?;
                // A concurrent grant change cannot return a previously selected
                // curator cache entry to a newly downgraded viewer.
                if role != source.role {
                    return Err(Error::Conflict);
                }
                let mut entry = if let Some(cached) = source.cached {
                    cached
                } else {
                    let item = self.reader.item(source.id)?;
                    if item.collection_id != source.collection_id {
                        return Err(Error::Conflict);
                    }
                    let effective = self.effective_metadata_for_language(&item, &language)?;
                    let entry = LibraryEntry::from(self.presented(item, role, actor, effective)?);
                    if !self.care.cache_library_entry(
                        &namespace,
                        &language,
                        role,
                        &source.generation,
                        &entry,
                    )? {
                        // Never label a calculation from mixed revisions current.
                        return Err(Error::Conflict);
                    }
                    entry
                };
                entry.import_in_progress = !entry.deleted
                    && entry.module.0 == "org.junk.music"
                    && self.provisional_imports.item_has_active_ingest(entry.id)?;
                if !attention
                    || (!entry.import_in_progress
                        && matches!(
                            entry.album_playback,
                            Some(
                                collector_api_types::AlbumPlaybackReadiness::NeedsAttention { .. }
                            )
                        ))
                {
                    items.push(entry);
                }
            }
            if !attention || items.len() == limit || examined >= 200 || page.next_cursor.is_none() {
                break page.next_cursor;
            }
            query.cursor = page.next_cursor;
        };
        Ok(Page { items, next_cursor })
    }
}
