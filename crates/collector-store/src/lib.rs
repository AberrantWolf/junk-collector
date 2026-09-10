//! Portable immutable revisions plus local SQLite projections and private operational state.
mod archive_v3;
mod auth;
mod commands;
mod files;
mod jobs;
mod migration;
mod migration_v3;
mod publication;
use chrono::Utc;
use collector_app::{
    AccessPort, AccountView, AdministrationPort, AuthenticatedPrincipal, AuthenticationPort,
    Catalog, ContentPort, CredentialKind, EnrollmentPort, EnrollmentStart, EnrollmentView,
    GrantView, IdempotencyPort, ImportPort, JobClaim, JobPort, JobSpec, MediaResource,
    NativeCredentialIssue, NativeDeviceView, ReceiptDecision, SessionView, StoredPasskey,
};
pub use collector_app::{Upload, UploadFile};
use collector_core::*;
use fs2::FileExt;
pub use publication::{
    NoPublicationFault, PublicationFaultInjector, PublicationFaultPoint, RecoveryReport,
};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, MutexGuard},
};

pub(crate) fn storage(e: impl std::fmt::Display) -> Error {
    Error::Storage(e.to_string())
}
pub(crate) fn lock<T>(mutex: &Mutex<T>) -> Result<MutexGuard<'_, T>> {
    mutex
        .lock()
        .map_err(|_| Error::Unavailable("storage lock poisoned; restart required".into()))
}
#[derive(Clone)]
pub struct Store {
    pub(crate) archive: PathBuf,
    pub(crate) state: PathBuf,
    pub(crate) db: Arc<Mutex<Connection>>,
    pub(crate) writer: Arc<Mutex<()>>,
    pub(crate) uploads: Arc<Mutex<()>>,
    pub(crate) job_publication_gate: Arc<Mutex<()>>,
    recovery_report: RecoveryReport,
    receipt: Option<collector_app::CommandReceipt>,
    command_result: Option<serde_json::Value>,
    publication_fault: std::sync::Arc<dyn PublicationFaultInjector>,
    pub(crate) clock: Arc<dyn Clock>,
    _archive_lock: Arc<File>,
    _private_lock: Arc<File>,
}

pub trait Clock: Send + Sync {
    fn now(&self) -> chrono::DateTime<Utc>;
    fn monotonic_millis(&self) -> u64;
}

pub struct SystemClock {
    started: std::time::Instant,
}

impl Default for SystemClock {
    fn default() -> Self {
        Self {
            started: std::time::Instant::now(),
        }
    }
}

impl Clock for SystemClock {
    fn now(&self) -> chrono::DateTime<Utc> {
        Utc::now()
    }

    fn monotonic_millis(&self) -> u64 {
        self.started
            .elapsed()
            .as_millis()
            .try_into()
            .unwrap_or(u64::MAX)
    }
}
#[derive(Serialize, Deserialize)]
pub(crate) struct Manifest {
    schema_version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    entity_type: Option<String>,
    item_id: ItemId,
    collection_id: CollectionId,
    revision: Revision,
    record_sha256: String,
}
#[derive(Serialize, Deserialize)]
pub(crate) struct RevisionRecord {
    item: Item,
    evidence: Vec<Evidence>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    legacy: Option<LegacyRevisionLocator>,
}
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct LegacyRevisionLocator {
    relative_path: String,
    record_sha256: String,
}

#[derive(Clone, Serialize, Deserialize)]
struct CollectionCreateRecovery {
    collection: Collection,
    owner_user_id: UserId,
    relative_path: String,
}
impl Store {
    pub fn open(archive: impl AsRef<Path>, state: impl AsRef<Path>) -> Result<Self> {
        Self::open_with_dependencies(
            archive,
            state,
            Arc::new(NoPublicationFault),
            Arc::new(SystemClock::default()),
        )
    }

    #[doc(hidden)]
    pub fn open_with_publication_fault(
        archive: impl AsRef<Path>,
        state: impl AsRef<Path>,
        publication_fault: std::sync::Arc<dyn PublicationFaultInjector>,
    ) -> Result<Self> {
        Self::open_with_dependencies(
            archive,
            state,
            publication_fault,
            Arc::new(SystemClock::default()),
        )
    }

    #[doc(hidden)]
    pub fn open_with_clock(
        archive: impl AsRef<Path>,
        state: impl AsRef<Path>,
        clock: Arc<dyn Clock>,
    ) -> Result<Self> {
        Self::open_with_dependencies(archive, state, Arc::new(NoPublicationFault), clock)
    }

    fn open_with_dependencies(
        archive: impl AsRef<Path>,
        state: impl AsRef<Path>,
        publication_fault: Arc<dyn PublicationFaultInjector>,
        clock: Arc<dyn Clock>,
    ) -> Result<Self> {
        fs::create_dir_all(archive.as_ref()).map_err(storage)?;
        fs::create_dir_all(state.as_ref()).map_err(storage)?;
        let archive = fs::canonicalize(archive).map_err(storage)?;
        let state = fs::canonicalize(state).map_err(storage)?;
        if state.starts_with(&archive) || archive.starts_with(&state) {
            return Err(Error::Invalid(
                "archive and private state must be separate directories".into(),
            ));
        }
        private_directory(&state)?;
        let private_lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(state.join(".private.lock"))
            .map_err(storage)?;
        private_lock
            .try_lock_exclusive()
            .map_err(|_| Error::Unavailable("private state is already in use".into()))?;
        let archive_lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(archive.join(".writer.lock"))
            .map_err(storage)?;
        archive_lock
            .try_lock_exclusive()
            .map_err(|_| Error::Unavailable("archive already has a writer".into()))?;
        for dir in ["collections", ".staging", ".transactions"] {
            fs::create_dir_all(archive.join(dir)).map_err(storage)?;
        }
        let journal_store = publication::PublicationJournalStore::new(&archive);
        journal_store.initialize()?;
        let recovery_report = journal_store.inspect_and_reconcile()?;
        for transaction_id in &recovery_report.pending {
            journal_store.abort(*transaction_id)?;
        }
        fs::create_dir_all(state.join("uploads")).map_err(storage)?;
        let mut db = Connection::open(state.join("catalog.sqlite")).map_err(storage)?;
        db.busy_timeout(std::time::Duration::from_secs(5))
            .map_err(storage)?;
        db.pragma_update(None, "journal_mode", "WAL")
            .map_err(storage)?;
        db.pragma_update(None, "foreign_keys", true)
            .map_err(storage)?;
        db.execute(
            "ATTACH DATABASE ? AS private",
            [state.join("private.sqlite").to_string_lossy().as_ref()],
        )
        .map_err(storage)?;
        migrate_databases(&mut db)?;
        reconcile_collection_creations(&mut db, &archive)?;
        commands::recover(&mut db, &archive, &state)?;
        migration::migrate_schema_one(&archive)?;
        migration_v3::migrate_schema_two(&archive)?;
        let store = Self {
            archive,
            state,
            db: Arc::new(Mutex::new(db)),
            writer: Arc::new(Mutex::new(())),
            uploads: Arc::new(Mutex::new(())),
            job_publication_gate: Arc::new(Mutex::new(())),
            recovery_report,
            receipt: None,
            command_result: None,
            publication_fault,
            clock,
            _archive_lock: Arc::new(archive_lock),
            _private_lock: Arc::new(private_lock),
        };
        store.reindex()?;
        store.recover_private_publications()?;
        store.reconcile_job_publications()?;
        let journal_store = publication::PublicationJournalStore::new(&store.archive);
        for transaction_id in &store.recovery_report.committed {
            journal_store.advance(
                *transaction_id,
                publication::PublicationPhase::ProjectionApplied,
            )?;
            journal_store.finish(*transaction_id)?;
        }
        Ok(store)
    }
    /// Startup's read-only classification of durable publication state.
    /// Unknown staging is retained for explicit operator inspection.
    pub fn recovery_report(&self) -> &RecoveryReport {
        &self.recovery_report
    }
    pub fn server_id(&self) -> Result<ServerId> {
        let db = lock(&self.db)?;
        let value: Option<String> = db
            .query_row(
                "SELECT value FROM private.settings WHERE key='server_id'",
                [],
                |r| r.get(0),
            )
            .optional()
            .map_err(storage)?;
        if let Some(value) = value {
            return value.parse().map_err(storage);
        }
        let id = ServerId::new();
        db.execute(
            "INSERT INTO private.settings VALUES ('server_id',?)",
            [id.to_string()],
        )
        .map_err(storage)?;
        Ok(id)
    }
    /// Scan only committed metadata revisions, never payload bytes. No grants are reconstructed.
    pub fn reindex(&self) -> Result<()> {
        let _writer = lock(&self.writer)?;
        let mut collections = Vec::new();
        let mut records = Vec::new();
        let mut works = Vec::new();
        let mut editions = Vec::new();
        let mut captures = Vec::new();
        let mut capture_locators = Vec::new();
        let mut assets_v3 = Vec::new();
        let mut evidence_v3 = Vec::new();
        for dir in directories(&self.archive.join("collections"))? {
            let path = dir.join("collection.toml");
            if !path.exists() {
                continue;
            }
            let collection: Collection =
                toml::from_str(&fs::read_to_string(&path).map_err(storage)?).map_err(storage)?;
            collections.push((collection.clone(), dir.clone()));
            assets_v3.extend(migration_v3::read_assets(&dir, collection.id)?);
            evidence_v3.extend(migration_v3::read_evidence(&dir, collection.id)?);
            works.extend(read_latest_records::<Work>(
                &dir.join("works"),
                collection.id,
            )?);
            editions.extend(read_latest_records::<Edition>(
                &dir.join("editions"),
                collection.id,
            )?);

            let items_dir = dir.join("items");
            if !items_dir.exists() {
                continue;
            }
            for item_dir in directories(&items_dir)? {
                let versions_dir = item_dir.join("revisions");
                if !versions_dir.exists() {
                    continue;
                }
                let mut versions = directories(&versions_dir)?;
                versions.sort();
                if let Some(version) = versions.last() {
                    if version.join("manifest.v3.toml").exists() {
                        let (item, references) = migration_v3::read_item(version, collection.id)?;
                        let referenced = references
                            .iter()
                            .map(|reference| (reference.id, reference.sha256.as_str()))
                            .collect::<std::collections::BTreeMap<_, _>>();
                        for reference in &references {
                            let evidence = evidence_v3
                                .iter()
                                .find(|evidence| {
                                    evidence.id == reference.id
                                        && evidence.collection_id == collection.id
                                })
                                .ok_or_else(|| {
                                    Error::Storage(
                                        "schema-3 Item references missing Evidence".into(),
                                    )
                                })?;
                            let bytes = serde_json::to_vec_pretty(evidence).map_err(storage)?;
                            if reference.sha256 != Digest::sha256(&bytes).hex {
                                return Err(Error::Storage(
                                    "schema-3 Item Evidence reference digest mismatch".into(),
                                ));
                            }
                        }
                        for evidence in evidence_v3
                            .iter()
                            .filter(|evidence| evidence.item_id == Some(item.id))
                        {
                            if !referenced.contains_key(&evidence.id) {
                                return Err(Error::Storage(
                                    "schema-3 Item omits associated Evidence reference".into(),
                                ));
                            }
                        }
                        records.push((
                            RevisionRecord {
                                item: item.into_item(vec![], vec![]),
                                evidence: vec![],
                                legacy: None,
                            },
                            item_dir,
                        ));
                        continue;
                    }
                    let migrated_manifest = version.join("manifest.v2.toml");
                    let (manifest_path, record_path) = if migrated_manifest.exists() {
                        (migrated_manifest, version.join("record.v2.json"))
                    } else {
                        (version.join("manifest.toml"), version.join("record.json"))
                    };
                    let manifest: Manifest =
                        toml::from_str(&fs::read_to_string(manifest_path).map_err(storage)?)
                            .map_err(storage)?;
                    if manifest.schema_version != 2 {
                        return Err(Error::Unavailable(
                            "unsupported archive revision schema".into(),
                        ));
                    }
                    let bytes = fs::read(record_path).map_err(storage)?;
                    if Digest::sha256(&bytes).hex != manifest.record_sha256 {
                        return Err(Error::Storage("archive metadata digest mismatch; restore or explicitly repair before writing".into()));
                    }
                    let record: RevisionRecord = serde_json::from_slice(&bytes).map_err(storage)?;
                    record.item.validate()?;
                    if record.item.id != manifest.item_id
                        || record.item.revision != manifest.revision
                        || record.item.collection_id != collection.id
                        || manifest.collection_id != collection.id
                    {
                        return Err(Error::Storage("archive identity mismatch".into()));
                    }
                    migration::emit_capture_records(&dir, &record.item)?;
                    records.push((record, item_dir));
                }
            }
        }
        // Compatibility snapshots may finish an interrupted item/capture publication,
        // but only standalone records decide the current capture state.
        for (collection, dir) in &collections {
            for record in migration_v3::read_captures(dir, collection.id)? {
                if let Some(locator) = record.legacy_root {
                    capture_locators.push((record.capture.id, locator));
                }
                captures.push(record.capture);
            }
        }
        for (record, _) in &mut records {
            record.item.captures = captures
                .iter()
                .filter(|capture| {
                    capture.collection_id == record.item.collection_id
                        && capture.association.item_id == Some(record.item.id)
                })
                .cloned()
                .collect();
            let mut item_assets = std::collections::BTreeMap::new();
            for asset in assets_v3
                .iter()
                .filter(|asset| !asset.deleted && asset_belongs_to_item(&asset.asset, &record.item))
                .map(|asset| asset.asset.clone())
                .chain(record.item.assets.clone())
            {
                item_assets.insert(asset.id, asset);
            }
            record.item.assets = item_assets.into_values().collect();
        }
        let mut graphs = Vec::<CollectionGraph>::new();
        for (collection, _) in &collections {
            let items = records
                .iter()
                .filter(|(record, _)| record.item.collection_id == collection.id)
                .map(|(record, _)| record.item.clone())
                .collect::<Vec<_>>();
            let graph = CollectionGraph {
                works: works
                    .iter()
                    .filter(|work| work.collection_id == collection.id)
                    .cloned()
                    .collect(),
                editions: editions
                    .iter()
                    .filter(|edition| edition.collection_id == collection.id)
                    .cloned()
                    .collect(),
                captures: captures
                    .iter()
                    .filter(|capture| capture.collection_id == collection.id)
                    .cloned()
                    .collect(),
                assets: {
                    let mut values = std::collections::BTreeMap::new();
                    for asset in assets_v3
                        .iter()
                        .filter(|asset| asset.collection_id == collection.id && !asset.deleted)
                        .map(|asset| asset.asset.clone())
                        .chain(items.iter().flat_map(|item| item.assets.clone()))
                    {
                        values.insert(asset.id, asset);
                    }
                    values.into_values().collect()
                },
                items,
            };
            graph.validate(collection.id)?;
            for previous in &graphs {
                graph.validate_disjoint(previous)?;
            }
            graphs.push(graph);
        }
        let mut db = lock(&self.db)?;
        let tx = db.transaction().map_err(storage)?;
        tx.execute_batch(
            "DELETE FROM collections; DELETE FROM items; DELETE FROM search; DELETE FROM evidence; DELETE FROM assets; DELETE FROM works; DELETE FROM editions; DELETE FROM captures; DELETE FROM components; DELETE FROM legacy_payload_locators;",
        )
        .map_err(storage)?;
        for (collection, path) in collections {
            tx.execute(
                "INSERT INTO collections VALUES (?,?,?)",
                params![
                    collection.id.to_string(),
                    serde_json::to_string(&collection).map_err(storage)?,
                    relative(&self.archive, &path)?
                ],
            )
            .map_err(storage)?;
        }
        for work in works {
            tx.execute(
                "INSERT INTO works VALUES (?,?,?,?)",
                params![
                    work.id.to_string(),
                    work.collection_id.to_string(),
                    work.revision.0,
                    serde_json::to_string(&work).map_err(storage)?
                ],
            )
            .map_err(storage)?;
        }
        for edition in editions {
            tx.execute(
                "INSERT INTO editions VALUES (?,?,?,?,?)",
                params![
                    edition.id.to_string(),
                    edition.collection_id.to_string(),
                    edition.revision.0,
                    edition.work_id.map(|id| id.to_string()),
                    serde_json::to_string(&edition).map_err(storage)?
                ],
            )
            .map_err(storage)?;
        }
        for capture in captures {
            project_capture(&tx, &capture)?;
        }
        for (capture_id, locator) in capture_locators {
            tx.execute(
                "INSERT INTO legacy_payload_locators(capture_id,item_id,relative_path) VALUES(?,?,?)",
                params![capture_id.to_string(), locator.item_id.to_string(), locator.relative_path],
            )
            .map_err(storage)?;
        }
        for asset in assets_v3 {
            tx.execute(
                "INSERT INTO assets(id,collection_id,revision,record,deleted) VALUES(?,?,?,?,?)",
                params![
                    asset.asset.id.to_string(),
                    asset.collection_id.to_string(),
                    asset.revision.0,
                    serde_json::to_string(&asset).map_err(storage)?,
                    asset.deleted
                ],
            )
            .map_err(storage)?;
        }
        for evidence in evidence_v3 {
            tx.execute(
                "INSERT INTO evidence(id,collection_id,item_id,record) VALUES(?,?,?,?)",
                params![
                    evidence.id.to_string(),
                    evidence.collection_id.to_string(),
                    evidence.item_id.map(|id| id.to_string()),
                    serde_json::to_string(&evidence).map_err(storage)?
                ],
            )
            .map_err(storage)?;
        }
        for (record, path) in records {
            project(&tx, &record, &relative(&self.archive, &path)?)?;
        }
        self.record_command(&tx, &())?;
        tx.commit().map_err(storage)?;
        Ok(())
    }
    pub(crate) fn item_path(&self, id: ItemId) -> Result<PathBuf> {
        let path: String = lock(&self.db)?
            .query_row("SELECT path FROM items WHERE id=?", [id.to_string()], |r| {
                r.get(0)
            })
            .optional()
            .map_err(storage)?
            .ok_or(Error::NotFound)?;
        Ok(self.archive.join(path))
    }
    pub(crate) fn publish(
        &self,
        item: Item,
        expected: Revision,
        added_evidence: Vec<Evidence>,
    ) -> Result<Item> {
        let _writer = lock(&self.writer)?;
        self.publish_under_writer(item, expected, added_evidence)
    }

