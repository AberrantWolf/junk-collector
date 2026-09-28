use super::*;
use collector_app::{WorkEntry, WorkHistoryAction, WorkHistoryCommand, WorkQuery, WorkTarget};

const WORK: &str = r#"
WITH roots AS (
 SELECT 'import:'||i.id AS key, 'import' AS target_kind,i.id,user_id,i.collection_id,
 CASE WHEN i.phase='published' THEN json_extract(c.record,'$.association.item_id') ELSE i.item_id END AS item_id,
 i.package_name AS name,'import' AS kind,i.phase,i.created_at,i.updated_at,i.revision,
 EXISTS(SELECT 1 FROM private.guided_ingest_intents g WHERE g.import_id=i.id) AS guided,
 (EXISTS(SELECT 1 FROM private.jobs j WHERE json_extract(j.request,'$.import_id')=i.id AND j.state IN ('queued','running','cancel_requested')) OR (i.phase IN ('analyzing','publishing','published') AND EXISTS(SELECT 1 FROM private.guided_ingest_intents g WHERE g.import_id=i.id AND g.settled=0 AND g.cancelled=0))) AS active,
 EXISTS(SELECT 1 FROM private.jobs j JOIN private.scan_requests r ON r.job_id=j.id WHERE json_extract(j.request,'$.import_id')=i.id AND r.kind='direct' AND j.state IN ('queued','running','cancel_requested')) AS shared,
 CASE WHEN EXISTS(SELECT 1 FROM private.jobs j WHERE json_extract(j.request,'$.import_id')=i.id AND j.state IN ('queued','running','cancel_requested')) THEN 'active'
 WHEN i.phase='cancelled' THEN 'cancelled'
 WHEN EXISTS(SELECT 1 FROM private.jobs j WHERE json_extract(j.request,'$.import_id')=i.id AND j.state='failed') OR EXISTS(SELECT 1 FROM private.guided_ingest_intents g WHERE g.import_id=i.id AND (g.enrichment_error IS NOT NULL OR g.preparation_error IS NOT NULL)) THEN 'failed'
 WHEN i.phase IN ('staging','awaiting_review') OR (i.phase='analyzing' AND NOT EXISTS(SELECT 1 FROM private.guided_ingest_intents g WHERE g.import_id=i.id AND g.settled=0 AND g.cancelled=0)) THEN 'needs_input'
 WHEN i.phase='published' AND NOT EXISTS(SELECT 1 FROM private.guided_ingest_intents g WHERE g.import_id=i.id AND g.settled=0 AND g.cancelled=0) THEN 'completed' ELSE 'active' END AS status,
 coalesce((SELECT sum(json_extract(value,'$.received_bytes')) FROM json_each(i.files)),0) AS received_bytes,
 coalesce(cl.reclaimed_bytes,0) AS reclaimed_bytes,coalesce(cl.pending_bytes,0) AS pending_bytes,cl.error AS cleanup_error,
 coalesce((SELECT group_concat(json_extract(value,'$.name'),' ') FROM json_each(i.files)),'') AS search_files
 FROM private.imports i LEFT JOIN captures c ON c.id=i.capture_id LEFT JOIN private.import_cleanup cl ON cl.import_id=i.id
 UNION ALL
 SELECT 'job:'||j.id,'job',j.id,j.user_id,j.collection_id,j.item_id,
 replace(j.kind,'_',' '),j.kind,j.phase,j.created_at,j.updated_at,NULL,0,
 j.state IN ('queued','running','cancel_requested'),
 (EXISTS(SELECT 1 FROM private.preparation_members m JOIN private.jobs b ON b.id=m.batch_id WHERE m.job_id=j.id AND b.state IN ('queued','running','cancel_requested')) OR EXISTS(SELECT 1 FROM private.scan_requests sr WHERE sr.job_id=j.id AND sr.kind='import')),
 CASE WHEN j.state IN ('queued','running','cancel_requested') THEN 'active' ELSE j.state END,
 0,0,0,NULL,''
 FROM private.jobs j
 WHERE (NOT EXISTS(SELECT 1 FROM private.imports i WHERE i.id=json_extract(j.request,'$.import_id')) OR EXISTS(SELECT 1 FROM private.scan_requests sr WHERE sr.job_id=j.id AND sr.kind='direct'))
 AND (NOT EXISTS(SELECT 1 FROM private.preparation_members m WHERE m.job_id=j.id) OR EXISTS(SELECT 1 FROM private.derivative_direct_requests d WHERE d.job_id=j.id))
), work AS (
 SELECT r.*,coalesce(json_extract(c.record,'$.name'),'Server work') AS collection_name,it.title AS item_title,
 coalesce(h.archived,0) AS archived,coalesce(h.pending,0) AS clear_pending,h.error AS clear_error
 FROM roots r LEFT JOIN collections c ON c.id=r.collection_id LEFT JOIN items it ON it.id=r.item_id
 LEFT JOIN private.work_history h ON h.user_id=r.user_id AND h.target=r.key
)
SELECT json_object('target',json_object('kind',target_kind,'id',id),'collection_id',collection_id,'collection_name',collection_name,
 'item_id',item_id,'item_title',item_title,'name',name,'kind',kind,'phase',phase,'status',status,'created_at',created_at,'updated_at',updated_at,
 'archived',json(CASE WHEN archived THEN 'true' ELSE 'false' END),'clear_pending',json(CASE WHEN clear_pending THEN 'true' ELSE 'false' END),
 'clear_error',clear_error,'received_bytes',received_bytes,'reclaimed_bytes',reclaimed_bytes,'pending_bytes',pending_bytes,'cleanup_error',cleanup_error,
 'revision',revision,'guided',json(CASE WHEN guided THEN 'true' ELSE 'false' END),'active',json(CASE WHEN active OR status='active' THEN 'true' ELSE 'false' END),'shared',json(CASE WHEN shared THEN 'true' ELSE 'false' END))
