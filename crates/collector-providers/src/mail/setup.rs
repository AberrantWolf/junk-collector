//! Private setup mechanisms. Only sanitized status and MailProfile leave this adapter.
use super::*;
use collector_app::{
    BridgeSyncPhase, BridgeSyncStatus, ConnectBridge, MailSetupPort, MailSetupStatus, RepairBridge,
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
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RepairReceipt {
    activated: bool,
    input_digest: String,
    profile: MailProfile,
    port: u16,
}
fn read_repair(path: &Path) -> Result<RepairReceipt> {
    serde_json::from_slice(&private_bytes(path)?).map_err(|_| unavailable())
}
fn read_private(path: &Path) -> Result<Receipt> {
    serde_json::from_slice(&private_bytes(path)?).map_err(|_| unavailable())
}
fn active_profile_path(directory: &Path, owner: UserId, id: &str) -> PathBuf {
    if id == format!("bridge-{owner}") {
        directory.join(format!("{owner}.json"))
    } else {
        directory.join(format!(
            "profile-{}.json",
            hex::encode(Sha256::digest(id.as_bytes()))
        ))
    }
}
fn repair_path(directory: &Path, owner: UserId, key: &str) -> Result<PathBuf> {
    let digest = hex::encode(Sha256::digest(
        serde_json::to_vec(&(owner, key)).map_err(|_| unavailable())?,
    ));
    Ok(directory.join(".repairs").join(format!("{digest}.json")))
}
fn mark_active_repair(directory: &Path, active: &Receipt) -> Result<()> {
    let path = repair_path(directory, active.profile.owner, &active.request_key)?;
    if !path.try_exists().map_err(|_| unavailable())? {
        return Ok(());
    }
    let mut receipt = read_repair(&path)?;
    if receipt.input_digest != active.input_digest
        || receipt.profile.id != active.profile.id
        || receipt.profile.owner != active.profile.owner
        || receipt.profile.configuration_revision != active.profile.revision
    {
        return Err(Error::Conflict);
    }
    // The active rename must be durable before an activation receipt can be durable.
    fs::File::open(directory)
        .and_then(|f| f.sync_all())
        .map_err(|_| unavailable())?;
    if !receipt.activated {
        receipt.activated = true;
        let parent = path.parent().ok_or_else(unavailable)?;
        let mut pending = tempfile::NamedTempFile::new_in(parent).map_err(|_| unavailable())?;
        serde_json::to_writer(&mut pending, &receipt).map_err(|_| unavailable())?;
        pending.as_file().sync_all().map_err(|_| unavailable())?;
        pending.persist(&path).map_err(|_| unavailable())?;
        fs::File::open(parent)
            .and_then(|f| f.sync_all())
            .map_err(|_| unavailable())?;
    }
    Ok(())
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
        configuration_revision: p.revision,
        folders: p.folders.clone(),
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
fn activate(
    directory: &Path,
    target: &Path,
    receipt: &Receipt,
    on_replaced: impl FnOnce() -> Result<()>,
) -> Result<()> {
    let mut pending = tempfile::NamedTempFile::new_in(directory).map_err(|_| unavailable())?;
    serde_json::to_writer(&mut pending, receipt).map_err(|_| unavailable())?;
    pending.as_file().sync_all().map_err(|_| unavailable())?;
    pending.persist(target).map_err(|_| unavailable())?;
    on_replaced()?;
    fs::File::open(directory)
        .and_then(|f| f.sync_all())
        .map_err(|_| unavailable())
}
fn private_bytes(path: &Path) -> Result<Vec<u8>> {
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
    bounded_bytes(path, 16384)
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
            let receipt = read_private(&entry.path())?;
            if let Some(existing) = profiles.iter_mut().find(|p| p.id == receipt.profile.id) {
                if existing.owner != receipt.profile.owner {
                    return Err(unavailable());
                }
                *existing = receipt.profile;
            } else {
                if profiles.len() >= 16 || profiles.iter().any(|p| p.owner == receipt.profile.owner)
                {
                    return Err(unavailable());
                }
                profiles.push(receipt.profile);
            }
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
    fn repair_bridge(
        &self,
        owner: UserId,
        id: &str,
        command: &RepairBridge,
        request_key: &str,
    ) -> Result<MailProfile> {
        self.repair_with_probe(owner, id, command, request_key, |profile| {
            let request = serde_json::to_vec(&serde_json::json!({"profile":profile,"cursor":"","operation":"authenticate"})).map_err(|_| unavailable())?;
            let batch = super::run_mail(request, "", 20)?;
            if batch.failure.is_some() { return Err(Error::Unavailable("Could not revalidate Bridge. Check its credentials and certificate. Saved configuration is unchanged.".into())); }
            Ok(())
        })
    }
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
            super::run_mail(request,"",20).and_then(|b| if b.failure.is_none() { Ok(()) } else { Err(unavailable()) }).map_err(|_|Error::Unavailable("Could not connect: check local IMAP credentials, folder names, Bridge login and certificate. No profile was saved.".into()))
        })
    }
}
impl ProtonMailProvider {
    fn repair_with_probe(
        &self,
        owner: UserId,
        id: &str,
        command: &RepairBridge,
        request_key: &str,
        probe: impl FnOnce(&Profile) -> Result<()>,
    ) -> Result<MailProfile> {
        let paths = self.setup.as_ref().ok_or_else(unavailable)?;
        let input_digest = hex::encode(Sha256::digest(
            serde_json::to_vec(&(owner, id, command)).map_err(|_| unavailable())?,
        ));
        let mut profiles = self.profiles.lock().map_err(|_| unavailable())?;
        let index = profiles
            .iter()
            .position(|p| p.id == id && p.owner == owner)
            .ok_or(Error::NotFound)?;
        let receipt_path = repair_path(&paths.directory, owner, request_key)?;
        // Bind the submitted inputs before reading or settling mutable active state.
        let replay = if receipt_path.try_exists().map_err(|_| unavailable())? {
            let receipt = read_repair(&receipt_path)?;
            if receipt.input_digest != input_digest {
                return Err(Error::IdempotencyConflict);
            }
            if receipt.profile.owner != owner || receipt.profile.id != id {
                return Err(Error::Conflict);
            }
            Some(receipt)
        } else {
            None
        };
        let active_path = active_profile_path(&paths.directory, owner, id);
        let mut active_matches_replay = false;
        if active_path.try_exists().map_err(|_| unavailable())? {
            let active = read_private(&active_path)?;
            if active.profile.id != id || active.profile.owner != owner {
                return Err(Error::Conflict);
            }
            active_matches_replay =
                active.request_key == request_key && active.input_digest == input_digest;
            profiles[index] = active.profile.clone();
            mark_active_repair(&paths.directory, &active)?;
        }
        let receipts = paths.directory.join(".repairs");
        fs::create_dir_all(&receipts).map_err(|_| unavailable())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&receipts, fs::Permissions::from_mode(0o700))
                .map_err(|_| unavailable())?;
        }
        if let Some(receipt) = replay {
            let target = receipt.profile.configuration_revision;
            if receipt.activated || (active_matches_replay && profiles[index].revision == target) {
                if profiles[index].revision.0 < target.0
                    || (profiles[index].revision == target && !active_matches_replay)
                {
                    return Err(Error::Conflict);
                }
                return Ok(receipt.profile);
            }
            if profiles[index].revision != command.expected_revision
                || receipt.profile.configuration_revision.0
                    != command
                        .expected_revision
                        .0
                        .checked_add(1)
                        .ok_or(Error::Conflict)?
            {
                return Err(Error::Conflict);
            }
            let mut profile = profiles[index].clone();
            profile.password = command.password.clone();
            profile.certificate_sha256 = command.certificate_sha256.clone();
            profile.port = receipt.port;
            profile.revision = receipt.profile.configuration_revision;
            let active = Receipt {
                request_key: request_key.into(),
                input_digest: input_digest.clone(),
                profile,
            };
            activate(&paths.directory, &active_path, &active, || {
                profiles[index] = active.profile.clone();
                Ok(())
            })?;
            mark_active_repair(&paths.directory, &active)?;
            return Ok(receipt.profile);
        }
        if profiles[index].revision != command.expected_revision {
            return Err(Error::Conflict);
        }
        if profiles[index].username != command.username {
            return Err(Error::Invalid("Revalidation must use the same Bridge IMAP username so receipts cannot switch accounts.".into()));
        }
        let status = self.setup_status()?;
        let bridge = status
            .bridge
            .filter(|_| status.can_connect)
            .ok_or_else(unavailable)?;
        if bridge.certificate_sha256.as_deref() != Some(command.certificate_sha256.as_str()) {
            return Err(Error::Conflict);
        }
        let mut profile = profiles[index].clone();
        profile.password = command.password.clone();
        profile.certificate_sha256 = command.certificate_sha256.clone();
        profile.port = bridge.imap_port;
        profile.revision =
            collector_core::Revision(profile.revision.0.checked_add(1).ok_or(Error::Conflict)?);
        probe(&profile)?;
        let receipt = Receipt {
            request_key: request_key.into(),
            input_digest,
            profile,
        };
        let mut pending = tempfile::NamedTempFile::new_in(&receipts).map_err(|_| unavailable())?;
        serde_json::to_writer(
            &mut pending,
            &RepairReceipt {
                activated: false,
                input_digest: receipt.input_digest.clone(),
                profile: public(&receipt.profile),
                port: receipt.profile.port,
            },
        )
        .map_err(|_| unavailable())?;
        pending.as_file().sync_all().map_err(|_| unavailable())?;
        pending
            .persist_noclobber(&receipt_path)
            .map_err(|_| Error::Conflict)?;
        fs::File::open(&receipts)
            .and_then(|f| f.sync_all())
            .map_err(|_| unavailable())?;
        activate(&paths.directory, &active_path, &receipt, || {
            profiles[index] = receipt.profile.clone();
            Ok(())
        })?;
        mark_active_repair(&paths.directory, &receipt)?;
        Ok(public(&profiles[index]))
    }
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
            revision: collector_core::Revision(1),
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
    #[test]
    fn revalidation_is_pinned_revision_checked_replayable_and_retains_identity_without_old_secrets()
    {
        let (root, provider, directory, status) = fixture();
        let owner = UserId::new();
        let initial = provider
            .connect_with_probe(owner, &command(), "initial", |_| Ok(()))
            .unwrap();
        let repair = RepairBridge {
            username: command().username,
            password: "new-private-password".into(),
            certificate_sha256: "a".repeat(64),
            expected_revision: initial.configuration_revision,
        };
        assert!(
            provider
                .repair_with_probe(owner, &initial.id, &repair, "failed", |_| Err(
                    Error::Unavailable("fixture".into())
                ))
                .is_err()
        );
        assert_eq!(
            provider.profiles()[0].configuration_revision,
            initial.configuration_revision
        );
        let repaired = provider
            .repair_with_probe(owner, &initial.id, &repair, "repair-one", |p| {
                assert_eq!(p.password, repair.password);
                assert_eq!(p.certificate_sha256, repair.certificate_sha256);
                Ok(())
            })
            .unwrap();
        assert_eq!(repaired.id, initial.id);
        assert!(matches!(
            provider.with_configuration(
                &initial.id,
                initial.configuration_revision,
                &mut || panic!("stale fetch must not publish")
            ),
            Err(Error::Conflict)
        ));
        let mut admitted = false;
        provider
            .with_configuration(&repaired.id, repaired.configuration_revision, &mut || {
                admitted = true;
                Ok(())
            })
            .unwrap();
        assert!(admitted);
        assert!(repaired.configuration_revision.0 > initial.configuration_revision.0);
        let active = directory.join(format!("{owner}.json"));
        let saved = fs::read(&active).unwrap();
        assert!(!String::from_utf8_lossy(&saved).contains(&command().password));
        for entry in fs::read_dir(directory.join(".repairs")).unwrap() {
            let bytes = fs::read(entry.unwrap().path()).unwrap();
            let text = String::from_utf8_lossy(&bytes);
            assert!(!text.contains(&repair.password));
            assert!(!text.contains(&command().password));
            assert!(!text.contains(&command().username));
        }
        let mut changed = RepairBridge {
            username: repair.username.clone(),
            password: "different".into(),
            certificate_sha256: repair.certificate_sha256.clone(),
            expected_revision: repair.expected_revision,
        };
        assert!(matches!(
            provider.repair_with_probe(owner, &initial.id, &changed, "repair-one", |_| panic!()),
            Err(Error::IdempotencyConflict)
        ));
        assert!(matches!(
            provider.repair_with_probe(owner, &initial.id, &repair, "stale", |_| panic!()),
            Err(Error::Conflict)
        ));
        changed.expected_revision = repaired.configuration_revision;
        changed.username = "another-account".into();
        assert!(matches!(
            provider.repair_with_probe(owner, &initial.id, &changed, "wrong-account", |_| panic!()),
            Err(Error::Invalid(_))
        ));
        let mut report: serde_json::Value =
            serde_json::from_slice(&fs::read(&status).unwrap()).unwrap();
        report["certificate_sha256"] = serde_json::json!("b".repeat(64));
        fs::write(&status, serde_json::to_vec(&report).unwrap()).unwrap();
        let mut second = RepairBridge {
            username: repair.username.clone(),
            password: "third-private-password".into(),
            certificate_sha256: repair.certificate_sha256.clone(),
            expected_revision: repaired.configuration_revision,
        };
        assert!(matches!(
            provider.repair_with_probe(owner, &initial.id, &second, "wrong-pin", |_| panic!()),
            Err(Error::Conflict)
        ));
        second.certificate_sha256 = "b".repeat(64);
        let latest = provider
            .repair_with_probe(owner, &initial.id, &second, "repair-two", |_| Ok(()))
            .unwrap();
        fs::remove_file(&status).unwrap();
        assert_eq!(
            provider
                .repair_with_probe(owner, &initial.id, &repair, "repair-one", |_| panic!())
                .unwrap()
                .configuration_revision,
            repaired.configuration_revision
        );
        assert_eq!(
            provider.profiles()[0].configuration_revision,
            latest.configuration_revision
        );
        // Simulate the prepared repair receipt surviving a crash before active-file replacement.
        fs::write(&active, &saved).unwrap();
        let prepared_path = repair_path(&directory, owner, "repair-two").unwrap();
        let mut prepared = read_repair(&prepared_path).unwrap();
        prepared.activated = false;
        fs::write(&prepared_path, serde_json::to_vec(&prepared).unwrap()).unwrap();
        drop(provider);
        let resumed = ProtonMailProvider::load(&root.path().join("profiles.json"))
            .unwrap()
            .with_setup(&directory, &status)
            .unwrap();
        assert_eq!(
            resumed.profiles()[0].configuration_revision,
            repaired.configuration_revision
        );
        assert_eq!(
            resumed
                .repair_with_probe(owner, &initial.id, &second, "repair-two", |_| panic!())
                .unwrap()
                .configuration_revision,
            latest.configuration_revision
        );
        drop(resumed);
        let restarted = ProtonMailProvider::load(&root.path().join("profiles.json"))
            .unwrap()
            .with_setup(&directory, &status)
            .unwrap();
        assert_eq!(
            restarted.profiles()[0].configuration_revision,
            latest.configuration_revision
        );
        assert_eq!(restarted.profiles()[0].id, initial.id);
    }
    #[test]
    fn operator_profile_revalidation_uses_a_managed_override_without_changing_its_id() {
        let (root, provider, directory, status) = fixture();
        drop(provider);
        let owner = UserId::new();
        let legacy = Profile {
            revision: collector_core::Revision(0),
            id: "operator-profile".into(),
            owner,
            label: "Operator mail".into(),
            username: command().username,
            password: command().password,
            certificate_sha256: "a".repeat(64),
            port: 1143,
            folders: vec!["INBOX".into()],
        };
        let path = root.path().join("profiles.json");
        let mut sibling = legacy.clone();
        sibling.id = "operator-sibling".into();
        fs::write(&path, serde_json::to_vec(&vec![legacy, sibling]).unwrap()).unwrap();
        let original = fs::read(&path).unwrap();
        let provider = ProtonMailProvider::load(&path)
            .unwrap()
            .with_setup(&directory, &status)
            .unwrap();
        let repair = RepairBridge {
            username: command().username,
            password: "new-private-password".into(),
            certificate_sha256: "a".repeat(64),
            expected_revision: collector_core::Revision(0),
        };
        provider
            .repair_with_probe(
                owner,
                "operator-profile",
                &repair,
                "operator-repair",
                |_| Ok(()),
            )
            .unwrap();
        provider
            .repair_with_probe(owner, "operator-sibling", &repair, "sibling-repair", |_| {
                Ok(())
            })
            .unwrap();
        assert_eq!(fs::read(&path).unwrap(), original);
        drop(provider);
        let restored = ProtonMailProvider::load(&path)
            .unwrap()
            .with_setup(&directory, &status)
            .unwrap();
        assert_eq!(restored.profiles().len(), 2);
        assert!(
            restored
                .profiles()
                .iter()
                .all(|p| p.configuration_revision == collector_core::Revision(1))
        );
        assert_eq!(restored.profiles()[0].id, "operator-profile");
        assert_eq!(
            restored.profiles()[0].configuration_revision,
            collector_core::Revision(1)
        );
    }
    #[test]
    fn an_unactivated_prepared_repair_cannot_replay_another_repair_success() {
        let (_root, provider, directory, _status) = fixture();
        let owner = UserId::new();
        let initial = provider
            .connect_with_probe(owner, &command(), "initial", |_| Ok(()))
            .unwrap();
        let active = directory.join(format!("{owner}.json"));
        let backup = directory.join(".active-backup");
        let first = RepairBridge {
            username: command().username,
            password: "first-password".into(),
            certificate_sha256: "a".repeat(64),
            expected_revision: initial.configuration_revision,
        };
        assert!(
            provider
                .repair_with_probe(owner, &initial.id, &first, "pending-A", |_| {
                    fs::rename(&active, &backup).unwrap();
                    fs::create_dir(&active).unwrap();
                    Ok(())
                })
                .is_err()
        );
        fs::remove_dir(&active).unwrap();
        fs::rename(&backup, &active).unwrap();
        let mut second = RepairBridge {
            username: first.username.clone(),
            password: "second-password".into(),
            certificate_sha256: first.certificate_sha256.clone(),
            expected_revision: first.expected_revision,
        };
        let saved = provider
            .repair_with_probe(owner, &initial.id, &second, "committed-B", |_| Ok(()))
            .unwrap();
        assert!(matches!(
            provider.repair_with_probe(owner, &initial.id, &first, "pending-A", |_| panic!()),
            Err(Error::Conflict)
        ));
        second.expected_revision = saved.configuration_revision;
        second.password = "third-password".into();
        provider
            .repair_with_probe(owner, &initial.id, &second, "committed-C", |_| Ok(()))
            .unwrap();
        assert!(matches!(
            provider.repair_with_probe(owner, &initial.id, &first, "pending-A", |_| panic!()),
            Err(Error::Conflict)
        ));
        assert_eq!(
            read_private(&active).unwrap().profile.password,
            "third-password"
        );
    }
    #[test]
    fn post_rename_failure_keeps_the_runtime_fence_aligned_with_visible_credentials() {
        let (_root, provider, directory, _status) = fixture();
        let owner = UserId::new();
        let initial = provider
            .connect_with_probe(owner, &command(), "initial", |_| Ok(()))
            .unwrap();
        let active = directory.join(format!("{owner}.json"));
        let mut replacement = read_private(&active).unwrap();
        replacement.profile.revision =
            collector_core::Revision(initial.configuration_revision.0 + 1);
        replacement.profile.password = "changed-password".into();
        assert!(
            activate(&directory, &active, &replacement, || {
                provider.profiles.lock().unwrap()[0] = replacement.profile.clone();
                Err(Error::Unavailable(
                    "injected failure after rename before directory sync".into(),
                ))
            })
            .is_err()
        );
        assert!(matches!(
            provider.with_configuration(
                &initial.id,
                initial.configuration_revision,
                &mut || panic!()
            ),
            Err(Error::Conflict)
        ));
        assert_eq!(
            provider.profiles()[0].configuration_revision,
            read_private(&active).unwrap().profile.revision
        );
    }
}
