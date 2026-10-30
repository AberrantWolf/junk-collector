use super::*;
use sha2::{Digest as _, Sha256};
use std::io::{Read, Seek, SeekFrom};

const CHUNK_LIMIT: usize = 8 * 1024 * 1024;

enum ExistingAsset {
    Reject,
    Reuse { restore_archived: Option<Revision> },
}

pub(crate) fn safe_name(name: &str) -> Result<()> {
    if name.is_empty()
        || name.len() > 1024
        || name.contains('\\')
        || name.contains(':')
        || name.chars().any(char::is_control)
        || name.starts_with('/')
        || name
            .split('/')
            .any(|p| p.is_empty() || p == "." || p == "..")
    {
        return Err(Error::Invalid(
            "package names must be safe relative paths".into(),
        ));
    }
    Ok(())
}
pub(crate) fn hash_file(path: &Path) -> Result<(ByteLength, Digest)> {
    hash_file_checked(path, u64::MAX, &mut || Ok(()))
}
pub(crate) fn hash_file_checked(
    path: &Path,
    maximum: u64,
    checkpoint: &mut dyn FnMut() -> Result<()>,
) -> Result<(ByteLength, Digest)> {
    let mut file = File::open(path).map_err(storage)?;
    let mut hash = Sha256::new();
    let mut bytes = 0u64;
    let mut buffer = vec![0u8; 1024 * 1024];
    loop {
        checkpoint()?;
        let read = file.read(&mut buffer).map_err(storage)?;
        if bytes.saturating_add(read as u64) > maximum {
            return Err(Error::Invalid(
                "staged file exceeds its declared length".into(),
            ));
        }
        if read == 0 {
            break;
        }
        hash.update(&buffer[..read]);
        bytes += read as u64;
    }
    Ok((
        ByteLength(bytes),
        Digest {
            algorithm: HashAlgorithm::Sha256,
            hex: hex::encode(hash.finalize()),
        },
    ))
}

fn write_new_or_verify(path: &Path, bytes: &[u8]) -> Result<()> {
    if path.try_exists().map_err(storage)? {
        if fs::read(path).map_err(storage)? == bytes {
            return Ok(());
        }
        return Err(Error::Storage(
            "existing Asset payload differs from the retried command".into(),
        ));
    }
    write_new(path, bytes)
}

