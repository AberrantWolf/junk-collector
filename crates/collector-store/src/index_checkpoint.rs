//! Rebuildable catalog checkpoints. Archive publications remain authoritative.
use super::*;
use std::collections::BTreeMap;

const FORMAT: u32 = 5;
pub(crate) const CATALOG_SCHEMA: u32 = 15;
pub(crate) const TABLES: &[&str] = &[
    "collections",
    "collection_lists",
    "items",
    "works",
    "editions",
    "captures",
    "assets",
    "evidence",
    "components",
    "legacy_payload_locators",
    "browse_items",
    "music_search",
    "metadata_authorities",
    "metadata_health",
    "preparation_readiness",
    "album_group_members",
    "metadata_language_display",
    "metadata_candidates",
    "metadata_refreshes",
    "processing_observations",
    "snapshots",
    "conversion_recipes",
    "processing_groups",
    "processing_members",
    "entity_owners",
    "entity_references",
    "archive_index_files",
];

pub(crate) fn install(db: &Connection) -> Result<()> {
    db.execute_batch(
        "CREATE TABLE IF NOT EXISTS catalog_index_state(singleton INTEGER PRIMARY KEY CHECK(singleton=1), epoch INTEGER NOT NULL);
         INSERT OR IGNORE INTO catalog_index_state VALUES(1,0);
         CREATE TABLE IF NOT EXISTS catalog_checkpoint(singleton INTEGER PRIMARY KEY CHECK(singleton=1), format INTEGER NOT NULL, archive TEXT NOT NULL, epoch INTEGER NOT NULL);
         CREATE TABLE IF NOT EXISTS archive_index_files(path TEXT PRIMARY KEY, stamp TEXT NOT NULL);",
    ).map_err(storage)?;
    for table in TABLES {
        for operation in ["INSERT", "UPDATE", "DELETE"] {
            db.execute_batch(&format!(
                "CREATE TRIGGER IF NOT EXISTS checkpoint_{table}_{operation} AFTER {operation} ON {table}
                 BEGIN UPDATE catalog_index_state SET epoch=epoch+1 WHERE singleton=1; END;"
            )).map_err(storage)?;
        }
    }
    // FTS uses internal tables; edits to the searchable document set must also
    // invalidate reuse, including direct stopped-server projection repairs.
    for operation in ["INSERT", "UPDATE", "DELETE"] {
        db.execute_batch(&format!(
            "CREATE TRIGGER IF NOT EXISTS checkpoint_search_{operation} AFTER {operation} ON search_content
             BEGIN UPDATE catalog_index_state SET epoch=epoch+1 WHERE singleton=1; END;"
        )).map_err(storage)?;
    }
    Ok(())
}

fn stamp(path: &Path) -> Result<String> {
    let metadata = fs::symlink_metadata(path).map_err(storage)?;
    if !metadata.is_file() {
        return Err(Error::Storage(
            "archive index contains a nonregular metadata file".into(),
        ));
    }
    let modified = metadata
        .modified()
        .map_err(storage)?
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(storage)?
        .as_nanos();
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        Ok(format!(
            "{}:{modified}:{}:{}:{}:{}",
            metadata.len(),
            metadata.dev(),
            metadata.ino(),
            metadata.ctime(),
            metadata.ctime_nsec()
        ))
    }
    #[cfg(not(unix))]
    Ok(format!("{}:{modified}", metadata.len()))
}

fn tracked(path: &Path) -> bool {
    let components = path
        .components()
        .filter_map(|part| part.as_os_str().to_str())
        .collect::<Vec<_>>();
    let snapshot = components.contains(&"snapshots");
    if components.contains(&"files")
        || (components.contains(&"payload") && !snapshot)
        || (components.contains(&"items") && components.contains(&"assets"))
    {
        return false;
    }
    snapshot
        || components.contains(&".archive-migrations")
        || matches!(
            path.extension().and_then(|part| part.to_str()),
            Some("json" | "toml")
        )
}

/// Walk metadata names and stat information, never media contents. Payload trees
/// are pruned before traversal; archived snapshot bytes are statted only.
fn inventory(archive: &Path) -> Result<BTreeMap<String, String>> {
    fn visit(archive: &Path, path: &Path, files: &mut BTreeMap<String, String>) -> Result<()> {
        for entry in fs::read_dir(path).map_err(storage)? {
            let entry = entry.map_err(storage)?;
            let path = entry.path();
            let kind = entry.file_type().map_err(storage)?;
            let relative_path = path.strip_prefix(archive).map_err(storage)?;
            let parts = relative_path
                .components()
                .filter_map(|part| part.as_os_str().to_str())
                .collect::<Vec<_>>();
            if kind.is_symlink() {
                return Err(Error::Storage("symlink in archive metadata tree".into()));
            }
            if kind.is_dir() {
                let name = entry.file_name();
                if name == "files"
                    || (name == "payload" && !parts.contains(&"snapshots"))
                    || (name == "assets" && parts.contains(&"items"))
                {
                    continue;
                }
                visit(archive, &path, files)?;
            } else if tracked(relative_path) {
                files.insert(relative(archive, &path)?, stamp(&path)?);
            }
        }
        Ok(())
    }
    let mut files = BTreeMap::new();
    visit(archive, &archive.join("collections"), &mut files)?;
    let migrations = archive.join(".archive-migrations");
    if migrations.try_exists().map_err(storage)? {
        visit(archive, &migrations, &mut files)?;
    }
    Ok(files)
}

