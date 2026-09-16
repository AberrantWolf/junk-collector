use super::*;
use crate::files::safe_name;
use collector_app::{
    GuidedIngestIntent, ImportIdentityDecision, ImportPhase, ImportRecord, ImportSourceKind,
    IngestDiscardResult, IngestEnrichmentPolicy, PortableProcessingRecord, ProcessingPort,
    ProvisionalImportPort,
};
use rusqlite::TransactionBehavior;
use std::collections::HashSet;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

const IMPORT_CHUNK_LIMIT: usize = 8 * 1024 * 1024;

fn source_kind(value: ImportSourceKind) -> &'static str {
    match value {
        ImportSourceKind::Browser => "browser",
        ImportSourceKind::Inbox => "inbox",
    }
}

fn phase(value: ImportPhase) -> &'static str {
    match value {
        ImportPhase::Staging => "staging",
        ImportPhase::Analyzing => "analyzing",
        ImportPhase::AwaitingReview => "awaiting_review",
        ImportPhase::Publishing => "publishing",
        ImportPhase::Published => "published",
        ImportPhase::Cancelled => "cancelled",
    }
}

fn parse_source(value: &str) -> Result<ImportSourceKind> {
    match value {
        "browser" => Ok(ImportSourceKind::Browser),
        "inbox" => Ok(ImportSourceKind::Inbox),
        _ => Err(Error::Storage(
            "stored Import source kind is invalid".into(),
        )),
    }
}

fn parse_phase(value: &str) -> Result<ImportPhase> {
    match value {
        "staging" => Ok(ImportPhase::Staging),
        "analyzing" => Ok(ImportPhase::Analyzing),
        "awaiting_review" => Ok(ImportPhase::AwaitingReview),
        "publishing" => Ok(ImportPhase::Publishing),
        "published" => Ok(ImportPhase::Published),
        "cancelled" => Ok(ImportPhase::Cancelled),
        _ => Err(Error::Storage("stored Import phase is invalid".into())),
    }
}

fn enrichment(value: IngestEnrichmentPolicy) -> &'static str {
    match value {
        IngestEnrichmentPolicy::Automatic => "automatic",
        IngestEnrichmentPolicy::None => "none",
    }
}

fn parse_enrichment(value: &str) -> Result<IngestEnrichmentPolicy> {
    match value {
        "automatic" => Ok(IngestEnrichmentPolicy::Automatic),
        "none" => Ok(IngestEnrichmentPolicy::None),
        _ => Err(Error::Storage(
            "stored guided enrichment policy is invalid".into(),
        )),
    }
}

fn safe_summary(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 4096
        && !value.contains(['/', '\\'])
        && !value.chars().any(char::is_control)
}

fn validate(record: &ImportRecord) -> Result<()> {
    let view = &record.view;
    if view.revision == 0 || !safe_summary(&view.source.package_name) {
        return Err(Error::Invalid(
            "Import revision and direct package name are required".into(),
        ));
    }
    match (&view.source.kind, &view.source.label) {
        (ImportSourceKind::Browser, None) => {}
        (ImportSourceKind::Inbox, Some(label)) if safe_summary(label) => {}
        _ => {
            return Err(Error::Invalid(
                "browser imports have no source label; inbox imports require a configured label"
                    .into(),
            ));
        }
    }
    if view.files.is_empty() || view.files.len() > 10_000 {
        return Err(Error::Invalid(
            "Import file inventory is empty or too large".into(),
        ));
    }
    let mut names = HashSet::new();
    for file in &view.files {
        crate::files::safe_name(&file.name)?;
        if !names.insert(file.name.to_lowercase()) {
            return Err(Error::Invalid(
                "Import file inventory has a case-colliding name".into(),
            ));
        }
        if file.received_bytes.0 > file.bytes.0 {
            return Err(Error::Invalid("Import offset exceeds declared size".into()));
        }
        if let Some(digest) = &file.digest {
            digest.validate()?;
        }
    }
    if view.candidates.len() > 128 || view.limitations.len() > 64 {
        return Err(Error::Invalid(
            "Import candidates or limitations exceed their bound".into(),
        ));
    }
    let mut candidate_keys = HashSet::new();
    for candidate in &view.candidates {
        candidate.module.validate()?;
        if candidate.key.is_empty()
            || candidate.key.len() > 200
            || candidate.label.len() > 4096
            || candidate.observation.get().len() > 64 * 1024
            || !candidate_keys.insert(candidate.key.as_str())
        {
            return Err(Error::Invalid(
                "Import candidate is invalid or duplicated".into(),
            ));
        }
    }
    if view
        .limitations
        .iter()
        .any(|value| value.is_empty() || value.len() > 4096)
    {
        return Err(Error::Invalid("Import limitation text is invalid".into()));
    }
    if view.association.item_id.is_none() && view.expected_item_revision.is_some() {
        return Err(Error::Invalid(
            "an unattached Import cannot expect an Item revision".into(),
        ));
    }
    if let Some(review) = &view.review {
        if review.association != view.association {
            return Err(Error::Invalid(
                "Import review and association must describe the same target".into(),
            ));
        }
        if let ImportIdentityDecision::Candidate { key } = &review.identity
            && !candidate_keys.contains(key.as_str())
        {
            return Err(Error::Invalid(
                "Import review selected an absent candidate".into(),
            ));
        }
    }
    if matches!(view.phase, ImportPhase::Staging | ImportPhase::Analyzing) && view.review.is_some()
    {
        return Err(Error::Invalid(
            "Import review cannot precede the review phase".into(),
        ));
    }
    if matches!(view.phase, ImportPhase::Publishing | ImportPhase::Published)
        && (view.review.is_none()
            || view
                .files
                .iter()
                .any(|file| file.received_bytes != file.bytes || file.digest.is_none()))
    {
        return Err(Error::Invalid(
            "publishing requires a review decision and completely staged, digested files".into(),
        ));
    }
    Ok(())
}

fn stored_json(value: &impl Serialize, limit: usize, label: &str) -> Result<String> {
    let value = serde_json::to_string(value).map_err(storage)?;
    if value.len() > limit {
        return Err(Error::Invalid(format!(
            "Import {label} exceeds its storage bound"
        )));
    }
    Ok(value)
}

