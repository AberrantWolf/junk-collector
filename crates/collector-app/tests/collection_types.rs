use collector_app::{
    CollectionTypeAdministrationService, CollectionTypeAuthorityFact,
    CollectionTypePendingReference, CollectionTypePort, CollectionTypeSnapshot,
    CreateCollectionCommand,
};
use collector_core::*;
use collector_modules::Registry;
use std::sync::Arc;

struct SnapshotPort(CollectionTypeSnapshot);
impl CollectionTypePort for SnapshotPort {
    fn collection_type_snapshot(
        &self,
        _collection: CollectionId,
    ) -> Result<CollectionTypeSnapshot> {
        Ok(self.0.clone())
    }
    fn publish_collection_binding(
        &self,
        expected: Revision,
        revision: CollectionBindingRevision,
    ) -> Result<Collection> {
        assert_eq!(self.0.collection.revision, expected);
        let mut collection = self.0.collection.clone();
        collection.revision = revision.revision;
        collection.binding = revision.binding;
        Ok(collection)
    }
}

fn snapshot(domains: &[&str]) -> CollectionTypeSnapshot {
    CollectionTypeSnapshot {
        collection: Collection {
            id: CollectionId::new(),
            revision: Revision(1),
            name: "Legacy".into(),
            created_at: chrono::Utc::now(),
            binding: CollectionBinding::LegacyUnassigned,
        },
        item_domains: domains
            .iter()
            .map(|module| (ItemId::new(), ModuleId((*module).into())))
            .collect(),
        pending: vec![],
        authorities: vec![],
    }
}

#[test]
fn typed_collection_creation_requires_a_valid_assigned_module() {
    let command: CreateCollectionCommand = serde_json::from_value(serde_json::json!({
        "name": "Games",
        "module": "org.junk.games"
    }))
    .unwrap();
    command.validate().unwrap();

    assert!(
        serde_json::from_value::<CreateCollectionCommand>(serde_json::json!({
            "name": "Legacy"
        }))
        .is_err()
    );
    let invalid: CreateCollectionCommand = serde_json::from_value(serde_json::json!({
        "name": "Invalid",
        "module": "INVALID/module"
    }))
    .unwrap();
    assert!(invalid.validate().is_err());
}

