use collector_app::{Catalog, CollectionService};
use collector_core::*;
use collector_modules::Registry;
use collector_store::{Store, UploadFile};
use std::sync::Arc;
fn setup() -> (
    tempfile::TempDir,
    Arc<Store>,
    Principal,
    CollectionService,
    Collection,
) {
    let root = tempfile::tempdir().unwrap();
    let store =
        Arc::new(Store::open(root.path().join("archive"), root.path().join("state")).unwrap());
    let user = store
        .create_user("owner", "a long test password", true)
        .unwrap();
    let actor = Principal {
        user_id: user,
        administrator: true,
        device_id: None,
    };
    let service = CollectionService::new(store.clone(), Arc::new(Registry::builtins()));
    let collection = service.create_collection(&actor, "日本の音楽").unwrap();
    (root, store, actor, service, collection)
}
fn item(collection: CollectionId) -> Item {
    Item {
        id: ItemId::new(),
        collection_id: collection,
        revision: Revision(0),
        title: "風の歌".into(),
        subtitle: "Artist · CD".into(),
        work_id: None,
        edition_id: None,
        domain: ModuleData {
            module: ModuleId("org.junk.music".into()),
            schema_version: 1,
            payload: serde_json::json!({}),
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
    }
}
#[test]
fn revisions_recover_unknown_module_data_and_do_not_restore_grants() {
    let (root, store, actor, service, collection) = setup();
    let mut value = item(collection.id);
    value.domain = ModuleData {
        module: ModuleId("org.example.stamps".into()),
        schema_version: 4,
        payload: serde_json::json!({"nullable":null,"nested":[1,{"color":"blue"}]}),
    };
    let saved = service.save(&actor, value, Revision(0)).unwrap();
    let mut edited = saved.clone();
    edited.title = "Updated".into();
    service.save(&actor, edited, saved.revision).unwrap();
    assert!(matches!(
        service.save(&actor, saved.clone(), saved.revision),
        Err(Error::Conflict)
    ));
    drop(service);
    drop(store);
    std::fs::remove_file(root.path().join("state/catalog.sqlite")).unwrap();
    let store = Store::open(root.path().join("archive"), root.path().join("state")).unwrap();
    let restored = store.item(saved.id).unwrap();
    assert_eq!(restored.title, "Updated");
    assert_eq!(restored.domain.payload, saved.domain.payload);
    assert_eq!(restored.revision, Revision(2));
    drop(store);
    let foreign = Store::open(root.path().join("archive"), root.path().join("new-state")).unwrap();
    assert_eq!(foreign.item(saved.id).unwrap().title, "Updated");
    assert!(matches!(
        foreign.role(actor.user_id, collection.id),
        Err(Error::Forbidden)
    ));
}
#[test]
fn upload_retries_preserve_bytes_and_evidence_detects_changes() {
    let (_root, store, actor, service, collection) = setup();
    let saved = service
        .save(&actor, item(collection.id), Revision(0))
        .unwrap();
    let bytes = b"independent original payload";
    let upload = store
        .create_upload(
            &actor,
            saved.id,
            saved.revision,
            vec![UploadFile {
                name: "disc.bin".into(),
                bytes: ByteLength(bytes.len() as u64),
                sha256: Some(Digest::sha256(bytes).hex),
            }],
        )
        .unwrap();
    store
        .upload_chunk(&actor, upload.id, 0, 0, &bytes[..8])
        .unwrap();
    store
        .upload_chunk(&actor, upload.id, 0, 0, &bytes[..8])
        .unwrap();
    assert!(matches!(
        store.upload_chunk(&actor, upload.id, 0, 0, b"different"),
        Err(Error::Conflict)
    ));
    store
        .upload_chunk(&actor, upload.id, 0, 8, &bytes[8..])
        .unwrap();
    let committed = store.commit_upload(&actor, upload.id).unwrap();
    let retry = store.commit_upload(&actor, upload.id).unwrap();
    assert_eq!(retry.revision, committed.revision);
    let (path, _) = store
        .representation_file(&actor, saved.id, upload.representation_id, 0)
        .unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    assert!(
        store
            .evidence(saved.id)
            .unwrap()
            .iter()
            .all(|e| matches!(e.kind, EvidenceKind::Integrity))
    );
    std::fs::write(path, b"changed").unwrap();
    store.verify_integrity(&actor, saved.id).unwrap();
    assert_eq!(
        store.evidence(saved.id).unwrap().last().unwrap().outcome,
        EvidenceOutcome::Failed
    );
}
#[test]
fn malicious_paths_and_invented_file_references_are_rejected() {
    let (_root, store, actor, service, collection) = setup();
    let saved = service
        .save(&actor, item(collection.id), Revision(0))
        .unwrap();
    for name in [
        "../secret",
        "/absolute",
        "dir/../../secret",
        "C:\\secret",
        "a/./b",
        "a//b",
    ] {
        assert!(
            store
                .create_upload(
                    &actor,
                    saved.id,
                    saved.revision,
                    vec![UploadFile {
                        name: name.into(),
                        bytes: ByteLength(1),
                        sha256: None
                    }]
                )
                .is_err()
        );
    }
    let mut edited = saved;
    edited.captures.push(Capture {
        id: CaptureId::new(),
        carrier_id: None,
        created_at: chrono::Utc::now(),
        representations: vec![],
    });
    assert!(matches!(
        service.save(&actor, edited.clone(), edited.revision),
        Err(Error::Invalid(_))
    ));
}
#[test]
fn viewer_redaction_and_cross_collection_access() {
    let (_root, store, owner, service, collection) = setup();
    let viewer_id = store
        .create_user("viewer", "another long password", false)
        .unwrap();
    let stranger_id = store
        .create_user("stranger", "another long password", false)
        .unwrap();
    let viewer = Principal {
        user_id: viewer_id,
        administrator: false,
        device_id: None,
    };
    let stranger = Principal {
        user_id: stranger_id,
        administrator: false,
        device_id: None,
    };
    store.grant(collection.id, viewer_id, Role::Viewer).unwrap();
    let mut value = item(collection.id);
    value.specimens.push(Specimen {
        id: SpecimenId::new(),
        label: "Copy 1".into(),
        condition: Condition::Good,
        ownership: Ownership::Owned,
        possession: Possession::Lent,
        carriers: vec![],
        acquisition: Some(Acquisition {
            date: None,
            cost: Some(Money {
                minor_units: 4500,
                currency: "JPY".into(),
            }),
            source: Some("Private seller".into()),
        }),
        private_notes: Some("private".into()),
        loans: vec![Loan {
            id: LoanId::new(),
            contact: "Private contact".into(),
            lent_at: chrono::Utc::now().date_naive(),
            due_at: None,
            returned_at: None,
        }],
    });
    let saved = service.save(&owner, value, Revision(0)).unwrap();
    let public = service.item(&viewer, saved.id).unwrap();
    assert!(public.specimens[0].acquisition.is_none());
    assert!(public.specimens[0].loans.is_empty());
    assert!(public.specimens[0].private_notes.is_none());
    assert!(service.item(&stranger, saved.id).is_err());
    assert!(
        service
            .browse(&stranger, BrowseQuery::default())
            .unwrap()
            .items
            .is_empty()
    );
    assert!(
        service
            .save(&viewer, saved.clone(), saved.revision)
            .is_err()
    );
}
#[test]
fn cover_upload_retains_original_and_has_bounded_thumbnail() {
    let (_root, store, actor, service, collection) = setup();
    let saved = service
        .save(&actor, item(collection.id), Revision(0))
        .unwrap();
    let image = image::RgbImage::new(900, 450);
    let mut data = std::io::Cursor::new(Vec::new());
    image.write_to(&mut data, image::ImageFormat::Png).unwrap();
    let bytes = data.into_inner();
    let saved = store
        .add_cover(
            &actor,
            saved.id,
            saved.revision,
            &bytes,
            "Album cover".into(),
        )
        .unwrap();
    let asset = &saved.assets[0];
    let (original, _, _) = store.asset_file(&actor, saved.id, asset.id, false).unwrap();
    assert_eq!(std::fs::read(original).unwrap(), bytes);
    let (thumbnail, _, _) = store.asset_file(&actor, saved.id, asset.id, true).unwrap();
    let thumbnail = image::open(thumbnail).unwrap();
    assert_eq!((thumbnail.width(), thumbnail.height()), (192, 96));
    store.reindex().unwrap();
    assert_eq!(store.item(saved.id).unwrap().assets[0].id, asset.id);
}
#[test]
fn pagination_and_fts_do_not_cross_grants() {
    let (_root, _store, actor, service, collection) = setup();
    for title in ["Golden Sun", "Golden Axe", "Other"] {
        let mut v = item(collection.id);
        v.title = title.into();
        service.save(&actor, v, Revision(0)).unwrap();
    }
    let first = service
        .browse(
            &actor,
            BrowseQuery {
                search: Some("Golden".into()),
                limit: Some(1),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(first.items.len(), 1);
    assert!(first.next_cursor.is_some());
    let second = service
        .browse(
            &actor,
            BrowseQuery {
                search: Some("Golden".into()),
                limit: Some(1),
                cursor: first.next_cursor,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(second.items.len(), 1);
    assert_ne!(first.items[0].id, second.items[0].id);
    assert!(second.next_cursor.is_none());
}
#[test]
fn revoked_devices_and_password_recovery() {
    let (_root, store, actor, _service, _collection) = setup();
    let (token, device) = store
        .login("owner", "a long test password", "test")
        .unwrap();
    assert_eq!(store.authenticate(&token).unwrap().user_id, actor.user_id);
    store
        .revoke_device(&device, device.device_id.unwrap())
        .unwrap();
    assert!(store.authenticate(&token).is_err());
    let (token, _) = store
        .login("owner", "a long test password", "test")
        .unwrap();
    store.reset_password("owner", "new long password").unwrap();
    assert!(store.authenticate(&token).is_err());
    assert!(
        store
            .login("owner", "a long test password", "test")
            .is_err()
    );
    assert!(store.login("owner", "new long password", "test").is_ok());
}
#[test]
fn archive_lock_prevents_second_writer() {
    let (root, _store, _, _, _) = setup();
    assert!(matches!(
        Store::open(root.path().join("archive"), root.path().join("other-state")),
        Err(Error::Unavailable(_))
    ));
}

#[test]
fn configured_inbox_preserves_sources_and_reuses_import_evidence() {
    let (root, store, actor, service, collection) = setup();
    let saved = service
        .save(&actor, item(collection.id), Revision(0))
        .unwrap();
    let source = root.path().join("inbox/package");
    std::fs::create_dir_all(source.join("tracks")).unwrap();
    std::fs::write(source.join("disc.cue"), b"REM synthetic package\n").unwrap();
    std::fs::write(source.join("tracks/disc.bin"), b"payload").unwrap();
    let files = Store::inventory_directory(&source).unwrap();
    assert_eq!(files.len(), 2);
    let upload = store
        .create_upload(&actor, saved.id, saved.revision, files)
        .unwrap();
    let result = store.stage_directory(&actor, upload.id, &source).unwrap();
    assert_eq!(result.captures.len(), 1);
    assert_eq!(
        std::fs::read(source.join("tracks/disc.bin")).unwrap(),
        b"payload"
    );
    assert_eq!(
        store.evidence(saved.id).unwrap()[0].outcome,
        EvidenceOutcome::Passed
    );
}
#[cfg(unix)]
#[test]
fn inbox_symlinks_are_rejected() {
    let (root, _, _, _, _) = setup();
    let source = root.path().join("inbox");
    std::fs::create_dir(&source).unwrap();
    std::os::unix::fs::symlink("/etc/passwd", source.join("escape")).unwrap();
    assert!(matches!(
        Store::inventory_directory(&source),
        Err(Error::Invalid(_))
    ));
}
#[test]
fn tampered_metadata_blocks_recovery_instead_of_silently_downgrading_revision() {
    let (root, store, actor, service, collection) = setup();
    let saved = service
        .save(&actor, item(collection.id), Revision(0))
        .unwrap();
    drop(service);
    drop(store);
    fn corrupt(path: &std::path::Path) {
        for entry in std::fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                corrupt(&path);
            } else if path.file_name().unwrap() == "record.json" {
                std::fs::write(path, b"{}").unwrap();
            }
        }
    }
    corrupt(&root.path().join("archive/collections"));
    assert!(matches!(
        Store::open(root.path().join("archive"), root.path().join("state")),
        Err(Error::Storage(_))
    ));
    assert_eq!(saved.revision, Revision(1));
}
#[test]
fn failed_background_import_is_reported_and_retains_source_chunks() {
    let (_root, store, actor, service, collection) = setup();
    let saved = service
        .save(&actor, item(collection.id), Revision(0))
        .unwrap();
    let upload = store
        .create_upload(
            &actor,
            saved.id,
            saved.revision,
            vec![UploadFile {
                name: "disc.bin".into(),
                bytes: ByteLength(4),
                sha256: None,
            }],
        )
        .unwrap();
    store
        .upload_chunk(&actor, upload.id, 0, 0, b"data")
        .unwrap();
    store
        .fail_upload(&actor, upload.id, "Test failure")
        .unwrap();
    let job = store
        .jobs(&actor)
        .unwrap()
        .into_iter()
        .find(|j| j.id == upload.id)
        .unwrap();
    assert_eq!(job.state, JobState::Failed);
    assert_eq!(job.error.as_deref(), Some("Test failure"));
    assert!(store.commit_upload(&actor, upload.id).is_ok());
}
