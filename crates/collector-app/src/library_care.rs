//! Application-owned library maintenance. Durable execution lives in private state;
//! metadata decisions remain portable authority records.
use crate::*;

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct MetadataPreferences {
    pub revision: Revision,
    pub language: LanguageTag,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct UpdateMetadataPreferences {
    pub expected_revision: Revision,
    pub language: LanguageTag,
}

pub trait LibraryCarePort: Send + Sync {
    fn metadata_item_scope(&self, item: ItemId) -> Result<CollectionId>;
    fn metadata_read_clock(&self) -> Result<String>;
    fn library_item_scope(&self, item: ItemId) -> Result<CollectionId>;
    fn library_track_count(
        &self,
        item: ItemId,
        namespace: &str,
        language: &str,
        role: Role,
    ) -> Result<Option<(String, u64)>>;
    fn library_track_cache(
        &self,
        item: ItemId,
        namespace: &str,
        language: &str,
        role: Role,
    ) -> Result<Option<(String, Vec<collector_api_types::MusicTrackAvailability>)>>;
    fn cache_library_tracks(
        &self,
        namespace: &str,
        language: &str,
        role: Role,
        generation: &str,
        entry: &LibraryEntry,
        tracks: &[collector_api_types::MusicTrackAvailability],
    ) -> Result<()>;
    fn library_revision(&self) -> Result<String>;
    fn library_count(
        &self,
        user: UserId,
        query: BrowseQuery,
        namespace: &str,
        language: &str,
    ) -> Result<u64>;

    fn next_library_entry_build(&self, namespace: &str) -> Result<Option<LibraryEntryBuild>>;
    fn defer_library_entry_build(&self, namespace: &str, build: &LibraryEntryBuild) -> Result<()>;
    fn library_entry_sources(
        &self,
        user: UserId,
        query: BrowseQuery,
        namespace: &str,
        language: &str,
    ) -> Result<Page<LibraryEntrySource>>;
    fn cache_library_entry(
        &self,
        namespace: &str,
        language: &str,
        role: Role,
        generation: &str,
        entry: &LibraryEntry,
    ) -> Result<bool>;
    fn digital_matching_jobs(&self, user: UserId, item: ItemId) -> Result<Vec<JobRecord>>;
    fn cd_check_jobs(&self, user: UserId, item: ItemId) -> Result<Vec<JobRecord>>;
    fn cd_settings(&self, collection: CollectionId) -> Result<CdLibrarySettings>;
    fn save_cd_settings(
        &self,
        collection: CollectionId,
        command: &UpdateCdLibrarySettings,
    ) -> Result<CdLibrarySettings>;
    fn metadata_resolution_jobs(
        &self,
        user: UserId,
        item: ItemId,
    ) -> Result<Vec<crate::MetadataResolutionRecord>>;
    fn metadata_care_entries(
        &self,
        user: UserId,
        query: &MetadataCareQuery,
        namespace: &str,
        language: &str,
    ) -> Result<Page<MetadataCareRow>>;
    fn metadata_care_items(&self, user: UserId, query: &MetadataCareQuery) -> Result<Page<Item>>;
    fn album_group_proposals(
        &self,
        user: UserId,
        item: ItemId,
        after: Option<ItemId>,
        limit: u16,
    ) -> Result<Page<Item>>;
    fn preparation_care_items(&self, user: UserId, query: &MetadataCareQuery)
    -> Result<Page<Item>>;
    fn metadata_preferences(&self, user: UserId) -> Result<MetadataPreferences>;
    fn save_metadata_preferences(
        &self,
        user: UserId,
        command: &UpdateMetadataPreferences,
    ) -> Result<MetadataPreferences>;
}

impl CollectionService {
    pub fn metadata_preferences(&self, actor: &Principal) -> Result<MetadataPreferences> {
        self.care.metadata_preferences(actor.user_id)
    }
    pub fn update_metadata_preferences(
        &self,
        actor: &Principal,
        command: UpdateMetadataPreferences,
        context: &CommandContext,
    ) -> Result<MetadataPreferences> {
        LanguageTag::new(command.language.as_str())?;
        self.execute_command(
            actor,
            context,
            "metadata.preferences",
            &command,
            |effects| effects.save_metadata_preferences(actor.user_id, &command),
        )
    }
    pub(crate) fn metadata_language(&self, user: UserId) -> Result<String> {
        Ok(self
            .care
            .metadata_preferences(user)?
            .language
            .as_str()
            .to_string())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct MetadataCareQuery {
    pub collection_id: Option<CollectionId>,
    #[serde(default)]
    pub search: String,
    pub reason: Option<MetadataReviewReason>,
    #[serde(default = "needs_attention_default")]
    pub needs_attention: bool,
    pub cursor: Option<String>,
    pub limit: Option<u16>,
}
fn needs_attention_default() -> bool {
    true
}
impl CollectionService {
    pub fn metadata_care(
        &self,
        actor: &Principal,
        query: MetadataCareQuery,
    ) -> Result<Page<MetadataCareEntry>> {
        let revision = self.care.library_revision()?;
        if let Some(collection) = query.collection_id {
            self.authorize(actor, collection, Action::Curate)?;
        }
        let page = self.care.metadata_care_entries(
            actor.user_id,
            &query,
            &self.library_namespace()?,
            &self.metadata_language(actor.user_id)?,
        )?;
        let mut items = Vec::new();
        let mut roles = std::collections::BTreeMap::new();
        for source in page.items {
            let role = match roles.get(&source.entry.collection_id) {
                Some(role) => *role,
                None => {
                    let role = self.authorize(actor, source.entry.collection_id, Action::Curate)?;
                    roles.insert(source.entry.collection_id, role);
                    role
                }
            };
            if role != source.role {
                return Err(Error::Conflict);
            }
            items.push(source.entry);
        }
        if revision != self.care.library_revision()? {
            return Err(Error::CatalogBusy);
        }
        Ok(Page {
            items,
            next_cursor: page.next_cursor,
        })
    }
}

#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
pub struct PreparationCareEntry {
    pub item: PresentedItem,
    pub plan: MusicAlbumPreparationPlan,
}
impl CollectionService {
    pub fn preparation_care(
        &self,
        actor: &Principal,
        query: MetadataCareQuery,
    ) -> Result<Page<PreparationCareEntry>> {
        if let Some(collection) = query.collection_id {
            self.authorize(actor, collection, Action::Curate)?;
        }
        let page = self.care.preparation_care_items(actor.user_id, &query)?;
        let mut items = Vec::new();
        for item in page.items {
            items.push(PreparationCareEntry {
                plan: self.music_album_preparation_plan(actor, item.id)?,
                item: self.item_presented(actor, item.id)?,
            });
        }
        Ok(Page {
            items,
            next_cursor: page.next_cursor,
        })
    }
}

impl CollectionService {
    pub fn album_group_proposals(
        &self,
        actor: &Principal,
        item: ItemId,
        after: Option<ItemId>,
        limit: u16,
    ) -> Result<Page<PresentedItem>> {
        let entry = self.reader.item(item)?;
        self.authorize(actor, entry.collection_id, Action::Curate)?;
        let page = self
            .care
            .album_group_proposals(actor.user_id, item, after, limit)?;
        let mut items = Vec::new();
        for item in page.items {
            items.push(self.item_presented(actor, item.id)?);
        }
        Ok(Page {
            items,
            next_cursor: page.next_cursor,
        })
    }
}

/// Only fields consumed by Metadata rows; full graphs stay behind detail reads.
#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
pub struct MetadataCareEntry {
    pub id: ItemId,
    pub collection_id: CollectionId,
    pub title: String,
    pub subtitle: String,
    pub module: ModuleId,
    pub artwork: Option<LibraryArtwork>,
    pub summary_state: LibrarySummaryState,
    pub metadata_provisional: bool,
    pub metadata_tentative: bool,
    pub matching: Option<MetadataMatchingStatus>,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct MetadataMatchingStatus {
    pub phase: String,
    pub id: JobId,
    pub state: JobState,
}
pub struct MetadataCareRow {
    pub entry: MetadataCareEntry,
    pub role: Role,
}

#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
pub struct MetadataReadRevision {
    pub revision: String,
}
impl CollectionService {
    pub fn metadata_read_revision(
        &self,
        actor: &Principal,
        item: ItemId,
    ) -> Result<MetadataReadRevision> {
        let collection = self.care.metadata_item_scope(item)?;
        let role = self.authorize(actor, collection, Action::Curate)?;
        let revision = Digest::sha256(
            &serde_json::to_vec(&(
                item,
                role,
                self.metadata_language(actor.user_id)?,
                self.care.metadata_read_clock()?,
                self.library_namespace()?,
                self.streaming_providers.keys().collect::<Vec<_>>(),
            ))
            .map_err(|e| Error::Storage(e.to_string()))?,
        )
        .hex;
        Ok(MetadataReadRevision { revision })
    }
}
