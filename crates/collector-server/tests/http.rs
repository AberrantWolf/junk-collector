use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use collector_app::{
    AuthenticatedPrincipal, Catalog, CommandContext, CreateCollectionCommand, CredentialKind,
    JobPort, JobSpec,
};
use collector_client::Pairing;
use collector_core::*;
use collector_server::*;
use collector_store::{Store, UploadFile};
use std::sync::Arc;
use tower::ServiceExt;

struct AbsentFixtureModule;
struct StampWebFixtureModule;
impl collector_modules::CollectionModule for AbsentFixtureModule {
    fn descriptor(&self) -> collector_modules::ModuleDescriptor {
        collector_modules::ModuleDescriptor {
            id: ModuleId("org.example.absent".into()),
            schema_version: 8,
            display_name: "Temporary absent fixture".into(),
            collection_type_label: "Temporary fixtures".into(),
            fields: vec![],
            capabilities: vec![],
        }
    }
    fn validate_current(&self, _data: &ModuleData) -> collector_modules::ModuleResult<()> {
        Ok(())
    }
    fn apply_edit(
        &self,
        current: &ModuleData,
        _edit: &RawJson,
    ) -> collector_modules::ModuleResult<ModuleData> {
        Ok(current.clone())
    }
    fn validate_metadata_overrides(
        &self,
        _overrides: &ModuleData,
    ) -> collector_modules::ModuleResult<()> {
        Ok(())
    }
    fn validate_operation_result(
        &self,
        _capability: &collector_modules::CapabilityId,
        _result: &RawJson,
    ) -> collector_modules::ModuleResult<()> {
        Ok(())
    }
    fn present(
        &self,
        _data: &ModuleData,
    ) -> collector_modules::ModuleResult<collector_modules::ModulePresentation> {
        Ok(Default::default())
    }
}
impl collector_modules::CollectionModule for StampWebFixtureModule {
    fn descriptor(&self) -> collector_modules::ModuleDescriptor {
        use collector_modules::{DomainFieldDescriptor, DomainFieldKind, ModuleDescriptor};
        ModuleDescriptor {
            id: ModuleId("org.example.stamps".into()),
            schema_version: 1,
            display_name: "Stamp fixture module".into(),
            collection_type_label: "Postage stamps".into(),
            fields: vec![
                DomainFieldDescriptor {
                    key: "denomination".into(),
                    label: "Denomination".into(),
                    required: true,
                    editable: true,
                    kind: DomainFieldKind::Integer {
                        minimum: 1,
                        maximum: 10_000,
                    },
                },
                DomainFieldDescriptor {
                    key: "unit".into(),
                    label: "Currency unit".into(),
                    required: true,
                    editable: true,
                    kind: DomainFieldKind::Text { maximum_bytes: 16 },
                },
                DomainFieldDescriptor {
                    key: "perforations".into(),
                    label: "Perforation observations".into(),
                    required: false,
                    editable: false,
                    kind: DomainFieldKind::Structured,
                },
            ],
            capabilities: vec![],
        }
    }
    fn validate_current(&self, data: &ModuleData) -> collector_modules::ModuleResult<()> {
        let value = data
            .payload
            .value()
            .map_err(|error| collector_modules::ModuleError::Invalid(error.to_string()))?;
        if value["denomination"]
            .as_u64()
            .is_some_and(|value| (1..=10_000).contains(&value))
            && value["unit"]
                .as_str()
                .is_some_and(|value| !value.is_empty() && value.len() <= 16)
        {
            Ok(())
        } else {
            Err(collector_modules::ModuleError::Invalid(
                "valid denomination and currency unit are required".into(),
            ))
        }
    }
    fn apply_edit(
        &self,
        current: &ModuleData,
        edit: &RawJson,
    ) -> collector_modules::ModuleResult<ModuleData> {
        let mut value = current
            .payload
            .value()
            .map_err(|error| collector_modules::ModuleError::Invalid(error.to_string()))?;
        let patch = edit
            .value()
            .map_err(|error| collector_modules::ModuleError::Invalid(error.to_string()))?;
        let patch = patch.as_object().ok_or_else(|| {
            collector_modules::ModuleError::Invalid("stamp patch must be an object".into())
        })?;
        if patch
            .keys()
            .any(|key| key != "denomination" && key != "unit")
        {
            return Err(collector_modules::ModuleError::Invalid(
                "unknown stamp field".into(),
            ));
        }
        let object = value.as_object_mut().ok_or_else(|| {
            collector_modules::ModuleError::Invalid("stamp data must be an object".into())
        })?;
        for (key, field) in patch {
            object.insert(key.clone(), field.clone());
        }
        let result = ModuleData {
            module: current.module.clone(),
            schema_version: current.schema_version,
            payload: RawJson::from_value(value),
        };
        self.validate_current(&result)?;
        Ok(result)
    }
    fn validate_metadata_overrides(
        &self,
        _overrides: &ModuleData,
    ) -> collector_modules::ModuleResult<()> {
        Ok(())
    }
    fn validate_operation_result(
        &self,
        _capability: &collector_modules::CapabilityId,
        _result: &RawJson,
    ) -> collector_modules::ModuleResult<()> {
        Ok(())
    }
    fn present(
        &self,
        data: &ModuleData,
    ) -> collector_modules::ModuleResult<collector_modules::ModulePresentation> {
        self.validate_current(data)?;
        let value = data.payload.value().unwrap();
        Ok(collector_modules::ModulePresentation {
            module_id: data.module.0.clone(),
            schema_version: data.schema_version,
            available: true,
            fields: vec![collector_modules::PresentationText {
                label: "Denomination".into(),
                value: format!(
                    "{} {}",
                    value["denomination"],
                    value["unit"].as_str().unwrap()
                ),
            }],
            ..Default::default()
        })
    }
}
fn command() -> CommandContext {
    CommandContext::new(TransactionId::new().to_string()).unwrap()
}
fn typed_collection(name: &str) -> CreateCollectionCommand {
    CreateCollectionCommand {
        name: name.into(),
        module: ModuleId("org.junk.music".into()),
    }
}
fn test_authentication(principal: &Principal) -> AuthenticatedPrincipal {
    AuthenticatedPrincipal {
        principal: principal.clone(),
        credential_kind: CredentialKind::TestLocal,
        authenticated_at: chrono::Utc::now(),
    }
}
fn state() -> (tempfile::TempDir, AppState) {
    let root = tempfile::tempdir().unwrap();
    let store =
        Arc::new(Store::open(root.path().join("archive"), root.path().join("state")).unwrap());
    let id = store.server_id().unwrap();
    let mut state = AppState::new(
        store,
        Pairing {
            server_id: id,
            certificate_pem: String::new(),
            certificate_sha256: String::new(),
            endpoints: vec![],
        },
    );
    state.allowed_origins = Arc::new(vec!["https://collector.example".into()]);
    (root, state)
}
#[tokio::test]
async fn public_surface_matrix_ignores_spoofed_proxy_headers() {
    for curation in [false, true] {
        for admin in [false, true] {
            let (_root, mut state) = state();
            state.exposure = Exposure {
                public_curation: curation,
                public_administration: admin,
            };
            let app = router(state);
            for (path, enabled) in [
                ("/curate", curation || admin),
                ("/curation.js", curation || admin),
                ("/admin", admin),
                ("/administration.js", admin),
            ] {
                let response = app
                    .clone()
                    .oneshot(
                        Request::builder()
                            .uri(path)
                            .header("x-forwarded-for", "127.0.0.1")
                            .header("forwarded", "for=127.0.0.1")
                            .body(Body::empty())
                            .unwrap(),
                    )
                    .await
                    .unwrap();
                assert_eq!(
                    response.status(),
                    if enabled {
                        StatusCode::OK
                    } else {
                        StatusCode::NOT_FOUND
                    },
                    "{path} curation={curation}, admin={admin}"
                );
            }
            let response = app
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/v1/admin/collections")
                        .header("origin", "https://collector.example")
                        .header("content-type", "application/json")
                        .body(Body::from(
                            "{\"name\":\"Test\",\"module\":\"org.junk.music\"}",
                        ))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(
                response.status(),
                if admin {
                    StatusCode::UNAUTHORIZED
                } else {
                    StatusCode::NOT_FOUND
                }
            );
        }
    }
}

