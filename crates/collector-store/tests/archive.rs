use collector_app::{
    AccessPort, ActiveOwnership, AuthenticatedPrincipal, CarrierDetails, Catalog,
    CollectionService, CommandContext, ComponentDetails, ContentPort, CreateAssetCommand,
    CreateImportCommand, CredentialKind, DisposeSpecimenCommand, IdempotencyPort, ImportCandidate,
    ImportDeclaredFile, ImportIdentityDecision, ImportPhase, ImportPort, ImportRecord,
    ImportReviewDecision, ImportSourceKind, ImportSourceSummary, ImportView, JobPort, JobSpec,
    MediaResource, ProvisionalImportPort, ReceiptDecision, ReturnLoanCommand, ReturnedPossession,
    ReviewImportCommand, SaveItemCommand, SpecimenDetails, StartLoanCommand, UpdateAssetCommand,
};
use collector_core::*;
use collector_modules::*;
use collector_store::{Clock, Store, UploadFile};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

struct TestClock {
    seconds: std::sync::atomic::AtomicI64,
    millis: std::sync::atomic::AtomicU64,
}

struct ArtworkSelectingModule {
    selected: AssetId,
}

struct ImportAnalyzingModule;

impl CollectionModule for ImportAnalyzingModule {
    fn descriptor(&self) -> ModuleDescriptor {
        ModuleDescriptor {
            id: ModuleId("org.example.imports".into()),
            schema_version: 1,
            display_name: "Import analysis fixture".into(),
            capabilities: vec![Capability {
                id: CapabilityId::new("org.example.imports.import-analysis").unwrap(),
                display_name: "Import analysis".into(),
                availability: Availability::Available,
            }],
        }
    }

    fn validate_current(&self, _data: &ModuleData) -> ModuleResult<()> {
        Ok(())
    }

    fn present(&self, _data: &ModuleData) -> ModuleResult<ModulePresentation> {
        Ok(ModulePresentation::default())
    }

    fn operate(
        &self,
        request: OperationRequest,
        _host: &dyn ModuleHost,
    ) -> ModuleResult<OperationOutput> {
        let input: serde_json::Value = request
            .input
            .parse()
            .map_err(|error| ModuleError::Invalid(error.to_string()))?;
        if input["files"][0]["digest"].is_null() {
            return Err(ModuleError::Invalid(
                "analysis received an unstaged file".into(),
            ));
        }
        Ok(OperationOutput {
            value: RawJson::from_json(
                r#"{"candidates":[{"key":"observed-disc","label":"Observed disc","observation":{"unknown":null,"ordered":[2,1],"number":1e+02}}],"limitations":["Fixture candidate is an observation, not verification"]}"#,
            )
            .unwrap(),
        })
    }
}

impl CollectionModule for ArtworkSelectingModule {
    fn descriptor(&self) -> ModuleDescriptor {
        ModuleDescriptor {
            id: ModuleId("org.junk.music".into()),
            schema_version: 1,
            display_name: "Artwork selection fixture".into(),
            capabilities: vec![],
        }
    }

    fn validate_current(&self, _data: &ModuleData) -> ModuleResult<()> {
        Ok(())
    }

    fn present(&self, data: &ModuleData) -> ModuleResult<ModulePresentation> {
        Ok(ModulePresentation {
            module_id: data.module.0.clone(),
            schema_version: data.schema_version,
            available: true,
            artwork: vec![ArtworkReference {
                asset_id: self.selected,
                alt_text: "Module selection".into(),
            }],
            ..Default::default()
        })
    }
}

impl TestClock {
    fn new(seconds: i64) -> Self {
        Self {
            seconds: std::sync::atomic::AtomicI64::new(seconds),
            millis: std::sync::atomic::AtomicU64::new(0),
        }
    }

    fn advance(&self, seconds: i64) {
        self.seconds.fetch_add(seconds, Ordering::SeqCst);
        self.millis
            .fetch_add((seconds as u64) * 1_000, Ordering::SeqCst);
    }
}

impl Clock for TestClock {
    fn now(&self) -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::from_timestamp(self.seconds.load(Ordering::SeqCst), 0).unwrap()
    }

    fn monotonic_millis(&self) -> u64 {
        self.millis.load(Ordering::SeqCst)
    }
}

fn command() -> CommandContext {
    CommandContext::new(TransactionId::new().to_string()).unwrap()
}

fn test_authentication(principal: &Principal) -> AuthenticatedPrincipal {
    AuthenticatedPrincipal {
        principal: principal.clone(),
        credential_kind: CredentialKind::TestLocal,
        authenticated_at: chrono::Utc::now(),
    }
}

fn resource_bytes(mut resource: MediaResource) -> Vec<u8> {
    let mut bytes = Vec::new();
    resource.reader.read_to_end(&mut bytes).unwrap();
    bytes
}

fn find_file_with_bytes(root: &Path, expected: &[u8]) -> Option<PathBuf> {
    for entry in std::fs::read_dir(root).ok()? {
        let path = entry.ok()?.path();
        if path.is_dir() {
            if let Some(found) = find_file_with_bytes(&path, expected) {
                return Some(found);
            }
        } else if std::fs::read(&path).is_ok_and(|bytes| bytes == expected) {
            return Some(path);
        }
    }
    None
}

fn files_named(root: &Path, name: &str) -> Vec<PathBuf> {
    let mut result = Vec::new();
    for entry in std::fs::read_dir(root).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_dir() {
            result.extend(files_named(&entry.path(), name));
        } else if entry.file_name() == name {
            result.push(entry.path());
        }
    }
    result
}

fn copy_tree(source: &Path, target: &Path) {
    std::fs::create_dir_all(target).unwrap();
    for entry in std::fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let destination = target.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &destination);
        } else {
            std::fs::copy(entry.path(), destination).unwrap();
        }
    }
}
fn tree_hashes(root: &Path) -> std::collections::BTreeMap<PathBuf, Digest> {
    fn visit(
        root: &Path,
        directory: &Path,
        values: &mut std::collections::BTreeMap<PathBuf, Digest>,
    ) {
        for entry in std::fs::read_dir(directory).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_dir() {
                visit(root, &entry.path(), values);
            } else {
                values.insert(
                    entry.path().strip_prefix(root).unwrap().to_owned(),
                    Digest::sha256(&std::fs::read(entry.path()).unwrap()),
                );
            }
        }
    }
    let mut values = std::collections::BTreeMap::new();
    visit(root, root, &mut values);
    values
}

struct FailPublicationOnce {
    point: collector_store::PublicationFaultPoint,
    failed: AtomicBool,
}

struct PauseAtRevisionCommit {
    reached: std::sync::Mutex<Option<std::sync::mpsc::Sender<()>>>,
    resume: std::sync::Mutex<std::sync::mpsc::Receiver<()>>,
}

impl collector_store::PublicationFaultInjector for PauseAtRevisionCommit {
    fn check(&self, point: collector_store::PublicationFaultPoint) -> Result<()> {
        if point == collector_store::PublicationFaultPoint::RevisionCommitted
            && let Some(sender) = self.reached.lock().unwrap().take()
        {
            sender.send(()).unwrap();
            self.resume.lock().unwrap().recv().unwrap();
        }
        Ok(())
    }
}

impl collector_store::PublicationFaultInjector for FailPublicationOnce {
    fn check(&self, point: collector_store::PublicationFaultPoint) -> Result<()> {
        if point == self.point && !self.failed.swap(true, Ordering::SeqCst) {
            Err(Error::Unavailable(format!(
                "controlled publication failure at {point:?}"
            )))
        } else {
            Ok(())
        }
    }
}

#[test]
fn entity_fault_at_each_publication_phase_recovers_to_pre_or_post_state() {
    use collector_store::PublicationFaultPoint as Point;
    for point in [
        Point::Prepared,
        Point::BytesStaged,
        Point::PayloadPublished,
        Point::RevisionCommitted,
        Point::ProjectionApplied,
        Point::Completed,
    ] {
        let root = tempfile::tempdir().unwrap();
        let archive = root.path().join("archive");
        let state = root.path().join("state");
        let store = Store::open(&archive, &state).unwrap();
        let user = store
            .create_user("owner", "a long test password", true)
            .unwrap();
        let collection = store
            .create_collection(
                user,
                Collection {
                    id: CollectionId::new(),
                    revision: Revision(1),
                    name: "fault fixture".into(),
                    created_at: chrono::Utc::now(),
                },
            )
            .unwrap();
        drop(store);

        let store = Store::open_with_publication_fault(
            &archive,
            &state,
            Arc::new(FailPublicationOnce {
                point,
                failed: AtomicBool::new(false),
            }),
        )
        .unwrap();
        let candidate = item(collection.id);
        let id = candidate.id;
        assert!(Catalog::save(&store, candidate, Revision(0)).is_err());
        drop(store);

        let reopened = Store::open(&archive, &state).unwrap();
        let committed = matches!(
            point,
            Point::RevisionCommitted | Point::ProjectionApplied | Point::Completed
        );
        if committed {
            assert_eq!(Catalog::item(&reopened, id).unwrap().revision, Revision(1));
        } else {
            assert!(matches!(Catalog::item(&reopened, id), Err(Error::NotFound)));
        }
        assert!(
            std::fs::read_dir(archive.join(".transactions"))
                .unwrap()
                .next()
                .is_none()
        );
    }
}

#[test]
fn targeted_projection_and_full_reindex_are_equivalent() {
    let (_root, store, actor, service, collection) = setup();
    let saved = save_inventory(&service, &actor, item(collection.id), Revision(0)).unwrap();
    let before_item = serde_json::to_value(Catalog::item(&*store, saved.id).unwrap()).unwrap();
    let before_evidence =
        serde_json::to_value(Catalog::evidence(&*store, saved.id).unwrap()).unwrap();
    store.reindex().unwrap();
    assert_eq!(
        serde_json::to_value(Catalog::item(&*store, saved.id).unwrap()).unwrap(),
        before_item
    );
    assert_eq!(
        serde_json::to_value(Catalog::evidence(&*store, saved.id).unwrap()).unwrap(),
        before_evidence
    );
}

fn publication_points() -> [collector_store::PublicationFaultPoint; 6] {
    use collector_store::PublicationFaultPoint as Point;
    [
        Point::Prepared,
        Point::BytesStaged,
        Point::PayloadPublished,
        Point::RevisionCommitted,
        Point::ProjectionApplied,
        Point::Completed,
    ]
}

fn ready_upload(
    store: &Arc<Store>,
    actor: &Principal,
    service: &CollectionService,
    collection: CollectionId,
    bytes: &[u8],
) -> (Item, collector_app::Upload) {
    let saved = save_inventory(service, actor, item(collection), Revision(0)).unwrap();
    let upload = service
        .create_upload(
            actor,
            saved.id,
            saved.revision,
            vec![UploadFile {
                name: "payload.bin".into(),
                bytes: ByteLength(bytes.len() as u64),
                sha256: Some(Digest::sha256(bytes).hex),
            }],
            &command(),
        )
        .unwrap();
    ImportPort::upload_chunk(&**store, upload.id, 0, 0, bytes).unwrap();
    (saved, upload)
}

#[test]
fn upload_fault_at_each_phase_recovers_with_stable_identities() {
    let bytes = b"durable payload fixture";
    for point in publication_points() {
        let (root, store, actor, service, collection) = setup();
        let (saved, upload) = ready_upload(&store, &actor, &service, collection.id, bytes);
        drop(service);
        drop(store);
        let archive = root.path().join("archive");
        let state = root.path().join("state");
        let faulted = Store::open_with_publication_fault(
            &archive,
            &state,
            Arc::new(FailPublicationOnce {
                point,
                failed: AtomicBool::new(false),
            }),
        )
        .unwrap();
        assert!(ImportPort::commit_upload(&faulted, upload.id).is_err());
        drop(faulted);

        let recovered = Store::open(&archive, &state)
            .unwrap_or_else(|error| panic!("recovery after {point:?} failed: {error}"));
        let committed = ImportPort::commit_upload(&recovered, upload.id).unwrap();
        assert_eq!(committed.captures.len(), 1);
        assert_eq!(committed.captures[0].id, upload.capture_id);
        assert_eq!(
            committed.captures[0].representations[0].id,
            upload.representation_id
        );
        assert_eq!(Catalog::evidence(&recovered, saved.id).unwrap().len(), 1);
        assert_eq!(
            resource_bytes(
                ContentPort::representation(&recovered, saved.id, upload.representation_id, 0)
                    .unwrap()
            ),
            bytes
        );
        assert!(
            recovered
                .upload(upload.id)
                .unwrap()
                .state
                .eq(&JobState::Completed)
        );
        assert!(
            state
                .join("uploads")
                .join(upload.id.to_string())
                .join("0.part")
                .exists()
        );
    }
}

struct BlockAtPublicationPoint(collector_store::PublicationFaultPoint);

impl collector_store::PublicationFaultInjector for BlockAtPublicationPoint {
    fn check(&self, point: collector_store::PublicationFaultPoint) -> Result<()> {
        if point == self.0 {
            use std::io::Write as _;
            println!("P01_PHASE:{point:?}");
            std::io::stdout().flush().unwrap();
            loop {
                std::thread::park();
            }
        }
        Ok(())
    }
}

fn parse_publication_point(value: &str) -> collector_store::PublicationFaultPoint {
    use collector_store::PublicationFaultPoint as Point;
    match value {
        "Prepared" => Point::Prepared,
        "BytesStaged" => Point::BytesStaged,
        "PayloadPublished" => Point::PayloadPublished,
        "RevisionCommitted" => Point::RevisionCommitted,
        "ProjectionApplied" => Point::ProjectionApplied,
        "Completed" => Point::Completed,
        _ => panic!("unknown publication point"),
    }
}

#[test]
fn subprocess_upload_commit_worker() {
    let Ok(archive) = std::env::var("P01_CHILD_ARCHIVE") else {
        return;
    };
    let state = std::env::var("P01_CHILD_STATE").unwrap();
    let upload: JobId = std::env::var("P01_CHILD_UPLOAD").unwrap().parse().unwrap();
    let point = parse_publication_point(&std::env::var("P01_CHILD_POINT").unwrap());
    let store = Store::open_with_publication_fault(
        archive,
        state,
        Arc::new(BlockAtPublicationPoint(point)),
    )
    .unwrap();
    let _ = ImportPort::commit_upload(&store, upload);
    panic!("child publication unexpectedly returned");
}

#[test]
fn sigkill_at_each_upload_publication_phase_recovers() {
    use std::io::BufRead as _;
    let bytes = b"subprocess termination payload";
    for point in publication_points() {
        let (root, store, actor, service, collection) = setup();
        let (saved, upload) = ready_upload(&store, &actor, &service, collection.id, bytes);
        drop(service);
        drop(store);
        let archive = root.path().join("archive");
        let state = root.path().join("state");
        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "subprocess_upload_commit_worker", "--nocapture"])
            .env("P01_CHILD_ARCHIVE", &archive)
            .env("P01_CHILD_STATE", &state)
            .env("P01_CHILD_UPLOAD", upload.id.to_string())
            .env("P01_CHILD_POINT", format!("{point:?}"))
            .stdout(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        let mut output = std::io::BufReader::new(child.stdout.take().unwrap());
        let mut line = String::new();
        loop {
            line.clear();
            assert_ne!(
                output.read_line(&mut line).unwrap(),
                0,
                "child exited before phase marker"
            );
            if line.contains("P01_PHASE:") {
                break;
            }
        }
        child.kill().unwrap();
        let status = child.wait().unwrap();
        assert!(!status.success());

        let recovered = Store::open(&archive, &state).unwrap();
        let committed = ImportPort::commit_upload(&recovered, upload.id).unwrap();
        assert_eq!(committed.captures.len(), 1);
        assert_eq!(committed.captures[0].id, upload.capture_id);
        assert_eq!(Catalog::evidence(&recovered, saved.id).unwrap().len(), 1);
        assert_eq!(
            resource_bytes(
                ContentPort::representation(&recovered, saved.id, upload.representation_id, 0)
                    .unwrap()
            ),
            bytes
        );
    }
}

fn collection_recovery_points() -> [collector_store::PublicationFaultPoint; 4] {
    use collector_store::PublicationFaultPoint as Point;
    [
        Point::Prepared,
        Point::RevisionCommitted,
        Point::ProjectionApplied,
        Point::Completed,
    ]
}

#[test]
fn collection_and_owner_grant_recover_as_one_retryable_effect() {
    for point in collection_recovery_points() {
        let root = tempfile::tempdir().unwrap();
        let archive = root.path().join("archive");
        let state = root.path().join("state");
        let store = Store::open(&archive, &state).unwrap();
        let user = store
            .create_user("owner", "a long test password", true)
            .unwrap();
        drop(store);
        let actor = Principal {
            user_id: user,
            administrator: true,
            device_id: None,
        };
        let faulted = Arc::new(
            Store::open_with_publication_fault(
                &archive,
                &state,
                Arc::new(FailPublicationOnce {
                    point,
                    failed: AtomicBool::new(false),
                }),
            )
            .unwrap(),
        );
        let service = CollectionService::new(faulted.clone(), Arc::new(Registry::builtins()));
        let context = CommandContext::new("stable-collection-create").unwrap();
        assert!(
            service
                .create_collection(
                    &test_authentication(&actor),
                    "Recovered collection",
                    &context
                )
                .is_err()
        );
        drop(service);
        drop(faulted);

        let recovered = Arc::new(Store::open(&archive, &state).unwrap());
        let service = CollectionService::new(recovered.clone(), Arc::new(Registry::builtins()));
        let collection = service
            .create_collection(
                &test_authentication(&actor),
                "Recovered collection",
                &context,
            )
            .unwrap();
        assert_eq!(
            Catalog::role(&*recovered, user, collection.id).unwrap(),
            Role::Owner
        );
        assert_eq!(Catalog::collections(&*recovered, user).unwrap().len(), 1);
        let private = rusqlite::Connection::open(state.join("private.sqlite")).unwrap();
        assert_eq!(
            private
                .query_row("SELECT COUNT(*) FROM publication_recovery", [], |row| {
                    row.get::<_, u32>(0)
                })
                .unwrap(),
            0
        );
        drop(service);
        drop(recovered);

        let restored = Store::open(&archive, root.path().join("archive-only-state")).unwrap();
        assert!(Catalog::collections(&restored, user).unwrap().is_empty());
        assert!(matches!(
            Catalog::role(&restored, user, collection.id),
            Err(Error::Forbidden)
        ));
    }
}

#[test]
fn subprocess_collection_create_worker() {
    let Ok(archive) = std::env::var("P01_COLLECTION_CHILD_ARCHIVE") else {
        return;
    };
    let state = std::env::var("P01_COLLECTION_CHILD_STATE").unwrap();
    let user: UserId = std::env::var("P01_COLLECTION_CHILD_USER")
        .unwrap()
        .parse()
        .unwrap();
    let point = parse_publication_point(&std::env::var("P01_COLLECTION_CHILD_POINT").unwrap());
    let store = Arc::new(
        Store::open_with_publication_fault(
            archive,
            state,
            Arc::new(BlockAtPublicationPoint(point)),
        )
        .unwrap(),
    );
    let service = CollectionService::new(store, Arc::new(Registry::builtins()));
    let actor = Principal {
        user_id: user,
        administrator: true,
        device_id: None,
    };
    let _ = service.create_collection(
        &test_authentication(&actor),
        "Terminated collection",
        &CommandContext::new("terminated-collection-create").unwrap(),
    );
    panic!("child collection creation unexpectedly returned");
}

#[test]
fn sigkill_collection_create_recovers_exactly_one_owner_grant() {
    use std::io::BufRead as _;
    for point in collection_recovery_points() {
        let root = tempfile::tempdir().unwrap();
        let archive = root.path().join("archive");
        let state = root.path().join("state");
        let store = Store::open(&archive, &state).unwrap();
        let user = store
            .create_user("owner", "a long test password", true)
            .unwrap();
        drop(store);
        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "subprocess_collection_create_worker",
                "--nocapture",
            ])
            .env("P01_COLLECTION_CHILD_ARCHIVE", &archive)
            .env("P01_COLLECTION_CHILD_STATE", &state)
            .env("P01_COLLECTION_CHILD_USER", user.to_string())
            .env("P01_COLLECTION_CHILD_POINT", format!("{point:?}"))
            .stdout(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        let mut output = std::io::BufReader::new(child.stdout.take().unwrap());
        let mut line = String::new();
        loop {
            line.clear();
            assert_ne!(
                output.read_line(&mut line).unwrap(),
                0,
                "child exited before phase marker"
            );
            if line.contains("P01_PHASE:") {
                break;
            }
        }
        child.kill().unwrap();
        child.wait().unwrap();

        let recovered = Arc::new(Store::open(&archive, &state).unwrap());
        let actor = Principal {
            user_id: user,
            administrator: true,
            device_id: None,
        };
        let service = CollectionService::new(recovered.clone(), Arc::new(Registry::builtins()));
        let collection = service
            .create_collection(
                &test_authentication(&actor),
                "Terminated collection",
                &CommandContext::new("terminated-collection-create").unwrap(),
            )
            .unwrap();
        assert_eq!(Catalog::collections(&*recovered, user).unwrap().len(), 1);
        assert_eq!(
            Catalog::role(&*recovered, user, collection.id).unwrap(),
            Role::Owner
        );
    }
}

#[test]
fn startup_retains_and_reports_unknown_staging() {
    let root = tempfile::tempdir().unwrap();
    let archive = root.path().join("archive");
    let state = root.path().join("state");
    std::fs::create_dir_all(archive.join(".staging/operator-copy")).unwrap();
    std::fs::write(
        archive.join(".staging/operator-copy/unrecognized.bin"),
        b"do not delete",
    )
    .unwrap();

    let store = Store::open(&archive, &state).unwrap();
    assert_eq!(
        store.recovery_report().unknown_staging,
        vec!["operator-copy"]
    );
    assert_eq!(
        std::fs::read(archive.join(".staging/operator-copy/unrecognized.bin")).unwrap(),
        b"do not delete"
    );
}