FROM work
"#;

fn read_entry(db: &Connection, user: UserId, target: &WorkTarget) -> Result<WorkEntry> {
    let value: String = db
        .query_row(
            &format!("{WORK} WHERE user_id=? AND key=?"),
            params![user.to_string(), target.key()],
            |r| r.get(0),
        )
        .optional()
        .map_err(storage)?
        .ok_or(Error::NotFound)?;
    decode_entry(&value)
}
fn decode_entry(value: &str) -> Result<WorkEntry> {
    let mut entry: WorkEntry = serde_json::from_str(value).map_err(storage)?;
    entry.actions = [
        WorkHistoryAction::Dismiss,
        WorkHistoryAction::Restore,
        WorkHistoryAction::Discard,
        WorkHistoryAction::CancelAndClear,
        WorkHistoryAction::RetryCleanup,
    ]
    .into_iter()
    .filter(|action| collector_app::work_action_allowed(&entry, *action).is_ok())
    .collect();
    Ok(entry)
}
impl Store {
    pub(crate) fn work_entry_record(&self, user: UserId, target: &WorkTarget) -> Result<WorkEntry> {
        read_entry(&*lock(&self.db)?, user, target)
    }
    pub(crate) fn work_page_records(
        &self,
        user: UserId,
        query: &WorkQuery,
    ) -> Result<Page<WorkEntry>> {
        let cursor: Option<(i64, String)> = query
            .cursor
            .as_deref()
            .map(serde_json::from_str)
            .transpose()
            .map_err(|_| Error::Invalid("Invalid work cursor".into()))?;
        let sql = format!(
            "{WORK} WHERE user_id=?1 AND (collection_id IS NULL OR EXISTS (SELECT 1 FROM private.grants g WHERE g.user_id=?1 AND g.collection_id=work.collection_id AND g.role>=1)) AND (?2 IS NULL OR collection_id=?2) AND (?3 IS NULL OR item_id=?3) AND (?4=0 OR item_id IS NULL) AND (?5='' OR instr(lower(name||' '||coalesce(item_title,'')||' '||search_files),lower(?5))>0) AND (?6='' OR status=?6) AND archived=?7 AND (?8 IS NULL OR created_at>=?8) AND (?9 IS NULL OR created_at<=?9) AND (?10 IS NULL OR created_at<?10 OR (created_at=?10 AND key<?11)) ORDER BY created_at DESC,key DESC LIMIT ?12"
        );
        let db = lock(&self.db)?;
        let mut statement = db.prepare(&sql).map_err(storage)?;
        let rows = statement
            .query_map(
                params![
                    user.to_string(),
                    query.collection_id.map(|id| id.to_string()),
                    query.item_id.map(|id| id.to_string()),
                    query.unattached,
                    query.search,
                    query.status,
                    query.archived,
                    query.after,
                    query.before,
                    cursor.as_ref().map(|v| v.0),
                    cursor.as_ref().map(|v| v.1.as_str()),
                    u32::from(query.limit.unwrap_or(50)) + 1
                ],
                |r| r.get::<_, String>(0),
            )
            .map_err(storage)?;
        let mut items = rows
            .map(|r| decode_entry(&r.map_err(storage)?))
            .collect::<Result<Vec<_>>>()?;
        let next_cursor = if items.len() > usize::from(query.limit.unwrap_or(50)) {
            items.pop();
            items
                .last()
                .map(|e| serde_json::to_string(&(e.created_at, e.target.key())).map_err(storage))
                .transpose()?
        } else {
            None
        };
        Ok(Page { items, next_cursor })
    }
    pub(crate) fn change_work_history_record(
        &self,
        user: UserId,
        command: &WorkHistoryCommand,
    ) -> Result<()> {
        let _writer = lock(&self.writer)?;
        let _publication_gate = lock(&self.job_publication_gate)?;
        let _uploads = lock(&self.uploads)?;
        let mut db = lock(&self.db)?;
        let tx = db
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(storage)?;
        let entry = read_entry(&tx, user, &command.target)?;
        collector_app::work_action_allowed(&entry, command.action)?;
        let (archived, pending) = match command.action {
            WorkHistoryAction::Dismiss => (true, false),
            WorkHistoryAction::Restore => (false, false),
            _ => (false, true),
        };
        tx.execute("INSERT INTO private.work_history(user_id,target,archived,pending,action,error) VALUES(?,?,?,?,?,NULL) ON CONFLICT(user_id,target) DO UPDATE SET archived=excluded.archived,pending=excluded.pending,action=excluded.action,error=NULL",params![user.to_string(),command.target.key(),archived,pending,serde_json::to_string(&command.action).map_err(storage)?]).map_err(storage)?;
        self.record_command(&tx, &())?;
        tx.commit().map_err(storage)
    }
    pub(crate) fn reconcile_work_history_records(
        &self,
        settle: &mut dyn FnMut(UserId, &WorkTarget) -> Result<bool>,
    ) -> Result<()> {
        // Only pending intentions are loaded, in bounded pages. A failing entry does not
        // prevent later requests from settling; its error is persisted for the owner.
        let mut after = String::new();
        loop {
            let rows = {
                let db = lock(&self.db)?;
                let mut stmt=db.prepare("SELECT user_id,target FROM private.work_history WHERE pending=1 AND target>? ORDER BY target LIMIT 100").map_err(storage)?;
                stmt.query_map([&after], |r| {
                    Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
                })
                .map_err(storage)?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(storage)?
            };
            if rows.is_empty() {
                break;
            }
            for (user, key) in rows {
                after = key.clone();
                let user = user.parse().map_err(storage)?;
                let (kind, id) = key.split_once(':').ok_or(Error::Conflict)?;
                let target = if kind == "import" {
                    WorkTarget::Import(id.parse().map_err(storage)?)
                } else {
                    WorkTarget::Job(id.parse().map_err(storage)?)
                };
                let result = settle(user, &target);
                match result {
                    Ok(true) => {
                        lock(&self.db)?.execute("UPDATE private.work_history SET pending=0,archived=1,error=NULL WHERE user_id=? AND target=?",params![user.to_string(),key]).map_err(storage)?;
                    }
                    Ok(false) => {}
                    Err(error) => {
                        lock(&self.db)?.execute("UPDATE private.work_history SET error=? WHERE user_id=? AND target=?",params![error.to_string(),user.to_string(),key]).map_err(storage)?;
                    }
                }
            }
        }
        Ok(())
    }
}
