//! Optional local Proton Bridge adapter; all retailer interpretation is transport-independent.
use collector_app::{MailBatch, MailProfile, MailProviderPort};
use std::sync::Mutex;
mod setup;
use collector_core::{Error, Result, UserId};
use serde::{Deserialize, Serialize};
use std::{
    io::{Read, Write},
    path::Path,
    process::{Command, Stdio},
    time::{Duration, Instant},
};
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Profile {
    id: String,
    owner: UserId,
    label: String,
    username: String,
    password: String,
    certificate_sha256: String,
    #[serde(default = "port")]
    port: u16,
    #[serde(default = "folders")]
    folders: Vec<String>,
}
fn port() -> u16 {
    1143
}
fn folders() -> Vec<String> {
    vec!["INBOX".into()]
}
/// Secret fields deliberately have no Debug implementation and never enter API DTOs.
pub struct ProtonMailProvider {
    profiles: Mutex<Vec<Profile>>,
    setup: Option<setup::Paths>,
}
impl ProtonMailProvider {
    pub fn load(path: &Path) -> Result<Self> {
        let metadata = std::fs::metadata(path)
            .map_err(|_| Error::Unavailable("Mail profiles file unavailable".into()))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if metadata.permissions().mode() & 0o077 != 0 {
                return Err(Error::Invalid(
                    "Mail profiles file must have mode 0600 or 0400".into(),
                ));
            }
        }
        if metadata.len() > 65536 {
            return Err(Error::Invalid("Mail profiles file too large".into()));
        }
        let profiles: Vec<Profile> = serde_json::from_slice(
            &std::fs::read(path)
                .map_err(|_| Error::Unavailable("Cannot read mail profiles".into()))?,
        )
        .map_err(|_| Error::Invalid("Invalid mail profiles configuration".into()))?;
        let mut ids = std::collections::HashSet::new();
        if profiles.len() > 16 {
            return Err(Error::Invalid("Too many mail profiles".into()));
        }
        for p in &profiles {
            if p.id.is_empty()
                || p.id.len() > 64
                || !p
                    .id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
                || !ids.insert(&p.id)
                || p.label.len() > 100
                || p.certificate_sha256.len() != 64
                || !p.certificate_sha256.bytes().all(|b| b.is_ascii_hexdigit())
                || p.username.len() > 320
                || p.password.len() > 1024
                || p.folders.is_empty()
                || p.folders.len() > 16
            {
                return Err(Error::Invalid("Invalid mail profile limits".into()));
            }
        }
        Ok(Self {
            profiles: Mutex::new(profiles),
            setup: None,
        })
    }
}
impl MailProviderPort for ProtonMailProvider {
    fn profiles(&self) -> Vec<MailProfile> {
        self.profiles
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .map(|p| MailProfile {
                id: p.id.clone(),
                owner: p.owner,
                label: p.label.clone(),
                provider: "proton_bridge".into(),
            })
            .collect()
    }
    fn fetch(&self, id: &str, cursor: &str) -> Result<MailBatch> {
        let profiles = self.profiles.lock().map_err(|_| Error::Conflict)?;
        let profile = profiles
            .iter()
            .find(|p| p.id == id)
            .ok_or(Error::NotFound)?;
        let request = serde_json::to_vec(&serde_json::json!({"profile":profile,"cursor":cursor}))
            .map_err(|_| Error::Conflict)?;
        drop(profiles);
        run_mail(request, cursor, 90)
    }
}
fn run_mail(request: Vec<u8>, cursor: &str, timeout: u64) -> Result<MailBatch> {
    if cursor.len() > 65536 {
        return Err(Error::Invalid("Mail cursor too large".into()));
    }
    let mut child = Command::new("python3")
        .args(["-I", "-c", include_str!("mail/connector.py")])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| Error::Unavailable("Mail adapter requires Python 3".into()))?;
    let mut stdin = child.stdin.take().ok_or(Error::Conflict)?;
    let input = std::thread::spawn(move || stdin.write_all(&request));
    let stdout = child.stdout.take().ok_or(Error::Conflict)?;
    let output = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout
            .take(2 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map(|_| bytes)
    });
    let start = Instant::now();
    let success = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.success(),
            Ok(None) if start.elapsed() < Duration::from_secs(timeout) => {
                std::thread::sleep(Duration::from_millis(50))
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                break false;
            }
        }
    };
    let input_ok = input.join().is_ok_and(|result| result.is_ok());
    let bytes = output
        .join()
        .map_err(|_| Error::Unavailable("Mail adapter output failed".into()))?
        .map_err(|_| Error::Unavailable("Mail adapter output unavailable".into()))?;
    if !success || !input_ok || bytes.len() > 2 * 1024 * 1024 {
        return Err(Error::Unavailable("Mail adapter failed".into()));
    }
    serde_json::from_slice(&bytes)
        .map_err(|_| Error::Unavailable("Mail adapter returned invalid results".into()))
}
