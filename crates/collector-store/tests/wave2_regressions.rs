use collector_app::AccessPort;
use collector_core::*;
use collector_store::Store;
fn enrolled(store: &Store, user: UserId) -> collector_app::NativeCredentialIssue {
    let start = store
        .start_enrollment(Some(user), "review device", "https://example.test")
        .unwrap();
    store.approve_enrollment(user, &start.user_code).unwrap();
    store
        .complete_enrollment(start.id, &start.completion_secret)
        .unwrap()
}
#[test]
fn review_disable_reenable_must_not_resurrect_native() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path().join("archive"), dir.path().join("state")).unwrap();
    store
        .create_user("admin", "long enough password", true)
        .unwrap();
    let user = store
        .create_user("member", "long enough password", false)
        .unwrap();
    let issue = enrolled(&store, user);
    let pending = store
        .start_enrollment(Some(user), "pending device", "https://example.test")
        .unwrap();
    store.approve_enrollment(user, &pending.user_code).unwrap();
    store
        .update_account(user, Revision(1), "Member", false, false)
        .unwrap();
    assert!(store.authenticate_native(&issue.credential).is_err());
    store
        .update_account(user, Revision(2), "Member", true, false)
        .unwrap();
    assert!(
        store.authenticate_native(&issue.credential).is_err(),
        "revoked device resurrected after reenable"
    );
    assert!(matches!(
        store.complete_enrollment(pending.id, &pending.completion_secret),
        Err(Error::EnrollmentCancelled)
    ));
}
#[test]
fn review_generating_codes_preserves_native_but_consumption_revokes_it() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path().join("archive"), dir.path().join("state")).unwrap();
    let user = store
        .create_user("admin", "long enough password", true)
        .unwrap();
    let issue = enrolled(&store, user);
    let pending = store
        .start_enrollment(Some(user), "pending device", "https://example.test")
        .unwrap();
    store.approve_enrollment(user, &pending.user_code).unwrap();
    let codes = store.replace_recovery_codes(user).unwrap();
    assert!(
        store
            .complete_enrollment(pending.id, &pending.completion_secret)
            .is_ok()
    );
    // Generating a replacement batch still retains apps. Consuming a code now
    // follows the explicitly accepted 2026-10-07 recovery revocation policy.
    assert!(store.authenticate_native(&issue.credential).is_ok());
    store
        .consume_recovery_code(
            "admin",
            &codes[0],
            "replacement password",
            "recovery browser",
        )
        .unwrap();
    assert!(
        store.authenticate_native(&issue.credential).is_err(),
        "consuming a recovery code must revoke app access"
    );
}
#[test]
fn review_removed_grant_revision_must_not_repeat() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path().join("archive"), dir.path().join("state")).unwrap();
    let user = store
        .create_user("admin", "long enough password", true)
        .unwrap();
    let collection = CollectionId::new();
    store
        .set_grant(collection, user, Revision(0), Some(Role::Viewer))
        .unwrap();
    store
        .set_grant(collection, user, Revision(1), None)
        .unwrap();
    store
        .set_grant(collection, user, Revision(0), Some(Role::Curator))
        .unwrap();
    assert!(
        store
            .set_grant(collection, user, Revision(1), None)
            .is_err(),
        "stale revocation removed newly issued grant"
    );
}
#[test]
fn review_partial_prepared_stage_recovers() {
    let dir = tempfile::tempdir().unwrap();
    let archive = dir.path().join("archive");
    let state = dir.path().join("state");
    drop(Store::open(&archive, &state).unwrap());
    let txn = TransactionId::new();
    let staged = format!(".staging/{txn}/record.v3.json");
    let path = archive.join(&staged);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, b"partial").unwrap();
    let journal = serde_json::json!({"schema_version":1,"transaction_id":txn,"entity_type":"item","entity_id":ItemId::new().to_string(),"expected_revision":0,"target_revision":1,"phase":"prepared","artifacts":[{"kind":"revision","staged_path":staged,"published_path":"collections/example/items/example/revisions/00000000000000000001/record.v3.json","sha256":Digest::sha256(b"complete record").hex}]});
    std::fs::write(
        archive.join(format!(".transactions/{txn}.json")),
        serde_json::to_vec(&journal).unwrap(),
    )
    .unwrap();
    let result = Store::open(&archive, &state);
    assert!(
        result.is_ok(),
        "ordinary interrupted staging blocked startup: {:?}",
        result.err()
    );
}

#[test]
fn incomplete_journal_replacements_do_not_block_startup() {
    for existing in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let archive = root.path().join("archive");
        let state = root.path().join("state");
        drop(Store::open(&archive, &state).unwrap());
        let txn = TransactionId::new();
        if existing {
            let journal = serde_json::json!({"schema_version":1,"transaction_id":txn,"entity_type":"item","entity_id":ItemId::new().to_string(),"expected_revision":0,"target_revision":1,"phase":"prepared","artifacts":[{"kind":"revision","staged_path":format!(".staging/{txn}/record.v3.json"),"published_path":"collections/example/items/example/revisions/00000000000000000001/record.v3.json","sha256":Digest::sha256(b"record").hex}]});
            std::fs::write(
                archive.join(format!(".transactions/{txn}.json")),
                serde_json::to_vec(&journal).unwrap(),
            )
            .unwrap();
        }
        let temporary = archive.join(format!(".transactions/{txn}.json.tmp"));
        std::fs::write(&temporary, br#"{"interrupted":"#).unwrap();
        let reopened = Store::open(&archive, &state).unwrap();
        assert!(!temporary.exists());
        assert!(reopened.recovery_report().committed.is_empty());
    }
}
