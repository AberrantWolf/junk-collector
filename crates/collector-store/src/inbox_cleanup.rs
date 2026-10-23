//! Durable, explicitly requested removal of preserved inbox originals.
use super::*;
use collector_app::{
    ImportPhase, ImportRecord, InboxCleanupView, ProcessingPort, ProvisionalImportPort,
};
use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Identity {
    device: u64,
    inode: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct FileStamp {
    identity: Identity,
    bytes: u64,
    modified: i128,
}
#[derive(Clone, Serialize, Deserialize)]
struct Cleanup {
    source: PathBuf,
    parent: Identity,
    directories: BTreeMap<String, Identity>,
    files: BTreeMap<String, FileStamp>,
    moving: bool,
    finalizing: bool,
    deleting: Option<String>,
    removed: BTreeSet<String>,
    completed: bool,
    retained: bool,
}

fn identity(metadata: &fs::Metadata) -> Result<Identity> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        Ok(Identity {
            device: metadata.dev(),
            inode: metadata.ino(),
        })
    }
    #[cfg(not(unix))]
    {
        let _ = metadata;
        Err(Error::Unavailable(
            "Inbox cleanup requires Unix filesystem identities.".into(),
        ))
    }
}
fn real_directory(path: &Path) -> Result<Identity> {
    let metadata = fs::symlink_metadata(path).map_err(storage)?;
    if !metadata.is_dir() {
        return Err(Error::Invalid(
            "Inbox directory changed or became a link; files retained.".into(),
        ));
    }
    identity(&metadata)
}
fn stamp(path: &Path) -> Result<FileStamp> {
    let metadata = fs::symlink_metadata(path).map_err(storage)?;
    if !metadata.is_file() {
        return Err(Error::Invalid(
            "Inbox file changed or became a link; files retained.".into(),
        ));
    }
    let modified = metadata
        .modified()
        .map_err(storage)?
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(storage)?
        .as_nanos();
    Ok(FileStamp {
        identity: identity(&metadata)?,
        bytes: metadata.len(),
        modified: modified.try_into().map_err(storage)?,
    })
}
fn overlaps(left: &Path, right: &Path) -> bool {
    left.starts_with(right) || right.starts_with(left)
}
fn quarantine(id: ImportId, record: &Cleanup) -> Result<PathBuf> {
    Ok(record
        .source
        .parent()
        .ok_or(Error::Conflict)?
        .join(format!(".collector-cleanup-{id}")))
}
fn saved(db: &Connection, id: ImportId) -> Result<Option<Cleanup>> {
    db.query_row(
        "SELECT record FROM private.inbox_cleanup WHERE import_id=?",
        [id.to_string()],
        |row| row.get::<_, String>(0),
    )
    .optional()
    .map_err(storage)?
    .map(|raw| serde_json::from_str(&raw).map_err(storage))
    .transpose()
}

impl Store {
    pub(crate) fn inbox_source_cleanup_status(
        &self,
        id: ImportId,
    ) -> Result<Option<InboxCleanupView>> {
        let value = {
            let db = lock(&self.db)?;
            let Some(record) = saved(&db, id)? else {
                return Ok(None);
            };
            let job: String = db
                .query_row(
                    "SELECT job_id FROM private.inbox_cleanup WHERE import_id=?",
                    [id.to_string()],
                    |row| row.get(0),
                )
                .map_err(storage)?;
            (record, job.parse().map_err(storage)?)
        };
        let import = self.import_record(id)?;
        let job = self.job_record(value.1, import.user_id)?;
        let removed = import
            .view
            .files
            .iter()
            .filter(|file| value.0.removed.contains(&file.name))
            .map(|file| file.bytes.0)
            .sum();
        Ok(Some(InboxCleanupView {
            job_id: job.id,
            state: job.state,
            removed_bytes: ByteLength(removed),
            total_bytes: ByteLength(import.view.files.iter().map(|f| f.bytes.0).sum()),
            completed: value.0.completed,
            retained_files: value.0.retained,
            recovery_directory: value.0.moving.then(|| {
                format!(
                    "{}/.collector-cleanup-{id}/package",
                    import.view.source.label.as_deref().unwrap_or("inbox")
                )
            }),
            error: job.error,
        }))
    }

