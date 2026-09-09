//! Shared use cases. Every adapter and frontend crosses this authorization boundary.
use collector_core::*;
use collector_modules::ModulePresentation;
use collector_modules::Registry;
use serde::Serialize;
use std::sync::Arc;

pub trait Catalog: Send + Sync {
    fn collections(&self, user: UserId) -> Result<Vec<Collection>>;
    fn create_collection(&self, user: UserId, name: &str) -> Result<Collection>;
    fn role(&self, user: UserId, collection: CollectionId) -> Result<Role>;
    fn grant(&self, collection: CollectionId, user: UserId, role: Role) -> Result<()>;
    fn browse(&self, user: UserId, query: BrowseQuery) -> Result<Page<Item>>;
    fn item(&self, id: ItemId) -> Result<Item>;
    fn save(&self, item: Item, expected: Revision) -> Result<Item>;
    fn work(&self, id: WorkId) -> Result<Work>;
    fn save_work(&self, work: Work, expected: Revision) -> Result<Work>;
    fn edition(&self, id: EditionId) -> Result<Edition>;
    fn save_edition(&self, edition: Edition, expected: Revision) -> Result<Edition>;
    fn capture(&self, id: CaptureId) -> Result<Capture>;
    fn save_capture(&self, capture: Capture, expected: Revision) -> Result<Capture>;
    fn evidence(&self, item: ItemId) -> Result<Vec<Evidence>>;
}
#[derive(Clone)]
pub struct CollectionService {
    catalog: Arc<dyn Catalog>,
    modules: Arc<Registry>,
}
#[derive(Clone, Debug, Serialize)]
pub struct PresentedItem {
    #[serde(flatten)]
    pub item: Item,
    pub presentation: ModulePresentation,
}
impl CollectionService {
    pub fn new(catalog: Arc<dyn Catalog>, modules: Arc<Registry>) -> Self {
        Self { catalog, modules }
    }
    pub fn collections(&self, actor: &Principal) -> Result<Vec<Collection>> {
        self.catalog.collections(actor.user_id)
    }
    pub fn create_collection(&self, actor: &Principal, name: &str) -> Result<Collection> {
        if !actor.administrator {
            return Err(Error::Forbidden);
        }
        if name.trim().is_empty() || name.len() > 256 {
            return Err(Error::Invalid("collection name must be 1–256 bytes".into()));
        }
        self.catalog.create_collection(actor.user_id, name.trim())
    }
    pub fn authorize(
        &self,
        actor: &Principal,
        collection: CollectionId,
        action: Action,
    ) -> Result<Role> {
        let role = self.catalog.role(actor.user_id, collection)?;
        if !role.allows(action) {
            return Err(Error::Forbidden);
        }
        Ok(role)
    }
    pub fn browse(&self, actor: &Principal, query: BrowseQuery) -> Result<Page<Item>> {
        let mut page = self.catalog.browse(actor.user_id, query)?;
        for item in &mut page.items {
            item.redact_for(self.authorize(actor, item.collection_id, Action::Browse)?);
        }
        Ok(page)
    }
    pub fn item(&self, actor: &Principal, id: ItemId) -> Result<Item> {
        let mut item = self.catalog.item(id)?;
        let role = self.authorize(actor, item.collection_id, Action::Browse)?;
        item.redact_for(role);
        Ok(item)
    }
    /// Presentation is calculated only after role-based entity redaction. It is a
    /// read model; module commands remain behind A04.
    pub fn browse_presented(
        &self,
        actor: &Principal,
        query: BrowseQuery,
    ) -> Result<Page<PresentedItem>> {
        let page = self.browse(actor, query)?;
        Ok(Page {
            items: page
                .items
                .into_iter()
                .map(|item| self.presented(item))
                .collect(),
            next_cursor: page.next_cursor,
        })
    }
    pub fn item_presented(&self, actor: &Principal, id: ItemId) -> Result<PresentedItem> {
        let item = self.item(actor, id)?;
        Ok(self.presented(item))
    }
    fn presented(&self, item: Item) -> PresentedItem {
        let mut presentation = self.modules.presentation(&item.domain);
        presentation.artwork.retain(|reference| {
            item.assets
                .iter()
                .any(|asset| asset.id == reference.asset_id)
        });
        PresentedItem { presentation, item }
    }
    pub fn save(&self, actor: &Principal, item: Item, expected: Revision) -> Result<Item> {
        self.authorize(actor, item.collection_id, Action::Curate)?;
        item.validate()?;
        self.modules.validate(&item.domain)?;
        match self.catalog.item(item.id) {
            Ok(previous) => {
                if previous.collection_id != item.collection_id {
                    return Err(Error::Invalid(
                        "moving items across collections is not supported".into(),
                    ));
                }
                // File/evidence ownership is exclusively controlled by import and asset services.
                if serde_json::to_value((&previous.captures, &previous.assets)).unwrap()
                    != serde_json::to_value((&item.captures, &item.assets)).unwrap()
                {
                    return Err(Error::Invalid(
                        "stored files must be changed through import/asset operations".into(),
                    ));
                }
            }
            Err(Error::NotFound) => {
                if !item.captures.is_empty() || !item.assets.is_empty() {
                    return Err(Error::Invalid(
                        "new inventory cannot claim stored files".into(),
                    ));
                }
            }
            Err(e) => return Err(e),
        }
        self.catalog.save(item, expected)
    }
    pub fn capture(&self, actor: &Principal, id: CaptureId) -> Result<Capture> {
        let capture = self.catalog.capture(id)?;
        self.authorize(actor, capture.collection_id, Action::Browse)?;
        Ok(capture)
    }
    pub fn work(&self, actor: &Principal, id: WorkId) -> Result<Work> {
        let work = self.catalog.work(id)?;
        self.authorize(actor, work.collection_id, Action::Browse)?;
        Ok(work)
    }
    pub fn save_work(&self, actor: &Principal, work: Work, expected: Revision) -> Result<Work> {
        self.authorize(actor, work.collection_id, Action::Curate)?;
        work.validate()?;
        self.catalog.save_work(work, expected)
    }
    pub fn edition(&self, actor: &Principal, id: EditionId) -> Result<Edition> {
        let edition = self.catalog.edition(id)?;
        self.authorize(actor, edition.collection_id, Action::Browse)?;
        Ok(edition)
    }
    pub fn save_edition(
        &self,
        actor: &Principal,
        edition: Edition,
        expected: Revision,
    ) -> Result<Edition> {
        self.authorize(actor, edition.collection_id, Action::Curate)?;
        edition.validate()?;
        self.catalog.save_edition(edition, expected)
    }
    pub fn save_capture(
        &self,
        actor: &Principal,
        capture: Capture,
        expected: Revision,
    ) -> Result<Capture> {
        match self.catalog.capture(capture.id) {
            Ok(previous) => {
                self.authorize(actor, previous.collection_id, Action::Curate)?;
                if previous.collection_id != capture.collection_id {
                    return Err(Error::Invalid(
                        "capture cannot move across collections".into(),
                    ));
                }
                if serde_json::to_value((
                    &previous.representations,
                    &previous.legacy_payload_locator,
                ))
                .unwrap()
                    != serde_json::to_value((
                        &capture.representations,
                        &capture.legacy_payload_locator,
                    ))
                    .unwrap()
                {
                    return Err(Error::Invalid(
                        "capture files must be changed through import/representation services"
                            .into(),
                    ));
                }
            }
            Err(Error::NotFound) => {
                if !capture.representations.is_empty() || capture.legacy_payload_locator.is_some() {
                    return Err(Error::Invalid(
                        "new capture cannot claim stored files".into(),
                    ));
                }
            }
            Err(error) => return Err(error),
        }
        self.authorize(actor, capture.collection_id, Action::Curate)?;
        capture.validate()?;
        self.catalog.save_capture(capture, expected)
    }
    pub fn evidence(&self, actor: &Principal, id: ItemId) -> Result<Vec<Evidence>> {
        self.item(actor, id)?;
        self.catalog.evidence(id)
    }
    pub fn grant(
        &self,
        actor: &Principal,
        collection: CollectionId,
        user: UserId,
        role: Role,
    ) -> Result<()> {
        self.authorize(actor, collection, Action::ManageAccess)?;
        self.catalog.grant(collection, user, role)
    }
}