    pub(crate) fn publish_under_writer(
        &self,
        mut item: Item,
        expected: Revision,
        added_evidence: Vec<Evidence>,
    ) -> Result<Item> {
        item.validate()?;
        let representations = item
            .captures
            .iter()
            .flat_map(|capture| capture.representations.iter())
            .map(|representation| representation.id)
            .collect::<std::collections::HashSet<_>>();
        for evidence in &added_evidence {
            if evidence.item_id != item.id || !representations.contains(&evidence.representation_id)
            {
                return Err(Error::Invalid(
                    "evidence references a representation outside the item compatibility view"
                        .into(),
                ));
            }
            for digest in &evidence.inputs {
                digest.validate()?;
            }
            if let Some(catalog) = &evidence.catalog {
                catalog.provider.validate()?;
                catalog.digest.validate()?;
            }
        }
        let previous = self.item(item.id);
        let (current, item_path) = match previous {
            Ok(previous) => (previous.revision, self.item_path(item.id)?),
            Err(Error::NotFound) => {
                let dir: String = lock(&self.db)?
                    .query_row(
                        "SELECT path FROM collections WHERE id=?",
                        [item.collection_id.to_string()],
                        |r| r.get(0),
                    )
                    .optional()
                    .map_err(storage)?
                    .ok_or(Error::NotFound)?;
                (
                    Revision(0),
                    self.archive.join(dir).join("items").join(format!(
                        "{}--{}",
                        slug(&item.title),
                        item.id
                    )),
                )
            }
            Err(e) => return Err(e),
        };
        if current != expected {
            return Err(Error::Conflict);
        }
        {
            let db = lock(&self.db)?;
            // Item revisions do not lock independent capture revisions. Refuse a
            // stale compatibility view rather than overwriting a newer capture.
            for capture in &item.captures {
                let current: Option<String> = db
                    .query_row(
                        "SELECT record FROM captures WHERE id=?",
                        [capture.id.to_string()],
                        |row| row.get(0),
                    )
                    .optional()
                    .map_err(storage)?;
                if let Some(current) = current {
                    let current: Capture = serde_json::from_str(&current).map_err(storage)?;
                    if serde_json::to_value(&current).map_err(storage)?
                        != serde_json::to_value(capture).map_err(storage)?
                    {
                        return Err(Error::Conflict);
                    }
                }
            }
            let mut graph = graph_from_projection(&db, item.collection_id)?;
            graph.items.retain(|value| value.id != item.id);
            graph.items.push(item.clone());
            for capture in &item.captures {
                graph.captures.retain(|value| value.id != capture.id);
                graph.captures.push(capture.clone());
            }
            graph.assets.retain(|asset| match asset.subject {
                AssetSubject::Item { item_id } => item_id != item.id,
                _ => true,
            });
            graph.assets.extend(item.assets.clone());
            validate_projected_graph(&db, &graph, item.collection_id)?;
        }
        item.revision = Revision(
            current
                .0
                .checked_add(1)
                .ok_or_else(|| Error::Invalid("revision overflow".into()))?,
        );
        item.updated_at = Utc::now();
        let mut evidence = self.evidence(item.id)?;
        evidence.extend(added_evidence);
        let collection_path = item_path
            .parent()
            .and_then(Path::parent)
            .ok_or_else(|| Error::Storage("item path has no collection parent".into()))?;
        let mut publications = Vec::new();
        let mut evidence_references = Vec::new();
        for value in &evidence {
            let converted = archive_v3::EvidenceRecordV3::from_wave1(item.collection_id, value);
            converted.validate()?;
            let canonical = serde_json::to_vec_pretty(&converted).map_err(storage)?;
            evidence_references.push(archive_v3::EvidenceReferenceV3 {
                id: converted.id,
                sha256: Digest::sha256(&canonical).hex,
            });
            let existing: Option<String> = lock(&self.db)?
                .query_row(
                    "SELECT record FROM evidence WHERE id=?",
                    [converted.id.to_string()],
                    |row| row.get(0),
                )
                .optional()
                .map_err(storage)?;
            match existing {
                Some(existing)
                    if serde_json::to_vec_pretty(
                        &serde_json::from_str::<archive_v3::EvidenceRecordV3>(&existing)
                            .map_err(storage)?,
                    )
                    .map_err(storage)?
                        == canonical => {}
                Some(_) => {
                    return Err(Error::Storage(format!(
                        "Evidence identity {} conflicts with its immutable body",
                        converted.id
                    )));
                }
                None => publications.push(publish_v3_revision(
                    self,
                    V3RevisionTarget {
                        collection: collection_path,
                        directory_kind: "evidence",
                        entity_id: converted.id.to_string(),
                        collection_id: item.collection_id,
                        expected: Revision(0),
                        revision: Revision(1),
                    },
                    converted,
                    vec![],
                )?),
            }
        }

        let mut asset_records = Vec::new();
        let mut staged_assets = Vec::new();
        for asset in &item.assets {
            let existing: Option<String> = lock(&self.db)?
                .query_row(
                    "SELECT record FROM assets WHERE id=?",
                    [asset.id.to_string()],
                    |row| row.get(0),
                )
                .optional()
                .map_err(storage)?;
            let (expected_asset_revision, changed) = match existing {
                Some(existing) => {
                    let existing: archive_v3::AssetRecordV3 =
                        serde_json::from_str(&existing).map_err(storage)?;
                    if existing.collection_id != item.collection_id {
                        return Err(Error::Storage(
                            "Asset identity cannot move across collections".into(),
                        ));
                    }
                    (
                        existing.revision,
                        serde_json::to_value(&existing.asset).map_err(storage)?
                            != serde_json::to_value(asset).map_err(storage)?,
                    )
                }
                None => (Revision(0), true),
            };
            if changed {
                let revision = Revision(
                    expected_asset_revision
                        .0
                        .checked_add(1)
                        .ok_or_else(|| Error::Invalid("Asset revision overflow".into()))?,
                );
                let record = archive_v3::AssetRecordV3 {
                    collection_id: item.collection_id,
                    revision,
                    asset: asset.clone(),
                    deleted: false,
                    updated_at: item.updated_at,
                    legacy_payloads: asset_legacy_payloads(
                        &self.archive,
                        &self.state,
                        &item_path,
                        asset,
                    )?,
                };
                let payloads =
                    staged_asset_payloads(&self.archive, &self.state, collection_path, asset)?;
                if !payloads.is_empty() {
                    staged_assets.push(asset.id);
                }
                publications.push(publish_v3_revision_with_payloads(
                    self,
                    V3RevisionTarget {
                        collection: collection_path,
                        directory_kind: "assets",
                        entity_id: asset.id.to_string(),
                        collection_id: item.collection_id,
                        expected: expected_asset_revision,
                        revision,
                    },
                    record.clone(),
                    vec![],
                    payloads,
                )?);
                asset_records.push(record);
            }
        }
        let record = RevisionRecord {
            item: item.clone(),
            evidence,
            legacy: None,
        };
        publications.push(publish_v3_revision(
            &self.with_command_result(&item)?,
            V3RevisionTarget {
                collection: collection_path,
                directory_kind: "items",
                entity_id: item.id.to_string(),
                collection_id: item.collection_id,
                expected: current,
                revision: item.revision,
            },
            archive_v3::ItemRecordV3::from(&item),
            evidence_references,
        )?);
        for capture in &item.captures {
            let projected = projected_revision(
                &*lock(&self.db)?,
                "captures",
                &capture.id.to_string(),
                capture.collection_id,
            )?;
            if projected == 0 {
                let revision = Revision(capture.revision.0.max(1));
                publications.push(publish_v3_revision(
                    self,
                    V3RevisionTarget {
                        collection: collection_path,
                        directory_kind: "captures",
                        entity_id: capture.id.to_string(),
                        collection_id: capture.collection_id,
                        expected: Revision(0),
                        revision,
                    },
                    archive_v3::CaptureRecordV3 {
                        capture: Capture {
                            revision,
                            ..capture.clone()
                        },
                        legacy_root: None,
                    },
                    vec![],
                )?);
            }
        }
        let mut db = lock(&self.db)?;
        let tx = db.transaction().map_err(storage)?;
        project(&tx, &record, &relative(&self.archive, &item_path)?)?;
        for asset in asset_records {
            tx.execute(
                "INSERT INTO assets(id,collection_id,revision,record,deleted) VALUES(?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET collection_id=excluded.collection_id,revision=excluded.revision,record=excluded.record,deleted=excluded.deleted",
                params![asset.asset.id.to_string(), asset.collection_id.to_string(), asset.revision.0, serde_json::to_string(&asset).map_err(storage)?, asset.deleted],
            )
            .map_err(storage)?;
        }
        for capture in &item.captures {
            project_capture(&tx, capture)?;
        }
        tx.commit().map_err(storage)?;
        for publication in publications {
            finish_v3_publication(self, publication)?;
        }
        for asset_id in staged_assets {
            let staging = self
                .state
                .join("asset-publication")
                .join(asset_id.to_string());
            if staging.exists() {
                fs::remove_dir_all(&staging).map_err(storage)?;
                sync_directory(
                    staging
                        .parent()
                        .ok_or_else(|| Error::Storage("Asset staging has no parent".into()))?,
                )?;
            }
        }
        Ok(item)
    }
    fn save_activity_record(
        &self,
        user: UserId,
        item: ItemId,
        value: serde_json::Value,
    ) -> Result<()> {
        self.item(item)?;
        let bytes = serde_json::to_string(&value).map_err(storage)?;
        if bytes.len() > 8192 {
            return Err(Error::Invalid("activity too large".into()));
        }
        let mut db = lock(&self.db)?;
        let tx = db.transaction().map_err(storage)?;
        tx.execute("INSERT INTO private.activity VALUES (?,?,?) ON CONFLICT(user_id,item_id) DO UPDATE SET record=excluded.record", params![user.to_string(),item.to_string(),bytes]).map_err(storage)?;
        self.record_command(&tx, &())?;
        tx.commit().map_err(storage)
    }
    fn activity_record(&self, user: UserId, item: ItemId) -> Result<serde_json::Value> {
        self.item(item)?;
        let value: Option<String> = lock(&self.db)?
            .query_row(
                "SELECT record FROM private.activity WHERE user_id=? AND item_id=?",
                params![user.to_string(), item.to_string()],
                |r| r.get(0),
            )
            .optional()
            .map_err(storage)?;
        value
            .map(|s| serde_json::from_str(&s).map_err(storage))
            .unwrap_or(Ok(serde_json::json!({})))
    }
}

