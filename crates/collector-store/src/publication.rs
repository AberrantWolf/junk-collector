//! Durable archive-publication journal primitives.
//!
//! Journals deliberately contain no users, grants, credentials, or other private
//! policy state. Application-owned publication plans are adopted by later P01
//! children; this module owns only crash-recoverable filesystem mechanisms.
#![allow(
    dead_code,
    reason = "P01.2 provides the kernel adopted by the P01.3-P01.5 writers"
)]

use crate::{storage, sync_directory};
use collector_core::{CollectionId, Digest, Error, Result, Revision, TransactionId};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    fs::{self, OpenOptions},
    io::Write,
    path::{Component, Path, PathBuf},
};

const JOURNAL_SCHEMA: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PublicationPhase {
    Prepared,
    BytesStaged,
    PayloadPublished,
    RevisionCommitted,
    ProjectionApplied,
    Completed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PublicationFaultPoint {
    Prepared,
    BytesStaged,
    PayloadPublished,
    RevisionCommitted,
    ProjectionApplied,
    Completed,
}

pub trait PublicationFaultInjector: Send + Sync {
    fn check(&self, point: PublicationFaultPoint) -> Result<()>;
}

pub struct NoPublicationFault;

impl PublicationFaultInjector for NoPublicationFault {
    fn check(&self, _point: PublicationFaultPoint) -> Result<()> {
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    Payload,
    Revision,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct JournalArtifact {
    pub kind: ArtifactKind,
    pub staged_path: String,
    pub published_path: String,
    pub sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PublicationJournal {
    pub schema_version: u32,
    pub transaction_id: TransactionId,
    pub entity_type: String,
    pub entity_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collection_id: Option<CollectionId>,
    pub expected_revision: Revision,
    pub target_revision: Revision,
    pub phase: PublicationPhase,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artifacts: Vec<JournalArtifact>,
}

impl PublicationJournal {
    pub fn new(
        transaction_id: TransactionId,
        entity_type: impl Into<String>,
        entity_id: impl Into<String>,
        collection_id: Option<CollectionId>,
        expected_revision: Revision,
        target_revision: Revision,
        artifacts: Vec<JournalArtifact>,
    ) -> Result<Self> {
        let journal = Self {
            schema_version: JOURNAL_SCHEMA,
            transaction_id,
            entity_type: entity_type.into(),
            entity_id: entity_id.into(),
            collection_id,
            expected_revision,
            target_revision,
            phase: PublicationPhase::Prepared,
            artifacts,
        };
        journal.validate()?;
        Ok(journal)
    }

    fn validate(&self) -> Result<()> {
        if self.schema_version != JOURNAL_SCHEMA {
            return Err(Error::Storage(format!(
                "unsupported publication journal schema {}",
                self.schema_version
            )));
        }
        if self.entity_type.is_empty()
            || self.entity_id.is_empty()
            || self.entity_type.chars().any(char::is_control)
            || self.entity_id.chars().any(char::is_control)
        {
            return Err(Error::Storage(
                "publication journal has an invalid entity identity".into(),
            ));
        }
        if self.target_revision.0 != self.expected_revision.0.saturating_add(1) {
            return Err(Error::Storage(
                "publication journal revisions are not adjacent".into(),
            ));
        }
        if self.artifacts.is_empty() {
            return Err(Error::Storage(
                "publication journal must describe at least one artifact".into(),
            ));
        }
        let mut paths = BTreeSet::new();
        for artifact in &self.artifacts {
            validate_relative_path(&artifact.staged_path)?;
            validate_relative_path(&artifact.published_path)?;
            if artifact.sha256.len() != 64
                || !artifact
                    .sha256
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
            {
                return Err(Error::Storage(
                    "publication journal has an invalid SHA-256 digest".into(),
                ));
            }
            if !paths.insert((&artifact.staged_path, &artifact.published_path)) {
                return Err(Error::Storage(
                    "publication journal repeats an artifact path".into(),
                ));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct RecoveryReport {
    pub pending: Vec<TransactionId>,
    pub committed: Vec<TransactionId>,
    pub completed: Vec<TransactionId>,
    pub unknown_staging: Vec<String>,
}

pub(crate) struct PublicationJournalStore<'a> {
    archive: &'a Path,
}

impl<'a> PublicationJournalStore<'a> {
    pub(crate) fn new(archive: &'a Path) -> Self {
        Self { archive }
    }

    pub(crate) fn initialize(&self) -> Result<()> {
        fs::create_dir_all(self.archive.join(".transactions")).map_err(storage)?;
        fs::create_dir_all(self.archive.join(".staging")).map_err(storage)?;
        sync_directory(&self.archive.join(".transactions"))?;
        sync_directory(self.archive)?;
        Ok(())
    }

    pub(crate) fn phase(&self, transaction_id: TransactionId) -> Result<Option<PublicationPhase>> {
        let path = self.journal_path(transaction_id);
        if !path.exists() {
            return Ok(None);
        }
        Ok(Some(self.read(&path)?.phase))
    }

    pub(crate) fn write_new(&self, journal: &PublicationJournal) -> Result<()> {
        journal.validate()?;
        let path = self.journal_path(journal.transaction_id);
        if path.exists() {
            let existing = self.read(&path)?;
            return if existing == *journal {
                Ok(())
            } else {
                Err(Error::Storage(
                    "publication transaction identity conflicts with existing journal".into(),
                ))
            };
        }
        self.persist(journal)
    }

    pub(crate) fn advance(
        &self,
        transaction_id: TransactionId,
        phase: PublicationPhase,
    ) -> Result<PublicationJournal> {
        let path = self.journal_path(transaction_id);
        let mut journal = self.read(&path)?;
        if phase < journal.phase {
            return Err(Error::Storage(
                "publication journal phase cannot move backwards".into(),
            ));
        }
        if phase == journal.phase {
            return Ok(journal);
        }
        journal.phase = phase;
        self.persist(&journal)?;
        Ok(journal)
    }

    pub(crate) fn inspect_and_reconcile(&self) -> Result<RecoveryReport> {
        let mut report = RecoveryReport::default();
        let mut known = BTreeSet::new();
        for entry in sorted_entries(&self.archive.join(".transactions"))? {
            if !entry.file_type().map_err(storage)?.is_file() {
                return Err(Error::Storage(
                    "unknown entry in publication journal directory".into(),
                ));
            }
            let path = entry.path();
            // A journal replacement is committed only by rename. Its temporary
            // file may be truncated at any byte; the previous .json stays authoritative.
            if let Some(name) = path.file_name().and_then(|name| name.to_str())
                && let Some(id) = name.strip_suffix(".json.tmp")
                && id.parse::<TransactionId>().is_ok()
            {
                fs::remove_file(&path).map_err(storage)?;
                sync_directory(&self.archive.join(".transactions"))?;
                continue;
            }
            if path.extension().and_then(|value| value.to_str()) != Some("json") {
                return Err(Error::Storage(
                    "incomplete or unknown publication journal file".into(),
                ));
            }
            let journal = self.read(&path)?;
            let expected_name = format!("{}.json", journal.transaction_id);
            if entry.file_name() != expected_name.as_str() {
                return Err(Error::Storage(
                    "publication journal filename does not match its transaction".into(),
                ));
            }
            known.insert(journal.transaction_id.to_string());
            let mut revision_published = false;
            for artifact in &journal.artifacts {
                let staged = self.resolve(&artifact.staged_path)?;
                let published = self.resolve(&artifact.published_path)?;
                if staged.exists() && journal.phase >= PublicationPhase::BytesStaged {
                    verify_file(&staged, &artifact.sha256)?;
                }
                if published.exists() {
                    verify_file(&published, &artifact.sha256)?;
                    if artifact.kind == ArtifactKind::Revision {
                        revision_published = true;
                    }
                }
                if journal.phase >= PublicationPhase::BytesStaged
                    && !staged.exists()
                    && !published.exists()
                {
                    return Err(Error::Storage(format!(
                        "known publication transaction {} is missing a recorded artifact",
                        journal.transaction_id
                    )));
                }
            }
            if journal.phase >= PublicationPhase::RevisionCommitted && !revision_published {
                return Err(Error::Storage(format!(
                    "known publication transaction {} records a commit without committed revision bytes",
                    journal.transaction_id
                )));
            }
            if journal.phase == PublicationPhase::Completed {
                report.completed.push(journal.transaction_id);
            } else if revision_published {
                report.committed.push(journal.transaction_id);
            } else {
                report.pending.push(journal.transaction_id);
            }
        }
        for entry in sorted_entries(&self.archive.join(".staging"))? {
            let name = entry.file_name().to_string_lossy().into_owned();
            if !known.contains(&name) {
                report.unknown_staging.push(name);
            }
        }
        for transaction_id in &report.completed {
            self.remove_completed(*transaction_id)?;
        }
        Ok(report)
    }

    pub(crate) fn finish(&self, transaction_id: TransactionId) -> Result<()> {
        self.advance(transaction_id, PublicationPhase::Completed)?;
        self.remove_completed(transaction_id)
    }

    pub(crate) fn abort(&self, transaction_id: TransactionId) -> Result<()> {
        let journal = self.read(&self.journal_path(transaction_id))?;
        for artifact in &journal.artifacts {
            let published = self.resolve(&artifact.published_path)?;
            if published.exists() {
                if artifact.kind == ArtifactKind::Revision {
                    return Err(Error::Storage(
                        "cannot abort a publication with committed revision bytes".into(),
                    ));
                }
                fs::remove_file(&published).map_err(storage)?;
                self.remove_empty_parents(
                    published.parent().ok_or_else(|| {
                        Error::Storage("publication artifact has no parent".into())
                    })?,
                )?;
            }
        }
        let stage = self
            .archive
            .join(".staging")
            .join(transaction_id.to_string());
        if stage.exists() {
            fs::remove_dir_all(stage).map_err(storage)?;
            sync_directory(&self.archive.join(".staging"))?;
        }
        fs::remove_file(self.journal_path(transaction_id)).map_err(storage)?;
        sync_directory(&self.archive.join(".transactions"))
    }

    fn remove_empty_parents(&self, start: &Path) -> Result<()> {
        let mut current = Some(start);
        while let Some(directory) = current {
            if directory == self.archive || directory.join("collection.toml").exists() {
                break;
            }
            if fs::read_dir(directory).map_err(storage)?.next().is_some() {
                break;
            }
            let parent = directory
                .parent()
                .ok_or_else(|| Error::Storage("publication path has no archive parent".into()))?;
            fs::remove_dir(directory).map_err(storage)?;
            sync_directory(parent)?;
            current = Some(parent);
        }
        Ok(())
    }

    fn remove_completed(&self, transaction_id: TransactionId) -> Result<()> {
        let stage = self
            .archive
            .join(".staging")
            .join(transaction_id.to_string());
        if stage.exists() {
            fs::remove_dir_all(&stage).map_err(storage)?;
            sync_directory(&self.archive.join(".staging"))?;
        }
        let path = self.journal_path(transaction_id);
        fs::remove_file(path).map_err(storage)?;
        sync_directory(&self.archive.join(".transactions"))
    }

    fn resolve(&self, relative: &str) -> Result<PathBuf> {
        validate_relative_path(relative)?;
        Ok(self.archive.join(relative))
    }

    fn journal_path(&self, transaction_id: TransactionId) -> PathBuf {
        self.archive
            .join(".transactions")
            .join(format!("{transaction_id}.json"))
    }

    fn read(&self, path: &Path) -> Result<PublicationJournal> {
        let journal: PublicationJournal =
            serde_json::from_slice(&fs::read(path).map_err(storage)?).map_err(storage)?;
        journal.validate()?;
        Ok(journal)
    }

    fn persist(&self, journal: &PublicationJournal) -> Result<()> {
        journal.validate()?;
        let directory = self.archive.join(".transactions");
        let destination = self.journal_path(journal.transaction_id);
        let temporary = directory.join(format!("{}.json.tmp", journal.transaction_id));
        let bytes = serde_json::to_vec_pretty(journal).map_err(storage)?;
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&temporary)
            .map_err(storage)?;
        file.write_all(&bytes).map_err(storage)?;
        file.sync_all().map_err(storage)?;
        drop(file);
        fs::rename(&temporary, &destination).map_err(storage)?;
        sync_directory(&directory)
    }
}

fn validate_relative_path(value: &str) -> Result<()> {
    if value.is_empty() || value.contains('\\') || value.chars().any(char::is_control) {
        return Err(Error::Storage(
            "publication journal path must be a safe archive-relative path".into(),
        ));
    }
    let path = Path::new(value);
    if path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(Error::Storage(
            "publication journal path must be a safe archive-relative path".into(),
        ));
    }
    Ok(())
}

fn verify_file(path: &Path, expected: &str) -> Result<()> {
    if !path.is_file() {
        return Err(Error::Storage(format!(
            "publication artifact is not a regular file: {}",
            path.display()
        )));
    }
    let actual = Digest::sha256(&fs::read(path).map_err(storage)?).hex;
    if actual != expected {
        return Err(Error::Storage(format!(
            "publication artifact digest mismatch: {}",
            path.display()
        )));
    }
    Ok(())
}

fn sorted_entries(directory: &Path) -> Result<Vec<fs::DirEntry>> {
    let mut entries = fs::read_dir(directory)
        .map_err(storage)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(storage)?;
    entries.sort_by_key(fs::DirEntry::file_name);
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn artifact(transaction: TransactionId, bytes: &[u8]) -> JournalArtifact {
        JournalArtifact {
            kind: ArtifactKind::Revision,
            staged_path: format!(".staging/{transaction}/record.json"),
            published_path: format!("collections/test/revisions/{transaction}/record.json"),
            sha256: Digest::sha256(bytes).hex,
        }
    }

    fn setup() -> (tempfile::TempDir, PublicationJournalStore<'static>) {
        let root = tempfile::tempdir().unwrap();
        let archive: &'static Path = Box::leak(root.path().to_path_buf().into_boxed_path());
        let store = PublicationJournalStore::new(archive);
        store.initialize().unwrap();
        (root, store)
    }

    #[test]
    fn paths_and_phases_are_validated_and_persisted() {
        let (_root, store) = setup();
        let transaction = TransactionId::new();
        let mut invalid = artifact(transaction, b"record");
        invalid.staged_path = "../outside".into();
        assert!(
            PublicationJournal::new(
                transaction,
                "item",
                "one",
                None,
                Revision(0),
                Revision(1),
                vec![invalid]
            )
            .is_err()
        );

        let journal = PublicationJournal::new(
            transaction,
            "item",
            "one",
            None,
            Revision(0),
            Revision(1),
            vec![artifact(transaction, b"record")],
        )
        .unwrap();
        store.write_new(&journal).unwrap();
        assert_eq!(
            store
                .advance(transaction, PublicationPhase::Prepared)
                .unwrap()
                .phase,
            PublicationPhase::Prepared
        );
        assert_eq!(
            store
                .advance(transaction, PublicationPhase::BytesStaged)
                .unwrap()
                .phase,
            PublicationPhase::BytesStaged
        );
        assert!(
            store
                .advance(transaction, PublicationPhase::Prepared)
                .is_err()
        );
    }

    #[test]
    fn actual_files_override_a_lagging_phase_and_unknown_staging_is_retained() {
        let (root, store) = setup();
        let transaction = TransactionId::new();
        let bytes = b"committed revision";
        let artifact = artifact(transaction, bytes);
        let journal = PublicationJournal::new(
            transaction,
            "item",
            "one",
            None,
            Revision(0),
            Revision(1),
            vec![artifact.clone()],
        )
        .unwrap();
        store.write_new(&journal).unwrap();
        let published = root.path().join(&artifact.published_path);
        fs::create_dir_all(published.parent().unwrap()).unwrap();
        fs::write(&published, bytes).unwrap();
        fs::create_dir(root.path().join(".staging/unknown-entry")).unwrap();

        let report = store.inspect_and_reconcile().unwrap();
        assert_eq!(report.committed, vec![transaction]);
        assert_eq!(report.unknown_staging, vec!["unknown-entry"]);
        assert!(root.path().join(".staging/unknown-entry").exists());
    }

    #[test]
    fn corrupt_known_artifact_fails_closed() {
        let (root, store) = setup();
        let transaction = TransactionId::new();
        let artifact = artifact(transaction, b"expected");
        let journal = PublicationJournal::new(
            transaction,
            "item",
            "one",
            None,
            Revision(0),
            Revision(1),
            vec![artifact.clone()],
        )
        .unwrap();
        store.write_new(&journal).unwrap();
        store
            .advance(transaction, PublicationPhase::BytesStaged)
            .unwrap();
        let staged = root.path().join(&artifact.staged_path);
        fs::create_dir_all(staged.parent().unwrap()).unwrap();
        fs::write(staged, b"wrong").unwrap();
        assert!(store.inspect_and_reconcile().is_err());
    }

    #[test]
    fn completion_is_durable_and_removes_known_scratch() {
        let (root, store) = setup();
        let transaction = TransactionId::new();
        let journal = PublicationJournal::new(
            transaction,
            "item",
            "one",
            None,
            Revision(0),
            Revision(1),
            vec![artifact(transaction, b"record")],
        )
        .unwrap();
        store.write_new(&journal).unwrap();
        let staged = root
            .path()
            .join(".staging")
            .join(transaction.to_string())
            .join("record.json");
        fs::create_dir_all(staged.parent().unwrap()).unwrap();
        fs::write(&staged, b"record").unwrap();
        let published = root
            .path()
            .join("collections/test/revisions")
            .join(transaction.to_string())
            .join("record.json");
        fs::create_dir_all(published.parent().unwrap()).unwrap();
        fs::rename(staged, published).unwrap();
        store
            .advance(transaction, PublicationPhase::RevisionCommitted)
            .unwrap();
        store.finish(transaction).unwrap();
        assert!(store.inspect_and_reconcile().unwrap().completed.is_empty());
        assert!(
            !root
                .path()
                .join(".staging")
                .join(transaction.to_string())
                .exists()
        );
    }

    struct FailAt(PublicationFaultPoint);

    impl PublicationFaultInjector for FailAt {
        fn check(&self, point: PublicationFaultPoint) -> Result<()> {
            if point == self.0 {
                Err(Error::Unavailable(format!("injected fault at {point:?}")))
            } else {
                Ok(())
            }
        }
    }

    fn faulted_workflow(root: &Path, fault: &dyn PublicationFaultInjector) -> TransactionId {
        let store = PublicationJournalStore::new(root);
        let transaction = TransactionId::new();
        let payload = b"payload";
        let revision = b"revision";
        let artifacts = vec![
            JournalArtifact {
                kind: ArtifactKind::Payload,
                staged_path: format!(".staging/{transaction}/payload"),
                published_path: format!("collections/test/payloads/{transaction}"),
                sha256: Digest::sha256(payload).hex,
            },
            artifact(transaction, revision),
        ];
        let journal = PublicationJournal::new(
            transaction,
            "capture",
            "fixture",
            None,
            Revision(0),
            Revision(1),
            artifacts.clone(),
        )
        .unwrap();
        store.write_new(&journal).unwrap();
        if fault.check(PublicationFaultPoint::Prepared).is_err() {
            return transaction;
        }
        for (artifact, bytes) in artifacts
            .iter()
            .zip([payload.as_slice(), revision.as_slice()])
        {
            let path = root.join(&artifact.staged_path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, bytes).unwrap();
        }
        store
            .advance(transaction, PublicationPhase::BytesStaged)
            .unwrap();
        if fault.check(PublicationFaultPoint::BytesStaged).is_err() {
            return transaction;
        }
        let payload_destination = root.join(&artifacts[0].published_path);
        fs::create_dir_all(payload_destination.parent().unwrap()).unwrap();
        fs::rename(root.join(&artifacts[0].staged_path), payload_destination).unwrap();
        store
            .advance(transaction, PublicationPhase::PayloadPublished)
            .unwrap();
        if fault
            .check(PublicationFaultPoint::PayloadPublished)
            .is_err()
        {
            return transaction;
        }
        let revision_destination = root.join(&artifacts[1].published_path);
        fs::create_dir_all(revision_destination.parent().unwrap()).unwrap();
        fs::rename(root.join(&artifacts[1].staged_path), revision_destination).unwrap();
        store
            .advance(transaction, PublicationPhase::RevisionCommitted)
            .unwrap();
        if fault
            .check(PublicationFaultPoint::RevisionCommitted)
            .is_err()
        {
            return transaction;
        }
        store
            .advance(transaction, PublicationPhase::ProjectionApplied)
            .unwrap();
        if fault
            .check(PublicationFaultPoint::ProjectionApplied)
            .is_err()
        {
            return transaction;
        }
        store
            .advance(transaction, PublicationPhase::Completed)
            .unwrap();
        let _ = fault.check(PublicationFaultPoint::Completed);
        transaction
    }

    #[test]
    fn controlled_fault_at_every_phase_leaves_classifiable_real_files() {
        let points = [
            PublicationFaultPoint::Prepared,
            PublicationFaultPoint::BytesStaged,
            PublicationFaultPoint::PayloadPublished,
            PublicationFaultPoint::RevisionCommitted,
            PublicationFaultPoint::ProjectionApplied,
            PublicationFaultPoint::Completed,
        ];
        for point in points {
            let root = tempfile::tempdir().unwrap();
            let store = PublicationJournalStore::new(root.path());
            store.initialize().unwrap();
            let transaction = faulted_workflow(root.path(), &FailAt(point));
            let report = store.inspect_and_reconcile().unwrap();
            match point {
                PublicationFaultPoint::Prepared
                | PublicationFaultPoint::BytesStaged
                | PublicationFaultPoint::PayloadPublished => {
                    assert_eq!(report.pending, vec![transaction]);
                }
                PublicationFaultPoint::RevisionCommitted
                | PublicationFaultPoint::ProjectionApplied => {
                    assert_eq!(report.committed, vec![transaction]);
                }
                PublicationFaultPoint::Completed => {
                    assert_eq!(report.completed, vec![transaction]);
                    assert!(
                        !root
                            .path()
                            .join(".transactions")
                            .join(format!("{transaction}.json"))
                            .exists()
                    );
                }
            }
        }
    }
}
