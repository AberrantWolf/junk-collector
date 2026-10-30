//! Accepted acquisition facts. Orders do not assert possession, files or catalog identity.
use crate::*;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PurchaseStatus {
    Preordered,
    Ordered,
    Purchased,
    Shipped,
    Delivered,
    Cancelled,
    Refunded,
    Unknown,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PurchaseOrder {
    pub id: String,
    pub retailer: String,
    pub order_reference: String,
    pub title: String,
    pub platform: String,
    pub edition: String,
    pub format: String,
    pub status: PurchaseStatus,
    pub observed_at: DateTime<Utc>,
    pub intentional_duplicate: bool,
    /// Opaque hashes only; never mailbox identifiers, email addresses or raw mail.
    pub source_keys: Vec<String>,
}
impl PurchaseOrder {
    pub fn validate(&self) -> Result<()> {
        for text in [
            &self.id,
            &self.retailer,
            &self.order_reference,
            &self.title,
            &self.platform,
            &self.edition,
            &self.format,
        ] {
            if text.len() > 1024 || text.chars().any(char::is_control) {
                return Err(Error::Invalid("Invalid purchase text".into()));
            }
        }
        if self.id.is_empty()
            || self.title.trim().is_empty()
            || self.source_keys.is_empty()
            || self.source_keys.len() > 256
            || self
                .source_keys
                .iter()
                .any(|key| key.len() != 64 || !key.bytes().all(|b| b.is_ascii_hexdigit()))
        {
            return Err(Error::Invalid(
                "Invalid purchase identity or source references".into(),
            ));
        }
        Ok(())
    }
}