impl Store {
    fn upload_directory(&self, id: JobId) -> PathBuf {
        self.state.join("uploads").join(id.to_string())
    }
    pub(crate) fn recover_private_publications(&self) -> Result<()> {
        // Reconcile private bookkeeping after the portable archive has been rebuilt.
        for entry in fs::read_dir(self.state.join("uploads")).map_err(storage)? {
            let path = entry.map_err(storage)?.path();
            let Some(id) = path
                .file_name()
                .and_then(|name| name.to_str())
                .and_then(|name| name.parse::<JobId>().ok())
            else {
                continue;
            };
            if !path.join("upload.json").try_exists().map_err(storage)? {
                continue;
            }
            let mut upload = self.upload_record(id)?;
            if self
                .evidence(upload.item_id)?
                .iter()
                .any(|evidence| evidence.id == EvidenceId(id.0))
            {
                upload.state = JobState::Completed;
            }
            self.persist_upload(&upload)?;
        }
        let staging = self.state.join("asset-publication");
        if staging.try_exists().map_err(storage)? {
            for entry in fs::read_dir(&staging).map_err(storage)? {
                let path = entry.map_err(storage)?.path();
                let Some(id) = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .and_then(|name| name.parse::<AssetId>().ok())
                else {
                    continue;
                };
                let record: Option<String> = lock(&self.db)?
                    .query_row(
                        "SELECT record FROM assets WHERE id=?",
                        [id.to_string()],
                        |row| row.get(0),
                    )
                    .optional()
                    .map_err(storage)?;
                if let Some(record) = record {
                    let record: archive_v3::AssetRecordV3 =
                        serde_json::from_str(&record).map_err(storage)?;
                    for payload in &record.legacy_payloads {
                        if hash_file(&self.archive.join(&payload.relative_path))?.1.hex
                            != payload.sha256
                        {
                            return Err(Error::Storage(
                                "published asset differs from recovery record".into(),
                            ));
                        }
                    }
                    fs::remove_dir_all(&path).map_err(storage)?;
                    sync_directory(&staging)?;
                }
            }
        }
        Ok(())
    }
    fn persist_upload(&self, upload: &Upload) -> Result<()> {
        let dir = self.upload_directory(upload.id);
        let temp = dir.join(format!("{}.tmp", TransactionId::new()));
        let bytes = serde_json::to_vec_pretty(upload).map_err(storage)?;
        if let Some(result) = &self.command_result {
            self.prepare_command(
                "state",
                &relative(&self.state, &dir.join("upload.json"))?,
                &Digest::sha256(&bytes).hex,
                result,
            )?;
        }
        write_new(&temp, &bytes)?;
        fs::rename(temp, dir.join("upload.json")).map_err(storage)?;
        sync_directory(&dir)?;
        let job = Job {
            id: upload.id,
            user_id: upload.user_id,
            item_id: upload.item_id,
            kind: "preservation_import".into(),
            state: upload.state.clone(),
            completed_bytes: upload.offsets.iter().sum(),
            total_bytes: upload.files.iter().map(|f| f.bytes.0).sum(),
            error: None,
        };
        let (durable_state, durable_phase) = match upload.state {
            JobState::Queued => ("running", "awaiting_source"),
            JobState::Running => ("running", "staging"),
            JobState::CancelRequested => ("cancel_requested", "cancelling"),
            JobState::Completed => ("completed", "completed"),
            JobState::Failed => ("failed", "failed"),
            JobState::Cancelled => ("cancelled", "cancelled"),
        };
        let collection_id = self.item(upload.item_id)?.collection_id;
        let mut db = lock(&self.db)?;
        let tx = db.transaction().map_err(storage)?;
        let now = Utc::now().timestamp();
        tx.execute(
            "INSERT INTO private.item_job_generations(job_id,generation)
             SELECT ?1,COALESCE(json_extract(record,'$.lifecycle.generation'),0)
             FROM items WHERE id=?2 AND NOT EXISTS(SELECT 1 FROM private.jobs WHERE id=?1)
             ON CONFLICT(job_id) DO NOTHING",
            params![job.id.to_string(), job.item_id.to_string()],
        )
        .map_err(storage)?;
        tx.execute(
            "INSERT INTO private.jobs(
              id,user_id,collection_id,item_id,kind,state,phase,
              completed_units,total_units,attempt,created_at,updated_at,
              error,limitation,request,result,worker_instance,lease_expires,
              claim_token,cancellation_requested
            ) VALUES(?,?,?,?,?,?,?,?,?,1,?,?,?,NULL,?,NULL,NULL,NULL,NULL,0)
            ON CONFLICT(id) DO UPDATE SET
              state=CASE WHEN jobs.claim_token IS NULL AND jobs.phase IN ('awaiting_source','staging') THEN excluded.state ELSE jobs.state END,
              phase=CASE WHEN jobs.claim_token IS NULL AND jobs.phase IN ('awaiting_source','staging') THEN excluded.phase ELSE jobs.phase END,
              completed_units=CASE WHEN jobs.claim_token IS NULL AND jobs.phase IN ('awaiting_source','staging') THEN excluded.completed_units ELSE jobs.completed_units END,
              total_units=excluded.total_units,updated_at=excluded.updated_at,
              error=CASE WHEN jobs.claim_token IS NULL AND jobs.phase IN ('awaiting_source','staging') THEN excluded.error ELSE jobs.error END,
              request=CASE WHEN jobs.claim_token IS NULL AND jobs.phase IN ('awaiting_source','staging') THEN excluded.request ELSE jobs.request END",
            params![
                job.id.to_string(),
                job.user_id.to_string(),
                collection_id.to_string(),
                job.item_id.to_string(),
                job.kind,
                durable_state,
                durable_phase,
                job.completed_bytes,
                job.total_bytes,
                now,
                now,
                job.error,
                serde_json::to_string(upload).map_err(storage)?,
            ],
        )
        .map_err(storage)?;
        if let Some(result) = &self.command_result {
            self.record_command(&tx, result)?;
        }
        tx.commit().map_err(storage)?;
        Ok(())
    }
    pub(crate) fn create_upload_record(
        &self,
        user: UserId,
        item_id: ItemId,
        expected_revision: Revision,
        files: Vec<UploadFile>,
    ) -> Result<Upload> {
        let item = self.item(item_id)?;
        if item.revision != expected_revision {
            return Err(Error::Conflict);
        }
        if files.is_empty() || files.len() > 4096 {
            return Err(Error::Invalid("package requires 1–4096 files".into()));
        }
        let mut names = std::collections::HashSet::new();
        let mut total = 0u64;
        for file in &files {
            safe_name(&file.name)?;
            if !names.insert(file.name.to_lowercase()) {
                return Err(Error::Invalid(
                    "duplicate or case-colliding package name".into(),
                ));
            }
            if let Some(hash) = &file.sha256 {
                Digest {
                    algorithm: HashAlgorithm::Sha256,
                    hex: hash.clone(),
                }
                .validate()?;
            }
            total = total
                .checked_add(file.bytes.0)
                .ok_or_else(|| Error::Invalid("package size overflow".into()))?;
        }
        if total > 16 * 1024u64.pow(4) {
            return Err(Error::Invalid("package exceeds 16 TiB limit".into()));
        }
        let available = fs2::available_space(&self.state).map_err(storage)?;
        if total.saturating_add(64 * 1024 * 1024) > available {
            return Err(Error::Unavailable(
                "insufficient upload scratch space".into(),
            ));
        }
        let _guard = lock(&self.uploads)?;
        let upload = Upload {
            id: JobId::new(),
            user_id: user,
            item_id,
            expected_revision,
            representation_id: RepresentationId::new(),
            capture_id: CaptureId::new(),
            offsets: vec![0; files.len()],
            files,
            state: JobState::Queued,
        };
        fs::create_dir(self.upload_directory(upload.id)).map_err(storage)?;
        sync_directory(&self.state.join("uploads"))?;
        self.with_command_result(&upload)?.persist_upload(&upload)?;
        Ok(upload)
    }
    pub(crate) fn upload_record(&self, id: JobId) -> Result<Upload> {
        let path = self.upload_directory(id).join("upload.json");
        let bytes = fs::read(path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                Error::NotFound
            } else {
                storage(e)
            }
        })?;
        let mut upload: Upload = serde_json::from_slice(&bytes).map_err(storage)?;
        let durable_state: Option<String> = lock(&self.db)?
            .query_row(
                "SELECT state FROM private.jobs WHERE id=?",
                [id.to_string()],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage)?;
        match durable_state.as_deref() {
            Some("queued") => upload.state = JobState::Running,
            Some("completed") => upload.state = JobState::Completed,
            Some("failed") => upload.state = JobState::Failed,
            Some("cancelled") => upload.state = JobState::Cancelled,
            _ => {}
        }
        // File length is the durable upload cursor after a crash between fsync and metadata update.
        if matches!(upload.state, JobState::Queued | JobState::Running) {
            for (i, offset) in upload.offsets.iter_mut().enumerate() {
                *offset = fs::metadata(self.upload_directory(id).join(format!("{i}.part")))
                    .map(|m| m.len())
                    .unwrap_or(0);
            }
        }
        Ok(upload)
    }
    pub(crate) fn upload_chunk_record(
        &self,
        id: JobId,
        index: usize,
        offset: u64,
        bytes: &[u8],
    ) -> Result<Upload> {
        if bytes.len() > CHUNK_LIMIT {
            return Err(Error::Invalid("chunk exceeds 8 MiB".into()));
        }
        let _guard = lock(&self.uploads)?;
        let mut upload = self.upload_record(id)?;
        if !matches!(upload.state, JobState::Queued | JobState::Running) {
            return Err(Error::Conflict);
        }
        let expected = upload.files.get(index).ok_or(Error::NotFound)?;
        let end = offset
            .checked_add(bytes.len() as u64)
            .ok_or_else(|| Error::Invalid("offset overflow".into()))?;
        if end > expected.bytes.0 {
            return Err(Error::Invalid("chunk exceeds declared file length".into()));
        }
        let path = self.upload_directory(id).join(format!("{index}.part"));
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(path)
            .map_err(storage)?;
        let current = file.metadata().map_err(storage)?.len();
        if offset < current {
            if end > current {
                return Err(Error::Conflict);
            }
            let mut previous = vec![0; bytes.len()];
            file.seek(SeekFrom::Start(offset)).map_err(storage)?;
            file.read_exact(&mut previous).map_err(storage)?;
            if previous != bytes {
                return Err(Error::Conflict);
            }
        } else {
            if offset != current {
                return Err(Error::Conflict);
            }
            file.seek(SeekFrom::End(0)).map_err(storage)?;
            file.write_all(bytes).map_err(storage)?;
            file.sync_all().map_err(storage)?;
        }
        upload.offsets[index] = file.metadata().map_err(storage)?.len();
        upload.state = JobState::Running;
        self.persist_upload(&upload)?;
        Ok(upload)
    }
    pub(crate) fn cancel_upload_record(&self, id: JobId) -> Result<()> {
        let upload = self.upload_record(id)?;
        self.request_job_cancellation_record(id, upload.user_id)
    }
    pub(crate) fn queue_upload_record(&self, id: JobId, retry: bool) -> Result<JobId> {
        let _guard = lock(&self.uploads)?;
        let upload = self.upload_record(id)?;
        if upload
            .files
            .iter()
            .zip(&upload.offsets)
            .any(|(file, offset)| file.bytes.0 != *offset)
        {
            return Err(Error::Invalid(
                "all declared upload bytes must be staged before publication".into(),
            ));
        }
        let mut db = lock(&self.db)?;
        let tx = db
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(storage)?;
        let obsolete: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM private.jobs j JOIN items i ON i.id=j.item_id
             WHERE j.id=? AND (i.deleted=1 OR COALESCE(json_extract(i.record,'$.lifecycle.generation'),0)
             <>COALESCE((SELECT generation FROM private.item_job_generations g WHERE g.job_id=j.id),0)))",
            [id.to_string()], |row| row.get(0),
        ).map_err(storage)?;
        if obsolete {
            return Err(Error::Conflict);
        }
        let (state, phase, claim_token, prior_result, error, attempt, failed_at): (
            String,
            String,
            Option<String>,
            Option<String>,
            Option<String>,
            u32,
            i64,
        ) = tx
            .query_row(
                "SELECT state,phase,claim_token,result,error,attempt,updated_at
                 FROM private.jobs WHERE id=?",
                [id.to_string()],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                    ))
                },
            )
            .map_err(storage)?;
        let eligible = if retry {
            state == "failed"
        } else {
            state == "running"
                && claim_token.is_none()
                && matches!(phase.as_str(), "awaiting_source" | "staging")
        };
        if !eligible {
            return Err(Error::Conflict);
        }
        let result = if retry {
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
            Some(serde_json::to_string(&result).map_err(storage)?)
        } else {
            None
        };
        tx.execute(
            "UPDATE private.jobs SET state='queued',phase='queued',completed_units=0,
               attempt=attempt+?,error=NULL,result=?,cancellation_requested=0,updated_at=?
             WHERE id=?",
            params![
                u8::from(retry),
                result,
                self.clock.now().timestamp(),
                id.to_string()
            ],
        )
        .map_err(storage)?;
        self.record_command(&tx, &id)?;
        tx.commit().map_err(storage)?;
        Ok(id)
    }
    pub(crate) fn queue_inbox_upload_record(
        &self,
        id: JobId,
        source_locator: &str,
    ) -> Result<JobId> {
        if source_locator.is_empty() || source_locator.len() > 4096 {
            return Err(Error::Invalid("inbox source locator is invalid".into()));
        }
        let _guard = lock(&self.uploads)?;
        let upload = self.upload_record(id)?;
        let request = serde_json::to_string(&serde_json::json!({
            "upload": upload,
            "inbox_source_locator": source_locator,
        }))
        .map_err(storage)?;
        let mut db = lock(&self.db)?;
        let tx = db
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(storage)?;
        let changed = tx
            .execute(
                "UPDATE private.jobs SET state='queued',phase='queued',completed_units=0,
                   request=?,error=NULL,result=NULL,cancellation_requested=0,updated_at=?
                 WHERE id=? AND state='running' AND claim_token IS NULL
                   AND phase IN ('awaiting_source','staging')",
                params![request, self.clock.now().timestamp(), id.to_string()],
            )
            .map_err(storage)?;
        if changed != 1 {
            return Err(Error::Conflict);
        }
        self.record_command(&tx, &id)?;
        tx.commit().map_err(storage)?;
        Ok(id)
    }
    pub(crate) fn commit_upload_record(&self, id: JobId) -> Result<Item> {
        self.commit_upload_inner(id, None, None, &mut || Ok(()))
    }
    pub(crate) fn commit_upload_job_record(
        &self,
        id: JobId,
        claim_token: &str,
        before_publish: &mut dyn FnMut() -> Result<()>,
    ) -> Result<Item> {
        let (created_at, request): (i64, String) = lock(&self.db)?
            .query_row(
                "SELECT created_at,request FROM private.jobs WHERE id=? AND claim_token=?",
                params![id.to_string(), claim_token],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(storage)?;
        let created_at = chrono::DateTime::from_timestamp(created_at, 0)
            .ok_or_else(|| Error::Storage("job creation timestamp is invalid".into()))?;
        let request: serde_json::Value = serde_json::from_str(&request).map_err(storage)?;
        if let Some(source_locator) = request
            .get("inbox_source_locator")
            .and_then(serde_json::Value::as_str)
        {
            self.stage_inbox_upload_record(id, claim_token, Path::new(source_locator))?;
        }
        let journals = publication::PublicationJournalStore::new(&self.archive);
        let result =
            self.commit_upload_inner(id, Some(claim_token), Some(created_at), before_publish);
        if matches!(result, Err(Error::JobCancelled))
            && journals
                .phase(TransactionId(id.0))?
                .is_some_and(|phase| phase < publication::PublicationPhase::RevisionCommitted)
        {
            journals.abort(TransactionId(id.0))?;
        } else if result.is_err()
            && journals
                .phase(TransactionId(id.0))?
                .is_some_and(|phase| phase >= publication::PublicationPhase::RevisionCommitted)
        {
            self.reindex()?;
            finish_v3_publication(self, TransactionId(id.0))?;
            return self.commit_upload_inner(
                id,
                Some(claim_token),
                Some(created_at),
                before_publish,
            );
        }
        result
    }

    fn stage_inbox_upload_record(&self, id: JobId, claim_token: &str, source: &Path) -> Result<()> {
        if fs::symlink_metadata(source)
            .map_err(storage)?
            .file_type()
            .is_symlink()
        {
            return Err(Error::Invalid("inbox package changed while staging".into()));
        }
        let root = fs::canonicalize(source).map_err(storage)?;
        let upload = self.upload_record(id)?;
        let mut completed = upload.offsets.iter().sum::<u64>();
        for (index, entry) in upload.files.iter().enumerate() {
            self.ensure_job_continues(id, Some(claim_token))?;
            safe_name(&entry.name)?;
            let path = root.join(&entry.name);
            let metadata = fs::symlink_metadata(&path).map_err(storage)?;
            let canonical = fs::canonicalize(&path).map_err(storage)?;
            if metadata.file_type().is_symlink()
                || !metadata.is_file()
                || metadata.len() != entry.bytes.0
                || !canonical.starts_with(&root)
            {
                return Err(Error::Invalid("inbox package changed while staging".into()));
            }
            let mut file = File::open(&canonical).map_err(storage)?;
            let mut offset = upload.offsets[index];
            file.seek(SeekFrom::Start(offset)).map_err(storage)?;
            let mut buffer = vec![0u8; 1024 * 1024];
            loop {
                let read = file.read(&mut buffer).map_err(storage)?;
                if read == 0 {
                    break;
                }
                self.upload_chunk_record(id, index, offset, &buffer[..read])?;
                offset += read as u64;
                completed += read as u64;
                self.heartbeat_job_record(id, claim_token, "staging_inbox", completed, 30)?;
                self.ensure_job_continues(id, Some(claim_token))?;
            }
            if file.metadata().map_err(storage)?.len() != entry.bytes.0 {
                return Err(Error::Invalid("inbox package changed while staging".into()));
            }
        }
        Ok(())
    }
    fn commit_upload_inner(
        &self,
        id: JobId,
        claim_token: Option<&str>,
        durable_created_at: Option<chrono::DateTime<Utc>>,
        before_publish: &mut dyn FnMut() -> Result<()>,
    ) -> Result<Item> {
        let _guard = lock(&self.uploads)?;
        let _writer = lock(&self.writer)?;
        self.ensure_job_continues(id, claim_token)?;
        let mut upload = self.upload_record(id)?;
        let mut item = self.item(upload.item_id)?;
        let existing_capture = item
            .captures
            .iter()
            .find(|capture| {
                capture
                    .representations
                    .iter()
                    .any(|representation| representation.id == upload.representation_id)
            })
            .cloned();
        let evidence_id = EvidenceId(upload.id.0);
        if existing_capture.is_some()
            && self
                .evidence(item.id)?
                .iter()
                .any(|evidence| evidence.id == evidence_id)
        {
            upload.state = JobState::Completed;
            self.persist_upload(&upload)?;
            return Ok(item);
        }
        if upload.state == JobState::Cancelled {
            return Err(Error::Conflict);
        }
        if item.revision != upload.expected_revision && existing_capture.is_none() {
            return Err(Error::Conflict);
        }
        let total = upload.files.iter().map(|f| f.bytes.0).sum::<u64>();
        if fs2::available_space(&self.archive).map_err(storage)?
            < total.saturating_add(64 * 1024 * 1024)
        {
            return Err(Error::Unavailable("insufficient archive space".into()));
        }
        let mut files = Vec::new();
        for (index, file) in upload.files.iter().enumerate() {
            self.ensure_job_continues(id, claim_token)?;
            let source = self.upload_directory(id).join(format!("{index}.part"));
            if file.bytes.0 == 0 && !source.try_exists().map_err(storage)? {
                write_new(&source, b"")?;
            }
            let (length, digest) = hash_file(&source)?;
            if length != file.bytes
                || file
                    .sha256
                    .as_ref()
                    .is_some_and(|h| !h.eq_ignore_ascii_case(&digest.hex))
            {
                return Err(Error::Invalid(
                    "uploaded bytes do not match package manifest".into(),
                ));
            }
            files.push(StoredFile {
                visibility: file.visibility.unwrap_or(Visibility::Shared),
                name: file.name.clone(),
                bytes: length,
                digest,
            });
        }
        let item_root = self.item_path(item.id)?;
        let collection_root = item_root
            .parent()
            .and_then(std::path::Path::parent)
            .ok_or_else(|| Error::Storage("item path has no collection parent".into()))?;
        let inputs = files.iter().map(|f| f.digest.clone()).collect();
        let capture = match existing_capture {
            Some(capture) => capture,
            None => {
                let capture = Capture {
                    purchase: None,
                    id: upload.capture_id,
                    collection_id: item.collection_id,
                    revision: Revision(1),
                    association: CaptureAssociation {
                        item_id: Some(item.id),
                        ..Default::default()
                    },
                    created_at: durable_created_at.unwrap_or_else(Utc::now),
                    representations: vec![Representation {
                        id: upload.representation_id,
                        role: RepresentationRole::PreservationMaster,
                        format: "original-package".into(),
                        files,
                        source: None,
                        profile: None,
                    }],
                };
                let sources = upload
                    .files
                    .iter()
                    .enumerate()
                    .map(|(index, _)| {
                        self.upload_directory(upload.id)
                            .join(format!("{index}.part"))
                    })
                    .collect::<Vec<_>>();
                self.publish_capture_payloads(
                    upload.id,
                    collection_root,
                    capture,
                    &sources,
                    claim_token,
                    before_publish,
                )?
            }
        };
        self.ensure_job_continues(id, claim_token)?;
        if !item.captures.iter().any(|value| value.id == capture.id) {
            item.captures.push(capture.clone());
        }
        let evidence = Evidence {
            id: evidence_id,
            item_id: item.id,
            representation_id: upload.representation_id,
            kind: EvidenceKind::Integrity,
            outcome: EvidenceOutcome::Passed,
            inputs,
            implementation: "collector-store staged copy and destination reread".into(),
            implementation_version: env!("CARGO_PKG_VERSION").into(),
            catalog: None,
            details: serde_json::json!({"claim":"destination bytes equal uploaded bytes; no catalog verification asserted"}),
            created_at: capture.created_at,
        };
        let result = self.publish_under_writer(item.clone(), item.revision, vec![evidence])?;
        upload.state = JobState::Completed;
        self.persist_upload(&upload)?;
        Ok(result)
    }

    pub(crate) fn publish_capture_payloads(
        &self,
        job_id: JobId,
        collection_root: &Path,
        capture: Capture,
        sources: &[PathBuf],
        claim_token: Option<&str>,
        before_publish: &mut dyn FnMut() -> Result<()>,
    ) -> Result<Capture> {
        let transaction_id = TransactionId(job_id.0);
        if sources.len() != capture.representations[0].files.len() {
            return Err(Error::Storage(
                "Capture payload source inventory changed".into(),
            ));
        }
        let record = archive_v3::CaptureRecordV3 {
            capture: capture.clone(),
            legacy_root: None,
        };
        let envelope = archive_v3::RevisionEnvelopeV3::new(transaction_id, record, vec![]);
        let record_bytes = serde_json::to_vec_pretty(&envelope).map_err(storage)?;
        let manifest = V3TypedManifest {
            schema_version: 3,
            entity_type: "capture".into(),
            entity_id: capture.id.to_string(),
            collection_id: capture.collection_id,
            revision: capture.revision,
            record_sha256: Digest::sha256(&record_bytes).hex,
            transaction_id: Some(transaction_id),
        };
        let manifest_bytes = toml::to_string_pretty(&manifest)
            .map_err(storage)?
            .into_bytes();
        let collection_relative = relative(&self.archive, collection_root)?;
        let stage_relative = format!(".staging/{transaction_id}");
        let payload_relative = format!(
            "{collection_relative}/captures/{}/payload/{}",
            capture.id, capture.representations[0].id
        );
        let revision_relative = format!(
            "{collection_relative}/captures/{}/revisions/{:020}",
            capture.id, capture.revision.0
        );
        let mut artifacts = Vec::new();
        for file in &capture.representations[0].files {
            artifacts.push(publication::JournalArtifact {
                kind: publication::ArtifactKind::Payload,
                staged_path: format!("{stage_relative}/payload/{}", file.name),
                published_path: format!("{payload_relative}/{}", file.name),
                sha256: file.digest.hex.clone(),
            });
        }
        artifacts.extend([
            publication::JournalArtifact {
                kind: publication::ArtifactKind::Revision,
                staged_path: format!("{stage_relative}/revision/record.v3.json"),
                published_path: format!("{revision_relative}/record.v3.json"),
                sha256: Digest::sha256(&record_bytes).hex,
            },
            publication::JournalArtifact {
                kind: publication::ArtifactKind::Revision,
                staged_path: format!("{stage_relative}/revision/manifest.v3.toml"),
                published_path: format!("{revision_relative}/manifest.v3.toml"),
                sha256: Digest::sha256(&manifest_bytes).hex,
            },
        ]);
        let journal = publication::PublicationJournal::new(
            transaction_id,
            "capture_import",
            capture.id.to_string(),
            Some(capture.collection_id),
            Revision(0),
            capture.revision,
            artifacts,
        )?;
        let journals = publication::PublicationJournalStore::new(&self.archive);
        journals.write_new(&journal)?;
        self.publication_fault
            .check(PublicationFaultPoint::Prepared)?;

        let stage = self.archive.join(&stage_relative);
        let payload_stage = stage.join("payload");
        let revision_stage = stage.join("revision");
        fs::create_dir_all(&payload_stage).map_err(storage)?;
        fs::create_dir_all(&revision_stage).map_err(storage)?;
        for (index, file) in capture.representations[0].files.iter().enumerate() {
            self.ensure_job_continues(job_id, claim_token)?;
            let source = &sources[index];
            let target = payload_stage.join(&file.name);
            fs::create_dir_all(target.parent().unwrap()).map_err(storage)?;
            fs::copy(source, &target).map_err(storage)?;
            File::open(&target)
                .and_then(|file| file.sync_all())
                .map_err(storage)?;
            if hash_file(&target)? != (file.bytes, file.digest.clone()) {
                return Err(Error::Storage("destination verification failed".into()));
            }
            sync_directory(target.parent().unwrap())?;
        }
        write_new(&revision_stage.join("record.v3.json"), &record_bytes)?;
        write_new(&revision_stage.join("manifest.v3.toml"), &manifest_bytes)?;
        sync_directory(&revision_stage)?;
        sync_directory(&stage)?;
        journals.advance(transaction_id, publication::PublicationPhase::BytesStaged)?;
        self.publication_fault
            .check(PublicationFaultPoint::BytesStaged)?;

        let payload_destination = self.archive.join(&payload_relative);
        fs::create_dir_all(payload_destination.parent().unwrap()).map_err(storage)?;
        if payload_destination.try_exists().map_err(storage)? {
            for file in &capture.representations[0].files {
                if hash_file(&payload_destination.join(&file.name))?
                    != (file.bytes, file.digest.clone())
                {
                    return Err(Error::Storage("orphan package differs from retry".into()));
                }
            }
            fs::remove_dir_all(&payload_stage).map_err(storage)?;
        } else {
            fs::rename(&payload_stage, &payload_destination).map_err(storage)?;
        }
        sync_directory(payload_destination.parent().unwrap())?;
        journals.advance(
            transaction_id,
            publication::PublicationPhase::PayloadPublished,
        )?;
        self.publication_fault
            .check(PublicationFaultPoint::PayloadPublished)?;

        {
            let _publication_gate = lock(&self.job_publication_gate)?;
            self.ensure_job_continues(job_id, claim_token)?;
            if let Err(error) = before_publish() {
                // Payload copies are still provisional. Retain upload sources while
                // clearing this uncommitted attempt so an authorized retry can resume.
                journals.abort(transaction_id)?;
                return Err(error);
            }
            let revision_destination = self.archive.join(&revision_relative);
            fs::create_dir_all(revision_destination.parent().unwrap()).map_err(storage)?;
            if revision_destination.try_exists().map_err(storage)? {
                if fs::read(revision_destination.join("record.v3.json")).map_err(storage)?
                    != record_bytes
                    || fs::read(revision_destination.join("manifest.v3.toml")).map_err(storage)?
                        != manifest_bytes
                {
                    return Err(Error::Storage(
                        "orphan Capture revision differs from retry".into(),
                    ));
                }
                fs::remove_dir_all(&revision_stage).map_err(storage)?;
            } else {
                fs::rename(&revision_stage, &revision_destination).map_err(storage)?;
            }
            sync_directory(revision_destination.parent().unwrap())?;
            journals.advance(
                transaction_id,
                publication::PublicationPhase::RevisionCommitted,
            )?;
            if let Some(claim_token) = claim_token {
                self.heartbeat_job_record(
                    job_id,
                    claim_token,
                    "revision_committed",
                    capture.representations[0]
                        .files
                        .iter()
                        .map(|file| file.bytes.0)
                        .sum(),
                    30,
                )?;
            }
        }
        self.publication_fault
            .check(PublicationFaultPoint::RevisionCommitted)?;

        let mut db = lock(&self.db)?;
        let tx = db.transaction().map_err(storage)?;
        project_capture(&tx, &capture)?;
        tx.commit().map_err(storage)?;
        drop(db);
        finish_v3_publication(self, transaction_id)?;
        Ok(capture)
    }

    pub(crate) fn ensure_job_continues(&self, id: JobId, claim_token: Option<&str>) -> Result<()> {
        if let Some(token) = claim_token
            && self.cancellation_requested_record(id, token)?
        {
            return Err(Error::JobCancelled);
        }
        Ok(())
    }
    pub(crate) fn jobs_for_user(&self, user: UserId) -> Result<Vec<Job>> {
        let db = lock(&self.db)?;
        let mut stmt = db
            .prepare("SELECT id,user_id,item_id,kind,state,completed_units,total_units,error FROM private.jobs WHERE user_id=? ORDER BY created_at DESC,id DESC LIMIT 500")
            .map_err(storage)?;
        let rows = stmt
            .query_map([user.to_string()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, u64>(5)?,
                    row.get::<_, u64>(6)?,
                    row.get::<_, Option<String>>(7)?,
                ))
            })
            .map_err(storage)?;
        let mut jobs = Vec::new();
        for row in rows {
            let (id, stored_user, item_id, kind, state, completed_bytes, total_bytes, error) =
                row.map_err(storage)?;
            let Some(item_id) = item_id else { continue };
            jobs.push(Job {
                id: id.parse().map_err(storage)?,
                user_id: stored_user.parse().map_err(storage)?,
                item_id: item_id.parse().map_err(storage)?,
                kind,
                state: crate::parse_job_state(&state)?,
                completed_bytes,
                total_bytes,
                error,
            });
        }
        Ok(jobs)
    }
    pub(crate) fn representation_path(
        &self,
        item_id: ItemId,
        representation: RepresentationId,
        index: usize,
    ) -> Result<(PathBuf, StoredFile, bool)> {
        let item = self.item(item_id)?;
        if item.deleted {
            return Err(Error::NotFound);
        }
        // Historical files remain addressable for existing queues and provenance.
        // Recheck the live association: moving a capture must revoke the old URL.
        let mut captures = item.captures.clone();
        for id in &item.lifecycle.historical_captures {
            let capture = match self.capture(*id) {
                Ok(capture) => capture,
                // A superseded replacement may have stopped before publishing.
                Err(Error::NotFound) => continue,
                Err(error) => return Err(error),
            };
            if capture.collection_id == item.collection_id
                && capture.association.item_id == Some(item.id)
            {
                captures.push(capture);
            }
        }
        let capture = captures
            .iter()
            .find(|capture| {
                capture
                    .representations
                    .iter()
                    .any(|value| value.id == representation)
            })
            .ok_or(Error::NotFound)?;
        let file = capture
            .representations
            .iter()
            .find(|value| value.id == representation)
            .and_then(|value| value.files.get(index))
            .ok_or(Error::NotFound)?
            .clone();
        safe_name(&file.name)?;
        let item_root = self.item_path(item.id)?;
        let legacy_locator: Option<(String, String)> = lock(&self.db)?
            .query_row(
                "SELECT item_id,relative_path FROM legacy_payload_locators WHERE capture_id=?",
                [capture.id.to_string()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(storage)?;
        // New derivatives of migrated masters use the canonical Capture path.
        // Existing legacy representations retain their original locator.
        let canonical_derivative: bool = lock(&self.db)?
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM conversion_recipes WHERE id=? AND collection_id=?)",
                params![representation.to_string(), item.collection_id.to_string()],
                |row| row.get(0),
            )
            .map_err(storage)?;
        let root = if let Some((legacy_item_id, relative_path)) =
            legacy_locator.filter(|_| !canonical_derivative)
        {
            if relative_path != "files" {
                return Err(Error::Invalid("unsupported legacy payload locator".into()));
            }
            let legacy_item_id: ItemId = legacy_item_id.parse().map_err(storage)?;
            let original_item = self.item(legacy_item_id)?;
            if original_item.collection_id != item.collection_id {
                return Err(Error::Forbidden);
            }
            self.item_path(legacy_item_id)?
                .join("files")
                .join(representation.to_string())
        } else {
            let collection_root = item_root
                .parent()
                .and_then(std::path::Path::parent)
                .ok_or_else(|| Error::Storage("item path has no collection parent".into()))?;
            collection_root
                .join("captures")
                .join(capture.id.to_string())
                .join("payload")
                .join(representation.to_string())
        };
        let path = fs::canonicalize(root.join(&file.name)).map_err(|error| {
            if canonical_derivative {
                tracing::error!(%error, representation = %representation, "prepared media path unavailable");
                Error::PreparedMediaInvalid
            } else {
                storage(error)
            }
        })?;
        if !path.starts_with(fs::canonicalize(root).map_err(storage)?) {
            return Err(Error::Forbidden);
        }
        Ok((path, file, canonical_derivative))
    }
    pub(crate) fn asset_state_record(&self, asset_id: AssetId) -> Result<AssetState> {
        let record: String = lock(&self.db)?
            .query_row(
                "SELECT record FROM assets WHERE id=?",
                [asset_id.to_string()],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage)?
            .ok_or(Error::NotFound)?;
        let record: archive_v3::AssetRecordV3 = serde_json::from_str(&record).map_err(storage)?;
        Ok(AssetState {
            collection_id: record.collection_id,
            revision: record.revision,
            asset: record.asset,
            deleted: record.deleted,
            updated_at: record.updated_at,
        })
    }

    pub(crate) fn asset_states_for_item_record(&self, item_id: ItemId) -> Result<Vec<AssetState>> {
        let item = self.item(item_id)?;
        let inherited_work = match item.identity_link {
            ItemIdentityLink::Work { work_id } => Some(work_id),
            ItemIdentityLink::Edition { edition_id } => self.edition(edition_id)?.work_id,
            ItemIdentityLink::None => None,
        };
        let db = lock(&self.db)?;
        let mut statement = db
            .prepare("SELECT record FROM assets WHERE collection_id=? ORDER BY id")
            .map_err(storage)?;
        let rows = statement
            .query_map([item.collection_id.to_string()], |row| {
                row.get::<_, String>(0)
            })
            .map_err(storage)?;
        let mut states = Vec::new();
        for row in rows {
            let record: archive_v3::AssetRecordV3 =
                serde_json::from_str(&row.map_err(storage)?).map_err(storage)?;
            if asset_belongs_to_item(&record.asset, &item, inherited_work) {
                states.push(AssetState {
                    collection_id: record.collection_id,
                    revision: record.revision,
                    asset: record.asset,
                    deleted: record.deleted,
                    updated_at: record.updated_at,
                });
            }
        }
        Ok(states)
    }

    pub(crate) fn asset_subject_collection_record(
        &self,
        subject: AssetSubject,
    ) -> Result<CollectionId> {
        match subject {
            AssetSubject::Item { item_id } => Ok(self.item(item_id)?.collection_id),
            AssetSubject::Work { work_id } => Ok(self.work(work_id)?.collection_id),
            AssetSubject::Edition { edition_id } => Ok(self.edition(edition_id)?.collection_id),
            AssetSubject::Capture { capture_id } => Ok(self.capture(capture_id)?.collection_id),
            AssetSubject::Specimen { .. } | AssetSubject::Component { .. } => {
                let id = self
                    .asset_subject_item_record(subject)?
                    .ok_or(Error::NotFound)?;
                Ok(self.item(id)?.collection_id)
            }
        }
    }

    pub(crate) fn asset_subject_item_record(
        &self,
        subject: AssetSubject,
    ) -> Result<Option<ItemId>> {
        match subject {
            AssetSubject::Item { item_id } => Ok(Some(item_id)),
            AssetSubject::Capture { capture_id } => {
                Ok(self.capture(capture_id)?.association.item_id)
            }
            AssetSubject::Specimen { specimen_id } => self
                .subject_owner("specimen", specimen_id.0)
                .map(|(id, _)| Some(id)),
            AssetSubject::Component { component_id } => self
                .subject_owner("component", component_id.0)
                .map(|(id, _)| Some(id)),
            AssetSubject::Work { .. } | AssetSubject::Edition { .. } => Ok(None),
        }
    }

    fn subject_owner(&self, kind: &str, id: uuid::Uuid) -> Result<(ItemId, CollectionId)> {
        let db = lock(&self.db)?;
        db.query_row("SELECT root_id,collection_id FROM entity_owners WHERE kind=? AND id=? AND root_kind='item'",
            params![kind,id.to_string()], |row| Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?)))
            .optional().map_err(storage)?.map(|(item,collection)| Ok((ItemId(uuid::Uuid::parse_str(&item).map_err(storage)?),CollectionId(uuid::Uuid::parse_str(&collection).map_err(storage)?))))
            .unwrap_or(Err(Error::NotFound))
    }

    pub(crate) fn create_asset_record(
        &self,
        asset_id: AssetId,
        command: CreateAssetCommand,
        bytes: &[u8],
        context: collector_app::AssetWriteContext,
    ) -> Result<AssetState> {
        self.write_asset_record(asset_id, command, bytes, context, ExistingAsset::Reject)
    }

    pub(crate) fn materialize_provider_artwork_record(
        &self,
        asset_id: AssetId,
        command: CreateAssetCommand,
        bytes: &[u8],
        context: collector_app::ItemEffectContext,
        restore_archived: Option<Revision>,
    ) -> Result<AssetState> {
        self.write_asset_record(
            asset_id,
            command,
            bytes,
            collector_app::AssetWriteContext::Item(context),
            ExistingAsset::Reuse { restore_archived },
        )
    }

    fn write_asset_record(
        &self,
        asset_id: AssetId,
        command: CreateAssetCommand,
        bytes: &[u8],
        context: collector_app::AssetWriteContext,
        existing: ExistingAsset,
    ) -> Result<AssetState> {
        let CreateAssetCommand {
            collection_id,
            subject,
            kind,
            visibility,
            alt_text,
        } = command;
        let provider_cover = kind == AssetKind::Cover
            && matches!(
                context,
                collector_app::AssetWriteContext::Item(collector_app::ItemEffectContext {
                    source: collector_app::ItemEffectSource::Snapshot(_)
                        | collector_app::ItemEffectSource::Job { .. },
                    ..
                })
            );
        let byte_limit = if provider_cover {
            collector_app::MAX_PROVIDER_ARTWORK_BYTES
        } else {
            collector_app::MAX_ASSET_BYTES
        };
        if bytes.len() > byte_limit || alt_text.len() > 4096 {
            return Err(Error::Invalid("asset exceeds upload limit".into()));
        }
        let (extension, media_type, thumbnail) = if kind == AssetKind::Document {
            let format = junk_libs_core::document::classify(bytes, collector_app::MAX_ASSET_BYTES)
                .map_err(|e| Error::Invalid(e.to_string()))?;
            (format.extension(), format.media_type(), None)
        } else {
            if !matches!(kind, AssetKind::Cover | AssetKind::Photo | AssetKind::Scan) {
                return Err(Error::Invalid("unsupported asset kind".into()));
            }
            let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes))
                .with_guessed_format()
                .map_err(storage)?;
            let format = reader
                .format()
                .ok_or_else(|| Error::Invalid("unsupported image".into()))?;
            let (extension, media_type) = match format {
                image::ImageFormat::Png => ("png", "image/png"),
                image::ImageFormat::Jpeg => ("jpg", "image/jpeg"),
                image::ImageFormat::WebP => ("webp", "image/webp"),
                _ => return Err(Error::Invalid("use PNG, JPEG, or WebP".into())),
            };
            let mut limits = image::Limits::default();
            limits.max_image_width = Some(12000);
            limits.max_image_height = Some(12000);
            limits.max_alloc = Some(256 * 1024 * 1024);
            reader.limits(limits);
            let image = reader
                .decode()
                .map_err(|error| Error::Invalid(error.to_string()))?;
            let mut thumbnail = std::io::Cursor::new(Vec::new());
            image
                .thumbnail(192, 192)
                .write_to(&mut thumbnail, image::ImageFormat::Png)
                .map_err(storage)?;
            let thumbnail = thumbnail.into_inner();
            (extension, media_type, Some(thumbnail))
        };
        let digest = Digest::sha256(bytes);

        let _writer = lock(&self.writer)?;
        let _publication_gate = lock(&self.job_publication_gate)?;
        if let collector_app::AssetWriteContext::Item(context) = context {
            if self.asset_subject_item_record(subject)? != Some(context.item_id) {
                return Err(Error::Conflict);
            }
            self.validate_item_effect(context, collection_id)?;
            if matches!(existing, ExistingAsset::Reuse { .. })
                && let collector_app::ItemEffectSource::Job { id, claim } = context.source
                && self.cancellation_requested_record(id, &claim.to_string())?
            {
                return Err(Error::JobCancelled);
            }
        }
        if self.asset_subject_collection_record(subject)? != collection_id {
            return Err(Error::Invalid(
                "asset subject is outside the selected collection".into(),
            ));
        }
        match self.asset_record_v3(asset_id) {
            Ok(mut current) => {
                let ExistingAsset::Reuse { restore_archived } = existing else {
                    return Err(Error::IdempotencyConflict);
                };
                if current.collection_id != collection_id
                    || current.asset.subject != subject
                    || current.asset.kind != kind
                    || current.asset.media_type != media_type
                    || current.asset.file.digest != digest
                    || current.asset.file.bytes != ByteLength(bytes.len() as u64)
                {
                    return Err(Error::IdempotencyConflict);
                }
                if !current.deleted {
                    // Preserve curated visibility and alternative text on reuse.
                    return self.asset_state_record(asset_id);
                }
                if restore_archived != Some(current.revision) {
                    return Err(Error::Conflict);
                }
                current.revision =
                    Revision(current.revision.0.checked_add(1).ok_or(Error::Conflict)?);
                current.deleted = false;
                current.updated_at = self.clock.now();
                let collection = collection_path(&*lock(&self.db)?, &self.archive, collection_id)?;
                return self.publish_asset_state(current, &collection, false);
            }
            Err(Error::NotFound) => {
                if matches!(
                    existing,
                    ExistingAsset::Reuse {
                        restore_archived: Some(_)
                    }
                ) {
                    return Err(Error::Conflict);
                }
            }
            Err(error) => return Err(error),
        }
        let collection = collection_path(&*lock(&self.db)?, &self.archive, collection_id)?;
        let staging = self
            .state
            .join("asset-publication")
            .join(asset_id.to_string());
        fs::create_dir_all(&staging).map_err(storage)?;
        let original_name = format!("original.{extension}");
        write_new_or_verify(&staging.join(&original_name), bytes)?;
        if let Some(thumbnail) = &thumbnail {
            write_new_or_verify(&staging.join("thumbnail.png"), thumbnail)?;
        }
        sync_directory(&staging)?;
        sync_directory(
            staging
                .parent()
                .ok_or_else(|| Error::Storage("Asset staging has no parent".into()))?,
        )?;
        let asset = Asset {
            id: asset_id,
            subject,
            kind,
            visibility,
            media_type: media_type.into(),
            file: StoredFile {
                visibility: Default::default(),
                name: original_name,
                bytes: ByteLength(bytes.len() as u64),
                digest,
            },
            thumbnail: thumbnail.as_ref().map(|thumbnail| StoredFile {
                visibility: Default::default(),
                name: "thumbnail.png".into(),
                bytes: ByteLength(thumbnail.len() as u64),
                digest: Digest::sha256(thumbnail),
            }),
            alt_text,
        };
        let payload_root = collection
            .join("assets")
            .join(asset_id.to_string())
            .join("payload");
        let mut legacy_payloads = vec![archive_v3::LegacyPayloadV3 {
            role: "original".into(),
            relative_path: relative(&self.archive, &payload_root.join(&asset.file.name))?,
            sha256: asset.file.digest.hex.clone(),
        }];
        if let Some(thumbnail) = &asset.thumbnail {
            legacy_payloads.push(archive_v3::LegacyPayloadV3 {
                role: "thumbnail".into(),
                relative_path: relative(&self.archive, &payload_root.join(&thumbnail.name))?,
                sha256: thumbnail.digest.hex.clone(),
            });
        }
        let record = archive_v3::AssetRecordV3 {
            collection_id,
            revision: Revision(1),
            legacy_payloads,
            asset,
            deleted: false,
            updated_at: self.clock.now(),
        };
        self.publish_asset_state(record, &collection, true)
    }

    pub(crate) fn update_asset_record(
        &self,
        asset_id: AssetId,
        expected: Revision,
        visibility: Visibility,
        alt_text: String,
    ) -> Result<AssetState> {
        if alt_text.len() > 4096 {
            return Err(Error::Invalid("alt text exceeds upload limit".into()));
        }
        let _writer = lock(&self.writer)?;
        let current = self.asset_record_v3(asset_id)?;
        if current.deleted {
            return Err(Error::NotFound);
        }
        if current.revision != expected {
            return Err(Error::Conflict);
        }
        let collection = collection_path(&*lock(&self.db)?, &self.archive, current.collection_id)?;
        let mut record = current;
        record.revision = Revision(
            expected
                .0
                .checked_add(1)
                .ok_or_else(|| Error::Invalid("Asset revision overflow".into()))?,
        );
        record.asset.visibility = visibility;
        record.asset.alt_text = alt_text;
        record.updated_at = self.clock.now();
        self.publish_asset_state(record, &collection, false)
    }

    pub(crate) fn tombstone_asset_record(
        &self,
        asset_id: AssetId,
        expected: Revision,
    ) -> Result<AssetState> {
        let _writer = lock(&self.writer)?;
        let current = self.asset_record_v3(asset_id)?;
        if current.deleted {
            return Err(Error::NotFound);
        }
        if current.revision != expected {
            return Err(Error::Conflict);
        }
        let collection = collection_path(&*lock(&self.db)?, &self.archive, current.collection_id)?;
        let mut record = current;
        record.revision = Revision(
            expected
                .0
                .checked_add(1)
                .ok_or_else(|| Error::Invalid("Asset revision overflow".into()))?,
        );
        record.deleted = true;
        record.updated_at = self.clock.now();
        self.publish_asset_state(record, &collection, false)
    }

    fn asset_record_v3(&self, asset_id: AssetId) -> Result<archive_v3::AssetRecordV3> {
        let record: String = lock(&self.db)?
            .query_row(
                "SELECT record FROM assets WHERE id=?",
                [asset_id.to_string()],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage)?
            .ok_or(Error::NotFound)?;
        serde_json::from_str(&record).map_err(storage)
    }

    fn publish_asset_state(
        &self,
        record: archive_v3::AssetRecordV3,
        collection: &Path,
        publish_payloads: bool,
    ) -> Result<AssetState> {
        let state = AssetState {
            collection_id: record.collection_id,
            revision: record.revision,
            asset: record.asset.clone(),
            deleted: record.deleted,
            updated_at: record.updated_at,
        };
        let payloads = if publish_payloads {
            staged_asset_payloads(&self.archive, &self.state, collection, &record.asset)?
        } else {
            vec![]
        };
        let transaction = publish_v3_revision_with_payloads(
            &self.with_command_result(&state)?,
            V3RevisionTarget {
                collection,
                directory_kind: "assets",
                entity_id: record.asset.id.to_string(),
                collection_id: record.collection_id,
                expected: Revision(record.revision.0 - 1),
                revision: record.revision,
            },
            record.clone(),
            vec![],
            payloads,
        )?;
        lock(&self.db)?
            .execute(
                "INSERT INTO assets(id,collection_id,revision,record,deleted) VALUES(?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET collection_id=excluded.collection_id,revision=excluded.revision,record=excluded.record,deleted=excluded.deleted",
                params![record.asset.id.to_string(),record.collection_id.to_string(),record.revision.0,serde_json::to_string(&record).map_err(storage)?,record.deleted],
            )
            .map_err(storage)?;
        finish_v3_publication(self, transaction)?;
        if publish_payloads {
            let staging = self
                .state
                .join("asset-publication")
                .join(record.asset.id.to_string());
            if staging.try_exists().map_err(storage)? {
                fs::remove_dir_all(&staging).map_err(storage)?;
                sync_directory(
                    staging
                        .parent()
                        .ok_or_else(|| Error::Storage("Asset staging has no parent".into()))?,
                )?;
            }
        }
        Ok(state)
    }

    pub(crate) fn asset_resource_path(
        &self,
        asset_id: AssetId,
        thumbnail: bool,
    ) -> Result<(PathBuf, String, Digest, String)> {
        let record = self.asset_record_v3(asset_id)?;
        if record.deleted {
            return Err(Error::NotFound);
        }
        let asset = &record.asset;
        let file = if thumbnail {
            asset.thumbnail.as_ref().ok_or(Error::NotFound)?
        } else {
            &asset.file
        };
        safe_name(&file.name)?;
        let role = if thumbnail { "thumbnail" } else { "original" };
        let locator = record
            .legacy_payloads
            .iter()
            .find(|locator| locator.role == role)
            .ok_or_else(|| Error::Storage("Asset payload locator is missing".into()))?;
        let path = fs::canonicalize(self.archive.join(&locator.relative_path)).map_err(storage)?;
        if !path.starts_with(fs::canonicalize(&self.archive).map_err(storage)?) {
            return Err(Error::Forbidden);
        }
        if Digest::sha256(&fs::read(&path).map_err(storage)?).hex != locator.sha256 {
            return Err(Error::Storage("Asset payload digest mismatch".into()));
        }
        Ok((
            path,
            if thumbnail {
                "image/png".into()
            } else {
                asset.media_type.clone()
            },
            file.digest.clone(),
            file.name.clone(),
        ))
    }
    pub(crate) fn verify_integrity_record(&self, item_id: ItemId) -> Result<Item> {
        self.verify_integrity_inner(item_id, None, Utc::now(), None)
    }
    pub(crate) fn verify_integrity_job_record(
        &self,
        item_id: ItemId,
        job_id: JobId,
        claim_token: &str,
        created_at: chrono::DateTime<Utc>,
        progress: &mut dyn FnMut(u64) -> Result<()>,
    ) -> Result<Item> {
        self.verify_integrity_inner(
            item_id,
            Some((job_id, claim_token)),
            created_at,
            Some(progress),
        )
    }
    fn verify_integrity_inner(
        &self,
        item_id: ItemId,
        job: Option<(JobId, &str)>,
        created_at: chrono::DateTime<Utc>,
        mut progress: Option<&mut dyn FnMut(u64) -> Result<()>>,
    ) -> Result<Item> {
        let item = self.item(item_id)?;
        let mut evidence = Vec::new();
        let mut completed_units = 0;
        for representation in item.captures.iter().flat_map(|c| &c.representations) {
            let mut passed = true;
            let mut details = Vec::new();
            for (index, file) in representation.files.iter().enumerate() {
                let check = self
                    .representation_path(item.id, representation.id, index)
                    .and_then(|(path, _, _)| hash_file(&path));
                let matches = check
                    .is_ok_and(|(length, digest)| length == file.bytes && digest == file.digest);
                passed &= matches;
                details.push(serde_json::json!({"file":file.name,"matches":matches}));
                completed_units += 1;
                if let Some(progress) = progress.as_deref_mut() {
                    progress(completed_units)?;
                }
            }
            evidence.push(Evidence {
                id: job.map_or_else(EvidenceId::new, |(job_id, _)| {
                    deterministic_integrity_evidence_id(job_id, representation.id)
                }),
                item_id: item.id,
                representation_id: representation.id,
                kind: EvidenceKind::Integrity,
                outcome: if passed {
                    EvidenceOutcome::Passed
                } else {
                    EvidenceOutcome::Failed
                },
                inputs: representation
                    .files
                    .iter()
                    .map(|f| f.digest.clone())
                    .collect(),
                implementation: "collector-store sha256 reread".into(),
                implementation_version: env!("CARGO_PKG_VERSION").into(),
                catalog: None,
                details: serde_json::json!({"files":details}),
                created_at,
            });
        }
        if job.is_some() {
            let existing = self.evidence(item.id)?;
            let mut present = 0usize;
            for intended in &evidence {
                if let Some(recorded) = existing.iter().find(|value| value.id == intended.id) {
                    if serde_json::to_value(recorded).map_err(storage)?
                        != serde_json::to_value(intended).map_err(storage)?
                    {
                        return Err(Error::Storage(
                            "durable integrity retry conflicts with published evidence".into(),
                        ));
                    }
                    present += 1;
                }
            }
            if present == evidence.len() {
                return Ok(item);
            }
            if present != 0 {
                return Err(Error::Storage(
                    "durable integrity evidence is only partially published".into(),
                ));
            }
        }
        let expected = item.revision;
        if let Some((job_id, claim_token)) = job {
            let _publication_gate = lock(&self.job_publication_gate)?;
            if let Some(progress) = progress {
                progress(completed_units)?;
            }
            let result = self.publish(item, expected, evidence)?;
            self.heartbeat_job_record(
                job_id,
                claim_token,
                "revision_committed",
                completed_units,
                30,
            )?;
            Ok(result)
        } else {
            self.publish(item, expected, evidence)
        }
    }
}

