use super::*;
use collector_app::{JobClaim, JobRecord, JobSpec};
use rusqlite::TransactionBehavior;

type StoredJobIdentity = (String, String, Option<String>, Option<String>, String);
type StoredJobRecord = (
    String,
    String,
    Option<String>,
    Option<String>,
    String,
    String,
    String,
    u64,
    u64,
    u32,
    i64,
    i64,
    Option<String>,
    Option<String>,
);

fn read_job_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<StoredJobRecord> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
        row.get(6)?,
        row.get(7)?,
        row.get(8)?,
        row.get(9)?,
        row.get(10)?,
        row.get(11)?,
        row.get(12)?,
        row.get(13)?,
    ))
}

fn parse_job_record(row: StoredJobRecord) -> Result<JobRecord> {
    Ok(JobRecord {
        id: row.0.parse().map_err(storage)?,
        user_id: row.1.parse().map_err(storage)?,
        collection_id: row
            .2
            .map(|value| value.parse().map_err(storage))
            .transpose()?,
        item_id: row
            .3
            .map(|value| value.parse().map_err(storage))
            .transpose()?,
        kind: row.4,
        state: crate::parse_job_state(&row.5)?,
        phase: row.6,
        completed_units: row.7,
        total_units: row.8,
        attempt: row.9,
        created_at: chrono::DateTime::from_timestamp(row.10, 0)
            .ok_or_else(|| Error::Storage("job creation timestamp is invalid".into()))?,
        updated_at: chrono::DateTime::from_timestamp(row.11, 0)
            .ok_or_else(|| Error::Storage("job update timestamp is invalid".into()))?,
        error: row.12,
        limitation: row.13,
    })
}

