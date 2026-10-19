#![cfg(unix)]

use std::{
    fs,
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

struct RunningServer(Child);
impl Drop for RunningServer {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

// Exercise the actual executable: receiving a signal must return successfully,
// release the archive lock and leave the installation openable on the next start.
#[test]
fn terminate_and_interrupt_exit_successfully_and_release_storage() {
    let root = tempfile::tempdir().unwrap();
    let config = root.path().join("collector.toml");
    let reservations: Vec<_> = (0..3)
        .map(|_| TcpListener::bind("127.0.0.1:0").unwrap())
        .collect();
    let addresses: Vec<SocketAddr> = reservations
        .iter()
        .map(|listener| listener.local_addr().unwrap())
        .collect();
    fs::write(
        &config,
        format!(
            "archive = {:?}\nstate = {:?}\npublic_bind = {:?}\nprivate_bind = {:?}\nlan_bind = {:?}\n",
            root.path().join("archive"),
            root.path().join("state"),
            addresses[0].to_string(),
            addresses[1].to_string(),
            addresses[2].to_string()
        ),
    )
    .unwrap();
    let binary = env!("CARGO_BIN_EXE_collector-server");
    let log = fs::File::create(root.path().join("server.log")).unwrap();
    let mut setup = Command::new(binary)
        .arg("--config")
        .arg(&config)
        .arg("setup")
        .stdin(Stdio::piped())
        .stdout(log.try_clone().unwrap())
        .stderr(log.try_clone().unwrap())
        .spawn()
        .unwrap();
    setup
        .stdin
        .take()
        .unwrap()
        .write_all(b"disposable signal test password\n")
        .unwrap();
    assert!(setup.wait().unwrap().success());
    drop(reservations);
    for signal in ["TERM", "INT"] {
        let mut server = RunningServer(
            Command::new(binary)
                .arg("--config")
                .arg(&config)
                .arg("serve")
                .stdout(log.try_clone().unwrap())
                .stderr(log.try_clone().unwrap())
                .spawn()
                .unwrap(),
        );
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            assert!(
                server.0.try_wait().unwrap().is_none(),
                "server exited early"
            );
            if let Ok(mut stream) =
                TcpStream::connect_timeout(&addresses[1], Duration::from_millis(100))
            {
                stream
                    .set_read_timeout(Some(Duration::from_millis(200)))
                    .unwrap();
                stream
                    .write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
                    .unwrap();
                let mut response = [0; 12];
                if stream.read_exact(&mut response).is_ok() && &response == b"HTTP/1.1 200" {
                    break;
                }
            }
            assert!(Instant::now() < deadline, "server did not start");
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(
            Command::new("kill")
                .args([format!("-{signal}"), server.0.id().to_string()])
                .status()
                .unwrap()
                .success()
        );
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let Some(status) = server.0.try_wait().unwrap() {
                assert!(status.success(), "{signal} returned {status}");
                break;
            }
            assert!(Instant::now() < deadline, "{signal} shutdown timed out");
            std::thread::sleep(Duration::from_millis(20));
        }
        // Confirm process teardown released durable state, independently of HTTP.
        drop(
            collector_store::Store::open(root.path().join("archive"), root.path().join("state"))
                .unwrap(),
        );
    }
}
