//! Filesystem admission and fail-closed archive identity. No payload hashing.
use super::*;
use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};

const IDENTITY: &str = ".archive-identity";
const BINDING: &str = "archive-binding";

/// Exact source reported by Linux mountinfo, e.g. `nas:/volume1/archive`.
/// Presence requires Linux, an NFS filesystem and local private state.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveMount {
    pub nfs_source: String,
}

pub(crate) struct ArchiveGuard {
    configured: PathBuf,
    identity: String,
    root: File,
    mount: Option<ArchiveMount>,
    unavailable: AtomicBool,
}

fn unavailable(reason: impl std::fmt::Display) -> Error {
    Error::Unavailable(format!(
        "archive storage unavailable: {reason}; stop services, restore the intended mount and restart"
    ))
}

fn identity(path: &Path) -> Result<String> {
    let metadata = fs::symlink_metadata(path).map_err(unavailable)?;
    if !metadata.is_file() || metadata.len() != 37 {
        return Err(unavailable("missing or malformed archive identity"));
    }
    let mut bytes = String::new();
    File::open(path)
        .map_err(unavailable)?
        .take(38)
        .read_to_string(&mut bytes)
        .map_err(unavailable)?;
    uuid::Uuid::parse_str(bytes.trim()).map_err(unavailable)?;
    Ok(bytes)
}

impl ArchiveGuard {
    pub(crate) fn open(
        archive: &Path,
        state: &Path,
        mount: Option<ArchiveMount>,
    ) -> Result<Arc<Self>> {
        check_filesystems(archive, state, mount.as_ref())?;
        let expected = identity(&state.join(BINDING))?;
        if identity(&archive.join(IDENTITY))? != expected {
            return Err(unavailable("archive identity does not match local state"));
        }
        let guard = Arc::new(Self {
            configured: archive.to_owned(),
            identity: expected,
            root: File::open(archive).map_err(unavailable)?,
            mount,
            unavailable: AtomicBool::new(false),
        });
        guard.check()?;
        Ok(guard)
    }

    pub(crate) fn path(&self) -> PathBuf {
        // Resolve all mutations relative to the originally opened directory.
        // An unmounted/replaced pathname cannot redirect them to a local tree.
        #[cfg(target_os = "linux")]
        {
            use std::os::fd::AsRawFd;
            PathBuf::from(format!("/proc/self/fd/{}/.", self.root.as_raw_fd()))
        }
        #[cfg(not(target_os = "linux"))]
        self.configured.clone()
    }

    pub(crate) fn check(&self) -> Result<()> {
        if self.unavailable.load(Ordering::Acquire) {
            return Err(unavailable("this process has latched a storage failure"));
        }
        let result = (|| {
            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;
                let original = self.root.metadata().map_err(unavailable)?;
                let current = fs::metadata(&self.configured).map_err(unavailable)?;
                if (original.dev(), original.ino()) != (current.dev(), current.ino()) {
                    return Err(unavailable("archive mount or root directory changed"));
                }
            }
            for name in ["collections", ".staging", ".transactions"] {
                let metadata = fs::symlink_metadata(self.path().join(name)).map_err(unavailable)?;
                if !metadata.is_dir() {
                    return Err(unavailable("required archive directory missing or aliased"));
                }
                #[cfg(unix)]
                {
                    use std::os::unix::fs::MetadataExt;
                    if metadata.dev() != self.root.metadata().map_err(unavailable)?.dev() {
                        return Err(unavailable(
                            "archive staging/records must use one filesystem",
                        ));
                    }
                }
            }
            if identity(&self.path().join(IDENTITY))? != self.identity {
                return Err(unavailable("archive identity changed"));
            }
            if let Some(mount) = &self.mount {
                check_nfs(&self.configured, mount)?;
            }
            Ok(())
        })();
        if result.is_err() {
            self.unavailable.store(true, Ordering::Release);
        }
        result
    }
}

