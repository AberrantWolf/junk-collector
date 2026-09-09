//! Portable immutable revisions plus local SQLite projections and private operational state.
mod auth;
mod files;
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
struct Manifest {
    schema_version: u32,
    item_id: ItemId,
    collection_id: CollectionId,
    revision: Revision,
    record_sha256: String,
}
#[derive(Serialize, Deserialize)]
struct RevisionRecord {
    item: Item,
    evidence: Vec<Evidence>,
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
        fs::create_dir_all(state.join("uploads")).map_err(storage)?;
        let db = Connection::open(state.join("catalog.sqlite")).map_err(storage)?;
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
        db.execute_batch("
          CREATE TABLE IF NOT EXISTS collections (id TEXT PRIMARY KEY, record TEXT NOT NULL, path TEXT NOT NULL);
          CREATE TABLE IF NOT EXISTS items (id TEXT PRIMARY KEY, collection_id TEXT NOT NULL, revision INTEGER NOT NULL, title TEXT NOT NULL, subtitle TEXT NOT NULL, record TEXT NOT NULL, path TEXT NOT NULL, deleted INTEGER NOT NULL DEFAULT 0);
          CREATE INDEX IF NOT EXISTS items_collection ON items(collection_id, id);
          CREATE VIRTUAL TABLE IF NOT EXISTS search USING fts5(id UNINDEXED, title, subtitle, tokenize='unicode61 remove_diacritics 2');
          CREATE TABLE IF NOT EXISTS evidence (id TEXT PRIMARY KEY, item_id TEXT NOT NULL, record TEXT NOT NULL);
          CREATE INDEX IF NOT EXISTS evidence_item ON evidence(item_id);
          CREATE TABLE IF NOT EXISTS private.settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
          CREATE TABLE IF NOT EXISTS private.users (id TEXT PRIMARY KEY, username TEXT UNIQUE NOT NULL, password_hash TEXT NOT NULL, administrator INTEGER NOT NULL);
          CREATE TABLE IF NOT EXISTS private.grants (collection_id TEXT NOT NULL, user_id TEXT NOT NULL, role INTEGER NOT NULL, PRIMARY KEY(collection_id,user_id));
          CREATE TABLE IF NOT EXISTS private.sessions (hash TEXT PRIMARY KEY, user_id TEXT NOT NULL, device_id TEXT NOT NULL, label TEXT NOT NULL, expires INTEGER NOT NULL);
          CREATE TABLE IF NOT EXISTS private.jobs (id TEXT PRIMARY KEY, record TEXT NOT NULL);
          CREATE TABLE IF NOT EXISTS private.passkeys (credential_id TEXT PRIMARY KEY, user_id TEXT NOT NULL, record TEXT NOT NULL);
          CREATE TABLE IF NOT EXISTS private.activity (user_id TEXT NOT NULL, item_id TEXT NOT NULL, record TEXT NOT NULL, PRIMARY KEY(user_id,item_id));
          PRAGMA user_version = 1;
        ").map_err(storage)?;
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
        for dir in directories(&self.archive.join("collections"))? {
            let path = dir.join("collection.toml");
            if !path.exists() {
                continue;
            }
            let collection: Collection =
                toml::from_str(&fs::read_to_string(&path).map_err(storage)?).map_err(storage)?;
            collections.push((collection.clone(), dir.clone()));
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
                    let manifest: Manifest = toml::from_str(
                        &fs::read_to_string(version.join("manifest.toml")).map_err(storage)?,
                    )
                    .map_err(storage)?;
                    if manifest.schema_version != 1 {
                        return Err(Error::Unavailable(
                            "unsupported archive revision schema".into(),
                        ));
                    }
                    let bytes = fs::read(version.join("record.json")).map_err(storage)?;
                    if Digest::sha256(&bytes).hex != manifest.record_sha256 {
                        return Err(Error::Storage("archive metadata digest mismatch; restore or explicitly repair before writing".into()));
                    }
                    let record: RevisionRecord = serde_json::from_slice(&bytes).map_err(storage)?;
                    if record.item.id != manifest.item_id
                        || record.item.revision != manifest.revision
                        || record.item.collection_id != collection.id
                        || manifest.collection_id != collection.id
                    {
                        return Err(Error::Storage("archive identity mismatch".into()));
                    }
                    records.push((record, item_dir));
                }
            }
        }
        let mut db = lock(&self.db)?;
        let tx = db.transaction().map_err(storage)?;
        tx.execute_batch(
            "DELETE FROM collections; DELETE FROM items; DELETE FROM search; DELETE FROM evidence;",
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
        };
        let bytes = serde_json::to_vec_pretty(&record).map_err(storage)?;
        let manifest = Manifest {
            schema_version: 1,
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
        // The rename is the durable commit point. Reindex replays it if SQLite publication is interrupted.
        let mut db = lock(&self.db)?;
        let tx = db.transaction().map_err(storage)?;
        project(&tx, &record, &relative(&self.archive, &item_path)?)?;
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
        serde_json::from_str(&record).map_err(storage)
    }
    fn save(&self, item: Item, expected: Revision) -> Result<Item> {
        self.publish(item, expected, vec![])
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
    Ok(())
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
