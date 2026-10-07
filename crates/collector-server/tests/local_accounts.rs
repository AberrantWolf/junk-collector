use std::{
    io::Write,
    process::{Command, Stdio},
};

#[test]
fn stopped_cli_uses_existing_state_independent_of_configuration_and_archive() {
    let root = tempfile::tempdir().unwrap();
    let archive = root.path().join("archive");
    let state = root.path().join("state");
    let config = root.path().join("damaged.toml");
    let binary = env!("CARGO_BIN_EXE_collector-server");
    std::fs::write(&config, "invalid = [").unwrap();
    let invoke = |arguments: &[&str]| {
        Command::new(binary)
            .arg("--config")
            .arg(&config)
            .args(arguments)
            .output()
            .unwrap()
    };
    let missing = root.path().join("missing");
    assert!(
        !invoke(&["accounts", "--state", missing.to_str().unwrap()])
            .status
            .success()
    );
    assert!(!missing.exists());
    assert!(!invoke(&["init"]).status.success());
    assert!(
        !invoke(&["accounts", "--private-state", state.to_str().unwrap()])
            .status
            .success()
    );
    assert!(
        !invoke(&["recover-account", "--state", state.to_str().unwrap()])
            .status
            .success()
    );
    assert!(
        !invoke(&[
            "recover-account",
            "--state",
            state.to_str().unwrap(),
            "--username",
            "owner",
            "--user-id",
            "00000000-0000-0000-0000-000000000000"
        ])
        .status
        .success()
    );
    let store = collector_store::Store::open(&archive, &state).unwrap();
    let _user = store
        .create_user("owner", "a sufficiently long password", true)
        .unwrap();
    assert!(
        !invoke(&["accounts", "--state", state.to_str().unwrap()])
            .status
            .success()
    );
    std::fs::write(
        &config,
        format!("state = {:?}\narchive = {:?}\n", state, archive),
    )
    .unwrap();
    assert!(invoke(&["paths"]).status.success());
    assert!(!invoke(&["accounts"]).status.success());
    drop(store);
    std::fs::remove_dir_all(&archive).unwrap();
    std::fs::write(&config, "invalid = [").unwrap();
    let accounts = invoke(&[
        "accounts",
        "--state",
        state.to_str().unwrap(),
        "--limit",
        "1",
    ]);
    assert!(
        accounts.status.success(),
        "{}",
        String::from_utf8_lossy(&accounts.stderr)
    );
    let output = String::from_utf8(accounts.stdout).unwrap();
    assert!(output.contains("owner"));
    assert!(!output.contains("\"id\""));
    std::fs::write(
        &config,
        format!(
            "state = {:?}\narchive = {:?}\npublic_bind = \"invalid\"\n",
            state, archive
        ),
    )
    .unwrap();
    assert!(invoke(&["accounts"]).status.success());
    let paths = invoke(&["paths"]);
    assert!(paths.status.success());
    assert!(String::from_utf8_lossy(&paths.stdout).contains(state.to_str().unwrap()));
    std::fs::write(&config, "invalid = [").unwrap();
    assert!(!invoke(&["accounts"]).status.success());
    let mut recover = Command::new(binary)
        .arg("--config")
        .arg(&config)
        .args([
            "recover-account",
            "--state",
            state.to_str().unwrap(),
            "--username",
            "owner",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    recover
        .stdin
        .take()
        .unwrap()
        .write_all(b"replacement long password\n")
        .unwrap();
    let result = recover.wait_with_output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(!archive.exists());
    std::fs::write(
        &config,
        format!("state = {:?}\narchive = {:?}\n", state, archive),
    )
    .unwrap();
    let mut configured = Command::new(binary)
        .arg("--config")
        .arg(&config)
        .args(["recover-account", "--username", "owner"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    configured
        .stdin
        .take()
        .unwrap()
        .write_all(b"another replacement password\n")
        .unwrap();
    let result = configured.wait_with_output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(!archive.exists());
    std::fs::write(
        state.join("private.sqlite"),
        b"unsupported damaged private state",
    )
    .unwrap();
    assert!(
        !invoke(&["accounts", "--state", state.to_str().unwrap()])
            .status
            .success()
    );
}
