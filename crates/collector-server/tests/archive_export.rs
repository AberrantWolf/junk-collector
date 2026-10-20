use collector_app::{
    Catalog, CollectionListChange, CollectionListPort, CollectionService, CommandContext,
    CreateCollectionList, UpdateCollectionList,
};
use collector_core::*;
use collector_modules::Registry;
use collector_store::Store;
use std::{fs, io::Write, path::Path, process::Command, sync::Arc};

fn invoke(config: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_collector-server"))
        .arg("--config")
        .arg(config)
        .args(args)
        .output()
        .unwrap()
}
fn success(output: std::process::Output) -> String {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}
fn copy_tree(source: &Path, destination: &Path) {
    fs::create_dir(destination).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let target = destination.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

#[test]
fn archive_export_cli_preserves_history_unknown_data_artwork_and_lists_with_fresh_access() {
    let root = tempfile::tempdir().unwrap();
    let archive = root.path().join("original");
    let state = root.path().join("original-state");
    let config = root.path().join("original.toml");
    copy_tree(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../collector-store/tests/fixtures/schema-1"),
        &archive,
    );
    fs::write(
        &config,
        format!("archive = {archive:?}\nstate = {state:?}\n"),
    )
    .unwrap();
    let collection_id: CollectionId = "018f0000-0000-7000-8000-000000000001".parse().unwrap();
    let item_id: ItemId = "018f0000-0000-7000-8000-000000000010".parse().unwrap();
    let original = Arc::new(Store::open(&archive, &state).unwrap());
    let user = original
        .create_user("original-owner", "an original long password", true)
        .unwrap();
    Catalog::grant(&*original, collection_id, user, Role::Owner).unwrap();
    let (_, authenticated) = original
        .login(
            "original-owner",
            "an original long password",
            "export source",
        )
        .unwrap();
    let identity = original.server_id().unwrap();
    fs::create_dir(state.join("tls")).unwrap();
    fs::write(
        state.join("tls/key.pem"),
        b"private key sentinel never exported",
    )
    .unwrap();
    fs::write(
        state.join("uploads/private-scratch"),
        b"unfinished private upload",
    )
    .unwrap();
    fs::create_dir(archive.join("unknown-empty")).unwrap();
    let unknown = b"{\"z\":1e+02,\"a\":null,\"ordered\":[3,2,1]}";
    fs::write(archive.join("future-record.json"), unknown).unwrap();
    let service = CollectionService::new(original.clone(), Arc::new(Registry::builtins()));
    let list = service
        .create_collection_list(
            &authenticated.principal,
            collection_id,
            CreateCollectionList {
                name: "Preserved order".into(),
            },
            &CommandContext::new("create-export-list").unwrap(),
        )
        .unwrap();
    let mut revision = list.revision;
    for index in 0..2 {
        revision = service
            .update_collection_list(
                &authenticated.principal,
                collection_id,
                list.id,
                UpdateCollectionList {
                    expected_revision: revision,
                    change: CollectionListChange::Append {
                        item_id,
                        source_representation_id: None,
                        track_position: None,
                    },
                },
                &CommandContext::new(format!("append-{index}")).unwrap(),
            )
            .unwrap()
            .revision;
    }
    let expected_list = serde_json::to_value(original.collection_list(list.id).unwrap()).unwrap();
    let expected_item = serde_json::to_value(original.item(item_id).unwrap()).unwrap();
    let expected_evidence = serde_json::to_value(original.evidence(item_id).unwrap()).unwrap();
    let export = root.path().join("portable");
    assert!(
        !invoke(
            &config,
            &["export-archive", "--destination", export.to_str().unwrap()]
        )
        .status
        .success()
    );
    assert!(!export.exists()); // A running source cannot be exported through another process.
    drop(service);
    drop(original);
    success(invoke(
        &config,
        &["export-archive", "--destination", export.to_str().unwrap()],
    ));
    let missing_config = root.path().join("not-a-config.toml");
    success(invoke(
        &missing_config,
        &[
            "validate-archive-export",
            "--export",
            export.to_str().unwrap(),
        ],
    ));
    let inventory = collector_store::validate_archive_export(&export).unwrap();
    assert!(
        inventory
            .files
            .iter()
            .all(|file| file.path.starts_with("archive/") && !file.path.ends_with(".writer.lock"))
    );
    assert!(!export.join("state").exists() && !export.join("collector.toml").exists());
    assert!(export.join("archive/unknown-empty").is_dir());
    assert_eq!(
        fs::read(export.join("archive/future-record.json")).unwrap(),
        unknown
    );
    // A later ordinary source write cannot become part of the frozen export.
    let original = Arc::new(Store::open(&archive, &state).unwrap());
    let service = CollectionService::new(original.clone(), Arc::new(Registry::builtins()));
    let later = service
        .create_collection_list(
            &authenticated.principal,
            collection_id,
            CreateCollectionList {
                name: "Created after export".into(),
            },
            &CommandContext::new("after-export").unwrap(),
        )
        .unwrap();
    let imported_archive = root.path().join("fresh-archive");
    let imported_state = root.path().join("fresh-state");
    let imported_config = root.path().join("fresh.toml");
    // Import needs neither the source installation nor its config/private state;
    // the original may keep running once its offline export has completed.
    success(invoke(
        &missing_config,
        &[
            "import-archive",
            "--export",
            export.to_str().unwrap(),
            "--archive",
            imported_archive.to_str().unwrap(),
        ],
    ));
    assert!(!imported_state.exists() && !missing_config.exists());
    for file in &inventory.files {
        let local = file.path.strip_prefix("archive/").unwrap();
        assert_eq!(
            fs::read(imported_archive.join(local)).unwrap(),
            fs::read(export.join(&file.path)).unwrap(),
            "{local}"
        );
    }
    fs::write(
        &imported_config,
        format!("archive = {imported_archive:?}\nstate = {imported_state:?}\n"),
    )
    .unwrap();
    let mut setup = Command::new(env!("CARGO_BIN_EXE_collector-server"))
        .arg("--config")
        .arg(&imported_config)
        .args(["setup", "--username", "new-owner"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    writeln!(
        setup.stdin.take().unwrap(),
        "a fresh sufficiently long password"
    )
    .unwrap();
    success(setup.wait_with_output().unwrap());
    let imported = Store::open(&imported_archive, &imported_state).unwrap();
    imported.reindex().unwrap();
    assert_ne!(imported.server_id().unwrap(), identity);
    assert!(
        imported
            .login(
                "original-owner",
                "an original long password",
                "old credentials"
            )
            .is_err()
    );
    let (_, new_owner) = imported
        .login(
            "new-owner",
            "a fresh sufficiently long password",
            "new account",
        )
        .unwrap();
    assert!(
        imported
            .collections(new_owner.principal.user_id)
            .unwrap()
            .is_empty()
    );
    assert!(imported.collection_list(later.id).is_err());
    assert_eq!(
        serde_json::to_value(imported.item(item_id).unwrap()).unwrap(),
        expected_item
    );
    assert_eq!(
        serde_json::to_value(imported.evidence(item_id).unwrap()).unwrap(),
        expected_evidence
    );
    assert_eq!(
        serde_json::to_value(imported.collection_list(list.id).unwrap()).unwrap(),
        expected_list
    );
    assert!(!imported_state.join("uploads/private-scratch").exists());
    assert_ne!(
        fs::read(imported_state.join("tls/key.pem")).unwrap(),
        b"private key sentinel never exported"
    );
    drop(imported);
    let discovered = success(invoke(&imported_config, &["unowned-collections"]));
    assert!(discovered.contains(&collection_id.to_string()));
    success(invoke(
        &imported_config,
        &[
            "recover-collection-owner",
            "--collection-id",
            &collection_id.to_string(),
            "--username",
            "new-owner",
        ],
    ));
    let imported = Store::open(&imported_archive, &imported_state).unwrap();
    assert_eq!(
        Catalog::role(&imported, new_owner.principal.user_id, collection_id).unwrap(),
        Role::Owner
    );
    assert_eq!(
        imported.collections(new_owner.principal.user_id).unwrap()[0].id,
        collection_id
    );
    collector_store::validate_archive_export(&export).unwrap();
}
