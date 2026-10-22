use std::{
    fs,
    io::Write,
    net::TcpListener,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

struct Process(Child);
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn cli(config: &std::path::Path, command: &str) -> String {
    let log = config.with_extension("log");
    let output = fs::File::create(&log).unwrap();
    let mut process = Process(
        Command::new(env!("CARGO_BIN_EXE_collector-server"))
            .arg("--config")
            .arg(config)
            .arg(command)
            .stdin(Stdio::piped())
            .stdout(output.try_clone().unwrap())
            .stderr(output)
            .spawn()
            .unwrap(),
    );
    process
        .0
        .stdin
        .take()
        .unwrap()
        .write_all(b"disposable origin password\n")
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        if let Some(status) = process.0.try_wait().unwrap() {
            let text = fs::read_to_string(log).unwrap();
            assert!(status.success(), "{command}: {text}");
            return text;
        }
        assert!(Instant::now() < deadline, "{command} timed out");
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[tokio::test]
async fn numeric_origins_start_with_password_login_and_preserve_state_across_dns_changes() {
    let root = tempfile::tempdir().unwrap();
    let config = root.path().join("collector.toml");
    let ports: Vec<_> = (0..3)
        .map(|_| TcpListener::bind("127.0.0.1:0").unwrap())
        .collect();
    let addresses: Vec<_> = ports
        .iter()
        .map(|port| port.local_addr().unwrap())
        .collect();
    let base = format!("https://{}", addresses[2]);
    let origins = [
        base.clone(),
        format!("https://localhost:{}", addresses[2].port()),
        format!("https://[::1]:{}", addresses[2].port()),
        base.clone(),
    ];
    drop(ports);
    let mut previous_cookie = None;
    let mut previous_identity = None;
    let mut original_certificate = None;
    for (index, origin) in origins.iter().enumerate() {
        fs::write(&config, format!(
            "archive = {:?}\nstate = {:?}\npublic_bind = {:?}\nprivate_bind = {:?}\nlan_bind = {:?}\nlocal_playback_bind = \"127.0.0.1:0\"\nauthentication_origin = {:?}\nallowed_origins = [{:?}]\ntls_names = [\"127.0.0.1\", \"localhost\"]\n",
            root.path().join("archive"), root.path().join("state"), addresses[0].to_string(), addresses[1].to_string(), addresses[2].to_string(), origin, base)).unwrap();
        cli(&config, "setup");
        cli(&config, "doctor");
        let pem = fs::read(root.path().join("state/tls/cert.pem")).unwrap();
        if let Some(ref certificate) = original_certificate {
            assert_eq!(certificate, &pem);
        }
        original_certificate = Some(pem.clone());
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .add_root_certificate(reqwest::Certificate::from_pem(&pem).unwrap())
            .build()
            .unwrap();
        let log = fs::File::create(root.path().join("serve.log")).unwrap();
        let mut process = Process(
            Command::new(env!("CARGO_BIN_EXE_collector-server"))
                .arg("--config")
                .arg(&config)
                .arg("serve")
                .stdout(log.try_clone().unwrap())
                .stderr(log)
                .spawn()
                .unwrap(),
        );
        let deadline = Instant::now() + Duration::from_secs(20);
        let features = loop {
            assert!(
                process.0.try_wait().unwrap().is_none(),
                "{}",
                fs::read_to_string(root.path().join("serve.log")).unwrap()
            );
            if let Ok(response) = client.get(format!("{base}/api/v1/features")).send().await {
                break response
                    .error_for_status()
                    .unwrap()
                    .json::<serde_json::Value>()
                    .await
                    .unwrap();
            }
            assert!(Instant::now() < deadline, "HTTPS startup timed out");
            tokio::time::sleep(Duration::from_millis(30)).await;
        };
        let dns = index == 1;
        assert_eq!(features["passkeys"], dns);
        assert_eq!(
            features["passkeys_unavailable_reason"],
            if dns {
                serde_json::Value::Null
            } else {
                serde_json::json!("dns_name_required")
            }
        );
        let identity: serde_json::Value = client
            .get(format!("{base}/api/v1/identity"))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        if let Some(ref previous) = previous_identity {
            assert_eq!(previous, &identity);
        }
        previous_identity = Some(identity);
        if let Some(ref cookie) = previous_cookie {
            assert_eq!(
                client
                    .get(format!("{base}/api/v1/me"))
                    .header("Cookie", cookie)
                    .send()
                    .await
                    .unwrap()
                    .status(),
                200
            );
        }
        let response = client.post(format!("{base}/api/v1/auth/login")).header("Origin", &base)
            .json(&serde_json::json!({"username":"owner","password":"disposable origin password","device_name":"Origin qualification"})).send().await.unwrap();
        assert_eq!(response.status(), 200);
        let cookie = response.headers()["set-cookie"]
            .to_str()
            .unwrap()
            .split(';')
            .next()
            .unwrap()
            .to_owned();
        let registration = client
            .post(format!("{base}/api/v1/auth/passkeys/register/start"))
            .header("Origin", &base)
            .header("Cookie", &cookie)
            .send()
            .await
            .unwrap();
        assert_eq!(registration.status(), if dns { 200 } else { 503 });
        previous_cookie = Some(cookie);
        drop(process);
    }
}

#[test]
fn malformed_authentication_origins_fail_before_setup_or_startup_writes() {
    for origin in [
        "http://localhost:9844",
        "not a URL",
        "https://name.test/path",
        "https://name.test/?query",
        "https://name.test/#fragment",
        "https://user:secret@name.test",
    ] {
        for command in ["setup", "doctor", "serve"] {
            let root = tempfile::tempdir().unwrap();
            let config = root.path().join("collector.toml");
            let text = format!(
                "archive = {:?}\nstate = {:?}\nauthentication_origin = {:?}\n",
                root.path().join("archive"),
                root.path().join("state"),
                origin
            );
            fs::write(&config, &text).unwrap();
            let mut process = Process(
                Command::new(env!("CARGO_BIN_EXE_collector-server"))
                    .arg("--config")
                    .arg(&config)
                    .arg(command)
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()
                    .unwrap(),
            );
            let deadline = Instant::now() + Duration::from_secs(10);
            loop {
                if let Some(status) = process.0.try_wait().unwrap() {
                    assert!(!status.success(), "{origin}: {command}");
                    break;
                }
                assert!(Instant::now() < deadline, "invalid origin did not fail");
                std::thread::sleep(Duration::from_millis(20));
            }
            assert!(!root.path().join("archive").exists(), "{origin}: {command}");
            assert!(!root.path().join("state").exists(), "{origin}: {command}");
            assert_eq!(fs::read_to_string(&config).unwrap(), text);
        }
    }
}