fn seal(db: &Connection, archive: &Path) -> Result<()> {
    db.execute("INSERT INTO catalog_checkpoint SELECT 1,?1,?2,epoch FROM catalog_index_state WHERE singleton=1
                ON CONFLICT(singleton) DO UPDATE SET format=excluded.format,archive=excluded.archive,epoch=excluded.epoch",
        params![FORMAT, archive.canonicalize().map_err(storage)?.to_string_lossy()]).map_err(storage)?;
    Ok(())
}

pub(crate) fn reusable(db: &Connection, archive: &Path) -> Result<(bool, usize)> {
    let valid: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM catalog_checkpoint c JOIN catalog_index_state s USING(singleton)
         WHERE c.format=?1 AND c.archive=?2 AND c.epoch=s.epoch)",
        params![FORMAT, archive.canonicalize().map_err(storage)?.to_string_lossy()], |row| row.get(0),
    ).map_err(storage)?;
    if !valid {
        return Ok((false, 0));
    }
    let files = inventory(archive)?;
    let mut statement = db
        .prepare("SELECT path,stamp FROM archive_index_files ORDER BY path")
        .map_err(storage)?;
    let saved = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(storage)?
        .collect::<std::result::Result<BTreeMap<_, _>, _>>()
        .map_err(storage)?;
    Ok((files == saved, files.len()))
}

pub(crate) fn rebuilt(db: &Connection, archive: &Path) -> Result<usize> {
    let files = inventory(archive)?;
    db.execute("DELETE FROM archive_index_files", [])
        .map_err(storage)?;
    let mut statement = db
        .prepare("INSERT INTO archive_index_files VALUES(?,?)")
        .map_err(storage)?;
    for (path, stamp) in &files {
        statement.execute(params![path, stamp]).map_err(storage)?;
    }
    seal(db, archive)?;
    Ok(files.len())
}

/// Called only after the publication's projection has committed, while the
/// archive writer is held. The journal remains until this transaction commits.
pub(crate) fn published(db: &mut Connection, archive: &Path, paths: &[String]) -> Result<()> {
    let tx = db.transaction().map_err(storage)?;
    published_in_transaction(&tx, archive, paths)?;
    tx.commit().map_err(storage)
}

pub(crate) fn published_in_transaction(
    db: &Connection,
    archive: &Path,
    paths: &[String],
) -> Result<()> {
    for path in paths {
        publication::validate_relative_path(path)?;
        if tracked(Path::new(path)) {
            db.execute("INSERT INTO archive_index_files VALUES(?,?) ON CONFLICT(path) DO UPDATE SET stamp=excluded.stamp",
                params![path, stamp(&archive.join(path))?]).map_err(storage)?;
        }
    }
    seal(db, archive)
}

/// Retain corrupt or obsolete catalog files for diagnosis. Private state lives
/// in a separate database and is never replaced by projection recovery.
pub(crate) fn open_catalog(state: &Path) -> Result<Connection> {
    let path = state.join("catalog.sqlite");
    let db = Connection::open(&path).map_err(storage)?;
    let check = db.query_row("PRAGMA quick_check(1)", [], |row| row.get::<_, String>(0));
    let mut corrupt = match check {
        Ok(value) => value != "ok",
        Err(rusqlite::Error::SqliteFailure(error, _))
            if matches!(
                error.code,
                rusqlite::ErrorCode::DatabaseCorrupt | rusqlite::ErrorCode::NotADatabase
            ) =>
        {
            true
        }
        Err(error) => return Err(storage(error)),
    };
    let mut obsolete = false;
    if !corrupt {
        let version: u32 = db
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .map_err(storage)?;
        // Catalog rows are disposable. Recreate the SQLite schema before replay,
        // rather than applying current inserts to tables from an older release.
        obsolete = version != 0 && version < CATALOG_SCHEMA;
        if version == CATALOG_SCHEMA {
            obsolete = !music_search::schema_is_current(&db)?;
            let mut required = TABLES
                .iter()
                .map(|name| name.to_string())
                .collect::<Vec<_>>();
            required.extend(
                [
                    "search",
                    "search_content",
                    "catalog_checkpoint",
                    "catalog_index_state",
                ]
                .map(String::from),
            );
            for table in TABLES {
                for operation in ["INSERT", "UPDATE", "DELETE"] {
                    required.push(format!("checkpoint_{table}_{operation}"));
                }
            }
            for operation in ["INSERT", "UPDATE", "DELETE"] {
                required.push(format!("checkpoint_search_{operation}"));
                for table in ["items", "works", "editions", "captures", "assets"] {
                    required.push(format!("references_{table}_{operation}"));
                }
            }
            for name in required {
                let exists: bool = db
                    .query_row(
                        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name=?1)",
                        [name],
                        |row| row.get(0),
                    )
                    .map_err(storage)?;
                if !exists {
                    corrupt = true;
                    break;
                }
            }
        }
    }
    if !corrupt && !obsolete {
        return Ok(db);
    }
    drop(db);
    let reason = if corrupt { "corrupt" } else { "obsolete" };
    let suffix = TransactionId::new();
    for name in ["catalog.sqlite", "catalog.sqlite-wal", "catalog.sqlite-shm"] {
        let source = state.join(name);
        if source.try_exists().map_err(storage)? {
            fs::rename(source, state.join(format!("{name}.{reason}-{suffix}"))).map_err(storage)?;
        }
    }
    sync_directory(state)?;
    tracing::warn!(
        reason,
        "catalog retained; rebuilding projection from archive"
    );
    Connection::open(path).map_err(storage)
}
