//! Offline portable archive export/import mechanisms; policy lives in collector-app.
use super::*;
use crate::backup::{
    copy_file, create_private_dir, empty_root, prospective_path, read_bounded_json, regular_file,
    root_lock, scan_tree, sync_tree_directories, write_private,
};
use collector_app::{ArchiveExportInventory, BackupFile};
use std::collections::BTreeSet;

const INVENTORY: &str = "archive-export.v1.json";
const COMPLETE: &str = "complete.sha256";
const RESTORING: &str = ".restore-in-progress";
const INVENTORY_LIMIT: u64 = 64 * 1024 * 1024;

impl Store {
    /// The caller owns an offline Store without workers, as for full backup.
    pub fn export_archive(&self, destination: &Path) -> Result<()> {
        let _writer = lock(&self.writer)?;
        let _gate = lock(&self.job_publication_gate)?;
        let destination = prospective_path(&std::path::absolute(destination).map_err(storage)?)?;
        if destination.starts_with(&self.archive) || destination.starts_with(&self.state) {
            return Err(Error::Invalid(
                "export must be outside storage roots".into(),
            ));
        }
        for name in [".transactions", ".staging"] {
            let path = self.archive.join(name);
            if !fs::symlink_metadata(&path).map_err(storage)?.is_dir()
                || fs::read_dir(path).map_err(storage)?.next().is_some()
            {
                return Err(Error::Unavailable(
                    "unsettled archive scratch; inspect recovery before export".into(),
                ));
            }
        }
        let (files, directories) = scan_tree(&self.archive, "archive")?;
        let mut inventory = ArchiveExportInventory {
            version: 1,
            files: vec![],
            directories: directories
                .into_iter()
                .filter(|path| ArchiveExportInventory::includes(path))
                .collect(),
        };
        create_private_dir(&destination)?;
        for directory in &inventory.directories {
            create_private_dir(&destination.join(directory))?;
        }
        for path in files
            .into_iter()
            .filter(|path| ArchiveExportInventory::includes(path))
        {
            let local = path.strip_prefix("archive/").ok_or(Error::Conflict)?;
            let (bytes, digest) =
                copy_file(&self.archive.join(local), &destination.join(&path), None)?;
            inventory.files.push(BackupFile {
                path,
                bytes,
                digest,
            });
        }
        inventory.validate()?;
        let data = serde_json::to_vec(&inventory).map_err(storage)?;
        if data.len() as u64 > INVENTORY_LIMIT {
            return Err(Error::Invalid("export inventory exceeds 64 MiB".into()));
        }
        write_private(&destination.join(INVENTORY), &data)?;
        verify_archive_tree(&destination.join("archive"), &inventory, false)?;
        sync_tree_directories(&destination, &inventory.directories)?;
        self.publication_fault
            .check(PublicationFaultPoint::BytesStaged)?;
        write_private(
            &destination.join(COMPLETE),
            Digest::sha256(&data).hex.as_bytes(),
        )?;
        sync_directory(&destination)?;
        sync_directory(destination.parent().ok_or(Error::Conflict)?)?;
        validate_archive_export(&destination).map(|_| ())
    }
}

fn verify_archive_tree(
    archive: &Path,
    inventory: &ArchiveExportInventory,
    importing: bool,
) -> Result<()> {
    let (mut files, directories) = scan_tree(archive, "archive")?;
    if importing {
        files.remove("archive/.writer.lock");
        files.remove("archive/.restore-in-progress");
    }
    let expected = inventory
        .files
        .iter()
        .map(|file| file.path.clone())
        .collect::<BTreeSet<_>>();
    let mut expected_directories = inventory.directories.clone();
    expected_directories.sort();
    if files != expected || directories != expected_directories {
        return Err(Error::Invalid(
            "archive export has missing or unlisted entries".into(),
        ));
    }
    for file in &inventory.files {
        let path = archive.join(file.path.strip_prefix("archive/").ok_or(Error::Conflict)?);
        regular_file(&path)?;
        if crate::files::hash_file_checked(&path, file.bytes.0, &mut || Ok(()))?
            != (file.bytes, file.digest.clone())
        {
            return Err(Error::Storage(format!(
                "archive export digest/length mismatch: {}",
                file.path
            )));
        }
    }
    Ok(())
}

