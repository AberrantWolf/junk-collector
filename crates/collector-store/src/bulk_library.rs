//! Private selections, member outcomes and atomic parent-owned Job admission.
use super::*;
use collector_app::{
    BulkLibraryKind, BulkLibraryPort, BulkLibraryState, BulkLibrarySummary, BulkMemberFilter,
    BulkMemberState, FrozenBulkLibraryMember, PrivateBulkLibrary,
};
pub(crate) fn install(db: &Connection) -> Result<()> {
    db.execute_batch("CREATE TABLE IF NOT EXISTS private.bulk_library(id TEXT PRIMARY KEY,user_id TEXT NOT NULL,state TEXT NOT NULL,record TEXT NOT NULL);CREATE INDEX IF NOT EXISTS private.bulk_library_owner ON bulk_library(user_id,id);CREATE INDEX IF NOT EXISTS private.bulk_library_recovery ON bulk_library(state,id);CREATE TABLE IF NOT EXISTS private.bulk_library_members(batch_id TEXT NOT NULL,position INTEGER NOT NULL,collection_id TEXT NOT NULL,item_id TEXT NOT NULL,state TEXT NOT NULL,job_id TEXT,record TEXT NOT NULL,PRIMARY KEY(batch_id,position));CREATE INDEX IF NOT EXISTS private.bulk_library_admission ON bulk_library_members(batch_id,state,position);CREATE INDEX IF NOT EXISTS private.bulk_library_jobs ON bulk_library_members(job_id,batch_id);CREATE INDEX IF NOT EXISTS private.bulk_library_collection ON bulk_library_members(collection_id,batch_id);").map_err(storage)
}
fn label<T: serde::Serialize>(value: T) -> Result<String> {
    serde_json::to_value(value)
        .map_err(storage)?
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| Error::Storage("invalid bulk state".into()))
}
fn read(db: &Connection, user: UserId, id: JobId) -> Result<PrivateBulkLibrary> {
    let raw: Option<String> = db
        .query_row(
            "SELECT record FROM private.bulk_library WHERE id=? AND user_id=?",
            params![id.to_string(), user.to_string()],
            |r| r.get(0),
        )
        .optional()
        .map_err(storage)?;
    serde_json::from_str(&raw.ok_or(Error::NotFound)?).map_err(storage)
}
fn write(db: &Connection, record: &PrivateBulkLibrary) -> Result<()> {
    db.execute(
        "UPDATE private.bulk_library SET state=?,record=? WHERE id=? AND user_id=?",
        params![
            label(record.summary.state)?,
            serde_json::to_string(record).map_err(storage)?,
            record.summary.id.to_string(),
            record.user_id.to_string()
        ],
    )
    .map_err(storage)?;
    Ok(())
}
fn allowed(db: &Connection, user: UserId, collection: CollectionId) -> Result<()> {
    let yes:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM private.users u JOIN private.grants g ON g.user_id=u.id WHERE u.id=? AND u.enabled=1 AND g.collection_id=? AND g.role>=1)",params![user.to_string(),collection.to_string()],|r|r.get(0)).map_err(storage)?;
    if yes { Ok(()) } else { Err(Error::Forbidden) }
}
fn limit(value: u16) -> Result<u16> {
    if (1..=200).contains(&value) {
        Ok(value)
    } else {
        Err(Error::Invalid("Use a 1–200 member page.".into()))
    }
}
impl BulkLibraryPort for Store {
    fn freeze_bulk_library(
        &self,
        record: PrivateBulkLibrary,
        members: Vec<FrozenBulkLibraryMember>,
    ) -> Result<BulkLibrarySummary> {
        let _writer = lock(&self.writer)?;
        for member in &members {
            if serde_json::to_value(self.item(member.item.id)?).map_err(storage)?
                != serde_json::to_value(&member.item).map_err(storage)?
            {
                return Err(Error::Conflict);
            }
        }
        let mut db = lock(&self.db)?;
        let tx = db.transaction().map_err(storage)?;
        tx.execute(
            "INSERT INTO private.bulk_library VALUES(?,?,?,?)",
            params![
                record.summary.id.to_string(),
                record.user_id.to_string(),
                label(record.summary.state)?,
                serde_json::to_string(&record).map_err(storage)?
            ],
        )
        .map_err(storage)?;
        for member in members {
            if serde_json::to_vec(&member).map_err(storage)?.len() > 12 * 1024 * 1024 {
                return Err(Error::Invalid(
                    "An album exceeds the 12 MiB frozen manifest limit. Select fewer disc sources."
                        .into(),
                ));
            }
            allowed(&tx, record.user_id, member.item.collection_id)?;
            tx.execute(
                "INSERT INTO private.bulk_library_members VALUES(?,?,?,?,?,NULL,?)",
                params![
                    record.summary.id.to_string(),
                    member.view.position,
                    member.item.collection_id.to_string(),
                    member.item.id.to_string(),
                    label(member.view.state)?,
                    serde_json::to_string(&member).map_err(storage)?
                ],
            )
            .map_err(storage)?;
        }
        self.record_command(&tx, &record.summary)?;
        tx.commit().map_err(storage)?;
        Ok(record.summary)
    }
    fn bulk_library(&self, user: UserId, id: JobId) -> Result<PrivateBulkLibrary> {
        {
            let db = lock(&self.db)?;
            read(&db, user, id)
        }
    }
    fn bulk_library_page(
        &self,
        user: UserId,
        collection: Option<CollectionId>,
        after: Option<JobId>,
        page_limit: u16,
    ) -> Result<Page<BulkLibrarySummary>> {
        let page_limit = limit(page_limit)?;
        let db = lock(&self.db)?;
        let mut stmt=db.prepare("SELECT record FROM private.bulk_library b WHERE user_id=? AND (? IS NULL OR id>?) AND (? IS NULL OR EXISTS(SELECT 1 FROM private.bulk_library_members m WHERE m.batch_id=b.id AND m.collection_id=?)) ORDER BY id LIMIT ?").map_err(storage)?;
        let mut items = stmt
            .query_map(
                params![
                    user.to_string(),
                    after.map(|id| id.to_string()),
                    after.map(|id| id.to_string()),
                    collection.map(|v| v.to_string()),
                    collection.map(|v| v.to_string()),
                    u32::from(page_limit) + 1
                ],
                |r| r.get::<_, String>(0),
            )
            .map_err(storage)?
            .map(|r| {
                serde_json::from_str::<PrivateBulkLibrary>(&r.map_err(storage)?)
                    .map(|v| v.summary)
                    .map_err(storage)
            })
            .collect::<Result<Vec<_>>>()?;
        let next_cursor = if items.len() > usize::from(page_limit) {
            items.pop();
            items.last().map(|v| v.id.to_string())
        } else {
            None
        };
        Ok(Page { items, next_cursor })
    }
    fn bulk_library_members(
        &self,
        user: UserId,
        id: JobId,
        after: Option<u32>,
        page_limit: u16,
        filter: BulkMemberFilter,
    ) -> Result<Page<FrozenBulkLibraryMember>> {
        let page_limit = limit(page_limit)?;
        let db = lock(&self.db)?;
        read(&db, user, id)?;
        let mut stmt=db.prepare("SELECT record FROM private.bulk_library_members WHERE batch_id=? AND (? IS NULL OR position>?) AND (?='all' OR ?='selected' AND state!='excluded' OR ?='admission' AND state='ready' OR ?='unsettled' AND state IN ('queued','running')) ORDER BY position LIMIT ?").map_err(storage)?;
        let filter = match filter {
            BulkMemberFilter::All => "all",
            BulkMemberFilter::Selected => "selected",
            BulkMemberFilter::Admission => "admission",
            BulkMemberFilter::Unsettled => "unsettled",
        };
        let mut rows = stmt
            .query(params![
                id.to_string(),
                after,
                after,
                filter,
                filter,
                filter,
                filter,
                u32::from(page_limit) + 1
            ])
            .map_err(storage)?;
        let mut items = Vec::new();
        let mut bytes = 0usize;
        let mut more = false;
        while let Some(row) = rows.next().map_err(storage)? {
            let raw: String = row.get(0).map_err(storage)?;
            if items.len() >= usize::from(page_limit)
                || (!items.is_empty() && bytes + raw.len() > 12 * 1024 * 1024)
            {
                more = true;
                break;
            }
            if raw.len() > 12 * 1024 * 1024 {
                return Err(Error::Storage(
                    "Frozen album exceeds its 12 MiB manifest limit.".into(),
                ));
            }
            bytes += raw.len();
            items.push(serde_json::from_str::<FrozenBulkLibraryMember>(&raw).map_err(storage)?);
        }
        let next_cursor = if more {
            items.last().map(|m| m.view.position.to_string())
        } else {
            None
        };
        for member in &mut items {
            if allowed(&db, user, member.item.collection_id).is_err() {
                member.view.title = "Access to this entry was revoked".into();
                member.view.artwork_asset_id = None;
                member.view.reason = Some(
                    "Collection permission is required to inspect or process this entry.".into(),
                );
                member.view.state = BulkMemberState::Review;
                member.input = None;
            }
        }
        Ok(Page { items, next_cursor })
    }
    fn accept_bulk_library(&self, user: UserId, id: JobId) -> Result<BulkLibrarySummary> {
        let _publication_gate = lock(&self.job_publication_gate)?;
        let mut db = lock(&self.db)?;
        let tx = db.transaction().map_err(storage)?;
        let mut record = read(&tx, user, id)?;
        if record.summary.state != BulkLibraryState::Preview || !record.summary.can_start {
            return Err(Error::Conflict);
        }
        let mut stmt=tx.prepare("SELECT DISTINCT collection_id FROM private.bulk_library_members WHERE batch_id=? AND state='ready'").map_err(storage)?;
        let collections = stmt
            .query_map([id.to_string()], |r| r.get::<_, String>(0))
            .map_err(storage)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(storage)?;
        drop(stmt);
        for collection in collections {
            allowed(&tx, user, collection.parse().map_err(storage)?)?;
        }
        let now = self.clock.now().timestamp();
        tx.execute("INSERT INTO private.jobs(id,user_id,kind,state,phase,completed_units,total_units,attempt,created_at,updated_at,request,cancellation_requested) VALUES(?,?,'bulk_library','queued','queued',0,?,1,?,?,?,0)",params![id.to_string(),user.to_string(),record.summary.selected,now,now,serde_json::json!({"bulk_id":id}).to_string()]).map_err(storage)?;
        record.summary.state = BulkLibraryState::Accepted;
        record.summary.can_start = false;
        record.summary.can_cancel = true;
        record.summary.updated_at = self.clock.now();
        write(&tx, &record)?;
        self.record_command(&tx, &record.summary)?;
        tx.commit().map_err(storage)?;
        Ok(record.summary)
    }
    fn admit_bulk_library_member(
        &self,
        claim: &JobClaim,
        position: u32,
        spec: JobSpec,
    ) -> Result<JobId> {
        let _writer = lock(&self.writer)?;
        let member = {
            let db = lock(&self.db)?;
            let raw:String=db.query_row("SELECT record FROM private.bulk_library_members WHERE batch_id=? AND position=?",params![claim.id.to_string(),position],|r|r.get(0)).map_err(storage)?;
            serde_json::from_str::<FrozenBulkLibraryMember>(&raw).map_err(storage)?
        };
        if serde_json::to_value(self.item(member.item.id)?).map_err(storage)?
            != serde_json::to_value(&member.item).map_err(storage)?
            || serde_json::to_value(&spec.frozen_input).map_err(storage)?
                != serde_json::to_value(&member.input).map_err(storage)?
        {
            return Err(Error::Conflict);
        }
        self.enqueue_job_record_with_owners(
            spec,
            None,
            Some((claim.id, &claim.claim_token, position)),
        )
    }
    fn bulk_library_capacity(&self, user: UserId, kind: BulkLibraryKind) -> Result<u32> {
        let db = lock(&self.db)?;
        let count: u32 = if kind == BulkLibraryKind::AccurateRip {
            db.query_row("SELECT count(*) FROM private.jobs WHERE user_id=? AND kind='accuraterip_check' AND state IN ('queued','running','cancel_requested')",[user.to_string()],|r|r.get(0)).map_err(storage)?
        } else if kind == BulkLibraryKind::MusicPreparation {
            db.query_row("SELECT count(*) FROM private.jobs j WHERE user_id=? AND kind='music_derivative_batch' AND cancellation_requested=0 AND (state IN ('queued','running','cancel_requested') OR EXISTS(SELECT 1 FROM private.preparation_members m JOIN private.jobs c ON c.id=m.job_id WHERE m.batch_id=j.id AND c.state IN ('queued','running','cancel_requested')))",[user.to_string()],|r|r.get(0)).map_err(storage)?
        } else {
            db.query_row("SELECT count(*) FROM private.jobs WHERE user_id=? AND kind IN ('metadata_resolution','metadata_existing') AND state IN ('queued','running','cancel_requested')",[user.to_string()],|r|r.get(0)).map_err(storage)?
        };
        Ok(32u32.saturating_sub(count))
    }
    fn yield_bulk_library(&self, claim: &JobClaim) -> Result<()> {
        let db = lock(&self.db)?;
        if db.execute("UPDATE private.jobs SET state='queued',phase='waiting_capacity',claim_token=NULL,worker_instance=NULL,lease_expires=NULL,updated_at=? WHERE id=? AND claim_token=? AND state='running' AND cancellation_requested=0",params![self.clock.now().timestamp(),claim.id.to_string(),claim.claim_token]).map_err(storage)?!=1{return Err(Error::JobCancelled);}
        Ok(())
    }
    fn change_bulk_library(
        &self,
        user: UserId,
        id: JobId,
        cancel: bool,
    ) -> Result<BulkLibrarySummary> {
        self.change_bulk_library_record(user, id, cancel, true)
    }
    fn settle_bulk_library_member(
        &self,
        user: UserId,
        id: JobId,
        position: u32,
        state: BulkMemberState,
        reason: Option<String>,
    ) -> Result<()> {
        let mut db = lock(&self.db)?;
        let tx = db.transaction().map_err(storage)?;
        read(&tx, user, id)?;
        let raw: String = tx
            .query_row(
                "SELECT record FROM private.bulk_library_members WHERE batch_id=? AND position=?",
                params![id.to_string(), position],
                |r| r.get(0),
            )
            .map_err(storage)?;
        let mut member: FrozenBulkLibraryMember = serde_json::from_str(&raw).map_err(storage)?;
        member.view.state = state;
        member.view.reason = reason;
        tx.execute("UPDATE private.bulk_library_members SET state=?,record=? WHERE batch_id=? AND position=?",params![label(state)?,serde_json::to_string(&member).map_err(storage)?,id.to_string(),position]).map_err(storage)?;
        tx.commit().map_err(storage)?;
        Ok(())
    }
    fn unfinished_bulk_library(
        &self,
        after: Option<JobId>,
        page_limit: u16,
    ) -> Result<Vec<PrivateBulkLibrary>> {
        let page_limit = limit(page_limit)?;
        let db = lock(&self.db)?;
        let mut stmt=db.prepare("SELECT record FROM private.bulk_library WHERE state IN ('accepted','cancelling') AND (? IS NULL OR id>?) ORDER BY id LIMIT ?").map_err(storage)?;
        stmt.query_map(
            params![
                after.map(|id| id.to_string()),
                after.map(|id| id.to_string()),
                page_limit
            ],
            |r| r.get::<_, String>(0),
        )
        .map_err(storage)?
        .map(|r| serde_json::from_str(&r.map_err(storage)?).map_err(storage))
        .collect()
    }
    fn reconcile_bulk_library(&self, user: UserId, id: JobId) -> Result<BulkLibrarySummary> {
        let mut db = lock(&self.db)?;
        let tx = db.transaction().map_err(storage)?;
        let mut record = read(&tx, user, id)?;
        let (pending,success,failed):(u32,u32,u32)=tx.query_row("SELECT COALESCE(SUM(state IN ('ready','queued','running')),0),COALESCE(SUM(state='succeeded'),0),COALESCE(SUM(state IN ('failed','review')),0) FROM private.bulk_library_members WHERE batch_id=?",[id.to_string()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).map_err(storage)?;
        record.summary.pending = pending;
        record.summary.succeeded = success;
        record.summary.failed = failed;
        let retryable:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM private.bulk_library_members WHERE batch_id=? AND state='failed')",[id.to_string()],|r|r.get(0)).map_err(storage)?;
        record.summary.admission_error = tx
            .query_row(
                "SELECT error FROM private.jobs WHERE id=? AND state='failed'",
                [id.to_string()],
                |r| r.get::<_, Option<String>>(0),
            )
            .optional()
            .map_err(storage)?
            .flatten();
        record.summary.can_retry = (retryable || record.summary.admission_error.is_some())
            && record.summary.state != BulkLibraryState::Cancelling
            && record.summary.state != BulkLibraryState::Cancelled;
        if pending == 0 {
            record.summary.state = if record.summary.state == BulkLibraryState::Cancelling {
                BulkLibraryState::Cancelled
            } else {
                BulkLibraryState::Completed
            };
            record.summary.can_cancel = false;
        }
        tx.execute("UPDATE private.jobs SET phase='queued',updated_at=? WHERE id=? AND state='queued' AND phase='waiting_capacity' AND cancellation_requested=0",params![self.clock.now().timestamp(),id.to_string()]).map_err(storage)?;
        record.summary.updated_at = self.clock.now();
        write(&tx, &record)?;
        tx.commit().map_err(storage)?;
        Ok(record.summary)
    }
}

