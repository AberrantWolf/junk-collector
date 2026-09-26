use super::*;
use collector_app::{FrozenJobInput, JobClaim, JobRecord, JobSpec, ProvisionalImportPort};
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
        row.get(14)?,
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
        result: row
            .14
            .map(|value| serde_json::from_str(&value).map_err(storage))
            .transpose()?,
    })
}

impl Store {
    pub(crate) fn grouped_work_jobs(
        &self,
        user: UserId,
        query: &collector_app::WorkJobsQuery,
    ) -> Result<Page<JobRecord>> {
        let (kind, id) = match query.target {
            collector_app::WorkTarget::Import(id) => ("import", id.to_string()),
            collector_app::WorkTarget::Job(id) => ("job", id.to_string()),
        };
        let db = lock(&self.db)?;
        let mut stmt=db.prepare("SELECT id,user_id,collection_id,item_id,kind,state,phase,completed_units,total_units,attempt,created_at,updated_at,error,limitation,result FROM private.jobs WHERE user_id=?1 AND ((?2='import' AND json_extract(request,'$.import_id')=?3) OR (?2='job' AND (id=?3 OR id IN (SELECT job_id FROM private.preparation_members WHERE batch_id=?3)))) AND (?4 IS NULL OR id>?4) ORDER BY id LIMIT ?5").map_err(storage)?;
        let mut items = stmt
            .query_map(
                params![
                    user.to_string(),
                    kind,
                    id,
                    query.cursor.map(|id| id.to_string()),
                    u32::from(query.limit.unwrap_or(25)) + 1
                ],
                read_job_record,
            )
            .map_err(storage)?
            .map(|r| parse_job_record(r.map_err(storage)?))
            .collect::<Result<Vec<_>>>()?;
        let next_cursor = if items.len() > usize::from(query.limit.unwrap_or(25)) {
            items.pop();
            items.last().map(|j| j.id.to_string())
        } else {
            None
        };
        Ok(Page { items, next_cursor })
    }

    pub(crate) fn job_records_for_import(
        &self,
        import: ImportId,
        user: UserId,
    ) -> Result<Vec<JobRecord>> {
        let db = lock(&self.db)?;
        let mut statement = db
            .prepare(
                "SELECT id,user_id,collection_id,item_id,kind,state,phase,
                        completed_units,total_units,attempt,created_at,updated_at,
                        error,limitation,result
                 FROM private.jobs
                 WHERE user_id=? AND json_extract(request,'$.import_id')=?
                 ORDER BY created_at,id LIMIT 256",
            )
            .map_err(storage)?;
        let rows = statement
            .query_map(
                params![user.to_string(), import.to_string()],
                read_job_record,
            )
            .map_err(storage)?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(storage)?
            .into_iter()
            .map(parse_job_record)
            .collect()
    }