impl ContentPort for Store {
    fn activity(&self, user: UserId, item: ItemId) -> Result<serde_json::Value> {
        self.activity_record(user, item)
    }

    fn save_activity(&self, user: UserId, item: ItemId, value: serde_json::Value) -> Result<()> {
        self.save_activity_record(user, item, value)
    }

    fn representation(
        &self,
        item: ItemId,
        representation: RepresentationId,
        index: usize,
    ) -> Result<MediaResource> {
        let (path, file) = self.representation_path(item, representation, index)?;
        let reader = File::open(&path).map_err(storage)?;
        let length = reader.metadata().map_err(storage)?.len();
        let media_type = match path
            .extension()
            .and_then(|value| value.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref()
        {
            Some("flac") => "audio/flac",
            Some("mp3") => "audio/mpeg",
            Some("wav") => "audio/wav",
            Some("opus") | Some("ogg") => "audio/ogg",
            _ => "application/octet-stream",
        };
        Ok(MediaResource {
            reader: Box::new(reader),
            length,
            media_type: media_type.into(),
            digest: file.digest,
            download_name: file.name,
        })
    }

    fn asset(&self, item: ItemId, asset: AssetId, thumbnail: bool) -> Result<MediaResource> {
        let (path, media_type, digest, download_name) = self.asset_path(item, asset, thumbnail)?;
        let reader = File::open(path).map_err(storage)?;
        let length = reader.metadata().map_err(storage)?.len();
        Ok(MediaResource {
            reader: Box::new(reader),
            length,
            media_type,
            digest,
            download_name,
        })
    }

    fn publish_cover(
        &self,
        item: ItemId,
        asset: AssetId,
        expected: Revision,
        bytes: &[u8],
        alt_text: String,
    ) -> Result<Item> {
        self.publish_cover_record(item, asset, expected, bytes, alt_text)
    }
}

impl ImportPort for Store {
    fn create_upload(
        &self,
        user: UserId,
        item: ItemId,
        expected: Revision,
        files: Vec<UploadFile>,
    ) -> Result<Upload> {
        self.create_upload_record(user, item, expected, files)
    }

    fn upload(&self, id: JobId) -> Result<Upload> {
        self.upload_record(id)
    }

    fn upload_chunk(&self, id: JobId, index: usize, offset: u64, bytes: &[u8]) -> Result<Upload> {
        self.upload_chunk_record(id, index, offset, bytes)
    }

    fn cancel_upload(&self, id: JobId) -> Result<()> {
        self.cancel_upload_record(id)
    }

    fn commit_upload(&self, id: JobId) -> Result<Item> {
        self.commit_upload_record(id)
    }

    fn commit_upload_for_job(&self, id: JobId, claim_token: &str) -> Result<Item> {
        self.commit_upload_job_record(id, claim_token)
    }

    fn queue_upload(&self, id: JobId) -> Result<JobId> {
        self.queue_upload_record(id, false)
    }

    fn queue_inbox_upload(&self, id: JobId, source_locator: &str) -> Result<JobId> {
        self.queue_inbox_upload_record(id, source_locator)
    }

    fn retry_upload(&self, id: JobId) -> Result<JobId> {
        self.queue_upload_record(id, true)
    }

    fn fail_upload(&self, id: JobId, message: &str) -> Result<()> {
        self.fail_upload_record(id, message)
    }

    fn verify_integrity(&self, item: ItemId) -> Result<Item> {
        self.verify_integrity_record(item)
    }

    fn verify_integrity_for_job(
        &self,
        item: ItemId,
        job: JobId,
        claim_token: &str,
        created_at: chrono::DateTime<Utc>,
        progress: &mut dyn FnMut(u64) -> Result<()>,
    ) -> Result<Item> {
        self.verify_integrity_job_record(item, job, claim_token, created_at, progress)
    }

    fn jobs(&self, user: UserId) -> Result<Vec<Job>> {
        self.jobs_for_user(user)
    }
}

impl JobPort for Store {
    fn enqueue_job(&self, spec: JobSpec) -> Result<JobId> {
        self.enqueue_job_record(spec)
    }

    fn claim_job(&self, worker_instance: &str, lease_seconds: i64) -> Result<Option<JobClaim>> {
        self.claim_job_record(worker_instance, lease_seconds)
    }

    fn renew_job_lease(&self, id: JobId, claim_token: &str, lease_seconds: i64) -> Result<()> {
        self.renew_job_lease_record(id, claim_token, lease_seconds)
    }

    fn heartbeat_job(
        &self,
        id: JobId,
        claim_token: &str,
        phase: &str,
        completed_units: u64,
        lease_seconds: i64,
    ) -> Result<()> {
        self.heartbeat_job_record(id, claim_token, phase, completed_units, lease_seconds)
    }

    fn complete_job(&self, id: JobId, claim_token: &str, result: serde_json::Value) -> Result<()> {
        self.complete_job_record(id, claim_token, result)
    }

    fn fail_job(&self, id: JobId, claim_token: &str, message: &str) -> Result<()> {
        self.fail_job_record(id, claim_token, message)
    }

    fn cancel_claimed_job(&self, id: JobId, claim_token: &str) -> Result<()> {
        self.cancel_claimed_job_record(id, claim_token)
    }

    fn cancellation_requested(&self, id: JobId, claim_token: &str) -> Result<bool> {
        self.cancellation_requested_record(id, claim_token)
    }

    fn request_job_cancellation(&self, id: JobId, user: UserId) -> Result<()> {
        self.request_job_cancellation_record(id, user)
    }
}

impl AdministrationPort for Store {
    fn create_user(&self, username: &str, password: &str, administrator: bool) -> Result<UserId> {
        Store::create_user(self, username, password, administrator)
    }

    fn reindex(&self) -> Result<()> {
        Store::reindex(self)
    }

    fn accounts(&self) -> Result<Vec<AccountView>> {
        Store::accounts(self)
    }

    fn account(&self, user: UserId) -> Result<AccountView> {
        Store::account(self, user)
    }

    fn update_account(
        &self,
        user: UserId,
        expected: Revision,
        display_name: &str,
        enabled: bool,
        administrator: bool,
    ) -> Result<AccountView> {
        Store::update_account(self, user, expected, display_name, enabled, administrator)
    }

    fn reassign_default(&self, user: UserId) -> Result<()> {
        Store::reassign_default(self, user)
    }
}

impl AuthenticationPort for Store {
    fn now(&self) -> chrono::DateTime<Utc> {
        self.clock.now()
    }

    fn login(
        &self,
        username: &str,
        password: &str,
        label: &str,
    ) -> Result<(String, AuthenticatedPrincipal)> {
        Store::login(self, username, password, label)
    }

    fn authenticate(&self, token: &str) -> Result<AuthenticatedPrincipal> {
        Store::authenticate(self, token)
    }

    fn authenticate_native(&self, credential: &str) -> Result<AuthenticatedPrincipal> {
        Store::authenticate_native(self, credential)
    }

    fn revoke_session(&self, user: UserId, device: DeviceId) -> Result<()> {
        self.revoke_session_record(user, device)
    }

    fn sessions(&self, user: UserId, current: Option<DeviceId>) -> Result<Vec<SessionView>> {
        Store::sessions(self, user, current)
    }

    fn replace_recovery_codes(&self, user: UserId) -> Result<Vec<String>> {
        Store::replace_recovery_codes(self, user)
    }

    fn consume_recovery_code(
        &self,
        username: &str,
        code: &str,
        new_password: &str,
        label: &str,
    ) -> Result<(String, AuthenticatedPrincipal)> {
        Store::consume_recovery_code(self, username, code, new_password, label)
    }

    fn username(&self, user: UserId) -> Result<String> {
        Store::username(self, user)
    }

    fn user_by_name(&self, username: &str) -> Result<UserId> {
        Store::user_by_name(self, username)
    }

    fn passkeys(&self, user: UserId) -> Result<Vec<StoredPasskey>> {
        Store::passkeys(self, user)
    }

    fn insert_passkey(
        &self,
        user: UserId,
        credential_id: &str,
        credential: serde_json::Value,
    ) -> Result<()> {
        Store::insert_passkey(self, user, credential_id, credential)
    }

    fn complete_passkey_authentication(
        &self,
        user: UserId,
        passkey: PasskeyId,
        expected_revision: Revision,
        expected_record: serde_json::Value,
        credential: serde_json::Value,
        label: &str,
    ) -> Result<(String, AuthenticatedPrincipal)> {
        Store::complete_passkey_authentication(
            self,
            user,
            passkey,
            expected_revision,
            expected_record,
            credential,
            label,
        )
    }

    fn revoke_passkey(&self, user: UserId, passkey: PasskeyId, expected: Revision) -> Result<()> {
        Store::revoke_passkey(self, user, passkey, expected)
    }

    fn issue_session(&self, user: UserId, label: &str) -> Result<(String, AuthenticatedPrincipal)> {
        Store::issue_session(self, user, label)
    }
}

impl EnrollmentPort for Store {
    fn start_enrollment(
        &self,
        user: UserId,
        label: &str,
        verification_url: &str,
    ) -> Result<EnrollmentStart> {
        Store::start_enrollment(self, user, label, verification_url)
    }

    fn approve_enrollment(&self, user: UserId, user_code: &str) -> Result<EnrollmentView> {
        Store::approve_enrollment(self, user, user_code)
    }

    fn cancel_enrollment(&self, user: UserId, enrollment: EnrollmentId) -> Result<()> {
        Store::cancel_enrollment(self, user, enrollment)
    }

    fn complete_enrollment(
        &self,
        enrollment: EnrollmentId,
        completion_secret: &str,
    ) -> Result<NativeCredentialIssue> {
        Store::complete_enrollment(self, enrollment, completion_secret)
    }

    fn native_devices(&self, user: UserId) -> Result<Vec<NativeDeviceView>> {
        Store::native_devices(self, user)
    }

    fn revoke_native_device(
        &self,
        user: UserId,
        credential: NativeCredentialId,
        expected: Revision,
    ) -> Result<()> {
        Store::revoke_native_device(self, user, credential, expected)
    }
}

impl IdempotencyPort for Store {
    fn command_effects(
        &self,
        receipt: collector_app::CommandReceipt,
    ) -> Box<dyn collector_app::CommandEffects> {
        Box::new(Self {
            receipt: Some(receipt),
            ..self.clone()
        })
    }

