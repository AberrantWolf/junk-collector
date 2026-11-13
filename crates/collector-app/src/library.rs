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
    pub catalog_revision: String,
    pub membership_revision: String,
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
    #[serde(default = "metadata_is_provisional")]
    pub metadata_provisional: bool,
    pub metadata_tentative: bool,
    pub import_in_progress: bool,
    pub album_playback: Option<collector_api_types::AlbumPlaybackReadiness>,
    pub digital_audio: bool,
    pub accuraterip: Option<Vec<PresentedAudioCheck>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub preparation: Vec<PreparationReadiness>,
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
    pub cursor: String,
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

fn metadata_is_provisional() -> bool {
    true
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
            metadata_provisional: true,
            metadata_tentative: false,
            import_in_progress: false,
            album_playback: None,
            digital_audio: false,
            accuraterip: None,
            preparation: Vec::new(),
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
            metadata_provisional: item.effective_metadata.title.source
                == MetadataValueSource::Provisional,
            metadata_tentative: item.effective_metadata.resolution
                == MetadataResolutionKind::Tentative,
            // Operational progress is refreshed separately on every read.
            import_in_progress: false,
            album_playback: item.album_playback,
            digital_audio: item.digital_audio,
            accuraterip: item.accuraterip,
            preparation: Vec::new(),
        }
    }
}