    pub(crate) fn active_derivative_job_records(
        &self,
        capture: CaptureId,
    ) -> Result<Vec<collector_app::ActiveDerivativeJob>> {
        let db = lock(&self.db)?;
        let mut statement = db
            .prepare(
                "SELECT j.id,j.user_id,p.frozen_input
                 FROM private.processing_jobs p
                 JOIN private.jobs j ON j.id=p.job_id
                 WHERE j.kind='music_derivative'
                   AND j.state IN ('queued','running','cancel_requested')
                   AND json_extract(j.request,'$.capture_id')=?
                 ORDER BY j.created_at,j.id LIMIT 1000",
            )
            .map_err(storage)?;
        let rows = statement
            .query_map([capture.to_string()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(storage)?;
        let mut active = Vec::new();
        for value in rows {
            let (id, user_id, value) = value.map_err(storage)?;
            let input: FrozenJobInput = serde_json::from_str(&value).map_err(storage)?;
            if let FrozenJobInput::Derivative(job) = input
                && job.capture_id == capture
            {
                job.validate()?;
                active.push(collector_app::ActiveDerivativeJob {
                    id: id.parse().map_err(storage)?,
                    user_id: user_id.parse().map_err(storage)?,
                    input: job,
                });
            }
        }
        Ok(active)
    }

    pub(crate) fn frozen_job_input_record(
        &self,
        id: JobId,
        user: UserId,
    ) -> Result<Option<FrozenJobInput>> {
        let value: Option<String> = lock(&self.db)?
            .query_row(
                "SELECT p.frozen_input FROM private.jobs j
                 LEFT JOIN private.processing_jobs p ON p.job_id=j.id
                 WHERE j.id=? AND j.user_id=?",
                params![id.to_string(), user.to_string()],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage)?
            .ok_or(Error::NotFound)?;
        value
            .map(|value| serde_json::from_str(&value).map_err(storage))
            .transpose()
    }

    pub(crate) fn preparation_batch_record_page(
        &self,
        user: UserId,
        item: ItemId,
        limit: u16,
    ) -> Result<Vec<JobRecord>> {
        if limit == 0 || limit > 100 {
            return Err(Error::Invalid("batch list limit must be 1–100".into()));
        }
        let db = lock(&self.db)?;
        let mut statement = db
            .prepare(
                "SELECT id,user_id,collection_id,item_id,kind,state,phase,
                   completed_units,total_units,attempt,created_at,updated_at,error,limitation,result
                 FROM private.jobs
                 WHERE user_id=? AND item_id=? AND kind='music_derivative_batch'
                 ORDER BY (cancellation_requested=0 AND (state IN ('queued','running','cancel_requested') OR EXISTS(
                   SELECT 1 FROM private.preparation_members m JOIN private.jobs child ON child.id=m.job_id
                   WHERE m.batch_id=private.jobs.id AND child.state IN ('queued','running','cancel_requested')))) DESC,
                   created_at DESC,id DESC LIMIT ?",
            )
            .map_err(storage)?;
        let rows = statement
            .query_map(
                params![user.to_string(), item.to_string(), limit],
                read_job_record,
            )
            .map_err(storage)?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(storage)?
            .into_iter()
            .map(parse_job_record)
            .collect()
    }

    fn preparation_batch_input(
        tx: &rusqlite::Transaction<'_>,
        id: JobId,
        user: UserId,
    ) -> Result<collector_app::FrozenDerivativeBatchJob> {
        let (kind, value): (String, String) = tx
            .query_row(
                "SELECT j.kind,p.frozen_input FROM private.jobs j
                 JOIN private.processing_jobs p ON p.job_id=j.id
                 WHERE j.id=? AND j.user_id=?",
                params![id.to_string(), user.to_string()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(storage)?
            .ok_or(Error::NotFound)?;
        if kind != "music_derivative_batch" {
            return Err(Error::NotFound);
        }
        let FrozenJobInput::DerivativeBatch(frozen) =
            serde_json::from_str(&value).map_err(storage)?
        else {
            return Err(Error::Storage(
                "batch frozen input has the wrong kind".into(),
            ));
        };
        frozen.validate()?;
        Ok(frozen)
    }

    pub(crate) fn cancel_preparation_batch_record(&self, id: JobId, user: UserId) -> Result<()> {
        let _publication_gate = lock(&self.job_publication_gate)?;
        let mut db = lock(&self.db)?;
        let tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage)?;
        Self::cancel_preparation_batch_in_tx(&tx, id, user, self.clock.now().timestamp())?;
        self.record_command(&tx, &())?;
        tx.commit().map_err(storage)
    }

    pub(crate) fn cancel_preparation_batch_in_tx(
        tx: &rusqlite::Transaction<'_>,
        id: JobId,
        user: UserId,
        now: i64,
    ) -> Result<()> {
        let frozen = Self::preparation_batch_input(tx, id, user)?;
        tx.execute(
            "UPDATE private.jobs SET cancellation_requested=1,
               state=CASE WHEN state='running' THEN 'cancel_requested'
                          WHEN state='queued' THEN 'cancelled' ELSE state END,
               phase=CASE WHEN state IN ('completed','failed','cancelled') THEN 'batch_cancelled'
                          ELSE phase END,updated_at=?
             WHERE id=? AND user_id=?",
            params![now, id.to_string(), user.to_string()],
        )
        .map_err(storage)?;
        for member in frozen.members {
            let collector_app::FrozenPreparationMember::Prepare { job_id, .. } = member else {
                continue;
            };
            let (direct, shared): (bool, bool) = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM private.derivative_direct_requests WHERE job_id=?1),
                   EXISTS(SELECT 1 FROM private.preparation_members m JOIN private.jobs j ON j.id=m.batch_id
                          WHERE m.job_id=?1 AND m.batch_id<>?2 AND j.cancellation_requested=0 AND j.state<>'cancelled')",
                params![job_id.to_string(),id.to_string()], |row| Ok((row.get(0)?,row.get(1)?))
            ).map_err(storage)?;
            if !collector_app::may_cancel_preparation_child(direct, shared) {
                continue;
            }
            tx.execute(
                "UPDATE private.jobs SET
                   state=CASE WHEN state='running' THEN 'cancel_requested'
                              WHEN state='queued' THEN 'cancelled' ELSE state END,
                   phase=CASE WHEN state='queued' THEN 'cancelled' ELSE phase END,
                   cancellation_requested=CASE WHEN state IN ('queued','running','cancel_requested') THEN 1 ELSE cancellation_requested END,
                   updated_at=?
                 WHERE id=? AND user_id=? AND state IN ('queued','running','cancel_requested')",
                params![now, job_id.to_string(), user.to_string()],
            )
            .map_err(storage)?;
        }
        Ok(())
    }

    pub(crate) fn retry_preparation_batch_record(&self, id: JobId, user: UserId) -> Result<()> {
        let mut db = lock(&self.db)?;
        let tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage)?;
        if tx.query_row("SELECT (EXISTS(SELECT 1 FROM items i WHERE i.id=COALESCE(jobs.item_id,(SELECT item_id FROM private.imports WHERE id=json_extract(jobs.request,'$.import_id'))) AND COALESCE(json_extract(i.record,'$.lifecycle.generation'),0)<>COALESCE((SELECT generation FROM private.item_job_generations g WHERE g.job_id=jobs.id),0)) AND NOT EXISTS(SELECT 1 FROM private.scan_requests r WHERE r.job_id=jobs.id AND r.kind='direct') AND NOT EXISTS(SELECT 1 FROM private.derivative_direct_requests r WHERE r.job_id=jobs.id) AND NOT (jobs.kind='import_publication' AND EXISTS(SELECT 1 FROM private.imports p JOIN captures c ON c.id=p.capture_id WHERE p.id=json_extract(jobs.request,'$.import_id')))) FROM private.jobs WHERE id=?",[id.to_string()],|row|row.get::<_,bool>(0)).optional().map_err(storage)?.unwrap_or(false) { return Err(Error::Conflict); }
        Self::ensure_work_retry_allowed(&tx, id, user)?;
        let frozen = Self::preparation_batch_input(&tx, id, user)?;
        ensure_album_admission_capacity(&tx, user, id)?;
        let now = self.clock.now().timestamp();
        let mut missing = false;
        for member in &frozen.members {
            let collector_app::FrozenPreparationMember::Prepare { job_id, .. } = member else {
                continue;
            };
            let state: Option<String> = tx
                .query_row(
                    "SELECT state FROM private.jobs WHERE id=? AND user_id=?",
                    params![job_id.to_string(), user.to_string()],
                    |row| row.get(0),
                )
                .optional()
                .map_err(storage)?;
            match state.as_deref() {
                Some("failed" | "cancelled") => {
                    requeue_failed_preparation(&tx, *job_id, user, now)?;
                }
                Some("cancel_requested") => return Err(Error::CommandInProgress),
                None => missing = true,
                _ => {}
            }
        }
        let (parent_state, parent_request): (String, String) = tx
            .query_row(
                "SELECT state,request FROM private.jobs WHERE id=? AND user_id=?",
                params![id.to_string(), user.to_string()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(storage)?;
        let request: serde_json::Value = serde_json::from_str(&parent_request).map_err(storage)?;
        ensure_import_allows_work(&tx, &request)?;
        if parent_state == "cancel_requested" {
            return Err(Error::CommandInProgress);
        }
        requeue_failed_preparation(&tx, id, user, now)?;
        if missing || matches!(parent_state.as_str(), "failed" | "cancelled") {
            tx.execute(
                "UPDATE private.jobs SET state='queued',phase='queued',completed_units=0,
                   error=NULL,cancellation_requested=0,worker_instance=NULL,
                   lease_expires=NULL,claim_token=NULL,updated_at=? WHERE id=? AND user_id=?",
                params![now, id.to_string(), user.to_string()],
            )
            .map_err(storage)?;
        } else {
            tx.execute(
                "UPDATE private.jobs SET cancellation_requested=0,
                   phase=CASE WHEN phase='batch_cancelled' THEN 'completed' ELSE phase END,
                   updated_at=? WHERE id=? AND user_id=?",
                params![now, id.to_string(), user.to_string()],
            )
            .map_err(storage)?;
        }
        let parent_request: serde_json::Value =
            serde_json::from_str(&parent_request).map_err(storage)?;
        if let Some(import) = parent_request
            .get("import_id")
            .and_then(serde_json::Value::as_str)
        {
            tx.execute(
                "UPDATE private.guided_ingest_intents
                 SET settled=0,preparation_error=NULL,updated_at=? WHERE import_id=?",
                params![now, import],
            )
            .map_err(storage)?;
        }
        self.record_command(&tx, &())?;
        tx.commit().map_err(storage)
    }

    pub(crate) fn reconcile_job_publications(&self) -> Result<()> {
        let candidates = {
            let db = lock(&self.db)?;
            let mut statement = db
                .prepare(
                    "SELECT id,kind,item_id,request,state FROM private.jobs
                     WHERE state IN ('running','cancel_requested') OR (kind='music_derivative' AND state='failed')",
                )
                .map_err(storage)?;
            statement
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                    ))
                })
                .map_err(storage)?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(storage)?
        };
        for (id, kind, item_id, request, state) in candidates {
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
            } else if kind == "import_publication" {
                let request: serde_json::Value = serde_json::from_str(&request).map_err(storage)?;
                let import_id: ImportId = request
                    .get("import_id")
                    .and_then(serde_json::Value::as_str)
                    .ok_or_else(|| Error::Storage("publication Job has no Import identity".into()))?
                    .parse()
                    .map_err(storage)?;
                publication::PublicationJournalStore::new(&self.archive)
                    .phase(TransactionId(job_id.0))?
                    .is_some_and(|phase| phase >= publication::PublicationPhase::RevisionCommitted)
                    || self
                        .import_record(import_id)
                        .is_ok_and(|record| self.capture(record.view.capture_id).is_ok())
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
            } else if kind == "catalog_acquisition" {
                let request: serde_json::Value = serde_json::from_str(&request).map_err(storage)?;
                request
                    .get("snapshot_id")
                    .and_then(serde_json::Value::as_str)
                    .and_then(|value| value.parse::<SnapshotId>().ok())
                    .is_some_and(|snapshot_id| {
                        lock(&self.db).is_ok_and(|db| {
                            db.query_row(
                                "SELECT EXISTS(SELECT 1 FROM snapshots WHERE id=?)",
                                [snapshot_id.to_string()],
                                |row| row.get::<_, bool>(0),
                            )
                            .unwrap_or(false)
                        })
                    })
            } else if kind == "music_derivative" {
                let request: serde_json::Value = serde_json::from_str(&request).map_err(storage)?;
                let capture: CaptureId =
                    serde_json::from_value(request["capture_id"].clone()).map_err(storage)?;
                let representation: RepresentationId =
                    serde_json::from_value(request["representation_id"].clone())
                        .map_err(storage)?;
                self.capture(capture)?
                    .representations
                    .iter()
                    .any(|r| r.id == representation)
            } else if kind == "processing_operation" {
                let request: serde_json::Value = serde_json::from_str(&request).map_err(storage)?;
                request
                    .get("observation_id")
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|observation_id| {
                        lock(&self.db).is_ok_and(|db| {
                            db.query_row(
                                "SELECT EXISTS(SELECT 1 FROM processing_observations WHERE id=?)",
                                [observation_id],
                                |row| row.get::<_, bool>(0),
                            )
                            .unwrap_or(false)
                        })
                    })
            } else {
                false
            };
            if committed {
                lock(&self.db)?
                    .execute(
                        "UPDATE private.jobs SET state='running',phase='revision_committed',
                           cancellation_requested=0,claim_token=COALESCE(claim_token,?) WHERE id=?",
                        params![TransactionId::new().to_string(), id],
                    )
                    .map_err(storage)?;
            } else if state == "cancel_requested" {
                // Startup owns both process locks; no old worker can acknowledge
                // this cancellation. Resolve the job and its Import atomically.
                let mut db = lock(&self.db)?;
                let tx = db
                    .transaction_with_behavior(TransactionBehavior::Immediate)
                    .map_err(storage)?;
                let now = self.clock.now().timestamp();
                tx.execute(
                    "UPDATE private.jobs SET state='cancelled',phase='cancelled',error=NULL,
                     worker_instance=NULL,lease_expires=NULL,claim_token=NULL,updated_at=? WHERE id=?",
                    params![now, id],
                ).map_err(storage)?;
                cancel_job_import(&tx, &kind, &request, now)?;
                tx.commit().map_err(storage)?;
            }
        }
        Ok(())
    }

    pub(crate) fn enqueue_job_record(&self, spec: JobSpec) -> Result<JobId> {
        self.enqueue_job_record_with_batch(spec, None)
    }

    pub(crate) fn enqueue_job_record_with_batch(
        &self,
        spec: JobSpec,
        batch: Option<(JobId, &str)>,
    ) -> Result<JobId> {
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
        if let Some(input) = &spec.frozen_input {
            input.validate()?;
        }
        let mut frozen_input = spec
            .frozen_input
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(storage)?;
        if request.len() > 16 * 1024 * 1024 {
            return Err(Error::Invalid("job request is too large".into()));
        }
        let now = self.clock.now().timestamp();
        let mut db = lock(&self.db)?;
        let tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage)?;
        if let Some(item_id) = spec.item_id {
            let removed: bool = tx
                .query_row(
                    "SELECT COALESCE((SELECT deleted FROM items WHERE id=?),0)",
                    [item_id.to_string()],
                    |row| row.get(0),
                )
                .map_err(storage)?;
            if removed {
                return Err(Error::Conflict);
            }
        }
        ensure_import_allows_work(&tx, &spec.request)?;
        if let Some((parent, token)) = batch {
            let allowed: bool = tx
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM private.jobs j
                JOIN private.preparation_members m ON m.batch_id=j.id
                WHERE j.id=? AND j.user_id=? AND j.state='running' AND j.cancellation_requested=0
                  AND j.claim_token=? AND m.job_id=? AND NOT EXISTS(SELECT 1 FROM private.work_history h WHERE h.user_id=j.user_id AND (h.pending=1 OR h.archived=1) AND (h.target='job:'||j.id OR h.target='import:'||json_extract(j.request,'$.import_id'))))",
                    params![
                        parent.to_string(),
                        spec.user_id.to_string(),
                        token,
                        spec.id.to_string()
                    ],
                    |row| row.get(0),
                )
                .map_err(storage)?;
            if !allowed {
                return Err(Error::JobCancelled);
            }
        }
        let scan_key = scan_key_for_spec(&tx, &spec)?;
        if let Some(key) = &scan_key {
            ensure_import_allows_work(&tx, &spec.request)?;
            if let Some(id) = active_scan(&tx, spec.user_id, key, Some(spec.id))? {
                Self::ensure_work_retry_allowed(&tx, id, spec.user_id)?;
                retain_scan_request(&tx, id, &spec)?;
                self.record_command(&tx, &id)?;
                tx.commit().map_err(storage)?;
                return Ok(id);
            }
            // The first admitted timestamp belongs to the durable computation.
            if let Some(FrozenJobInput::Processing(mut proposed)) = spec.frozen_input.clone() {
                let old: Option<String> = tx
                    .query_row(
                        "SELECT frozen_input FROM private.processing_jobs WHERE job_id=?",
                        [spec.id.to_string()],
                        |r| r.get(0),
                    )
                    .optional()
                    .map_err(storage)?;
                if let Some(old) = old
                    && let FrozenJobInput::Processing(original) =
                        serde_json::from_str(&old).map_err(storage)?
                {
                    proposed.created_at = original.created_at;
                    frozen_input = Some(
                        serde_json::to_string(&FrozenJobInput::Processing(proposed))
                            .map_err(storage)?,
                    );
                }
            }
        }
        // Creation time belongs to the first durable request, not output identity.
        if let Some(FrozenJobInput::Derivative(mut proposed)) = spec.frozen_input.clone() {
            let retained: Option<String> = tx
                .query_row(
                    "SELECT frozen_input FROM private.processing_jobs WHERE job_id=?",
                    [spec.id.to_string()],
                    |row| row.get(0),
                )
                .optional()
                .map_err(storage)?;
            if let Some(retained) = retained {
                let FrozenJobInput::Derivative(original) =
                    serde_json::from_str(&retained).map_err(storage)?
                else {
                    return Err(Error::IdempotencyConflict);
                };
                proposed.created_at = original.created_at;
                frozen_input = Some(
                    serde_json::to_string(&FrozenJobInput::Derivative(proposed))
                        .map_err(storage)?,
                );
            }
        }
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
            if spec.kind == "music_derivative_batch" {
                ensure_album_admission_capacity(&tx, spec.user_id, spec.id)?;
            }
            ensure_import_allows_work(&tx, &spec.request)?;
            tx.execute("INSERT OR IGNORE INTO private.item_job_generations(job_id,generation) SELECT ?1,COALESCE(json_extract(record,'$.lifecycle.generation'),0) FROM items WHERE id=COALESCE(?2,(SELECT item_id FROM private.imports WHERE id=?3))",params![spec.id.to_string(),spec.item_id.map(|id|id.to_string()),spec.request.get("import_id").and_then(serde_json::Value::as_str)]).map_err(storage)?;
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
        if let Some(frozen_input) = frozen_input {
            let existing: Option<String> = tx
                .query_row(
                    "SELECT frozen_input FROM private.processing_jobs WHERE job_id=?",
                    [spec.id.to_string()],
                    |row| row.get(0),
                )
                .optional()
                .map_err(storage)?;
            if existing
                .as_ref()
                .is_some_and(|value| value != &frozen_input)
            {
                return Err(Error::IdempotencyConflict);
            }
            tx.execute(
                "INSERT OR IGNORE INTO private.processing_jobs(job_id,frozen_input) VALUES(?,?)",
                params![spec.id.to_string(), frozen_input],
            )
            .map_err(storage)?;
        }
        if let Some(key) = scan_key {
            tx.execute(
                "INSERT OR IGNORE INTO private.scan_admissions(job_id,scan_key) VALUES(?,?)",
                params![spec.id.to_string(), key],
            )
            .map_err(storage)?;
            retain_scan_request(&tx, spec.id, &spec)?;
        }
        if spec.kind == "music_derivative" && batch.is_none() {
            tx.execute(
                "INSERT OR IGNORE INTO private.derivative_direct_requests(job_id) VALUES(?)",
                [spec.id.to_string()],
            )
            .map_err(storage)?;
        }
        if let Some(FrozenJobInput::DerivativeBatch(batch)) = &spec.frozen_input {
            if let Some(import) = spec
                .request
                .get("import_id")
                .and_then(serde_json::Value::as_str)
            {
                let changed = tx.execute("UPDATE private.guided_ingest_intents SET preparation_batch_id=?1 WHERE import_id=?2 AND user_id=?3 AND cancelled=0 AND (preparation_batch_id IS NULL OR preparation_batch_id=?1)",
                    params![spec.id.to_string(), import, spec.user_id.to_string()]).map_err(storage)?;
                if changed != 1 {
                    return Err(Error::Conflict);
                }
            }
            for member in &batch.members {
                if let collector_app::FrozenPreparationMember::Prepare { job_id, .. } = member {
                    tx.execute("INSERT OR IGNORE INTO private.preparation_members(batch_id,job_id) VALUES(?,?)", params![spec.id.to_string(),job_id.to_string()]).map_err(storage)?;
                }
            }
        }
        if spec.kind == "music_derivative" {
            requeue_failed_preparation(&tx, spec.id, spec.user_id, now)?;
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
            Option<String>,
            String,
            String,
            u32,
            String,
            u64,
            i64,
        ) = tx
            .query_row(
                "SELECT user_id,collection_id,item_id,kind,request,attempt,phase,completed_units,created_at
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
                        row.get(8)?,
                    ))
                },
            )
            .map_err(storage)?;
        let frozen_input: Option<String> = tx
            .query_row(
                "SELECT frozen_input FROM private.processing_jobs WHERE job_id=?",
                [&id],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage)?;
        tx.commit().map_err(storage)?;
        let created_at = chrono::DateTime::from_timestamp(row.8, 0)
            .ok_or_else(|| Error::Storage("job creation timestamp is invalid".into()))?;
        Ok(Some(JobClaim {
            id: id.parse().map_err(storage)?,
            user_id: row.0.parse().map_err(storage)?,
            collection_id: row
                .1
                .map(|value| value.parse().map_err(storage))
                .transpose()?,
            item_id: row
                .2
                .map(|value| value.parse().map_err(storage))
                .transpose()?,
            kind: row.3,
            request: serde_json::from_str(&row.4).map_err(storage)?,
            claim_token,
            attempt: row.5,
            phase: row.6,
            completed_units: row.7,
            created_at,
            frozen_input: frozen_input
                .map(|value| serde_json::from_str(&value).map_err(storage))
                .transpose()?,
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
                "SELECT state='cancel_requested' OR (EXISTS(SELECT 1 FROM items i WHERE i.id=COALESCE(jobs.item_id,(SELECT item_id FROM private.imports WHERE id=json_extract(jobs.request,'$.import_id'))) AND COALESCE(json_extract(i.record,'$.lifecycle.generation'),0)<>COALESCE((SELECT generation FROM private.item_job_generations g WHERE g.job_id=jobs.id),0)) AND NOT EXISTS(SELECT 1 FROM private.scan_requests r WHERE r.job_id=jobs.id AND r.kind='direct') AND NOT EXISTS(SELECT 1 FROM private.derivative_direct_requests r WHERE r.job_id=jobs.id) AND NOT (jobs.kind='import_publication' AND EXISTS(SELECT 1 FROM private.imports p JOIN captures c ON c.id=p.capture_id WHERE p.id=json_extract(jobs.request,'$.import_id')))) OR EXISTS(SELECT 1 FROM private.work_history h WHERE h.user_id=jobs.user_id AND h.pending=1 AND (h.target='job:'||jobs.id OR h.target='import:'||json_extract(jobs.request,'$.import_id'))) FROM private.jobs
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
        let (state, phase, claim_token, kind, request): (
            String,
            String,
            Option<String>,
            String,
            String,
        ) = tx
            .query_row(
                "SELECT state,phase,claim_token,kind,request FROM private.jobs WHERE id=? AND user_id=?",
                params![id.to_string(), user.to_string()],
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
            .map_err(storage)?
            .ok_or(Error::NotFound)?;
        if state == "completed" || phase == "revision_committed" {
            return Err(Error::CancellationTooLate);
        }
        if kind == "music_derivative" {
            let shared: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM private.preparation_members m JOIN private.jobs j ON j.id=m.batch_id
                 WHERE m.job_id=? AND j.cancellation_requested=0 AND j.state<>'cancelled')",
                [id.to_string()], |row| row.get(0)).map_err(storage)?;
            if shared {
                return Err(Error::Conflict);
            }
        }
        let scan: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM private.scan_admissions WHERE job_id=?)",
                [id.to_string()],
                |r| r.get(0),
            )
            .map_err(storage)?;
        if scan {
            tx.execute(
                "DELETE FROM private.scan_requests WHERE job_id=? AND kind='direct' AND owner=?",
                params![id.to_string(), user.to_string()],
            )
            .map_err(storage)?;
            let shared: bool = tx
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM private.scan_requests WHERE job_id=?)",
                    [id.to_string()],
                    |r| r.get(0),
                )
                .map_err(storage)?;
            if shared {
                self.record_command(&tx, &())?;
                tx.commit().map_err(storage)?;
                return Ok(());
            }
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
            cancel_job_import(&tx, &kind, &request, self.clock.now().timestamp())?;
        }
        if kind == "music_derivative" {
            tx.execute(
                "DELETE FROM private.derivative_direct_requests WHERE job_id=?",
                [id.to_string()],
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
                   completed_units,total_units,attempt,created_at,updated_at,error,limitation,result
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
                   completed_units,total_units,attempt,created_at,updated_at,error,limitation,result
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

    fn ensure_work_retry_allowed(db: &Connection, id: JobId, user: UserId) -> Result<()> {
        let blocked:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM private.work_history h JOIN private.jobs j ON j.id=? WHERE h.user_id=? AND (h.archived=1 OR h.pending=1) AND (h.target='job:'||j.id OR h.target='import:'||json_extract(j.request,'$.import_id')))",params![id.to_string(),user.to_string()],|r|r.get(0)).map_err(storage)?;
        if blocked {
            Err(Error::Conflict)
        } else {
            Ok(())
        }
    }
    pub(crate) fn retry_failed_job_record(&self, id: JobId, user: UserId) -> Result<JobId> {
        let mut db = lock(&self.db)?;
        let tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage)?;
        if tx.query_row("SELECT (EXISTS(SELECT 1 FROM items i WHERE i.id=COALESCE(jobs.item_id,(SELECT item_id FROM private.imports WHERE id=json_extract(jobs.request,'$.import_id'))) AND COALESCE(json_extract(i.record,'$.lifecycle.generation'),0)<>COALESCE((SELECT generation FROM private.item_job_generations g WHERE g.job_id=jobs.id),0)) AND NOT EXISTS(SELECT 1 FROM private.scan_requests r WHERE r.job_id=jobs.id AND r.kind='direct') AND NOT EXISTS(SELECT 1 FROM private.derivative_direct_requests r WHERE r.job_id=jobs.id) AND NOT (jobs.kind='import_publication' AND EXISTS(SELECT 1 FROM private.imports p JOIN captures c ON c.id=p.capture_id WHERE p.id=json_extract(jobs.request,'$.import_id')))) FROM private.jobs WHERE id=?",[id.to_string()],|row|row.get::<_,bool>(0)).optional().map_err(storage)?.unwrap_or(false) { return Err(Error::Conflict); }
        Self::ensure_work_retry_allowed(&tx, id, user)?;
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
                "integrity_verification"
                    | "import_staging"
                    | "import_analysis"
                    | "import_publication"
                    | "catalog_acquisition"
                    | "processing_operation"
                    | "music_derivative"
            )
        {
            return Err(Error::Conflict);
        }
        let request: String = tx
            .query_row(
                "SELECT request FROM private.jobs WHERE id=?",
                [id.to_string()],
                |row| row.get(0),
            )
            .map_err(storage)?;
        let request: serde_json::Value = serde_json::from_str(&request).map_err(storage)?;
        let direct:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM private.scan_requests WHERE job_id=? AND kind='direct' AND owner=?)",params![id.to_string(),user.to_string()],|row|row.get(0)).map_err(storage)?;
        ensure_import_work(&tx, &request, direct && kind == "import_analysis")?;
        let scan_key: Option<String> = tx
            .query_row(
                "SELECT scan_key FROM private.scan_admissions WHERE job_id=?",
                [id.to_string()],
                |r| r.get(0),
            )
            .optional()
            .map_err(storage)?;
        if let Some(key) = scan_key {
            if let Some(active) = active_scan(&tx, user, &key, Some(id))? {
                Self::ensure_work_retry_allowed(&tx, active, user)?;
                tx.execute(
                    "INSERT OR IGNORE INTO private.scan_requests VALUES(?,'direct',?)",
                    params![active.to_string(), user.to_string()],
                )
                .map_err(storage)?;
                self.record_command(&tx, &active)?;
                tx.commit().map_err(storage)?;
                return Ok(active);
            }
            tx.execute(
                "INSERT OR IGNORE INTO private.scan_requests VALUES(?,'direct',?)",
                params![id.to_string(), user.to_string()],
            )
            .map_err(storage)?;
        }
        if let Some(import) = request.get("import_id").and_then(serde_json::Value::as_str) {
            tx.execute("UPDATE private.guided_ingest_intents SET settled=0,enrichment_error=NULL WHERE import_id=? AND cancelled=0", [import]).map_err(storage)?;
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

// Called inside the same transaction that makes cancellation terminal.
fn cancel_job_import(
    tx: &rusqlite::Transaction<'_>,
    kind: &str,
    request: &str,
    now: i64,
) -> Result<()> {
    let request: serde_json::Value = serde_json::from_str(request).map_err(storage)?;
    if let Some(import_id) = request.get("import_id").and_then(serde_json::Value::as_str) {
        // Generic Job cancellation must also settle guided provider work. Its
        // Capture can already be published, so do not cancel preservation here.
        tx.execute(
            "UPDATE private.guided_ingest_intents SET cancelled=1,settled=1,updated_at=? WHERE import_id=?",
            params![now, import_id],
        ).map_err(storage)?;
        if kind.starts_with("import_") {
            tx.execute(
                "UPDATE private.imports SET phase='cancelled',revision=revision+1,updated_at=?
                 WHERE id=? AND phase<>'published' AND phase<>'cancelled'",
                params![now, import_id],
            )
            .map_err(storage)?;
        }
    }
    Ok(())
}

// Admission and cancellation share the private-state transaction. A stopped
// import cannot be resurrected through either the guided or generic job API.
fn ensure_import_allows_work(
    tx: &rusqlite::Transaction<'_>,
    request: &serde_json::Value,
) -> Result<()> {
    ensure_import_work(tx, request, false)
}

fn ensure_import_work(
    tx: &rusqlite::Transaction<'_>,
    request: &serde_json::Value,
    independent_analysis: bool,
) -> Result<()> {
    if let Some(import) = request.get("import_id").and_then(serde_json::Value::as_str) {
        let stopped: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM private.imports WHERE id=?1 AND phase='cancelled')
             OR EXISTS(SELECT 1 FROM private.guided_ingest_intents WHERE import_id=?1 AND cancelled=1 AND NOT ?2)
             OR EXISTS(SELECT 1 FROM private.import_cleanup WHERE import_id=?1)
             OR EXISTS(SELECT 1 FROM private.imports p JOIN items i ON i.id=p.item_id
                       WHERE p.id=?1 AND NOT ?2 AND COALESCE(json_extract(i.record,'$.lifecycle.generation'),0)<>COALESCE((SELECT generation FROM private.item_import_generations g WHERE g.import_id=p.id),0))
             OR EXISTS(SELECT 1 FROM private.guided_ingest_intents g JOIN items i
                       ON i.id=g.target_item_id WHERE g.import_id=?1 AND i.deleted=1 AND NOT ?2)
             OR EXISTS(SELECT 1 FROM private.imports p JOIN items i ON i.id=p.item_id
                       WHERE p.id=?1 AND i.deleted=1 AND NOT ?2)
             OR EXISTS(SELECT 1 FROM private.work_history WHERE target='import:'||?1 AND (pending=1 OR archived=1))",
            params![import,independent_analysis], |row| row.get(0)
        ).map_err(storage)?;
        if stopped {
            return Err(Error::Conflict);
        }
    }
    Ok(())
}