type StoredImport = (
    String,
    String,
    u64,
    String,
    String,
    Option<String>,
    String,
    String,
    String,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<u64>,
    String,
    String,
    Option<String>,
    Option<String>,
    String,
    i64,
    i64,
);

fn read(row: &rusqlite::Row<'_>) -> rusqlite::Result<StoredImport> {
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
        row.get(15)?,
        row.get(16)?,
        row.get(17)?,
        row.get(18)?,
        row.get(19)?,
        row.get(20)?,
        row.get(21)?,
    ))
}

fn parse(row: StoredImport) -> Result<ImportRecord> {
    let association = CaptureAssociation {
        item_id: row
            .10
            .map(|value| value.parse().map_err(storage))
            .transpose()?,
        specimen_id: row
            .11
            .map(|value| value.parse().map_err(storage))
            .transpose()?,
        component_id: row
            .12
            .map(|value| value.parse().map_err(storage))
            .transpose()?,
        carrier_id: row
            .13
            .map(|value| value.parse().map_err(storage))
            .transpose()?,
    };
    let record = ImportRecord {
        user_id: row.1.parse().map_err(storage)?,
        view: collector_app::ImportView {
            id: row.0.parse().map_err(storage)?,
            revision: row.2,
            collection_id: row.3.parse().map_err(storage)?,
            source: collector_app::ImportSourceSummary {
                kind: parse_source(&row.4)?,
                label: row.5,
                package_name: row.6,
            },
            files: serde_json::from_str(&row.7).map_err(storage)?,
            capture_id: row.8.parse().map_err(storage)?,
            representation_id: row.9.parse().map_err(storage)?,
            association,
            expected_item_revision: row.14.map(Revision),
            candidates: serde_json::from_str(&row.15).map_err(storage)?,
            limitations: serde_json::from_str(&row.16).map_err(storage)?,
            review: row
                .17
                .map(|value| serde_json::from_str(&value).map_err(storage))
                .transpose()?,
            analysis_job_id: row
                .18
                .map(|value| value.parse().map_err(storage))
                .transpose()?,
            phase: parse_phase(&row.19)?,
            created_at: chrono::DateTime::from_timestamp(row.20, 0)
                .ok_or_else(|| Error::Storage("Import creation timestamp is invalid".into()))?,
            updated_at: chrono::DateTime::from_timestamp(row.21, 0)
                .ok_or_else(|| Error::Storage("Import update timestamp is invalid".into()))?,
        },
    };
    validate(&record).map_err(|error| Error::Storage(error.to_string()))?;
    Ok(record)
}

const SELECT: &str = "SELECT id,user_id,revision,collection_id,source_kind,source_label,package_name,files,capture_id,representation_id,item_id,specimen_id,component_id,carrier_id,expected_item_revision,candidates,limitations,review,analysis_job_id,phase,created_at,updated_at FROM private.imports";

fn insert_import(tx: &rusqlite::Transaction<'_>, record: &ImportRecord) -> Result<()> {
    validate(record)?;
    if record.view.revision != 1 {
        return Err(Error::Invalid("a new Import starts at revision 1".into()));
    }
    let files = stored_json(&record.view.files, 16 * 1024 * 1024, "files")?;
    let candidates = stored_json(&record.view.candidates, 1024 * 1024, "candidates")?;
    let limitations = stored_json(&record.view.limitations, 256 * 1024, "limitations")?;
    let review = record
        .view
        .review
        .as_ref()
        .map(|value| stored_json(value, 1024 * 1024, "review"))
        .transpose()?;
    tx.execute(
        "INSERT INTO private.imports(id,user_id,revision,collection_id,source_kind,source_label,package_name,files,capture_id,representation_id,item_id,specimen_id,component_id,carrier_id,expected_item_revision,candidates,limitations,review,analysis_job_id,phase,created_at,updated_at) VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
        params![record.view.id.to_string(),record.user_id.to_string(),record.view.revision,record.view.collection_id.to_string(),source_kind(record.view.source.kind),record.view.source.label,record.view.source.package_name,files,record.view.capture_id.to_string(),record.view.representation_id.to_string(),record.view.association.item_id.map(|id|id.to_string()),record.view.association.specimen_id.map(|id|id.to_string()),record.view.association.component_id.map(|id|id.to_string()),record.view.association.carrier_id.map(|id|id.to_string()),record.view.expected_item_revision.map(|revision|revision.0),candidates,limitations,review,record.view.analysis_job_id.map(|id|id.to_string()),phase(record.view.phase),record.view.created_at.timestamp(),record.view.updated_at.timestamp()],
    )
    .map_err(|error| {
        if error.to_string().contains("UNIQUE constraint failed") {
            Error::IdempotencyConflict
        } else {
            storage(error)
        }
    })?;
    Ok(())
}