/// Explicit stopped-server admission, before normal recovery or catalog opening.
/// `adopt` permits existing unmarked data; never call this on automatic startup.
pub(crate) fn enroll(
    archive: &Path,
    state: &Path,
    mount: Option<&ArchiveMount>,
    adopt: bool,
) -> Result<()> {
    for root in [archive, state] {
        ensure_restore_complete(root)?;
    }
    // A configured remote archive must exist on the intended NFS before even mkdir.
    if let Some(mount) = mount {
        check_filesystems(archive, state, Some(mount))?;
    }
    let binding = state.join(BINDING);
    if binding.try_exists().map_err(unavailable)? {
        ArchiveGuard::open(archive, state, mount.cloned())?.check()?;
        return Ok(());
    }
    if !adopt {
        // Existing private state is never treated as fresh, even if the NAS is empty.
        if state
            .join("private.sqlite")
            .try_exists()
            .map_err(unavailable)?
        {
            return Err(unavailable(
                "installation needs explicit enroll-archive; setup cannot replace its archive",
            ));
        }
        if archive.try_exists().map_err(unavailable)? {
            for entry in fs::read_dir(archive).map_err(unavailable)? {
                let name = entry.map_err(unavailable)?.file_name();
                if ![IDENTITY, ".writer.lock"]
                    .iter()
                    .any(|allowed| name == *allowed)
                {
                    return Err(unavailable(
                        "nonempty archive needs explicit enroll-archive",
                    ));
                }
            }
        }
    } else if !archive.join("collections").is_dir() {
        return Err(unavailable(
            "enrollment requires the existing collections directory",
        ));
    }
    fs::create_dir_all(state).map_err(storage)?;
    fs::create_dir_all(archive).map_err(storage)?;
    let canonical_archive = archive.canonicalize().map_err(storage)?;
    let canonical_state = state.canonicalize().map_err(storage)?;
    if canonical_archive.starts_with(&canonical_state)
        || canonical_state.starts_with(&canonical_archive)
    {
        return Err(Error::Invalid(
            "archive and state must be separate directories".into(),
        ));
    }
    private_directory(state)?;
    for root in [archive, state] {
        ensure_restore_complete(root)?;
    }
    check_filesystems(archive, state, mount)?;
    // Pin admission writes too. Recheck the opened object after mount inspection,
    // so even enrollment cannot fall through to an unmounted underlying tree.
    let opened_root = File::open(&canonical_archive).map_err(unavailable)?;
    if let Some(mount) = mount {
        check_nfs(&canonical_archive, mount)?;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let opened = opened_root.metadata().map_err(unavailable)?;
        let current = fs::metadata(&canonical_archive).map_err(unavailable)?;
        if (opened.dev(), opened.ino()) != (current.dev(), current.ino()) {
            return Err(unavailable("archive changed during enrollment"));
        }
    }
    #[cfg(target_os = "linux")]
    let anchored = {
        use std::os::fd::AsRawFd;
        PathBuf::from(format!("/proc/self/fd/{}/.", opened_root.as_raw_fd()))
    };
    #[cfg(not(target_os = "linux"))]
    let anchored = canonical_archive;
    let archive = anchored.as_path();
    let marker = archive.join(IDENTITY);
    // Serialize with server, offline tools and other enrollment attempts.
    let _private = crate::backup::root_lock(state, ".private.lock")?;
    let _writer = crate::backup::root_lock(archive, ".writer.lock")?;
    for name in ["collections", ".staging", ".transactions"] {
        let directory = archive.join(name);
        fs::create_dir_all(&directory).map_err(storage)?;
        if !fs::symlink_metadata(&directory)
            .map_err(unavailable)?
            .is_dir()
        {
            return Err(unavailable(
                "required archive directories missing or aliased",
            ));
        }
    }
    let value = if marker.try_exists().map_err(unavailable)? {
        identity(&marker)?
    } else {
        let value = format!("{}\n", uuid::Uuid::now_v7());
        write_new(&marker, value.as_bytes())?;
        sync_directory(archive)?;
        value
    };
    if binding.try_exists().map_err(unavailable)? {
        if identity(&binding)? != value {
            return Err(unavailable("concurrent enrollment differs"));
        }
    } else {
        write_new(&binding, value.as_bytes())?;
        sync_directory(state)?;
    }
    Ok(())
}