/// Read-only byte validation: no database, source path or installed module is opened.
pub fn validate_archive_export(source: &Path) -> Result<ArchiveExportInventory> {
    // Scanning below also refuses a symlink export root and special entries.
    regular_file(&source.join(INVENTORY))?;
    regular_file(&source.join(COMPLETE))?;
    let data = read_bounded_json(&source.join(INVENTORY), INVENTORY_LIMIT)?;
    if read_bounded_json(&source.join(COMPLETE), 128)? != Digest::sha256(&data).hex.as_bytes() {
        return Err(Error::Invalid(
            "archive export is incomplete or its inventory is corrupt".into(),
        ));
    }
    let inventory: ArchiveExportInventory = serde_json::from_slice(&data).map_err(storage)?;
    inventory.validate()?;
    if !fs::symlink_metadata(source).map_err(storage)?.is_dir() {
        return Err(Error::Invalid(
            "archive export must be a real directory".into(),
        ));
    }
    for entry in fs::read_dir(source).map_err(storage)? {
        if !matches!(
            entry.map_err(storage)?.file_name().to_str(),
            Some("archive" | INVENTORY | COMPLETE)
        ) {
            return Err(Error::Invalid(
                "archive export has unlisted top-level entries".into(),
            ));
        }
    }
    verify_archive_tree(&source.join("archive"), &inventory, false)?;
    Ok(inventory)
}

/// Copies a portable export into an empty archive. Creates no private state or config.
pub fn import_archive(source: &Path, archive: &Path) -> Result<()> {
    import_with_fault(source, archive, &publication::NoPublicationFault)
}

