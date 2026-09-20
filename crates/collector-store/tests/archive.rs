use collector_app::{
    AccessPort, AcquireAccurateRipCommand, ActiveOwnership, AdministrationPort,
    AuthenticatedPrincipal, CarrierDetails, Catalog, CollectionService,
    CollectionTypeAdministrationService, CollectionTypePort, CommandContext, ComponentDetails,
    ContentPort, CreateAssetCommand, CreateCollectionCommand, CreateImportCommand, CredentialKind,
    DisposeSpecimenCommand, FrozenAcquisitionJob, FrozenJobInput, FrozenPreparationMember,
    IdempotencyPort, ImportCandidate, ImportCatalogSnapshotCommand, ImportDeclaredFile,
    ImportIdentityDecision, ImportPhase, ImportPort, ImportRecord, ImportReviewDecision,
    ImportSourceKind, ImportSourceSummary, ImportView, IngestEnrichmentPolicy,
    IngestPackageSelection, JobPort, JobSpec, MediaResource, MetadataAuthorityChange,
    MetadataAuthorityRecord, PlanIngestCommand, PrepareMusicAlbum, PrepareMusicTrack,
    ProcessingExecutionContext, ProcessingPort, ProcessingQuery, ProviderArtworkLink, ProviderPort,
    ProviderRequest, ProviderResponse, ProviderStreamResponse, ProvisionalImportPort,
    PublishProviderArtworkCommand, ReceiptDecision, RefreshMetadataCommand,
    RefreshProviderMetadataCommand, ReturnLoanCommand, ReturnedPossession, ReviewImportCommand,
    SaveItemCommand, SnapshotPort, SnapshotQuery, SnapshotRecord, SpecimenDetails,
    StartIngestCommand, StartLoanCommand, StartProcessingCommand, StreamingProviderPort,
    UpdateAssetCommand, UpdateMetadataAuthorityCommand,
};
use collector_core::*;
use collector_modules::*;
use collector_providers::{
    ACCURATERIP_PROVIDER_ID, AccurateRipProvider, CoverArtArchiveProvider, LOCAL_DAT_PROVIDER_ID,
    LocalDatProvider, LocalDatSource, MusicBrainzProvider,
};
use collector_store::{Clock, Store, UploadFile};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::{io::Write, net::TcpListener, thread, time::Duration};

struct TestClock {
    seconds: std::sync::atomic::AtomicI64,
    millis: std::sync::atomic::AtomicU64,
}

struct ArtworkSelectingModule {
    selected: AssetId,
}

struct ImportAnalyzingModule;
struct ReaderProcessingModule;
struct UnknownStampsModule;
struct LegacyUnavailableModule;

impl CollectionModule for LegacyUnavailableModule {
    fn descriptor(&self) -> ModuleDescriptor {
        ModuleDescriptor {
            id: ModuleId("org.example.unavailable".into()),
            schema_version: 7,
            display_name: "Temporary legacy fixture".into(),
            collection_type_label: "Legacy fixtures".into(),
            fields: vec![],
            capabilities: vec![],
        }
    }
    fn validate_current(&self, _data: &ModuleData) -> ModuleResult<()> {
        Ok(())
    }
    fn apply_edit(&self, current: &ModuleData, _edit: &RawJson) -> ModuleResult<ModuleData> {
        Ok(current.clone())
    }
    fn validate_metadata_overrides(&self, _overrides: &ModuleData) -> ModuleResult<()> {
        Ok(())
    }
    fn validate_operation_result(
        &self,
        _capability: &CapabilityId,
        _result: &RawJson,
    ) -> ModuleResult<()> {
        Ok(())
    }
    fn present(&self, _data: &ModuleData) -> ModuleResult<ModulePresentation> {
        Ok(Default::default())
    }
}

impl CollectionModule for UnknownStampsModule {
    fn descriptor(&self) -> ModuleDescriptor {
        ModuleDescriptor {
            id: ModuleId("org.example.stamps".into()),
            schema_version: 4,
            display_name: "Temporary stamps fixture".into(),
            collection_type_label: "Stamp fixtures".into(),
            fields: vec![],
            capabilities: vec![],
        }
    }
    fn validate_current(&self, _data: &ModuleData) -> ModuleResult<()> {
        Ok(())
    }
    fn apply_edit(&self, current: &ModuleData, _edit: &RawJson) -> ModuleResult<ModuleData> {
        Ok(current.clone())
    }
    fn validate_metadata_overrides(&self, _overrides: &ModuleData) -> ModuleResult<()> {
        Ok(())
    }
    fn validate_operation_result(
        &self,
        _capability: &CapabilityId,
        _result: &RawJson,
    ) -> ModuleResult<()> {
        Ok(())
    }
    fn present(&self, _data: &ModuleData) -> ModuleResult<ModulePresentation> {
        Ok(Default::default())
    }
}

impl CollectionModule for ReaderProcessingModule {
    fn descriptor(&self) -> ModuleDescriptor {
        ModuleDescriptor {
            id: ModuleId("org.example.reader".into()),
            schema_version: 1,
            display_name: "Reader processing fixture".into(),
            collection_type_label: "Reader fixtures".into(),
            fields: vec![],
            capabilities: vec![Capability {
                id: CapabilityId::new("org.example.reader.inspect").unwrap(),
                display_name: "Inspect".into(),
                availability: Availability::Available,
                scope: OperationScope::SharedMedia,
            }],
        }
    }
    fn validate_current(&self, _data: &ModuleData) -> ModuleResult<()> {
        Ok(())
    }
    fn apply_edit(&self, current: &ModuleData, _edit: &RawJson) -> ModuleResult<ModuleData> {
        Ok(current.clone())
    }
    fn validate_metadata_overrides(&self, _overrides: &ModuleData) -> ModuleResult<()> {
        Ok(())
    }
    fn validate_operation_result(
        &self,
        _capability: &CapabilityId,
        result: &RawJson,
    ) -> ModuleResult<()> {
        #[derive(serde::Deserialize)]
        #[serde(deny_unknown_fields)]
        struct ResultBody {
            sha256: String,
        }
        let result: ResultBody = result
            .parse()
            .map_err(|error| ModuleError::Invalid(error.to_string()))?;
        Digest::new(HashAlgorithm::Sha256, result.sha256)
            .map(|_| ())
            .map_err(|error| ModuleError::Invalid(error.to_string()))
    }
    fn present(&self, _data: &ModuleData) -> ModuleResult<ModulePresentation> {
        Ok(ModulePresentation::default())
    }
    fn operate(
        &self,
        request: OperationRequest,
        host: &dyn ModuleHost,
    ) -> ModuleResult<OperationOutput> {
        let representation = match request.entity {
            OperationEntity::Representation(id) => id,
            _ => return Err(ModuleError::Invalid("wrong entity".into())),
        };
        let access = host.representation_access(representation)?;
        let inventory = host.package_inventory(access)?;
        for wrong in [
            PackageAccess {
                collection_id: CollectionId::new(),
                ..access
            },
            PackageAccess {
                scope: PackageScope::Representation(RepresentationId::new()),
                ..access
            },
        ] {
            assert!(matches!(
                host.package_inventory(wrong),
                Err(ModuleError::AccessDenied)
            ));
            assert!(matches!(
                host.package_file(wrong, inventory.files[0].id),
                Err(ModuleError::AccessDenied)
            ));
        }
        let mut reader = host.package_file(access, inventory.files[0].id)?;
        let mut bytes = Vec::new();
        reader
            .read_to_end(&mut bytes)
            .map_err(|error| ModuleError::FailedOperation(error.to_string()))?;
        Ok(OperationOutput {
            value: RawJson::from_value(serde_json::json!({"sha256":Digest::sha256(&bytes).hex})),
        })
    }
}

struct FixtureStreamingProvider {
    calls: Arc<std::sync::atomic::AtomicUsize>,
    bytes: Vec<u8>,
}

struct FixtureAccurateRipProvider {
    bytes: Vec<u8>,
    unavailable: Arc<AtomicBool>,
    no_data: Arc<AtomicBool>,
}

struct MetadataFixtureServer {
    stop: Arc<AtomicBool>,
    origin: String,
    join: Option<thread::JoinHandle<()>>,
}

impl Drop for MetadataFixtureServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(join) = self.join.take() {
            join.join().unwrap();
        }
    }
}

fn metadata_fixture_server(replies: Vec<(&'static str, Vec<u8>)>) -> MetadataFixtureServer {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    listener.set_nonblocking(true).unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let stopped = stop.clone();
    let join = thread::spawn(move || {
        for (status_and_headers, body) in replies {
            let (mut stream, _) = loop {
                if stopped.load(Ordering::SeqCst) {
                    return;
                }
                match listener.accept() {
                    Ok(stream) => break stream,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5))
                    }
                    Err(error) => panic!("fixture accept: {error}"),
                }
            };
            stream.set_nonblocking(false).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut request = [0_u8; 8192];
            let _ = stream.read(&mut request).unwrap();
            let response = format!(
                "HTTP/1.1 {status_and_headers}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            stream.write_all(response.as_bytes()).unwrap();
            stream.write_all(&body).unwrap();
        }
    });
    MetadataFixtureServer {
        stop,
        origin: format!("http://{address}"),
        join: Some(join),
    }
}

impl ProviderPort for FixtureAccurateRipProvider {
    fn acquire(
        &self,
        request: &ProviderRequest,
        execution: &mut dyn ProcessingExecutionContext,
    ) -> Result<ProviderResponse> {
        if execution.cancellation_requested()? {
            return Err(Error::JobCancelled);
        }
        if self.unavailable.load(Ordering::SeqCst) {
            return Err(Error::Unavailable("synthetic provider outage".into()));
        }
        let query: serde_json::Value = request.query.value()?;
        let no_data = self.no_data.load(Ordering::SeqCst);
        let bytes = if no_data {
            Vec::new()
        } else {
            self.bytes.clone()
        };
        let response = ProviderResponse {
            media_type: "application/vnd.accuraterip.dbar".into(),
            digest: Digest::sha256(&bytes),
            bytes,
            parsed_observation: RawJson::from_value(serde_json::json!({
                "schema_version":1,
                "query":query,
                "outcome":if no_data { "no_data" } else { "data" },
                "response_count":if no_data { 0 } else { 1 }
            })),
            limitations: vec![],
        };
        response.validate(request.maximum_response_bytes)?;
        Ok(response)
    }
}

impl StreamingProviderPort for FixtureStreamingProvider {
    fn acquire_stream(
        &self,
        _request: &ProviderRequest,
        _execution: &mut dyn ProcessingExecutionContext,
    ) -> Result<ProviderStreamResponse> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(ProviderStreamResponse {
            media_type: "application/octet-stream".into(),
            bytes: ByteLength(self.bytes.len() as u64),
            digest: Digest::sha256(&self.bytes),
            parsed_observation: RawJson::from_value(serde_json::json!({"fixture":true})),
            reader: Box::new(std::io::Cursor::new(self.bytes.clone())),
        })
    }
}

impl CollectionModule for ImportAnalyzingModule {
    fn descriptor(&self) -> ModuleDescriptor {
        ModuleDescriptor {
            id: ModuleId("org.junk.music".into()),
            schema_version: 1,
            display_name: "Import analysis fixture".into(),
            collection_type_label: "Import fixtures".into(),
            fields: vec![],
            capabilities: vec![Capability {
                id: CapabilityId::new("org.junk.music.import-analysis").unwrap(),
                display_name: "Import analysis".into(),
                availability: Availability::Available,
                scope: OperationScope::CollectionDomain,
            }],
        }
    }

    fn validate_current(&self, _data: &ModuleData) -> ModuleResult<()> {
        Ok(())
    }

    fn apply_edit(&self, current: &ModuleData, _edit: &RawJson) -> ModuleResult<ModuleData> {
        Ok(current.clone())
    }

    fn validate_metadata_overrides(&self, _overrides: &ModuleData) -> ModuleResult<()> {
        Ok(())
    }