    pub(crate) fn queue_inbox_source_cleanup(
        &self,
        id: ImportId,
        source: String,
        mut spec: JobSpec,
    ) -> Result<JobId> {
        let previous = saved(&*lock(&self.db)?, id)?;
        let record = if let Some(previous) = previous.filter(|value| value.moving) {
            previous
        } else {
            let path = PathBuf::from(source);
            let root = real_directory(&path)?;
            let path = path.canonicalize().map_err(storage)?;
            if overlaps(&path, &self.archive) || overlaps(&path, &self.state) {
                return Err(Error::Invalid(
                    "Inbox cleanup must be outside archive and private storage.".into(),
                ));
            }
            let import = self.import_record(id)?;
            let names = import
                .view
                .files
                .iter()
                .map(|file| file.name.clone())
                .collect::<Vec<_>>();
            let current = Self::inventory_directory_retaining(&path, &names)?;
            if current.len() != names.len()
                || current
                    .iter()
                    .zip(&import.view.files)
                    .any(|(left, right)| left.name != right.name || left.bytes != right.bytes)
            {
                return Err(Error::Invalid(
                    "Inbox inventory differs from this import; nothing removed.".into(),
                ));
            }
            let mut directories = BTreeMap::from([(String::new(), root)]);
            let mut files = BTreeMap::new();
            let mut budget = 0usize;
            for file in &import.view.files {
                crate::files::safe_name(&file.name)?;
                charge_observation(&mut budget, &file.name)?;
                let mut parent = Path::new(&file.name).parent();
                while let Some(relative) = parent.filter(|path| !path.as_os_str().is_empty()) {
                    let name = relative.to_str().ok_or(Error::Conflict)?;
                    if !directories.contains_key(name) {
                        charge_observation(&mut budget, name)?;
                    }
                    directories.insert(
                        relative.to_str().ok_or(Error::Conflict)?.to_owned(),
                        real_directory(&path.join(relative))?,
                    );
                    parent = relative.parent();
                }
                let observed = stamp(&path.join(&file.name))?;
                if observed.bytes != file.bytes.0 {
                    return Err(Error::Conflict);
                }
                files.insert(file.name.clone(), observed);
            }
            Cleanup {
                parent: real_directory(path.parent().ok_or(Error::Conflict)?)?,
                source: path,
                directories,
                files,
                moving: false,
                finalizing: false,
                deleting: None,
                removed: BTreeSet::new(),
                completed: false,
                retained: false,
            }
        };
        spec.request["inbox_cleanup"] = serde_json::to_value(record).map_err(storage)?;
        self.enqueue_job_record(spec)
    }

    // Inbox admission and cleanup are serialized, including aliases/nested roots.
    pub(crate) fn ensure_inbox_acceptance(db: &Connection, record: &ImportRecord) -> Result<()> {
        if record.view.source.kind != collector_app::ImportSourceKind::Inbox {
            return Ok(());
        }
        let claimed: bool=db.query_row("SELECT EXISTS(SELECT 1 FROM private.inbox_cleanup c JOIN private.jobs j ON j.id=c.job_id WHERE json_extract(c.record,'$.completed')=0 AND j.state IN ('queued','running','cancel_requested'))",[],|row|row.get(0)).map_err(storage)?;
        if claimed {
            return Err(Error::Unavailable("Inbox cleanup is in progress. Finish or cancel it before starting another inbox import.".into()));
        }
        Ok(())
    }

