//! Durable shared provider execution. Internal Jobs use an unassignable nil owner;
//! consumer Jobs and their receipts remain requester-private.
use super::*;
use collector_app::{FrozenAcquisitionJob, FrozenJobInput, JobClaim, JobRecord};
use rusqlite::TransactionBehavior;

impl Store {
    pub(crate) fn adopt_existing_acquisitions(tx: &Connection) -> Result<()> {
        let mut after: Option<(i64, String)> = None;
        loop {
            let rows = {
                let mut stmt = tx.prepare("SELECT j.id,j.created_at,p.frozen_input FROM private.jobs j JOIN private.processing_jobs p ON p.job_id=j.id WHERE j.kind IN ('catalog_acquisition','metadata_resolution') AND j.state IN ('queued','running') AND NOT EXISTS(SELECT 1 FROM private.acquisition_links l WHERE l.owner_job=j.id) AND (? IS NULL OR j.created_at>? OR (j.created_at=? AND j.id>?)) ORDER BY j.created_at,j.id LIMIT 256").map_err(storage)?;
                let time = after.as_ref().map(|v| v.0);
                let id = after.as_ref().map(|v| &v.1);
                stmt.query_map(params![time, time, time, id], |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, i64>(1)?,
                        r.get::<_, String>(2)?,
                    ))
                })
                .map_err(storage)?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(storage)?
            };
            if rows.is_empty() {
                break;
            }
            after = rows.last().map(|(id, time, _)| (*time, id.clone()));
            for (job, created, raw) in rows {
                let Ok(input) = serde_json::from_str::<FrozenJobInput>(&raw) else {
                    continue;
                };
                if serde_json::to_value(&input).map_err(storage)?
                    != serde_json::from_str::<serde_json::Value>(&raw).map_err(storage)?
                {
                    continue;
                }
                let frozen = match input {
                    FrozenJobInput::Acquisition(input) => input,
                    FrozenJobInput::MetadataResolution(input)
                        if input.reviewed_digital_match.is_none() =>
                    {
                        input.acquisition
                    }
                    _ => continue,
                };
                Self::link_acquisition(tx, job.parse().map_err(storage)?, &frozen, created, false)?;
                tx.execute("INSERT OR REPLACE INTO private.acquisition_waits(owner_job,acquisition_job) SELECT owner_job,acquisition_job FROM private.acquisition_links WHERE owner_job=? AND input_key=?",params![job,collector_app::acquisition_key(&frozen)?]).map_err(storage)?;
                tx.execute("UPDATE private.jobs SET phase=CASE WHEN (SELECT COUNT(*) FROM private.acquisition_links WHERE acquisition_job=(SELECT acquisition_job FROM private.acquisition_waits WHERE owner_job=jobs.id))>1 THEN 'waiting_shared_provider' ELSE 'waiting_provider' END WHERE id=? AND state='queued'",[job]).map_err(storage)?;
            }
        }
        Ok(())
    }

    pub(crate) fn attach_acquisition(
        tx: &Connection,
        owner: JobId,
        frozen: &FrozenAcquisitionJob,
        now: i64,
    ) -> Result<()> {
        Self::link_acquisition(tx, owner, frozen, now, true)
    }

    fn link_acquisition(
        tx: &Connection,
        owner: JobId,
        frozen: &FrozenAcquisitionJob,
        now: i64,
        enforce_owner_limit: bool,
    ) -> Result<()> {
        let key = collector_app::acquisition_key(frozen)?;
        let existing: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM private.acquisition_links WHERE owner_job=? AND input_key=?)",params![owner.to_string(),key],|r|r.get(0)).map_err(storage)?;
        if existing {
            return Ok(());
        }
        let active: Option<(String,String)> = tx.query_row("SELECT j.id,j.state FROM private.jobs j WHERE j.kind='provider_acquisition' AND json_extract(j.request,'$.input_key')=? AND j.state IN ('queued','running','cancel_requested') ORDER BY j.created_at,j.id LIMIT 1",[&key],|r|Ok((r.get(0)?,r.get(1)?))).optional().map_err(storage)?;
        let id = match active {
            Some((_, state)) if state == "cancel_requested" => {
                return Err(Error::CommandInProgress);
            }
            Some((id, _)) => {
                let count:u32=tx.query_row("SELECT COUNT(*) FROM private.acquisition_links l JOIN private.jobs j ON j.id=l.owner_job WHERE l.acquisition_job=? AND j.state IN ('queued','running') AND j.cancellation_requested=0",[&id],|r|r.get(0)).map_err(storage)?;
                if count == 0 {
                    return Err(Error::CommandInProgress);
                }
                if enforce_owner_limit && count >= 256 {
                    return Err(Error::Unavailable(
                        "This provider download already has 256 request owners".into(),
                    ));
                }
                id
            }
            None => {
                let id = JobId::new().to_string();
                let mut input = frozen.clone();
                input.item_id = None;
                // The first admitted snapshot/time stays immutable on convergence.
                tx.execute("INSERT INTO private.jobs(id,user_id,collection_id,kind,state,phase,completed_units,total_units,attempt,created_at,updated_at,request,cancellation_requested) VALUES(?,?,?,'provider_acquisition','queued','queued',0,?,1,?,?,?,0)",params![id,uuid::Uuid::nil().to_string(),frozen.request.collection_id.to_string(),frozen.request.maximum_response_bytes,now,now,serde_json::json!({"input_key":key,"snapshot_id":input.snapshot_id}).to_string()]).map_err(storage)?;
                tx.execute(
                    "INSERT INTO private.processing_jobs(job_id,frozen_input) VALUES(?,?)",
                    params![
                        id,
                        serde_json::to_string(&FrozenJobInput::Acquisition(input))
                            .map_err(storage)?
                    ],
                )
                .map_err(storage)?;
                id
            }
        };
        tx.execute("INSERT INTO private.acquisition_links(owner_job,input_key,acquisition_job) VALUES(?,?,?)",params![owner.to_string(),key,id]).map_err(storage)?;
        tx.execute("UPDATE private.jobs SET phase='waiting_shared_provider' WHERE state='queued' AND phase='waiting_provider' AND id IN (SELECT owner_job FROM private.acquisition_links WHERE acquisition_job=? AND (SELECT COUNT(*) FROM private.acquisition_links WHERE acquisition_job=?)>1)",params![id,id]).map_err(storage)?;
        Ok(())
    }

    pub(crate) fn acquire_for_owner(
        &self,
        claim: &JobClaim,
        frozen: &FrozenAcquisitionJob,
    ) -> Result<Option<SnapshotId>> {
        let mut db = lock(&self.db)?;
        let tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage)?;
        let live: bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM private.jobs WHERE id=? AND claim_token=? AND state='running' AND cancellation_requested=0 AND lease_expires>?)",params![claim.id.to_string(),claim.claim_token,self.clock.now().timestamp()],|r|r.get(0)).map_err(storage)?;
        if !live {
            return Err(Error::JobCancelled);
        }
        Self::attach_acquisition(&tx, claim.id, frozen, self.clock.now().timestamp())?;
        let key = collector_app::acquisition_key(frozen)?;
        let (state,result,error):(String,Option<String>,Option<String>)=tx.query_row("SELECT j.state,j.result,j.error FROM private.acquisition_links l JOIN private.jobs j ON j.id=l.acquisition_job WHERE l.owner_job=? AND l.input_key=?",params![claim.id.to_string(),key],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).map_err(storage)?;
        match state.as_str() {
            "completed" => {
                let value: serde_json::Value =
                    serde_json::from_str(result.as_deref().ok_or(Error::Conflict)?)
                        .map_err(storage)?;
                let snapshot: SnapshotId = value
                    .get("snapshot_id")
                    .and_then(|v| v.as_str())
                    .ok_or(Error::Conflict)?
                    .parse()
                    .map_err(storage)?;
                Self::bind_snapshot(&tx, claim.id, snapshot)?;
                tx.execute("DELETE FROM private.acquisition_waits WHERE owner_job=? AND acquisition_job=(SELECT acquisition_job FROM private.acquisition_links WHERE owner_job=? AND input_key=?)",params![claim.id.to_string(),claim.id.to_string(),key]).map_err(storage)?;
                tx.execute("UPDATE private.acquisition_links SET snapshot_id=? WHERE owner_job=? AND input_key=?",params![snapshot.to_string(),claim.id.to_string(),key]).map_err(storage)?;
                tx.commit().map_err(storage)?;
                Ok(Some(snapshot))
            }
            "failed" => Err(Error::Unavailable(
                error.unwrap_or_else(|| "Provider acquisition failed".into()),
            )),
            "cancelled" | "cancel_requested" => Err(Error::JobCancelled),
            _ => {
                tx.execute("INSERT OR REPLACE INTO private.acquisition_waits(owner_job,acquisition_job) SELECT owner_job,acquisition_job FROM private.acquisition_links WHERE owner_job=? AND input_key=?",params![claim.id.to_string(),key]).map_err(storage)?;
                tx.execute("UPDATE private.jobs SET state='queued',phase=CASE WHEN (SELECT COUNT(*) FROM private.acquisition_links l WHERE l.acquisition_job=(SELECT acquisition_job FROM private.acquisition_waits WHERE owner_job=jobs.id))>1 THEN 'waiting_shared_provider' ELSE 'waiting_provider' END,worker_instance=NULL,claim_token=NULL,lease_expires=NULL WHERE id=? AND claim_token=? AND state='running'",params![claim.id.to_string(),claim.claim_token]).map_err(storage)?;
                tx.commit().map_err(storage)?;
                Ok(None)
            }
        }
    }

    pub(crate) fn bind_snapshot(tx: &Connection, job: JobId, snapshot: SnapshotId) -> Result<()> {
        tx.execute("INSERT OR IGNORE INTO private.snapshot_bindings(job_id,snapshot_id,item_id,generation) SELECT j.id,?,j.item_id,COALESCE(g.generation,0) FROM private.jobs j LEFT JOIN private.item_job_generations g ON g.job_id=j.id WHERE j.id=?",params![snapshot.to_string(),job.to_string()]).map_err(storage)?;
        Ok(())
    }

    pub(crate) fn provider_owners(&self, claim: &JobClaim) -> Result<Vec<JobRecord>> {
        let ids = {
            let db = lock(&self.db)?;
            let valid:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM private.jobs WHERE id=? AND claim_token=? AND state='running' AND lease_expires>?)",params![claim.id.to_string(),claim.claim_token,self.clock.now().timestamp()],|r|r.get(0)).map_err(storage)?;
            if !valid {
                return Err(Error::JobCancelled);
            }
            let mut stmt=db.prepare("SELECT j.id,j.user_id FROM private.acquisition_links l JOIN private.jobs j ON j.id=l.owner_job WHERE l.acquisition_job=? AND l.snapshot_id IS NULL AND j.state IN ('queued','running') AND j.cancellation_requested=0 ORDER BY j.created_at,j.id").map_err(storage)?;
            stmt.query_map([claim.id.to_string()], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            })
            .map_err(storage)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(storage)?
        };
        let mut owners = Vec::new();
        for (id, user) in ids {
            let id = id.parse().map_err(storage)?;
            let obsolete = {
                let db = lock(&self.db)?;
                crate::jobs::job_generation_obsolete(&db, id)?
            };
            if !obsolete {
                owners.push(self.job_record(id, user.parse().map_err(storage)?)?);
            }
        }
        Ok(owners)
    }

    pub(crate) fn reconcile_provider_work(&self) -> Result<()> {
        let mut db = lock(&self.db)?;
        let tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage)?;
        // Remove only unsettled ownership; completed bindings remain for uncertain retries.
        tx.execute("DELETE FROM private.acquisition_links WHERE snapshot_id IS NULL AND owner_job IN (SELECT j.id FROM private.jobs j LEFT JOIN items i ON i.id=j.item_id LEFT JOIN private.item_job_generations g ON g.job_id=j.id WHERE j.state IN ('cancelled','cancel_requested','failed') OR j.cancellation_requested=1 OR (i.id IS NOT NULL AND (i.deleted=1 OR COALESCE(g.generation,0)<>COALESCE(json_extract(i.record,'$.lifecycle.generation'),0))))",[]).map_err(storage)?;
        tx.execute("UPDATE private.jobs SET state=CASE WHEN state='queued' THEN 'cancelled' ELSE 'cancel_requested' END,cancellation_requested=1 WHERE kind='provider_acquisition' AND state IN ('queued','running') AND NOT EXISTS(SELECT 1 FROM private.acquisition_links l JOIN private.jobs owner ON owner.id=l.owner_job WHERE l.acquisition_job=jobs.id AND owner.state IN ('queued','running') AND owner.cancellation_requested=0)",[]).map_err(storage)?;
        tx.execute("UPDATE private.jobs SET phase='queued' WHERE state='queued' AND phase IN ('waiting_provider','waiting_shared_provider') AND EXISTS(SELECT 1 FROM private.acquisition_waits w JOIN private.jobs a ON a.id=w.acquisition_job WHERE w.owner_job=jobs.id AND a.state IN ('completed','failed','cancelled'))",[]).map_err(storage)?;
        tx.commit().map_err(storage)
    }
}