fn import_with_fault(
    source: &Path,
    archive: &Path,
    fault: &dyn PublicationFaultInjector,
) -> Result<()> {
    let inventory = validate_archive_export(source)?;
    let source = source.canonicalize().map_err(storage)?;
    let archive = prospective_path(&std::path::absolute(archive).map_err(storage)?)?;
    if archive.starts_with(&source) || source.starts_with(&archive) {
        return Err(Error::Invalid(
            "export and imported archive must be separate".into(),
        ));
    }
    let archive = empty_root(&archive)?;
    let _archive = root_lock(&archive, ".writer.lock")?;
    write_private(
        &archive.join(RESTORING),
        b"Incomplete archive import. Retry into a newly empty archive.\n",
    )?;
    sync_directory(&archive)?;
    sync_directory(archive.parent().ok_or(Error::Conflict)?)?;
    fault.check(PublicationFaultPoint::Prepared)?;
    let local = |path: &str| -> Result<PathBuf> {
        Ok(archive.join(path.strip_prefix("archive/").ok_or(Error::Conflict)?))
    };
    let mut directories = inventory.directories.clone();
    directories.sort();
    for directory in &directories {
        if directory != "archive" {
            create_private_dir(&local(directory)?)?;
        }
    }
    for file in &inventory.files {
        if copy_file(
            &source.join(&file.path),
            &local(&file.path)?,
            Some(file.bytes),
        )? != (file.bytes, file.digest.clone())
        {
            return Err(Error::Storage("export changed during import".into()));
        }
        fault.check(PublicationFaultPoint::BytesStaged)?;
    }
    for directory in directories.iter().rev() {
        let path = if directory == "archive" {
            archive.clone()
        } else {
            local(directory)?
        };
        sync_directory(&path)?;
    }
    verify_archive_tree(&archive, &inventory, true)?;
    fault.check(PublicationFaultPoint::RevisionCommitted)?;
    fs::remove_file(archive.join(RESTORING)).map_err(storage)?;
    sync_directory(&archive)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FailAt(PublicationFaultPoint);
    impl PublicationFaultInjector for FailAt {
        fn check(&self, point: PublicationFaultPoint) -> Result<()> {
            if point == self.0 {
                Err(Error::Storage("interrupted archive copy".into()))
            } else {
                Ok(())
            }
        }
    }

    #[test]
    fn archive_export_interruption_never_seals_a_partial_copy_or_overlays_a_retry() {
        let root = tempfile::tempdir().unwrap();
        let store = Store::open_with_publication_fault(
            root.path().join("archive"),
            root.path().join("state"),
            Arc::new(FailAt(PublicationFaultPoint::BytesStaged)),
        )
        .unwrap();
        fs::write(store.archive.join("unknown.bin"), b"unknown portable data").unwrap();
        let failed = root.path().join("failed");
        assert!(store.export_archive(&failed).is_err());
        assert!(!failed.join(COMPLETE).exists());
        assert!(validate_archive_export(&failed).is_err());
        drop(store);
        let store = Store::open(root.path().join("archive"), root.path().join("state")).unwrap();
        assert!(store.export_archive(&failed).is_err());
        let export = root.path().join("complete");
        store.export_archive(&export).unwrap();
        for (index, phase) in [
            PublicationFaultPoint::Prepared,
            PublicationFaultPoint::BytesStaged,
            PublicationFaultPoint::RevisionCommitted,
        ]
        .into_iter()
        .enumerate()
        {
            let archive = root.path().join(format!("interrupted-{index}"));
            assert!(import_with_fault(&export, &archive, &FailAt(phase)).is_err());
            assert!(archive.join(RESTORING).is_file());
            let state = root.path().join(format!("state-{index}"));
            let error = Store::open(&archive, &state).err().unwrap().to_string();
            assert!(error.contains("incomplete restore"), "{error}");
            assert!(!state.join("private.sqlite").exists());
            assert!(import_archive(&export, &archive).is_err());
        }
        let archive = root.path().join("retry");
        import_archive(&export, &archive).unwrap();
        assert_eq!(
            fs::read(archive.join("unknown.bin")).unwrap(),
            b"unknown portable data"
        );
        Store::open(&archive, root.path().join("retry-state")).unwrap();
        validate_archive_export(&export).unwrap();
    }

    #[test]
    fn archive_export_validation_refuses_changes_unsafe_entries_and_operational_state() {
        let root = tempfile::tempdir().unwrap();
        let store = Store::open(root.path().join("archive"), root.path().join("state")).unwrap();
        fs::write(store.archive.join("unknown.bin"), b"original").unwrap();
        let export = root.path().join("export");
        store.export_archive(&export).unwrap();
        let inventory = fs::read(export.join(INVENTORY)).unwrap();
        let seal = fs::read(export.join(COMPLETE)).unwrap();
        for path in [
            "../escape",
            "archive/../escape",
            "state/private.sqlite",
            "archive/.writer.lock",
            "archive/.staging/secret",
            "archive/a\\b",
            "archive/a:b",
            "archive/a\nb",
        ] {
            let mut changed: ArchiveExportInventory = serde_json::from_slice(&inventory).unwrap();
            changed.files[0].path = path.into();
            let data = serde_json::to_vec(&changed).unwrap();
            fs::write(export.join(INVENTORY), &data).unwrap();
            fs::write(export.join(COMPLETE), Digest::sha256(&data).hex).unwrap();
            let archive = root.path().join("invalid-import");
            assert!(import_archive(&export, &archive).is_err(), "{path:?}");
            assert!(!archive.exists());
        }
        fs::write(export.join(INVENTORY), &inventory).unwrap();
        fs::write(export.join(COMPLETE), &seal).unwrap();
        fs::write(export.join("archive/unknown.bin"), b"modified").unwrap();
        assert!(validate_archive_export(&export).is_err());
        fs::remove_file(export.join("archive/unknown.bin")).unwrap();
        assert!(validate_archive_export(&export).is_err());
        fs::write(export.join("archive/unknown.bin"), b"original").unwrap();
        for path in ["archive/unlisted", "private.sqlite"] {
            fs::write(export.join(path), b"unlisted data").unwrap();
            assert!(validate_archive_export(&export).is_err());
            fs::remove_file(export.join(path)).unwrap();
        }
        validate_archive_export(&export).unwrap();
        assert_eq!(fs::read(export.join(INVENTORY)).unwrap(), inventory);
        assert_eq!(fs::read(export.join(COMPLETE)).unwrap(), seal);
        // An export never silently drops unresolved recovery inputs.
        let scratch = store.archive.join(".staging/unknown");
        fs::write(&scratch, b"needs operator inspection").unwrap();
        let refused = root.path().join("unsettled");
        assert!(store.export_archive(&refused).is_err());
        assert!(!refused.exists());
        assert_eq!(fs::read(scratch).unwrap(), b"needs operator inspection");
    }

    #[test]
    fn archive_import_rechecks_bytes_changed_after_initial_validation() {
        struct ChangeAfterValidation(PathBuf);
        impl PublicationFaultInjector for ChangeAfterValidation {
            fn check(&self, point: PublicationFaultPoint) -> Result<()> {
                if point == PublicationFaultPoint::Prepared {
                    fs::write(&self.0, b"modified").map_err(storage)?;
                }
                Ok(())
            }
        }
        let root = tempfile::tempdir().unwrap();
        let store = Store::open(root.path().join("source"), root.path().join("state")).unwrap();
        fs::write(store.archive.join("unknown.bin"), b"original").unwrap();
        let export = root.path().join("export");
        store.export_archive(&export).unwrap();
        let archive = root.path().join("imported");
        let error = import_with_fault(
            &export,
            &archive,
            &ChangeAfterValidation(export.join("archive/unknown.bin")),
        )
        .unwrap_err();
        assert!(
            error.to_string().contains("changed during import"),
            "{error}"
        );
        assert!(archive.join(RESTORING).is_file());
    }

    #[cfg(unix)]
    #[test]
    fn archive_exports_reject_symlinks_aliases_and_populated_destinations() {
        use std::os::unix::fs::{PermissionsExt, symlink};
        let root = tempfile::tempdir().unwrap();
        let store = Store::open(root.path().join("source"), root.path().join("state")).unwrap();
        fs::write(store.archive.join("unknown.bin"), b"original").unwrap();
        symlink(&store.state, store.archive.join("private-alias")).unwrap();
        let export = root.path().join("export");
        assert!(store.export_archive(&export).is_err());
        assert!(!export.exists());
        fs::remove_file(store.archive.join("private-alias")).unwrap();
        store.export_archive(&export).unwrap();
        assert_eq!(
            fs::metadata(&export).unwrap().permissions().mode() & 0o777,
            0o700
        );
        assert_eq!(
            fs::metadata(export.join("archive/unknown.bin"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
        let alias = root.path().join("alias");
        symlink(&store.archive, &alias).unwrap();
        assert!(store.export_archive(&alias.join("export")).is_err());
        assert!(import_archive(&export, &alias).is_err());
        assert!(import_archive(&export, &export.join("archive")).is_err());
        let destination = root.path().join("populated");
        fs::create_dir(&destination).unwrap();
        fs::write(destination.join("keep"), b"never overwrite").unwrap();
        assert!(import_archive(&export, &destination).is_err());
        assert_eq!(
            fs::read(destination.join("keep")).unwrap(),
            b"never overwrite"
        );
        let exported = export.join("archive/unknown.bin");
        fs::remove_file(&exported).unwrap();
        symlink(store.archive.join("unknown.bin"), &exported).unwrap();
        assert!(validate_archive_export(&export).is_err());
    }
}
