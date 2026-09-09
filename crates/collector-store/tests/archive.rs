use collector_app::{Catalog, CollectionService};
use collector_core::*;
use collector_modules::Registry;
use collector_store::{Store, UploadFile};
use std::path::Path;
use std::sync::Arc;

fn copy_tree(source: &Path, target: &Path) {
    std::fs::create_dir_all(target).unwrap();
    for entry in std::fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let destination = target.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &destination);
        } else {
            std::fs::copy(entry.path(), destination).unwrap();
        }
    }
}
fn rewrite_record(path: &Path, update: impl FnOnce(&mut serde_json::Value)) {
    let mut value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    update(&mut value);
    let bytes = serde_json::to_vec_pretty(&value).unwrap();
    std::fs::write(path, &bytes).unwrap();
    let manifest = path.parent().unwrap().join("manifest.toml");
    let digest = Digest::sha256(&bytes).hex;
    let updated = std::fs::read_to_string(&manifest)
        .unwrap()
        .lines()
        .map(|line| {
            if line.starts_with("record_sha256 =") {
                format!("record_sha256 = \"{digest}\"")
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    std::fs::write(manifest, updated).unwrap();
}
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
    }
}

#[test]
fn committed_schema_one_fixture_migrates_additively_and_restarts() {
    let root = tempfile::tempdir().unwrap();
    let archive = root.path().join("archive");
    copy_tree(
        Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/schema-1"
        )),
        &archive,
    );
    let original = std::fs::read(archive.join("collections/synthetic--018f0000-0000-7000-8000-000000000001/items/two-disc--018f0000-0000-7000-8000-000000000010/revisions/00000000000000000002/record.json")).unwrap();
    let original_digest = Digest::sha256(&original);
    let store = Store::open(&archive, root.path().join("state")).unwrap();
    let item = store
        .item("018f0000-0000-7000-8000-000000000010".parse().unwrap())
        .unwrap();
    assert_eq!(item.revision, Revision(2));
    assert!(matches!(
        item.identity_link,
        ItemIdentityLink::Edition { .. }
    ));
    assert_eq!(
        item.domain.payload.get(),
        r#"{"z":1e+02,"a":null,"ordered":[3,2,1]}"#
    );
    assert_eq!(
        item.metadata.candidates[0].observation.get(),
        r#"{"providerOrder":2,"number":1.00,"missing":null}"#
    );
    assert_eq!(item.specimens.len(), 2);
    assert_eq!(item.specimens[0].components.len(), 4);
    assert!(item.specimens[0].components[0].carrier.is_some());
    assert!(item.specimens[0].components[2].carrier.is_none());
    assert_eq!(
        item.specimens[0]
            .acquisition
            .as_ref()
            .unwrap()
            .cost
            .as_ref()
            .unwrap()
            .currency
            .as_str(),
        "JPY"
    );
    assert_eq!(item.captures[0].revision, Revision(2));
    assert_eq!(
        item.captures[0].representations[0].files[0].digest.hex,
        "a".repeat(64)
    );
    assert!(
        String::from_utf8(original.clone())
            .unwrap()
            .contains(&"A".repeat(64))
    );
    assert!(
        archive
            .join(".archive-migrations/schema-1-to-2/complete")
            .exists()
    );
    assert!(archive.join("collections/synthetic--018f0000-0000-7000-8000-000000000001/works/018f0000-0000-7000-8000-000000000020/revisions/00000000000000000001/record.json").exists());
    assert_eq!(Digest::sha256(&std::fs::read(archive.join("collections/synthetic--018f0000-0000-7000-8000-000000000001/items/two-disc--018f0000-0000-7000-8000-000000000010/revisions/00000000000000000002/record.json")).unwrap()), original_digest);
    let user = store
        .create_user("fixture-owner", "a sufficiently long password", true)
        .unwrap();
    store.grant(item.collection_id, user, Role::Owner).unwrap();
    let actor = Principal {
        user_id: user,
        administrator: true,
        device_id: None,
    };
    let (legacy_file, _) = store
        .representation_file(
            &actor,
            item.id,
            "018f0000-0000-7000-8000-000000000060".parse().unwrap(),
            0,
        )
        .unwrap();
    assert_eq!(std::fs::read(legacy_file).unwrap(), b"disc-one\n");
    drop(store);
    std::fs::remove_file(archive.join(".archive-migrations/schema-1-to-2/complete")).unwrap();
    let second = archive.join("collections/synthetic--018f0000-0000-7000-8000-000000000001/items/two-disc--018f0000-0000-7000-8000-000000000010/revisions/00000000000000000002");
    std::fs::remove_file(second.join("record.v2.json")).unwrap();
    std::fs::remove_file(second.join("manifest.v2.toml")).unwrap();
    std::fs::remove_file(archive.join(".archive-migrations/schema-1-to-2/records/018f0000-0000-7000-8000-000000000010-00000000000000000002.done")).unwrap();
    let reopened = Store::open(&archive, root.path().join("state")).unwrap();
    assert_eq!(
        reopened.item(item.id).unwrap().domain.payload.get(),
        item.domain.payload.get()
    );
    assert_eq!(
        reopened.item(item.id).unwrap().specimens[0].components[0].id,
        item.specimens[0].components[0].id
    );
    drop(reopened);
    let repeated = Store::open(&archive, root.path().join("state")).unwrap();
    assert_eq!(repeated.item(item.id).unwrap().revision, Revision(2));
}

