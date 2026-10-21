//! Attachment authorization and preview policy; shared libraries own byte reads.
use crate::*;
pub const MAX_ASSET_BYTES: usize = 20 * 1024 * 1024;
const TEXT_PREVIEW_BYTES: usize = 128 * 1024;
#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
pub struct DocumentTextPreview {
    pub text: String,
    pub truncated: bool,
}
impl CollectionService {
    pub fn document_text_preview(
        &self,
        actor: &Principal,
        collection: CollectionId,
        asset: AssetId,
    ) -> Result<DocumentTextPreview> {
        let record = self.asset_presented(actor, collection, asset)?;
        if record.kind != AssetKind::Document || !record.media_type.starts_with("text/plain") {
            return Err(Error::Invalid(
                "This attachment does not have a plain-text preview".into(),
            ));
        }
        let resource = self.asset_resource(actor, collection, asset, false)?;
        let preview = junk_libs_core::document::text_preview(resource.reader, TEXT_PREVIEW_BYTES)
            .map_err(|e| Error::Invalid(e.to_string()))?;
        Ok(DocumentTextPreview {
            text: preview.text,
            truncated: preview.truncated,
        })
    }
}