pub(crate) fn deterministic_integrity_evidence_id(
    job_id: JobId,
    representation_id: RepresentationId,
) -> EvidenceId {
    let mut bytes: [u8; 16] = Sha256::digest(format!(
        "junk-collector:integrity:{job_id}:{representation_id}"
    ))[..16]
        .try_into()
        .expect("SHA-256 prefix has fixed length");
    bytes[6] = (bytes[6] & 0x0f) | 0x50;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    EvidenceId(uuid::Uuid::from_bytes(bytes))
}

impl Store {
    /// Record a failed background attempt durably. The source chunks remain available for retry.
    pub(crate) fn fail_upload_record(&self, id: JobId, message: &str) -> Result<()> {
        let _guard = lock(&self.uploads)?;
        let mut upload = self.upload_record(id)?;
        if upload.state == JobState::Completed {
            return Ok(());
        }
        upload.state = JobState::Failed;
        self.persist_upload(&upload)?;
        let job = Job {
            id,
            user_id: upload.user_id,
            item_id: upload.item_id,
            kind: "preservation_import".into(),
            state: JobState::Failed,
            completed_bytes: upload.offsets.iter().sum(),
            total_bytes: upload.files.iter().map(|f| f.bytes.0).sum(),
            error: Some(message.chars().take(512).collect()),
        };
        lock(&self.db)?
            .execute(
                "UPDATE private.jobs SET state='failed',phase='failed',updated_at=?,error=?
                 WHERE id=? AND claim_token IS NULL",
                params![Utc::now().timestamp(), job.error, id.to_string()],
            )
            .map_err(storage)?;
        Ok(())
    }
    /// Inventory a server-admin configured inbox directory. Reject symlinks and special files.
    pub fn inventory_directory(source: &Path) -> Result<Vec<UploadFile>> {
        Self::inventory_directory_retaining(source, &[])
    }