#[test]
fn private_migration_pins_the_existing_first_row_default_once() {
    let root = tempfile::tempdir().unwrap();
    let state = root.path().join("state");
    std::fs::create_dir_all(&state).unwrap();
    let private = rusqlite::Connection::open(state.join("private.sqlite")).unwrap();
    private.execute_batch("CREATE TABLE users (id TEXT PRIMARY KEY, username TEXT UNIQUE NOT NULL, password_hash TEXT NOT NULL, administrator INTEGER NOT NULL); CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL); INSERT INTO users VALUES ('018f0000-0000-7000-8000-000000000090','first','preserved-hash',1); INSERT INTO users VALUES ('018f0000-0000-7000-8000-000000000091','second','preserved-hash-2',0);").unwrap();
    drop(private);
    let store = Store::open(root.path().join("archive"), &state).unwrap();
    assert_eq!(store.default_username().unwrap(), "first");
    store
        .create_user("third", "a sufficiently long password", false)
        .unwrap();
    assert_eq!(store.default_username().unwrap(), "first");
    drop(store);
    let reopened = Store::open(root.path().join("archive"), &state).unwrap();
    assert_eq!(reopened.default_username().unwrap(), "first");
}

#[test]
fn standalone_unidentified_capture_round_trips_without_fabricated_links() {
    let (_root, store, actor, service, collection) = setup();
    let capture = Capture {
        id: CaptureId::new(),
        collection_id: collection.id,
        revision: Revision(0),
        association: CaptureAssociation::default(),
        created_at: chrono::Utc::now(),
        representations: vec![],
        legacy_payload_locator: None,
    };
    let saved = service.save_capture(&actor, capture, Revision(0)).unwrap();
    assert_eq!(saved.revision, Revision(1));
    assert_eq!(saved.association, CaptureAssociation::default());
    store.reindex().unwrap();
    let restored = service.capture(&actor, saved.id).unwrap();
    assert_eq!(restored.association, CaptureAssociation::default());
}

#[test]
fn clean_work_edition_and_edition_linked_item_round_trip() {
    let (_root, store, actor, service, collection) = setup();
    let work = service
        .save_work(
            &actor,
            Work {
                id: WorkId::new(),
                collection_id: collection.id,
                revision: Revision(0),
                provisional: false,
                title: "A work".into(),
                translations: vec![],
                identifiers: vec![],
                migration_provenance: vec![],
                deleted: false,
                updated_at: chrono::Utc::now(),
            },
            Revision(0),
        )
        .unwrap();
    let edition = service
        .save_edition(
            &actor,
            Edition {
                id: EditionId::new(),
                collection_id: collection.id,
                revision: Revision(0),
                work_id: Some(work.id),
                provisional: false,
                title: "An edition".into(),
                expected_components: vec![ExpectedComponent {
                    position: 1,
                    label: "Disc 1".into(),
                    kind: ComponentKind::OpticalDisc,
                }],
                identifiers: vec![],
                migration_provenance: vec![],
                deleted: false,
                updated_at: chrono::Utc::now(),
            },
            Revision(0),
        )
        .unwrap();
    let mut value = item(collection.id);
    value.identity_link = ItemIdentityLink::Edition {
        edition_id: edition.id,
    };
    let saved = service.save(&actor, value, Revision(0)).unwrap();
    store.reindex().unwrap();
    assert_eq!(service.work(&actor, work.id).unwrap().revision, Revision(1));
    assert_eq!(
        service.edition(&actor, edition.id).unwrap().work_id,
        Some(work.id)
    );
    assert!(
        matches!(service.item(&actor, saved.id).unwrap().identity_link, ItemIdentityLink::Edition { edition_id } if edition_id == edition.id)
    );
}