pub(crate) fn check_filesystems(
    archive: &Path,
    state: &Path,
    mount: Option<&ArchiveMount>,
) -> Result<()> {
    #[cfg(not(target_os = "linux"))]
    let _ = state;
    #[cfg(target_os = "linux")]
    {
        let table = fs::read_to_string("/proc/self/mountinfo").map_err(unavailable)?;
        let intended_state = std::path::absolute(state).map_err(unavailable)?;
        let state = nearest_existing(state)?;
        let filesystem = mounted_filesystem(&state, &table)?;
        for line in table.lines() {
            let fields: Vec<_> = line.split_whitespace().collect();
            if fields.len() < 6 {
                return Err(unavailable("malformed mountinfo"));
            }
            let target = mount_path(fields[4]);
            if target != intended_state && target.starts_with(&intended_state) {
                return Err(unavailable("nested private-state mounts are unsupported"));
            }
        }
        if matches!(
            filesystem.kind.as_str(),
            "nfs" | "nfs4" | "cifs" | "smb3" | "fuse.sshfs"
        ) {
            return Err(unavailable("private state/SQLite must be on local disk"));
        }
    }
    if let Some(mount) = mount {
        check_nfs(archive, mount)?;
    }
    validate_private_layout(state)?;
    Ok(())
}

fn validate_private_layout(state: &Path) -> Result<()> {
    let root = match fs::metadata(state) {
        Ok(root) => root,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(unavailable(error)),
    };
    for (name, directory) in [
        ("private.sqlite", false),
        ("catalog.sqlite", false),
        ("private.sqlite-wal", false),
        ("private.sqlite-shm", false),
        ("catalog.sqlite-wal", false),
        ("catalog.sqlite-shm", false),
        ("uploads", true),
        ("imports", true),
        ("derivative-scratch", true),
        ("asset-publication", true),
        ("catalog-uploads", true),
        ("tls", true),
    ] {
        let metadata = match fs::symlink_metadata(state.join(name)) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(unavailable(error)),
        };
        if (directory && !metadata.is_dir()) || (!directory && !metadata.is_file()) {
            return Err(unavailable(
                "private databases/scratch must not be aliases or special files",
            ));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            if metadata.dev() != root.dev() {
                return Err(unavailable(
                    "private state must stay on its local filesystem",
                ));
            }
        }
    }
    #[cfg(not(unix))]
    let _ = root;
    Ok(())
}

#[cfg(target_os = "linux")]
fn nearest_existing(path: &Path) -> Result<PathBuf> {
    let absolute = std::path::absolute(path).map_err(unavailable)?;
    let mut current = absolute.as_path();
    loop {
        if current.try_exists().map_err(unavailable)? {
            return current.canonicalize().map_err(unavailable);
        }
        current = current
            .parent()
            .ok_or_else(|| unavailable("state has no existing parent"))?;
    }
}

fn check_nfs(archive: &Path, expected: &ArchiveMount) -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        let table = fs::read_to_string("/proc/self/mountinfo").map_err(unavailable)?;
        let archive = archive.canonicalize().map_err(unavailable)?;
        validate_nfs(&archive, expected, &table)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (archive, expected);
        Err(unavailable("remote archive enforcement requires Linux"))
    }
}

#[cfg(any(target_os = "linux", test))]
fn validate_nfs(archive: &Path, expected: &ArchiveMount, table: &str) -> Result<()> {
    let filesystem = mounted_filesystem(archive, table)?;
    let options: Vec<_> = filesystem.options.split(',').collect();
    if expected.nfs_source.is_empty()
        || !matches!(filesystem.kind.as_str(), "nfs" | "nfs4")
        || filesystem.source != expected.nfs_source
        || !options.contains(&"hard")
        || options.contains(&"soft")
        || options.contains(&"softerr")
        || !options
            .iter()
            .any(|value| matches!(*value, "vers=4.1" | "vers=4.2"))
        || !filesystem.writable
    {
        return Err(unavailable("expected NFS filesystem/source is not mounted"));
    }
    // No nested mount may redirect archive-local staging/publication or records.
    for line in table.lines() {
        let fields: Vec<_> = line.split_whitespace().collect();
        if fields.len() < 6 {
            return Err(unavailable("malformed mountinfo"));
        }
        let target = mount_path(fields[4]);
        if target != archive && target.starts_with(archive) {
            return Err(unavailable("nested archive mounts are unsupported"));
        }
    }
    Ok(())
}

#[cfg(any(target_os = "linux", test))]
fn mount_path(value: &str) -> PathBuf {
    value
        .replace("\\040", " ")
        .replace("\\011", "\t")
        .replace("\\012", "\n")
        .replace("\\134", "\\")
        .into()
}

#[cfg(any(target_os = "linux", test))]
struct MountedFilesystem {
    target: PathBuf,
    kind: String,
    source: String,
    options: String,
    writable: bool,
}

