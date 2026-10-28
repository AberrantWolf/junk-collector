//! Compact, rebuildable Library presentation; full album reads stay in detail.
use crate::*;

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct LibraryArtwork {
    pub id: AssetId,
    pub revision: Revision,
    pub alt_text: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum LibrarySummaryState {
    Ready,
    Pending,
    Retrying,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct LibraryPage {
    pub items: Vec<LibraryEntry>,
    pub next_cursor: Option<String>,
    pub summary_state: LibrarySummaryState,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct LibraryEntry {
    pub summary_state: LibrarySummaryState,
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
    pub pending: LibraryEntry,
    pub retrying: bool,
}

/// One derived view required by an enabled user's current grant/preference.
pub struct LibraryEntryBuild {
    pub actor: Principal,
    pub id: ItemId,
    pub collection_id: CollectionId,
    pub role: Role,
    pub language: String,
    pub generation: String,
}

impl LibraryEntry {
    /// Cheap projection at inventory publication; no captures, assets or evidence.
    pub fn pending(item: &Item) -> Self {
        Self {
            summary_state: LibrarySummaryState::Pending,
            id: item.id,
            collection_id: item.collection_id,
            title: item.title.clone(),
            subtitle: item.subtitle.clone(),
            context: String::new(),
            module: item.domain.module.clone(),
            identity: item.identity.clone(),
            wishlist: item.wishlist,
            deleted: item.deleted,
            can_edit: false,
            artwork: None,
            metadata_tentative: false,
            import_in_progress: false,
            album_playback: None,
            digital_audio: false,
            accuraterip: None,
        }
    }
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
            summary_state: LibrarySummaryState::Ready,
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
    fn library_namespace(&self) -> Result<String> {
        Ok(Digest::sha256(
            &serde_json::to_vec(&(
                "library-row-v2",
                env!("CARGO_PKG_VERSION"),
                self.modules.descriptors(),
            ))
            .map_err(|error| Error::Storage(error.to_string()))?,
        )
        .hex)
    }

    /// A server worker calls this independently of browsing, one row at a time.
    /// Source generations remain dirty until a matching calculation is published.
    pub fn rebuild_next_library_entry(&self) -> Result<bool> {
        let namespace = self.library_namespace()?;
        let Some(build) = self.care.next_library_entry_build(&namespace)? else {
            return Ok(false);
        };
        let result = (|| {
            let role = self.authorize(&build.actor, build.collection_id, Action::Browse)?;
            if role != build.role {
                return Err(Error::Conflict);
            }
            let item = self.reader.item(build.id)?;
            if item.collection_id != build.collection_id {
                return Err(Error::Conflict);
            }
            let effective = self.effective_metadata_for_language(&item, &build.language)?;
            let entry = LibraryEntry::from(self.presented(item, role, &build.actor, effective)?);
            // A superseded token remains eligible for the next worker iteration.
            self.care.cache_library_entry(
                &namespace,
                &build.language,
                role,
                &build.generation,
                &entry,
            )?;
            Ok(())
        })();
        if result.is_err() {
            self.care.defer_library_entry_build(&namespace, &build)?;
        }
        result.map(|()| true)
    }

    pub fn library_entries(&self, actor: &Principal, query: BrowseQuery) -> Result<LibraryPage> {
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
        let namespace = self.library_namespace()?;
        let attention = query.filter == BrowseFilter::AlbumPlaybackNeedsAttention;
        let limit = usize::from(query.limit.unwrap_or(50).clamp(1, 200));
        let mut query = query;
        let mut items = Vec::new();
        let mut examined = 0;
        let mut summary_state = LibrarySummaryState::Ready;
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
                let mut entry = source.cached.unwrap_or_else(|| {
                    let mut entry = source.pending;
                    if source.retrying {
                        entry.summary_state = LibrarySummaryState::Retrying;
                    }
                    entry
                });
                if entry.summary_state == LibrarySummaryState::Retrying {
                    summary_state = LibrarySummaryState::Retrying;
                } else if entry.summary_state == LibrarySummaryState::Pending
                    && summary_state == LibrarySummaryState::Ready
                {
                    summary_state = LibrarySummaryState::Pending;
                }
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
        Ok(LibraryPage {
            items,
            next_cursor,
            summary_state,
        })
    }
}