impl Store {
    pub(crate) fn change_bulk_library_record(
        &self,
        user: UserId,
        id: JobId,
        cancel: bool,
        summary_receipt: bool,
    ) -> Result<BulkLibrarySummary> {
        let _publication_gate = if cancel {
            Some(lock(&self.job_publication_gate)?)
        } else {
            None
        };
        let mut db = lock(&self.db)?;
        let tx = db.transaction().map_err(storage)?;
        let mut record = read(&tx, user, id)?;
        if record.summary.state == BulkLibraryState::Preview
            || (!cancel && !record.summary.can_retry)
        {
            return Err(Error::Conflict);
        }
        let mut stmt=tx.prepare("SELECT position,record FROM private.bulk_library_members WHERE batch_id=? ORDER BY position").map_err(storage)?;
        let rows = stmt
            .query_map([id.to_string()], |r| {
                Ok((r.get::<_, u32>(0)?, r.get::<_, String>(1)?))
            })
            .map_err(storage)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(storage)?;
        drop(stmt);
        for (position, raw) in rows {
            let mut member: FrozenBulkLibraryMember =
                serde_json::from_str(&raw).map_err(storage)?;
            if cancel {
                if let Some(job) = member.view.job_id {
                    if record.summary.kind == BulkLibraryKind::MusicPreparation {
                        Self::cancel_preparation_batch_in_tx(
                            &tx,
                            job,
                            user,
                            self.clock.now().timestamp(),
                        )?;
                    } else {
                        tx.execute("UPDATE private.jobs SET cancellation_requested=1,state=CASE WHEN state='running' THEN 'cancel_requested' WHEN state='queued' THEN 'cancelled' ELSE state END WHERE id=? AND user_id=?",params![job.to_string(),user.to_string()]).map_err(storage)?;
                    }
                }
                if member.view.state == BulkMemberState::Ready {
                    member.view.state = BulkMemberState::Cancelled;
                }
            } else if matches!(member.view.state, BulkMemberState::Failed) {
                allowed(&tx, user, member.item.collection_id)?;
                if member.input.is_none() {
                    continue;
                }
                if let Some(job) = member.view.job_id {
                    if record.summary.kind == BulkLibraryKind::MusicPreparation {
                        Self::retry_preparation_batch_in_tx(
                            &tx,
                            job,
                            user,
                            self.clock.now().timestamp(),
                        )?;
                        member.view.state = BulkMemberState::Queued;
                    } else {
                        self.retry_failed_job_in_tx(&tx, job, user)?;
                        member.view.state = BulkMemberState::Queued;
                    }
                } else {
                    member.view.state = BulkMemberState::Ready;
                }
                member.view.reason = None;
            }
            tx.execute("UPDATE private.bulk_library_members SET state=?,record=? WHERE batch_id=? AND position=?",params![label(member.view.state)?,serde_json::to_string(&member).map_err(storage)?,id.to_string(),position]).map_err(storage)?;
        }
        record.summary.state = if cancel {
            BulkLibraryState::Cancelling
        } else {
            BulkLibraryState::Accepted
        };
        record.summary.can_cancel = !cancel;
        record.summary.can_retry = false;
        if !cancel && record.summary.admission_error.is_some() {
            super::jobs::requeue_failed_preparation(&tx, id, user, self.clock.now().timestamp())?;
            record.summary.admission_error = None;
        }
        write(&tx, &record)?;
        tx.execute("UPDATE private.jobs SET cancellation_requested=?,state=CASE WHEN ?=1 THEN CASE WHEN state='running' THEN 'cancel_requested' WHEN state='queued' THEN 'cancelled' ELSE state END ELSE 'queued' END,phase=CASE WHEN ?=1 THEN phase ELSE 'queued' END,claim_token=CASE WHEN ?=1 THEN claim_token ELSE NULL END,worker_instance=CASE WHEN ?=1 THEN worker_instance ELSE NULL END,lease_expires=CASE WHEN ?=1 THEN lease_expires ELSE NULL END WHERE id=? AND user_id=?",params![cancel,cancel,cancel,cancel,cancel,cancel,id.to_string(),user.to_string()]).map_err(storage)?;
        if summary_receipt {
            self.record_command(&tx, &record.summary)?;
        } else {
            self.record_command(&tx, &())?;
        }
        tx.commit().map_err(storage)?;
        Ok(record.summary)
    }
}
