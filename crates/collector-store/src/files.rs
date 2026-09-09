use super::*;
use sha2::{Digest as _, Sha256};
use std::io::{Read, Seek, SeekFrom};

const CHUNK_LIMIT: usize = 8 * 1024 * 1024;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UploadFile {
    pub name: String,
    pub bytes: ByteLength,
    pub sha256: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Upload {
    pub id: JobId,
    pub user_id: UserId,
    pub item_id: ItemId,
    pub expected_revision: Revision,
    pub representation_id: RepresentationId,
    pub capture_id: CaptureId,
    pub files: Vec<UploadFile>,
    pub offsets: Vec<u64>,
    pub state: JobState,
}
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
impl Store {
    fn upload_directory(&self, id: JobId) -> PathBuf {
        self.state.join("uploads").join(id.to_string())
    }
    fn persist_upload(&self, upload: &Upload) -> Result<()> {
        let dir = self.upload_directory(upload.id);
        let temp = dir.join(format!("{}.tmp", TransactionId::new()));
        write_new(&temp, &serde_json::to_vec_pretty(upload).map_err(storage)?)?;
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
        lock(&self.db)?.execute("INSERT INTO private.jobs VALUES (?,?) ON CONFLICT(id) DO UPDATE SET record=excluded.record",params![job.id.to_string(),serde_json::to_string(&job).map_err(storage)?]).map_err(storage)?;
        Ok(())
    }
    pub fn create_upload(
        &self,
        actor: &Principal,
        item_id: ItemId,
        expected_revision: Revision,
        files: Vec<UploadFile>,
    ) -> Result<Upload> {
        let item = self.item(item_id)?;
        if !self
            .role(actor.user_id, item.collection_id)?
            .allows(Action::Curate)
        {
            return Err(Error::Forbidden);
        }
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
            user_id: actor.user_id,
            item_id,
            expected_revision,
            representation_id: RepresentationId::new(),
            capture_id: CaptureId::new(),
            offsets: vec![0; files.len()],
            files,
            state: JobState::Queued,
        };
        fs::create_dir(self.upload_directory(upload.id)).map_err(storage)?;
        self.persist_upload(&upload)?;
        Ok(upload)
    }
    pub fn upload(&self, actor: &Principal, id: JobId) -> Result<Upload> {
        let path = self.upload_directory(id).join("upload.json");
        let bytes = fs::read(path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                Error::NotFound
            } else {
                storage(e)
            }
        })?;
        let mut upload: Upload = serde_json::from_slice(&bytes).map_err(storage)?;
        if upload.user_id != actor.user_id {
            return Err(Error::Forbidden);
        }
        let item = self.item(upload.item_id)?;
        if !self
            .role(actor.user_id, item.collection_id)?
            .allows(Action::Curate)
        {
            return Err(Error::Forbidden);
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
    pub fn upload_chunk(
        &self,
        actor: &Principal,
        id: JobId,
        index: usize,
        offset: u64,
        bytes: &[u8],
    ) -> Result<Upload> {
        if bytes.len() > CHUNK_LIMIT {
            return Err(Error::Invalid("chunk exceeds 8 MiB".into()));
        }
        let _guard = lock(&self.uploads)?;
        let mut upload = self.upload(actor, id)?;
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
    pub fn cancel_upload(&self, actor: &Principal, id: JobId) -> Result<()> {
        let _guard = lock(&self.uploads)?;
        let mut upload = self.upload(actor, id)?;
        if upload.state == JobState::Completed {
            return Err(Error::Conflict);
        }
        upload.state = JobState::Cancelled;
        self.persist_upload(&upload)
    }
    pub fn commit_upload(&self, actor: &Principal, id: JobId) -> Result<Item> {
        let _guard = lock(&self.uploads)?;
        let mut upload = self.upload(actor, id)?;
        let mut item = self.item(upload.item_id)?;
        if item
            .captures
            .iter()
            .flat_map(|c| &c.representations)
            .any(|r| r.id == upload.representation_id)
        {
            upload.state = JobState::Completed;
            self.persist_upload(&upload)?;
            return Ok(item);
        }
        if upload.state == JobState::Cancelled {
            return Err(Error::Conflict);
        }
        if item.revision != upload.expected_revision {
            return Err(Error::Conflict);
        }
        let total = upload.files.iter().map(|f| f.bytes.0).sum::<u64>();
        if fs2::available_space(&self.archive).map_err(storage)?
            < total.saturating_add(64 * 1024 * 1024)
        {
            return Err(Error::Unavailable("insufficient archive space".into()));
        }
        let stage = self
            .archive
            .join(".staging")
            .join(TransactionId::new().to_string());
        fs::create_dir(&stage).map_err(storage)?;
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
            let target = stage.join(&file.name);
            fs::create_dir_all(target.parent().unwrap()).map_err(storage)?;
            fs::copy(&source, &target).map_err(storage)?;
            File::open(&target)
                .and_then(|f| f.sync_all())
                .map_err(storage)?;
            if hash_file(&target)? != (length, digest.clone()) {
                return Err(Error::Storage("destination verification failed".into()));
            }
            sync_directory(target.parent().unwrap())?;
            files.push(StoredFile {
                name: file.name.clone(),
                bytes: length,
                digest,
            });
        }
        sync_directory(&stage)?;
        let item_root = self.item_path(item.id)?;
        let collection_root = item_root
            .parent()
            .and_then(std::path::Path::parent)
            .ok_or_else(|| Error::Storage("item path has no collection parent".into()))?;
        let package_root = collection_root
            .join("captures")
            .join(upload.capture_id.to_string())
            .join("payload");
        fs::create_dir_all(&package_root).map_err(storage)?;
        let destination = package_root.join(upload.representation_id.to_string());
        if destination.exists() {
            // an earlier attempt published files but not the item revision
            for file in &files {
                if hash_file(&destination.join(&file.name))? != (file.bytes, file.digest.clone()) {
                    return Err(Error::Storage("orphan package differs from retry".into()));
                }
            }
            fs::remove_dir_all(&stage).map_err(storage)?;
        } else {
            fs::rename(&stage, &destination).map_err(storage)?;
        }
        sync_directory(&package_root)?;
        let inputs = files.iter().map(|f| f.digest.clone()).collect();
        item.captures.push(Capture {
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
            legacy_payload_locator: None,
        });
        let evidence = Evidence {
            id: EvidenceId::new(),
            item_id: item.id,
            representation_id: upload.representation_id,
            kind: EvidenceKind::Integrity,
            outcome: EvidenceOutcome::Passed,
            inputs,
            implementation: "collector-store staged copy and destination reread".into(),
            implementation_version: env!("CARGO_PKG_VERSION").into(),
            catalog: None,
            details: serde_json::json!({"claim":"destination bytes equal uploaded bytes; no catalog verification asserted"}),
            created_at: Utc::now(),
        };
        let result = self.publish(item, upload.expected_revision, vec![evidence])?;
        upload.state = JobState::Completed;
        self.persist_upload(&upload)?;
        Ok(result)
    }
    pub fn jobs(&self, actor: &Principal) -> Result<Vec<Job>> {
        let db = lock(&self.db)?;
        let mut stmt = db
            .prepare("SELECT record FROM private.jobs ORDER BY id DESC LIMIT 500")
            .map_err(storage)?;
        let rows = stmt
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(storage)?;
        let mut jobs = Vec::new();
        for row in rows {
            let job: Job = serde_json::from_str(&row.map_err(storage)?).map_err(storage)?;
            if job.user_id == actor.user_id {
                jobs.push(job);
            }
        }
        Ok(jobs)
    }
    pub fn representation_file(
        &self,
        actor: &Principal,
        item_id: ItemId,
        representation: RepresentationId,
        index: usize,
    ) -> Result<(PathBuf, StoredFile)> {
        let item = self.item(item_id)?;
        self.role(actor.user_id, item.collection_id)?;
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
        let root = if let Some(locator) = &capture.legacy_payload_locator {
            if locator.relative_path != "files" {
                return Err(Error::Invalid("unsupported legacy payload locator".into()));
            }
            let original_item = self.item(locator.item_id)?;
            if original_item.collection_id != item.collection_id {
                return Err(Error::Forbidden);
            }
            self.item_path(locator.item_id)?
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
    pub fn add_cover(
        &self,
        actor: &Principal,
        item_id: ItemId,
        expected: Revision,
        bytes: &[u8],
        alt_text: String,
    ) -> Result<Item> {
        let mut item = self.item(item_id)?;
        if !self
            .role(actor.user_id, item.collection_id)?
            .allows(Action::Curate)
        {
            return Err(Error::Forbidden);
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
        let id = AssetId::new();
        let root = self.item_path(item_id)?.join("assets").join(id.to_string());
        fs::create_dir_all(&root).map_err(storage)?;
        let name = format!("original.{extension}");
        write_new(&root.join(&name), bytes)?;
        write_new(&root.join("thumbnail.png"), &thumbnail)?;
        sync_directory(&root)?;
        sync_directory(root.parent().unwrap())?;
        item.assets.push(Asset {
            id,
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
    pub fn asset_file(
        &self,
        actor: &Principal,
        item_id: ItemId,
        asset_id: AssetId,
        thumbnail: bool,
    ) -> Result<(PathBuf, String, Digest)> {
        let item = self.item(item_id)?;
        let role = self.role(actor.user_id, item.collection_id)?;
        if item.deleted {
            return Err(Error::NotFound);
        }
        let asset = item
            .assets
            .iter()
            .find(|a| a.id == asset_id)
            .ok_or(Error::NotFound)?;
        if role == Role::Viewer && asset.visibility == Visibility::Curators {
            return Err(Error::Forbidden);
        }
        let file = if thumbnail {
            asset.thumbnail.as_ref().ok_or(Error::NotFound)?
        } else {
            &asset.file
        };
        safe_name(&file.name)?;
        let root = self
            .item_path(item_id)?
            .join("assets")
            .join(asset_id.to_string());
        let path = fs::canonicalize(root.join(&file.name)).map_err(storage)?;
        if !path.starts_with(fs::canonicalize(root).map_err(storage)?) {
            return Err(Error::Forbidden);
        }
        Ok((
            path,
            if thumbnail {
                "image/png".into()
            } else {
                asset.media_type.clone()
            },
            file.digest.clone(),
        ))
    }
    pub fn verify_integrity(&self, actor: &Principal, item_id: ItemId) -> Result<Item> {
        let item = self.item(item_id)?;
        if !self
            .role(actor.user_id, item.collection_id)?
            .allows(Action::Curate)
        {
            return Err(Error::Forbidden);
        }
        let mut evidence = Vec::new();
        for representation in item.captures.iter().flat_map(|c| &c.representations) {
            let mut passed = true;
            let mut details = Vec::new();
            for (index, file) in representation.files.iter().enumerate() {
                let check = self
                    .representation_file(actor, item.id, representation.id, index)
                    .and_then(|(path, _)| hash_file(&path));
                let matches = check
                    .is_ok_and(|(length, digest)| length == file.bytes && digest == file.digest);
                passed &= matches;
                details.push(serde_json::json!({"file":file.name,"matches":matches}));
            }
            evidence.push(Evidence {
                id: EvidenceId::new(),
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
                created_at: Utc::now(),
            });
        }
        let expected = item.revision;
        self.publish(item, expected, evidence)
    }
}

impl Store {
    /// Record a failed background attempt durably. The source chunks remain available for retry.
    pub fn fail_upload(&self, actor: &Principal, id: JobId, message: &str) -> Result<()> {
        let _guard = lock(&self.uploads)?;
        let mut upload = self.upload(actor, id)?;
        if upload.state == JobState::Completed {
            return Ok(());
        }
        upload.state = JobState::Failed;
        self.persist_upload(&upload)?;
        let job = Job {
            id,
            user_id: actor.user_id,
            item_id: upload.item_id,
            kind: "preservation_import".into(),
            state: JobState::Failed,
            completed_bytes: upload.offsets.iter().sum(),
            total_bytes: upload.files.iter().map(|f| f.bytes.0).sum(),
            error: Some(message.chars().take(512).collect()),
        };
        lock(&self.db)?
            .execute(
                "UPDATE private.jobs SET record=? WHERE id=?",
                params![
                    serde_json::to_string(&job).map_err(storage)?,
                    id.to_string()
                ],
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
    /// Copy a previously inventoried package through the same upload/commit path as browser imports.
    pub fn stage_directory(&self, actor: &Principal, id: JobId, source: &Path) -> Result<Item> {
        let upload = self.upload(actor, id)?;
        let root = fs::canonicalize(source).map_err(storage)?;
        for (index, entry) in upload.files.iter().enumerate() {
            let path = root.join(&entry.name);
            if fs::symlink_metadata(&path)
                .map_err(storage)?
                .file_type()
                .is_symlink()
                || !fs::canonicalize(&path).map_err(storage)?.starts_with(&root)
            {
                return Err(Error::Invalid("inbox package changed while staging".into()));
            }
            let mut file = File::open(path).map_err(storage)?;
            let mut buffer = vec![0u8; 1024 * 1024];
            let mut offset = 0;
            loop {
                let read = file.read(&mut buffer).map_err(storage)?;
                if read == 0 {
                    break;
                }
                self.upload_chunk(actor, id, index, offset, &buffer[..read])?;
                offset += read as u64;
            }
        }
        self.commit_upload(actor, id)
    }
}