    fn begin_command(
        &self,
        user: UserId,
        api_major: u16,
        operation: &str,
        key: &str,
        request_sha256: &str,
    ) -> Result<ReceiptDecision> {
        let mut db = lock(&self.db)?;
        let tx = db.transaction().map_err(storage)?;
        let existing: Option<(String, String, Option<String>)> = tx
            .query_row(
                "SELECT request_sha256,state,result FROM private.command_receipts WHERE user_id=? AND api_major=? AND operation=? AND idempotency_key=?",
                params![user.to_string(), api_major, operation, key],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()
            .map_err(storage)?;
        let decision = match existing {
            Some((digest, _, _)) if digest != request_sha256 => {
                return Err(Error::IdempotencyConflict);
            }
            Some((_, state, Some(result))) if state == "completed" => {
                ReceiptDecision::Replay(serde_json::from_str(&result).map_err(storage)?)
            }
            Some((_, state, _)) if state == "in_progress" => {
                return Err(Error::CommandInProgress);
            }
            Some(_) => {
                return Err(Error::Storage("invalid command receipt state".into()));
            }
            None => {
                let now = Utc::now().timestamp();
                tx.execute(
                    "INSERT INTO private.command_receipts(user_id,api_major,operation,idempotency_key,request_sha256,state,result,created_at,updated_at) VALUES(?,?,?,?,?,'in_progress',NULL,?,?)",
                    params![user.to_string(), api_major, operation, key, request_sha256, now, now],
                )
                .map_err(storage)?;
                ReceiptDecision::Execute
            }
        };
        tx.commit().map_err(storage)?;
        Ok(decision)
    }

    fn complete_command(
        &self,
        user: UserId,
        api_major: u16,
        operation: &str,
        key: &str,
        request_sha256: &str,
        result: serde_json::Value,
    ) -> Result<()> {
        let mut db = lock(&self.db)?;
        let tx = db.transaction().map_err(storage)?;
        commands::complete(
            &tx,
            &collector_app::CommandReceipt {
                user,
                api_major,
                operation: operation.into(),
                key: key.into(),
                request_sha256: request_sha256.into(),
            },
            &result,
        )?;
        tx.commit().map_err(storage)
    }

    fn abandon_command(
        &self,
        user: UserId,
        api_major: u16,
        operation: &str,
        key: &str,
        request_sha256: &str,
    ) -> Result<()> {
        lock(&self.db)?
            .execute(
                "DELETE FROM private.command_receipts WHERE user_id=? AND api_major=? AND operation=? AND idempotency_key=? AND request_sha256=? AND state='in_progress' AND NOT EXISTS(SELECT 1 FROM private.command_outcomes o WHERE o.user_id=command_receipts.user_id AND o.api_major=command_receipts.api_major AND o.operation=command_receipts.operation AND o.idempotency_key=command_receipts.idempotency_key)",
                params![user.to_string(), api_major, operation, key, request_sha256],
            )
            .map_err(storage)?;
        Ok(())
    }
}

fn migrate_databases(db: &mut Connection) -> Result<()> {
    let private_version: u32 = db
        .query_row("PRAGMA private.user_version", [], |row| row.get(0))
        .map_err(storage)?;
    if private_version > 8 {
        return Err(Error::Unavailable(format!(
            "private database schema {private_version} is newer than this server"
        )));
    }
    if private_version < 2 {
        let tx = db.transaction().map_err(storage)?;
        tx.execute_batch("
          CREATE TABLE IF NOT EXISTS private.settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
          CREATE TABLE IF NOT EXISTS private.users (id TEXT PRIMARY KEY, username TEXT UNIQUE NOT NULL, password_hash TEXT NOT NULL, administrator INTEGER NOT NULL);
          CREATE TABLE IF NOT EXISTS private.grants (collection_id TEXT NOT NULL, user_id TEXT NOT NULL, role INTEGER NOT NULL, PRIMARY KEY(collection_id,user_id));
          CREATE TABLE IF NOT EXISTS private.sessions (hash TEXT PRIMARY KEY, user_id TEXT NOT NULL, device_id TEXT NOT NULL, label TEXT NOT NULL, expires INTEGER NOT NULL);
          CREATE TABLE IF NOT EXISTS private.jobs (id TEXT PRIMARY KEY, record TEXT NOT NULL);
          CREATE TABLE IF NOT EXISTS private.passkeys (credential_id TEXT PRIMARY KEY, user_id TEXT NOT NULL, record TEXT NOT NULL);
          CREATE TABLE IF NOT EXISTS private.activity (user_id TEXT NOT NULL, item_id TEXT NOT NULL, record TEXT NOT NULL, PRIMARY KEY(user_id,item_id));
        ").map_err(storage)?;
        let selected: Option<String> = tx
            .query_row(
                "SELECT id FROM private.users ORDER BY rowid LIMIT 1",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage)?;
        if let Some(user_id) = selected {
            tx.execute(
                "INSERT OR IGNORE INTO private.settings(key,value) VALUES('default_user_id',?)",
                [user_id],
            )
            .map_err(storage)?;
        }
        tx.execute_batch("PRAGMA private.user_version = 2;")
            .map_err(storage)?;
        tx.commit().map_err(storage)?;
    }
    if private_version < 3 {
        let tx = db.transaction().map_err(storage)?;
        tx.execute_batch(
            "CREATE TABLE IF NOT EXISTS private.command_receipts (
              user_id TEXT NOT NULL,
              api_major INTEGER NOT NULL,
              operation TEXT NOT NULL,
              idempotency_key TEXT NOT NULL,
              request_sha256 TEXT NOT NULL,
              state TEXT NOT NULL CHECK(state IN ('in_progress','completed')),
              result TEXT,
              created_at INTEGER NOT NULL,
              updated_at INTEGER NOT NULL,
              PRIMARY KEY(user_id,api_major,operation,idempotency_key)
            );
            PRAGMA private.user_version = 3;",
        )
        .map_err(storage)?;
        tx.commit().map_err(storage)?;
    }
    if private_version < 4 {
        let tx = db.transaction().map_err(storage)?;
        tx.execute_batch(
            "CREATE TABLE IF NOT EXISTS private.publication_recovery (
              transaction_id TEXT PRIMARY KEY,
              kind TEXT NOT NULL,
              state TEXT NOT NULL,
              record TEXT NOT NULL,
              created_at INTEGER NOT NULL,
              updated_at INTEGER NOT NULL
            );
            PRAGMA private.user_version = 4;",
        )
        .map_err(storage)?;
        tx.commit().map_err(storage)?;
    }
    if private_version < 5 {
        let tx = db.transaction().map_err(storage)?;
        tx.execute_batch(
            "ALTER TABLE private.users ADD COLUMN display_name TEXT NOT NULL DEFAULT '';
            ALTER TABLE private.users ADD COLUMN enabled INTEGER NOT NULL DEFAULT 1;
            ALTER TABLE private.users ADD COLUMN revision INTEGER NOT NULL DEFAULT 1;
            UPDATE private.users SET display_name=username WHERE display_name='';

            ALTER TABLE private.grants RENAME TO grants_v4;
            CREATE TABLE private.grants (
              collection_id TEXT NOT NULL,
              user_id TEXT NOT NULL,
              role INTEGER NOT NULL,
              revision INTEGER NOT NULL,
              PRIMARY KEY(collection_id,user_id)
            );
            INSERT INTO private.grants(collection_id,user_id,role,revision)
              SELECT collection_id,user_id,role,1 FROM private.grants_v4;
            DROP TABLE private.grants_v4;

            ALTER TABLE private.sessions RENAME TO sessions_v4;
            CREATE TABLE private.sessions (
              hash TEXT PRIMARY KEY,
              user_id TEXT NOT NULL,
              session_id TEXT UNIQUE NOT NULL,
              label TEXT NOT NULL,
              expires INTEGER NOT NULL,
              authenticated_at INTEGER NOT NULL,
              created_at INTEGER NOT NULL
            );
            INSERT INTO private.sessions(hash,user_id,session_id,label,expires,authenticated_at,created_at)
              SELECT hash,user_id,device_id,label,expires,0,0 FROM private.sessions_v4;
            DROP TABLE private.sessions_v4;

            ALTER TABLE private.passkeys RENAME TO passkeys_v4;
            CREATE TABLE private.passkeys (
              passkey_id TEXT PRIMARY KEY,
              credential_id TEXT UNIQUE NOT NULL,
              user_id TEXT NOT NULL,
              label TEXT NOT NULL,
              revision INTEGER NOT NULL,
              record TEXT NOT NULL,
              created_at INTEGER NOT NULL,
              revoked_at INTEGER
            );

            CREATE TABLE private.recovery_batches (
              id TEXT PRIMARY KEY,
              user_id TEXT NOT NULL,
              created_at INTEGER NOT NULL,
              invalidated_at INTEGER
            );
            CREATE TABLE private.recovery_codes (
              batch_id TEXT NOT NULL,
              code_hash TEXT NOT NULL,
              consumed_at INTEGER,
              PRIMARY KEY(batch_id,code_hash)
            );
            CREATE TABLE private.security_audit (
              id TEXT PRIMARY KEY,
              user_id TEXT,
              event TEXT NOT NULL,
              metadata TEXT NOT NULL,
              created_at INTEGER NOT NULL
            );",
        )
        .map_err(storage)?;
        let legacy_passkeys = {
            let mut statement = tx
                .prepare("SELECT credential_id,user_id,record FROM private.passkeys_v4")
                .map_err(storage)?;
            let rows = statement
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                })
                .map_err(storage)?;
            rows.collect::<std::result::Result<Vec<_>, _>>()
                .map_err(storage)?
        };
        for (credential_id, user_id, record) in legacy_passkeys {
            let passkey_id = stable_private_uuid(&format!("passkey:{credential_id}"));
            tx.execute(
                "INSERT INTO private.passkeys(passkey_id,credential_id,user_id,label,revision,record,created_at,revoked_at) VALUES(?,?,?,'Migrated passkey',1,?,0,NULL)",
                params![passkey_id.to_string(), credential_id, user_id, record],
            )
            .map_err(storage)?;
        }
        tx.execute_batch(
            "DROP TABLE private.passkeys_v4;
            PRAGMA private.user_version = 5;",
        )
        .map_err(storage)?;
        tx.commit().map_err(storage)?;
    }
    if private_version < 6 {
        let tx = db.transaction().map_err(storage)?;
        tx.execute_batch(
            "CREATE TABLE private.enrollments (
              id TEXT PRIMARY KEY,
              user_id TEXT NOT NULL,
              completion_secret_hash TEXT NOT NULL,
              user_code TEXT NOT NULL,
              requested_label TEXT NOT NULL,
              verification_url TEXT NOT NULL,
              phase TEXT NOT NULL,
              created_at INTEGER NOT NULL,
              expires_at INTEGER NOT NULL,
              approved_at INTEGER,
              issued_at INTEGER,
              cancelled_at INTEGER
            );
            CREATE UNIQUE INDEX private.enrollments_active_user_code
              ON enrollments(user_code) WHERE phase IN ('pending_approval','approved');
            CREATE TABLE private.native_credentials (
              id TEXT PRIMARY KEY,
              user_id TEXT NOT NULL,
              label TEXT NOT NULL,
              revision INTEGER NOT NULL,
              credential_hash TEXT UNIQUE NOT NULL,
              created_at INTEGER NOT NULL,
              expires_at INTEGER NOT NULL,
              revoked_at INTEGER
            );
            PRAGMA private.user_version = 6;",
        )
        .map_err(storage)?;
        tx.commit().map_err(storage)?;
    }
    if private_version < 7 {
        let tx = db.transaction().map_err(storage)?;
        tx.execute_batch("CREATE TABLE private.grant_revisions (
            collection_id TEXT NOT NULL, user_id TEXT NOT NULL, revision INTEGER NOT NULL,
            PRIMARY KEY(collection_id,user_id));
            INSERT INTO private.grant_revisions SELECT collection_id,user_id,revision FROM private.grants;
            CREATE TABLE private.command_outcomes (
              user_id TEXT NOT NULL, api_major INTEGER NOT NULL, operation TEXT NOT NULL, idempotency_key TEXT NOT NULL,
              root TEXT NOT NULL, path TEXT NOT NULL, sha256 TEXT NOT NULL, result TEXT NOT NULL,
              PRIMARY KEY(user_id,api_major,operation,idempotency_key));
            PRAGMA private.user_version = 7;").map_err(storage)?;
        tx.commit().map_err(storage)?;
    }
    if private_version < 8 {
        let tx = db.transaction().map_err(storage)?;
        tx.execute_batch(
            "ALTER TABLE private.jobs RENAME TO jobs_v7;
            CREATE TABLE private.jobs (
              id TEXT PRIMARY KEY,
              user_id TEXT NOT NULL,
              collection_id TEXT,
              item_id TEXT,
              kind TEXT NOT NULL,
              state TEXT NOT NULL CHECK(state IN ('queued','running','completed','failed','cancelled','cancel_requested')),
              phase TEXT NOT NULL,
              completed_units INTEGER NOT NULL,
              total_units INTEGER NOT NULL,
              attempt INTEGER NOT NULL,
              created_at INTEGER NOT NULL,
              updated_at INTEGER NOT NULL,
              error TEXT,
              limitation TEXT,
              request TEXT NOT NULL CHECK(length(request) <= 16777216),
              result TEXT CHECK(result IS NULL OR length(result) <= 1048576),
              worker_instance TEXT,
              lease_expires INTEGER,
              claim_token TEXT,
              cancellation_requested INTEGER NOT NULL DEFAULT 0
            );
            CREATE INDEX private.jobs_owner_order ON jobs(user_id,created_at,id);
            CREATE INDEX private.jobs_claim_order ON jobs(state,created_at,id);",
        )
        .map_err(storage)?;
        let legacy = {
            let mut statement = tx
                .prepare("SELECT id,record FROM private.jobs_v7 ORDER BY id")
                .map_err(storage)?;
            let rows = statement
                .query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })
                .map_err(storage)?;
            rows.collect::<std::result::Result<Vec<_>, _>>()
                .map_err(storage)?
        };
        for (id, record) in legacy {
            let job: serde_json::Value = serde_json::from_str(&record).map_err(storage)?;
            let field = |name: &str| {
                job.get(name)
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned)
            };
            let user_id = field("user_id")
                .ok_or_else(|| Error::Storage("legacy job has no user identity".into()))?;
            let item_id = field("item_id");
            let kind =
                field("kind").ok_or_else(|| Error::Storage("legacy job has no kind".into()))?;
            let state =
                field("state").ok_or_else(|| Error::Storage("legacy job has no state".into()))?;
            let phase = job_phase_storage(&parse_job_state(&state)?);
            let completed = job
                .get("completed_bytes")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            let total = job
                .get("total_bytes")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            let error = field("error");
            tx.execute(
                "INSERT INTO private.jobs(
                  id,user_id,collection_id,item_id,kind,state,phase,
                  completed_units,total_units,attempt,created_at,updated_at,
                  error,limitation,request,result,worker_instance,lease_expires,
                  claim_token,cancellation_requested
                ) VALUES(?,?,NULL,?,?,?,?,?,?,1,0,0,?,NULL,?,NULL,NULL,NULL,NULL,0)",
                params![
                    id, user_id, item_id, kind, state, phase, completed, total, error, record,
                ],
            )
            .map_err(storage)?;
        }
        tx.execute_batch(
            "DROP TABLE private.jobs_v7;
            PRAGMA private.user_version = 8;",
        )
        .map_err(storage)?;
        tx.commit().map_err(storage)?;
    }
    let catalog_version: u32 = db
        .query_row("PRAGMA main.user_version", [], |row| row.get(0))
        .map_err(storage)?;
    if catalog_version > 3 {
        return Err(Error::Unavailable(format!(
            "catalog projection schema {catalog_version} is newer than this server"
        )));
    }
    if catalog_version < 2 {
        let tx = db.transaction().map_err(storage)?;
        tx.execute_batch("
          CREATE TABLE IF NOT EXISTS collections (id TEXT PRIMARY KEY, record TEXT NOT NULL, path TEXT NOT NULL);
          CREATE TABLE IF NOT EXISTS items (id TEXT PRIMARY KEY, collection_id TEXT NOT NULL, revision INTEGER NOT NULL, title TEXT NOT NULL, subtitle TEXT NOT NULL, record TEXT NOT NULL, path TEXT NOT NULL, deleted INTEGER NOT NULL DEFAULT 0);
          CREATE INDEX IF NOT EXISTS items_collection ON items(collection_id, id);
          CREATE VIRTUAL TABLE IF NOT EXISTS search USING fts5(id UNINDEXED, title, subtitle, tokenize='unicode61 remove_diacritics 2');
          CREATE TABLE IF NOT EXISTS evidence (id TEXT PRIMARY KEY, item_id TEXT NOT NULL, record TEXT NOT NULL);
          CREATE INDEX IF NOT EXISTS evidence_item ON evidence(item_id);
          CREATE TABLE IF NOT EXISTS works (id TEXT PRIMARY KEY, collection_id TEXT NOT NULL, revision INTEGER NOT NULL, record TEXT NOT NULL);
          CREATE TABLE IF NOT EXISTS editions (id TEXT PRIMARY KEY, collection_id TEXT NOT NULL, revision INTEGER NOT NULL, work_id TEXT, record TEXT NOT NULL);
          CREATE TABLE IF NOT EXISTS captures (id TEXT PRIMARY KEY, collection_id TEXT NOT NULL, revision INTEGER NOT NULL, item_id TEXT, record TEXT NOT NULL);
          CREATE TABLE IF NOT EXISTS components (id TEXT PRIMARY KEY, collection_id TEXT NOT NULL, item_id TEXT NOT NULL, specimen_id TEXT NOT NULL, position INTEGER NOT NULL, carrier_id TEXT, record TEXT NOT NULL);
          CREATE TABLE IF NOT EXISTS legacy_payload_locators (capture_id TEXT PRIMARY KEY, item_id TEXT NOT NULL, relative_path TEXT NOT NULL);
        ").map_err(storage)?;
        tx.execute_batch("PRAGMA main.user_version = 2;")
            .map_err(storage)?;
        tx.commit().map_err(storage)?;
    }
    if catalog_version < 3 {
        let tx = db.transaction().map_err(storage)?;
        tx.execute_batch(
            "DROP TABLE IF EXISTS evidence;
            CREATE TABLE evidence (
              id TEXT PRIMARY KEY,
              collection_id TEXT NOT NULL,
              item_id TEXT,
              record TEXT NOT NULL
            );
            CREATE INDEX evidence_item ON evidence(item_id);
            CREATE TABLE IF NOT EXISTS assets (
              id TEXT PRIMARY KEY,
              collection_id TEXT NOT NULL,
              revision INTEGER NOT NULL,
              record TEXT NOT NULL,
              deleted INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS assets_collection ON assets(collection_id,id);
            PRAGMA main.user_version = 3;",
        )
        .map_err(storage)?;
        tx.commit().map_err(storage)?;
    }
    Ok(())
}

pub(crate) fn job_phase_storage(state: &JobState) -> &'static str {
    match state {
        JobState::Queued => "awaiting_source",
        JobState::Running => "staging",
        JobState::Completed => "completed",
        JobState::Failed => "failed",
        JobState::Cancelled => "cancelled",
    }
}