#[test]
fn corrupt_known_publication_blocks_store_open() {
    let root = tempfile::tempdir().unwrap();
    let archive = root.path().join("archive");
    let state = root.path().join("state");
    let transaction = TransactionId::new();
    std::fs::create_dir_all(archive.join(".transactions")).unwrap();
    std::fs::create_dir_all(archive.join(".staging").join(transaction.to_string())).unwrap();
    std::fs::write(
        archive
            .join(".staging")
            .join(transaction.to_string())
            .join("record.json"),
        b"corrupt",
    )
    .unwrap();
    let journal = serde_json::json!({
        "schema_version": 1,
        "transaction_id": transaction,
        "entity_type": "item",
        "entity_id": ItemId::new().to_string(),
        "expected_revision": 0,
        "target_revision": 1,
        "phase": "bytes_staged",
        "artifacts": [{
            "kind": "revision",
            "staged_path": format!(".staging/{transaction}/record.json"),
            "published_path": format!("collections/fixture/items/revisions/{transaction}/record.json"),
            "sha256": Digest::sha256(b"expected").hex
        }]
    });
    std::fs::write(
        archive
            .join(".transactions")
            .join(format!("{transaction}.json")),
        serde_json::to_vec_pretty(&journal).unwrap(),
    )
    .unwrap();

    let error = match Store::open(&archive, &state) {
        Ok(_) => panic!("corrupt known transaction must fail closed"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("digest mismatch"));
}
fn rewrite_record(path: &Path, update: impl FnOnce(&mut serde_json::Value)) {
    let mut value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    update(&mut value);
    let bytes = serde_json::to_vec_pretty(&value).unwrap();
    std::fs::write(path, &bytes).unwrap();
    let manifest = path.parent().unwrap().join("manifest.toml");
    let digest = Digest::sha256(&bytes).hex;
    let updated = std::fs::read_to_string(&manifest)
        .unwrap()
        .lines()
        .map(|line| {
            if line.starts_with("record_sha256 =") {
                format!("record_sha256 = \"{digest}\"")
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    std::fs::write(manifest, updated).unwrap();
}
fn setup() -> (
    tempfile::TempDir,
    Arc<Store>,
    Principal,
    CollectionService,
    Collection,
) {
    let root = tempfile::tempdir().unwrap();
    let store =
        Arc::new(Store::open(root.path().join("archive"), root.path().join("state")).unwrap());
    let user = store
        .create_user("owner", "a long test password", true)
        .unwrap();
    let actor = Principal {
        user_id: user,
        administrator: true,
        device_id: None,
    };
    let service = CollectionService::new(store.clone(), Arc::new(Registry::builtins()));
    let collection = service
        .create_collection(&test_authentication(&actor), "日本の音楽", &command())
        .unwrap();
    (root, store, actor, service, collection)
}
fn item(collection: CollectionId) -> Item {
    Item {
        id: ItemId::new(),
        collection_id: collection,
        revision: Revision(0),
        title: "風の歌".into(),
        subtitle: "Artist · CD".into(),
        identity_link: ItemIdentityLink::None,
        domain: ModuleData {
            module: ModuleId("org.junk.music".into()),
            schema_version: 1,
            payload: RawJson::from_value(serde_json::json!({})),
        },
        identity: IdentityState::Unidentified,
        translations: vec![],
        identifiers: vec![],
        specimens: vec![],
        wishlist: true,
        captures: vec![],
        assets: vec![],
        metadata: MetadataState::default(),
        deleted: false,
        updated_at: chrono::Utc::now(),
    }
}

#[test]
fn same_key_replays_after_restart_and_changed_payload_conflicts() {
    let (root, store, actor, service, collection) = setup();
    let original = item(collection.id);
    let id = original.id;
    let command = SaveItemCommand::from(original);
    let context = CommandContext::new("stable-item-command").unwrap();
    let saved = service
        .save_item(&actor, id, command.clone(), Revision(0), &context)
        .unwrap();
    drop(service);
    drop(store);

    let store =
        Arc::new(Store::open(root.path().join("archive"), root.path().join("state")).unwrap());
    let service = CollectionService::new(store.clone(), Arc::new(Registry::builtins()));
    let replay = service
        .save_item(&actor, id, command.clone(), Revision(0), &context)
        .unwrap();
    assert_eq!(replay.revision, saved.revision);
    assert_eq!(replay.title, saved.title);

    let mut changed = command;
    changed.title = "different request".into();
    assert!(matches!(
        service.save_item(&actor, id, changed, Revision(0), &context),
        Err(Error::IdempotencyConflict)
    ));
    assert_eq!(Catalog::item(&*store, id).unwrap().revision, Revision(1));
}

#[test]
fn concurrent_receipt_claim_has_one_executor_and_replays_completion() {
    let (_root, store, actor, _service, _collection) = setup();
    let barrier = Arc::new(std::sync::Barrier::new(3));
    let mut threads = Vec::new();
    for _ in 0..2 {
        let store = store.clone();
        let barrier = barrier.clone();
        threads.push(std::thread::spawn(move || {
            barrier.wait();
            match store.begin_command(actor.user_id, 1, "test.concurrent", "same-key", "digest") {
                Ok(ReceiptDecision::Execute) => "execute",
                Err(Error::CommandInProgress) => "in_progress",
                _ => "unexpected",
            }
        }));
    }
    barrier.wait();
    let mut outcomes = threads
        .into_iter()
        .map(|thread| thread.join().unwrap())
        .collect::<Vec<_>>();
    outcomes.sort_unstable();
    assert_eq!(outcomes, ["execute", "in_progress"]);

    store
        .complete_command(
            actor.user_id,
            1,
            "test.concurrent",
            "same-key",
            "digest",
            serde_json::json!({"entity":"one"}),
        )
        .unwrap();
    assert!(matches!(
        store
            .begin_command(actor.user_id, 1, "test.concurrent", "same-key", "digest")
            .unwrap(),
        ReceiptDecision::Replay(value) if value == serde_json::json!({"entity":"one"})
    ));
}

#[test]
fn stale_revision_abandons_receipt_without_publishing() {
    let (root, store, actor, service, collection) = setup();
    let saved = save_inventory(&service, &actor, item(collection.id), Revision(0)).unwrap();
    let mut update = SaveItemCommand::from(saved.clone());
    update.title = "accepted after retry".into();
    let context = CommandContext::new("stale-then-corrected").unwrap();
    assert!(matches!(
        service.save_item(&actor, saved.id, update.clone(), Revision(0), &context),
        Err(Error::Conflict)
    ));
    assert_eq!(
        Catalog::item(&*store, saved.id).unwrap().revision,
        Revision(1)
    );
    assert!(
        std::fs::read_dir(root.path().join("archive/.transactions"))
            .unwrap()
            .next()
            .is_none()
    );
    let accepted = service
        .save_item(&actor, saved.id, update, Revision(1), &context)
        .unwrap();
    assert_eq!(accepted.revision, Revision(2));
    assert_eq!(accepted.title, "accepted after retry");
}

#[test]
fn committed_wave1_schema2_and_private_v2_fixture_are_upgrade_inputs() {
    let root = tempfile::tempdir().unwrap();
    let fixture = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/wave-1-schema-2"
    ));
    let archive = root.path().join("archive");
    let state = root.path().join("state");
    copy_tree(&fixture.join("archive"), &archive);
    std::fs::create_dir_all(&state).unwrap();
    let private = rusqlite::Connection::open(state.join("private.sqlite")).unwrap();
    private
        .execute_batch(&std::fs::read_to_string(fixture.join("private-v2.sql")).unwrap())
        .unwrap();
    drop(private);
    let before = tree_hashes(&archive);

    let store = Store::open(&archive, &state).unwrap();
    for (path, digest) in before {
        assert_eq!(
            Digest::sha256(&std::fs::read(archive.join(path)).unwrap()),
            digest
        );
    }
    let item_id: ItemId = "018f0000-0000-7000-8000-000000000010".parse().unwrap();
    let capture_id: CaptureId = "018f0000-0000-7000-8000-000000000050".parse().unwrap();
    assert_eq!(
        Catalog::capture(&store, capture_id).unwrap().revision,
        Revision(3)
    );
    assert_eq!(
        Catalog::item(&store, item_id).unwrap().captures[0].revision,
        Revision(3)
    );
    let v3_item = archive.join(
        "collections/synthetic--018f0000-0000-7000-8000-000000000001/items/two-disc--018f0000-0000-7000-8000-000000000010/revisions/00000000000000000002/record.v3.json",
    );
    let v3_text = std::fs::read_to_string(v3_item).unwrap();
    let v3_json: serde_json::Value = serde_json::from_str(&v3_text).unwrap();
    assert!(v3_json["record"].get("captures").is_none());
    assert!(v3_json["record"].get("assets").is_none());
    assert!(v3_text.contains(r#""payload": {"z":1e+02,"a":null,"ordered":[3,2,1]}"#));
    assert!(archive
        .join("collections/synthetic--018f0000-0000-7000-8000-000000000001/evidence/018f0000-0000-7000-8000-000000000080/record.v3.json")
        .exists());
    assert!(archive
        .join("collections/synthetic--018f0000-0000-7000-8000-000000000001/assets/018f0000-0000-7000-8000-000000000070/revisions/00000000000000000001/record.v3.json")
        .exists());

    let catalog = rusqlite::Connection::open(state.join("catalog.sqlite")).unwrap();
    assert_eq!(
        catalog
            .query_row("PRAGMA user_version", [], |row| row.get::<_, u32>(0))
            .unwrap(),
        3
    );

    let private = rusqlite::Connection::open(state.join("private.sqlite")).unwrap();
    assert_eq!(
        private
            .query_row("PRAGMA user_version", [], |row| row.get::<_, u32>(0))
            .unwrap(),
        9
    );
    assert_eq!(
        private
            .query_row("SELECT COUNT(*) FROM publication_recovery", [], |row| {
                row.get::<_, u32>(0)
            })
            .unwrap(),
        0
    );
    assert_eq!(
        private
            .query_row("SELECT COUNT(*) FROM users", [], |row| row.get::<_, u32>(0))
            .unwrap(),
        2
    );
    assert_eq!(
        private
            .query_row(
                "SELECT COUNT(*) FROM users WHERE enabled=1 AND revision=1 AND display_name=username",
                [],
                |row| row.get::<_, u32>(0),
            )
            .unwrap(),
        2
    );
    assert_eq!(
        private
            .query_row("SELECT COUNT(*) FROM grants WHERE revision=1", [], |row| {
                row.get::<_, u32>(0)
            },)
            .unwrap(),
        2
    );
    assert_eq!(
        private
            .query_row(
                "SELECT COUNT(*) FROM sessions WHERE authenticated_at=0 AND created_at=0",
                [],
                |row| row.get::<_, u32>(0),
            )
            .unwrap(),
        1
    );
    let migrated_passkey_id: String = private
        .query_row("SELECT passkey_id FROM passkeys", [], |row| row.get(0))
        .unwrap();
    migrated_passkey_id.parse::<PasskeyId>().unwrap();
    assert_eq!(
        private
            .query_row("SELECT COUNT(*) FROM grants", [], |row| row
                .get::<_, u32>(0))
            .unwrap(),
        2
    );
    for table in ["sessions", "passkeys", "activity", "jobs"] {
        let count = private
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get::<_, u32>(0)
            })
            .unwrap();
        assert_eq!(count, 1, "lost private-v2 {table}");
    }
    assert_eq!(
        private
            .query_row(
                "SELECT state,completed_units,total_units FROM jobs WHERE id='018f0000-0000-7000-8000-000000000093'",
                [],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, u64>(1)?, row.get::<_, u64>(2)?))
            )
            .unwrap(),
        ("completed".to_owned(), 14, 14)
    );
    assert_eq!(
        private
            .query_row(
                "SELECT value FROM settings WHERE key='default_user_id'",
                [],
                |row| row.get::<_, String>(0)
            )
            .unwrap(),
        "018f0000-0000-7000-8000-000000000090"
    );
    assert!(archive.join("collections/synthetic--018f0000-0000-7000-8000-000000000001/items/two-disc--018f0000-0000-7000-8000-000000000010/assets/018f0000-0000-7000-8000-000000000070/original.png").is_file());
    assert!(archive.join("collections/synthetic--018f0000-0000-7000-8000-000000000001/items/two-disc--018f0000-0000-7000-8000-000000000010/files/018f0000-0000-7000-8000-000000000060/disc-1.bin").is_file());
}

#[test]
fn private_lock_prevents_two_servers_using_the_same_auth_state() {
    let root = tempfile::tempdir().unwrap();
    let state = root.path().join("state");
    let first = Store::open(root.path().join("archive-one"), &state).unwrap();
    assert!(matches!(
        Store::open(root.path().join("archive-two"), &state),
        Err(Error::Unavailable(message)) if message.contains("private state")
    ));
    drop(first);
    Store::open(root.path().join("archive-two"), &state).unwrap();
}

#[test]
fn recent_auth_expires_at_exact_boundary_and_activity_does_not_refresh_it() {
    let root = tempfile::tempdir().unwrap();
    let clock = Arc::new(TestClock::new(1_800_000_000));
    let store = Arc::new(
        Store::open_with_clock(
            root.path().join("archive"),
            root.path().join("state"),
            clock.clone(),
        )
        .unwrap(),
    );
    let user = store
        .create_user("owner", "a sufficiently long password", true)
        .unwrap();
    let (token, authenticated) = store
        .login("owner", "a sufficiently long password", "browser")
        .unwrap();
    let service = CollectionService::new(store.clone(), Arc::new(Registry::builtins()));
    service
        .require_recent_authentication(&authenticated)
        .unwrap();

    clock.advance(299);
    let authenticated_again = service.authenticate(&token).unwrap();
    service
        .require_recent_authentication(&authenticated_again)
        .unwrap();
    assert_eq!(authenticated_again.principal.user_id, user);

    clock.advance(1);
    let still_same_authentication = service.authenticate(&token).unwrap();
    assert!(matches!(
        service.require_recent_authentication(&still_same_authentication),
        Err(Error::RecentAuthenticationRequired)
    ));
    assert_eq!(
        still_same_authentication.authenticated_at,
        authenticated.authenticated_at
    );
}

#[test]
fn recovery_code_replacement_and_concurrent_consumption_have_one_winner() {
    let root = tempfile::tempdir().unwrap();
    let archive = root.path().join("archive");
    let state = root.path().join("state");
    let store = Arc::new(Store::open(&archive, &state).unwrap());
    let user = store
        .create_user("owner", "the original long password", true)
        .unwrap();
    let (old_token, authenticated) = store
        .login("owner", "the original long password", "old browser")
        .unwrap();
    let service = CollectionService::new(store.clone(), Arc::new(Registry::builtins()));
    let first_batch = service.replace_recovery_codes(&authenticated).unwrap();
    let second_batch = service.replace_recovery_codes(&authenticated).unwrap();
    assert_eq!(first_batch.len(), 10);
    assert_eq!(second_batch.len(), 10);
    assert!(first_batch.iter().all(|code| code.len() == 40));
    assert!(
        store
            .consume_recovery_code(
                "owner",
                &first_batch[0],
                "an invalidated new password",
                "recovered browser"
            )
            .is_err()
    );

    let barrier = Arc::new(std::sync::Barrier::new(3));
    let mut workers = Vec::new();
    for password in ["first concurrent password", "second concurrent password"] {
        let store = store.clone();
        let barrier = barrier.clone();
        let code = second_batch[0].clone();
        workers.push(std::thread::spawn(move || {
            barrier.wait();
            store.consume_recovery_code("owner", &code, password, "recovered browser")
        }));
    }
    barrier.wait();
    let results: Vec<_> = workers
        .into_iter()
        .map(|worker| worker.join().unwrap())
        .collect();
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(results.iter().filter(|result| result.is_err()).count(), 1);
    assert!(store.authenticate(&old_token).is_err());
    assert!(
        store
            .login("owner", "the original long password", "old password")
            .is_err()
    );
    assert!(
        store
            .consume_recovery_code(
                "owner",
                &second_batch[1],
                "another replacement password",
                "replay"
            )
            .is_err()
    );
    assert_eq!(store.sessions(user, None).unwrap().len(), 1);

    let first_won = store
        .login("owner", "first concurrent password", "verification")
        .is_ok();
    let second_won = store
        .login("owner", "second concurrent password", "verification")
        .is_ok();
    assert_ne!(first_won, second_won);

    for entry in std::fs::read_dir(&state).unwrap() {
        let path = entry.unwrap().path();
        if path.is_file() {
            let bytes = std::fs::read(path).unwrap();
            for code in first_batch.iter().chain(second_batch.iter()) {
                assert!(
                    !bytes
                        .windows(code.len())
                        .any(|window| window == code.as_bytes())
                );
            }
        }
    }
}

#[test]
fn concurrent_passkey_counter_update_fails_stale_assertion_and_revocation_is_immediate() {
    let root = tempfile::tempdir().unwrap();
    let store =
        Arc::new(Store::open(root.path().join("archive"), root.path().join("state")).unwrap());
    let user = store
        .create_user("owner", "a sufficiently long password", true)
        .unwrap();
    let original = serde_json::json!({"credential": "synthetic", "counter": 7});
    store
        .insert_passkey(user, "synthetic-credential", original.clone())
        .unwrap();
    let stored = store.passkeys(user).unwrap().remove(0);
    let barrier = Arc::new(std::sync::Barrier::new(3));
    let mut workers = Vec::new();
    for counter in [8, 9] {
        let store = store.clone();
        let barrier = barrier.clone();
        let stored = stored.clone();
        workers.push(std::thread::spawn(move || {
            barrier.wait();
            store.complete_passkey_authentication(
                user,
                stored.view.id,
                stored.view.revision,
                stored.record,
                serde_json::json!({"credential": "synthetic", "counter": counter}),
                "passkey browser",
            )
        }));
    }
    barrier.wait();
    let results: Vec<_> = workers
        .into_iter()
        .map(|worker| worker.join().unwrap())
        .collect();
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(store.sessions(user, None).unwrap().len(), 1);

    let current = store.passkeys(user).unwrap().remove(0);
    store
        .revoke_passkey(user, current.view.id, current.view.revision)
        .unwrap();
    assert!(store.passkeys(user).unwrap().is_empty());
    assert!(
        store
            .complete_passkey_authentication(
                user,
                current.view.id,
                Revision(current.view.revision.0 + 1),
                current.record,
                serde_json::json!({"credential": "synthetic", "counter": 10}),
                "replayed browser",
            )
            .is_err()
    );
    assert_eq!(store.sessions(user, None).unwrap().len(), 1);
}

#[test]
fn concurrent_last_authority_changes_cannot_both_commit() {
    let root = tempfile::tempdir().unwrap();
    let store =
        Arc::new(Store::open(root.path().join("archive"), root.path().join("state")).unwrap());
    let first = store
        .create_user("first", "a sufficiently long password", true)
        .unwrap();
    let second = store
        .create_user("second", "another sufficiently long password", true)
        .unwrap();
    let collection = store
        .create_collection(
            first,
            Collection {
                id: CollectionId::new(),
                revision: Revision(1),
                name: "Guarded collection".into(),
                created_at: chrono::Utc::now(),
            },
        )
        .unwrap();
    Catalog::grant(&*store, collection.id, second, Role::Owner).unwrap();

    let barrier = Arc::new(std::sync::Barrier::new(3));
    let mut workers = Vec::new();
    for user in [first, second] {
        let store = store.clone();
        let barrier = barrier.clone();
        workers.push(std::thread::spawn(move || {
            barrier.wait();
            store.update_account(user, Revision(1), "Demoted", true, false)
        }));
    }
    barrier.wait();
    let admin_results: Vec<_> = workers
        .into_iter()
        .map(|worker| worker.join().unwrap())
        .collect();
    assert_eq!(
        admin_results.iter().filter(|result| result.is_ok()).count(),
        1
    );
    assert!(
        store
            .accounts()
            .unwrap()
            .iter()
            .any(|account| account.enabled && account.administrator)
    );

    let grants = AccessPort::grants(&*store, collection.id).unwrap();
    let barrier = Arc::new(std::sync::Barrier::new(3));
    let mut workers = Vec::new();
    for grant in grants {
        let store = store.clone();
        let barrier = barrier.clone();
        workers.push(std::thread::spawn(move || {
            barrier.wait();
            AccessPort::set_grant(&*store, collection.id, grant.user_id, grant.revision, None)
        }));
    }
    barrier.wait();
    let owner_results: Vec<_> = workers
        .into_iter()
        .map(|worker| worker.join().unwrap())
        .collect();
    assert_eq!(
        owner_results.iter().filter(|result| result.is_ok()).count(),
        1
    );
    assert_eq!(
        AccessPort::grants(&*store, collection.id)
            .unwrap()
            .iter()
            .filter(|grant| grant.role == Role::Owner)
            .count(),
        1
    );
}

#[test]
fn local_recovery_requires_exclusive_lock_and_preserves_grants() {
    let root = tempfile::tempdir().unwrap();
    let archive = root.path().join("archive");
    let state = root.path().join("state");
    let store = Store::open(&archive, &state).unwrap();
    let user = store
        .create_user("owner", "the original long password", true)
        .unwrap();
    let collection = store
        .create_collection(
            user,
            Collection {
                id: CollectionId::new(),
                revision: Revision(1),
                name: "Preserved access".into(),
                created_at: chrono::Utc::now(),
            },
        )
        .unwrap();
    store.issue_session(user, "browser").unwrap();
    store
        .insert_passkey(
            user,
            "local-recovery-passkey",
            serde_json::json!({"counter": 0}),
        )
        .unwrap();
    assert!(matches!(
        Store::local_recover(&state, user, "the replacement password"),
        Err(Error::Unavailable(_))
    ));
    drop(store);

    Store::local_recover(&state, user, "the replacement password").unwrap();
    let recovered = Store::open(&archive, &state).unwrap();
    assert!(
        recovered
            .login("owner", "the original long password", "old")
            .is_err()
    );
    let (_, actor) = recovered
        .login("owner", "the replacement password", "new")
        .unwrap();
    assert!(actor.principal.administrator);
    assert_eq!(
        Catalog::role(&recovered, user, collection.id).unwrap(),
        Role::Owner
    );
    assert!(recovered.passkeys(user).unwrap().is_empty());
}

