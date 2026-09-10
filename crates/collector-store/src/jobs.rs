use super::*;
use collector_app::{JobClaim, JobSpec};
use rusqlite::TransactionBehavior;

type StoredJobIdentity = (String, String, Option<String>, Option<String>, String);

impl Store {
    pub(crate) fn enqueue_job_record(&self, spec: JobSpec) -> Result<JobId> {
        if spec.kind.is_empty()
            || spec.kind.len() > 120
            || spec.phase.is_empty()
            || spec.phase.len() > 120
        {
            return Err(Error::Invalid(
                "job kind and phase are required and bounded".into(),
            ));
        }
        let request = serde_json::to_string(&spec.request).map_err(storage)?;
        if request.len() > 16 * 1024 * 1024 {
            return Err(Error::Invalid("job request is too large".into()));
        }
        let now = self.clock.now().timestamp();
        let mut db = lock(&self.db)?;
        let tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage)?;
        let existing: Option<StoredJobIdentity> = tx
            .query_row(
                "SELECT user_id,kind,collection_id,item_id,request FROM private.jobs WHERE id=?",
                [spec.id.to_string()],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .optional()
            .map_err(storage)?;
        if let Some(existing) = existing {
            let intended = (
                spec.user_id.to_string(),
                spec.kind.clone(),
                spec.collection_id.map(|id| id.to_string()),
                spec.item_id.map(|id| id.to_string()),
                request.clone(),
            );
            if existing != intended {
                return Err(Error::IdempotencyConflict);
            }
        } else {
            tx.execute(
                "INSERT INTO private.jobs(
                  id,user_id,collection_id,item_id,kind,state,phase,
                  completed_units,total_units,attempt,created_at,updated_at,
                  error,limitation,request,result,worker_instance,lease_expires,
                  claim_token,cancellation_requested
                ) VALUES(?,?,?,?,?,'queued',?,0,?,1,?,?,NULL,NULL,?,NULL,NULL,NULL,NULL,0)",
                params![
                    spec.id.to_string(),
                    spec.user_id.to_string(),
                    spec.collection_id.map(|id| id.to_string()),
                    spec.item_id.map(|id| id.to_string()),
                    spec.kind,
                    spec.phase,
                    spec.total_units,
                    now,
                    now,
                    request,
                ],
            )
            .map_err(storage)?;
        }
        self.record_command(&tx, &spec.id)?;
        tx.commit().map_err(storage)?;
        Ok(spec.id)
    }

    pub(crate) fn claim_job_record(
        &self,
        worker_instance: &str,
        lease_seconds: i64,
    ) -> Result<Option<JobClaim>> {
        if worker_instance.is_empty()
            || worker_instance.len() > 128
            || !(1..=300).contains(&lease_seconds)
        {
            return Err(Error::Invalid("worker identity or lease is invalid".into()));
        }
        let now = self.clock.now().timestamp();
        let lease_expires = now.saturating_add(lease_seconds);
        let mut db = lock(&self.db)?;
        let tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage)?;
        let candidate: Option<String> = tx
            .query_row(
                "SELECT candidate.id FROM private.jobs candidate
                 WHERE (
                   candidate.state='queued' OR
                   (candidate.state='running' AND
                     (candidate.worker_instance IS NULL OR candidate.worker_instance<>? OR candidate.lease_expires<=?))
                 )
                 AND NOT EXISTS(
                   SELECT 1 FROM private.jobs active
                   WHERE active.user_id=candidate.user_id
                     AND active.state IN ('running','cancel_requested')
                     AND active.lease_expires>?
                     AND active.id<>candidate.id
                 )
                 AND (
                   candidate.state='running' OR
                   (SELECT COUNT(*) FROM private.jobs active
                    WHERE active.state IN ('running','cancel_requested')
                      AND active.lease_expires>?) < 2
                 )
                 ORDER BY candidate.created_at,candidate.id LIMIT 1",
                params![worker_instance, now, now, now],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage)?;
        let Some(id) = candidate else {
            tx.commit().map_err(storage)?;
            return Ok(None);
        };
        let claim_token = TransactionId::new().to_string();
        if tx
            .execute(
                "UPDATE private.jobs SET state='running',phase='starting',worker_instance=?,lease_expires=?,claim_token=?,updated_at=?
                 WHERE id=? AND (state='queued' OR (state='running' AND (worker_instance IS NULL OR worker_instance<>? OR lease_expires<=?)))",
                params![worker_instance, lease_expires, claim_token, now, id, worker_instance, now],
            )
            .map_err(storage)?
            != 1
        {
            return Err(Error::CommandInProgress);
        }
        let row: (String, Option<String>, String, String, u32, i64) = tx
            .query_row(
                "SELECT user_id,item_id,kind,request,attempt,created_at FROM private.jobs WHERE id=? AND claim_token=?",
                params![id, claim_token],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?)),
            )
            .map_err(storage)?;
        tx.commit().map_err(storage)?;
        let created_at = chrono::DateTime::from_timestamp(row.5, 0)
            .ok_or_else(|| Error::Storage("job creation timestamp is invalid".into()))?;
        Ok(Some(JobClaim {
            id: id.parse().map_err(storage)?,
            user_id: row.0.parse().map_err(storage)?,
            item_id: row
                .1
                .map(|value| value.parse().map_err(storage))
                .transpose()?,
            kind: row.2,
            request: serde_json::from_str(&row.3).map_err(storage)?,
            claim_token,
            attempt: row.4,
            created_at,
        }))
    }

    pub(crate) fn heartbeat_job_record(
        &self,
        id: JobId,
        claim_token: &str,
        phase: &str,
        completed_units: u64,
        lease_seconds: i64,
    ) -> Result<()> {
        if phase.is_empty() || phase.len() > 120 || !(1..=300).contains(&lease_seconds) {
            return Err(Error::Invalid("job heartbeat is out of bounds".into()));
        }
        let now = self.clock.now().timestamp();
        let changed = lock(&self.db)?
            .execute(
                "UPDATE private.jobs SET
                   phase=CASE WHEN phase<>? OR updated_at<? THEN ? ELSE phase END,
                   completed_units=CASE WHEN phase<>? OR updated_at<? THEN ? ELSE completed_units END,
                   lease_expires=?,
                   updated_at=CASE WHEN phase<>? OR updated_at<? THEN ? ELSE updated_at END
                 WHERE id=? AND claim_token=? AND state='running' AND completed_units<=? AND ?<=total_units",
                params![
                    phase,
                    now,
                    phase,
                    phase,
                    now,
                    completed_units,
                    now.saturating_add(lease_seconds),
                    phase,
                    now,
                    now,
                    id.to_string(),
                    claim_token,
                    completed_units,
                    completed_units,
                ],
            )
            .map_err(storage)?;
        if changed != 1 {
            return Err(Error::CommandInProgress);
        }
        Ok(())
    }

    pub(crate) fn renew_job_lease_record(
        &self,
        id: JobId,
        claim_token: &str,
        lease_seconds: i64,
    ) -> Result<()> {
        if !(1..=300).contains(&lease_seconds) {
            return Err(Error::Invalid("job lease is out of bounds".into()));
        }
        let now = self.clock.now().timestamp();
        let changed = lock(&self.db)?
            .execute(
                "UPDATE private.jobs SET lease_expires=?
                 WHERE id=? AND claim_token=? AND state='running'",
                params![
                    now.saturating_add(lease_seconds),
                    id.to_string(),
                    claim_token,
                ],
            )
            .map_err(storage)?;
        if changed != 1 {
            return Err(Error::CommandInProgress);
        }
        Ok(())
    }

    pub(crate) fn complete_job_record(
        &self,
        id: JobId,
        claim_token: &str,
        result: serde_json::Value,
    ) -> Result<()> {
        let result = serde_json::to_string(&result).map_err(storage)?;
        if result.len() > 1024 * 1024 {
            return Err(Error::Invalid("job result is too large".into()));
        }
        let changed = lock(&self.db)?
            .execute(
                "UPDATE private.jobs SET state='completed',phase='completed',completed_units=total_units,result=?,error=NULL,worker_instance=NULL,lease_expires=NULL,claim_token=NULL,updated_at=? WHERE id=? AND claim_token=? AND state='running'",
                params![result, self.clock.now().timestamp(), id.to_string(), claim_token],
            )
            .map_err(storage)?;
        if changed != 1 {
            return Err(Error::CommandInProgress);
        }
        Ok(())
    }

    pub(crate) fn fail_job_record(
        &self,
        id: JobId,
        claim_token: &str,
        message: &str,
    ) -> Result<()> {
        let message: String = message.chars().take(512).collect();
        let changed = lock(&self.db)?
            .execute(
                "UPDATE private.jobs SET state='failed',phase='failed',error=?,worker_instance=NULL,lease_expires=NULL,claim_token=NULL,updated_at=? WHERE id=? AND claim_token=? AND state='running'",
                params![message, self.clock.now().timestamp(), id.to_string(), claim_token],
            )
            .map_err(storage)?;
        if changed != 1 {
            return Err(Error::CommandInProgress);
        }
        Ok(())
    }
}