    /// Runs inside the same immediate transaction as Job admission and receipt.
    pub(crate) fn admit_inbox_cleanup(
        &self,
        tx: &rusqlite::Transaction<'_>,
        spec: &JobSpec,
    ) -> Result<()> {
        if let Some(_source) = spec
            .request
            .get("source_locator")
            .or_else(|| spec.request.get("inbox_source_locator"))
            .and_then(serde_json::Value::as_str)
        {
            let mut query = tx.prepare("SELECT c.source_path FROM private.inbox_cleanup c JOIN private.jobs j ON j.id=c.job_id WHERE json_extract(c.record,'$.completed')=0 AND j.state IN ('queued','running','cancel_requested')").map_err(storage)?;
            let claims = query
                .query_map([], |row| row.get::<_, String>(0))
                .map_err(storage)?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(storage)?;
            if !claims.is_empty() {
                return Err(Error::Unavailable("Inbox cleanup is in progress. Finish or cancel it before copying another inbox package.".into()));
            }
        }
        if spec.kind != "inbox_cleanup" {
            return Ok(());
        }
        let id: ImportId = spec
            .request
            .get("import_id")
            .and_then(serde_json::Value::as_str)
            .ok_or(Error::Conflict)?
            .parse()
            .map_err(storage)?;
        let record: Cleanup = serde_json::from_value(
            spec.request
                .get("inbox_cleanup")
                .cloned()
                .ok_or(Error::Conflict)?,
        )
        .map_err(storage)?;
        let eligible: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM private.imports i JOIN private.guided_ingest_intents g ON g.import_id=i.id WHERE i.id=? AND i.user_id=? AND i.source_kind='inbox' AND i.phase='published' AND g.settled=1 AND g.cancelled=0 AND NOT EXISTS(SELECT 1 FROM private.jobs j WHERE json_extract(j.request,'$.import_id')=i.id AND j.kind<>'inbox_cleanup' AND j.state IN ('queued','running','cancel_requested')))",params![id.to_string(),spec.user_id.to_string()],|row|row.get(0)).map_err(storage)?;
        if !eligible {
            return Err(Error::Conflict);
        }
        Self::ensure_inbox_imports_settled(tx, id)?;
        Self::ensure_inbox_source_idle(tx, &record.source, Some(spec.id))?;
        let claims: Vec<(String, String)> = {
            let mut query = tx.prepare("SELECT c.import_id,c.source_path FROM private.inbox_cleanup c JOIN private.jobs j ON j.id=c.job_id WHERE json_extract(c.record,'$.completed')=0 AND j.state IN ('queued','running','cancel_requested')").map_err(storage)?;
            query
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
                .map_err(storage)?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(storage)?
        };
        if claims.iter().any(|(other, _)| other != &id.to_string()) {
            return Err(Error::Conflict);
        }
        if let Some(old) = saved(tx, id)? {
            let job: String = tx
                .query_row(
                    "SELECT job_id FROM private.inbox_cleanup WHERE import_id=?",
                    [id.to_string()],
                    |row| row.get(0),
                )
                .map_err(storage)?;
            let active: bool = tx.query_row("SELECT state IN ('queued','running','cancel_requested') FROM private.jobs WHERE id=?",[&job],|row|row.get(0)).map_err(storage)?;
            if old.completed
                || active
                || (old.moving
                    && serde_json::to_value(&old).map_err(storage)?
                        != serde_json::to_value(&record).map_err(storage)?)
            {
                return Err(Error::Conflict);
            }
        }
        tx.execute("INSERT INTO private.inbox_cleanup(import_id,source_path,job_id,record) VALUES(?,?,?,?) ON CONFLICT(import_id) DO UPDATE SET source_path=excluded.source_path,job_id=excluded.job_id,record=excluded.record",params![id.to_string(),record.source.to_str().ok_or(Error::Conflict)?,spec.id.to_string(),serde_json::to_string(&record).map_err(storage)?]).map_err(storage)?;
        Ok(())
    }

    fn ensure_inbox_imports_settled(db: &Connection, id: ImportId) -> Result<()> {
        let pending: bool=db.query_row("SELECT EXISTS(SELECT 1 FROM private.imports WHERE id<>? AND source_kind='inbox' AND phase NOT IN ('published','cancelled'))",[id.to_string()],|row|row.get(0)).map_err(storage)?;
        if pending {
            return Err(Error::Unavailable(
                "Finish or discard unfinished inbox imports before starting cleanup.".into(),
            ));
        }
        Ok(())
    }

    fn ensure_inbox_source_idle(db: &Connection, _source: &Path, own: Option<JobId>) -> Result<()> {
        let mut query = db.prepare("SELECT id,request FROM private.jobs WHERE state IN ('queued','running','cancel_requested') AND kind IN ('import_staging','preservation_import')").map_err(storage)?;
        let jobs = query
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(storage)?;
        for job in jobs {
            let (id, request) = job.map_err(storage)?;
            if own.is_some_and(|own| id == own.to_string()) {
                continue;
            }
            let request: serde_json::Value = serde_json::from_str(&request).map_err(storage)?;
            if request
                .get("source_locator")
                .or_else(|| request.get("inbox_source_locator"))
                .and_then(serde_json::Value::as_str)
                .is_some()
            {
                return Err(Error::Unavailable(
                    "Another inbox copy is active; finish it before cleanup.".into(),
                ));
            }
        }
        Ok(())
    }

    fn save_cleanup(&self, id: ImportId, claim: &JobClaim, record: &Cleanup) -> Result<()> {
        self.cleanup_effect(claim, |tx| {
            let changed = tx
                .execute(
                    "UPDATE private.inbox_cleanup SET record=? WHERE import_id=? AND job_id=?",
                    params![
                        serde_json::to_string(record).map_err(storage)?,
                        id.to_string(),
                        claim.id.to_string()
                    ],
                )
                .map_err(storage)?;
            if changed != 1 {
                return Err(Error::Conflict);
            }
            Ok(())
        })
    }

    fn cleanup_effect<T>(
        &self,
        claim: &JobClaim,
        action: impl FnOnce(&rusqlite::Transaction<'_>) -> Result<T>,
    ) -> Result<T> {
        let mut db = lock(&self.db)?;
        let tx = db
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(storage)?;
        let live: Option<bool> = tx.query_row("SELECT state='running' AND cancellation_requested=0 AND NOT EXISTS(SELECT 1 FROM private.work_history h WHERE h.user_id=jobs.user_id AND h.pending=1 AND (h.target='job:'||jobs.id OR h.target='import:'||json_extract(jobs.request,'$.import_id'))) FROM private.jobs WHERE id=? AND claim_token=? AND lease_expires>?",params![claim.id.to_string(),claim.claim_token,self.clock.now().timestamp()],|row|row.get(0)).optional().map_err(storage)?;
        if !live.ok_or(Error::CommandInProgress)? {
            return Err(Error::JobCancelled);
        }
        let role: Option<i64> = tx.query_row("SELECT g.role FROM private.grants g JOIN private.users u ON u.id=g.user_id WHERE g.user_id=? AND g.collection_id=? AND u.enabled=1",params![claim.user_id.to_string(),claim.collection_id.map(|id|id.to_string())],|row|row.get(0)).optional().map_err(storage)?;
        if !role
            .map(role_from_database)
            .transpose()?
            .is_some_and(|role| role.allows(Action::Curate))
        {
            return Err(Error::Forbidden);
        }
        let result = action(&tx)?;
        tx.commit().map_err(storage)?;
        Ok(result)
    }

    pub(crate) fn remove_preserved_inbox_source(
        &self,
        id: ImportId,
        claim: &JobClaim,
        guard: &mut dyn FnMut() -> Result<()>,
    ) -> Result<InboxCleanupView> {
        let import = self.import_record(id)?;
        if import.user_id != claim.user_id
            || Some(import.view.collection_id) != claim.collection_id
            || import.view.phase != ImportPhase::Published
        {
            return Err(Error::Conflict);
        }
        let mut record = saved(&*lock(&self.db)?, id)?.ok_or(Error::NotFound)?;
        if self
            .inbox_source_cleanup_status(id)?
            .ok_or(Error::NotFound)?
            .job_id
            != claim.id
        {
            return Err(Error::Conflict);
        }
        self.ensure_job_continues(claim.id, Some(&claim.claim_token))?;
        if record.completed {
            return self.inbox_source_cleanup_status(id)?.ok_or(Error::NotFound);
        }
        Self::ensure_inbox_imports_settled(&*lock(&self.db)?, id)?;
        Self::ensure_inbox_source_idle(&*lock(&self.db)?, &record.source, Some(claim.id))?;
        let parent = record.source.parent().ok_or(Error::Conflict)?;
        if real_directory(parent)? != record.parent {
            return Err(Error::Conflict);
        }
        let wrapper = quarantine(id, &record)?;
        let held = wrapper.join("package");
        let moved = record.moving && (record.finalizing || held.try_exists().map_err(storage)?);
        let root = if moved {
            held.clone()
        } else {
            record.source.clone()
        };
        check_directories(&record, &root)?;
        let phase = std::cell::Cell::new("checking_inbox_copies");
        let mut checkpoint = || {
            guard()?;
            self.ensure_job_continues(claim.id, Some(&claim.claim_token))?;
            self.heartbeat_job_record(claim.id, &claim.claim_token, phase.get(), 0, 30)
        };
        let capture = self.capture(import.view.capture_id)?;
        let representation = capture
            .representations
            .iter()
            .find(|rep| {
                rep.id == import.view.representation_id
                    && rep.role == RepresentationRole::PreservationMaster
            })
            .ok_or(Error::Conflict)?;
        if representation.files.len() != import.view.files.len() {
            return Err(Error::Conflict);
        }
        let inventory = self.package_inventory(
            import.view.collection_id,
            PackageScope::Representation(representation.id),
        )?;
        for (index, file) in import.view.files.iter().enumerate() {
            checkpoint()?;
            let archived = &representation.files[index];
            let expected = file.digest.as_ref().ok_or(Error::Conflict)?;
            if archived.name != file.name
                || archived.bytes != file.bytes
                || &archived.digest != expected
            {
                return Err(Error::Conflict);
            }
            let mut master = self.package_file(
                import.view.collection_id,
                PackageScope::Representation(representation.id),
                inventory.files[index].id,
            )?;
            if hash_reader(&mut master, file.bytes.0, &mut checkpoint)? != *expected {
                return Err(Error::Storage(
                    "Preserved archive copy failed its checksum; inbox files retained.".into(),
                ));
            }
            if record.removed.contains(&file.name) {
                continue;
            }
            let path = root.join(&file.name);
            if record.deleting.as_deref() == Some(&file.name)
                && !path.try_exists().map_err(storage)?
            {
                record.removed.insert(file.name.clone());
                record.deleting = None;
                self.save_cleanup(id, claim, &record)?;
                continue;
            }
            verify_source(&record, &root, &file.name, expected, &mut checkpoint)?;
        }
        if !moved {
            let names = import
                .view
                .files
                .iter()
                .map(|file| file.name.clone())
                .collect::<Vec<_>>();
            let actual = Self::inventory_directory_retaining(&root, &names)?;
            if actual.len() != names.len()
                || actual
                    .iter()
                    .zip(&import.view.files)
                    .any(|(left, right)| left.name != right.name || left.bytes != right.bytes)
            {
                return Err(Error::Invalid(
                    "Inbox inventory changed; files retained.".into(),
                ));
            }
            checkpoint()?;
            create_wrapper(&wrapper, id, &record)?;
            record.moving = true;
            self.save_cleanup(id, claim, &record)?;
            check_directories(&record, &root)?;
            self.cleanup_effect(claim, |_| {
                check_directories(&record,&root)?;
                if held.try_exists().map_err(storage)? { return Err(Error::Conflict); }
                fs::rename(&root,&held).map_err(|_|Error::Unavailable("Inbox must allow renaming/removing packages. Check its read/write mount and directory permissions; files retained.".into()))?;
                sync_directory(parent)?; sync_directory(&wrapper)
            })?;
            self.publication_fault
                .check(PublicationFaultPoint::PayloadPublished)?;
        }
        check_directories(&record, &held)?;
        phase.set("removing_inbox_files");
        for file in &import.view.files {
            if record.removed.contains(&file.name) {
                continue;
            }
            checkpoint()?;
            verify_source(
                &record,
                &held,
                &file.name,
                file.digest.as_ref().ok_or(Error::Conflict)?,
                &mut checkpoint,
            )?;
            record.deleting = Some(file.name.clone());
            self.save_cleanup(id, claim, &record)?;
            checkpoint()?;
            self.cleanup_effect(claim, |_| {
                check_directories(&record, &held)?;
                if stamp(&held.join(&file.name))? != record.files[&file.name] {
                    return Err(Error::Conflict);
                }
                fs::remove_file(held.join(&file.name)).map_err(storage)?;
                sync_directory(held.join(&file.name).parent().ok_or(Error::Conflict)?)
            })?;
            self.publication_fault
                .check(PublicationFaultPoint::BytesStaged)?;
            record.removed.insert(file.name.clone());
            record.deleting = None;
            self.save_cleanup(id, claim, &record)?;
        }
        record.finalizing = true;
        self.save_cleanup(id, claim, &record)?;
        // Remove only empty recorded directories. Everything else is retained.
        let mut directories = record.directories.keys().collect::<Vec<_>>();
        directories.sort_by_key(|name| std::cmp::Reverse(name.len()));
        for name in directories {
            let path = held.join(name);
            self.cleanup_effect(claim, |_| {
                match fs::symlink_metadata(&path) {
                    Ok(_) if real_directory(&path)? != record.directories[name] => {
                        return Err(Error::Conflict);
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
                    Err(error) => return Err(storage(error)),
                    _ => {}
                }
                match fs::remove_dir(&path) {
                    Ok(()) => sync_directory(path.parent().ok_or(Error::Conflict)?),
                    Err(error)
                        if matches!(
                            error.kind(),
                            std::io::ErrorKind::DirectoryNotEmpty | std::io::ErrorKind::NotFound
                        ) =>
                    {
                        Ok(())
                    }
                    Err(error) => Err(storage(error)),
                }
            })?;
        }
        self.publication_fault
            .check(PublicationFaultPoint::RevisionCommitted)?;
        record.retained = held.try_exists().map_err(storage)?;
        record.completed = true;
        self.save_cleanup(id, claim, &record)?;
        self.inbox_source_cleanup_status(id)?.ok_or(Error::NotFound)
    }
}

fn charge_observation(budget: &mut usize, name: &str) -> Result<()> {
    *budget = budget
        .saturating_add(name.len().saturating_mul(2))
        .saturating_add(256);
    if *budget > 4 * 1024 * 1024 {
        return Err(Error::Invalid("Inbox cleanup observation exceeds 4 MiB; split the package or remove sources manually.".into()));
    }
    Ok(())
}

fn check_directories(record: &Cleanup, root: &Path) -> Result<()> {
    for (name, expected) in &record.directories {
        if record.finalizing && !root.join(name).try_exists().map_err(storage)? {
            continue;
        }
        if real_directory(&root.join(name))? != *expected {
            return Err(Error::Invalid(
                "Inbox directory identity changed; remaining files retained.".into(),
            ));
        }
    }
    Ok(())
}
fn verify_source(
    record: &Cleanup,
    root: &Path,
    name: &str,
    digest: &Digest,
    checkpoint: &mut dyn FnMut() -> Result<()>,
) -> Result<()> {
    check_directories(record, root)?;
    let expected = record.files.get(name).ok_or(Error::Conflict)?;
    let path = root.join(name);
    if stamp(&path)? != *expected {
        return Err(Error::Invalid(
            "Inbox file changed since cleanup was requested; remaining files retained.".into(),
        ));
    }
    let mut source = File::open(&path).map_err(storage)?;
    if identity(&source.metadata().map_err(storage)?)? != expected.identity
        || hash_reader(&mut source, expected.bytes, checkpoint)? != *digest
        || stamp(&path)? != *expected
    {
        return Err(Error::Invalid(
            "Inbox file differs from its preserved copy; remaining files retained.".into(),
        ));
    }
    Ok(())
}
fn hash_reader(
    reader: &mut dyn Read,
    expected: u64,
    checkpoint: &mut dyn FnMut() -> Result<()>,
) -> Result<Digest> {
    use sha2::{Digest as _, Sha256};
    let mut hash = Sha256::new();
    let mut completed = 0u64;
    let mut buffer = vec![0u8; 1024 * 1024];
    loop {
        checkpoint()?;
        let read = reader.read(&mut buffer).map_err(storage)?;
        if read == 0 {
            break;
        }
        completed = completed.checked_add(read as u64).ok_or(Error::Conflict)?;
        if completed > expected {
            return Err(Error::Invalid(
                "File exceeds its preserved length; files retained.".into(),
            ));
        }
        hash.update(&buffer[..read]);
    }
    if completed != expected {
        return Err(Error::Invalid("File is truncated; files retained.".into()));
    }
    Ok(Digest {
        algorithm: HashAlgorithm::Sha256,
        hex: hex::encode(hash.finalize()),
    })
}
fn create_wrapper(path: &Path, id: ImportId, record: &Cleanup) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
        match fs::DirBuilder::new().mode(0o700).create(path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(_) => {
                return Err(Error::Unavailable(
                    "Inbox cleanup requires a writable inbox mount and parent directory.".into(),
                ));
            }
        }
        let metadata = fs::symlink_metadata(path).map_err(storage)?;
        if !metadata.is_dir() || metadata.permissions().mode() & 0o077 != 0 {
            return Err(Error::Conflict);
        }
        let marker = path.join("source.json");
        let data =
            serde_json::to_vec(&(id, &record.source, &record.directories[""])).map_err(storage)?;
        if marker.exists() {
            if {
                crate::backup::regular_file(&marker)?;
                crate::backup::read_bounded_json(&marker, 16384)?
            } != data
            {
                return Err(Error::Conflict);
            }
        } else {
            if fs::read_dir(path).map_err(storage)?.next().is_some() {
                return Err(Error::Conflict);
            }
            crate::backup::write_private(&marker, &data)?;
        }
        sync_directory(path)?;
        sync_directory(path.parent().ok_or(Error::Conflict)?)?;
        Ok(())
    }
    #[cfg(not(unix))]
    {
        let _ = (path, id, record);
        Err(Error::Unavailable(
            "Inbox cleanup requires Unix filesystem identities.".into(),
        ))
    }
}
