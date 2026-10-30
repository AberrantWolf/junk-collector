//! Command-scoped persistence. Private effects and receipts share a SQLite commit.
//! Filesystem effects prepare an exact response against a publication marker;
//! startup resolves that marker before accepting requests. Nothing private is
//! copied into the portable archive.
use super::*;
use collector_app::CommandReceipt;

impl Store {
    pub(crate) fn with_command_result(&self, result: &impl Serialize) -> Result<Self> {
        Ok(Self {
            command_result: Some(serde_json::to_value(result).map_err(storage)?),
            ..self.clone()
        })
    }

    pub(crate) fn record_command(&self, db: &Connection, result: &impl Serialize) -> Result<()> {
        if let Some(receipt) = &self.receipt {
            complete(db, receipt, &serde_json::to_value(result).map_err(storage)?)?;
        }
        Ok(())
    }

    pub(crate) fn prepare_command(
        &self,
        root: &str,
        path: &str,
        sha256: &str,
        result: &impl Serialize,
    ) -> Result<()> {
        let mut db = Connection::open_in_memory().map_err(storage)?;
        db.execute(
            "ATTACH DATABASE ? AS private",
            [self.state.join("private.sqlite").to_string_lossy().as_ref()],
        )
        .map_err(storage)?;
        db.busy_timeout(std::time::Duration::from_secs(5))
            .map_err(storage)?;
        let tx = db
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(storage)?;
        // Archive publishers hold the writer lock. Settle a failed reservation
        // before another attempt can claim the same immutable revision path.
        if root == "archive" {
            release_absent_metadata_outcomes(
                &tx,
                &self.archive,
                Some(path),
                self.receipt.as_ref(),
            )?;
        }
        let Some(receipt) = &self.receipt else {
            return tx.commit().map_err(storage);
        };
        tx.execute("INSERT INTO private.command_outcomes(user_id,api_major,operation,idempotency_key,root,path,sha256,result) VALUES(?,?,?,?,?,?,?,?) ON CONFLICT(user_id,api_major,operation,idempotency_key) DO UPDATE SET root=excluded.root,path=excluded.path,sha256=excluded.sha256,result=excluded.result",
            params![receipt.user.to_string(),receipt.api_major,receipt.operation,receipt.key,root,path,sha256,serde_json::to_string(result).map_err(storage)?]).map_err(storage)?;
        tx.commit().map_err(storage)
    }
}

pub(super) fn release_absent_metadata_outcomes(
    db: &Connection,
    archive: &Path,
    target: Option<&str>,
    active: Option<&CommandReceipt>,
) -> Result<()> {
    let rows = {
        let mut statement = db.prepare("SELECT r.user_id,r.api_major,r.operation,r.idempotency_key,r.request_sha256,o.path FROM private.command_receipts r JOIN private.command_outcomes o USING(user_id,api_major,operation,idempotency_key) WHERE r.state='in_progress' AND r.operation='metadata.authority.update' AND o.root='archive' AND (? IS NULL OR o.path=?)").map_err(storage)?;
        statement
            .query_map(params![target, target], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, u16>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, String>(5)?,
                ))
            })
            .map_err(storage)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(storage)?
    };
    for (user, major, operation, key, digest, path) in rows {
        if active.is_some_and(|r| {
            r.user.to_string() == user
                && r.api_major == major
                && r.operation == operation
                && r.key == key
        }) {
            continue;
        }
        publication::validate_relative_path(&path)?;
        if archive.join(&path).try_exists().map_err(storage)? {
            continue;
        }
        if fs::read_dir(archive.join(".transactions"))
            .map_err(storage)?
            .next()
            .is_some()
        {
            return Err(Error::Unavailable(
                "unfinished archive publication; restart to settle recovery before retrying".into(),
            ));
        }
        db.execute("DELETE FROM private.command_outcomes WHERE user_id=? AND api_major=? AND operation=? AND idempotency_key=?",params![user,major,operation,key]).map_err(storage)?;
        db.execute("DELETE FROM private.command_receipts WHERE user_id=? AND api_major=? AND operation=? AND idempotency_key=? AND request_sha256=? AND state='in_progress'",params![user,major,operation,key,digest]).map_err(storage)?;
    }
    Ok(())
}

