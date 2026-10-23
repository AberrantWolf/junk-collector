//! Offline backup and restore filesystem mechanisms.
use super::*;
use collector_app::{BackupFile, BackupInventory, RestoreScope};
use std::collections::BTreeSet;
use std::io::Read;

const RESTORING: &str = ".restore-in-progress";

impl Store {
    /// The caller owns an offline Store, without application workers.
    pub fn backup(&self, destination: &Path, config: &Path) -> Result<()> {
        use collector_app::{BackupAttemptOutcome, BackupHistory};
        let attempted_at = self.clock.now();
        let mut history = BackupHistory {
            version: 1,
            last_completed_at: read_backup_history(&self.state)
                .ok()
                .flatten()
                .filter(|history| history.is_consistent_at(attempted_at))
                .and_then(|v| v.last_completed_at),
            attempted_at,
            outcome: BackupAttemptOutcome::Started,
        };
        self.write_backup_history(&history)?;
        let result = self.backup_inner(destination, config);
        history.attempted_at = self.clock.now();
        history.outcome = if result.is_ok() {
            BackupAttemptOutcome::Completed
        } else {
            BackupAttemptOutcome::Failed
        };
        if result.is_ok() {
            history.last_completed_at = Some(history.attempted_at);
        }
        if let Err(error) = self.write_backup_history(&history) {
            // A receipt failure cannot invalidate a fully validated backup. The
            // preceding Started receipt leaves a conservative unconfirmed warning.
            tracing::warn!(%error,"could not persist backup outcome; reminder receipt may be unconfirmed");
        }
        result
    }
    fn write_backup_history(&self, history: &collector_app::BackupHistory) -> Result<()> {
        let path = self.state.join("backup-history.v1.json");
        let temporary = self
            .state
            .join(format!(".backup-history-{}.tmp", TransactionId::new()));
        write_private(&temporary, &serde_json::to_vec(history).map_err(storage)?)?;
        fs::rename(&temporary, path).map_err(storage)?;
        sync_directory(&self.state)
    }
    fn backup_inner(&self, destination: &Path, config: &Path) -> Result<()> {
        let destination_path = std::path::absolute(destination).map_err(storage)?;
        let destination = destination_path.as_path();
        let _writer = lock(&self.writer)?;
        let _gate = lock(&self.job_publication_gate)?;
        let parent = destination
            .parent()
            .ok_or(Error::Conflict)?
            .canonicalize()
            .map_err(storage)?;
        if parent.starts_with(&self.archive) || parent.starts_with(&self.state) {
            return Err(Error::Invalid(
                "backup must be outside storage roots".into(),
            ));
        }
        // Store startup settles known journals. Unknown/unsettled journals need inspection.
        if fs::read_dir(self.archive.join(".transactions"))
            .map_err(storage)?
            .next()
            .is_some()
        {
            return Err(Error::Unavailable(
                "unsettled archive journals; inspect recovery before backup".into(),
            ));
        }
        let private_digest = {
            let db = lock(&self.db)?;
            for schema in ["main", "private"] {
                let busy: i64 = db
                    .query_row(
                        &format!("PRAGMA {schema}.wal_checkpoint(TRUNCATE)"),
                        [],
                        |r| r.get(0),
                    )
                    .map_err(storage)?;
                if busy != 0 {
                    return Err(Error::Unavailable("SQLite checkpoint is busy".into()));
                }
            }
            database_digest(&db, "private")?
        };
        fs::create_dir(destination).map_err(storage)?;
        private_directory(destination)?;
        let mut inventory = BackupInventory {
            version: 1,
            archive_root: self.archive.to_str().ok_or(Error::Conflict)?.into(),
            state_root: self.state.to_str().ok_or(Error::Conflict)?.into(),
            files: vec![],
            directories: vec![],
            private_digest,
        };
        for (prefix, source) in [("archive", &self.archive), ("state", &self.state)] {
            let (paths, directories) = scan_tree(source, prefix)?;
            for directory in directories {
                create_private_dir(&destination.join(&directory))?;
                inventory.directories.push(directory);
            }
            for path in paths {
                let local = path
                    .strip_prefix(&format!("{prefix}/"))
                    .ok_or(Error::Conflict)?;
                let target = destination.join(&path);
                let (bytes, digest) = copy_file(&source.join(local), &target, None)?;
                inventory.files.push(BackupFile {
                    path,
                    bytes,
                    digest,
                });
            }
        }
        let (bytes, digest) = copy_file(config, &destination.join("collector.toml"), None)?;
        inventory.files.push(BackupFile {
            path: "collector.toml".into(),
            bytes,
            digest,
        });
        inventory.validate()?;
        let data = serde_json::to_vec(&inventory).map_err(storage)?;
        if data.len() > 64 * 1024 * 1024 {
            return Err(Error::Invalid("backup inventory exceeds 64 MiB".into()));
        }
        write_private(&destination.join("inventory.v1.json"), &data)?;
        verify_inventory(destination, &inventory)?;
        sync_tree_directories(destination, &inventory.directories)?;
        self.publication_fault
            .check(PublicationFaultPoint::BytesStaged)?;
        write_private(
            &destination.join("complete.sha256"),
            Digest::sha256(&data).hex.as_bytes(),
        )?;
        sync_directory(destination)?;
        sync_directory(&parent)?;
        verify_backup(destination)
    }
}