fn requeue_failed_preparation(
    tx: &rusqlite::Transaction<'_>,
    id: JobId,
    user: UserId,
    now: i64,
) -> Result<()> {
    let (state, prior, error, attempt, finished): (String, Option<String>, Option<String>, u32, i64) = tx.query_row(
        "SELECT state,result,error,attempt,updated_at FROM private.jobs WHERE id=? AND user_id=?",
        params![id.to_string(),user.to_string()], |r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))
    ).map_err(storage)?;
    if state == "cancel_requested" {
        return Err(Error::CommandInProgress);
    }
    if !matches!(state.as_str(), "failed" | "cancelled") {
        return Ok(());
    }
    let mut result: serde_json::Value = prior
        .as_deref()
        .map(serde_json::from_str)
        .transpose()
        .map_err(storage)?
        .unwrap_or_else(|| serde_json::json!({}));
    let object = result
        .as_object_mut()
        .ok_or_else(|| Error::Storage("invalid preparation history".into()))?;
    let failures = object
        .entry("prior_failures")
        .or_insert_with(|| serde_json::json!([]))
        .as_array_mut()
        .ok_or_else(|| Error::Storage("invalid preparation attempts".into()))?;
    failures.push(
        serde_json::json!({"attempt":attempt,"state":state,"error":error,"failed_at":finished}),
    );
    tx.execute("UPDATE private.jobs SET state='queued',phase='queued',completed_units=0,attempt=attempt+1,error=NULL,result=?,
        cancellation_requested=0,worker_instance=NULL,lease_expires=NULL,claim_token=NULL,updated_at=? WHERE id=? AND user_id=?",
        params![serde_json::to_string(&result).map_err(storage)?,now,id.to_string(),user.to_string()]).map_err(storage)?;
    Ok(())
}