impl Store {
    pub(crate) fn reconcile_job_publications(&self) -> Result<()> {
        let candidates = {
            let db = lock(&self.db)?;
            let mut statement = db
                .prepare(
                    "SELECT id,kind,item_id FROM private.jobs
                     WHERE state IN ('running','cancel_requested')",
                )
                .map_err(storage)?;
            statement
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?,
                    ))
                })
                .map_err(storage)?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(storage)?
        };
        for (id, kind, item_id) in candidates {
            let job_id: JobId = id.parse().map_err(storage)?;
            let committed = if kind == "preservation_import" {
                publication::PublicationJournalStore::new(&self.archive)
                    .phase(TransactionId(job_id.0))?
                    .is_some_and(|phase| phase >= publication::PublicationPhase::RevisionCommitted)
                    || self
                        .upload_record(job_id)
                        .ok()
                        .and_then(|upload| {
                            self.item(upload.item_id).ok().map(|item| {
                                item.captures
                                    .iter()
                                    .any(|capture| capture.id == upload.capture_id)
                            })
                        })
                        .unwrap_or(false)
            } else if kind == "integrity_verification" {
                let Some(item_id) = item_id else { continue };
                let item_id: ItemId = item_id.parse().map_err(storage)?;
                let item = self.item(item_id)?;
                let expected = item
                    .captures
                    .iter()
                    .flat_map(|capture| &capture.representations)
                    .map(|representation| {
                        crate::files::deterministic_integrity_evidence_id(job_id, representation.id)
                    })
                    .collect::<Vec<_>>();
                let evidence = self.evidence(item_id)?;
                !expected.is_empty()
                    && expected
                        .iter()
                        .all(|id| evidence.iter().any(|value| value.id == *id))
            } else {
                false
            };
            if committed {
                lock(&self.db)?
                    .execute(
                        "UPDATE private.jobs SET state='running',phase='revision_committed',
                           cancellation_requested=0 WHERE id=?",
                        [id],
                    )
                    .map_err(storage)?;
            }
        }
        Ok(())
    }

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
                   (candidate.state='queued' AND candidate.phase='queued') OR
                   (candidate.state='running' AND
                     candidate.claim_token IS NOT NULL AND
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
                "UPDATE private.jobs SET state='running',
                   phase=CASE WHEN phase='revision_committed' THEN phase ELSE 'starting' END,
                   worker_instance=?,lease_expires=?,claim_token=?,updated_at=?
                 WHERE id=? AND (state='queued' OR (state='running' AND (worker_instance IS NULL OR worker_instance<>? OR lease_expires<=?)))",
                params![worker_instance, lease_expires, claim_token, now, id, worker_instance, now],
            )
            .map_err(storage)?
            != 1
        {
            return Err(Error::CommandInProgress);
        }
        let row: (
            String,
            Option<String>,
            String,
            String,
            u32,
            String,
            u64,
            i64,
        ) = tx
            .query_row(
                "SELECT user_id,item_id,kind,request,attempt,phase,completed_units,created_at
                 FROM private.jobs WHERE id=? AND claim_token=?",
                params![id, claim_token],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                        row.get(7)?,
                    ))
                },
            )
            .map_err(storage)?;
        tx.commit().map_err(storage)?;
        let created_at = chrono::DateTime::from_timestamp(row.7, 0)
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
            phase: row.5,
            completed_units: row.6,
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
        let db = lock(&self.db)?;
        let prior: Option<String> = db
            .query_row(
                "SELECT result FROM private.jobs WHERE id=? AND claim_token=? AND state='running'",
                params![id.to_string(), claim_token],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage)?
            .flatten();
        let prior = prior
            .as_deref()
            .map(serde_json::from_str::<serde_json::Value>)
            .transpose()
            .map_err(storage)?;
        let result = match prior {
            Some(mut history) if history.get("prior_failures").is_some() => {
                history["completion"] = result;
                history
            }
            _ => result,
        };
        let result = serde_json::to_string(&result).map_err(storage)?;
        if result.len() > 1024 * 1024 {
            return Err(Error::Invalid("job result is too large".into()));
        }
        let changed = db
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

    pub(crate) fn cancel_claimed_job_record(&self, id: JobId, claim_token: &str) -> Result<()> {
        let changed = lock(&self.db)?
            .execute(
                "UPDATE private.jobs SET state='cancelled',phase='cancelled',error=NULL,
                   worker_instance=NULL,lease_expires=NULL,claim_token=NULL,updated_at=?
                 WHERE id=? AND claim_token=? AND state='cancel_requested'",
                params![self.clock.now().timestamp(), id.to_string(), claim_token],
            )
            .map_err(storage)?;
        if changed != 1 {
            return Err(Error::CommandInProgress);
        }
        Ok(())
    }

    pub(crate) fn cancellation_requested_record(
        &self,
        id: JobId,
        claim_token: &str,
    ) -> Result<bool> {
        lock(&self.db)?
            .query_row(
                "SELECT state='cancel_requested' FROM private.jobs
                 WHERE id=? AND claim_token=? AND state IN ('running','cancel_requested')",
                params![id.to_string(), claim_token],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage)?
            .ok_or(Error::CommandInProgress)
    }

    pub(crate) fn request_job_cancellation_record(&self, id: JobId, user: UserId) -> Result<()> {
        let _publication_gate = lock(&self.job_publication_gate)?;
        if publication::PublicationJournalStore::new(&self.archive)
            .phase(TransactionId(id.0))?
            .is_some_and(|phase| phase >= publication::PublicationPhase::RevisionCommitted)
        {
            return Err(Error::CancellationTooLate);
        }
        let mut db = lock(&self.db)?;
        let tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage)?;
        let (state, phase, claim_token): (String, String, Option<String>) = tx
            .query_row(
                "SELECT state,phase,claim_token FROM private.jobs WHERE id=? AND user_id=?",
                params![id.to_string(), user.to_string()],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()
            .map_err(storage)?
            .ok_or(Error::NotFound)?;
        if state == "completed" || phase == "revision_committed" {
            return Err(Error::CancellationTooLate);
        }
        if matches!(state.as_str(), "running" | "cancel_requested") && claim_token.is_some() {
            tx.execute(
                "UPDATE private.jobs SET state='cancel_requested',cancellation_requested=1,updated_at=?
                 WHERE id=? AND claim_token IS NOT NULL AND state IN ('running','cancel_requested')",
                params![self.clock.now().timestamp(), id.to_string()],
            )
            .map_err(storage)?;
        } else if state != "cancelled" {
            tx.execute(
                "UPDATE private.jobs SET state='cancelled',phase='cancelled',
                   cancellation_requested=1,worker_instance=NULL,lease_expires=NULL,
                   claim_token=NULL,updated_at=? WHERE id=?",
                params![self.clock.now().timestamp(), id.to_string()],
            )
            .map_err(storage)?;
        }
        self.record_command(&tx, &())?;
        tx.commit().map_err(storage)?;
        Ok(())
    }

    pub(crate) fn job_record(&self, id: JobId, user: UserId) -> Result<JobRecord> {
        let row = lock(&self.db)?
            .query_row(
                "SELECT id,user_id,collection_id,item_id,kind,state,phase,
                   completed_units,total_units,attempt,created_at,updated_at,error,limitation
                 FROM private.jobs WHERE id=? AND user_id=?",
                params![id.to_string(), user.to_string()],
                read_job_record,
            )
            .optional()
            .map_err(storage)?
            .ok_or(Error::NotFound)?;
        parse_job_record(row)
    }

    pub(crate) fn job_records(
        &self,
        user: UserId,
        cursor: Option<JobId>,
        limit: u16,
    ) -> Result<Page<JobRecord>> {
        if limit == 0 || limit > 100 {
            return Err(Error::Invalid("job page limit must be 1–100".into()));
        }
        let db = lock(&self.db)?;
        let cursor = cursor
            .map(|id| {
                db.query_row(
                    "SELECT created_at,id FROM private.jobs WHERE id=? AND user_id=?",
                    params![id.to_string(), user.to_string()],
                    |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
                )
                .optional()
                .map_err(storage)?
                .ok_or(Error::NotFound)
            })
            .transpose()?;
        let mut statement = db
            .prepare(
                "SELECT id,user_id,collection_id,item_id,kind,state,phase,
                   completed_units,total_units,attempt,created_at,updated_at,error,limitation
                 FROM private.jobs
                 WHERE user_id=? AND (? IS NULL OR created_at<? OR (created_at=? AND id<?))
                 ORDER BY created_at DESC,id DESC LIMIT ?",
            )
            .map_err(storage)?;
        let (cursor_time, cursor_id) = cursor
            .map(|(time, id)| (Some(time), Some(id)))
            .unwrap_or((None, None));
        let rows = statement
            .query_map(
                params![
                    user.to_string(),
                    cursor_time,
                    cursor_time,
                    cursor_time,
                    cursor_id,
                    i64::from(limit) + 1,
                ],
                read_job_record,
            )
            .map_err(storage)?;
        let mut items = rows
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(storage)?
            .into_iter()
            .map(parse_job_record)
            .collect::<Result<Vec<_>>>()?;
        let next_cursor = if items.len() > usize::from(limit) {
            items.truncate(usize::from(limit));
            items.last().map(|job| job.id.to_string())
        } else {
            None
        };
        Ok(Page { items, next_cursor })
    }

    pub(crate) fn retry_failed_job_record(&self, id: JobId, user: UserId) -> Result<JobId> {
        let mut db = lock(&self.db)?;
        let tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage)?;
        let (kind, state, prior_result, error, attempt, failed_at): (
            String,
            String,
            Option<String>,
            Option<String>,
            u32,
            i64,
        ) = tx
            .query_row(
                "SELECT kind,state,result,error,attempt,updated_at FROM private.jobs
                 WHERE id=? AND user_id=?",
                params![id.to_string(), user.to_string()],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                    ))
                },
            )
            .optional()
            .map_err(storage)?
            .ok_or(Error::NotFound)?;
        if state != "failed"
            || !matches!(
                kind.as_str(),
                "integrity_verification" | "import_staging" | "import_analysis"
            )
        {
            return Err(Error::Conflict);
        }
        let mut result = prior_result
            .as_deref()
            .map(serde_json::from_str)
            .transpose()
            .map_err(storage)?
            .unwrap_or_else(|| serde_json::json!({"prior_failures":[]}));
        let failures = result
            .get_mut("prior_failures")
            .and_then(serde_json::Value::as_array_mut)
            .ok_or_else(|| Error::Storage("job failure history is malformed".into()))?;
        failures.push(serde_json::json!({
            "attempt": attempt,
            "error": error,
            "failed_at": failed_at,
        }));
        tx.execute(
            "UPDATE private.jobs SET state='queued',phase='queued',completed_units=0,
               attempt=attempt+1,error=NULL,result=?,cancellation_requested=0,
               worker_instance=NULL,lease_expires=NULL,claim_token=NULL,updated_at=?
             WHERE id=?",
            params![
                serde_json::to_string(&result).map_err(storage)?,
                self.clock.now().timestamp(),
                id.to_string()
            ],
        )
        .map_err(storage)?;
        self.record_command(&tx, &id)?;
        tx.commit().map_err(storage)?;
        Ok(id)
    }
}