pub(crate) fn regular_file(path: &Path) -> Result<()> {
    if !fs::symlink_metadata(path)
        .map_err(storage)?
        .file_type()
        .is_file()
    {
        return Err(Error::Invalid(
            "backup refuses symlinks and special files".into(),
        ));
    }
    Ok(())
}
pub(crate) fn create_private_dir(path: &Path) -> Result<()> {
    fs::create_dir(path).map_err(storage)?;
    private_directory(path)
}
fn private_file(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(storage)?;
    }
    Ok(())
}
pub(crate) fn write_private(path: &Path, data: &[u8]) -> Result<()> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path).map_err(storage)?;
    file.write_all(data).map_err(storage)?;
    file.sync_all().map_err(storage)
}
pub(crate) fn copy_file(
    source: &Path,
    target: &Path,
    expected: Option<ByteLength>,
) -> Result<(ByteLength, Digest)> {
    use sha2::{Digest as _, Sha256};
    regular_file(source)?;
    let input = File::open(source).map_err(storage)?;
    let length = input.metadata().map_err(storage)?.len();
    if expected.is_some_and(|expected| expected.0 != length) {
        return Err(Error::Storage("backup file length changed".into()));
    }
    let mut input = std::io::Read::take(input, length.saturating_add(1));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut output = options.open(target).map_err(storage)?;
    let mut hash = Sha256::new();
    let mut copied = 0u64;
    let mut buffer = vec![0u8; 1024 * 1024];
    loop {
        let read = input.read(&mut buffer).map_err(storage)?;
        if read == 0 {
            break;
        }
        copied = copied.saturating_add(read as u64);
        if copied > length {
            return Err(Error::Storage("file changed during backup/restore".into()));
        }
        hash.update(&buffer[..read]);
        output.write_all(&buffer[..read]).map_err(storage)?;
    }
    if copied != length {
        return Err(Error::Storage("file changed during backup/restore".into()));
    }
    output.sync_all().map_err(storage)?;
    let digest = Digest {
        algorithm: HashAlgorithm::Sha256,
        hex: hex::encode(hash.finalize()),
    };
    if crate::files::hash_file_checked(target, length, &mut || Ok(()))?
        != (ByteLength(length), digest.clone())
    {
        return Err(Error::Storage(
            "copied bytes differ from the source stream".into(),
        ));
    }
    Ok((ByteLength(length), digest))
}
pub(crate) fn scan_tree(root: &Path, prefix: &str) -> Result<(BTreeSet<String>, Vec<String>)> {
    if !fs::symlink_metadata(root)
        .map_err(storage)?
        .file_type()
        .is_dir()
    {
        return Err(Error::Invalid(
            "backup roots must be real directories".into(),
        ));
    }
    let mut files = BTreeSet::new();
    let mut directories = vec![prefix.to_owned()];
    let mut pending = vec![(root.to_owned(), prefix.to_owned())];
    let mut metadata_bytes = prefix.len();
    while let Some((directory, relative)) = pending.pop() {
        for entry in fs::read_dir(directory).map_err(storage)? {
            let entry = entry.map_err(storage)?;
            let kind = entry.file_type().map_err(storage)?;
            let name = entry.file_name();
            let name = name.to_str().ok_or(Error::Conflict)?;
            let path = format!("{relative}/{name}");
            metadata_bytes = metadata_bytes
                .saturating_add(path.len())
                .saturating_add(200);
            if metadata_bytes > 64 * 1024 * 1024 {
                return Err(Error::Invalid(
                    "backup tree exceeds the 64 MiB inventory budget".into(),
                ));
            }
            if kind.is_dir() {
                if path.split('/').count() > 256 {
                    return Err(Error::Invalid(
                        "backup directory nesting exceeds 256".into(),
                    ));
                }
                directories.push(path.clone());
                pending.push((entry.path(), path));
            } else if kind.is_file() {
                files.insert(path);
            } else {
                return Err(Error::Invalid(
                    "backup refuses symlinks and special files".into(),
                ));
            }
        }
    }
    directories.sort();
    Ok((files, directories))
}
pub(crate) fn sync_tree_directories(root: &Path, directories: &[String]) -> Result<()> {
    for directory in directories.iter().rev() {
        sync_directory(&root.join(directory))?;
    }
    sync_directory(root)
}
pub(crate) fn read_bounded_json(path: &Path, limit: u64) -> Result<Vec<u8>> {
    let mut file = File::open(path).map_err(storage)?;
    if file.metadata().map_err(storage)?.len() > limit {
        return Err(Error::Invalid("metadata exceeds limit".into()));
    }
    let mut data = Vec::new();
    (&mut file)
        .take(limit + 1)
        .read_to_end(&mut data)
        .map_err(storage)?;
    if data.len() as u64 > limit {
        return Err(Error::Invalid("metadata exceeds limit".into()));
    }
    Ok(data)
}

