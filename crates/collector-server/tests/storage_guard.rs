//! Real executable admission, with isolated directories only.
use collector_store::Store;
use std::{fs, path::Path, process::Command};

fn invoke(config: &Path, arguments: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_collector-server"))
        .arg("--config")
        .arg(config)
        .args(arguments)
        .output()
        .unwrap()
}

#[test]
fn executable_never_initializes_missing_storage_during_normal_startup() {
    let root = tempfile::tempdir().unwrap();
    let archive = root.path().join("archive");
    let state = root.path().join("state");
    let config = root.path().join("collector.toml");
    fs::write(&config, format!("archive={archive:?}\nstate={state:?}\n")).unwrap();
    for command in ["serve", "reindex", "enroll-archive"] {
        let output = invoke(&config, &[command]);
        assert!(!output.status.success(), "{command}");
        assert!(
            !archive.exists() && !state.exists(),
            "{command} created replacement storage"
        );
    }
}

#[test]
fn explicit_legacy_enrollment_preserves_accounts_and_missing_archive_refuses_all_openers() {
    let root = tempfile::tempdir().unwrap();
    let archive = root.path().join("archive");
    let state = root.path().join("state");
    let config = root.path().join("collector.toml");
    fs::write(&config, format!("archive={archive:?}\nstate={state:?}\n")).unwrap();
    let store = Store::initialize(&archive, &state, None).unwrap();
    store
        .create_user("owner", "retained account password", true)
        .unwrap();
    let server = store.server_id().unwrap();
    drop(store);
    // Model an actual pre-guard installation, not a corrupt/new private database.
    fs::remove_file(archive.join(".archive-identity")).unwrap();
    fs::remove_file(state.join("archive-binding")).unwrap();
    let before = fs::read(state.join("private.sqlite")).unwrap();
    assert!(!invoke(&config, &["reindex"]).status.success());
    let enrolled = invoke(&config, &["enroll-archive"]);
    assert!(
        enrolled.status.success(),
        "{}",
        String::from_utf8_lossy(&enrolled.stderr)
    );
    assert_eq!(fs::read(state.join("private.sqlite")).unwrap(), before);
    let store = Store::open_existing(&archive, &state, None).unwrap();
    assert_eq!(store.server_id().unwrap(), server);
    store
        .login("owner", "retained account password", "enrollment test")
        .unwrap();
    drop(store);
    fs::rename(&archive, root.path().join("detached")).unwrap();
    fs::create_dir(&archive).unwrap();
    for arguments in [
        vec!["serve"],
        vec!["reindex"],
        vec!["setup"],
        vec![
            "backup",
            "--destination",
            root.path().join("backup").to_str().unwrap(),
        ],
    ] {
        assert!(!invoke(&config, &arguments).status.success());
        assert!(fs::read_dir(&archive).unwrap().next().is_none());
    }
}

#[test]
fn remote_cli_policy_refuses_local_import_destination_before_copying() {
    let root = tempfile::tempdir().unwrap();
    let archive = root.path().join("must-not-create");
    let output = invoke(
        &root.path().join("missing.toml"),
        &[
            "--nfs-source",
            "fixture-nas:/archive",
            "import-archive",
            "--export",
            root.path().join("missing-export").to_str().unwrap(),
            "--archive",
            archive.to_str().unwrap(),
        ],
    );
    assert!(!output.status.success());
    assert!(!archive.exists());
    assert!(String::from_utf8_lossy(&output.stderr).contains("archive storage unavailable"));
}