pub(crate) fn parse_job_state(value: &str) -> Result<JobState> {
    match value {
        "queued" => Ok(JobState::Queued),
        "running" | "cancel_requested" => Ok(JobState::Running),
        "completed" => Ok(JobState::Completed),
        "failed" => Ok(JobState::Failed),
        "cancelled" => Ok(JobState::Cancelled),
        _ => Err(Error::Storage(format!("invalid durable job state {value}"))),
    }
}

fn stable_private_uuid(label: &str) -> uuid::Uuid {
    use sha2::{Digest as _, Sha256};
    let mut bytes: [u8; 16] = Sha256::digest(label.as_bytes())[..16].try_into().unwrap();
    bytes[6] = (bytes[6] & 0x0f) | 0x50;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    uuid::Uuid::from_bytes(bytes)
}

fn reconcile_collection_creations(db: &mut Connection, archive: &Path) -> Result<()> {
    let recoveries = {
        let mut statement = db
            .prepare(
                "SELECT transaction_id,state,record FROM private.publication_recovery WHERE kind='collection_create' ORDER BY transaction_id",
            )
            .map_err(storage)?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(storage)?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(storage)?
    };
    for (transaction_id, state, record) in recoveries {
        if state != "prepared" {
            return Err(Error::Storage(format!(
                "collection recovery {transaction_id} has unknown state"
            )));
        }
        let recovery: CollectionCreateRecovery = serde_json::from_str(&record).map_err(storage)?;
        if transaction_id != recovery.collection.id.to_string()
            || recovery.collection.revision != Revision(1)
            || !safe_collection_recovery_path(&recovery.relative_path)
        {
            return Err(Error::Storage(
                "collection recovery record has invalid identity or path".into(),
            ));
        }
        let directory = archive.join(&recovery.relative_path);
        let record_path = directory.join("collection.toml");
        if !record_path.exists() {
            let temporary = directory.join("collection.toml.tmp");
            if temporary.exists() {
                fs::remove_file(temporary).map_err(storage)?;
            }
            if directory.exists() {
                if fs::read_dir(&directory).map_err(storage)?.next().is_some() {
                    return Err(Error::Storage(
                        "incomplete collection recovery contains unknown archive data".into(),
                    ));
                }
                fs::remove_dir(&directory).map_err(storage)?;
                sync_directory(directory.parent().unwrap())?;
            }
            db.execute(
                "DELETE FROM private.publication_recovery WHERE transaction_id=?",
                [&transaction_id],
            )
            .map_err(storage)?;
            continue;
        }
        let archived: Collection =
            toml::from_str(&fs::read_to_string(&record_path).map_err(storage)?).map_err(storage)?;
        if serde_json::to_value(&archived).map_err(storage)?
            != serde_json::to_value(&recovery.collection).map_err(storage)?
        {
            return Err(Error::Storage(
                "collection recovery record differs from committed archive collection".into(),
            ));
        }
        let owner_exists: bool = db
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM private.users WHERE id=?)",
                [recovery.owner_user_id.to_string()],
                |row| row.get(0),
            )
            .map_err(storage)?;
        if !owner_exists {
            return Err(Error::Storage(
                "collection recovery intended owner no longer exists".into(),
            ));
        }
        let tx = db.transaction().map_err(storage)?;
        tx.execute(
            "INSERT INTO private.grants(collection_id,user_id,role,revision) VALUES(?,?,2,1) ON CONFLICT(collection_id,user_id) DO UPDATE SET role=2,revision=private.grants.revision+1",
            params![recovery.collection.id.to_string(), recovery.owner_user_id.to_string()],
        )
        .map_err(storage)?;
        tx.execute(
            "DELETE FROM private.publication_recovery WHERE transaction_id=?",
            [&transaction_id],
        )
        .map_err(storage)?;
        tx.commit().map_err(storage)?;
    }
    Ok(())
}

fn safe_collection_recovery_path(path: &str) -> bool {
    let path = Path::new(path);
    !path.is_absolute()
        && path.components().count() == 2
        && matches!(path.components().next(), Some(std::path::Component::Normal(value)) if value == "collections")
        && path
            .components()
            .all(|component| matches!(component, std::path::Component::Normal(_)))
}