#[test]
fn legacy_inspection_is_explicit_for_empty_mixed_unknown_and_preserved_data() {
    let empty = snapshot(&[]);
    let id = empty.collection.id;
    let service = CollectionTypeAdministrationService::new(
        Arc::new(SnapshotPort(empty)),
        Arc::new(Registry::builtins()),
    );
    let report = service.inspect(id).unwrap();
    assert!(report.eligible);
    assert_eq!(report.eligible_modules.len(), 2);
    assert!(matches!(
        service
            .assign(
                id,
                Revision(1),
                ModuleId("org.junk.games".into()),
                chrono::Utc::now(),
            )
            .unwrap()
            .binding,
        CollectionBinding::Assigned { module } if module == ModuleId("org.junk.games".into())
    ));

    let mixed = snapshot(&["org.junk.games", "org.junk.music"]);
    let id = mixed.collection.id;
    let report = CollectionTypeAdministrationService::new(
        Arc::new(SnapshotPort(mixed)),
        Arc::new(Registry::builtins()),
    )
    .inspect(id)
    .unwrap();
    assert!(!report.eligible);
    assert_eq!(report.item_domain_counts.len(), 2);
    assert!(
        report
            .unresolved
            .iter()
            .any(|value| value.contains("splitting"))
    );

    let unknown = snapshot(&["org.example.unknown"]);
    let id = unknown.collection.id;
    let report = CollectionTypeAdministrationService::new(
        Arc::new(SnapshotPort(unknown)),
        Arc::new(Registry::builtins()),
    )
    .inspect(id)
    .unwrap();
    assert!(!report.eligible);
    assert!(
        report
            .unresolved
            .iter()
            .any(|value| value.contains("unavailable"))
    );

    let mut blocked = snapshot(&["org.junk.music"]);
    let id = blocked.collection.id;
    let item_id = blocked.item_domains[0].0;
    blocked.pending.push(CollectionTypePendingReference {
        id: "job-actionable-id".into(),
        module: Some(ModuleId("org.junk.games".into())),
        operation: None,
        description: "active pre-commit job".into(),
    });
    blocked.authorities.push(CollectionTypeAuthorityFact {
        item_id,
        authority: MetadataAuthority {
            preserved_field_overrides: vec![
                RawJson::from_json(r#"{"field":"artist","language":"ja","value":"保存値"}"#)
                    .unwrap(),
            ],
            ..Default::default()
        },
    });
    let service = CollectionTypeAdministrationService::new(
        Arc::new(SnapshotPort(blocked)),
        Arc::new(Registry::builtins()),
    );
    let error = service
        .assign(
            id,
            Revision(1),
            ModuleId("org.junk.music".into()),
            chrono::Utc::now(),
        )
        .unwrap_err();
    assert!(error.to_string().contains("job-actionable-id"));
    assert!(error.to_string().contains("provenance only"));
}

#[test]
fn assignment_validates_active_authority_payload_and_schema() {
    for payload in [r#"{"artist":42}"#, r#"{"platform":"wrong owner schema"}"#] {
        let mut data = snapshot(&["org.junk.music"]);
        let id = data.collection.id;
        data.authorities.push(CollectionTypeAuthorityFact {
            item_id: data.item_domains[0].0,
            authority: MetadataAuthority {
                domain_overrides: Some(ModuleData {
                    module: ModuleId("org.junk.music".into()),
                    schema_version: 1,
                    payload: RawJson::from_json(payload).unwrap(),
                }),
                ..Default::default()
            },
        });
        let service = CollectionTypeAdministrationService::new(
            Arc::new(SnapshotPort(data)),
            Arc::new(Registry::builtins()),
        );
        assert!(!service.inspect(id).unwrap().eligible);
        assert!(
            service
                .assign(
                    id,
                    Revision(1),
                    ModuleId("org.junk.music".into()),
                    chrono::Utc::now()
                )
                .is_err()
        );
    }
}

struct SharedMediaModule;
impl collector_modules::CollectionModule for SharedMediaModule {
    fn descriptor(&self) -> collector_modules::ModuleDescriptor {
        use collector_modules::*;
        ModuleDescriptor {
            id: ModuleId("org.example.media".into()),
            schema_version: 1,
            display_name: "Media fixture".into(),
            collection_type_label: "Media fixture".into(),
            fields: vec![],
            capabilities: vec![Capability {
                id: "org.example.media.read".parse().unwrap(),
                display_name: "Read".into(),
                availability: Availability::Available,
                scope: OperationScope::SharedMedia,
            }],
        }
    }
    fn validate_current(&self, _: &ModuleData) -> collector_modules::ModuleResult<()> {
        Ok(())
    }
    fn apply_edit(
        &self,
        _: &ModuleData,
        _: &RawJson,
    ) -> collector_modules::ModuleResult<ModuleData> {
        Err(collector_modules::ModuleError::UnavailableCapability(
            "fixture".into(),
        ))
    }
    fn validate_metadata_overrides(&self, _: &ModuleData) -> collector_modules::ModuleResult<()> {
        Ok(())
    }
    fn validate_operation_result(
        &self,
        _: &collector_modules::CapabilityId,
        _: &RawJson,
    ) -> collector_modules::ModuleResult<()> {
        Err(collector_modules::ModuleError::UnavailableCapability(
            "fixture".into(),
        ))
    }
    fn present(
        &self,
        _: &ModuleData,
    ) -> collector_modules::ModuleResult<collector_modules::ModulePresentation> {
        Ok(Default::default())
    }
}

#[test]
fn shared_media_operations_do_not_claim_collection_ownership_and_unknown_jobs_block() {
    let mut registry = Registry::builtins();
    registry.register(Box::new(SharedMediaModule)).unwrap();
    let mut data = snapshot(&["org.junk.music"]);
    let id = data.collection.id;
    data.pending.push(CollectionTypePendingReference {
        id: "shared".into(),
        module: None,
        operation: Some("org.example.media.read".parse().unwrap()),
        description: "job".into(),
    });
    let service = CollectionTypeAdministrationService::new(
        Arc::new(SnapshotPort(data.clone())),
        Arc::new(registry),
    );
    assert!(
        service
            .assign(
                id,
                Revision(1),
                ModuleId("org.junk.music".into()),
                chrono::Utc::now()
            )
            .is_ok()
    );
    data.pending[0].operation = None;
    let service = CollectionTypeAdministrationService::new(
        Arc::new(SnapshotPort(data)),
        Arc::new(Registry::builtins()),
    );
    assert!(
        service
            .assign(
                id,
                Revision(1),
                ModuleId("org.junk.music".into()),
                chrono::Utc::now()
            )
            .is_err()
    );
}
