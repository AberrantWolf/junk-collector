//! Isolated directory substitution, never a real mount or production pathname.
use collector_app::{Catalog, JobPort, JobSpec};
use collector_core::*;
use collector_store::{ArchiveMount, Store};
use std::fs;

#[test]
fn normal_startup_never_initializes_storage_and_setup_cannot_replace_an_installation() {
    let root = tempfile::tempdir().unwrap();
    let archive = root.path().join("archive");
    let state = root.path().join("state");
    assert!(Store::open_existing(&archive, &state, None).is_err());
    assert!(!archive.exists() && !state.exists());
    let store = Store::initialize(&archive, &state, None).unwrap();
    let server = store.server_id().unwrap();
    drop(store);
    fs::rename(&archive, root.path().join("detached")).unwrap();
    for create_empty_mountpoint in [false, true] {
        if create_empty_mountpoint {
            fs::create_dir(&archive).unwrap();
        }
        assert!(Store::open_existing(&archive, &state, None).is_err());
        assert!(Store::initialize(&archive, &state, None).is_err());
        if create_empty_mountpoint {
            assert!(fs::read_dir(&archive).unwrap().next().is_none());
            fs::remove_dir(&archive).unwrap();
        } else {
            assert!(!archive.exists());
        }
    }
    fs::rename(root.path().join("detached"), &archive).unwrap();
    let recovered = Store::open_existing(&archive, &state, None).unwrap();
    assert_eq!(recovered.server_id().unwrap(), server);
}

#[test]
fn wrong_archive_and_missing_private_database_are_refused_without_repair() {
    let root = tempfile::tempdir().unwrap();
    let a = root.path().join("a");
    let b = root.path().join("b");
    let s = root.path().join("s");
    drop(Store::initialize(&a, &s, None).unwrap());
    drop(Store::initialize(&b, root.path().join("other-state"), None).unwrap());
    let private = fs::read(s.join("private.sqlite")).unwrap();
    assert!(Store::open_existing(&b, &s, None).is_err());
    assert_eq!(fs::read(s.join("private.sqlite")).unwrap(), private);
    fs::rename(s.join("private.sqlite"), s.join("retained-private.sqlite")).unwrap();
    assert!(Store::open_existing(&a, &s, None).is_err());
    assert!(!s.join("private.sqlite").exists());
}

#[test]
fn loss_during_work_blocks_browsing_completion_and_cleanup_until_restart() {
    let root = tempfile::tempdir().unwrap();
    let a = root.path().join("archive");
    let s = root.path().join("state");
    let store = Store::initialize(&a, &s, None).unwrap();
    let job = JobId::new();
    let user = UserId::new();
    let input = serde_json::json!({"recoverable":"frozen request"});
    store
        .enqueue_job(JobSpec {
            id: job,
            user_id: user,
            collection_id: None,
            item_id: None,
            kind: "test.processing".into(),
            phase: "queued".into(),
            total_units: 1,
            request: input.clone(),
            frozen_input: None,
        })
        .unwrap();
    let claim = store.claim_job("fixture", 60).unwrap().unwrap();
    fs::create_dir_all(s.join("imports/preserved-source")).unwrap();
    fs::write(
        s.join("imports/preserved-source/original"),
        b"original retained input",
    )
    .unwrap();
    let marker = a.join(".archive-identity");
    fs::rename(&marker, a.join("retained-marker")).unwrap();
    assert!(matches!(store.reindex(), Err(Error::Unavailable(_))));
    assert!(matches!(
        store.complete_job(job, &claim.claim_token, serde_json::json!({})),
        Err(Error::Unavailable(_))
    ));
    // Putting a mount back does not implicitly resume this process's jobs/cleanup.
    fs::rename(a.join("retained-marker"), marker).unwrap();
    assert!(matches!(
        store.collections(UserId::new()),
        Err(Error::Unavailable(_))
    ));
    drop(store);
    let recovered = Store::open_existing(&a, &s, None).unwrap();
    let retained = recovered.job_record(job, user).unwrap();
    assert_ne!(retained.state, JobState::Completed);
    assert_eq!(
        fs::read(s.join("imports/preserved-source/original")).unwrap(),
        b"original retained input"
    );
}

#[test]
fn absent_collection_tree_cannot_replace_catalog_with_empty_projection() {
    let root = tempfile::tempdir().unwrap();
    let a = root.path().join("archive");
    let s = root.path().join("state");
    drop(Store::initialize(&a, &s, None).unwrap());
    let before = fs::read(s.join("catalog.sqlite")).unwrap();
    fs::rename(a.join("collections"), a.join("retained-collections")).unwrap();
    assert!(Store::open_existing(&a, &s, None).is_err());
    assert!(!a.join("collections").exists());
    assert_eq!(fs::read(s.join("catalog.sqlite")).unwrap(), before);
}

#[test]
fn remote_mode_refuses_local_directory_before_any_initialization() {
    let root = tempfile::tempdir().unwrap();
    let a = root.path().join("archive");
    let s = root.path().join("state");
    fs::create_dir(&a).unwrap();
    let expected = Some(ArchiveMount {
        nfs_source: "fixture-nas:/archive".into(),
    });
    assert!(Store::initialize(&a, &s, expected.clone()).is_err());
    assert!(Store::enroll_archive(&a, &s, expected).is_err());
    assert!(fs::read_dir(&a).unwrap().next().is_none());
    assert!(!s.exists());
}

#[cfg(unix)]
#[test]
fn private_sqlite_and_import_roots_cannot_be_redirected_through_symlinks() {
    use std::os::unix::fs::symlink;
    for name in ["private.sqlite", "imports"] {
        let root = tempfile::tempdir().unwrap();
        let archive = root.path().join("archive");
        let state = root.path().join("state");
        drop(Store::initialize(&archive, &state, None).unwrap());
        let retained = state.join(format!("retained-{name}"));
        std::fs::rename(state.join(name), &retained).unwrap();
        symlink(&retained, state.join(name)).unwrap();
        assert!(Store::open_existing(&archive, &state, None).is_err());
    }
}
