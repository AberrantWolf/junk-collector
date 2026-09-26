//! Schema-3 archive records. These are persistence envelopes, not HTTP DTOs.
//! Publication/migration code adopts them in P01.2–P01.4.
use chrono::{DateTime, Utc};
use collector_core::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct ItemRecordV3 {
    #[serde(default)]
    pub lifecycle: ItemLifecycle,
    pub id: ItemId,
    pub collection_id: CollectionId,
    pub revision: Revision,
    pub title: String,
    pub subtitle: String,
    pub identity_link: ItemIdentityLink,
    pub domain: ModuleData,
    pub identity: IdentityState,
    pub translations: Vec<LocalizedText>,
    pub identifiers: Vec<ProviderIdentity>,
    pub specimens: Vec<Specimen>,
    pub wishlist: bool,
    pub metadata: MetadataState,
    pub deleted: bool,
    pub updated_at: DateTime<Utc>,
}

impl From<&Item> for ItemRecordV3 {
    fn from(item: &Item) -> Self {
        Self {
            lifecycle: item.lifecycle.clone(),
            id: item.id,
            collection_id: item.collection_id,
            revision: item.revision,
            title: item.title.clone(),
            subtitle: item.subtitle.clone(),
            identity_link: item.identity_link,
            domain: item.domain.clone(),
            identity: item.identity.clone(),
            translations: item.translations.clone(),
            identifiers: item.identifiers.clone(),
            specimens: item.specimens.clone(),
            wishlist: item.wishlist,
            metadata: item.metadata.clone(),
            deleted: item.deleted,
            updated_at: item.updated_at,
        }
    }
}