impl Catalog for Store {
    fn collections(&self, user: UserId) -> Result<Vec<Collection>> {
        let db = lock(&self.db)?;
        let mut stmt = db.prepare("SELECT c.record FROM collections c JOIN private.grants g ON c.id=g.collection_id WHERE g.user_id=? ORDER BY c.id").map_err(storage)?;
        let values = stmt
            .query_map([user.to_string()], |r| r.get::<_, String>(0))
            .map_err(storage)?;
        values
            .map(|v| serde_json::from_str(&v.map_err(storage)?).map_err(storage))
            .collect()
    }
    fn create_collection(&self, user: UserId, requested: Collection) -> Result<Collection> {
        let _writer = lock(&self.writer)?;
        if requested.revision != Revision(1) || requested.name.trim().is_empty() {
            return Err(Error::Invalid("invalid collection creation effect".into()));
        }
        let existing: Option<String> = lock(&self.db)?
            .query_row(
                "SELECT record FROM collections WHERE id=?",
                [requested.id.to_string()],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage)?;
        if let Some(existing) = existing {
            let existing: Collection = serde_json::from_str(&existing).map_err(storage)?;
            if existing.name != requested.name {
                return Err(Error::Conflict);
            }
            if self.role(user, existing.id)? != Role::Owner {
                return Err(Error::Storage(
                    "recovered collection is missing its intended owner".into(),
                ));
            }
            return Ok(existing);
        }
        let recovery: Option<String> = lock(&self.db)?
            .query_row(
                "SELECT record FROM private.publication_recovery WHERE transaction_id=? AND kind='collection_create'",
                [requested.id.to_string()],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage)?;
        let recovery = match recovery {
            Some(record) => {
                let recovery: CollectionCreateRecovery =
                    serde_json::from_str(&record).map_err(storage)?;
                if recovery.owner_user_id != user || recovery.collection.name != requested.name {
                    return Err(Error::Conflict);
                }
                recovery
            }
            None => CollectionCreateRecovery {
                collection: requested,
                owner_user_id: user,
                relative_path: String::new(),
            },
        };
        let mut recovery = recovery;
        if recovery.relative_path.is_empty() {
            recovery.relative_path = format!(
                "collections/{}--{}",
                slug(&recovery.collection.name),
                recovery.collection.id
            );
            let now = Utc::now().timestamp();
            lock(&self.db)?
                .execute(
                    "INSERT INTO private.publication_recovery(transaction_id,kind,state,record,created_at,updated_at) VALUES(?, 'collection_create', 'prepared', ?, ?, ?)",
                    params![recovery.collection.id.to_string(), serde_json::to_string(&recovery).map_err(storage)?, now, now],
                )
                .map_err(storage)?;
        }
        self.publication_fault
            .check(PublicationFaultPoint::Prepared)?;
        let collection = recovery.collection;
        let dir = self.archive.join(&recovery.relative_path);
        if !dir.exists() {
            fs::create_dir(&dir).map_err(storage)?;
        }
        let collection_bytes = toml::to_string_pretty(&collection).map_err(storage)?;
        self.prepare_command(
            "archive",
            &format!("{}/collection.toml", recovery.relative_path),
            &Digest::sha256(collection_bytes.as_bytes()).hex,
            &collection,
        )?;
        if dir.join("collection.toml").exists() {
            if fs::read(dir.join("collection.toml")).map_err(storage)?
                != collection_bytes.as_bytes()
            {
                return Err(Error::Storage(
                    "collection creation conflicts with durable archive data".into(),
                ));
            }
        } else {
            let temp = dir.join("collection.toml.tmp");
            if temp.exists() {
                fs::remove_file(&temp).map_err(storage)?;
            }
            write_new(&temp, collection_bytes.as_bytes())?;
            fs::rename(temp, dir.join("collection.toml")).map_err(storage)?;
        }
        sync_directory(&dir)?;
        sync_directory(dir.parent().unwrap())?;
        self.publication_fault
            .check(PublicationFaultPoint::RevisionCommitted)?;
        let mut db = lock(&self.db)?;
        let tx = db.transaction().map_err(storage)?;
        tx.execute(
            "INSERT INTO collections VALUES (?,?,?) ON CONFLICT(id) DO UPDATE SET record=excluded.record,path=excluded.path",
            params![
                collection.id.to_string(),
                serde_json::to_string(&collection).map_err(storage)?,
                recovery.relative_path
            ],
        )
        .map_err(storage)?;
        tx.execute(
            "INSERT INTO private.grants(collection_id,user_id,role,revision) VALUES (?,?,2,1) ON CONFLICT(collection_id,user_id) DO UPDATE SET role=2,revision=private.grants.revision+1",
            params![collection.id.to_string(), user.to_string()],
        )
        .map_err(storage)?;
        tx.execute(
            "DELETE FROM private.publication_recovery WHERE transaction_id=?",
            [collection.id.to_string()],
        )
        .map_err(storage)?;
        tx.commit().map_err(storage)?;
        self.publication_fault
            .check(PublicationFaultPoint::ProjectionApplied)?;
        self.publication_fault
            .check(PublicationFaultPoint::Completed)?;
        Ok(collection)
    }
    fn role(&self, user: UserId, collection: CollectionId) -> Result<Role> {
        let value: Option<i64> = lock(&self.db)?
            .query_row(
                "SELECT role FROM private.grants WHERE user_id=? AND collection_id=?",
                params![user.to_string(), collection.to_string()],
                |r| r.get(0),
            )
            .optional()
            .map_err(storage)?;
        match value {
            Some(0) => Ok(Role::Viewer),
            Some(1) => Ok(Role::Curator),
            Some(2) => Ok(Role::Owner),
            _ => Err(Error::Forbidden),
        }
    }
    fn grant(&self, collection: CollectionId, user: UserId, role: Role) -> Result<()> {
        let expected = self
            .grant_record(collection, user)?
            .map_or(Revision(0), |grant| grant.revision);
        self.set_grant_record(collection, user, expected, Some(role))?;
        Ok(())
    }
    fn browse(&self, user: UserId, query: BrowseQuery) -> Result<Page<Item>> {
        let db = lock(&self.db)?;
        let limit = query.limit.unwrap_or(50).clamp(1, 200) as usize;
        if query
            .cursor
            .as_ref()
            .is_some_and(|c| c.parse::<ItemId>().is_err())
        {
            return Err(Error::Invalid("invalid cursor".into()));
        }
        let collection = query.collection_id.map(|c| c.to_string());
        let search = query.search.unwrap_or_default();
        if search.len() > 256 {
            return Err(Error::Invalid("search is too long".into()));
        }
        let terms = search
            .split_whitespace()
            .map(|word| format!("\"{}\"*", word.replace('"', "\"\"")))
            .collect::<Vec<_>>()
            .join(" AND ");
        let base = "SELECT i.record FROM items i JOIN private.grants g ON g.collection_id=i.collection_id WHERE g.user_id=?1 AND i.deleted=0 AND (?2 IS NULL OR i.collection_id=?2) AND i.id > ?3";
        let sql = if terms.is_empty() {
            format!("{base} ORDER BY i.id LIMIT ?4")
        } else {
            format!(
                "{base} AND i.id IN (SELECT id FROM search WHERE search MATCH ?5) ORDER BY i.id LIMIT ?4"
            )
        };
        let mut stmt = db.prepare(&sql).map_err(storage)?;
        let cursor = query.cursor.unwrap_or_default();
        let mut rows = if terms.is_empty() {
            stmt.query(params![
                user.to_string(),
                collection,
                cursor,
                (limit + 1) as i64
            ])
        } else {
            stmt.query(params![
                user.to_string(),
                collection,
                cursor,
                (limit + 1) as i64,
                terms
            ])
        }
        .map_err(storage)?;
        let mut items: Vec<Item> = Vec::new();
        while let Some(row) = rows.next().map_err(storage)? {
            items.push(
                serde_json::from_str(&row.get::<_, String>(0).map_err(storage)?)
                    .map_err(storage)?,
            );
        }
        drop(rows);
        drop(stmt);
        for item in &mut items {
            hydrate_captures(&db, item)?;
        }
        let next_cursor = if items.len() > limit {
            items.truncate(limit);
            items.last().map(|i| i.id.to_string())
        } else {
            None
        };
        Ok(Page { items, next_cursor })
    }
    fn item(&self, id: ItemId) -> Result<Item> {
        let record: String = lock(&self.db)?
            .query_row(
                "SELECT record FROM items WHERE id=?",
                [id.to_string()],
                |r| r.get(0),
            )
            .optional()
            .map_err(storage)?
            .ok_or(Error::NotFound)?;
        let mut item: Item = serde_json::from_str(&record).map_err(storage)?;
        hydrate_captures(&*lock(&self.db)?, &mut item)?;
        Ok(item)
    }
    fn save(&self, item: Item, expected: Revision) -> Result<Item> {
        self.publish(item, expected, vec![])
    }
    fn work(&self, id: WorkId) -> Result<Work> {
        projected_record(&*lock(&self.db)?, "works", &id.to_string())
    }
    fn save_work(&self, mut work: Work, expected: Revision) -> Result<Work> {
        work.validate()?;
        let _writer = lock(&self.writer)?;
        let mut db = lock(&self.db)?;
        let current = projected_revision(&db, "works", &work.id.to_string(), work.collection_id)?;
        if current != expected.0 {
            return Err(Error::Conflict);
        }
        work.revision = Revision(
            current
                .checked_add(1)
                .ok_or_else(|| Error::Invalid("work revision overflow".into()))?,
        );
        let mut graph = graph_from_projection(&db, work.collection_id)?;
        graph.works.retain(|value| value.id != work.id);
        graph.works.push(work.clone());
        validate_projected_graph(&db, &graph, work.collection_id)?;
        let collection = collection_path(&db, &self.archive, work.collection_id)?;
        let publication = publish_v3_revision(
            &self.with_command_result(&work)?,
            V3RevisionTarget {
                collection: &collection,
                directory_kind: "works",
                entity_id: work.id.to_string(),
                collection_id: work.collection_id,
                expected: Revision(current),
                revision: work.revision,
            },
            work.clone(),
            vec![],
        )?;
        let tx = db.transaction().map_err(storage)?;
        tx.execute(
            "INSERT INTO works VALUES (?,?,?,?) ON CONFLICT(id) DO UPDATE SET collection_id=excluded.collection_id,revision=excluded.revision,record=excluded.record",
            params![work.id.to_string(), work.collection_id.to_string(), work.revision.0, serde_json::to_string(&work).map_err(storage)?],
        ).map_err(storage)?;
        tx.commit().map_err(storage)?;
        finish_v3_publication(self, publication)?;
        Ok(work)
    }
    fn edition(&self, id: EditionId) -> Result<Edition> {
        projected_record(&*lock(&self.db)?, "editions", &id.to_string())
    }
    fn save_edition(&self, mut edition: Edition, expected: Revision) -> Result<Edition> {
        edition.validate()?;
        let _writer = lock(&self.writer)?;
        let mut db = lock(&self.db)?;
        let current = projected_revision(
            &db,
            "editions",
            &edition.id.to_string(),
            edition.collection_id,
        )?;
        if current != expected.0 {
            return Err(Error::Conflict);
        }
        edition.revision = Revision(
            current
                .checked_add(1)
                .ok_or_else(|| Error::Invalid("edition revision overflow".into()))?,
        );
        let mut graph = graph_from_projection(&db, edition.collection_id)?;
        graph.editions.retain(|value| value.id != edition.id);
        graph.editions.push(edition.clone());
        validate_projected_graph(&db, &graph, edition.collection_id)?;
        let collection = collection_path(&db, &self.archive, edition.collection_id)?;
        let publication = publish_v3_revision(
            &self.with_command_result(&edition)?,
            V3RevisionTarget {
                collection: &collection,
                directory_kind: "editions",
                entity_id: edition.id.to_string(),
                collection_id: edition.collection_id,
                expected: Revision(current),
                revision: edition.revision,
            },
            edition.clone(),
            vec![],
        )?;
        let tx = db.transaction().map_err(storage)?;
        tx.execute(
            "INSERT INTO editions VALUES (?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET collection_id=excluded.collection_id,revision=excluded.revision,work_id=excluded.work_id,record=excluded.record",
            params![edition.id.to_string(), edition.collection_id.to_string(), edition.revision.0, edition.work_id.map(|id| id.to_string()), serde_json::to_string(&edition).map_err(storage)?],
        ).map_err(storage)?;
        tx.commit().map_err(storage)?;
        finish_v3_publication(self, publication)?;
        Ok(edition)
    }
    fn capture(&self, id: CaptureId) -> Result<Capture> {
        let record: String = lock(&self.db)?
            .query_row(
                "SELECT record FROM captures WHERE id=?",
                [id.to_string()],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage)?
            .ok_or(Error::NotFound)?;
        serde_json::from_str(&record).map_err(storage)
    }
    fn save_capture(&self, mut capture: Capture, expected: Revision) -> Result<Capture> {
        capture.validate()?;
        let _writer = lock(&self.writer)?;
        let mut db = lock(&self.db)?;
        let current = projected_revision(
            &db,
            "captures",
            &capture.id.to_string(),
            capture.collection_id,
        )?;
        if current != expected.0 {
            return Err(Error::Conflict);
        }
        let collection_path = collection_path(&db, &self.archive, capture.collection_id)?;
        let mut graph = graph_from_projection(&db, capture.collection_id)?;
        graph.captures.retain(|value| value.id != capture.id);
        capture.revision = Revision(
            current
                .checked_add(1)
                .ok_or_else(|| Error::Invalid("capture revision overflow".into()))?,
        );
        graph.captures.push(capture.clone());
        validate_projected_graph(&db, &graph, capture.collection_id)?;
        let legacy_root: Option<(String, String)> = db
            .query_row(
                "SELECT item_id,relative_path FROM legacy_payload_locators WHERE capture_id=?",
                [capture.id.to_string()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(storage)?;
        let legacy_root = match legacy_root {
            Some((item_id, relative_path)) => Some(archive_v3::LegacyCaptureRootV3 {
                item_id: item_id.parse().map_err(storage)?,
                relative_path,
            }),
            None => None,
        };
        let record = archive_v3::CaptureRecordV3 {
            capture: capture.clone(),
            legacy_root,
        };
        let publication = publish_v3_revision(
            &self.with_command_result(&capture)?,
            V3RevisionTarget {
                collection: &collection_path,
                directory_kind: "captures",
                entity_id: capture.id.to_string(),
                collection_id: capture.collection_id,
                expected: Revision(current),
                revision: capture.revision,
            },
            record,
            vec![],
        )?;
        let tx = db.transaction().map_err(storage)?;
        project_capture(&tx, &capture)?;
        tx.commit().map_err(storage)?;
        finish_v3_publication(self, publication)?;
        Ok(capture)
    }
    fn evidence(&self, id: ItemId) -> Result<Vec<Evidence>> {
        let db = lock(&self.db)?;
        let mut stmt = db
            .prepare("SELECT record FROM evidence WHERE item_id=? ORDER BY id")
            .map_err(storage)?;
        let values = stmt
            .query_map([id.to_string()], |r| r.get::<_, String>(0))
            .map_err(storage)?;
        let mut evidence = Vec::new();
        for value in values {
            let record: archive_v3::EvidenceRecordV3 =
                serde_json::from_str(&value.map_err(storage)?).map_err(storage)?;
            let item_id = record.item_id.ok_or_else(|| {
                Error::Storage("item Evidence projection contains unattached Evidence".into())
            })?;
            for representation_id in record.representation_ids {
                evidence.push(Evidence {
                    id: record.id,
                    item_id,
                    representation_id,
                    kind: record.kind.clone(),
                    outcome: record.outcome.clone(),
                    inputs: record.inputs.clone(),
                    implementation: record.implementation.clone(),
                    implementation_version: record.implementation_version.clone(),
                    catalog: record.catalog.clone(),
                    details: record.details.clone(),
                    created_at: record.created_at,
                });
            }
        }
        Ok(evidence)
    }
}

impl AccessPort for Store {
    fn grant_for_user(&self, collection: CollectionId, user: UserId) -> Result<Option<GrantView>> {
        self.grant_record(collection, user)
    }

    fn grants(&self, collection: CollectionId) -> Result<Vec<GrantView>> {
        self.grants_record(collection)
    }

    fn set_grant(
        &self,
        collection: CollectionId,
        user: UserId,
        expected: Revision,
        role: Option<Role>,
    ) -> Result<Option<GrantView>> {
        self.set_grant_record(collection, user, expected, role)
    }
}

impl Store {
    fn grant_record(&self, collection: CollectionId, user: UserId) -> Result<Option<GrantView>> {
        let value: Option<(i64, u64, String)> = lock(&self.db)?
            .query_row(
                "SELECT g.role,g.revision,u.username FROM private.grants g JOIN private.users u ON u.id=g.user_id WHERE g.collection_id=? AND g.user_id=?",
                params![collection.to_string(), user.to_string()],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()
            .map_err(storage)?;
        value
            .map(|(role, revision, username)| {
                Ok(GrantView {
                    collection_id: collection,
                    user_id: user,
                    username,
                    role: role_from_database(role)?,
                    revision: Revision(revision),
                })
            })
            .transpose()
    }

    fn grants_record(&self, collection: CollectionId) -> Result<Vec<GrantView>> {
        let db = lock(&self.db)?;
        let mut statement = db
            .prepare(
                "SELECT g.user_id,u.username,g.role,g.revision FROM private.grants g JOIN private.users u ON u.id=g.user_id WHERE g.collection_id=? ORDER BY u.username,g.user_id",
            )
            .map_err(storage)?;
        let rows = statement
            .query_map([collection.to_string()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, u64>(3)?,
                ))
            })
            .map_err(storage)?;
        rows.map(|row| {
            let (user, username, role, revision) = row.map_err(storage)?;
            Ok(GrantView {
                collection_id: collection,
                user_id: user.parse().map_err(storage)?,
                username,
                role: role_from_database(role)?,
                revision: Revision(revision),
            })
        })
        .collect()
    }

    fn set_grant_record(
        &self,
        collection: CollectionId,
        user: UserId,
        expected: Revision,
        role: Option<Role>,
    ) -> Result<Option<GrantView>> {
        let mut db = lock(&self.db)?;
        let tx = db
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(storage)?;
        let enabled: Option<bool> = tx
            .query_row(
                "SELECT enabled FROM private.users WHERE id=?",
                [user.to_string()],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage)?;
        let enabled = enabled.ok_or(Error::NotFound)?;
        let current: Option<(i64, u64)> = tx
            .query_row(
                "SELECT role,revision FROM private.grants WHERE collection_id=? AND user_id=?",
                params![collection.to_string(), user.to_string()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(storage)?;
        let current_revision = current.map_or(0, |(_, revision)| revision);
        let other_owners: u64 = tx.query_row(
            "SELECT COUNT(*) FROM private.grants g JOIN private.users u ON u.id=g.user_id WHERE g.collection_id=? AND g.user_id<>? AND g.role=? AND u.enabled=1",
            params![collection.to_string(), user.to_string(), Role::Owner as i64], |row| row.get(0)).map_err(storage)?;
        collector_app::security::grant_update(
            &collector_app::security::GrantAuthority {
                account_enabled: enabled,
                current_role: current
                    .map(|(role, _)| role_from_database(role))
                    .transpose()?,
                current_revision: Revision(current_revision),
                other_enabled_owners: other_owners,
            },
            expected,
            role,
        )?;
        let previous: u64 = tx
            .query_row(
                "SELECT revision FROM private.grant_revisions WHERE collection_id=? AND user_id=?",
                params![collection.to_string(), user.to_string()],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage)?
            .unwrap_or(current_revision);
        let next = previous
            .max(current_revision)
            .checked_add(1)
            .ok_or_else(|| Error::Storage("grant revision overflow".into()))?;
        tx.execute("INSERT INTO private.grant_revisions VALUES(?,?,?) ON CONFLICT(collection_id,user_id) DO UPDATE SET revision=excluded.revision",
            params![collection.to_string(),user.to_string(),next]).map_err(storage)?;
        let result = match role {
            Some(role) => {
                let revision = Revision(next);
                tx.execute(
                    "INSERT INTO private.grants(collection_id,user_id,role,revision) VALUES(?,?,?,?) ON CONFLICT(collection_id,user_id) DO UPDATE SET role=excluded.role,revision=excluded.revision",
                    params![collection.to_string(), user.to_string(), role as i64, revision.0],
                )
                .map_err(storage)?;
                Some(GrantView {
                    collection_id: collection,
                    user_id: user,
                    username: tx
                        .query_row(
                            "SELECT username FROM private.users WHERE id=?",
                            [user.to_string()],
                            |row| row.get(0),
                        )
                        .map_err(storage)?,
                    role,
                    revision,
                })
            }
            None => {
                tx.execute(
                    "DELETE FROM private.grants WHERE collection_id=? AND user_id=? AND revision=?",
                    params![collection.to_string(), user.to_string(), expected.0],
                )
                .map_err(storage)?;
                None
            }
        };
        self.record_command(&tx, &result)?;
        tx.commit().map_err(storage)?;
        Ok(result)
    }
}

fn role_from_database(value: i64) -> Result<Role> {
    match value {
        0 => Ok(Role::Viewer),
        1 => Ok(Role::Curator),
        2 => Ok(Role::Owner),
        _ => Err(Error::Storage("invalid collection grant role".into())),
    }
}
fn project(tx: &Connection, record: &RevisionRecord, path: &str) -> Result<()> {
    let item = &record.item;
    item.validate()?;
    tx.execute("INSERT INTO items VALUES (?,?,?,?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET revision=excluded.revision,title=excluded.title,subtitle=excluded.subtitle,record=excluded.record,path=excluded.path,deleted=excluded.deleted", params![item.id.to_string(),item.collection_id.to_string(),item.revision.0,item.title,item.subtitle,serde_json::to_string(item).map_err(storage)?,path,item.deleted]).map_err(storage)?;
    tx.execute("DELETE FROM search WHERE id=?", [item.id.to_string()])
        .map_err(storage)?;
    if !item.deleted {
        tx.execute(
            "INSERT INTO search VALUES (?,?,?)",
            params![item.id.to_string(), item.title, item.subtitle],
        )
        .map_err(storage)?;
    }
    for evidence in &record.evidence {
        let evidence = archive_v3::EvidenceRecordV3::from_wave1(item.collection_id, evidence);
        let serialized = serde_json::to_string(&evidence).map_err(storage)?;
        let existing: Option<String> = tx
            .query_row(
                "SELECT record FROM evidence WHERE id=?",
                [evidence.id.to_string()],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage)?;
        match existing {
            Some(existing) if existing == serialized => {}
            Some(_) => {
                return Err(Error::Storage(format!(
                    "Evidence identity {} conflicts with its projected body",
                    evidence.id
                )));
            }
            None => {
                tx.execute(
                    "INSERT INTO evidence(id,collection_id,item_id,record) VALUES (?,?,?,?)",
                    params![
                        evidence.id.to_string(),
                        item.collection_id.to_string(),
                        evidence.item_id.map(|id| id.to_string()),
                        serialized
                    ],
                )
                .map_err(storage)?;
            }
        }
    }
    tx.execute(
        "DELETE FROM components WHERE item_id=?",
        [item.id.to_string()],
    )
    .map_err(storage)?;
    for specimen in &item.specimens {
        for component in &specimen.components {
            tx.execute(
                "INSERT INTO components VALUES (?,?,?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET position=excluded.position,carrier_id=excluded.carrier_id,record=excluded.record",
                params![component.id.to_string(), item.collection_id.to_string(), item.id.to_string(), specimen.id.to_string(), component.position, component.carrier.as_ref().map(|carrier| carrier.id.to_string()), serde_json::to_string(component).map_err(storage)?],
            ).map_err(storage)?;
        }
    }
    Ok(())
}

fn asset_belongs_to_item(asset: &Asset, item: &Item) -> bool {
    match asset.subject {
        AssetSubject::Item { item_id } => item_id == item.id,
        AssetSubject::Specimen { specimen_id } => item
            .specimens
            .iter()
            .any(|specimen| specimen.id == specimen_id),
        AssetSubject::Component { component_id } => item.specimens.iter().any(|specimen| {
            specimen
                .components
                .iter()
                .any(|component| component.id == component_id)
        }),
        AssetSubject::Capture { capture_id } => {
            item.captures.iter().any(|capture| capture.id == capture_id)
        }
        AssetSubject::Work { work_id } => {
            matches!(item.identity_link, ItemIdentityLink::Work { work_id: id } if id == work_id)
        }
        AssetSubject::Edition { edition_id } => {
            matches!(item.identity_link, ItemIdentityLink::Edition { edition_id: id } if id == edition_id)
        }
    }
}

fn asset_legacy_payloads(
    archive: &Path,
    state: &Path,
    item_path: &Path,
    asset: &Asset,
) -> Result<Vec<archive_v3::LegacyPayloadV3>> {
    let mut values = Vec::new();
    for (role, file) in std::iter::once(("original", &asset.file))
        .chain(asset.thumbnail.as_ref().map(|file| ("thumbnail", file)))
    {
        let collection_path = item_path
            .parent()
            .and_then(Path::parent)
            .ok_or_else(|| Error::Storage("item path has no collection parent".into()))?;
        let current = collection_path
            .join("assets")
            .join(asset.id.to_string())
            .join("payload")
            .join(&file.name);
        let legacy = item_path
            .join("assets")
            .join(asset.id.to_string())
            .join(&file.name);
        let staged = state
            .join("asset-publication")
            .join(asset.id.to_string())
            .join(&file.name);
        let source = if current.is_file() {
            &current
        } else if staged.is_file() {
            &staged
        } else {
            &legacy
        };
        if !source.is_file() {
            return Err(Error::Storage(format!(
                "Asset payload is missing: {}",
                source.display()
            )));
        }
        let published = if source == &legacy { &legacy } else { &current };
        values.push(archive_v3::LegacyPayloadV3 {
            role: role.into(),
            relative_path: relative(archive, published)?,
            sha256: Digest::sha256(&fs::read(source).map_err(storage)?).hex,
        });
    }
    Ok(values)
}

fn staged_asset_payloads(
    archive: &Path,
    state: &Path,
    collection_path: &Path,
    asset: &Asset,
) -> Result<Vec<V3PayloadPublication>> {
    let staging = state.join("asset-publication").join(asset.id.to_string());
    if !staging.exists() {
        return Ok(vec![]);
    }
    let mut payloads = Vec::new();
    for file in std::iter::once(&asset.file).chain(asset.thumbnail.iter()) {
        let source = staging.join(&file.name);
        if !source.is_file()
            || Digest::sha256(&fs::read(&source).map_err(storage)?).hex != file.digest.hex
        {
            return Err(Error::Storage(
                "staged Asset payload does not match its record".into(),
            ));
        }
        let published = collection_path
            .join("assets")
            .join(asset.id.to_string())
            .join("payload")
            .join(&file.name);
        payloads.push(V3PayloadPublication {
            source,
            published_path: relative(archive, &published)?,
            sha256: file.digest.hex.clone(),
        });
    }
    Ok(payloads)
}

fn project_capture(tx: &Connection, capture: &Capture) -> Result<()> {
    tx.execute(
        "INSERT INTO captures VALUES (?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET collection_id=excluded.collection_id,revision=excluded.revision,item_id=excluded.item_id,record=excluded.record",
        params![capture.id.to_string(), capture.collection_id.to_string(), capture.revision.0, capture.association.item_id.map(|id| id.to_string()), serde_json::to_string(capture).map_err(storage)?],
    ).map_err(storage)?;
    Ok(())
}

fn hydrate_captures(db: &Connection, item: &mut Item) -> Result<()> {
    let mut statement = db
        .prepare("SELECT record FROM captures WHERE item_id=? AND collection_id=? ORDER BY id")
        .map_err(storage)?;
    let rows = statement
        .query_map(
            [item.id.to_string(), item.collection_id.to_string()],
            |row| row.get::<_, String>(0),
        )
        .map_err(storage)?;
    item.captures = rows
        .map(|row| serde_json::from_str(&row.map_err(storage)?).map_err(storage))
        .collect::<Result<Vec<_>>>()?;
    Ok(())
}

fn graph_from_projection(db: &Connection, collection: CollectionId) -> Result<CollectionGraph> {
    fn records<T: serde::de::DeserializeOwned>(
        db: &Connection,
        sql: &str,
        collection: CollectionId,
    ) -> Result<Vec<T>> {
        let mut statement = db.prepare(sql).map_err(storage)?;
        let rows = statement
            .query_map([collection.to_string()], |row| row.get::<_, String>(0))
            .map_err(storage)?;
        rows.map(|row| serde_json::from_str(&row.map_err(storage)?).map_err(storage))
            .collect()
    }
    let mut items: Vec<Item> = records(
        db,
        "SELECT record FROM items WHERE collection_id=?",
        collection,
    )?;
    for item in &mut items {
        hydrate_captures(db, item)?;
    }
    Ok(CollectionGraph {
        works: records(
            db,
            "SELECT record FROM works WHERE collection_id=?",
            collection,
        )?,
        editions: records(
            db,
            "SELECT record FROM editions WHERE collection_id=?",
            collection,
        )?,
        captures: records(
            db,
            "SELECT record FROM captures WHERE collection_id=?",
            collection,
        )?,
        assets: items.iter().flat_map(|item| item.assets.clone()).collect(),
        items,
    })
}

fn validate_projected_graph(
    db: &Connection,
    graph: &CollectionGraph,
    collection: CollectionId,
) -> Result<()> {
    graph.validate(collection)?;
    let mut statement = db
        .prepare("SELECT id FROM collections WHERE id<>?")
        .map_err(storage)?;
    let others = statement
        .query_map([collection.to_string()], |row| row.get::<_, String>(0))
        .map_err(storage)?;
    for other in others {
        let id = other.map_err(storage)?.parse().map_err(storage)?;
        graph.validate_disjoint(&graph_from_projection(db, id)?)?;
    }
    Ok(())
}

fn projected_record<T: serde::de::DeserializeOwned>(
    db: &Connection,
    table: &str,
    id: &str,
) -> Result<T> {
    let sql = format!("SELECT record FROM {table} WHERE id=?");
    let record: String = db
        .query_row(&sql, [id], |row| row.get(0))
        .optional()
        .map_err(storage)?
        .ok_or(Error::NotFound)?;
    serde_json::from_str(&record).map_err(storage)
}

fn projected_revision(
    db: &Connection,
    table: &str,
    id: &str,
    collection: CollectionId,
) -> Result<u64> {
    let sql = format!("SELECT revision,collection_id FROM {table} WHERE id=?");
    let current: Option<(u64, String)> = db
        .query_row(&sql, [id], |row| Ok((row.get(0)?, row.get(1)?)))
        .optional()
        .map_err(storage)?;
    match current {
        Some((_, owner)) if owner != collection.to_string() => Err(Error::Invalid(
            "entity identity cannot move across collections".into(),
        )),
        Some((revision, _)) => Ok(revision),
        None => Ok(0),
    }
}

fn collection_path(db: &Connection, archive: &Path, collection: CollectionId) -> Result<PathBuf> {
    let path: String = db
        .query_row(
            "SELECT path FROM collections WHERE id=?",
            [collection.to_string()],
            |row| row.get(0),
        )
        .optional()
        .map_err(storage)?
        .ok_or(Error::NotFound)?;
    Ok(archive.join(path))
}

fn publish_typed_revision<T: Serialize>(
    archive: &Path,
    collection: &Path,
    directory_kind: &str,
    entity_id: String,
    collection_id: CollectionId,
    revision: Revision,
    value: &T,
) -> Result<()> {
    #[derive(Serialize)]
    struct TypedManifest {
        schema_version: u32,
        entity_type: String,
        entity_id: String,
        collection_id: CollectionId,
        revision: Revision,
        record_sha256: String,
    }
    let bytes = serde_json::to_vec_pretty(value).map_err(storage)?;
    let manifest = TypedManifest {
        schema_version: 2,
        entity_type: directory_kind.trim_end_matches('s').into(),
        entity_id: entity_id.clone(),
        collection_id,
        revision,
        record_sha256: Digest::sha256(&bytes).hex,
    };
    let stage = archive
        .join(".staging")
        .join(TransactionId::new().to_string());
    fs::create_dir(&stage).map_err(storage)?;
    write_new(&stage.join("record.json"), &bytes)?;
    let manifest_bytes = toml::to_string_pretty(&manifest)
        .map_err(storage)?
        .into_bytes();
    write_new(&stage.join("manifest.toml"), &manifest_bytes)?;
    sync_directory(&stage)?;
    let revisions = collection
        .join(directory_kind)
        .join(entity_id)
        .join("revisions");
    fs::create_dir_all(&revisions).map_err(storage)?;
    let destination = revisions.join(format!("{:020}", revision.0));
    if destination.exists() {
        if fs::read(destination.join("record.json")).map_err(storage)? != bytes
            || fs::read(destination.join("manifest.toml")).map_err(storage)? != manifest_bytes
        {
            return Err(Error::Storage(
                "typed entity revision conflicts with immutable archive data".into(),
            ));
        }
        fs::remove_dir_all(&stage).map_err(storage)?;
    } else {
        fs::rename(&stage, &destination).map_err(storage)?;
    }
    sync_directory(&revisions)?;
    // Persist newly created entity/type directories as well as the revision entry.
    let mut parent = revisions.parent();
    while let Some(path) = parent {
        sync_directory(path)?;
        if path == archive {
            break;
        }
        parent = path.parent();
    }
    Ok(())
}

#[derive(Serialize, Deserialize)]
struct V3TypedManifest {
    schema_version: u32,
    entity_type: String,
    entity_id: String,
    collection_id: CollectionId,
    revision: Revision,
    record_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    transaction_id: Option<TransactionId>,
}

struct V3RevisionTarget<'a> {
    collection: &'a Path,
    directory_kind: &'a str,
    entity_id: String,
    collection_id: CollectionId,
    expected: Revision,
    revision: Revision,
}

struct V3PayloadPublication {
    source: PathBuf,
    published_path: String,
    sha256: String,
}

fn publish_v3_revision<T: Serialize>(
    store: &Store,
    target: V3RevisionTarget<'_>,
    value: T,
    evidence: Vec<archive_v3::EvidenceReferenceV3>,
) -> Result<TransactionId> {
    publish_v3_revision_with_payloads(store, target, value, evidence, vec![])
}

fn publish_v3_revision_with_payloads<T: Serialize>(
    store: &Store,
    target: V3RevisionTarget<'_>,
    value: T,
    evidence: Vec<archive_v3::EvidenceReferenceV3>,
    payloads: Vec<V3PayloadPublication>,
) -> Result<TransactionId> {
    let V3RevisionTarget {
        collection,
        directory_kind,
        entity_id,
        collection_id,
        expected,
        revision,
    } = target;
    let transaction_id = TransactionId::new();
    let envelope = archive_v3::RevisionEnvelopeV3::new(transaction_id, value, evidence);
    let record_bytes = serde_json::to_vec_pretty(&envelope).map_err(storage)?;
    let manifest = V3TypedManifest {
        schema_version: 3,
        entity_type: directory_kind.trim_end_matches('s').into(),
        entity_id: entity_id.clone(),
        collection_id,
        revision,
        record_sha256: Digest::sha256(&record_bytes).hex,
        transaction_id: Some(transaction_id),
    };
    let manifest_bytes = toml::to_string_pretty(&manifest)
        .map_err(storage)?
        .into_bytes();
    let stage_relative = format!(".staging/{transaction_id}");
    let destination_relative = format!(
        "{}/{directory_kind}/{entity_id}/revisions/{:020}",
        relative(&store.archive, collection)?,
        revision.0
    );
    let mut artifacts = payloads
        .iter()
        .enumerate()
        .map(|(index, payload)| publication::JournalArtifact {
            kind: publication::ArtifactKind::Payload,
            staged_path: format!("{stage_relative}/payloads/{index}"),
            published_path: payload.published_path.clone(),
            sha256: payload.sha256.clone(),
        })
        .collect::<Vec<_>>();
    artifacts.extend([
        publication::JournalArtifact {
            kind: publication::ArtifactKind::Revision,
            staged_path: format!("{stage_relative}/record.v3.json"),
            published_path: format!("{destination_relative}/record.v3.json"),
            sha256: Digest::sha256(&record_bytes).hex,
        },
        publication::JournalArtifact {
            kind: publication::ArtifactKind::Revision,
            staged_path: format!("{stage_relative}/manifest.v3.toml"),
            published_path: format!("{destination_relative}/manifest.v3.toml"),
            sha256: Digest::sha256(&manifest_bytes).hex,
        },
    ]);
    let journal = publication::PublicationJournal::new(
        transaction_id,
        directory_kind.trim_end_matches('s'),
        entity_id,
        Some(collection_id),
        expected,
        revision,
        artifacts,
    )?;
    if let Some(result) = &store.command_result {
        store.prepare_command(
            "archive",
            &format!("{destination_relative}/record.v3.json"),
            &Digest::sha256(&record_bytes).hex,
            result,
        )?;
    }
    let journals = publication::PublicationJournalStore::new(&store.archive);
    journals.write_new(&journal)?;
    store
        .publication_fault
        .check(PublicationFaultPoint::Prepared)?;

    let stage = store.archive.join(&stage_relative);
    fs::create_dir(&stage).map_err(storage)?;
    if !payloads.is_empty() {
        let payload_stage = stage.join("payloads");
        fs::create_dir(&payload_stage).map_err(storage)?;
        for (index, payload) in payloads.iter().enumerate() {
            let target = payload_stage.join(index.to_string());
            fs::copy(&payload.source, &target).map_err(storage)?;
            File::open(&target)
                .and_then(|file| file.sync_all())
                .map_err(storage)?;
            if Digest::sha256(&fs::read(&target).map_err(storage)?).hex != payload.sha256 {
                return Err(Error::Storage(
                    "staged Asset payload digest mismatch".into(),
                ));
            }
        }
        sync_directory(&payload_stage)?;
    }
    write_new(&stage.join("record.v3.json"), &record_bytes)?;
    write_new(&stage.join("manifest.v3.toml"), &manifest_bytes)?;
    sync_directory(&stage)?;
    journals.advance(transaction_id, publication::PublicationPhase::BytesStaged)?;
    store
        .publication_fault
        .check(PublicationFaultPoint::BytesStaged)?;
    for (index, payload) in payloads.iter().enumerate() {
        let staged = stage.join("payloads").join(index.to_string());
        let published = store.archive.join(&payload.published_path);
        let parent = published
            .parent()
            .ok_or_else(|| Error::Storage("Asset payload has no parent".into()))?;
        fs::create_dir_all(parent).map_err(storage)?;
        if published.exists() {
            if Digest::sha256(&fs::read(&published).map_err(storage)?).hex != payload.sha256 {
                return Err(Error::Storage(
                    "published Asset payload conflicts with retry".into(),
                ));
            }
            fs::remove_file(&staged).map_err(storage)?;
        } else {
            fs::rename(&staged, &published).map_err(storage)?;
        }
        sync_directory(parent)?;
    }
    let payload_stage = stage.join("payloads");
    if payload_stage.exists() {
        fs::remove_dir(&payload_stage).map_err(storage)?;
        sync_directory(&stage)?;
    }
    journals.advance(
        transaction_id,
        publication::PublicationPhase::PayloadPublished,
    )?;
    store
        .publication_fault
        .check(PublicationFaultPoint::PayloadPublished)?;

    let destination = store.archive.join(&destination_relative);
    let revisions = destination
        .parent()
        .ok_or_else(|| Error::Storage("revision destination has no parent".into()))?;
    fs::create_dir_all(revisions).map_err(storage)?;
    if destination.exists() {
        if fs::read(destination.join("record.v3.json")).map_err(storage)? != record_bytes
            || fs::read(destination.join("manifest.v3.toml")).map_err(storage)? != manifest_bytes
        {
            return Err(Error::Storage(
                "schema-3 entity revision conflicts with immutable archive data".into(),
            ));
        }
        fs::remove_dir_all(&stage).map_err(storage)?;
    } else {
        fs::rename(&stage, &destination).map_err(storage)?;
    }
    sync_directory(revisions)?;
    let mut parent = revisions.parent();
    while let Some(path) = parent {
        sync_directory(path)?;
        if path == store.archive {
            break;
        }
        parent = path.parent();
    }
    journals.advance(
        transaction_id,
        publication::PublicationPhase::RevisionCommitted,
    )?;
    store
        .publication_fault
        .check(PublicationFaultPoint::RevisionCommitted)?;
    Ok(transaction_id)
}

fn finish_v3_publication(store: &Store, transaction_id: TransactionId) -> Result<()> {
    let journals = publication::PublicationJournalStore::new(&store.archive);
    journals.advance(
        transaction_id,
        publication::PublicationPhase::ProjectionApplied,
    )?;
    store
        .publication_fault
        .check(PublicationFaultPoint::ProjectionApplied)?;
    journals.advance(transaction_id, publication::PublicationPhase::Completed)?;
    store
        .publication_fault
        .check(PublicationFaultPoint::Completed)?;
    journals.finish(transaction_id)
}

trait TypedArchiveEntity {
    const KIND: &'static str;
    fn archive_id(&self) -> String;
    fn archive_collection(&self) -> CollectionId;
    fn archive_revision(&self) -> Revision;
}
impl TypedArchiveEntity for Work {
    const KIND: &'static str = "work";
    fn archive_id(&self) -> String {
        self.id.to_string()
    }
    fn archive_collection(&self) -> CollectionId {
        self.collection_id
    }
    fn archive_revision(&self) -> Revision {
        self.revision
    }
}
impl TypedArchiveEntity for Edition {
    const KIND: &'static str = "edition";
    fn archive_id(&self) -> String {
        self.id.to_string()
    }
    fn archive_collection(&self) -> CollectionId {
        self.collection_id
    }
    fn archive_revision(&self) -> Revision {
        self.revision
    }
}
impl TypedArchiveEntity for Capture {
    const KIND: &'static str = "capture";
    fn archive_id(&self) -> String {
        self.id.to_string()
    }
    fn archive_collection(&self) -> CollectionId {
        self.collection_id
    }
    fn archive_revision(&self) -> Revision {
        self.revision
    }
}

fn read_latest_records<T: serde::de::DeserializeOwned + TypedArchiveEntity>(
    root: &Path,
    collection: CollectionId,
) -> Result<Vec<T>> {
    if !root.exists() {
        return Ok(Vec::new());
    }
    let mut result = Vec::new();
    for entity in directories(root)? {
        let revisions = entity.join("revisions");
        if !revisions.exists() {
            continue;
        }
        let mut versions = directories(&revisions)?;
        versions.sort();
        if let Some(version) = versions.last() {
            if version.join("manifest.v3.toml").exists() {
                let manifest: V3TypedManifest = toml::from_str(
                    &fs::read_to_string(version.join("manifest.v3.toml")).map_err(storage)?,
                )
                .map_err(storage)?;
                let bytes = fs::read(version.join("record.v3.json")).map_err(storage)?;
                if Digest::sha256(&bytes).hex != manifest.record_sha256 {
                    return Err(Error::Storage(
                        "typed schema-3 entity digest mismatch".into(),
                    ));
                }
                let envelope: archive_v3::RevisionEnvelopeV3<T> =
                    serde_json::from_slice(&bytes).map_err(storage)?;
                let record = envelope.record;
                if manifest.schema_version != 3
                    || envelope.schema_version != 3
                    || manifest
                        .transaction_id
                        .is_some_and(|value| value != envelope.transaction_id)
                    || manifest.entity_type != T::KIND
                    || manifest.entity_id != record.archive_id()
                    || manifest.collection_id != record.archive_collection()
                    || manifest.collection_id != collection
                    || manifest.revision != record.archive_revision()
                {
                    return Err(Error::Storage(
                        "typed schema-3 entity manifest identity mismatch".into(),
                    ));
                }
                result.push(record);
                continue;
            }
            let bytes = fs::read(version.join("record.json")).map_err(storage)?;
            #[derive(Deserialize)]
            struct TypedManifest {
                schema_version: u32,
                entity_type: String,
                entity_id: String,
                collection_id: CollectionId,
                revision: Revision,
                record_sha256: String,
            }
            let manifest: TypedManifest = toml::from_str(
                &fs::read_to_string(version.join("manifest.toml")).map_err(storage)?,
            )
            .map_err(storage)?;
            if Digest::sha256(&bytes).hex != manifest.record_sha256 {
                return Err(Error::Storage("typed entity digest mismatch".into()));
            }
            let record: T = serde_json::from_slice(&bytes).map_err(storage)?;
            if manifest.schema_version != 2
                || manifest.entity_type != T::KIND
                || manifest.entity_id != record.archive_id()
                || manifest.collection_id != record.archive_collection()
                || manifest.collection_id != collection
                || manifest.revision != record.archive_revision()
            {
                return Err(Error::Storage(
                    "typed entity manifest identity mismatch".into(),
                ));
            }
            result.push(record);
        }
    }
    Ok(result)
}
pub(crate) fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut f = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(storage)?;
    f.write_all(bytes).map_err(storage)?;
    f.sync_all().map_err(storage)
}
pub(crate) fn sync_directory(path: &Path) -> Result<()> {
    File::open(path).and_then(|f| f.sync_all()).map_err(storage)
}
pub(crate) fn private_directory(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).map_err(storage)?;
    }
    Ok(())
}
fn relative(root: &Path, path: &Path) -> Result<String> {
    path.strip_prefix(root)
        .map_err(storage)?
        .to_str()
        .map(str::to_owned)
        .ok_or_else(|| Error::Invalid("non-UTF8 archive path".into()))
}
fn directories(path: &Path) -> Result<Vec<PathBuf>> {
    fs::read_dir(path)
        .map_err(storage)?
        .filter_map(|entry| match entry {
            Ok(e) => match e.file_type() {
                Ok(t) if t.is_dir() => Some(Ok(e.path())),
                Ok(_) => None,
                Err(e) => Some(Err(storage(e))),
            },
            Err(e) => Some(Err(storage(e))),
        })
        .collect()
}
fn slug(value: &str) -> String {
    let text: String = value
        .chars()
        .take(60)
        .map(|c| {
            if c.is_alphanumeric() || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect();
    if text.trim_matches('-').is_empty() {
        "item".into()
    } else {
        text
    }
}