#[test]
fn enrollment_terminal_states_wrong_user_and_issuance_race_are_enforced() {
    let root = tempfile::tempdir().unwrap();
    let state = root.path().join("state");
    let clock = Arc::new(TestClock::new(1_800_100_000));
    let store = Arc::new(
        Store::open_with_clock(root.path().join("archive"), &state, clock.clone()).unwrap(),
    );
    let requested = store
        .create_user("requested", "a sufficiently long password", true)
        .unwrap();
    let wrong_user = store
        .create_user("wrong", "another sufficiently long password", false)
        .unwrap();

    let enrollment = store
        .start_enrollment(requested, "Living-room client", "/pair")
        .unwrap();
    assert!(
        store
            .approve_enrollment(wrong_user, &enrollment.user_code)
            .is_err()
    );
    store
        .approve_enrollment(requested, &enrollment.user_code)
        .unwrap();
    assert!(
        store
            .complete_enrollment(enrollment.id, &"0".repeat(64))
            .is_err()
    );

    let barrier = Arc::new(std::sync::Barrier::new(3));
    let mut workers = Vec::new();
    for _ in 0..2 {
        let store = store.clone();
        let barrier = barrier.clone();
        let secret = enrollment.completion_secret.clone();
        workers.push(std::thread::spawn(move || {
            barrier.wait();
            store.complete_enrollment(enrollment.id, &secret)
        }));
    }
    barrier.wait();
    let results: Vec<_> = workers
        .into_iter()
        .map(|worker| worker.join().unwrap())
        .collect();
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    let issued = results.into_iter().find_map(Result::ok).unwrap();
    assert!(
        store
            .complete_enrollment(enrollment.id, &enrollment.completion_secret)
            .is_err()
    );

    let cancelled = store
        .start_enrollment(requested, "Cancelled client", "/pair")
        .unwrap();
    store.cancel_enrollment(requested, cancelled.id).unwrap();
    assert!(matches!(
        store.approve_enrollment(requested, &cancelled.user_code),
        Err(Error::EnrollmentCancelled)
    ));
    assert!(matches!(
        store.complete_enrollment(cancelled.id, &cancelled.completion_secret),
        Err(Error::EnrollmentCancelled)
    ));

    let expired = store
        .start_enrollment(requested, "Expired client", "/pair")
        .unwrap();
    clock.advance(600);
    assert!(matches!(
        store.approve_enrollment(requested, &expired.user_code),
        Err(Error::EnrollmentExpired)
    ));

    for entry in std::fs::read_dir(&state).unwrap() {
        let path = entry.unwrap().path();
        if path.is_file() {
            let bytes = std::fs::read(path).unwrap();
            for secret in [&enrollment.completion_secret, &issued.credential] {
                assert!(
                    !bytes
                        .windows(secret.len())
                        .any(|window| window == secret.as_bytes())
                );
            }
        }
    }
}

#[test]
fn conflicting_duplicate_evidence_id_blocks_schema3_migration() {
    let root = tempfile::tempdir().unwrap();
    let fixture =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/wave-1-schema-2/archive");
    let archive = root.path().join("archive");
    let state = root.path().join("state");
    copy_tree(&fixture, &archive);
    let revision = archive.join(
        "collections/synthetic--018f0000-0000-7000-8000-000000000001/items/two-disc--018f0000-0000-7000-8000-000000000010/revisions/00000000000000000002",
    );
    let record_path = revision.join("record.v2.json");
    let mut record: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&record_path).unwrap()).unwrap();
    record["evidence"][0]["details"]["claim"] =
        serde_json::json!("conflicting body under a reused EvidenceId");
    let bytes = serde_json::to_vec_pretty(&record).unwrap();
    std::fs::write(&record_path, &bytes).unwrap();
    let manifest_path = revision.join("manifest.v2.toml");
    let manifest = std::fs::read_to_string(&manifest_path)
        .unwrap()
        .lines()
        .map(|line| {
            if line.starts_with("record_sha256 =") {
                format!("record_sha256 = \"{}\"", Digest::sha256(&bytes).hex)
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    std::fs::write(manifest_path, manifest).unwrap();

    let error = match Store::open(&archive, &state) {
        Ok(_) => panic!("conflicting Evidence identity must block migration"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("conflicting schema-2 bodies"));
    assert!(
        !archive
            .join(".archive-migrations/schema-2-to-3/complete")
            .exists()
    );
    let catalog = rusqlite::Connection::open(state.join("catalog.sqlite")).unwrap();
    assert_eq!(
        catalog
            .query_row("SELECT COUNT(*) FROM items", [], |row| row.get::<_, u32>(0))
            .unwrap(),
        0
    );
}

fn save_inventory(
    service: &CollectionService,
    actor: &Principal,
    item: Item,
    expected: Revision,
) -> Result<Item> {
    let id = item.id;
    service.save_item(actor, id, item.into(), expected, &command())
}

fn save_work_record(
    service: &CollectionService,
    actor: &Principal,
    work: Work,
    expected: Revision,
) -> Result<Work> {
    let id = work.id;
    service.save_work(actor, id, work.into(), expected, &command())
}

fn save_edition_record(
    service: &CollectionService,
    actor: &Principal,
    edition: Edition,
    expected: Revision,
) -> Result<Edition> {
    let id = edition.id;
    service.save_edition(actor, id, edition.into(), expected, &command())
}

fn save_capture_record(
    service: &CollectionService,
    actor: &Principal,
    capture: Capture,
    expected: Revision,
) -> Result<Capture> {
    let id = capture.id;
    service.save_capture(actor, id, capture.into(), expected, &command())
}

fn stage_directory(
    service: &CollectionService,
    actor: &Principal,
    id: JobId,
    source: &Path,
) -> Result<Item> {
    let upload = service.upload(actor, id)?;
    for (index, entry) in upload.files.iter().enumerate() {
        let bytes = std::fs::read(source.join(&entry.name)).unwrap();
        service.upload_chunk(actor, id, index, 0, &bytes)?;
    }
    service.commit_upload(actor, id, &command())
}

#[test]
fn committed_schema_one_fixture_migrates_additively_and_restarts() {
    let root = tempfile::tempdir().unwrap();
    let archive = root.path().join("archive");
    copy_tree(
        Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/schema-1"
        )),
        &archive,
    );
    let original = std::fs::read(archive.join("collections/synthetic--018f0000-0000-7000-8000-000000000001/items/two-disc--018f0000-0000-7000-8000-000000000010/revisions/00000000000000000002/record.json")).unwrap();
    let original_digest = Digest::sha256(&original);
    let store = Store::open(&archive, root.path().join("state")).unwrap();
    let item = store
        .item("018f0000-0000-7000-8000-000000000010".parse().unwrap())
        .unwrap();
    assert_eq!(item.revision, Revision(2));
    assert!(matches!(
        item.identity_link,
        ItemIdentityLink::Edition { .. }
    ));
    assert_eq!(
        item.domain.payload.get(),
        r#"{"z":1e+02,"a":null,"ordered":[3,2,1]}"#
    );
    assert_eq!(
        item.metadata.candidates[0].observation.get(),
        r#"{"providerOrder":2,"number":1.00,"missing":null}"#
    );
    assert_eq!(item.specimens.len(), 2);
    assert_eq!(item.specimens[0].components.len(), 4);
    assert!(item.specimens[0].components[0].carrier.is_some());
    assert!(item.specimens[0].components[2].carrier.is_none());
    assert_eq!(
        item.specimens[0]
            .acquisition
            .as_ref()
            .unwrap()
            .cost
            .as_ref()
            .unwrap()
            .currency
            .as_str(),
        "JPY"
    );
    assert_eq!(item.captures[0].revision, Revision(2));
    assert_eq!(
        item.captures[0].representations[0].files[0].digest.hex,
        "a".repeat(64)
    );
    assert!(
        String::from_utf8(original.clone())
            .unwrap()
            .contains(&"A".repeat(64))
    );
    assert!(
        archive
            .join(".archive-migrations/schema-1-to-2/complete")
            .exists()
    );
    assert!(archive.join("collections/synthetic--018f0000-0000-7000-8000-000000000001/works/018f0000-0000-7000-8000-000000000020/revisions/00000000000000000001/record.json").exists());
    assert_eq!(Digest::sha256(&std::fs::read(archive.join("collections/synthetic--018f0000-0000-7000-8000-000000000001/items/two-disc--018f0000-0000-7000-8000-000000000010/revisions/00000000000000000002/record.json")).unwrap()), original_digest);
    let user = store
        .create_user("fixture-owner", "a sufficiently long password", true)
        .unwrap();
    store.grant(item.collection_id, user, Role::Owner).unwrap();
    let legacy_file = ContentPort::representation(
        &store,
        item.id,
        "018f0000-0000-7000-8000-000000000060".parse().unwrap(),
        0,
    )
    .unwrap();
    assert_eq!(resource_bytes(legacy_file), b"disc-one\n");
    drop(store);
    std::fs::remove_file(archive.join(".archive-migrations/schema-1-to-2/complete")).unwrap();
    let second = archive.join("collections/synthetic--018f0000-0000-7000-8000-000000000001/items/two-disc--018f0000-0000-7000-8000-000000000010/revisions/00000000000000000002");
    std::fs::remove_file(second.join("record.v2.json")).unwrap();
    std::fs::remove_file(second.join("manifest.v2.toml")).unwrap();
    std::fs::remove_file(archive.join(".archive-migrations/schema-1-to-2/records/018f0000-0000-7000-8000-000000000010-00000000000000000002.done")).unwrap();
    let reopened = Store::open(&archive, root.path().join("state")).unwrap();
    assert_eq!(
        reopened.item(item.id).unwrap().domain.payload.get(),
        item.domain.payload.get()
    );
    assert_eq!(
        reopened.item(item.id).unwrap().specimens[0].components[0].id,
        item.specimens[0].components[0].id
    );
    drop(reopened);
    let repeated = Store::open(&archive, root.path().join("state")).unwrap();
    assert_eq!(repeated.item(item.id).unwrap().revision, Revision(2));
}

#[test]
fn private_migration_pins_the_existing_first_row_default_once() {
    let root = tempfile::tempdir().unwrap();
    let state = root.path().join("state");
    std::fs::create_dir_all(&state).unwrap();
    let private = rusqlite::Connection::open(state.join("private.sqlite")).unwrap();
    private.execute_batch("CREATE TABLE users (id TEXT PRIMARY KEY, username TEXT UNIQUE NOT NULL, password_hash TEXT NOT NULL, administrator INTEGER NOT NULL); CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL); INSERT INTO users VALUES ('018f0000-0000-7000-8000-000000000090','first','preserved-hash',1); INSERT INTO users VALUES ('018f0000-0000-7000-8000-000000000091','second','preserved-hash-2',0);").unwrap();
    drop(private);
    let store = Store::open(root.path().join("archive"), &state).unwrap();
    assert_eq!(store.default_username().unwrap(), "first");
    store
        .create_user("third", "a sufficiently long password", false)
        .unwrap();
    assert_eq!(store.default_username().unwrap(), "first");
    drop(store);
    let reopened = Store::open(root.path().join("archive"), &state).unwrap();
    assert_eq!(reopened.default_username().unwrap(), "first");
}

#[test]
fn standalone_unidentified_capture_round_trips_without_fabricated_links() {
    let (_root, store, actor, service, collection) = setup();
    let capture = Capture {
        id: CaptureId::new(),
        collection_id: collection.id,
        revision: Revision(0),
        association: CaptureAssociation::default(),
        created_at: chrono::Utc::now(),
        representations: vec![],
    };
    let saved = save_capture_record(&service, &actor, capture, Revision(0)).unwrap();
    assert_eq!(saved.revision, Revision(1));
    assert_eq!(saved.association, CaptureAssociation::default());
    store.reindex().unwrap();
    let restored = service.capture(&actor, saved.id).unwrap();
    assert_eq!(restored.association, CaptureAssociation::default());
}

#[test]
fn clean_work_edition_and_edition_linked_item_round_trip() {
    let (_root, store, actor, service, collection) = setup();
    let work = save_work_record(
        &service,
        &actor,
        Work {
            id: WorkId::new(),
            collection_id: collection.id,
            revision: Revision(0),
            provisional: false,
            title: "A work".into(),
            translations: vec![],
            identifiers: vec![],
            migration_provenance: vec![],
            deleted: false,
            updated_at: chrono::Utc::now(),
        },
        Revision(0),
    )
    .unwrap();
    let edition = save_edition_record(
        &service,
        &actor,
        Edition {
            id: EditionId::new(),
            collection_id: collection.id,
            revision: Revision(0),
            work_id: Some(work.id),
            provisional: false,
            title: "An edition".into(),
            expected_components: vec![ExpectedComponent {
                position: 1,
                label: "Disc 1".into(),
                kind: ComponentKind::OpticalDisc,
            }],
            identifiers: vec![],
            migration_provenance: vec![],
            deleted: false,
            updated_at: chrono::Utc::now(),
        },
        Revision(0),
    )
    .unwrap();
    let mut value = item(collection.id);
    value.identity_link = ItemIdentityLink::Edition {
        edition_id: edition.id,
    };
    let saved = save_inventory(&service, &actor, value, Revision(0)).unwrap();
    store.reindex().unwrap();
    assert_eq!(service.work(&actor, work.id).unwrap().revision, Revision(1));
    assert_eq!(
        service.edition(&actor, edition.id).unwrap().work_id,
        Some(work.id)
    );
    assert!(
        matches!(service.item_presented(&actor, saved.id).unwrap().identity_link, ItemIdentityLink::Edition { edition_id } if edition_id == edition.id)
    );
}

#[test]
fn schema_one_migration_rejects_conflicting_edition_works() {
    let root = tempfile::tempdir().unwrap();
    let archive = root.path().join("archive");
    copy_tree(
        Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/schema-1"
        )),
        &archive,
    );
    let record = archive.join("collections/synthetic--018f0000-0000-7000-8000-000000000001/items/two-disc--018f0000-0000-7000-8000-000000000010/revisions/00000000000000000002/record.json");
    rewrite_record(&record, |value| {
        value["item"]["work_id"] = serde_json::json!("018f0000-0000-7000-8000-000000000099")
    });
    assert!(
        matches!(Store::open(&archive, root.path().join("state")), Err(Error::Invalid(message)) if message.contains("conflicting works"))
    );
}

#[test]
fn schema_one_migration_rejects_identity_reuse_across_collections() {
    let root = tempfile::tempdir().unwrap();
    let archive = root.path().join("archive");
    copy_tree(
        Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/schema-1"
        )),
        &archive,
    );
    let source = archive.join("collections/synthetic--018f0000-0000-7000-8000-000000000001");
    let duplicate = archive.join("collections/duplicate--018f0000-0000-7000-8000-000000000002");
    copy_tree(&source, &duplicate);
    let old = "018f0000-0000-7000-8000-000000000001";
    let new = "018f0000-0000-7000-8000-000000000002";
    let collection = duplicate.join("collection.toml");
    std::fs::write(
        &collection,
        std::fs::read_to_string(&collection)
            .unwrap()
            .replace(old, new),
    )
    .unwrap();
    let revisions =
        duplicate.join("items/two-disc--018f0000-0000-7000-8000-000000000010/revisions");
    for revision in std::fs::read_dir(revisions).unwrap() {
        let record = revision.unwrap().path().join("record.json");
        rewrite_record(&record, |value| {
            value["item"]["collection_id"] = serde_json::json!(new)
        });
        let manifest = record.parent().unwrap().join("manifest.toml");
        std::fs::write(
            &manifest,
            std::fs::read_to_string(&manifest)
                .unwrap()
                .replace(old, new),
        )
        .unwrap();
    }
    assert!(
        matches!(Store::open(&archive, root.path().join("state")), Err(Error::Invalid(message)) if message.contains("across collections"))
    );
}
#[test]
fn revisions_recover_unknown_module_data_and_do_not_restore_grants() {
    let (root, store, actor, service, collection) = setup();
    let mut value = item(collection.id);
    value.domain = ModuleData {
        module: ModuleId("org.example.stamps".into()),
        schema_version: 4,
        payload: RawJson::from_json(
            r#"{"nullable":null,"nested":[1,{"color":"blue"}],"number":1e+02}"#,
        )
        .unwrap(),
    };
    let saved = save_inventory(&service, &actor, value, Revision(0)).unwrap();
    let mut edited = saved.clone();
    edited.title = "Updated".into();
    save_inventory(&service, &actor, edited, saved.revision).unwrap();
    assert!(matches!(
        save_inventory(&service, &actor, saved.clone(), saved.revision),
        Err(Error::Conflict)
    ));
    drop(service);
    drop(store);
    std::fs::remove_file(root.path().join("state/catalog.sqlite")).unwrap();
    let store = Store::open(root.path().join("archive"), root.path().join("state")).unwrap();
    let restored = store.item(saved.id).unwrap();
    assert_eq!(restored.title, "Updated");
    assert_eq!(restored.domain.payload, saved.domain.payload);
    assert_eq!(restored.revision, Revision(2));
    drop(store);
    let foreign = Store::open(root.path().join("archive"), root.path().join("new-state")).unwrap();
    assert_eq!(foreign.item(saved.id).unwrap().title, "Updated");
    assert!(matches!(
        foreign.role(actor.user_id, collection.id),
        Err(Error::Forbidden)
    ));
}
#[test]
fn upload_retries_preserve_bytes_and_evidence_detects_changes() {
    let (root, store, actor, service, collection) = setup();
    let saved = save_inventory(&service, &actor, item(collection.id), Revision(0)).unwrap();
    let bytes = b"independent original payload";
    let upload = service
        .create_upload(
            &actor,
            saved.id,
            saved.revision,
            vec![UploadFile {
                name: "disc.bin".into(),
                bytes: ByteLength(bytes.len() as u64),
                sha256: Some(Digest::sha256(bytes).hex),
            }],
            &command(),
        )
        .unwrap();
    service
        .upload_chunk(&actor, upload.id, 0, 0, &bytes[..8])
        .unwrap();
    service
        .upload_chunk(&actor, upload.id, 0, 0, &bytes[..8])
        .unwrap();
    assert!(matches!(
        service.upload_chunk(&actor, upload.id, 0, 0, b"different"),
        Err(Error::Conflict)
    ));
    service
        .upload_chunk(&actor, upload.id, 0, 8, &bytes[8..])
        .unwrap();
    let commit = command();
    let committed = service.commit_upload(&actor, upload.id, &commit).unwrap();
    let retry = service.commit_upload(&actor, upload.id, &commit).unwrap();
    assert_eq!(retry.revision, committed.revision);
    let resource = service
        .representation(&actor, saved.id, upload.representation_id, 0)
        .unwrap();
    assert_eq!(resource_bytes(resource), bytes);
    assert!(
        store
            .evidence(saved.id)
            .unwrap()
            .iter()
            .all(|e| matches!(e.kind, EvidenceKind::Integrity))
    );
    let path = find_file_with_bytes(&root.path().join("archive"), bytes).unwrap();
    std::fs::write(path, b"changed").unwrap();
    service
        .verify_integrity(&actor, saved.id, &command())
        .unwrap();
    assert_eq!(
        store.evidence(saved.id).unwrap().last().unwrap().outcome,
        EvidenceOutcome::Failed
    );
    let evidence_records = files_named(&root.path().join("archive/collections"), "record.v3.json")
        .into_iter()
        .filter(|path| path.components().any(|part| part.as_os_str() == "evidence"))
        .collect::<Vec<_>>();
    assert_eq!(evidence_records.len(), 2);
    let latest_item_envelope =
        files_named(&root.path().join("archive/collections"), "record.v3.json")
            .into_iter()
            .filter_map(|path| {
                let value: serde_json::Value =
                    serde_json::from_slice(&std::fs::read(&path).ok()?).ok()?;
                (value["record"]["id"] == saved.id.to_string()).then_some((path, value))
            })
            .max_by_key(|(path, _)| path.clone())
            .unwrap()
            .1;
    assert_eq!(
        latest_item_envelope["evidence"].as_array().unwrap().len(),
        2
    );
    assert!(latest_item_envelope["evidence"][0].get("details").is_none());
}
#[test]
fn malicious_paths_and_invented_file_references_are_rejected() {
    let (_root, _store, actor, service, collection) = setup();
    let saved = save_inventory(&service, &actor, item(collection.id), Revision(0)).unwrap();
    for name in [
        "../secret",
        "/absolute",
        "dir/../../secret",
        "C:\\secret",
        "a/./b",
        "a//b",
    ] {
        assert!(
            service
                .create_upload(
                    &actor,
                    saved.id,
                    saved.revision,
                    vec![UploadFile {
                        name: name.into(),
                        bytes: ByteLength(1),
                        sha256: None
                    }],
                    &command(),
                )
                .is_err()
        );
    }
    let mut edited = saved;
    edited.captures.push(Capture {
        id: CaptureId::new(),
        collection_id: edited.collection_id,
        revision: Revision(1),
        association: CaptureAssociation {
            item_id: Some(edited.id),
            ..Default::default()
        },
        created_at: chrono::Utc::now(),
        representations: vec![],
    });
    let mut command =
        serde_json::to_value(collector_app::SaveItemCommand::from(edited.clone())).unwrap();
    command.as_object_mut().unwrap().insert(
        "captures".into(),
        serde_json::to_value(edited.captures).unwrap(),
    );
    assert!(
        serde_json::from_value::<collector_app::SaveItemCommand>(command).is_err(),
        "item commands accepted invented capture records"
    );
}
#[test]
fn viewer_redaction_and_cross_collection_access() {
    let (_root, store, _owner, service, collection) = setup();
    let viewer_id = store
        .create_user("viewer", "another long password", false)
        .unwrap();
    let stranger_id = store
        .create_user("stranger", "another long password", false)
        .unwrap();
    let viewer = Principal {
        user_id: viewer_id,
        administrator: false,
        device_id: None,
    };
    let stranger = Principal {
        user_id: stranger_id,
        administrator: false,
        device_id: None,
    };
    store.grant(collection.id, viewer_id, Role::Viewer).unwrap();
    let mut value = item(collection.id);
    value.specimens.push(Specimen {
        id: SpecimenId::new(),
        label: "Copy 1".into(),
        condition: Condition::Good,
        ownership: Ownership::Owned,
        possession: Possession::Lent,
        components: vec![],
        acquisition: Some(Acquisition {
            date: None,
            cost: Some(Money {
                minor_units: 4500,
                currency: CurrencyCode::new("JPY").unwrap(),
            }),
            source: Some("Private seller".into()),
            coordinates: None,
        }),
        private_notes: Some("private".into()),
        loans: vec![Loan {
            id: LoanId::new(),
            contact: "Private contact".into(),
            lent_at: chrono::Utc::now().date_naive(),
            due_at: None,
            returned_at: None,
        }],
    });
    let saved = Catalog::save(&*store, value, Revision(0)).unwrap();
    let public = service.item_presented(&viewer, saved.id).unwrap();
    assert!(public.specimens[0].acquisition.is_none());
    assert!(public.specimens[0].loans.is_none());
    assert!(public.specimens[0].private_notes.is_none());
    assert!(service.item_presented(&stranger, saved.id).is_err());
    assert!(
        service
            .browse_presented(&stranger, BrowseQuery::default())
            .unwrap()
            .items
            .is_empty()
    );
    assert!(save_inventory(&service, &viewer, saved.clone(), saved.revision).is_err());
}
#[test]
fn cover_upload_retains_original_and_has_bounded_thumbnail() {
    let (root, store, actor, service, collection) = setup();
    let saved = save_inventory(&service, &actor, item(collection.id), Revision(0)).unwrap();
    let expected = saved.revision;
    let image = image::RgbImage::new(900, 450);
    let mut data = std::io::Cursor::new(Vec::new());
    image.write_to(&mut data, image::ImageFormat::Png).unwrap();
    let bytes = data.into_inner();
    let context = command();
    let saved = service
        .publish_cover(
            &actor,
            saved.id,
            expected,
            &bytes,
            "Album cover".into(),
            &context,
        )
        .unwrap();
    let asset = &saved.assets[0];
    let original = service.asset(&actor, saved.id, asset.id, false).unwrap();
    assert_eq!(resource_bytes(original), bytes);
    let thumbnail = service.asset(&actor, saved.id, asset.id, true).unwrap();
    let thumbnail = image::load_from_memory(&resource_bytes(thumbnail)).unwrap();
    assert_eq!((thumbnail.width(), thumbnail.height()), (192, 96));
    let stored = find_file_with_bytes(root.path(), &bytes).unwrap();
    let stored = stored.to_string_lossy();
    assert!(stored.contains(&format!("/assets/{}/payload/", asset.id)));
    assert!(!stored.contains("/items/"));
    let replayed = service
        .publish_cover(
            &actor,
            saved.id,
            expected,
            &bytes,
            "Album cover".into(),
            &context,
        )
        .unwrap();
    assert_eq!(replayed.assets.len(), 1);
    assert_eq!(replayed.assets[0].id, asset.id);
    store.reindex().unwrap();
    assert_eq!(store.item(saved.id).unwrap().assets[0].id, asset.id);
}

