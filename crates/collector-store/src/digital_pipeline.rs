//! Durable matching continuations and disposable content-addressed computation cache.
use super::*;
use collector_app::ProcessingPort;
use rusqlite::TransactionBehavior;
use sha2::{Digest as _, Sha256};
use std::io::{Read, Seek, SeekFrom, Write};

pub(crate) fn install(db: &Connection) -> Result<()> {
    db.execute_batch("CREATE TABLE IF NOT EXISTS private.digital_checkpoints(job_id TEXT PRIMARY KEY,record TEXT NOT NULL);
    CREATE TABLE IF NOT EXISTS private.digital_children(parent TEXT NOT NULL,child TEXT NOT NULL,PRIMARY KEY(parent,child));
    CREATE INDEX IF NOT EXISTS private.digital_children_child ON digital_children(child,parent);
    CREATE TABLE IF NOT EXISTS private.digital_waits(parent TEXT NOT NULL,child TEXT NOT NULL,PRIMARY KEY(parent,child));
    CREATE TABLE IF NOT EXISTS private.audio_fingerprint_cache(key TEXT PRIMARY KEY,record TEXT NOT NULL,digest TEXT NOT NULL,used_at INTEGER NOT NULL);
    CREATE INDEX IF NOT EXISTS private.provider_request_cache ON jobs(json_extract(request,'$.input_key'),kind,state,updated_at);").map_err(storage)
}

pub(crate) fn reconcile(tx: &Connection) -> Result<()> {
    // Cancellation/failure of a workflow releases its children before provider ownership is reconciled.
    tx.execute("UPDATE private.jobs SET cancellation_requested=1,state=CASE WHEN state='queued' THEN 'cancelled' ELSE 'cancel_requested' END WHERE state IN ('queued','running') AND id IN (SELECT d.child FROM private.digital_children d JOIN private.jobs p ON p.id=d.parent WHERE p.state IN ('failed','cancelled','cancel_requested') OR p.cancellation_requested=1) AND NOT EXISTS(SELECT 1 FROM private.digital_children live JOIN private.jobs owner ON owner.id=live.parent WHERE live.child=jobs.id AND owner.state IN ('queued','running','completed') AND owner.cancellation_requested=0) AND NOT EXISTS(SELECT 1 FROM private.scan_requests r WHERE r.job_id=jobs.id AND r.kind<>'digital_batch')",[]).map_err(storage)?;
    tx.execute("DELETE FROM private.scan_requests WHERE kind='digital_batch' AND owner IN (SELECT id FROM private.jobs WHERE state IN ('failed','cancelled','cancel_requested') OR cancellation_requested=1)",[]).map_err(storage)?;
    tx.execute("UPDATE private.jobs SET phase='queued' WHERE state='queued' AND phase='waiting_digital' AND EXISTS(SELECT 1 FROM private.digital_waits w JOIN private.jobs c ON c.id=w.child WHERE w.parent=jobs.id AND c.state IN ('completed','failed','cancelled'))",[]).map_err(storage)?;
    Ok(())
}
impl Store {
    pub(crate) fn digital_checkpoint_record(
        &self,
        id: JobId,
        user: UserId,
    ) -> Result<Option<serde_json::Value>> {
        self.decode_digital_checkpoint(id, user, |raw| serde_json::from_str(raw).map_err(storage))
    }
    fn decode_digital_checkpoint<T>(
        &self,
        id: JobId,
        user: UserId,
        decode: impl FnOnce(&str) -> Result<T>,
    ) -> Result<Option<T>> {
        let raw: Option<String> = {
            let db = self.read_db()?;
            db.query_row("SELECT c.record FROM private.digital_checkpoints c JOIN private.jobs j ON j.id=c.job_id WHERE j.id=? AND j.user_id=?",params![id.to_string(),user.to_string()],|r|r.get(0)).optional().map_err(storage)?
        };
        raw.as_deref().map(decode).transpose()
    }
    pub(crate) fn wait_digital_record(
        &self,
        claim: &collector_app::JobClaim,
        checkpoint: serde_json::Value,
        waiting: &[JobId],
    ) -> Result<()> {
        let raw = serde_json::to_string(&checkpoint).map_err(storage)?;
        if raw.len() > 8 * 1024 * 1024 || waiting.len() > 512 {
            return Err(Error::Invalid("invalid matching checkpoint".into()));
        }
        let mut db = lock(&self.db)?;
        let tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage)?;
        let sql = if waiting.is_empty() {
            "UPDATE private.jobs SET updated_at=? WHERE id=? AND claim_token=? AND state='running' AND cancellation_requested=0 AND lease_expires>?"
        } else {
            "UPDATE private.jobs SET state='queued',phase='waiting_digital',worker_instance=NULL,claim_token=NULL,lease_expires=NULL,updated_at=? WHERE id=? AND claim_token=? AND state='running' AND cancellation_requested=0 AND lease_expires>?"
        };
        let changed = tx
            .execute(
                sql,
                params![
                    self.clock.now().timestamp(),
                    claim.id.to_string(),
                    claim.claim_token,
                    self.clock.now().timestamp()
                ],
            )
            .map_err(storage)?;
        if changed != 1 {
            return Err(Error::JobCancelled);
        }
        tx.execute(
            "INSERT OR REPLACE INTO private.digital_checkpoints VALUES(?,?)",
            params![claim.id.to_string(), raw],
        )
        .map_err(storage)?;
        tx.execute(
            "DELETE FROM private.digital_waits WHERE parent=?",
            [claim.id.to_string()],
        )
        .map_err(storage)?;
        for child in waiting {
            let owned:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM private.digital_children WHERE parent=? AND child=?)",params![claim.id.to_string(),child.to_string()],|r|r.get(0)).map_err(storage)?;
            if !owned {
                return Err(Error::Conflict);
            }
            tx.execute(
                "INSERT INTO private.digital_waits VALUES(?,?)",
                params![claim.id.to_string(), child.to_string()],
            )
            .map_err(storage)?;
        }
        if checkpoint
            .get("artwork_error")
            .is_some_and(|v| !v.is_null())
        {
            tx.execute("UPDATE private.jobs SET cancellation_requested=1,state=CASE WHEN state='queued' THEN 'cancelled' ELSE 'cancel_requested' END WHERE id IN (SELECT child FROM private.digital_children WHERE parent=?) AND substr(json_extract(request,'$.stage'),1,6)='cover:' AND state IN ('queued','running')",[claim.id.to_string()]).map_err(storage)?;
        }
        reconcile(&tx)?;
        tx.commit().map_err(storage)
    }
    pub(crate) fn stage_fingerprint_file(
        &self,
        collection: CollectionId,
        source: PackageScope,
        expected: &PackageFileFingerprint,
        checkpoint: &mut dyn FnMut() -> Result<()>,
    ) -> Result<Box<dyn collector_app::MediaReader>> {
        // At most eight processing workers stage one file each; scratch is anonymous and unlinked.
        const MAXIMUM: u64 = 4 * 1024 * 1024 * 1024;
        if expected.bytes.0 > MAXIMUM {
            return Err(Error::Invalid(
                "fingerprinting file exceeds 4 GiB scratch limit".into(),
            ));
        }
        checkpoint()?;
        let recorded = self.recorded_source(collection, source)?;
        if !recorded.files.contains(expected) {
            return Err(Error::Conflict);
        }
        let mut input = self.package_file(collection, source, expected.id)?;
        let mut scratch = tempfile::tempfile_in(&self.state).map_err(storage)?;
        let mut hasher = Sha256::new();
        let mut bytes = 0u64;
        let mut buffer = vec![0; 1024 * 1024];
        loop {
            checkpoint()?;
            let count = input.read(&mut buffer).map_err(storage)?;
            if count == 0 {
                break;
            }
            bytes = bytes.checked_add(count as u64).ok_or(Error::Conflict)?;
            if bytes > expected.bytes.0 {
                return Err(Error::Conflict);
            }
            hasher.update(&buffer[..count]);
            scratch.write_all(&buffer[..count]).map_err(storage)?;
        }
        checkpoint()?;
        if bytes != expected.bytes.0 || hex::encode(hasher.finalize()) != expected.sha256.hex {
            return Err(Error::Conflict);
        }
        scratch.seek(SeekFrom::Start(0)).map_err(storage)?;
        Ok(Box::new(scratch))
    }
    pub(crate) fn read_fingerprint_cache(&self, key: &str) -> Result<Option<RawJson>> {
        let db = lock(&self.db)?;
        let row: Option<(String, String)> = db
            .query_row(
                "SELECT record,digest FROM private.audio_fingerprint_cache WHERE key=?",
                [key],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
            .map_err(storage)?;
        let Some((raw, digest)) = row else {
            return Ok(None);
        };
        if Digest::sha256(raw.as_bytes()).hex != digest {
            db.execute(
                "DELETE FROM private.audio_fingerprint_cache WHERE key=?",
                [key],
            )
            .map_err(storage)?;
            return Ok(None);
        }
        db.execute(
            "UPDATE private.audio_fingerprint_cache SET used_at=? WHERE key=?",
            params![self.clock.now().timestamp(), key],
        )
        .map_err(storage)?;
        Ok(Some(RawJson::from_value(
            serde_json::from_str(&raw).map_err(storage)?,
        )))
    }
    pub(crate) fn write_fingerprint_cache(&self, key: &str, value: &RawJson) -> Result<()> {
        if key.len() != 64 || value.get().len() > 128 * 1024 {
            return Err(Error::Invalid("oversized fingerprint cache entry".into()));
        }
        let db = lock(&self.db)?;
        db.execute(
            "INSERT OR REPLACE INTO private.audio_fingerprint_cache VALUES(?,?,?,?)",
            params![
                key,
                value.get(),
                Digest::sha256(value.get().as_bytes()).hex,
                self.clock.now().timestamp()
            ],
        )
        .map_err(storage)?;
        db.execute("DELETE FROM private.audio_fingerprint_cache WHERE key IN (SELECT key FROM (SELECT key,ROW_NUMBER() OVER (ORDER BY used_at DESC,key) AS n,SUM(length(record)) OVER (ORDER BY used_at DESC,key) AS bytes FROM private.audio_fingerprint_cache) WHERE n>2048 OR bytes>33554432)",[]).map_err(storage)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture_job(store: &Store, user: UserId, kind: &str, phase: &str) -> JobId {
        let id = JobId::new();
        store
            .enqueue_job_record(collector_app::JobSpec {
                id,
                user_id: user,
                collection_id: None,
                item_id: None,
                kind: kind.into(),
                phase: phase.into(),
                total_units: 1,
                request: serde_json::json!({}),
                frozen_input: None,
            })
            .unwrap();
        id
    }
    #[test]
    fn checkpoint_decoding_leaves_catalog_available_and_keeps_owner_fence() {
        let root = tempfile::tempdir().unwrap();
        let store = Store::open(root.path().join("archive"), root.path().join("state")).unwrap();
        let user = UserId::new();
        let job = fixture_job(&store, user, "digital_album_matching", "waiting_digital");
        let raw =
            serde_json::to_string(&serde_json::json!({"payload":"x".repeat(4*1024*1024)})).unwrap();
        lock(&store.db)
            .unwrap()
            .execute(
                "INSERT INTO private.digital_checkpoints VALUES(?,?)",
                params![job.to_string(), raw],
            )
            .unwrap();
        let decoded = store
            .decode_digital_checkpoint(job, user, |raw| {
                // A catalog writer can acquire the mutex during potentially expensive decoding.
                let db = store
                    .db
                    .inner
                    .try_lock()
                    .expect("checkpoint decode holds the catalog mutex");
                db.execute(
                    "UPDATE private.jobs SET completed_units=1 WHERE id=?",
                    [job.to_string()],
                )
                .unwrap();
                drop(db);
                serde_json::from_str::<serde_json::Value>(raw).map_err(storage)
            })
            .unwrap()
            .unwrap();
        assert_eq!(decoded["payload"].as_str().unwrap().len(), 4 * 1024 * 1024);
        assert!(
            store
                .digital_checkpoint_record(job, UserId::new())
                .unwrap()
                .is_none()
        );
    }
    #[test]
    fn idle_workers_leave_workflow_recovery_to_the_periodic_owner() {
        let root = tempfile::tempdir().unwrap();
        let store =
            Arc::new(Store::open(root.path().join("archive"), root.path().join("state")).unwrap());
        let user = UserId::new();
        let digital = fixture_job(&store, user, "digital_album_matching", "waiting_digital");
        let game = fixture_job(&store, user, "game_catalog_scan", "waiting_games");
        let child = fixture_job(&store, user, "processing_operation", "completed");
        {
            let db = lock(&store.db).unwrap();
            db.execute(
                "UPDATE private.jobs SET state='completed' WHERE id=?",
                [child.to_string()],
            )
            .unwrap();
            db.execute(
                "INSERT INTO private.digital_waits VALUES(?,?)",
                params![digital.to_string(), child.to_string()],
            )
            .unwrap();
        }
        let service = collector_app::CollectionService::new(
            store.clone(),
            Arc::new(collector_modules::Registry::builtins()),
        );
        lock(&store.db)
            .unwrap()
            .pragma_update(None, "query_only", true)
            .unwrap();
        // Idle polling must work even when this connection cannot reserve a writer.
        assert!(!service.run_next_job("idle-worker").unwrap());
        lock(&store.db)
            .unwrap()
            .pragma_update(None, "query_only", false)
            .unwrap();
        assert_eq!(
            store.job_record(digital, user).unwrap().phase,
            "waiting_digital"
        );
        assert_eq!(store.job_record(game, user).unwrap().phase, "waiting_games");
        service.reconcile_background_jobs().unwrap();
        assert_eq!(store.job_record(digital, user).unwrap().phase, "queued");
        assert_eq!(store.job_record(game, user).unwrap().phase, "queued");
        for _ in 0..collector_app::JobPort::job_settings(&*store)
            .unwrap()
            .processing_workers
        {
            let running = fixture_job(&store, user, "processing_operation", "working");
            lock(&store.db).unwrap().execute("UPDATE private.jobs SET state='running',worker_instance='idle-worker',claim_token=?,lease_expires=? WHERE id=?", params![running.to_string(), store.clock.now().timestamp()+30, running.to_string()]).unwrap();
        }
        lock(&store.db)
            .unwrap()
            .pragma_update(None, "query_only", true)
            .unwrap();
        // Queued work with exhausted capacity must not reserve a writer either.
        assert!(!service.run_next_job("idle-worker").unwrap());
        lock(&store.db)
            .unwrap()
            .pragma_update(None, "query_only", false)
            .unwrap();
    }
    #[test]
    fn fingerprint_cache_rejects_corruption_and_bounds_storage() {
        let root = tempfile::tempdir().unwrap();
        let store = Store::open(root.path().join("archive"), root.path().join("state")).unwrap();
        let key = Digest::sha256(b"fingerprint-cache-fixture").hex;
        let value = RawJson::from_value(serde_json::json!({"fingerprint":"fixture"}));
        store.write_fingerprint_cache(&key, &value).unwrap();
        assert_eq!(
            store.read_fingerprint_cache(&key).unwrap().unwrap().get(),
            value.get()
        );
        lock(&store.db)
            .unwrap()
            .execute(
                "UPDATE private.audio_fingerprint_cache SET record='{}' WHERE key=?",
                [&key],
            )
            .unwrap();
        assert!(store.read_fingerprint_cache(&key).unwrap().is_none());
        {
            let mut db = lock(&store.db).unwrap();
            let tx = db.transaction().unwrap();
            let data = "x".repeat(65536);
            for n in 0..600 {
                tx.execute(
                    "INSERT INTO private.audio_fingerprint_cache VALUES(?,?,?,0)",
                    params![format!("{n:064x}"), data, "unused fixture digest"],
                )
                .unwrap();
            }
            tx.commit().unwrap();
        }
        store.write_fingerprint_cache(&key, &value).unwrap();
        let (count, bytes): (u64, u64) = lock(&store.db)
            .unwrap()
            .query_row(
                "SELECT count(*),sum(length(record)) FROM private.audio_fingerprint_cache",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert!(count <= 2048 && bytes <= 32 * 1024 * 1024);
    }
}