    fn validate_operation_result(
        &self,
        _capability: &CapabilityId,
        result: &RawJson,
    ) -> ModuleResult<()> {
        let value = result
            .value()
            .map_err(|error| ModuleError::Invalid(error.to_string()))?;
        if value["candidates"].as_array().is_some() && value["limitations"].as_array().is_some() {
            Ok(())
        } else {
            Err(ModuleError::Invalid(
                "invalid import analysis result".into(),
            ))
        }
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
            collection_type_label: "Music CDs".into(),
            fields: vec![],
            capabilities: vec![],
        }
    }

    fn validate_current(&self, _data: &ModuleData) -> ModuleResult<()> {
        Ok(())
    }

    fn apply_edit(&self, current: &ModuleData, _edit: &RawJson) -> ModuleResult<ModuleData> {
        Ok(current.clone())
    }

    fn validate_metadata_overrides(&self, _overrides: &ModuleData) -> ModuleResult<()> {
        Ok(())
    }

    fn validate_operation_result(
        &self,
        capability: &CapabilityId,
        _result: &RawJson,
    ) -> ModuleResult<()> {
        Err(ModuleError::UnknownCapability(capability.as_str().into()))
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

fn synthetic_redumper_toc(first_lba: i32, leadout_lba: i32) -> Vec<u8> {
    let mut toc = vec![0, 18, 1, 1];
    toc.extend_from_slice(&[0, 0x10, 1, 0]);
    toc.extend_from_slice(&first_lba.to_be_bytes());
    toc.extend_from_slice(&[0, 0x10, 0xaa, 0]);
    toc.extend_from_slice(&leadout_lba.to_be_bytes());
    toc
}

fn typed_collection(name: &str) -> CreateCollectionCommand {
    CreateCollectionCommand {
        name: name.into(),
        module: ModuleId("org.junk.music".into()),
    }
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
                    binding: CollectionBinding::Assigned {
                        module: ModuleId("org.junk.music".into()),
                    },
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
                    typed_collection("Recovered collection"),
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
                typed_collection("Recovered collection"),
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
        typed_collection("Terminated collection"),
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
                typed_collection("Terminated collection"),
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
    setup_with_clock(None)
}

fn setup_with_clock(
    clock: Option<Arc<dyn Clock>>,
) -> (
    tempfile::TempDir,
    Arc<Store>,
    Principal,
    CollectionService,
    Collection,
) {
    let root = tempfile::tempdir().unwrap();
    let store = Arc::new(
        match clock {
            Some(clock) => Store::open_with_clock(
                root.path().join("archive"),
                root.path().join("state"),
                clock,
            ),
            None => Store::open(root.path().join("archive"), root.path().join("state")),
        }
        .unwrap(),
    );
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
        .create_collection(
            &store
                .login("owner", "a long test password", "setup")
                .unwrap()
                .1,
            typed_collection("日本の音楽"),
            &command(),
        )
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

fn legacy_collection_fixture() -> (tempfile::TempDir, UserId, CollectionId, ItemId) {
    let root = tempfile::tempdir().unwrap();
    let archive = root.path().join("archive");
    let state = root.path().join("state");
    let store = Store::open(&archive, &state).unwrap();
    let user = store
        .create_user("owner", "a long test password", true)
        .unwrap();
    let collection = Catalog::create_collection(
        &store,
        user,
        Collection {
            id: CollectionId::new(),
            revision: Revision(1),
            name: "Legacy typed fixture".into(),
            created_at: chrono::Utc::now(),
            binding: CollectionBinding::Assigned {
                module: ModuleId("org.junk.music".into()),
            },
        },
    )
    .unwrap();
    let saved = Catalog::save(&store, item(collection.id), Revision(0)).unwrap();
    drop(store);
    let manifest = files_named(&archive, "collection.toml").remove(0);
    let mut value: toml::Value = std::fs::read_to_string(&manifest).unwrap().parse().unwrap();
    value.as_table_mut().unwrap().remove("binding");
    std::fs::write(&manifest, toml::to_string_pretty(&value).unwrap()).unwrap();
    (root, user, collection.id, saved.id)
}

#[test]
fn ct2_legacy_assignment_is_journaled_idempotent_and_archive_rebuildable() {
    let (root, user, collection_id, item_id) = legacy_collection_fixture();
    let archive = root.path().join("archive");
    let state = root.path().join("state");
    let manifest = files_named(&archive, "collection.toml").remove(0);
    let original_manifest = std::fs::read(&manifest).unwrap();
    let item_record = files_named(&archive, "record.v3.json").remove(0);
    let original_item = std::fs::read(&item_record).unwrap();
    let store = Arc::new(Store::open(&archive, &state).unwrap());
    let snapshot = CollectionTypePort::collection_type_snapshot(&*store, collection_id).unwrap();
    assert_eq!(
        snapshot.collection.binding,
        CollectionBinding::LegacyUnassigned
    );
    assert_eq!(snapshot.item_domains.len(), 1);
    let mut rejected = Catalog::item(&*store, item_id).unwrap();
    rejected.title = "must remain unchanged".into();
    assert!(matches!(
        Catalog::save(&*store, rejected, Revision(1)),
        Err(Error::Invalid(message)) if message.contains("content-read-only")
    ));
    assert_eq!(std::fs::read(&item_record).unwrap(), original_item);

    let actor = Principal {
        user_id: user,
        administrator: true,
        device_id: None,
    };
    let application = CollectionService::new(store.clone(), Arc::new(Registry::builtins()));
    let presented = application
        .collections(&actor)
        .unwrap()
        .into_iter()
        .find(|collection| collection.id == collection_id)
        .unwrap();
    assert!(!presented.content_permissions.can_edit);
    assert_eq!(
        presented.content_permissions.status,
        collector_app::ContentAvailabilityStatus::LegacyUnassigned
    );
    assert!(
        presented
            .content_permissions
            .administrator_commands
            .iter()
            .any(|command| command.contains("inspect-collection-type --collection-id"))
    );
    assert!(
        presented
            .content_permissions
            .administrator_commands
            .iter()
            .any(|command| command.contains("assign-collection-type --collection-id"))
    );

    let service =
        CollectionTypeAdministrationService::new(store.clone(), Arc::new(Registry::builtins()));
    let report = service.inspect(collection_id).unwrap();
    assert_eq!(report.item_domain_counts["org.junk.music"], 1);
    assert_eq!(
        report.eligible_modules,
        vec![ModuleId("org.junk.music".into())]
    );
    assert!(
        service
            .assign(
                collection_id,
                Revision(1),
                ModuleId("org.junk.games".into()),
                chrono::Utc::now(),
            )
            .is_err()
    );
    assert!(
        files_named(&archive, "manifest.v3.toml")
            .into_iter()
            .all(|path| !path.to_string_lossy().contains("collection_metadata"))
    );
    assert_eq!(std::fs::read(&manifest).unwrap(), original_manifest);
    assert_eq!(std::fs::read(&item_record).unwrap(), original_item);
    let assigned = service
        .assign(
            collection_id,
            Revision(1),
            ModuleId("org.junk.music".into()),
            chrono::Utc::now(),
        )
        .unwrap();
    assert_eq!(assigned.revision, Revision(2));
    assert!(
        service
            .assign(
                collection_id,
                Revision(99),
                ModuleId("org.junk.music".into()),
                chrono::Utc::now()
            )
            .is_err()
    );

    assert_eq!(std::fs::read(&manifest).unwrap(), original_manifest);
    assert_eq!(std::fs::read(&item_record).unwrap(), original_item);
    assert_eq!(
        Catalog::role(&*store, user, collection_id).unwrap(),
        Role::Owner
    );
    assert_eq!(
        service
            .assign(
                collection_id,
                Revision(1),
                ModuleId("org.junk.music".into()),
                chrono::Utc::now(),
            )
            .unwrap()
            .revision,
        Revision(2)
    );
    drop(service);
    drop(application);
    drop(store);

    let collection_dir = manifest.parent().unwrap().to_path_buf();
    let renamed = collection_dir
        .parent()
        .unwrap()
        .join(format!("renamed--{collection_id}"));
    std::fs::rename(&collection_dir, &renamed).unwrap();
    let rebuilt_state = root.path().join("rebuilt-state");
    let rebuilt = Store::open(&archive, rebuilt_state).unwrap();
    let rebuilt_snapshot =
        CollectionTypePort::collection_type_snapshot(&rebuilt, collection_id).unwrap();
    assert_eq!(rebuilt_snapshot.collection.revision, Revision(2));
    assert!(matches!(
        rebuilt_snapshot.collection.binding,
        CollectionBinding::Assigned { module } if module == ModuleId("org.junk.music".into())
    ));
    assert!(matches!(
        Catalog::role(&rebuilt, user, collection_id),
        Err(Error::Forbidden)
    ));
}

#[test]
fn ct2_corrupt_binding_sidecar_cannot_replace_the_catalog() {
    let (root, _, collection_id, _) = legacy_collection_fixture();
    let archive = root.path().join("archive");
    let state = root.path().join("state");
    let store = Arc::new(Store::open(&archive, &state).unwrap());
    CollectionTypeAdministrationService::new(store.clone(), Arc::new(Registry::builtins()))
        .assign(
            collection_id,
            Revision(1),
            ModuleId("org.junk.music".into()),
            chrono::Utc::now(),
        )
        .unwrap();
    drop(store);
    let sidecar = files_named(&archive, "record.v3.json")
        .into_iter()
        .find(|path| path.to_string_lossy().contains("collection_metadata"))
        .unwrap();
    let mut bytes = std::fs::read(&sidecar).unwrap();
    bytes[0] ^= 1;
    std::fs::write(sidecar, bytes).unwrap();
    assert!(Store::open(&archive, &state).is_err());
}

#[test]
fn ct2_assignment_crashes_recover_to_pre_or_post_state() {
    use collector_store::PublicationFaultPoint as Point;
    for point in [
        Point::Prepared,
        Point::BytesStaged,
        Point::PayloadPublished,
        Point::RevisionCommitted,
        Point::ProjectionApplied,
        Point::Completed,
    ] {
        let (root, _, collection_id, _) = legacy_collection_fixture();
        let archive = root.path().join("archive");
        let state = root.path().join("state");
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
        let service =
            CollectionTypeAdministrationService::new(store.clone(), Arc::new(Registry::builtins()));
        assert!(
            service
                .assign(
                    collection_id,
                    Revision(1),
                    ModuleId("org.junk.music".into()),
                    chrono::Utc::now(),
                )
                .is_err()
        );
        drop(service);
        drop(store);
        let recovered = Store::open(&archive, &state).unwrap();
        let current = CollectionTypePort::collection_type_snapshot(&recovered, collection_id)
            .unwrap()
            .collection;
        if matches!(
            point,
            Point::Prepared | Point::BytesStaged | Point::PayloadPublished
        ) {
            assert_eq!(current.binding, CollectionBinding::LegacyUnassigned);
        } else {
            assert!(matches!(
                current.binding,
                CollectionBinding::Assigned { .. }
            ));
        }
    }
}

#[test]
fn ct2_creation_persists_builtin_and_third_module_and_binds_idempotency_to_type() {
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
    let mut registry = Registry::builtins();
    registry.register(Box::new(ReaderProcessingModule)).unwrap();
    let service = CollectionService::new(store.clone(), Arc::new(registry));
    for module in ["org.junk.games", "org.junk.music", "org.example.reader"] {
        let created = service
            .create_collection(
                &test_authentication(&actor),
                CreateCollectionCommand {
                    name: format!("{module} collection"),
                    module: ModuleId(module.into()),
                },
                &command(),
            )
            .unwrap();
        assert!(matches!(
            created.binding,
            CollectionBinding::Assigned { module: owner } if owner == ModuleId(module.into())
        ));
    }
    let context = CommandContext::new("ct2-type-bound-retry").unwrap();
    let request = CreateCollectionCommand {
        name: "Stable typed collection".into(),
        module: ModuleId("org.junk.games".into()),
    };
    let first = service
        .create_collection(&test_authentication(&actor), request.clone(), &context)
        .unwrap();
    assert_eq!(
        service
            .create_collection(&test_authentication(&actor), request, &context)
            .unwrap()
            .id,
        first.id
    );
    assert!(matches!(
        service.create_collection(
            &test_authentication(&actor),
            CreateCollectionCommand {
                name: "Stable typed collection".into(),
                module: ModuleId("org.junk.music".into()),
            },
            &context,
        ),
        Err(Error::IdempotencyConflict)
    ));
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
        6
    );

    let private = rusqlite::Connection::open(state.join("private.sqlite")).unwrap();
    assert_eq!(
        private
            .query_row("PRAGMA user_version", [], |row| row.get::<_, u32>(0))
            .unwrap(),
        13
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
                binding: CollectionBinding::Assigned {
                    module: ModuleId("org.junk.music".into()),
                },
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
                binding: CollectionBinding::Assigned {
                    module: ModuleId("org.junk.music".into()),
                },
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
    let mut registry = Registry::builtins();
    registry.register(Box::new(UnknownStampsModule)).unwrap();
    let service = CollectionService::new(store.clone(), Arc::new(registry));
    let collection = service
        .create_collection(
            &test_authentication(&actor),
            CreateCollectionCommand {
                name: "Stamps".into(),
                module: ModuleId("org.example.stamps".into()),
            },
            &command(),
        )
        .unwrap();
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
        .create_collection(
            &test_authentication(&actor),
            typed_collection("Other"),
            &command(),
        )
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
                    binding: CollectionBinding::Assigned {
                        module: ModuleId("org.junk.music".into()),
                    },
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
        frozen_input: None,
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
        frozen_input: None,
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
            frozen_input: None,
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
        frozen_input: None,
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
        store.commit_upload_for_job(claim.id, &claim.claim_token, &mut || Ok(())),
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
    // Recovery must not rehash changed/unavailable payloads or need the owner module.
    let payload = find_file_with_bytes(&root.path().join("archive"), b"data").unwrap();
    std::fs::write(payload, b"changed after the evidence commit").unwrap();
    drop(service);
    drop(store);
    let store =
        Arc::new(Store::open(root.path().join("archive"), root.path().join("state")).unwrap());
    let service = CollectionService::new(store.clone(), Arc::new(Registry::default()));

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
                frozen_input: None,
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
    let completed = service.job(&actor, job).unwrap();
    assert_eq!(
        completed.state,
        JobState::Completed,
        "{:?}",
        completed.error
    );
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
        .create_collection(
            &test_authentication(&attacker),
            typed_collection("Other"),
            &command(),
        )
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
    let mut registry = Registry::builtins();
    registry
        .register(Box::new(LegacyUnavailableModule))
        .unwrap();
    let registry = Arc::new(registry);
    let assignment = CollectionTypeAdministrationService::new(store.clone(), registry.clone());
    let report = assignment.inspect(original.collection_id).unwrap();
    let module = report.eligible_modules.first().unwrap().clone();
    assignment
        .assign(
            original.collection_id,
            Revision(1),
            module.clone(),
            chrono::Utc::now(),
        )
        .unwrap();
    drop(assignment);
    let service = CollectionService::new(store.clone(), registry);
    let mut new_value = item(original.collection_id);
    new_value.domain.module = module;
    new_value.domain.schema_version = 7;
    let new_item = save_inventory(&service, &actor, new_value, Revision(0)).unwrap();
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
        .create_collection(
            &test_authentication(&actor),
            typed_collection("Other"),
            &command(),
        )
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
                        binding: CollectionBinding::Assigned {
                            module: ModuleId("org.junk.music".into()),
                        },
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
fn schema_thirteen_preserves_durable_imports_without_private_path_columns() {
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
        13
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
fn guided_ingest_persists_selected_redumper_anchor_from_signed_plan() {
    let (_root, store, actor, service, collection) = setup();
    let names = [
        "disc.cue",
        "disc.toc",
        "disc.scram",
        "disc.state",
        "disc.subcode",
        "disc.bin",
    ];
    let plan = service
        .plan_ingest(
            &actor,
            PlanIngestCommand {
                collection_id: collection.id,
                packages: vec![IngestPackageSelection {
                    anchor_file_index: Some(1),
                    source: ImportSourceSummary {
                        kind: ImportSourceKind::Browser,
                        label: None,
                        package_name: "Redumper album".into(),
                    },
                    files: names
                        .into_iter()
                        .map(|name| UploadFile {
                            name: name.into(),
                            bytes: ByteLength(1),
                            sha256: None,
                        })
                        .collect(),
                }],
            },
        )
        .unwrap();
    assert!(plan.packages[0].blockers.is_empty());
    assert_eq!(
        plan.packages[0]
            .anchors
            .iter()
            .find(|anchor| anchor.selected)
            .map(|anchor| anchor.file_index),
        Some(1)
    );
    let accepted = service
        .start_ingest(
            &actor,
            StartIngestCommand {
                plan,
                enrichment: IngestEnrichmentPolicy::None,
                prepare_music: false,
            },
            &CommandContext::new("guided-redumper-anchor").unwrap(),
        )
        .unwrap();
    assert_eq!(
        store
            .guided_ingest_intent(accepted.imports[0].import_id)
            .unwrap()
            .unwrap()
            .anchor_file_index,
        Some(1)
    );
}

#[test]
fn guided_game_analysis_does_not_receive_the_music_anchor_extension() {
    let (_root, _store, actor, service, _collection) = setup();
    let collection = service
        .create_collection(
            &test_authentication(&actor),
            CreateCollectionCommand {
                name: "Guided games".into(),
                module: ModuleId("org.junk.games".into()),
            },
            &command(),
        )
        .unwrap();
    let rom = b"owned fixture";
    let plan = service
        .plan_ingest(
            &actor,
            PlanIngestCommand {
                collection_id: collection.id,
                packages: vec![IngestPackageSelection {
                    anchor_file_index: None,
                    source: ImportSourceSummary {
                        kind: ImportSourceKind::Browser,
                        label: None,
                        package_name: "fixture.rom".into(),
                    },
                    files: vec![UploadFile {
                        name: "fixture.rom".into(),
                        bytes: ByteLength(rom.len() as u64),
                        sha256: None,
                    }],
                }],
            },
        )
        .unwrap();
    let accepted = service
        .start_ingest(
            &actor,
            StartIngestCommand {
                plan,
                enrichment: IngestEnrichmentPolicy::None,
                prepare_music: false,
            },
            &command(),
        )
        .unwrap();
    let import_id = accepted.imports[0].import_id;
    service
        .stage_provisional_import_chunk(&actor, import_id, 0, 0, rom)
        .unwrap();

    assert!(service.run_next_job("guided-game-analysis").unwrap());
    let detail = service.ingest_detail(&actor, import_id).unwrap();
    let analysis = service
        .job(&actor, detail.import.analysis_job_id.unwrap())
        .unwrap();
    assert_eq!(analysis.state, JobState::Completed);
    assert!(
        detail
            .import
            .limitations
            .iter()
            .any(|limitation| { limitation.contains("No active DAT snapshot was selected") })
    );
}

#[test]
fn guided_ingest_acceptance_replays_stable_outputs_and_publishes_without_browser_chaining() {
    let (root, store, actor, service, collection) = setup();
    let cue = b"FILE \"disc.bin\" BINARY\n  TRACK 01 AUDIO\n    INDEX 01 00:00:00\n";
    let pcm = vec![0_u8; 2352 * 10];
    let selection = IngestPackageSelection {
        anchor_file_index: None,
        source: ImportSourceSummary {
            kind: ImportSourceKind::Browser,
            label: None,
            package_name: "Guided_album.cue".into(),
        },
        files: vec![
            UploadFile {
                name: "disc.cue".into(),
                bytes: ByteLength(cue.len() as u64),
                sha256: None,
            },
            UploadFile {
                name: "disc.bin".into(),
                bytes: ByteLength(pcm.len() as u64),
                sha256: None,
            },
        ],
    };
    let plan = service
        .plan_ingest(
            &actor,
            PlanIngestCommand {
                collection_id: collection.id,
                packages: vec![selection],
            },
        )
        .unwrap();
    assert_eq!(plan.packages[0].anchors.len(), 1);
    assert!(plan.packages[0].anchors[0].selected);
    let mut stale = plan.clone();
    stale.packages[0].files[1].bytes = ByteLength(pcm.len() as u64 + 1);
    assert!(matches!(
        service.start_ingest(
            &actor,
            StartIngestCommand {
                plan: stale,
                enrichment: IngestEnrichmentPolicy::None,
                prepare_music: false,
            },
            &command(),
        ),
        Err(Error::Conflict)
    ));
    let key = command();
    let accepted = service
        .start_ingest(
            &actor,
            StartIngestCommand {
                plan: plan.clone(),
                enrichment: IngestEnrichmentPolicy::None,
                prepare_music: false,
            },
            &key,
        )
        .unwrap();
    let replay = service
        .start_ingest(
            &actor,
            StartIngestCommand {
                plan: plan.clone(),
                enrichment: IngestEnrichmentPolicy::None,
                prepare_music: false,
            },
            &key,
        )
        .unwrap();
    let replay_again = service
        .start_ingest_with_preflight(
            &actor,
            StartIngestCommand {
                plan: plan.clone(),
                enrichment: IngestEnrichmentPolicy::None,
                prepare_music: false,
            },
            &key,
            |_| Err(Error::Unavailable("inbox or provider changed".into())),
        )
        .unwrap();
    assert_eq!(
        accepted.imports[0].import_id,
        replay_again.imports[0].import_id
    );
    assert_eq!(accepted.imports[0].import_id, replay.imports[0].import_id);
    assert_eq!(accepted.imports[0].capture_id, replay.imports[0].capture_id);
    let id = accepted.imports[0].import_id;
    service
        .stage_provisional_import_chunk(&actor, id, 0, 0, cue)
        .unwrap();
    service
        .stage_provisional_import_chunk(&actor, id, 1, 0, &pcm)
        .unwrap();
    let private = rusqlite::Connection::open(root.path().join("state/private.sqlite")).unwrap();
    assert_eq!(
        private
            .execute(
                "DELETE FROM jobs WHERE kind='import_analysis' AND json_extract(request,'$.import_id')=?",
                [id.to_string()],
            )
            .unwrap(),
        1,
        "simulate a restart in the durable transition-to-job crash window"
    );
    drop(private);
    let recovered = CollectionService::new(store.clone(), Arc::new(Registry::builtins()));
    recovered.resume_guided_ingests().unwrap();
    assert!(recovered.run_next_job("guided-analysis").unwrap());
    assert!(recovered.run_next_job("guided-publication").unwrap());
    let detail = recovered.ingest_detail(&actor, id).unwrap();
    assert_eq!(detail.import.phase, ImportPhase::Published);
    assert_eq!(detail.summary.capture_revision, Some(Revision(1)));
    assert!(matches!(
        recovered.discard_ingest(&actor, id, detail.import.revision, &command()),
        Err(Error::CancellationTooLate)
    ));
    let target = detail.summary.target_item_id.unwrap();
    let current_item = store.item(target).unwrap();
    assert_eq!(current_item.title, "Guided album");
    let authority = store
        .metadata_authority(collection.id, target)
        .unwrap()
        .unwrap();
    assert!(
        authority.authority.field_overrides.is_empty(),
        "filename defaults must remain provisional"
    );
    assert_eq!(authority.inventory.unwrap().title, "Guided album");
    let replacement = store.save(item(collection.id), Revision(0)).unwrap();
    let corrected = recovered
        .correct_ingest_association(
            &actor,
            id,
            collector_app::CorrectIngestAssociationCommand {
                expected_capture_revision: Revision(1),
                association: CaptureAssociation {
                    item_id: Some(replacement.id),
                    ..Default::default()
                },
                expected_item_revision: Some(replacement.revision),
            },
            &command(),
        )
        .unwrap();
    assert_eq!(corrected.summary.capture_revision, Some(Revision(2)));
    assert_eq!(corrected.import.association.item_id, Some(replacement.id));
    assert_eq!(corrected.summary.target_item_id, Some(replacement.id));
    assert_eq!(
        store
            .capture(corrected.summary.capture_id)
            .unwrap()
            .association
            .item_id,
        Some(replacement.id)
    );
    assert_eq!(
        store
            .capture(corrected.summary.capture_id)
            .unwrap()
            .representations[0]
            .files[0]
            .digest,
        Digest::sha256(cue)
    );
}

#[test]
fn guided_album_cancel_recovers_a_missing_intent_pointer_and_fences_retry() {
    let (root, store, actor, service, collection) = setup();
    let service = service.with_audio_converter(Arc::new(DerivativeFixtureEncoder::new()));
    let cue = b"FILE \"disc.bin\" BINARY\n  TRACK 01 AUDIO\n    INDEX 01 00:00:00\n";
    let pcm = vec![0; 2352 * 10];
    let plan = service
        .plan_ingest(
            &actor,
            PlanIngestCommand {
                collection_id: collection.id,
                packages: vec![IngestPackageSelection {
                    anchor_file_index: None,
                    source: ImportSourceSummary {
                        kind: ImportSourceKind::Browser,
                        label: None,
                        package_name: "album.cue".into(),
                    },
                    files: vec![
                        UploadFile {
                            name: "disc.cue".into(),
                            bytes: ByteLength(cue.len() as u64),
                            sha256: None,
                        },
                        UploadFile {
                            name: "disc.bin".into(),
                            bytes: ByteLength(pcm.len() as u64),
                            sha256: None,
                        },
                    ],
                }],
            },
        )
        .unwrap();
    let accepted = service
        .start_ingest(
            &actor,
            StartIngestCommand {
                plan,
                enrichment: IngestEnrichmentPolicy::None,
                prepare_music: true,
            },
            &command(),
        )
        .unwrap();
    let id = accepted.imports[0].import_id;
    service
        .stage_provisional_import_chunk(&actor, id, 0, 0, cue)
        .unwrap();
    service
        .stage_provisional_import_chunk(&actor, id, 1, 0, &pcm)
        .unwrap();
    for phase in ["analysis", "publication", "album-admission"] {
        assert!(service.run_next_job(phase).unwrap());
    }
    let detail = service.ingest_detail(&actor, id).unwrap();
    let batch = detail
        .steps
        .iter()
        .find(|step| step.step == "preparation")
        .unwrap()
        .job_id
        .unwrap();
    let view = service.music_preparation_batch(&actor, batch).unwrap();
    assert_eq!(view.queued, 1);
    let db = rusqlite::Connection::open(root.path().join("state/private.sqlite")).unwrap();
    db.execute(
        "UPDATE guided_ingest_intents SET preparation_batch_id=NULL WHERE import_id=?",
        [id.to_string()],
    )
    .unwrap();
    service
        .cancel_ingest(&actor, id, detail.import.revision, &command())
        .unwrap();
    assert_eq!(
        store
            .job_record(view.members[0].job_id.unwrap(), actor.user_id)
            .unwrap()
            .state,
        JobState::Cancelled
    );
    assert!(matches!(
        service.retry_music_preparation_batch(&actor, batch, &command()),
        Err(Error::Conflict)
    ));
    assert_eq!(
        store
            .capture(detail.summary.capture_id)
            .unwrap()
            .representations
            .len(),
        1
    );
}

#[test]
fn guided_music_preparation_uses_the_album_batch_and_reuses_its_valid_output() {
    let (_root, store, actor, service, collection) = setup();
    let cue = b"FILE \"disc.bin\" BINARY\n  TRACK 01 AUDIO\n    INDEX 01 00:00:00\n";
    let toc = b"TRACK AUDIO\nFILE \"disc.bin\" 00:00:00\n";
    let pcm = vec![0_u8; 2352 * 10];
    let selection = IngestPackageSelection {
        anchor_file_index: Some(0),
        source: ImportSourceSummary {
            kind: ImportSourceKind::Browser,
            label: None,
            package_name: "Prepared album.cue".into(),
        },
        files: vec![
            UploadFile {
                name: "disc.cue".into(),
                bytes: ByteLength(cue.len() as u64),
                sha256: None,
            },
            UploadFile {
                name: "disc.bin".into(),
                bytes: ByteLength(pcm.len() as u64),
                sha256: None,
            },
            UploadFile {
                name: "disc.toc".into(),
                bytes: ByteLength(toc.len() as u64),
                sha256: None,
            },
        ],
    };
    let unavailable = service
        .plan_ingest(
            &actor,
            PlanIngestCommand {
                collection_id: collection.id,
                packages: vec![selection.clone()],
            },
        )
        .unwrap();
    assert!(!unavailable.packages[0].preparation_available);
    assert!(matches!(
        service.start_ingest(
            &actor,
            StartIngestCommand {
                plan: unavailable,
                enrichment: IngestEnrichmentPolicy::None,
                prepare_music: true,
            },
            &command(),
        ),
        Err(Error::Unavailable(_))
    ));
    let encoder = Arc::new(DerivativeFixtureEncoder::new());
    let service = service.with_audio_converter(encoder.clone());
    let plan = service
        .plan_ingest(
            &actor,
            PlanIngestCommand {
                collection_id: collection.id,
                packages: vec![selection],
            },
        )
        .unwrap();
    assert!(plan.packages[0].preparation_available);
    let accepted = service
        .start_ingest(
            &actor,
            StartIngestCommand {
                plan,
                enrichment: IngestEnrichmentPolicy::None,
                prepare_music: true,
            },
            &command(),
        )
        .unwrap();
    let id = accepted.imports[0].import_id;
    service
        .stage_provisional_import_chunk(&actor, id, 0, 0, cue)
        .unwrap();
    service
        .stage_provisional_import_chunk(&actor, id, 1, 0, &pcm)
        .unwrap();
    service
        .stage_provisional_import_chunk(&actor, id, 2, 0, toc)
        .unwrap();
    assert!(service.run_next_job("guided-preparation-analysis").unwrap());
    assert!(
        service
            .run_next_job("guided-preparation-publication")
            .unwrap()
    );
    assert!(
        service.run_next_job("guided-preparation-batch").unwrap(),
        "{:?}",
        service.ingest_detail(&actor, id).unwrap()
    );
    let during = service.ingest_detail(&actor, id).unwrap();
    let preparation = during
        .steps
        .iter()
        .find(|step| step.step == "preparation")
        .unwrap();
    assert_eq!(preparation.state, collector_app::IngestStepState::Active);
    assert!(preparation.job_id.is_some());
    assert!(service.run_next_job("guided-preparation-track").unwrap());
    service.resume_guided_ingests().unwrap();
    let completed = service.ingest_detail(&actor, id).unwrap();
    assert_eq!(
        completed.summary.outcome,
        collector_app::IngestStepState::Succeeded
    );
    assert!(completed.steps.iter().any(|step| {
        step.step == "preparation" && step.state == collector_app::IngestStepState::Succeeded
    }));
    let target = completed.summary.target_item_id.unwrap();
    let capture = store.capture(completed.summary.capture_id).unwrap();
    assert_eq!(capture.representations.len(), 2);
    assert_eq!(
        capture.representations[0].role,
        RepresentationRole::PreservationMaster
    );
    assert_eq!(
        capture.representations[1].role,
        RepresentationRole::LosslessDerivative
    );

    let reused = service
        .prepare_music_album(
            &actor,
            target,
            PrepareMusicAlbum {
                source_representation_ids: vec![capture.representations[0].id],
            },
            &command(),
        )
        .unwrap();
    assert_eq!(reused.succeeded, 1);
    assert_eq!(
        reused.members[0].state,
        collector_app::MusicPreparationMemberState::AlreadyAvailable
    );
    assert_eq!(encoder.calls.load(Ordering::SeqCst), 1);
}

#[test]
fn guided_optional_provider_failure_preserves_master_and_manual_title_authority() {
    let (_root, store, actor, service, collection) = setup();
    let cue = b"FILE \"disc.bin\" BINARY\n  TRACK 01 AUDIO\n    INDEX 01 00:00:00\n";
    let pcm = vec![0_u8; 2352 * 10];
    let plan = service
        .plan_ingest(
            &actor,
            PlanIngestCommand {
                collection_id: collection.id,
                packages: vec![IngestPackageSelection {
                    anchor_file_index: None,
                    source: ImportSourceSummary {
                        kind: ImportSourceKind::Browser,
                        label: None,
                        package_name: "automatic.cue".into(),
                    },
                    files: vec![
                        UploadFile {
                            name: "disc.cue".into(),
                            bytes: ByteLength(cue.len() as u64),
                            sha256: None,
                        },
                        UploadFile {
                            name: "disc.bin".into(),
                            bytes: ByteLength(pcm.len() as u64),
                            sha256: None,
                        },
                    ],
                }],
            },
        )
        .unwrap();
    let accepted = service
        .start_ingest(
            &actor,
            StartIngestCommand {
                plan,
                enrichment: IngestEnrichmentPolicy::Automatic,
                prepare_music: false,
            },
            &command(),
        )
        .unwrap();
    let id = accepted.imports[0].import_id;
    service
        .stage_provisional_import_chunk(&actor, id, 0, 0, cue)
        .unwrap();
    service
        .stage_provisional_import_chunk(&actor, id, 1, 0, &pcm)
        .unwrap();
    let target = service
        .ingest_detail(&actor, id)
        .unwrap()
        .summary
        .target_item_id
        .unwrap();
    let mut edited = store.item(target).unwrap();
    edited.title = "My manual album title".into();
    store.save(edited.clone(), edited.revision).unwrap();
    assert!(service.run_next_job("guided-analysis").unwrap());
    assert!(service.run_next_job("guided-publication").unwrap());
    let detail = service.ingest_detail(&actor, id).unwrap();
    assert_eq!(detail.import.phase, ImportPhase::Published);
    assert!(
        detail
            .steps
            .iter()
            .any(|step| step.step == "artwork"
                && step.state == collector_app::IngestStepState::Failed)
    );
    let presented = service.item_presented(&actor, target).unwrap();
    assert_eq!(presented.title, "My manual album title");
    assert_eq!(
        store
            .capture(detail.summary.capture_id)
            .unwrap()
            .representations[0]
            .role,
        RepresentationRole::PreservationMaster
    );
    let retried = service
        .retry_ingest(
            &actor,
            id,
            collector_app::RetryIngestCommand { job_id: None },
            &command(),
        )
        .unwrap();
    assert_eq!(retried.import.phase, ImportPhase::Published);
    assert!(
        retried
            .steps
            .iter()
            .any(|step| step.step == "artwork"
                && step.state == collector_app::IngestStepState::Failed)
    );
    let capture_before = store.capture(detail.summary.capture_id).unwrap();
    let optional_job = JobId::new();
    store
        .enqueue_job(JobSpec {
            id: optional_job,
            user_id: actor.user_id,
            collection_id: Some(collection.id),
            item_id: Some(target),
            kind: "catalog_acquisition".into(),
            phase: "queued".into(),
            total_units: 0,
            request: serde_json::json!({"import_id": id}),
            frozen_input: None,
        })
        .unwrap();
    service
        .cancel_job(&actor, optional_job, &command())
        .unwrap();
    assert_eq!(
        service.ingest_detail(&actor, id).unwrap().summary.outcome,
        collector_app::IngestStepState::Cancelled,
        "generic cancellation must not leave a guided provider workflow active forever"
    );
    let cancelled = service
        .cancel_ingest(&actor, id, retried.import.revision, &command())
        .unwrap();
    assert_eq!(cancelled.import.phase, ImportPhase::Published);
    assert_eq!(
        cancelled.summary.outcome,
        collector_app::IngestStepState::Cancelled
    );
    assert_eq!(
        serde_json::to_value(store.capture(detail.summary.capture_id).unwrap()).unwrap(),
        serde_json::to_value(capture_before).unwrap()
    );
    assert!(matches!(
        service.retry_ingest(
            &actor,
            id,
            collector_app::RetryIngestCommand { job_id: None },
            &command()
        ),
        Err(Error::Conflict)
    ));
}

#[test]
fn guided_automatic_enrichment_applies_provider_title_and_front_art_without_manual_authority() {
    let (_root, store, actor, service, collection) = setup();
    let cue = b"FILE \"disc.bin\" BINARY\n  TRACK 01 AUDIO\n    INDEX 01 00:00:00\n";
    let pcm = vec![0_u8; 2352 * 10];
    let plan = service
        .plan_ingest(
            &actor,
            PlanIngestCommand {
                collection_id: collection.id,
                packages: vec![IngestPackageSelection {
                    anchor_file_index: None,
                    source: ImportSourceSummary {
                        kind: ImportSourceKind::Browser,
                        label: None,
                        package_name: "provider_default.cue".into(),
                    },
                    files: vec![
                        UploadFile {
                            name: "disc.cue".into(),
                            bytes: ByteLength(cue.len() as u64),
                            sha256: None,
                        },
                        UploadFile {
                            name: "disc.bin".into(),
                            bytes: ByteLength(pcm.len() as u64),
                            sha256: None,
                        },
                    ],
                }],
            },
        )
        .unwrap();
    let accepted = service
        .start_ingest(
            &actor,
            StartIngestCommand {
                plan,
                enrichment: IngestEnrichmentPolicy::Automatic,
                prepare_music: false,
            },
            &command(),
        )
        .unwrap();
    let id = accepted.imports[0].import_id;
    service
        .stage_provisional_import_chunk(&actor, id, 0, 0, cue)
        .unwrap();
    service
        .stage_provisional_import_chunk(&actor, id, 1, 0, &pcm)
        .unwrap();
    assert!(service.run_next_job("guided-provider-analysis").unwrap());
    let analysis = service
        .ingest_detail(&actor, id)
        .unwrap()
        .import
        .analysis_job_id
        .unwrap();
    let observation = store
        .provisional_observations(id)
        .unwrap()
        .into_iter()
        .find(|value| value.id == ObservationId(analysis.0))
        .unwrap();
    let payload: serde_json::Value = observation.typed_result.payload.parse().unwrap();
    let disc_id = payload["observations"][0]["disc_ids"]["musicbrainz"]
        .as_str()
        .unwrap();
    let release_id = "00000000-0000-4000-8000-000000000011";
    let mb_body = format!(
        r#"{{"releases":[{{"id":"{release_id}","title":"Automatic provider title","artist-credit":[{{"name":"Provider Artist","artist":{{"id":"00000000-0000-4000-8000-000000000012","name":"Provider Artist"}}}}],"text-representation":{{"language":"eng","script":"Latn"}},"media":[{{"discs":[{{"id":"{disc_id}"}}]}}],"release-group":{{"id":"00000000-0000-4000-8000-000000000013"}}}}],"count":1}}"#
    )
    .into_bytes();
    let mb_server =
        metadata_fixture_server(vec![("200 OK\r\nContent-Type: application/json", mb_body)]);
    let musicbrainz = Arc::new(
        MusicBrainzProvider::with_test_origin(
            "junk-collector-test/0.1 (test@example.invalid)",
            &mb_server.origin,
            Duration::from_secs(1),
            Duration::ZERO,
            Duration::from_secs(60),
            Duration::from_secs(60),
        )
        .unwrap(),
    );
    let mut encoded = std::io::Cursor::new(Vec::new());
    image::RgbImage::from_pixel(24, 24, image::Rgb([90, 40, 10]))
        .write_to(&mut encoded, image::ImageFormat::Png)
        .unwrap();
    let art = encoded.into_inner();
    let caa_listing = format!(
        r#"{{"release":"https://musicbrainz.org/release/{release_id}","images":[{{"id":"12345","front":true,"back":false,"approved":true,"types":["Front"],"comment":"","edit":1}}]}}"#
    )
    .into_bytes();
    let caa_server = metadata_fixture_server(vec![
        (
            "400 Bad Request\r\nContent-Type: text/plain",
            b"retry me".to_vec(),
        ),
        ("200 OK\r\nContent-Type: application/json", caa_listing),
        ("200 OK\r\nContent-Type: image/png", art.clone()),
    ]);
    let cover_art = Arc::new(
        CoverArtArchiveProvider::with_test_origins(
            "junk-collector-test/0.1 (test@example.invalid)",
            &caa_server.origin,
            &[caa_server.origin.as_str()],
            Duration::from_secs(1),
        )
        .unwrap(),
    );
    let service = service
        .with_streaming_provider(
            ModuleId(collector_modules::music::MUSICBRAINZ_PROVIDER_ID.into()),
            Arc::new(collector_app::BoundedProviderAdapter(musicbrainz.clone())),
        )
        .unwrap()
        .with_catalog_parser(
            ModuleId(collector_modules::music::MUSICBRAINZ_PROVIDER_ID.into()),
            musicbrainz,
        )
        .unwrap()
        .with_streaming_provider(
            ModuleId(collector_modules::music::COVER_ART_ARCHIVE_PROVIDER_ID.into()),
            Arc::new(collector_app::BoundedProviderAdapter(cover_art.clone())),
        )
        .unwrap()
        .with_catalog_parser(
            ModuleId(collector_modules::music::COVER_ART_ARCHIVE_PROVIDER_ID.into()),
            cover_art,
        )
        .unwrap();
    assert!(service.run_next_job("guided-provider-publication").unwrap());
    assert!(service.run_next_job("guided-provider-metadata").unwrap());
    assert!(
        service
            .run_next_job("guided-provider-art-listing-failure")
            .unwrap()
    );
    let artwork_job = service
        .ingest_detail(&actor, id)
        .unwrap()
        .steps
        .into_iter()
        .find(|step| step.step == "artwork")
        .and_then(|step| step.job_id)
        .unwrap();
    service
        .retry_ingest(
            &actor,
            id,
            collector_app::RetryIngestCommand {
                job_id: Some(artwork_job),
            },
            &command(),
        )
        .unwrap();
    assert!(
        service
            .run_next_job("guided-provider-art-listing-retry")
            .unwrap()
    );
    assert!(service.run_next_job("guided-provider-art-image").unwrap());
    assert!(!service.run_next_job("guided-provider-finished").unwrap());
    let target = service
        .ingest_detail(&actor, id)
        .unwrap()
        .summary
        .target_item_id
        .unwrap();
    let revision = store
        .metadata_authority(collection.id, target)
        .unwrap()
        .unwrap()
        .revision;
    let mut intent = store.guided_ingest_intent(id).unwrap().unwrap();
    intent.settled = false;
    store.save_guided_ingest_intent(intent).unwrap();
    service.resume_guided_ingests().unwrap();
    assert_eq!(
        store
            .metadata_authority(collection.id, target)
            .unwrap()
            .unwrap()
            .revision,
        revision,
        "reconciliation must not reapply already-observed metadata after artwork"
    );
    let detail = service.ingest_detail(&actor, id).unwrap();
    assert_eq!(
        detail.summary.outcome,
        collector_app::IngestStepState::Succeeded
    );
    assert!(detail.steps.iter().any(|step| {
        step.step == "metadata" && step.state == collector_app::IngestStepState::Succeeded
    }));
    assert!(detail.steps.iter().any(|step| {
        step.step == "artwork" && step.state == collector_app::IngestStepState::Succeeded
    }));
    let target = detail.summary.target_item_id.unwrap();
    assert_eq!(store.item(target).unwrap().title, "provider default");
    let metadata = service.metadata_view(&actor, target).unwrap();
    assert_eq!(metadata.display.title.value, "Automatic provider title");
    let artwork = metadata.display.artwork.unwrap();
    assert_eq!(
        store.asset_state(artwork).unwrap().asset.file.digest,
        Digest::sha256(&art)
    );
    assert!(metadata.authority.field_overrides.is_empty());
    assert!(metadata.authority.artwork_choice.is_none());
}

#[test]
fn guided_malformed_analysis_does_not_prevent_preserving_originals() {
    let (_root, store, actor, service, collection) = setup();
    let cue = b"FILE \"missing.bin\" BINARY\n TRACK 01 AUDIO\n INDEX 01 00:00:00\n";
    let files = [
        ("disc.cue", cue.as_slice()),
        ("other.bin", b"original bytes".as_slice()),
    ];
    let plan = service
        .plan_ingest(
            &actor,
            PlanIngestCommand {
                collection_id: collection.id,
                packages: vec![IngestPackageSelection {
                    anchor_file_index: None,
                    source: ImportSourceSummary {
                        kind: ImportSourceKind::Browser,
                        label: None,
                        package_name: "Malformed CD".into(),
                    },
                    files: files
                        .iter()
                        .map(|(name, bytes)| UploadFile {
                            name: (*name).into(),
                            bytes: ByteLength(bytes.len() as u64),
                            sha256: None,
                        })
                        .collect(),
                }],
            },
        )
        .unwrap();
    let accepted = service
        .start_ingest(
            &actor,
            StartIngestCommand {
                plan,
                enrichment: IngestEnrichmentPolicy::None,
                prepare_music: false,
            },
            &command(),
        )
        .unwrap();
    let id = accepted.imports[0].import_id;
    for (index, (_, bytes)) in files.iter().enumerate() {
        service
            .stage_provisional_import_chunk(&actor, id, index, 0, bytes)
            .unwrap();
    }
    assert!(service.run_next_job("malformed-analysis").unwrap());
    assert!(
        service
            .run_next_job("preserve-after-analysis-failure")
            .unwrap()
    );
    let detail = service.ingest_detail(&actor, id).unwrap();
    assert_eq!(detail.import.phase, ImportPhase::Published);
    assert!(detail.steps.iter().any(|step| step.step == "identification"
        && step.state == collector_app::IngestStepState::Failed));
    assert_eq!(
        store
            .capture(detail.summary.capture_id)
            .unwrap()
            .representations[0]
            .role,
        RepresentationRole::PreservationMaster
    );
}

#[test]
fn guided_discard_reclaims_only_unpublished_staging_and_denies_other_users() {
    let (root, store, actor, service, collection) = setup();
    let plan = service
        .plan_ingest(
            &actor,
            PlanIngestCommand {
                collection_id: collection.id,
                packages: vec![IngestPackageSelection {
                    anchor_file_index: None,
                    source: ImportSourceSummary {
                        kind: ImportSourceKind::Browser,
                        label: None,
                        package_name: "mistake.bin".into(),
                    },
                    files: vec![UploadFile {
                        name: "mistake.bin".into(),
                        bytes: ByteLength(4),
                        sha256: None,
                    }],
                }],
            },
        )
        .unwrap();
    let accepted = service
        .start_ingest(
            &actor,
            StartIngestCommand {
                plan,
                enrichment: IngestEnrichmentPolicy::Automatic,
                prepare_music: false,
            },
            &command(),
        )
        .unwrap();
    let id = accepted.imports[0].import_id;
    let initial = service.ingest_detail(&actor, id).unwrap();
    assert_eq!(initial.import.phase, ImportPhase::Staging);
    assert_eq!(
        initial.steps[0].state,
        collector_app::IngestStepState::NeedsInput
    );
    let staged = service
        .stage_provisional_import_chunk(&actor, id, 0, 0, b"data")
        .unwrap();
    let other = store
        .create_user("other-ingest", "a long other password", false)
        .unwrap();
    store.grant(collection.id, other, Role::Curator).unwrap();
    let other = Principal {
        user_id: other,
        administrator: false,
        device_id: None,
    };
    assert!(matches!(
        service.ingest_detail(&other, id),
        Err(Error::NotFound)
    ));
    let discarded = service
        .discard_ingest(&actor, id, staged.revision, &command())
        .unwrap();
    assert!(discarded.cleanup_pending);
    assert_eq!(discarded.pending_bytes, ByteLength(4));
    service.resume_guided_ingests().unwrap();
    let cleaned = service.ingest_detail(&actor, id).unwrap().cleanup.unwrap();
    assert_eq!(cleaned.reclaimed_bytes, ByteLength(4));
    assert_eq!(cleaned.pending_bytes, ByteLength(0));
    assert!(
        !root
            .path()
            .join("state/imports")
            .join(id.to_string())
            .exists()
    );
    assert!(matches!(
        service.stage_provisional_import_chunk(&actor, id, 0, 0, b"data"),
        Err(Error::Conflict)
    ));
}

#[test]
fn guided_cancel_stops_unpublished_work_without_discarding_retained_bytes() {
    let (root, store, actor, service, collection) = setup();
    let plan = service
        .plan_ingest(
            &actor,
            PlanIngestCommand {
                collection_id: collection.id,
                packages: vec![IngestPackageSelection {
                    anchor_file_index: None,
                    source: ImportSourceSummary {
                        kind: ImportSourceKind::Browser,
                        label: None,
                        package_name: "paused.bin".into(),
                    },
                    files: vec![UploadFile {
                        name: "paused.bin".into(),
                        bytes: ByteLength(8),
                        sha256: None,
                    }],
                }],
            },
        )
        .unwrap();
    let accepted = service
        .start_ingest(
            &actor,
            StartIngestCommand {
                plan,
                enrichment: IngestEnrichmentPolicy::None,
                prepare_music: false,
            },
            &command(),
        )
        .unwrap();
    let id = accepted.imports[0].import_id;
    let staged = service
        .stage_provisional_import_chunk(&actor, id, 0, 0, b"half")
        .unwrap();
    let cancelled = service
        .cancel_ingest(&actor, id, staged.revision, &command())
        .unwrap();
    assert_eq!(cancelled.import.phase, ImportPhase::Cancelled);
    assert!(matches!(
        service.retry_ingest(
            &actor,
            id,
            collector_app::RetryIngestCommand { job_id: None },
            &command()
        ),
        Err(Error::Conflict)
    ));
    assert!(
        matches!(
            store.enqueue_job(JobSpec {
                id: JobId::new(),
                user_id: actor.user_id,
                collection_id: Some(collection.id),
                item_id: None,
                kind: "import_analysis".into(),
                phase: "queued".into(),
                total_units: 0,
                request: serde_json::json!({"import_id": id}),
                frozen_input: None,
            }),
            Err(Error::Conflict)
        ),
        "cancelled imports cannot admit new jobs"
    );
    assert_eq!(
        std::fs::read(
            root.path()
                .join("state/imports")
                .join(id.to_string())
                .join("0.part")
        )
        .unwrap(),
        b"half",
        "cancellation retains staged bytes until explicit safe discard"
    );
    let discarded = service
        .discard_ingest(&actor, id, cancelled.import.revision, &command())
        .unwrap();
    assert!(discarded.cleanup_pending);
    service.resume_guided_ingests().unwrap();
    assert_eq!(
        service
            .ingest_detail(&actor, id)
            .unwrap()
            .cleanup
            .unwrap()
            .reclaimed_bytes,
        ByteLength(4)
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
fn stopped_server_copy_rehearsal_preserves_archive_accounts_and_pending_scratch() {
    let (root, store, actor, service, collection) = setup();
    let owner_session = store
        .login("owner", "a long test password", "restore rehearsal")
        .unwrap()
        .1;
    let viewer = store
        .create_user("restore-viewer", "another long password", false)
        .unwrap();
    service
        .grant(
            &owner_session,
            collection.id,
            viewer,
            Role::Viewer,
            &command(),
        )
        .unwrap();

    let saved = save_inventory(&service, &actor, item(collection.id), Revision(0)).unwrap();
    let original = b"synthetic stopped-server rehearsal original";
    let upload = service
        .create_upload(
            &actor,
            saved.id,
            saved.revision,
            vec![UploadFile {
                name: "master.bin".into(),
                bytes: ByteLength(original.len() as u64),
                sha256: Some(Digest::sha256(original).hex),
            }],
            &command(),
        )
        .unwrap();
    service
        .upload_chunk(&actor, upload.id, 0, 0, original)
        .unwrap();
    service
        .commit_upload(&actor, upload.id, &command())
        .unwrap();

    let pending = service
        .create_provisional_import(
            &actor,
            CreateImportCommand {
                collection_id: collection.id,
                source: ImportSourceSummary {
                    kind: ImportSourceKind::Browser,
                    label: None,
                    package_name: "restore-pending".into(),
                },
                files: vec![UploadFile {
                    name: "pending.bin".into(),
                    bytes: ByteLength(4),
                    sha256: Some(Digest::sha256(b"data").hex),
                }],
                association: CaptureAssociation::default(),
                expected_item_revision: None,
            },
            &command(),
        )
        .unwrap();
    let pending = service
        .stage_provisional_import_chunk(&actor, pending.id, 0, 0, b"da")
        .unwrap();
    assert_eq!(pending.files[0].received_bytes, ByteLength(2));

    let server_id = store.server_id().unwrap();
    let account_names = store
        .accounts()
        .unwrap()
        .into_iter()
        .map(|account| account.username)
        .collect::<Vec<_>>();
    let evidence_records = serde_json::to_value(store.evidence(saved.id).unwrap()).unwrap();
    drop(service);
    drop(store);

    let archive = root.path().join("archive");
    let state = root.path().join("state");
    let archive_hashes = tree_hashes(&archive);
    let state_hashes = tree_hashes(&state);
    let restore = root.path().join("isolated-restore");
    let restored_archive = restore.join("archive");
    let restored_state = restore.join("state");
    assert!(!restore.exists());
    copy_tree(&archive, &restored_archive);
    copy_tree(&state, &restored_state);
    assert_eq!(tree_hashes(&restored_archive), archive_hashes);
    assert_eq!(tree_hashes(&restored_state), state_hashes);

    let restored = Arc::new(Store::open(&restored_archive, &restored_state).unwrap());
    assert_eq!(restored.server_id().unwrap(), server_id);
    assert_eq!(
        restored
            .accounts()
            .unwrap()
            .into_iter()
            .map(|account| account.username)
            .collect::<Vec<_>>(),
        account_names
    );
    restored
        .login("owner", "a long test password", "restored owner")
        .unwrap();
    restored
        .login("restore-viewer", "another long password", "restored viewer")
        .unwrap();
    assert_eq!(
        Catalog::role(&*restored, viewer, collection.id).unwrap(),
        Role::Viewer
    );

    let restored_service = CollectionService::new(restored.clone(), Arc::new(Registry::builtins()));
    assert_eq!(
        resource_bytes(
            restored_service
                .representation(&actor, saved.id, upload.representation_id, 0)
                .unwrap()
        ),
        original
    );
    assert_eq!(
        serde_json::to_value(restored.evidence(saved.id).unwrap()).unwrap(),
        evidence_records
    );
    let resumed = restored_service
        .provisional_import(&actor, pending.id)
        .unwrap();
    assert_eq!(resumed.files[0].received_bytes, ByteLength(2));
    let completed = restored_service
        .stage_provisional_import_chunk(&actor, pending.id, 0, 2, b"ta")
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
            ModuleId("org.junk.music".into()),
            &command(),
        )
        .unwrap();
    assert!(service.run_next_job("analysis-worker").unwrap());
    let completed = service.job(&actor, job).unwrap();
    assert_eq!(
        completed.state,
        JobState::Completed,
        "{:?}",
        completed.error
    );
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
    assert!(unavailable.limitations[0].contains("exactly one CUE or redumper TOC"));
}

#[test]
fn import_observation_rebinds_to_representation_and_rebuilds_without_private_imports() {
    let (root, store, actor, base_service, collection) = setup();
    let mut registry = Registry::default();
    registry.register(Box::new(ImportAnalyzingModule)).unwrap();
    let service = CollectionService::new(store.clone(), Arc::new(registry));
    let import = staged_import(&service, &actor, collection.id, "portable-analysis");
    let analysis_job = service
        .enqueue_import_analysis(
            &actor,
            import.id,
            ModuleId("org.junk.music".into()),
            &command(),
        )
        .unwrap();
    service.run_next_job("analysis-worker").unwrap();
    let analyzed = service.provisional_import(&actor, import.id).unwrap();
    let reviewed = service
        .review_provisional_import(
            &actor,
            import.id,
            ReviewImportCommand {
                expected_revision: analyzed.revision,
                identity: ImportIdentityDecision::Unidentified,
                association: CaptureAssociation::default(),
                expected_item_revision: None,
            },
            &command(),
        )
        .unwrap();
    let publication_job = service
        .enqueue_import_publication(&actor, reviewed.id, &command())
        .unwrap();
    service.run_next_job("publication-worker").unwrap();

    let portable = store
        .processing_record(ObservationId(analysis_job.0))
        .unwrap();
    let rebinding = portable.import_rebinding.unwrap();
    assert_eq!(
        rebinding.provisional_fingerprint.source,
        PackageScope::Import(import.id)
    );
    assert_eq!(
        rebinding.committed_fingerprint.source,
        PackageScope::Representation(import.representation_id)
    );
    assert_ne!(
        rebinding.provisional_fingerprint.digest,
        rebinding.committed_fingerprint.digest
    );
    assert_eq!(rebinding.files[0].sha256, Digest::sha256(b"data"));
    let completed_publication = service.job(&actor, publication_job).unwrap();
    assert_eq!(
        completed_publication.state,
        JobState::Completed,
        "{:?}",
        completed_publication.error
    );

    drop(service);
    drop(base_service);
    drop(store);
    let collection_root = std::fs::read_dir(root.path().join("archive/collections"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let unknown =
        collection_root.join("future_records/opaque/revisions/00000000000000000001/record.v9.json");
    std::fs::create_dir_all(unknown.parent().unwrap()).unwrap();
    let unknown_bytes = br#"{"future":1e+02,"ordered":[3,1],"unknown":null}"#;
    std::fs::write(&unknown, unknown_bytes).unwrap();
    let archive_only = Store::open(
        root.path().join("archive"),
        root.path().join("archive-only-state"),
    )
    .unwrap();
    let rebuilt = archive_only
        .processing_record(ObservationId(analysis_job.0))
        .unwrap();
    assert_eq!(
        rebuilt.observation.source_fingerprint.source,
        PackageScope::Representation(import.representation_id)
    );
    assert_eq!(
        rebuilt.observation.typed_result.payload.get(),
        r#"{"candidates":[{"key":"observed-disc","label":"Observed disc","observation":{"unknown":null,"ordered":[2,1],"number":1e+02}}],"limitations":["Fixture candidate is an observation, not verification"]}"#
    );
    assert_eq!(std::fs::read(unknown).unwrap(), unknown_bytes);
}

#[test]
fn streamed_snapshot_is_hashed_and_survives_archive_only_rebuild() {
    let (root, store, _actor, service, collection) = setup();
    let bytes = b"streamed catalog bytes".to_vec();
    let snapshot = SnapshotRecord {
        id: SnapshotId::new(),
        collection_id: collection.id,
        provider: ModuleId("org.example.catalog".into()),
        media_type: "application/octet-stream".into(),
        bytes: ByteLength(bytes.len() as u64),
        digest: Digest::sha256(&bytes),
        parsed_observation: RawJson::from_value(serde_json::json!({"records":1})),
        created_at: chrono::Utc::now(),
    };
    let mut reader = std::io::Cursor::new(bytes.clone());
    store
        .publish_snapshot(
            JobId::new(),
            "fixture-claim",
            snapshot.clone(),
            &mut reader,
            &mut || Ok(false),
        )
        .unwrap();
    drop(service);
    drop(store);
    let archive_only = Store::open(
        root.path().join("archive"),
        root.path().join("snapshot-rebuild-state"),
    )
    .unwrap();
    let query = SnapshotQuery {
        collection_id: collection.id,
        snapshot_id: snapshot.id,
    };
    assert_eq!(
        archive_only.snapshot(query).unwrap().digest,
        snapshot.digest
    );
    let mut restored = Vec::new();
    archive_only
        .snapshot_bytes(query)
        .unwrap()
        .read_to_end(&mut restored)
        .unwrap();
    assert_eq!(restored, bytes);

    let mut corrupt = std::io::Cursor::new(b"wrong".to_vec());
    let mut changed = snapshot;
    changed.id = SnapshotId::new();
    assert!(matches!(
        archive_only.publish_snapshot(
            JobId::new(),
            "fixture-claim",
            changed,
            &mut corrupt,
            &mut || Ok(false)
        ),
        Err(Error::Invalid(_))
    ));
}

#[test]
fn streamed_snapshot_cancellation_aborts_before_commit_and_recovers_cleanly() {
    let (root, store, _actor, service, collection) = setup();
    let bytes = vec![7_u8; 2 * 1024 * 1024];
    let snapshot = SnapshotRecord {
        id: SnapshotId::new(),
        collection_id: collection.id,
        provider: ModuleId("org.example.catalog".into()),
        media_type: "application/octet-stream".into(),
        bytes: ByteLength(bytes.len() as u64),
        digest: Digest::sha256(&bytes),
        parsed_observation: RawJson::from_value(serde_json::json!({})),
        created_at: chrono::Utc::now(),
    };
    let mut checks = 0;
    assert!(matches!(
        store.publish_snapshot(
            JobId::new(),
            "claim",
            snapshot.clone(),
            &mut std::io::Cursor::new(bytes),
            &mut || {
                checks += 1;
                Ok(checks > 1)
            },
        ),
        Err(Error::JobCancelled)
    ));
    assert!(matches!(
        store.snapshot(SnapshotQuery {
            collection_id: collection.id,
            snapshot_id: snapshot.id
        }),
        Err(Error::NotFound)
    ));
    drop(service);
    drop(store);
    let reopened = Store::open(root.path().join("archive"), root.path().join("state")).unwrap();
    assert!(matches!(
        reopened.snapshot(SnapshotQuery {
            collection_id: collection.id,
            snapshot_id: snapshot.id
        }),
        Err(Error::NotFound)
    ));
}

#[test]
fn game_dat_workflow_preserves_old_snapshot_evidence_after_refresh_and_reindex() {
    let root = tempfile::tempdir().unwrap();
    let archive = root.path().join("archive");
    let state = root.path().join("state");
    let dat_path = root.path().join("games.dat");
    let dat = |version: &str, sha1: &str| {
        format!(
            r#"<datafile><header><name>Fixture Games</name><version>{version}</version></header><game name="Known Game"><rom name="game.bin" size="4" sha1="{sha1}"/></game></datafile>"#
        )
    };
    std::fs::write(
        &dat_path,
        dat("1", "a17c9aaa61e80a1bf71d0d850af4e5baa9800bbd"),
    )
    .unwrap();
    let store = Arc::new(Store::open(&archive, &state).unwrap());
    let user = store
        .create_user("game-owner", "a long test password", true)
        .unwrap();
    let actor = Principal {
        user_id: user,
        administrator: true,
        device_id: None,
    };
    let provider = Arc::new(
        LocalDatProvider::new(std::collections::BTreeMap::from([(
            "nes".into(),
            LocalDatSource {
                path: dat_path.clone(),
                system: "nes".into(),
                catalog: "fixture-games".into(),
                byte_domain: "org.junk.game.raw-bytes".into(),
                byte_domain_version: 1,
            },
        )]))
        .unwrap(),
    );
    let provider_id = ModuleId(LOCAL_DAT_PROVIDER_ID.into());
    let service = CollectionService::new(store.clone(), Arc::new(Registry::builtins()))
        .with_streaming_provider(provider_id.clone(), provider.clone())
        .unwrap()
        .with_catalog_parser(provider_id.clone(), provider)
        .unwrap();
    let collection = service
        .create_collection(
            &test_authentication(&actor),
            CreateCollectionCommand {
                name: "Qualified games".into(),
                module: ModuleId("org.junk.games".into()),
            },
            &command(),
        )
        .unwrap();
    let mut game = item(collection.id);
    game.title = "Unidentified game".into();
    game.subtitle = "NES".into();
    game.domain = ModuleData {
        module: ModuleId("org.junk.games".into()),
        schema_version: 1,
        payload: RawJson::from_value(serde_json::json!({
            "platform":"nes",
            "region":null,
            "publisher":null,
            "release_year":null
        })),
    };
    let game = Catalog::save(&*store, game, Revision(0)).unwrap();

    let import_snapshot = |service: &CollectionService| {
        let job = service
            .import_catalog_snapshot(
                &actor,
                ImportCatalogSnapshotCommand {
                    collection_id: collection.id,
                    provider: provider_id.clone(),
                    source: "nes".into(),
                },
                &command(),
            )
            .unwrap();
        assert!(service.run_next_job("dat-acquisition").unwrap());
        assert_eq!(service.job(&actor, job).unwrap().state, JobState::Completed);
        SnapshotId(job.0)
    };
    let first_snapshot = import_snapshot(&service);
    service
        .activate_catalog_snapshot(&actor, collection.id, first_snapshot, &command())
        .unwrap();
    let curator_id = store
        .create_user("game-curator", "another long test password", false)
        .unwrap();
    let viewer_id = store
        .create_user("game-viewer", "a third long test password", false)
        .unwrap();
    Catalog::grant(&*store, collection.id, curator_id, Role::Curator).unwrap();
    Catalog::grant(&*store, collection.id, viewer_id, Role::Viewer).unwrap();
    let curator = Principal {
        user_id: curator_id,
        administrator: false,
        device_id: None,
    };
    let viewer = Principal {
        user_id: viewer_id,
        administrator: false,
        device_id: None,
    };
    assert_eq!(
        service
            .curation_catalog_snapshots(&curator, collection.id, None, 10)
            .unwrap()
            .items
            .len(),
        1
    );
    assert!(matches!(
        service.catalog_snapshots(&curator, collection.id, None, 10),
        Err(Error::Forbidden)
    ));
    assert!(matches!(
        service.curation_catalog_snapshots(&viewer, collection.id, None, 10),
        Err(Error::Forbidden)
    ));

    let import = service
        .create_provisional_import(
            &actor,
            CreateImportCommand {
                collection_id: collection.id,
                source: ImportSourceSummary {
                    kind: ImportSourceKind::Browser,
                    label: None,
                    package_name: "synthetic-game".into(),
                },
                files: vec![UploadFile {
                    name: "game.bin".into(),
                    bytes: ByteLength(4),
                    sha256: None,
                }],
                association: CaptureAssociation {
                    item_id: Some(game.id),
                    ..Default::default()
                },
                expected_item_revision: Some(game.revision),
            },
            &command(),
        )
        .unwrap();
    service
        .stage_provisional_import_chunk(&actor, import.id, 0, 0, b"data")
        .unwrap();
    let analysis_job = service
        .enqueue_import_analysis(
            &actor,
            import.id,
            ModuleId("org.junk.games".into()),
            &command(),
        )
        .unwrap();
    assert!(service.run_next_job("game-analysis").unwrap());
    let analyzed = service.provisional_import(&actor, import.id).unwrap();
    assert_eq!(analyzed.candidates.len(), 1);
    assert!(analyzed.candidates[0].label.contains("exact catalog match"));
    let reviewed = service
        .review_provisional_import(
            &actor,
            import.id,
            ReviewImportCommand {
                expected_revision: analyzed.revision,
                identity: ImportIdentityDecision::Candidate {
                    key: analyzed.candidates[0].key.clone(),
                },
                association: analyzed.association,
                expected_item_revision: Some(game.revision),
            },
            &command(),
        )
        .unwrap();
    let publication_job = service
        .enqueue_import_publication(&actor, reviewed.id, &command())
        .unwrap();
    assert!(service.run_next_job("game-publication").unwrap());
    let completed_publication = service.job(&actor, publication_job).unwrap();
    assert_eq!(
        completed_publication.state,
        JobState::Completed,
        "{:?}",
        completed_publication.error
    );
    let published = service.provisional_import(&actor, import.id).unwrap();
    let capture = Catalog::capture(&*store, published.capture_id).unwrap();
    let representation = published.representation_id;

    let mismatched_snapshot_job = service
        .enqueue_processing_operation(
            &actor,
            capture.id,
            StartProcessingCommand {
                operation: OperationId::new("org.junk.games.catalog-verification").unwrap(),
                operation_version: 1,
                representation_id: representation,
                snapshot_id: Some(first_snapshot),
                selection: RawJson::from_value(serde_json::json!({
                    "snapshot_id":SnapshotId::new(),
                    "system":"nes",
                    "input":"raw_files"
                })),
            },
            &command(),
        )
        .unwrap();
    assert!(service.run_next_job("mismatched-snapshot").unwrap());
    assert_eq!(
        service.job(&actor, mismatched_snapshot_job).unwrap().state,
        JobState::Failed
    );

    let operation = service
        .enqueue_processing_operation(
            &actor,
            capture.id,
            StartProcessingCommand {
                operation: OperationId::new("org.junk.games.catalog-verification").unwrap(),
                operation_version: 1,
                representation_id: representation,
                snapshot_id: Some(first_snapshot),
                selection: RawJson::from_value(serde_json::json!({
                    "snapshot_id":first_snapshot,
                    "system":"nes",
                    "input":"raw_files"
                })),
            },
            &command(),
        )
        .unwrap();
    assert!(service.run_next_job("game-verification").unwrap());
    let completed_operation = service.job(&actor, operation).unwrap();
    assert_eq!(
        completed_operation.state,
        JobState::Completed,
        "{:?}",
        completed_operation.error
    );

    let dat_metadata = service
        .refresh_provider_metadata(
            &actor,
            game.id,
            RefreshProviderMetadataCommand {
                expected_revision: Revision(0),
                snapshot_id: first_snapshot,
                source_observation_id: Some(ObservationId(operation.0)),
                artwork: vec![],
            },
            &command(),
        )
        .unwrap();
    assert_eq!(dat_metadata.observations.len(), 1);
    assert_ne!(dat_metadata.observations[0].id.0, operation.0);
    assert_ne!(dat_metadata.observations[0].id.0, first_snapshot.0);
    assert_eq!(
        dat_metadata.observations[0].candidates[0].title.value,
        "Known Game"
    );
    assert!(dat_metadata.observations[0].candidates[0].complete_identity);
    assert_eq!(
        dat_metadata.observations[0].candidates[0].provider_priority,
        10
    );
    assert_eq!(
        dat_metadata.display.as_ref().unwrap().title.value,
        "Unidentified game",
        "DAT enrichment does not overwrite inventory-derived manual authority"
    );

    std::fs::write(
        &dat_path,
        dat("2", "580393f5a94fb469585f5dd2a6859a4aab899f37"),
    )
    .unwrap();
    let second_snapshot = import_snapshot(&service);
    service
        .activate_catalog_snapshot(&actor, collection.id, second_snapshot, &command())
        .unwrap();
    let statuses = service
        .catalog_snapshots(&actor, collection.id, None, 10)
        .unwrap();
    assert_eq!(statuses.items.len(), 2);
    assert!(
        statuses
            .items
            .iter()
            .any(|value| value.snapshot.id == first_snapshot && !value.active)
    );
    assert!(
        statuses
            .items
            .iter()
            .any(|value| value.snapshot.id == second_snapshot && value.active)
    );
    assert!(matches!(
        service.catalog_snapshots(&actor, collection.id, Some(SnapshotId::new()), 10),
        Err(Error::Invalid(_))
    ));

    let summaries = service
        .capture_processing_summaries(&actor, capture.id, representation, None, 20)
        .unwrap();
    assert!(
        summaries
            .items
            .iter()
            .any(|summary| summary.snapshot_id == Some(first_snapshot)
                && summary.result.get().contains("unique_complete_match"))
    );
    assert!(
        summaries
            .items
            .iter()
            .any(|summary| summary.observation_id == ObservationId(analysis_job.0))
    );

    store.reindex().unwrap();
    assert_eq!(
        store
            .snapshot(SnapshotQuery {
                collection_id: collection.id,
                snapshot_id: first_snapshot,
            })
            .unwrap()
            .digest,
        Digest::sha256(dat("1", "a17c9aaa61e80a1bf71d0d850af4e5baa9800bbd").as_bytes())
    );
    assert_eq!(
        store.active_snapshot(collection.id, &provider_id).unwrap(),
        Some(second_snapshot)
    );
    let rebuilt_metadata = service.metadata_view(&actor, game.id).unwrap();
    assert_eq!(rebuilt_metadata.candidates.len(), 1);
    assert_eq!(rebuilt_metadata.candidates[0].title.value, "Known Game");
}

#[test]
fn music_cd_workflow_publishes_shared_accuraterip_results_and_survives_reindex() {
    let (root, store, actor, base_service, collection) = setup();
    let frames = 451_u64 * 588;
    let mut pcm = Vec::with_capacity((frames * 4) as usize);
    for _ in 0..frames {
        pcm.extend_from_slice(&[1, 0, 0, 0]);
    }
    let checksum = ((2940_u64 + (frames - 2940)) * (frames - 5879) / 2) as u32;
    let id1 = 451_u32;
    let id2 = 903_u32;
    let cddb = 0x0200_0601_u32;
    let mut dbar = vec![1];
    dbar.extend_from_slice(&id1.to_le_bytes());
    dbar.extend_from_slice(&id2.to_le_bytes());
    dbar.extend_from_slice(&cddb.to_le_bytes());
    dbar.push(17);
    dbar.extend_from_slice(&checksum.to_le_bytes());
    dbar.extend_from_slice(&0_u32.to_le_bytes());
    let unavailable = Arc::new(AtomicBool::new(true));
    let no_data = Arc::new(AtomicBool::new(false));
    let provider = Arc::new(FixtureAccurateRipProvider {
        bytes: dbar,
        unavailable: unavailable.clone(),
        no_data: no_data.clone(),
    });
    let parser = Arc::new(
        AccurateRipProvider::with_test_origin(
            "junk-collector-test/1",
            "http://127.0.0.1:1/",
            std::time::Duration::from_millis(50),
        )
        .unwrap(),
    );
    let provider_id = ModuleId(ACCURATERIP_PROVIDER_ID.into());
    let service = base_service
        .with_streaming_provider(
            provider_id.clone(),
            Arc::new(collector_app::BoundedProviderAdapter(provider)),
        )
        .unwrap()
        .with_catalog_parser(provider_id.clone(), parser)
        .unwrap();
    let cue = b"FILE \"disc.bin\" BINARY\n  TRACK 01 AUDIO\n    INDEX 01 00:00:00\n";
    let created = service
        .create_provisional_import(
            &actor,
            CreateImportCommand {
                collection_id: collection.id,
                source: ImportSourceSummary {
                    kind: ImportSourceKind::Browser,
                    label: None,
                    package_name: "synthetic-bin-cue".into(),
                },
                files: vec![
                    UploadFile {
                        name: "disc.cue".into(),
                        bytes: ByteLength(cue.len() as u64),
                        sha256: None,
                    },
                    UploadFile {
                        name: "disc.bin".into(),
                        bytes: ByteLength(pcm.len() as u64),
                        sha256: None,
                    },
                ],
                association: CaptureAssociation::default(),
                expected_item_revision: None,
            },
            &command(),
        )
        .unwrap();
    service
        .stage_provisional_import_chunk(&actor, created.id, 0, 0, cue)
        .unwrap();
    service
        .stage_provisional_import_chunk(&actor, created.id, 1, 0, &pcm)
        .unwrap();
    let analysis_job = service
        .enqueue_import_analysis(
            &actor,
            created.id,
            ModuleId("org.junk.music".into()),
            &command(),
        )
        .unwrap();
    assert!(service.run_next_job("music-import-analysis").unwrap());
    assert_eq!(
        service.job(&actor, analysis_job).unwrap().state,
        JobState::Completed
    );
    let analyzed = service.provisional_import(&actor, created.id).unwrap();
    assert!(analyzed.candidates.is_empty());
    let reviewed = service
        .review_provisional_import(
            &actor,
            created.id,
            ReviewImportCommand {
                expected_revision: analyzed.revision,
                identity: ImportIdentityDecision::Unidentified,
                association: CaptureAssociation::default(),
                expected_item_revision: None,
            },
            &command(),
        )
        .unwrap();
    let publication = service
        .enqueue_import_publication(&actor, reviewed.id, &command())
        .unwrap();
    assert!(service.run_next_job("music-publication").unwrap());
    assert_eq!(
        service.job(&actor, publication).unwrap().state,
        JobState::Completed
    );
    let published = service.provisional_import(&actor, created.id).unwrap();
    let structure_job = service
        .enqueue_processing_operation(
            &actor,
            published.capture_id,
            StartProcessingCommand {
                operation: OperationId::new(collector_modules::music::CD_STRUCTURE_OPERATION)
                    .unwrap(),
                operation_version: 1,
                representation_id: published.representation_id,
                snapshot_id: None,
                selection: RawJson::from_value(serde_json::json!({
                    "package_kind":"cue",
                    "anchor_file_index":0
                })),
            },
            &command(),
        )
        .unwrap();
    assert!(service.run_next_job("music-structure").unwrap());
    assert_eq!(
        service.job(&actor, structure_job).unwrap().state,
        JobState::Completed
    );

    // Acquisition enforces collection roles before inspecting structural evidence.
    let viewer_id = store
        .create_user("cd-viewer", "viewer password", false)
        .unwrap();
    Catalog::grant(&*store, collection.id, viewer_id, Role::Viewer).unwrap();
    let viewer = Principal {
        user_id: viewer_id,
        administrator: false,
        device_id: None,
    };
    assert!(matches!(
        service.enqueue_accuraterip_acquisition(
            &viewer,
            published.capture_id,
            AcquireAccurateRipCommand {
                representation_id: published.representation_id,
                structure_observation_id: ObservationId(structure_job.0)
            },
            &command()
        ),
        Err(Error::Forbidden)
    ));

    let acquire = |service: &CollectionService| {
        service
            .enqueue_accuraterip_acquisition(
                &actor,
                published.capture_id,
                AcquireAccurateRipCommand {
                    representation_id: published.representation_id,
                    structure_observation_id: ObservationId(structure_job.0),
                },
                &command(),
            )
            .unwrap()
    };
    let outage_job = acquire(&service);
    assert!(service.run_next_job("accuraterip-outage").unwrap());
    assert_eq!(
        service.job(&actor, outage_job).unwrap().state,
        JobState::Failed
    );
    assert!(Catalog::capture(&*store, published.capture_id).is_ok());
    unavailable.store(false, Ordering::SeqCst);
    service.retry_job(&actor, outage_job, &command()).unwrap();
    assert!(service.run_next_job("accuraterip-retry").unwrap());
    assert_eq!(
        service.job(&actor, outage_job).unwrap().state,
        JobState::Completed
    );
    let first_snapshot = service
        .curation_catalog_snapshots(&actor, collection.id, None, 10)
        .unwrap()
        .items[0]
        .snapshot
        .id;

    let cancelled_job = acquire(&service);
    service
        .cancel_job(&actor, cancelled_job, &command())
        .unwrap();
    assert!(!service.run_next_job("accuraterip-cancelled").unwrap());
    assert_eq!(
        service.job(&actor, cancelled_job).unwrap().state,
        JobState::Cancelled
    );

    let refreshed_job = acquire(&service);
    assert!(service.run_next_job("accuraterip-refresh").unwrap());
    assert_eq!(
        service.job(&actor, refreshed_job).unwrap().state,
        JobState::Completed
    );
    let snapshots = service
        .curation_catalog_snapshots(&actor, collection.id, None, 10)
        .unwrap();
    assert_eq!(snapshots.items.len(), 2);
    let second_snapshot = snapshots
        .items
        .iter()
        .find(|value| value.snapshot.id != first_snapshot)
        .unwrap()
        .snapshot
        .id;

    for snapshot_id in [first_snapshot, second_snapshot] {
        let verification = service
            .enqueue_processing_operation(
                &actor,
                published.capture_id,
                StartProcessingCommand {
                    operation: OperationId::new(
                        collector_modules::music::ACCURATERIP_VERIFICATION_OPERATION,
                    )
                    .unwrap(),
                    operation_version: 1,
                    representation_id: published.representation_id,
                    snapshot_id: Some(snapshot_id),
                    selection: RawJson::from_value(serde_json::json!({
                        "package_kind":"cue",
                        "anchor_file_index":0,
                        "snapshot_id":snapshot_id,
                        "maximum_offset":0
                    })),
                },
                &command(),
            )
            .unwrap();
        assert!(service.run_next_job("accuraterip-verification").unwrap());
        let job = service.job(&actor, verification).unwrap();
        assert_eq!(job.state, JobState::Completed, "{:?}", job.error);
    }
    let summaries = service
        .capture_processing_summaries(
            &actor,
            published.capture_id,
            published.representation_id,
            None,
            20,
        )
        .unwrap();
    let verified = summaries.items.iter().filter(|summary| {
        summary.operation.as_str() == collector_modules::music::ACCURATERIP_VERIFICATION_OPERATION
            && summary.result.get().contains("\"status\":\"verified\"")
            && summary.result.get().contains("\"computed_offset\":0")
    });
    assert_eq!(verified.count(), 2);

    let collection_dir = std::fs::read_dir(root.path().join("archive/collections"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let snapshot_payload = collection_dir
        .join("snapshots")
        .join(first_snapshot.to_string())
        .join("payload/source.bin");
    let original_snapshot = std::fs::read(&snapshot_payload).unwrap();
    let mut corrupted_snapshot = original_snapshot.clone();
    corrupted_snapshot[0] ^= 0xff;
    std::fs::write(&snapshot_payload, corrupted_snapshot).unwrap();
    let corrupt_verification = service
        .enqueue_processing_operation(
            &actor,
            published.capture_id,
            StartProcessingCommand {
                operation: OperationId::new(
                    collector_modules::music::ACCURATERIP_VERIFICATION_OPERATION,
                )
                .unwrap(),
                operation_version: 1,
                representation_id: published.representation_id,
                snapshot_id: Some(first_snapshot),
                selection: RawJson::from_value(serde_json::json!({
                    "package_kind":"cue",
                    "anchor_file_index":0,
                    "snapshot_id":first_snapshot,
                    "maximum_offset":0
                })),
            },
            &command(),
        )
        .unwrap();
    assert!(service.run_next_job("corrupt-snapshot").unwrap());
    assert_eq!(
        service.job(&actor, corrupt_verification).unwrap().state,
        JobState::Failed
    );
    std::fs::write(&snapshot_payload, original_snapshot).unwrap();
    service
        .retry_job(&actor, corrupt_verification, &command())
        .unwrap();
    assert!(service.run_next_job("restored-snapshot").unwrap());
    assert_eq!(
        service.job(&actor, corrupt_verification).unwrap().state,
        JobState::Completed
    );
    no_data.store(true, Ordering::SeqCst);
    let no_data_acquisition = acquire(&service);
    assert!(service.run_next_job("accuraterip-no-data").unwrap());
    assert_eq!(
        service.job(&actor, no_data_acquisition).unwrap().state,
        JobState::Completed
    );
    let no_data_snapshot = service
        .curation_catalog_snapshots(&actor, collection.id, None, 10)
        .unwrap()
        .items
        .into_iter()
        .find(|value| value.snapshot.parsed_observation.get().contains("no_data"))
        .unwrap()
        .snapshot
        .id;
    let no_data_verification = service
        .enqueue_processing_operation(
            &actor,
            published.capture_id,
            StartProcessingCommand {
                operation: OperationId::new(
                    collector_modules::music::ACCURATERIP_VERIFICATION_OPERATION,
                )
                .unwrap(),
                operation_version: 1,
                representation_id: published.representation_id,
                snapshot_id: Some(no_data_snapshot),
                selection: RawJson::from_value(serde_json::json!({
                    "package_kind":"cue",
                    "anchor_file_index":0,
                    "snapshot_id":no_data_snapshot,
                    "maximum_offset":0
                })),
            },
            &command(),
        )
        .unwrap();
    assert!(
        service
            .run_next_job("accuraterip-no-data-verification")
            .unwrap()
    );
    assert_eq!(
        service.job(&actor, no_data_verification).unwrap().state,
        JobState::Completed
    );
    assert!(
        service
            .capture_processing_summaries(
                &actor,
                published.capture_id,
                published.representation_id,
                None,
                20,
            )
            .unwrap()
            .items
            .iter()
            .any(|summary| summary.snapshot_id == Some(no_data_snapshot)
                && summary.result.get().contains("\"status\":\"no_data\""))
    );
    store.reindex().unwrap();
    for snapshot_id in [first_snapshot, second_snapshot] {
        assert!(
            store
                .snapshot(SnapshotQuery {
                    collection_id: collection.id,
                    snapshot_id
                })
                .is_ok()
        );
    }

    // Exercise both an unsupported lead-in and a complete supported raw package.
    for supported in [false, true] {
        let toc = if supported {
            synthetic_redumper_toc(0, 451)
        } else {
            synthetic_redumper_toc(-45_150, -45_149)
        };
        let mut raw = if supported {
            vec![0_u8; 45_150 * 2352]
        } else {
            Vec::new()
        };
        if supported {
            raw.extend_from_slice(&pcm);
        } else {
            raw.resize(2352, 0);
        }
        let sectors = raw.len() / 2352;
        let redumper_files = [
            ("rip.toc", toc),
            ("rip.scram", raw),
            ("rip.state", vec![4_u8; sectors * 588]),
            ("rip.subcode", vec![0_u8; sectors * 96]),
        ];
        let redumper = service
            .create_provisional_import(
                &actor,
                CreateImportCommand {
                    collection_id: collection.id,
                    source: ImportSourceSummary {
                        kind: ImportSourceKind::Browser,
                        label: None,
                        package_name: "synthetic-redumper".into(),
                    },
                    files: redumper_files
                        .iter()
                        .map(|(name, bytes)| UploadFile {
                            name: (*name).into(),
                            bytes: ByteLength(bytes.len() as u64),
                            sha256: None,
                        })
                        .collect(),
                    association: CaptureAssociation::default(),
                    expected_item_revision: None,
                },
                &command(),
            )
            .unwrap();
        for (index, (_, bytes)) in redumper_files.iter().enumerate() {
            for (chunk, bytes) in bytes.chunks(1024 * 1024).enumerate() {
                service
                    .stage_provisional_import_chunk(
                        &actor,
                        redumper.id,
                        index,
                        (chunk * 1024 * 1024) as u64,
                        bytes,
                    )
                    .unwrap();
            }
        }
        let redumper_analysis = service
            .enqueue_import_analysis(
                &actor,
                redumper.id,
                ModuleId("org.junk.music".into()),
                &command(),
            )
            .unwrap();
        assert!(service.run_next_job("redumper-analysis").unwrap());
        assert_eq!(
            service.job(&actor, redumper_analysis).unwrap().state,
            JobState::Completed
        );
        let analyzed = service.provisional_import(&actor, redumper.id).unwrap();
        assert_eq!(
            analyzed
                .limitations
                .iter()
                .any(|value| value.contains("lead-in")),
            !supported
        );
        let reviewed = service
            .review_provisional_import(
                &actor,
                redumper.id,
                ReviewImportCommand {
                    expected_revision: analyzed.revision,
                    identity: ImportIdentityDecision::Unidentified,
                    association: CaptureAssociation::default(),
                    expected_item_revision: None,
                },
                &command(),
            )
            .unwrap();
        let redumper_publication = service
            .enqueue_import_publication(&actor, reviewed.id, &command())
            .unwrap();
        assert!(service.run_next_job("redumper-publication").unwrap());
        assert_eq!(
            service.job(&actor, redumper_publication).unwrap().state,
            JobState::Completed
        );
        let redumper = service.provisional_import(&actor, redumper.id).unwrap();
        let redumper_structure = service
            .enqueue_processing_operation(
                &actor,
                redumper.capture_id,
                StartProcessingCommand {
                    operation: OperationId::new(collector_modules::music::CD_STRUCTURE_OPERATION)
                        .unwrap(),
                    operation_version: 1,
                    representation_id: redumper.representation_id,
                    snapshot_id: None,
                    selection: RawJson::from_value(serde_json::json!({
                        "package_kind":"redumper",
                        "anchor_file_index":0
                    })),
                },
                &command(),
            )
            .unwrap();
        assert!(service.run_next_job("redumper-structure").unwrap());
        assert_eq!(
            service.job(&actor, redumper_structure).unwrap().state,
            JobState::Completed
        );
        let redumper_verification = service
            .enqueue_processing_operation(
                &actor,
                redumper.capture_id,
                StartProcessingCommand {
                    operation: OperationId::new(
                        collector_modules::music::ACCURATERIP_VERIFICATION_OPERATION,
                    )
                    .unwrap(),
                    operation_version: 1,
                    representation_id: redumper.representation_id,
                    snapshot_id: Some(first_snapshot),
                    selection: RawJson::from_value(serde_json::json!({
                        "package_kind":"redumper",
                        "anchor_file_index":0,
                        "snapshot_id":first_snapshot,
                        "maximum_offset":0
                    })),
                },
                &command(),
            )
            .unwrap();
        assert!(service.run_next_job("redumper-verification").unwrap());
        assert_eq!(
            service.job(&actor, redumper_verification).unwrap().state,
            JobState::Completed
        );
        let redumper_summaries = service
            .capture_processing_summaries(
                &actor,
                redumper.capture_id,
                redumper.representation_id,
                None,
                10,
            )
            .unwrap();
        assert!(redumper_summaries.items.iter().any(|summary| {
            summary.operation.as_str()
                == collector_modules::music::ACCURATERIP_VERIFICATION_OPERATION
                && if supported {
                    matches!(summary.support, OperationSupport::Supported)
                        && summary.result.get().contains("\"status\":\"verified\"")
                } else {
                    matches!(
                        summary.support,
                        OperationSupport::UnavailableDependency { .. }
                    )
                }
        }));
        assert!(Catalog::capture(&*store, redumper.capture_id).is_ok());
        store.reindex().unwrap();
        assert!(
            store
                .processing_record(ObservationId(redumper_structure.0))
                .is_ok()
        );
    }
    drop(service);
    drop(store);
    let restored = Store::open(
        root.path().join("archive"),
        root.path().join("reindexed-state"),
    )
    .unwrap();
    assert!(
        restored
            .processing_record(ObservationId(structure_job.0))
            .is_ok()
    );
}

#[test]
fn acquisition_restart_after_snapshot_commit_reuses_frozen_response() {
    let (root, store, actor, service, collection) = setup();
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let provider_id = ModuleId("org.example.catalog".into());
    let provider = Arc::new(FixtureStreamingProvider {
        calls: calls.clone(),
        bytes: b"one immutable provider response".to_vec(),
    });
    let service = service
        .with_streaming_provider(provider_id.clone(), provider.clone())
        .unwrap();
    let snapshot_id = SnapshotId::new();
    let frozen = FrozenAcquisitionJob {
        collection_binding: collection.binding.clone(),
        request: ProviderRequest {
            collection_id: collection.id,
            provider: provider_id.clone(),
            operation: OperationId::new("org.example.catalog.acquire").unwrap(),
            source_fingerprint: None,
            query: RawJson::from_value(serde_json::json!({"catalog":"fixture"})),
            maximum_response_bytes: 1024,
        },
        item_id: None,
        snapshot_id,
        created_at: chrono::Utc::now(),
    };
    let mut mismatched = frozen.clone();
    mismatched.collection_binding = CollectionBinding::Assigned {
        module: ModuleId("org.junk.games".into()),
    };
    assert!(matches!(
        service.enqueue_catalog_acquisition(&actor, mismatched, &command()),
        Err(Error::Conflict)
    ));
    let job = service
        .enqueue_catalog_acquisition(&actor, frozen, &command())
        .unwrap();
    let private = rusqlite::Connection::open(root.path().join("state/private.sqlite")).unwrap();
    let stored: String = private
        .query_row(
            "SELECT frozen_input FROM processing_jobs WHERE job_id=?",
            [job.to_string()],
            |row| row.get(0),
        )
        .unwrap();
    assert!(stored.contains("acquisition"));
    assert!(stored.contains(&snapshot_id.to_string()));
    drop(private);
    drop(service);
    drop(store);

    let crashing = Arc::new(
        Store::open_with_publication_fault(
            root.path().join("archive"),
            root.path().join("state"),
            Arc::new(PanicAtImportCommit),
        )
        .unwrap(),
    );
    let crashing_service = CollectionService::new(crashing.clone(), Arc::new(Registry::builtins()))
        .with_streaming_provider(provider_id.clone(), provider.clone())
        .unwrap();
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            crashing_service.run_next_job("acquisition-old")
        }))
        .is_err()
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    drop(crashing_service);
    drop(crashing);
    let mut attempts = 0;
    let reopened = loop {
        match Store::open(root.path().join("archive"), root.path().join("state")) {
            Ok(store) => break Arc::new(store),
            Err(Error::Unavailable(_)) if attempts < 100 => {
                attempts += 1;
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            Err(error) => panic!("processing recovery failed: {error}"),
        }
    };
    let recovered = CollectionService::new(reopened.clone(), Arc::new(Registry::default()));
    assert!(recovered.run_next_job("acquisition-new").unwrap());
    assert_eq!(
        recovered.job(&actor, job).unwrap().state,
        JobState::Completed
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        reopened
            .snapshot(SnapshotQuery {
                collection_id: collection.id,
                snapshot_id,
            })
            .unwrap()
            .digest,
        Digest::sha256(b"one immutable provider response")
    );
}

#[test]
fn processing_summary_marks_changed_input_historical_and_denies_other_collection() {
    let (_root, store, actor, _base_service, collection) = setup();
    let mut registry = Registry::default();
    registry.register(Box::new(ImportAnalyzingModule)).unwrap();
    let service = CollectionService::new(store.clone(), Arc::new(registry));
    let import = staged_import(&service, &actor, collection.id, "currency");
    let job = service
        .enqueue_import_analysis(
            &actor,
            import.id,
            ModuleId("org.junk.music".into()),
            &command(),
        )
        .unwrap();
    service.run_next_job("analysis").unwrap();
    let analyzed = service.provisional_import(&actor, import.id).unwrap();
    let provisional = store.provisional_observations(import.id).unwrap().remove(0);
    let reviewed = service
        .review_provisional_import(
            &actor,
            import.id,
            ReviewImportCommand {
                expected_revision: analyzed.revision,
                identity: ImportIdentityDecision::Unidentified,
                association: CaptureAssociation::default(),
                expected_item_revision: None,
            },
            &command(),
        )
        .unwrap();
    let publication = service
        .enqueue_import_publication(&actor, reviewed.id, &command())
        .unwrap();
    service.run_next_job("publication").unwrap();
    assert_eq!(
        service.job(&actor, publication).unwrap().state,
        JobState::Completed
    );
    let portable = store.processing_record(provisional.id).unwrap();
    // Exercise SQL pagination past its first LIMIT window, including tied times.
    let db = rusqlite::Connection::open(_root.path().join("state/catalog.sqlite")).unwrap();
    for _ in 0..6 {
        let mut copy = portable.clone();
        copy.observation.id = ObservationId::new();
        db.execute("INSERT INTO processing_observations(id,collection_id,source_scope,source_id,operation,created_at,record) VALUES(?,?,?,?,?,?,?)",
            rusqlite::params![copy.observation.id.to_string(),collection.id.to_string(),"representation",import.representation_id.to_string(),copy.observation.operation.to_string(),copy.observation.created_at.timestamp(),serde_json::to_string(&copy).unwrap()]).unwrap();
    }
    let mut page_query = ProcessingQuery {
        collection_id: collection.id,
        source: PackageScope::Representation(import.representation_id),
        source_fingerprint: portable.observation.source_fingerprint.clone(),
        operation: None,
        after: None,
        limit: 2,
    };
    let mut seen = std::collections::HashSet::new();
    loop {
        let page = collector_app::ProcessingReadPort::observations(&*store, &page_query).unwrap();
        for entry in page.items {
            assert!(seen.insert(entry.id));
        }
        match page.next_cursor {
            Some(cursor) => page_query.after = Some(cursor.parse().unwrap()),
            None => break,
        }
    }
    assert_eq!(seen.len(), 7);
    page_query.after = Some(ObservationId::new());
    assert!(collector_app::ProcessingReadPort::observations(&*store, &page_query).is_err());
    db.execute(
        "DELETE FROM processing_observations WHERE id != ?",
        [provisional.id.to_string()],
    )
    .unwrap();
    let query = ProcessingQuery {
        collection_id: collection.id,
        source: PackageScope::Representation(import.representation_id),
        source_fingerprint: portable.observation.source_fingerprint.clone(),
        operation: None,
        after: None,
        limit: 10,
    };
    assert!(
        service
            .processing_summaries(&actor, query.clone())
            .unwrap()
            .items[0]
            .current
    );
    // Historical evidence remains readable when its module is not installed.
    let without_modules = CollectionService::new(store.clone(), Arc::new(Registry::default()));
    assert_eq!(
        without_modules
            .processing_summaries(&actor, query.clone())
            .unwrap()
            .items
            .len(),
        1
    );
    let mut changed_files = query.source_fingerprint.files.clone();
    changed_files[0].sha256 = Digest::sha256(b"changed bytes");
    let changed = SourceFingerprint::calculate(
        query.source,
        changed_files,
        query.source_fingerprint.layout_profile_digest.clone(),
        query.source_fingerprint.parameters.clone(),
    )
    .unwrap();
    let historical = service
        .processing_summaries(
            &actor,
            ProcessingQuery {
                source_fingerprint: changed,
                ..query.clone()
            },
        )
        .unwrap();
    assert!(
        historical.items[0].current,
        "client-supplied fingerprints cannot change authoritative currency"
    );
    let db = rusqlite::Connection::open(_root.path().join("state/catalog.sqlite")).unwrap();
    let mut capture = Catalog::capture(&*store, import.capture_id).unwrap();
    capture.representations[0].files[0].digest = Digest::sha256(b"new!");
    db.execute(
        "UPDATE captures SET record=? WHERE id=?",
        rusqlite::params![
            serde_json::to_string(&capture).unwrap(),
            capture.id.to_string()
        ],
    )
    .unwrap();
    assert!(
        !service
            .processing_summaries(&actor, query.clone())
            .unwrap()
            .items[0]
            .current
    );
    assert_eq!(historical.items[0].observation_id.0, job.0);

    let stranger = Principal {
        user_id: store
            .create_user("stranger", "another long password", false)
            .unwrap(),
        administrator: false,
        device_id: None,
    };
    assert!(matches!(
        service.processing_summaries(&stranger, query),
        Err(Error::Forbidden)
    ));
}

#[test]
fn representation_executor_uses_registered_owner_and_publishes_once() {
    let (_root, store, actor, base_service, collection) = setup();
    let reviewed =
        reviewed_unattached_import(&base_service, &actor, collection.id, "representation-op");
    let publication = base_service
        .enqueue_import_publication(&actor, reviewed.id, &command())
        .unwrap();
    base_service.run_next_job("publish").unwrap();
    assert_eq!(
        base_service.job(&actor, publication).unwrap().state,
        JobState::Completed
    );

    let mut registry = Registry::builtins();
    registry.register(Box::new(ReaderProcessingModule)).unwrap();
    let service = CollectionService::new(store.clone(), Arc::new(registry));
    let context = command();
    let request = collector_app::StartProcessingCommand {
        operation: OperationId::new("org.example.reader.inspect").unwrap(),
        operation_version: 1,
        representation_id: reviewed.representation_id,
        snapshot_id: None,
        selection: RawJson::from_value(serde_json::json!({})),
    };
    let job = service
        .enqueue_processing_operation(&actor, reviewed.capture_id, request.clone(), &context)
        .unwrap();
    assert_eq!(
        service
            .enqueue_processing_operation(&actor, reviewed.capture_id, request, &context)
            .unwrap(),
        job
    );
    service.run_next_job("processing").unwrap();
    assert_eq!(service.job(&actor, job).unwrap().state, JobState::Completed);
    let record = store.processing_record(ObservationId(job.0)).unwrap();
    assert_eq!(
        record.observation.operation.as_str(),
        "org.example.reader.inspect"
    );
    assert_eq!(
        record.observation.typed_result.payload.value().unwrap()["sha256"],
        Digest::sha256(b"data").hex
    );
    assert!(record.import_rebinding.is_none());
    assert!(!service.run_next_job("processing").unwrap());
}

#[test]
fn recipe_and_metadata_authority_are_validated_before_v4_publication() {
    let (_root, store, actor, service, collection) = setup();
    let saved = save_inventory(&service, &actor, item(collection.id), Revision(0)).unwrap();
    let invalid_authority = MetadataAuthorityRecord {
        current_candidates: None,
        inventory: None,
        collection_id: collection.id,
        item_id: saved.id,
        revision: Revision(1),
        authority: MetadataAuthority {
            field_overrides: vec![
                MetadataFieldOverride {
                    field: MetadataField::Title,
                    value: "one".into(),
                    language: None,
                },
                MetadataFieldOverride {
                    field: MetadataField::Title,
                    value: "two".into(),
                    language: None,
                },
            ],
            ..Default::default()
        },
        observations: vec![],
        display: None,
        updated_at: chrono::Utc::now(),
    };
    assert!(matches!(
        store.publish_metadata_authority(TransactionId::new(), invalid_authority),
        Err(Error::Invalid(_))
    ));

    let imported = reviewed_unattached_import(&service, &actor, collection.id, "recipe-source");
    service
        .enqueue_import_publication(&actor, imported.id, &command())
        .unwrap();
    service.run_next_job("recipe-source").unwrap();
    let source = store
        .recorded_source(
            collection.id,
            PackageScope::Representation(imported.representation_id),
        )
        .unwrap();
    let recipe = ConversionRecipe {
        id: RecipeId::new(),
        version: 1,
        settings: ConversionSettings::Flac { level: 5 },
        source_fingerprint: source,
        tracks: vec![ConversionTrack {
            position: 1,
            start: PcmFrame(0),
            length: PcmFrame(2),
        }],
        encoder: "flac".into(),
        encoder_version: "1.5.0".into(),
        input_format: PcmSampleFormat {
            sample_rate: 44100,
            channels: 2,
            bits_per_sample: 16,
            signed: true,
            little_endian: true,
        },
        output_format: PcmSampleFormat {
            sample_rate: 44100,
            channels: 2,
            bits_per_sample: 16,
            signed: true,
            little_endian: true,
        },
        digest: Digest::sha256(b"not canonical"),
    };
    assert!(matches!(
        store.publish_recipe(JobId::new(), collection.id, recipe.clone()),
        Err(Error::Invalid(_))
    ));
    let sealed = recipe.seal().unwrap();
    store
        .publish_recipe(JobId::new(), collection.id, sealed.clone())
        .unwrap();
    assert_eq!(store.recipe(sealed.id).unwrap().digest, sealed.digest);

    let authority = MetadataAuthorityRecord {
        current_candidates: None,
        inventory: None,
        collection_id: collection.id,
        item_id: saved.id,
        revision: Revision(1),
        authority: MetadataAuthority {
            retained_unknown_fields: std::collections::BTreeMap::from([(
                "future-authority".into(),
                RawJson::from_json(r#"{"ordered":[2,1],"number":1e+02}"#).unwrap(),
            )]),
            ..Default::default()
        },
        observations: vec![],
        display: None,
        updated_at: chrono::Utc::now(),
    };
    store
        .publish_metadata_authority(TransactionId::new(), authority)
        .unwrap();
    AdministrationPort::reindex(&*store).unwrap();
    let db = rusqlite::Connection::open(_root.path().join("state/catalog.sqlite")).unwrap();
    assert_eq!(
        db.query_row("SELECT COUNT(*) FROM conversion_recipes", [], |row| row
            .get::<_, u32>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        db.query_row("SELECT COUNT(*) FROM metadata_authorities", [], |row| row
            .get::<_, u32>(
            0
        ))
        .unwrap(),
        1
    );
    let encoded: String = db
        .query_row("SELECT record FROM metadata_authorities", [], |row| {
            row.get(0)
        })
        .unwrap();
    let rebuilt: MetadataAuthorityRecord = serde_json::from_str(&encoded).unwrap();
    assert_eq!(
        rebuilt.authority.retained_unknown_fields["future-authority"].get(),
        r#"{"ordered":[2,1],"number":1e+02}"#
    );
}

#[test]
fn metadata_authority_refresh_selection_replay_and_reindex_preserve_manual_and_raw_history() {
    let (root, store, actor, service, collection) = setup();
    let saved = save_inventory(&service, &actor, item(collection.id), Revision(0)).unwrap();
    let japanese = MetadataCandidateObservation {
        identity: MetadataCandidateIdentity {
            provider: ModuleId("org.example.musicbrainz".into()),
            provider_record_id: "release-ja".into(),
            provider_record_version: Some("2026-09-15".into()),
        },
        title: MetadataText {
            value: "強い候補".into(),
            language: Some(LanguageTag::new("ja").unwrap()),
        },
        subtitle: None,
        artwork: None,
        complete_identity: true,
        strength: MatchStrength::Identifier,
        provider_priority: 20,
        observation: RawJson::from_json(r#"{"unknown":null,"number":1e+02}"#).unwrap(),
    };
    let english = MetadataCandidateObservation {
        identity: MetadataCandidateIdentity {
            provider: ModuleId("org.example.names".into()),
            provider_record_id: "name-en".into(),
            provider_record_version: None,
        },
        title: MetadataText {
            value: "English candidate".into(),
            language: Some(LanguageTag::new("en").unwrap()),
        },
        subtitle: None,
        artwork: None,
        complete_identity: false,
        strength: MatchStrength::Name,
        provider_priority: 1,
        observation: RawJson::from_json(r#"{"provider_extra":[2,1]}"#).unwrap(),
    };
    let refresh = RefreshMetadataCommand {
        expected_revision: Revision(0),
        observation: MetadataRefreshObservation {
            id: ObservationId::new(),
            outcome: MetadataRefreshOutcome::Candidates,
            candidates: vec![english.clone(), japanese.clone()],
            raw_observation: RawJson::from_json(
                r#"{"response":{"unknown":null,"number":1e+02},"ordered":[2,1]}"#,
            )
            .unwrap(),
            observed_at: chrono::Utc::now(),
        },
    };
    let first = service
        .refresh_metadata(&actor, saved.id, refresh, &command())
        .unwrap();
    assert_eq!(
        first.display.as_ref().unwrap().title.value,
        "風の歌",
        "the existing inventory title becomes manual authority"
    );

    let mut altered = japanese.clone();
    altered.title.value = "Silently replaced".into();
    assert!(matches!(
        service.refresh_metadata(
            &actor,
            saved.id,
            RefreshMetadataCommand {
                expected_revision: Revision(1),
                observation: MetadataRefreshObservation {
                    id: ObservationId::new(),
                    outcome: MetadataRefreshOutcome::Candidates,
                    candidates: vec![altered],
                    raw_observation: RawJson::from_json("{}").unwrap(),
                    observed_at: chrono::Utc::now(),
                }
            },
            &command()
        ),
        Err(Error::Conflict)
    ));

    let cleared = service
        .update_metadata_authority(
            &actor,
            saved.id,
            UpdateMetadataAuthorityCommand {
                expected_revision: Revision(1),
                change: MetadataAuthorityChange::ClearFieldOverride {
                    field: MetadataField::Title,
                },
            },
            &command(),
        )
        .unwrap();
    assert_eq!(cleared.display.as_ref().unwrap().title.value, "強い候補");

    let selected = service
        .update_metadata_authority(
            &actor,
            saved.id,
            UpdateMetadataAuthorityCommand {
                expected_revision: Revision(2),
                change: MetadataAuthorityChange::SelectCandidate {
                    candidate: english.identity.clone(),
                },
            },
            &command(),
        )
        .unwrap();
    assert_eq!(
        selected.display.as_ref().unwrap().title.value,
        "English candidate"
    );
    assert!(matches!(
        service.update_metadata_authority(
            &actor,
            saved.id,
            UpdateMetadataAuthorityCommand {
                expected_revision: Revision(2),
                change: MetadataAuthorityChange::ClearSelectedCandidate,
            },
            &command(),
        ),
        Err(Error::Conflict)
    ));

    let outage_context = command();
    let outage = RefreshMetadataCommand {
        expected_revision: Revision(3),
        observation: MetadataRefreshObservation {
            id: ObservationId::new(),
            outcome: MetadataRefreshOutcome::Unavailable,
            candidates: vec![],
            raw_observation: RawJson::from_json(r#"{"error":"offline","unknown":{"kept":true}}"#)
                .unwrap(),
            observed_at: chrono::Utc::now(),
        },
    };
    let after_outage = service
        .refresh_metadata(&actor, saved.id, outage.clone(), &outage_context)
        .unwrap();
    let replayed = service
        .refresh_metadata(&actor, saved.id, outage, &outage_context)
        .unwrap();
    assert_eq!(replayed.revision, after_outage.revision);
    assert_eq!(
        replayed.display.as_ref().unwrap().title.value,
        "English candidate",
        "a selected historical candidate survives a provider outage"
    );
    let changed_replay = RefreshMetadataCommand {
        expected_revision: Revision(3),
        observation: MetadataRefreshObservation {
            id: ObservationId::new(),
            outcome: MetadataRefreshOutcome::Unavailable,
            candidates: vec![],
            raw_observation: RawJson::from_json(r#"{"error":"different"}"#).unwrap(),
            observed_at: chrono::Utc::now(),
        },
    };
    let changed_replay =
        service.refresh_metadata(&actor, saved.id, changed_replay, &outage_context);
    assert!(
        matches!(changed_replay, Err(Error::IdempotencyConflict)),
        "changed replay returned {changed_replay:?}"
    );

    let curator_user = store
        .create_user("metadata-curator", "another long password", false)
        .unwrap();
    Catalog::grant(&*store, collection.id, curator_user, Role::Curator).unwrap();
    let curator = Principal {
        user_id: curator_user,
        administrator: false,
        device_id: None,
    };
    let curator_context = command();
    let curator_update = UpdateMetadataAuthorityCommand {
        expected_revision: Revision(4),
        change: MetadataAuthorityChange::SetFieldOverride {
            field: MetadataField::Subtitle,
            value: "Curator note".into(),
            language: Some(LanguageTag::new("en").unwrap()),
        },
    };
    service
        .update_metadata_authority(&curator, saved.id, curator_update.clone(), &curator_context)
        .unwrap();
    AccessPort::set_grant(&*store, collection.id, curator_user, Revision(1), None).unwrap();
    assert!(matches!(
        service.update_metadata_authority(&curator, saved.id, curator_update, &curator_context,),
        Err(Error::Forbidden)
    ));

    AdministrationPort::reindex(&*store).unwrap();
    let rebuilt = service.metadata_view(&actor, saved.id).unwrap();
    assert_eq!(rebuilt.revision, Revision(5));
    assert_eq!(rebuilt.candidates.len(), 2);
    assert_eq!(rebuilt.display.title.value, "English candidate");
    assert_eq!(
        rebuilt
            .candidates
            .iter()
            .find(|candidate| candidate.identity == japanese.identity)
            .unwrap()
            .observation
            .get(),
        r#"{"unknown":null,"number":1e+02}"#
    );

    let authority_files = files_named(&root.path().join("archive"), "record.v4.json");
    assert!(authority_files.iter().any(|path| {
        std::fs::read_to_string(path).is_ok_and(|body| {
            body.contains(r#""number":1e+02"#) && body.contains(r#""ordered":[2,1]"#)
        })
    }));

    let stranger = Principal {
        user_id: store
            .create_user("metadata-stranger", "another long password", false)
            .unwrap(),
        administrator: false,
        device_id: None,
    };
    assert!(matches!(
        service.update_metadata_authority(
            &stranger,
            saved.id,
            UpdateMetadataAuthorityCommand {
                expected_revision: Revision(5),
                change: MetadataAuthorityChange::ClearSelectedCandidate,
            },
            &command(),
        ),
        Err(Error::Forbidden)
    ));

    let archive = root.path().join("archive");
    let state = root.path().join("state");
    drop(service);
    drop(store);
    let reopened = Arc::new(Store::open(archive, state).unwrap());
    let reopened_service = CollectionService::new(reopened, Arc::new(Registry::builtins()));
    let rebuilt = reopened_service.metadata_view(&actor, saved.id).unwrap();
    assert_eq!(rebuilt.revision, Revision(5));
    assert_eq!(rebuilt.display.title.value, "English candidate");
}

#[test]
fn metadata_provider_jobs_publish_art_before_refresh_and_preserve_manual_authority() {
    let (root, store, actor, service, collection) = setup();
    let saved = save_inventory(&service, &actor, item(collection.id), Revision(0)).unwrap();
    let release_id = "00000000-0000-4000-8000-000000000001";
    let mb_body = format!(
        r#"{{"releases":[{{"id":"{release_id}","title":"Provider title","artist-credit":[{{"name":"Provider Artist","artist":{{"id":"00000000-0000-4000-8000-000000000002","name":"Provider Artist"}}}}],"text-representation":{{"language":"eng","script":"Latn"}},"media":[{{"discs":[{{"id":"known-disc"}}]}}],"release-group":{{"id":"00000000-0000-4000-8000-000000000003"}},"raw-number":1e+02}}],"count":1}}"#
    )
    .into_bytes();
    let mb_server = metadata_fixture_server(vec![
        ("200 OK\r\nContent-Type: application/json", mb_body),
        (
            "404 Not Found\r\nContent-Type: text/html",
            b"<html>Not found</html>".to_vec(),
        ),
    ]);
    let musicbrainz = Arc::new(
        MusicBrainzProvider::with_test_origin(
            "junk-collector-test/0.1 (test@example.invalid)",
            &mb_server.origin,
            Duration::from_secs(1),
            Duration::ZERO,
            Duration::from_secs(60),
            Duration::from_secs(60),
        )
        .unwrap(),
    );

    let mut encoded = std::io::Cursor::new(Vec::new());
    image::RgbImage::from_pixel(24, 24, image::Rgb([12, 34, 56]))
        .write_to(&mut encoded, image::ImageFormat::Png)
        .unwrap();
    let valid_art = encoded.into_inner();
    let caa_server = metadata_fixture_server(vec![
        ("200 OK\r\nContent-Type: image/png", valid_art.clone()),
        (
            "200 OK\r\nContent-Type: image/png",
            b"not an image".to_vec(),
        ),
    ]);
    let cover_art = Arc::new(
        CoverArtArchiveProvider::with_test_origins(
            "junk-collector-test/0.1 (test@example.invalid)",
            &caa_server.origin,
            &[caa_server.origin.as_str()],
            Duration::from_secs(1),
        )
        .unwrap(),
    );
    let service = service
        .with_streaming_provider(
            ModuleId(collector_modules::music::MUSICBRAINZ_PROVIDER_ID.into()),
            Arc::new(collector_app::BoundedProviderAdapter(musicbrainz.clone())),
        )
        .unwrap()
        .with_catalog_parser(
            ModuleId(collector_modules::music::MUSICBRAINZ_PROVIDER_ID.into()),
            musicbrainz,
        )
        .unwrap()
        .with_streaming_provider(
            ModuleId(collector_modules::music::COVER_ART_ARCHIVE_PROVIDER_ID.into()),
            Arc::new(collector_app::BoundedProviderAdapter(cover_art.clone())),
        )
        .unwrap()
        .with_catalog_parser(
            ModuleId(collector_modules::music::COVER_ART_ARCHIVE_PROVIDER_ID.into()),
            cover_art,
        )
        .unwrap();

    let mb_context = command();
    let mb_job = service
        .enqueue_musicbrainz_metadata_acquisition(
            &actor,
            saved.id,
            collector_modules::music::MusicBrainzQuery::DiscId {
                disc_id: "known-disc".into(),
                refresh: true,
            },
            &mb_context,
        )
        .unwrap();
    std::thread::sleep(std::time::Duration::from_millis(2));
    assert_eq!(
        service
            .enqueue_musicbrainz_metadata_acquisition(
                &actor,
                saved.id,
                collector_modules::music::MusicBrainzQuery::DiscId {
                    disc_id: "known-disc".into(),
                    refresh: true
                },
                &mb_context
            )
            .unwrap(),
        mb_job
    );
    assert!(matches!(
        service.enqueue_musicbrainz_metadata_acquisition(
            &actor,
            saved.id,
            collector_modules::music::MusicBrainzQuery::Search {
                query: "different".into(),
                refresh: true
            },
            &mb_context
        ),
        Err(Error::IdempotencyConflict)
    ));
    assert!(service.run_next_job("metadata-provider").unwrap());
    assert_eq!(
        service.job(&actor, mb_job).unwrap().state,
        JobState::Completed
    );

    let art_job = service
        .enqueue_cover_art_acquisition(
            &actor,
            saved.id,
            collector_modules::music::CoverArtArchiveQuery::Image {
                release_id: release_id.into(),
                image_id: "12345".into(),
            },
            &command(),
        )
        .unwrap();
    assert!(service.run_next_job("metadata-provider").unwrap());
    let provider_art = service
        .publish_provider_artwork(
            &actor,
            saved.id,
            PublishProviderArtworkCommand {
                snapshot_id: SnapshotId(art_job.0),
            },
            &command(),
        )
        .unwrap();
    assert!(!provider_art.deleted);
    assert_eq!(provider_art.asset.file.digest, Digest::sha256(&valid_art));

    let manual = service
        .create_asset(
            &actor,
            CreateAssetCommand {
                collection_id: collection.id,
                subject: AssetSubject::Item { item_id: saved.id },
                kind: AssetKind::Cover,
                visibility: Visibility::Shared,
                alt_text: "Manual cover".into(),
            },
            &valid_art,
            &command(),
        )
        .unwrap();
    service
        .update_metadata_authority(
            &actor,
            saved.id,
            UpdateMetadataAuthorityCommand {
                expected_revision: Revision(0),
                change: MetadataAuthorityChange::SetArtworkChoice {
                    asset_id: manual.id,
                },
            },
            &command(),
        )
        .unwrap();

    let refreshed = service
        .refresh_provider_metadata(
            &actor,
            saved.id,
            RefreshProviderMetadataCommand {
                expected_revision: Revision(1),
                snapshot_id: SnapshotId(mb_job.0),
                source_observation_id: None,
                artwork: vec![ProviderArtworkLink {
                    provider_record_id: release_id.into(),
                    asset_id: provider_art.asset.id,
                }],
            },
            &command(),
        )
        .unwrap();
    assert_eq!(
        refreshed.display.as_ref().unwrap().title.value,
        "風の歌",
        "provider refresh cannot replace the inventory-derived manual title"
    );
    assert_eq!(
        refreshed.display.as_ref().unwrap().artwork,
        Some(manual.id),
        "automatic provider artwork cannot replace an explicit choice"
    );
    assert_eq!(refreshed.observations.len(), 1);
    assert_ne!(refreshed.observations[0].id.0, mb_job.0);
    assert!(
        refreshed.observations[0]
            .raw_observation
            .get()
            .contains("1e+02")
    );
    assert_eq!(
        refreshed.observations[0].candidates[0].artwork,
        Some(provider_art.asset.id)
    );
    assert!(
        refreshed.observations[0].candidates[0]
            .identity
            .provider_record_version
            .as_deref()
            .unwrap()
            .starts_with("sha256:")
    );

    let bad_job = service
        .enqueue_cover_art_acquisition(
            &actor,
            saved.id,
            collector_modules::music::CoverArtArchiveQuery::Image {
                release_id: release_id.into(),
                image_id: "67890".into(),
            },
            &command(),
        )
        .unwrap();
    assert!(service.run_next_job("metadata-provider").unwrap());
    assert!(
        service
            .publish_provider_artwork(
                &actor,
                saved.id,
                PublishProviderArtworkCommand {
                    snapshot_id: SnapshotId(bad_job.0),
                },
                &command(),
            )
            .is_err()
    );
    assert_eq!(
        service
            .metadata_view(&actor, saved.id)
            .unwrap()
            .display
            .artwork,
        Some(manual.id),
        "failed artwork decoding leaves the previous active artwork usable"
    );

    let missing = service
        .enqueue_musicbrainz_metadata_acquisition(
            &actor,
            saved.id,
            collector_modules::music::MusicBrainzQuery::Search {
                query: "missing".into(),
                refresh: true,
            },
            &command(),
        )
        .unwrap();
    assert!(service.run_next_job("metadata-provider").unwrap());
    let no_match = service
        .refresh_provider_metadata(
            &actor,
            saved.id,
            RefreshProviderMetadataCommand {
                expected_revision: Revision(2),
                snapshot_id: SnapshotId(missing.0),
                source_observation_id: None,
                artwork: vec![],
            },
            &command(),
        )
        .unwrap();
    assert_eq!(
        no_match.observations[0].outcome,
        MetadataRefreshOutcome::NoMatch
    );
    assert_eq!(no_match.display.unwrap().artwork, Some(manual.id));
    assert!(
        no_match.observations[0]
            .raw_observation
            .get()
            .contains(&missing.to_string())
    );

    AdministrationPort::reindex(&*store).unwrap();
    let rebuilt = service.metadata_view(&actor, saved.id).unwrap();
    assert_eq!(rebuilt.display.artwork, Some(manual.id));
    assert_eq!(rebuilt.candidates.len(), 1);
    assert!(
        files_named(&root.path().join("archive"), "record.v4.json")
            .iter()
            .any(|path| std::fs::read_to_string(path).is_ok_and(|body| body.contains("1e+02")))
    );
}

#[test]
fn metadata_refresh_receipt_recovers_after_revision_commit_before_projection() {
    let (root, store, actor, service, collection) = setup();
    let saved = save_inventory(&service, &actor, item(collection.id), Revision(0)).unwrap();
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
    let service = CollectionService::new(faulted.clone(), Arc::new(Registry::builtins()));
    let context = command();
    let request = RefreshMetadataCommand {
        expected_revision: Revision(0),
        observation: MetadataRefreshObservation {
            id: ObservationId::new(),
            outcome: MetadataRefreshOutcome::NoMatch,
            candidates: vec![],
            raw_observation: RawJson::from_json(r#"{"outage":false,"extra":null}"#).unwrap(),
            observed_at: chrono::Utc::now(),
        },
    };
    assert!(matches!(
        service.refresh_metadata(&actor, saved.id, request.clone(), &context),
        Err(Error::Unavailable(_))
    ));
    drop(service);
    drop(faulted);

    let reopened =
        Arc::new(Store::open(root.path().join("archive"), root.path().join("state")).unwrap());
    let service = CollectionService::new(reopened, Arc::new(Registry::builtins()));
    let recovered = service
        .refresh_metadata(&actor, saved.id, request, &context)
        .unwrap();
    assert_eq!(recovered.revision, Revision(1));
    assert_eq!(recovered.observations.len(), 1);
    assert_eq!(
        service.metadata_view(&actor, saved.id).unwrap().revision,
        Revision(1)
    );
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
fn revoked_curator_grant_before_import_commit_fails_without_publication() {
    let (_root, store, owner, service, collection) = setup();
    let curator_id = store
        .create_user("processing-curator", "a curator password", false)
        .unwrap();
    Catalog::grant(&*store, collection.id, curator_id, Role::Curator).unwrap();
    let curator = Principal {
        user_id: curator_id,
        administrator: false,
        device_id: None,
    };
    let reviewed = reviewed_unattached_import(&service, &curator, collection.id, "revoked-before");
    let job = service
        .enqueue_import_publication(&curator, reviewed.id, &command())
        .unwrap();
    AccessPort::set_grant(&*store, collection.id, curator_id, Revision(1), None).unwrap();
    assert!(service.run_next_job("revoked-worker").unwrap());
    assert_eq!(service.job(&curator, job).unwrap().state, JobState::Failed);
    assert!(matches!(
        Catalog::capture(&*store, reviewed.capture_id),
        Err(Error::NotFound)
    ));
    assert!(service.collections(&owner).is_ok());
}

#[test]
fn revoked_grant_after_import_commit_does_not_block_reconciliation() {
    let (root, store, _owner, service, collection) = setup();
    let curator_id = store
        .create_user("post-commit-curator", "a curator password", false)
        .unwrap();
    Catalog::grant(&*store, collection.id, curator_id, Role::Curator).unwrap();
    let curator = Principal {
        user_id: curator_id,
        administrator: false,
        device_id: None,
    };
    let reviewed = reviewed_unattached_import(&service, &curator, collection.id, "revoked-after");
    let job = service
        .enqueue_import_publication(&curator, reviewed.id, &command())
        .unwrap();
    drop(service);
    drop(store);

    let crashing = Arc::new(
        Store::open_with_publication_fault(
            root.path().join("archive"),
            root.path().join("state"),
            Arc::new(PanicAtImportCommit),
        )
        .unwrap(),
    );
    let crashing_service = CollectionService::new(crashing.clone(), Arc::new(Registry::builtins()));
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            crashing_service.run_next_job("old-worker")
        }))
        .is_err()
    );
    drop(crashing_service);
    drop(crashing);

    let reopened =
        Arc::new(Store::open(root.path().join("archive"), root.path().join("state")).unwrap());
    AccessPort::set_grant(&*reopened, collection.id, curator_id, Revision(1), None).unwrap();
    let recovered = CollectionService::new(reopened.clone(), Arc::new(Registry::builtins()));
    assert!(recovered.run_next_job("new-worker").unwrap());
    assert_eq!(
        recovered.job(&curator, job).unwrap().state,
        JobState::Completed
    );
    assert!(Catalog::capture(&*reopened, reviewed.capture_id).is_ok());
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
        .publish_import_for_job(reviewed.id, job, &claim.claim_token, &mut || Ok(()))
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

#[test]
fn final_chunk_replay() {
    let (_root, _store, actor, service, collection) = setup();
    let import = staged_import(&service, &actor, collection.id, "final-retry");
    assert!(
        service
            .stage_provisional_import_chunk(&actor, import.id, 0, 0, b"data")
            .is_ok(),
        "exact final chunk replay should succeed"
    );
    assert!(matches!(
        service.stage_provisional_import_chunk(&actor, import.id, 0, 0, b"evil"),
        Err(Error::Conflict)
    ));
    let reviewed = service
        .review_provisional_import(
            &actor,
            import.id,
            ReviewImportCommand {
                expected_revision: import.revision,
                identity: ImportIdentityDecision::Unidentified,
                association: CaptureAssociation::default(),
                expected_item_revision: None,
            },
            &command(),
        )
        .unwrap();
    let job = service
        .enqueue_import_publication(&actor, reviewed.id, &command())
        .unwrap();
    service.run_next_job("worker").unwrap();
    assert_eq!(service.job(&actor, job).unwrap().state, JobState::Completed);
    let published = service.provisional_import(&actor, import.id).unwrap();
    assert_eq!(
        service
            .stage_provisional_import_chunk(&actor, import.id, 0, 0, b"data")
            .unwrap()
            .revision,
        published.revision
    );
}

#[test]
fn cancel_requested_restart() {
    let (root, store, actor, service, collection) = setup();
    let import = reviewed_unattached_import(&service, &actor, collection.id, "cancel-restart");
    let job = service
        .enqueue_import_publication(&actor, import.id, &command())
        .unwrap();
    store.claim_job("old-process", 30).unwrap().unwrap();
    store.request_job_cancellation(job, actor.user_id).unwrap();
    drop(service);
    drop(store);
    let reopened =
        Arc::new(Store::open(root.path().join("archive"), root.path().join("state")).unwrap());
    let service = CollectionService::new(reopened.clone(), Arc::new(Registry::builtins()));
    service.run_next_job("new-process").unwrap();
    assert_eq!(service.job(&actor, job).unwrap().state, JobState::Cancelled);
    assert_eq!(
        service.provisional_import(&actor, import.id).unwrap().phase,
        ImportPhase::Cancelled
    );
    assert_eq!(
        std::fs::read(
            root.path()
                .join("state/imports")
                .join(import.id.to_string())
                .join("0.part")
        )
        .unwrap(),
        b"data"
    );
    assert!(matches!(
        service.stage_provisional_import_chunk(&actor, import.id, 0, 0, b"data"),
        Err(Error::Conflict)
    ));
}

struct PanicAtImportCommit;
impl collector_store::PublicationFaultInjector for PanicAtImportCommit {
    fn check(&self, point: collector_store::PublicationFaultPoint) -> Result<()> {
        if point == collector_store::PublicationFaultPoint::RevisionCommitted {
            panic!("simulate process interruption after durable commit");
        }
        Ok(())
    }
}

#[test]
fn committed_import_after_item_edit() {
    let (root, store, actor, service, collection) = setup();
    let saved = save_inventory(&service, &actor, item(collection.id), Revision(0)).unwrap();
    let import = staged_import(&service, &actor, collection.id, "commit-restart");
    let reviewed = service
        .review_provisional_import(
            &actor,
            import.id,
            ReviewImportCommand {
                expected_revision: import.revision,
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
    let job = service
        .enqueue_import_publication(&actor, reviewed.id, &command())
        .unwrap();
    drop(service);
    drop(store);
    let crashing = Store::open_with_publication_fault(
        root.path().join("archive"),
        root.path().join("state"),
        Arc::new(PanicAtImportCommit),
    )
    .unwrap();
    let claim = crashing.claim_job("old-process", 30).unwrap().unwrap();
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(
            || crashing.publish_import_for_job(import.id, job, &claim.claim_token, &mut || Ok(()))
        ))
        .is_err()
    );
    drop(crashing);
    let reopened =
        Arc::new(Store::open(root.path().join("archive"), root.path().join("state")).unwrap());
    assert!(Catalog::capture(&*reopened, import.capture_id).is_ok());
    let service = CollectionService::new(reopened.clone(), Arc::new(Registry::builtins()));
    let mut edited = Catalog::item(&*reopened, saved.id).unwrap();
    let revision = edited.revision;
    edited.title = "Updated title after restart".into();
    save_inventory(&service, &actor, edited, revision).unwrap();
    service.run_next_job("new-process").unwrap();
    assert_eq!(
        service.provisional_import(&actor, import.id).unwrap().phase,
        ImportPhase::Published
    );
    assert_eq!(service.job(&actor, job).unwrap().state, JobState::Completed);
    assert_eq!(Catalog::evidence(&*reopened, saved.id).unwrap().len(), 1);
    assert!(!service.run_next_job("new-process").unwrap());
    assert_eq!(
        Catalog::item(&*reopened, saved.id).unwrap().title,
        "Updated title after restart"
    );
}

#[test]
fn stale_analysis_cannot_be_rebound_to_different_published_bytes() {
    let (root, store, actor, _base_service, collection) = setup();
    let mut registry = Registry::default();
    registry.register(Box::new(ImportAnalyzingModule)).unwrap();
    let service = CollectionService::new(store.clone(), Arc::new(registry));
    let import = staged_import(&service, &actor, collection.id, "portable-analysis");
    let _analysis_job = service
        .enqueue_import_analysis(
            &actor,
            import.id,
            ModuleId("org.junk.music".into()),
            &command(),
        )
        .unwrap();
    service.run_next_job("analysis-worker").unwrap();
    let analyzed = service.provisional_import(&actor, import.id).unwrap();
    let reviewed = service
        .review_provisional_import(
            &actor,
            import.id,
            ReviewImportCommand {
                expected_revision: analyzed.revision,
                identity: ImportIdentityDecision::Unidentified,
                association: CaptureAssociation::default(),
                expected_item_revision: None,
            },
            &command(),
        )
        .unwrap();
    let mut observation = store.provisional_observations(import.id).unwrap().remove(0);
    let mut files = observation.source_fingerprint.files.clone();
    files[0].sha256 = Digest::sha256(b"else");
    observation.source_fingerprint = SourceFingerprint::calculate(
        PackageScope::Import(import.id),
        files,
        None,
        Default::default(),
    )
    .unwrap();
    let db = rusqlite::Connection::open(root.path().join("state/private.sqlite")).unwrap();
    db.execute(
        "UPDATE provisional_processing_observations SET record=? WHERE observation_id=?",
        rusqlite::params![
            serde_json::to_string(&observation).unwrap(),
            observation.id.to_string()
        ],
    )
    .unwrap();
    let job = service
        .enqueue_import_publication(&actor, reviewed.id, &command())
        .unwrap();
    service.run_next_job("stale-publication").unwrap();
    assert_eq!(service.job(&actor, job).unwrap().state, JobState::Failed);
    assert!(matches!(
        Catalog::capture(&*store, import.capture_id),
        Err(Error::NotFound)
    ));
    assert_eq!(
        std::fs::read(
            root.path()
                .join("state/imports")
                .join(import.id.to_string())
                .join("0.part")
        )
        .unwrap(),
        b"data"
    );
}

#[test]
fn snapshot_envelope_mismatch_does_not_partially_replace_catalog() {
    let (root, store, _actor, _service, collection) = setup();
    let snapshot = SnapshotRecord {
        id: SnapshotId::new(),
        collection_id: collection.id,
        provider: ModuleId("org.example.catalog".into()),
        media_type: "application/octet-stream".into(),
        bytes: ByteLength(4),
        digest: Digest::sha256(b"data"),
        parsed_observation: RawJson::from_value(serde_json::json!({})),
        created_at: chrono::Utc::now(),
    };
    store
        .publish_snapshot(
            JobId::new(),
            "fixture",
            snapshot.clone(),
            &mut std::io::Cursor::new(b"data"),
            &mut || Ok(false),
        )
        .unwrap();
    let record_path = files_named(&root.path().join("archive"), "record.v4.json").remove(0);
    let mut changed = snapshot.clone();
    changed.collection_id = CollectionId::new();
    let bytes = serde_json::to_vec_pretty(&changed).unwrap();
    std::fs::write(&record_path, &bytes).unwrap();
    let manifest_path = record_path.with_file_name("manifest.v4.toml");
    let mut manifest: toml::Value =
        toml::from_str(&std::fs::read_to_string(&manifest_path).unwrap()).unwrap();
    manifest["record_sha256"] = toml::Value::String(Digest::sha256(&bytes).hex);
    std::fs::write(manifest_path, toml::to_string(&manifest).unwrap()).unwrap();
    // Change a valid base record too: failed processing rebuild must roll it back.
    let collection_path = files_named(&root.path().join("archive"), "collection.toml").remove(0);
    let mut changed_collection = collection.clone();
    changed_collection.name = "must not reach catalog".into();
    std::fs::write(
        collection_path,
        toml::to_string(&changed_collection).unwrap(),
    )
    .unwrap();
    assert!(AdministrationPort::reindex(&*store).is_err());
    let db = rusqlite::Connection::open(root.path().join("state/catalog.sqlite")).unwrap();
    let old: String = db
        .query_row(
            "SELECT record FROM collections WHERE id=?",
            [collection.id.to_string()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        serde_json::from_str::<Collection>(&old).unwrap().name,
        collection.name
    );
    assert_eq!(
        store
            .snapshot(SnapshotQuery {
                collection_id: collection.id,
                snapshot_id: snapshot.id
            })
            .unwrap()
            .digest,
        snapshot.digest
    );
}

#[test]
fn historical_domain_authority_rebuilds_without_rewriting_original_records() {
    let (root, store, actor, service, collection) = setup();
    let saved = save_inventory(&service, &actor, item(collection.id), Revision(0)).unwrap();
    store
        .publish_metadata_authority(
            TransactionId::new(),
            MetadataAuthorityRecord {
                current_candidates: None,
                inventory: None,
                collection_id: collection.id,
                item_id: saved.id,
                revision: Revision(1),
                authority: MetadataAuthority::default(),
                observations: vec![],
                display: None,
                updated_at: chrono::Utc::now(),
            },
        )
        .unwrap();
    let path = files_named(&root.path().join("archive"), "record.v4.json").remove(0);
    let mut record: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    record["authority"]["field_overrides"] = serde_json::json!([
        {"field":"artist","value":"Original artist","language":"ja"},
        {"field":"platform","value":"Original platform","language":"en"}
    ]);
    let original = serde_json::to_vec_pretty(&record).unwrap();
    std::fs::write(&path, &original).unwrap();
    let manifest_path = path.with_file_name("manifest.v4.toml");
    let mut manifest: toml::Value =
        toml::from_str(&std::fs::read_to_string(&manifest_path).unwrap()).unwrap();
    manifest["record_sha256"] = toml::Value::String(Digest::sha256(&original).hex);
    std::fs::write(&manifest_path, toml::to_string(&manifest).unwrap()).unwrap();
    let original_manifest = std::fs::read(&manifest_path).unwrap();
    drop(service);
    drop(store);
    let rebuilt =
        Store::open(root.path().join("archive"), root.path().join("fresh-state")).unwrap();
    AdministrationPort::reindex(&rebuilt).unwrap();
    let db = rusqlite::Connection::open(root.path().join("fresh-state/catalog.sqlite")).unwrap();
    let body: String = db
        .query_row(
            "SELECT record FROM metadata_authorities WHERE id=?",
            [saved.id.to_string()],
            |row| row.get(0),
        )
        .unwrap();
    let decoded: MetadataAuthorityRecord = serde_json::from_str(&body).unwrap();
    assert!(decoded.authority.field_overrides.is_empty());
    assert!(decoded.authority.domain_overrides.is_none());
    assert_eq!(decoded.authority.preserved_field_overrides.len(), 2);
    assert_eq!(
        decoded.authority.preserved_field_overrides[0]
            .value()
            .unwrap()["language"],
        "ja"
    );
    assert_eq!(std::fs::read(path).unwrap(), original);
    assert_eq!(std::fs::read(manifest_path).unwrap(), original_manifest);
}

#[test]
fn ct2_unknown_active_job_is_reported_and_blocks_assignment() {
    let (root, user, collection, _) = legacy_collection_fixture();
    let store =
        Arc::new(Store::open(root.path().join("archive"), root.path().join("state")).unwrap());
    let db = rusqlite::Connection::open(root.path().join("state/private.sqlite")).unwrap();
    let job = JobId::new();
    db.execute("INSERT INTO jobs(id,user_id,collection_id,kind,state,phase,completed_units,total_units,attempt,created_at,updated_at,request) VALUES(?,?,?,'future_operation','queued','queued',0,1,0,0,0,'{}')",
        rusqlite::params![job.to_string(),user.to_string(),collection.to_string()]).unwrap();
    let service = CollectionTypeAdministrationService::new(store, Arc::new(Registry::builtins()));
    assert!(!service.inspect(collection).unwrap().eligible);
    let error = service
        .assign(
            collection,
            Revision(1),
            ModuleId("org.junk.music".into()),
            chrono::Utc::now(),
        )
        .unwrap_err();
    assert!(error.to_string().contains(&job.to_string()));
    db.execute(
        "UPDATE jobs SET state='cancelled' WHERE id=?",
        [job.to_string()],
    )
    .unwrap();
    assert!(service.inspect(collection).unwrap().eligible);
}

#[test]
fn ct3_content_policy_keeps_legacy_reads_and_independent_activity_and_access() {
    let (root, user, collection_id, item_id) = legacy_collection_fixture();
    let store =
        Arc::new(Store::open(root.path().join("archive"), root.path().join("state")).unwrap());
    let actor = Principal {
        user_id: user,
        administrator: true,
        device_id: None,
    };
    let service = CollectionService::new(store.clone(), Arc::new(Registry::builtins()));

    let presented = service.item_presented(&actor, item_id).unwrap();
    assert_eq!(presented.id, item_id);
    service
        .save_activity(
            &actor,
            item_id,
            serde_json::json!({"favorite":true}),
            &command(),
        )
        .unwrap();
    assert_eq!(service.activity(&actor, item_id).unwrap()["favorite"], true);
    let viewer = store
        .create_user("ct3-viewer", "a long viewer password", false)
        .unwrap();
    service
        .grant(
            &test_authentication(&actor),
            collection_id,
            viewer,
            Role::Viewer,
            &command(),
        )
        .unwrap();

    let current = Catalog::item(&*store, item_id).unwrap();
    let mut changed = SaveItemCommand::from(current.clone());
    changed.title = "blocked legacy write".into();
    assert!(matches!(
        service.save_item(&actor, item_id, changed, current.revision, &command()),
        Err(Error::Unavailable(message)) if message.contains("read-only")
    ));
    assert!(matches!(
        service.create_specimen(
            &actor,
            item_id,
            current.revision,
            SpecimenDetails {
                label: "blocked".into(),
                condition: Condition::Unknown,
                ownership: ActiveOwnership::Unknown,
                possession: Possession::Unknown,
                acquisition: None,
                private_notes: None,
            },
            &command(),
        ),
        Err(Error::Unavailable(_))
    ));
    assert!(matches!(
        service.save_work(
            &actor,
            WorkId::new(),
            collector_app::SaveWorkCommand {
                collection_id,
                provisional: true,
                title: "blocked work".into(),
                translations: vec![],
                identifiers: vec![],
            },
            Revision(0),
            &command(),
        ),
        Err(Error::Unavailable(_))
    ));
    assert!(matches!(
        service.save_edition(
            &actor,
            EditionId::new(),
            collector_app::SaveEditionCommand {
                collection_id,
                work_id: None,
                provisional: true,
                title: "blocked edition".into(),
                expected_components: vec![],
                identifiers: vec![],
            },
            Revision(0),
            &command(),
        ),
        Err(Error::Unavailable(_))
    ));
    assert!(matches!(
        service.save_capture(
            &actor,
            CaptureId::new(),
            collector_app::SaveCaptureCommand {
                collection_id,
                association: CaptureAssociation::default(),
            },
            Revision(0),
            &command(),
        ),
        Err(Error::Unavailable(_))
    ));
    assert!(matches!(
        service.publish_cover(
            &actor,
            item_id,
            current.revision,
            b"not parsed because policy denies first",
            "blocked cover".into(),
            &command(),
        ),
        Err(Error::Unavailable(_))
    ));
    assert!(matches!(
        service.create_upload(&actor, item_id, current.revision, vec![], &command()),
        Err(Error::Unavailable(_))
    ));
    assert!(matches!(
        service.enqueue_integrity_verification(&actor, item_id, &command()),
        Err(Error::Unavailable(_))
    ));
    assert!(matches!(
        service.create_provisional_import(
            &actor,
            CreateImportCommand {
                collection_id,
                source: ImportSourceSummary {
                    kind: ImportSourceKind::Browser,
                    label: None,
                    package_name: "unidentified.bin".into(),
                },
                files: vec![],
                association: CaptureAssociation::default(),
                expected_item_revision: None,
            },
            &command(),
        ),
        Err(Error::Unavailable(_))
    ));
}

#[test]
fn ct3_item_owner_and_schema_are_application_policy() {
    let (_root, _store, actor, service, collection) = setup();
    let mut wrong_owner = item(collection.id);
    wrong_owner.domain = ModuleData {
        module: ModuleId("org.junk.games".into()),
        schema_version: 1,
        payload: RawJson::from_value(serde_json::json!({})),
    };
    assert!(matches!(
        service.save_item(
            &actor,
            wrong_owner.id,
            wrong_owner.clone().into(),
            Revision(0),
            &command(),
        ),
        Err(Error::Invalid(message)) if message.contains("conflicts with collection owner")
    ));

    let mut unsupported = item(collection.id);
    unsupported.domain.schema_version = 99;
    assert!(matches!(
        service.save_item(
            &actor,
            unsupported.id,
            unsupported.into(),
            Revision(0),
            &command(),
        ),
        Err(Error::Unavailable(message)) if message.contains("unsupported")
    ));

    let wishlist = save_inventory(&service, &actor, item(collection.id), Revision(0)).unwrap();
    assert!(wishlist.wishlist);
    assert!(wishlist.captures.is_empty());

    let import = service
        .create_provisional_import(
            &actor,
            CreateImportCommand {
                collection_id: collection.id,
                source: ImportSourceSummary {
                    kind: ImportSourceKind::Browser,
                    label: None,
                    package_name: "manual-and-soundtrack-package".into(),
                },
                files: vec![UploadFile {
                    name: "manual-and-soundtrack.bin".into(),
                    bytes: ByteLength(1),
                    sha256: Some(Digest::sha256(b"x").hex),
                }],
                association: CaptureAssociation::default(),
                expected_item_revision: None,
            },
            &command(),
        )
        .unwrap();
    service
        .stage_provisional_import_chunk(&actor, import.id, 0, 0, b"x")
        .unwrap();
    assert!(matches!(
        service.enqueue_import_analysis(
            &actor,
            import.id,
            ModuleId("org.junk.games".into()),
            &command(),
        ),
        Err(Error::Invalid(message)) if message.contains("collection module")
    ));
}

#[test]
fn ct3_queued_job_rechecks_owner_availability() {
    let (_root, store, actor, service, collection) = setup();
    let provider = ModuleId("org.example.catalog".into());
    let frozen = FrozenAcquisitionJob {
        request: ProviderRequest {
            collection_id: collection.id,
            provider,
            operation: OperationId::new("org.example.catalog.acquire").unwrap(),
            source_fingerprint: None,
            query: RawJson::from_value(serde_json::json!({"catalog":"fixture"})),
            maximum_response_bytes: 1024,
        },
        collection_binding: collection.binding.clone(),
        item_id: None,
        snapshot_id: SnapshotId::new(),
        created_at: chrono::Utc::now(),
    };
    let job = service
        .enqueue_catalog_acquisition(&actor, frozen, &command())
        .unwrap();
    drop(service);

    let unavailable = CollectionService::new(store, Arc::new(Registry::default()));
    assert!(unavailable.run_next_job("ct3-missing-owner").unwrap());
    let failed = unavailable.job(&actor, job).unwrap();
    assert_eq!(failed.state, JobState::Failed);
    assert!(failed.error.unwrap().contains("not installed"));
}

#[test]
fn ct3_edit_cannot_downgrade_an_archived_unknown_schema() {
    let (_root, store, actor, service, collection) = setup();
    let mut future = item(collection.id);
    future.domain.schema_version = 99;
    future.domain.payload =
        RawJson::from_json(r#"{"future":{"precise":1.000,"unknown":[true,null]}}"#).unwrap();
    let saved = Catalog::save(&*store, future, Revision(0)).unwrap();
    let mut replacement = saved.clone();
    replacement.domain = item(collection.id).domain;
    replacement.title = "attempted downgrade".into();
    assert!(matches!(
        service.save_item(
            &actor,
            saved.id,
            replacement.clone().into(),
            saved.revision,
            &command()
        ),
        Err(Error::Unavailable(_))
    ));
    assert!(matches!(
        service.adapt_wave1_item_save(&actor, saved.id, replacement, saved.revision),
        Err(Error::Unavailable(_))
    ));
    let retained = store.item(saved.id).unwrap();
    assert_eq!(retained.revision, saved.revision);
    assert_eq!(retained.domain.payload.get(), saved.domain.payload.get());
}

#[test]
fn ct3_capture_publication_rechecks_policy_after_staging() {
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
        .enqueue_upload_commit(&actor, upload.id, &command())
        .unwrap();
    let claim = store.claim_job("policy-boundary", 30).unwrap().unwrap();
    let mut checked = false;
    let result = store.commit_upload_for_job(claim.id, &claim.claim_token, &mut || {
        checked = true;
        Err(Error::Forbidden)
    });
    assert!(checked);
    assert!(matches!(result, Err(Error::Forbidden)));
    assert!(store.item(saved.id).unwrap().captures.is_empty());
    // Retained staged bytes can be published on an authorized retry.
    let committed = store
        .commit_upload_for_job(claim.id, &claim.claim_token, &mut || Ok(()))
        .unwrap();
    assert_eq!(committed.captures.len(), 1);
}

#[test]
fn ct3_provisional_publication_policy_failure_keeps_staged_bytes() {
    let (_root, store, actor, service, collection) = setup();
    let reviewed = reviewed_unattached_import(&service, &actor, collection.id, "policy-check");
    let job = service
        .enqueue_import_publication(&actor, reviewed.id, &command())
        .unwrap();
    let claim = store.claim_job("policy-worker", 30).unwrap().unwrap();
    let mut checked = false;
    assert!(matches!(
        store.publish_import_for_job(reviewed.id, job, &claim.claim_token, &mut || {
            checked = true;
            Err(Error::Forbidden)
        }),
        Err(Error::Forbidden)
    ));
    assert!(checked);
    assert!(matches!(
        Catalog::capture(&*store, reviewed.capture_id),
        Err(Error::NotFound)
    ));
    let published = store
        .publish_import_for_job(reviewed.id, job, &claim.claim_token, &mut || Ok(()))
        .unwrap();
    assert_eq!(published.view.phase, ImportPhase::Published);
}

#[test]
fn ct3_legacy_upload_can_be_cancelled_without_its_owner_module() {
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
    let unavailable = CollectionService::new(store.clone(), Arc::new(Registry::default()));
    unavailable
        .cancel_upload(&actor, upload.id, &command())
        .unwrap();
    assert!(unavailable.upload(&actor, upload.id).is_ok());
    assert!(matches!(
        unavailable.upload_chunk(&actor, upload.id, 0, 0, b"data"),
        Err(Error::Unavailable(_))
    ));
}

#[test]
fn ct4_domain_patch_receipts_bind_original_request_not_resolved_payload() {
    let (_root, store, actor, service, collection) = setup();
    let id = ItemId::new();
    let edit = |fields: serde_json::Value| -> collector_app::SaveItemEditCommand {
        serde_json::from_value(serde_json::json!({
            "collection_id":collection.id,"title":"Patch replay","subtitle":"",
            "identity_link":{"kind":"none"},"domain_edit":{"module":"org.junk.music","fields":fields},
            "identity":"unidentified","translations":[],"identifiers":[],"wishlist":false
        })).unwrap()
    };
    let first = edit(serde_json::json!({"artist":"A"}));
    let key = command();
    let created = service
        .save_item_edit(&actor, id, first.clone(), Revision(0), &key)
        .unwrap();
    let later = service
        .save_item_edit(
            &actor,
            id,
            edit(serde_json::json!({"release_year":2001})),
            created.revision,
            &command(),
        )
        .unwrap();
    let replayed = service
        .save_item_edit(&actor, id, first, Revision(0), &key)
        .unwrap();
    assert_eq!(replayed.revision, created.revision);
    assert_eq!(store.item(id).unwrap().revision, later.revision);
    assert_eq!(
        store
            .item(id)
            .unwrap()
            .domain
            .payload
            .parse::<serde_json::Value>()
            .unwrap()["release_year"],
        2001
    );
    assert!(matches!(
        service.save_item_edit(
            &actor,
            id,
            edit(serde_json::json!({"artist":"A","release_year":2001})),
            Revision(0),
            &key
        ),
        Err(Error::IdempotencyConflict)
    ));
}

fn derivative_import(
    service: &CollectionService,
    actor: &Principal,
    collection: CollectionId,
) -> ImportView {
    derivative_import_variant(service, actor, collection, 0)
}

fn derivative_import_variant(
    service: &CollectionService,
    actor: &Principal,
    collection: CollectionId,
    variant: u8,
) -> ImportView {
    // Track 1 owns stored HTOA plus track 2's stored INDEX 00: 3 sectors.
    // Track 2 owns the final two sectors. No AccurateRip evidence is requested.
    let cue = b"FILE \"disc.bin\" BINARY\n TRACK 01 AUDIO\n INDEX 00 00:00:00\n INDEX 01 00:00:01\n TRACK 02 AUDIO\n INDEX 00 00:00:02\n INDEX 01 00:00:03\n";
    let pcm: Vec<u8> = (0..5 * 588)
        .flat_map(|i| [(i as u8).wrapping_add(variant), 0x80, 0xff, 0x7f])
        .collect();
    let created = service
        .create_provisional_import(
            actor,
            CreateImportCommand {
                collection_id: collection,
                source: ImportSourceSummary {
                    kind: ImportSourceKind::Browser,
                    label: None,
                    package_name: format!("derivative-fixture-{variant}"),
                },
                files: vec![
                    UploadFile {
                        name: "disc.cue".into(),
                        bytes: ByteLength(cue.len() as u64),
                        sha256: None,
                    },
                    UploadFile {
                        name: "disc.bin".into(),
                        bytes: ByteLength(pcm.len() as u64),
                        sha256: None,
                    },
                ],
                association: CaptureAssociation::default(),
                expected_item_revision: None,
            },
            &command(),
        )
        .unwrap();
    service
        .stage_provisional_import_chunk(actor, created.id, 0, 0, cue)
        .unwrap();
    service
        .stage_provisional_import_chunk(actor, created.id, 1, 0, &pcm)
        .unwrap();
    service
        .enqueue_import_analysis(
            actor,
            created.id,
            ModuleId("org.junk.music".into()),
            &command(),
        )
        .unwrap();
    service.run_next_job("derivative-import-analysis").unwrap();
    let analyzed = service.provisional_import(actor, created.id).unwrap();
    let reviewed = service
        .review_provisional_import(
            actor,
            created.id,
            ReviewImportCommand {
                expected_revision: analyzed.revision,
                identity: ImportIdentityDecision::Unidentified,
                association: CaptureAssociation::default(),
                expected_item_revision: None,
            },
            &command(),
        )
        .unwrap();
    service
        .enqueue_import_publication(actor, reviewed.id, &command())
        .unwrap();
    service
        .run_next_job("derivative-import-publication")
        .unwrap();
    let published = service.provisional_import(actor, created.id).unwrap();
    assert_eq!(published.phase, ImportPhase::Published);
    published
}

// Explicit test adapter: its bytes are not a real codec. It tests publication,
// never codec qualification; the enabled real-FLAC test below uses the process port.
struct DerivativeFixtureEncoder {
    version: std::sync::Mutex<String>,
    calls: std::sync::atomic::AtomicUsize,
    after_encode: std::sync::Mutex<Option<Box<dyn Fn() + Send>>>,
    broken: AtomicBool,
}
impl DerivativeFixtureEncoder {
    fn new() -> Self {
        Self {
            calls: 0.into(),
            version: std::sync::Mutex::new("fixture encoder 1".into()),
            after_encode: Default::default(),
            broken: AtomicBool::new(false),
        }
    }
}
impl collector_app::AudioConversionPort for DerivativeFixtureEncoder {
    fn configured_availability(
        &self,
        profile: &ConversionProfile,
    ) -> collector_app::EncoderAvailability {
        collector_app::EncoderAvailability::Available {
            encoder: match profile {
                ConversionProfile::Flac => "flac",
                ConversionProfile::Opus { .. } => "opusenc",
            }
            .into(),
            version: self.version.lock().unwrap().clone(),
        }
    }

    fn availability(&self, recipe: &ConversionRecipe) -> collector_app::EncoderAvailability {
        if recipe.encoder_version == *self.version.lock().unwrap() {
            collector_app::EncoderAvailability::Available {
                encoder: recipe.encoder.clone(),
                version: recipe.encoder_version.clone(),
            }
        } else {
            collector_app::EncoderAvailability::Unavailable {
                reason: "fixture encoder version changed".into(),
            }
        }
    }
    fn convert(
        &self,
        recipe: &ConversionRecipe,
        _: &mut (dyn junk_libs_disc::CdPcmReader + Send),
        scratch: &mut dyn collector_app::ConversionScratch,
        _: &mut dyn ProcessingExecutionContext,
    ) -> Result<collector_app::ConversionValidation> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if let collector_app::EncoderAvailability::Unavailable { reason } =
            self.availability(recipe)
        {
            return Err(Error::Unavailable(reason));
        }
        scratch.write_all(b"fixture output").unwrap();
        if let Some(hook) = self.after_encode.lock().unwrap().take() {
            hook();
        }
        Ok(collector_app::ConversionValidation {
            media_type: "audio/flac".into(),
            bytes: ByteLength(14),
            digest: Digest::sha256(b"fixture output"),
            decoded_format: recipe.output_format.clone(),
            decoded_frames: recipe.tracks[0].length.0,
            lossless_pcm_equal: !self.broken.load(Ordering::SeqCst),
            encoder_delay_frames: 0,
            end_padding_frames: 0,
        })
    }
}
fn derivative_command(import: &ImportView, position: u16) -> collector_app::PrepareMusicDerivative {
    collector_app::PrepareMusicDerivative {
        representation_id: import.representation_id,
        selection: music::CdPackageSelection {
            package_kind: music::CdPackageKind::Cue,
            anchor_file_index: 0,
        },
        track_position: position,
        profile: ConversionProfile::Flac,
        encoder_version: "fixture encoder 1".into(),
    }
}

#[test]
fn derivative_jobs_converge_replay_keep_track_ownership_and_survive_item_edits_reindex() {
    let (_root, store, actor, service, collection) = setup();
    let imported = derivative_import(&service, &actor, collection.id);
    let encoder = Arc::new(DerivativeFixtureEncoder::new());
    let service = Arc::new(service.with_audio_converter(encoder.clone()));
    let key = command();
    let first = service
        .prepare_music_derivative(
            &actor,
            imported.capture_id,
            derivative_command(&imported, 1),
            &key,
        )
        .unwrap();
    assert_eq!(
        first,
        service
            .prepare_music_derivative(
                &actor,
                imported.capture_id,
                derivative_command(&imported, 1),
                &key
            )
            .unwrap()
    );
    let duplicate = service
        .prepare_music_derivative(
            &actor,
            imported.capture_id,
            derivative_command(&imported, 1),
            &command(),
        )
        .unwrap();
    let second = service
        .prepare_music_derivative(
            &actor,
            imported.capture_id,
            derivative_command(&imported, 2),
            &command(),
        )
        .unwrap();
    std::thread::scope(|scope| {
        for _ in 0..3 {
            let service = &service;
            scope.spawn(move || service.run_next_job("derivative-worker").unwrap());
        }
    });
    while service.run_next_job("derivative-worker").unwrap() {}
    for job in [first, duplicate, second] {
        assert_eq!(service.job(&actor, job).unwrap().state, JobState::Completed);
    }
    let capture = Catalog::capture(store.as_ref(), imported.capture_id).unwrap();
    assert_eq!(capture.representations.len(), 3);
    assert_eq!(capture.revision, Revision(3));
    for representation in &capture.representations[1..] {
        assert_eq!(representation.files.len(), 1);
        let recipe = store.recipe(RecipeId(representation.id.0)).unwrap();
        assert_eq!(recipe.tracks.len(), 1);
        assert_eq!(
            recipe.tracks[0].length.0,
            if recipe.tracks[0].position == 1 {
                3 * 588
            } else {
                2 * 588
            }
        );
        assert_eq!(
            recipe.tracks[0].start.0,
            if recipe.tracks[0].position == 1 {
                0
            } else {
                3 * 588
            }
        );
    }
    let draft = item(collection.id);
    let saved = service
        .save_item(
            &actor,
            draft.id,
            SaveItemCommand::from(draft),
            Revision(0),
            &command(),
        )
        .unwrap();
    let mut associated = capture.clone();
    associated.association.item_id = Some(saved.id);
    Catalog::save_capture(store.as_ref(), associated, capture.revision).unwrap();
    let mut edit = Catalog::item(store.as_ref(), saved.id).unwrap();
    edit.title = "Manual edit survives".into();
    let rev = edit.revision;
    service
        .save_item(
            &actor,
            edit.id,
            SaveItemCommand::from(edit),
            rev,
            &command(),
        )
        .unwrap();
    store.reindex().unwrap();
    let edited = Catalog::item(store.as_ref(), saved.id).unwrap();
    assert_eq!(edited.title, "Manual edit survives");
    assert_eq!(edited.captures[0].representations.len(), 3);
    for r in &edited.captures[0].representations[1..] {
        assert_eq!(
            resource_bytes(ContentPort::representation(store.as_ref(), saved.id, r.id, 0).unwrap()),
            b"fixture output"
        );
    }
    let disabled = CollectionService::new(store.clone(), Arc::new(Registry::builtins()));
    assert_eq!(
        disabled
            .prepare_music_derivative(
                &actor,
                imported.capture_id,
                derivative_command(&imported, 1),
                &key
            )
            .unwrap(),
        first
    );
    assert!(matches!(
        disabled.prepare_music_derivative(
            &actor,
            imported.capture_id,
            derivative_command(&imported, 1),
            &command()
        ),
        Err(Error::Unavailable(_))
    ));
    assert!(matches!(
        service.prepare_music_derivative(
            &actor,
            imported.capture_id,
            derivative_command(&imported, 2),
            &key
        ),
        Err(Error::IdempotencyConflict)
    ));
}

#[test]
fn album_batches_fan_out_durably_overlap_without_duplicate_tracks_and_retry_partial_failure() {
    let clock = Arc::new(TestClock::new(chrono::Utc::now().timestamp()));
    let (root, store, actor, service, collection) = setup_with_clock(Some(clock.clone()));
    let saved = save_inventory(&service, &actor, item(collection.id), Revision(0)).unwrap();
    let imported = derivative_import(&service, &actor, collection.id);
    let mut capture = Catalog::capture(store.as_ref(), imported.capture_id).unwrap();
    let revision = capture.revision;
    capture.association.item_id = Some(saved.id);
    Catalog::save_capture(store.as_ref(), capture, revision).unwrap();
    let encoder = Arc::new(DerivativeFixtureEncoder::new());
    let service = service.with_audio_converter(encoder.clone());
    let plan = service
        .music_album_preparation_plan(&actor, saved.id)
        .unwrap();
    assert_eq!(plan.discs.len(), 1);
    assert_eq!(plan.total_tracks, 2);
    assert!(plan.discs[0].sources[0].default_selected);
    let request = PrepareMusicAlbum {
        source_representation_ids: vec![imported.representation_id],
    };
    let first = service
        .prepare_music_album(&actor, saved.id, request.clone(), &command())
        .unwrap();
    clock.advance(7);
    let second = service
        .prepare_music_album(&actor, saved.id, request, &command())
        .unwrap();
    assert_ne!(first.batch_id, second.batch_id);
    assert_eq!(first.members.len(), 2);
    assert_eq!(
        first
            .members
            .iter()
            .map(|member| member.job_id)
            .collect::<Vec<_>>(),
        second
            .members
            .iter()
            .map(|member| member.job_id)
            .collect::<Vec<_>>()
    );
    clock.advance(7);
    service.run_next_job("album-fanout-1").unwrap();
    let second = service
        .music_preparation_batch(&actor, second.batch_id)
        .unwrap();
    assert_eq!(second.queued, 2);
    service
        .cancel_music_preparation_batch(&actor, first.batch_id, &command())
        .unwrap();
    service.run_next_job("album-fanout-2").unwrap();
    encoder.broken.store(true, Ordering::SeqCst);
    service.run_next_job("album-track-failure").unwrap();
    encoder.broken.store(false, Ordering::SeqCst);
    service.run_next_job("album-track-success").unwrap();
    let partial = service
        .music_preparation_batch(&actor, second.batch_id)
        .unwrap();
    assert_eq!(partial.failed, 1);
    assert_eq!(partial.succeeded, 1);
    assert!(partial.can_retry);
    service
        .retry_music_preparation_batch(&actor, second.batch_id, &command())
        .unwrap();
    drop(service);
    drop(store);
    let store =
        Arc::new(Store::open(root.path().join("archive"), root.path().join("state")).unwrap());
    let service = CollectionService::new(store.clone(), Arc::new(Registry::builtins()))
        .with_audio_converter(encoder.clone());
    service.run_next_job("album-restart-retry").unwrap();
    let completed = service
        .music_preparation_batch(&actor, second.batch_id)
        .unwrap();
    assert_eq!(completed.succeeded, 2);
    assert_eq!(completed.failed, 0);
    let attempts = completed
        .members
        .iter()
        .filter_map(|member| member.job_id)
        .map(|id| store.job_record(id, actor.user_id).unwrap())
        .collect::<Vec<_>>();
    let retried = attempts.iter().find(|job| job.attempt == 2).unwrap();
    assert_eq!(
        retried.result.as_ref().unwrap()["prior_failures"][0]["attempt"],
        1
    );
    assert!(
        retried.result.as_ref().unwrap()["prior_failures"][0]["error"]
            .as_str()
            .is_some()
    );

    assert_eq!(encoder.calls.load(Ordering::SeqCst), 3);
    assert_eq!(
        Catalog::capture(store.as_ref(), imported.capture_id)
            .unwrap()
            .representations
            .len(),
        3
    );
}

#[test]
fn album_cancellation_respects_later_direct_requests_and_last_shared_request() {
    let clock = Arc::new(TestClock::new(chrono::Utc::now().timestamp()));
    let (_root, store, actor, service, collection) = setup_with_clock(Some(clock.clone()));
    let saved = save_inventory(&service, &actor, item(collection.id), Revision(0)).unwrap();
    let imported = derivative_import(&service, &actor, collection.id);
    let mut capture = Catalog::capture(store.as_ref(), imported.capture_id).unwrap();
    let revision = capture.revision;
    capture.association.item_id = Some(saved.id);
    Catalog::save_capture(store.as_ref(), capture, revision).unwrap();
    let service = service.with_audio_converter(Arc::new(DerivativeFixtureEncoder::new()));
    let request = PrepareMusicAlbum {
        source_representation_ids: vec![imported.representation_id],
    };
    let first = service
        .prepare_music_album(&actor, saved.id, request.clone(), &command())
        .unwrap();
    clock.advance(2);
    let second = service
        .prepare_music_album(&actor, saved.id, request, &command())
        .unwrap();
    clock.advance(2);
    service.run_next_job("first-batch").unwrap();
    service.run_next_job("second-batch").unwrap();
    clock.advance(8);
    let direct = service
        .prepare_configured_music_track(
            &actor,
            saved.id,
            imported.capture_id,
            PrepareMusicTrack {
                source_representation_id: imported.representation_id,
                profile: ConversionProfile::Flac,
            },
            first.members[0].track_position,
            &command(),
        )
        .unwrap();
    assert_eq!(Some(direct), first.members[0].job_id);
    service
        .cancel_music_preparation_batch(&actor, first.batch_id, &command())
        .unwrap();
    assert_eq!(
        store
            .job_record(first.members[1].job_id.unwrap(), actor.user_id)
            .unwrap()
            .state,
        JobState::Queued
    );
    service
        .cancel_music_preparation_batch(&actor, second.batch_id, &command())
        .unwrap();
    assert_eq!(
        store
            .job_record(first.members[1].job_id.unwrap(), actor.user_id)
            .unwrap()
            .state,
        JobState::Cancelled
    );
    assert_eq!(
        store.job_record(direct, actor.user_id).unwrap().state,
        JobState::Queued
    );
    service.cancel_job(&actor, direct, &command()).unwrap();
    let fresh = service
        .prepare_music_album(
            &actor,
            saved.id,
            PrepareMusicAlbum {
                source_representation_ids: vec![imported.representation_id],
            },
            &command(),
        )
        .unwrap();
    service
        .run_next_job("fresh-batch-after-direct-cancel")
        .unwrap();
    service
        .cancel_music_preparation_batch(&actor, fresh.batch_id, &command())
        .unwrap();
    assert_eq!(
        store.job_record(direct, actor.user_id).unwrap().state,
        JobState::Cancelled,
        "an explicitly cancelled direct request cannot pin future batch work"
    );
}

#[test]
fn album_requests_from_another_curator_keep_job_details_private_and_respect_role_changes() {
    let (_root, store, actor, service, collection) = setup();
    let saved = save_inventory(&service, &actor, item(collection.id), Revision(0)).unwrap();
    let imported = derivative_import(&service, &actor, collection.id);
    let mut capture = Catalog::capture(store.as_ref(), imported.capture_id).unwrap();
    let revision = capture.revision;
    capture.association.item_id = Some(saved.id);
    Catalog::save_capture(store.as_ref(), capture, revision).unwrap();
    let service = service.with_audio_converter(Arc::new(DerivativeFixtureEncoder::new()));
    let request = PrepareMusicAlbum {
        source_representation_ids: vec![imported.representation_id],
    };
    let owner_batch = service
        .prepare_music_album(&actor, saved.id, request.clone(), &command())
        .unwrap();
    service.run_next_job("owner-admission").unwrap();
    for _ in 1..collector_app::MAX_ACTIVE_MUSIC_PREPARATION_BATCHES {
        service
            .prepare_music_album(&actor, saved.id, request.clone(), &command())
            .unwrap();
    }
    assert!(matches!(
        service.prepare_music_album(&actor, saved.id, request.clone(), &command()),
        Err(Error::Unavailable(_))
    ));
    service
        .cancel_music_preparation_batch(&actor, owner_batch.batch_id, &command())
        .unwrap();
    service
        .prepare_music_album(&actor, saved.id, request.clone(), &command())
        .unwrap();
    let oldest_live = service
        .music_preparation_batches(&actor, saved.id)
        .unwrap()
        .into_iter()
        .filter(|batch| !batch.cancelled_by_user)
        .min_by_key(|batch| (batch.created_at, batch.batch_id))
        .unwrap()
        .batch_id;
    let mut rolling = service.music_preparation_batches(&actor, saved.id).unwrap()[0].batch_id;
    for _ in 0..52 {
        service
            .cancel_music_preparation_batch(&actor, rolling, &command())
            .unwrap();
        rolling = service
            .prepare_music_album(&actor, saved.id, request.clone(), &command())
            .unwrap()
            .batch_id;
    }
    let history = service.music_preparation_batches(&actor, saved.id).unwrap();
    assert_eq!(history.len(), 50);
    assert!(
        history.iter().any(|batch| batch.batch_id == oldest_live),
        "cancelled shared history displaced unfinished work"
    );
    let user = store
        .create_user("other-curator", "a long other password", false)
        .unwrap();
    store.grant(collection.id, user, Role::Curator).unwrap();
    let curator = Principal {
        user_id: user,
        administrator: false,
        device_id: None,
    };
    let other_batch = service
        .prepare_music_album(&curator, saved.id, request, &command())
        .unwrap();
    assert_ne!(owner_batch.members[0].job_id, other_batch.members[0].job_id);
    assert!(matches!(
        service.music_preparation_batch(&curator, owner_batch.batch_id),
        Err(Error::NotFound)
    ));
    assert_eq!(
        service
            .music_preparation_batches(&curator, saved.id)
            .unwrap()
            .len(),
        1
    );
    AccessPort::set_grant(
        store.as_ref(),
        collection.id,
        user,
        Revision(1),
        Some(Role::Viewer),
    )
    .unwrap();
    let view = service
        .music_preparation_batch(&curator, other_batch.batch_id)
        .unwrap();
    assert!(!view.can_cancel && !view.can_retry);
    assert!(matches!(
        service.cancel_music_preparation_batch(&curator, other_batch.batch_id, &command()),
        Err(Error::Forbidden)
    ));
}

#[test]
fn album_failed_admission_is_visible_retryable_and_cancel_fences_late_children() {
    let (_root, store, actor, service, collection) = setup();
    let saved = save_inventory(&service, &actor, item(collection.id), Revision(0)).unwrap();
    let imported = derivative_import(&service, &actor, collection.id);
    let mut capture = Catalog::capture(store.as_ref(), imported.capture_id).unwrap();
    let revision = capture.revision;
    capture.association.item_id = Some(saved.id);
    Catalog::save_capture(store.as_ref(), capture, revision).unwrap();
    let service = service.with_audio_converter(Arc::new(DerivativeFixtureEncoder::new()));
    let batch = service
        .prepare_music_album(
            &actor,
            saved.id,
            PrepareMusicAlbum {
                source_representation_ids: vec![imported.representation_id],
            },
            &command(),
        )
        .unwrap();
    let claim = store
        .claim_job("interrupted-admission", 30)
        .unwrap()
        .unwrap();
    assert_eq!(claim.id, batch.batch_id);
    store
        .fail_job(claim.id, &claim.claim_token, "injected admission failure")
        .unwrap();
    let failed = service
        .music_preparation_batch(&actor, batch.batch_id)
        .unwrap();
    assert_eq!(failed.failed, 2);
    assert_eq!(failed.queued, 0);
    assert!(failed.can_retry);
    service
        .retry_music_preparation_batch(&actor, batch.batch_id, &command())
        .unwrap();
    let claim = store.claim_job("retried-admission", 30).unwrap().unwrap();
    let Some(FrozenJobInput::DerivativeBatch(frozen)) = &claim.frozen_input else {
        panic!("batch");
    };
    let FrozenPreparationMember::Prepare { job_id, input, .. } = &frozen.members[0] else {
        panic!("member");
    };
    service
        .cancel_music_preparation_batch(&actor, batch.batch_id, &command())
        .unwrap();
    let late = store.enqueue_preparation_member(claim.id, &claim.claim_token, JobSpec {
        id: *job_id, user_id: actor.user_id, collection_id: Some(collection.id), item_id: None,
        kind: "music_derivative".into(), phase: "queued".into(), total_units: 1,
        request: serde_json::json!({"capture_id": input.capture_id, "representation_id": input.representation_id}),
        frozen_input: Some(FrozenJobInput::Derivative(input.as_ref().clone())),
    });
    assert!(matches!(late, Err(Error::JobCancelled)));
    assert!(matches!(
        store.job_record(*job_id, actor.user_id),
        Err(Error::NotFound)
    ));
}

#[test]
fn album_plan_keeps_discs_separate_requires_ambiguous_source_choice_and_reports_missing_encoder() {
    let (_root, store, actor, service, collection) = setup();
    let saved = save_inventory(&service, &actor, item(collection.id), Revision(0)).unwrap();
    let first = derivative_import(&service, &actor, collection.id);
    let second = derivative_import(&service, &actor, collection.id);
    for imported in [&first, &second] {
        let mut capture = Catalog::capture(store.as_ref(), imported.capture_id).unwrap();
        let revision = capture.revision;
        capture.association.item_id = Some(saved.id);
        Catalog::save_capture(store.as_ref(), capture, revision).unwrap();
    }
    let unavailable = service
        .music_album_preparation_plan(&actor, saved.id)
        .unwrap();
    assert_eq!(unavailable.discs.len(), 2);
    assert!(!unavailable.can_submit);
    assert_eq!(unavailable.maximum_members, 256);
    assert!(
        unavailable
            .blockers
            .iter()
            .any(|reason| reason.contains("encoder"))
    );
    assert!(matches!(
        service.prepare_music_album(
            &actor,
            saved.id,
            PrepareMusicAlbum {
                source_representation_ids: vec![first.representation_id, second.representation_id],
            },
            &command(),
        ),
        Err(Error::Unavailable(_))
    ));

    let encoder = Arc::new(DerivativeFixtureEncoder::new());
    let service = service.with_audio_converter(encoder);
    let plan = service
        .music_album_preparation_plan(&actor, saved.id)
        .unwrap();
    assert_eq!(plan.discs.len(), 2);
    let first_disc = plan
        .discs
        .iter()
        .find(|disc| disc.capture_id == first.capture_id)
        .unwrap();
    assert!(!first_disc.ambiguous);
    assert!(first_disc.sources[0].default_selected);
    let unambiguous = plan
        .discs
        .iter()
        .find(|disc| disc.capture_id == second.capture_id)
        .unwrap();
    assert!(!unambiguous.ambiguous);
    assert!(unambiguous.sources[0].default_selected);

    let selected = service
        .prepare_music_album(
            &actor,
            saved.id,
            PrepareMusicAlbum {
                source_representation_ids: vec![first.representation_id, second.representation_id],
            },
            &command(),
        )
        .unwrap();
    assert_eq!(selected.members.len(), 4);
    assert_eq!(
        selected
            .members
            .iter()
            .map(|member| member.capture_id)
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        2
    );
    assert!(matches!(
        service.prepare_music_album(
            &actor,
            saved.id,
            PrepareMusicAlbum {
                source_representation_ids: vec![first.representation_id, first.representation_id],
            },
            &command(),
        ),
        Err(Error::Invalid(_))
    ));
    // Distinct captures of one known carrier are alternatives, not extra discs.
    let inventory = service
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
            &command(),
        )
        .unwrap();
    let specimen_id = inventory.specimens[0].id;
    let inventory = service
        .create_component(
            &actor,
            saved.id,
            specimen_id,
            inventory.revision,
            ComponentDetails {
                label: "disc".into(),
                position: 1,
                kind: ComponentKind::OpticalDisc,
                carrier: Some(CarrierDetails {
                    label: "disc".into(),
                    kind: CarrierKind::OpticalDisc,
                }),
            },
            &command(),
        )
        .unwrap();
    let component_id = inventory.specimens[0].components[0].id;
    let carrier_id = inventory.specimens[0].components[0]
        .carrier
        .as_ref()
        .unwrap()
        .id;
    for id in [first.capture_id, second.capture_id] {
        let mut capture = Catalog::capture(store.as_ref(), id).unwrap();
        let revision = capture.revision;
        capture.association = CaptureAssociation {
            item_id: Some(saved.id),
            specimen_id: Some(specimen_id),
            component_id: Some(component_id),
            carrier_id: Some(carrier_id),
        };
        Catalog::save_capture(store.as_ref(), capture, revision).unwrap();
    }
    let plan = service
        .music_album_preparation_plan(&actor, saved.id)
        .unwrap();
    assert_eq!(plan.discs.len(), 1);
    assert!(plan.discs[0].ambiguous);
    assert!(
        plan.discs[0]
            .sources
            .iter()
            .all(|source| !source.default_selected)
    );
    assert!(matches!(
        service.prepare_music_album(
            &actor,
            saved.id,
            PrepareMusicAlbum {
                source_representation_ids: vec![first.representation_id, second.representation_id],
            },
            &command()
        ),
        Err(Error::Invalid(_))
    ));
}

#[test]
fn derivative_failed_validation_cancellation_and_revoked_authority_cannot_publish() {
    for mode in ["validation", "cancel", "revoke", "source"] {
        let (_root, store, actor, service, collection) = setup();
        let imported = derivative_import(&service, &actor, collection.id);
        let encoder = Arc::new(DerivativeFixtureEncoder::new());
        let service = service.with_audio_converter(encoder.clone());
        let job = service
            .prepare_music_derivative(
                &actor,
                imported.capture_id,
                derivative_command(&imported, 1),
                &command(),
            )
            .unwrap();
        match mode {
            "validation" => encoder.broken.store(true, Ordering::SeqCst),
            "cancel" => {
                let store = store.clone();
                let user = actor.user_id;
                *encoder.after_encode.lock().unwrap() = Some(Box::new(move || {
                    store.request_job_cancellation(job, user).unwrap()
                }));
            }
            "revoke" => {
                let other = store
                    .create_user("other-owner", "a long test password", false)
                    .unwrap();
                Catalog::grant(store.as_ref(), collection.id, other, Role::Owner).unwrap();
                let store = store.clone();
                let user = actor.user_id;
                *encoder.after_encode.lock().unwrap() = Some(Box::new(move || {
                    Catalog::grant(store.as_ref(), collection.id, user, Role::Viewer).unwrap()
                }));
            }
            "source" => {
                let store = store.clone();
                *encoder.after_encode.lock().unwrap() = Some(Box::new(move || {
                    let mut capture =
                        Catalog::capture(store.as_ref(), imported.capture_id).unwrap();
                    let rev = capture.revision;
                    capture.representations[0].files[1].digest =
                        Digest::sha256(b"changed source record");
                    Catalog::save_capture(store.as_ref(), capture, rev).unwrap();
                }));
            }
            _ => unreachable!(),
        }
        service.run_next_job("derivative-denial").unwrap();
        assert_eq!(
            Catalog::capture(store.as_ref(), imported.capture_id)
                .unwrap()
                .representations
                .len(),
            1,
            "{mode}"
        );
        assert_eq!(
            service.job(&actor, job).unwrap().state,
            if mode == "cancel" {
                JobState::Cancelled
            } else {
                JobState::Failed
            }
        );
        if mode == "validation" {
            encoder.broken.store(false, Ordering::SeqCst);
            service.retry_job(&actor, job, &command()).unwrap();
            service.run_next_job("derivative-retry").unwrap();
            assert_eq!(service.job(&actor, job).unwrap().state, JobState::Completed);
        }
    }
}

fn derivative_phases() -> [collector_store::PublicationFaultPoint; 6] {
    use collector_store::PublicationFaultPoint::*;
    [
        Prepared,
        BytesStaged,
        PayloadPublished,
        RevisionCommitted,
        ProjectionApplied,
        Completed,
    ]
}

#[test]
fn derivative_publication_failures_restart_without_reencoding_committed_output() {
    for (index, point) in derivative_phases().into_iter().enumerate() {
        let (root, store, actor, service, collection) = setup();
        let imported = derivative_import(&service, &actor, collection.id);
        drop(service);
        drop(store);
        let store = Arc::new(
            Store::open_with_publication_fault(
                root.path().join("archive"),
                root.path().join("state"),
                Arc::new(FailPublicationOnce {
                    point,
                    failed: AtomicBool::new(false),
                }),
            )
            .unwrap(),
        );
        let encoder = Arc::new(DerivativeFixtureEncoder::new());
        let service = CollectionService::new(store.clone(), Arc::new(Registry::builtins()))
            .with_audio_converter(encoder.clone());
        let job = service
            .prepare_music_derivative(
                &actor,
                imported.capture_id,
                derivative_command(&imported, 1),
                &command(),
            )
            .unwrap();
        let result = service.run_next_job("derivative-fault");
        if index < 3 {
            result.unwrap();
            assert_eq!(service.job(&actor, job).unwrap().state, JobState::Failed);
        } else {
            assert!(result.is_err(), "{point:?}");
        }
        assert_eq!(encoder.calls.load(Ordering::SeqCst), 1);
        drop(service);
        drop(store);
        if index == 3 {
            // Model a returned I/O error after the Capture rename but before the
            // private job checkpoint caught up. Archive bytes remain authoritative.
            let db = rusqlite::Connection::open(root.path().join("state/private.sqlite")).unwrap();
            db.execute(
                "UPDATE jobs SET state='failed',phase='failed',claim_token=NULL,worker_instance=NULL,lease_expires=NULL WHERE id=?",
                [job.to_string()],
            )
            .unwrap();
        }
        let store =
            Arc::new(Store::open(root.path().join("archive"), root.path().join("state")).unwrap());
        let service = CollectionService::new(
            store.clone(),
            Arc::new(if index < 3 {
                Registry::builtins()
            } else {
                Registry::default()
            }),
        );
        if index < 3 {
            assert_eq!(
                Catalog::capture(store.as_ref(), imported.capture_id)
                    .unwrap()
                    .representations
                    .len(),
                1,
                "{point:?}"
            );
            let service = service.with_audio_converter(encoder.clone());
            service.retry_job(&actor, job, &command()).unwrap();
            service.run_next_job("derivative-retry-fault").unwrap();
            assert_eq!(
                service.job(&actor, job).unwrap().state,
                JobState::Completed,
                "{point:?}"
            );
            assert_eq!(encoder.calls.load(Ordering::SeqCst), 2);
        } else {
            let other = store
                .create_user("other-owner", "a long test password", false)
                .unwrap();
            Catalog::grant(store.as_ref(), collection.id, other, Role::Owner).unwrap();
            Catalog::grant(store.as_ref(), collection.id, actor.user_id, Role::Viewer).unwrap();
            assert!(
                service
                    .run_next_job("derivative-recover-no-tool-no-grant")
                    .unwrap(),
                "{point:?}"
            );
            assert_eq!(service.job(&actor, job).unwrap().state, JobState::Completed);
            assert_eq!(encoder.calls.load(Ordering::SeqCst), 1);
        }
        store.reindex().unwrap();
        assert_eq!(
            Catalog::capture(store.as_ref(), imported.capture_id)
                .unwrap()
                .representations
                .len(),
            2
        );
    }
}

struct PauseDerivativePhase {
    point: collector_store::PublicationFaultPoint,
    reached: std::sync::Mutex<Option<std::sync::mpsc::Sender<()>>>,
    resume: std::sync::Mutex<std::sync::mpsc::Receiver<()>>,
}
impl collector_store::PublicationFaultInjector for PauseDerivativePhase {
    fn check(&self, point: collector_store::PublicationFaultPoint) -> Result<()> {
        if point == self.point
            && let Some(sender) = self.reached.lock().unwrap().take()
        {
            sender.send(()).unwrap();
            self.resume
                .lock()
                .unwrap()
                .recv_timeout(std::time::Duration::from_secs(15))
                .unwrap();
        }
        Ok(())
    }
}

#[test]
fn derivative_cancellation_at_every_publication_phase_is_atomic() {
    for (index, point) in derivative_phases().into_iter().enumerate() {
        let (root, store, actor, service, collection) = setup();
        let imported = derivative_import(&service, &actor, collection.id);
        drop(service);
        drop(store);
        let (reached_tx, reached_rx) = std::sync::mpsc::channel();
        let (resume_tx, resume_rx) = std::sync::mpsc::channel();
        let store = Arc::new(
            Store::open_with_publication_fault(
                root.path().join("archive"),
                root.path().join("state"),
                Arc::new(PauseDerivativePhase {
                    point,
                    reached: std::sync::Mutex::new(Some(reached_tx)),
                    resume: std::sync::Mutex::new(resume_rx),
                }),
            )
            .unwrap(),
        );
        let service = Arc::new(
            CollectionService::new(store.clone(), Arc::new(Registry::builtins()))
                .with_audio_converter(Arc::new(DerivativeFixtureEncoder::new())),
        );
        let job = service
            .prepare_music_derivative(
                &actor,
                imported.capture_id,
                derivative_command(&imported, 1),
                &command(),
            )
            .unwrap();
        std::thread::scope(|scope| {
            let worker = scope.spawn(|| service.run_next_job("derivative-cancel-phase"));
            reached_rx
                .recv_timeout(std::time::Duration::from_secs(15))
                .unwrap();
            let cancel = store.request_job_cancellation(job, actor.user_id);
            if index < 3 {
                cancel.unwrap();
            } else {
                assert!(matches!(cancel, Err(Error::CancellationTooLate)));
            }
            resume_tx.send(()).unwrap();
            worker.join().unwrap().unwrap();
        });
        assert_eq!(
            service.job(&actor, job).unwrap().state,
            if index < 3 {
                JobState::Cancelled
            } else {
                JobState::Completed
            },
            "{point:?}"
        );
        drop(service);
        drop(store);
        let store = Store::open(root.path().join("archive"), root.path().join("state")).unwrap();
        assert_eq!(
            Catalog::capture(&store, imported.capture_id)
                .unwrap()
                .representations
                .len(),
            if index < 3 { 1 } else { 2 },
            "{point:?}"
        );
    }
}

#[test]
fn derivative_real_flac_publication_uses_independent_pcm_validation() {
    if std::env::var_os("COLLECTOR_TEST_FLAC").is_none() {
        return;
    }
    let tool = collector_providers::ConfiguredEncoder::probe(
        collector_providers::EncoderKind::Flac,
        "flac",
    )
    .expect("COLLECTOR_TEST_FLAC requires real flac");
    let version = tool.version().to_string();
    let converter = collector_providers::ProcessAudioConverter::new(Some(tool), None).unwrap();
    let (_root, store, actor, service, collection) = setup();
    let imported = derivative_import(&service, &actor, collection.id);
    let service = service.with_audio_converter(Arc::new(converter));
    for position in [1, 2] {
        let mut request = derivative_command(&imported, position);
        request.encoder_version = version.clone();
        let job = service
            .prepare_music_derivative(&actor, imported.capture_id, request, &command())
            .unwrap();
        service.run_next_job("real-flac-publication").unwrap();
        assert_eq!(
            service.job(&actor, job).unwrap().state,
            JobState::Completed,
            "{:?}",
            service.job(&actor, job).unwrap()
        );
    }
    store.reindex().unwrap();
    let capture = Catalog::capture(store.as_ref(), imported.capture_id).unwrap();
    assert_eq!(capture.representations.len(), 3);
    for r in &capture.representations[1..] {
        let recipe = store.recipe(RecipeId(r.id.0)).unwrap();
        assert_eq!(recipe.encoder_version, version);
        let inventory = store
            .package_inventory(collection.id, PackageScope::Representation(r.id))
            .unwrap();
        let mut reader = store
            .package_file(collection.id, inventory.scope, inventory.files[0].id)
            .unwrap();
        let mut magic = [0; 4];
        reader.read_exact(&mut magic).unwrap();
        assert_eq!(&magic, b"fLaC");
    }
}

#[test]
fn derivative_queued_cancellation_changed_encoder_source_bytes_and_corrupt_scratch_fail_closed() {
    for mode in [
        "queued_cancel",
        "missing_encoder",
        "changed_encoder",
        "source_bytes",
        "scratch_bytes",
    ] {
        let (root, store, actor, service, collection) = setup();
        let imported = derivative_import(&service, &actor, collection.id);
        let encoder = Arc::new(DerivativeFixtureEncoder::new());
        let configured = service.with_audio_converter(encoder.clone());
        let job = configured
            .prepare_music_derivative(
                &actor,
                imported.capture_id,
                derivative_command(&imported, 1),
                &command(),
            )
            .unwrap();
        let service = match mode {
            "queued_cancel" => {
                store.request_job_cancellation(job, actor.user_id).unwrap();
                configured
            }
            "missing_encoder" => {
                CollectionService::new(store.clone(), Arc::new(Registry::builtins()))
            }
            "changed_encoder" => configured.with_audio_converter(Arc::new(
                collector_providers::ProcessAudioConverter::default(),
            )),
            "source_bytes" => {
                let collection_path = std::fs::read_dir(root.path().join("archive/collections"))
                    .unwrap()
                    .next()
                    .unwrap()
                    .unwrap()
                    .path();
                let path = collection_path.join(format!(
                    "captures/{}/payload/{}/disc.bin",
                    imported.capture_id, imported.representation_id
                ));
                let mut data = std::fs::read(&path).unwrap();
                data[0] ^= 1;
                std::fs::write(path, data).unwrap();
                configured
            }
            "scratch_bytes" => configured.with_audio_converter(Arc::new(CorruptDerivativeEncoder)),
            _ => unreachable!(),
        };
        service.run_next_job("derivative-preflight").unwrap();
        assert_eq!(
            Catalog::capture(store.as_ref(), imported.capture_id)
                .unwrap()
                .representations
                .len(),
            1
        );
        assert_eq!(
            service.job(&actor, job).unwrap().state,
            if mode == "queued_cancel" {
                JobState::Cancelled
            } else {
                JobState::Failed
            },
            "{mode}"
        );
        assert_eq!(encoder.calls.load(Ordering::SeqCst), 0, "{mode}");
    }
}

struct CorruptDerivativeEncoder;
impl collector_app::AudioConversionPort for CorruptDerivativeEncoder {
    fn availability(&self, recipe: &ConversionRecipe) -> collector_app::EncoderAvailability {
        DerivativeFixtureEncoder::new().availability(recipe)
    }
    fn convert(
        &self,
        recipe: &ConversionRecipe,
        source: &mut (dyn junk_libs_disc::CdPcmReader + Send),
        scratch: &mut dyn collector_app::ConversionScratch,
        execution: &mut dyn ProcessingExecutionContext,
    ) -> Result<collector_app::ConversionValidation> {
        let validation =
            DerivativeFixtureEncoder::new().convert(recipe, source, scratch, execution)?;
        scratch.rewind().unwrap();
        scratch.write_all(b"bad").unwrap();
        Ok(validation)
    }
}

#[test]
fn derivative_reindex_rejects_corrupt_published_bytes_and_preserves_master() {
    let (root, store, actor, service, collection) = setup();
    let imported = derivative_import(&service, &actor, collection.id);
    let service = service.with_audio_converter(Arc::new(DerivativeFixtureEncoder::new()));
    let job = service
        .prepare_music_derivative(
            &actor,
            imported.capture_id,
            derivative_command(&imported, 1),
            &command(),
        )
        .unwrap();
    service.run_next_job("derivative-corruption").unwrap();
    assert_eq!(service.job(&actor, job).unwrap().state, JobState::Completed);
    let capture = Catalog::capture(store.as_ref(), imported.capture_id).unwrap();
    let collection_path = std::fs::read_dir(root.path().join("archive/collections"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let master = collection_path.join(format!(
        "captures/{}/payload/{}/disc.bin",
        imported.capture_id, imported.representation_id
    ));
    let preserved = std::fs::read(&master).unwrap();
    let r = &capture.representations[1];
    std::fs::write(
        collection_path.join(format!(
            "captures/{}/payload/{}/{}",
            capture.id, r.id, r.files[0].name
        )),
        b"broken output!",
    )
    .unwrap();
    assert!(store.reindex().is_err());
    assert_eq!(std::fs::read(master).unwrap(), preserved);
    drop(service);
    drop(store);
    assert!(Store::open(root.path().join("archive"), root.path().join("state")).is_err());
}

#[test]
fn subprocess_derivative_worker() {
    let Ok(root) = std::env::var("D072_CHILD_ROOT") else {
        return;
    };
    let point = parse_publication_point(&std::env::var("D072_CHILD_POINT").unwrap());
    let store = Arc::new(
        Store::open_with_publication_fault(
            Path::new(&root).join("archive"),
            Path::new(&root).join("state"),
            Arc::new(BlockAtPublicationPoint(point)),
        )
        .unwrap(),
    );
    let service = CollectionService::new(store, Arc::new(Registry::builtins()))
        .with_audio_converter(Arc::new(DerivativeFixtureEncoder::new()));
    let _ = service.run_next_job("derivative-child");
    panic!("child derivative unexpectedly returned");
}

#[test]
fn derivative_sigkill_at_each_publication_phase_recovers_original_identity() {
    use std::io::BufRead as _;
    for (index, point) in derivative_phases().into_iter().enumerate() {
        let (root, store, actor, service, collection) = setup();
        let imported = derivative_import(&service, &actor, collection.id);
        let service = service.with_audio_converter(Arc::new(DerivativeFixtureEncoder::new()));
        let job = service
            .prepare_music_derivative(
                &actor,
                imported.capture_id,
                derivative_command(&imported, 1),
                &command(),
            )
            .unwrap();
        drop(service);
        drop(store);
        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "subprocess_derivative_worker", "--nocapture"])
            .env("D072_CHILD_ROOT", root.path())
            .env("D072_CHILD_POINT", format!("{point:?}"))
            .stdout(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        let mut output = std::io::BufReader::new(child.stdout.take().unwrap());
        loop {
            let mut line = String::new();
            assert_ne!(
                output.read_line(&mut line).unwrap(),
                0,
                "child exited before {point:?}"
            );
            if line.contains("P01_PHASE:") {
                break;
            }
        }
        child.kill().unwrap();
        child.wait().unwrap();
        let store =
            Arc::new(Store::open(root.path().join("archive"), root.path().join("state")).unwrap());
        let service = CollectionService::new(store.clone(), Arc::new(Registry::builtins()));
        let encoder = Arc::new(DerivativeFixtureEncoder::new());
        let service = if index < 3 {
            service.with_audio_converter(encoder.clone())
        } else {
            service
        };
        service.run_next_job("derivative-parent-recovery").unwrap();
        assert_eq!(
            service.job(&actor, job).unwrap().state,
            JobState::Completed,
            "{point:?}"
        );
        assert_eq!(encoder.calls.load(Ordering::SeqCst), usize::from(index < 3));
        assert_eq!(
            Catalog::capture(store.as_ref(), imported.capture_id)
                .unwrap()
                .representations
                .len(),
            2
        );
    }
}

struct ConcurrentDerivativeEncoder(std::sync::Barrier);
impl collector_app::AudioConversionPort for ConcurrentDerivativeEncoder {
    fn availability(&self, recipe: &ConversionRecipe) -> collector_app::EncoderAvailability {
        DerivativeFixtureEncoder::new().availability(recipe)
    }
    fn convert(
        &self,
        recipe: &ConversionRecipe,
        source: &mut (dyn junk_libs_disc::CdPcmReader + Send),
        scratch: &mut dyn collector_app::ConversionScratch,
        execution: &mut dyn ProcessingExecutionContext,
    ) -> Result<collector_app::ConversionValidation> {
        self.0.wait();
        DerivativeFixtureEncoder::new().convert(recipe, source, scratch, execution)
    }
}

#[test]
fn derivative_simultaneous_encoders_converge_on_one_output_and_recipe() {
    let (_root, store, actor, service, collection) = setup();
    let imported = derivative_import(&service, &actor, collection.id);
    let user = store
        .create_user("second-curator", "a long test password", false)
        .unwrap();
    Catalog::grant(store.as_ref(), collection.id, user, Role::Curator).unwrap();
    let other = Principal {
        user_id: user,
        administrator: false,
        device_id: None,
    };
    let service = Arc::new(
        service.with_audio_converter(Arc::new(ConcurrentDerivativeEncoder(
            std::sync::Barrier::new(2),
        ))),
    );
    let first = service
        .prepare_music_derivative(
            &actor,
            imported.capture_id,
            derivative_command(&imported, 1),
            &command(),
        )
        .unwrap();
    let second = service
        .prepare_music_derivative(
            &other,
            imported.capture_id,
            derivative_command(&imported, 1),
            &command(),
        )
        .unwrap();
    std::thread::scope(|scope| {
        for _ in 0..2 {
            let service = &service;
            scope.spawn(move || service.run_next_job("same-server-instance").unwrap());
        }
    });
    assert_eq!(
        service.job(&actor, first).unwrap().state,
        JobState::Completed
    );
    assert_eq!(
        service.job(&other, second).unwrap().state,
        JobState::Completed
    );
    assert_eq!(
        Catalog::capture(store.as_ref(), imported.capture_id)
            .unwrap()
            .representations
            .len(),
        2
    );
    store.reindex().unwrap();
}

#[test]
fn derivative_encoder_version_changes_require_a_new_recipe_and_frozen_jobs_do_not_upgrade() {
    let (_root, store, actor, service, collection) = setup();
    let imported = derivative_import(&service, &actor, collection.id);
    let encoder = Arc::new(DerivativeFixtureEncoder::new());
    let service = service.with_audio_converter(encoder.clone());
    let old = service
        .prepare_music_derivative(
            &actor,
            imported.capture_id,
            derivative_command(&imported, 1),
            &command(),
        )
        .unwrap();
    *encoder.version.lock().unwrap() = "fixture encoder 2".into();
    service.run_next_job("changed-encoder").unwrap();
    assert_eq!(service.job(&actor, old).unwrap().state, JobState::Failed);
    assert_eq!(
        Catalog::capture(store.as_ref(), imported.capture_id)
            .unwrap()
            .representations
            .len(),
        1
    );
    let mut next = derivative_command(&imported, 1);
    next.encoder_version = "fixture encoder 2".into();
    let new = service
        .prepare_music_derivative(&actor, imported.capture_id, next, &command())
        .unwrap();
    service.run_next_job("new-encoder-recipe").unwrap();
    assert_eq!(service.job(&actor, new).unwrap().state, JobState::Completed);
    *encoder.version.lock().unwrap() = "fixture encoder 1".into();
    service.retry_job(&actor, old, &command()).unwrap();
    service.run_next_job("original-encoder-retry").unwrap();
    assert_eq!(service.job(&actor, old).unwrap().state, JobState::Completed);
    let capture = Catalog::capture(store.as_ref(), imported.capture_id).unwrap();
    assert_eq!(capture.representations.len(), 3);
    assert_ne!(capture.representations[1].id, capture.representations[2].id);
    assert_ne!(
        store
            .recipe(RecipeId(capture.representations[1].id.0))
            .unwrap()
            .digest,
        store
            .recipe(RecipeId(capture.representations[2].id.0))
            .unwrap()
            .digest
    );
}

#[test]
fn derivative_recovery_and_reindex_reject_recipe_presentation_disagreement() {
    for field in ["format", "role", "profile"] {
        let (_root, store, actor, service, collection) = setup();
        let imported = derivative_import(&service, &actor, collection.id);
        let encoder = Arc::new(DerivativeFixtureEncoder::new());
        let service = service.with_audio_converter(encoder.clone());
        service
            .prepare_music_derivative(
                &actor,
                imported.capture_id,
                derivative_command(&imported, 1),
                &command(),
            )
            .unwrap();
        service.run_next_job("original-derivative").unwrap();
        let mut capture = Catalog::capture(store.as_ref(), imported.capture_id).unwrap();
        let expected = capture.revision;
        // Simulate a structurally valid but contradictory retained Capture. Its
        // file digest and validation envelope still match the original output.
        match field {
            "format" => capture.representations[1].format = "opus".into(),
            "role" => capture.representations[1].role = RepresentationRole::LossyDerivative,
            "profile" => {
                capture.representations[1].profile =
                    Some(ConversionProfile::Opus { bitrate_kbps: 96 })
            }
            _ => unreachable!(),
        }
        Catalog::save_capture(store.as_ref(), capture, expected).unwrap();
        let retry = service
            .prepare_music_derivative(
                &actor,
                imported.capture_id,
                derivative_command(&imported, 1),
                &command(),
            )
            .unwrap();
        service.run_next_job("retained-derivative-check").unwrap();
        assert_eq!(
            service.job(&actor, retry).unwrap().state,
            JobState::Failed,
            "{field}"
        );
        assert_eq!(encoder.calls.load(Ordering::SeqCst), 1);
        let error = store.reindex().unwrap_err();
        assert!(
            error
                .to_string()
                .contains("inconsistent archived derivative evidence"),
            "{field}: {error}"
        );
    }
}

#[test]
fn metadata_inventory_edits_and_observation_identity_are_preserved() {
    let (_root, store, actor, service, collection) = setup();
    let saved = save_inventory(&service, &actor, item(collection.id), Revision(0)).unwrap();
    let observation = MetadataRefreshObservation {
        id: ObservationId::new(),
        outcome: MetadataRefreshOutcome::NoMatch,
        candidates: vec![],
        raw_observation: RawJson::from_json("{}").unwrap(),
        observed_at: chrono::Utc::now(),
    };
    let first = service
        .refresh_metadata(
            &actor,
            saved.id,
            RefreshMetadataCommand {
                expected_revision: Revision(0),
                observation: observation.clone(),
            },
            &command(),
        )
        .unwrap();
    let mut forged = first.clone();
    forged.revision = Revision(2);
    forged.display.as_mut().unwrap().title.value = "Forged display".into();
    assert!(matches!(
        store.publish_metadata_authority(TransactionId::new(), forged),
        Err(Error::Invalid(_))
    ));
    let mut changed = observation.clone();
    changed.raw_observation = RawJson::from_json("{\"changed\":true}").unwrap();
    assert!(matches!(
        service.refresh_metadata(
            &actor,
            saved.id,
            RefreshMetadataCommand {
                expected_revision: Revision(1),
                observation: changed
            },
            &command()
        ),
        Err(Error::Conflict)
    ));
    let mut edited = saved.clone();
    edited.title = "Later manual correction".into();
    edited.subtitle = "New subtitle".into();
    save_inventory(&service, &actor, edited, saved.revision).unwrap();
    assert_eq!(
        service
            .metadata_view(&actor, saved.id)
            .unwrap()
            .display
            .title
            .value,
        "Later manual correction"
    );
    let mut stale = first;
    stale.revision = Revision(2);
    assert!(matches!(
        store.publish_metadata_authority(TransactionId::new(), stale),
        Err(Error::Conflict)
    ));
    let refreshed = service
        .refresh_metadata(
            &actor,
            saved.id,
            RefreshMetadataCommand {
                expected_revision: Revision(1),
                observation,
            },
            &command(),
        )
        .unwrap();
    assert_eq!(
        refreshed.display.unwrap().title.value,
        "Later manual correction"
    );
    store.reindex().unwrap();
    assert_eq!(
        service
            .metadata_view(&actor, saved.id)
            .unwrap()
            .display
            .subtitle
            .unwrap()
            .value,
        "New subtitle"
    );
}

#[test]
fn metadata_refresh_uses_current_versions_and_keeps_historical_selection() {
    let (_root, store, actor, service, collection) = setup();
    let saved = save_inventory(&service, &actor, item(collection.id), Revision(0)).unwrap();
    let mut candidate = MetadataCandidateObservation {
        identity: MetadataCandidateIdentity {
            provider: ModuleId("org.example.metadata".into()),
            provider_record_id: "release".into(),
            provider_record_version: Some("a-old".into()),
        },
        title: MetadataText {
            value: "Old title".into(),
            language: None,
        },
        subtitle: None,
        artwork: None,
        complete_identity: false,
        strength: MatchStrength::Name,
        provider_priority: 10,
        observation: RawJson::from_json("{}").unwrap(),
    };
    let original = candidate.identity.clone();
    let refresh = |expected_revision, candidate| RefreshMetadataCommand {
        expected_revision: Revision(expected_revision),
        observation: MetadataRefreshObservation {
            id: ObservationId::new(),
            outcome: MetadataRefreshOutcome::Candidates,
            candidates: vec![candidate],
            raw_observation: RawJson::from_json("{}").unwrap(),
            observed_at: chrono::Utc::now(),
        },
    };
    service
        .refresh_metadata(&actor, saved.id, refresh(0, candidate.clone()), &command())
        .unwrap();
    service
        .update_metadata_authority(
            &actor,
            saved.id,
            UpdateMetadataAuthorityCommand {
                expected_revision: Revision(1),
                change: MetadataAuthorityChange::ClearFieldOverride {
                    field: MetadataField::Title,
                },
            },
            &command(),
        )
        .unwrap();
    candidate.identity.provider_record_version = Some("z-new".into());
    candidate.title.value = "New title".into();
    let new = service
        .refresh_metadata(&actor, saved.id, refresh(2, candidate), &command())
        .unwrap();
    assert_eq!(
        new.display.unwrap().title.value,
        "New title",
        "old hash order cannot win automatic selection"
    );
    store.reindex().unwrap();
    assert_eq!(
        service
            .metadata_view(&actor, saved.id)
            .unwrap()
            .display
            .title
            .value,
        "New title"
    );
    service
        .update_metadata_authority(
            &actor,
            saved.id,
            UpdateMetadataAuthorityCommand {
                expected_revision: Revision(3),
                change: MetadataAuthorityChange::SelectCandidate {
                    candidate: original,
                },
            },
            &command(),
        )
        .unwrap();
    store.reindex().unwrap();
    let historical = service.metadata_view(&actor, saved.id).unwrap();
    assert_eq!(historical.display.title.value, "Old title");
    assert_eq!(historical.candidates.len(), 2);
}