pub(crate) fn backup_inventory(backup: &Path) -> Result<BackupInventory> {
    regular_file(&backup.join("inventory.v1.json"))?;
    regular_file(&backup.join("complete.sha256"))?;
    let data = read_bounded_json(&backup.join("inventory.v1.json"), 64 * 1024 * 1024)?;
    if read_bounded_json(&backup.join("complete.sha256"), 128)?
        != Digest::sha256(&data).hex.as_bytes()
    {
        return Err(Error::Invalid(
            "backup is incomplete or its inventory is corrupt".into(),
        ));
    }
    let inventory: BackupInventory = serde_json::from_slice(&data).map_err(storage)?;
    inventory.validate()?;
    let archive = Path::new(&inventory.archive_root);
    let state = Path::new(&inventory.state_root);
    if !archive.is_absolute()
        || !state.is_absolute()
        || archive.starts_with(state)
        || state.starts_with(archive)
    {
        return Err(Error::Invalid("invalid original backup roots".into()));
    }
    Ok(inventory)
}
fn verify_inventory(backup: &Path, inventory: &BackupInventory) -> Result<()> {
    let mut actual = BTreeSet::from(["collector.toml".to_owned()]);
    let mut directories = vec![];
    for prefix in ["archive", "state"] {
        let (files, dirs) = scan_tree(&backup.join(prefix), prefix)?;
        actual.extend(files);
        directories.extend(dirs);
    }
    let expected = inventory
        .files
        .iter()
        .map(|f| f.path.clone())
        .collect::<BTreeSet<_>>();
    directories.sort();
    let mut expected_dirs = inventory.directories.clone();
    expected_dirs.sort();
    if actual != expected || (!expected_dirs.is_empty() && directories != expected_dirs) {
        return Err(Error::Invalid(
            "backup has missing or unlisted entries".into(),
        ));
    }
    for entry in fs::read_dir(backup).map_err(storage)? {
        let entry = entry.map_err(storage)?;
        if !matches!(
            entry.file_name().to_str(),
            Some("archive" | "state" | "collector.toml" | "inventory.v1.json" | "complete.sha256")
        ) {
            return Err(Error::Invalid(
                "backup has unlisted top-level entries".into(),
            ));
        }
    }
    for entry in &inventory.files {
        let path = backup.join(&entry.path);
        regular_file(&path)?;
        if crate::files::hash_file(&path)? != (entry.bytes, entry.digest.clone()) {
            return Err(Error::Storage(format!(
                "backup digest/length mismatch: {}",
                entry.path
            )));
        }
    }
    Ok(())
}
/// Read-only byte validation. In particular, no SQLite connection is opened.
pub fn validate_backup(backup: &Path) -> Result<BackupInventory> {
    let inventory = backup_inventory(backup)?;
    verify_inventory(backup, &inventory)?;
    Ok(inventory)
}
pub(crate) fn verify_backup(backup: &Path) -> Result<()> {
    validate_backup(backup).map(|_| ())
}

pub(crate) fn root_lock(root: &Path, name: &str) -> Result<ProcessLock> {
    if !fs::symlink_metadata(root)
        .map_err(storage)?
        .file_type()
        .is_dir()
    {
        return Err(Error::Invalid(
            "recovery root must be a real directory".into(),
        ));
    }
    let path = root.join(name);
    if path.exists() {
        regular_file(&path)?;
    }
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .map_err(storage)?;
    lock_root_file(root, file)
}

