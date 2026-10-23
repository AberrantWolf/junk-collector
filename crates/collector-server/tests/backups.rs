use std::{fs, path::Path, process::Command};

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

#[test]
fn stopped_backup_cli_restores_identity_tls_and_reports_external_paths() {
    let root = tempfile::tempdir().unwrap();
    let archive = root.path().join("archive");
    let state = root.path().join("state");
    let config = root.path().join("collector.toml");
    fs::write(&config, format!("archive = {archive:?}\nstate = {state:?}\n[inboxes]\nincoming = '/old/incoming'\n[encoders]\nflac = '/old/bin/flac'\n")).unwrap();
    let store = collector_store::Store::open(&archive, &state).unwrap();
    store
        .create_user("owner", "a sufficiently long password", true)
        .unwrap();
    let id = store.server_id().unwrap();
    fs::create_dir(state.join("tls")).unwrap();
    fs::write(state.join("tls/key.pem"), b"private TLS test key").unwrap();
    fs::create_dir(archive.join("unknown-empty")).unwrap();
    fs::write(archive.join("unknown.bin"), b"unrecognized archive bytes").unwrap();
    let backup = root.path().join("backup");
    assert!(
        !invoke(
            &config,
            &["backup", "--destination", backup.to_str().unwrap()]
        )
        .status
        .success()
    );
    assert!(!backup.exists());
    drop(store);
    success(invoke(
        &config,
        &["backup", "--destination", backup.to_str().unwrap()],
    ));
    // Validate/restore must not read the current configuration or open source databases.
    let missing = root.path().join("missing.toml");
    success(invoke(
        &missing,
        &["validate-backup", "--backup", backup.to_str().unwrap()],
    ));
    let a = root.path().join("restored-archive");
    let s = root.path().join("restored-state");
    let c = root.path().join("restored.toml");
    let args = [
        "restore",
        "--backup",
        backup.to_str().unwrap(),
        "--archive",
        a.to_str().unwrap(),
        "--state",
        s.to_str().unwrap(),
    ];
    let running = collector_store::Store::open(&archive, &state).unwrap();
    assert!(!invoke(&c, &args).status.success());
    assert!(!a.exists());
    assert!(!s.exists());
    drop(running);
    let message = success(invoke(&c, &args));
    assert!(message.contains("/old/incoming") && message.contains("/old/bin/flac"));
    assert!(message.contains("never run both copies"));
    assert_eq!(
        fs::read(s.join("tls/key.pem")).unwrap(),
        b"private TLS test key"
    );
    assert_eq!(
        fs::read(a.join("unknown.bin")).unwrap(),
        b"unrecognized archive bytes"
    );
    assert!(a.join("unknown-empty").is_dir());
    let relocated: toml::Value = toml::from_str(&fs::read_to_string(&c).unwrap()).unwrap();
    assert_eq!(relocated["archive"].as_str(), a.to_str());
    assert_eq!(relocated["state"].as_str(), s.to_str());
    let restored = collector_store::Store::open(&a, &s).unwrap();
    assert_eq!(restored.server_id().unwrap(), id);
    restored
        .login("owner", "a sufficiently long password", "backup drill")
        .unwrap();
    assert!(!invoke(&c, &args).status.success());
    drop(restored);
    let clone_archive = root.path().join("independent-archive");
    let clone_state = root.path().join("independent-state");
    let clone_config = root.path().join("independent.toml");
    let report = success(invoke(
        &clone_config,
        &[
            "restore",
            "--backup",
            backup.to_str().unwrap(),
            "--archive",
            clone_archive.to_str().unwrap(),
            "--state",
            clone_state.to_str().unwrap(),
            "--archive-only",
        ],
    ));
    assert!(report.contains("Fresh private identity required"));
    assert!(!clone_state.join("private.sqlite").exists());
    assert!(!clone_state.join("tls").exists());
    let mut setup = Command::new(env!("CARGO_BIN_EXE_collector-server"))
        .arg("--config")
        .arg(&clone_config)
        .args(["setup", "--username", "new-owner"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    {
        use std::io::Write;
        setup
            .stdin
            .take()
            .unwrap()
            .write_all(b"a fresh sufficiently long password\n")
            .unwrap();
    }
    success(setup.wait_with_output().unwrap());
    let independent = collector_store::Store::open(&clone_archive, &clone_state).unwrap();
    assert_ne!(independent.server_id().unwrap(), id);
    independent
        .login(
            "new-owner",
            "a fresh sufficiently long password",
            "independent drill",
        )
        .unwrap();
    assert!(
        independent
            .login("owner", "a sufficiently long password", "old account")
            .is_err()
    );
    assert_ne!(
        fs::read(clone_state.join("tls/key.pem")).unwrap(),
        b"private TLS test key"
    );
    drop(independent);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for path in [&backup, &s] {
            assert_eq!(
                fs::metadata(path).unwrap().permissions().mode() & 0o777,
                0o700
            );
        }
        for path in [
            c,
            s.join("tls/key.pem"),
            backup.join("collector.toml"),
            backup.join("state/private.sqlite"),
        ] {
            assert_eq!(
                fs::metadata(path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
    }
    // Corruption is rejected before either destination is created.
    fs::write(backup.join("archive/unknown.bin"), b"corrupt").unwrap();
    let bad_a = root.path().join("bad-archive");
    let bad_s = root.path().join("bad-state");
    assert!(
        !invoke(
            &missing,
            &[
                "restore",
                "--backup",
                backup.to_str().unwrap(),
                "--archive",
                bad_a.to_str().unwrap(),
                "--state",
                bad_s.to_str().unwrap()
            ]
        )
        .status
        .success()
    );
    assert!(!bad_a.exists() && !bad_s.exists() && !missing.exists());
    let absent_backup = root.path().join("absent-installation-backup");
    assert!(
        !invoke(
            &missing,
            &["backup", "--destination", absent_backup.to_str().unwrap()]
        )
        .status
        .success()
    );
    assert!(!absent_backup.exists());
}

fn setup_account(config: &Path, username: &str, password: &str) {
    use std::io::Write;
    let mut child = Command::new(env!("CARGO_BIN_EXE_collector-server"))
        .arg("--config")
        .arg(config)
        .args(["setup", "--username", username])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    writeln!(child.stdin.take().unwrap(), "{password}").unwrap();
    success(child.wait_with_output().unwrap());
}

#[test]
fn archive_only_cli_recovers_first_owner_and_preserves_normal_access_policy() {
    use collector_app::{
        AccessPort, Catalog, CollectionService, CommandContext, CreateCollectionCommand,
    };
    use collector_core::*;
    use collector_modules::Registry;
    use std::sync::Arc;
    let root = tempfile::tempdir().unwrap();
    let archive = root.path().join("archive");
    let state = root.path().join("state");
    let config = root.path().join("collector.toml");
    fs::write(
        &config,
        format!("archive = {archive:?}\nstate = {state:?}\n"),
    )
    .unwrap();
    setup_account(&config, "original-owner", "an original long password");
    let original = Arc::new(collector_store::Store::open(&archive, &state).unwrap());
    let authenticated = original
        .login("original-owner", "an original long password", "source")
        .unwrap()
        .1;
    let service = CollectionService::new(original.clone(), Arc::new(Registry::builtins()));
    let mut collections = (0..3)
        .map(|index| {
            service
                .create_collection(
                    &authenticated,
                    CreateCollectionCommand {
                        name: format!("Recovered collection {index}"),
                        module: ModuleId("org.junk.music".into()),
                    },
                    &CommandContext::new(format!("create-{index}")).unwrap(),
                )
                .unwrap()
        })
        .collect::<Vec<_>>();
    collections.sort_by_key(|collection| collection.id);
    drop(service);
    drop(original);
    let backup = root.path().join("backup");
    success(invoke(
        &config,
        &["backup", "--destination", backup.to_str().unwrap()],
    ));
    let da = root.path().join("clone-archive");
    let ds = root.path().join("clone-state");
    let dc = root.path().join("clone.toml");
    success(invoke(
        &dc,
        &[
            "restore",
            "--backup",
            backup.to_str().unwrap(),
            "--archive",
            da.to_str().unwrap(),
            "--state",
            ds.to_str().unwrap(),
            "--archive-only",
        ],
    ));
    setup_account(&dc, "recovered-owner", "a recovered long password");
    let recovered = Arc::new(collector_store::Store::open(&da, &ds).unwrap());
    let owner = recovered
        .login(
            "recovered-owner",
            "a recovered long password",
            "fresh administrator",
        )
        .unwrap()
        .1;
    let peer = recovered
        .create_user("peer", "another sufficiently long password", false)
        .unwrap();
    let disabled = recovered
        .create_user("disabled", "another sufficiently long password", false)
        .unwrap();
    recovered
        .update_account(disabled, Revision(1), "Disabled", false, false)
        .unwrap();
    let service = CollectionService::new(recovered.clone(), Arc::new(Registry::builtins()));
    assert!(service.collections(&owner.principal).unwrap().is_empty());
    assert!(matches!(
        service.grant(
            &owner,
            collections[0].id,
            owner.principal.user_id,
            Role::Owner,
            &CommandContext::new("remote-bootstrap-denied").unwrap()
        ),
        Err(Error::Forbidden)
    ));
    let id = collections[0].id.to_string();
    let recover = [
        "recover-collection-owner",
        "--collection-id",
        &id,
        "--username",
        "recovered-owner",
    ];
    // Both commands must refuse an installation currently opened by a server.
    assert!(!invoke(&dc, &["unowned-collections"]).status.success());
    assert!(!invoke(&dc, &recover).status.success());
    assert!(
        AccessPort::grant_for_user(&*recovered, collections[0].id, owner.principal.user_id)
            .unwrap()
            .is_none()
    );
    drop(service);
    drop(recovered);
    let first = success(invoke(&dc, &["unowned-collections", "--limit", "1"]));
    assert!(first.contains(&id));
    assert!(!first.contains(&collections[1].id.to_string()));
    let second = success(invoke(
        &dc,
        &["unowned-collections", "--limit", "1", "--cursor", &id],
    ));
    assert!(second.contains(&collections[1].id.to_string()));
    assert!(!second.contains(&id));
    assert!(
        !invoke(&dc, &["unowned-collections", "--limit", "0"])
            .status
            .success()
    );
    for username in ["missing-account", "disabled"] {
        assert!(
            !invoke(
                &dc,
                &[
                    "recover-collection-owner",
                    "--collection-id",
                    &id,
                    "--username",
                    username
                ]
            )
            .status
            .success()
        );
    }
    let missing = CollectionId::new().to_string();
    assert!(
        !invoke(
            &dc,
            &[
                "recover-collection-owner",
                "--collection-id",
                &missing,
                "--username",
                "recovered-owner"
            ]
        )
        .status
        .success()
    );
    success(invoke(&dc, &recover));
    let recovered = Arc::new(collector_store::Store::open(&da, &ds).unwrap());
    let first_grant =
        AccessPort::grant_for_user(&*recovered, collections[0].id, owner.principal.user_id)
            .unwrap()
            .unwrap();
    let owner = recovered
        .login(
            "recovered-owner",
            "a recovered long password",
            "owner after local recovery",
        )
        .unwrap()
        .1;
    let service = CollectionService::new(recovered.clone(), Arc::new(Registry::builtins()));
    assert_eq!(
        service.collections(&owner.principal).unwrap()[0].id,
        collections[0].id
    );
    assert_eq!(
        service
            .authorize(&owner.principal, collections[0].id, Action::ManageAccess)
            .unwrap(),
        Role::Owner
    );
    // Subsequent access goes through the ordinary application workflow.
    service
        .grant(
            &owner,
            collections[0].id,
            peer,
            Role::Viewer,
            &CommandContext::new("owner-shares-recovery").unwrap(),
        )
        .unwrap();
    let peer_auth = recovered
        .login(
            "peer",
            "another sufficiently long password",
            "shared access",
        )
        .unwrap()
        .1;
    assert_eq!(
        service.collections(&peer_auth.principal).unwrap()[0].id,
        collections[0].id
    );
    assert!(matches!(
        service.grant(
            &peer_auth,
            collections[0].id,
            peer,
            Role::Owner,
            &CommandContext::new("viewer-cannot-promote").unwrap()
        ),
        Err(Error::Forbidden)
    ));
    drop(service);
    drop(recovered);
    success(invoke(&dc, &recover)); // Repeating the explicit command is read-only.
    let denied = invoke(
        &dc,
        &[
            "recover-collection-owner",
            "--collection-id",
            &id,
            "--username",
            "peer",
        ],
    );
    assert!(!denied.status.success());
    assert!(String::from_utf8_lossy(&denied.stderr).contains("already has an Owner"));
    let remaining = success(invoke(&dc, &["unowned-collections"]));
    assert!(!remaining.contains(&id));
    assert!(remaining.contains(&collections[1].id.to_string()));
    assert!(remaining.contains(&collections[2].id.to_string()));
    let recovered = collector_store::Store::open(&da, &ds).unwrap();
    let repeated =
        AccessPort::grant_for_user(&recovered, collections[0].id, owner.principal.user_id)
            .unwrap()
            .unwrap();
    assert_eq!(repeated.revision, first_grant.revision);
    assert_eq!(
        Catalog::role(&recovered, peer, collections[0].id).unwrap(),
        Role::Viewer
    );
    for collection in &collections[1..] {
        assert!(
            AccessPort::grant_for_user(&recovered, collection.id, owner.principal.user_id)
                .unwrap()
                .is_none()
        );
    }
}

#[test]
fn restore_cli_recovers_with_missing_original_lockfiles_and_still_refuses_live_sources() {
    for scenario in [
        "empty-roots",
        "missing-private-lock",
        "missing-archive-lock",
    ] {
        let root = tempfile::tempdir().unwrap();
        let archive = root.path().join("archive");
        let state = root.path().join("state");
        let config = root.path().join("collector.toml");
        fs::write(
            &config,
            format!("archive = {archive:?}\nstate = {state:?}\n"),
        )
        .unwrap();
        let store = collector_store::Store::open(&archive, &state).unwrap();
        let identity = store.server_id().unwrap();
        drop(store);
        let backup = root.path().join("backup");
        success(invoke(
            &config,
            &["backup", "--destination", backup.to_str().unwrap()],
        ));
        let seal = fs::read(backup.join("complete.sha256")).unwrap();
        let da = root.path().join("new-archive");
        let ds = root.path().join("new-state");
        let dc = root.path().join("new.toml");
        let args = [
            "restore",
            "--backup",
            backup.to_str().unwrap(),
            "--archive",
            da.to_str().unwrap(),
            "--state",
            ds.to_str().unwrap(),
        ];
        if scenario == "empty-roots" {
            fs::rename(&archive, root.path().join("saved-archive")).unwrap();
            fs::rename(&state, root.path().join("saved-state")).unwrap();
            fs::create_dir(&archive).unwrap();
            fs::create_dir(&state).unwrap();
        } else {
            let (missing, other) = if scenario == "missing-private-lock" {
                (state.join(".private.lock"), archive.join(".writer.lock"))
            } else {
                (archive.join(".writer.lock"), state.join(".private.lock"))
            };
            fs::remove_file(&missing).unwrap();
            #[cfg(unix)]
            {
                let outside = root.path().join("outside-lock-target");
                fs::write(&outside, b"never follow a source lock alias").unwrap();
                std::os::unix::fs::symlink(&outside, &missing).unwrap();
                assert!(!invoke(&dc, &args).status.success());
                assert!(!da.exists() && !ds.exists() && !dc.exists());
                assert_eq!(
                    fs::read(&outside).unwrap(),
                    b"never follow a source lock alias"
                );
                fs::remove_file(&missing).unwrap();
            }
            // Even with the other lockfile missing, a live source must remain fenced.
            let held = fs::File::open(&other).unwrap();
            held.try_lock().unwrap();
            assert!(!invoke(&dc, &args).status.success());
            assert!(!da.exists() && !ds.exists() && !dc.exists());
            held.unlock().unwrap();
            if missing.exists() {
                fs::remove_file(missing).unwrap();
            }
        }
        let private_before = fs::read(state.join("private.sqlite")).ok();
        success(invoke(&dc, &args));
        assert!(archive.join(".writer.lock").is_file());
        assert!(state.join(".private.lock").is_file());
        assert_eq!(fs::read(state.join("private.sqlite")).ok(), private_before);
        assert_eq!(fs::read(backup.join("complete.sha256")).unwrap(), seal);
        success(invoke(
            &dc,
            &["validate-backup", "--backup", backup.to_str().unwrap()],
        ));
        let recovered = collector_store::Store::open(&da, &ds).unwrap();
        assert_eq!(recovered.server_id().unwrap(), identity);
    }
}