#[cfg(any(target_os = "linux", test))]
fn mounted_filesystem(path: &Path, table: &str) -> Result<MountedFilesystem> {
    let mut selected: Option<MountedFilesystem> = None;
    for line in table.lines() {
        let fields: Vec<_> = line.split_whitespace().collect();
        let split = fields
            .iter()
            .position(|field| *field == "-")
            .ok_or_else(|| unavailable("malformed mountinfo"))?;
        if split < 6 || fields.len() < split + 4 {
            return Err(unavailable("malformed mountinfo"));
        }
        let target = mount_path(fields[4]);
        if path.starts_with(&target)
            && selected
                .as_ref()
                .is_none_or(|old| target.components().count() >= old.target.components().count())
        {
            selected = Some(MountedFilesystem {
                target,
                kind: fields[split + 1].into(),
                source: mount_path(fields[split + 2]).to_string_lossy().into(),
                options: fields[split + 3].into(),
                writable: fields[5].split(',').any(|option| option == "rw"),
            });
        }
    }
    selected.ok_or_else(|| unavailable("filesystem absent from mountinfo"))
}

// Fence every publication phase, including continuations after a long copy/encode.
pub(crate) struct GuardedPublicationFault {
    pub inner: Arc<dyn PublicationFaultInjector>,
    pub guard: Arc<ArchiveGuard>,
}
impl PublicationFaultInjector for GuardedPublicationFault {
    fn check(&self, point: PublicationFaultPoint) -> Result<()> {
        self.guard.check()?;
        let result = self.inner.check(point);
        self.guard.check()?;
        result
    }
}

/// Admission for explicit import/restore destinations, before creating anything.
/// The destination may be an absent child of the intended mounted filesystem.
pub fn validate_remote_archive_target(
    archive: &Path,
    state: Option<&Path>,
    mount: &ArchiveMount,
) -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        let parent = nearest_existing(archive)?;
        check_nfs(&parent, mount)?;
        if let Some(state) = state {
            check_filesystems(&parent, state, Some(mount))?;
        }
        Ok(())
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = state;
        check_nfs(archive, mount)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nfs_admission_checks_source_type_and_nested_mounts() {
        let source = ArchiveMount {
            nfs_source: "nas:/archive".into(),
        };
        let root = "1 0 0:1 / / rw - ext4 /dev/local rw\n";
        let nfs = "2 1 0:2 / /archive rw - nfs4 nas:/archive rw,hard,vers=4.1\n";
        assert!(validate_nfs(Path::new("/archive"), &source, &format!("{root}{nfs}")).is_ok());
        for table in [
            root.into(),
            format!("{root}{}", nfs.replace("nas:/archive", "other:/archive")),
            format!("{root}{}", nfs.replace("nfs4", "ext4")),
            format!("{root}{nfs}3 2 0:3 / /archive/.staging rw - ext4 /dev/b rw\n"),
            format!("{root}{}", nfs.replace("hard", "soft")),
            format!("{root}{}", nfs.replace("vers=4.1", "vers=3")),
            format!("{root}{}", nfs.replace("/archive rw", "/archive ro")),
            "malformed".into(),
        ] {
            assert!(validate_nfs(Path::new("/archive"), &source, &table).is_err());
        }
    }
}

#[cfg(all(test, target_os = "linux"))]
mod anchored_tests {
    use super::*;
    #[test]
    fn detached_root_cannot_redirect_mutations_into_a_replacement_directory() {
        let root = tempfile::tempdir().unwrap();
        let archive = root.path().join("archive");
        let state = root.path().join("state");
        enroll(&archive, &state, None, false).unwrap();
        let guard = ArchiveGuard::open(&archive, &state, None).unwrap();
        let detached = root.path().join("detached");
        fs::rename(&archive, &detached).unwrap();
        fs::create_dir(&archive).unwrap();
        fs::copy(detached.join(IDENTITY), archive.join(IDENTITY)).unwrap();
        // Even an operation already admitted before pathname replacement stays anchored.
        fs::write(guard.path().join("admitted-before-detach"), b"preserved").unwrap();
        assert!(!archive.join("admitted-before-detach").exists());
        assert_eq!(
            fs::read(detached.join("admitted-before-detach")).unwrap(),
            b"preserved"
        );
        assert!(guard.check().is_err());
        fs::remove_dir_all(&archive).unwrap();
        fs::rename(detached, archive).unwrap();
        assert!(guard.check().is_err());
    }
}