#[test]
fn standalone_assets_cover_all_subjects_and_keep_independent_history() {
    let (root, store, actor, service, collection) = setup();
    let work = save_work_record(
        &service,
        &actor,
        Work {
            id: WorkId::new(),
            collection_id: collection.id,
            revision: Revision(0),
            provisional: false,
            title: "Asset work".into(),
            translations: vec![],
            identifiers: vec![],
            migration_provenance: vec![],
            deleted: false,
            updated_at: chrono::Utc::now(),
        },
        Revision(0),
    )
    .unwrap();
    let edition = save_edition_record(
        &service,
        &actor,
        Edition {
            id: EditionId::new(),
            collection_id: collection.id,
            revision: Revision(0),
            work_id: Some(work.id),
            provisional: false,
            title: "Asset edition".into(),
            expected_components: vec![],
            identifiers: vec![],
            migration_provenance: vec![],
            deleted: false,
            updated_at: chrono::Utc::now(),
        },
        Revision(0),
    )
    .unwrap();
    let mut inventory = item(collection.id);
    inventory.identity_link = ItemIdentityLink::Edition {
        edition_id: edition.id,
    };
    let inventory = save_inventory(&service, &actor, inventory, Revision(0)).unwrap();
    let inventory = service
        .create_specimen(
            &actor,
            inventory.id,
            inventory.revision,
            SpecimenDetails {
                label: "Asset copy".into(),
                condition: Condition::Good,
                ownership: ActiveOwnership::Owned,
                possession: Possession::Present,
                acquisition: None,
                private_notes: None,
            },
            &CommandContext::new("asset-specimen").unwrap(),
        )
        .unwrap();
    let specimen_id = inventory.specimens[0].id;
    let inventory = service
        .create_component(
            &actor,
            inventory.id,
            specimen_id,
            inventory.revision,
            ComponentDetails {
                label: "Disc one".into(),
                kind: ComponentKind::OpticalDisc,
                position: 1,
                carrier: Some(CarrierDetails {
                    label: "CD".into(),
                    kind: CarrierKind::OpticalDisc,
                }),
            },
            &CommandContext::new("asset-component").unwrap(),
        )
        .unwrap();
    let component_id = inventory.specimens[0].components[0].id;
    let carrier_id = inventory.specimens[0].components[0]
        .carrier
        .as_ref()
        .unwrap()
        .id;
    let capture = save_capture_record(
        &service,
        &actor,
        Capture {
            id: CaptureId::new(),
            collection_id: collection.id,
            revision: Revision(0),
            association: CaptureAssociation {
                item_id: Some(inventory.id),
                specimen_id: Some(specimen_id),
                component_id: Some(component_id),
                carrier_id: Some(carrier_id),
            },
            created_at: chrono::Utc::now(),
            representations: vec![],
        },
        Revision(0),
    )
    .unwrap();
    let item_revision = store.item(inventory.id).unwrap().revision;

    let subjects = [
        AssetSubject::Item {
            item_id: inventory.id,
        },
        AssetSubject::Specimen { specimen_id },
        AssetSubject::Component { component_id },
        AssetSubject::Capture {
            capture_id: capture.id,
        },
        AssetSubject::Work { work_id: work.id },
        AssetSubject::Edition {
            edition_id: edition.id,
        },
    ];
    let formats = [
        (image::ImageFormat::Png, "image/png"),
        (image::ImageFormat::Jpeg, "image/jpeg"),
        (image::ImageFormat::WebP, "image/webp"),
    ];
    let kinds = [AssetKind::Cover, AssetKind::Photo, AssetKind::Scan];
    let mut states = Vec::new();
    let mut originals = Vec::new();
    for (index, subject) in subjects.into_iter().enumerate() {
        let mut encoded = std::io::Cursor::new(Vec::new());
        image::RgbImage::from_pixel(480 + index as u32, 240, image::Rgb([index as u8, 40, 80]))
            .write_to(&mut encoded, formats[index % formats.len()].0)
            .unwrap();
        let bytes = encoded.into_inner();
        let context = CommandContext::new(format!("asset-create-{index}")).unwrap();
        let command = CreateAssetCommand {
            collection_id: collection.id,
            subject,
            kind: kinds[index % kinds.len()].clone(),
            visibility: Visibility::Shared,
            alt_text: format!("Asset {index}"),
        };
        let state = service
            .create_asset(&actor, command.clone(), &bytes, &context)
            .unwrap();
        assert_eq!(state.revision, Revision(1));
        assert_eq!(state.subject, subject);
        assert_eq!(state.media_type, formats[index % formats.len()].1);
        assert_eq!(
            state.original.as_ref().unwrap().bytes,
            ByteLength(bytes.len() as u64)
        );
        let thumbnail = state.thumbnail.as_ref().unwrap();
        assert_eq!(thumbnail.name, "thumbnail.png");
        let thumbnail_path = files_named(&root.path().join("archive"), "thumbnail.png")
            .into_iter()
            .find(|path| path.to_string_lossy().contains(&state.id.to_string()))
            .unwrap();
        let thumbnail_image = image::open(thumbnail_path).unwrap();
        assert!(thumbnail_image.width() <= 192 && thumbnail_image.height() <= 192);
        let replay = service
            .create_asset(&actor, command, &bytes, &context)
            .unwrap();
        assert_eq!(replay.revision, Revision(1));
        assert_eq!(replay.id, state.id);
        originals.push(bytes);
        states.push(state);
    }
    assert_eq!(store.item(inventory.id).unwrap().revision, item_revision);
    for bytes in &originals {
        assert!(find_file_with_bytes(&root.path().join("archive"), bytes).is_some());
    }
    let mut registry = Registry::default();
    registry
        .register(Box::new(ArtworkSelectingModule {
            selected: states[1].id,
        }))
        .unwrap();
    let selection_service = CollectionService::new(store.clone(), Arc::new(registry));
    let presented = selection_service
        .item_presented(&actor, inventory.id)
        .unwrap();
    assert_eq!(presented.primary_artwork, Some(states[1].id));
    assert_eq!(presented.presentation.artwork[0].asset_id, states[1].id);

    let original_id = states[0].id;
    let updated = service
        .update_asset(
            &actor,
            original_id,
            Revision(1),
            UpdateAssetCommand {
                visibility: Visibility::Curators,
                alt_text: "Private archival cover".into(),
            },
            &CommandContext::new("asset-update").unwrap(),
        )
        .unwrap();
    assert_eq!(updated.revision, Revision(2));
    assert_eq!(updated.subject, states[0].subject);
    assert_eq!(
        updated.original.as_ref().unwrap().digest,
        states[0].original.as_ref().unwrap().digest
    );
    assert_eq!(store.item(inventory.id).unwrap().revision, item_revision);
    let tombstone = service
        .tombstone_asset(
            &actor,
            original_id,
            Revision(2),
            &CommandContext::new("asset-tombstone").unwrap(),
        )
        .unwrap();
    assert_eq!(tombstone.revision, Revision(3));
    assert!(tombstone.archived);
    assert!(
        service
            .asset(&actor, inventory.id, original_id, false)
            .is_err()
    );
    assert!(find_file_with_bytes(&root.path().join("archive"), &originals[0]).is_some());
    let revision_records = files_named(&root.path().join("archive"), "record.v3.json")
        .into_iter()
        .filter(|path| path.to_string_lossy().contains(&original_id.to_string()))
        .count();
    assert_eq!(revision_records, 3);
    assert_eq!(
        ContentPort::asset_state(&*store, original_id)
            .unwrap()
            .revision,
        Revision(3)
    );
    assert_eq!(store.item(inventory.id).unwrap().revision, item_revision);

    store.reindex().unwrap();
    for state in states.iter().skip(1) {
        assert_eq!(
            ContentPort::asset_state(&*store, state.id)
                .unwrap()
                .revision,
            Revision(1)
        );
    }
    assert!(
        ContentPort::asset_state(&*store, original_id)
            .unwrap()
            .deleted
    );
}

#[test]
fn unsupported_or_invalid_asset_inputs_publish_nothing() {
    let (root, _store, actor, service, collection) = setup();
    let inventory = save_inventory(&service, &actor, item(collection.id), Revision(0)).unwrap();
    let staging = root.path().join("state/asset-publication");
    let staging_count = || {
        std::fs::read_dir(&staging)
            .map(|entries| entries.count())
            .unwrap_or(0)
    };
    let before = staging_count();
    let command_for = |kind| CreateAssetCommand {
        collection_id: collection.id,
        subject: AssetSubject::Item {
            item_id: inventory.id,
        },
        kind,
        visibility: Visibility::Shared,
        alt_text: "Evidence".into(),
    };
    assert!(matches!(
        service.create_asset(
            &actor,
            command_for(AssetKind::Document),
            b"document",
            &CommandContext::new("asset-document").unwrap()
        ),
        Err(Error::Unavailable(message)) if message.contains("not implemented")
    ));
    assert!(matches!(
        service.create_asset(
            &actor,
            command_for(AssetKind::Cover),
            b"not an image",
            &CommandContext::new("asset-corrupt").unwrap()
        ),
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        service.create_asset(
            &actor,
            command_for(AssetKind::Photo),
            &vec![0; 20 * 1024 * 1024 + 1],
            &CommandContext::new("asset-oversized").unwrap()
        ),
        Err(Error::Invalid(_))
    ));
    let other = service
        .create_collection(&test_authentication(&actor), "Other", &command())
        .unwrap();
    let mut png = std::io::Cursor::new(Vec::new());
    image::RgbaImage::new(1, 1)
        .write_to(&mut png, image::ImageFormat::Png)
        .unwrap();
    assert!(matches!(
        service.create_asset(
            &actor,
            CreateAssetCommand {
                collection_id: other.id,
                ..command_for(AssetKind::Scan)
            },
            png.get_ref(),
            &CommandContext::new("asset-cross-collection").unwrap()
        ),
        Err(Error::Invalid(message)) if message.contains("outside")
    ));
    assert_eq!(staging_count(), before);
    assert!(
        files_named(&root.path().join("archive"), "record.v3.json")
            .into_iter()
            .all(|path| !path.to_string_lossy().contains("/assets/"))
    );
}

#[test]
fn cover_payload_faults_recover_with_one_collection_asset() {
    for point in publication_points() {
        let root = tempfile::tempdir().unwrap();
        let archive = root.path().join("archive");
        let state = root.path().join("state");
        let store = Arc::new(Store::open(&archive, &state).unwrap());
        let user = store
            .create_user("owner", "a long test password", true)
            .unwrap();
        let actor = Principal {
            user_id: user,
            administrator: true,
            device_id: None,
        };
        let collection = store
            .create_collection(
                user,
                Collection {
                    id: CollectionId::new(),
                    revision: Revision(1),
                    name: "cover fault fixture".into(),
                    created_at: chrono::Utc::now(),
                },
            )
            .unwrap();
        let service = CollectionService::new(store.clone(), Arc::new(Registry::builtins()));
        let saved = save_inventory(&service, &actor, item(collection.id), Revision(0)).unwrap();
        drop(service);
        drop(store);

        let store = Arc::new(
            Store::open_with_publication_fault(
                &archive,
                &state,
                Arc::new(FailPublicationOnce {
                    point,
                    failed: AtomicBool::new(false),
                }),
            )
            .unwrap(),
        );
        let service = CollectionService::new(store.clone(), Arc::new(Registry::builtins()));
        let mut image = std::io::Cursor::new(Vec::new());
        image::RgbImage::new(2, 2)
            .write_to(&mut image, image::ImageFormat::Png)
            .unwrap();
        let bytes = image.into_inner();
        let context = CommandContext::new("stable-cover-command").unwrap();
        assert!(
            service
                .publish_cover(
                    &actor,
                    saved.id,
                    saved.revision,
                    &bytes,
                    "cover".into(),
                    &context,
                )
                .is_err()
        );
        drop(service);
        drop(store);

        let store = Arc::new(Store::open(&archive, &state).unwrap());
        let service = CollectionService::new(store.clone(), Arc::new(Registry::builtins()));
        let recovered = service
            .publish_cover(
                &actor,
                saved.id,
                saved.revision,
                &bytes,
                "cover".into(),
                &context,
            )
            .unwrap();
        assert_eq!(recovered.assets.len(), 1, "fault at {point:?}");
        assert_eq!(
            resource_bytes(
                service
                    .asset(&actor, recovered.id, recovered.assets[0].id, false)
                    .unwrap()
            ),
            bytes
        );
        assert!(
            !state
                .join("asset-publication")
                .join(recovered.assets[0].id.to_string())
                .exists()
        );
    }
}