// Existing archives can already contain a losing reservation from a failed save.
// Accept an alternate winner only with a coherent immutable metadata revision AND
// an independent durable command attesting that record. Other mismatches fail.
fn attested_metadata_winner(
    db: &Connection,
    archive: &Path,
    path: &Path,
    bytes: &[u8],
    digest: &str,
    result: &str,
    receipt: &CommandReceipt,
) -> Result<bool> {
    if receipt.operation != "metadata.authority.update" {
        return Ok(false);
    }
    let expected: collector_app::MetadataAuthorityRecord =
        serde_json::from_str(result).map_err(storage)?;
    let actual: collector_app::MetadataAuthorityRecord =
        serde_json::from_slice(bytes).map_err(storage)?;
    if actual.collection_id != expected.collection_id
        || actual.item_id != expected.item_id
        || actual.revision != expected.revision
    {
        return Ok(false);
    }
    let suffix = format!(
        "metadata_authorities/{}/revisions/{:020}/record.v4.json",
        actual.item_id, actual.revision.0
    );
    if !path.ends_with(&suffix) {
        return Ok(false);
    }
    let manifest: processing::ManifestV4 = toml::from_str(
        &fs::read_to_string(path.with_file_name("manifest.v4.toml")).map_err(storage)?,
    )
    .map_err(storage)?;
    if manifest.schema_version != 4
        || manifest.entity_type != "metadata_authority"
        || manifest.entity_id != actual.item_id.to_string()
        || manifest.collection_id != actual.collection_id
        || manifest.revision != actual.revision
        || manifest.record_sha256 != digest
    {
        return Ok(false);
    }
    actual.validate()?;
    let actual_value = serde_json::from_slice::<serde_json::Value>(bytes).map_err(storage)?;
    let canonical = serde_json::to_string(&actual_value).map_err(storage)?;
    let completed: bool = db.query_row("SELECT EXISTS(SELECT 1 FROM private.command_receipts WHERE api_major=? AND operation=? AND state='completed' AND result=?)",params![receipt.api_major,receipt.operation,canonical],|row|row.get(0)).map_err(storage)?;
    if completed {
        return Ok(true);
    }
    let mut statement=db.prepare("SELECT o.result FROM private.command_receipts r JOIN private.command_outcomes o USING(user_id,api_major,operation,idempotency_key) WHERE r.api_major=? AND r.operation=? AND r.state='in_progress' AND o.root='archive' AND o.path=? AND o.sha256=?").map_err(storage)?;
    let relative = path.strip_prefix(archive).map_err(storage)?;
    for row in statement
        .query_map(
            params![
                receipt.api_major,
                receipt.operation,
                relative
                    .to_str()
                    .ok_or_else(|| Error::Storage("metadata recovery path is not UTF-8".into()))?,
                digest
            ],
            |row| row.get::<_, String>(0),
        )
        .map_err(storage)?
    {
        if serde_json::from_str::<serde_json::Value>(&row.map_err(storage)?).map_err(storage)?
            == actual_value
        {
            return Ok(true);
        }
    }
    Ok(false)
}

pub(super) fn complete(
    db: &Connection,
    receipt: &CommandReceipt,
    result: &serde_json::Value,
) -> Result<()> {
    let encoded = serde_json::to_string(result).map_err(storage)?;
    let changed = db.execute("UPDATE private.command_receipts SET state='completed',result=?,updated_at=? WHERE user_id=? AND api_major=? AND operation=? AND idempotency_key=? AND request_sha256=? AND (state='in_progress' OR (state='completed' AND result=?))",
        params![encoded,Utc::now().timestamp(),receipt.user.to_string(),receipt.api_major,receipt.operation,receipt.key,receipt.request_sha256,encoded]).map_err(storage)?;
    if changed != 1 {
        return Err(Error::Storage(
            "command receipt completion lost ownership or changed its result".into(),
        ));
    }
    db.execute("DELETE FROM private.command_outcomes WHERE user_id=? AND api_major=? AND operation=? AND idempotency_key=?",
        params![receipt.user.to_string(),receipt.api_major,receipt.operation,receipt.key]).map_err(storage)?;
    Ok(())
}

pub(super) fn recover(db: &mut Connection, archive: &Path, state: &Path) -> Result<()> {
    let rows = {
        let mut stmt = db.prepare("SELECT r.user_id,r.api_major,r.operation,r.idempotency_key,r.request_sha256,o.root,o.path,o.sha256,o.result FROM private.command_receipts r JOIN private.command_outcomes o USING(user_id,api_major,operation,idempotency_key) WHERE r.state='in_progress'").map_err(storage)?;
        stmt.query_map([], |row| {
            Ok((
                CommandReceipt {
                    user: row
                        .get::<_, String>(0)?
                        .parse()
                        .map_err(|_| rusqlite::Error::InvalidQuery)?,
                    api_major: row.get(1)?,
                    operation: row.get(2)?,
                    key: row.get(3)?,
                    request_sha256: row.get(4)?,
                },
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, String>(8)?,
            ))
        })
        .map_err(storage)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(storage)?
    };
    let tx = db.transaction().map_err(storage)?;
    for (receipt, root, path, digest, result) in rows {
        if Path::new(&path)
            .components()
            .any(|part| !matches!(part, std::path::Component::Normal(_)))
        {
            return Err(Error::Storage("unsafe command recovery path".into()));
        }
        let archive_marker = root == "archive";
        let root = match root.as_str() {
            "archive" => archive,
            "state" => state,
            _ => return Err(Error::Storage("unknown command recovery root".into())),
        };
        let path = root.join(path);
        if path.try_exists().map_err(storage)? {
            let bytes = fs::read(&path).map_err(storage)?;
            let actual_digest = Digest::sha256(&bytes).hex;
            if actual_digest != digest {
                if archive_marker
                    && attested_metadata_winner(
                        &tx,
                        archive,
                        &path,
                        &bytes,
                        &actual_digest,
                        &result,
                        &receipt,
                    )?
                {
                    continue;
                }
                return Err(Error::Storage(
                    "command commit marker digest mismatch".into(),
                ));
            }
            if archive_marker
                && receipt.operation == "metadata.authority.update"
                && serde_json::from_slice::<serde_json::Value>(&bytes).map_err(storage)?
                    != serde_json::from_str::<serde_json::Value>(&result).map_err(storage)?
            {
                return Err(Error::Storage(
                    "metadata command result differs from its published record".into(),
                ));
            }
            complete(
                &tx,
                &receipt,
                &serde_json::from_str(&result).map_err(storage)?,
            )?;
        }
    }
    tx.execute("DELETE FROM private.command_outcomes WHERE (user_id,api_major,operation,idempotency_key) IN (SELECT user_id,api_major,operation,idempotency_key FROM private.command_receipts WHERE state='in_progress')",[]).map_err(storage)?;
    tx.execute(
        "DELETE FROM private.command_receipts WHERE state='in_progress'",
        [],
    )
    .map_err(storage)?;
    tx.commit().map_err(storage)
}
