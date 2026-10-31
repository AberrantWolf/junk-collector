//! Private setup mechanisms. Only sanitized status and MailProfile leave this adapter.
use super::*;
use collector_app::{
    BridgeSyncPhase, BridgeSyncStatus, ConnectBridge, MailSetupPort, MailSetupStatus,
};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

pub(super) struct Paths {
    directory: PathBuf,
    status: PathBuf,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Receipt {
    request_key: String,
    input_digest: String,
    profile: Profile,
}
fn unavailable() -> Error {
    Error::Unavailable(
        "Bridge setup storage is unavailable; check its private directory and status reporter."
            .into(),
    )
}
fn public(p: &Profile) -> MailProfile {
    MailProfile {
        id: p.id.clone(),
        owner: p.owner,
        label: p.label.clone(),
        provider: "proton_bridge".into(),
    }
}
fn bounded_bytes(path: &Path, limit: u64) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(|_| unavailable())?
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| unavailable())?;
    if bytes.len() as u64 > limit {
        return Err(unavailable());
    }
    Ok(bytes)
}
fn read_private(path: &Path) -> Result<Receipt> {
    let m = fs::symlink_metadata(path).map_err(|_| unavailable())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if m.permissions().mode() & 0o077 != 0 {
            return Err(unavailable());
        }
    }
    if !m.is_file() || m.len() > 16384 {
        return Err(unavailable());
    }
    serde_json::from_slice(&bounded_bytes(path, 16384)?).map_err(|_| unavailable())
}
impl ProtonMailProvider {
    pub fn with_setup(mut self, directory: &Path, status: &Path) -> Result<Self> {
        fs::create_dir_all(directory).map_err(|_| unavailable())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(directory, fs::Permissions::from_mode(0o700))
                .map_err(|_| unavailable())?;
        }
        let mut profiles = self.profiles.lock().map_err(|_| unavailable())?;
        for (index, entry) in fs::read_dir(directory)
            .map_err(|_| unavailable())?
            .enumerate()
        {
            if index >= 32 {
                return Err(unavailable());
            }
            let entry = entry.map_err(|_| unavailable())?;
            if entry.file_name().to_string_lossy().starts_with('.') {
                continue;
            }
            if profiles.len() >= 16 {
                return Err(unavailable());
            }
            let receipt = read_private(&entry.path())?;
            if receipt.profile.id != format!("bridge-{}", receipt.profile.owner)
                || profiles
                    .iter()
                    .any(|p| p.id == receipt.profile.id || p.owner == receipt.profile.owner)
            {
                return Err(unavailable());
            }
            profiles.push(receipt.profile);
        }
        drop(profiles);
        self.setup = Some(Paths {
            directory: directory.to_owned(),
            status: status.to_owned(),
        });
        Ok(self)
    }
}
impl MailSetupPort for ProtonMailProvider {
    fn setup_status(&self) -> Result<MailSetupStatus> {
        let Some(paths) = &self.setup else {
            return Ok(MailSetupStatus {
                can_connect: false,
                bridge: None,
                problem: Some("Web connection setup is not installed on this server.".into()),
            });
        };
        let result = (|| -> Result<BridgeSyncStatus> {
            let m = fs::symlink_metadata(&paths.status).map_err(|_| unavailable())?;
            if !m.is_file() || m.len() > 8192 {
                return Err(unavailable());
            }
            let s: BridgeSyncStatus = serde_json::from_slice(&bounded_bytes(&paths.status, 8192)?)
                .map_err(|_| unavailable())?;
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|_| unavailable())?
                .as_secs();
            if s.imap_port == 0
                || s.schema != 1
                || s.observed_at > now + 5
                || now.saturating_sub(s.observed_at) > 20
                || s.event_at.is_some_and(|t| t > s.observed_at)
                || (matches!(
                    s.phase,
                    BridgeSyncPhase::Syncing
                        | BridgeSyncPhase::Synced
                        | BridgeSyncPhase::Interrupted
                ) && s.event_at.is_none())
                || s.percent
                    .is_some_and(|v| !v.is_finite() || !(0.0..=100.0).contains(&v))
                || [s.elapsed_seconds, s.remaining_seconds]
                    .into_iter()
                    .flatten()
                    .any(|v| !v.is_finite() || v < 0.0)
                || s.certificate_sha256
                    .as_ref()
                    .is_some_and(|v| v.len() != 64 || !v.bytes().all(|c| c.is_ascii_hexdigit()))
            {
                return Err(unavailable());
            }
            if s.phase != BridgeSyncPhase::Syncing
                && (s.percent.is_some() || s.remaining_seconds.is_some())
            {
                return Err(unavailable());
            }
            Ok(s)
        })();
        Ok(match result {
            Ok(s)=>MailSetupStatus{can_connect:!matches!(s.phase,BridgeSyncPhase::Starting|BridgeSyncPhase::Stopped) && s.certificate_sha256.is_some(),bridge:Some(s),problem:None},
            Err(_)=>MailSetupStatus{can_connect:false,bridge:None,problem:Some("Bridge status is unavailable or stale. Check the Bridge container and status reporter.".into())},
        })
    }
    fn connect_bridge(
        &self,
        owner: UserId,
        command: &ConnectBridge,
        request_key: &str,
    ) -> Result<MailProfile> {
        self.connect_with_probe(owner,command,request_key,|profile| {
            let request=serde_json::to_vec(&serde_json::json!({"profile":profile,"cursor":"","operation":"probe"})).map_err(|_|unavailable())?;
            super::run_mail(request,"",20).map(|_|()).map_err(|_|Error::Unavailable("Could not connect: check local IMAP credentials, folder names, Bridge login and certificate. No profile was saved.".into()))
        })
    }
}
impl ProtonMailProvider {
    fn connect_with_probe(
        &self,
        owner: UserId,
        command: &ConnectBridge,
        request_key: &str,
        probe: impl FnOnce(&Profile) -> Result<()>,
    ) -> Result<MailProfile> {
        // Bind/replay submitted inputs before resolving mutable status/certificate state.
        let input_digest = hex::encode(Sha256::digest(
            serde_json::to_vec(&(owner, command)).map_err(|_| unavailable())?,
        ));
        let paths = self.setup.as_ref().ok_or_else(unavailable)?;
        let mut profiles = self.profiles.lock().map_err(|_| unavailable())?;
        let path = paths.directory.join(format!("{owner}.json"));
        if path.try_exists().map_err(|_| unavailable())? {
            let receipt = read_private(&path)?;
            if receipt.request_key == request_key && receipt.input_digest != input_digest {
                return Err(Error::IdempotencyConflict);
            }
            if receipt.input_digest != input_digest || receipt.profile.owner != owner {
                return Err(Error::Conflict);
            }
            // A prior attempt may have committed the inode but failed the directory sync.
            fs::File::open(&paths.directory)
                .and_then(|f| f.sync_all())
                .map_err(|_| unavailable())?;
            if !profiles.iter().any(|p| p.id == receipt.profile.id) {
                profiles.push(receipt.profile);
            }
            return profiles
                .iter()
                .find(|p| p.owner == owner)
                .map(public)
                .ok_or(Error::Conflict);
        }
        if profiles.len() >= 16 || profiles.iter().any(|p| p.owner == owner) {
            return Err(Error::Conflict);
        }
        let status = self.setup_status()?;
        if !status.can_connect {
            return Err(Error::Unavailable("Bridge is not ready for a connection. Check its status and sign in through the private Bridge terminal first.".into()));
        }
        let bridge = status.bridge.ok_or_else(unavailable)?;
        let profile = Profile {
            id: format!("bridge-{owner}"),
            owner,
            label: "My Proton Mail".into(),
            username: command.username.clone(),
            password: command.password.clone(),
            certificate_sha256: bridge.certificate_sha256.ok_or_else(unavailable)?,
            port: bridge.imap_port,
            folders: command.folders.clone(),
        };
        probe(&profile)?;
        let receipt = Receipt {
            request_key: request_key.into(),
            input_digest,
            profile,
        };
        // Profile plus receipt is one immutable publication, never an overwritten credentials file.
        let mut pending =
            tempfile::NamedTempFile::new_in(&paths.directory).map_err(|_| unavailable())?;
        serde_json::to_writer(&mut pending, &receipt).map_err(|_| unavailable())?;
        pending.as_file().sync_all().map_err(|_| unavailable())?;
        pending
            .persist_noclobber(&path)
            .map_err(|_| Error::Conflict)?;
        fs::File::open(&paths.directory)
            .and_then(|f| f.sync_all())
            .map_err(|_| unavailable())?;
        let value = public(&receipt.profile);
        profiles.push(receipt.profile);
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn command() -> ConnectBridge {
        ConnectBridge {
            username: "local-imap-user".into(),
            password: "private-imap-password".into(),
            folders: vec!["INBOX".into()],
        }
    }
    fn fixture() -> (tempfile::TempDir, ProtonMailProvider, PathBuf, PathBuf) {
        let root = tempfile::tempdir().unwrap();
        let profiles = root.path().join("profiles.json");
        fs::write(&profiles, "[]").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&profiles, fs::Permissions::from_mode(0o600)).unwrap();
        }
        let directory = root.path().join("managed");
        let status = root.path().join("status.json");
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        fs::write(&status,serde_json::to_vec(&serde_json::json!({"schema":1,"imap_port":1143,"observed_at":now,"event_at":now,"phase":"syncing","percent":37.5,"elapsed_seconds":12.0,"remaining_seconds":20.0,"certificate_sha256":"a".repeat(64)})).unwrap()).unwrap();
        let provider = ProtonMailProvider::load(&profiles)
            .unwrap()
            .with_setup(&directory, &status)
            .unwrap();
        (root, provider, directory, status)
    }
    #[test]
    fn private_profile_and_receipt_commit_together_and_replay_before_mutable_status() {
        let (root, provider, directory, status) = fixture();
        let owner = UserId::new();
        let input = command();
        let profile = provider
            .connect_with_probe(owner, &input, "request-one", |p| {
                assert_eq!(p.certificate_sha256, "a".repeat(64));
                assert_eq!(p.username, input.username);
                assert_eq!(p.port, 1143);
                Ok(())
            })
            .unwrap();
        let safe = serde_json::to_string(&profile).unwrap();
        assert!(!safe.contains(&input.password));
        assert!(!safe.contains(&input.username));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(directory.join(format!("{owner}.json")))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
        }
        fs::remove_file(status).unwrap();
        let replay = provider
            .connect_with_probe(owner, &input, "request-one", |_| {
                panic!("replay must not probe mutable server state")
            })
            .unwrap();
        assert_eq!(replay.id, profile.id);
        let mut changed = command();
        changed.password = "changed-secret".into();
        assert!(matches!(
            provider.connect_with_probe(owner, &changed, "request-one", |_| panic!()),
            Err(Error::IdempotencyConflict)
        ));
        drop(provider);
        let reopened = ProtonMailProvider::load(&root.path().join("profiles.json"))
            .unwrap()
            .with_setup(&directory, &root.path().join("status.json"))
            .unwrap();
        assert_eq!(reopened.profiles()[0].owner, owner);
        assert_eq!(
            reopened
                .connect_with_probe(owner, &input, "request-two", |_| panic!())
                .unwrap()
                .id,
            profile.id
        );
    }
    #[test]
    fn failed_probe_does_not_publish_and_stale_or_inconsistent_status_is_unavailable() {
        let (_root, provider, directory, path) = fixture();
        assert!(provider.setup_status().unwrap().can_connect);
        assert!(
            provider
                .connect_with_probe(UserId::new(), &command(), "failed", |_| Err(
                    Error::Unavailable("fixture".into())
                ))
                .is_err()
        );
        assert_eq!(fs::read_dir(directory).unwrap().count(), 0);
        assert!(provider.profiles().is_empty());
        let original: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        let mut complete = original.clone();
        complete["phase"] = serde_json::json!("synced");
        complete["percent"] = serde_json::Value::Null;
        complete["remaining_seconds"] = serde_json::Value::Null;
        complete["event_at"] = serde_json::Value::Null;
        fs::write(&path, serde_json::to_vec(&complete).unwrap()).unwrap();
        assert!(provider.setup_status().unwrap().bridge.is_none());
        complete["event_at"] = original["event_at"].clone();
        fs::write(&path, serde_json::to_vec(&complete).unwrap()).unwrap();
        assert_eq!(
            provider.setup_status().unwrap().bridge.unwrap().phase,
            BridgeSyncPhase::Synced
        );
        for (key, value) in [
            ("observed_at", serde_json::json!(0)),
            ("percent", serde_json::json!(101.0)),
            ("phase", serde_json::json!("synced")),
            ("imap_port", serde_json::json!(0)),
            ("certificate_sha256", serde_json::json!("invalid")),
        ] {
            let mut candidate = original.clone();
            candidate[key] = value;
            fs::write(&path, serde_json::to_vec(&candidate).unwrap()).unwrap();
            let status = provider.setup_status().unwrap();
            assert!(!status.can_connect);
            assert!(status.bridge.is_none());
            assert!(status.problem.is_some());
        }
    }
}