impl ItemRecordV3 {
    pub(crate) fn into_item(self, captures: Vec<Capture>, assets: Vec<Asset>) -> Item {
        Item {
            lifecycle: self.lifecycle,
            id: self.id,
            collection_id: self.collection_id,
            revision: self.revision,
            title: self.title,
            subtitle: self.subtitle,
            identity_link: self.identity_link,
            domain: self.domain,
            identity: self.identity,
            translations: self.translations,
            identifiers: self.identifiers,
            specimens: self.specimens,
            wishlist: self.wishlist,
            captures,
            assets,
            metadata: self.metadata,
            deleted: self.deleted,
            updated_at: self.updated_at,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct AssetRecordV3 {
    pub collection_id: CollectionId,
    pub revision: Revision,
    pub asset: Asset,
    pub deleted: bool,
    pub updated_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub legacy_payloads: Vec<LegacyPayloadV3>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct CaptureRecordV3 {
    pub capture: Capture,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub legacy_root: Option<LegacyCaptureRootV3>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct LegacyCaptureRootV3 {
    pub item_id: ItemId,
    pub relative_path: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct EvidenceRecordV3 {
    pub id: EvidenceId,
    pub collection_id: CollectionId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_id: Option<ItemId>,
    pub representation_ids: Vec<RepresentationId>,
    pub kind: EvidenceKind,
    pub outcome: EvidenceOutcome,
    pub inputs: Vec<Digest>,
    pub implementation: String,
    pub implementation_version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub catalog: Option<CatalogSnapshot>,
    pub details: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

impl EvidenceRecordV3 {
    pub(crate) fn from_wave1(collection_id: CollectionId, evidence: &Evidence) -> Self {
        Self {
            id: evidence.id,
            collection_id,
            item_id: Some(evidence.item_id),
            representation_ids: vec![evidence.representation_id],
            kind: evidence.kind.clone(),
            outcome: evidence.outcome.clone(),
            inputs: evidence.inputs.clone(),
            implementation: evidence.implementation.clone(),
            implementation_version: evidence.implementation_version.clone(),
            catalog: evidence.catalog.clone(),
            details: evidence.details.clone(),
            created_at: evidence.created_at,
        }
    }

    pub(crate) fn validate(&self) -> Result<()> {
        if self.representation_ids.is_empty() {
            return Err(Error::Invalid(
                "evidence must refer to at least one exact representation".into(),
            ));
        }
        let unique = self
            .representation_ids
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>();
        if unique.len() != self.representation_ids.len() {
            return Err(Error::Invalid(
                "evidence representation identities must be unique".into(),
            ));
        }
        for digest in &self.inputs {
            digest.validate()?;
        }
        if let Some(catalog) = &self.catalog {
            catalog.provider.validate()?;
            catalog.digest.validate()?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(dead_code, reason = "P01.4 adopts tombstones in the entity writer")]
pub(crate) struct TombstoneRecordV3 {
    pub collection_id: CollectionId,
    pub entity_type: String,
    pub entity_id: String,
    pub revision: Revision,
    pub deleted_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct EvidenceReferenceV3 {
    pub id: EvidenceId,
    pub sha256: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct LegacyPayloadV3 {
    pub role: String,
    pub relative_path: String,
    pub sha256: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct RevisionEnvelopeV3<T> {
    pub schema_version: u32,
    pub transaction_id: TransactionId,
    pub record: T,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<EvidenceReferenceV3>,
}

impl<T> RevisionEnvelopeV3<T> {
    pub(crate) fn new(
        transaction_id: TransactionId,
        record: T,
        evidence: Vec<EvidenceReferenceV3>,
    ) -> Self {
        Self {
            schema_version: 3,
            transaction_id,
            record,
            evidence,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item() -> Item {
        Item {
            lifecycle: Default::default(),
            id: ItemId::new(),
            collection_id: CollectionId::new(),
            revision: Revision(4),
            title: "fixture".into(),
            subtitle: String::new(),
            identity_link: ItemIdentityLink::None,
            domain: ModuleData {
                module: ModuleId("org.example.preserved".into()),
                schema_version: 9,
                payload: RawJson::from_json(r#"{"z":1e+02,"a":null}"#).unwrap(),
            },
            identity: IdentityState::Unidentified,
            translations: vec![],
            identifiers: vec![],
            specimens: vec![],
            wishlist: false,
            captures: vec![Capture {
                id: CaptureId::new(),
                collection_id: CollectionId::new(),
                revision: Revision(2),
                association: CaptureAssociation::default(),
                created_at: Utc::now(),
                representations: vec![],
            }],
            assets: vec![],
            metadata: MetadataState::default(),
            deleted: false,
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn canonical_item_record_omits_compatibility_captures_and_assets() {
        let item = item();
        let bytes = serde_json::to_vec(&ItemRecordV3::from(&item)).unwrap();
        let text = String::from_utf8(bytes).unwrap();
        let document: serde_json::Value = serde_json::from_str(&text).unwrap();
        // Portable lifecycle references now include historical Capture IDs;
        // the prohibition is against embedding the compatibility projections.
        assert!(document.get("captures").is_none());
        assert!(document.get("assets").is_none());
        assert_eq!(
            document["lifecycle"]["historical_captures"],
            serde_json::json!([])
        );
        assert!(text.contains(r#""z":1e+02,"a":null"#));
        let restored = ItemRecordV3::from(&item).into_item(item.captures.clone(), vec![]);
        assert_eq!(restored.captures[0].id, item.captures[0].id);
    }

    #[test]
    fn evidence_requires_unique_exact_representations_but_not_an_item() {
        let representation = RepresentationId::new();
        let mut evidence = EvidenceRecordV3 {
            id: EvidenceId::new(),
            collection_id: CollectionId::new(),
            item_id: None,
            representation_ids: vec![representation],
            kind: EvidenceKind::Integrity,
            outcome: EvidenceOutcome::Passed,
            inputs: vec![Digest::sha256(b"fixture")],
            implementation: "fixture".into(),
            implementation_version: "1".into(),
            catalog: None,
            details: serde_json::json!({}),
            created_at: Utc::now(),
        };
        evidence.validate().unwrap();
        evidence.representation_ids.push(representation);
        assert!(evidence.validate().is_err());
        evidence.representation_ids.clear();
        assert!(evidence.validate().is_err());
    }
}
