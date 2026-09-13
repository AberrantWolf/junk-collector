use collector_core::{Collection, CollectionBinding, ModuleId};

#[test]
fn collection_binding_has_validated_stable_wire_forms() {
    let assigned = CollectionBinding::Assigned {
        module: ModuleId("org.junk.games".into()),
    };
    assert_eq!(
        serde_json::to_value(&assigned).unwrap(),
        serde_json::json!({"state":"assigned","module":"org.junk.games"})
    );
    assert_eq!(
        serde_json::from_value::<CollectionBinding>(serde_json::json!({
            "state":"assigned",
            "module":"org.junk.games"
        }))
        .unwrap(),
        assigned
    );
    assert_eq!(
        serde_json::to_value(CollectionBinding::LegacyUnassigned).unwrap(),
        serde_json::json!({"state":"legacy_unassigned"})
    );
    assert!(
        serde_json::from_value::<CollectionBinding>(serde_json::json!({
            "state":"assigned",
            "module":"INVALID/module"
        }))
        .is_err()
    );
    assert!(
        serde_json::from_value::<CollectionBinding>(serde_json::json!({
            "state":"legacy_unassigned",
            "module":"org.junk.games"
        }))
        .is_err()
    );
}

#[test]
fn a_collection_manifest_without_binding_is_legacy_unassigned() {
    let collection: Collection = serde_json::from_value(serde_json::json!({
        "id": "018f47be-3cc0-7ab0-8000-000000000001",
        "revision": 1,
        "name": "Historical archive",
        "created_at": "2024-05-06T07:08:09Z"
    }))
    .unwrap();
    assert_eq!(collection.binding, CollectionBinding::LegacyUnassigned);
}
