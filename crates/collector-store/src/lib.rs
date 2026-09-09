//! Portable immutable revisions plus local SQLite projections and private operational state.
mod auth;
mod files;
mod migration;
use chrono::Utc;
use collector_app::Catalog;
use collector_core::*;
pub use files::{Upload, UploadFile};
use fs2::FileExt;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::{Mutex, MutexGuard},
};

pub(crate) fn storage(e: impl std::fmt::Display) -> Error {
    Error::Storage(e.to_string())
}
pub(crate) fn lock<T>(mutex: &Mutex<T>) -> Result<MutexGuard<'_, T>> {
    mutex
        .lock()
        .map_err(|_| Error::Unavailable("storage lock poisoned; restart required".into()))
}
pub struct Store {
    pub(crate) archive: PathBuf,
    pub(crate) state: PathBuf,
    pub(crate) db: Mutex<Connection>,
    pub(crate) writer: Mutex<()>,
    pub(crate) uploads: Mutex<()>,
    _archive_lock: File,
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
impl Store {
    pub fn open(archive: impl AsRef<Path>, state: impl AsRef<Path>) -> Result<Self> {
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
        for dir in ["collections", ".staging"] {
            fs::create_dir_all(archive.join(dir)).map_err(storage)?;
        }
        migration::migrate_schema_one(&archive)?;
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
        let store = Self {
            archive,
            state,
            db: Mutex::new(db),
            writer: Mutex::new(()),
            uploads: Mutex::new(()),
            _archive_lock: archive_lock,
        };
        store.reindex()?;
        Ok(store)
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
        for dir in directories(&self.archive.join("collections"))? {
            let path = dir.join("collection.toml");
            if !path.exists() {
                continue;
            }
            let collection: Collection =
                toml::from_str(&fs::read_to_string(&path).map_err(storage)?).map_err(storage)?;
            collections.push((collection.clone(), dir.clone()));
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
            captures.extend(read_latest_records::<Capture>(
                &dir.join("captures"),
                collection.id,
            )?);
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
                assets: items.iter().flat_map(|item| item.assets.clone()).collect(),
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
            "DELETE FROM collections; DELETE FROM items; DELETE FROM search; DELETE FROM evidence; DELETE FROM works; DELETE FROM editions; DELETE FROM captures; DELETE FROM components; DELETE FROM legacy_payload_locators;",
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
        for (record, path) in records {
            project(&tx, &record, &relative(&self.archive, &path)?)?;
        }
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
        let _writer = lock(&self.writer)?;
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
        let record = RevisionRecord {
            item: item.clone(),
            evidence,
            legacy: None,
        };
        let bytes = serde_json::to_vec_pretty(&record).map_err(storage)?;
        let manifest = Manifest {
            schema_version: 2,
            entity_type: Some("item".into()),
            item_id: item.id,
            collection_id: item.collection_id,
            revision: item.revision,
            record_sha256: Digest::sha256(&bytes).hex,
        };
        let stage = self
            .archive
            .join(".staging")
            .join(TransactionId::new().to_string());
        fs::create_dir(&stage).map_err(storage)?;
        write_new(&stage.join("record.json"), &bytes)?;
        write_new(
            &stage.join("manifest.toml"),
            toml::to_string_pretty(&manifest)
                .map_err(storage)?
                .as_bytes(),
        )?;
        sync_directory(&stage)?;
        let revisions = item_path.join("revisions");
        fs::create_dir_all(&revisions).map_err(storage)?;
        sync_directory(&item_path)?;
        sync_directory(item_path.parent().unwrap())?;
        fs::rename(&stage, revisions.join(format!("{:020}", item.revision.0))).map_err(storage)?;
        sync_directory(&revisions)?;
        let collection_path = item_path
            .parent()
            .and_then(Path::parent)
            .ok_or_else(|| Error::Storage("item path has no collection parent".into()))?;
        for capture in &item.captures {
            publish_typed_revision(
                &self.archive,
                collection_path,
                "captures",
                capture.id.to_string(),
                capture.collection_id,
                Revision(capture.revision.0.max(1)),
                capture,
            )?;
        }
        // The rename is the durable commit point. Reindex replays it if SQLite publication is interrupted.
        let mut db = lock(&self.db)?;
        let tx = db.transaction().map_err(storage)?;
        project(&tx, &record, &relative(&self.archive, &item_path)?)?;
        for capture in &item.captures {
            project_capture(&tx, capture)?;
        }
        tx.commit().map_err(storage)?;
        Ok(item)
    }
    pub fn save_activity(
        &self,
        actor: &Principal,
        item: ItemId,
        value: serde_json::Value,
    ) -> Result<()> {
        let record = self.item(item)?;
        self.role(actor.user_id, record.collection_id)?;
        let bytes = serde_json::to_string(&value).map_err(storage)?;
        if bytes.len() > 8192 {
            return Err(Error::Invalid("activity too large".into()));
        }
        lock(&self.db)?.execute("INSERT INTO private.activity VALUES (?,?,?) ON CONFLICT(user_id,item_id) DO UPDATE SET record=excluded.record", params![actor.user_id.to_string(),item.to_string(),bytes]).map_err(storage)?;
        Ok(())
    }
    pub fn activity(&self, actor: &Principal, item: ItemId) -> Result<serde_json::Value> {
        let record = self.item(item)?;
        self.role(actor.user_id, record.collection_id)?;
        let value: Option<String> = lock(&self.db)?
            .query_row(
                "SELECT record FROM private.activity WHERE user_id=? AND item_id=?",
                params![actor.user_id.to_string(), item.to_string()],
                |r| r.get(0),
            )
            .optional()
            .map_err(storage)?;
        value
            .map(|s| serde_json::from_str(&s).map_err(storage))
            .unwrap_or(Ok(serde_json::json!({})))
    }
}

fn migrate_databases(db: &mut Connection) -> Result<()> {
    let private_version: u32 = db
        .query_row("PRAGMA private.user_version", [], |row| row.get(0))
        .map_err(storage)?;
    if private_version > 2 {
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
    let catalog_version: u32 = db
        .query_row("PRAGMA main.user_version", [], |row| row.get(0))
        .map_err(storage)?;
    if catalog_version > 2 {
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
    Ok(())
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
    fn create_collection(&self, user: UserId, name: &str) -> Result<Collection> {
        let _writer = lock(&self.writer)?;
        let collection = Collection {
            id: CollectionId::new(),
            revision: Revision(1),
            name: name.into(),
            created_at: Utc::now(),
        };
        let dir =
            self.archive
                .join("collections")
                .join(format!("{}--{}", slug(name), collection.id));
        fs::create_dir(&dir).map_err(storage)?;
        write_new(
            &dir.join("collection.toml"),
            toml::to_string_pretty(&collection)
                .map_err(storage)?
                .as_bytes(),
        )?;
        sync_directory(&dir)?;
        sync_directory(dir.parent().unwrap())?;
        let db = lock(&self.db)?;
        db.execute(
            "INSERT INTO collections VALUES (?,?,?)",
            params![
                collection.id.to_string(),
                serde_json::to_string(&collection).map_err(storage)?,
                relative(&self.archive, &dir)?
            ],
        )
        .map_err(storage)?;
        db.execute(
            "INSERT INTO private.grants VALUES (?,?,2)",
            params![collection.id.to_string(), user.to_string()],
        )
        .map_err(storage)?;
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
        let db = lock(&self.db)?;
        let exists: bool = db
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM private.users WHERE id=?)",
                [user.to_string()],
                |r| r.get(0),
            )
            .map_err(storage)?;
        if !exists {
            return Err(Error::NotFound);
        }
        db.execute("INSERT INTO private.grants VALUES (?,?,?) ON CONFLICT(collection_id,user_id) DO UPDATE SET role=excluded.role",params![collection.to_string(),user.to_string(),role as i64]).map_err(storage)?;
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
        publish_typed_revision(
            &self.archive,
            &collection,
            "works",
            work.id.to_string(),
            work.collection_id,
            work.revision,
            &work,
        )?;
        let tx = db.transaction().map_err(storage)?;
        tx.execute(
            "INSERT INTO works VALUES (?,?,?,?) ON CONFLICT(id) DO UPDATE SET collection_id=excluded.collection_id,revision=excluded.revision,record=excluded.record",
            params![work.id.to_string(), work.collection_id.to_string(), work.revision.0, serde_json::to_string(&work).map_err(storage)?],
        ).map_err(storage)?;
        tx.commit().map_err(storage)?;
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
        publish_typed_revision(
            &self.archive,
            &collection,
            "editions",
            edition.id.to_string(),
            edition.collection_id,
            edition.revision,
            &edition,
        )?;
        let tx = db.transaction().map_err(storage)?;
        tx.execute(
            "INSERT INTO editions VALUES (?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET collection_id=excluded.collection_id,revision=excluded.revision,work_id=excluded.work_id,record=excluded.record",
            params![edition.id.to_string(), edition.collection_id.to_string(), edition.revision.0, edition.work_id.map(|id| id.to_string()), serde_json::to_string(&edition).map_err(storage)?],
        ).map_err(storage)?;
        tx.commit().map_err(storage)?;
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
        publish_typed_revision(
            &self.archive,
            &collection_path,
            "captures",
            capture.id.to_string(),
            capture.collection_id,
            capture.revision,
            &capture,
        )?;
        let tx = db.transaction().map_err(storage)?;
        project_capture(&tx, &capture)?;
        tx.commit().map_err(storage)?;
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
        values
            .map(|v| serde_json::from_str(&v.map_err(storage)?).map_err(storage))
            .collect()
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
        tx.execute(
            "INSERT OR IGNORE INTO evidence VALUES (?,?,?)",
            params![
                evidence.id.to_string(),
                item.id.to_string(),
                serde_json::to_string(evidence).map_err(storage)?
            ],
        )
        .map_err(storage)?;
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

fn project_capture(tx: &Connection, capture: &Capture) -> Result<()> {
    tx.execute(
        "INSERT INTO captures VALUES (?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET collection_id=excluded.collection_id,revision=excluded.revision,item_id=excluded.item_id,record=excluded.record",
        params![capture.id.to_string(), capture.collection_id.to_string(), capture.revision.0, capture.association.item_id.map(|id| id.to_string()), serde_json::to_string(capture).map_err(storage)?],
    ).map_err(storage)?;
    tx.execute(
        "DELETE FROM legacy_payload_locators WHERE capture_id=?",
        [capture.id.to_string()],
    )
    .map_err(storage)?;
    if let Some(locator) = &capture.legacy_payload_locator {
        tx.execute(
            "INSERT INTO legacy_payload_locators VALUES (?,?,?) ON CONFLICT(capture_id) DO UPDATE SET item_id=excluded.item_id,relative_path=excluded.relative_path",
            params![capture.id.to_string(), locator.item_id.to_string(), locator.relative_path],
        ).map_err(storage)?;
    }
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
    let manifest_bytes = toml::to_string_pretty(&manifest).map_err(storage)?;
    write_new(&stage.join("manifest.toml"), manifest_bytes.as_bytes())?;
    sync_directory(&stage)?;
    let revisions = collection
        .join(directory_kind)
        .join(entity_id)
        .join("revisions");
    fs::create_dir_all(&revisions).map_err(storage)?;
    let destination = revisions.join(format!("{:020}", revision.0));
    if destination.exists() {
        if fs::read(destination.join("record.json")).map_err(storage)? != bytes
            || fs::read(destination.join("manifest.toml")).map_err(storage)?
                != manifest_bytes.as_bytes()
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
