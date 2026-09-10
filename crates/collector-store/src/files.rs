use super::*;
use sha2::{Digest as _, Sha256};
use std::io::{Read, Seek, SeekFrom};

const CHUNK_LIMIT: usize = 8 * 1024 * 1024;
fn safe_name(name: &str) -> Result<()> {
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
fn hash_file(path: &Path) -> Result<(ByteLength, Digest)> {
    let mut file = File::open(path).map_err(storage)?;
    let mut hash = Sha256::new();
    let mut bytes = 0u64;
    let mut buffer = vec![0u8; 1024 * 1024];
    loop {
        let read = file.read(&mut buffer).map_err(storage)?;
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
    if path.exists() {
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
            if !path.join("upload.json").exists() {
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
        if staging.exists() {
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
        let collection_id = self.item(upload.item_id)?.collection_id;
        let mut db = lock(&self.db)?;
        let tx = db.transaction().map_err(storage)?;
        let now = Utc::now().timestamp();
        tx.execute(
            "INSERT INTO private.jobs(
              id,user_id,collection_id,item_id,kind,state,phase,
              completed_units,total_units,attempt,created_at,updated_at,
              error,limitation,request,result,worker_instance,lease_expires,
              claim_token,cancellation_requested
            ) VALUES(?,?,?,?,?,?,?,?,?,1,?,?,?,NULL,?,NULL,NULL,NULL,NULL,0)
            ON CONFLICT(id) DO UPDATE SET
              state=excluded.state,phase=excluded.phase,
              completed_units=excluded.completed_units,total_units=excluded.total_units,
              updated_at=excluded.updated_at,error=excluded.error,request=excluded.request",
            params![
                job.id.to_string(),
                job.user_id.to_string(),
                collection_id.to_string(),
                job.item_id.to_string(),
                job.kind,
                crate::job_state_storage(&job.state),
                crate::job_phase_storage(&job.state),
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
        let _guard = lock(&self.uploads)?;
        let mut upload = self.upload_record(id)?;
        if upload.state == JobState::Completed {
            return Err(Error::Conflict);
        }
        upload.state = JobState::Cancelled;
        self.with_command_result(&())?.persist_upload(&upload)
    }
    pub(crate) fn commit_upload_record(&self, id: JobId) -> Result<Item> {
        let _guard = lock(&self.uploads)?;
        let _writer = lock(&self.writer)?;
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
            let source = self.upload_directory(id).join(format!("{index}.part"));
            if file.bytes.0 == 0 && !source.exists() {
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
                    id: upload.capture_id,
                    collection_id: item.collection_id,
                    revision: Revision(1),
                    association: CaptureAssociation {
                        item_id: Some(item.id),
                        ..Default::default()
                    },
                    created_at: Utc::now(),
                    representations: vec![Representation {
                        id: upload.representation_id,
                        role: RepresentationRole::PreservationMaster,
                        format: "original-package".into(),
                        files,
                        source: None,
                        profile: None,
                    }],
                };
                self.publish_upload_capture(&upload, collection_root, capture)?
            }
        };
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

    fn publish_upload_capture(
        &self,
        upload: &Upload,
        collection_root: &Path,
        capture: Capture,
    ) -> Result<Capture> {
        let transaction_id = TransactionId(upload.id.0);
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
            capture.id, upload.representation_id
        );
        let revision_relative = format!(
            "{collection_relative}/captures/{}/revisions/{:020}",
            capture.id, capture.revision.0
        );
        let mut artifacts = Vec::new();
        for (index, file) in capture.representations[0].files.iter().enumerate() {
            artifacts.push(publication::JournalArtifact {
                kind: publication::ArtifactKind::Payload,
                staged_path: format!("{stage_relative}/payload/{}", file.name),
                published_path: format!("{payload_relative}/{}", file.name),
                sha256: file.digest.hex.clone(),
            });
            if upload.files[index].name != file.name {
                return Err(Error::Storage("upload file identity changed".into()));
            }
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
            let source = self
                .upload_directory(upload.id)
                .join(format!("{index}.part"));
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
        if payload_destination.exists() {
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

        let revision_destination = self.archive.join(&revision_relative);
        fs::create_dir_all(revision_destination.parent().unwrap()).map_err(storage)?;
        if revision_destination.exists() {
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
        self.publication_fault
            .check(PublicationFaultPoint::RevisionCommitted)?;

        let mut db = lock(&self.db)?;
        let tx = db.transaction().map_err(storage)?;
        project_capture(&tx, &capture)?;
        tx.commit().map_err(storage)?;
        finish_v3_publication(self, transaction_id)?;
        Ok(capture)
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
    ) -> Result<(PathBuf, StoredFile)> {
        let item = self.item(item_id)?;
        if item.deleted {
            return Err(Error::NotFound);
        }
        let capture = item
            .captures
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
        let root = if let Some((legacy_item_id, relative_path)) = legacy_locator {
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
        let path = fs::canonicalize(root.join(&file.name)).map_err(storage)?;
        if !path.starts_with(fs::canonicalize(root).map_err(storage)?) {
            return Err(Error::Forbidden);
        }
        Ok((path, file))
    }
    pub(crate) fn publish_cover_record(
        &self,
        item_id: ItemId,
        asset_id: AssetId,
        expected: Revision,
        bytes: &[u8],
        alt_text: String,
    ) -> Result<Item> {
        let mut item = self.item(item_id)?;
        if let Some(existing) = item.assets.iter().find(|asset| asset.id == asset_id) {
            if existing.file.digest == Digest::sha256(bytes) && existing.alt_text == alt_text {
                let staging = self
                    .state
                    .join("asset-publication")
                    .join(asset_id.to_string());
                if staging.exists() {
                    fs::remove_dir_all(&staging).map_err(storage)?;
                    sync_directory(
                        staging
                            .parent()
                            .ok_or_else(|| Error::Storage("Asset staging has no parent".into()))?,
                    )?;
                }
                return Ok(item);
            }
            return Err(Error::IdempotencyConflict);
        }
        if item.revision != expected {
            return Err(Error::Conflict);
        }
        if bytes.len() > 20 * 1024 * 1024 || alt_text.len() > 4096 {
            return Err(Error::Invalid("image exceeds upload limit".into()));
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
        let image = reader.decode().map_err(|e| Error::Invalid(e.to_string()))?;
        let mut thumbnail = std::io::Cursor::new(Vec::new());
        image
            .thumbnail(192, 192)
            .write_to(&mut thumbnail, image::ImageFormat::Png)
            .map_err(storage)?;
        let thumbnail = thumbnail.into_inner();
        let root = self
            .state
            .join("asset-publication")
            .join(asset_id.to_string());
        fs::create_dir_all(&root).map_err(storage)?;
        let name = format!("original.{extension}");
        write_new_or_verify(&root.join(&name), bytes)?;
        write_new_or_verify(&root.join("thumbnail.png"), &thumbnail)?;
        sync_directory(&root)?;
        sync_directory(root.parent().unwrap())?;
        item.assets.push(Asset {
            id: asset_id,
            subject: AssetSubject::Item { item_id },
            kind: AssetKind::Cover,
            visibility: Visibility::Shared,
            media_type: media_type.into(),
            file: StoredFile {
                name,
                bytes: ByteLength(bytes.len() as u64),
                digest: Digest::sha256(bytes),
            },
            thumbnail: Some(StoredFile {
                name: "thumbnail.png".into(),
                bytes: ByteLength(thumbnail.len() as u64),
                digest: Digest::sha256(&thumbnail),
            }),
            alt_text,
        });
        self.publish(item, expected, vec![])
    }
    pub(crate) fn asset_path(
        &self,
        item_id: ItemId,
        asset_id: AssetId,
        thumbnail: bool,
    ) -> Result<(PathBuf, String, Digest, String)> {
        let item = self.item(item_id)?;
        if item.deleted {
            return Err(Error::NotFound);
        }
        let asset = item
            .assets
            .iter()
            .find(|a| a.id == asset_id)
            .ok_or(Error::NotFound)?;
        let file = if thumbnail {
            asset.thumbnail.as_ref().ok_or(Error::NotFound)?
        } else {
            &asset.file
        };
        safe_name(&file.name)?;
        let record: String = lock(&self.db)?
            .query_row(
                "SELECT record FROM assets WHERE id=?",
                [asset_id.to_string()],
                |row| row.get(0),
            )
            .map_err(storage)?;
        let record: archive_v3::AssetRecordV3 = serde_json::from_str(&record).map_err(storage)?;
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
        self.verify_integrity_inner(item_id, None, Utc::now())
    }
    pub(crate) fn verify_integrity_job_record(
        &self,
        item_id: ItemId,
        job_id: JobId,
        created_at: chrono::DateTime<Utc>,
    ) -> Result<Item> {
        self.verify_integrity_inner(item_id, Some(job_id), created_at)
    }
    fn verify_integrity_inner(
        &self,
        item_id: ItemId,
        job_id: Option<JobId>,
        created_at: chrono::DateTime<Utc>,
    ) -> Result<Item> {
        let item = self.item(item_id)?;
        let mut evidence = Vec::new();
        for representation in item.captures.iter().flat_map(|c| &c.representations) {
            let mut passed = true;
            let mut details = Vec::new();
            for (index, file) in representation.files.iter().enumerate() {
                let check = self
                    .representation_path(item.id, representation.id, index)
                    .and_then(|(path, _)| hash_file(&path));
                let matches = check
                    .is_ok_and(|(length, digest)| length == file.bytes && digest == file.digest);
                passed &= matches;
                details.push(serde_json::json!({"file":file.name,"matches":matches}));
            }
            evidence.push(Evidence {
                id: job_id.map_or_else(EvidenceId::new, |job_id| {
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
        if job_id.is_some() {
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
        self.publish(item, expected, evidence)
    }
}

fn deterministic_integrity_evidence_id(
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
                "UPDATE private.jobs SET state='failed',phase='failed',updated_at=?,error=? WHERE id=?",
                params![Utc::now().timestamp(), job.error, id.to_string()],
            )
            .map_err(storage)?;
        Ok(())
    }
    /// Inventory a server-admin configured inbox directory. Reject symlinks and special files.
    pub fn inventory_directory(source: &Path) -> Result<Vec<UploadFile>> {
        fn walk(root: &Path, path: &Path, out: &mut Vec<UploadFile>, depth: usize) -> Result<()> {
            if depth > 32 || out.len() > 4096 {
                return Err(Error::Invalid(
                    "package is too deeply nested or contains too many files".into(),
                ));
            }
            let metadata = fs::symlink_metadata(path).map_err(storage)?;
            if metadata.file_type().is_symlink() {
                return Err(Error::Invalid(
                    "inbox packages cannot contain symlinks".into(),
                ));
            }
            if metadata.is_dir() {
                for entry in fs::read_dir(path).map_err(storage)? {
                    walk(root, &entry.map_err(storage)?.path(), out, depth + 1)?;
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
        walk(source, source, &mut files, 0)?;
        files.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(files)
    }
}