impl CollectionService {
    pub(crate) fn library_namespace(&self) -> Result<String> {
        Ok(Digest::sha256(
            &serde_json::to_vec(&(
                "library-row-v6",
                env!("CARGO_PKG_VERSION"),
                self.modules.descriptors(),
                self.encoder_availability(&ConversionProfile::Flac),
                self.encoder_availability(&ConversionProfile::Opus { bitrate_kbps: 96 }),
                self.encoder_availability(&ConversionProfile::Opus { bitrate_kbps: 128 }),
                self.encoder_availability(&ConversionProfile::Opus { bitrate_kbps: 192 }),
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
            let (tracks, incomplete_coverage) = if item.deleted {
                (Vec::new(), false)
            } else {
                self.recorded_preparation_tracks(build.actor.user_id, &item)?
            };
            let readiness = if role >= Role::Curator
                && !item.deleted
                && self.modules.supports_listening(&item.domain.module)
            {
                ConversionProfile::PRESETS
                    .iter()
                    .map(|profile| {
                        let plan =
                            crate::album_preparation::album_plan_for_item(&item, &tracks, profile);
                        PreparationReadiness::from_plan(&plan, incomplete_coverage)
                    })
                    .collect()
            } else {
                Vec::new()
            };
            let mut entry =
                LibraryEntry::from(self.presented(item, role, &build.actor, effective)?);
            entry.preparation = readiness;
            self.care.cache_library_tracks(
                &namespace,
                &build.language,
                role,
                &build.generation,
                &entry,
                &tracks,
            )?;
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

    pub fn library_summary(
        &self,
        actor: &Principal,
        mut query: BrowseQuery,
    ) -> Result<collector_api_types::CatalogSummary> {
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
        query.cursor = None;
        let (revision, membership) = self.care.library_read_revisions(query.filter)?;
        let total = self.care.library_count(
            actor.user_id,
            query.clone(),
            &self.library_namespace()?,
            &self.metadata_language(actor.user_id)?,
        )?;
        let (current, current_membership) = self.care.library_read_revisions(query.filter)?;
        if revision != current || membership != current_membership {
            return Err(Error::Conflict);
        }
        Ok(collector_api_types::CatalogSummary {
            total_items: total,
            total_is_exact: query.filter != BrowseFilter::AlbumPlaybackNeedsAttention,
            catalog_revision: revision,
        })
    }

    pub fn library_entries(&self, actor: &Principal, query: BrowseQuery) -> Result<LibraryPage> {
        self.library_page(actor, query, None, None)
    }

    pub fn refresh_library_rows(
        &self,
        actor: &Principal,
        mut query: BrowseQuery,
        ids: Vec<ItemId>,
        membership_revision: &str,
    ) -> Result<LibraryPage> {
        if ids.is_empty() || ids.len() > 50 || query.cursor.is_some() {
            return Err(Error::Invalid(
                "row refresh requires 1–50 IDs without a cursor".into(),
            ));
        }
        query.limit = Some(ids.len() as u16);
        self.library_page(actor, query, Some(&ids), Some(membership_revision))
    }

    fn library_page(
        &self,
        actor: &Principal,
        query: BrowseQuery,
        ids: Option<&[ItemId]>,
        expected_membership: Option<&str>,
    ) -> Result<LibraryPage> {
        let (revision, cursor_revision) = self.care.library_read_revisions(query.filter)?;
        if query
            .revision
            .as_ref()
            .is_some_and(|value| value != &revision)
        {
            return Err(Error::Conflict);
        }
        if expected_membership.is_some_and(|expected| expected != cursor_revision) {
            return Err(Error::Conflict);
        }
        let strict_revision = query.revision.is_some();
        let budget = query.max_bytes.unwrap_or(64 * 1024).clamp(4096, 128 * 1024) as usize;
        let mut encoded_bytes = 0usize;
        let mut last_cursor = query.cursor.clone();
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
        let mut roles = std::collections::BTreeMap::new();
        let mut summary_state = LibrarySummaryState::Ready;
        let next_cursor = 'pages: loop {
            query.limit = Some((limit - items.len()).min(200 - examined) as u16);
            let page = self.care.library_entry_sources(
                actor.user_id,
                query.clone(),
                &namespace,
                &language,
                ids,
            )?;
            examined += page.items.len();
            let active = self.provisional_imports.items_with_active_ingest(
                &page
                    .items
                    .iter()
                    .map(|source| source.id)
                    .collect::<Vec<_>>(),
            )?;
            for source in page.items {
                let role = if let Some(role) = roles.get(&source.collection_id) {
                    *role
                } else {
                    let role = self.authorize(
                        actor,
                        source.collection_id,
                        if query.filter == BrowseFilter::Removed {
                            Action::Curate
                        } else {
                            Action::Browse
                        },
                    )?;
                    roles.insert(source.collection_id, role);
                    role
                };
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
                    && active.contains(&entry.id);
                if !attention
                    || (!entry.import_in_progress
                        && matches!(
                            entry.album_playback,
                            Some(
                                collector_api_types::AlbumPlaybackReadiness::NeedsAttention { .. }
                            )
                        ))
                {
                    let bytes = serde_json::to_vec(&entry)
                        .map_err(|e| Error::Storage(e.to_string()))?
                        .len()
                        + 1;
                    let envelope = LibraryPage {
                        items: Vec::new(),
                        next_cursor: Some(source.cursor.clone()),
                        summary_state,
                        catalog_revision: revision.clone(),
                        membership_revision: cursor_revision.clone(),
                    };
                    let overhead = serde_json::to_vec(&envelope)
                        .map_err(|e| Error::Storage(e.to_string()))?
                        .len();
                    if encoded_bytes + bytes + overhead > budget {
                        if items.is_empty() {
                            return Err(Error::Invalid(
                                "library row exceeds requested page byte budget".into(),
                            ));
                        }
                        break 'pages last_cursor;
                    }
                    encoded_bytes += bytes;
                    items.push(entry);
                }
                last_cursor = Some(source.cursor);
            }
            if !attention || items.len() == limit || examined >= 200 || page.next_cursor.is_none() {
                break page.next_cursor;
            }
            query.cursor = page.next_cursor;
        };
        let (current, current_membership) = self.care.library_read_revisions(query.filter)?;
        if cursor_revision != current_membership || (strict_revision && revision != current) {
            return Err(Error::Conflict);
        }
        let page = LibraryPage {
            items,
            next_cursor,
            summary_state,
            catalog_revision: revision,
            membership_revision: cursor_revision,
        };
        if serde_json::to_vec(&page)
            .map_err(|e| Error::Storage(e.to_string()))?
            .len()
            > budget
        {
            return Err(Error::Invalid(
                "library continuation exceeds requested byte budget".into(),
            ));
        }
        Ok(page)
    }
}

impl CollectionService {
    pub fn library_track_summary(
        &self,
        actor: &Principal,
        item: ItemId,
    ) -> Result<collector_api_types::TrackSummary> {
        let collection = self.care.library_item_scope(item)?;
        let role = self.authorize(actor, collection, Action::Browse)?;
        let namespace = self.library_namespace()?;
        let language = self.metadata_language(actor.user_id)?;
        let cached = self
            .care
            .library_track_count(item, &namespace, &language, role)?;
        if cached
            != self
                .care
                .library_track_count(item, &namespace, &language, role)?
            || self.authorize(actor, collection, Action::Browse)? != role
            || self.metadata_language(actor.user_id)? != language
            || self.care.library_item_scope(item)? != collection
        {
            return Err(Error::Conflict);
        }
        match cached {
            Some((token, total)) => Ok(collector_api_types::TrackSummary {
                tracks_revision: track_revision(item, &token, &namespace, &language, role)?,
                total_items: Some(total),
                pending: false,
            }),
            None => Ok(collector_api_types::TrackSummary {
                tracks_revision: String::new(),
                total_items: None,
                pending: true,
            }),
        }
    }
    pub fn library_tracks(
        &self,
        actor: &Principal,
        item: ItemId,
        query: collector_api_types::TrackPageQuery,
    ) -> Result<collector_api_types::TrackPage> {
        let collection = self.care.library_item_scope(item)?;
        let role = self.authorize(actor, collection, Action::Browse)?;
        let namespace = self.library_namespace()?;
        let language = self.metadata_language(actor.user_id)?;
        let cached = self
            .care
            .library_track_cache(item, &namespace, &language, role)?;
        let Some((token, tracks)) = cached else {
            return Ok(collector_api_types::TrackPage {
                summary: collector_api_types::TrackSummary {
                    tracks_revision: String::new(),
                    total_items: None,
                    pending: true,
                },
                items: Vec::new(),
                next_cursor: None,
            });
        };
        let revision = track_revision(item, &token, &namespace, &language, role)?;
        if query.revision.as_ref().is_some_and(|r| r != &revision) {
            return Err(Error::Conflict);
        }
        if query.cursor.is_some() && query.revision.is_none() {
            return Err(Error::Invalid(
                "track continuation requires revision".into(),
            ));
        }
        let offset = query.cursor.unwrap_or(0) as usize;
        if offset > tracks.len() {
            return Err(Error::Invalid("invalid track cursor".into()));
        }
        let total = tracks.len();
        let budget = query.max_bytes.unwrap_or(64 * 1024).clamp(4096, 128 * 1024) as usize;
        let mut bytes = 1024;
        let mut items = Vec::new();
        for track in tracks.into_iter().skip(offset).take(200) {
            let size = serde_json::to_vec(&track)
                .map_err(|e| Error::Storage(e.to_string()))?
                .len()
                + 1;
            if bytes + size > budget {
                if items.is_empty() {
                    return Err(Error::Invalid("track exceeds page byte budget".into()));
                }
                break;
            }
            bytes += size;
            items.push(track);
        }
        if self
            .care
            .library_track_count(item, &namespace, &language, role)?
            != Some((token, total as u64))
            || self.authorize(actor, collection, Action::Browse)? != role
            || self.metadata_language(actor.user_id)? != language
            || self.care.library_item_scope(item)? != collection
        {
            return Err(Error::Conflict);
        }
        let next = offset + items.len();
        Ok(collector_api_types::TrackPage {
            summary: collector_api_types::TrackSummary {
                tracks_revision: revision,
                total_items: Some(total as u64),
                pending: false,
            },
            items,
            next_cursor: (next < total).then_some(next as u32),
        })
    }
}

fn track_revision(
    item: ItemId,
    token: &str,
    namespace: &str,
    language: &str,
    role: Role,
) -> Result<String> {
    Ok(Digest::sha256(
        &serde_json::to_vec(&(item, token, namespace, language, role))
            .map_err(|e| Error::Storage(e.to_string()))?,
    )
    .hex)
}