impl ProvisionalImportPort for Store {
    fn create_import(&self, record: ImportRecord) -> Result<ImportRecord> {
        let mut db = lock(&self.db)?;
        let tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage)?;
        insert_import(&tx, &record)?;
        self.record_command(&tx, &record.view)?;
        tx.commit().map_err(storage)?;
        Ok(record)
    }

    fn import_record(&self, id: ImportId) -> Result<ImportRecord> {
        let sql = format!("{SELECT} WHERE id=?");
        let row = lock(&self.db)?
            .query_row(&sql, [id.to_string()], read)
            .optional()
            .map_err(storage)?
            .ok_or(Error::NotFound)?;
        parse(row)
    }

    fn save_import(
        &self,
        mut record: ImportRecord,
        expected_revision: u64,
    ) -> Result<ImportRecord> {
        if record.view.revision != expected_revision {
            return Err(Error::Conflict);
        }
        let current = self.import_record(record.view.id)?;
        let immutable_current = serde_json::to_value((
            current.user_id,
            current.view.collection_id,
            &current.view.source,
            current.view.capture_id,
            current.view.representation_id,
            current.view.created_at,
        ))
        .map_err(storage)?;
        let immutable_next = serde_json::to_value((
            record.user_id,
            record.view.collection_id,
            &record.view.source,
            record.view.capture_id,
            record.view.representation_id,
            record.view.created_at,
        ))
        .map_err(storage)?;
        if immutable_current != immutable_next {
            return Err(Error::Invalid("Import immutable identity changed".into()));
        }
        record.view.revision = expected_revision
            .checked_add(1)
            .ok_or_else(|| Error::Invalid("Import revision overflow".into()))?;
        record.view.updated_at = self.clock.now();
        validate(&record)?;
        let files = stored_json(&record.view.files, 16 * 1024 * 1024, "files")?;
        let candidates = stored_json(&record.view.candidates, 1024 * 1024, "candidates")?;
        let limitations = stored_json(&record.view.limitations, 256 * 1024, "limitations")?;
        let review = record
            .view
            .review
            .as_ref()
            .map(|value| stored_json(value, 1024 * 1024, "review"))
            .transpose()?;
        let mut db = lock(&self.db)?;
        let tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage)?;
        let changed = tx.execute(
            "UPDATE private.imports SET revision=?,files=?,item_id=?,specimen_id=?,component_id=?,carrier_id=?,expected_item_revision=?,candidates=?,limitations=?,review=?,analysis_job_id=?,phase=?,updated_at=? WHERE id=? AND revision=?",
            params![record.view.revision,files,record.view.association.item_id.map(|id|id.to_string()),record.view.association.specimen_id.map(|id|id.to_string()),record.view.association.component_id.map(|id|id.to_string()),record.view.association.carrier_id.map(|id|id.to_string()),record.view.expected_item_revision.map(|revision|revision.0),candidates,limitations,review,record.view.analysis_job_id.map(|id|id.to_string()),phase(record.view.phase),record.view.updated_at.timestamp(),record.view.id.to_string(),expected_revision],
        ).map_err(storage)?;
        if changed != 1 {
            return Err(Error::Conflict);
        }
        self.record_command(&tx, &record.view)?;
        tx.commit().map_err(storage)?;
        Ok(record)
    }

    fn stage_import_chunk(
        &self,
        id: ImportId,
        index: usize,
        offset: u64,
        bytes: &[u8],
    ) -> Result<ImportRecord> {
        if bytes.len() > IMPORT_CHUNK_LIMIT {
            return Err(Error::Invalid("chunk exceeds 8 MiB".into()));
        }
        let _guard = lock(&self.uploads)?;
        let mut record = self.import_record(id)?;
        if record.view.phase == ImportPhase::Cancelled {
            return Err(Error::Conflict);
        }
        let declared = record.view.files.get(index).ok_or(Error::NotFound)?;
        let end = offset
            .checked_add(bytes.len() as u64)
            .ok_or_else(|| Error::Invalid("offset overflow".into()))?;
        if offset > declared.received_bytes.0 || end > declared.bytes.0 {
            return Err(Error::Conflict);
        }
        let directory = self.state.join("imports").join(id.to_string());
        // A lost response may be retried after staging has completed. Verify
        // durable bytes read-only; later phases must never mutate the source.
        if record.view.phase != ImportPhase::Staging {
            if end > declared.received_bytes.0 {
                return Err(Error::Conflict);
            }
            let mut file = File::open(directory.join(format!("{index}.part"))).map_err(storage)?;
            let mut previous = vec![0; bytes.len()];
            file.seek(SeekFrom::Start(offset)).map_err(storage)?;
            file.read_exact(&mut previous).map_err(storage)?;
            return if previous == bytes {
                Ok(record)
            } else {
                Err(Error::Conflict)
            };
        }
        fs::create_dir_all(&directory).map_err(storage)?;
        let path = directory.join(format!("{index}.part"));
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&path)
            .map_err(storage)?;
        let mut current = file.metadata().map_err(storage)?.len();
        if current < offset {
            return Err(Error::Storage(
                "Import scratch is shorter than its durable offset".into(),
            ));
        }
        if current > declared.received_bytes.0 {
            file.set_len(declared.received_bytes.0).map_err(storage)?;
            file.sync_all().map_err(storage)?;
            current = declared.received_bytes.0;
        }
        if current > offset {
            if end > current || end > declared.received_bytes.0 {
                return Err(Error::Conflict);
            }
            let mut previous = vec![0; bytes.len()];
            file.seek(SeekFrom::Start(offset)).map_err(storage)?;
            file.read_exact(&mut previous).map_err(storage)?;
            if previous != bytes {
                return Err(Error::Conflict);
            }
            return Ok(record);
        } else {
            file.seek(SeekFrom::End(0)).map_err(storage)?;
            file.write_all(bytes).map_err(storage)?;
            file.sync_all().map_err(storage)?;
        }
        let length = file.metadata().map_err(storage)?.len();
        let entry = &mut record.view.files[index];
        entry.received_bytes = ByteLength(length);
        if length == entry.bytes.0 {
            let (_, observed) = crate::files::hash_file(&path)?;
            if entry
                .digest
                .as_ref()
                .is_some_and(|expected| *expected != observed)
            {
                return Err(Error::Invalid(
                    "staged file does not match its declared digest".into(),
                ));
            }
            entry.digest = Some(observed);
        }
        if record
            .view
            .files
            .iter()
            .all(|file| file.received_bytes == file.bytes && file.digest.is_some())
        {
            record.view.phase = ImportPhase::Analyzing;
        }
        let revision = record.view.revision;
        self.save_import(record, revision)
    }

    fn stage_inbox_import(
        &self,
        id: ImportId,
        job: JobId,
        claim_token: &str,
        source_locator: &str,
    ) -> Result<ImportRecord> {
        let source = Path::new(source_locator);
        let source_metadata = fs::symlink_metadata(source).map_err(storage)?;
        if source_metadata.file_type().is_symlink() || !source_metadata.is_dir() {
            return Err(Error::Invalid("inbox package changed while staging".into()));
        }
        let root = fs::canonicalize(source).map_err(storage)?;
        let initial_inventory = Self::inventory_directory(&root)?;
        let mut record = self.import_record(id)?;
        if record.view.phase == ImportPhase::Analyzing
            && record
                .view
                .files
                .iter()
                .all(|file| file.received_bytes == file.bytes && file.digest.is_some())
        {
            return Ok(record);
        }
        if record.view.phase != ImportPhase::Staging
            || initial_inventory.len() != record.view.files.len()
            || initial_inventory
                .iter()
                .zip(&record.view.files)
                .any(|(left, right)| left.name != right.name || left.bytes != right.bytes)
        {
            return Err(Error::Invalid("inbox package changed while staging".into()));
        }
        for index in 0..record.view.files.len() {
            self.ensure_job_continues(job, Some(claim_token))?;
            let declared = record.view.files[index].clone();
            safe_name(&declared.name)?;
            let path = root.join(&declared.name);
            let metadata_before = fs::symlink_metadata(&path).map_err(storage)?;
            let canonical = fs::canonicalize(&path).map_err(storage)?;
            if metadata_before.file_type().is_symlink()
                || !metadata_before.is_file()
                || metadata_before.len() != declared.bytes.0
                || !canonical.starts_with(&root)
            {
                return Err(Error::Invalid("inbox package changed while staging".into()));
            }
            let mut file = File::open(&canonical).map_err(storage)?;
            let mut offset = declared.received_bytes.0;
            file.seek(SeekFrom::Start(offset)).map_err(storage)?;
            let mut buffer = vec![0u8; 1024 * 1024];
            while offset < declared.bytes.0 {
                let read = file.read(&mut buffer).map_err(storage)?;
                if read == 0 {
                    return Err(Error::Invalid("inbox package changed while staging".into()));
                }
                record = self.stage_import_chunk(id, index, offset, &buffer[..read])?;
                offset += read as u64;
                let completed = record
                    .view
                    .files
                    .iter()
                    .map(|entry| entry.received_bytes.0)
                    .sum();
                self.heartbeat_job_record(job, claim_token, "staging_inbox", completed, 30)?;
                self.ensure_job_continues(job, Some(claim_token))?;
            }
            let metadata_after = file.metadata().map_err(storage)?;
            let (_, source_digest) = crate::files::hash_file(&canonical)?;
            if metadata_after.len() != metadata_before.len()
                || metadata_after.modified().ok() != metadata_before.modified().ok()
                || record.view.files[index].digest.as_ref() != Some(&source_digest)
            {
                return Err(Error::Invalid("inbox package changed while staging".into()));
            }
        }
        let final_inventory = Self::inventory_directory(&root)?;
        if final_inventory.len() != initial_inventory.len()
            || final_inventory
                .iter()
                .zip(&initial_inventory)
                .any(|(left, right)| left.name != right.name || left.bytes != right.bytes)
        {
            return Err(Error::Invalid("inbox package changed while staging".into()));
        }
        Ok(record)
    }

    fn publish_import_for_job(
        &self,
        id: ImportId,
        job: JobId,
        claim_token: &str,
        before_publish: &mut dyn FnMut() -> Result<()>,
    ) -> Result<ImportRecord> {
        let journals = publication::PublicationJournalStore::new(&self.archive);
        let result = self.publish_import_inner(id, job, claim_token, before_publish);
        let transaction = TransactionId(job.0);
        if matches!(result, Err(Error::JobCancelled))
            && journals
                .phase(transaction)?
                .is_none_or(|phase| phase < publication::PublicationPhase::RevisionCommitted)
        {
            if journals.phase(transaction)?.is_some() {
                journals.abort(transaction)?;
            }
            let mut record = self.import_record(id)?;
            record.view.phase = ImportPhase::Cancelled;
            let revision = record.view.revision;
            self.save_import(record, revision)?;
        } else if result.is_err()
            && journals
                .phase(transaction)?
                .is_some_and(|phase| phase >= publication::PublicationPhase::RevisionCommitted)
        {
            self.reindex()?;
            finish_v3_publication(self, transaction)?;
            return self.publish_import_inner(id, job, claim_token, before_publish);
        }
        result
    }

    fn cancel_import(&self, id: ImportId) -> Result<ImportRecord> {
        let mut record = self.import_record(id)?;
        if matches!(
            record.view.phase,
            ImportPhase::Publishing | ImportPhase::Published
        ) {
            return Err(Error::CancellationTooLate);
        }
        if record.view.phase == ImportPhase::Cancelled {
            return Ok(record);
        }
        record.view.phase = ImportPhase::Cancelled;
        let revision = record.view.revision;
        self.save_import(record, revision)
    }

    fn accept_guided_imports(
        &self,
        records: Vec<ImportRecord>,
        intents: Vec<GuidedIngestIntent>,
    ) -> Result<Vec<collector_app::ImportView>> {
        if records.is_empty() || records.len() != intents.len() || records.len() > 64 {
            return Err(Error::Invalid(
                "guided acceptance requires 1..=64 paired packages".into(),
            ));
        }
        let mut db = lock(&self.db)?;
        let tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage)?;
        let mut views = Vec::with_capacity(records.len());
        for (record, intent) in records.iter().zip(&intents) {
            if intent.import_id != record.view.id
                || intent.user_id != record.user_id
                || intent.default_title.trim().is_empty()
                || intent.default_title.len() > 4096
                || intent.plan_token.len() != 64
            {
                return Err(Error::Invalid(
                    "guided intent disagrees with its Import".into(),
                ));
            }
            insert_import(&tx, record)?;
            tx.execute(
                "INSERT INTO private.guided_ingest_intents(import_id,user_id,plan_token,target_item_id,default_title,enrichment,enrichment_error,created_at,updated_at) VALUES(?,?,?,?,?,?,NULL,?,?)",
                params![intent.import_id.to_string(),intent.user_id.to_string(),intent.plan_token,intent.target_item_id.to_string(),intent.default_title,enrichment(intent.enrichment),intent.created_at.timestamp(),intent.updated_at.timestamp()],
            )
            .map_err(storage)?;
            views.push(record.view.clone());
        }
        self.record_command(&tx, &views)?;
        tx.commit().map_err(storage)?;
        Ok(views)
    }

    fn guided_ingest_intent(&self, id: ImportId) -> Result<Option<GuidedIngestIntent>> {
        lock(&self.db)?
            .query_row(
                "SELECT user_id,plan_token,target_item_id,default_title,enrichment,enrichment_error,created_at,updated_at,settled,cancelled FROM private.guided_ingest_intents WHERE import_id=?",
                [id.to_string()],
                |row| Ok((row.get::<_, String>(0)?,row.get::<_, String>(1)?,row.get::<_, String>(2)?,row.get::<_, String>(3)?,row.get::<_, String>(4)?,row.get::<_, Option<String>>(5)?,row.get::<_, i64>(6)?,row.get::<_, i64>(7)?,row.get::<_, bool>(8)?,row.get::<_, bool>(9)?)),
            )
            .optional()
            .map_err(storage)?
            .map(|row| {
                Ok(GuidedIngestIntent {
                    import_id: id,
                    user_id: row.0.parse().map_err(storage)?,
                    plan_token: row.1,
                    target_item_id: row.2.parse().map_err(storage)?,
                    default_title: row.3,
                    enrichment: parse_enrichment(&row.4)?,
                    enrichment_error: row.5,
                    settled: row.8,
                    cancelled: row.9,
                    created_at: chrono::DateTime::from_timestamp(row.6, 0).ok_or_else(|| Error::Storage("guided intent creation timestamp is invalid".into()))?,
                    updated_at: chrono::DateTime::from_timestamp(row.7, 0).ok_or_else(|| Error::Storage("guided intent update timestamp is invalid".into()))?,
                })
            })
            .transpose()
    }

    fn resumable_guided_ingest_ids(
        &self,
        after: Option<ImportId>,
        limit: u16,
    ) -> Result<Vec<ImportId>> {
        if !(1..=256).contains(&limit) {
            return Err(Error::Invalid(
                "guided recovery limit must be 1..=256".into(),
            ));
        }
        let db = lock(&self.db)?;
        let mut statement = db
            .prepare(
                "SELECT g.import_id
                 FROM private.guided_ingest_intents g
                 JOIN private.imports i ON i.id=g.import_id
                 WHERE i.phase IN ('analyzing','awaiting_review','published') AND g.settled=0 AND g.cancelled=0 AND (? IS NULL OR g.import_id>?)
                 ORDER BY g.import_id
                 LIMIT ?",
            )
            .map_err(storage)?;
        statement
            .query_map(
                params![
                    after.map(|id| id.to_string()),
                    after.map(|id| id.to_string()),
                    i64::from(limit)
                ],
                |row| row.get::<_, String>(0),
            )
            .map_err(storage)?
            .map(|value| value.map_err(storage)?.parse::<ImportId>().map_err(storage))
            .collect()
    }

    fn resumable_guided_inbox_records(
        &self,
        after: Option<ImportId>,
        limit: u16,
    ) -> Result<Vec<ImportRecord>> {
        if !(1..=256).contains(&limit) {
            return Err(Error::Invalid(
                "guided inbox recovery limit must be 1..=256".into(),
            ));
        }
        let db = lock(&self.db)?;
        let sql = format!(
            "{SELECT} WHERE phase='staging' AND source_kind='inbox'
             AND EXISTS(SELECT 1 FROM private.guided_ingest_intents g WHERE g.import_id=imports.id AND g.cancelled=0) AND (? IS NULL OR id>?)
             ORDER BY id LIMIT ?"
        );
        let mut statement = db.prepare(&sql).map_err(storage)?;
        statement
            .query_map(
                params![
                    after.map(|id| id.to_string()),
                    after.map(|id| id.to_string()),
                    i64::from(limit)
                ],
                read,
            )
            .map_err(storage)?
            .map(|row| parse(row.map_err(storage)?))
            .collect()
    }

    fn reconcile_guided_cleanup(&self) -> Result<()> {
        self.reconcile_import_cleanup()
    }

    fn guided_cleanup_status(&self, id: ImportId) -> Result<Option<IngestDiscardResult>> {
        lock(&self.db)?.query_row("SELECT reclaimed_bytes,pending_bytes,completed FROM private.import_cleanup WHERE import_id=?",[id.to_string()],|row| Ok((row.get::<_,u64>(0)?,row.get::<_,u64>(1)?,row.get::<_,bool>(2)?))).optional().map_err(storage).map(|value| value.map(|(reclaimed,pending,completed)| IngestDiscardResult { import_id:id,reclaimed_bytes:ByteLength(reclaimed),pending_bytes:ByteLength(pending),cleanup_pending:!completed }))
    }

    fn save_guided_ingest_intent(&self, intent: GuidedIngestIntent) -> Result<()> {
        let mut db = lock(&self.db)?;
        let tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage)?;
        let changed = tx.execute("UPDATE private.guided_ingest_intents SET enrichment_error=?,updated_at=?,settled=CASE WHEN cancelled=1 THEN 1 ELSE ? END,cancelled=MAX(cancelled,?) WHERE import_id=? AND user_id=? AND plan_token=? AND target_item_id=?", params![intent.enrichment_error,intent.updated_at.timestamp(),intent.settled,intent.cancelled,intent.import_id.to_string(),intent.user_id.to_string(),intent.plan_token,intent.target_item_id.to_string()]).map_err(storage)?;
        if changed != 1 {
            return Err(Error::Conflict);
        }
        self.record_command(&tx, &())?;
        tx.commit().map_err(storage)
    }

    fn guided_import_records(
        &self,
        user: UserId,
        collection: Option<CollectionId>,
        cursor: Option<ImportId>,
        limit: u16,
    ) -> Result<Page<ImportRecord>> {
        if !(1..=100).contains(&limit) {
            return Err(Error::Invalid("Import page limit must be 1..=100".into()));
        }
        let db = lock(&self.db)?;
        let mut sql = format!(
            "{SELECT} WHERE user_id=? AND EXISTS(SELECT 1 FROM private.guided_ingest_intents g WHERE g.import_id=imports.id)"
        );
        if collection.is_some() {
            sql.push_str(" AND collection_id=?");
        }
        if cursor.is_some() {
            sql.push_str(" AND id>?");
        }
        sql.push_str(" ORDER BY id LIMIT ?");
        let mut statement = db.prepare(&sql).map_err(storage)?;
        let mut values: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(user.to_string())];
        if let Some(collection) = collection {
            values.push(Box::new(collection.to_string()));
        }
        if let Some(cursor) = cursor {
            values.push(Box::new(cursor.to_string()));
        }
        values.push(Box::new(i64::from(limit) + 1));
        let refs = values
            .iter()
            .map(|value| value.as_ref())
            .collect::<Vec<_>>();
        let rows = statement
            .query_map(refs.as_slice(), read)
            .map_err(storage)?;
        let mut items = rows
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(storage)?
            .into_iter()
            .map(parse)
            .collect::<Result<Vec<_>>>()?;
        let next_cursor = if items.len() > usize::from(limit) {
            items.truncate(usize::from(limit));
            items.last().map(|record| record.view.id.to_string())
        } else {
            None
        };
        Ok(Page { items, next_cursor })
    }

    fn import_job_records(
        &self,
        id: ImportId,
        user: UserId,
    ) -> Result<Vec<collector_app::JobRecord>> {
        self.job_records_for_import(id, user)
    }

    fn discard_guided_import(
        &self,
        id: ImportId,
        expected_revision: u64,
    ) -> Result<IngestDiscardResult> {
        let _writer = lock(&self.writer)?;
        let record = self.import_record(id)?;
        if self.guided_cleanup_status(id)?.is_some() {
            return Err(Error::Conflict);
        }

        if record.view.revision != expected_revision {
            return Err(Error::Conflict);
        }
        if matches!(
            record.view.phase,
            ImportPhase::Publishing | ImportPhase::Published
        ) {
            return Err(Error::CancellationTooLate);
        }
        let pending = record
            .view
            .files
            .iter()
            .map(|file| file.received_bytes.0)
            .sum::<u64>();
        let now = self.clock.now().timestamp();
        {
            let mut db = lock(&self.db)?;
            let tx = db
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(storage)?;
            tx.execute("UPDATE private.jobs SET state='cancelled',phase='cancelled',updated_at=? WHERE user_id=? AND state='queued' AND json_extract(request,'$.import_id')=?",params![now,record.user_id.to_string(),id.to_string()]).map_err(storage)?;
            tx.execute("UPDATE private.jobs SET state='cancel_requested',cancellation_requested=1,updated_at=? WHERE user_id=? AND state='running' AND json_extract(request,'$.import_id')=?",params![now,record.user_id.to_string(),id.to_string()]).map_err(storage)?;
            let changed = tx.execute("UPDATE private.imports SET phase='cancelled',revision=revision+1,updated_at=? WHERE id=? AND revision=?",params![now,id.to_string(),expected_revision]).map_err(storage)?;
            if changed != 1 {
                return Err(Error::Conflict);
            }
            tx.execute("INSERT INTO private.import_cleanup(import_id,requested_at,reclaimed_bytes,pending_bytes,error) VALUES(?,?,0,?,NULL) ON CONFLICT(import_id) DO UPDATE SET requested_at=excluded.requested_at,pending_bytes=excluded.pending_bytes,error=NULL",params![id.to_string(),now,pending]).map_err(storage)?;
            let result = IngestDiscardResult {
                import_id: id,
                reclaimed_bytes: ByteLength(0),
                pending_bytes: ByteLength(pending),
                cleanup_pending: true,
            };
            self.record_command(&tx, &result)?;
            tx.commit().map_err(storage)?;
        }
        Ok(IngestDiscardResult {
            import_id: id,
            reclaimed_bytes: ByteLength(0),
            pending_bytes: ByteLength(pending),
            cleanup_pending: true,
        })
    }

    fn cancel_guided_import(&self, id: ImportId, expected_revision: u64) -> Result<ImportRecord> {
        let _writer = lock(&self.writer)?;
        let record = self.import_record(id)?;
        if record.view.revision != expected_revision {
            return Err(Error::Conflict);
        }
        let committed = matches!(record.view.phase, ImportPhase::Published)
            || self.capture(record.view.capture_id).is_ok();
        let now = self.clock.now().timestamp();
        let mut db = lock(&self.db)?;
        let tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage)?;
        tx.execute("UPDATE private.jobs SET state='cancelled',phase='cancelled',updated_at=? WHERE user_id=? AND state='queued' AND json_extract(request,'$.import_id')=?",params![now,record.user_id.to_string(),id.to_string()]).map_err(storage)?;
        tx.execute("UPDATE private.jobs SET state='cancel_requested',cancellation_requested=1,updated_at=? WHERE user_id=? AND state='running' AND json_extract(request,'$.import_id')=?",params![now,record.user_id.to_string(),id.to_string()]).map_err(storage)?;
        tx.execute(
            "UPDATE private.guided_ingest_intents SET cancelled=1,settled=1 WHERE import_id=?",
            [id.to_string()],
        )
        .map_err(storage)?;
        let next_phase = if committed { "published" } else { "cancelled" };
        let changed = tx.execute("UPDATE private.imports SET phase=?,revision=revision+1,updated_at=? WHERE id=? AND revision=?",params![next_phase,now,id.to_string(),expected_revision]).map_err(storage)?;
        if changed != 1 {
            return Err(Error::Conflict);
        }
        let mut result = record;
        result.view.phase = if committed {
            ImportPhase::Published
        } else {
            ImportPhase::Cancelled
        };
        result.view.revision += 1;
        result.view.updated_at = chrono::DateTime::from_timestamp(now, 0)
            .ok_or_else(|| Error::Storage("cancellation timestamp is invalid".into()))?;
        self.record_command(&tx, &result)?;
        tx.commit().map_err(storage)?;
        Ok(result)
    }
}