    /// Existing accepted inventories retain their exact housekeeping originals on recovery.
    pub fn inventory_directory_retaining(
        source: &Path,
        retained: &[String],
    ) -> Result<Vec<UploadFile>> {
        fn walk(
            root: &Path,
            path: &Path,
            out: &mut Vec<UploadFile>,
            depth: usize,
            retained: &[String],
        ) -> Result<()> {
            if depth > 32 || out.len() > 4096 {
                return Err(Error::Invalid(
                    "package is too deeply nested or contains too many files".into(),
                ));
            }
            let metadata = fs::symlink_metadata(path).map_err(storage)?;
            let relative = path
                .strip_prefix(root)
                .map_err(storage)?
                .to_str()
                .ok_or_else(|| Error::Invalid("package names must be UTF-8".into()))?;
            if collector_app::is_intake_housekeeping(relative, metadata.is_dir())
                && !retained.iter().any(|name| {
                    name == relative
                        || (metadata.is_dir() && name.starts_with(&format!("{relative}/")))
                })
            {
                return Ok(());
            }
            if metadata.file_type().is_symlink() {
                return Err(Error::Invalid(
                    "inbox packages cannot contain symlinks".into(),
                ));
            }
            if metadata.is_dir() {
                for entry in fs::read_dir(path).map_err(storage)? {
                    walk(
                        root,
                        &entry.map_err(storage)?.path(),
                        out,
                        depth + 1,
                        retained,
                    )?;
                }
            } else if metadata.is_file() {
                let name = path
                    .strip_prefix(root)
                    .map_err(storage)?
                    .to_str()
                    .ok_or_else(|| Error::Invalid("package names must be UTF-8".into()))?
                    .to_owned();
                safe_name(&name)?;
                out.push(UploadFile {
                    visibility: Default::default(),
                    name,
                    bytes: ByteLength(metadata.len()),
                    sha256: None,
                });
            } else {
                return Err(Error::Invalid(
                    "inbox package contains a special file".into(),
                ));
            }
            Ok(())
        }
        let mut files = Vec::new();
        walk(source, source, &mut files, 0, retained)?;
        files.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(files)
    }
}

#[cfg(test)]
mod retained_hash_tests {
    use super::*;
    #[test]
    fn staged_hash_checks_cancellation_and_declared_size_during_read() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("payload");
        fs::write(&path, vec![0u8; 3 * 1024 * 1024]).unwrap();
        let mut checkpoints = 0;
        let result = hash_file_checked(&path, 3 * 1024 * 1024, &mut || {
            checkpoints += 1;
            if checkpoints == 2 {
                Err(Error::JobCancelled)
            } else {
                Ok(())
            }
        });
        assert!(matches!(result, Err(Error::JobCancelled)));
        assert_eq!(checkpoints, 2);
        assert!(matches!(
            hash_file_checked(&path, 1024, &mut || Ok(())),
            Err(Error::Invalid(_))
        ));
    }
}