#[tokio::test]
async fn authenticated_descriptors_drive_third_module_create_edit_browse_and_reindex() {
    let root = tempfile::tempdir().unwrap();
    let store =
        Arc::new(Store::open(root.path().join("archive"), root.path().join("state")).unwrap());
    let user = store
        .create_user("stamp-owner", "a long stamp owner password", true)
        .unwrap();
    let actor = Principal {
        user_id: user,
        administrator: true,
        device_id: None,
    };
    let mut registry = collector_modules::Registry::builtins();
    registry.register(Box::new(StampWebFixtureModule)).unwrap();
    let id = store.server_id().unwrap();
    let mut state = AppState::new_with_registry(
        store.clone(),
        Pairing {
            server_id: id,
            certificate_pem: String::new(),
            certificate_sha256: String::new(),
            endpoints: vec![],
        },
        Arc::new(registry),
    );
    state.exposure.public_curation = true;
    state.allowed_origins = Arc::new(vec!["https://collector.example".into()]);
    let collection = state
        .service
        .create_collection(
            &test_authentication(&actor),
            CreateCollectionCommand {
                name: "Stamp album".into(),
                module: ModuleId("org.example.stamps".into()),
            },
            &command(),
        )
        .unwrap();
    let (token, _) = store.issue_session(user, "browser").unwrap();
    let app = router(state);

    let unauthenticated = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/collection-types")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(unauthenticated.status(), StatusCode::UNAUTHORIZED);
    let descriptors = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/collection-types")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(descriptors.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(descriptors.into_body(), 64 * 1024)
        .await
        .unwrap();
    let descriptors: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let stamps = descriptors
        .as_array()
        .unwrap()
        .iter()
        .find(|value| value["id"] == "org.example.stamps")
        .unwrap();
    assert_eq!(stamps["collection_type_label"], "Postage stamps");
    assert_eq!(stamps["fields"][0]["kind"]["type"], "integer");
    assert_eq!(stamps["fields"][2]["editable"], false);

    let item_id = ItemId::new();
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/v1/items/{item_id}"))
                .header("authorization", format!("Bearer {token}"))
                .header("origin", "https://collector.example")
                .header("idempotency-key", "third-module-web-edit")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&serde_json::json!({
                        "edit": {
                            "collection_id": collection.id,
                            "title": "Blue definitive",
                            "subtitle": "Perforated issue",
                            "identity_link": {"kind":"none"},
                            "domain_edit": {
                                "module": "org.example.stamps",
                                "fields": {"denomination":20,"unit":"cents"}
                            },
                            "identity": "unidentified",
                            "translations": [],
                            "identifiers": [],
                            "wishlist": false
                        },
                        "expected_revision": 0
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 64 * 1024)
        .await
        .unwrap();
    let presented: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(presented["domain"]["payload"]["denomination"], 20);
    assert_eq!(presented["content_permissions"]["can_edit"], true);
    assert_eq!(
        presented["presentation"]["fields"][0]["label"],
        "Denomination"
    );

    store.reindex().unwrap();
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/items?collection_id={}", collection.id))
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 64 * 1024)
        .await
        .unwrap();
    let page: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(page["items"][0]["domain"]["module"], "org.example.stamps");
    assert_eq!(page["items"][0]["domain"]["payload"]["unit"], "cents");

    let rejected = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/v1/items/{}", ItemId::new()))
                .header("authorization", format!("Bearer {token}"))
                .header("origin", "https://collector.example")
                .header("idempotency-key", "third-module-invalid-edit")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&serde_json::json!({
                        "edit": {
                            "collection_id": collection.id,
                            "title": "Invalid",
                            "subtitle": "",
                            "identity_link": {"kind":"none"},
                            "domain_edit": {"module":"org.example.stamps","fields":{"denomination":"twenty","unit":"cents"}},
                            "identity": "unidentified",
                            "translations": [],
                            "identifiers": [],
                            "wishlist": false
                        },
                        "expected_revision": 0
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(rejected.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn provisional_import_routes_converge_sources_and_hide_inbox_paths_and_other_users() {
    let (root, mut state) = state();
    state.exposure.public_curation = true;
    let owner = state
        .store
        .create_user("import-owner", "a long owner password", true)
        .unwrap();
    let stranger = state
        .store
        .create_user("import-stranger", "a long stranger password", true)
        .unwrap();
    let actor = Principal {
        user_id: owner,
        administrator: true,
        device_id: None,
    };
    let collection = state
        .service
        .create_collection(
            &test_authentication(&actor),
            typed_collection("Imports"),
            &command(),
        )
        .unwrap();
    let (owner_token, _) = state.store.issue_session(owner, "browser").unwrap();
    let (stranger_token, _) = state.store.issue_session(stranger, "browser").unwrap();
    let inbox_root = root.path().join("private-drop-root");
    std::fs::create_dir_all(inbox_root.join("package-one")).unwrap();
    std::fs::write(inbox_root.join("package-one/disc.bin"), b"data").unwrap();
    std::fs::write(inbox_root.join("not-a-package"), b"ignored").unwrap();
    Arc::make_mut(&mut state.inboxes).insert("Optical shelf".into(), inbox_root.clone());
    let service = state.service.clone();
    let app = router(state);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/collections/{}/inboxes", collection.id))
                .header("authorization", format!("Bearer {owner_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), 8192)
        .await
        .unwrap();
    let text = String::from_utf8(body.to_vec()).unwrap();
    assert!(text.contains("Optical shelf"));
    assert!(text.contains("package-one"));
    assert!(!text.contains("not-a-package"));
    assert!(!text.contains(inbox_root.to_str().unwrap()));

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/imports")
                .header("authorization", format!("Bearer {owner_token}"))
                .header("origin", "https://collector.example")
                .header("idempotency-key", "create-browser-import")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&serde_json::json!({
                        "collection_id": collection.id,
                        "package_name": "browser-package",
                        "files": [{"name":"disc.bin","bytes":4,"sha256":null}]
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), 16 * 1024)
        .await
        .unwrap();
    let browser: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let import_id = browser["id"].as_str().unwrap();
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/v1/imports/{import_id}/files/0?offset=0"))
                .header("authorization", format!("Bearer {owner_token}"))
                .header("origin", "https://collector.example")
                .header("content-type", "application/octet-stream")
                .body(Body::from("data"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), 16 * 1024)
        .await
        .unwrap();
    let staged: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(staged["phase"], "analyzing");
    assert_eq!(staged["files"][0]["received_bytes"], 4);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/imports/{import_id}/analysis"))
                .header("authorization", format!("Bearer {owner_token}"))
                .header("origin", "https://collector.example")
                .header("idempotency-key", "analyze-browser-import")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"module":"org.junk.music"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    assert!(service.run_next_job("http-analysis-worker").unwrap());
    let analyzed = service
        .provisional_import(&actor, import_id.parse().unwrap())
        .unwrap();
    assert_eq!(analyzed.phase, collector_app::ImportPhase::AwaitingReview);
    assert!(analyzed.candidates.is_empty());
    assert!(analyzed.limitations[0].contains("D04/D05"));
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/v1/imports/{import_id}/review"))
                .header("authorization", format!("Bearer {owner_token}"))
                .header("origin", "https://collector.example")
                .header("idempotency-key", "review-browser-import")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&serde_json::json!({
                        "expected_revision": analyzed.revision,
                        "identity": {"identity":"unidentified"},
                        "association": {"item_id":null,"specimen_id":null,"component_id":null,"carrier_id":null},
                        "expected_item_revision": null
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), 16 * 1024)
        .await
        .unwrap();
    let reviewed: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(reviewed["review"]["identity"]["identity"], "unidentified");
    assert!(
        !String::from_utf8(body.to_vec())
            .unwrap()
            .contains("verified")
    );

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/imports/{import_id}/publish"))
                .header("authorization", format!("Bearer {owner_token}"))
                .header("origin", "https://collector.example")
                .header("idempotency-key", "publish-browser-import")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    assert!(service.run_next_job("http-publication-worker").unwrap());
    assert_eq!(
        service
            .provisional_import(&actor, import_id.parse().unwrap())
            .unwrap()
            .phase,
        collector_app::ImportPhase::Published
    );

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/imports/{import_id}"))
                .header("authorization", format!("Bearer {stranger_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/v1/collections/{}/inboxes/Optical%20shelf/stage",
                    collection.id
                ))
                .header("authorization", format!("Bearer {owner_token}"))
                .header("origin", "https://collector.example")
                .header("idempotency-key", "stage-inbox-import")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"package":"package-one"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    let body = axum::body::to_bytes(response.into_body(), 16 * 1024)
        .await
        .unwrap();
    let inbox: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(
        !String::from_utf8(body.to_vec())
            .unwrap()
            .contains(inbox_root.to_str().unwrap())
    );
    assert!(service.run_next_job("http-import-worker").unwrap());
    let inbox_id: ImportId = inbox["import"]["id"].as_str().unwrap().parse().unwrap();
    let completed = service.provisional_import(&actor, inbox_id).unwrap();
    assert_eq!(completed.phase, collector_app::ImportPhase::Analyzing);
    assert_eq!(completed.files[0].digest, Some(Digest::sha256(b"data")));
}

#[tokio::test]
async fn legacy_upload_routes_delegate_to_imports_and_use_a_distinct_publication_job() {
    let (_root, mut state) = state();
    state.ingress = Ingress::Private;
    let user = state
        .store
        .create_user("legacy-upload-owner", "a long owner password", true)
        .unwrap();
    let actor = Principal {
        user_id: user,
        administrator: true,
        device_id: None,
    };
    let collection = state
        .service
        .create_collection(
            &test_authentication(&actor),
            typed_collection("Legacy uploads"),
            &command(),
        )
        .unwrap();
    let item = Item {
        id: ItemId::new(),
        collection_id: collection.id,
        revision: Revision(0),
        title: "Compatibility adapter".into(),
        subtitle: String::new(),
        identity_link: ItemIdentityLink::None,
        domain: ModuleData {
            module: ModuleId("org.junk.music".into()),
            schema_version: 1,
            payload: RawJson::from_value(serde_json::json!({})),
        },
        identity: IdentityState::Unidentified,
        translations: vec![],
        identifiers: vec![],
        specimens: vec![],
        wishlist: false,
        captures: vec![],
        assets: vec![],
        metadata: MetadataState::default(),
        deleted: false,
        updated_at: chrono::Utc::now(),
    };
    let saved = state
        .service
        .save_item(&actor, item.id, item.into(), Revision(0), &command())
        .unwrap();
    let (token, _) = state.store.issue_session(user, "browser").unwrap();
    let service = state.service.clone();
    let app = router(state);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/uploads")
                .header("authorization", format!("Bearer {token}"))
                .header("origin", "https://collector.example")
                .header("idempotency-key", "legacy-create")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&serde_json::json!({
                        "item_id": saved.id,
                        "expected_revision": saved.revision,
                        "files": [{"name":"disc.bin","bytes":4,"sha256":null}]
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), 16 * 1024)
        .await
        .unwrap();
    let upload: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let handle: JobId = upload["id"].as_str().unwrap().parse().unwrap();
    let import_id = ImportId(handle.0);
    assert_eq!(
        service.provisional_import(&actor, import_id).unwrap().phase,
        collector_app::ImportPhase::Staging
    );
    assert!(service.jobs(&actor).unwrap().is_empty());

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/v1/uploads/{handle}/files/0?offset=0"))
                .header("authorization", format!("Bearer {token}"))
                .header("origin", "https://collector.example")
                .header("content-type", "application/octet-stream")
                .body(Body::from("data"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/uploads/{handle}/commit"))
                .header("authorization", format!("Bearer {token}"))
                .header("origin", "https://collector.example")
                .header("idempotency-key", "legacy-commit")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    let body = axum::body::to_bytes(response.into_body(), 16 * 1024)
        .await
        .unwrap();
    let accepted: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let job: JobId = accepted["job_id"].as_str().unwrap().parse().unwrap();
    assert_ne!(job, handle);
    assert!(service.run_next_job("legacy-http-worker").unwrap());
    assert_eq!(service.job(&actor, job).unwrap().state, JobState::Completed);
    assert_eq!(
        service.provisional_import(&actor, import_id).unwrap().phase,
        collector_app::ImportPhase::Published
    );
    assert_eq!(
        service
            .item_presented(&actor, saved.id)
            .unwrap()
            .captures
            .len(),
        1
    );

    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/uploads/{handle}"))
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), 16 * 1024)
        .await
        .unwrap();
    let upload: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(upload["state"], "completed");
}

#[tokio::test]
async fn provisional_import_routes_follow_curation_listener_composition() {
    let import = ImportId::new();
    let collection = CollectionId::new();
    for enabled in [false, true] {
        let (_root, mut state) = state();
        state.exposure.public_curation = enabled;
        let app = router(state);
        for (method, path, body) in [
            (
                "POST",
                "/api/v1/imports".into(),
                serde_json::json!({"collection_id":collection,"package_name":"package","files":[{"name":"disc.bin","bytes":4,"sha256":null}]}),
            ),
            (
                "GET",
                format!("/api/v1/imports/{import}"),
                serde_json::Value::Null,
            ),
            (
                "POST",
                format!("/api/v1/imports/{import}/analysis"),
                serde_json::json!({"module":"org.junk.music"}),
            ),
            (
                "PUT",
                format!("/api/v1/imports/{import}/review"),
                serde_json::json!({"expected_revision":1,"identity":{"identity":"unidentified"},"association":{"item_id":null,"specimen_id":null,"component_id":null,"carrier_id":null},"expected_item_revision":null}),
            ),
            (
                "POST",
                format!("/api/v1/imports/{import}/publish"),
                serde_json::Value::Null,
            ),
            (
                "PUT",
                format!("/api/v1/imports/{import}/files/0?offset=0"),
                serde_json::Value::Null,
            ),
            (
                "GET",
                format!("/api/v1/collections/{collection}/inboxes"),
                serde_json::Value::Null,
            ),
            (
                "POST",
                format!("/api/v1/collections/{collection}/inboxes/label/stage"),
                serde_json::json!({"package":"package"}),
            ),
        ] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method(method)
                        .uri(path)
                        .header("origin", "https://collector.example")
                        .header("idempotency-key", "import-listener-probe")
                        .header("content-type", "application/json")
                        .body(Body::from(if body.is_null() {
                            Vec::new()
                        } else {
                            serde_json::to_vec(&body).unwrap()
                        }))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(
                response.status(),
                if enabled {
                    StatusCode::UNAUTHORIZED
                } else {
                    StatusCode::NOT_FOUND
                }
            );
        }
    }
}

#[tokio::test]
async fn physical_lifecycle_routes_follow_curation_listener_composition() {
    let id = ItemId::new();
    let specimen = SpecimenId::new();
    let component = ComponentId::new();
    let loan = LoanId::new();
    let requests = [
        (
            "POST",
            format!("/api/v1/items/{id}/specimens"),
            serde_json::json!({"expected_revision":1,"details":{"label":"copy","condition":"good","ownership":"owned","possession":"present","acquisition":null,"private_notes":null}}),
        ),
        (
            "PUT",
            format!("/api/v1/items/{id}/specimens/{specimen}"),
            serde_json::json!({"expected_revision":1,"details":{"label":"copy","condition":"good","ownership":"owned","possession":"present","acquisition":null,"private_notes":null}}),
        ),
        (
            "POST",
            format!("/api/v1/items/{id}/specimens/{specimen}/components"),
            serde_json::json!({"expected_revision":1,"details":{"label":"disc","kind":"optical_disc","position":1,"carrier":{"label":"disc","kind":"optical_disc"}}}),
        ),
        (
            "PUT",
            format!("/api/v1/items/{id}/specimens/{specimen}/components/{component}"),
            serde_json::json!({"expected_revision":1,"details":{"label":"disc","kind":"optical_disc","position":1,"carrier":{"label":"disc","kind":"optical_disc"}}}),
        ),
        (
            "DELETE",
            format!(
                "/api/v1/items/{id}/specimens/{specimen}/components/{component}?expected_revision=1"
            ),
            serde_json::Value::Null,
        ),
        (
            "POST",
            format!("/api/v1/items/{id}/specimens/{specimen}/loans"),
            serde_json::json!({"expected_revision":1,"command":{"contact":"person","lent_at":"2026-09-10","due_at":null}}),
        ),
        (
            "POST",
            format!("/api/v1/items/{id}/specimens/{specimen}/loans/{loan}/return"),
            serde_json::json!({"expected_revision":1,"command":{"returned_at":"2026-09-11","possession":"present"}}),
        ),
        (
            "POST",
            format!("/api/v1/items/{id}/specimens/{specimen}/dispose"),
            serde_json::json!({"expected_revision":1,"command":{"date":"2026-09-12","reason":"test","possession":"elsewhere"}}),
        ),
    ];
    for enabled in [false, true] {
        let (_root, mut state) = state();
        state.exposure.public_curation = enabled;
        let app = router(state);
        for (method, path, body) in &requests {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method(*method)
                        .uri(path)
                        .header("origin", "https://collector.example")
                        .header("idempotency-key", "listener-composition-probe")
                        .header("content-type", "application/json")
                        .body(Body::from(if body.is_null() {
                            Vec::new()
                        } else {
                            serde_json::to_vec(body).unwrap()
                        }))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(
                response.status(),
                if enabled {
                    StatusCode::UNAUTHORIZED
                } else {
                    StatusCode::NOT_FOUND
                },
                "{method} {path}, curation={enabled}"
            );
        }
    }
}

#[tokio::test]
async fn asset_mutations_follow_curation_listener_composition() {
    let collection = CollectionId::new();
    let asset = AssetId::new();
    let item = ItemId::new();
    let requests = [
        (
            "POST",
            format!(
                "/api/v1/collections/{collection}/assets?subject_kind=item&subject_id={item}&kind=cover&visibility=shared"
            ),
            b"image".to_vec(),
        ),
        (
            "PUT",
            format!("/api/v1/collections/{collection}/assets/{asset}/metadata"),
            serde_json::to_vec(&serde_json::json!({
                "expected_revision": 1,
                "command": {"visibility":"shared","alt_text":"cover"}
            }))
            .unwrap(),
        ),
        (
            "POST",
            format!("/api/v1/collections/{collection}/assets/{asset}/archive?expected_revision=1"),
            vec![],
        ),
    ];
    for enabled in [false, true] {
        let (_root, mut state) = state();
        state.exposure.public_curation = enabled;
        let app = router(state);
        for (method, path, body) in &requests {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method(*method)
                        .uri(path)
                        .header("origin", "https://collector.example")
                        .header("idempotency-key", "asset-listener-probe")
                        .header("content-type", "application/json")
                        .body(Body::from(body.clone()))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(
                response.status(),
                if enabled {
                    StatusCode::UNAUTHORIZED
                } else {
                    StatusCode::NOT_FOUND
                },
                "{method} {path}, curation={enabled}"
            );
        }
    }
}
#[tokio::test]
async fn private_surface_still_requires_authentication_and_csrf_origin() {
    let (_root, mut state) = state();
    state.ingress = Ingress::Private;
    let viewer = state
        .store
        .create_user("viewer", "a long viewer password", false)
        .unwrap();
    let (viewer_token, _) = state.store.issue_session(viewer, "browser").unwrap();
    let app = router(state);
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/admin/settings")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let body = axum::body::to_bytes(response.into_body(), 8192)
        .await
        .unwrap();
    let error: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(error["error"]["code"], "unauthenticated");
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/admin/settings")
                .header("authorization", format!("Bearer {viewer_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let body = axum::body::to_bytes(response.into_body(), 8192)
        .await
        .unwrap();
    let error: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(error["error"]["code"], "forbidden");
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header("origin", "https://attacker.example")
                .header("content-type", "application/json")
                .body(Body::from(
                    "{\"username\":\"owner\",\"password\":\"secret\"}",
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let body = axum::body::to_bytes(response.into_body(), 8192)
        .await
        .unwrap();
    let error: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(error["error"]["code"], "origin_not_allowed");
}

#[tokio::test]
async fn browser_origin_attack_matrix() {
    let (_root, state) = state();
    let app = router(state);
    for origin in [
        None,
        Some("null"),
        Some("not-an-origin"),
        Some("https://collector.example/"),
        Some("https://sibling.collector.example"),
        Some("https://collector.example:444"),
        Some("http://collector.example"),
    ] {
        let mut request = Request::builder()
            .method("POST")
            .uri("/api/v1/auth/login")
            .header("content-type", "application/json");
        if let Some(origin) = origin {
            request = request.header("origin", origin);
        }
        let response = app
            .clone()
            .oneshot(
                request
                    .body(Body::from(
                        "{\"username\":\"owner\",\"password\":\"wrong password\"}",
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN, "{origin:?}");
        let body = axum::body::to_bytes(response.into_body(), 8192)
            .await
            .unwrap();
        let error: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(error["error"]["code"], "origin_not_allowed");
    }

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header("origin", "https://collector.example")
                .header("content-type", "application/json")
                .body(Body::from(
                    "{\"username\":\"owner\",\"password\":\"wrong password\"}",
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn browser_approved_enrollment_issues_and_revokes_native_credential() {
    let (_root, state) = state();
    state
        .store
        .create_user("owner", "a sufficiently long password", true)
        .unwrap();
    let app = router(state);
    let login = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header("origin", "https://collector.example")
                .header("x-collector-client", "native")
                .header("content-type", "application/json")
                .body(Body::from("{\"username\":\"owner\",\"password\":\"a sufficiently long password\",\"device_name\":\"Browser\"}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(login.status(), StatusCode::OK);
    let cookie = login.headers()["set-cookie"]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    let login_body = axum::body::to_bytes(login.into_body(), 8192).await.unwrap();
    assert!(serde_json::from_slice::<serde_json::Value>(&login_body).unwrap()["token"].is_null());

    let started = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/enrollments/start")
                .header("content-type", "application/json")
                .body(Body::from("{\"device_name\":\"Native test\"}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(started.status(), StatusCode::OK);
    let started: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(started.into_body(), 8192)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(started["verification_url"], "/pair");

    let approved = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/enrollments/approve")
                .header("origin", "https://collector.example")
                .header("cookie", &cookie)
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"user_code": started["user_code"]}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(approved.status(), StatusCode::OK);

    let completed = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/v1/enrollments/{}/complete",
                    started["id"].as_str().unwrap()
                ))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"completion_secret": started["completion_secret"]})
                        .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(completed.status(), StatusCode::OK);
    let completed: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(completed.into_body(), 16384)
            .await
            .unwrap(),
    )
    .unwrap();
    let credential = completed["credential"].as_str().unwrap();

    let accepted = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/collections")
                .header("authorization", format!("Bearer {credential}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(accepted.status(), StatusCode::OK);

    let revoked = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!(
                    "/api/v1/security/native-devices/{}?expected_revision=1",
                    completed["device_id"].as_str().unwrap()
                ))
                .header("origin", "https://collector.example")
                .header("cookie", &cookie)
                .header("idempotency-key", "revoke-native-test")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(revoked.status(), StatusCode::NO_CONTENT);
    let rejected = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/collections")
                .header("authorization", format!("Bearer {credential}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(rejected.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn direct_grant_api_matches_owner_policy_and_removal_is_immediate() {
    let (_root, mut state) = state();
    state.exposure.public_curation = true;
    let owner = state
        .store
        .create_user("owner", "a sufficiently long password", true)
        .unwrap();
    let member = state
        .store
        .create_user("member", "another sufficiently long password", false)
        .unwrap();
    let administrator = state
        .store
        .create_user("administrator", "administrator long password", true)
        .unwrap();
    let (owner_token, owner_auth) = state.store.issue_session(owner, "owner browser").unwrap();
    let (member_token, _) = state.store.issue_session(member, "member browser").unwrap();
    let (administrator_token, _) = state
        .store
        .issue_session(administrator, "administrator browser")
        .unwrap();
    let collection = state
        .service
        .create_collection(&owner_auth, typed_collection("Access policy"), &command())
        .unwrap();
    let app = router(state);

    let added = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!(
                    "/api/v1/collections/{}/grants/member",
                    collection.id
                ))
                .header("origin", "https://collector.example")
                .header("authorization", format!("Bearer {owner_token}"))
                .header("idempotency-key", "add-member")
                .header("content-type", "application/json")
                .body(Body::from("{\"expected_revision\":0,\"role\":\"viewer\"}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(added.status(), StatusCode::OK);

    for token in [&member_token, &administrator_token] {
        let denied = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/v1/collections/{}/grants", collection.id))
                    .header("authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(denied.status(), StatusCode::FORBIDDEN);
    }

    let removed = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!(
                    "/api/v1/collections/{}/grants/member?expected_revision=1",
                    collection.id
                ))
                .header("origin", "https://collector.example")
                .header("authorization", format!("Bearer {owner_token}"))
                .header("idempotency-key", "remove-member")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(removed.status(), StatusCode::NO_CONTENT);
    let no_access = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/items?collection_id={}", collection.id))
                .header("authorization", format!("Bearer {member_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(no_access.status(), StatusCode::FORBIDDEN);

    let last_owner = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!(
                    "/api/v1/collections/{}/grants/owner?expected_revision=1",
                    collection.id
                ))
                .header("origin", "https://collector.example")
                .header("authorization", format!("Bearer {owner_token}"))
                .header("idempotency-key", "remove-last-owner")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(last_owner.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn account_revisions_and_last_admin_default_guards_surface_stable_errors() {
    let (_root, mut state) = state();
    state.exposure.public_administration = true;
    let administrator = state
        .store
        .create_user("owner", "a sufficiently long password", true)
        .unwrap();
    let member = state
        .store
        .create_user("member", "another sufficiently long password", false)
        .unwrap();
    let (token, _) = state
        .store
        .issue_session(administrator, "administrator browser")
        .unwrap();
    let app = router(state);
    let update = |expected_revision: u64, key: &str| {
        Request::builder()
            .method("PUT")
            .uri(format!("/api/v1/admin/users/{member}"))
            .header("origin", "https://collector.example")
            .header("authorization", format!("Bearer {token}"))
            .header("idempotency-key", key)
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({"expected_revision":expected_revision,"display_name":"Member profile","enabled":true,"administrator":false}).to_string(),
            ))
            .unwrap()
    };
    assert_eq!(
        app.clone()
            .oneshot(update(1, "member-update-first"))
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
    assert_eq!(
        app.clone()
            .oneshot(update(1, "member-update-stale"))
            .await
            .unwrap()
            .status(),
        StatusCode::CONFLICT
    );

    let demote_last = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/v1/admin/users/{administrator}"))
                .header("origin", "https://collector.example")
                .header("authorization", format!("Bearer {token}"))
                .header("idempotency-key", "demote-last-administrator")
                .header("content-type", "application/json")
                .body(Body::from("{\"expected_revision\":1,\"display_name\":\"Owner\",\"enabled\":true,\"administrator\":false}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(demote_last.status(), StatusCode::CONFLICT);
    let body = axum::body::to_bytes(demote_last.into_body(), 8192)
        .await
        .unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&body).unwrap()["error"]["code"],
        "last_administrator"
    );

    let disable_default = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/v1/admin/users/{administrator}"))
                .header("origin", "https://collector.example")
                .header("authorization", format!("Bearer {token}"))
                .header("idempotency-key", "disable-default-account")
                .header("content-type", "application/json")
                .body(Body::from("{\"expected_revision\":1,\"display_name\":\"Owner\",\"enabled\":false,\"administrator\":true}"))
                .unwrap(),
        )
        .await
        .unwrap();
    let body = axum::body::to_bytes(disable_default.into_body(), 8192)
        .await
        .unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&body).unwrap()["error"]["code"],
        "default_reassignment_required"
    );
}

#[test]
fn range_boundaries() {
    assert_eq!(parse_range("bytes=0-0", 10), Some((0, 0)));
    assert_eq!(parse_range("bytes=5-", 10), Some((5, 9)));
    assert_eq!(parse_range("bytes=-3", 10), Some((7, 9)));
    assert_eq!(parse_range("bytes=0-99", 10), Some((0, 9)));
    assert_eq!(parse_range("bytes=10-", 10), None);
    assert_eq!(parse_range("bytes=0-0,2-2", 10), None);
    assert_eq!(parse_range("bytes=-0", 10), None);
    assert_eq!(parse_range("bytes=0-0", 0), None);
}

#[tokio::test]
async fn annotated_contract_covers_api_and_is_not_served_as_public_ui() {
    let document = serde_json::to_value(openapi()).unwrap();
    let paths = document["paths"].as_object().unwrap();
    assert_eq!(paths.len(), 74);
    for path in [
        "/api/v1/items/{id}/specimens",
        "/api/v1/items/{id}/specimens/{specimen_id}",
        "/api/v1/items/{id}/specimens/{specimen_id}/components",
        "/api/v1/items/{id}/specimens/{specimen_id}/components/{component_id}",
        "/api/v1/items/{id}/specimens/{specimen_id}/loans",
        "/api/v1/items/{id}/specimens/{specimen_id}/loans/{loan_id}/return",
        "/api/v1/items/{id}/specimens/{specimen_id}/dispose",
        "/api/v1/imports",
        "/api/v1/imports/{id}",
        "/api/v1/captures/{id}/operations",
        "/api/v1/imports/{id}/analysis",
        "/api/v1/imports/{id}/review",
        "/api/v1/imports/{id}/publish",
        "/api/v1/imports/{id}/files/{index}",
        "/api/v1/collections/{id}/inboxes",
        "/api/v1/collections/{collection}/inboxes/{label}/stage",
        "/api/v1/collection-types",
    ] {
        assert!(
            paths.contains_key(path),
            "missing focused lifecycle path {path}"
        );
    }
    for path in [
        "/api/v1/collections/{collection}/assets",
        "/api/v1/collections/{collection}/assets/{asset}",
        "/api/v1/collections/{collection}/assets/{asset}/metadata",
        "/api/v1/collections/{collection}/assets/{asset}/archive",
        "/api/v1/collections/{collection}/assets/{asset}/original",
        "/api/v1/collections/{collection}/assets/{asset}/thumbnail",
    ] {
        assert!(paths.contains_key(path), "missing Asset path {path}");
    }
    assert!(paths.values().all(|path| {
        path.as_object().is_some_and(|operations| {
            operations.values().all(|operation| {
                operation
                    .get("responses")
                    .and_then(serde_json::Value::as_object)
                    .is_some_and(|responses| !responses.is_empty())
            })
        })
    }));
    for path in paths.values() {
        for operation in path.as_object().unwrap().values() {
            if let Some(body) = operation.get("requestBody") {
                assert!(body["x-body-limit-bytes"].as_u64().is_some());
            }
        }
    }
    assert!(
        document["components"]["schemas"]["PresentedCapture"]["properties"]
            .get("legacy_payload_locator")
            .is_none()
    );
    assert_eq!(
        document["components"]["schemas"]["SaveItemEnvelope"]["oneOf"]
            .as_array()
            .unwrap()
            .len(),
        3
    );

    let (_root, state) = state();
    let response = router(state)
        .oneshot(
            Request::builder()
                .uri("/api/openapi.v1.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn unavailable_module_is_preserved_and_presented_as_non_executable() {
    let (_root, mut state) = state();
    state.ingress = Ingress::Private;
    let user = state
        .store
        .create_user("owner", "a long owner password", true)
        .unwrap();
    let actor = Principal {
        user_id: user,
        administrator: true,
        device_id: None,
    };
    let mut creation_registry = collector_modules::Registry::builtins();
    creation_registry
        .register(Box::new(AbsentFixtureModule))
        .unwrap();
    state.service =
        collector_app::CollectionService::new(state.store.clone(), Arc::new(creation_registry));
    let collection = state
        .service
        .create_collection(
            &test_authentication(&actor),
            CreateCollectionCommand {
                name: "Unknown data".into(),
                module: ModuleId("org.example.absent".into()),
            },
            &command(),
        )
        .unwrap();
    let item = Item {
        id: ItemId::new(),
        collection_id: collection.id,
        revision: Revision(0),
        title: "Opaque record".into(),
        subtitle: String::new(),
        identity_link: ItemIdentityLink::None,
        domain: ModuleData {
            module: ModuleId("org.example.absent".into()),
            schema_version: 8,
            payload: RawJson::from_json(r#"{"z":1e+02,"a":null}"#).unwrap(),
        },
        identity: IdentityState::Unidentified,
        translations: vec![],
        identifiers: vec![],
        specimens: vec![],
        wishlist: true,
        captures: vec![],
        assets: vec![],
        metadata: MetadataState::default(),
        deleted: false,
        updated_at: chrono::Utc::now(),
    };
    let saved = state
        .service
        .save_item(&actor, item.id, item.into(), Revision(0), &command())
        .unwrap();
    let png = [
        137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 4,
        0, 0, 0, 181, 28, 12, 2, 0, 0, 0, 11, 73, 68, 65, 84, 120, 218, 99, 100, 248, 15, 0, 1, 5,
        1, 1, 39, 24, 227, 102, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
    ];
    let saved = state
        .service
        .publish_cover(
            &actor,
            saved.id,
            saved.revision,
            &png,
            "Preserved cover".into(),
            &command(),
        )
        .unwrap();
    let asset_id = saved.assets[0].id;
    state.service = collector_app::CollectionService::new(
        state.store.clone(),
        Arc::new(collector_modules::Registry::builtins()),
    );
    let mut rejected = collector_app::SaveItemCommand::from(saved.clone());
    rejected.title = "Must remain read-only".into();
    assert!(matches!(
        state
            .service
            .save_item(&actor, saved.id, rejected, saved.revision, &command()),
        Err(Error::Unavailable(_))
    ));
    assert!(
        state
            .service
            .asset(&actor, saved.id, asset_id, true)
            .is_ok()
    );
    let (token, _) = state.store.issue_session(user, "test").unwrap();
    let response = router(state)
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/items/{}", saved.id))
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 64 * 1024)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["presentation"]["available"], false);
    assert_eq!(body["effective_permissions"]["can_curate"], false);
    assert_eq!(body["content_permissions"]["can_edit"], false);
    assert_eq!(body["content_permissions"]["status"], "owner_unavailable");
    assert!(
        body["presentation"]["limitations"][0]
            .as_str()
            .unwrap()
            .contains("not executable")
    );
    assert!(
        String::from_utf8(bytes.to_vec())
            .unwrap()
            .contains(r#""z":1e+02,"a":null"#)
    );
}

#[tokio::test]
async fn integrity_endpoint_enqueues_a_durable_job_instead_of_doing_request_work() {
    let (_root, mut state) = state();
    state.ingress = Ingress::Private;
    let user = state
        .store
        .create_user("owner", "a long owner password", true)
        .unwrap();
    let actor = Principal {
        user_id: user,
        administrator: true,
        device_id: None,
    };
    let collection = state
        .service
        .create_collection(
            &test_authentication(&actor),
            typed_collection("Durable work"),
            &command(),
        )
        .unwrap();
    let item = Item {
        id: ItemId::new(),
        collection_id: collection.id,
        revision: Revision(0),
        title: "Queued verification".into(),
        subtitle: String::new(),
        identity_link: ItemIdentityLink::None,
        domain: ModuleData {
            module: ModuleId("org.junk.music".into()),
            schema_version: 1,
            payload: RawJson::from_value(serde_json::json!({})),
        },
        identity: IdentityState::Unidentified,
        translations: vec![],
        identifiers: vec![],
        specimens: vec![],
        wishlist: false,
        captures: vec![],
        assets: vec![],
        metadata: MetadataState::default(),
        deleted: false,
        updated_at: chrono::Utc::now(),
    };
    let saved = state
        .service
        .save_item(&actor, item.id, item.into(), Revision(0), &command())
        .unwrap();
    let upload = state
        .service
        .create_upload(
            &actor,
            saved.id,
            saved.revision,
            vec![UploadFile {
                name: "fixture.bin".into(),
                bytes: ByteLength(4),
                sha256: None,
            }],
            &command(),
        )
        .unwrap();
    state
        .service
        .upload_chunk(&actor, upload.id, 0, 0, b"data")
        .unwrap();
    state
        .service
        .commit_upload(&actor, upload.id, &command())
        .unwrap();
    let (token, _) = state.store.issue_session(user, "browser").unwrap();
    let service = state.service.clone();
    let response = router(state)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/items/{}/verify", saved.id))
                .header("origin", "https://collector.example")
                .header("authorization", format!("Bearer {token}"))
                .header("idempotency-key", "durable-integrity-http")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    let bytes = axum::body::to_bytes(response.into_body(), 8192)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let job: JobId = body["job_id"].as_str().unwrap().parse().unwrap();
    assert_eq!(
        service
            .jobs(&actor)
            .unwrap()
            .into_iter()
            .find(|candidate| candidate.id == job)
            .unwrap()
            .state,
        JobState::Queued
    );
    assert!(service.run_next_job("http-test-worker").unwrap());
}

#[tokio::test]
async fn job_routes_are_submitter_scoped_idempotent_and_secret_free() {
    let (_root, mut state) = state();
    state.ingress = Ingress::Private;
    let owner = state
        .store
        .create_user("job-owner", "a long owner password", false)
        .unwrap();
    let other_admin = state
        .store
        .create_user("other-admin", "a long admin password", true)
        .unwrap();
    let job = JobId::new();
    state
        .store
        .enqueue_job(JobSpec {
            frozen_input: None,
            id: job,
            user_id: owner,
            collection_id: None,
            item_id: None,
            kind: "integrity_verification".into(),
            phase: "queued".into(),
            total_units: 2,
            request: serde_json::json!({"secret_request":"do-not-return"}),
        })
        .unwrap();
    let (owner_token, _) = state.store.issue_session(owner, "browser").unwrap();
    let (admin_token, _) = state.store.issue_session(other_admin, "browser").unwrap();
    let app = router(state);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/jobs?limit=1")
                .header("authorization", format!("Bearer {owner_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 16 * 1024)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["items"][0]["id"], job.to_string());
    let serialized = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(!serialized.contains("secret_request"));
    assert!(!serialized.contains("claim_token"));
    assert!(!serialized.contains("worker_instance"));

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/jobs/{job}"))
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    for _ in 0..2 {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/v1/jobs/{job}/cancel"))
                    .header("origin", "https://collector.example")
                    .header("authorization", format!("Bearer {owner_token}"))
                    .header("idempotency-key", "cancel-same-job")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), 16 * 1024)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["state"], "cancelled");
        assert_eq!(body["cancellable"], false);
    }
}

#[tokio::test]
async fn viewer_item_dto_omits_private_fields_and_admin_without_grant_is_denied() {
    let (_root, mut state) = state();
    state.ingress = Ingress::Private;
    let owner = state
        .store
        .create_user("read-model-owner", "a long owner password", true)
        .unwrap();
    let viewer = state
        .store
        .create_user("read-model-viewer", "a long viewer password", false)
        .unwrap();
    let curator = state
        .store
        .create_user("read-model-curator", "a long curator password", false)
        .unwrap();
    let ungranted_admin = state
        .store
        .create_user("ungranted-admin", "a long admin password", true)
        .unwrap();
    let owner_actor = Principal {
        user_id: owner,
        administrator: true,
        device_id: None,
    };
    let collection = state
        .service
        .create_collection(
            &test_authentication(&owner_actor),
            typed_collection("Read model firewall"),
            &command(),
        )
        .unwrap();
    state
        .service
        .grant(
            &test_authentication(&owner_actor),
            collection.id,
            viewer,
            Role::Viewer,
            &command(),
        )
        .unwrap();
    state
        .service
        .grant(
            &test_authentication(&owner_actor),
            collection.id,
            curator,
            Role::Curator,
            &command(),
        )
        .unwrap();
    let item_id = ItemId::new();
    let item = Item {
        id: item_id,
        collection_id: collection.id,
        revision: Revision(0),
        title: "Viewer-safe title".into(),
        subtitle: String::new(),
        identity_link: ItemIdentityLink::None,
        domain: ModuleData {
            module: ModuleId("org.junk.music".into()),
            schema_version: 1,
            payload: RawJson::from_json(r#"{"private_raw_marker":"do-not-serialize"}"#).unwrap(),
        },
        identity: IdentityState::Unidentified,
        translations: vec![],
        identifiers: vec![],
        specimens: vec![Specimen {
            id: SpecimenId::new(),
            label: "Owned copy".into(),
            condition: Condition::Good,
            ownership: Ownership::Owned,
            possession: Possession::Present,
            components: vec![],
            acquisition: Some(Acquisition {
                date: None,
                cost: None,
                source: Some("private shop marker".into()),
                coordinates: None,
            }),
            private_notes: Some("private note marker".into()),
            loans: vec![Loan {
                id: LoanId::new(),
                contact: "private contact marker".into(),
                lent_at: chrono::NaiveDate::from_ymd_opt(2026, 8, 1).unwrap(),
                due_at: Some(chrono::NaiveDate::from_ymd_opt(2026, 8, 8).unwrap()),
                returned_at: Some(chrono::NaiveDate::from_ymd_opt(2026, 8, 4).unwrap()),
            }],
        }],
        wishlist: false,
        captures: vec![],
        assets: vec![],
        metadata: MetadataState {
            candidates: vec![],
            selected: Some("private provider marker".into()),
            overrides: Default::default(),
        },
        deleted: false,
        updated_at: chrono::Utc::now(),
    };
    // Seed a trusted stored record with provider metadata; the public item
    // command intentionally cannot submit this server-owned state.
    let saved = Catalog::save(&*state.store, item, Revision(0)).unwrap();
    let png = [
        137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 4,
        0, 0, 0, 181, 28, 12, 2, 0, 0, 0, 11, 73, 68, 65, 84, 120, 218, 99, 100, 248, 15, 0, 1, 5,
        1, 1, 39, 24, 227, 102, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
    ];
    let saved = state
        .service
        .publish_cover(
            &owner_actor,
            saved.id,
            saved.revision,
            &png,
            "private asset marker".into(),
            &command(),
        )
        .unwrap();
    let private_asset_id = saved.assets[0].id;
    state
        .service
        .update_asset(
            &owner_actor,
            private_asset_id,
            Revision(1),
            collector_app::UpdateAssetCommand {
                visibility: Visibility::Curators,
                alt_text: "private asset marker".into(),
            },
            &command(),
        )
        .unwrap();
    let upload_bytes = b"read-model-evidence-fixture";
    let upload = state
        .service
        .create_upload(
            &owner_actor,
            saved.id,
            saved.revision,
            vec![UploadFile {
                name: "fixture.bin".into(),
                bytes: ByteLength(upload_bytes.len() as u64),
                sha256: None,
            }],
            &command(),
        )
        .unwrap();
    state
        .service
        .upload_chunk(&owner_actor, upload.id, 0, 0, upload_bytes)
        .unwrap();
    let saved = state
        .service
        .commit_upload(&owner_actor, upload.id, &command())
        .unwrap();
    let (viewer_token, _) = state.store.issue_session(viewer, "browser").unwrap();
    let (curator_token, _) = state.store.issue_session(curator, "browser").unwrap();
    let (admin_token, _) = state
        .store
        .issue_session(ungranted_admin, "browser")
        .unwrap();
    let viewer_actor = Principal {
        user_id: viewer,
        administrator: false,
        device_id: None,
    };
    assert!(matches!(
        state.service.upload(&viewer_actor, upload.id),
        Err(Error::Forbidden)
    ));
    assert!(matches!(
        state
            .service
            .asset(&viewer_actor, saved.id, private_asset_id, false),
        Err(Error::Forbidden)
    ));
    let app = router(state);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/items/{}", saved.id))
                .header("authorization", format!("Bearer {viewer_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 64 * 1024)
        .await
        .unwrap();
    let text = String::from_utf8(bytes.to_vec()).unwrap();
    for private_marker in [
        "private_raw_marker",
        "do-not-serialize",
        "private shop marker",
        "private note marker",
        "private contact marker",
        "private provider marker",
    ] {
        assert!(
            !text.contains(private_marker),
            "viewer response leaked {private_marker}: {text}"
        );
    }
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(body["domain"].get("payload").is_none());
    assert!(body.get("metadata").is_none());
    assert!(body["specimens"][0].get("acquisition").is_none());
    assert!(body["specimens"][0].get("private_notes").is_none());
    assert!(body["specimens"][0].get("loans").is_none());
    assert_eq!(body["effective_permissions"]["can_browse"], true);
    assert_eq!(body["effective_permissions"]["can_curate"], false);
    assert_eq!(body["effective_permissions"]["can_manage_access"], false);
    assert_eq!(body["presentation"]["values"], serde_json::json!([]));
    assert!(body["captures"][0].get("legacy_payload_locator").is_none());

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/items?collection_id={}", collection.id))
                .header("authorization", format!("Bearer {viewer_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 64 * 1024)
        .await
        .unwrap();
    let text = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(!text.contains("private shop marker"));
    assert!(!text.contains("private note marker"));
    assert!(!text.contains("private contact marker"));
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    for field in ["acquisition", "private_notes", "loans"] {
        assert!(
            body["items"][0]["specimens"][0].get(field).is_none(),
            "browse DTO serialized private specimen field {field}"
        );
    }

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/items/{}/evidence", saved.id))
                .header("authorization", format!("Bearer {viewer_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 64 * 1024)
        .await
        .unwrap();
    let evidence: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(evidence[0]["kind"], "integrity");
    assert_eq!(evidence[0]["outcome"], "passed");
    for private_field in [
        "inputs",
        "implementation",
        "implementation_version",
        "catalog",
        "details",
    ] {
        assert!(evidence[0].get(private_field).is_none());
    }

    for private_path in [
        format!("/api/v1/uploads/{}", upload.id),
        format!("/api/v1/items/{}/assets/{private_asset_id}", saved.id),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(private_path)
                    .header("authorization", format!("Bearer {viewer_token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/items/{}", saved.id))
                .header("authorization", format!("Bearer {curator_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 64 * 1024)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        body["domain"]["payload"]["private_raw_marker"],
        "do-not-serialize"
    );
    assert_eq!(body["metadata"]["selected"], "private provider marker");
    assert_eq!(body["specimens"][0]["private_notes"], "private note marker");
    assert_eq!(
        body["specimens"][0]["loans"][0]["contact"],
        "private contact marker"
    );
    assert_eq!(body["effective_permissions"]["can_curate"], true);
    assert_eq!(body["effective_permissions"]["can_manage_access"], false);
    assert!(body["captures"][0].get("legacy_payload_locator").is_none());

    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/items/{}", saved.id))
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn asset_routes_filter_metadata_and_bytes_before_role_aware_artwork_selection() {
    let (_root, mut state) = state();
    state.exposure.public_curation = true;
    let owner = state
        .store
        .create_user("asset-owner", "a long owner password", true)
        .unwrap();
    let viewer = state
        .store
        .create_user("asset-viewer", "a long viewer password", false)
        .unwrap();
    let stranger = state
        .store
        .create_user("asset-stranger", "a long stranger password", false)
        .unwrap();
    let owner_actor = Principal {
        user_id: owner,
        administrator: true,
        device_id: None,
    };
    let collection = state
        .service
        .create_collection(
            &test_authentication(&owner_actor),
            typed_collection("Asset authorization"),
            &command(),
        )
        .unwrap();
    state
        .service
        .grant(
            &test_authentication(&owner_actor),
            collection.id,
            viewer,
            Role::Viewer,
            &command(),
        )
        .unwrap();
    let other_collection = state
        .service
        .create_collection(
            &test_authentication(&owner_actor),
            typed_collection("Other asset collection"),
            &command(),
        )
        .unwrap();
    let work = Catalog::save_work(
        &*state.store,
        Work {
            id: WorkId::new(),
            collection_id: collection.id,
            revision: Revision(0),
            provisional: false,
            title: "Inherited art work".into(),
            translations: vec![],
            identifiers: vec![],
            migration_provenance: vec![],
            deleted: false,
            updated_at: chrono::Utc::now(),
        },
        Revision(0),
    )
    .unwrap();
    let edition = Catalog::save_edition(
        &*state.store,
        Edition {
            id: EditionId::new(),
            collection_id: collection.id,
            revision: Revision(0),
            work_id: Some(work.id),
            provisional: false,
            title: "Inherited art edition".into(),
            expected_components: vec![],
            identifiers: vec![],
            migration_provenance: vec![],
            deleted: false,
            updated_at: chrono::Utc::now(),
        },
        Revision(0),
    )
    .unwrap();
    let item = Catalog::save(
        &*state.store,
        Item {
            id: ItemId::new(),
            collection_id: collection.id,
            revision: Revision(0),
            title: "Inherited art item".into(),
            subtitle: String::new(),
            identity_link: ItemIdentityLink::Edition {
                edition_id: edition.id,
            },
            domain: ModuleData {
                module: ModuleId("org.junk.music".into()),
                schema_version: 1,
                payload: RawJson::from_value(serde_json::json!({})),
            },
            identity: IdentityState::Unidentified,
            translations: vec![],
            identifiers: vec![],
            specimens: vec![],
            wishlist: false,
            captures: vec![],
            assets: vec![],
            metadata: MetadataState::default(),
            deleted: false,
            updated_at: chrono::Utc::now(),
        },
        Revision(0),
    )
    .unwrap();
    let (owner_token, _) = state.store.issue_session(owner, "owner browser").unwrap();
    let (viewer_token, _) = state.store.issue_session(viewer, "viewer browser").unwrap();
    let (stranger_token, _) = state
        .store
        .issue_session(stranger, "stranger browser")
        .unwrap();
    let app = router(state);
    let png = [
        137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 4,
        0, 0, 0, 181, 28, 12, 2, 0, 0, 0, 11, 73, 68, 65, 84, 120, 218, 99, 100, 248, 15, 0, 1, 5,
        1, 1, 39, 24, 227, 102, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
    ];
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/v1/collections/{}/assets?subject_kind=work&subject_id={}&kind=cover&visibility=shared&alt_text=Inherited%20cover",
                    collection.id, work.id
                ))
                .header("authorization", format!("Bearer {owner_token}"))
                .header("origin", "https://collector.example")
                .header("idempotency-key", "http-work-asset")
                .header("content-type", "application/octet-stream")
                .body(Body::from(png.to_vec()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), 64 * 1024)
        .await
        .unwrap();
    let created: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let asset_id: AssetId = created["id"].as_str().unwrap().parse().unwrap();
    assert_eq!(created["revision"], 1);
    assert_eq!(created["visibility"], "shared");
    assert!(created.get("original").is_some());
    assert!(created.get("thumbnail").is_some());

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/v1/collections/{}/assets/{asset_id}",
                    collection.id
                ))
                .header("authorization", format!("Bearer {viewer_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), 64 * 1024)
        .await
        .unwrap();
    let viewer_asset: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(viewer_asset.get("visibility").is_none());
    assert!(viewer_asset.get("original").is_none());
    assert!(viewer_asset.get("thumbnail").is_none());
    assert_eq!(viewer_asset["alt_text"], "Inherited cover");
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/v1/collections/{}/assets/{asset_id}",
                    other_collection.id
                ))
                .header("authorization", format!("Bearer {owner_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    for path in [
        format!(
            "/api/v1/collections/{}/assets/{asset_id}/original",
            collection.id
        ),
        format!(
            "/api/v1/collections/{}/assets/{asset_id}/thumbnail",
            collection.id
        ),
        format!("/api/v1/items/{}/assets/{asset_id}", item.id),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(path)
                    .header("authorization", format!("Bearer {viewer_token}"))
                    .header("range", "bytes=0-7")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT);
    }
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/items/{}", item.id))
                .header("authorization", format!("Bearer {viewer_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = axum::body::to_bytes(response.into_body(), 64 * 1024)
        .await
        .unwrap();
    let presented: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(presented["primary_artwork"], asset_id.to_string());
    assert_eq!(presented["assets"][0]["subject"]["kind"], "work");

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!(
                    "/api/v1/collections/{}/assets/{asset_id}/metadata",
                    collection.id
                ))
                .header("authorization", format!("Bearer {owner_token}"))
                .header("origin", "https://collector.example")
                .header("idempotency-key", "http-private-asset")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&serde_json::json!({
                        "expected_revision": 1,
                        "command": {"visibility":"curators","alt_text":"Private cover marker"}
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), 64 * 1024)
        .await
        .unwrap();
    let updated: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(updated["revision"], 2);
    assert_eq!(updated["subject"], created["subject"]);

    for path in [
        format!("/api/v1/collections/{}/assets/{asset_id}", collection.id),
        format!(
            "/api/v1/collections/{}/assets/{asset_id}/original",
            collection.id
        ),
        format!(
            "/api/v1/collections/{}/assets/{asset_id}/thumbnail",
            collection.id
        ),
        format!("/api/v1/items/{}/assets/{asset_id}", item.id),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(path)
                    .header("authorization", format!("Bearer {viewer_token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/items/{}", item.id))
                .header("authorization", format!("Bearer {viewer_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = axum::body::to_bytes(response.into_body(), 64 * 1024)
        .await
        .unwrap();
    let viewer_item: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(viewer_item.get("primary_artwork").is_none());
    assert_eq!(viewer_item["assets"], serde_json::json!([]));
    assert!(
        !String::from_utf8(body.to_vec())
            .unwrap()
            .contains("Private cover marker")
    );

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/v1/collections/{}/assets/{asset_id}",
                    collection.id
                ))
                .header("authorization", format!("Bearer {stranger_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/v1/collections/{}/assets/{asset_id}/archive?expected_revision=2",
                    collection.id
                ))
                .header("authorization", format!("Bearer {owner_token}"))
                .header("origin", "https://collector.example")
                .header("idempotency-key", "http-tombstone-asset")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), 64 * 1024)
        .await
        .unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&body).unwrap()["archived"],
        true
    );
    for path in [
        format!("/api/v1/collections/{}/assets/{asset_id}", collection.id),
        format!(
            "/api/v1/collections/{}/assets/{asset_id}/original",
            collection.id
        ),
        format!(
            "/api/v1/collections/{}/assets/{asset_id}/thumbnail",
            collection.id
        ),
        format!("/api/v1/items/{}/assets/{asset_id}", item.id),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(path)
                    .header("authorization", format!("Bearer {owner_token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}

#[tokio::test]
async fn bounded_item_command_accepts_editable_fields_and_rejects_owned_state() {
    let (_root, mut state) = state();
    state.ingress = Ingress::Private;
    let owner = state
        .store
        .create_user("command-owner", "a long owner password", true)
        .unwrap();
    let actor = Principal {
        user_id: owner,
        administrator: true,
        device_id: None,
    };
    let collection = state
        .service
        .create_collection(
            &test_authentication(&actor),
            typed_collection("Commands"),
            &command(),
        )
        .unwrap();
    let (token, _) = state.store.issue_session(owner, "browser").unwrap();
    let id = ItemId::new();
    let command = serde_json::json!({
        "collection_id": collection.id,
        "title": "Bounded command",
        "subtitle": "Editable",
        "identity_link": {"kind":"none"},
        "domain": {
            "module": "org.junk.music",
            "schema_version": 1,
            "payload": {"preserved":true}
        },
        "identity": "unidentified",
        "translations": [],
        "identifiers": [],
        "wishlist": false
    });
    let app = router(state);
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/v1/items/{id}"))
                .header("origin", "https://collector.example")
                .header("authorization", format!("Bearer {token}"))
                .header("idempotency-key", "bounded-item-create")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&serde_json::json!({
                        "command": command,
                        "expected_revision": 0
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let response_body = axum::body::to_bytes(response.into_body(), 8192)
        .await
        .unwrap();
    assert_eq!(
        status,
        StatusCode::OK,
        "{}",
        String::from_utf8_lossy(&response_body)
    );

    for forbidden_field in [
        "specimens",
        "captures",
        "assets",
        "evidence",
        "representations",
        "legacy_payload_locator",
        "metadata",
        "revision",
        "updated_at",
    ] {
        let mut injected = command.clone();
        injected
            .as_object_mut()
            .unwrap()
            .insert(forbidden_field.into(), serde_json::json!([]));
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/api/v1/items/{id}"))
                    .header("origin", "https://collector.example")
                    .header("authorization", format!("Bearer {token}"))
                    .header(
                        "idempotency-key",
                        format!("forbidden-field-{forbidden_field}"),
                    )
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&serde_json::json!({
                            "command": injected,
                            "expected_revision": 1
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            response.status(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "server-owned field {forbidden_field} was accepted"
        );
    }
}

#[tokio::test]
async fn focused_inventory_routes_require_curator_role_and_revisioned_idempotent_commands() {
    let (_root, mut state) = state();
    state.ingress = Ingress::Private;
    let owner = state
        .store
        .create_user("inventory-owner", "a long owner password", true)
        .unwrap();
    let viewer = state
        .store
        .create_user("inventory-viewer", "a long viewer password", false)
        .unwrap();
    let actor = Principal {
        user_id: owner,
        administrator: true,
        device_id: None,
    };
    let collection = state
        .service
        .create_collection(
            &test_authentication(&actor),
            typed_collection("Inventory"),
            &command(),
        )
        .unwrap();
    state
        .store
        .grant(collection.id, viewer, Role::Viewer)
        .unwrap();
    let stored = Item {
        id: ItemId::new(),
        collection_id: collection.id,
        revision: Revision(0),
        title: "Lifecycle".into(),
        subtitle: String::new(),
        identity_link: ItemIdentityLink::None,
        domain: ModuleData {
            module: ModuleId("org.junk.music".into()),
            schema_version: 1,
            payload: RawJson::from_value(serde_json::json!({})),
        },
        identity: IdentityState::Unidentified,
        translations: vec![],
        identifiers: vec![],
        specimens: vec![],
        wishlist: true,
        captures: vec![],
        assets: vec![],
        metadata: MetadataState::default(),
        deleted: false,
        updated_at: chrono::Utc::now(),
    };
    let saved = state
        .service
        .save_item(&actor, stored.id, stored.into(), Revision(0), &command())
        .unwrap();
    let (owner_token, _) = state.store.issue_session(owner, "owner browser").unwrap();
    let (viewer_token, _) = state.store.issue_session(viewer, "viewer browser").unwrap();
    let app = router(state);
    let body = serde_json::to_vec(&serde_json::json!({
        "expected_revision": saved.revision,
        "details": {
            "label": "Owned copy",
            "condition": "good",
            "ownership": "owned",
            "possession": "present",
            "acquisition": null,
            "private_notes": "private"
        }
    }))
    .unwrap();

    let denied = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/items/{}/specimens", saved.id))
                .header("origin", "https://collector.example")
                .header("authorization", format!("Bearer {viewer_token}"))
                .header("idempotency-key", "viewer-cannot-create-specimen")
                .header("content-type", "application/json")
                .body(Body::from(body.clone()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);

    let missing_key = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/items/{}/specimens", saved.id))
                .header("origin", "https://collector.example")
                .header("authorization", format!("Bearer {owner_token}"))
                .header("content-type", "application/json")
                .body(Body::from(body.clone()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(missing_key.status(), StatusCode::BAD_REQUEST);

    for _ in 0..2 {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/v1/items/{}/specimens", saved.id))
                    .header("origin", "https://collector.example")
                    .header("authorization", format!("Bearer {owner_token}"))
                    .header("idempotency-key", "create-owned-specimen")
                    .header("content-type", "application/json")
                    .body(Body::from(body.clone()))
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), 32 * 1024)
            .await
            .unwrap();
        assert_eq!(
            status,
            StatusCode::OK,
            "{}",
            String::from_utf8_lossy(&bytes)
        );
        let item: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(item["specimens"].as_array().unwrap().len(), 1);
        assert_eq!(item["specimens"][0]["private_notes"], "private");
    }
}

#[tokio::test]
async fn mutation_requires_key_and_http_replay_has_stable_conflict_code() {
    let (_root, mut state) = state();
    state.ingress = Ingress::Private;
    let owner = state
        .store
        .create_user("idempotency-owner", "a long owner password", true)
        .unwrap();
    let actor = Principal {
        user_id: owner,
        administrator: true,
        device_id: None,
    };
    let collection = state
        .service
        .create_collection(
            &test_authentication(&actor),
            typed_collection("Idempotency"),
            &command(),
        )
        .unwrap();
    let (token, _) = state.store.issue_session(owner, "browser").unwrap();
    let id = ItemId::new();
    let command = serde_json::json!({
        "collection_id": collection.id,
        "title": "One effect",
        "subtitle": "",
        "identity_link": {"kind":"none"},
        "domain": {"module":"org.junk.music","schema_version":1,"payload":{}},
        "identity": "unidentified",
        "translations": [],
        "identifiers": [],
        "wishlist": false
    });
    let body = serde_json::to_vec(&serde_json::json!({
        "command": command,
        "expected_revision": 0
    }))
    .unwrap();
    let app = router(state);

    let missing = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/v1/items/{id}"))
                .header("origin", "https://collector.example")
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(body.clone()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(missing.status(), StatusCode::BAD_REQUEST);

    for _ in 0..2 {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/api/v1/items/{id}"))
                    .header("origin", "https://collector.example")
                    .header("authorization", format!("Bearer {token}"))
                    .header("idempotency-key", "http-retry-key")
                    .header("content-type", "application/json")
                    .body(Body::from(body.clone()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    let mut changed = command;
    changed["title"] = serde_json::json!("Changed payload");
    let conflict = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/v1/items/{id}"))
                .header("origin", "https://collector.example")
                .header("authorization", format!("Bearer {token}"))
                .header("idempotency-key", "http-retry-key")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&serde_json::json!({
                        "command": changed,
                        "expected_revision": 0
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(conflict.status(), StatusCode::CONFLICT);
    let body = axum::body::to_bytes(conflict.into_body(), 8192)
        .await
        .unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&body).unwrap()["error"]["code"],
        "idempotency_conflict"
    );
}

#[tokio::test]
async fn wave1_item_envelope_preserves_server_owned_state_and_rejects_changes() {
    let (_root, mut state) = state();
    state.ingress = Ingress::Private;
    let owner = state
        .store
        .create_user("legacy-owner", "a long owner password", true)
        .unwrap();
    let actor = Principal {
        user_id: owner,
        administrator: true,
        device_id: None,
    };
    let collection = state
        .service
        .create_collection(
            &test_authentication(&actor),
            typed_collection("Legacy client"),
            &command(),
        )
        .unwrap();
    let stored = Item {
        id: ItemId::new(),
        collection_id: collection.id,
        revision: Revision(0),
        title: "Before".into(),
        subtitle: String::new(),
        identity_link: ItemIdentityLink::None,
        domain: ModuleData {
            module: ModuleId("org.junk.music".into()),
            schema_version: 1,
            payload: RawJson::from_value(serde_json::json!({})),
        },
        identity: IdentityState::Unidentified,
        translations: vec![],
        identifiers: vec![],
        specimens: vec![],
        wishlist: false,
        captures: vec![],
        assets: vec![],
        metadata: MetadataState::default(),
        deleted: false,
        updated_at: chrono::Utc::now(),
    };
    let saved = state
        .service
        .save_item(&actor, stored.id, stored.into(), Revision(0), &command())
        .unwrap();
    let mut legacy = Catalog::item(&*state.store, saved.id).unwrap();
    legacy.title = "Updated through Wave 1 envelope".into();
    let store = state.store.clone();
    let (token, _) = store.issue_session(owner, "browser").unwrap();
    let app = router(state);
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/v1/items/{}", saved.id))
                .header("origin", "https://collector.example")
                .header("authorization", format!("Bearer {token}"))
                .header("idempotency-key", "wave1-compatible-update")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&serde_json::json!({
                        "item": legacy,
                        "expected_revision": saved.revision
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        Catalog::item(&*store, saved.id).unwrap().title,
        "Updated through Wave 1 envelope"
    );

    let mut tampered = Catalog::item(&*store, saved.id).unwrap();
    tampered.deleted = true;
    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/v1/items/{}", saved.id))
                .header("origin", "https://collector.example")
                .header("authorization", format!("Bearer {token}"))
                .header("idempotency-key", "wave1-owned-field-change")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&serde_json::json!({
                        "item": tampered,
                        "expected_revision": Revision(2)
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert!(!Catalog::item(&*store, saved.id).unwrap().deleted);
}

#[tokio::test]
async fn static_modules_load_for_wrong_origin_guidance_but_api_stays_protected() {
    let (_root, state) = state();
    let app = router(state);
    for (path, status) in [
        ("/app.js", StatusCode::OK),
        ("/api/v1/features", StatusCode::FORBIDDEN),
        ("/administration.js", StatusCode::NOT_FOUND),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(path)
                    .header("origin", "http://127.0.0.1:9842")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), status, "{path}");
    }
}
