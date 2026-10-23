//! Offline recovery policy; filesystem and SQLite mechanisms belong to the adapter.
use collector_core::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::sync::Arc;

/// Local operator recovery only. The adapter holds both normal server locks;
/// grant policy and effects must be evaluated in the same database transaction.
pub trait LocalOwnershipRecoveryPort: Send + Sync {
    fn unowned_collections(
        &self,
        after: Option<CollectionId>,
        limit: u16,
    ) -> Result<Page<Collection>>;
    fn recover_collection_owner(
        &self,
        collection: CollectionId,
        username: &str,
    ) -> Result<crate::GrantView>;
}

pub struct LocalOwnershipRecoveryService {
    port: Arc<dyn LocalOwnershipRecoveryPort>,
}

impl LocalOwnershipRecoveryService {
    pub fn new<P: LocalOwnershipRecoveryPort + 'static>(port: Arc<P>) -> Self {
        Self { port }
    }

    pub fn unowned_collections(
        &self,
        after: Option<CollectionId>,
        limit: u16,
    ) -> Result<Page<Collection>> {
        if !(1..=100).contains(&limit) {
            return Err(Error::Invalid(
                "collection page must contain 1–100 rows".into(),
            ));
        }
        self.port.unowned_collections(after, limit)
    }

    pub fn recover_owner(
        &self,
        collection: CollectionId,
        username: &str,
    ) -> Result<crate::GrantView> {
        if username.trim().is_empty() || username.len() > 256 {
            return Err(Error::Invalid("select an existing sign-in username".into()));
        }
        self.port.recover_collection_owner(collection, username)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RestoreScope {
    Full,
    ArchiveOnly,
}

impl RestoreScope {
    pub fn includes(self, path: &str) -> bool {
        path.starts_with("archive/") || (self == Self::Full && path.starts_with("state/"))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BackupFile {
    pub path: String,
    pub bytes: ByteLength,
    pub digest: Digest,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BackupInventory {
    pub version: u32,
    pub archive_root: String,
    pub state_root: String,
    pub files: Vec<BackupFile>,
    // Existing conversion backups recorded files but not empty directories.
    #[serde(default)]
    pub directories: Vec<String>,
    pub private_digest: String,
}

impl BackupInventory {
    pub fn validate(&self) -> Result<()> {
        let invalid = || Error::Invalid("invalid or unsupported backup inventory".into());
        if self.version != 1 || self.archive_root.is_empty() || self.state_root.is_empty() {
            return Err(invalid());
        }
        let safe = |path: &str| {
            !path.contains('\\')
                && path
                    .split('/')
                    .all(|part| !matches!(part, "" | "." | "..") && !part.contains(':'))
        };
        let mut names = BTreeSet::new();
        let mut total = 0u64;
        for file in &self.files {
            if !safe(&file.path)
                || !(file.path == "collector.toml"
                    || file.path.starts_with("archive/")
                    || file.path.starts_with("state/"))
                || !names.insert(file.path.as_str())
                || file.digest.algorithm != HashAlgorithm::Sha256
                || file.digest.hex.len() != 64
                || !file.digest.hex.bytes().all(|c| c.is_ascii_hexdigit())
            {
                return Err(invalid());
            }
            total = total.checked_add(file.bytes.0).ok_or_else(invalid)?;
        }
        for required in [
            "collector.toml",
            "state/private.sqlite",
            "archive/.writer.lock",
            "state/.private.lock",
        ] {
            if !names.contains(required) {
                return Err(invalid());
            }
        }
        for directory in &self.directories {
            if !safe(directory)
                || !(matches!(directory.as_str(), "archive" | "state")
                    || directory.starts_with("archive/")
                    || directory.starts_with("state/"))
                || !names.insert(directory.as_str())
            {
                return Err(invalid());
            }
        }
        Ok(())
    }
}
