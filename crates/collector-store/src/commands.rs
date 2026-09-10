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
        let Some(receipt) = &self.receipt else {
            return Ok(());
        };
        let db = Connection::open_in_memory().map_err(storage)?;
        db.execute(
            "ATTACH DATABASE ? AS private",
            [self.state.join("private.sqlite").to_string_lossy().as_ref()],
        )
        .map_err(storage)?;
        db.busy_timeout(std::time::Duration::from_secs(5))
            .map_err(storage)?;
        db.execute("INSERT INTO private.command_outcomes(user_id,api_major,operation,idempotency_key,root,path,sha256,result) VALUES(?,?,?,?,?,?,?,?) ON CONFLICT(user_id,api_major,operation,idempotency_key) DO UPDATE SET root=excluded.root,path=excluded.path,sha256=excluded.sha256,result=excluded.result",
            params![receipt.user.to_string(),receipt.api_major,receipt.operation,receipt.key,root,path,sha256,serde_json::to_string(result).map_err(storage)?]).map_err(storage)?;
        Ok(())
    }
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
        let root = match root.as_str() {
            "archive" => archive,
            "state" => state,
            _ => return Err(Error::Storage("unknown command recovery root".into())),
        };
        let path = root.join(path);
        if path.exists() {
            if Digest::sha256(&fs::read(path).map_err(storage)?).hex != digest {
                return Err(Error::Storage(
                    "command commit marker digest mismatch".into(),
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