#[test]
fn schema_one_migration_rejects_conflicting_edition_works() {
    let root = tempfile::tempdir().unwrap();
    let archive = root.path().join("archive");
    copy_tree(
        Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/schema-1"
        )),
        &archive,
    );
    let record = archive.join("collections/synthetic--018f0000-0000-7000-8000-000000000001/items/two-disc--018f0000-0000-7000-8000-000000000010/revisions/00000000000000000002/record.json");
    rewrite_record(&record, |value| {
        value["item"]["work_id"] = serde_json::json!("018f0000-0000-7000-8000-000000000099")
    });
    assert!(
        matches!(Store::open(&archive, root.path().join("state")), Err(Error::Invalid(message)) if message.contains("conflicting works"))
    );
}

#[test]
fn schema_one_migration_rejects_identity_reuse_across_collections() {
    let root = tempfile::tempdir().unwrap();
    let archive = root.path().join("archive");
    copy_tree(
        Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/schema-1"
        )),
        &archive,
    );
    let source = archive.join("collections/synthetic--018f0000-0000-7000-8000-000000000001");
    let duplicate = archive.join("collections/duplicate--018f0000-0000-7000-8000-000000000002");
    copy_tree(&source, &duplicate);
    let old = "018f0000-0000-7000-8000-000000000001";
    let new = "018f0000-0000-7000-8000-000000000002";
    let collection = duplicate.join("collection.toml");
    std::fs::write(
        &collection,
        std::fs::read_to_string(&collection)
            .unwrap()
            .replace(old, new),
    )
    .unwrap();
    let revisions =
        duplicate.join("items/two-disc--018f0000-0000-7000-8000-000000000010/revisions");
    for revision in std::fs::read_dir(revisions).unwrap() {
        let record = revision.unwrap().path().join("record.json");
        rewrite_record(&record, |value| {
            value["item"]["collection_id"] = serde_json::json!(new)
        });
        let manifest = record.parent().unwrap().join("manifest.toml");
        std::fs::write(
            &manifest,
            std::fs::read_to_string(&manifest)
                .unwrap()
                .replace(old, new),
        )
        .unwrap();
    }
    assert!(
        matches!(Store::open(&archive, root.path().join("state")), Err(Error::Invalid(message)) if message.contains("across collections"))
    );
}
#[test]
fn revisions_recover_unknown_module_data_and_do_not_restore_grants() {
    let (root, store, actor, service, collection) = setup();
    let mut value = item(collection.id);
    value.domain = ModuleData {
        module: ModuleId("org.example.stamps".into()),
        schema_version: 4,
        payload: RawJson::from_json(
            r#"{"nullable":null,"nested":[1,{"color":"blue"}],"number":1e+02}"#,
        )
        .unwrap(),
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
        collection_id: edited.collection_id,
        revision: Revision(1),
        association: CaptureAssociation {
            item_id: Some(edited.id),
            ..Default::default()
        },
        created_at: chrono::Utc::now(),
        representations: vec![],
        legacy_payload_locator: None,
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
        components: vec![],
        acquisition: Some(Acquisition {
            date: None,
            cost: Some(Money {
                minor_units: 4500,
                currency: CurrencyCode::new("JPY").unwrap(),
            }),
            source: Some("Private seller".into()),
            coordinates: None,
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

#[test]
fn review_capture_update_survives_reindex() {
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
    let uploaded = store.commit_upload(&actor, upload.id).unwrap();
    let mut capture = uploaded.captures[0].clone();
    capture.association = CaptureAssociation::default();
    let revised = service
        .save_capture(&actor, capture.clone(), capture.revision)
        .unwrap();
    assert_eq!(
        service.capture(&actor, capture.id).unwrap().revision,
        revised.revision
    );
    store.reindex().unwrap();
    assert_eq!(
        service.capture(&actor, capture.id).unwrap().revision,
        revised.revision,
        "canonical capture revision regressed"
    );
}

#[test]
fn review_cannot_move_another_users_capture() {
    let (_root, store, owner, service, private_collection) = setup();
    let capture = service
        .save_capture(
            &owner,
            Capture {
                id: CaptureId::new(),
                collection_id: private_collection.id,
                revision: Revision(0),
                association: CaptureAssociation::default(),
                created_at: chrono::Utc::now(),
                representations: vec![],
                legacy_payload_locator: None,
            },
            Revision(0),
        )
        .unwrap();
    let attacker_id = store
        .create_user("other", "another long password", true)
        .unwrap();
    let attacker = Principal {
        user_id: attacker_id,
        administrator: true,
        device_id: None,
    };
    let other_collection = service.create_collection(&attacker, "Other").unwrap();
    assert!(service.capture(&attacker, capture.id).is_err());
    let mut stolen = capture.clone();
    stolen.collection_id = other_collection.id;
    assert!(
        service
            .save_capture(&attacker, stolen, capture.revision)
            .is_err(),
        "unauthorized capture move was accepted"
    );
}

#[test]
fn review_incomplete_migration_record_is_recoverable() {
    let root = tempfile::tempdir().unwrap();
    let archive = root.path().join("archive");
    copy_tree(
        Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/schema-1"
        )),
        &archive,
    );
    let partial = archive.join("collections/synthetic--018f0000-0000-7000-8000-000000000001/items/two-disc--018f0000-0000-7000-8000-000000000010/revisions/00000000000000000001/record.v2.json");
    std::fs::write(&partial, b"{\"item\":").unwrap();
    assert!(
        Store::open(&archive, root.path().join("state")).is_ok(),
        "an interrupted uncommitted migration cannot recover"
    );
}

#[test]
fn review_same_specimen_cannot_belong_to_two_items() {
    let (_root, _store, actor, service, collection) = setup();
    let mut first = item(collection.id);
    first.specimens.push(Specimen {
        id: SpecimenId::new(),
        label: "copy".into(),
        components: vec![],
        condition: Condition::Unknown,
        ownership: Ownership::Owned,
        possession: Possession::Present,
        acquisition: None,
        private_notes: None,
        loans: vec![],
    });
    let mut second = first.clone();
    second.id = ItemId::new();
    service.save(&actor, first, Revision(0)).unwrap();
    assert!(
        service.save(&actor, second, Revision(0)).is_err(),
        "duplicate physical identity was accepted"
    );
}

#[test]
fn review_capture_links_must_form_chain_when_middle_links_omitted() {
    let (_root, _store, actor, service, collection) = setup();
    let first = service
        .save(&actor, item(collection.id), Revision(0))
        .unwrap();
    let mut second = item(collection.id);
    let carrier_id = CarrierId::new();
    second.specimens.push(Specimen {
        id: SpecimenId::new(),
        label: "copy".into(),
        components: vec![Component {
            id: ComponentId::new(),
            label: "disc".into(),
            position: 1,
            kind: ComponentKind::OpticalDisc,
            carrier: Some(Carrier {
                id: carrier_id,
                label: "disc".into(),
                kind: CarrierKind::OpticalDisc,
            }),
            migration_provenance: None,
        }],
        condition: Condition::Unknown,
        ownership: Ownership::Owned,
        possession: Possession::Present,
        acquisition: None,
        private_notes: None,
        loans: vec![],
    });
    service.save(&actor, second, Revision(0)).unwrap();
    let capture = Capture {
        id: CaptureId::new(),
        collection_id: collection.id,
        revision: Revision(0),
        association: CaptureAssociation {
            item_id: Some(first.id),
            carrier_id: Some(carrier_id),
            ..Default::default()
        },
        created_at: chrono::Utc::now(),
        representations: vec![],
        legacy_payload_locator: None,
    };
    assert!(
        service.save_capture(&actor, capture, Revision(0)).is_err(),
        "links to one item and another item's carrier were accepted"
    );
}

#[test]
fn capture_reassociation_retains_payload_and_survives_restart() {
    let root = tempfile::tempdir().unwrap();
    let archive = root.path().join("archive");
    copy_tree(
        Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/schema-1"
        )),
        &archive,
    );
    let store = Arc::new(Store::open(&archive, root.path().join("state")).unwrap());
    let user = store
        .create_user("owner", "a long test password", true)
        .unwrap();
    let original = store
        .item("018f0000-0000-7000-8000-000000000010".parse().unwrap())
        .unwrap();
    store
        .grant(original.collection_id, user, Role::Owner)
        .unwrap();
    let actor = Principal {
        user_id: user,
        administrator: true,
        device_id: None,
    };
    let service = CollectionService::new(store.clone(), Arc::new(Registry::builtins()));
    let new_item = service
        .save(&actor, item(original.collection_id), Revision(0))
        .unwrap();
    let mut capture = original.captures[0].clone();
    capture.association = CaptureAssociation {
        item_id: Some(new_item.id),
        ..Default::default()
    };
    let updated = service
        .save_capture(&actor, capture.clone(), capture.revision)
        .unwrap();
    // A stale item compatibility snapshot cannot downgrade the capture.
    assert!(matches!(
        store.save(original.clone(), original.revision),
        Err(Error::Conflict)
    ));
    store.reindex().unwrap();
    let moved = service.item(&actor, new_item.id).unwrap();
    assert_eq!(moved.captures[0].revision, updated.revision);
    let representation = moved.captures[0].representations[0].id;
    let (path, _) = store
        .representation_file(&actor, new_item.id, representation, 0)
        .unwrap();
    assert_eq!(std::fs::read(path).unwrap(), b"disc-one\n");
    let mut malicious = updated.clone();
    malicious
        .legacy_payload_locator
        .as_mut()
        .unwrap()
        .relative_path = "/private".into();
    assert!(
        service
            .save_capture(&actor, malicious, updated.revision)
            .is_err()
    );
    drop(service);
    drop(store);
    let reopened = Store::open(&archive, root.path().join("state")).unwrap();
    assert_eq!(
        reopened.capture(updated.id).unwrap().revision,
        updated.revision
    );
    assert_eq!(
        reopened.item(new_item.id).unwrap().captures[0].id,
        updated.id
    );
}