fn existing_root_lock(root: &Path, name: &str) -> Result<ProcessLock> {
    if !fs::symlink_metadata(root)
        .map_err(storage)?
        .file_type()
        .is_dir()
    {
        return Err(Error::Invalid(
            "recovery root must be a real directory".into(),
        ));
    }
    let path = root.join(name);
    regular_file(&path)?;
    lock_root_file(root, File::open(path).map_err(storage)?)
}

fn lock_root_file(root: &Path, file: File) -> Result<ProcessLock> {
    file.try_lock_exclusive().map_err(|_| {
        Error::Unavailable(format!(
            "storage is in use; stop the server: {}",
            root.display()
        ))
    })?;
    Ok(ProcessLock(file))
}
pub(crate) fn empty_root(path: &Path) -> Result<PathBuf> {
    if !path.exists() {
        create_private_dir(path)?;
    }
    if !fs::symlink_metadata(path)
        .map_err(storage)?
        .file_type()
        .is_dir()
        || fs::read_dir(path).map_err(storage)?.next().is_some()
    {
        return Err(Error::Invalid(
            "restore requires empty real roots; never overlay an installation".into(),
        ));
    }
    path.canonicalize().map_err(storage)
}

pub(crate) fn prospective_path(path: &Path) -> Result<PathBuf> {
    let name = path.file_name().ok_or_else(|| {
        Error::Invalid("select an explicit root or configuration filename".into())
    })?;
    let parent = path
        .parent()
        .ok_or(Error::Conflict)?
        .canonicalize()
        .map_err(storage)?;
    let target = parent.join(name);
    if target.exists()
        && fs::symlink_metadata(&target)
            .map_err(storage)?
            .file_type()
            .is_symlink()
    {
        return Err(Error::Invalid(
            "restore refuses symlink destinations".into(),
        ));
    }
    Ok(target)
}

/// Restore is an offline copy only; callers may open the result after success.
/// The output configuration must already bind the two destination roots.
pub fn restore_backup(
    backup: &Path,
    archive: &Path,
    state: &Path,
    config: &Path,
    configuration: &[u8],
    scope: RestoreScope,
) -> Result<()> {
    restore_with_fault(
        backup,
        archive,
        state,
        config,
        configuration,
        scope,
        &publication::NoPublicationFault,
    )
}

