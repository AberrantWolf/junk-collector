//! Private durable intake manifests and fenced child acceptance.
use super::*;
use collector_app::{
    BulkIntakePort, GuidedIngestIntent, IntakePackageRecord, IntakeSession, IntakeState,
    PrivateIntake,
};

pub(crate) fn install(db: &Connection) -> Result<()> {
    db.execute_batch("CREATE TABLE IF NOT EXISTS private.intake_discoveries(id TEXT PRIMARY KEY,user_id TEXT NOT NULL,collection_id TEXT NOT NULL,record TEXT NOT NULL);CREATE TABLE IF NOT EXISTS private.intake_discovery_pages(id TEXT NOT NULL,kind TEXT NOT NULL,position INTEGER NOT NULL,record TEXT NOT NULL,PRIMARY KEY(id,kind,position));CREATE TABLE IF NOT EXISTS private.intake_sessions(id TEXT PRIMARY KEY,user_id TEXT NOT NULL,collection_id TEXT NOT NULL,state TEXT NOT NULL,record TEXT NOT NULL);CREATE INDEX IF NOT EXISTS private.intake_owner ON intake_sessions(user_id,collection_id,id);CREATE INDEX IF NOT EXISTS private.intake_unfinished ON intake_sessions(state,id);CREATE TABLE IF NOT EXISTS private.intake_members(intake_id TEXT NOT NULL,position INTEGER NOT NULL,file_count INTEGER NOT NULL,import_id TEXT UNIQUE,record TEXT NOT NULL,PRIMARY KEY(intake_id,position));CREATE INDEX IF NOT EXISTS private.intake_admission ON intake_members(intake_id,import_id,position);").map_err(storage)?;
    let columns = db
        .prepare("PRAGMA private.table_info(guided_ingest_intents)")
        .map_err(storage)?
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(storage)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(storage)?;
    if !columns.iter().any(|column| column == "bulk_context") {
        db.execute_batch("ALTER TABLE private.guided_ingest_intents ADD COLUMN bulk_context TEXT;")
            .map_err(storage)?;
    }
    Ok(())
}
fn state(state: &IntakeState) -> &'static str {
    match state {
        IntakeState::Draft => "draft",
        IntakeState::Accepted => "accepted",
        IntakeState::Completed => "completed",
        IntakeState::Cancelling => "cancelling",
        IntakeState::Cancelled => "cancelled",
    }
}
fn read_intake(db: &Connection, user: UserId, id: JobId) -> Result<PrivateIntake> {
    let raw: String = db
        .query_row(
            "SELECT record FROM private.intake_sessions WHERE id=? AND user_id=?",
            params![id.to_string(), user.to_string()],
            |row| row.get(0),
        )
        .optional()
        .map_err(storage)?
        .ok_or(Error::NotFound)?;
    serde_json::from_str(&raw).map_err(storage)
}
fn write_intake(db: &Connection, record: &PrivateIntake) -> Result<()> {
    db.execute(
        "UPDATE private.intake_sessions SET state=?,record=? WHERE id=? AND user_id=?",
        params![
            state(&record.session.state),
            serde_json::to_string(record).map_err(storage)?,
            record.session.id.to_string(),
            record.user_id.to_string()
        ],
    )
    .map_err(storage)?;
    Ok(())
}
fn allowed(db: &Connection, user: UserId, collection: CollectionId) -> Result<()> {
    let permitted:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM private.users u JOIN private.grants g ON g.user_id=u.id WHERE u.id=? AND u.enabled=1 AND g.collection_id=? AND g.role>=1)",params![user.to_string(),collection.to_string()],|row|row.get(0)).map_err(storage)?;
    if permitted {
        Ok(())
    } else {
        Err(Error::Forbidden)
    }
}
fn page_limit(limit: u16) -> Result<usize> {
    if !(1..=200).contains(&limit) {
        return Err(Error::Invalid(
            "Intake pages must contain 1–200 members.".into(),
        ));
    }
    Ok(usize::from(limit))
}
impl BulkIntakePort for Store {
    fn reject_intake_member(&self, claim: &JobClaim, position: u32, reason: &str) -> Result<()> {
        let mut db = lock(&self.db)?;
        let tx = db.transaction().map_err(storage)?;
        let mut intake = read_intake(&tx, claim.user_id, claim.id)?;
        let owns:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM private.jobs WHERE id=? AND user_id=? AND state='running' AND claim_token=? AND cancellation_requested=0)",params![claim.id.to_string(),claim.user_id.to_string(),claim.claim_token],|r|r.get(0)).map_err(storage)?;
        if !owns || !matches!(intake.session.state, IntakeState::Accepted) {
            return Err(Error::JobCancelled);
        }
        allowed(&tx, claim.user_id, intake.session.collection_id)?;
        let raw:String=tx.query_row("SELECT record FROM private.intake_members WHERE intake_id=? AND position=? AND import_id IS NULL",params![claim.id.to_string(),position],|r|r.get(0)).map_err(storage)?;
        let mut member: IntakePackageRecord = serde_json::from_str(&raw).map_err(storage)?;
        if member.error.is_none() {
            intake.session.rejected += 1;
        }
        member.error = Some(reason.chars().take(4096).collect());
        tx.execute(
            "UPDATE private.intake_members SET record=? WHERE intake_id=? AND position=?",
            params![
                serde_json::to_string(&member).map_err(storage)?,
                claim.id.to_string(),
                position
            ],
        )
        .map_err(storage)?;
        write_intake(&tx, &intake)?;
        tx.commit().map_err(storage)
    }
    fn repair_intake_destination(
        &self,
        user: UserId,
        id: JobId,
        position: u32,
        command: collector_app::RepairIntakeDestination,
    ) -> Result<IntakeSession> {
        let mut db = lock(&self.db)?;
        let tx = db.transaction().map_err(storage)?;
        let mut intake = read_intake(&tx, user, id)?;
        allowed(&tx, user, intake.session.collection_id)?;
        if !matches!(
            intake.session.state,
            IntakeState::Accepted | IntakeState::Completed
        ) || intake.session.revision != command.expected_revision
        {
            return Err(Error::Conflict);
        }
        let active:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM private.jobs WHERE id=? AND state IN ('running','cancel_requested'))",[id.to_string()],|r|r.get(0)).map_err(storage)?;
        if active {
            return Err(Error::CommandInProgress);
        }
        let raw:String=tx.query_row("SELECT record FROM private.intake_members WHERE intake_id=? AND position=? AND import_id IS NULL",params![id.to_string(),position],|r|r.get(0)).optional().map_err(storage)?.ok_or(Error::Conflict)?;
        let mut member: IntakePackageRecord = serde_json::from_str(&raw).map_err(storage)?;
        if member.error.is_none() {
            return Err(Error::Conflict);
        }
        member.destination = command.destination;
        member.error = None;
        tx.execute(
            "UPDATE private.intake_members SET record=? WHERE intake_id=? AND position=?",
            params![
                serde_json::to_string(&member).map_err(storage)?,
                id.to_string(),
                position
            ],
        )
        .map_err(storage)?;
        intake.session.rejected = intake
            .session
            .rejected
            .checked_sub(1)
            .ok_or(Error::Conflict)?;
        intake.session.state = IntakeState::Accepted;
        intake.session.revision = Revision(
            intake
                .session
                .revision
                .0
                .checked_add(1)
                .ok_or(Error::Conflict)?,
        );
        intake.session.updated_at = self.clock.now();
        tx.execute("UPDATE private.jobs SET state='queued',phase='queued',cancellation_requested=0,claim_token=NULL,worker_instance=NULL,lease_expires=NULL,attempt=attempt+1,error=NULL,updated_at=? WHERE id=?",params![self.clock.now().timestamp(),id.to_string()]).map_err(storage)?;
        write_intake(&tx, &intake)?;
        self.record_command(&tx, &intake.session)?;
        tx.commit().map_err(storage)?;
        Ok(intake.session)
    }
    fn freeze_intake_discovery(
        &self,
        user: UserId,
        discovery: collector_app::IntakeDiscovery,
        packages: Vec<collector_app::IngestPlanPackage>,
        unassigned: Vec<String>,
    ) -> Result<collector_app::IntakeDiscovery> {
        let mut db = lock(&self.db)?;
        let tx = db.transaction().map_err(storage)?;
        allowed(&tx, user, discovery.collection_id)?;
        tx.execute(
            "INSERT INTO private.intake_discoveries VALUES(?,?,?,?)",
            params![
                discovery.id.to_string(),
                user.to_string(),
                discovery.collection_id.to_string(),
                serde_json::to_string(&discovery).map_err(storage)?
            ],
        )
        .map_err(storage)?;
        for (kind, records) in [
            (
                "package",
                packages
                    .iter()
                    .map(serde_json::to_string)
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .map_err(storage)?,
            ),
            (
                "unassigned",
                unassigned
                    .iter()
                    .map(serde_json::to_string)
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .map_err(storage)?,
            ),
        ] {
            for (position, raw) in records.iter().enumerate() {
                if raw.len() > 12 * 1024 * 1024 {
                    return Err(Error::Invalid(
                        "A package manifest exceeds 12 MiB. Shorten paths or split the package."
                            .into(),
                    ));
                }
                tx.execute(
                    "INSERT INTO private.intake_discovery_pages VALUES(?,?,?,?)",
                    params![discovery.id.to_string(), kind, position, raw],
                )
                .map_err(storage)?;
            }
        }
        self.record_command(&tx, &discovery)?;
        tx.commit().map_err(storage)?;
        Ok(discovery)
    }
    fn intake_discovery_page(
        &self,
        user: UserId,
        id: JobId,
        after: Option<u32>,
        unassigned_after: Option<u32>,
    ) -> Result<collector_app::IntakeDiscoveryPage> {
        let db = lock(&self.db)?;
        let raw: Option<String> = db
            .query_row(
                "SELECT record FROM private.intake_discoveries WHERE id=? AND user_id=?",
                params![id.to_string(), user.to_string()],
                |r| r.get(0),
            )
            .optional()
            .map_err(storage)?;
        let discovery: collector_app::IntakeDiscovery =
            serde_json::from_str(&raw.ok_or(Error::NotFound)?).map_err(storage)?;
        allowed(&db, user, discovery.collection_id)?;
        fn page<T: serde::de::DeserializeOwned>(
            db: &Connection,
            id: JobId,
            kind: &str,
            after: Option<u32>,
            limit: u32,
        ) -> Result<Page<T>> {
            let mut stmt=db.prepare("SELECT position,record FROM private.intake_discovery_pages WHERE id=? AND kind=? AND (? IS NULL OR position>?) ORDER BY position LIMIT ?").map_err(storage)?;
            let mut rows = stmt
                .query(params![id.to_string(), kind, after, after, limit + 1])
                .map_err(storage)?;
            let mut items = Vec::new();
            let mut size = 0;
            let mut last = None;
            let mut more = false;
            while let Some(row) = rows.next().map_err(storage)? {
                let raw: String = row.get(1).map_err(storage)?;
                if items.len() >= limit as usize
                    || (!items.is_empty() && size + raw.len() > 12 * 1024 * 1024)
                {
                    more = true;
                    break;
                }
                size += raw.len();
                last = Some(row.get::<_, u32>(0).map_err(storage)?);
                items.push(serde_json::from_str(&raw).map_err(storage)?);
            }
            Ok(Page {
                items,
                next_cursor: if more {
                    last.map(|v| v.to_string())
                } else {
                    None
                },
            })
        }
        Ok(collector_app::IntakeDiscoveryPage {
            discovery,
            packages: page(&db, id, "package", after, 64)?,
            unassigned_files: page(&db, id, "unassigned", unassigned_after, 1000)?,
        })
    }
    fn create_intake(&self, record: PrivateIntake) -> Result<IntakeSession> {
        let mut db = lock(&self.db)?;
        let tx = db.transaction().map_err(storage)?;
        allowed(&tx, record.user_id, record.session.collection_id)?;
        tx.execute(
            "INSERT INTO private.intake_sessions VALUES(?,?,?,?,?)",
            params![
                record.session.id.to_string(),
                record.user_id.to_string(),
                record.session.collection_id.to_string(),
                state(&record.session.state),
                serde_json::to_string(&record).map_err(storage)?
            ],
        )
        .map_err(storage)?;
        self.record_command(&tx, &record.session)?;
        tx.commit().map_err(storage)?;
        Ok(record.session)
    }
    fn intake(&self, user: UserId, id: JobId) -> Result<PrivateIntake> {
        let db = lock(&self.db)?;
        let record = read_intake(&db, user, id)?;
        allowed(&db, user, record.session.collection_id)?;
        Ok(record)
    }
    fn intake_sessions(
        &self,
        user: UserId,
        collection: Option<CollectionId>,
        after: Option<JobId>,
        limit: u16,
    ) -> Result<Page<IntakeSession>> {
        let limit = page_limit(limit)?;
        let db = lock(&self.db)?;
        let mut stmt=db.prepare("SELECT s.record FROM private.intake_sessions s JOIN private.grants g ON g.collection_id=s.collection_id AND g.user_id=s.user_id AND g.role>=1 JOIN private.users u ON u.id=s.user_id AND u.enabled=1 WHERE s.user_id=? AND (? IS NULL OR s.collection_id=?) AND (? IS NULL OR s.id>?) ORDER BY s.id LIMIT ?").map_err(storage)?;
        let mut items = stmt
            .query_map(
                params![
                    user.to_string(),
                    collection.map(|id| id.to_string()),
                    collection.map(|id| id.to_string()),
                    after.map(|id| id.to_string()),
                    after.map(|id| id.to_string()),
                    limit + 1
                ],
                |row| row.get::<_, String>(0),
            )
            .map_err(storage)?
            .map(|row| {
                serde_json::from_str::<PrivateIntake>(&row.map_err(storage)?)
                    .map(|record| record.session)
                    .map_err(storage)
            })
            .collect::<Result<Vec<_>>>()?;
        let next_cursor = if items.len() > limit {
            items.pop();
            items.last().map(|session| session.id.to_string())
        } else {
            None
        };
        Ok(Page { items, next_cursor })
    }
    fn append_intake(
        &self,
        user: UserId,
        id: JobId,
        expected: Revision,
        mut packages: Vec<IntakePackageRecord>,
    ) -> Result<IntakeSession> {
        if packages.is_empty() || packages.len() > 64 {
            return Err(Error::Invalid("Append 1–64 packages at a time.".into()));
        }
        let mut db = lock(&self.db)?;
        let tx = db.transaction().map_err(storage)?;
        let mut record = read_intake(&tx, user, id)?;
        allowed(&tx, user, record.session.collection_id)?;
        if !matches!(record.session.state, IntakeState::Draft)
            || record.session.revision != expected
        {
            return Err(Error::Conflict);
        }
        let package_count = record
            .session
            .package_count
            .checked_add(packages.len() as u32)
            .ok_or(Error::Conflict)?;
        let files = packages
            .iter()
            .try_fold(0u32, |sum, member| {
                sum.checked_add(member.plan.files.len() as u32)
            })
            .ok_or(Error::Conflict)?;
        let file_count = record
            .session
            .file_count
            .checked_add(files)
            .ok_or(Error::Conflict)?;
        if package_count > collector_app::INTAKE_PACKAGE_LIMIT
            || file_count > collector_app::INTAKE_FILE_LIMIT
            || package_count > record.session.expected_packages
            || file_count > record.session.expected_files
        {
            return Err(Error::Invalid("This intake exceeds 10,000 packages or 100,000 files. Create another intake for the remaining packages; this page was not added.".into()));
        }
        for (offset, member) in packages.iter_mut().enumerate() {
            member.position = record.session.package_count + offset as u32;
            if serde_json::to_vec(&member).map_err(storage)?.len() > 12 * 1024 * 1024 {
                return Err(Error::Invalid("This package manifest exceeds 12 MiB. Shorten path names or split the package.".into()));
            }
            tx.execute(
                "INSERT INTO private.intake_members VALUES(?,?,?,NULL,?)",
                params![
                    id.to_string(),
                    member.position,
                    member.plan.files.len(),
                    serde_json::to_string(member).map_err(storage)?
                ],
            )
            .map_err(storage)?;
        }
        record.session.package_count = package_count;
        record.session.file_count = file_count;
        record.session.revision = Revision(expected.0.checked_add(1).ok_or(Error::Conflict)?);
        record.session.updated_at = self.clock.now();
        write_intake(&tx, &record)?;
        self.record_command(&tx, &record.session)?;
        tx.commit().map_err(storage)?;
        Ok(record.session)
    }
    fn accept_intake(&self, user: UserId, id: JobId, expected: Revision) -> Result<IntakeSession> {
        let mut db = lock(&self.db)?;
        let tx = db.transaction().map_err(storage)?;
        let mut record = read_intake(&tx, user, id)?;
        allowed(&tx, user, record.session.collection_id)?;
        if !matches!(record.session.state, IntakeState::Draft)
            || record.session.revision != expected
            || record.session.package_count == 0
        {
            return Err(Error::Conflict);
        }
        if record.session.package_count != record.session.expected_packages
            || record.session.file_count != record.session.expected_files
        {
            return Err(Error::Invalid("The package manifest is incomplete. Resume saving the reviewed packages before starting this import.".into()));
        }
        let conflicting_group: Option<String> = tx.query_row(
            "SELECT json_extract(record,'$.destination.key') FROM private.intake_members WHERE intake_id=? AND json_extract(record,'$.destination.destination')='album_group' GROUP BY json_extract(record,'$.destination.key') HAVING min(json_extract(record,'$.destination.title'))<>max(json_extract(record,'$.destination.title')) LIMIT 1",
            [id.to_string()], |row| row.get(0),
        ).optional().map_err(storage)?;
        if let Some(group) = conflicting_group {
            return Err(Error::Invalid(format!(
                "Album group {group} has conflicting titles. Give every package in that group the same album title before starting."
            )));
        }
        record.session.state = IntakeState::Accepted;
        record.session.revision = Revision(expected.0.checked_add(1).ok_or(Error::Conflict)?);
        record.session.updated_at = self.clock.now();
        tx.execute("INSERT INTO private.jobs(id,user_id,collection_id,item_id,kind,state,phase,completed_units,total_units,attempt,created_at,updated_at,error,limitation,request,result,worker_instance,lease_expires,claim_token,cancellation_requested) VALUES(?,?,?,NULL,'bulk_intake','queued','queued',0,?,1,?,?,NULL,NULL,?,NULL,NULL,NULL,NULL,0)",params![id.to_string(),user.to_string(),record.session.collection_id.to_string(),record.session.package_count,record.session.created_at.timestamp(),record.session.updated_at.timestamp(),serde_json::json!({"intake_id":id}).to_string()]).map_err(storage)?;
        write_intake(&tx, &record)?;
        self.record_command(&tx, &record.session)?;
        tx.commit().map_err(storage)?;
        Ok(record.session)
    }
    fn intake_members(
        &self,
        user: UserId,
        id: JobId,
        after: Option<u32>,
        limit: u16,
        pending: bool,
    ) -> Result<Page<IntakePackageRecord>> {
        let limit = page_limit(limit)?;
        let db = lock(&self.db)?;
        let record = read_intake(&db, user, id)?;
        allowed(&db, user, record.session.collection_id)?;
        let mut stmt=db.prepare("SELECT record FROM private.intake_members WHERE intake_id=? AND (?=0 OR (import_id IS NULL AND json_extract(record,'$.error') IS NULL)) AND (? IS NULL OR position>?) ORDER BY position LIMIT ?").map_err(storage)?;
        let mut rows = stmt
            .query(params![id.to_string(), pending, after, after, limit + 1])
            .map_err(storage)?;
        let mut items = Vec::new();
        let mut bytes = 0usize;
        let mut more = false;
        while let Some(row) = rows.next().map_err(storage)? {
            let raw: String = row.get(0).map_err(storage)?;
            if items.len() >= limit || (!items.is_empty() && bytes + raw.len() > 12 * 1024 * 1024) {
                more = true;
                break;
            }
            if raw.len() > 12 * 1024 * 1024 {
                return Err(Error::Storage(
                    "Stored package manifest exceeds its 12 MiB limit.".into(),
                ));
            }
            bytes += raw.len();
            items.push(serde_json::from_str::<IntakePackageRecord>(&raw).map_err(storage)?);
        }
        let next_cursor = if more {
            items.last().map(|member| member.position.to_string())
        } else {
            None
        };
        Ok(Page { items, next_cursor })
    }
    fn admit_intake_members(
        &self,
        claim: &JobClaim,
        positions: &[u32],
        records: Vec<collector_app::ImportRecord>,
        intents: Vec<GuidedIngestIntent>,
    ) -> Result<()> {
        if positions.len() != records.len() || records.len() != intents.len() {
            return Err(Error::Invalid(
                "Intake child admission does not match its manifest page.".into(),
            ));
        }
        let _writer = lock(&self.writer)?;
        let mut db = lock(&self.db)?;
        let tx = db.transaction().map_err(storage)?;
        let mut intake = read_intake(&tx, claim.user_id, claim.id)?;
        allowed(&tx, claim.user_id, intake.session.collection_id)?;
        let owns:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM private.jobs WHERE id=? AND user_id=? AND kind='bulk_intake' AND state='running' AND claim_token=? AND cancellation_requested=0)",params![claim.id.to_string(),claim.user_id.to_string(),claim.claim_token],|row|row.get(0)).map_err(storage)?;
        if !owns || !matches!(intake.session.state, IntakeState::Accepted) {
            return Err(Error::JobCancelled);
        }
        let views = self.accept_guided_records_in_transaction(&tx, &records, &intents)?;
        for ((position, view), intent) in positions.iter().zip(views).zip(&intents) {
            if intent
                .bulk_context
                .as_ref()
                .is_none_or(|context| context.intake_id != claim.id)
            {
                return Err(Error::Invalid("Intake child ownership is absent.".into()));
            }
            let raw:String=tx.query_row("SELECT record FROM private.intake_members WHERE intake_id=? AND position=? AND import_id IS NULL",params![claim.id.to_string(),position],|row|row.get(0)).optional().map_err(storage)?.ok_or(Error::Conflict)?;
            let mut member: IntakePackageRecord = serde_json::from_str(&raw).map_err(storage)?;
            if member.plan.files.len() != view.files.len()
                || member
                    .plan
                    .files
                    .iter()
                    .zip(&view.files)
                    .any(|(expected, actual)| {
                        expected.name != actual.name || expected.bytes != actual.bytes
                    })
            {
                return Err(Error::Conflict);
            }
            member.import_id = Some(view.id);
            tx.execute("UPDATE private.intake_members SET import_id=?,record=? WHERE intake_id=? AND position=?",params![view.id.to_string(),serde_json::to_string(&member).map_err(storage)?,claim.id.to_string(),position]).map_err(storage)?;
        }
        intake.session.admitted = intake
            .session
            .admitted
            .checked_add(positions.len() as u32)
            .ok_or(Error::Conflict)?;
        intake.session.updated_at = self.clock.now();
        write_intake(&tx, &intake)?;
        tx.commit().map_err(storage)
    }
    fn cancel_intake(&self, user: UserId, id: JobId) -> Result<IntakeSession> {
        self.cancel_intake_record(user, id, true)
    }
    fn unfinished_intakes(&self, after: Option<JobId>, limit: u16) -> Result<Vec<PrivateIntake>> {
        page_limit(limit)?;
        let db = lock(&self.db)?;
        let mut stmt=db.prepare("SELECT record FROM private.intake_sessions WHERE state IN ('accepted','cancelling') AND (? IS NULL OR id>?) ORDER BY id LIMIT ?").map_err(storage)?;
        stmt.query_map(
            params![
                after.map(|id| id.to_string()),
                after.map(|id| id.to_string()),
                limit
            ],
            |row| row.get::<_, String>(0),
        )
        .map_err(storage)?
        .map(|row| serde_json::from_str(&row.map_err(storage)?).map_err(storage))
        .collect()
    }
    fn reconcile_intake_state(&self, user: UserId, id: JobId) -> Result<IntakeSession> {
        let mut db = lock(&self.db)?;
        let tx = db.transaction().map_err(storage)?;
        let mut record = read_intake(&tx, user, id)?;
        let (preserved,attention,unfinished,cancelled):(u32,u32,u32,u32)=tx.query_row("SELECT COALESCE(sum(p.phase='published'),0),COALESCE(sum(g.enrichment_error IS NOT NULL OR g.preparation_error IS NOT NULL OR EXISTS(SELECT 1 FROM private.jobs j WHERE json_extract(j.request,'$.import_id')=p.id AND j.state='failed')),0),COALESCE(sum(g.settled=0 AND g.cancelled=0),0),COALESCE(sum(g.cancelled=1),0) FROM private.intake_members m LEFT JOIN private.imports p ON p.id=m.import_id LEFT JOIN private.guided_ingest_intents g ON g.import_id=m.import_id WHERE m.intake_id=?",[id.to_string()],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?))).map_err(storage)?;
        record.session.preserved = preserved;
        record.session.needs_attention = attention + record.session.rejected;
        let cancelling:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM private.jobs j WHERE j.state IN ('running','cancel_requested') AND j.cancellation_requested=1 AND (j.id=?1 OR json_extract(j.request,'$.import_id') IN (SELECT import_id FROM private.intake_members WHERE intake_id=?1) OR EXISTS(SELECT 1 FROM private.preparation_members child JOIN private.jobs parent ON parent.id=child.batch_id JOIN private.intake_members m ON m.import_id=json_extract(parent.request,'$.import_id') WHERE child.job_id=j.id AND m.intake_id=?1)))",[id.to_string()],|r|r.get(0)).map_err(storage)?;
        if matches!(record.session.state, IntakeState::Cancelling)
            && !cancelling
            && cancelled == record.session.admitted
        {
            record.session.state = IntakeState::Cancelled;
        }
        if matches!(record.session.state, IntakeState::Accepted)
            && record.session.admitted + record.session.rejected == record.session.package_count
            && unfinished == 0
        {
            record.session.state = IntakeState::Completed;
        }
        record.session.updated_at = self.clock.now();
        write_intake(&tx, &record)?;
        tx.commit().map_err(storage)?;
        Ok(record.session)
    }
    fn intake_album_preservation_settled(
        &self,
        user: UserId,
        id: JobId,
        item: ItemId,
    ) -> Result<bool> {
        let db = lock(&self.db)?;
        let record = read_intake(&db, user, id)?;
        if record.session.admitted + record.session.rejected < record.session.package_count {
            return Ok(false);
        }
        db.query_row("SELECT NOT EXISTS(SELECT 1 FROM private.intake_members m JOIN private.guided_ingest_intents g ON g.import_id=m.import_id JOIN private.imports p ON p.id=m.import_id WHERE m.intake_id=? AND g.target_item_id=? AND p.phase NOT IN ('published','cancelled') AND NOT EXISTS(SELECT 1 FROM private.jobs j WHERE json_extract(j.request,'$.import_id')=p.id AND j.kind='import_publication' AND j.state='failed'))",params![id.to_string(),item.to_string()],|row|row.get(0)).map_err(storage)
    }
}

