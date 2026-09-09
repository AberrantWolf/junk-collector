//! Shared use cases. Every adapter and frontend crosses this authorization boundary.
use collector_core::*;
use collector_modules::Registry;
use std::sync::Arc;

pub trait Catalog: Send + Sync {
    fn collections(&self, user: UserId) -> Result<Vec<Collection>>;
    fn create_collection(&self, user: UserId, name: &str) -> Result<Collection>;
    fn role(&self, user: UserId, collection: CollectionId) -> Result<Role>;
    fn grant(&self, collection: CollectionId, user: UserId, role: Role) -> Result<()>;
    fn browse(&self, user: UserId, query: BrowseQuery) -> Result<Page<Item>>;
    fn item(&self, id: ItemId) -> Result<Item>;
    fn save(&self, item: Item, expected: Revision) -> Result<Item>;
    fn evidence(&self, item: ItemId) -> Result<Vec<Evidence>>;
}
#[derive(Clone)]
pub struct CollectionService {
    catalog: Arc<dyn Catalog>,
    modules: Arc<Registry>,
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
