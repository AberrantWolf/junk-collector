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
    fn metadata_resolution_jobs(&self, user: UserId, item: ItemId) -> Result<Vec<JobRecord>>;
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
    ) -> Result<Page<PresentedItem>> {
        if let Some(collection) = query.collection_id {
            self.authorize(actor, collection, Action::Curate)?;
        }
        let page = self.care.metadata_care_items(actor.user_id, &query)?;
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