#[test]
fn pagination_and_fts_do_not_cross_grants() {
    let (_root, _store, actor, service, collection) = setup();
    for title in ["Golden Sun", "Golden Axe", "Other"] {
        let mut v = item(collection.id);
        v.title = title.into();
        save_inventory(&service, &actor, v, Revision(0)).unwrap();
    }
    let first = service
        .browse_presented(
            &actor,
            BrowseQuery {
                search: Some("Golden".into()),
                limit: Some(1),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(first.items.len(), 1);
    assert!(first.next_cursor.is_some());
    let second = service
        .browse_presented(
            &actor,
            BrowseQuery {
                search: Some("Golden".into()),
                limit: Some(1),
                cursor: first.next_cursor,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(second.items.len(), 1);
    assert_ne!(first.items[0].id, second.items[0].id);
    assert!(second.next_cursor.is_none());
}
#[test]
fn revoked_devices_and_password_recovery() {
    let (_root, store, actor, service, _collection) = setup();
    let (token, device) = service
        .login("owner", "a long test password", "test")
        .unwrap();
    assert_eq!(
        service.authenticate(&token).unwrap().principal.user_id,
        actor.user_id
    );
    service
        .revoke_session(&device, device.principal.device_id.unwrap(), &command())
        .unwrap();
    assert!(service.authenticate(&token).is_err());
    let (token, _) = service
        .login("owner", "a long test password", "test")
        .unwrap();
    store.reset_password("owner", "new long password").unwrap();
    assert!(service.authenticate(&token).is_err());
    assert!(
        service
            .login("owner", "a long test password", "test")
            .is_err()
    );
    assert!(service.login("owner", "new long password", "test").is_ok());
}
#[test]
fn archive_lock_prevents_second_writer() {
    let (root, _store, _, _, _) = setup();
    assert!(matches!(
        Store::open(root.path().join("archive"), root.path().join("other-state")),
        Err(Error::Unavailable(_))
    ));
}

#[test]
fn configured_inbox_preserves_sources_and_reuses_import_evidence() {
    let (root, store, actor, service, collection) = setup();
    let saved = save_inventory(&service, &actor, item(collection.id), Revision(0)).unwrap();
    let source = root.path().join("inbox/package");
    std::fs::create_dir_all(source.join("tracks")).unwrap();
    std::fs::write(source.join("disc.cue"), b"REM synthetic package\n").unwrap();
    std::fs::write(source.join("tracks/disc.bin"), b"payload").unwrap();
    let files = Store::inventory_directory(&source).unwrap();
    assert_eq!(files.len(), 2);
    let upload = service
        .create_upload(&actor, saved.id, saved.revision, files, &command())
        .unwrap();
    let result = stage_directory(&service, &actor, upload.id, &source).unwrap();
    assert_eq!(result.captures.len(), 1);
    assert_eq!(
        std::fs::read(source.join("tracks/disc.bin")).unwrap(),
        b"payload"
    );
    assert_eq!(
        store.evidence(saved.id).unwrap()[0].outcome,
        EvidenceOutcome::Passed
    );
}

#[test]
fn configured_inbox_staging_runs_through_the_durable_worker() {
    let (root, store, actor, service, collection) = setup();
    let saved = save_inventory(&service, &actor, item(collection.id), Revision(0)).unwrap();
    let source = root.path().join("inbox/durable-package");
    std::fs::create_dir_all(source.join("tracks")).unwrap();
    std::fs::write(source.join("disc.cue"), b"REM durable package\n").unwrap();
    std::fs::write(source.join("tracks/disc.bin"), b"durable payload").unwrap();
    let files = Store::inventory_directory(&source).unwrap();
    let upload = service
        .create_upload(&actor, saved.id, saved.revision, files, &command())
        .unwrap();
    service
        .enqueue_inbox_upload(&actor, upload.id, source.to_str().unwrap(), &command())
        .unwrap();

    assert!(service.run_next_job("server-a").unwrap());
    assert_eq!(
        service.upload(&actor, upload.id).unwrap().state,
        JobState::Completed
    );
    assert_eq!(
        std::fs::read(source.join("tracks/disc.bin")).unwrap(),
        b"durable payload"
    );
    assert_eq!(store.evidence(saved.id).unwrap().len(), 1);
}
#[cfg(unix)]
#[test]
fn inbox_symlinks_are_rejected() {
    let (root, _, _, _, _) = setup();
    let source = root.path().join("inbox");
    std::fs::create_dir(&source).unwrap();
    std::os::unix::fs::symlink("/etc/passwd", source.join("escape")).unwrap();
    assert!(matches!(
        Store::inventory_directory(&source),
        Err(Error::Invalid(_))
    ));
}
#[test]
fn tampered_metadata_blocks_recovery_instead_of_silently_downgrading_revision() {
    let (root, store, actor, service, collection) = setup();
    let saved = save_inventory(&service, &actor, item(collection.id), Revision(0)).unwrap();
    drop(service);
    drop(store);
    fn corrupt(path: &std::path::Path) {
        for entry in std::fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                corrupt(&path);
            } else if matches!(
                path.file_name().and_then(|name| name.to_str()),
                Some("record.json" | "record.v3.json")
            ) {
                std::fs::write(path, b"{}").unwrap();
            }
        }
    }
    corrupt(&root.path().join("archive/collections"));
    assert!(matches!(
        Store::open(root.path().join("archive"), root.path().join("state")),
        Err(Error::Storage(_))
    ));
    assert_eq!(saved.revision, Revision(1));
}
#[test]
fn failed_background_import_is_reported_and_retains_source_chunks() {
    let (_root, _store, actor, service, collection) = setup();
    let saved = save_inventory(&service, &actor, item(collection.id), Revision(0)).unwrap();
    let upload = service
        .create_upload(
            &actor,
            saved.id,
            saved.revision,
            vec![UploadFile {
                name: "disc.bin".into(),
                bytes: ByteLength(4),
                sha256: None,
            }],
            &command(),
        )
        .unwrap();
    service
        .upload_chunk(&actor, upload.id, 0, 0, b"data")
        .unwrap();
    service
        .fail_upload(&actor, upload.id, "Test failure")
        .unwrap();
    let job = service
        .jobs(&actor)
        .unwrap()
        .into_iter()
        .find(|j| j.id == upload.id)
        .unwrap();
    assert_eq!(job.state, JobState::Failed);
    assert_eq!(job.error.as_deref(), Some("Test failure"));
    assert!(service.commit_upload(&actor, upload.id, &command()).is_ok());
}

#[test]
fn durable_job_rows_store_typed_state_without_losing_upload_identity() {
    let (root, _store, actor, service, collection) = setup();
    let saved = save_inventory(&service, &actor, item(collection.id), Revision(0)).unwrap();
    let upload = service
        .create_upload(
            &actor,
            saved.id,
            saved.revision,
            vec![UploadFile {
                name: "disc.bin".into(),
                bytes: ByteLength(4),
                sha256: None,
            }],
            &command(),
        )
        .unwrap();

    let db = rusqlite::Connection::open(root.path().join("state/private.sqlite")).unwrap();
    let columns = db
        .prepare("SELECT name FROM pragma_table_info('jobs') ORDER BY cid")
        .unwrap()
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .collect::<std::result::Result<Vec<_>, _>>()
        .unwrap();
    for required in [
        "user_id",
        "item_id",
        "kind",
        "state",
        "phase",
        "completed_units",
        "total_units",
        "attempt",
        "request",
        "result",
        "worker_instance",
        "lease_expires",
        "claim_token",
        "cancellation_requested",
    ] {
        assert!(
            columns.iter().any(|column| column == required),
            "{required}"
        );
    }
    let (id, state, completed, total): (String, String, u64, u64) = db
        .query_row(
            "SELECT id,state,completed_units,total_units FROM jobs WHERE id=?",
            [upload.id.to_string()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .unwrap();
    assert_eq!(id, upload.id.to_string());
    assert_eq!(state, "running");
    assert_eq!(completed, 0);
    assert_eq!(total, 4);
}

#[test]
fn durable_claims_fence_stale_workers_and_limit_one_active_job_per_user() {
    let root = tempfile::tempdir().unwrap();
    let clock = Arc::new(TestClock::new(1_800_000_000));
    let store = Store::open_with_clock(
        root.path().join("archive"),
        root.path().join("state"),
        clock.clone(),
    )
    .unwrap();
    let user = UserId::new();
    let first = JobSpec {
        id: JobId::new(),
        user_id: user,
        collection_id: None,
        item_id: None,
        kind: "test.first".into(),
        phase: "queued".into(),
        total_units: 1,
        request: serde_json::json!({"fixture":1}),
    };
    let second = JobSpec {
        id: JobId::new(),
        kind: "test.second".into(),
        request: serde_json::json!({"fixture":2}),
        ..first.clone()
    };
    store.enqueue_job(first.clone()).unwrap();
    store.enqueue_job(second.clone()).unwrap();

    let original = store.claim_job("server-a", 30).unwrap().unwrap();
    assert_eq!(original.id, first.id);
    assert!(store.claim_job("server-a", 30).unwrap().is_none());
    assert!(matches!(
        store.complete_job(original.id, "wrong-token", serde_json::json!({})),
        Err(Error::CommandInProgress)
    ));

    clock.advance(31);
    let recovered = store.claim_job("server-b", 30).unwrap().unwrap();
    assert_eq!(recovered.id, original.id);
    assert_ne!(recovered.claim_token, original.claim_token);
    assert!(matches!(
        store.complete_job(original.id, &original.claim_token, serde_json::json!({})),
        Err(Error::CommandInProgress)
    ));
    assert!(matches!(
        store.renew_job_lease(original.id, &original.claim_token, 30),
        Err(Error::CommandInProgress)
    ));
    store
        .complete_job(
            recovered.id,
            &recovered.claim_token,
            serde_json::json!({"completed":true}),
        )
        .unwrap();
    assert_eq!(
        store.claim_job("server-b", 30).unwrap().unwrap().id,
        second.id
    );
}

#[test]
fn durable_claims_enforce_the_global_limit_in_storage() {
    let root = tempfile::tempdir().unwrap();
    let store = Store::open(root.path().join("archive"), root.path().join("state")).unwrap();
    let jobs = (0..3)
        .map(|fixture| JobSpec {
            id: JobId::new(),
            user_id: UserId::new(),
            collection_id: None,
            item_id: None,
            kind: format!("test.{fixture}"),
            phase: "queued".into(),
            total_units: 1,
            request: serde_json::json!({"fixture":fixture}),
        })
        .collect::<Vec<_>>();
    for job in &jobs {
        store.enqueue_job(job.clone()).unwrap();
    }

    let first = store.claim_job("server-a", 30).unwrap().unwrap();
    let second = store.claim_job("server-a", 30).unwrap().unwrap();
    assert_ne!(first.user_id, second.user_id);
    assert!(store.claim_job("server-a", 30).unwrap().is_none());
}

#[test]
fn durable_progress_is_monotonic_and_throttled_with_immediate_phase_changes() {
    let root = tempfile::tempdir().unwrap();
    let clock = Arc::new(TestClock::new(1_800_000_000));
    let state = root.path().join("state");
    let store = Store::open_with_clock(root.path().join("archive"), &state, clock.clone()).unwrap();
    let job = JobSpec {
        id: JobId::new(),
        user_id: UserId::new(),
        collection_id: None,
        item_id: None,
        kind: "test.progress".into(),
        phase: "queued".into(),
        total_units: 3,
        request: serde_json::json!({}),
    };
    store.enqueue_job(job).unwrap();
    let claim = store.claim_job("server-a", 30).unwrap().unwrap();

    store
        .heartbeat_job(claim.id, &claim.claim_token, "reading", 1, 30)
        .unwrap();
    store
        .heartbeat_job(claim.id, &claim.claim_token, "reading", 2, 30)
        .unwrap();
    let db = rusqlite::Connection::open(state.join("private.sqlite")).unwrap();
    let first: (String, u64) = db
        .query_row(
            "SELECT phase,completed_units FROM jobs WHERE id=?",
            [claim.id.to_string()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(first, ("reading".into(), 1));

    store
        .heartbeat_job(claim.id, &claim.claim_token, "hashing", 2, 30)
        .unwrap();
    let phase_change: (String, u64) = db
        .query_row(
            "SELECT phase,completed_units FROM jobs WHERE id=?",
            [claim.id.to_string()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(phase_change, ("hashing".into(), 2));

    assert!(matches!(
        store.heartbeat_job(claim.id, &claim.claim_token, "hashing", 1, 30),
        Err(Error::CommandInProgress)
    ));
    clock.advance(1);
    store
        .heartbeat_job(claim.id, &claim.claim_token, "hashing", 3, 30)
        .unwrap();
}

#[test]
fn upload_source_staging_is_not_claimable_until_commit_is_enqueued() {
    let (_root, store, actor, service, collection) = setup();
    let (_saved, upload) = ready_upload(&store, &actor, &service, collection.id, b"ready");
    assert!(store.claim_job("server-a", 30).unwrap().is_none());

    assert_eq!(
        service
            .enqueue_upload_commit(&actor, upload.id, &command())
            .unwrap(),
        upload.id
    );
    assert!(service.run_next_job("server-a").unwrap());
    assert_eq!(
        service.upload(&actor, upload.id).unwrap().state,
        JobState::Completed
    );
}

#[test]
fn queued_and_running_upload_cancellation_retain_source_material() {
    let (root, store, actor, service, collection) = setup();
    let (_saved, queued) = ready_upload(&store, &actor, &service, collection.id, b"queued");
    service
        .enqueue_upload_commit(&actor, queued.id, &command())
        .unwrap();
    service
        .cancel_upload(&actor, queued.id, &command())
        .unwrap();
    assert_eq!(
        service.upload(&actor, queued.id).unwrap().state,
        JobState::Cancelled
    );
    assert!(store.claim_job("server-a", 30).unwrap().is_none());

    let saved = save_inventory(&service, &actor, item(collection.id), Revision(0)).unwrap();
    let running = service
        .create_upload(
            &actor,
            saved.id,
            saved.revision,
            vec![UploadFile {
                name: "running.bin".into(),
                bytes: ByteLength(7),
                sha256: None,
            }],
            &command(),
        )
        .unwrap();
    service
        .upload_chunk(&actor, running.id, 0, 0, b"running")
        .unwrap();
    service
        .enqueue_upload_commit(&actor, running.id, &command())
        .unwrap();
    let claim = store.claim_job("server-a", 30).unwrap().unwrap();
    service
        .cancel_upload(&actor, running.id, &command())
        .unwrap();
    assert!(
        store
            .cancellation_requested(claim.id, &claim.claim_token)
            .unwrap()
    );
    assert!(matches!(
        store.commit_upload_for_job(claim.id, &claim.claim_token),
        Err(Error::JobCancelled)
    ));
    store
        .cancel_claimed_job(claim.id, &claim.claim_token)
        .unwrap();
    assert_eq!(
        service.upload(&actor, running.id).unwrap().state,
        JobState::Cancelled
    );
    assert!(find_file_with_bytes(&root.path().join("state"), b"running").is_some());
}

#[test]
fn failed_upload_retry_reuses_job_and_publication_identities() {
    let bytes = b"retry source";
    let (root, store, actor, service, collection) = setup();
    let (_saved, upload) = ready_upload(&store, &actor, &service, collection.id, bytes);
    drop(service);
    drop(store);
    let faulted = Arc::new(
        Store::open_with_publication_fault(
            root.path().join("archive"),
            root.path().join("state"),
            Arc::new(FailPublicationOnce {
                point: collector_store::PublicationFaultPoint::Prepared,
                failed: AtomicBool::new(false),
            }),
        )
        .unwrap(),
    );
    let service = CollectionService::new(faulted.clone(), Arc::new(Registry::builtins()));
    service
        .enqueue_upload_commit(&actor, upload.id, &command())
        .unwrap();
    assert!(service.run_next_job("server-a").unwrap());
    assert_eq!(
        service.upload(&actor, upload.id).unwrap().state,
        JobState::Failed
    );

    assert_eq!(
        service
            .retry_upload_job(&actor, upload.id, &command())
            .unwrap(),
        upload.id
    );
    assert!(service.run_next_job("server-a").unwrap());
    let retried_job = service
        .jobs(&actor)
        .unwrap()
        .into_iter()
        .find(|job| job.id == upload.id)
        .unwrap();
    assert_eq!(
        retried_job.state,
        JobState::Completed,
        "retry failed: {:?}",
        retried_job.error
    );
    let item = faulted.item(upload.item_id).unwrap();
    assert!(
        item.captures
            .iter()
            .any(|capture| capture.id == upload.capture_id)
    );
    assert!(item.captures.iter().any(|capture| {
        capture
            .representations
            .iter()
            .any(|representation| representation.id == upload.representation_id)
    }));
    let private = rusqlite::Connection::open(root.path().join("state/private.sqlite")).unwrap();
    let (attempt, history): (u32, String) = private
        .query_row(
            "SELECT attempt,result FROM jobs WHERE id=?",
            [upload.id.to_string()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(attempt, 2);
    let history: serde_json::Value = serde_json::from_str(&history).unwrap();
    assert_eq!(history["prior_failures"].as_array().unwrap().len(), 1);
}

#[test]
fn cancellation_after_revision_commit_has_a_stable_too_late_result() {
    let bytes = b"committed source";
    let (root, store, actor, service, collection) = setup();
    let (_saved, upload) = ready_upload(&store, &actor, &service, collection.id, bytes);
    drop(service);
    drop(store);
    let faulted = Arc::new(
        Store::open_with_publication_fault(
            root.path().join("archive"),
            root.path().join("state"),
            Arc::new(FailPublicationOnce {
                point: collector_store::PublicationFaultPoint::RevisionCommitted,
                failed: AtomicBool::new(false),
            }),
        )
        .unwrap(),
    );
    let service = CollectionService::new(faulted, Arc::new(Registry::builtins()));
    service
        .enqueue_upload_commit(&actor, upload.id, &command())
        .unwrap();
    assert!(service.run_next_job("server-a").unwrap());
    assert!(matches!(
        service.cancel_upload(&actor, upload.id, &command()),
        Err(Error::CancellationTooLate)
    ));
}

#[test]
fn cancellation_at_payload_boundary_aborts_before_revision_commit() {
    struct PauseAtPayload {
        arrived: std::sync::Barrier,
        release: std::sync::Barrier,
        paused: AtomicBool,
    }
    impl collector_store::PublicationFaultInjector for PauseAtPayload {
        fn check(&self, point: collector_store::PublicationFaultPoint) -> Result<()> {
            if point == collector_store::PublicationFaultPoint::PayloadPublished
                && !self.paused.swap(true, Ordering::SeqCst)
            {
                self.arrived.wait();
                self.release.wait();
            }
            Ok(())
        }
    }

    let (root, store, actor, service, collection) = setup();
    let (saved, upload) = ready_upload(&store, &actor, &service, collection.id, b"cancel me");
    drop(service);
    drop(store);
    let pause = Arc::new(PauseAtPayload {
        arrived: std::sync::Barrier::new(2),
        release: std::sync::Barrier::new(2),
        paused: AtomicBool::new(false),
    });
    let store = Arc::new(
        Store::open_with_publication_fault(
            root.path().join("archive"),
            root.path().join("state"),
            pause.clone(),
        )
        .unwrap(),
    );
    let service = CollectionService::new(store.clone(), Arc::new(Registry::builtins()));
    service
        .enqueue_upload_commit(&actor, upload.id, &command())
        .unwrap();
    let worker = {
        let service = service.clone();
        std::thread::spawn(move || service.run_next_job("server-a"))
    };
    pause.arrived.wait();
    service
        .cancel_upload(&actor, upload.id, &command())
        .unwrap();
    pause.release.wait();
    assert!(worker.join().unwrap().unwrap());

    assert_eq!(
        service.upload(&actor, upload.id).unwrap().state,
        JobState::Cancelled
    );
    assert!(store.item(saved.id).unwrap().captures.is_empty());
    assert!(store.evidence(saved.id).unwrap().is_empty());
    assert!(
        std::fs::read_dir(root.path().join("archive/.transactions"))
            .unwrap()
            .next()
            .is_none()
    );
    assert!(find_file_with_bytes(&root.path().join("state"), b"cancel me").is_some());
}

#[test]
fn integrity_verification_runs_as_a_real_durable_job() {
    let (_root, store, actor, service, collection) = setup();
    let saved = save_inventory(&service, &actor, item(collection.id), Revision(0)).unwrap();
    let upload = service
        .create_upload(
            &actor,
            saved.id,
            saved.revision,
            vec![UploadFile {
                name: "disc.bin".into(),
                bytes: ByteLength(4),
                sha256: None,
            }],
            &command(),
        )
        .unwrap();
    service
        .upload_chunk(&actor, upload.id, 0, 0, b"data")
        .unwrap();
    service
        .commit_upload(&actor, upload.id, &command())
        .unwrap();
    let before = store.evidence(saved.id).unwrap().len();

    let key = command();
    let job = service
        .enqueue_integrity_verification(&actor, saved.id, &key)
        .unwrap();
    assert_eq!(
        service
            .enqueue_integrity_verification(&actor, saved.id, &key)
            .unwrap(),
        job
    );
    assert!(service.run_next_job("test-server").unwrap());
    assert_eq!(store.evidence(saved.id).unwrap().len(), before + 1);
    assert_eq!(
        service
            .jobs(&actor)
            .unwrap()
            .into_iter()
            .find(|candidate| candidate.id == job)
            .unwrap()
            .state,
        JobState::Completed
    );
}

#[test]
fn reclaimed_integrity_job_does_not_publish_twice() {
    let (root, store, actor, service, collection) = setup();
    let saved = save_inventory(&service, &actor, item(collection.id), Revision(0)).unwrap();
    let upload = service
        .create_upload(
            &actor,
            saved.id,
            saved.revision,
            vec![UploadFile {
                name: "disc.bin".into(),
                bytes: ByteLength(4),
                sha256: None,
            }],
            &command(),
        )
        .unwrap();
    service
        .upload_chunk(&actor, upload.id, 0, 0, b"data")
        .unwrap();
    service
        .commit_upload(&actor, upload.id, &command())
        .unwrap();
    let job = service
        .enqueue_integrity_verification(&actor, saved.id, &command())
        .unwrap();
    let claim = store.claim_job("server-before-crash", 30).unwrap().unwrap();
    assert_eq!(claim.id, job);
    store
        .verify_integrity_for_job(
            saved.id,
            job,
            &claim.claim_token,
            claim.created_at,
            &mut |_| Ok(()),
        )
        .unwrap();
    let evidence_after_publication = store.evidence(saved.id).unwrap().len();
    let revision_after_publication = store.item(saved.id).unwrap().revision;
    drop(service);
    drop(store);
    let store =
        Arc::new(Store::open(root.path().join("archive"), root.path().join("state")).unwrap());
    let service = CollectionService::new(store.clone(), Arc::new(Registry::builtins()));

    assert!(service.run_next_job("server-after-restart").unwrap());
    assert_eq!(
        store.evidence(saved.id).unwrap().len(),
        evidence_after_publication
    );
    assert_eq!(
        store.item(saved.id).unwrap().revision,
        revision_after_publication
    );
    assert_eq!(
        service
            .jobs(&actor)
            .unwrap()
            .into_iter()
            .find(|candidate| candidate.id == job)
            .unwrap()
            .state,
        JobState::Completed
    );
}

#[test]
fn integrity_job_observes_cancellation_between_files() {
    let (_root, store, actor, service, collection) = setup();
    let saved = save_inventory(&service, &actor, item(collection.id), Revision(0)).unwrap();
    let upload = service
        .create_upload(
            &actor,
            saved.id,
            saved.revision,
            vec![
                UploadFile {
                    name: "one.bin".into(),
                    bytes: ByteLength(3),
                    sha256: None,
                },
                UploadFile {
                    name: "two.bin".into(),
                    bytes: ByteLength(3),
                    sha256: None,
                },
            ],
            &command(),
        )
        .unwrap();
    service
        .upload_chunk(&actor, upload.id, 0, 0, b"one")
        .unwrap();
    service
        .upload_chunk(&actor, upload.id, 1, 0, b"two")
        .unwrap();
    service
        .commit_upload(&actor, upload.id, &command())
        .unwrap();
    let before = store.evidence(saved.id).unwrap().len();
    let job = service
        .enqueue_integrity_verification(&actor, saved.id, &command())
        .unwrap();
    let claim = store.claim_job("server-a", 30).unwrap().unwrap();
    let mut progress = |completed| {
        if completed == 1 {
            store.request_job_cancellation(job, actor.user_id)?;
        }
        if store.cancellation_requested(job, &claim.claim_token)? {
            Err(Error::JobCancelled)
        } else {
            Ok(())
        }
    };
    assert!(matches!(
        store.verify_integrity_for_job(
            saved.id,
            job,
            &claim.claim_token,
            claim.created_at,
            &mut progress,
        ),
        Err(Error::JobCancelled)
    ));
    store.cancel_claimed_job(job, &claim.claim_token).unwrap();
    assert_eq!(store.evidence(saved.id).unwrap().len(), before);
}

#[test]
fn job_views_are_cursor_paged_and_omit_execution_secrets() {
    let root = tempfile::tempdir().unwrap();
    let store =
        Arc::new(Store::open(root.path().join("archive"), root.path().join("state")).unwrap());
    let user = UserId::new();
    let other = UserId::new();
    let mut ids = Vec::new();
    for fixture in 0..3 {
        let id = JobId::new();
        ids.push(id);
        store
            .enqueue_job(JobSpec {
                id,
                user_id: user,
                collection_id: None,
                item_id: None,
                kind: "integrity_verification".into(),
                phase: "queued".into(),
                total_units: 1,
                request: serde_json::json!({"private_marker":format!("secret-{fixture}")}),
            })
            .unwrap();
    }
    let first = store.job_records(user, None, 2).unwrap();
    assert_eq!(first.items.len(), 2);
    let second = store
        .job_records(user, Some(first.next_cursor.unwrap().parse().unwrap()), 2)
        .unwrap();
    assert_eq!(second.items.len(), 1);
    assert!(second.next_cursor.is_none());
    let listed = first
        .items
        .into_iter()
        .chain(second.items)
        .map(|job| job.id)
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(listed, ids.into_iter().collect());
    assert!(matches!(
        store.job_record(listed.iter().copied().next().unwrap(), other),
        Err(Error::NotFound)
    ));

    let service = CollectionService::new(store.clone(), Arc::new(Registry::builtins()));
    let actor = Principal {
        user_id: user,
        administrator: false,
        device_id: None,
    };
    let claim = store.claim_job("private-worker", 30).unwrap().unwrap();
    let serialized = serde_json::to_string(&service.job(&actor, claim.id).unwrap()).unwrap();
    assert!(!serialized.contains("private_marker"));
    assert!(!serialized.contains(&claim.claim_token));
    assert!(!serialized.contains("private-worker"));
}

#[test]
fn failed_integrity_job_retries_with_the_same_identity_and_attempt_history() {
    let (_root, store, actor, service, collection) = setup();
    let saved = save_inventory(&service, &actor, item(collection.id), Revision(0)).unwrap();
    let upload = service
        .create_upload(
            &actor,
            saved.id,
            saved.revision,
            vec![UploadFile {
                name: "retry-integrity.bin".into(),
                bytes: ByteLength(4),
                sha256: None,
            }],
            &command(),
        )
        .unwrap();
    service
        .upload_chunk(&actor, upload.id, 0, 0, b"data")
        .unwrap();
    service
        .commit_upload(&actor, upload.id, &command())
        .unwrap();
    let job = service
        .enqueue_integrity_verification(&actor, saved.id, &command())
        .unwrap();
    let claim = store.claim_job("server-a", 30).unwrap().unwrap();
    store
        .fail_job(job, &claim.claim_token, "injected tool failure")
        .unwrap();

    assert_eq!(service.retry_job(&actor, job, &command()).unwrap(), job);
    let queued = service.job(&actor, job).unwrap();
    assert_eq!(queued.state, JobState::Queued);
    assert_eq!(queued.attempt, 2);
    assert!(service.run_next_job("server-a").unwrap());
    assert_eq!(service.job(&actor, job).unwrap().state, JobState::Completed);
}

#[test]
fn review_capture_update_survives_reindex() {
    let (_root, store, actor, service, collection) = setup();
    let saved = save_inventory(&service, &actor, item(collection.id), Revision(0)).unwrap();
    let upload = service
        .create_upload(
            &actor,
            saved.id,
            saved.revision,
            vec![UploadFile {
                name: "disc.bin".into(),
                bytes: ByteLength(4),
                sha256: None,
            }],
            &command(),
        )
        .unwrap();
    service
        .upload_chunk(&actor, upload.id, 0, 0, b"data")
        .unwrap();
    let uploaded = service
        .commit_upload(&actor, upload.id, &command())
        .unwrap();
    let mut capture = uploaded.captures[0].clone();
    capture.association = CaptureAssociation::default();
    let revised = save_capture_record(&service, &actor, capture.clone(), capture.revision).unwrap();
    assert_eq!(
        service.capture(&actor, capture.id).unwrap().revision,
        revised.revision
    );
    store.reindex().unwrap();
    assert_eq!(
        service.capture(&actor, capture.id).unwrap().revision,
        revised.revision,
        "canonical capture revision regressed"
    );
}

#[test]
fn review_cannot_move_another_users_capture() {
    let (_root, store, owner, service, private_collection) = setup();
    let capture = save_capture_record(
        &service,
        &owner,
        Capture {
            id: CaptureId::new(),
            collection_id: private_collection.id,
            revision: Revision(0),
            association: CaptureAssociation::default(),
            created_at: chrono::Utc::now(),
            representations: vec![],
        },
        Revision(0),
    )
    .unwrap();
    let attacker_id = store
        .create_user("other", "another long password", true)
        .unwrap();
    let attacker = Principal {
        user_id: attacker_id,
        administrator: true,
        device_id: None,
    };
    let other_collection = service
        .create_collection(&test_authentication(&attacker), "Other", &command())
        .unwrap();
    assert!(service.capture(&attacker, capture.id).is_err());
    let mut stolen = capture.clone();
    stolen.collection_id = other_collection.id;
    assert!(
        save_capture_record(&service, &attacker, stolen, capture.revision).is_err(),
        "unauthorized capture move was accepted"
    );
}

#[test]
fn review_incomplete_migration_record_is_recoverable() {
    let root = tempfile::tempdir().unwrap();
    let archive = root.path().join("archive");
    copy_tree(
        Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/schema-1"
        )),
        &archive,
    );
    let partial = archive.join("collections/synthetic--018f0000-0000-7000-8000-000000000001/items/two-disc--018f0000-0000-7000-8000-000000000010/revisions/00000000000000000001/record.v2.json");
    std::fs::write(&partial, b"{\"item\":").unwrap();
    assert!(
        Store::open(&archive, root.path().join("state")).is_ok(),
        "an interrupted uncommitted migration cannot recover"
    );
}

#[test]
fn review_same_specimen_cannot_belong_to_two_items() {
    let (_root, store, _actor, _service, collection) = setup();
    let mut first = item(collection.id);
    first.specimens.push(Specimen {
        id: SpecimenId::new(),
        label: "copy".into(),
        components: vec![],
        condition: Condition::Unknown,
        ownership: Ownership::Owned,
        possession: Possession::Present,
        acquisition: None,
        private_notes: None,
        loans: vec![],
    });
    let mut second = first.clone();
    second.id = ItemId::new();
    Catalog::save(&*store, first, Revision(0)).unwrap();
    assert!(
        Catalog::save(&*store, second, Revision(0)).is_err(),
        "duplicate physical identity was accepted"
    );
}

#[test]
fn review_capture_links_must_form_chain_when_middle_links_omitted() {
    let (_root, _store, actor, service, collection) = setup();
    let first = save_inventory(&service, &actor, item(collection.id), Revision(0)).unwrap();
    let mut second = item(collection.id);
    let carrier_id = CarrierId::new();
    second.specimens.push(Specimen {
        id: SpecimenId::new(),
        label: "copy".into(),
        components: vec![Component {
            id: ComponentId::new(),
            label: "disc".into(),
            position: 1,
            kind: ComponentKind::OpticalDisc,
            carrier: Some(Carrier {
                id: carrier_id,
                label: "disc".into(),
                kind: CarrierKind::OpticalDisc,
            }),
            migration_provenance: None,
        }],
        condition: Condition::Unknown,
        ownership: Ownership::Owned,
        possession: Possession::Present,
        acquisition: None,
        private_notes: None,
        loans: vec![],
    });
    save_inventory(&service, &actor, second, Revision(0)).unwrap();
    let capture = Capture {
        id: CaptureId::new(),
        collection_id: collection.id,
        revision: Revision(0),
        association: CaptureAssociation {
            item_id: Some(first.id),
            carrier_id: Some(carrier_id),
            ..Default::default()
        },
        created_at: chrono::Utc::now(),
        representations: vec![],
    };
    assert!(
        save_capture_record(&service, &actor, capture, Revision(0)).is_err(),
        "links to one item and another item's carrier were accepted"
    );
}

#[test]
fn capture_reassociation_retains_payload_and_survives_restart() {
    let root = tempfile::tempdir().unwrap();
    let archive = root.path().join("archive");
    copy_tree(
        Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/schema-1"
        )),
        &archive,
    );
    let store = Arc::new(Store::open(&archive, root.path().join("state")).unwrap());
    let user = store
        .create_user("owner", "a long test password", true)
        .unwrap();
    let original = store
        .item("018f0000-0000-7000-8000-000000000010".parse().unwrap())
        .unwrap();
    store
        .grant(original.collection_id, user, Role::Owner)
        .unwrap();
    let actor = Principal {
        user_id: user,
        administrator: true,
        device_id: None,
    };
    let service = CollectionService::new(store.clone(), Arc::new(Registry::builtins()));
    let new_item =
        save_inventory(&service, &actor, item(original.collection_id), Revision(0)).unwrap();
    let mut capture = original.captures[0].clone();
    capture.association = CaptureAssociation {
        item_id: Some(new_item.id),
        ..Default::default()
    };
    let updated = save_capture_record(&service, &actor, capture.clone(), capture.revision).unwrap();
    // A stale item compatibility snapshot cannot downgrade the capture.
    assert!(matches!(
        store.save(original.clone(), original.revision),
        Err(Error::Conflict)
    ));
    store.reindex().unwrap();
    let moved = service.item_presented(&actor, new_item.id).unwrap();
    assert_eq!(moved.captures[0].revision, updated.revision);
    let representation = moved.captures[0].representations[0].id;
    let resource = service
        .representation(&actor, new_item.id, representation, 0)
        .unwrap();
    assert_eq!(resource_bytes(resource), b"disc-one\n");
    assert!(
        serde_json::from_value::<collector_app::SaveCaptureCommand>(serde_json::json!({
            "collection_id": updated.collection_id,
            "association": updated.association,
            "legacy_payload_locator": {"item_id": new_item.id, "relative_path": "/private"}
        }))
        .is_err(),
        "capture commands accepted a private archive locator"
    );
    drop(service);
    drop(store);
    let reopened = Store::open(&archive, root.path().join("state")).unwrap();
    assert_eq!(
        reopened.capture(updated.id).unwrap().revision,
        updated.revision
    );
    assert_eq!(
        reopened.item(new_item.id).unwrap().captures[0].id,
        updated.id
    );
}

#[test]
fn incomplete_typed_migration_output_recovers_but_committed_sidecars_do_not_change() {
    let root = tempfile::tempdir().unwrap();
    let archive = root.path().join("archive");
    copy_tree(
        Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/schema-1"
        )),
        &archive,
    );
    let collection = archive.join("collections/synthetic--018f0000-0000-7000-8000-000000000001");
    let partial = collection
        .join("works/018f0000-0000-7000-8000-000000000020/revisions/00000000000000000001");
    std::fs::create_dir_all(&partial).unwrap();
    std::fs::write(partial.join("record.json"), b"{").unwrap();
    let store = Store::open(&archive, root.path().join("state")).unwrap();
    drop(store);
    // A checkpoint exists, so missing/modified output must fail instead of being silently regenerated.
    std::fs::remove_file(archive.join(".archive-migrations/schema-1-to-2/complete")).unwrap();
    let sidecar = collection.join("items/two-disc--018f0000-0000-7000-8000-000000000010/revisions/00000000000000000001/record.v2.json");
    std::fs::write(sidecar, b"{}").unwrap();
    assert!(
        matches!(Store::open(&archive, root.path().join("state")), Err(Error::Storage(message)) if message.contains("committed migration output"))
    );
}

#[test]
fn nested_identity_cannot_be_reused_across_collections() {
    let (_root, store, actor, service, collection) = setup();
    let mut first = item(collection.id);
    first.specimens.push(Specimen {
        id: SpecimenId::new(),
        label: "copy".into(),
        components: vec![],
        condition: Condition::Unknown,
        ownership: Ownership::Owned,
        possession: Possession::Present,
        acquisition: None,
        private_notes: None,
        loans: vec![],
    });
    let mut second = first.clone();
    second.id = ItemId::new();
    second.collection_id = service
        .create_collection(&test_authentication(&actor), "Other", &command())
        .unwrap()
        .id;
    Catalog::save(&*store, first, Revision(0)).unwrap();
    assert!(
        matches!(Catalog::save(&*store, second, Revision(0)), Err(Error::Invalid(message)) if message.contains("across collections"))
    );
}

#[test]
fn focused_inventory_commands_preserve_history_and_are_idempotent() {
    let (root, store, actor, service, collection) = setup();
    let base = save_inventory(&service, &actor, item(collection.id), Revision(0)).unwrap();
    assert!(base.wishlist);

    let specimen_context = CommandContext::new("specimen-one").unwrap();
    let first = service
        .create_specimen(
            &actor,
            base.id,
            base.revision,
            SpecimenDetails {
                label: "Shop copy".into(),
                condition: Condition::Good,
                ownership: ActiveOwnership::Owned,
                possession: Possession::Present,
                acquisition: Some(Acquisition {
                    date: Some(chrono::NaiveDate::from_ymd_opt(2026, 9, 1).unwrap()),
                    cost: None,
                    source: Some("Koenji".into()),
                    coordinates: None,
                }),
                private_notes: Some("Obi retained".into()),
            },
            &specimen_context,
        )
        .unwrap();
    let replay = service
        .create_specimen(
            &actor,
            base.id,
            base.revision,
            SpecimenDetails {
                label: "Shop copy".into(),
                condition: Condition::Good,
                ownership: ActiveOwnership::Owned,
                possession: Possession::Present,
                acquisition: Some(Acquisition {
                    date: Some(chrono::NaiveDate::from_ymd_opt(2026, 9, 1).unwrap()),
                    cost: None,
                    source: Some("Koenji".into()),
                    coordinates: None,
                }),
                private_notes: Some("Obi retained".into()),
            },
            &specimen_context,
        )
        .unwrap();
    assert_eq!(replay.revision, first.revision);
    assert_eq!(replay.specimens.len(), 1);
    let first_specimen = first.specimens[0].id;

    let second = service
        .create_specimen(
            &actor,
            base.id,
            first.revision,
            SpecimenDetails {
                label: "Listening copy".into(),
                condition: Condition::Fair,
                ownership: ActiveOwnership::Borrowed,
                possession: Possession::Elsewhere,
                acquisition: None,
                private_notes: None,
            },
            &CommandContext::new("specimen-two").unwrap(),
        )
        .unwrap();
    assert_ne!(second.specimens[0].id, second.specimens[1].id);
    let second_specimen = second.specimens[1].id;
    let second = service
        .update_specimen(
            &actor,
            base.id,
            second_specimen,
            second.revision,
            SpecimenDetails {
                label: "Listening copy".into(),
                condition: Condition::Poor,
                ownership: ActiveOwnership::Borrowed,
                possession: Possession::Present,
                acquisition: None,
                private_notes: Some("Jewel case cracked".into()),
            },
            &CommandContext::new("update-specimen-two").unwrap(),
        )
        .unwrap();
    assert_eq!(second.specimens[1].possession, Possession::Present);

    let loaned = service
        .start_loan(
            &actor,
            base.id,
            first_specimen,
            second.revision,
            StartLoanCommand {
                contact: "A trusted friend".into(),
                lent_at: chrono::NaiveDate::from_ymd_opt(2026, 9, 2).unwrap(),
                due_at: None,
            },
            &CommandContext::new("loan-one").unwrap(),
        )
        .unwrap();
    assert_eq!(loaned.specimens[0].possession, Possession::Lent);
    let loan_id = loaned.specimens[0].loans[0].id;
    assert!(matches!(
        service.dispose_specimen(
            &actor,
            base.id,
            first_specimen,
            loaned.revision,
            DisposeSpecimenCommand {
                date: chrono::NaiveDate::from_ymd_opt(2026, 9, 3).unwrap(),
                reason: "Cannot dispose while lent".into(),
                possession: ReturnedPossession::Elsewhere,
            },
            &CommandContext::new("reject-active-loan-disposal").unwrap(),
        ),
        Err(Error::Conflict)
    ));
    let returned = service
        .return_loan(
            &actor,
            (base.id, first_specimen, loan_id),
            loaned.revision,
            ReturnLoanCommand {
                returned_at: chrono::NaiveDate::from_ymd_opt(2026, 9, 4).unwrap(),
                possession: ReturnedPossession::Present,
            },
            &CommandContext::new("return-one").unwrap(),
        )
        .unwrap();
    assert_eq!(returned.specimens[0].possession, Possession::Present);
    assert_eq!(returned.specimens[0].loans.len(), 1);
    assert!(returned.specimens[0].loans[0].returned_at.is_some());

    let mut broad = SaveItemCommand::from(returned.clone());
    broad.title = "Edited without inventory payload".into();
    let edited = service
        .save_item(
            &actor,
            base.id,
            broad,
            returned.revision,
            &CommandContext::new("broad-edit").unwrap(),
        )
        .unwrap();
    assert_eq!(edited.specimens.len(), 2);
    assert_eq!(edited.specimens[0].loans.len(), 1);

    service
        .adapt_wave1_item_save(&actor, edited.id, edited.clone(), edited.revision)
        .unwrap();

    let mut legacy = edited.clone();
    legacy.specimens[0].loans.clear();
    assert!(
        service
            .adapt_wave1_item_save(&actor, edited.id, legacy, edited.revision)
            .is_err(),
        "the Wave 1 envelope erased server-owned loan history"
    );

    save_capture_record(
        &service,
        &actor,
        Capture {
            id: CaptureId::new(),
            collection_id: collection.id,
            revision: Revision(0),
            association: CaptureAssociation {
                item_id: Some(base.id),
                specimen_id: Some(first_specimen),
                ..Default::default()
            },
            created_at: chrono::Utc::now(),
            representations: vec![],
        },
        Revision(0),
    )
    .unwrap();
    let current = store.item(base.id).unwrap();
    let mut png = std::io::Cursor::new(Vec::new());
    image::RgbaImage::new(2, 2)
        .write_to(&mut png, image::ImageFormat::Png)
        .unwrap();
    let with_artwork = service
        .publish_cover(
            &actor,
            base.id,
            current.revision,
            png.get_ref(),
            "Preserved cover".into(),
            &CommandContext::new("lifecycle-cover").unwrap(),
        )
        .unwrap();
    let asset_id = with_artwork.assets[0].id;

    let disposed = service
        .dispose_specimen(
            &actor,
            base.id,
            first_specimen,
            with_artwork.revision,
            DisposeSpecimenCommand {
                date: chrono::NaiveDate::from_ymd_opt(2026, 9, 5).unwrap(),
                reason: "Damaged beyond use".into(),
                possession: ReturnedPossession::Elsewhere,
            },
            &CommandContext::new("dispose-one").unwrap(),
        )
        .unwrap();
    assert!(matches!(
        disposed.specimens[0].ownership,
        Ownership::Disposed { .. }
    ));
    assert_eq!(disposed.specimens[0].loans.len(), 1);
    assert_eq!(disposed.captures.len(), 1);
    assert_eq!(disposed.assets.len(), 1);
    assert_eq!(
        resource_bytes(service.asset(&actor, base.id, asset_id, false).unwrap()),
        *png.get_ref()
    );

    store.reindex().unwrap();
    drop(service);
    drop(store);
    let reopened = Store::open(root.path().join("archive"), root.path().join("state")).unwrap();
    let durable = reopened.item(base.id).unwrap();
    assert_eq!(durable.specimens.len(), 2);
    assert_eq!(durable.specimens[0].loans.len(), 1);
    assert_eq!(
        durable.specimens[0].loans[0].returned_at,
        Some(chrono::NaiveDate::from_ymd_opt(2026, 9, 4).unwrap())
    );
    assert!(matches!(
        durable.specimens[0].ownership,
        Ownership::Disposed { .. }
    ));
    assert_eq!(durable.specimens[1].label, "Listening copy");
    assert_eq!(durable.specimens[1].possession, Possession::Present);
    assert!(matches!(
        durable.specimens[1].ownership,
        Ownership::Borrowed
    ));
    assert!(durable.specimens[1].loans.is_empty());
}

#[test]
fn component_order_and_references_are_protected_by_focused_commands() {
    let (_root, store, actor, service, collection) = setup();
    let base = save_inventory(&service, &actor, item(collection.id), Revision(0)).unwrap();
    let with_specimen = service
        .create_specimen(
            &actor,
            base.id,
            base.revision,
            SpecimenDetails {
                label: "Two-disc set".into(),
                condition: Condition::Excellent,
                ownership: ActiveOwnership::Owned,
                possession: Possession::Present,
                acquisition: None,
                private_notes: None,
            },
            &CommandContext::new("component-specimen").unwrap(),
        )
        .unwrap();
    let specimen_id = with_specimen.specimens[0].id;
    let disc_two = service
        .create_component(
            &actor,
            base.id,
            specimen_id,
            with_specimen.revision,
            ComponentDetails {
                label: "Disc 2".into(),
                kind: ComponentKind::OpticalDisc,
                position: 2,
                carrier: Some(CarrierDetails {
                    label: "Disc 2".into(),
                    kind: CarrierKind::OpticalDisc,
                }),
            },
            &CommandContext::new("disc-two").unwrap(),
        )
        .unwrap();
    let disc_one = service
        .create_component(
            &actor,
            base.id,
            specimen_id,
            disc_two.revision,
            ComponentDetails {
                label: "Disc 1".into(),
                kind: ComponentKind::OpticalDisc,
                position: 1,
                carrier: Some(CarrierDetails {
                    label: "Disc 1".into(),
                    kind: CarrierKind::OpticalDisc,
                }),
            },
            &CommandContext::new("disc-one").unwrap(),
        )
        .unwrap();
    assert_eq!(
        disc_one.specimens[0]
            .components
            .iter()
            .map(|component| component.position)
            .collect::<Vec<_>>(),
        vec![1, 2]
    );
    let second_component_id = disc_one.specimens[0].components[1].id;
    let updated = service
        .update_component(
            &actor,
            (base.id, specimen_id, second_component_id),
            disc_one.revision,
            ComponentDetails {
                label: "Bonus disc".into(),
                kind: ComponentKind::OpticalDisc,
                position: 2,
                carrier: Some(CarrierDetails {
                    label: "Bonus disc".into(),
                    kind: CarrierKind::OpticalDisc,
                }),
            },
            &CommandContext::new("rename-disc-two").unwrap(),
        )
        .unwrap();
    assert_eq!(updated.specimens[0].components[1].label, "Bonus disc");
    let disc_one = service
        .remove_component(
            &actor,
            base.id,
            specimen_id,
            second_component_id,
            updated.revision,
            &CommandContext::new("remove-unreferenced-disc").unwrap(),
        )
        .unwrap();
    assert_eq!(disc_one.specimens[0].components.len(), 1);
    let component_id = disc_one.specimens[0].components[0].id;
    let carrier_id = disc_one.specimens[0].components[0]
        .carrier
        .as_ref()
        .unwrap()
        .id;
    let capture = Capture {
        id: CaptureId::new(),
        collection_id: collection.id,
        revision: Revision(0),
        association: CaptureAssociation {
            item_id: Some(base.id),
            specimen_id: Some(specimen_id),
            component_id: Some(component_id),
            carrier_id: Some(carrier_id),
        },
        created_at: chrono::Utc::now(),
        representations: vec![],
    };
    save_capture_record(&service, &actor, capture, Revision(0)).unwrap();
    let current = store.item(base.id).unwrap();
    assert!(matches!(
        service.remove_component(
            &actor,
            base.id,
            specimen_id,
            component_id,
            current.revision,
            &CommandContext::new("remove-referenced-disc").unwrap(),
        ),
        Err(Error::Conflict)
    ));

    let current = store.item(base.id).unwrap();
    let with_insert = service
        .create_component(
            &actor,
            base.id,
            specimen_id,
            current.revision,
            ComponentDetails {
                label: "Booklet".into(),
                kind: ComponentKind::Booklet,
                position: 2,
                carrier: None,
            },
            &CommandContext::new("booklet-component").unwrap(),
        )
        .unwrap();
    let booklet_id = with_insert.specimens[0].components[1].id;
    let mut png = std::io::Cursor::new(Vec::new());
    image::RgbaImage::new(2, 2)
        .write_to(&mut png, image::ImageFormat::Png)
        .unwrap();
    let mut with_asset = service
        .publish_cover(
            &actor,
            base.id,
            with_insert.revision,
            png.get_ref(),
            "Booklet scan".into(),
            &CommandContext::new("booklet-cover").unwrap(),
        )
        .unwrap();
    with_asset.assets[0].subject = AssetSubject::Component {
        component_id: booklet_id,
    };
    let with_asset = Catalog::save(&*store, with_asset.clone(), with_asset.revision).unwrap();
    assert!(matches!(
        service.remove_component(
            &actor,
            base.id,
            specimen_id,
            booklet_id,
            with_asset.revision,
            &CommandContext::new("remove-asset-subject").unwrap(),
        ),
        Err(Error::Conflict)
    ));
}

#[test]
fn removed_components_are_removed_from_projection() {
    let (root, store, actor, service, collection) = setup();
    let saved = save_inventory(&service, &actor, item(collection.id), Revision(0)).unwrap();
    let saved = service
        .create_specimen(
            &actor,
            saved.id,
            saved.revision,
            SpecimenDetails {
                label: "copy".into(),
                condition: Condition::Unknown,
                ownership: ActiveOwnership::Owned,
                possession: Possession::Present,
                acquisition: None,
                private_notes: None,
            },
            &CommandContext::new("projection-specimen").unwrap(),
        )
        .unwrap();
    let specimen_id = saved.specimens[0].id;
    let saved = service
        .create_component(
            &actor,
            saved.id,
            specimen_id,
            saved.revision,
            ComponentDetails {
                label: "case".into(),
                kind: ComponentKind::Case,
                position: 1,
                carrier: None,
            },
            &CommandContext::new("projection-component").unwrap(),
        )
        .unwrap();
    let component_id = saved.specimens[0].components[0].id;
    service
        .remove_component(
            &actor,
            saved.id,
            specimen_id,
            component_id,
            saved.revision,
            &CommandContext::new("projection-remove").unwrap(),
        )
        .unwrap();
    let db = rusqlite::Connection::open(root.path().join("state/catalog.sqlite")).unwrap();
    let count: u64 = db
        .query_row("SELECT COUNT(*) FROM components", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 0);
    store.reindex().unwrap();
    assert_eq!(
        db.query_row("SELECT COUNT(*) FROM components", [], |row| row
            .get::<_, u64>(0))
            .unwrap(),
        0
    );
}

#[test]
fn subprocess_command_receipt_worker() {
    use collector_app::CommandReceipt;
    let Ok(root) = std::env::var("W2_RECEIPT_ROOT") else {
        return;
    };
    let root = Path::new(&root);
    let operation = std::env::var("W2_RECEIPT_OPERATION").unwrap();
    let fault: Arc<dyn collector_store::PublicationFaultInjector> =
        if std::env::var_os("W2_RECEIPT_EARLY").is_some() {
            Arc::new(BlockAtPublicationPoint(
                collector_store::PublicationFaultPoint::RevisionCommitted,
            ))
        } else {
            Arc::new(collector_store::NoPublicationFault)
        };
    let store = Store::open_with_publication_fault(root.join("archive"), root.join("state"), fault)
        .unwrap();
    let data: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("command.json")).unwrap()).unwrap();
    let user: UserId = serde_json::from_value(data["user"].clone()).unwrap();
    let target: UserId = serde_json::from_value(data["target"].clone()).unwrap();
    let collection: CollectionId = serde_json::from_value(data["collection"].clone()).unwrap();
    let item_id: ItemId = serde_json::from_value(data["item"].clone()).unwrap();
    let upload_id: JobId = serde_json::from_value(data["upload"].clone()).unwrap();
    assert!(matches!(
        store
            .begin_command(user, 1, &operation, "crash-command", "digest")
            .unwrap(),
        ReceiptDecision::Execute
    ));
    let effects = store.command_effects(CommandReceipt {
        user,
        api_major: 1,
        operation: operation.clone(),
        key: "crash-command".into(),
        request_sha256: "digest".into(),
    });
    let result = match operation.as_str() {
        "account.create" => serde_json::to_value(
            effects
                .create_user("created-once", "a sufficiently long password", false)
                .unwrap(),
        )
        .unwrap(),
        "account.update" => serde_json::to_value(
            effects
                .update_account(target, Revision(1), "Changed", true, false)
                .unwrap(),
        )
        .unwrap(),
        "grant.update" => serde_json::to_value(
            effects
                .set_grant(collection, target, Revision(0), Some(Role::Viewer))
                .unwrap(),
        )
        .unwrap(),
        "activity.save" => {
            effects
                .save_activity(user, item_id, serde_json::json!({"favorite":true}))
                .unwrap();
            serde_json::Value::Null
        }
        "session.revoke" => {
            effects
                .revoke_session(
                    user,
                    serde_json::from_value(data["session"].clone()).unwrap(),
                )
                .unwrap();
            serde_json::Value::Null
        }
        "passkey.revoke" => {
            effects
                .revoke_passkey(
                    user,
                    serde_json::from_value(data["passkey"].clone()).unwrap(),
                    Revision(1),
                )
                .unwrap();
            serde_json::Value::Null
        }
        "native_credential.revoke" => {
            effects
                .revoke_native_device(
                    user,
                    serde_json::from_value(data["native"].clone()).unwrap(),
                    Revision(1),
                )
                .unwrap();
            serde_json::Value::Null
        }
        "account.default.reassign" => {
            effects.reassign_default(target).unwrap();
            serde_json::Value::Null
        }
        "projection.reindex" => {
            effects.reindex().unwrap();
            serde_json::Value::Null
        }
        "item.save" => {
            let mut item = store.item(item_id).unwrap();
            item.title = "Updated once".into();
            serde_json::to_value(effects.save(item, Revision(1)).unwrap()).unwrap()
        }
        "collection.create" => serde_json::to_value(
            effects
                .create_collection(
                    user,
                    Collection {
                        id: CollectionId::new(),
                        revision: Revision(1),
                        name: "New collection".into(),
                        created_at: chrono::Utc::now(),
                    },
                )
                .unwrap(),
        )
        .unwrap(),
        "work.save" => serde_json::to_value(
            effects
                .save_work(
                    Work {
                        id: WorkId::new(),
                        collection_id: collection,
                        revision: Revision(0),
                        provisional: true,
                        title: "Work".into(),
                        translations: vec![],
                        identifiers: vec![],
                        migration_provenance: vec![],
                        deleted: false,
                        updated_at: chrono::Utc::now(),
                    },
                    Revision(0),
                )
                .unwrap(),
        )
        .unwrap(),
        "edition.save" => serde_json::to_value(
            effects
                .save_edition(
                    Edition {
                        id: EditionId::new(),
                        collection_id: collection,
                        revision: Revision(0),
                        work_id: None,
                        provisional: true,
                        title: "Edition".into(),
                        expected_components: vec![],
                        identifiers: vec![],
                        migration_provenance: vec![],
                        deleted: false,
                        updated_at: chrono::Utc::now(),
                    },
                    Revision(0),
                )
                .unwrap(),
        )
        .unwrap(),
        "capture.save" => serde_json::to_value(
            effects
                .save_capture(
                    Capture {
                        id: CaptureId::new(),
                        collection_id: collection,
                        revision: Revision(0),
                        association: Default::default(),
                        created_at: chrono::Utc::now(),
                        representations: vec![],
                    },
                    Revision(0),
                )
                .unwrap(),
        )
        .unwrap(),
        "upload.create" => serde_json::to_value(
            effects
                .create_upload(
                    user,
                    item_id,
                    Revision(1),
                    vec![UploadFile {
                        name: "another.bin".into(),
                        bytes: ByteLength(3),
                        sha256: None,
                    }],
                )
                .unwrap(),
        )
        .unwrap(),
        "upload.cancel" => {
            effects.cancel_upload(upload_id).unwrap();
            serde_json::Value::Null
        }
        "upload.commit" => serde_json::to_value(effects.commit_upload(upload_id).unwrap()).unwrap(),
        "integrity.verify" => {
            // Complete the fixture import outside this command scope.
            store.commit_upload(upload_id).unwrap();
            serde_json::to_value(effects.verify_integrity(item_id).unwrap()).unwrap()
        }
        "asset.create.compat-cover" => {
            let mut png = std::io::Cursor::new(Vec::new());
            image::RgbaImage::new(2, 2)
                .write_to(&mut png, image::ImageFormat::Png)
                .unwrap();
            serde_json::to_value(
                effects
                    .create_asset(
                        AssetId::new(),
                        collector_app::CreateAssetCommand {
                            collection_id: collection,
                            subject: AssetSubject::Item { item_id },
                            kind: AssetKind::Cover,
                            visibility: Visibility::Shared,
                            alt_text: "Cover".into(),
                        },
                        png.get_ref(),
                    )
                    .unwrap(),
            )
            .unwrap()
        }
        _ => panic!("unknown operation"),
    };
    // The application has not called complete_command. Terminate at this exact gap.
    println!("W2_EFFECT:{}", serde_json::to_string(&result).unwrap());
    std::io::Write::flush(&mut std::io::stdout()).unwrap();
    loop {
        std::thread::park();
    }
}

#[test]
fn sigkill_receipt_effect_gap_replays_every_mutation_family() {
    use std::io::BufRead;
    let operations = [
        "account.create",
        "account.update",
        "grant.update",
        "activity.save",
        "session.revoke",
        "passkey.revoke",
        "native_credential.revoke",
        "account.default.reassign",
        "projection.reindex",
        "item.save",
        "collection.create",
        "work.save",
        "edition.save",
        "capture.save",
        "upload.create",
        "upload.cancel",
        "upload.commit",
        "integrity.verify",
        "asset.create.compat-cover",
    ];
    for operation in operations {
        for early in [false, true] {
            if early
                && !matches!(
                    operation,
                    "item.save"
                        | "collection.create"
                        | "work.save"
                        | "edition.save"
                        | "capture.save"
                )
            {
                continue;
            }
            let (root, store, actor, service, collection) = setup();
            let saved = store.save(item(collection.id), Revision(0)).unwrap();
            let upload = store
                .create_upload(
                    actor.user_id,
                    saved.id,
                    Revision(1),
                    vec![UploadFile {
                        name: "raw.bin".into(),
                        bytes: ByteLength(3),
                        sha256: None,
                    }],
                )
                .unwrap();
            store.upload_chunk(upload.id, 0, 0, b"raw").unwrap();
            let target = store
                .create_user("member", "a long test password", false)
                .unwrap();
            let (_, session) = store.issue_session(actor.user_id, "session").unwrap();
            store
                .insert_passkey(
                    actor.user_id,
                    "fixture-key",
                    serde_json::json!({"synthetic":true}),
                )
                .unwrap();
            let passkey = store.passkeys(actor.user_id).unwrap().remove(0).view.id;
            let enrollment = store
                .start_enrollment(actor.user_id, "native", "https://example.test")
                .unwrap();
            store
                .approve_enrollment(actor.user_id, &enrollment.user_code)
                .unwrap();
            let native = store
                .complete_enrollment(enrollment.id, &enrollment.completion_secret)
                .unwrap()
                .device_id;
            std::fs::write(root.path().join("command.json"),serde_json::to_vec(&serde_json::json!({"user":actor.user_id,"target":target,"collection":collection.id,"item":saved.id,"upload":upload.id,"session":session.principal.device_id,"passkey":passkey,"native":native})).unwrap()).unwrap();
            drop(service);
            drop(store);
            let mut command = std::process::Command::new(std::env::current_exe().unwrap());
            command
                .args([
                    "--exact",
                    "subprocess_command_receipt_worker",
                    "--nocapture",
                ])
                .env("W2_RECEIPT_ROOT", root.path())
                .env("W2_RECEIPT_OPERATION", operation)
                .stdout(std::process::Stdio::piped());
            if early {
                command.env("W2_RECEIPT_EARLY", "1");
            }
            let mut child = command.spawn().unwrap();
            let output = child.stdout.take().unwrap();
            let (sender, receiver) = std::sync::mpsc::channel();
            let reader = std::thread::spawn(move || {
                for line in std::io::BufReader::new(output).lines() {
                    let line = line.unwrap();
                    if line.starts_with("W2_EFFECT:") || line.starts_with("P01_PHASE:") {
                        sender.send(line).unwrap();
                        return;
                    }
                }
            });
            let marker = receiver.recv_timeout(std::time::Duration::from_secs(30));
            child.kill().unwrap();
            child.wait().unwrap();
            reader.join().unwrap();
            let marker =
                marker.unwrap_or_else(|error| panic!("{operation} early={early}: {error}"));
            let expected: serde_json::Value = if early {
                let private =
                    rusqlite::Connection::open(root.path().join("state/private.sqlite")).unwrap();
                let result: String = private
                    .query_row(
                        "SELECT result FROM command_outcomes WHERE operation=?",
                        [operation],
                        |row| row.get(0),
                    )
                    .unwrap();
                serde_json::from_str(&result).unwrap()
            } else {
                serde_json::from_str(marker.strip_prefix("W2_EFFECT:").unwrap()).unwrap()
            };
            let recovered =
                Store::open(root.path().join("archive"), root.path().join("state")).unwrap();
            let ReceiptDecision::Replay(actual) = recovered
                .begin_command(actor.user_id, 1, operation, "crash-command", "digest")
                .unwrap()
            else {
                panic!("{operation} early={early}: committed effect lost its receipt")
            };
            assert_eq!(actual, expected, "{operation} early={early}");
            assert!(matches!(
                recovered.begin_command(actor.user_id, 1, operation, "crash-command", "changed"),
                Err(Error::IdempotencyConflict)
            ));
            if operation == "upload.commit" {
                assert!(matches!(
                    recovered.upload(upload.id).unwrap().state,
                    JobState::Completed
                ));
            }
        }
    }
}

#[test]
fn successful_command_replay_is_not_revalidated_against_later_revisions() {
    let (_root, store, actor, service, collection) = setup();
    let viewer = store
        .create_user("viewer", "a long viewer password", false)
        .unwrap();
    let grant_key = command();
    for _ in 0..2 {
        service
            .grant(
                &test_authentication(&actor),
                collection.id,
                viewer,
                Role::Viewer,
                &grant_key,
            )
            .unwrap();
    }
    assert_eq!(
        store
            .grant_for_user(collection.id, viewer)
            .unwrap()
            .unwrap()
            .revision,
        Revision(1)
    );
    let saved = store.save(item(collection.id), Revision(0)).unwrap();
    let upload_key = command();
    let files = vec![UploadFile {
        name: "raw.bin".into(),
        bytes: ByteLength(3),
        sha256: None,
    }];
    let original = service
        .create_upload(&actor, saved.id, Revision(1), files.clone(), &upload_key)
        .unwrap();
    let mut changed = saved.clone();
    changed.title = "Later edit".into();
    store.save(changed, Revision(1)).unwrap();
    let replay = service
        .create_upload(&actor, saved.id, Revision(1), files, &upload_key)
        .unwrap();
    assert_eq!(
        serde_json::to_value(original).unwrap(),
        serde_json::to_value(replay).unwrap()
    );
}

fn provisional_import(user_id: UserId, collection_id: CollectionId) -> ImportRecord {
    let timestamp = chrono::DateTime::from_timestamp(1_800_000_000, 0).unwrap();
    ImportRecord {
        user_id,
        view: ImportView {
            id: ImportId::new(),
            revision: 1,
            collection_id,
            source: ImportSourceSummary {
                kind: ImportSourceKind::Browser,
                label: None,
                package_name: "disc-one".into(),
            },
            files: vec![ImportDeclaredFile {
                name: "track01.bin".into(),
                bytes: ByteLength(4),
                received_bytes: ByteLength(0),
                digest: None,
            }],
            capture_id: CaptureId::new(),
            representation_id: RepresentationId::new(),
            association: CaptureAssociation::default(),
            expected_item_revision: None,
            candidates: vec![],
            limitations: vec![],
            review: None,
            analysis_job_id: None,
            phase: ImportPhase::Staging,
            created_at: timestamp,
            updated_at: timestamp,
        },
    }
}

#[test]
fn schema_nine_persists_durable_imports_without_private_path_columns() {
    let root = tempfile::tempdir().unwrap();
    let archive = root.path().join("archive");
    let state = root.path().join("state");
    let store = Store::open(&archive, &state).unwrap();
    let record = provisional_import(UserId::new(), CollectionId::new());
    let expected = serde_json::to_value(store.create_import(record).unwrap()).unwrap();
    drop(store);

    let private = rusqlite::Connection::open(state.join("private.sqlite")).unwrap();
    assert_eq!(
        private
            .query_row("PRAGMA user_version", [], |row| row.get::<_, u32>(0))
            .unwrap(),
        9
    );
    let mut statement = private.prepare("PRAGMA table_info(imports)").unwrap();
    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap();
    assert!(columns.contains(&"capture_id".into()));
    assert!(columns.contains(&"representation_id".into()));
    assert!(!columns.iter().any(|column| {
        matches!(
            column.as_str(),
            "inbox_root" | "source_path" | "scratch_path" | "archive_path"
        )
    }));
    drop(statement);
    drop(private);

    let reopened = Store::open(&archive, &state).unwrap();
    let id: ImportId = expected["view"]["id"].as_str().unwrap().parse().unwrap();
    assert_eq!(
        serde_json::to_value(reopened.import_record(id).unwrap()).unwrap(),
        expected
    );
}

#[test]
fn durable_import_updates_preserve_identity_and_raw_review_observations() {
    let root = tempfile::tempdir().unwrap();
    let clock = Arc::new(TestClock::new(1_800_000_000));
    let store = Store::open_with_clock(
        root.path().join("archive"),
        root.path().join("state"),
        clock.clone(),
    )
    .unwrap();
    let mut record = store
        .create_import(provisional_import(UserId::new(), CollectionId::new()))
        .unwrap();
    let capture_id = record.view.capture_id;
    let representation_id = record.view.representation_id;
    let raw = r#"{"ordered":[3,2,1],"unknown":null,"number":1e+02}"#;
    record.view.files[0].received_bytes = ByteLength(4);
    record.view.files[0].digest = Some(Digest::sha256(b"data"));
    record.view.candidates.push(ImportCandidate {
        key: "candidate-one".into(),
        module: ModuleId("org.junk.music".into()),
        label: "Observed disc".into(),
        observation: RawJson::from_json(raw).unwrap(),
    });
    record.view.limitations = vec!["No built-in music analyzer is available".into()];
    record.view.analysis_job_id = Some(JobId::new());
    record.view.phase = ImportPhase::AwaitingReview;
    record.view.review = Some(ImportReviewDecision {
        identity: ImportIdentityDecision::Candidate {
            key: "candidate-one".into(),
        },
        association: CaptureAssociation::default(),
    });
    clock.advance(5);
    let saved = store.save_import(record, 1).unwrap();
    assert_eq!(saved.view.revision, 2);
    assert_eq!(saved.view.capture_id, capture_id);
    assert_eq!(saved.view.representation_id, representation_id);
    assert_eq!(saved.view.updated_at.timestamp(), 1_800_000_005);
    assert_eq!(saved.view.candidates[0].observation.get(), raw);

    let mut stale = saved.clone();
    stale.view.revision = 1;
    assert!(matches!(store.save_import(stale, 1), Err(Error::Conflict)));

    let mut changed_identity = saved.clone();
    changed_identity.view.source.package_name = "other-package".into();
    assert!(matches!(
        store.save_import(changed_identity, 2),
        Err(Error::Invalid(message)) if message.contains("immutable identity")
    ));
}

#[test]
fn import_validation_rejects_unsafe_or_incoherent_staging_state() {
    let root = tempfile::tempdir().unwrap();
    let store = Store::open(root.path().join("archive"), root.path().join("state")).unwrap();
    let user = UserId::new();
    let collection = CollectionId::new();

    let mut path = provisional_import(user, collection);
    path.view.source.package_name = "/private/inbox/disc".into();
    assert!(matches!(store.create_import(path), Err(Error::Invalid(_))));

    let mut collision = provisional_import(user, collection);
    collision.view.files.push(ImportDeclaredFile {
        name: "TRACK01.BIN".into(),
        bytes: ByteLength(4),
        received_bytes: ByteLength(0),
        digest: None,
    });
    assert!(matches!(
        store.create_import(collision),
        Err(Error::Invalid(message)) if message.contains("case-colliding")
    ));

    let mut bad_offset = provisional_import(user, collection);
    bad_offset.view.files[0].received_bytes = ByteLength(5);
    assert!(matches!(
        store.create_import(bad_offset),
        Err(Error::Invalid(message)) if message.contains("offset")
    ));

    let mut absent_candidate = provisional_import(user, collection);
    absent_candidate.view.phase = ImportPhase::AwaitingReview;
    absent_candidate.view.review = Some(ImportReviewDecision {
        identity: ImportIdentityDecision::Candidate {
            key: "missing".into(),
        },
        association: CaptureAssociation::default(),
    });
    assert!(matches!(
        store.create_import(absent_candidate),
        Err(Error::Invalid(message)) if message.contains("absent candidate")
    ));

    let mut incomplete = provisional_import(user, collection);
    incomplete.view.phase = ImportPhase::Publishing;
    incomplete.view.review = Some(ImportReviewDecision {
        identity: ImportIdentityDecision::Unidentified,
        association: CaptureAssociation::default(),
    });
    assert!(matches!(
        store.create_import(incomplete),
        Err(Error::Invalid(message)) if message.contains("completely staged")
    ));
}

#[test]
fn durable_import_allows_published_unidentified_unattached_capture_identity() {
    let root = tempfile::tempdir().unwrap();
    let store = Store::open(root.path().join("archive"), root.path().join("state")).unwrap();
    let mut record = provisional_import(UserId::new(), CollectionId::new());
    record.view.files[0].received_bytes = ByteLength(4);
    record.view.files[0].digest = Some(Digest::sha256(b"data"));
    record.view.review = Some(ImportReviewDecision {
        identity: ImportIdentityDecision::Unidentified,
        association: CaptureAssociation::default(),
    });
    record.view.phase = ImportPhase::Published;
    let stored = store.create_import(record).unwrap();
    assert_eq!(stored.view.association, CaptureAssociation::default());
    assert!(stored.view.expected_item_revision.is_none());
    assert!(matches!(
        stored.view.review.unwrap().identity,
        ImportIdentityDecision::Unidentified
    ));
}

#[test]
fn browser_and_inbox_sources_converge_on_the_same_durable_staging_state() {
    let (root, store, actor, service, collection) = setup();
    let declared = vec![UploadFile {
        name: "disc/track01.bin".into(),
        bytes: ByteLength(4),
        sha256: None,
    }];
    let browser = service
        .create_provisional_import(
            &actor,
            CreateImportCommand {
                collection_id: collection.id,
                source: ImportSourceSummary {
                    kind: ImportSourceKind::Browser,
                    label: None,
                    package_name: "browser-disc".into(),
                },
                files: declared.clone(),
                association: CaptureAssociation::default(),
                expected_item_revision: None,
            },
            &command(),
        )
        .unwrap();
    let browser = service
        .stage_provisional_import_chunk(&actor, browser.id, 0, 0, b"data")
        .unwrap();

    let inbox_root = root.path().join("configured-inbox");
    let package = inbox_root.join("inbox-disc");
    std::fs::create_dir_all(package.join("disc")).unwrap();
    std::fs::write(package.join("disc/track01.bin"), b"data").unwrap();
    let inbox = service
        .create_provisional_import(
            &actor,
            CreateImportCommand {
                collection_id: collection.id,
                source: ImportSourceSummary {
                    kind: ImportSourceKind::Inbox,
                    label: Some("Optical shelf".into()),
                    package_name: "inbox-disc".into(),
                },
                files: Store::inventory_directory(&package).unwrap(),
                association: CaptureAssociation::default(),
                expected_item_revision: None,
            },
            &command(),
        )
        .unwrap();
    let job = service
        .enqueue_inbox_import_staging(&actor, inbox.id, package.to_str().unwrap(), &command())
        .unwrap();
    assert!(service.run_next_job("test-worker").unwrap());
    assert_eq!(service.job(&actor, job).unwrap().state, JobState::Completed);
    let inbox = service.provisional_import(&actor, inbox.id).unwrap();

    assert_eq!(browser.phase, ImportPhase::Analyzing);
    assert_eq!(inbox.phase, ImportPhase::Analyzing);
    assert_eq!(
        serde_json::to_value(&browser.files).unwrap(),
        serde_json::to_value(&inbox.files).unwrap()
    );
    let job_request: String = rusqlite::Connection::open(root.path().join("state/private.sqlite"))
        .unwrap()
        .query_row(
            "SELECT request FROM jobs WHERE id=?",
            [job.to_string()],
            |row| row.get(0),
        )
        .unwrap();
    assert!(job_request.contains(package.to_str().unwrap()));
    assert!(
        !serde_json::to_string(&inbox)
            .unwrap()
            .contains(package.to_str().unwrap())
    );
    drop(store);
}

#[test]
fn zero_byte_import_files_are_complete_without_a_synthetic_chunk() {
    let (_root, _store, actor, service, collection) = setup();
    let empty = service
        .create_provisional_import(
            &actor,
            CreateImportCommand {
                collection_id: collection.id,
                source: ImportSourceSummary {
                    kind: ImportSourceKind::Browser,
                    label: None,
                    package_name: "empty-file".into(),
                },
                files: vec![UploadFile {
                    name: "empty.bin".into(),
                    bytes: ByteLength(0),
                    sha256: None,
                }],
                association: CaptureAssociation::default(),
                expected_item_revision: None,
            },
            &command(),
        )
        .unwrap();
    assert_eq!(empty.phase, ImportPhase::Analyzing);
    assert_eq!(empty.files[0].received_bytes, ByteLength(0));
    assert_eq!(empty.files[0].digest, Some(Digest::sha256(b"")));

    assert!(matches!(
        service.create_provisional_import(
            &actor,
            CreateImportCommand {
                collection_id: collection.id,
                source: ImportSourceSummary {
                    kind: ImportSourceKind::Browser,
                    label: None,
                    package_name: "bad-empty-file".into(),
                },
                files: vec![UploadFile {
                    name: "empty.bin".into(),
                    bytes: ByteLength(0),
                    sha256: Some(Digest::sha256(b"not empty").hex),
                }],
                association: CaptureAssociation::default(),
                expected_item_revision: None,
            },
            &command(),
        ),
        Err(Error::Invalid(message)) if message.contains("empty file")
    ));
}

#[test]
fn browser_import_resumes_from_durable_offsets_after_restart() {
    let (root, store, actor, service, collection) = setup();
    let create_key = command();
    let source = ImportSourceSummary {
        kind: ImportSourceKind::Browser,
        label: None,
        package_name: "restart-disc".into(),
    };
    let files = vec![UploadFile {
        name: "track.bin".into(),
        bytes: ByteLength(4),
        sha256: Some(Digest::sha256(b"data").hex),
    }];
    let created = service
        .create_provisional_import(
            &actor,
            CreateImportCommand {
                collection_id: collection.id,
                source: source.clone(),
                files: files.clone(),
                association: CaptureAssociation::default(),
                expected_item_revision: None,
            },
            &create_key,
        )
        .unwrap();
    let replay = service
        .create_provisional_import(
            &actor,
            CreateImportCommand {
                collection_id: collection.id,
                source,
                files,
                association: CaptureAssociation::default(),
                expected_item_revision: None,
            },
            &create_key,
        )
        .unwrap();
    assert_eq!(
        serde_json::to_value(&replay).unwrap(),
        serde_json::to_value(&created).unwrap()
    );
    let partial = service
        .stage_provisional_import_chunk(&actor, created.id, 0, 0, b"da")
        .unwrap();
    assert_eq!(partial.files[0].received_bytes, ByteLength(2));
    drop(service);
    drop(store);

    let reopened =
        Arc::new(Store::open(root.path().join("archive"), root.path().join("state")).unwrap());
    let service = CollectionService::new(reopened, Arc::new(Registry::builtins()));
    let restored = service.provisional_import(&actor, created.id).unwrap();
    assert_eq!(restored.files[0].received_bytes, ByteLength(2));
    assert!(matches!(
        service.stage_provisional_import_chunk(&actor, created.id, 0, 3, b"a"),
        Err(Error::Conflict)
    ));
    let completed = service
        .stage_provisional_import_chunk(&actor, created.id, 0, 2, b"ta")
        .unwrap();
    assert_eq!(completed.phase, ImportPhase::Analyzing);
    assert_eq!(completed.files[0].digest, Some(Digest::sha256(b"data")));
}

#[test]
fn changed_inbox_input_fails_the_staging_job_and_retains_resumable_import() {
    let (root, _store, actor, service, collection) = setup();
    let package = root.path().join("configured-inbox/package");
    std::fs::create_dir_all(&package).unwrap();
    std::fs::write(package.join("disc.bin"), b"old").unwrap();
    let created = service
        .create_provisional_import(
            &actor,
            CreateImportCommand {
                collection_id: collection.id,
                source: ImportSourceSummary {
                    kind: ImportSourceKind::Inbox,
                    label: Some("Drop zone".into()),
                    package_name: "package".into(),
                },
                files: Store::inventory_directory(&package).unwrap(),
                association: CaptureAssociation::default(),
                expected_item_revision: None,
            },
            &command(),
        )
        .unwrap();
    std::fs::write(package.join("disc.bin"), b"changed").unwrap();
    let job = service
        .enqueue_inbox_import_staging(&actor, created.id, package.to_str().unwrap(), &command())
        .unwrap();
    assert!(service.run_next_job("test-worker").unwrap());
    assert_eq!(service.job(&actor, job).unwrap().state, JobState::Failed);
    let retained = service.provisional_import(&actor, created.id).unwrap();
    assert_eq!(retained.phase, ImportPhase::Staging);
    assert_eq!(retained.files[0].received_bytes, ByteLength(0));
}

fn staged_import(
    service: &CollectionService,
    actor: &Principal,
    collection: CollectionId,
    package: &str,
) -> ImportView {
    let created = service
        .create_provisional_import(
            actor,
            CreateImportCommand {
                collection_id: collection,
                source: ImportSourceSummary {
                    kind: ImportSourceKind::Browser,
                    label: None,
                    package_name: package.into(),
                },
                files: vec![UploadFile {
                    name: "disc.bin".into(),
                    bytes: ByteLength(4),
                    sha256: None,
                }],
                association: CaptureAssociation::default(),
                expected_item_revision: None,
            },
            &command(),
        )
        .unwrap();
    service
        .stage_provisional_import_chunk(actor, created.id, 0, 0, b"data")
        .unwrap()
}

#[test]
fn optional_import_analysis_persists_candidates_or_honest_unavailability() {
    let (_root, store, actor, base_service, collection) = setup();
    let mut registry = Registry::default();
    registry.register(Box::new(ImportAnalyzingModule)).unwrap();
    let service = CollectionService::new(store.clone(), Arc::new(registry));
    let import = staged_import(&service, &actor, collection.id, "analyzed");
    let job = service
        .enqueue_import_analysis(
            &actor,
            import.id,
            ModuleId("org.example.imports".into()),
            &command(),
        )
        .unwrap();
    assert!(service.run_next_job("analysis-worker").unwrap());
    assert_eq!(service.job(&actor, job).unwrap().state, JobState::Completed);
    let analyzed = service.provisional_import(&actor, import.id).unwrap();
    assert_eq!(analyzed.phase, ImportPhase::AwaitingReview);
    assert_eq!(analyzed.analysis_job_id, Some(job));
    assert_eq!(analyzed.candidates[0].key, "observed-disc");
    assert_eq!(
        analyzed.candidates[0].observation.get(),
        r#"{"unknown":null,"ordered":[2,1],"number":1e+02}"#
    );
    assert!(analyzed.limitations[0].contains("not verification"));

    let unavailable = staged_import(&base_service, &actor, collection.id, "unavailable");
    let job = base_service
        .enqueue_import_analysis(
            &actor,
            unavailable.id,
            ModuleId("org.junk.music".into()),
            &command(),
        )
        .unwrap();
    assert!(base_service.run_next_job("analysis-worker").unwrap());
    assert_eq!(
        base_service.job(&actor, job).unwrap().state,
        JobState::Completed
    );
    let unavailable = base_service
        .provisional_import(&actor, unavailable.id)
        .unwrap();
    assert_eq!(unavailable.phase, ImportPhase::AwaitingReview);
    assert!(unavailable.candidates.is_empty());
    assert!(unavailable.limitations[0].contains("D04/D05"));
}

#[test]
fn import_review_preserves_selection_on_stale_item_and_supports_unattached_review() {
    let (_root, _store, actor, service, collection) = setup();
    let saved = save_inventory(&service, &actor, item(collection.id), Revision(0)).unwrap();
    let import = staged_import(&service, &actor, collection.id, "stale-review");
    let mut changed = saved.clone();
    changed.title = "Concurrent title".into();
    let changed = save_inventory(&service, &actor, changed, saved.revision).unwrap();
    let command_input = ReviewImportCommand {
        expected_revision: import.revision,
        identity: ImportIdentityDecision::Unidentified,
        association: CaptureAssociation {
            item_id: Some(saved.id),
            ..Default::default()
        },
        expected_item_revision: Some(saved.revision),
    };
    assert!(matches!(
        service.review_provisional_import(&actor, import.id, command_input, &command()),
        Err(Error::Conflict)
    ));
    let preserved = service.provisional_import(&actor, import.id).unwrap();
    assert_eq!(preserved.phase, ImportPhase::AwaitingReview);
    assert_eq!(preserved.expected_item_revision, Some(saved.revision));
    assert_eq!(
        preserved.association,
        CaptureAssociation {
            item_id: Some(saved.id),
            ..Default::default()
        }
    );
    assert_eq!(changed.revision, Revision(saved.revision.0 + 1));
    assert!(matches!(
        preserved.review.unwrap().identity,
        ImportIdentityDecision::Unidentified
    ));

    let unattached = staged_import(&service, &actor, collection.id, "unattached-review");
    let reviewed = service
        .review_provisional_import(
            &actor,
            unattached.id,
            ReviewImportCommand {
                expected_revision: unattached.revision,
                identity: ImportIdentityDecision::Unidentified,
                association: CaptureAssociation::default(),
                expected_item_revision: None,
            },
            &command(),
        )
        .unwrap();
    assert_eq!(reviewed.association, CaptureAssociation::default());
    assert!(reviewed.limitations[0].contains("No analyzer was selected"));
}

fn reviewed_unattached_import(
    service: &CollectionService,
    actor: &Principal,
    collection: CollectionId,
    package: &str,
) -> ImportView {
    let staged = staged_import(service, actor, collection, package);
    service
        .review_provisional_import(
            actor,
            staged.id,
            ReviewImportCommand {
                expected_revision: staged.revision,
                identity: ImportIdentityDecision::Unidentified,
                association: CaptureAssociation::default(),
                expected_item_revision: None,
            },
            &command(),
        )
        .unwrap()
}

#[test]
fn unattached_import_publication_commits_preallocated_capture_payload_and_evidence_once() {
    let (root, store, actor, service, collection) = setup();
    let reviewed =
        reviewed_unattached_import(&service, &actor, collection.id, "publish-unattached");
    let job = service
        .enqueue_import_publication(&actor, reviewed.id, &command())
        .unwrap();
    assert!(service.run_next_job("publication-worker").unwrap());
    assert_eq!(service.job(&actor, job).unwrap().state, JobState::Completed);
    let published = service.provisional_import(&actor, reviewed.id).unwrap();
    assert_eq!(published.phase, ImportPhase::Published);
    let capture = Catalog::capture(&*store, reviewed.capture_id).unwrap();
    assert_eq!(capture.association, CaptureAssociation::default());
    assert_eq!(capture.representations[0].id, reviewed.representation_id);
    assert_eq!(
        capture.representations[0].files[0].digest,
        Digest::sha256(b"data")
    );
    let private = rusqlite::Connection::open(root.path().join("state/catalog.sqlite")).unwrap();
    let evidence: String = private
        .query_row(
            "SELECT record FROM evidence WHERE id=?",
            [EvidenceId(reviewed.id.0).to_string()],
            |row| row.get(0),
        )
        .unwrap();
    let evidence: serde_json::Value = serde_json::from_str(&evidence).unwrap();
    assert!(evidence["item_id"].is_null());
    assert_eq!(
        evidence["representation_ids"][0],
        reviewed.representation_id.to_string()
    );
    assert!(
        evidence["details"]["claim"]
            .as_str()
            .unwrap()
            .contains("no catalog verification")
    );
    let payload = find_file_with_bytes(&root.path().join("archive"), b"data").unwrap();
    assert!(
        payload
            .to_string_lossy()
            .contains(&reviewed.capture_id.to_string())
    );
    drop(private);
    drop(service);
    drop(store);
    let reopened = Store::open(root.path().join("archive"), root.path().join("state")).unwrap();
    assert_eq!(
        Catalog::capture(&reopened, reviewed.capture_id)
            .unwrap()
            .revision,
        Revision(1)
    );
    assert_eq!(
        reopened.import_record(reviewed.id).unwrap().view.phase,
        ImportPhase::Published
    );
}

#[test]
fn queued_import_publication_cancellation_retains_scratch_without_publishing() {
    let (root, store, actor, service, collection) = setup();
    let reviewed = reviewed_unattached_import(&service, &actor, collection.id, "cancel-publish");
    let job = service
        .enqueue_import_publication(&actor, reviewed.id, &command())
        .unwrap();
    service.cancel_job(&actor, job, &command()).unwrap();
    assert_eq!(service.job(&actor, job).unwrap().state, JobState::Cancelled);
    assert_eq!(
        service
            .provisional_import(&actor, reviewed.id)
            .unwrap()
            .phase,
        ImportPhase::Cancelled
    );
    assert!(matches!(
        Catalog::capture(&*store, reviewed.capture_id),
        Err(Error::NotFound)
    ));
    assert!(
        root.path()
            .join("state/imports")
            .join(reviewed.id.to_string())
            .join("0.part")
            .exists()
    );
}

#[test]
fn import_cancellation_is_too_late_after_capture_revision_commit_and_finishes_once() {
    let (root, store, actor, service, collection) = setup();
    let reviewed = reviewed_unattached_import(&service, &actor, collection.id, "commit-boundary");
    let job = service
        .enqueue_import_publication(&actor, reviewed.id, &command())
        .unwrap();
    drop(service);
    drop(store);

    let (reached_tx, reached_rx) = std::sync::mpsc::channel();
    let (resume_tx, resume_rx) = std::sync::mpsc::channel();
    let store = Arc::new(
        Store::open_with_publication_fault(
            root.path().join("archive"),
            root.path().join("state"),
            Arc::new(PauseAtRevisionCommit {
                reached: std::sync::Mutex::new(Some(reached_tx)),
                resume: std::sync::Mutex::new(resume_rx),
            }),
        )
        .unwrap(),
    );
    let service = CollectionService::new(store.clone(), Arc::new(Registry::builtins()));
    let worker = {
        let service = service.clone();
        std::thread::spawn(move || service.run_next_job("commit-boundary-worker"))
    };
    reached_rx.recv().unwrap();
    assert!(matches!(
        service.cancel_job(&actor, job, &command()),
        Err(Error::CancellationTooLate)
    ));
    resume_tx.send(()).unwrap();
    assert!(worker.join().unwrap().unwrap());
    assert_eq!(service.job(&actor, job).unwrap().state, JobState::Completed);
    assert_eq!(
        service
            .provisional_import(&actor, reviewed.id)
            .unwrap()
            .phase,
        ImportPhase::Published
    );
    assert_eq!(
        Catalog::capture(&*store, reviewed.capture_id)
            .unwrap()
            .revision,
        Revision(1)
    );
    let db = rusqlite::Connection::open(root.path().join("state/catalog.sqlite")).unwrap();
    assert_eq!(
        db.query_row(
            "SELECT COUNT(*) FROM evidence WHERE id=?",
            [EvidenceId(reviewed.id.0).to_string()],
            |row| row.get::<_, u32>(0),
        )
        .unwrap(),
        1
    );
}

#[test]
fn import_publication_returns_to_review_when_the_associated_item_changes() {
    let (_root, store, actor, service, collection) = setup();
    let saved = save_inventory(&service, &actor, item(collection.id), Revision(0)).unwrap();
    let staged = staged_import(&service, &actor, collection.id, "publication-race");
    let reviewed = service
        .review_provisional_import(
            &actor,
            staged.id,
            ReviewImportCommand {
                expected_revision: staged.revision,
                identity: ImportIdentityDecision::Unidentified,
                association: CaptureAssociation {
                    item_id: Some(saved.id),
                    ..Default::default()
                },
                expected_item_revision: Some(saved.revision),
            },
            &command(),
        )
        .unwrap();
    let mut changed = saved.clone();
    changed.title = "Changed during review".into();
    save_inventory(&service, &actor, changed, saved.revision).unwrap();

    let job = service
        .enqueue_import_publication(&actor, reviewed.id, &command())
        .unwrap();
    assert!(service.run_next_job("publication-race-worker").unwrap());
    assert_eq!(service.job(&actor, job).unwrap().state, JobState::Failed);
    let preserved = service.provisional_import(&actor, reviewed.id).unwrap();
    assert_eq!(preserved.phase, ImportPhase::AwaitingReview);
    assert_eq!(
        serde_json::to_value(&preserved.review).unwrap(),
        serde_json::to_value(&reviewed.review).unwrap()
    );
    assert_eq!(preserved.association, reviewed.association);
    assert!(matches!(
        Catalog::capture(&*store, reviewed.capture_id),
        Err(Error::NotFound)
    ));
}

#[test]
fn published_import_reconciles_a_crash_before_job_completion() {
    let (root, store, actor, service, collection) = setup();
    let reviewed = reviewed_unattached_import(&service, &actor, collection.id, "job-gap");
    let job = service
        .enqueue_import_publication(&actor, reviewed.id, &command())
        .unwrap();
    let claim = store
        .claim_job("crashed-publication-worker", 30)
        .unwrap()
        .unwrap();
    assert_eq!(claim.id, job);
    store
        .publish_import_for_job(reviewed.id, job, &claim.claim_token)
        .unwrap();
    assert_eq!(
        service
            .provisional_import(&actor, reviewed.id)
            .unwrap()
            .phase,
        ImportPhase::Published
    );
    assert_eq!(service.job(&actor, job).unwrap().state, JobState::Running);
    drop(service);
    drop(store);

    let reopened =
        Arc::new(Store::open(root.path().join("archive"), root.path().join("state")).unwrap());
    let service = CollectionService::new(reopened.clone(), Arc::new(Registry::builtins()));
    assert!(
        service
            .run_next_job("replacement-publication-worker")
            .unwrap()
    );
    assert_eq!(service.job(&actor, job).unwrap().state, JobState::Completed);
    assert_eq!(
        service
            .provisional_import(&actor, reviewed.id)
            .unwrap()
            .phase,
        ImportPhase::Published
    );
    assert_eq!(
        Catalog::capture(&*reopened, reviewed.capture_id)
            .unwrap()
            .revision,
        Revision(1)
    );
}