fn ensure_album_admission_capacity(
    tx: &rusqlite::Transaction<'_>,
    user: UserId,
    id: JobId,
) -> Result<()> {
    let pending: u32 = tx.query_row("SELECT COUNT(*) FROM private.jobs j WHERE user_id=?1 AND id<>?2 AND kind='music_derivative_batch' AND cancellation_requested=0
        AND (state IN ('queued','running','cancel_requested') OR EXISTS(SELECT 1 FROM private.preparation_members m JOIN private.jobs child ON child.id=m.job_id
        WHERE m.batch_id=j.id AND child.state IN ('queued','running','cancel_requested')))", params![user.to_string(), id.to_string()], |row| row.get(0)).map_err(storage)?;
    if pending >= collector_app::MAX_ACTIVE_MUSIC_PREPARATION_BATCHES {
        return Err(Error::Unavailable("Too many album batches are pending; wait for one to finish or cancel outstanding work.".into()));
    }
    Ok(())
}

fn scan_key_for_spec(tx: &Connection, spec: &JobSpec) -> Result<Option<String>> {
    if let Some(FrozenJobInput::Processing(frozen)) = &spec.frozen_input {
        let raw = frozen
            .snapshot_id
            .map(|id| {
                tx.query_row(
                    "SELECT record FROM snapshots WHERE id=? AND collection_id=?",
                    params![id.to_string(), frozen.collection_id.to_string()],
                    |r| r.get::<_, String>(0),
                )
                .optional()
                .map_err(storage)
            })
            .transpose()?
            .flatten();
        let snapshot = raw
            .map(|raw| serde_json::from_str(&raw).map_err(storage))
            .transpose()?;
        return collector_app::scan_identity_from(frozen, snapshot)?
            .key()
            .map(Some);
    }
    if spec.kind == "integrity_verification" {
        // The Item inventory record is metadata; no file bytes are read here.
        let raw: String = tx
            .query_row(
                "SELECT record FROM items WHERE id=?",
                [spec.item_id.ok_or(Error::NotFound)?.to_string()],
                |r| r.get(0),
            )
            .map_err(storage)?;
        let mut item: Item = serde_json::from_str(&raw).map_err(storage)?;
        hydrate_captures(tx, &mut item)?;
        return Ok(Some(
            Digest::sha256(
                &serde_json::to_vec(&(
                    "integrity-v1",
                    item.id,
                    item.captures
                        .iter()
                        .flat_map(|c| &c.representations)
                        .map(|r| {
                            (
                                r.id,
                                r.files
                                    .iter()
                                    .map(|f| (&f.name, f.bytes, &f.digest))
                                    .collect::<Vec<_>>(),
                            )
                        })
                        .collect::<Vec<_>>(),
                ))
                .map_err(storage)?,
            )
            .hex,
        ));
    }
    Ok(None)
}
fn active_scan(
    tx: &Connection,
    user: UserId,
    key: &str,
    except: Option<JobId>,
) -> Result<Option<JobId>> {
    let id:Option<String>=tx.query_row("SELECT j.id FROM private.scan_admissions a JOIN private.jobs j ON j.id=a.job_id WHERE a.scan_key=? AND j.user_id=? AND j.state IN ('queued','running','cancel_requested') AND (? IS NULL OR j.id<>?) ORDER BY j.created_at,j.id LIMIT 1",params![key,user.to_string(),except.map(|v|v.to_string()),except.map(|v|v.to_string())],|r|r.get(0)).optional().map_err(storage)?;
    id.map(|v| v.parse().map_err(storage)).transpose()
}
fn retain_scan_request(tx: &Connection, id: JobId, spec: &JobSpec) -> Result<()> {
    let parent = spec
        .request
        .get("guided")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
        .then(|| {
            spec.request
                .get("import_id")
                .and_then(serde_json::Value::as_str)
        })
        .flatten();
    let owner = parent
        .map(str::to_owned)
        .unwrap_or_else(|| spec.user_id.to_string());
    tx.execute(
        "INSERT OR IGNORE INTO private.scan_requests(job_id,kind,owner) VALUES(?,?,?)",
        params![
            id.to_string(),
            if parent.is_some() { "import" } else { "direct" },
            owner
        ],
    )
    .map_err(storage)?;
    Ok(())
}

impl Store {
    pub(crate) fn rebuild_scan_admissions(&self) -> Result<()> {
        let mut db = lock(&self.db)?;
        let tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage)?;
        let specs = {
            let mut stmt=tx.prepare("SELECT j.id,j.user_id,j.collection_id,j.item_id,j.kind,j.phase,j.total_units,j.request,p.frozen_input FROM private.jobs j JOIN private.processing_jobs p ON p.job_id=j.id LEFT JOIN private.scan_admissions a ON a.job_id=j.id WHERE a.job_id IS NULL").map_err(storage)?;
            let rows = stmt
                .query_map([], |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, Option<String>>(2)?,
                        r.get::<_, Option<String>>(3)?,
                        r.get::<_, String>(4)?,
                        r.get::<_, String>(5)?,
                        r.get::<_, u64>(6)?,
                        r.get::<_, String>(7)?,
                        r.get::<_, String>(8)?,
                    ))
                })
                .map_err(storage)?;
            rows.collect::<std::result::Result<Vec<_>, _>>()
                .map_err(storage)?
        };
        for (id, user, collection, item, kind, phase, total, request, frozen) in specs {
            let spec = JobSpec {
                id: id.parse().map_err(storage)?,
                user_id: user.parse().map_err(storage)?,
                collection_id: collection.map(|v| v.parse().map_err(storage)).transpose()?,
                item_id: item.map(|v| v.parse().map_err(storage)).transpose()?,
                kind,
                phase,
                total_units: total,
                request: serde_json::from_str(&request).map_err(storage)?,
                frozen_input: Some(serde_json::from_str(&frozen).map_err(storage)?),
            };
            if let Some(key) = scan_key_for_spec(&tx, &spec)? {
                tx.execute(
                    "INSERT INTO private.scan_admissions VALUES(?,?)",
                    params![id, key],
                )
                .map_err(storage)?;
                retain_scan_request(&tx, spec.id, &spec)?;
            }
        }
        tx.commit().map_err(storage)?;
        Ok(())
    }
}
