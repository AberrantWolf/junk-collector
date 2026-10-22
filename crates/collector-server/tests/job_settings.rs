use std::{
    fs,
    io::Write,
    process::{Command, Stdio},
};

fn run(config: &std::path::Path, command: &str) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_collector-server"))
        .args(["--config", config.to_str().unwrap(), command])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"isolated worker settings password\n")
        .ok();
    child.wait_with_output().unwrap()
}

#[test]
fn invalid_worker_settings_fail_before_storage_is_opened() {
    for jobs in [
        "processing_workers = 0",
        "processing_workers = 9",
        "processing_workers = 256",
        "processing_workers = 4\ncopy_workers = 0",
        "processing_workers = 1\ncopy_workers = 2",
        "provider_workers = 2",
    ] {
        let root = tempfile::tempdir().unwrap();
        let config = root.path().join("collector.toml");
        let archive = root.path().join("archive");
        let state = root.path().join("state");
        fs::write(
            &config,
            format!("archive = {archive:?}\nstate = {state:?}\n[jobs]\n{jobs}\n"),
        )
        .unwrap();
        for command in ["serve", "setup", "doctor"] {
            let output = run(&config, command);
            assert!(!output.status.success(), "{command} accepted {jobs}");
            assert!(
                !archive.exists() && !state.exists(),
                "invalid settings created storage"
            );
        }
    }
}

#[test]
fn setup_persists_defaults_and_doctor_reports_each_supported_limit() {
    let root = tempfile::tempdir().unwrap();
    let config = root.path().join("collector.toml");
    let archive = root.path().join("archive");
    let state = root.path().join("state");
    fs::write(
        &config,
        format!("archive = {archive:?}\nstate = {state:?}\n"),
    )
    .unwrap();
    let setup = run(&config, "setup");
    assert!(
        setup.status.success(),
        "{}",
        String::from_utf8_lossy(&setup.stderr)
    );
    let mut saved: toml::Value = toml::from_str(&fs::read_to_string(&config).unwrap()).unwrap();
    assert_eq!(saved["jobs"]["processing_workers"].as_integer(), Some(4));
    assert_eq!(saved["jobs"]["copy_workers"].as_integer(), Some(1));
    for workers in 1..=8 {
        saved["jobs"]["processing_workers"] = toml::Value::Integer(workers);
        fs::write(&config, toml::to_string(&saved).unwrap()).unwrap();
        let doctor = run(&config, "doctor");
        assert!(
            doctor.status.success(),
            "{}",
            String::from_utf8_lossy(&doctor.stderr)
        );
        let text = String::from_utf8(doctor.stdout).unwrap();
        assert!(text.contains(&format!(
            "{workers} processing workers; inbox copy limit 1; provider limit 1"
        )));
    }
}
