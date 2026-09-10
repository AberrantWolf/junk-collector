use super::*;
use crate::files::safe_name;
use collector_app::{
    ImportIdentityDecision, ImportPhase, ImportRecord, ImportSourceKind, ProvisionalImportPort,
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

impl ProvisionalImportPort for Store {
    fn create_import(&self, record: ImportRecord) -> Result<ImportRecord> {
        validate(&record)?;
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
        let mut db = lock(&self.db)?;
        let tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage)?;
        let changed = tx
            .execute(
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
        if changed != 1 {
            return Err(Error::Storage(
                "Import insert did not change one row".into(),
            ));
        }
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
        if record.view.phase != ImportPhase::Staging {
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
}
