//! Portable full-archive inventory policy. Local adapters own copying and locking.
use crate::BackupFile;
use collector_core::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveExportInventory {
    pub version: u32,
    pub files: Vec<BackupFile>,
    pub directories: Vec<String>,
}

impl ArchiveExportInventory {
    /// Unknown archive records are portable too; only known operational scratch
    /// and the local writer lock are excluded. Nonempty scratch must be refused.
    pub fn includes(path: &str) -> bool {
        path == "archive"
            || path.strip_prefix("archive/").is_some_and(|local| {
                !matches!(
                    local.split('/').next(),
                    Some(".writer.lock" | ".staging" | ".transactions" | ".restore-in-progress")
                )
            })
    }

    pub fn validate(&self) -> Result<()> {
        let invalid = || Error::Invalid("invalid or unsupported archive export inventory".into());
        if self.version != 1 {
            return Err(invalid());
        }
        let safe = |path: &str| {
            Self::includes(path)
                && !path.contains('\\')
                && path.split('/').count() <= 256
                && path.split('/').all(|part| {
                    !matches!(part, "" | "." | "..")
                        && !part.contains(':')
                        && !part.chars().any(char::is_control)
                })
        };
        let mut names = BTreeSet::new();
        let mut total = 0u64;
        for file in &self.files {
            if file.path == "archive"
                || !safe(&file.path)
                || !names.insert(file.path.as_str())
                || file.digest.algorithm != HashAlgorithm::Sha256
                || file.digest.hex.len() != 64
                || !file.digest.hex.bytes().all(|c| c.is_ascii_hexdigit())
            {
                return Err(invalid());
            }
            total = total.checked_add(file.bytes.0).ok_or_else(invalid)?;
        }
        for directory in &self.directories {
            if !safe(directory) || !names.insert(directory.as_str()) {
                return Err(invalid());
            }
        }
        if !self.directories.iter().any(|path| path == "archive")
            || !self
                .directories
                .iter()
                .any(|path| path == "archive/collections")
        {
            return Err(invalid());
        }
        Ok(())
    }
}