impl Store {
    pub(crate) fn cancel_intake_record(
        &self,
        user: UserId,
        id: JobId,
        summary_receipt: bool,
    ) -> Result<IntakeSession> {
        let _publication_gate = lock(&self.job_publication_gate)?;
        let mut db = lock(&self.db)?;
        let tx = db.transaction().map_err(storage)?;
        let mut record = read_intake(&tx, user, id)?;
        allowed(&tx, user, record.session.collection_id)?;
        if matches!(
            record.session.state,
            IntakeState::Cancelled | IntakeState::Cancelling
        ) {
            if summary_receipt {
                self.record_command(&tx, &record.session)?;
            } else {
                self.record_command(&tx, &())?;
            }
            tx.commit().map_err(storage)?;
            return Ok(record.session);
        }
        let imports = {
            let mut statement=tx.prepare("SELECT import_id FROM private.intake_members WHERE intake_id=? AND import_id IS NOT NULL ORDER BY position").map_err(storage)?;
            statement
                .query_map([id.to_string()], |r| r.get::<_, String>(0))
                .map_err(storage)?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(storage)?
        };
        Self::cancel_guided_requests_in_tx(&tx, user, &imports, self.clock.now().timestamp())?;
        record.session.state = IntakeState::Cancelling;
        record.session.revision = Revision(
            record
                .session
                .revision
                .0
                .checked_add(1)
                .ok_or(Error::Conflict)?,
        );
        record.session.updated_at = self.clock.now();
        tx.execute("UPDATE private.jobs SET state=CASE WHEN state='running' THEN 'cancel_requested' WHEN state='queued' THEN 'cancelled' ELSE state END,cancellation_requested=1 WHERE id=? AND user_id=?",params![id.to_string(),user.to_string()]).map_err(storage)?;
        write_intake(&tx, &record)?;
        if summary_receipt {
            self.record_command(&tx, &record.session)?;
        } else {
            self.record_command(&tx, &())?;
        }
        tx.commit().map_err(storage)?;
        Ok(record.session)
    }
}