fn restore_with_fault(
    backup: &Path,
    archive: &Path,
    state: &Path,
    config: &Path,
    configuration: &[u8],
    scope: RestoreScope,
    fault: &dyn PublicationFaultInjector,
) -> Result<()> {
    let inventory = validate_backup(backup)?;
    let archive = prospective_path(archive)?;
    let state = prospective_path(state)?;
    let config = prospective_path(config)?;
    if config.exists() {
        return Err(Error::Invalid(
            "restore configuration must not exist".into(),
        ));
    }
    let backup = backup.canonicalize().map_err(storage)?;
    if archive.starts_with(&state)
        || state.starts_with(&archive)
        || archive.starts_with(&backup)
        || state.starts_with(&backup)
        || backup.starts_with(&archive)
        || backup.starts_with(&state)
        || config.starts_with(&archive)
        || config.starts_with(&state)
        || config.starts_with(&backup)
    {
        return Err(Error::Invalid(
            "restore roots, configuration and backup must be separate".into(),
        ));
    }
    let configured: toml::Value =
        toml::from_str(std::str::from_utf8(configuration).map_err(storage)?).map_err(storage)?;
    for (name, path) in [("archive", &archive), ("state", &state)] {
        let bound = configured
            .get(name)
            .and_then(|v| v.as_str())
            .ok_or(Error::Conflict)?;
        if prospective_path(Path::new(bound))? != *path {
            return Err(Error::Invalid(
                "restored configuration points at different storage roots".into(),
            ));
        }
    }
    // Validate both destinations before creating either one.
    for root in [&archive, &state] {
        if root.exists()
            && (!fs::symlink_metadata(root)
                .map_err(storage)?
                .file_type()
                .is_dir()
                || fs::read_dir(root).map_err(storage)?.next().is_some())
        {
            return Err(Error::Invalid(
                "restore requires empty roots; never overlay an installation".into(),
            ));
        }
    }
    // Acquire original locks when this host can see them. Cross-host identity
    // retirement is necessarily an operator responsibility.
    let mut original_locks = vec![];
    for (root, name) in [
        (&inventory.state_root, ".private.lock"),
        (&inventory.archive_root, ".writer.lock"),
    ] {
        let root = Path::new(root);
        if root.exists() && root != archive && root != state {
            original_locks.push(match fs::symlink_metadata(root.join(name)) {
                Ok(_) => existing_root_lock(root, name)?,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    root_lock(root, name)?
                }
                Err(error) => return Err(storage(error)),
            });
        }
    }
    let _backup_private = existing_root_lock(&backup.join("state"), ".private.lock")?;
    let _backup_archive = existing_root_lock(&backup.join("archive"), ".writer.lock")?;
    let archive = empty_root(&archive)?;
    let state = empty_root(&state)?;
    let _private = root_lock(&state, ".private.lock")?;
    let _archive = root_lock(&archive, ".writer.lock")?;
    for root in [&archive, &state] {
        private_directory(root)?;
        write_private(
            &root.join(RESTORING),
            b"Incomplete restore: do not open these roots. Retry into newly empty roots.\n",
        )?;
        sync_directory(root)?;
    }
    fault.check(PublicationFaultPoint::Prepared)?;
    let target_for = |path: &str| -> Result<PathBuf> {
        if let Some(local) = path.strip_prefix("archive/") {
            Ok(archive.join(local))
        } else if let Some(local) = path.strip_prefix("state/") {
            Ok(state.join(local))
        } else {
            Err(Error::Conflict)
        }
    };
    let mut directories = BTreeSet::new();
    for directory in &inventory.directories {
        if scope.includes(directory) {
            directories.insert(directory.clone());
        }
    }
    // Older conversion backups did not inventory empty directories. Preserve
    // those present in the validated backup as well as recorded file parents.
    if inventory.directories.is_empty() {
        for prefix in ["archive", "state"] {
            directories.extend(
                scan_tree(&backup.join(prefix), prefix)?
                    .1
                    .into_iter()
                    .filter(|path| scope.includes(path)),
            );
        }
    }
    for file in &inventory.files {
        if scope.includes(&file.path) {
            let mut parent = Path::new(&file.path).parent();
            while let Some(path) = parent.filter(|p| p.components().count() > 1) {
                directories.insert(path.to_str().ok_or(Error::Conflict)?.to_owned());
                parent = path.parent();
            }
        }
    }
    for directory in &directories {
        create_private_dir(&target_for(directory)?)?;
    }
    for file in &inventory.files {
        if !scope.includes(&file.path) {
            continue;
        }
        let target = target_for(&file.path)?;
        if matches!(
            file.path.as_str(),
            "archive/.writer.lock" | "state/.private.lock"
        ) {
            // Keep the inode carrying the destination lock, never replace it.
            regular_file(&backup.join(&file.path))?;
            let mut input = File::open(backup.join(&file.path)).map_err(storage)?;
            let mut output = OpenOptions::new()
                .write(true)
                .truncate(true)
                .open(&target)
                .map_err(storage)?;
            std::io::copy(&mut input, &mut output).map_err(storage)?;
            private_file(&target)?;
            output.sync_all().map_err(storage)?;
            if crate::files::hash_file(&target)? != (file.bytes, file.digest.clone()) {
                return Err(Error::Conflict);
            }
            continue;
        }
        if copy_file(&backup.join(&file.path), &target, Some(file.bytes))?
            != (file.bytes, file.digest.clone())
        {
            return Err(Error::Storage("backup changed during restore".into()));
        }
        fault.check(PublicationFaultPoint::BytesStaged)?;
    }
    for directory in directories.iter().rev() {
        sync_directory(&target_for(directory)?)?;
    }
    let mut actual_files = BTreeSet::new();
    let mut actual_directories = BTreeSet::new();
    for (prefix, root) in [("archive", &archive), ("state", &state)] {
        let (mut files, dirs) = scan_tree(root, prefix)?;
        files.remove(&format!("{prefix}/{RESTORING}"));
        actual_files.extend(files);
        actual_directories.extend(dirs);
    }
    let mut expected_files = inventory
        .files
        .iter()
        .filter(|file| scope.includes(&file.path))
        .map(|file| file.path.clone())
        .collect::<BTreeSet<_>>();
    expected_files.extend(["archive/.writer.lock".into(), "state/.private.lock".into()]);
    let mut expected_directories = directories;
    expected_directories.extend(["archive".into(), "state".into()]);
    if actual_files != expected_files || actual_directories != expected_directories {
        return Err(Error::Conflict);
    }
    for root in [&archive, &state] {
        sync_directory(root)?;
        sync_directory(root.parent().ok_or(Error::Conflict)?)?;
    }
    write_private(&config, configuration)?;
    sync_directory(config.parent().ok_or(Error::Conflict)?)?;
    fault.check(PublicationFaultPoint::RevisionCommitted)?;
    for root in [&archive, &state] {
        fs::remove_file(root.join(RESTORING)).map_err(storage)?;
        sync_directory(root)?;
    }
    Ok(())
}
fn stable_backup_file(path: &str) -> bool {
    path != "state/backup-history.v1.json"
        && !Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| {
                n.ends_with(".sqlite")
                    || n.ends_with("-wal")
                    || n.ends_with("-shm")
                    || n.ends_with(".lock")
            })
}