impl Store {
    pub(crate) fn reconcile_import_cleanup(&self) -> Result<()> {
        let ids = {
            let db = lock(&self.db)?;
            let mut statement = db
                .prepare("SELECT import_id FROM private.import_cleanup WHERE completed=0 ORDER BY import_id")
                .map_err(storage)?;
            statement
                .query_map([], |row| row.get::<_, String>(0))
                .map_err(storage)?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(storage)?
        };
        let mut first_error = None;
        for id in ids {
            let id = id.parse().map_err(storage)?;
            if let Err(error) = self.finish_import_cleanup(id) {
                first_error.get_or_insert(error);
            }
        }
        first_error.map_or(Ok(()), Err)
    }

    fn finish_import_cleanup(&self, id: ImportId) -> Result<IngestDiscardResult> {
        let _uploads = lock(&self.uploads)?;
        let record = self.import_record(id)?;
        if record.view.phase != ImportPhase::Cancelled
            || self.capture(record.view.capture_id).is_ok()
        {
            return Err(Error::CancellationTooLate);
        }
        let active: bool = lock(&self.db)?
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM private.jobs WHERE state IN ('running','cancel_requested') AND json_extract(request,'$.import_id')=?)",
                [id.to_string()],
                |row| row.get(0),
            )
            .map_err(storage)?;
        let pending: u64 = lock(&self.db)?
            .query_row(
                "SELECT pending_bytes FROM private.import_cleanup WHERE import_id=?",
                [id.to_string()],
                |row| row.get(0),
            )
            .map_err(storage)?;
        if active {
            let result = IngestDiscardResult {
                import_id: id,
                reclaimed_bytes: ByteLength(0),
                pending_bytes: ByteLength(pending),
                cleanup_pending: true,
            };
            return Ok(result);
        }
        let directory = self.state.join("imports").join(id.to_string());
        let result = if directory.exists() {
            fs::remove_dir_all(&directory).map_err(storage)
        } else {
            Ok(())
        };
        match result {
            Ok(()) => {
                let result = IngestDiscardResult {
                    import_id: id,
                    reclaimed_bytes: ByteLength(pending),
                    pending_bytes: ByteLength(0),
                    cleanup_pending: false,
                };
                let mut db = lock(&self.db)?;
                let tx = db.transaction().map_err(storage)?;
                tx.execute(
                        "UPDATE private.import_cleanup SET reclaimed_bytes=?,pending_bytes=0,error=NULL,completed=1 WHERE import_id=?",
                        params![pending, id.to_string()],
                    )
                    .map_err(storage)?;

                tx.commit().map_err(storage)?;
                Ok(result)
            }
            Err(error) => {
                lock(&self.db)?
                    .execute(
                        "UPDATE private.import_cleanup SET error=? WHERE import_id=?",
                        params![error.to_string(), id.to_string()],
                    )
                    .map_err(storage)?;
                Err(error)
            }
        }
    }

    fn publish_import_inner(
        &self,
        id: ImportId,
        job: JobId,
        claim_token: &str,
        before_publish: &mut dyn FnMut() -> Result<()>,
    ) -> Result<ImportRecord> {
        self.ensure_job_continues(job, Some(claim_token))?;
        let _writer = lock(&self.writer)?;
        let mut record = self.import_record(id)?;
        if record.view.phase == ImportPhase::Published {
            return Ok(record);
        }
        if !matches!(
            record.view.phase,
            ImportPhase::AwaitingReview | ImportPhase::Publishing
        ) || record.view.review.is_none()
        {
            return Err(Error::Conflict);
        }
        // Revision checks guard a new publication, not completion of an
        // irreversible archive commit recovered after process interruption.
        let committed_capture = match self.capture(record.view.capture_id) {
            Ok(capture) => Some(capture),
            Err(Error::NotFound) => None,
            Err(error) => return Err(error),
        };
        if let (Some(item_id), Some(expected)) = (
            record.view.association.item_id,
            record.view.expected_item_revision,
        ) && committed_capture.is_none()
            && self.item(item_id)?.revision != expected
        {
            if record.view.phase != ImportPhase::AwaitingReview {
                record.view.phase = ImportPhase::AwaitingReview;
                let revision = record.view.revision;
                self.save_import(record, revision)?;
            }
            return Err(Error::Conflict);
        }
        if record.view.phase == ImportPhase::AwaitingReview {
            record.view.phase = ImportPhase::Publishing;
            let revision = record.view.revision;
            record = self.save_import(record, revision)?;
        }
        let total = record
            .view
            .files
            .iter()
            .map(|file| file.bytes.0)
            .sum::<u64>();
        if committed_capture.is_none()
            && fs2::available_space(&self.archive).map_err(storage)?
                < total.saturating_add(64 * 1024 * 1024)
        {
            return Err(Error::Unavailable("insufficient archive space".into()));
        }
        let scratch = self.state.join("imports").join(id.to_string());
        fs::create_dir_all(&scratch).map_err(storage)?;
        let mut files = Vec::new();
        let mut sources = Vec::new();
        for (index, declared) in record.view.files.iter().enumerate() {
            self.ensure_job_continues(job, Some(claim_token))?;
            let source = scratch.join(format!("{index}.part"));
            if declared.bytes.0 == 0 && !source.exists() {
                let file = File::create(&source).map_err(storage)?;
                file.sync_all().map_err(storage)?;
            }
            let (bytes, digest) = crate::files::hash_file(&source)?;
            if bytes != declared.bytes || declared.digest.as_ref() != Some(&digest) {
                return Err(Error::Invalid(
                    "staged bytes do not match the durable Import inventory".into(),
                ));
            }
            files.push(StoredFile {
                name: declared.name.clone(),
                bytes,
                digest,
            });
            sources.push(source);
        }
        let provisional = self.provisional_observations(id)?;
        for observation in &provisional {
            let inventory =
                self.package_inventory(record.view.collection_id, PackageScope::Import(id))?;
            observation
                .source_fingerprint
                .validate_inventory(&inventory)?;
            if observation
                .source_fingerprint
                .files
                .iter()
                .zip(&files)
                .any(|(observed, staged)| observed.sha256 != staged.digest)
            {
                return Err(Error::Conflict);
            }
        }
        let capture = Capture {
            id: record.view.capture_id,
            collection_id: record.view.collection_id,
            revision: Revision(1),
            association: record.view.association,
            created_at: record.view.created_at,
            representations: vec![Representation {
                id: record.view.representation_id,
                role: RepresentationRole::PreservationMaster,
                format: "original-package".into(),
                files: files.clone(),
                source: None,
                profile: None,
            }],
        };
        capture.validate()?;
        {
            let db = lock(&self.db)?;
            let mut graph = graph_from_projection(&db, record.view.collection_id)?;
            graph.captures.retain(|value| value.id != capture.id);
            graph.captures.push(capture.clone());
            validate_projected_graph(&db, &graph, record.view.collection_id)?;
        }
        let collection = {
            let db = lock(&self.db)?;
            collection_path(&db, &self.archive, record.view.collection_id)?
        };
        match self.capture(capture.id) {
            Ok(existing)
                if serde_json::to_value(&existing).map_err(storage)?
                    == serde_json::to_value(&capture).map_err(storage)? =>
            {
                self.heartbeat_job_record(job, claim_token, "revision_committed", total, 30)?;
            }
            Ok(_) => {
                return Err(Error::Storage(
                    "preallocated Capture identity has different published content".into(),
                ));
            }
            Err(Error::NotFound) => {
                self.publish_capture_payloads(
                    job,
                    &collection,
                    capture.clone(),
                    &sources,
                    Some(claim_token),
                    before_publish,
                )?;
            }
            Err(error) => return Err(error),
        }

        // Provisional analysis is not portable evidence until its Import source
        // has been rebound to the committed Representation inventory. The two
        // fingerprints intentionally retain distinct scoped file identities.
        if !provisional.is_empty() {
            let import_inventory =
                self.package_inventory(record.view.collection_id, PackageScope::Import(id))?;
            let representation_inventory = self.package_inventory(
                record.view.collection_id,
                PackageScope::Representation(record.view.representation_id),
            )?;
            if import_inventory.files.len() != representation_inventory.files.len() {
                return Err(Error::Storage(
                    "published package inventory changed shape".into(),
                ));
            }
            for observation in provisional {
                observation
                    .source_fingerprint
                    .validate_inventory(&import_inventory)?;
                let files = observation
                    .source_fingerprint
                    .files
                    .iter()
                    .zip(&representation_inventory.files)
                    .zip(&files)
                    .map(|((source, target), published)| {
                        if source.sha256 != published.digest || source.bytes != target.bytes {
                            return Err(Error::Storage("published package length changed".into()));
                        }
                        Ok(PackageFileFingerprint {
                            id: target.id,
                            bytes: target.bytes,
                            sha256: source.sha256.clone(),
                        })
                    })
                    .collect::<Result<Vec<_>>>()?;
                let committed_fingerprint = SourceFingerprint::calculate(
                    PackageScope::Representation(record.view.representation_id),
                    files,
                    observation.source_fingerprint.layout_profile_digest.clone(),
                    observation.source_fingerprint.parameters.clone(),
                )?;
                let mappings = observation
                    .source_fingerprint
                    .files
                    .iter()
                    .zip(&committed_fingerprint.files)
                    .map(|(source, target)| SourceFileRebinding {
                        import_file_id: source.id,
                        representation_file_id: target.id,
                        bytes: source.bytes,
                        sha256: source.sha256.clone(),
                    })
                    .collect();
                let rebinding = ImportSourceRebinding {
                    provisional_fingerprint: observation.source_fingerprint.clone(),
                    committed_fingerprint: committed_fingerprint.clone(),
                    files: mappings,
                };
                rebinding.validate()?;
                let mut committed = observation;
                committed.source_fingerprint = committed_fingerprint;
                self.publish_processing_record_under_writer(
                    JobId(committed.id.0),
                    PortableProcessingRecord {
                        observation: committed,
                        import_rebinding: Some(rebinding),
                    },
                    &mut || Ok(false),
                )?;
            }
        }

        let evidence = archive_v3::EvidenceRecordV3 {
            id: EvidenceId(id.0),
            collection_id: record.view.collection_id,
            item_id: record.view.association.item_id,
            representation_ids: vec![record.view.representation_id],
            kind: EvidenceKind::Integrity,
            outcome: EvidenceOutcome::Passed,
            inputs: files.iter().map(|file| file.digest.clone()).collect(),
            implementation: "collector-store staged copy and destination reread".into(),
            implementation_version: env!("CARGO_PKG_VERSION").into(),
            catalog: None,
            details: serde_json::json!({
                "claim":"destination bytes equal staged source bytes; no catalog verification asserted"
            }),
            created_at: record.view.created_at,
        };
        evidence.validate()?;
        let evidence_exists: Option<String> = lock(&self.db)?
            .query_row(
                "SELECT record FROM evidence WHERE id=?",
                [evidence.id.to_string()],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage)?;
        if let Some(existing) = evidence_exists {
            if existing != serde_json::to_string(&evidence).map_err(storage)? {
                return Err(Error::Storage(
                    "preallocated Evidence identity has different published content".into(),
                ));
            }
        } else {
            let publication = publish_v3_revision(
                self,
                V3RevisionTarget {
                    collection: &collection,
                    directory_kind: "evidence",
                    entity_id: evidence.id.to_string(),
                    collection_id: evidence.collection_id,
                    expected: Revision(0),
                    revision: Revision(1),
                },
                evidence.clone(),
                vec![],
            )?;
            let mut db = lock(&self.db)?;
            let tx = db.transaction().map_err(storage)?;
            project_evidence_record(&tx, &evidence)?;
            tx.commit().map_err(storage)?;
            finish_v3_publication(self, publication)?;
        }
        if let Some(item_id) = record.view.association.item_id {
            let item_path = self.item_path(item_id).map_err(|error| {
                Error::Storage(format!("Import Evidence Item path lookup failed: {error}"))
            })?;
            let projected_revisions = item_path.join("revisions");
            let revisions_path = if projected_revisions.exists() {
                projected_revisions
            } else {
                item_path
                    .parent()
                    .ok_or_else(|| Error::Storage("Item archive path has no parent".into()))?
                    .join(item_id.to_string())
                    .join("revisions")
            };
            let mut revisions = directories(&revisions_path).map_err(|error| {
                Error::Storage(format!(
                    "Import Evidence Item history lookup failed at {}: {error}",
                    revisions_path.display()
                ))
            })?;
            revisions.sort();
            let already_referenced = revisions
                .last()
                .filter(|revision| revision.join("manifest.v3.toml").exists())
                .map(|revision| {
                    migration_v3::read_item(revision, record.view.collection_id).map(
                        |(_, references)| references.iter().any(|value| value.id == evidence.id),
                    )
                })
                .transpose()?
                .unwrap_or(false);
            if !already_referenced {
                let item = self.item(item_id)?;
                let revision = item.revision;
                self.publish_under_writer(item, revision, vec![])
                    .map_err(|error| {
                        Error::Storage(format!(
                            "Import Evidence Item reference publication failed: {error}"
                        ))
                    })?;
            }
        }
        record = self.import_record(id)?;
        record.view.phase = ImportPhase::Published;
        let revision = record.view.revision;
        self.save_import(record, revision)
    }
}
