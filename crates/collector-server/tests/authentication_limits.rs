//! Actual TCP listener identity, throttling and process-restart behavior.
use std::{
    fs,
    io::Write,
    net::TcpListener,
    path::Path,
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

async fn serve(config: &Path, client: &reqwest::Client, public: &str) -> Process {
    let log = config.with_extension("serve.log");
    let output = fs::File::create(&log).unwrap();
    let mut process = Process(
        Command::new(env!("CARGO_BIN_EXE_collector-server"))
            .arg("--config")
            .arg(config)
            .arg("serve")
            .stdout(output.try_clone().unwrap())
            .stderr(output)
            .spawn()
            .unwrap(),
    );
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        assert!(
            process.0.try_wait().unwrap().is_none(),
            "{}",
            fs::read_to_string(&log).unwrap()
        );
        if client
            .get(format!("{public}/healthz"))
            .send()
            .await
            .is_ok_and(|response| response.status() == 200)
        {
            return process;
        }
        assert!(
            Instant::now() < deadline,
            "Rate qualification server timed out"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

async fn login(
    client: &reqwest::Client,
    base: &str,
    origin: &str,
    username: &str,
    password: &str,
    alias: u16,
) -> reqwest::Response {
    client.post(format!("{base}/api/v1/auth/login"))
        .header("Origin", origin)
        .header("X-Forwarded-For", format!("203.0.113.{}", alias % 254 + 1))
        .header("Forwarded", format!("for=198.51.100.{}", alias % 254 + 1))
        .json(&serde_json::json!({"username":username,"password":password,"device_name":"Rate qualification"}))
        .send().await.unwrap()
}

#[tokio::test]
async fn actual_listeners_share_direct_peer_limits_and_restart_resets_only_transient_limits() {
    let root = tempfile::tempdir().unwrap();
    let config = root.path().join("collector.toml");
    let ports: Vec<_> = (0..3)
        .map(|_| TcpListener::bind("127.0.0.1:0").unwrap())
        .collect();
    let addresses: Vec<_> = ports
        .iter()
        .map(|port| port.local_addr().unwrap())
        .collect();
    let public = format!("http://{}", addresses[0]);
    let private = format!("http://{}", addresses[1]);
    fs::write(&config, format!(
        "archive = {:?}\nstate = {:?}\npublic_bind = {:?}\nprivate_bind = {:?}\nlan_bind = {:?}\nlocal_playback_bind = \"127.0.0.1:0\"\nallowed_origins = [{private:?}]\n",
        root.path().join("archive"),root.path().join("state"),addresses[0].to_string(),addresses[1].to_string(),addresses[2].to_string()
    )).unwrap();
    let mut setup = Process(
        Command::new(env!("CARGO_BIN_EXE_collector-server"))
            .arg("--config")
            .arg(&config)
            .arg("setup")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    const PASSWORD: &str = "disposable rate qualification password";
    setup
        .0
        .stdin
        .take()
        .unwrap()
        .write_all(PASSWORD.as_bytes())
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        if let Some(status) = setup.0.try_wait().unwrap() {
            assert!(status.success());
            break;
        }
        assert!(Instant::now() < deadline, "Rate fixture setup timed out");
        std::thread::sleep(Duration::from_millis(20));
    }
    drop(ports);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();
    let process = serve(&config, &client, &public).await;
    let initial = login(&client, &private, &private, "owner", PASSWORD, 0).await;
    assert_eq!(initial.status(), 200);
    let cookie = initial.headers()["set-cookie"]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    for index in 1..10 {
        let base = if index % 2 == 0 { &private } else { &public };
        assert_eq!(
            login(&client, base, &private, "owner", "wrong password", index)
                .await
                .status(),
            401
        );
    }
    let limited = login(&client, &public, &private, "owner", PASSWORD, 10).await;
    assert_eq!(limited.status(), 429);
    assert_eq!(limited.headers()["retry-after"], "60");
    assert_eq!(
        limited.json::<serde_json::Value>().await.unwrap()["error"]["code"],
        "rate_limited"
    );
    // Different account names still reach authentication until the shared real
    // TCP peer's 30 admitted attempts are exhausted. Proxy aliases cannot reset it.
    for index in 10..30 {
        assert_eq!(
            login(
                &client,
                &private,
                &private,
                &format!("unknown-{index}"),
                "wrong",
                index
            )
            .await
            .status(),
            401
        );
    }
    for index in 30..62 {
        assert_eq!(
            login(
                &client,
                &public,
                &private,
                &format!("unknown-{index}"),
                "wrong",
                index
            )
            .await
            .status(),
            429
        );
    }
    assert_eq!(
        client
            .get(format!("{private}/api/v1/me"))
            .header("Cookie", &cookie)
            .send()
            .await
            .unwrap()
            .status(),
        200
    );
    drop(process);
    let _restarted = serve(&config, &client, &public).await;
    assert_eq!(
        client
            .get(format!("{private}/api/v1/me"))
            .header("Cookie", &cookie)
            .send()
            .await
            .unwrap()
            .status(),
        200
    );
    assert_eq!(
        login(&client, &public, &private, "owner", PASSWORD, 63)
            .await
            .status(),
        200
    );
    assert_eq!(
        login(
            &client,
            &private,
            "https://attacker.example",
            "owner",
            PASSWORD,
            64
        )
        .await
        .status(),
        403
    );
}