pub(crate) fn verify_backup_roots(store: &Store, backup: &Path) -> Result<()> {
    let inventory = backup_inventory(backup)?;
    if Path::new(&inventory.archive_root) != store.archive.canonicalize().map_err(storage)?
        || Path::new(&inventory.state_root) != store.state.canonicalize().map_err(storage)?
    {
        return Err(Error::Conflict);
    }
    {
        let db = lock(&store.db)?;
        if database_digest(&db, "private")? != inventory.private_digest {
            return Err(Error::Conflict);
        }
    }
    let (mut current, _) = scan_tree(&store.archive, "archive")?;
    current.extend(scan_tree(&store.state, "state")?.0);
    let expected = inventory
        .files
        .iter()
        .filter(|entry| entry.path.starts_with("archive/") || entry.path.starts_with("state/"))
        .map(|entry| entry.path.clone())
        .collect::<std::collections::BTreeSet<_>>();
    if current
        .iter()
        .filter(|p| stable_backup_file(p))
        .collect::<Vec<_>>()
        != expected
            .iter()
            .filter(|p| stable_backup_file(p))
            .collect::<Vec<_>>()
    {
        return Err(Error::Conflict);
    }
    for entry in inventory.files {
        let source = if let Some(path) = entry.path.strip_prefix("archive/") {
            store.archive.join(path)
        } else if let Some(path) = entry.path.strip_prefix("state/") {
            store.state.join(path)
        } else {
            continue;
        };
        // Live completion receipts and SQLite/lock mechanisms may change after
        // capture. The backup's own receipt is still fully verified above; this
        // exception never covers archive records or unknown private files.
        if !stable_backup_file(&entry.path) {
            continue;
        }
        if crate::files::hash_file(&source)? != (entry.bytes, entry.digest) {
            return Err(Error::Conflict);
        }
    }
    Ok(())
}

fn database_digest(db: &Connection, schema: &str) -> Result<String> {
    use sha2::{Digest as _, Sha256};
    let mut hash = Sha256::new();
    let mut tables=db.prepare(&format!("SELECT name FROM {schema}.sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name")).map_err(storage)?;
    let names = tables
        .query_map([], |r| r.get::<_, String>(0))
        .map_err(storage)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(storage)?;
    for name in names {
        hash.update((name.len() as u64).to_be_bytes());
        hash.update(name.as_bytes());
        let mut stmt = db
            .prepare(&format!(
                "SELECT * FROM {schema}.\"{}\"",
                name.replace('"', "\"\"")
            ))
            .map_err(storage)?;
        let columns = stmt.column_count();
        let mut rows = stmt.query([]).map_err(storage)?;
        let mut encoded_rows = Vec::new();
        while let Some(row) = rows.next().map_err(storage)? {
            let mut encoded = Vec::new();
            for i in 0..columns {
                use rusqlite::types::ValueRef;
                let value = row.get_ref(i).map_err(storage)?;
                let (tag, bytes) = match value {
                    ValueRef::Null => (0, vec![]),
                    ValueRef::Integer(v) => (1, v.to_be_bytes().to_vec()),
                    ValueRef::Real(v) => (2, v.to_bits().to_be_bytes().to_vec()),
                    ValueRef::Text(v) => (3, v.to_vec()),
                    ValueRef::Blob(v) => (4, v.to_vec()),
                };
                encoded.push(tag);
                encoded.extend_from_slice(&(bytes.len() as u64).to_be_bytes());
                encoded.extend_from_slice(&bytes);
            }
            encoded_rows.push(sha2::Sha256::digest(&encoded).to_vec());
        }
        encoded_rows.sort();
        for row in encoded_rows {
            hash.update(row);
        }
    }
    Ok(hex::encode(hash.finalize()))
}

impl collector_app::BackupHistoryPort for Store {
    fn backup_history(&self) -> Result<Option<collector_app::BackupHistory>> {
        read_backup_history(&self.state)
    }
}