#[test]
fn incomplete_typed_migration_output_recovers_but_committed_sidecars_do_not_change() {
    let root = tempfile::tempdir().unwrap();
    let archive = root.path().join("archive");
    copy_tree(
        Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/schema-1"
        )),
        &archive,
    );
    let collection = archive.join("collections/synthetic--018f0000-0000-7000-8000-000000000001");
    let partial = collection
        .join("works/018f0000-0000-7000-8000-000000000020/revisions/00000000000000000001");
    std::fs::create_dir_all(&partial).unwrap();
    std::fs::write(partial.join("record.json"), b"{").unwrap();
    let store = Store::open(&archive, root.path().join("state")).unwrap();
    drop(store);
    // A checkpoint exists, so missing/modified output must fail instead of being silently regenerated.
    std::fs::remove_file(archive.join(".archive-migrations/schema-1-to-2/complete")).unwrap();
    let sidecar = collection.join("items/two-disc--018f0000-0000-7000-8000-000000000010/revisions/00000000000000000001/record.v2.json");
    std::fs::write(sidecar, b"{}").unwrap();
    assert!(
        matches!(Store::open(&archive, root.path().join("state")), Err(Error::Storage(message)) if message.contains("committed migration output"))
    );
}

#[test]
fn nested_identity_cannot_be_reused_across_collections() {
    let (_root, _store, actor, service, collection) = setup();
    let mut first = item(collection.id);
    first.specimens.push(Specimen {
        id: SpecimenId::new(),
        label: "copy".into(),
        components: vec![],
        condition: Condition::Unknown,
        ownership: Ownership::Owned,
        possession: Possession::Present,
        acquisition: None,
        private_notes: None,
        loans: vec![],
    });
    let mut second = first.clone();
    second.id = ItemId::new();
    second.collection_id = service.create_collection(&actor, "Other").unwrap().id;
    service.save(&actor, first, Revision(0)).unwrap();
    assert!(
        matches!(service.save(&actor, second, Revision(0)), Err(Error::Invalid(message)) if message.contains("across collections"))
    );
}

#[test]
fn removed_components_are_removed_from_projection() {
    let (root, store, actor, service, collection) = setup();
    let mut value = item(collection.id);
    value.specimens.push(Specimen {
        id: SpecimenId::new(),
        label: "copy".into(),
        components: vec![Component {
            id: ComponentId::new(),
            label: "case".into(),
            kind: ComponentKind::Case,
            position: 1,
            carrier: None,
            migration_provenance: None,
        }],
        condition: Condition::Unknown,
        ownership: Ownership::Owned,
        possession: Possession::Present,
        acquisition: None,
        private_notes: None,
        loans: vec![],
    });
    let mut saved = service.save(&actor, value, Revision(0)).unwrap();
    saved.specimens[0].components.clear();
    service.save(&actor, saved.clone(), saved.revision).unwrap();
    let db = rusqlite::Connection::open(root.path().join("state/catalog.sqlite")).unwrap();
    let count: u64 = db
        .query_row("SELECT COUNT(*) FROM components", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 0);
    store.reindex().unwrap();
    assert_eq!(
        db.query_row("SELECT COUNT(*) FROM components", [], |row| row
            .get::<_, u64>(0))
            .unwrap(),
        0
    );
}