/// Read one bounded operational receipt, without opening databases or scanning backups.
pub fn read_backup_history(state: &Path) -> Result<Option<collector_app::BackupHistory>> {
    let path = state.join("backup-history.v1.json");
    match fs::symlink_metadata(&path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(storage(error)),
        Ok(metadata) if !metadata.file_type().is_file() || metadata.len() > 4096 => {
            return Err(Error::Invalid(
                "backup history is not a bounded regular file".into(),
            ));
        }
        Ok(_) => {}
    }
    let mut data = Vec::new();
    File::open(path)
        .map_err(storage)?
        .take(4097)
        .read_to_end(&mut data)
        .map_err(storage)?;
    if data.len() > 4096 {
        return Err(Error::Invalid("backup history exceeds its bound".into()));
    }
    serde_json::from_slice(&data).map(Some).map_err(storage)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FailAt(PublicationFaultPoint);
    impl PublicationFaultInjector for FailAt {
        fn check(&self, point: PublicationFaultPoint) -> Result<()> {
            if point == self.0 {
                Err(Error::Storage("interrupted recovery drill".into()))
            } else {
                Ok(())
            }
        }
    }
    fn config(archive: &Path, state: &Path) -> Vec<u8> {
        format!("archive = {archive:?}\nstate = {state:?}\n").into_bytes()
    }
    #[test]
    fn interrupted_backup_has_no_completion_marker_and_can_be_retried_separately() {
        let root = tempfile::tempdir().unwrap();
        let archive = root.path().join("archive");
        let state = root.path().join("state");
        let config_path = root.path().join("collector.toml");
        fs::write(&config_path, config(&archive, &state)).unwrap();
        let store = Store::open_with_publication_fault(
            &archive,
            &state,
            Arc::new(FailAt(PublicationFaultPoint::BytesStaged)),
        )
        .unwrap();
        let failed = root.path().join("incomplete");
        assert!(store.backup(&failed, &config_path).is_err());
        assert!(!failed.join("complete.sha256").exists());
        assert!(validate_backup(&failed).is_err());
        drop(store);
        let store = Store::open(&archive, &state).unwrap();
        let backup = root.path().join("complete");
        store.backup(&backup, &config_path).unwrap();
        validate_backup(&backup).unwrap();
        assert!(store.backup(&failed, &config_path).is_err());
    }
    #[test]
    fn interrupted_restores_are_fenced_before_database_open_and_retry_uses_empty_roots() {
        let root = tempfile::tempdir().unwrap();
        let archive = root.path().join("archive");
        let state = root.path().join("state");
        let config_path = root.path().join("collector.toml");
        fs::write(&config_path, config(&archive, &state)).unwrap();
        let store = Store::open(&archive, &state).unwrap();
        let backup = root.path().join("backup");
        store.backup(&backup, &config_path).unwrap();
        drop(store);
        for (i, point) in [
            PublicationFaultPoint::Prepared,
            PublicationFaultPoint::BytesStaged,
            PublicationFaultPoint::RevisionCommitted,
        ]
        .into_iter()
        .enumerate()
        {
            let a = root.path().join(format!("archive-{i}"));
            let s = root.path().join(format!("state-{i}"));
            let c = root.path().join(format!("config-{i}.toml"));
            assert!(
                restore_with_fault(
                    &backup,
                    &a,
                    &s,
                    &c,
                    &config(&a, &s),
                    RestoreScope::Full,
                    &FailAt(point)
                )
                .is_err()
            );
            assert!(a.join(RESTORING).exists());
            assert!(s.join(RESTORING).exists());
            // A database too new to open proves the restore guard runs before SQLite.
            fs::write(
                s.join("private.sqlite"),
                b"must never be opened or migrated",
            )
            .unwrap();
            let local_error = Store::local_accounts(&s, None, 10).unwrap_err().to_string();
            assert!(local_error.contains("incomplete restore"), "{local_error}");
            let error = Store::open(&a, &s).err().unwrap().to_string();
            assert!(error.contains("incomplete restore"), "{error}");
            assert_eq!(
                fs::read(s.join("private.sqlite")).unwrap(),
                b"must never be opened or migrated"
            );
            assert!(
                restore_backup(&backup, &a, &s, &c, &config(&a, &s), RestoreScope::Full).is_err()
            );
        }
        let a = root.path().join("retry-archive");
        let s = root.path().join("retry-state");
        let c = root.path().join("retry.toml");
        restore_backup(&backup, &a, &s, &c, &config(&a, &s), RestoreScope::Full).unwrap();
        Store::open(&a, &s).unwrap();
    }
    #[test]
    fn either_original_root_lock_and_destination_aliases_refuse_restore() {
        let root = tempfile::tempdir().unwrap();
        let archive = root.path().join("archive");
        let state = root.path().join("state");
        let config_path = root.path().join("collector.toml");
        fs::write(&config_path, config(&archive, &state)).unwrap();
        let store = Store::open(&archive, &state).unwrap();
        let backup = root.path().join("backup");
        store.backup(&backup, &config_path).unwrap();
        drop(store);
        let a = root.path().join("new-archive");
        let s = root.path().join("new-state");
        let c = root.path().join("new.toml");
        for (path, name) in [(&archive, ".writer.lock"), (&state, ".private.lock")] {
            let guard = root_lock(path, name).unwrap();
            let error = restore_backup(&backup, &a, &s, &c, &config(&a, &s), RestoreScope::Full)
                .unwrap_err();
            assert!(error.to_string().contains("in use"));
            assert!(!a.exists() && !s.exists() && !c.exists());
            drop(guard);
        }
        fs::create_dir(&a).unwrap();
        fs::write(a.join("keep"), b"never overwrite").unwrap();
        assert!(restore_backup(&backup, &a, &s, &c, &config(&a, &s), RestoreScope::Full).is_err());
        assert_eq!(fs::read(a.join("keep")).unwrap(), b"never overwrite");
        assert!(!s.exists());
        assert!(
            restore_backup(
                &backup,
                &backup.join("archive"),
                &s,
                &c,
                &config(&backup.join("archive"), &s),
                RestoreScope::Full
            )
            .is_err()
        );
        #[cfg(unix)]
        {
            let alias = root.path().join("alias");
            std::os::unix::fs::symlink(&archive, &alias).unwrap();
            assert!(
                restore_backup(
                    &backup,
                    &alias,
                    &s,
                    &c,
                    &config(&alias, &s),
                    RestoreScope::Full
                )
                .is_err()
            );
        }
    }
    #[test]
    fn validation_is_read_only_and_rejects_extra_missing_corrupt_and_unsafe_entries() {
        let root = tempfile::tempdir().unwrap();
        let store = Store::open(root.path().join("archive"), root.path().join("state")).unwrap();
        let config = root.path().join("collector.toml");
        fs::write(&config, "configuration").unwrap();
        let backup = root.path().join("backup");
        store.backup(&backup, &config).unwrap();
        drop(store);
        let before = scan_tree(&backup, "backup").unwrap();
        let before_digests = before
            .0
            .iter()
            .map(|path| {
                (
                    path.clone(),
                    crate::files::hash_file(&root.path().join(path)).unwrap(),
                )
            })
            .collect::<Vec<_>>();
        validate_backup(&backup).unwrap();
        assert_eq!(scan_tree(&backup, "backup").unwrap(), before);
        for (path, digest) in before_digests {
            assert_eq!(
                crate::files::hash_file(&root.path().join(path)).unwrap(),
                digest
            );
        }
        fs::write(backup.join("archive/unlisted"), b"extra").unwrap();
        assert!(validate_backup(&backup).is_err());
        fs::remove_file(backup.join("archive/unlisted")).unwrap();
        fs::create_dir(backup.join("archive/empty-extra")).unwrap();
        assert!(validate_backup(&backup).is_err());
        fs::remove_dir(backup.join("archive/empty-extra")).unwrap();
        let original = fs::read(backup.join("collector.toml")).unwrap();
        fs::write(backup.join("collector.toml"), b"corrupt").unwrap();
        assert!(validate_backup(&backup).is_err());
        fs::write(backup.join("collector.toml"), &original).unwrap();
        fs::remove_file(backup.join("collector.toml")).unwrap();
        assert!(validate_backup(&backup).is_err());
        fs::write(backup.join("collector.toml"), &original).unwrap();
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(root.path(), backup.join("archive/alias")).unwrap();
            assert!(validate_backup(&backup).is_err());
            fs::remove_file(backup.join("archive/alias")).unwrap();
        }
        let valid_inventory = backup_inventory(&backup).unwrap();
        for bad in [
            "../escape",
            "archive/../../escape",
            "/absolute",
            "state/alias\\escape",
            "state/./escape",
            "archive/C:/escape",
        ] {
            let mut inventory = valid_inventory.clone();
            inventory.files[0].path = bad.into();
            let data = serde_json::to_vec(&inventory).unwrap();
            fs::write(backup.join("inventory.v1.json"), &data).unwrap();
            fs::write(backup.join("complete.sha256"), Digest::sha256(&data).hex).unwrap();
            assert!(validate_backup(&backup).is_err(), "{bad}");
        }
    }
}
