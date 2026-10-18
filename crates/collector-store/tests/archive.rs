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
            metadata_terms: collector_modules::metadata_terms("unknown"),
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
            metadata_terms: collector_modules::metadata_terms("unknown"),
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
    fn scan_inputs(
        &self,
        _operation: &CapabilityId,
        _inventory: &collector_core::PackageInventory,
        input: &RawJson,
    ) -> ModuleResult<collector_modules::scan_inputs::ScanInputs> {
        collector_modules::scan_inputs::ScanInputs::new(input)
    }

    fn descriptor(&self) -> ModuleDescriptor {
        ModuleDescriptor {
            id: ModuleId("org.example.reader".into()),
            schema_version: 1,
            display_name: "Reader processing fixture".into(),
            collection_type_label: "Reader fixtures".into(),
            metadata_terms: collector_modules::metadata_terms("unknown"),
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
    requests: Arc<std::sync::Mutex<Vec<String>>>,
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
    let requests = Arc::new(std::sync::Mutex::new(Vec::new()));
    let recorded = requests.clone();
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
            let count = stream.read(&mut request).unwrap();
            recorded
                .lock()
                .unwrap()
                .push(String::from_utf8_lossy(&request[..count]).into_owned());
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
        requests,
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
    fn scan_inputs(
        &self,
        _operation: &CapabilityId,
        _inventory: &collector_core::PackageInventory,
        input: &RawJson,
    ) -> ModuleResult<collector_modules::scan_inputs::ScanInputs> {
        collector_modules::scan_inputs::ScanInputs::new(input)
    }

    fn descriptor(&self) -> ModuleDescriptor {
        ModuleDescriptor {
            id: ModuleId("org.junk.music".into()),
            schema_version: 1,
            display_name: "Import analysis fixture".into(),
            collection_type_label: "Import fixtures".into(),
            metadata_terms: collector_modules::metadata_terms("unknown"),
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
            metadata_terms: collector_modules::metadata_terms("unknown"),
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
    let manifest = path
        .parent()
        .unwrap()
        .join(if path.file_name().unwrap() == "record.v4.json" {
            "manifest.v4.toml"
        } else {
            "manifest.toml"
        });
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
        lifecycle: Default::default(),
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
fn removed_item_is_recoverable_and_keeps_inventory_across_reindex() {
    let (root, store, actor, service, collection) = setup();
    let mut inventory = item(collection.id);
    inventory.specimens.push(Specimen {
        id: SpecimenId::new(),
        label: "Owned copy".into(),
        condition: Condition::Unknown,
        ownership: Ownership::Owned,
        possession: Possession::Present,
        acquisition: None,
        components: vec![],
        loans: vec![],
        private_notes: None,
    });
    let saved = Catalog::save(&*store, inventory, Revision(0)).unwrap();
    let mut encoded = std::io::Cursor::new(Vec::new());
    image::RgbImage::from_pixel(24, 24, image::Rgb([12, 34, 56]))
        .write_to(&mut encoded, image::ImageFormat::Png)
        .unwrap();
    let artwork = service
        .create_asset(
            &actor,
            CreateAssetCommand {
                collection_id: collection.id,
                subject: AssetSubject::Item { item_id: saved.id },
                kind: AssetKind::Cover,
                visibility: Visibility::Shared,
                alt_text: "Preserved cover".into(),
            },
            &encoded.into_inner(),
            &command(),
        )
        .unwrap();
    let saved = Catalog::item(&*store, saved.id).unwrap();
    assert!(Catalog::item(&*store, saved.id).is_ok());
    assert!(service.item_presented(&actor, saved.id).is_ok());
    let pending_plan = service
        .plan_ingest(
            &actor,
            PlanIngestCommand {
                collection_id: collection.id,
                packages: vec![IngestPackageSelection {
                    anchor_file_index: None,
                    source: ImportSourceSummary {
                        kind: ImportSourceKind::Browser,
                        label: None,
                        package_name: "unfinished replacement".into(),
                    },
                    files: vec![UploadFile {
                        name: "disc.bin".into(),
                        bytes: ByteLength(1),
                        sha256: None,
                    }],
                }],
            },
        )
        .unwrap();
    let pending = service
        .start_ingest(
            &actor,
            StartIngestCommand {
                clean_reimport: None,
                plan: pending_plan,
                analysis: collector_app::IngestAnalysisPolicy::PreserveOnly,
                enrichment: IngestEnrichmentPolicy::None,
                prepare_music: false,
                destination: Some(collector_app::IngestDestination {
                    item_id: saved.id,
                    expected_revision: saved.revision,
                }),
            },
            &command(),
        )
        .unwrap();
    let pending_id = pending.imports[0].import_id;
    assert_eq!(
        store.import_record(pending_id).unwrap().view.phase,
        ImportPhase::Staging
    );
    let viewer_id = store
        .create_user("removed-viewer", "another long test password", false)
        .unwrap();
    store.grant(collection.id, viewer_id, Role::Viewer).unwrap();
    let viewer = Principal {
        user_id: viewer_id,
        administrator: false,
        device_id: None,
    };
    let queued = JobId::new();
    store
        .enqueue_job(JobSpec {
            id: queued,
            user_id: actor.user_id,
            collection_id: Some(collection.id),
            item_id: Some(saved.id),
            kind: "catalog_acquisition".into(),
            phase: "queued".into(),
            total_units: 1,
            request: serde_json::json!({}),
            frozen_input: None,
        })
        .unwrap();
    let removed = service
        .set_item_removed(
            &actor,
            saved.id,
            saved.revision,
            true,
            &CommandContext::new("remove-preserved-item").unwrap(),
        )
        .unwrap();
    assert!(removed.deleted);
    assert_eq!(removed.specimens.len(), 1);
    assert_eq!(
        store.import_record(pending_id).unwrap().view.phase,
        ImportPhase::Cancelled
    );
    assert!(
        store
            .guided_ingest_intent(pending_id)
            .unwrap()
            .unwrap()
            .cancelled
    );
    assert!(matches!(
        service.create_provisional_import(
            &actor,
            CreateImportCommand {
                collection_id: collection.id,
                source: ImportSourceSummary {
                    kind: ImportSourceKind::Browser,
                    label: None,
                    package_name: "replacement".into(),
                },
                files: vec![UploadFile {
                    name: "disc.bin".into(),
                    bytes: ByteLength(1),
                    sha256: None,
                }],
                association: CaptureAssociation {
                    item_id: Some(saved.id),
                    ..Default::default()
                },
                expected_item_revision: Some(removed.revision),
            },
            &CommandContext::new("removed-destination-denied").unwrap(),
        ),
        Err(Error::Conflict)
    ));
    assert_eq!(
        store.job_record(queued, actor.user_id).unwrap().state,
        JobState::Cancelled
    );
    assert!(matches!(
        service.item_presented(&viewer, saved.id),
        Err(Error::Forbidden)
    ));
    assert!(matches!(
        service.evidence(&viewer, saved.id),
        Err(Error::Forbidden)
    ));
    assert!(matches!(
        service.activity(&viewer, saved.id),
        Err(Error::Forbidden)
    ));
    assert!(matches!(
        service.asset_resource(&viewer, collection.id, artwork.id, true),
        Err(Error::Forbidden)
    ));
    assert!(matches!(
        service.asset(&viewer, saved.id, artwork.id, true),
        Err(Error::Forbidden)
    ));
    assert!(
        service
            .asset_resource(&actor, collection.id, artwork.id, true)
            .is_ok()
    );
    assert!(
        service
            .browse_presented(
                &viewer,
                BrowseQuery {
                    filter: BrowseFilter::Removed,
                    ..Default::default()
                }
            )
            .unwrap()
            .items
            .is_empty()
    );
    assert!(
        service
            .browse_presented(&actor, BrowseQuery::default())
            .unwrap()
            .items
            .is_empty()
    );
    let removed_page = service
        .browse_presented(
            &actor,
            BrowseQuery {
                filter: BrowseFilter::Removed,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(removed_page.items.len(), 1);
    assert!(removed_page.items[0].deleted);
    assert_eq!(
        service
            .browse_presented(
                &actor,
                BrowseQuery {
                    filter: BrowseFilter::Removed,
                    search: Some("風の歌".into()),
                    ..Default::default()
                }
            )
            .unwrap()
            .items
            .len(),
        1
    );
    store.reindex().unwrap();
    assert!(Catalog::item(&*store, saved.id).unwrap().deleted);
    assert_eq!(
        store.import_record(pending_id).unwrap().view.phase,
        ImportPhase::Cancelled
    );
    let restored = service
        .set_item_removed(
            &actor,
            saved.id,
            removed.revision,
            false,
            &CommandContext::new("restore-preserved-item").unwrap(),
        )
        .unwrap();
    assert_eq!(restored.id, saved.id);
    assert_eq!(restored.specimens.len(), 1);
    assert!(!restored.deleted);
    assert_eq!(
        service
            .browse_presented(&actor, BrowseQuery::default())
            .unwrap()
            .items
            .len(),
        1
    );
    drop(service);
    drop(store);
    let reopened = Store::open(root.path().join("archive"), root.path().join("state")).unwrap();
    assert_eq!(
        Catalog::item(&reopened, saved.id).unwrap().specimens.len(),
        1
    );
    assert_eq!(
        reopened.import_record(pending_id).unwrap().view.phase,
        ImportPhase::Cancelled
    );
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
        13 // Song search adds a rebuildable projection; archived fixture bytes stay unchanged.
    );

    let private = rusqlite::Connection::open(state.join("private.sqlite")).unwrap();
    assert_eq!(
        private
            .query_row("PRAGMA user_version", [], |row| row.get::<_, u32>(0))
            .unwrap(),
        20 // Provider ownership/bindings preserve existing accounts, credentials and requests. preserves existing accounts, credentials and bound requests.
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
fn account_search_pages_reach_later_users_and_exact_reads() {
    let root = tempfile::tempdir().unwrap();
    let store = Store::open(root.path().join("archive"), root.path().join("state")).unwrap();
    for index in 0..7 {
        let username = format!("user-{index:02}");
        store
            .create_user(&username, "a sufficiently long password", index == 0)
            .unwrap();
    }
    let first = store.accounts_page("", None, 3).unwrap();
    assert_eq!(
        first
            .items
            .iter()
            .map(|account| account.username.as_str())
            .collect::<Vec<_>>(),
        vec!["user-00", "user-01", "user-02"]
    );
    let second = store
        .accounts_page("", first.next_cursor.as_deref(), 3)
        .unwrap();
    assert_eq!(
        second
            .items
            .iter()
            .map(|account| account.username.as_str())
            .collect::<Vec<_>>(),
        vec!["user-03", "user-04", "user-05"]
    );
    let third = store
        .accounts_page("", second.next_cursor.as_deref(), 3)
        .unwrap();
    assert_eq!(third.items.len(), 1);
    assert_eq!(third.items[0].username, "user-06");
    assert!(third.next_cursor.is_none());
    let target = store.account(third.items[0].id).unwrap();
    assert_eq!(target.username, "user-06");
    store
        .update_account(
            target.id,
            target.revision,
            "Later display name",
            true,
            false,
        )
        .unwrap();
    let matched = store.accounts_page("later DISPLAY", None, 3).unwrap();
    assert_eq!(matched.items.len(), 1);
    assert_eq!(matched.items[0].id, target.id);
}

#[test]
fn collection_search_pages_authorized_rows_and_filters_before_limit() {
    let root = tempfile::tempdir().unwrap();
    let store = Store::open(root.path().join("archive"), root.path().join("state")).unwrap();
    let owner = store
        .create_user("owner", "a sufficiently long password", true)
        .unwrap();
    let other = store
        .create_user("other", "a sufficiently long password", false)
        .unwrap();
    let mut created = Vec::new();
    for index in 0..8 {
        let collection = Catalog::create_collection(
            &store,
            owner,
            Collection {
                id: CollectionId::new(),
                revision: Revision(1),
                name: format!("Shelf {index:02}"),
                created_at: chrono::Utc::now(),
                binding: CollectionBinding::Assigned {
                    module: ModuleId(
                        if index % 2 == 0 {
                            "org.junk.games"
                        } else {
                            "org.junk.music"
                        }
                        .into(),
                    ),
                },
            },
        )
        .unwrap();
        if index != 3 {
            Catalog::grant(
                &store,
                collection.id,
                other,
                if index % 2 == 0 {
                    Role::Curator
                } else {
                    Role::Viewer
                },
            )
            .unwrap();
        }
        created.push(collection);
    }
    let first = Catalog::collections_page(
        &store,
        other,
        "Shelf",
        None,
        2,
        Some(1),
        Some("org.junk.games"),
    )
    .unwrap();
    assert_eq!(first.items.len(), 2);
    let second = Catalog::collections_page(
        &store,
        other,
        "Shelf",
        first
            .next_cursor
            .as_deref()
            .map(str::parse)
            .transpose()
            .unwrap(),
        2,
        Some(1),
        Some("org.junk.games"),
    )
    .unwrap();
    assert_eq!(second.items.len(), 2);
    assert!(second.next_cursor.is_none());
    let ids: std::collections::HashSet<_> = first
        .items
        .iter()
        .chain(second.items.iter())
        .map(|value| value.id)
        .collect();
    assert_eq!(ids.len(), 4);
    assert!(!ids.contains(&created[3].id));
    assert!(
        Catalog::collections_page(&store, other, "Shelf 03", None, 50, None, None)
            .unwrap()
            .items
            .is_empty()
    );
}

#[test]
fn exact_collection_read_checks_grant_and_rejects_invalid_page_inputs() {
    let (_root, store, owner, service, collection) = setup();
    let outsider = Principal {
        user_id: store
            .create_user("outsider", "a sufficiently long password", false)
            .unwrap(),
        administrator: false,
        device_id: None,
    };
    assert_eq!(
        service
            .presented_collection(&owner, collection.id)
            .unwrap()
            .id,
        collection.id
    );
    assert!(matches!(
        service.presented_collection(&outsider, collection.id),
        Err(Error::Forbidden)
    ));
    assert!(matches!(
        service.collections_page(&owner, "", Some("invalid"), 50, None, None),
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        service.collections_page(&owner, "", None, 0, None, None),
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        service.collections_page(&owner, "", None, 50, Some(3), None),
        Err(Error::Invalid(_))
    ));
}

#[test]
fn personal_session_pages_preserve_current_marker_and_reject_bad_cursors() {
    let root = tempfile::tempdir().unwrap();
    let store = Store::open(root.path().join("archive"), root.path().join("state")).unwrap();
    let owner = store
        .create_user("owner", "a sufficiently long password", true)
        .unwrap();
    let other = store
        .create_user("other", "a sufficiently long password", false)
        .unwrap();
    let mut current = None;
    for index in 0..7 {
        let (_, authenticated) = store
            .login(
                "owner",
                "a sufficiently long password",
                &format!("Browser {index}"),
            )
            .unwrap();
        if index == 6 {
            current = authenticated.principal.device_id;
        }
    }
    store
        .login("other", "a sufficiently long password", "Other browser")
        .unwrap();
    let mut cursor = None;
    let mut seen = Vec::new();
    loop {
        let page = store
            .sessions_page(owner, current, cursor.as_deref(), 3)
            .unwrap();
        assert!(page.items.len() <= 3);
        seen.extend(page.items);
        cursor = page.next_cursor;
        if cursor.is_none() {
            break;
        }
    }
    assert_eq!(seen.len(), 7);
    assert_eq!(seen.iter().filter(|session| session.current).count(), 1);
    assert_eq!(
        store
            .sessions_page(other, None, None, 3)
            .unwrap()
            .items
            .len(),
        1
    );
    assert!(
        store
            .sessions_page(owner, current, Some("malformed"), 3)
            .is_err()
    );
    for index in 0..7 {
        store
            .insert_passkey(
                owner,
                &format!("credential-{index}"),
                serde_json::json!({"counter":index}),
            )
            .unwrap();
    }
    let mut cursor = None;
    let mut passkeys = Vec::new();
    loop {
        let page = store.passkeys_page(owner, cursor.as_deref(), 3).unwrap();
        passkeys.extend(page.items);
        cursor = page.next_cursor;
        if cursor.is_none() {
            break;
        }
    }
    assert_eq!(passkeys.len(), 7);
    for index in 0..4 {
        let enrollment = store
            .start_enrollment(Some(owner), &format!("Device {index}"), "/pair")
            .unwrap();
        store
            .approve_enrollment(owner, &enrollment.user_code)
            .unwrap();
        store
            .complete_enrollment(enrollment.id, &enrollment.completion_secret)
            .unwrap();
    }
    let first_devices = store.native_devices_page(owner, None, 2).unwrap();
    let second_devices = store
        .native_devices_page(owner, first_devices.next_cursor.as_deref(), 2)
        .unwrap();
    assert_eq!(first_devices.items.len(), 2);
    assert_eq!(second_devices.items.len(), 2);
    assert!(second_devices.next_cursor.is_none());
    assert!(store.passkeys_page(owner, Some("malformed"), 3).is_err());
    assert!(
        store
            .native_devices_page(owner, Some("malformed"), 3)
            .is_err()
    );
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

    let grants = AccessPort::grants(&*store, collection.id, "", None, 50)
        .unwrap()
        .items;
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
        AccessPort::grants(&*store, collection.id, "", None, 50)
            .unwrap()
            .items
            .iter()
            .filter(|grant| grant.role == Role::Owner)
            .count(),
        1
    );
}

#[test]
fn collection_access_pages_search_authorized_grants_without_truncation() {
    let root = tempfile::tempdir().unwrap();
    let store = Store::open(root.path().join("archive"), root.path().join("state")).unwrap();
    let owner = store
        .create_user("owner", "a sufficiently long password", true)
        .unwrap();
    let collection = store
        .create_collection(
            owner,
            Collection {
                id: CollectionId::new(),
                revision: Revision(1),
                name: "Paged access".into(),
                created_at: chrono::Utc::now(),
                binding: CollectionBinding::Assigned {
                    module: ModuleId("org.junk.music".into()),
                },
            },
        )
        .unwrap();
    for username in ["alpha", "bravo", "charlie", "delta", "echo"] {
        let user = store
            .create_user(username, "a sufficiently long password", false)
            .unwrap();
        Catalog::grant(&store, collection.id, user, Role::Viewer).unwrap();
    }
    let first = AccessPort::grants(&store, collection.id, "", None, 2).unwrap();
    assert_eq!(
        first
            .items
            .iter()
            .map(|grant| grant.username.as_str())
            .collect::<Vec<_>>(),
        vec!["alpha", "bravo"]
    );
    let second =
        AccessPort::grants(&store, collection.id, "", first.next_cursor.as_deref(), 2).unwrap();
    assert_eq!(
        second
            .items
            .iter()
            .map(|grant| grant.username.as_str())
            .collect::<Vec<_>>(),
        vec!["charlie", "delta"]
    );
    let third =
        AccessPort::grants(&store, collection.id, "", second.next_cursor.as_deref(), 2).unwrap();
    assert_eq!(
        third
            .items
            .iter()
            .map(|grant| grant.username.as_str())
            .collect::<Vec<_>>(),
        vec!["echo", "owner"]
    );
    assert!(third.next_cursor.is_none());
    let filtered = AccessPort::grants(&store, collection.id, "HAR", None, 2).unwrap();
    assert_eq!(filtered.items.len(), 1);
    assert_eq!(filtered.items[0].username, "charlie");
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
        Store::local_recover(
            &state,
            collector_app::LocalAccountSelector::Id(user),
            "the replacement password"
        ),
        Err(Error::Unavailable(_))
    ));
    drop(store);

    Store::local_recover(
        &state,
        collector_app::LocalAccountSelector::Id(user),
        "the replacement password",
    )
    .unwrap();
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
        .start_enrollment(Some(requested), "Living-room client", "/pair")
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
        .start_enrollment(Some(requested), "Cancelled client", "/pair")
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
        .start_enrollment(Some(requested), "Expired client", "/pair")
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
fn revoked_browser_session_and_recovery_code_password_replacement() {
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
    let (_, authenticated) = service
        .login("owner", "a long test password", "recovery setup")
        .unwrap();
    let codes = service.replace_recovery_codes(&authenticated).unwrap();
    store
        .consume_recovery_code("owner", &codes[0], "new long password", "recovered")
        .unwrap();
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
fn durable_claims_fence_stale_workers_and_allow_same_user_parallelism() {
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
    let concurrent = store.claim_job("server-a", 30).unwrap().unwrap();
    assert_eq!(concurrent.id, second.id);
    assert!(store.claim_job("server-a", 30).unwrap().is_none());
    store
        .complete_job(
            concurrent.id,
            &concurrent.claim_token,
            serde_json::json!({}),
        )
        .unwrap();
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
    assert!(store.claim_job("server-b", 30).unwrap().is_none());
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
                kind: "fixture".into(),
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
                        collector_app::AssetWriteContext::Manual,
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
                .start_enrollment(Some(actor.user_id), "native", "https://example.test")
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
fn current_private_schema_preserves_durable_imports_without_private_path_columns() {
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
        20 // Provider ownership and bindings add no private paths to Import records.
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
                clean_reimport: None,
                plan,
                analysis: collector_app::IngestAnalysisPolicy::Automatic,
                enrichment: IngestEnrichmentPolicy::None,
                prepare_music: false,
                destination: None,
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
                clean_reimport: None,
                plan,
                analysis: collector_app::IngestAnalysisPolicy::Automatic,
                enrichment: IngestEnrichmentPolicy::None,
                prepare_music: false,
                destination: None,
            },
            &command(),
        )
        .unwrap();
    let import_id = accepted.imports[0].import_id;
    service
        .stage_provisional_import_chunk(&actor, import_id, 0, 0, rom)
        .unwrap();

    provider_drain(&service);
    let detail = service.ingest_detail(&actor, import_id).unwrap();
    assert_eq!(detail.import.phase, ImportPhase::Published, "{detail:?}");
    assert!(detail.import.analysis_job_id.is_none());
    let ids = service
        .identify_game_entry(&actor, detail.summary.target_item_id.unwrap(), &command())
        .unwrap();
    provider_drain(&service);
    assert_eq!(
        service.job(&actor, ids[0]).unwrap().state,
        JobState::Completed
    );
    let review = service
        .metadata_review(&actor, detail.summary.target_item_id.unwrap())
        .unwrap();
    assert!(matches!(
        review.identification.unwrap().status,
        collector_modules::games::GameDetectionStatus::Unrecognized
    ));
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
                clean_reimport: None,
                plan: stale,
                analysis: collector_app::IngestAnalysisPolicy::Automatic,
                enrichment: IngestEnrichmentPolicy::None,
                prepare_music: false,
                destination: None,
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
                clean_reimport: None,
                plan: plan.clone(),
                analysis: collector_app::IngestAnalysisPolicy::Automatic,
                enrichment: IngestEnrichmentPolicy::None,
                prepare_music: false,
                destination: None,
            },
            &key,
        )
        .unwrap();
    let replay = service
        .start_ingest(
            &actor,
            StartIngestCommand {
                clean_reimport: None,
                plan: plan.clone(),
                analysis: collector_app::IngestAnalysisPolicy::Automatic,
                enrichment: IngestEnrichmentPolicy::None,
                prepare_music: false,
                destination: None,
            },
            &key,
        )
        .unwrap();
    let replay_again = service
        .start_ingest_with_preflight(
            &actor,
            StartIngestCommand {
                clean_reimport: None,
                plan: plan.clone(),
                analysis: collector_app::IngestAnalysisPolicy::Automatic,
                enrichment: IngestEnrichmentPolicy::None,
                prepare_music: false,
                destination: None,
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
    assert_eq!(detail.import.phase, ImportPhase::Published, "{detail:?}");
    assert_eq!(detail.summary.capture_revision, Some(Revision(1)));
    assert!(matches!(
        recovered.discard_ingest(&actor, id, detail.import.revision, &command()),
        Err(Error::CancellationTooLate)
    ));
    let target = detail.summary.target_item_id.unwrap();
    let current_item = store.item(target).unwrap();
    assert_eq!(current_item.title, "Guided album");
    assert_eq!(current_item.specimens.len(), 1);
    assert!(matches!(
        current_item.specimens[0].ownership,
        Ownership::Owned
    ));
    assert_eq!(current_item.specimens[0].possession, Possession::Present);
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
    assert!(
        store.item(target).unwrap().deleted,
        "unused import placeholder still browses"
    );
    store.reindex().unwrap();
    assert!(
        store.item(target).unwrap().deleted,
        "retirement was not portable"
    );
    assert_eq!(store.item(replacement.id).unwrap().captures.len(), 1);

    // Recreate the crash window: association committed, tombstone not yet written.
    let mut original = store.item(target).unwrap();
    original.deleted = false;
    let revision = original.revision;
    store.save(original, revision).unwrap();
    recovered.resume_guided_ingests().unwrap();
    assert!(store.item(target).unwrap().deleted);

    // Meaningful inventory is independent of the moved dump.
    let mut original = store.item(target).unwrap();
    original.deleted = false;
    original.wishlist = true;
    let revision = original.revision;
    store.save(original, revision).unwrap();
    recovered.resume_guided_ingests().unwrap();
    assert!(!store.item(target).unwrap().deleted);
    let mut original = store.item(target).unwrap();
    original.wishlist = false;
    original.title = "My inventory title".into();
    let revision = original.revision;
    store.save(original, revision).unwrap();
    recovered.resume_guided_ingests().unwrap();
    assert!(!store.item(target).unwrap().deleted);

    let mut original = store.item(target).unwrap();
    original.title = "Guided album".into();
    let revision = original.revision;
    let snapshot = store.save(original, revision).unwrap();
    let authority = store
        .metadata_authority(collection.id, target)
        .unwrap()
        .unwrap();
    let overridden = recovered
        .update_metadata_authority(
            &actor,
            target,
            UpdateMetadataAuthorityCommand {
                expected_revision: authority.revision,
                change: MetadataAuthorityChange::SetFieldOverride {
                    field: MetadataField::Subtitle,
                    value: "Keep this note".into(),
                    language: None,
                },
            },
            &command(),
        )
        .unwrap();
    assert!(
        matches!(
            Catalog::retire_item_if_unchanged(&*store, snapshot.clone(), Some(authority.revision)),
            Err(Error::Conflict)
        ),
        "manual authority change lost the retirement race"
    );
    recovered.resume_guided_ingests().unwrap();
    assert!(
        !store.item(target).unwrap().deleted,
        "manual metadata must survive"
    );
    let authority = recovered
        .update_metadata_authority(
            &actor,
            target,
            UpdateMetadataAuthorityCommand {
                expected_revision: overridden.revision,
                change: MetadataAuthorityChange::ClearFieldOverride {
                    field: MetadataField::Subtitle,
                },
            },
            &command(),
        )
        .unwrap();
    let mut other = store.capture(corrected.summary.capture_id).unwrap();
    other.id = CaptureId::new();
    other.revision = Revision(0);
    other.association = CaptureAssociation {
        item_id: Some(target),
        ..Default::default()
    };
    other.representations.clear();
    let _other = store.save_capture(other, Revision(0)).unwrap();
    assert!(
        matches!(
            Catalog::retire_item_if_unchanged(&*store, snapshot, Some(authority.revision)),
            Err(Error::Conflict)
        ),
        "independent capture attachment lost the retirement race"
    );
    recovered.resume_guided_ingests().unwrap();
    assert!(
        !store.item(target).unwrap().deleted,
        "another disc must keep the entry alive"
    );
}

#[test]
fn guided_two_created_items_retire_only_the_reassigned_source_after_settlement() {
    let (_root, store, actor, service, collection) = setup();
    let cue = b"FILE \"disc.bin\" BINARY\n  TRACK 01 AUDIO\n    INDEX 01 00:00:00\n";
    let pcm = vec![0_u8; 2352 * 10];
    let packages = ["Disc A", "Disc B"]
        .into_iter()
        .map(|name| IngestPackageSelection {
            anchor_file_index: None,
            source: ImportSourceSummary {
                kind: ImportSourceKind::Browser,
                label: None,
                package_name: name.into(),
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
        })
        .collect();
    let plan = service
        .plan_ingest(
            &actor,
            PlanIngestCommand {
                collection_id: collection.id,
                packages,
            },
        )
        .unwrap();
    let accepted = service
        .start_ingest(
            &actor,
            StartIngestCommand {
                clean_reimport: None,
                plan,
                analysis: collector_app::IngestAnalysisPolicy::Automatic,
                enrichment: IngestEnrichmentPolicy::None,
                prepare_music: false,
                destination: None,
            },
            &command(),
        )
        .unwrap();
    assert_eq!(accepted.imports.len(), 2);
    for summary in &accepted.imports {
        service
            .stage_provisional_import_chunk(&actor, summary.import_id, 0, 0, cue)
            .unwrap();
        service
            .stage_provisional_import_chunk(&actor, summary.import_id, 1, 0, &pcm)
            .unwrap();
    }
    for _ in 0..20 {
        if !service.run_next_job("two-guided-imports").unwrap() {
            break;
        }
    }
    let source = accepted.imports[0].target_item_id.unwrap();
    let target = accepted.imports[1].target_item_id.unwrap();
    assert_ne!(source, target);
    let first = service
        .ingest_detail(&actor, accepted.imports[0].import_id)
        .unwrap();
    let second = service
        .ingest_detail(&actor, accepted.imports[1].import_id)
        .unwrap();
    assert_eq!(first.import.phase, ImportPhase::Published);
    assert_eq!(second.import.phase, ImportPhase::Published);
    service
        .correct_ingest_association(
            &actor,
            first.summary.import_id,
            collector_app::CorrectIngestAssociationCommand {
                expected_capture_revision: first.summary.capture_revision.unwrap(),
                association: CaptureAssociation {
                    item_id: Some(target),
                    ..Default::default()
                },
                expected_item_revision: Some(store.item(target).unwrap().revision),
            },
            &command(),
        )
        .unwrap();
    assert!(store.item(source).unwrap().deleted);
    service.resume_guided_ingests().unwrap();
    assert!(
        store.item(source).unwrap().deleted,
        "reconciliation revived a retired placeholder"
    );
    store.reindex().unwrap();
    assert!(store.item(source).unwrap().deleted);
    assert_eq!(store.item(target).unwrap().captures.len(), 2);
}

#[test]
fn guided_existing_destination_preserves_inventory_and_is_never_retired_as_a_placeholder() {
    let (_root, store, actor, service, collection) = setup();
    let existing = store.save(item(collection.id), Revision(0)).unwrap();
    let cue = b"FILE \"disc.bin\" BINARY\n TRACK 01 AUDIO\n INDEX 01 00:00:00\n";
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
                        package_name: "260919_134632_disk7".into(),
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
    let destination = Some(collector_app::IngestDestination {
        item_id: existing.id,
        expected_revision: existing.revision,
    });
    assert!(matches!(
        service.start_ingest(
            &actor,
            StartIngestCommand {
                clean_reimport: None,
                plan: plan.clone(),
                analysis: collector_app::IngestAnalysisPolicy::Automatic,
                enrichment: IngestEnrichmentPolicy::None,
                prepare_music: false,
                destination: Some(collector_app::IngestDestination {
                    item_id: existing.id,
                    expected_revision: Revision(0)
                })
            },
            &command()
        ),
        Err(Error::Conflict)
    ));
    let accepted_command = StartIngestCommand {
        clean_reimport: None,
        plan,
        analysis: collector_app::IngestAnalysisPolicy::Automatic,
        enrichment: IngestEnrichmentPolicy::None,
        prepare_music: false,
        destination,
    };
    let acceptance_key = command();
    let accepted = service
        .start_ingest(&actor, accepted_command.clone(), &acceptance_key)
        .unwrap();
    let id = accepted.imports[0].import_id;
    assert!(
        !store
            .guided_ingest_intent(id)
            .unwrap()
            .unwrap()
            .created_item
    );
    service
        .stage_provisional_import_chunk(&actor, id, 0, 0, cue)
        .unwrap();
    service
        .stage_provisional_import_chunk(&actor, id, 1, 0, &pcm)
        .unwrap();
    assert!(service.run_next_job("existing-analysis").unwrap());
    assert!(service.run_next_job("existing-publication").unwrap());
    let detail = service.ingest_detail(&actor, id).unwrap();
    assert_eq!(detail.import.phase, ImportPhase::Published, "{detail:?}");
    assert_eq!(detail.summary.target_item_id, Some(existing.id));
    let saved = store.item(existing.id).unwrap();
    assert_eq!(saved.title, existing.title);
    assert_eq!(saved.subtitle, existing.subtitle);
    assert_eq!(saved.specimens.len(), existing.specimens.len());
    let mut edited = saved.clone();
    edited.title = "Edited after acceptance".into();
    let edited = store.save(edited, saved.revision).unwrap();
    let replay = service
        .start_ingest(&actor, accepted_command.clone(), &acceptance_key)
        .unwrap();
    assert_eq!(replay.imports[0].import_id, id);
    let mut changed_request = accepted_command;
    changed_request
        .destination
        .as_mut()
        .unwrap()
        .expected_revision = edited.revision;
    assert!(matches!(
        service.start_ingest(&actor, changed_request, &acceptance_key),
        Err(Error::IdempotencyConflict)
    ));
    let other = store.save(item(collection.id), Revision(0)).unwrap();
    service
        .correct_ingest_association(
            &actor,
            id,
            collector_app::CorrectIngestAssociationCommand {
                expected_capture_revision: detail.summary.capture_revision.unwrap(),
                association: CaptureAssociation {
                    item_id: Some(other.id),
                    ..Default::default()
                },
                expected_item_revision: Some(other.revision),
            },
            &command(),
        )
        .unwrap();
    service.resume_guided_ingests().unwrap();
    assert!(!store.item(existing.id).unwrap().deleted);
}

#[test]
fn guided_existing_destination_receipt_replays_before_upload_after_an_item_edit() {
    let (_root, store, actor, service, collection) = setup();
    let existing = store.save(item(collection.id), Revision(0)).unwrap();
    let cue = b"FILE \"disc.bin\" BINARY\n TRACK 01 AUDIO\n INDEX 01 00:00:00\n";
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
                        package_name: "Replay before upload".into(),
                    },
                    files: vec![
                        UploadFile {
                            name: "disc.cue".into(),
                            bytes: ByteLength(cue.len() as u64),
                            sha256: None,
                        },
                        UploadFile {
                            name: "disc.bin".into(),
                            bytes: ByteLength(2352 * 10),
                            sha256: None,
                        },
                    ],
                }],
            },
        )
        .unwrap();
    let request = StartIngestCommand {
        clean_reimport: None,
        plan,
        analysis: collector_app::IngestAnalysisPolicy::PreserveOnly,
        enrichment: IngestEnrichmentPolicy::None,
        prepare_music: false,
        destination: Some(collector_app::IngestDestination {
            item_id: existing.id,
            expected_revision: existing.revision,
        }),
    };
    let key = command();
    let accepted = service.start_ingest(&actor, request.clone(), &key).unwrap();
    let mut edited = existing.clone();
    edited.title = "Edited while upload is pending".into();
    let edited = store.save(edited, existing.revision).unwrap();
    let replay = service.start_ingest(&actor, request.clone(), &key).unwrap();
    assert_eq!(accepted.imports[0].import_id, replay.imports[0].import_id);
    let fresh_key = command();
    assert!(matches!(
        service.start_ingest(&actor, request, &fresh_key),
        Err(Error::Conflict)
    ));
    assert_eq!(store.item(existing.id).unwrap().revision, edited.revision);
}

#[test]
fn guided_preservation_only_publishes_without_claiming_analysis() {
    let (_root, store, actor, service, collection) = setup();
    let cue = b"FILE \"disc.bin\" BINARY\n TRACK 01 AUDIO\n INDEX 01 00:00:00\n";
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
                        package_name: "unidentified-disc".into(),
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
                clean_reimport: None,
                plan,
                analysis: collector_app::IngestAnalysisPolicy::PreserveOnly,
                enrichment: IngestEnrichmentPolicy::None,
                prepare_music: false,
                destination: None,
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
    assert!(
        store
            .import_job_records(id, actor.user_id)
            .unwrap()
            .iter()
            .all(|job| job.kind != "import_analysis")
    );
    assert!(service.run_next_job("preservation-only").unwrap());
    let detail = service.ingest_detail(&actor, id).unwrap();
    assert_eq!(detail.import.phase, ImportPhase::Published, "{detail:?}");
    assert_eq!(
        detail
            .steps
            .iter()
            .find(|step| step.step == "identification")
            .unwrap()
            .state,
        collector_app::IngestStepState::Skipped
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
                clean_reimport: None,
                plan,
                analysis: collector_app::IngestAnalysisPolicy::Automatic,
                enrichment: IngestEnrichmentPolicy::None,
                prepare_music: true,
                destination: None,
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
                clean_reimport: None,
                plan: unavailable,
                analysis: collector_app::IngestAnalysisPolicy::Automatic,
                enrichment: IngestEnrichmentPolicy::None,
                prepare_music: true,
                destination: None,
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
                clean_reimport: None,
                plan,
                analysis: collector_app::IngestAnalysisPolicy::Automatic,
                enrichment: IngestEnrichmentPolicy::None,
                prepare_music: true,
                destination: None,
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
                clean_reimport: None,
                plan,
                analysis: collector_app::IngestAnalysisPolicy::Automatic,
                enrichment: IngestEnrichmentPolicy::Automatic,
                prepare_music: false,
                destination: None,
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
    assert_eq!(detail.import.phase, ImportPhase::Published, "{detail:?}");
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
                        package_name: "260919_134632_disk7".into(),
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
                clean_reimport: None,
                plan,
                analysis: collector_app::IngestAnalysisPolicy::Automatic,
                enrichment: IngestEnrichmentPolicy::Automatic,
                prepare_music: false,
                destination: None,
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
    provider_drain(&service);
    let requests = mb_server.requests.lock().unwrap();
    assert!(
        requests
            .iter()
            .any(|request| request.contains("/ws/2/discid/"))
    );
    assert!(requests.iter().all(|request| !request.contains("260919")
        && !request.contains("134632")
        && !request.contains("disk7")));
    drop(requests);
    assert!(
        !service
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
    provider_drain(&service);
    assert!(!service.run_next_job("guided-provider-art-image").unwrap());
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
        collector_app::IngestStepState::Succeeded,
        "{:?}",
        detail
    );
    assert!(detail.steps.iter().any(|step| {
        step.step == "metadata" && step.state == collector_app::IngestStepState::Succeeded
    }));
    assert!(detail.steps.iter().any(|step| {
        step.step == "artwork" && step.state == collector_app::IngestStepState::Succeeded
    }));
    let target = detail.summary.target_item_id.unwrap();
    assert_eq!(store.item(target).unwrap().title, "260919 134632 disk7");
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
                clean_reimport: None,
                plan,
                analysis: collector_app::IngestAnalysisPolicy::Automatic,
                enrichment: IngestEnrichmentPolicy::None,
                prepare_music: false,
                destination: None,
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
    assert_eq!(detail.import.phase, ImportPhase::Published, "{detail:?}");
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
                clean_reimport: None,
                plan,
                analysis: collector_app::IngestAnalysisPolicy::Automatic,
                enrichment: IngestEnrichmentPolicy::Automatic,
                prepare_music: false,
                destination: None,
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
                clean_reimport: None,
                plan,
                analysis: collector_app::IngestAnalysisPolicy::Automatic,
                enrichment: IngestEnrichmentPolicy::None,
                prepare_music: false,
                destination: None,
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
fn stopped_backup_restore_preserves_archive_accounts_authority_loans_and_pending_scratch() {
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

    let mut value = item(collection.id);
    value.specimens.push(Specimen {
        id: SpecimenId::new(),
        label: "Loaned copy".into(),
        condition: Condition::Good,
        ownership: Ownership::Owned,
        possession: Possession::Lent,
        components: vec![],
        acquisition: None,
        private_notes: Some("retain private inventory".into()),
        loans: vec![Loan {
            id: LoanId::new(),
            contact: "Trusted friend".into(),
            lent_at: chrono::Utc::now().date_naive(),
            due_at: None,
            returned_at: None,
        }],
    });
    let saved = Catalog::save(&*store, value, Revision(0)).unwrap();
    service
        .update_metadata_authority(
            &actor,
            saved.id,
            UpdateMetadataAuthorityCommand {
                expected_revision: Revision(0),
                change: MetadataAuthorityChange::SetFieldOverride {
                    field: MetadataField::Title,
                    value: "Manual restore title".into(),
                    language: None,
                },
            },
            &command(),
        )
        .unwrap();
    let saved = store.item(saved.id).unwrap();
    let mut unknown = item(collection.id);
    unknown.domain = ModuleData {
        module: ModuleId("org.junk.music".into()),
        schema_version: 900,
        payload: RawJson::from_json(r#"{"future":{"nested":[1,"retained"]}}"#).unwrap(),
    };
    let unknown = Catalog::save(&*store, unknown, Revision(0)).unwrap();
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
    let authority =
        serde_json::to_value(store.metadata_authority(collection.id, saved.id).unwrap()).unwrap();
    let loans = serde_json::to_value(store.item(saved.id).unwrap().specimens).unwrap();
    let archive = root.path().join("archive");
    let state = root.path().join("state");
    let config = root.path().join("collector.toml");
    std::fs::write(
        &config,
        format!("archive = {archive:?}\nstate = {state:?}\n"),
    )
    .unwrap();
    let backup = root.path().join("backup");
    store.backup(&backup, &config).unwrap();
    drop(service);
    drop(store);

    let restore = root.path().join("isolated-restore");
    std::fs::create_dir(&restore).unwrap();
    let restored_archive = restore.join("archive");
    let restored_state = restore.join("state");
    let restored_config = restore.join("collector.toml");
    let configuration = format!("archive = {restored_archive:?}\nstate = {restored_state:?}\n");
    collector_store::restore_backup(
        &backup,
        &restored_archive,
        &restored_state,
        &restored_config,
        configuration.as_bytes(),
        collector_app::RestoreScope::Full,
    )
    .unwrap();
    assert_eq!(
        tree_hashes(&restored_archive),
        tree_hashes(&backup.join("archive"))
    );
    assert_eq!(
        tree_hashes(&restored_state),
        tree_hashes(&backup.join("state"))
    );

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

    assert_eq!(
        serde_json::to_value(restored.item(unknown.id).unwrap().domain).unwrap(),
        serde_json::to_value(&unknown.domain).unwrap()
    );
    assert_eq!(
        serde_json::to_value(restored.item(saved.id).unwrap().specimens).unwrap(),
        loans
    );
    assert_eq!(
        serde_json::to_value(
            restored
                .metadata_authority(collection.id, saved.id)
                .unwrap()
        )
        .unwrap(),
        authority
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
    drop(restored_service);
    drop(restored);
    let independent_archive = root.path().join("independent-archive");
    let independent_state = root.path().join("independent-state");
    let independent_config = root.path().join("independent.toml");
    let configuration =
        format!("archive = {independent_archive:?}\nstate = {independent_state:?}\n");
    collector_store::restore_backup(
        &backup,
        &independent_archive,
        &independent_state,
        &independent_config,
        configuration.as_bytes(),
        collector_app::RestoreScope::ArchiveOnly,
    )
    .unwrap();
    assert!(!independent_state.join("private.sqlite").exists());
    let independent = Arc::new(Store::open(&independent_archive, &independent_state).unwrap());
    assert_ne!(independent.server_id().unwrap(), server_id);
    assert_eq!(independent.user_count().unwrap(), 0);
    assert!(Catalog::role(&*independent, viewer, collection.id).is_err());
    let new_owner = independent
        .create_user("new-owner", "a fresh long password", true)
        .unwrap();
    assert!(Catalog::role(&*independent, new_owner, collection.id).is_err());
    collector_app::LocalOwnershipRecoveryService::new(independent.clone())
        .recover_owner(collection.id, "new-owner")
        .unwrap();
    assert_eq!(
        Catalog::role(&*independent, new_owner, collection.id).unwrap(),
        Role::Owner
    );
    assert_eq!(
        serde_json::to_value(independent.item(unknown.id).unwrap().domain).unwrap(),
        serde_json::to_value(&unknown.domain).unwrap()
    );
    assert_eq!(
        serde_json::to_value(
            independent
                .metadata_authority(collection.id, saved.id)
                .unwrap()
        )
        .unwrap(),
        authority
    );
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
        acquisition_key: None,
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
            &snapshot_claim(&store, collection.id),
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
            &snapshot_claim(&archive_only, collection.id),
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
        acquisition_key: None,
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
            &snapshot_claim(&store, collection.id),
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
            r#"<datafile><header><name>Nintendo - Game Boy</name><version>{version}</version></header><game name="Known Game"><rom name="game.bin" size="4" sha1="{sha1}"/></game></datafile>"#
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
            "gb".into(),
            LocalDatSource {
                path: dat_path.clone(),
                system: "gb".into(),
                catalog: "No-Intro".into(),
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
            "platform":"gb",
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
                    source: "gb".into(),
                },
                &command(),
            )
            .unwrap();
        provider_drain(service);
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
    provider_drain(&service);
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
    provider_drain(&service);
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
                    "system":"gb",
                    "input":"raw_files"
                })),
            },
            &command(),
        )
        .unwrap_err();
    assert!(matches!(mismatched_snapshot_job, Error::Invalid(_)));
    assert!(!service.run_next_job("mismatched-snapshot").unwrap());

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
                    "system":"gb",
                    "input":"raw_files"
                })),
            },
            &command(),
        )
        .unwrap();
    provider_drain(&service);
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
        store.active_game_snapshots(collection.id).unwrap(),
        vec![second_snapshot]
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
    // Imported structure is enough for response selection. A planning read must
    // not open the audio payload, even when it has become unavailable on disk.
    let payloads = files_named(&root.path().join("archive"), "disc.bin");
    assert!(!payloads.is_empty());
    for path in &payloads {
        std::fs::rename(path, path.with_extension("bin-unavailable")).unwrap();
    }
    let import_plan = service
        .processing_plan(
            &actor,
            published.capture_id,
            StartProcessingCommand {
                operation: OperationId::new(
                    collector_modules::music::ACCURATERIP_VERIFICATION_OPERATION,
                )
                .unwrap(),
                operation_version: 1,
                representation_id: published.representation_id,
                snapshot_id: None,
                selection: RawJson::from_value(serde_json::json!({"maximum_offset":0})),
            },
        )
        .unwrap();
    for path in &payloads {
        std::fs::rename(path.with_extension("bin-unavailable"), path).unwrap();
    }
    assert_eq!(
        import_plan.accuraterip_query.unwrap(),
        collector_modules::music::catalog::DbarQuery {
            track_count: 1,
            accuraterip_id1: id1,
            accuraterip_id2: id2,
            cddb_id: cddb,
        }
    );
    assert_eq!(import_plan.files.len(), 1);
    assert_eq!(
        import_plan.blockers,
        ["Choose an AccurateRip response for this disc before checking."]
    );
    assert_eq!(
        import_plan.accuraterip_structure_observation_id,
        Some(ObservationId(analysis_job.0))
    );
    // Fetch directly from imported layout, without manufacturing a second check.
    unavailable.store(false, Ordering::SeqCst);
    let import_acquisition = service
        .enqueue_accuraterip_acquisition(
            &actor,
            published.capture_id,
            AcquireAccurateRipCommand {
                representation_id: published.representation_id,
                structure_observation_id: import_plan.accuraterip_structure_observation_id.unwrap(),
            },
            &command(),
        )
        .unwrap();
    provider_drain(&service);
    assert_eq!(
        service.job(&actor, import_acquisition).unwrap().state,
        JobState::Completed
    );
    unavailable.store(true, Ordering::SeqCst);
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
    provider_drain(&service);
    assert_eq!(
        service.job(&actor, outage_job).unwrap().state,
        JobState::Failed
    );
    assert!(Catalog::capture(&*store, published.capture_id).is_ok());
    unavailable.store(false, Ordering::SeqCst);
    service.retry_job(&actor, outage_job, &command()).unwrap();
    provider_drain(&service);
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
    provider_drain(&service);
    assert_eq!(
        service.job(&actor, cancelled_job).unwrap().state,
        JobState::Cancelled
    );

    let refreshed_job = acquire(&service);
    provider_drain(&service);
    assert_eq!(
        service.job(&actor, refreshed_job).unwrap().state,
        JobState::Completed
    );
    let snapshots = service
        .curation_catalog_snapshots(&actor, collection.id, None, 10)
        .unwrap();
    assert_eq!(
        snapshots.items.len(),
        1,
        "identical AccurateRip responses do not create permanent snapshots"
    );
    let second_snapshot = completed_provider_snapshot(&service, &actor, refreshed_job);
    assert_eq!(second_snapshot, first_snapshot);

    let verification_command = |snapshot_id| StartProcessingCommand {
        operation: OperationId::new(collector_modules::music::ACCURATERIP_VERIFICATION_OPERATION)
            .unwrap(),
        operation_version: 1,
        representation_id: published.representation_id,
        snapshot_id: Some(snapshot_id),
        selection: RawJson::from_value(serde_json::json!({
            "anchor_file_index":0,"snapshot_id":snapshot_id,"maximum_offset":0
        })),
    };
    let matching_plan = service
        .processing_plan(
            &actor,
            published.capture_id,
            verification_command(first_snapshot),
        )
        .unwrap();
    let expected_query = matching_plan.accuraterip_query.unwrap();
    assert!(matching_plan.blockers.is_empty());
    assert_eq!(expected_query.accuraterip_id1, id1);
    assert_eq!(expected_query.accuraterip_id2, id2);
    assert_eq!(expected_query.cddb_id, cddb);
    // Omitting the envelope reference cannot bypass the prerequisite by leaving
    // a valid response ID only inside the module selection.
    let mut unfrozen_reference = verification_command(first_snapshot);
    unfrozen_reference.snapshot_id = None;
    assert!(matches!(
        service
            .processing_plan(&actor, published.capture_id, unfrozen_reference.clone())
            .unwrap()
            .state,
        collector_app::ScanState::Blocked
    ));
    assert!(matches!(
        service.enqueue_processing_operation(
            &actor,
            published.capture_id,
            unfrozen_reference,
            &command()
        ),
        Err(Error::Invalid(_))
    ));
    // Equal track counts alone cannot choose a reference: each disc ID matters.
    for field in [
        "track_count",
        "accuraterip_id1",
        "accuraterip_id2",
        "cddb_id",
    ] {
        let mut query = serde_json::to_value(&expected_query).unwrap();
        query[field] = serde_json::json!(query[field].as_u64().unwrap() + 1);
        let foreign = SnapshotRecord {
            acquisition_key: None,
            id: SnapshotId::new(),
            collection_id: collection.id,
            provider: ModuleId("org.junk.providers.accuraterip".into()),
            media_type: "application/vnd.accuraterip.dbar".into(),
            bytes: ByteLength(0),
            digest: Digest::sha256(&[]),
            parsed_observation: RawJson::from_value(serde_json::json!({
                "schema_version":1,"query":query,"outcome":"no_data","response_count":0
            })),
            created_at: chrono::Utc::now(),
        };
        let claim = snapshot_claim(&store, collection.id);
        store
            .publish_snapshot(
                &claim,
                foreign.clone(),
                &mut std::io::Cursor::new(Vec::<u8>::new()),
                &mut || Ok(false),
            )
            .unwrap();
        store
            .complete_job(claim.id, &claim.claim_token, serde_json::json!({}))
            .unwrap();
        let request = verification_command(foreign.id);
        let plan = service
            .processing_plan(&actor, published.capture_id, request.clone())
            .unwrap();
        assert!(
            matches!(plan.state, collector_app::ScanState::Blocked),
            "{field}"
        );
        assert!(
            plan.blockers
                .iter()
                .any(|value| value.contains("different disc")),
            "{field}"
        );
        assert!(
            matches!(
                service.enqueue_processing_operation(
                    &actor,
                    published.capture_id,
                    request,
                    &command()
                ),
                Err(Error::Invalid(_))
            ),
            "{field}"
        );
    }

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
    let verified = verified.collect::<Vec<_>>();
    assert_eq!(verified.len(), 1); // Identical actual checks share a result.
    assert_eq!(verified[0].occurrences, 2);
    // Equal verified labels with different evaluated offset coverage are not the same result.
    let wider=service.enqueue_processing_operation(&actor,published.capture_id,StartProcessingCommand {
        operation:OperationId::new(collector_modules::music::ACCURATERIP_VERIFICATION_OPERATION).unwrap(),operation_version:1,
        representation_id:published.representation_id,snapshot_id:Some(first_snapshot),
        selection:RawJson::from_value(serde_json::json!({"package_kind":"cue","anchor_file_index":0,"snapshot_id":first_snapshot,"maximum_offset":1})),
    },&command()).unwrap();
    service.run_next_job("wider-coverage").unwrap();
    assert_eq!(
        service.job(&actor, wider).unwrap().state,
        JobState::Completed
    );
    let wider_summary = service
        .capture_processing_summaries(
            &actor,
            published.capture_id,
            published.representation_id,
            None,
            20,
        )
        .unwrap();
    let verified_groups = wider_summary
        .items
        .iter()
        .filter(|s| {
            s.operation.as_str() == collector_modules::music::ACCURATERIP_VERIFICATION_OPERATION
        })
        .collect::<Vec<_>>();
    assert_eq!(verified_groups.len(), 2);
    assert!(
        verified_groups
            .iter()
            .any(|s| s.observation_id == ObservationId(wider.0) && s.occurrences == 1)
    );

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
    provider_drain(&service);
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
    let repeated_no_data = acquire(&service);
    provider_drain(&service);
    assert_eq!(
        completed_provider_snapshot(&service, &actor, repeated_no_data),
        no_data_snapshot
    );
    assert_ne!(
        no_data_snapshot, first_snapshot,
        "no-data is distinct from successful data"
    );
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
    provider_drain(&recovered);
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
        resolution_policy: 2,
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
        resolution_policy: 2,
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
    provider_drain(&service);
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
    provider_drain(&service);
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
    provider_drain(&service);
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
    provider_drain(&service);
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
    assert_eq!(no_match.display.as_ref().unwrap().artwork, Some(manual.id));
    assert!(
        no_match.observations[0]
            .raw_observation
            .get()
            .contains(&missing.to_string())
    );

    // Equivalent historical response IDs do not create another no-data occurrence.
    let mut legacy = store
        .snapshot(SnapshotQuery {
            collection_id: collection.id,
            snapshot_id: SnapshotId(missing.0),
        })
        .unwrap();
    let mut payload = Vec::new();
    store
        .snapshot_bytes(SnapshotQuery {
            collection_id: collection.id,
            snapshot_id: legacy.id,
        })
        .unwrap()
        .read_to_end(&mut payload)
        .unwrap();
    legacy.id = SnapshotId::new();
    legacy.acquisition_key = None;
    let alias = legacy.id;
    let claim = snapshot_claim(&store, collection.id);
    store
        .publish_snapshot(
            &claim,
            legacy,
            &mut std::io::Cursor::new(payload),
            &mut || Ok(false),
        )
        .unwrap();
    store
        .complete_job(claim.id, &claim.claim_token, serde_json::json!({}))
        .unwrap();
    let unchanged = service
        .refresh_provider_metadata(
            &actor,
            saved.id,
            RefreshProviderMetadataCommand {
                expected_revision: no_match.revision,
                snapshot_id: alias,
                source_observation_id: None,
                artwork: vec![],
            },
            &command(),
        )
        .unwrap();
    assert_eq!(unchanged.revision, no_match.revision);
    assert_eq!(unchanged.observations, no_match.observations);

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
fn removed_item_finishes_an_already_committed_import_receipt() {
    let (root, store, actor, service, collection) = setup();
    let saved = save_inventory(&service, &actor, item(collection.id), Revision(0)).unwrap();
    let import = staged_import(&service, &actor, collection.id, "remove-commit-gap");
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
    let current = Catalog::item(&*reopened, saved.id).unwrap();
    service
        .set_item_removed(&actor, saved.id, current.revision, true, &command())
        .unwrap();
    assert_ne!(service.job(&actor, job).unwrap().state, JobState::Cancelled);
    service.run_next_job("new-process").unwrap();
    let outcome = service.job(&actor, job).unwrap();
    assert_eq!(outcome.state, JobState::Completed, "{outcome:?}");
    assert_eq!(
        service.provisional_import(&actor, import.id).unwrap().phase,
        ImportPhase::Published
    );
    assert!(Catalog::item(&*reopened, saved.id).unwrap().deleted);
    assert_eq!(Catalog::evidence(&*reopened, saved.id).unwrap().len(), 1);
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
        acquisition_key: None,
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
            &snapshot_claim(&store, collection.id),
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
                resolution_policy: 2,
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
    provider_drain(&unavailable);
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
    derivative_import_sectors(service, actor, collection, variant, 5)
}

fn derivative_import_sectors(
    service: &CollectionService,
    actor: &Principal,
    collection: CollectionId,
    variant: u8,
    sectors: usize,
) -> ImportView {
    // Track 1 owns stored HTOA plus track 2's stored INDEX 00: 3 sectors.
    // Track 2 owns the final two sectors. No AccurateRip evidence is requested.
    let cue = b"FILE \"disc.bin\" BINARY\n TRACK 01 AUDIO\n INDEX 00 00:00:00\n INDEX 01 00:00:01\n TRACK 02 AUDIO\n INDEX 00 00:00:02\n INDEX 01 00:00:03\n";
    let extended_cue = format!(
        "FILE \"disc.bin\" BINARY\n TRACK 01 AUDIO\n INDEX 01 00:00:00\n TRACK 02 AUDIO\n INDEX 01 {:02}:{:02}:{:02}\n",
        sectors / 2 / 4500,
        sectors / 2 / 75 % 60,
        sectors / 2 % 75
    );
    let cue = if sectors == 5 {
        cue.as_slice()
    } else {
        extended_cue.as_bytes()
    };
    let pcm: Vec<u8> = (0..sectors * 588)
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
    for (index, chunk) in pcm.chunks(1024 * 1024).enumerate() {
        service
            .stage_provisional_import_chunk(
                actor,
                created.id,
                1,
                (index * 1024 * 1024) as u64,
                chunk,
            )
            .unwrap();
    }
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
    assert_eq!(
        service
            .browse_presented(
                &actor,
                BrowseQuery {
                    filter: BrowseFilter::Playable,
                    ..Default::default()
                }
            )
            .unwrap()
            .items
            .iter()
            .map(|v| v.id)
            .collect::<Vec<_>>(),
        vec![saved.id]
    );

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
fn album_admission_uses_recorded_identity_but_workers_reject_changed_payloads() {
    let (root, store, actor, service, collection) = setup();
    let saved = save_inventory(&service, &actor, item(collection.id), Revision(0)).unwrap();
    let imported = derivative_import(&service, &actor, collection.id);
    let mut capture = Catalog::capture(store.as_ref(), imported.capture_id).unwrap();
    let revision = capture.revision;
    capture.association.item_id = Some(saved.id);
    Catalog::save_capture(store.as_ref(), capture, revision).unwrap();
    let encoder = Arc::new(DerivativeFixtureEncoder::new());
    let service = service.with_audio_converter(encoder.clone());
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
    let mut bytes = std::fs::read(&path).unwrap();
    bytes[0] ^= 1;
    std::fs::write(path, bytes).unwrap();
    // Browsing/admission describe the recorded package; they do not attest to
    // current payload integrity. Background validation must still fail closed.
    let plan = service
        .music_album_preparation_plan(&actor, saved.id)
        .unwrap();
    assert_eq!(plan.total_tracks, 2);
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
    while service.run_next_job("changed-album-source").unwrap() {}
    let batch = service
        .music_preparation_batch(&actor, batch.batch_id)
        .unwrap();
    assert_eq!(batch.failed, 2);
    assert_eq!(batch.succeeded, 0);
    assert_eq!(encoder.calls.load(Ordering::SeqCst), 0);
    assert_eq!(
        Catalog::capture(store.as_ref(), imported.capture_id)
            .unwrap()
            .representations
            .len(),
        1
    );
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
    let reopened = Store::open(root.path().join("archive"), root.path().join("state")).unwrap();
    assert!(reopened.catalog_reused_at_startup());
    assert!(reopened.reindex().is_err());
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
fn metadata_refresh_replaces_versions_and_preserves_unmatched_choice() {
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
        observation: RawJson::from_json(r#"{"a":1,"b":2}"#).unwrap(),
    };
    let original = candidate.identity.clone();
    let old_candidate = candidate.clone();
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
    // Name-only observations are now suggestions. Selection is an explicit
    // decision, rather than a side effect of a subsequent refresh.
    service
        .update_metadata_authority(
            &actor,
            saved.id,
            UpdateMetadataAuthorityCommand {
                expected_revision: Revision(2),
                change: MetadataAuthorityChange::SelectCandidate {
                    candidate: original.clone(),
                },
            },
            &command(),
        )
        .unwrap();
    candidate.identity.provider_record_version = Some("z-new".into());
    candidate.title.value = "New title".into();
    let new = service
        .refresh_metadata(&actor, saved.id, refresh(3, candidate), &command())
        .unwrap();
    assert_eq!(
        new.display.unwrap().title.value,
        "Old title",
        "the displayed candidate must survive a non-equivalent refresh"
    );
    store.reindex().unwrap();
    let historical = service.metadata_view(&actor, saved.id).unwrap();
    assert_eq!(historical.display.title.value, "Old title");
    assert_eq!(historical.candidates.len(), 2);
    assert_eq!(
        historical.authority.selected_candidate,
        Some(original.clone())
    );
    let mut equivalent = old_candidate;
    equivalent.identity.provider_record_version = Some("equivalent-new".into());
    equivalent.observation = RawJson::from_json(r#"{"b":2,"a":1}"#).unwrap();
    service
        .refresh_metadata(&actor, saved.id, refresh(4, equivalent.clone()), &command())
        .unwrap();
    let matched = service.metadata_view(&actor, saved.id).unwrap();
    assert_eq!(
        matched.candidates.len(),
        3,
        "Selectable immutable versions are retained separately from current suggestions."
    );
    assert_eq!(
        matched.authority.selected_candidate,
        Some(equivalent.identity.clone())
    );
    assert_eq!(matched.display.title.value, "Old title");
    store.reindex().unwrap();
    assert_eq!(
        service
            .metadata_view(&actor, saved.id)
            .unwrap()
            .candidates
            .len(),
        3
    );
    let mut reused_identity = equivalent;
    reused_identity.identity = original;
    reused_identity.title.value = "Conflicting reuse".into();
    assert!(matches!(
        service.refresh_metadata(&actor, saved.id, refresh(5, reused_identity), &command()),
        Err(Error::Conflict)
    ));
}

#[test]
fn folder_discovery_is_authorized_disjoint_and_validates_paths() {
    let (_root, _store, actor, service, collection) = setup();
    let input = collector_app::DiscoverIngestCommand {
        collection_id: collection.id,
        files: [
            "Dumps/disc1/disc.cue",
            "Dumps/disc1/disc.bin",
            "Dumps/disc2/disc.toc",
            "Dumps/disc2/disc.scram",
            "Dumps/disc2/disc.state",
            "Dumps/disc2/disc.subcode",
            "Dumps/readme.txt",
        ]
        .into_iter()
        .map(|name| UploadFile {
            name: name.into(),
            bytes: ByteLength(32),
            sha256: None,
        })
        .collect(),
    };
    let discovered = service.discover_ingest(&actor, input.clone()).unwrap();
    assert_eq!(discovered.unassigned_files, vec!["Dumps/readme.txt"]);
    let plan = discovered.plan.unwrap();
    assert_eq!(plan.packages.len(), 2);
    assert_eq!(plan.packages[0].files.len(), 2);
    assert_eq!(plan.packages[1].files.len(), 4);
    assert!(
        plan.packages
            .iter()
            .all(|package| package.anchors.len() == 1)
    );
    let outsider = Principal {
        user_id: UserId::new(),
        administrator: false,
        device_id: None,
    };
    assert!(service.discover_ingest(&outsider, input.clone()).is_err());
    let mut malicious = input.clone();
    malicious.files[0].name = "../outside.cue".into();
    assert!(service.discover_ingest(&actor, malicious).is_err());
    let mut duplicate = input;
    duplicate.files[1].name = "Dumps/disc1/DISC.CUE".into();
    assert!(service.discover_ingest(&actor, duplicate).is_err());
}

#[test]
fn fx01_browse_keysets_filters_effective_titles_and_creation_order_survive_reindex() {
    let clock = Arc::new(TestClock {
        seconds: std::sync::atomic::AtomicI64::new(1_780_000_000),
        millis: std::sync::atomic::AtomicU64::new(0),
    });
    let (_root, store, actor, service, collection) = setup_with_clock(Some(clock.clone()));
    let mut saved = Vec::new();
    for n in 0..105 {
        clock.seconds.fetch_add(1, Ordering::SeqCst);
        let mut value = item(collection.id);
        value.title = format!("Album {:03}", 104 - n);
        value.wishlist = n % 2 == 0;
        value.domain.payload =
            RawJson::from_value(serde_json::json!({"artist":if n%2==0 {"Zebra"} else {"Álvaro"}}));
        saved.push(save_inventory(&service, &actor, value, Revision(0)).unwrap());
    }
    let collect = |sort, filter| {
        let mut cursor = None;
        let mut result = Vec::new();
        loop {
            let page = service
                .browse_presented(
                    &actor,
                    BrowseQuery {
                        sort,
                        filter,
                        limit: Some(17),
                        cursor,
                        ..Default::default()
                    },
                )
                .unwrap();
            assert!(page.items.len() <= 17);
            result.extend(page.items.into_iter().map(|v| v.id));
            cursor = page.next_cursor;
            if cursor.is_none() {
                break;
            }
        }
        let unique = result.iter().collect::<std::collections::HashSet<_>>();
        assert_eq!(unique.len(), result.len());
        result
    };
    let title = collect(BrowseSort::Title, BrowseFilter::All);
    assert_eq!(title, saved.iter().rev().map(|v| v.id).collect::<Vec<_>>());
    assert_eq!(collect(BrowseSort::Title, BrowseFilter::Wishlist).len(), 53);
    assert!(collect(BrowseSort::Title, BrowseFilter::FilesAttached).is_empty());
    assert!(collect(BrowseSort::Title, BrowseFilter::Playable).is_empty());
    let first = service
        .browse_presented(
            &actor,
            BrowseQuery {
                limit: Some(1),
                ..Default::default()
            },
        )
        .unwrap();
    assert!(matches!(
        service.browse_presented(
            &actor,
            BrowseQuery {
                sort: BrowseSort::Artist,
                cursor: first.next_cursor,
                ..Default::default()
            }
        ),
        Err(Error::Invalid(_))
    ));
    let newest = saved.last().unwrap().id;
    let mut edited = saved[0].clone();
    edited.title = "First after edit".into();
    clock.seconds.fetch_add(100, Ordering::SeqCst);
    save_inventory(&service, &actor, edited, saved[0].revision).unwrap();
    assert_eq!(
        collect(BrowseSort::RecentlyAdded, BrowseFilter::All)[0],
        newest
    );
    service
        .update_metadata_authority(
            &actor,
            saved[1].id,
            UpdateMetadataAuthorityCommand {
                expected_revision: Revision(0),
                change: MetadataAuthorityChange::SetFieldOverride {
                    field: MetadataField::Title,
                    value: "000 effective title".into(),
                    language: None,
                },
            },
            &command(),
        )
        .unwrap();
    assert_eq!(
        collect(BrowseSort::Title, BrowseFilter::All)[0],
        saved[1].id
    );
    assert_eq!(
        service
            .browse_presented(
                &actor,
                BrowseQuery {
                    search: Some("Zebra".into()),
                    ..Default::default()
                }
            )
            .unwrap()
            .items
            .len(),
        50
    );
    let expected = collect(BrowseSort::Artist, BrowseFilter::All);
    store.reindex().unwrap();
    assert_eq!(collect(BrowseSort::Artist, BrowseFilter::All), expected);
    assert_eq!(
        collect(BrowseSort::RecentlyAdded, BrowseFilter::All)[0],
        newest
    );
    assert_eq!(
        collect(BrowseSort::Title, BrowseFilter::All)[0],
        saved[1].id
    );
    let other = store
        .create_user("browse-viewer", "a long viewer password", false)
        .unwrap();
    store.grant(collection.id, other, Role::Viewer).unwrap();
    let job = JobSpec {
        id: JobId::new(),
        user_id: actor.user_id,
        collection_id: Some(collection.id),
        item_id: Some(saved[0].id),
        kind: "integrity_verification".into(),
        phase: "queued".into(),
        total_units: 1,
        request: serde_json::json!({}),
        frozen_input: None,
    };
    store.enqueue_job(job).unwrap();
    let claim = store.claim_job("fx01", 30).unwrap().unwrap();
    store
        .fail_job(claim.id, &claim.claim_token, "fixture failure")
        .unwrap();
    assert_eq!(
        collect(BrowseSort::Title, BrowseFilter::NeedsAttention),
        vec![saved[0].id]
    );
    let viewer = Principal {
        user_id: other,
        administrator: false,
        device_id: None,
    };
    assert!(
        service
            .browse_presented(
                &viewer,
                BrowseQuery {
                    filter: BrowseFilter::NeedsAttention,
                    ..Default::default()
                }
            )
            .unwrap()
            .items
            .is_empty()
    );
    let stranger = store
        .create_user("browse-stranger", "a long stranger password", true)
        .unwrap();
    assert!(
        store
            .browse(stranger, BrowseQuery::default())
            .unwrap()
            .items
            .is_empty()
    );
}

#[test]
fn fx01_attached_audio_filters_read_committed_records_without_payload_reads() {
    let (root, store, actor, service, collection) = setup();
    let saved = save_inventory(&service, &actor, item(collection.id), Revision(0)).unwrap();
    // A tiny valid PCM WAVE master is playable directly, without preparation.
    let mut wav = Vec::new();
    wav.extend(b"RIFF");
    wav.extend(40u32.to_le_bytes());
    wav.extend(b"WAVEfmt ");
    wav.extend(16u32.to_le_bytes());
    wav.extend(1u16.to_le_bytes());
    wav.extend(1u16.to_le_bytes());
    wav.extend(44100u32.to_le_bytes());
    wav.extend(88200u32.to_le_bytes());
    wav.extend(2u16.to_le_bytes());
    wav.extend(16u16.to_le_bytes());
    wav.extend(b"data");
    wav.extend(4u32.to_le_bytes());
    wav.extend([0u8; 4]);
    let upload = service
        .create_upload(
            &actor,
            saved.id,
            saved.revision,
            vec![UploadFile {
                name: "tiny.wav".into(),
                bytes: ByteLength(wav.len() as u64),
                sha256: None,
            }],
            &command(),
        )
        .unwrap();
    service.upload_chunk(&actor, upload.id, 0, 0, &wav).unwrap();
    let _published = service
        .commit_upload(&actor, upload.id, &command())
        .unwrap();
    for filter in [BrowseFilter::FilesAttached, BrowseFilter::Playable] {
        let page = service
            .browse_presented(
                &actor,
                BrowseQuery {
                    filter,
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(
            page.items.iter().map(|v| v.id).collect::<Vec<_>>(),
            vec![saved.id]
        );
    }
    // Missing physical bytes must not turn an inventory query into a full read.
    // Accessing the actual media still reports the missing file independently.
    store.reindex().unwrap();
    let paths = files_named(&root.path().join("archive"), "tiny.wav");
    assert!(!paths.is_empty());
    for path in paths {
        std::fs::remove_file(path).unwrap();
    }
    assert_eq!(
        service
            .browse_presented(
                &actor,
                BrowseQuery {
                    filter: BrowseFilter::Playable,
                    ..Default::default()
                }
            )
            .unwrap()
            .items
            .len(),
        1
    );
}

#[test]
fn fx02_concurrent_scans_converge_and_rechecks_survive_archive_rebuild() {
    let clock = Arc::new(TestClock::new(1_900_000_000));
    let (root, store, actor, base, collection) = setup_with_clock(Some(clock.clone()));
    let reviewed = reviewed_unattached_import(&base, &actor, collection.id, "scan-convergence");
    base.enqueue_import_publication(&actor, reviewed.id, &command())
        .unwrap();
    base.run_next_job("publish").unwrap();
    let mut registry = Registry::builtins();
    registry.register(Box::new(ReaderProcessingModule)).unwrap();
    let service = Arc::new(CollectionService::new(store.clone(), Arc::new(registry)));
    let request = StartProcessingCommand {
        operation: OperationId::new("org.example.reader.inspect").unwrap(),
        operation_version: 1,
        representation_id: reviewed.representation_id,
        snapshot_id: None,
        selection: RawJson::from_value(serde_json::json!({})),
    };
    assert!(matches!(
        service
            .processing_plan(&actor, reviewed.capture_id, request.clone())
            .unwrap()
            .state,
        collector_app::ScanState::Ready
    ));
    let barrier = Arc::new(std::sync::Barrier::new(4));
    let handles = (0..4)
        .map(|_| {
            let service = service.clone();
            let actor = actor.clone();
            let request = request.clone();
            let barrier = barrier.clone();
            thread::spawn(move || {
                barrier.wait();
                service
                    .enqueue_processing_operation(&actor, reviewed.capture_id, request, &command())
                    .unwrap()
            })
        })
        .collect::<Vec<_>>();
    let jobs = handles
        .into_iter()
        .map(|h| h.join().unwrap())
        .collect::<Vec<_>>();
    assert!(jobs.iter().all(|id| id == &jobs[0]));
    clock.advance(60);
    assert!(matches!(
        service
            .processing_plan(&actor, reviewed.capture_id, request.clone())
            .unwrap()
            .state,
        collector_app::ScanState::Running
    ));
    service.run_next_job("scan").unwrap();
    assert_eq!(
        service.job(&actor, jobs[0]).unwrap().state,
        JobState::Completed
    );
    let first = service
        .processing_plan(&actor, reviewed.capture_id, request.clone())
        .unwrap()
        .result
        .unwrap();
    assert_eq!(first.occurrences, 1);
    clock.advance(120);
    let receipt = command();
    let repeat = service
        .enqueue_processing_operation(&actor, reviewed.capture_id, request.clone(), &receipt)
        .unwrap();
    assert_ne!(repeat, jobs[0]);
    service.run_next_job("scan").unwrap();
    let second = service
        .processing_plan(&actor, reviewed.capture_id, request.clone())
        .unwrap()
        .result
        .unwrap();
    assert_eq!(second.occurrences, 2);
    assert_eq!(second.observation_id, first.observation_id);
    assert_eq!(second.created_at, first.created_at);
    assert_eq!(
        second.last_checked_at - first.last_checked_at,
        chrono::Duration::seconds(120)
    );
    clock.advance(300);
    assert_eq!(
        service
            .enqueue_processing_operation(&actor, reviewed.capture_id, request.clone(), &receipt)
            .unwrap(),
        repeat
    );
    assert!(!service.run_next_job("scan").unwrap());
    assert_eq!(
        service
            .processing_plan(&actor, reviewed.capture_id, request.clone())
            .unwrap()
            .result
            .unwrap()
            .last_checked_at,
        second.last_checked_at
    );
    let count_kind = |kind: &str| {
        files_named(&root.path().join("archive"), "record.v4.json")
            .into_iter()
            .filter(|e| e.to_string_lossy().contains(kind))
            .count()
    };
    assert_eq!(count_kind("processing_observations"), 1);
    assert_eq!(count_kind("processing_occurrences"), 1);
    store.reindex().unwrap();
    let rebuilt = service
        .processing_plan(&actor, reviewed.capture_id, request.clone())
        .unwrap()
        .result
        .unwrap();
    assert_eq!(rebuilt.occurrences, 2);
    assert_eq!(rebuilt.last_checked_at, second.last_checked_at);
    let repeated = store.processing_record(ObservationId(repeat.0)).unwrap();
    let original = store.processing_record(ObservationId(jobs[0].0)).unwrap();
    assert_eq!(
        collector_app::processing_result_key(&original).unwrap(),
        collector_app::processing_result_key(&repeated).unwrap()
    );
    let mut distinct = repeated.clone();
    distinct
        .observation
        .limitations
        .push("Partial coverage".into());
    assert_ne!(
        collector_app::processing_result_key(&original).unwrap(),
        collector_app::processing_result_key(&distinct).unwrap()
    );
    let mut distinct = repeated;
    distinct.inputs.as_mut().unwrap().selection = serde_json::json!({"offset":1});
    assert_ne!(
        collector_app::processing_result_key(&original).unwrap(),
        collector_app::processing_result_key(&distinct).unwrap()
    );
    let active = service
        .enqueue_processing_operation(&actor, reviewed.capture_id, request.clone(), &command())
        .unwrap();
    let claim = store.claim_job("cancel-gap", 30).unwrap().unwrap();
    assert_eq!(claim.id, active);
    service.cancel_job(&actor, active, &command()).unwrap();
    assert!(matches!(
        service
            .processing_plan(&actor, reviewed.capture_id, request.clone())
            .unwrap()
            .state,
        collector_app::ScanState::Cancelling
    ));
    assert_eq!(
        service
            .enqueue_processing_operation(&actor, reviewed.capture_id, request.clone(), &command())
            .unwrap(),
        active
    );
    store
        .cancel_claimed_job(active, &claim.claim_token)
        .unwrap();
    let failed = service
        .enqueue_processing_operation(&actor, reviewed.capture_id, request.clone(), &command())
        .unwrap();
    let claim = store.claim_job("fail-once", 30).unwrap().unwrap();
    store
        .fail_job(failed, &claim.claim_token, "interrupted scan")
        .unwrap();
    let replacement = service
        .enqueue_processing_operation(&actor, reviewed.capture_id, request.clone(), &command())
        .unwrap();
    assert_eq!(
        service.retry_job(&actor, failed, &command()).unwrap(),
        replacement
    );
    assert_eq!(service.job(&actor, failed).unwrap().state, JobState::Failed);
    service.run_next_job("replacement").unwrap();
    assert!(!service.run_next_job("no-duplicate").unwrap());
    let archive_copy = tempfile::tempdir().unwrap();
    copy_tree(
        &root.path().join("archive"),
        &archive_copy.path().join("archive"),
    );
    let recovered = Store::open(
        archive_copy.path().join("archive"),
        archive_copy.path().join("fresh-state"),
    )
    .unwrap();
    let recovered_record = recovered
        .processing_record(ObservationId(repeat.0))
        .unwrap();
    assert_eq!(
        collector_app::processing_result_key(&recovered_record).unwrap(),
        collector_app::processing_result_key(&original).unwrap()
    );
    let occurrence = files_named(&archive_copy.path().join("archive"), "record.v4.json")
        .into_iter()
        .find(|p| p.to_string_lossy().contains("processing_occurrences"))
        .unwrap();
    rewrite_record(&occurrence, |v| {
        v["snapshot_id"] = serde_json::json!(SnapshotId::new())
    });
    assert!(
        recovered.reindex().is_err(),
        "a contradictory occurrence must not acquire its reference's successful result"
    );
    let payload = find_file_with_bytes(&root.path().join("archive"), b"data").unwrap();
    std::fs::remove_file(payload).unwrap();
    let admitted = service
        .enqueue_processing_operation(&actor, reviewed.capture_id, request.clone(), &command())
        .unwrap();
    assert!(matches!(
        service
            .processing_plan(&actor, reviewed.capture_id, request)
            .unwrap()
            .state,
        collector_app::ScanState::Running
    ));
    service.run_next_job("missing-bytes").unwrap();
    assert_eq!(
        service.job(&actor, admitted).unwrap().state,
        JobState::Failed
    );
}

#[test]
fn fx02_manual_analysis_ownership_survives_guided_cancel_and_restart() {
    let clock = Arc::new(TestClock::new(1_900_100_000));
    let (root, store, actor, service, collection) = setup_with_clock(Some(clock.clone()));
    let cue = b"FILE \"disc.bin\" BINARY\n TRACK 01 AUDIO\n INDEX 01 00:00:00\n";
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
                        package_name: "shared.cue".into(),
                    },
                    files: vec![
                        UploadFile {
                            name: "disc.bin".into(),
                            bytes: ByteLength(pcm.len() as u64),
                            sha256: None,
                        },
                        UploadFile {
                            name: "disc.cue".into(),
                            bytes: ByteLength(cue.len() as u64),
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
                clean_reimport: None,
                plan,
                analysis: collector_app::IngestAnalysisPolicy::Automatic,
                enrichment: IngestEnrichmentPolicy::None,
                prepare_music: false,
                destination: None,
            },
            &command(),
        )
        .unwrap();
    let id = accepted.imports[0].import_id;
    service
        .stage_provisional_import_chunk(&actor, id, 0, 0, &pcm)
        .unwrap();
    service
        .stage_provisional_import_chunk(&actor, id, 1, 0, cue)
        .unwrap();
    service.resume_guided_ingests().unwrap();
    let jobs = store.import_job_records(id, actor.user_id).unwrap();
    let guided = jobs.iter().find(|j| j.kind == "import_analysis").unwrap();
    clock.advance(60);
    let direct = service
        .enqueue_import_analysis(&actor, id, ModuleId("org.junk.music".into()), &command())
        .unwrap();
    assert_eq!(direct, guided.id);
    let shared_history = service
        .work_history_entry(&actor, &collector_app::WorkTarget::Import(id))
        .unwrap();
    assert!(shared_history.shared);
    assert!(
        !shared_history
            .actions
            .contains(&collector_app::WorkHistoryAction::CancelAndClear)
    );
    assert!(
        service
            .change_work_history(
                &actor,
                collector_app::WorkHistoryCommand {
                    target: collector_app::WorkTarget::Import(id),
                    action: collector_app::WorkHistoryAction::CancelAndClear
                },
                &command()
            )
            .is_err()
    );
    let direct_history = service
        .work_history_entry(&actor, &collector_app::WorkTarget::Job(direct))
        .unwrap();
    assert!(direct_history.shared);
    let claim = store
        .claim_job("interrupted-shared-worker", 30)
        .unwrap()
        .unwrap();
    assert_eq!(claim.id, direct);
    let revision = service.provisional_import(&actor, id).unwrap().revision;
    service
        .cancel_ingest(&actor, id, revision, &command())
        .unwrap();
    assert_eq!(
        service.job(&actor, direct).unwrap().state,
        JobState::Running
    );
    assert_eq!(
        service.provisional_import(&actor, id).unwrap().phase,
        ImportPhase::Analyzing
    );
    assert_eq!(
        service.provisional_import(&actor, id).unwrap().revision,
        revision,
        "parent cancellation invalidated the independent analysis revision"
    );
    store
        .fail_job(
            direct,
            &claim.claim_token,
            "interrupted independently requested analysis",
        )
        .unwrap();
    assert_eq!(
        service.retry_job(&actor, direct, &command()).unwrap(),
        direct
    );
    assert_eq!(
        store
            .claim_job("interrupted-again", 30)
            .unwrap()
            .unwrap()
            .id,
        direct
    );
    clock.advance(31);
    drop(service);
    drop(store);
    let store = Arc::new(
        Store::open_with_clock(
            root.path().join("archive"),
            root.path().join("state"),
            clock,
        )
        .unwrap(),
    );
    let service = CollectionService::new(store.clone(), Arc::new(Registry::builtins()));
    service.run_next_job("manual-after-restart").unwrap();
    assert_eq!(
        service.job(&actor, direct).unwrap().state,
        JobState::Completed
    );
    assert_eq!(
        service.provisional_import(&actor, id).unwrap().phase,
        ImportPhase::AwaitingReview
    );
    assert!(!service.run_next_job("cancelled-parent").unwrap());
}

#[test]
fn fx03_history_filters_dismissal_clear_recovery_and_authorization() {
    use collector_app::{WorkHistoryAction as A, WorkHistoryCommand as C, WorkQuery, WorkTarget};
    let clock = Arc::new(TestClock::new(1_900_000_000));
    let (root, store, actor, service, collection) = setup_with_clock(Some(clock.clone()));
    let first = staged_import(&service, &actor, collection.id, "first package");
    clock.advance(10);
    let second = reviewed_unattached_import(&service, &actor, collection.id, "second package");
    let job = service
        .enqueue_import_publication(&actor, second.id, &command())
        .unwrap();
    service.run_next_job("publish").unwrap();
    assert_eq!(service.job(&actor, job).unwrap().state, JobState::Completed);
    let q = WorkQuery {
        limit: Some(1),
        ..Default::default()
    };
    let page = service.work_history(&actor, q.clone()).unwrap();
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].name, "second package");
    assert_eq!(page.items[0].status, "completed");
    let cursor = page.next_cursor.unwrap();
    clock.advance(10);
    let third = staged_import(&service, &actor, collection.id, "third package");
    let older = service
        .work_history(
            &actor,
            WorkQuery {
                cursor: Some(cursor),
                ..q
            },
        )
        .unwrap();
    assert_eq!(older.items[0].name, "first package");
    assert!(older.next_cursor.is_none());
    assert_eq!(
        service
            .work_history(
                &actor,
                WorkQuery {
                    search: "DISC.BIN".into(),
                    ..Default::default()
                }
            )
            .unwrap()
            .items
            .len(),
        3
    );
    assert_eq!(
        service
            .work_history(
                &actor,
                WorkQuery {
                    status: "needs_input".into(),
                    ..Default::default()
                }
            )
            .unwrap()
            .items
            .len(),
        2
    );
    assert!(
        service
            .work_history(
                &actor,
                WorkQuery {
                    search: "received_bytes".into(),
                    ..Default::default()
                }
            )
            .unwrap()
            .items
            .is_empty(),
        "search must match file names, not storage JSON field names"
    );
    let published = WorkTarget::Import(second.id);
    let dismiss = C {
        target: published.clone(),
        action: A::Dismiss,
    };
    let receipt = command();
    service
        .change_work_history(&actor, dismiss.clone(), &receipt)
        .unwrap();
    service
        .change_work_history(&actor, dismiss, &receipt)
        .unwrap();
    assert_eq!(
        service
            .work_history(&actor, WorkQuery::default())
            .unwrap()
            .items
            .len(),
        2
    );
    assert_eq!(
        service
            .work_history(
                &actor,
                WorkQuery {
                    archived: true,
                    ..Default::default()
                }
            )
            .unwrap()
            .items
            .len(),
        1
    );
    assert!(Catalog::capture(&*store, second.capture_id).is_ok());
    assert!(
        service
            .change_work_history(
                &actor,
                C {
                    target: published.clone(),
                    action: A::Discard
                },
                &command()
            )
            .is_err()
    );
    service
        .change_work_history(
            &actor,
            C {
                target: published,
                action: A::Restore,
            },
            &command(),
        )
        .unwrap();
    let target = WorkTarget::Import(first.id);
    assert!(
        service
            .change_work_history(
                &actor,
                C {
                    target: target.clone(),
                    action: A::Dismiss
                },
                &command()
            )
            .is_err()
    );
    service
        .change_work_history(
            &actor,
            C {
                target: target.clone(),
                action: A::CancelAndClear,
            },
            &command(),
        )
        .unwrap();
    assert!(
        service
            .stage_provisional_import_chunk(&actor, first.id, 0, 0, b"data")
            .is_err()
    );
    assert!(
        service
            .enqueue_import_analysis(
                &actor,
                first.id,
                ModuleId("org.junk.music".into()),
                &command()
            )
            .is_err()
    );
    let user = store
        .create_user("another-work-user", "another very long password", false)
        .unwrap();
    store.grant(collection.id, user, Role::Curator).unwrap();
    let other = Principal {
        user_id: user,
        administrator: false,
        device_id: None,
    };
    assert!(
        service
            .work_history(&other, WorkQuery::default())
            .unwrap()
            .items
            .is_empty()
    );
    assert!(matches!(
        service.work_history_entry(&other, &target),
        Err(Error::NotFound)
    ));
    assert!(
        service
            .change_work_history(
                &other,
                C {
                    target: target.clone(),
                    action: A::Restore
                },
                &command()
            )
            .is_err()
    );
    drop(service);
    drop(store);
    let store =
        Arc::new(Store::open(root.path().join("archive"), root.path().join("state")).unwrap());
    let service = CollectionService::new(store.clone(), Arc::new(Registry::builtins()));
    service.resume_guided_ingests().unwrap();
    let cleared = service.work_history_entry(&actor, &target).unwrap();
    assert!(cleared.archived);
    assert!(!cleared.clear_pending);
    assert_eq!(cleared.reclaimed_bytes, 4);
    assert_eq!(cleared.pending_bytes, 0);
    assert!(
        !root
            .path()
            .join("state/imports")
            .join(first.id.to_string())
            .exists()
    );
    assert!(
        root.path()
            .join("state/imports")
            .join(third.id.to_string())
            .exists()
    );
    service
        .change_work_history(
            &actor,
            C {
                target,
                action: A::Restore,
            },
            &command(),
        )
        .unwrap();
    assert_eq!(
        service.provisional_import(&actor, first.id).unwrap().phase,
        ImportPhase::Cancelled
    );
    assert!(Catalog::capture(&*store, second.capture_id).is_ok());
    let failed_target = WorkTarget::Import(third.id);
    service
        .change_work_history(
            &actor,
            C {
                target: failed_target.clone(),
                action: A::CancelAndClear,
            },
            &command(),
        )
        .unwrap();
    let dir = root.path().join("state/imports").join(third.id.to_string());
    let retained = dir.with_extension("retained");
    std::fs::rename(&dir, &retained).unwrap();
    std::fs::write(&dir, b"blocked cleanup fixture").unwrap();
    service.resume_guided_ingests().unwrap();
    let failed = service.work_history_entry(&actor, &failed_target).unwrap();
    assert!(failed.clear_pending);
    assert!(!failed.archived);
    assert!(failed.cleanup_error.is_some());
    assert_eq!(failed.pending_bytes, 4);
    assert!(failed.actions.contains(&A::RetryCleanup));
    std::fs::remove_file(&dir).unwrap();
    std::fs::rename(&retained, &dir).unwrap();
    service
        .change_work_history(
            &actor,
            C {
                target: failed_target.clone(),
                action: A::RetryCleanup,
            },
            &command(),
        )
        .unwrap();
    service.resume_guided_ingests().unwrap();
    let settled = service.work_history_entry(&actor, &failed_target).unwrap();
    assert!(settled.archived);
    assert_eq!(settled.reclaimed_bytes, 4);
    let repeated = store.cleanup_import(third.id).unwrap();
    assert_eq!(repeated.reclaimed_bytes, ByteLength(4));
    assert_eq!(repeated.pending_bytes, ByteLength(0));
    assert_eq!(
        service
            .work_history_entry(&actor, &failed_target)
            .unwrap()
            .reclaimed_bytes,
        4
    );
    assert!(Catalog::capture(&*store, second.capture_id).is_ok());
    let retry_import = reviewed_unattached_import(&service, &actor, collection.id, "retry fence");
    let retry_job = service
        .enqueue_import_publication(&actor, retry_import.id, &command())
        .unwrap();
    let claim = store.claim_job("fixture", 60).unwrap().unwrap();
    assert_eq!(claim.id, retry_job);
    store
        .fail_job(claim.id, &claim.claim_token, "first failed attempt")
        .unwrap();
    let retry_target = WorkTarget::Import(retry_import.id);
    service
        .change_work_history(
            &actor,
            C {
                target: retry_target.clone(),
                action: A::Dismiss,
            },
            &command(),
        )
        .unwrap();
    assert!(service.retry_job(&actor, retry_job, &command()).is_err());
    service
        .change_work_history(
            &actor,
            C {
                target: retry_target,
                action: A::Restore,
            },
            &command(),
        )
        .unwrap();
    service.retry_job(&actor, retry_job, &command()).unwrap();
    service.run_next_job("restored retry").unwrap();
    assert_eq!(
        service.job(&actor, retry_job).unwrap().state,
        JobState::Completed
    );
}

fn fx05_plan(
    service: &CollectionService,
    actor: &Principal,
    collection: CollectionId,
    files: Vec<UploadFile>,
) -> collector_app::IngestPlan {
    service
        .plan_ingest(
            actor,
            PlanIngestCommand {
                collection_id: collection,
                packages: vec![IngestPackageSelection {
                    source: ImportSourceSummary {
                        kind: ImportSourceKind::Browser,
                        label: None,
                        package_name: "Replacement".into(),
                    },
                    files,
                    anchor_file_index: None,
                }],
            },
        )
        .unwrap()
}
fn fx05_drain(service: &CollectionService) {
    for _ in 0..20 {
        service.resume_guided_ingests().unwrap();
        if !service.run_next_job("fx05").unwrap() {
            return;
        }
    }
    panic!("reimport failed to settle");
}
fn fx05_original(
    store: &Store,
    service: &CollectionService,
    actor: &Principal,
    collection: CollectionId,
) -> Item {
    let original = store.save(item(collection), Revision(0)).unwrap();
    let plan = fx05_plan(
        service,
        actor,
        collection,
        vec![UploadFile {
            name: "disc.bin".into(),
            bytes: ByteLength(4),
            sha256: None,
        }],
    );
    let accepted = service
        .start_ingest(
            actor,
            StartIngestCommand {
                clean_reimport: None,
                plan,
                analysis: collector_app::IngestAnalysisPolicy::PreserveOnly,
                enrichment: IngestEnrichmentPolicy::None,
                prepare_music: false,
                destination: Some(collector_app::IngestDestination {
                    item_id: original.id,
                    expected_revision: original.revision,
                }),
            },
            &command(),
        )
        .unwrap();
    service
        .stage_provisional_import_chunk(actor, accepted.imports[0].import_id, 0, 0, b"data")
        .unwrap();
    fx05_drain(service);
    store.item(original.id).unwrap()
}

#[test]
fn clean_reimport_reuses_validated_originals_preserves_authority_and_survives_restart() {
    let (root, store, actor, service, collection) = setup();
    let original = fx05_original(&store, &service, &actor, collection.id);
    assert_eq!(original.captures.len(), 1);
    let old = &original.captures[0];
    let representation = &old.representations[0];
    let authority = service
        .update_metadata_authority(
            &actor,
            original.id,
            UpdateMetadataAuthorityCommand {
                expected_revision: Revision(0),
                change: MetadataAuthorityChange::SetFieldOverride {
                    field: MetadataField::Title,
                    value: "My chosen title".into(),
                    language: None,
                },
            },
            &command(),
        )
        .unwrap();
    let plan = fx05_plan(
        &service,
        &actor,
        collection.id,
        representation
            .files
            .iter()
            .map(|file| UploadFile {
                name: file.name.clone(),
                bytes: file.bytes,
                sha256: Some(file.digest.hex.clone()),
            })
            .collect(),
    );
    let request = StartIngestCommand {
        clean_reimport: Some(collector_app::CleanReimportOptions {
            replace_captures: vec![old.id],
            retained_representation: Some(representation.id),
            reset_metadata: false,
            reset_artwork: false,
            expected_metadata_revision: authority.revision,
        }),
        plan,
        analysis: collector_app::IngestAnalysisPolicy::PreserveOnly,
        enrichment: IngestEnrichmentPolicy::None,
        prepare_music: false,
        destination: Some(collector_app::IngestDestination {
            item_id: original.id,
            expected_revision: original.revision,
        }),
    };
    let key = command();
    let accepted = service.start_ingest(&actor, request.clone(), &key).unwrap();
    let id = accepted.imports[0].import_id;
    assert_eq!(
        store.item(original.id).unwrap().captures[0].id,
        old.id,
        "old usable capture disappeared before replacement"
    );
    assert_eq!(
        service
            .start_ingest(&actor, request.clone(), &key)
            .unwrap()
            .imports[0]
            .import_id,
        id
    );
    drop(service);
    drop(store);
    let store =
        Arc::new(Store::open(root.path().join("archive"), root.path().join("state")).unwrap());
    let service = CollectionService::new(store.clone(), Arc::new(Registry::builtins()));
    fx05_drain(&service);
    let detail = service.ingest_detail(&actor, id).unwrap();
    assert_eq!(
        detail.import.phase,
        ImportPhase::Published,
        "{:?}",
        detail.summary
    );
    let replaced = store.item(original.id).unwrap();
    assert_eq!(replaced.captures.len(), 1);
    assert_ne!(replaced.captures[0].id, old.id);
    assert_eq!(
        replaced.captures[0].representations[0].files[0].digest,
        Digest::sha256(b"data")
    );
    assert_eq!(replaced.title, original.title);
    assert_eq!(replaced.wishlist, original.wishlist);
    assert_eq!(
        service
            .metadata_view(&actor, original.id)
            .unwrap()
            .display
            .title
            .value,
        "My chosen title"
    );
    assert!(store.capture(old.id).is_ok(), "historical originals lost");
    let mut media = service
        .representation(&actor, original.id, representation.id, 0)
        .unwrap();
    let mut bytes = Vec::new();
    std::io::Read::read_to_end(&mut media.reader, &mut bytes).unwrap();
    assert_eq!(bytes, b"data");
    store.reindex().unwrap();
    assert_eq!(
        store.item(original.id).unwrap().captures[0].id,
        replaced.captures[0].id
    );
    assert_eq!(
        service.start_ingest(&actor, request, &key).unwrap().imports[0].import_id,
        id
    );
}

#[test]
fn clean_reimport_failed_replacement_keeps_original_and_explicit_reset_is_portable() {
    let (_root, store, actor, service, collection) = setup();
    let original = fx05_original(&store, &service, &actor, collection.id);
    let old = original.captures[0].id;
    let authority = service
        .update_metadata_authority(
            &actor,
            original.id,
            UpdateMetadataAuthorityCommand {
                expected_revision: Revision(0),
                change: MetadataAuthorityChange::SetFieldOverride {
                    field: MetadataField::Title,
                    value: "Override to reset".into(),
                    language: None,
                },
            },
            &command(),
        )
        .unwrap();
    let plan = fx05_plan(
        &service,
        &actor,
        collection.id,
        vec![UploadFile {
            name: "replacement.bin".into(),
            bytes: ByteLength(4),
            sha256: Some(Digest::sha256(b"good").hex),
        }],
    );
    let accepted = service
        .start_ingest(
            &actor,
            StartIngestCommand {
                clean_reimport: Some(collector_app::CleanReimportOptions {
                    replace_captures: vec![old],
                    retained_representation: None,
                    reset_metadata: true,
                    reset_artwork: true,
                    expected_metadata_revision: authority.revision,
                }),
                plan,
                analysis: collector_app::IngestAnalysisPolicy::PreserveOnly,
                enrichment: IngestEnrichmentPolicy::None,
                prepare_music: false,
                destination: Some(collector_app::IngestDestination {
                    item_id: original.id,
                    expected_revision: original.revision,
                }),
            },
            &command(),
        )
        .unwrap();
    let id = accepted.imports[0].import_id;
    assert!(
        service
            .stage_provisional_import_chunk(&actor, id, 0, 0, b"evil")
            .is_err()
    );
    assert_eq!(store.item(original.id).unwrap().captures[0].id, old);
    assert_eq!(
        service
            .metadata_view(&actor, original.id)
            .unwrap()
            .display
            .title
            .value,
        original.title
    );
    store.reindex().unwrap();
    assert_eq!(store.item(original.id).unwrap().captures[0].id, old);
    assert_eq!(
        service
            .metadata_view(&actor, original.id)
            .unwrap()
            .display
            .title
            .value,
        original.title
    );
    // Cancelling a failed replacement leaves the old usable state intact.
    let detail = service.ingest_detail(&actor, id).unwrap();
    service
        .cancel_ingest(&actor, id, detail.import.revision, &command())
        .unwrap();
    fx05_drain(&service);
    assert_eq!(store.item(original.id).unwrap().captures[0].id, old);
}

#[test]
fn clean_reimport_fences_old_jobs_but_keeps_direct_scan_owners_and_later_manual_edits() {
    let (_root, store, actor, service, collection) = setup();
    let original = fx05_original(&store, &service, &actor, collection.id);
    let cue = b"FILE \"disc.bin\" BINARY\n TRACK 01 AUDIO\n INDEX 01 00:00:00\n";
    let pcm = vec![0; 2352 * 10];
    let plan = fx05_plan(
        &service,
        &actor,
        collection.id,
        vec![
            UploadFile {
                name: "disc.bin".into(),
                bytes: ByteLength(pcm.len() as u64),
                sha256: None,
            },
            UploadFile {
                name: "disc.cue".into(),
                bytes: ByteLength(cue.len() as u64),
                sha256: None,
            },
        ],
    );
    let earlier = service
        .start_ingest(
            &actor,
            StartIngestCommand {
                clean_reimport: None,
                plan,
                analysis: collector_app::IngestAnalysisPolicy::Automatic,
                enrichment: IngestEnrichmentPolicy::None,
                prepare_music: false,
                destination: Some(collector_app::IngestDestination {
                    item_id: original.id,
                    expected_revision: original.revision,
                }),
            },
            &command(),
        )
        .unwrap()
        .imports[0]
        .import_id;
    service
        .stage_provisional_import_chunk(&actor, earlier, 0, 0, &pcm)
        .unwrap();
    service
        .stage_provisional_import_chunk(&actor, earlier, 1, 0, cue)
        .unwrap();
    service.resume_guided_ingests().unwrap();
    let direct = service
        .enqueue_import_analysis(
            &actor,
            earlier,
            ModuleId("org.junk.music".into()),
            &command(),
        )
        .unwrap();
    let obsolete = JobId::new();
    store
        .enqueue_job(JobSpec {
            id: obsolete,
            user_id: actor.user_id,
            collection_id: Some(collection.id),
            item_id: Some(original.id),
            kind: "import_staging".into(),
            phase: "queued".into(),
            total_units: 1,
            request: serde_json::json!({"import_id":earlier}),
            frozen_input: None,
        })
        .unwrap();
    let old = &original.captures[0];
    let rep = &old.representations[0];
    let plan = fx05_plan(
        &service,
        &actor,
        collection.id,
        rep.files
            .iter()
            .map(|file| UploadFile {
                name: file.name.clone(),
                bytes: file.bytes,
                sha256: Some(file.digest.hex.clone()),
            })
            .collect(),
    );
    let accepted = service
        .start_ingest(
            &actor,
            StartIngestCommand {
                clean_reimport: Some(collector_app::CleanReimportOptions {
                    replace_captures: vec![old.id],
                    retained_representation: Some(rep.id),
                    reset_metadata: false,
                    reset_artwork: false,
                    expected_metadata_revision: Revision(0),
                }),
                plan,
                analysis: collector_app::IngestAnalysisPolicy::PreserveOnly,
                enrichment: IngestEnrichmentPolicy::None,
                prepare_music: false,
                destination: Some(collector_app::IngestDestination {
                    item_id: original.id,
                    expected_revision: original.revision,
                }),
            },
            &command(),
        )
        .unwrap();
    assert_eq!(
        service.job(&actor, obsolete).unwrap().state,
        JobState::Cancelled
    );
    assert!(service.retry_job(&actor, obsolete, &command()).is_err());
    assert_eq!(service.job(&actor, direct).unwrap().state, JobState::Queued);
    let mut edited = store.item(original.id).unwrap();
    let revision = edited.revision;
    edited.subtitle = "Changed while replacing".into();
    store.save(edited, revision).unwrap();
    fx05_drain(&service);
    assert_eq!(
        service.job(&actor, direct).unwrap().state,
        JobState::Completed
    );
    let replaced = store.item(original.id).unwrap();
    assert_eq!(replaced.subtitle, "Changed while replacing");
    assert_eq!(replaced.captures.len(), 1);
    assert_eq!(
        service
            .ingest_detail(&actor, accepted.imports[0].import_id)
            .unwrap()
            .import
            .phase,
        ImportPhase::Published
    );
    assert_eq!(
        service
            .ingest_detail(&actor, earlier)
            .unwrap()
            .summary
            .outcome,
        collector_app::IngestStepState::Cancelled
    );
}

fn fx05_retained_command(
    service: &CollectionService,
    actor: &Principal,
    item: &Item,
) -> StartIngestCommand {
    let capture = &item.captures[0];
    let representation = &capture.representations[0];
    StartIngestCommand {
        plan: fx05_plan(
            service,
            actor,
            item.collection_id,
            representation
                .files
                .iter()
                .map(|file| UploadFile {
                    name: file.name.clone(),
                    bytes: file.bytes,
                    sha256: Some(file.digest.hex.clone()),
                })
                .collect(),
        ),
        destination: Some(collector_app::IngestDestination {
            item_id: item.id,
            expected_revision: item.revision,
        }),
        clean_reimport: Some(collector_app::CleanReimportOptions {
            replace_captures: vec![capture.id],
            retained_representation: Some(representation.id),
            reset_metadata: false,
            reset_artwork: false,
            expected_metadata_revision: Revision(0),
        }),
        analysis: collector_app::IngestAnalysisPolicy::PreserveOnly,
        enrichment: IngestEnrichmentPolicy::None,
        prepare_music: false,
    }
}

#[test]
fn clean_reimport_checks_retained_bytes_and_retries_after_repair() {
    let (root, store, actor, service, collection) = setup();
    let original = fx05_original(&store, &service, &actor, collection.id);
    let request = fx05_retained_command(&service, &actor, &original);
    let accepted = service.start_ingest(&actor, request, &command()).unwrap();
    let id = accepted.imports[0].import_id;
    let collection_path = std::fs::read_dir(root.path().join("archive/collections"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let capture = &original.captures[0];
    let path = collection_path.join(format!(
        "captures/{}/payload/{}/disc.bin",
        capture.id, capture.representations[0].id
    ));
    std::fs::write(&path, b"evil").unwrap();
    fx05_drain(&service);
    let detail = service.ingest_detail(&actor, id).unwrap();
    assert_eq!(detail.import.phase, ImportPhase::Staging);
    assert!(
        !detail.summary.permissions.can_resume_upload,
        "retained bytes must not ask for a browser upload"
    );
    assert_eq!(store.item(original.id).unwrap().captures[0].id, capture.id);
    let failed = service
        .jobs(&actor)
        .unwrap()
        .into_iter()
        .find(|job| job.kind == "import_staging" && job.state == JobState::Failed)
        .unwrap();
    assert!(failed.error.as_deref().unwrap().contains("digest"));
    std::fs::write(&path, b"data").unwrap();
    service.retry_job(&actor, failed.id, &command()).unwrap();
    fx05_drain(&service);
    assert_eq!(
        service.ingest_detail(&actor, id).unwrap().import.phase,
        ImportPhase::Published
    );
    assert_ne!(store.item(original.id).unwrap().captures[0].id, capture.id);
}

#[test]
fn clean_reimport_admission_receipt_recovers_at_every_item_commit_boundary() {
    use collector_store::PublicationFaultPoint as Point;
    for point in [
        Point::Prepared,
        Point::BytesStaged,
        Point::PayloadPublished,
        Point::RevisionCommitted,
        Point::ProjectionApplied,
        Point::Completed,
    ] {
        let (root, store, actor, service, collection) = setup();
        let original = fx05_original(&store, &service, &actor, collection.id);
        let request = fx05_retained_command(&service, &actor, &original);
        let key = command();
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
        let service = CollectionService::new(store.clone(), Arc::new(Registry::builtins()));
        assert!(
            service.start_ingest(&actor, request.clone(), &key).is_err(),
            "{point:?}"
        );
        drop(service);
        drop(store);
        let store =
            Arc::new(Store::open(root.path().join("archive"), root.path().join("state")).unwrap());
        let service = CollectionService::new(store.clone(), Arc::new(Registry::builtins()));
        let accepted = service.start_ingest(&actor, request.clone(), &key).unwrap();
        assert_eq!(
            store.item(original.id).unwrap().lifecycle.generation,
            1,
            "generation advanced twice at {point:?}"
        );
        fx05_drain(&service);
        assert_eq!(
            service
                .ingest_detail(&actor, accepted.imports[0].import_id)
                .unwrap()
                .import
                .phase,
            ImportPhase::Published,
            "{point:?}"
        );
        store.reindex().unwrap();
        assert_eq!(store.item(original.id).unwrap().captures.len(), 1);
        assert_ne!(
            store.item(original.id).unwrap().captures[0].id,
            original.captures[0].id
        );
        assert_eq!(
            service.start_ingest(&actor, request, &key).unwrap().imports[0].import_id,
            accepted.imports[0].import_id
        );
    }
}

#[test]
fn clean_reimport_replacement_recovers_at_every_capture_commit_boundary() {
    use collector_store::PublicationFaultPoint as Point;
    for point in [
        Point::Prepared,
        Point::BytesStaged,
        Point::PayloadPublished,
        Point::RevisionCommitted,
        Point::ProjectionApplied,
        Point::Completed,
    ] {
        let (root, store, actor, service, collection) = setup();
        let original = fx05_original(&store, &service, &actor, collection.id);
        let accepted = service
            .start_ingest(
                &actor,
                fx05_retained_command(&service, &actor, &original),
                &command(),
            )
            .unwrap();
        let id = accepted.imports[0].import_id;
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
        let service = CollectionService::new(store.clone(), Arc::new(Registry::builtins()));
        for _ in 0..6 {
            service.resume_guided_ingests().unwrap();
            let _ = service.run_next_job("fx05-interrupted");
            if service
                .jobs(&actor)
                .unwrap()
                .iter()
                .any(|job| job.state == JobState::Failed)
            {
                break;
            }
        }
        let interrupted = store.item(original.id).unwrap();
        assert_eq!(
            interrupted.captures.len(),
            1,
            "partial association at {point:?}"
        );
        if interrupted.captures[0].id != original.captures[0].id {
            // Recovery can finish within a later coordinator sweep in this loop.
            // The replacement is visible only with its committed lifecycle switch.
            assert!(
                interrupted.lifecycle.reimport.as_ref().unwrap().completed,
                "premature replacement at {point:?}"
            );
            assert_eq!(
                service.ingest_detail(&actor, id).unwrap().import.phase,
                ImportPhase::Published
            );
        }
        drop(service);
        drop(store);
        let store =
            Arc::new(Store::open(root.path().join("archive"), root.path().join("state")).unwrap());
        let service = CollectionService::new(store.clone(), Arc::new(Registry::builtins()));
        for job in service.jobs(&actor).unwrap() {
            if job.state == JobState::Failed {
                service.retry_job(&actor, job.id, &command()).unwrap();
            }
        }
        fx05_drain(&service);
        assert_eq!(
            service.ingest_detail(&actor, id).unwrap().import.phase,
            ImportPhase::Published,
            "{point:?}"
        );
        let replaced = store.item(original.id).unwrap();
        assert_eq!(replaced.captures.len(), 1);
        assert_ne!(replaced.captures[0].id, original.captures[0].id);
        store.reindex().unwrap();
        assert_eq!(
            store.item(original.id).unwrap().captures[0].id,
            replaced.captures[0].id
        );
    }
}

#[test]
fn clean_reimport_preserves_artwork_by_default_and_resets_only_explicit_authority() {
    let (_root, store, actor, service, collection) = setup();
    let original = fx05_original(&store, &service, &actor, collection.id);
    let mut encoded = std::io::Cursor::new(Vec::new());
    image::RgbImage::from_pixel(24, 24, image::Rgb([12, 34, 56]))
        .write_to(&mut encoded, image::ImageFormat::Png)
        .unwrap();
    let image_bytes = encoded.into_inner();
    let artwork = service
        .create_asset(
            &actor,
            CreateAssetCommand {
                collection_id: collection.id,
                subject: AssetSubject::Item {
                    item_id: original.id,
                },
                kind: AssetKind::Cover,
                visibility: Visibility::Shared,
                alt_text: "Chosen cover".into(),
            },
            &image_bytes,
            &command(),
        )
        .unwrap();
    let authority = service
        .update_metadata_authority(
            &actor,
            original.id,
            UpdateMetadataAuthorityCommand {
                expected_revision: Revision(0),
                change: MetadataAuthorityChange::SetArtworkChoice {
                    asset_id: artwork.id,
                },
            },
            &command(),
        )
        .unwrap();
    let original = store.item(original.id).unwrap();
    let mut request = fx05_retained_command(&service, &actor, &original);
    request
        .clean_reimport
        .as_mut()
        .unwrap()
        .expected_metadata_revision = authority.revision;
    service.start_ingest(&actor, request, &command()).unwrap();
    fx05_drain(&service);
    assert_eq!(
        service
            .metadata_view(&actor, original.id)
            .unwrap()
            .authority
            .artwork_choice,
        Some(artwork.id)
    );
    let mut request = fx05_retained_command(&service, &actor, &store.item(original.id).unwrap());
    let options = request.clean_reimport.as_mut().unwrap();
    options.expected_metadata_revision = authority.revision;
    options.reset_artwork = true;
    service.start_ingest(&actor, request, &command()).unwrap();
    fx05_drain(&service);
    store.reindex().unwrap();
    assert_eq!(
        service
            .metadata_view(&actor, original.id)
            .unwrap()
            .authority
            .artwork_choice,
        None
    );
    let late_art = ContentPort::create_asset(
        store.as_ref(),
        AssetId::new(),
        CreateAssetCommand {
            collection_id: collection.id,
            subject: AssetSubject::Item {
                item_id: original.id,
            },
            kind: AssetKind::Cover,
            visibility: Visibility::Shared,
            alt_text: "Late old provider image".into(),
        },
        &image_bytes,
        collector_app::AssetWriteContext::Item(collector_app::ItemEffectContext {
            item_id: original.id,
            generation: original.lifecycle.generation,
            source: collector_app::ItemEffectSource::Command,
        }),
    );
    assert!(
        matches!(late_art, Err(Error::Conflict)),
        "late artwork crossed the writer generation fence"
    );
    assert!(
        service
            .asset(&actor, original.id, artwork.id, false)
            .is_ok(),
        "reset deleted artwork bytes"
    );
}

#[test]
fn clean_reimport_rejects_prior_generation_snapshot_completion() {
    let (_root, store, actor, service, collection) = setup();
    let original = fx05_original(&store, &service, &actor, collection.id);
    let id = JobId::new();
    let snapshot = SnapshotId::new();
    store
        .enqueue_job(JobSpec {
            id,
            user_id: actor.user_id,
            collection_id: Some(collection.id),
            item_id: Some(original.id),
            kind: "provider_acquisition".into(),
            phase: "queued".into(),
            total_units: 1,
            request: serde_json::json!({}),
            frozen_input: None,
        })
        .unwrap();
    let claim = store.claim_job("provider-fixture", 30).unwrap().unwrap();
    assert_eq!(claim.id, id);
    store
        .complete_job(
            id,
            &claim.claim_token,
            serde_json::json!({"completion":{"snapshot_id":snapshot}}),
        )
        .unwrap();
    store
        .ensure_snapshot_current(original.id, snapshot)
        .unwrap();
    service
        .start_ingest(
            &actor,
            fx05_retained_command(&service, &actor, &original),
            &command(),
        )
        .unwrap();
    assert!(matches!(
        store.ensure_snapshot_current(original.id, snapshot),
        Err(Error::Conflict)
    ));
    store.reindex().unwrap();
    assert!(matches!(
        store.ensure_snapshot_current(original.id, snapshot),
        Err(Error::Conflict)
    ));
    // Settle the reimport's real queued work before admitting the independent
    // binding fixture; its ordering is not part of this freshness assertion.
    fx05_drain(&service);
    // The same retained response may be consumed by a current generation. Older
    // bindings remain evidence and must not veto the new requester.
    let current_job = JobId::new();
    store
        .enqueue_job(JobSpec {
            id: current_job,
            user_id: actor.user_id,
            collection_id: Some(collection.id),
            item_id: Some(original.id),
            kind: "test.snapshot_binding".into(),
            phase: "queued".into(),
            total_units: 1,
            request: serde_json::json!({}),
            frozen_input: None,
        })
        .unwrap();
    let current_claim = store.claim_job("provider-fixture", 30).unwrap().unwrap();
    assert_eq!(current_claim.id, current_job);
    store
        .complete_job(
            current_job,
            &current_claim.claim_token,
            serde_json::json!({"snapshot_id":snapshot}),
        )
        .unwrap();
    store
        .ensure_snapshot_current(original.id, snapshot)
        .unwrap();
    store.reindex().unwrap();
    store
        .ensure_snapshot_current(original.id, snapshot)
        .unwrap();
}

#[test]
fn clean_reimport_supersedes_a_partially_committed_replacement_without_resurrection() {
    let (root, store, actor, service, collection) = setup();
    let original = fx05_original(&store, &service, &actor, collection.id);
    let first = service
        .start_ingest(
            &actor,
            fx05_retained_command(&service, &actor, &original),
            &command(),
        )
        .unwrap()
        .imports[0]
        .clone();
    service.run_next_job("stage-first").unwrap();
    drop(service);
    drop(store);
    let store = Arc::new(
        Store::open_with_publication_fault(
            root.path().join("archive"),
            root.path().join("state"),
            Arc::new(PanicAtImportCommit),
        )
        .unwrap(),
    );
    let claim = store.claim_job("interrupted-process", 30).unwrap().unwrap();
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            store.publish_import_for_job(first.import_id, claim.id, &claim.claim_token, &mut || {
                Ok(())
            })
        }))
        .is_err()
    );
    drop(store);
    let store =
        Arc::new(Store::open(root.path().join("archive"), root.path().join("state")).unwrap());
    let service = CollectionService::new(store.clone(), Arc::new(Registry::builtins()));
    let pending = store.item(original.id).unwrap();
    assert!(!pending.lifecycle.reimport.as_ref().unwrap().completed);
    assert_eq!(pending.captures[0].id, original.captures[0].id);
    let second = service
        .start_ingest(
            &actor,
            fx05_retained_command(&service, &actor, &pending),
            &command(),
        )
        .unwrap()
        .imports[0]
        .clone();
    let during = store.item(original.id).unwrap();
    assert_eq!(during.captures.len(), 1);
    assert_eq!(during.captures[0].id, original.captures[0].id);
    fx05_drain(&service);
    store.reindex().unwrap();
    let finished = store.item(original.id).unwrap();
    assert_eq!(finished.captures.len(), 1);
    assert_eq!(finished.captures[0].id, second.capture_id);
    assert_ne!(finished.captures[0].id, first.capture_id);
    assert!(
        service
            .representation(
                &actor,
                original.id,
                original.captures[0].representations[0].id,
                0
            )
            .is_ok()
    );
}

fn snapshot_claim(store: &Store, collection: CollectionId) -> collector_app::JobClaim {
    store
        .enqueue_job(JobSpec {
            id: JobId::new(),
            user_id: UserId::new(),
            collection_id: Some(collection),
            item_id: None,
            kind: "test.snapshot".into(),
            phase: "queued".into(),
            total_units: 1,
            request: serde_json::json!({}),
            frozen_input: None,
        })
        .unwrap();
    store.claim_job("snapshot-fixture", 300).unwrap().unwrap()
}

#[test]
fn snapshot_publication_rejects_foreign_and_cancelled_claims_without_trusting_callback() {
    let (_root, store, _actor, _service, collection) = setup();
    let claim = snapshot_claim(&store, collection.id);
    let snapshot = SnapshotRecord {
        acquisition_key: None,
        id: SnapshotId::new(),
        collection_id: collection.id,
        provider: ModuleId("org.example.catalog".into()),
        media_type: "application/octet-stream".into(),
        bytes: ByteLength(4),
        digest: Digest::sha256(b"data"),
        parsed_observation: RawJson::from_value(serde_json::json!({})),
        created_at: chrono::Utc::now(),
    };
    let mut wrong = claim.clone();
    wrong.claim_token = "obsolete-claim".into();
    assert!(
        store
            .publish_snapshot(
                &wrong,
                snapshot.clone(),
                &mut std::io::Cursor::new(b"data"),
                &mut || Ok(false)
            )
            .is_err()
    );
    let mut wrong = claim.clone();
    wrong.collection_id = Some(CollectionId::new());
    assert!(matches!(
        store.publish_snapshot(
            &wrong,
            snapshot.clone(),
            &mut std::io::Cursor::new(b"data"),
            &mut || Ok(false)
        ),
        Err(Error::Conflict)
    ));
    let mut wrong = claim.clone();
    wrong.attempt += 1;
    assert!(matches!(
        store.publish_snapshot(
            &wrong,
            snapshot.clone(),
            &mut std::io::Cursor::new(b"data"),
            &mut || Ok(false)
        ),
        Err(Error::Conflict)
    ));
    store
        .request_job_cancellation(claim.id, claim.user_id)
        .unwrap();
    assert!(matches!(
        store.publish_snapshot(
            &claim,
            snapshot.clone(),
            &mut std::io::Cursor::new(b"data"),
            &mut || Ok(false)
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
}

#[test]
fn derived_asset_publication_requires_its_live_source_and_exact_item() {
    let (_root, store, actor, service, collection) = setup();
    let original = fx05_original(&store, &service, &actor, collection.id);
    let accepted = service
        .start_ingest(
            &actor,
            fx05_retained_command(&service, &actor, &original),
            &command(),
        )
        .unwrap();
    let import = accepted.imports[0].import_id;
    let item = store.item(original.id).unwrap();
    let detail = service.ingest_detail(&actor, import).unwrap();
    service
        .cancel_ingest(&actor, import, detail.import.revision, &command())
        .unwrap();
    let mut encoded = std::io::Cursor::new(Vec::new());
    image::RgbImage::from_pixel(4, 4, image::Rgb([12, 34, 56]))
        .write_to(&mut encoded, image::ImageFormat::Png)
        .unwrap();
    for source in [
        collector_app::ItemEffectSource::GuidedImport(import),
        collector_app::ItemEffectSource::Snapshot(SnapshotId::new()),
    ] {
        let asset = AssetId::new();
        let result = ContentPort::create_asset(
            store.as_ref(),
            asset,
            CreateAssetCommand {
                collection_id: collection.id,
                subject: AssetSubject::Item { item_id: item.id },
                kind: AssetKind::Cover,
                visibility: Visibility::Shared,
                alt_text: "Obsolete source".into(),
            },
            encoded.get_ref(),
            collector_app::AssetWriteContext::Item(collector_app::ItemEffectContext {
                item_id: item.id,
                generation: item.lifecycle.generation,
                source,
            }),
        );
        assert!(
            matches!(result, Err(Error::Conflict)),
            "generation alone admitted {source:?}"
        );
        assert!(matches!(store.asset_state(asset), Err(Error::NotFound)));
    }
    let result = ContentPort::create_asset(
        store.as_ref(),
        AssetId::new(),
        CreateAssetCommand {
            collection_id: collection.id,
            subject: AssetSubject::Item { item_id: item.id },
            kind: AssetKind::Cover,
            visibility: Visibility::Shared,
            alt_text: "Wrong owner".into(),
        },
        encoded.get_ref(),
        collector_app::AssetWriteContext::Item(collector_app::ItemEffectContext {
            item_id: ItemId::new(),
            generation: item.lifecycle.generation,
            source: collector_app::ItemEffectSource::Command,
        }),
    );
    assert!(matches!(result, Err(Error::Conflict)));
}

#[test]
fn checkpoint_reuses_completed_writes_and_rebuilds_mutated_projection() {
    let (root, store, actor, service, collection) = setup();
    let saved = save_inventory(&service, &actor, item(collection.id), Revision(0)).unwrap();
    drop(service);
    drop(store);
    let reopened = Store::open(root.path().join("archive"), root.path().join("state")).unwrap();
    assert!(reopened.catalog_reused_at_startup());
    let expected = serde_json::to_value(reopened.item(saved.id).unwrap()).unwrap();
    reopened.reindex().unwrap();
    assert_eq!(
        serde_json::to_value(reopened.item(saved.id).unwrap()).unwrap(),
        expected
    );
    drop(reopened);
    let db = rusqlite::Connection::open(root.path().join("state/catalog.sqlite")).unwrap();
    db.execute("DELETE FROM items", []).unwrap();
    drop(db);
    let reopened = Store::open(root.path().join("archive"), root.path().join("state")).unwrap();
    assert!(!reopened.catalog_reused_at_startup());
    assert_eq!(
        serde_json::to_value(reopened.item(saved.id).unwrap()).unwrap(),
        expected
    );
}

#[test]
fn checkpoint_missing_malformed_and_incomplete_projection_rebuild_without_losing_access() {
    for damage in [
        "DELETE FROM catalog_checkpoint",
        "UPDATE catalog_checkpoint SET format=999",
        "DROP TABLE entity_owners",
        "DROP TABLE collection_lists",
        "DROP TRIGGER checkpoint_collection_lists_INSERT",
        "DROP TABLE items",
        "DROP TRIGGER references_items_INSERT",
    ] {
        let (root, store, actor, service, collection) = setup();
        let saved = save_inventory(&service, &actor, item(collection.id), Revision(0)).unwrap();
        drop(service);
        drop(store);
        let db = rusqlite::Connection::open(root.path().join("state/catalog.sqlite")).unwrap();
        db.execute_batch(damage).unwrap();
        drop(db);
        let reopened = Store::open(root.path().join("archive"), root.path().join("state")).unwrap();
        assert!(!reopened.catalog_reused_at_startup(), "{damage}");
        assert_eq!(reopened.item(saved.id).unwrap().title, saved.title);
        let service = CollectionService::new(Arc::new(reopened), Arc::new(Registry::builtins()));
        assert!(service.item_presented(&actor, saved.id).is_ok());
    }
}

#[test]
fn checkpoint_corrupt_catalog_is_retained_and_private_access_survives() {
    let (root, store, actor, service, collection) = setup();
    let saved = save_inventory(&service, &actor, item(collection.id), Revision(0)).unwrap();
    drop(service);
    drop(store);
    std::fs::write(
        root.path().join("state/catalog.sqlite"),
        b"damaged projection",
    )
    .unwrap();
    let reopened = Store::open(root.path().join("archive"), root.path().join("state")).unwrap();
    assert!(!reopened.catalog_reused_at_startup());
    assert!(
        std::fs::read_dir(root.path().join("state"))
            .unwrap()
            .any(|entry| entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with("catalog.sqlite.corrupt-"))
    );
    let service = CollectionService::new(Arc::new(reopened), Arc::new(Registry::builtins()));
    assert!(service.item_presented(&actor, saved.id).is_ok());
}

#[test]
fn checkpoint_detects_restored_archive_and_rejects_changed_metadata() {
    let (root, store, actor, service, collection) = setup();
    let saved = save_inventory(&service, &actor, item(collection.id), Revision(0)).unwrap();
    let old_records = files_named(&root.path().join("archive"), "record.v3.json");
    let old_revision = old_records[0].parent().unwrap().to_path_buf();
    let mut changed = saved.clone();
    changed.title = "Later title".into();
    save_inventory(&service, &actor, changed, saved.revision).unwrap();
    drop(service);
    drop(store);
    let new_record = files_named(&root.path().join("archive"), "record.v3.json")
        .into_iter()
        .find(|path| path.parent().unwrap() != old_revision)
        .unwrap();
    std::fs::remove_dir_all(new_record.parent().unwrap()).unwrap();
    let reopened = Store::open(root.path().join("archive"), root.path().join("state")).unwrap();
    assert!(!reopened.catalog_reused_at_startup());
    assert_eq!(reopened.item(saved.id).unwrap().title, saved.title);
    drop(reopened);
    std::fs::write(old_records[0].clone(), b"changed bytes").unwrap();
    assert!(Store::open(root.path().join("archive"), root.path().join("state")).is_err());
}

/// Reproducible, offline scale experiment; deliberately excluded from ordinary tests.
#[test]
#[ignore = "generated scale timings; run explicitly with --ignored --nocapture"]
fn checkpoint_generated_scale_measurements() {
    use std::time::Instant;
    fn percentile(mut times: Vec<f64>, percentile: usize) -> f64 {
        times.sort_by(f64::total_cmp);
        times[(times.len() - 1) * percentile / 100]
    }
    for count in [100, 1_000, 10_000] {
        let (root, store, actor, service, collection) = setup();
        let saved = save_inventory(&service, &actor, item(collection.id), Revision(0)).unwrap();
        let record_path = files_named(&root.path().join("archive"), "record.v3.json").remove(0);
        let record: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&record_path).unwrap()).unwrap();
        let manifest: toml::Value = toml::from_str(
            &std::fs::read_to_string(record_path.with_file_name("manifest.v3.toml")).unwrap(),
        )
        .unwrap();
        let items_dir = record_path.ancestors().nth(4).unwrap();
        for position in 1..count {
            let id = ItemId::new();
            let transaction = TransactionId::new();
            let mut record = record.clone();
            record["record"]["id"] = serde_json::json!(id);
            record["record"]["title"] = serde_json::json!(format!("Generated album {position:06}"));
            record["transaction_id"] = serde_json::json!(transaction);
            let bytes = serde_json::to_vec_pretty(&record).unwrap();
            let mut manifest = manifest.clone();
            manifest["entity_id"] = toml::Value::String(id.to_string());
            manifest["transaction_id"] = toml::Value::String(transaction.to_string());
            manifest["record_sha256"] = toml::Value::String(Digest::sha256(&bytes).hex);
            let dir = items_dir.join(format!("generated--{id}/revisions/00000000000000000001"));
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("record.v3.json"), bytes).unwrap();
            std::fs::write(
                dir.join("manifest.v3.toml"),
                toml::to_string(&manifest).unwrap(),
            )
            .unwrap();
        }
        let started = Instant::now();
        store.reindex().unwrap();
        let rebuild = started.elapsed().as_secs_f64() * 1000.0;
        let catalog = rusqlite::Connection::open(root.path().join("state/catalog.sqlite")).unwrap();
        assert_eq!(
            catalog
                .query_row("SELECT COUNT(*) FROM items", [], |row| row
                    .get::<_, usize>(0))
                .unwrap(),
            count
        );
        drop(catalog);
        let mut writes = vec![];
        let mut queries = vec![];
        let mut current = saved;
        for iteration in 0..30 {
            current.title = format!("Edited album {iteration}");
            let started = Instant::now();
            let revision = current.revision;
            current = save_inventory(&service, &actor, current, revision).unwrap();
            writes.push(started.elapsed().as_secs_f64() * 1000.0);
            let started = Instant::now();
            service
                .browse_presented(
                    &actor,
                    BrowseQuery {
                        collection_id: Some(collection.id),
                        limit: Some(50),
                        ..Default::default()
                    },
                )
                .unwrap();
            queries.push(started.elapsed().as_secs_f64() * 1000.0);
        }
        drop(service);
        drop(store);
        let mut startups = vec![];
        for _ in 0..5 {
            let started = Instant::now();
            let reopened =
                Store::open(root.path().join("archive"), root.path().join("state")).unwrap();
            startups.push(started.elapsed().as_secs_f64() * 1000.0);
            assert!(reopened.catalog_reused_at_startup());
            drop(reopened);
        }
        println!(
            "items={count} full_rebuild_ms={rebuild:.1} startup_p50_ms={:.1} startup_p95_ms={:.1} edit_p50_ms={:.1} edit_p95_ms={:.1} browse_p50_ms={:.1} browse_p95_ms={:.1}",
            percentile(startups.clone(), 50),
            percentile(startups, 95),
            percentile(writes.clone(), 50),
            percentile(writes, 95),
            percentile(queries.clone(), 50),
            percentile(queries, 95)
        );
    }
}

#[test]
fn durable_metadata_resolution_preserves_language_receipts_manual_fields_and_reopens() {
    let (root, store, actor, service, collection) = setup();
    let saved = save_inventory(&service, &actor, item(collection.id), Revision(0)).unwrap();
    let imported = derivative_import(&service, &actor, collection.id);
    let mut capture = store.capture(imported.capture_id).unwrap();
    let revision = capture.revision;
    capture.association.item_id = Some(saved.id);
    store.save_capture(capture, revision).unwrap();
    let disc_id = service
        .metadata_review(&actor, saved.id)
        .unwrap()
        .lookup_choices
        .into_iter()
        .find(|choice| choice.kind == collector_app::MetadataLookupKind::DiscId)
        .unwrap()
        .value;
    let group = "11111111-1111-4111-8111-111111111111";
    let body=serde_json::to_vec(&serde_json::json!({"count":2,"releases":[
        {"id":"22222222-2222-4222-8222-222222222222","title":"Album","text-representation":{"language":"eng","script":"Latn"},"release-group":{"id":group},"media":[{"position":1,"discs":[{"id":disc_id}]}]},
        {"id":"33333333-3333-4333-8333-333333333333","title":"アルバム","text-representation":{"language":"jpn","script":"Jpan"},"release-group":{"id":group},"media":[{"position":1,"discs":[{"id":disc_id}]}]}
    ]})).unwrap();
    let fixture = metadata_fixture_server(vec![("200 OK\r\nContent-Type: application/json", body)]);
    let provider = Arc::new(
        MusicBrainzProvider::with_test_origin(
            "collector-test/0.1 (test@example.invalid)",
            &fixture.origin,
            Duration::from_secs(1),
            Duration::ZERO,
            Duration::from_secs(60),
            Duration::from_secs(60),
        )
        .unwrap(),
    );
    let service = service
        .with_streaming_provider(
            ModuleId(collector_modules::music::MUSICBRAINZ_PROVIDER_ID.into()),
            Arc::new(collector_app::BoundedProviderAdapter(provider.clone())),
        )
        .unwrap()
        .with_catalog_parser(
            ModuleId(collector_modules::music::MUSICBRAINZ_PROVIDER_ID.into()),
            provider.clone(),
        )
        .unwrap();
    assert_eq!(
        service
            .metadata_preferences(&actor)
            .unwrap()
            .language
            .as_str(),
        "en"
    );
    let preference_command = command();
    let change = collector_app::UpdateMetadataPreferences {
        expected_revision: Revision(0),
        language: LanguageTag::new("ja").unwrap(),
    };
    let preferences = service
        .update_metadata_preferences(&actor, change.clone(), &preference_command)
        .unwrap();
    assert_eq!(
        service
            .update_metadata_preferences(&actor, change, &preference_command)
            .unwrap()
            .revision,
        preferences.revision
    );
    assert!(matches!(
        service.update_metadata_preferences(
            &actor,
            collector_app::UpdateMetadataPreferences {
                expected_revision: Revision(0),
                language: LanguageTag::new("fr").unwrap()
            },
            &command()
        ),
        Err(Error::Conflict)
    ));
    service
        .update_metadata_authority(
            &actor,
            saved.id,
            UpdateMetadataAuthorityCommand {
                expected_revision: Revision(0),
                change: MetadataAuthorityChange::ClearFieldOverride {
                    field: MetadataField::Title,
                },
            },
            &command(),
        )
        .unwrap();
    let request = collector_app::ResolveMetadataCommand {
        lookup: collector_app::AcquireMetadataCommand::MusicBrainzDisc {
            disc_id: disc_id.clone(),
            refresh: true,
        },
        acquire_artwork: true,
    };
    let receipt = command();
    let job = service
        .resolve_metadata(&actor, saved.id, request.clone(), &receipt)
        .unwrap();
    service
        .update_metadata_preferences(
            &actor,
            collector_app::UpdateMetadataPreferences {
                expected_revision: preferences.revision,
                language: LanguageTag::new("en").unwrap(),
            },
            &command(),
        )
        .unwrap();
    assert_eq!(
        service
            .resolve_metadata(&actor, saved.id, request.clone(), &receipt)
            .unwrap(),
        job
    );
    assert!(matches!(
        service.resolve_metadata(
            &actor,
            saved.id,
            collector_app::ResolveMetadataCommand {
                acquire_artwork: false,
                ..request.clone()
            },
            &receipt
        ),
        Err(Error::IdempotencyConflict)
    ));
    let direct = service
        .enqueue_musicbrainz_metadata_acquisition(
            &actor,
            saved.id,
            collector_modules::music::MusicBrainzQuery::DiscId {
                disc_id: disc_id.clone(),
                refresh: false,
            },
            &command(),
        )
        .unwrap();
    assert!(matches!(
        service.job(&actor, direct).unwrap().provider_status,
        Some(collector_app::ProviderResponseStatus::Shared)
    ));
    provider_drain(&service);
    let direct_snapshot = completed_provider_snapshot(&service, &actor, direct);
    assert_eq!(
        service
            .catalog_snapshots(&actor, collection.id, None, 100)
            .unwrap()
            .items
            .len(),
        1
    );

    let resolution = service
        .metadata_resolutions(&actor, saved.id)
        .unwrap()
        .into_iter()
        .find(|value| value.job.id == job)
        .unwrap();
    assert_eq!(resolution.job.state, JobState::Completed);
    assert!(
        resolution.result.unwrap().artwork_error.is_some(),
        "unconfigured artwork must not roll back successful text"
    );
    assert!(resolution.job.retryable);
    let authority = store
        .metadata_authority(collection.id, saved.id)
        .unwrap()
        .unwrap();
    assert_eq!(
        authority.authority.automatic_language.unwrap().as_str(),
        "ja",
        "accepted inputs remain frozen after preference changes"
    );
    assert_eq!(authority.display.as_ref().unwrap().title.value, "アルバム");
    let repeated = service
        .refresh_provider_metadata(
            &actor,
            saved.id,
            RefreshProviderMetadataCommand {
                expected_revision: authority.revision,
                snapshot_id: direct_snapshot,
                source_observation_id: None,
                artwork: vec![],
            },
            &command(),
        )
        .unwrap();
    // Current language preferences changed after admission, so this resolution is meaningful.
    assert!(repeated.revision > authority.revision);
    let unchanged = service
        .refresh_provider_metadata(
            &actor,
            saved.id,
            RefreshProviderMetadataCommand {
                expected_revision: repeated.revision,
                snapshot_id: direct_snapshot,
                source_observation_id: None,
                artwork: vec![],
            },
            &command(),
        )
        .unwrap();
    assert_eq!(unchanged.revision, repeated.revision);
    assert_eq!(unchanged.observations.len(), repeated.observations.len());

    let review = service.metadata_review(&actor, saved.id).unwrap();
    assert_eq!(
        review.display.title.value, "Album",
        "presentation follows the current user's language within the accepted album"
    );
    assert!(
        !review
            .review_reasons
            .contains(&collector_app::MetadataReviewReason::AlbumAmbiguity)
    );
    assert!(review.authority.selected_candidate.is_none());
    assert!(review.authority.automatic_candidate.is_some());
    store.reindex().unwrap();
    assert_eq!(
        service
            .metadata_review(&actor, saved.id)
            .unwrap()
            .display
            .title
            .value,
        "Album"
    );
    drop(service);
    drop(store);
    let store =
        Arc::new(Store::open(root.path().join("archive"), root.path().join("state")).unwrap());
    let service = CollectionService::new(store.clone(), Arc::new(Registry::builtins()));
    assert_eq!(
        service
            .metadata_preferences(&actor)
            .unwrap()
            .language
            .as_str(),
        "en"
    );
    assert_eq!(
        service.metadata_resolutions(&actor, saved.id).unwrap()[0]
            .job
            .id,
        job
    );
    let stranger = Principal {
        user_id: store
            .create_user("care-stranger", "a long stranger password", false)
            .unwrap(),
        administrator: false,
        device_id: None,
    };
    assert!(matches!(
        service.metadata_resolutions(&stranger, saved.id),
        Err(Error::Forbidden)
    ));
    let query = collector_app::MetadataCareQuery {
        collection_id: Some(collection.id),
        search: String::new(),
        reason: None,
        needs_attention: true,
        cursor: None,
        limit: Some(1),
    };
    assert_eq!(
        service
            .metadata_care(&actor, query.clone())
            .unwrap()
            .items
            .len(),
        1
    );
    assert!(matches!(
        service.metadata_care(&stranger, query),
        Err(Error::Forbidden)
    ));
}

#[test]
fn album_group_commit_preserves_distinct_inventory_and_guarded_undo() {
    use collector_app::{AlbumGroupChoices, ConfirmAlbumGroup, GroupEntry, PreviewAlbumGroup};
    let (_root, store, actor, service, collection) = setup();
    let mut left = item(collection.id);
    let mut right = item(collection.id);
    for (inventory, position) in [(&mut left, 1), (&mut right, 2)] {
        inventory.wishlist = false;
        inventory.specimens.push(Specimen {
            id: SpecimenId::new(),
            label: format!("Copy {position}"),
            condition: Condition::Good,
            ownership: Ownership::Owned,
            possession: Possession::Present,
            components: vec![Component {
                id: ComponentId::new(),
                label: format!("Disc {position}"),
                kind: ComponentKind::OpticalDisc,
                position,
                carrier: Some(Carrier {
                    id: CarrierId::new(),
                    kind: CarrierKind::OpticalDisc,
                    label: String::new(),
                }),
                migration_provenance: None,
            }],
            acquisition: None,
            private_notes: Some("Preserved private note".into()),
            loans: vec![],
        });
    }
    let left = Catalog::save(&*store, left, Revision(0)).unwrap();
    let right = Catalog::save(&*store, right, Revision(0)).unwrap();
    let upload = service
        .create_upload(
            &actor,
            right.id,
            right.revision,
            vec![UploadFile {
                name: "original.bin".into(),
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
    let right = store.item(right.id).unwrap();
    let original_evidence = store.evidence(right.id).unwrap();
    assert!(!original_evidence.is_empty());
    let request = PreviewAlbumGroup {
        destination: left.id,
        entries: vec![
            GroupEntry {
                item_id: left.id,
                expected_revision: left.revision,
            },
            GroupEntry {
                item_id: right.id,
                expected_revision: right.revision,
            },
        ],
        choices: AlbumGroupChoices::default(),
    };
    let preview = service
        .preview_album_group(&actor, request.clone())
        .unwrap();
    assert!(preview.can_confirm, "{:?}", preview.conflicts);
    let context = command();
    let merged = service
        .confirm_album_group(
            &actor,
            ConfirmAlbumGroup {
                preview: request.clone(),
                token: preview.token.clone(),
            },
            &context,
        )
        .unwrap();
    store.reindex().unwrap();
    assert_eq!(
        store.evidence(right.id).unwrap().len(),
        original_evidence.len()
    );
    assert_eq!(store.item(left.id).unwrap().captures.len(), 1);
    assert_eq!(
        service
            .confirm_album_group(
                &actor,
                ConfirmAlbumGroup {
                    preview: request,
                    token: preview.token
                },
                &context
            )
            .unwrap()
            .group_id,
        merged.group_id
    );
    let destination = Catalog::item(&*store, left.id).unwrap();
    assert_eq!(destination.specimens.len(), 2);
    assert_eq!(destination.specimens[0].id, left.specimens[0].id);
    assert_eq!(
        destination.specimens[1].components[0]
            .carrier
            .as_ref()
            .unwrap()
            .id,
        right.specimens[0].components[0]
            .carrier
            .as_ref()
            .unwrap()
            .id
    );
    let source = Catalog::item(&*store, right.id).unwrap();
    assert!(source.deleted);
    assert_eq!(source.lifecycle.merged_into, Some(left.id));
    assert!(source.specimens.is_empty());
    store.reindex().unwrap();
    assert_eq!(
        serde_json::to_string(&Catalog::item(&*store, left.id).unwrap()).unwrap(),
        serde_json::to_string(&destination).unwrap()
    );
    assert!(
        service
            .album_group_undo(&actor, merged.group_id)
            .unwrap()
            .can_undo
    );
    service
        .undo_album_group(&actor, merged.group_id, &command())
        .unwrap();
    assert_eq!(Catalog::item(&*store, left.id).unwrap().specimens.len(), 1);
    assert_eq!(
        Catalog::item(&*store, right.id).unwrap().specimens[0].id,
        right.specimens[0].id
    );
    assert!(
        !service
            .album_group_undo(&actor, merged.group_id)
            .unwrap()
            .can_undo
    );
    store.reindex().unwrap();
    assert!(!Catalog::item(&*store, right.id).unwrap().deleted);
}

#[test]
fn album_group_publication_faults_recover_all_members_or_none() {
    use collector_app::{AlbumGroupChoices, ConfirmAlbumGroup, GroupEntry, PreviewAlbumGroup};
    use collector_store::PublicationFaultPoint as Point;
    for point in publication_points() {
        let (root, store, actor, service, collection) = setup();
        let left = Catalog::save(&*store, item(collection.id), Revision(0)).unwrap();
        let right = Catalog::save(&*store, item(collection.id), Revision(0)).unwrap();
        drop(service);
        drop(store);
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
        let service = CollectionService::new(store.clone(), Arc::new(Registry::builtins()));
        let request = PreviewAlbumGroup {
            destination: left.id,
            entries: vec![
                GroupEntry {
                    item_id: left.id,
                    expected_revision: left.revision,
                },
                GroupEntry {
                    item_id: right.id,
                    expected_revision: right.revision,
                },
            ],
            choices: AlbumGroupChoices::default(),
        };
        let preview = service
            .preview_album_group(&actor, request.clone())
            .unwrap();
        let context = command();
        assert!(
            service
                .confirm_album_group(
                    &actor,
                    ConfirmAlbumGroup {
                        preview: request.clone(),
                        token: preview.token.clone()
                    },
                    &context
                )
                .is_err(),
            "{point:?}"
        );
        let committed = matches!(
            point,
            Point::RevisionCommitted | Point::ProjectionApplied | Point::Completed
        );
        if !committed {
            store.reindex().unwrap();
            assert_eq!(
                Catalog::item(&*store, left.id).unwrap().revision,
                left.revision
            );
            assert!(!Catalog::item(&*store, right.id).unwrap().deleted);
        }
        drop(service);
        drop(store);
        let reopened = Arc::new(Store::open(&archive, &state).unwrap());
        assert_eq!(
            Catalog::item(&*reopened, left.id).unwrap().revision,
            Revision(if committed { 2 } else { 1 }),
            "{point:?}"
        );
        assert_eq!(
            Catalog::item(&*reopened, right.id).unwrap().deleted,
            committed,
            "{point:?}"
        );
        let service = CollectionService::new(reopened.clone(), Arc::new(Registry::builtins()));
        let result = service
            .confirm_album_group(
                &actor,
                ConfirmAlbumGroup {
                    preview: request,
                    token: preview.token,
                },
                &context,
            )
            .unwrap();
        assert!(
            service
                .album_group_undo(&actor, result.group_id)
                .unwrap()
                .can_undo,
            "{point:?}"
        );
        assert!(
            std::fs::read_dir(archive.join(".transactions"))
                .unwrap()
                .next()
                .is_none()
        );
    }
}

#[test]
fn bulk_intake_is_paged_durable_and_preconfirmed_groups_share_one_album() {
    use collector_app::{
        AppendIntake, CreateIntake, IntakeDestination, IntakePackage, StartIntake,
    };
    let (root, store, actor, service, collection) = setup();
    let mut session = service
        .create_intake(
            &actor,
            CreateIntake {
                collection_id: collection.id,
                expected_packages: 70,
                expected_files: 70,
                automatic_metadata: false,
                prepare_music: false,
            },
            &command(),
        )
        .unwrap();
    let packages = (0..70)
        .map(|position| IntakePackage {
            package: IngestPackageSelection {
                source: ImportSourceSummary {
                    kind: ImportSourceKind::Browser,
                    label: None,
                    package_name: format!("Disc {position}"),
                },
                files: vec![UploadFile {
                    name: "original.bin".into(),
                    bytes: ByteLength(0),
                    sha256: None,
                }],
                anchor_file_index: None,
            },
            destination: if position < 2 {
                IntakeDestination::AlbumGroup {
                    key: "two-disc-album".into(),
                    title: "A confirmed album".into(),
                    copy_key: "one set".into(),
                }
            } else {
                IntakeDestination::Separate
            },
        })
        .collect::<Vec<_>>();
    for page in packages.chunks(35) {
        session = service
            .append_intake_with_preflight(
                &actor,
                session.id,
                AppendIntake {
                    expected_revision: session.revision,
                    packages: page.to_vec(),
                },
                &command(),
                |_| Ok(()),
            )
            .unwrap();
    }
    assert_eq!(session.package_count, 70);
    assert_eq!(session.file_count, 70);
    let page = service
        .intake_members(&actor, session.id, None, 30)
        .unwrap();
    assert_eq!(page.items.len(), 30);
    assert!(page.next_cursor.is_some());
    let context = command();
    let start = StartIntake {
        expected_revision: session.revision,
    };
    session = service
        .start_intake(&actor, session.id, start.clone(), &context)
        .unwrap();
    assert_eq!(
        service
            .start_intake(&actor, session.id, start, &context)
            .unwrap()
            .id,
        session.id
    );
    assert!(service.run_next_job("bulk-intake-admission").unwrap());
    assert_eq!(service.intake(&actor, session.id).unwrap().admitted, 70);
    service.resume_guided_ingests().unwrap();
    let members = service
        .intake_members(&actor, session.id, None, 100)
        .unwrap();
    assert_eq!(members.items.len(), 70);
    let first = store
        .guided_ingest_intent(members.items[0].import_id.unwrap())
        .unwrap()
        .unwrap();
    let second = store
        .guided_ingest_intent(members.items[1].import_id.unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(first.target_item_id, second.target_item_id);
    assert_eq!(
        first.bulk_context.as_ref().unwrap().copy_id,
        second.bulk_context.as_ref().unwrap().copy_id
    );
    assert_eq!(
        Catalog::item(&*store, first.target_item_id)
            .unwrap()
            .specimens
            .len(),
        1
    );
    drop(service);
    drop(store);
    let store =
        Arc::new(Store::open(root.path().join("archive"), root.path().join("state")).unwrap());
    let service = CollectionService::new(store.clone(), Arc::new(Registry::builtins()));
    assert_eq!(
        service
            .intake_members(&actor, session.id, Some(59), 30)
            .unwrap()
            .items
            .len(),
        10
    );
    assert_eq!(service.intake(&actor, session.id).unwrap().admitted, 70);
    let stranger = Principal {
        user_id: UserId::new(),
        administrator: false,
        device_id: None,
    };
    assert!(matches!(
        service.intake(&stranger, session.id),
        Err(Error::NotFound)
    ));
    service
        .cancel_intake(&actor, session.id, &command())
        .unwrap();
    service.resume_bulk_intakes().unwrap();
    assert!(matches!(
        service.intake(&actor, session.id).unwrap().state,
        collector_app::IntakeState::Cancelled
    ));
    for member in members.items {
        assert_eq!(
            store
                .import_record(member.import_id.unwrap())
                .unwrap()
                .view
                .phase,
            ImportPhase::Cancelled
        );
    }
}

#[test]
fn bulk_preparation_freezes_filter_membership_reuses_outputs_and_retries_only_failed_albums() {
    use collector_app::{
        BulkLibraryKind, BulkLibraryState, LibrarySelection, MetadataCareQuery, PreviewBulkLibrary,
    };
    let (root, store, actor, service, collection) = setup();
    let encoder = Arc::new(DerivativeFixtureEncoder::new());
    let service = service.with_audio_converter(encoder.clone());
    let mut albums = Vec::new();
    let mut imports = Vec::new();
    for title in ["Bulk healthy", "Bulk damaged"] {
        let mut inventory = item(collection.id);
        inventory.title = title.into();
        let saved = save_inventory(&service, &actor, inventory, Revision(0)).unwrap();
        let imported = derivative_import(&service, &actor, collection.id);
        let mut capture = store.capture(imported.capture_id).unwrap();
        let rev = capture.revision;
        capture.association.item_id = Some(saved.id);
        store.save_capture(capture, rev).unwrap();
        albums.push(saved);
        imports.push(imported);
    }
    let collection_path = std::fs::read_dir(root.path().join("archive/collections"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let damaged = collection_path.join(format!(
        "captures/{}/payload/{}/disc.bin",
        imports[1].capture_id, imports[1].representation_id
    ));
    let original = std::fs::read(&damaged).unwrap();
    let mut changed = original.clone();
    changed[0] ^= 1;
    std::fs::write(&damaged, changed).unwrap();
    let query = MetadataCareQuery {
        collection_id: Some(collection.id),
        search: "Bulk".into(),
        reason: None,
        needs_attention: true,
        cursor: None,
        limit: Some(1),
    };
    assert_eq!(
        service
            .preparation_care(&actor, query.clone())
            .unwrap()
            .items
            .len(),
        1
    );
    let receipt = command();
    let input = PreviewBulkLibrary {
        kind: BulkLibraryKind::MusicPreparation,
        selection: LibrarySelection::AllMatching {
            query,
            exclusions: vec![],
        },
        sources: Default::default(),
    };
    let preview = service
        .preview_bulk_library(&actor, input.clone(), &receipt)
        .unwrap();
    assert_eq!(preview.selected, 2);
    assert_eq!(preview.tracks, 4);
    assert_eq!(preview.excluded, 0);
    assert_eq!(
        service
            .preview_bulk_library(&actor, input, &receipt)
            .unwrap()
            .id,
        preview.id
    );
    let mut added = item(collection.id);
    added.title = "Bulk added after selection".into();
    save_inventory(&service, &actor, added, Revision(0)).unwrap();
    let start_receipt = command();
    service
        .start_bulk_library(&actor, preview.id, &start_receipt)
        .unwrap();
    for _ in 0..40 {
        while service.run_next_job("bulk-care").unwrap() {}
        service.resume_bulk_library().unwrap();
        if service.bulk_library(&actor, preview.id).unwrap().state == BulkLibraryState::Completed {
            break;
        }
    }
    let result = service.bulk_library(&actor, preview.id).unwrap();
    assert_eq!(result.state, BulkLibraryState::Completed);
    assert_eq!(result.succeeded, 1);
    assert_eq!(result.failed, 1);
    assert_eq!(encoder.calls.load(Ordering::SeqCst), 2);
    let first = service
        .bulk_library_members(&actor, preview.id, None, 1)
        .unwrap();
    assert_eq!(first.items.len(), 1);
    assert!(first.next_cursor.is_some());
    let next = first.next_cursor.unwrap().parse().unwrap();
    assert_eq!(
        service
            .bulk_library_members(&actor, preview.id, Some(next), 1)
            .unwrap()
            .items
            .len(),
        1
    );
    assert_eq!(
        service
            .start_bulk_library(&actor, preview.id, &start_receipt)
            .unwrap()
            .id,
        preview.id
    );
    std::fs::write(damaged, original).unwrap();
    service
        .change_bulk_library(&actor, preview.id, false, &command())
        .unwrap();
    for _ in 0..40 {
        while service.run_next_job("bulk-retry").unwrap() {}
        service.resume_bulk_library().unwrap();
        if service.bulk_library(&actor, preview.id).unwrap().state == BulkLibraryState::Completed {
            break;
        }
    }
    let result = service.bulk_library(&actor, preview.id).unwrap();
    assert_eq!(result.succeeded, 2);
    assert_eq!(result.failed, 0);
    assert_eq!(
        encoder.calls.load(Ordering::SeqCst),
        4,
        "Successful albums were not encoded again."
    );
    store.reindex().unwrap();
    assert!(
        service
            .preparation_care(
                &actor,
                MetadataCareQuery {
                    collection_id: Some(collection.id),
                    search: "Bulk healthy".into(),
                    reason: None,
                    needs_attention: true,
                    cursor: None,
                    limit: None
                }
            )
            .unwrap()
            .items
            .is_empty()
    );
    drop(service);
    drop(store);
    let reopened =
        Arc::new(Store::open(root.path().join("archive"), root.path().join("state")).unwrap());
    let service = CollectionService::new(reopened, Arc::new(Registry::builtins()));
    assert_eq!(
        service.bulk_library(&actor, preview.id).unwrap().succeeded,
        2
    );
}

#[test]
fn bulk_preparation_parent_claim_cancellation_revoked_access_and_changed_inventory_are_fenced() {
    use collector_app::{BulkLibraryKind, BulkLibraryState, LibrarySelection, PreviewBulkLibrary};
    let (_root, store, actor, service, collection) = setup();
    let inventory = save_inventory(&service, &actor, item(collection.id), Revision(0)).unwrap();
    let imported = derivative_import(&service, &actor, collection.id);
    let mut capture = store.capture(imported.capture_id).unwrap();
    let rev = capture.revision;
    capture.association.item_id = Some(inventory.id);
    store.save_capture(capture, rev).unwrap();
    let service = service.with_audio_converter(Arc::new(DerivativeFixtureEncoder::new()));
    let preview = service
        .preview_bulk_library(
            &actor,
            PreviewBulkLibrary {
                kind: BulkLibraryKind::MusicPreparation,
                selection: LibrarySelection::Explicit {
                    items: vec![inventory.id],
                },
                sources: Default::default(),
            },
            &command(),
        )
        .unwrap();
    service
        .start_bulk_library(&actor, preview.id, &command())
        .unwrap();
    service
        .change_bulk_library(&actor, preview.id, true, &command())
        .unwrap();
    assert!(!service.run_next_job("cancelled-bulk-parent").unwrap());
    service.resume_bulk_library().unwrap();
    assert_eq!(
        service.bulk_library(&actor, preview.id).unwrap().state,
        BulkLibraryState::Cancelled
    );
    assert!(matches!(
        service.retry_job(&actor, preview.id, &command()),
        Err(Error::Conflict) | Err(Error::Invalid(_))
    ));
    let next = service
        .preview_bulk_library(
            &actor,
            PreviewBulkLibrary {
                kind: BulkLibraryKind::MusicPreparation,
                selection: LibrarySelection::Explicit {
                    items: vec![inventory.id],
                },
                sources: Default::default(),
            },
            &command(),
        )
        .unwrap();
    let mut edited = store.item(inventory.id).unwrap();
    let rev = edited.revision;
    edited.title = "Changed after preview".into();
    save_inventory(&service, &actor, edited, rev).unwrap();
    service
        .start_bulk_library(&actor, next.id, &command())
        .unwrap();
    assert!(service.run_next_job("changed-frozen-inventory").unwrap());
    service.resume_bulk_library().unwrap();
    assert_eq!(service.bulk_library(&actor, next.id).unwrap().failed, 1);
    assert!(
        service
            .bulk_library_members(&actor, next.id, None, 20)
            .unwrap()
            .items[0]
            .job_id
            .is_none()
    );
}

#[test]
fn bulk_discovery_preserves_nested_members_and_paginates_more_than_one_guided_page() {
    let (_root, _store, actor, service, collection) = setup();
    let mut files = Vec::new();
    for position in 0..130 {
        for suffix in ["disc.cue", "tracks/audio.bin"] {
            files.push(UploadFile {
                name: format!("Albums/{position:04}/{suffix}"),
                bytes: ByteLength(0),
                sha256: None,
            });
        }
    }
    files.push(UploadFile {
        name: "Albums/readme.txt".into(),
        bytes: ByteLength(10),
        sha256: None,
    });
    let request = collector_app::DiscoverIngestCommand {
        collection_id: collection.id,
        files,
    };
    let receipt = command();
    let discovery = service
        .discover_bulk_intake(&actor, request.clone(), &receipt)
        .unwrap();
    assert_eq!(
        service
            .discover_bulk_intake(&actor, request, &receipt)
            .unwrap()
            .id,
        discovery.id
    );
    assert_eq!(discovery.package_count, 130);
    assert_eq!(discovery.unassigned_count, 1);
    let mut cursor = None;
    let mut count = 0;
    loop {
        let page = service
            .intake_discovery_page(&actor, discovery.id, cursor, None)
            .unwrap();
        assert!(page.packages.items.len() <= 64);
        for p in &page.packages.items {
            assert_eq!(p.files.len(), 2);
            assert!(p.files.iter().any(|f| f.name.ends_with("tracks/audio.bin")));
        }
        count += page.packages.items.len();
        assert_eq!(page.unassigned_files.items, ["Albums/readme.txt"]);
        let Some(next) = page.packages.next_cursor else {
            break;
        };
        cursor = Some(next.parse().unwrap());
    }
    assert_eq!(count, 130);
}

#[test]
fn interrupted_intake_manifest_cannot_start_a_partial_batch() {
    use collector_app::{
        AppendIntake, CreateIntake, IntakeDestination, IntakePackage, StartIntake,
    };
    let (_root, store, actor, service, collection) = setup();
    let session = service
        .create_intake(
            &actor,
            CreateIntake {
                collection_id: collection.id,
                expected_packages: 2,
                expected_files: 2,
                automatic_metadata: false,
                prepare_music: false,
            },
            &command(),
        )
        .unwrap();
    let package = IntakePackage {
        package: IngestPackageSelection {
            source: ImportSourceSummary {
                kind: ImportSourceKind::Browser,
                label: None,
                package_name: "Unknown original".into(),
            },
            files: vec![UploadFile {
                name: "original.bin".into(),
                bytes: ByteLength(0),
                sha256: None,
            }],
            anchor_file_index: None,
        },
        destination: IntakeDestination::Separate,
    };
    let receipt = command();
    let request = AppendIntake {
        expected_revision: session.revision,
        packages: vec![package.clone()],
    };
    let session = service
        .append_intake_with_preflight(&actor, session.id, request.clone(), &receipt, |_| Ok(()))
        .unwrap();
    assert_eq!(
        service
            .append_intake_with_preflight(&actor, session.id, request, &receipt, |_| panic!(
                "receipt must bypass preflight"
            ))
            .unwrap()
            .package_count,
        1
    );
    assert!(matches!(
        service.start_intake(
            &actor,
            session.id,
            StartIntake {
                expected_revision: session.revision
            },
            &command()
        ),
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        store.job_record(session.id, actor.user_id),
        Err(Error::NotFound)
    ));
    let session = service
        .append_intake_with_preflight(
            &actor,
            session.id,
            AppendIntake {
                expected_revision: session.revision,
                packages: vec![package],
            },
            &command(),
            |_| Ok(()),
        )
        .unwrap();
    service
        .start_intake(
            &actor,
            session.id,
            StartIntake {
                expected_revision: session.revision,
            },
            &command(),
        )
        .unwrap();
    service.run_next_job("complete-manifest").unwrap();
    assert_eq!(service.intake(&actor, session.id).unwrap().admitted, 2);
}

#[test]
fn album_group_conflicts_copy_assembly_unknown_fields_and_later_edits() {
    use collector_app::{
        AlbumGroupChoices, ConfirmAlbumGroup, GroupConflictKind, GroupDiscPosition, GroupEntry,
        PhysicalSetAssembly, PreviewAlbumGroup,
    };
    let (_root, store, actor, service, collection) = setup();
    let mut inventories = Vec::new();
    for i in 0..2 {
        let mut v = item(collection.id);
        v.title = format!("Manual album {i}");
        v.wishlist = false;
        v.domain.payload = RawJson::from_value(
            serde_json::json!({"future_extension":{"copy":i,"unknown":[null,{"keep":true}]}}),
        );
        v.specimens.push(Specimen {
            id: SpecimenId::new(),
            label: format!("Set {i}"),
            condition: Condition::Good,
            ownership: Ownership::Owned,
            possession: Possession::Present,
            acquisition: None,
            private_notes: None,
            loans: vec![],
            components: vec![Component {
                id: ComponentId::new(),
                label: "Disc".into(),
                kind: ComponentKind::OpticalDisc,
                position: 1,
                carrier: None,
                migration_provenance: None,
            }],
        });
        inventories.push(Catalog::save(&*store, v, Revision(0)).unwrap());
    }
    let left = &inventories[0];
    let right = &inventories[1];
    let mut request = PreviewAlbumGroup {
        destination: left.id,
        entries: inventories
            .iter()
            .map(|i| GroupEntry {
                item_id: i.id,
                expected_revision: i.revision,
            })
            .collect(),
        choices: AlbumGroupChoices::default(),
    };
    let preview = service
        .preview_album_group(&actor, request.clone())
        .unwrap();
    assert!(!preview.can_confirm);
    assert!(
        preview
            .conflicts
            .iter()
            .any(|c| matches!(c.kind, GroupConflictKind::ManualTitle))
    );
    assert!(
        preview
            .conflicts
            .iter()
            .any(|c| matches!(c.kind, GroupConflictKind::DomainInformation))
    );
    request.choices.title_from = Some(left.id);
    request.choices.domain_from = Some(left.id);
    request.choices.assemblies = vec![PhysicalSetAssembly {
        destination: left.specimens[0].id,
        sources: vec![right.specimens[0].id],
    }];
    let preview = service
        .preview_album_group(&actor, request.clone())
        .unwrap();
    assert!(
        preview
            .conflicts
            .iter()
            .any(|c| matches!(c.kind, GroupConflictKind::PhysicalInventory))
    );
    request.choices.disc_positions = vec![GroupDiscPosition {
        component_id: right.specimens[0].components[0].id,
        position: 2,
    }];
    let preview = service
        .preview_album_group(&actor, request.clone())
        .unwrap();
    assert!(preview.can_confirm, "{:?}", preview.conflicts);
    let group = service
        .confirm_album_group(
            &actor,
            ConfirmAlbumGroup {
                preview: request,
                token: preview.token,
            },
            &command(),
        )
        .unwrap();
    let merged = Catalog::item(&*store, left.id).unwrap();
    assert_eq!(merged.specimens.len(), 1);
    assert_eq!(merged.specimens[0].components.len(), 2);
    let archived =
        collector_app::AlbumGroupingPort::album_group_publication(&*store, group.group_id).unwrap();
    assert_eq!(
        archived.entries[1].before.domain.payload,
        right.domain.payload
    );
    let mut edited = merged.clone();
    edited.subtitle = "Edited after grouping".into();
    Catalog::save(&*store, edited, merged.revision).unwrap();
    let undo = service.album_group_undo(&actor, group.group_id).unwrap();
    assert!(!undo.can_undo);
    assert!(undo.reason.is_some());
    assert!(matches!(
        service.undo_album_group(&actor, group.group_id, &command()),
        Err(Error::Conflict)
    ));
    store.reindex().unwrap();
    assert!(
        !service
            .album_group_undo(&actor, group.group_id)
            .unwrap()
            .can_undo
    );
}

#[test]
fn album_group_keeps_loaned_copies_distinct_and_blocks_contradictory_set_assembly() {
    use collector_app::{
        AlbumGroupChoices, GroupConflictKind, GroupEntry, PhysicalSetAssembly, PreviewAlbumGroup,
    };
    let (_root, store, actor, service, collection) = setup();
    let mut inventories = Vec::new();
    for i in 0..2 {
        let mut v = item(collection.id);
        v.wishlist = false;
        v.specimens.push(Specimen {
            id: SpecimenId::new(),
            label: "Copy".into(),
            condition: Condition::Good,
            ownership: Ownership::Owned,
            possession: if i == 0 {
                Possession::Lent
            } else {
                Possession::Present
            },
            acquisition: None,
            private_notes: None,
            components: vec![],
            loans: if i == 0 {
                vec![Loan {
                    id: LoanId::new(),
                    contact: "A friend".into(),
                    lent_at: chrono::NaiveDate::from_ymd_opt(2026, 10, 1).unwrap(),
                    due_at: None,
                    returned_at: None,
                }]
            } else {
                vec![]
            },
        });
        inventories.push(Catalog::save(&*store, v, Revision(0)).unwrap());
    }
    let mut request = PreviewAlbumGroup {
        destination: inventories[0].id,
        entries: inventories
            .iter()
            .map(|i| GroupEntry {
                item_id: i.id,
                expected_revision: i.revision,
            })
            .collect(),
        choices: AlbumGroupChoices::default(),
    };
    assert!(
        service
            .preview_album_group(&actor, request.clone())
            .unwrap()
            .can_confirm
    );
    request.choices.assemblies = vec![PhysicalSetAssembly {
        destination: inventories[0].specimens[0].id,
        sources: vec![inventories[1].specimens[0].id],
    }];
    assert!(
        service
            .preview_album_group(&actor, request.clone())
            .unwrap()
            .conflicts
            .iter()
            .any(|c| matches!(c.kind, GroupConflictKind::ActiveLoan))
    );
    let mut v = inventories[0].clone();
    v.specimens[0].loans[0].returned_at =
        Some(chrono::NaiveDate::from_ymd_opt(2026, 10, 2).unwrap());
    v.specimens[0].possession = Possession::Elsewhere;
    let rev = v.revision;
    let v = Catalog::save(&*store, v, rev).unwrap();
    request.entries[0].expected_revision = v.revision;
    assert!(
        service
            .preview_album_group(&actor, request)
            .unwrap()
            .conflicts
            .iter()
            .any(|c| matches!(c.kind, GroupConflictKind::Possession))
    );
}

#[test]
fn subprocess_album_group_worker() {
    let Ok(archive) = std::env::var("BL_CHILD_ARCHIVE") else {
        return;
    };
    let state = std::env::var("BL_CHILD_STATE").unwrap();
    let point = parse_publication_point(&std::env::var("BL_CHILD_POINT").unwrap());
    let store = Arc::new(
        Store::open_with_publication_fault(
            &archive,
            &state,
            Arc::new(BlockAtPublicationPoint(point)),
        )
        .unwrap(),
    );
    let service = CollectionService::new(store, Arc::new(Registry::builtins()));
    let actor = Principal {
        user_id: std::env::var("BL_CHILD_USER").unwrap().parse().unwrap(),
        administrator: false,
        device_id: None,
    };
    let request: collector_app::ConfirmAlbumGroup =
        serde_json::from_slice(&std::fs::read(std::env::var("BL_CHILD_INPUT").unwrap()).unwrap())
            .unwrap();
    let context = CommandContext::new(std::env::var("BL_CHILD_KEY").unwrap()).unwrap();
    service
        .confirm_album_group(&actor, request, &context)
        .unwrap();
    panic!("The fault worker must wait at its phase marker.");
}
#[test]
fn sigkill_group_publication_recovers_all_members_and_the_same_receipt() {
    use collector_app::{AlbumGroupChoices, ConfirmAlbumGroup, GroupEntry, PreviewAlbumGroup};
    use collector_store::PublicationFaultPoint as Point;
    use std::io::BufRead as _;
    for point in publication_points() {
        let (root, store, actor, service, collection) = setup();
        let left = Catalog::save(&*store, item(collection.id), Revision(0)).unwrap();
        let right = Catalog::save(&*store, item(collection.id), Revision(0)).unwrap();
        let request = PreviewAlbumGroup {
            destination: left.id,
            entries: vec![
                GroupEntry {
                    item_id: left.id,
                    expected_revision: left.revision,
                },
                GroupEntry {
                    item_id: right.id,
                    expected_revision: right.revision,
                },
            ],
            choices: AlbumGroupChoices::default(),
        };
        let preview = service
            .preview_album_group(&actor, request.clone())
            .unwrap();
        let request = ConfirmAlbumGroup {
            preview: request,
            token: preview.token,
        };
        let context = command();
        let input = root.path().join("group-input.json");
        std::fs::write(&input, serde_json::to_vec(&request).unwrap()).unwrap();
        drop(service);
        drop(store);
        let archive = root.path().join("archive");
        let state = root.path().join("state");
        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "subprocess_album_group_worker", "--nocapture"])
            .env("BL_CHILD_ARCHIVE", &archive)
            .env("BL_CHILD_STATE", &state)
            .env("BL_CHILD_POINT", format!("{point:?}"))
            .env("BL_CHILD_USER", actor.user_id.to_string())
            .env("BL_CHILD_KEY", context.key())
            .env("BL_CHILD_INPUT", input)
            .stdout(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        let mut reader = std::io::BufReader::new(child.stdout.take().unwrap());
        let mut line = String::new();
        loop {
            line.clear();
            assert_ne!(
                reader.read_line(&mut line).unwrap(),
                0,
                "child exited before marker"
            );
            if line.contains("P01_PHASE:") {
                break;
            }
        }
        child.kill().unwrap();
        assert!(!child.wait().unwrap().success());
        let store = Arc::new(Store::open(&archive, &state).unwrap());
        let committed = matches!(
            point,
            Point::RevisionCommitted | Point::ProjectionApplied | Point::Completed
        );
        assert_eq!(
            Catalog::item(&*store, left.id).unwrap().revision,
            Revision(if committed { 2 } else { 1 }),
            "{point:?}"
        );
        assert_eq!(
            Catalog::item(&*store, right.id).unwrap().deleted,
            committed,
            "{point:?}"
        );
        let service = CollectionService::new(store.clone(), Arc::new(Registry::builtins()));
        let result = service
            .confirm_album_group(&actor, request.clone(), &context)
            .unwrap();
        assert_eq!(
            service
                .confirm_album_group(&actor, request, &context)
                .unwrap()
                .group_id,
            result.group_id
        );
        store.reindex().unwrap();
        assert!(Catalog::item(&*store, right.id).unwrap().deleted);
        assert!(
            service
                .album_group_undo(&actor, result.group_id)
                .unwrap()
                .can_undo
        );
    }
}

#[test]
#[ignore = "generated 2,400-package/album scale qualification; run explicitly"]
fn bulk_generated_thousands_query_admission_and_restart_measurements() {
    use collector_app::{
        AppendIntake, BulkLibraryKind, CreateIntake, IntakeDestination, IntakePackage,
        LibrarySelection, MetadataCareQuery, PreviewBulkLibrary, StartIntake,
    };
    use std::time::Instant;
    let count = 2400usize;
    let (root, store, actor, service, collection) = setup();
    let mut session = service
        .create_intake(
            &actor,
            CreateIntake {
                collection_id: collection.id,
                expected_packages: count as u32,
                expected_files: count as u32,
                automatic_metadata: false,
                prepare_music: false,
            },
            &command(),
        )
        .unwrap();
    let started = Instant::now();
    for offset in (0..count).step_by(64) {
        let packages = (offset..(offset + 64).min(count))
            .map(|position| IntakePackage {
                package: IngestPackageSelection {
                    source: ImportSourceSummary {
                        kind: ImportSourceKind::Browser,
                        label: None,
                        package_name: format!("Generated package {position:06}"),
                    },
                    files: vec![UploadFile {
                        name: format!("Originals/{position:06}/disc.bin"),
                        bytes: ByteLength(0),
                        sha256: None,
                    }],
                    anchor_file_index: None,
                },
                destination: IntakeDestination::Separate,
            })
            .collect();
        session = service
            .append_intake_with_preflight(
                &actor,
                session.id,
                AppendIntake {
                    expected_revision: session.revision,
                    packages,
                },
                &command(),
                |_| Ok(()),
            )
            .unwrap();
    }
    let manifest_ms = started.elapsed().as_secs_f64() * 1000.0;
    service
        .start_intake(
            &actor,
            session.id,
            StartIntake {
                expected_revision: session.revision,
            },
            &command(),
        )
        .unwrap();
    let started = Instant::now();
    service.run_next_job("scale-intake-admission").unwrap();
    let admission_ms = started.elapsed().as_secs_f64() * 1000.0;
    assert_eq!(
        service.intake(&actor, session.id).unwrap().admitted,
        count as u32
    );
    let mut cursor = None;
    let mut seen = 0;
    loop {
        let p = service
            .intake_members(&actor, session.id, cursor, 200)
            .unwrap();
        seen += p.items.len();
        let Some(next) = p.next_cursor else { break };
        cursor = Some(next.parse().unwrap());
    }
    assert_eq!(seen, count);
    service
        .cancel_intake(&actor, session.id, &command())
        .unwrap();
    // Generate valid ordinary archive records directly, avoiding timing thousands
    // of interactive single-entry saves. The archive reader validates the graph.
    let saved = Catalog::save(&*store, item(collection.id), Revision(0)).unwrap();
    let record_path = files_named(&root.path().join("archive"), "record.v3.json")
        .into_iter()
        .find(|p| p.to_string_lossy().contains(&saved.id.to_string()))
        .unwrap();
    let record: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&record_path).unwrap()).unwrap();
    let manifest: toml::Value = toml::from_str(
        &std::fs::read_to_string(record_path.with_file_name("manifest.v3.toml")).unwrap(),
    )
    .unwrap();
    let items_dir = record_path.ancestors().nth(4).unwrap();
    for position in 1..count {
        let id = ItemId::new();
        let transaction = TransactionId::new();
        let mut record = record.clone();
        record["record"]["id"] = serde_json::json!(id);
        record["record"]["title"] = serde_json::json!(format!("Generated album {position:06}"));
        record["transaction_id"] = serde_json::json!(transaction);
        let bytes = serde_json::to_vec_pretty(&record).unwrap();
        let mut manifest = manifest.clone();
        manifest["entity_id"] = toml::Value::String(id.to_string());
        manifest["transaction_id"] = toml::Value::String(transaction.to_string());
        manifest["record_sha256"] = toml::Value::String(Digest::sha256(&bytes).hex);
        let dir = items_dir.join(format!("generated--{id}/revisions/00000000000000000001"));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("record.v3.json"), bytes).unwrap();
        std::fs::write(
            dir.join("manifest.v3.toml"),
            toml::to_string(&manifest).unwrap(),
        )
        .unwrap();
    }
    store.reindex().unwrap();
    let provider = Arc::new(
        MusicBrainzProvider::with_test_origin(
            "collector-scale/0.1 (test@example.invalid)",
            "http://127.0.0.1:1/",
            Duration::from_millis(50),
            Duration::ZERO,
            Duration::from_secs(60),
            Duration::from_secs(60),
        )
        .unwrap(),
    );
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let service = service
        .with_streaming_provider(
            ModuleId(collector_modules::music::MUSICBRAINZ_PROVIDER_ID.into()),
            Arc::new(FixtureStreamingProvider {
                calls: calls.clone(),
                bytes: br#"{"releases":[]}"#.to_vec(),
            }),
        )
        .unwrap()
        .with_catalog_parser(
            ModuleId(collector_modules::music::MUSICBRAINZ_PROVIDER_ID.into()),
            provider,
        )
        .unwrap();
    let query = MetadataCareQuery {
        collection_id: Some(collection.id),
        search: String::new(),
        reason: None,
        needs_attention: false,
        cursor: None,
        limit: Some(50),
    };
    let started = Instant::now();
    assert_eq!(
        service
            .metadata_care(&actor, query.clone())
            .unwrap()
            .items
            .len(),
        50
    );
    let query_ms = started.elapsed().as_secs_f64() * 1000.0;
    let started = Instant::now();
    let preview = service
        .preview_bulk_library(
            &actor,
            PreviewBulkLibrary {
                kind: BulkLibraryKind::Metadata,
                selection: LibrarySelection::AllMatching {
                    query,
                    exclusions: vec![],
                },
                sources: Default::default(),
            },
            &command(),
        )
        .unwrap();
    let freeze_ms = started.elapsed().as_secs_f64() * 1000.0;
    assert_eq!(preview.selected, count as u32);
    assert_eq!(preview.excluded, 0);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    service
        .start_bulk_library(&actor, preview.id, &command())
        .unwrap();
    let started = Instant::now();
    service.run_next_job("scale-metadata-admission").unwrap();
    let metadata_admission_ms = started.elapsed().as_secs_f64() * 1000.0;
    let before = service
        .bulk_library_members(&actor, preview.id, None, 200)
        .unwrap();
    assert_eq!(
        before.items.iter().filter(|m| m.job_id.is_some()).count(),
        16
    );
    assert_eq!(
        calls.load(Ordering::SeqCst),
        0,
        "Admission performed provider work"
    );
    drop(service);
    drop(store);
    let started = Instant::now();
    let reopened =
        Arc::new(Store::open(root.path().join("archive"), root.path().join("state")).unwrap());
    let restart_ms = started.elapsed().as_secs_f64() * 1000.0;
    assert!(reopened.catalog_reused_at_startup());
    let service = CollectionService::new(reopened, Arc::new(Registry::builtins()));
    assert_eq!(
        service.intake(&actor, session.id).unwrap().admitted,
        count as u32
    );
    assert_eq!(
        service.bulk_library(&actor, preview.id).unwrap().selected,
        count as u32
    );
    let after = service
        .bulk_library_members(&actor, preview.id, None, 200)
        .unwrap();
    assert_eq!(
        serde_json::to_value(before.items).unwrap(),
        serde_json::to_value(after.items).unwrap()
    );
    println!(
        "packages={count} albums={count} manifest_ms={manifest_ms:.1} intake_admission_ms={admission_ms:.1} care_page_ms={query_ms:.1} frozen_selection_ms={freeze_ms:.1} rolling_16_admission_ms={metadata_admission_ms:.1} checkpoint_restart_ms={restart_ms:.1}"
    );
}

#[test]
fn intake_destination_conflict_is_revisitable_and_does_not_stop_other_packages() {
    use collector_app::{
        AppendIntake, CreateIntake, IntakeDestination, IntakePackage, RepairIntakeDestination,
        StartIntake,
    };
    let (_root, store, actor, service, collection) = setup();
    let destination = Catalog::save(&*store, item(collection.id), Revision(0)).unwrap();
    let session = service
        .create_intake(
            &actor,
            CreateIntake {
                collection_id: collection.id,
                expected_packages: 2,
                expected_files: 2,
                automatic_metadata: false,
                prepare_music: false,
            },
            &command(),
        )
        .unwrap();
    let packages = (0..2)
        .map(|i| IntakePackage {
            package: IngestPackageSelection {
                source: ImportSourceSummary {
                    kind: ImportSourceKind::Browser,
                    label: None,
                    package_name: format!("Pending {i}"),
                },
                files: vec![UploadFile {
                    name: "original.bin".into(),
                    bytes: ByteLength(0),
                    sha256: None,
                }],
                anchor_file_index: None,
            },
            destination: if i == 0 {
                IntakeDestination::Existing {
                    item_id: destination.id,
                    expected_revision: destination.revision,
                    copy_key: "Original copy".into(),
                }
            } else {
                IntakeDestination::Separate
            },
        })
        .collect();
    let session = service
        .append_intake_with_preflight(
            &actor,
            session.id,
            AppendIntake {
                expected_revision: session.revision,
                packages,
            },
            &command(),
            |_| Ok(()),
        )
        .unwrap();
    service
        .start_intake(
            &actor,
            session.id,
            StartIntake {
                expected_revision: session.revision,
            },
            &command(),
        )
        .unwrap();
    let mut changed = destination.clone();
    changed.title = "Changed after manifest review".into();
    Catalog::save(&*store, changed, destination.revision).unwrap();
    service.run_next_job("destination-conflict").unwrap();
    let session = service.intake(&actor, session.id).unwrap();
    assert_eq!(session.admitted, 1);
    assert_eq!(session.rejected, 1);
    let members = service
        .intake_members(&actor, session.id, None, 32)
        .unwrap();
    assert!(members.items[0].error.is_some());
    assert!(members.items[0].import_id.is_none());
    assert!(members.items[1].import_id.is_some());
    let receipt = command();
    let repair = RepairIntakeDestination {
        expected_revision: session.revision,
        destination: IntakeDestination::Separate,
    };
    service
        .repair_intake_destination(&actor, session.id, 0, repair.clone(), &receipt)
        .unwrap();
    service.run_next_job("repaired-destination").unwrap();
    assert_eq!(service.intake(&actor, session.id).unwrap().admitted, 2);
    assert_eq!(service.intake(&actor, session.id).unwrap().rejected, 0);
    assert_eq!(
        service
            .repair_intake_destination(&actor, session.id, 0, repair, &receipt)
            .unwrap()
            .id,
        session.id
    );
    service.cancel_job(&actor, session.id, &command()).unwrap();
    assert!(matches!(
        service.intake(&actor, session.id).unwrap().state,
        collector_app::IntakeState::Cancelling
    ));
    service.resume_bulk_intakes().unwrap();
    assert!(matches!(
        service.intake(&actor, session.id).unwrap().state,
        collector_app::IntakeState::Cancelled
    ));
}

#[test]
fn bulk_parent_cancellation_preserves_later_direct_and_other_batch_requests() {
    use collector_app::{BulkLibraryKind, LibrarySelection, PreviewBulkLibrary};
    let clock = Arc::new(TestClock::new(chrono::Utc::now().timestamp()));
    let (_root, store, actor, service, collection) = setup_with_clock(Some(clock.clone()));
    let saved = save_inventory(&service, &actor, item(collection.id), Revision(0)).unwrap();
    let imported = derivative_import(&service, &actor, collection.id);
    let mut capture = store.capture(imported.capture_id).unwrap();
    let revision = capture.revision;
    capture.association.item_id = Some(saved.id);
    store.save_capture(capture, revision).unwrap();
    let service = service.with_audio_converter(Arc::new(DerivativeFixtureEncoder::new()));
    let request = PreviewBulkLibrary {
        kind: BulkLibraryKind::MusicPreparation,
        selection: LibrarySelection::Explicit {
            items: vec![saved.id],
        },
        sources: Default::default(),
    };
    let first = service
        .preview_bulk_library(&actor, request.clone(), &command())
        .unwrap();
    service
        .start_bulk_library(&actor, first.id, &command())
        .unwrap();
    clock.advance(1);
    let second = service
        .preview_bulk_library(&actor, request, &command())
        .unwrap();
    service
        .start_bulk_library(&actor, second.id, &command())
        .unwrap();
    // Both bulk roots precede their album children; both album children precede
    // their track children. Advancing time defeats accidental timestamp equality.
    for _ in 0..4 {
        clock.advance(2);
        assert!(service.run_next_job("shared-bulk-admission").unwrap());
    }
    let child = |id| {
        service
            .bulk_library_members(&actor, id, None, 20)
            .unwrap()
            .items[0]
            .job_id
            .unwrap()
    };
    let first_album = service
        .music_preparation_batch(&actor, child(first.id))
        .unwrap();
    let second_album = service
        .music_preparation_batch(&actor, child(second.id))
        .unwrap();
    assert_eq!(
        first_album.members[0].job_id,
        second_album.members[0].job_id
    );
    let shared = first_album.members[1].job_id.unwrap();
    let original_timestamp = store.job_record(shared, actor.user_id).unwrap().created_at;
    clock.advance(10);
    let direct = service
        .prepare_configured_music_track(
            &actor,
            saved.id,
            imported.capture_id,
            PrepareMusicTrack {
                source_representation_id: imported.representation_id,
                profile: ConversionProfile::Flac,
            },
            first_album.members[0].track_position,
            &command(),
        )
        .unwrap();
    assert_eq!(Some(direct), first_album.members[0].job_id);
    assert_eq!(
        store.job_record(shared, actor.user_id).unwrap().created_at,
        original_timestamp
    );
    service
        .change_bulk_library(&actor, first.id, true, &command())
        .unwrap();
    assert_eq!(
        store.job_record(shared, actor.user_id).unwrap().state,
        JobState::Queued
    );
    clock.advance(2);
    service.cancel_job(&actor, second.id, &command()).unwrap();
    assert_eq!(
        store.job_record(shared, actor.user_id).unwrap().state,
        JobState::Cancelled
    );
    assert_eq!(
        store.job_record(direct, actor.user_id).unwrap().state,
        JobState::Queued
    );
    assert!(
        service
            .run_next_job("direct-survives-bulk-cancellation")
            .unwrap()
    );
    assert_eq!(
        store.job_record(direct, actor.user_id).unwrap().state,
        JobState::Completed
    );
}

#[test]
fn bulk_failed_parent_admission_is_actionable_and_retries_without_losing_members() {
    use collector_app::{BulkLibraryKind, LibrarySelection, PreviewBulkLibrary};
    let (_root, store, actor, service, collection) = setup();
    let saved = save_inventory(&service, &actor, item(collection.id), Revision(0)).unwrap();
    let imported = derivative_import(&service, &actor, collection.id);
    let mut capture = store.capture(imported.capture_id).unwrap();
    let revision = capture.revision;
    capture.association.item_id = Some(saved.id);
    store.save_capture(capture, revision).unwrap();
    let service = service.with_audio_converter(Arc::new(DerivativeFixtureEncoder::new()));
    let preview = service
        .preview_bulk_library(
            &actor,
            PreviewBulkLibrary {
                kind: BulkLibraryKind::MusicPreparation,
                selection: LibrarySelection::Explicit {
                    items: vec![saved.id],
                },
                sources: Default::default(),
            },
            &command(),
        )
        .unwrap();
    service
        .start_bulk_library(&actor, preview.id, &command())
        .unwrap();
    let claim = store
        .claim_job("interrupted-admission", 30)
        .unwrap()
        .unwrap();
    assert_eq!(claim.id, preview.id);
    store
        .fail_job(claim.id, &claim.claim_token, "Temporary admission failure")
        .unwrap();
    service.resume_bulk_library().unwrap();
    let summary = service.bulk_library(&actor, preview.id).unwrap();
    assert!(summary.can_retry);
    assert_eq!(
        summary.admission_error.as_deref(),
        Some("Temporary admission failure")
    );
    service
        .change_bulk_library(&actor, preview.id, false, &command())
        .unwrap();
    assert!(service.run_next_job("retried-bulk-root").unwrap());
    service.resume_bulk_library().unwrap();
    assert!(
        service
            .bulk_library(&actor, preview.id)
            .unwrap()
            .admission_error
            .is_none()
    );
    assert!(
        service
            .bulk_library_members(&actor, preview.id, None, 20)
            .unwrap()
            .items[0]
            .job_id
            .is_some()
    );
}

#[test]
fn album_group_cannot_assemble_copies_without_known_disc_inventory() {
    use collector_app::{
        AlbumGroupChoices, GroupConflictKind, GroupEntry, PhysicalSetAssembly, PreviewAlbumGroup,
    };
    let (_root, store, actor, service, collection) = setup();
    let mut entries = Vec::new();
    for label in ["Uncatalogued first copy", "Uncatalogued second copy"] {
        let mut inventory = item(collection.id);
        inventory.specimens.push(Specimen {
            id: SpecimenId::new(),
            label: label.into(),
            condition: Condition::Unknown,
            ownership: Ownership::Owned,
            possession: Possession::Present,
            components: vec![],
            acquisition: None,
            private_notes: None,
            loans: vec![],
        });
        entries.push(Catalog::save(store.as_ref(), inventory, Revision(0)).unwrap());
    }
    let preview = service
        .preview_album_group(
            &actor,
            PreviewAlbumGroup {
                destination: entries[0].id,
                entries: entries
                    .iter()
                    .map(|i| GroupEntry {
                        item_id: i.id,
                        expected_revision: i.revision,
                    })
                    .collect(),
                choices: AlbumGroupChoices {
                    assemblies: vec![PhysicalSetAssembly {
                        destination: entries[0].specimens[0].id,
                        sources: vec![entries[1].specimens[0].id],
                    }],
                    ..Default::default()
                },
            },
        )
        .unwrap();
    assert!(!preview.can_confirm);
    assert!(
        preview
            .conflicts
            .iter()
            .any(|c| matches!(c.kind, GroupConflictKind::PhysicalInventory))
    );
}

#[test]
fn collection_lists_track_occurrences_replay_conflict_and_rebuild_without_payload_reads() {
    use collector_app::{
        CollectionListChange as Change, CollectionListPort, CreateCollectionList, ListEntriesQuery,
        UpdateCollectionList,
    };
    let (root, store, actor, service, collection) = setup();
    let album = save_inventory(&service, &actor, item(collection.id), Revision(0)).unwrap();
    let imported = derivative_import(&service, &actor, collection.id);
    let mut capture = Catalog::capture(store.as_ref(), imported.capture_id).unwrap();
    let expected = capture.revision;
    capture.association.item_id = Some(album.id);
    Catalog::save_capture(store.as_ref(), capture, expected).unwrap();
    let create = command();
    let list = service
        .create_collection_list(
            &actor,
            collection.id,
            CreateCollectionList {
                name: "Evening".into(),
            },
            &create,
        )
        .unwrap();
    assert_eq!(
        service
            .create_collection_list(
                &actor,
                collection.id,
                CreateCollectionList {
                    name: "Evening".into()
                },
                &create
            )
            .unwrap()
            .id,
        list.id
    );
    let append = command();
    let update = UpdateCollectionList {
        expected_revision: list.revision,
        change: Change::Append {
            item_id: album.id,
            source_representation_id: None,
            track_position: None,
        },
    };
    let added = service
        .update_collection_list(&actor, collection.id, list.id, update.clone(), &append)
        .unwrap();
    assert_eq!(added.entry_count, 2);
    assert_eq!(
        service
            .update_collection_list(&actor, collection.id, list.id, update.clone(), &append)
            .unwrap()
            .revision,
        added.revision
    );
    assert!(matches!(
        service.update_collection_list(&actor, collection.id, list.id, update, &command()),
        Err(Error::Conflict)
    ));
    let again = service
        .update_collection_list(
            &actor,
            collection.id,
            list.id,
            UpdateCollectionList {
                expected_revision: added.revision,
                change: Change::Append {
                    item_id: album.id,
                    source_representation_id: Some(imported.representation_id),
                    track_position: Some(1),
                },
            },
            &command(),
        )
        .unwrap();
    let record = store.collection_list(list.id).unwrap();
    assert_eq!(record.entries.len(), 3);
    assert_ne!(record.entries[0].id, record.entries[2].id);
    // Missing bytes must not make list queries or additions turn into an integrity scan.
    let pcm: Vec<u8> = (0..5 * 588)
        .flat_map(|i| [i as u8, 0x80, 0xff, 0x7f])
        .collect();
    let payload = find_file_with_bytes(&root.path().join("archive"), &pcm).unwrap();
    std::fs::remove_file(payload).unwrap();
    let first = service
        .collection_list_view(
            &actor,
            collection.id,
            list.id,
            ListEntriesQuery {
                cursor: None,
                limit: Some(1),
            },
        )
        .unwrap();
    assert_eq!(first.entries.items.len(), 1);
    let cursor = first.entries.next_cursor.unwrap();
    assert!(matches!(
        first.entries.items[0].availability,
        collector_app::ListEntryAvailability::SourceUnavailable
    ));
    let moved = service
        .update_collection_list(
            &actor,
            collection.id,
            list.id,
            UpdateCollectionList {
                expected_revision: again.revision,
                change: Change::Move {
                    entry_id: record.entries[2].id,
                    before: Some(record.entries[0].id),
                },
            },
            &command(),
        )
        .unwrap();
    assert!(matches!(
        service.collection_list_view(
            &actor,
            collection.id,
            list.id,
            ListEntriesQuery {
                cursor: Some(cursor),
                limit: None
            }
        ),
        Err(Error::Conflict)
    ));
    assert_eq!(
        store.collection_list(list.id).unwrap().entries[0].id,
        record.entries[2].id
    );
    let removed = service
        .update_collection_list(
            &actor,
            collection.id,
            list.id,
            UpdateCollectionList {
                expected_revision: moved.revision,
                change: Change::Remove {
                    entry_id: record.entries[2].id,
                },
            },
            &command(),
        )
        .unwrap();
    assert_eq!(removed.entry_count, 2);
    drop(service);
    drop(store);
    let store =
        Arc::new(Store::open(root.path().join("archive"), root.path().join("state")).unwrap());
    assert!(store.catalog_reused_at_startup());
    let service = CollectionService::new(store.clone(), Arc::new(Registry::builtins()));
    assert_eq!(store.collection_list(list.id).unwrap().entries.len(), 2);
    store.reindex().unwrap();
    assert_eq!(store.collection_list(list.id).unwrap().entries.len(), 2);
    let deleted = service
        .update_collection_list(
            &actor,
            collection.id,
            list.id,
            UpdateCollectionList {
                expected_revision: removed.revision,
                change: Change::Delete,
            },
            &command(),
        )
        .unwrap();
    assert_eq!(deleted.entry_count, 2);
    assert!(matches!(
        service.collection_list_view(
            &actor,
            collection.id,
            list.id,
            ListEntriesQuery {
                cursor: None,
                limit: None
            }
        ),
        Err(Error::NotFound)
    ));
    store.reindex().unwrap();
    assert!(store.collection_list(list.id).unwrap().deleted);
}

#[test]
fn collection_lists_enforce_collection_roles_and_frozen_source_revisions() {
    use collector_app::{
        CollectionListChange as Change, CollectionListPort, CreateCollectionList, ListEntriesQuery,
        ListSourceRevision, UpdateCollectionList,
    };
    let (_root, store, actor, service, collection) = setup();
    let curator = Principal {
        user_id: store
            .create_user("list-curator", "another long password", false)
            .unwrap(),
        administrator: false,
        device_id: None,
    };
    Catalog::grant(
        store.as_ref(),
        collection.id,
        curator.user_id,
        Role::Curator,
    )
    .unwrap();
    let list = service
        .create_collection_list(
            &curator,
            collection.id,
            CreateCollectionList {
                name: "Shared".into(),
            },
            &command(),
        )
        .unwrap();
    let album = save_inventory(&service, &actor, item(collection.id), Revision(0)).unwrap();
    let imported = derivative_import(&service, &actor, collection.id);
    let mut capture = Catalog::capture(store.as_ref(), imported.capture_id).unwrap();
    let revision = capture.revision;
    capture.association.item_id = Some(album.id);
    Catalog::save_capture(store.as_ref(), capture, revision).unwrap();
    let appended = service
        .update_collection_list(
            &curator,
            collection.id,
            list.id,
            UpdateCollectionList {
                expected_revision: list.revision,
                change: Change::Append {
                    item_id: album.id,
                    source_representation_id: None,
                    track_position: None,
                },
            },
            &command(),
        )
        .unwrap();
    let mut forged = store.collection_list(list.id).unwrap();
    forged.revision = Revision(appended.revision.0 + 1);
    assert!(matches!(
        store.publish_collection_list(
            curator.user_id,
            forged.clone(),
            appended.revision,
            vec![ListSourceRevision {
                item_id: album.id,
                item_revision: Revision(900),
                metadata_revision: Revision(0),
                captures: vec![]
            }]
        ),
        Err(Error::Conflict)
    ));
    Catalog::grant(store.as_ref(), collection.id, curator.user_id, Role::Viewer).unwrap();
    assert!(
        !service
            .collection_list_view(
                &curator,
                collection.id,
                list.id,
                ListEntriesQuery {
                    cursor: None,
                    limit: None
                }
            )
            .unwrap()
            .list
            .can_edit
    );
    assert!(matches!(
        store.publish_collection_list(curator.user_id, forged, appended.revision, vec![]),
        Err(Error::Forbidden)
    ));
    assert!(matches!(
        service.update_collection_list(
            &curator,
            collection.id,
            list.id,
            UpdateCollectionList {
                expected_revision: appended.revision,
                change: Change::Delete
            },
            &command()
        ),
        Err(Error::Forbidden)
    ));
    let outsider = Principal {
        user_id: store
            .create_user("list-outsider", "another long password", false)
            .unwrap(),
        administrator: false,
        device_id: None,
    };
    assert!(matches!(
        service.collection_list_view(
            &outsider,
            collection.id,
            list.id,
            ListEntriesQuery {
                cursor: None,
                limit: None
            }
        ),
        Err(Error::Forbidden)
    ));
    let other = service
        .create_collection(
            &store
                .login("owner", "a long test password", "lists")
                .unwrap()
                .1,
            typed_collection("Other"),
            &command(),
        )
        .unwrap();
    let foreign = save_inventory(&service, &actor, item(other.id), Revision(0)).unwrap();
    assert!(matches!(
        service.update_collection_list(
            &actor,
            collection.id,
            list.id,
            UpdateCollectionList {
                expected_revision: appended.revision,
                change: Change::Append {
                    item_id: foreign.id,
                    source_representation_id: None,
                    track_position: None
                }
            },
            &command()
        ),
        Err(Error::Forbidden)
    ));
}

#[test]
fn collection_lists_recover_each_archive_commit_boundary_and_command_receipt() {
    use collector_app::{CollectionListsQuery, CreateCollectionList};
    for point in derivative_phases() {
        let (root, store, actor, service, collection) = setup();
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
        let service = CollectionService::new(store.clone(), Arc::new(Registry::builtins()));
        let receipt = command();
        let request = CreateCollectionList {
            name: "Recovered".into(),
        };
        assert!(
            service
                .create_collection_list(&actor, collection.id, request.clone(), &receipt)
                .is_err()
        );
        drop(service);
        drop(store);
        let store =
            Arc::new(Store::open(root.path().join("archive"), root.path().join("state")).unwrap());
        let service = CollectionService::new(store.clone(), Arc::new(Registry::builtins()));
        let list = service
            .create_collection_list(&actor, collection.id, request.clone(), &receipt)
            .unwrap();
        assert_eq!(
            service
                .create_collection_list(&actor, collection.id, request, &receipt)
                .unwrap()
                .id,
            list.id
        );
        let page = service
            .collection_lists(
                &actor,
                collection.id,
                CollectionListsQuery {
                    search: None,
                    cursor: None,
                    limit: None,
                },
            )
            .unwrap();
        assert_eq!(page.items.len(), 1, "{point:?}");
    }
}

#[test]
fn collection_lists_generic_items_and_music_source_choices_remain_distinct() {
    use collector_app::{
        CollectionListChange as Change, CollectionListPort, CreateCollectionList, ListEntriesQuery,
        UpdateCollectionList,
    };
    let (_root, store, actor, service, collection) = setup();
    let games = service
        .create_collection(
            &store
                .login("owner", "a long test password", "lists")
                .unwrap()
                .1,
            CreateCollectionCommand {
                name: "Games".into(),
                module: ModuleId("org.junk.games".into()),
            },
            &command(),
        )
        .unwrap();
    let mut game = item(games.id);
    game.domain = ModuleData {
        module: ModuleId("org.junk.games".into()),
        schema_version: 1,
        payload: RawJson::from_json("{}").unwrap(),
    };
    let game = save_inventory(&service, &actor, game, Revision(0)).unwrap();
    let list = service
        .create_collection_list(
            &actor,
            games.id,
            CreateCollectionList {
                name: "To finish".into(),
            },
            &command(),
        )
        .unwrap();
    assert_eq!(list.kind, CollectionListKind::Items);
    let updated = service
        .update_collection_list(
            &actor,
            games.id,
            list.id,
            UpdateCollectionList {
                expected_revision: list.revision,
                change: Change::Append {
                    item_id: game.id,
                    source_representation_id: None,
                    track_position: None,
                },
            },
            &command(),
        )
        .unwrap();
    let view = service
        .collection_list_view(
            &actor,
            games.id,
            list.id,
            ListEntriesQuery {
                cursor: None,
                limit: None,
            },
        )
        .unwrap();
    assert!(matches!(
        view.entries.items[0].availability,
        collector_app::ListEntryAvailability::Entry
    ));
    assert!(matches!(
        service.update_collection_list(
            &actor,
            games.id,
            list.id,
            UpdateCollectionList {
                expected_revision: updated.revision,
                change: Change::Append {
                    item_id: game.id,
                    source_representation_id: Some(RepresentationId::new()),
                    track_position: Some(1)
                }
            },
            &command()
        ),
        Err(Error::Invalid(_))
    ));
    let album = save_inventory(&service, &actor, item(collection.id), Revision(0)).unwrap();
    let first = derivative_import(&service, &actor, collection.id);
    let second = derivative_import(&service, &actor, collection.id);
    for imported in [&first, &second] {
        let mut capture = Catalog::capture(store.as_ref(), imported.capture_id).unwrap();
        let revision = capture.revision;
        capture.association.item_id = Some(album.id);
        Catalog::save_capture(store.as_ref(), capture, revision).unwrap();
    }
    let playlist = service
        .create_collection_list(
            &actor,
            collection.id,
            CreateCollectionList {
                name: "Music".into(),
            },
            &command(),
        )
        .unwrap();
    // Unlinked originals do not acquire album positions from UUID/discovery order.
    assert!(matches!(
        service.update_collection_list(
            &actor,
            collection.id,
            playlist.id,
            UpdateCollectionList {
                expected_revision: playlist.revision,
                change: Change::Append {
                    item_id: album.id,
                    source_representation_id: None,
                    track_position: None
                }
            },
            &command()
        ),
        Err(Error::Invalid(_))
    ));
    let added = service
        .update_collection_list(
            &actor,
            collection.id,
            playlist.id,
            UpdateCollectionList {
                expected_revision: playlist.revision,
                change: Change::Append {
                    item_id: album.id,
                    source_representation_id: Some(second.representation_id),
                    track_position: None,
                },
            },
            &command(),
        )
        .unwrap();
    assert_eq!(added.entry_count, 2);
    let record = store.collection_list(playlist.id).unwrap();
    assert!(record.entries.iter().all(|entry|matches!(entry.target,ListTarget::MusicTrack{source_representation_id,..} if source_representation_id==second.representation_id)));
    assert!(matches!(
        service.update_collection_list(
            &actor,
            collection.id,
            playlist.id,
            UpdateCollectionList {
                expected_revision: added.revision,
                change: Change::Append {
                    item_id: album.id,
                    source_representation_id: Some(first.representation_id),
                    track_position: Some(99)
                }
            },
            &command()
        ),
        Err(Error::Invalid(_))
    ));
}

#[test]
fn collection_lists_follow_merged_capture_ownership_and_guarded_undo() {
    use collector_app::{
        AlbumGroupChoices, CollectionListChange as Change, ConfirmAlbumGroup, CreateCollectionList,
        GroupEntry, ListEntriesQuery, PreviewAlbumGroup, UpdateCollectionList,
    };
    let (_root, store, actor, service, collection) = setup();
    let left = save_inventory(&service, &actor, item(collection.id), Revision(0)).unwrap();
    let right = save_inventory(&service, &actor, item(collection.id), Revision(0)).unwrap();
    let imported = derivative_import(&service, &actor, collection.id);
    let mut capture = Catalog::capture(store.as_ref(), imported.capture_id).unwrap();
    let revision = capture.revision;
    capture.association.item_id = Some(left.id);
    Catalog::save_capture(store.as_ref(), capture, revision).unwrap();
    let list = service
        .create_collection_list(
            &actor,
            collection.id,
            CreateCollectionList {
                name: "Across grouping".into(),
            },
            &command(),
        )
        .unwrap();
    service
        .update_collection_list(
            &actor,
            collection.id,
            list.id,
            UpdateCollectionList {
                expected_revision: list.revision,
                change: Change::Append {
                    item_id: left.id,
                    source_representation_id: None,
                    track_position: None,
                },
            },
            &command(),
        )
        .unwrap();
    let preview = PreviewAlbumGroup {
        destination: right.id,
        entries: vec![
            GroupEntry {
                item_id: left.id,
                expected_revision: left.revision,
            },
            GroupEntry {
                item_id: right.id,
                expected_revision: right.revision,
            },
        ],
        choices: AlbumGroupChoices::default(),
    };
    let checked = service
        .preview_album_group(&actor, preview.clone())
        .unwrap();
    assert!(checked.can_confirm, "{:?}", checked.conflicts);
    let merged = service
        .confirm_album_group(
            &actor,
            ConfirmAlbumGroup {
                preview,
                token: checked.token,
            },
            &command(),
        )
        .unwrap();
    let view = service
        .collection_list_view(
            &actor,
            collection.id,
            list.id,
            ListEntriesQuery {
                cursor: None,
                limit: None,
            },
        )
        .unwrap();
    assert_eq!(view.entries.items.len(), 2);
    assert!(
        view.entries
            .items
            .iter()
            .all(|entry| entry.item_id == Some(right.id))
    );
    service
        .undo_album_group(&actor, merged.group_id, &command())
        .unwrap();
    let view = service
        .collection_list_view(
            &actor,
            collection.id,
            list.id,
            ListEntriesQuery {
                cursor: None,
                limit: None,
            },
        )
        .unwrap();
    assert!(
        view.entries
            .items
            .iter()
            .all(|entry| entry.item_id == Some(left.id))
    );
    let missing = CollectionService::new(store.clone(), Arc::new(Registry::default()));
    let view = missing
        .collection_list_view(
            &actor,
            collection.id,
            list.id,
            ListEntriesQuery {
                cursor: None,
                limit: None,
            },
        )
        .unwrap();
    assert!(view.entries.items.iter().all(|entry| matches!(
        entry.availability,
        collector_app::ListEntryAvailability::ModuleUnavailable
    )));
    service
        .set_item_removed(
            &actor,
            left.id,
            Catalog::item(store.as_ref(), left.id).unwrap().revision,
            true,
            &command(),
        )
        .unwrap();
    let view = service
        .collection_list_view(
            &actor,
            collection.id,
            list.id,
            ListEntriesQuery {
                cursor: None,
                limit: None,
            },
        )
        .unwrap();
    assert!(
        view.entries
            .items
            .iter()
            .all(|entry| entry.item_id.is_none()
                && matches!(
                    entry.availability,
                    collector_app::ListEntryAvailability::EntryRemoved
                ))
    );
}

#[test]
fn collection_lists_block_partial_disc_expansion_when_originals_are_missing() {
    use collector_app::{
        CollectionListChange as Change, CollectionListPort, CreateCollectionList,
        UpdateCollectionList,
    };
    for prepared_tracks in 0..=2 {
        let (root, store, actor, service, collection) = setup();
        let album = save_inventory(&service, &actor, item(collection.id), Revision(0)).unwrap();
        let imported = derivative_import(&service, &actor, collection.id);
        let mut capture = Catalog::capture(store.as_ref(), imported.capture_id).unwrap();
        let revision = capture.revision;
        capture.association.item_id = Some(album.id);
        Catalog::save_capture(store.as_ref(), capture, revision).unwrap();
        let service = service.with_audio_converter(Arc::new(DerivativeFixtureEncoder::new()));
        for position in 1..=prepared_tracks {
            service
                .prepare_music_derivative(
                    &actor,
                    imported.capture_id,
                    derivative_command(&imported, position),
                    &command(),
                )
                .unwrap();
        }
        while service.run_next_job("list-partial-source").unwrap() {}
        let pcm: Vec<u8> = (0..5 * 588)
            .flat_map(|i| [i as u8, 0x80, 0xff, 0x7f])
            .collect();
        let payload = find_file_with_bytes(&root.path().join("archive"), &pcm).unwrap();
        std::fs::remove_file(&payload).unwrap();
        for source in [None, Some(imported.representation_id)] {
            let list = service
                .create_collection_list(
                    &actor,
                    collection.id,
                    CreateCollectionList {
                        name: "Complete disc scope".into(),
                    },
                    &command(),
                )
                .unwrap();
            let request = UpdateCollectionList {
                expected_revision: list.revision,
                change: Change::Append {
                    item_id: album.id,
                    source_representation_id: source,
                    track_position: None,
                },
            };
            let result =
                service.update_collection_list(&actor, collection.id, list.id, request, &command());
            if prepared_tracks == 2 {
                assert_eq!(result.unwrap().entry_count, 2);
            } else {
                assert!(
                    matches!(result, Err(Error::Invalid(message)) if message.contains("complete disc audio scope"))
                );
                assert!(store.collection_list(list.id).unwrap().entries.is_empty());
                if prepared_tracks == 1 {
                    let single = service
                        .update_collection_list(
                            &actor,
                            collection.id,
                            list.id,
                            UpdateCollectionList {
                                expected_revision: list.revision,
                                change: Change::Append {
                                    item_id: album.id,
                                    source_representation_id: Some(imported.representation_id),
                                    track_position: Some(1),
                                },
                            },
                            &command(),
                        )
                        .unwrap();
                    assert_eq!(single.entry_count, 1);
                }
            }
        }
        // Restoring the original makes both recorded tracks available to expand
        // again, without requiring new preparation or a payload integrity scan.
        std::fs::write(payload, &pcm).unwrap();
        let list = service
            .create_collection_list(
                &actor,
                collection.id,
                CreateCollectionList {
                    name: "Restored disc".into(),
                },
                &command(),
            )
            .unwrap();
        assert_eq!(
            service
                .update_collection_list(
                    &actor,
                    collection.id,
                    list.id,
                    UpdateCollectionList {
                        expected_revision: list.revision,
                        change: Change::Append {
                            item_id: album.id,
                            source_representation_id: None,
                            track_position: None,
                        },
                    },
                    &command(),
                )
                .unwrap()
                .entry_count,
            2
        );
    }
}

#[test]
fn song_search_is_literal_unicode_paged_authorized_and_rebuilt_with_manual_metadata() {
    use collector_app::{MusicSearchQuery, UpdateMetadataAuthorityCommand};
    let (root, store, actor, service, collection) = setup();
    let mut inventory = item(collection.id);
    inventory.domain.payload =
        RawJson::from_value(serde_json::json!({"artist":"Ａｒｔｉｓｔ", "tracks":[
            {"position":1,"title":"Ｓｏｎｇ %_ Café","start":0,"length":588},
            {"position":2,"title":"Second song","start":588,"length":588}
        ]}));
    let saved = save_inventory(&service, &actor, inventory, Revision(0)).unwrap();
    let query = |search: &str, cursor: Option<String>, limit| MusicSearchQuery {
        search: Some(search.into()),
        cursor,
        limit: Some(limit),
    };
    let hit = service
        .music_search(
            &actor,
            collection.id,
            query("song %_ cafe\u{301}", None, 50),
        )
        .unwrap();
    assert_eq!(hit.items.len(), 1);
    assert_eq!(hit.items[0].song.item_id, saved.id);
    assert!(hit.items[0].sources.is_empty());
    let first = service
        .music_search(&actor, collection.id, query("song", None, 1))
        .unwrap();
    let second = service
        .music_search(
            &actor,
            collection.id,
            query("song", first.next_cursor.clone(), 1),
        )
        .unwrap();
    assert_eq!(first.items.len(), 1);
    assert_eq!(second.items.len(), 1);
    assert_ne!(
        first.items[0].song.track_position,
        second.items[0].song.track_position
    );
    assert!(second.next_cursor.is_none());
    assert!(matches!(
        service.music_search(
            &actor,
            collection.id,
            query("different", first.next_cursor, 1)
        ),
        Err(Error::Invalid(_))
    ));
    let stranger = Principal {
        user_id: UserId::new(),
        administrator: false,
        device_id: None,
    };
    assert!(matches!(
        service.music_search(&stranger, collection.id, query("", None, 50)),
        Err(Error::Forbidden)
    ));
    let authority = service.metadata_view(&actor, saved.id).unwrap();
    service
        .update_metadata_authority(
            &actor,
            saved.id,
            UpdateMetadataAuthorityCommand {
                expected_revision: authority.revision,
                change: MetadataAuthorityChange::SetDomainOverrides {
                    overrides: ModuleData {
                        module: ModuleId("org.junk.music".into()),
                        schema_version: 1,
                        payload: RawJson::from_value(serde_json::json!({"artist":"Manual Artist"})),
                    },
                },
            },
            &command(),
        )
        .unwrap();
    assert_eq!(
        service
            .music_search(&actor, collection.id, query("manual artist", None, 50))
            .unwrap()
            .items
            .len(),
        2
    );
    let mut edited = Catalog::item(store.as_ref(), saved.id).unwrap();
    let revision = edited.revision;
    edited.domain.payload = RawJson::from_value(
        serde_json::json!({"artist":"Later inventory artist", "tracks":[{"position":1,"title":"New song title","start":0,"length":588}]}),
    );
    save_inventory(&service, &actor, edited, revision).unwrap();
    assert_eq!(
        service
            .music_search(&actor, collection.id, query("new song title", None, 50))
            .unwrap()
            .items
            .len(),
        1
    );
    store.reindex().unwrap();
    assert_eq!(
        service
            .music_search(&actor, collection.id, query("new song title", None, 50))
            .unwrap()
            .items
            .len(),
        1
    );
    drop(service);
    drop(store);
    let store =
        Arc::new(Store::open(root.path().join("archive"), root.path().join("state")).unwrap());
    let service = CollectionService::new(store, Arc::new(Registry::builtins()));
    assert_eq!(
        service
            .music_search(&actor, collection.id, query("new song title", None, 50))
            .unwrap()
            .items
            .len(),
        1
    );
}

#[test]
fn unbound_enrollment_secret_gates_state_and_two_accounts_have_one_approval_winner() {
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
    let first = store
        .create_user("first", "a sufficiently long password", true)
        .unwrap();
    let second = store
        .create_user("second", "a sufficiently long password", false)
        .unwrap();
    let pending = store.start_enrollment(None, "Phono", "/pair").unwrap();
    assert!(
        store
            .enrollment_status(pending.id, &"0".repeat(64), true)
            .is_err()
    );
    let waiting = store
        .enrollment_status(pending.id, &pending.completion_secret, false)
        .unwrap();
    assert_eq!(
        waiting.phase,
        collector_app::EnrollmentPhase::PendingApproval
    );
    assert_eq!(waiting.user_id, None);
    assert_eq!(
        store
            .preview_enrollment(second, &pending.user_code)
            .unwrap()
            .requested_label,
        "Phono"
    );
    let barrier = Arc::new(std::sync::Barrier::new(3));
    let workers: Vec<_> = [first, second]
        .into_iter()
        .map(|user| {
            let store = store.clone();
            let barrier = barrier.clone();
            let code = pending.user_code.clone();
            thread::spawn(move || {
                barrier.wait();
                store.approve_enrollment(user, &code)
            })
        })
        .collect();
    barrier.wait();
    let results: Vec<_> = workers.into_iter().map(|w| w.join().unwrap()).collect();
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    let winner = results
        .into_iter()
        .find_map(Result::ok)
        .unwrap()
        .user_id
        .unwrap();
    let issued = store
        .complete_enrollment(pending.id, &pending.completion_secret)
        .unwrap();
    assert_eq!(issued.user_id, winner);
    assert_eq!(
        store
            .authenticate_native(&issued.credential)
            .unwrap()
            .principal
            .user_id,
        winner
    );
    assert_eq!(
        store
            .enrollment_status(pending.id, &pending.completion_secret, false)
            .unwrap()
            .phase,
        collector_app::EnrollmentPhase::Issued
    );
    let expiring = store.start_enrollment(None, "Expires", "/pair").unwrap();
    clock.advance(600);
    assert!(
        store
            .enrollment_status(expiring.id, &"0".repeat(64), true)
            .is_err()
    );
    assert_eq!(
        store
            .enrollment_status(expiring.id, &expiring.completion_secret, false)
            .unwrap()
            .phase,
        collector_app::EnrollmentPhase::Expired
    );
}

#[test]
fn password_change_receipt_retains_access_then_explicit_revocation_is_account_scoped() {
    let root = tempfile::tempdir().unwrap();
    let store =
        Arc::new(Store::open(root.path().join("archive"), root.path().join("state")).unwrap());
    let user = store
        .create_user("owner", "a sufficiently long password", true)
        .unwrap();
    let other = store
        .create_user("other", "a sufficiently long password", false)
        .unwrap();
    let (current, actor) = store
        .login("owner", "a sufficiently long password", "current")
        .unwrap();
    let (old, _) = store
        .login("owner", "a sufficiently long password", "other browser")
        .unwrap();
    store
        .insert_passkey(user, "retained-passkey", serde_json::json!({"counter":0}))
        .unwrap();
    let service = CollectionService::new(store.clone(), Arc::new(Registry::builtins()));
    let codes = service.replace_recovery_codes(&actor).unwrap();
    let issue = |user| {
        let p = store.start_enrollment(Some(user), "App", "/pair").unwrap();
        store.approve_enrollment(user, &p.user_code).unwrap();
        store
            .complete_enrollment(p.id, &p.completion_secret)
            .unwrap()
    };
    let app = issue(user);
    let other_app = issue(other);
    let pending = store
        .start_enrollment(Some(user), "Pending", "/pair")
        .unwrap();
    let unbound = store.start_enrollment(None, "Unbound", "/pair").unwrap();
    let binding = store
        .password_change_binding(user, "binding-test", "a changed long password")
        .unwrap();
    assert!(binding.starts_with("$argon2id$"));
    assert_eq!(
        binding,
        store
            .password_change_binding(user, "binding-test", "a changed long password")
            .unwrap()
    );
    assert_ne!(
        binding,
        store
            .password_change_binding(user, "another-key", "a changed long password")
            .unwrap()
    );
    use argon2::PasswordVerifier;
    argon2::Argon2::default()
        .verify_password(
            b"a changed long password",
            &argon2::PasswordHash::new(&binding).unwrap(),
        )
        .unwrap();
    let expected = service.profile(&actor).unwrap().revision;
    let receipt = command();
    let next = service
        .change_password(
            &actor,
            expected,
            "a changed long password",
            "a changed long password",
            &receipt,
        )
        .unwrap();
    assert_eq!(
        service
            .change_password(
                &actor,
                expected,
                "a changed long password",
                "a changed long password",
                &receipt
            )
            .unwrap(),
        next
    );
    assert!(
        service
            .change_password(
                &actor,
                expected,
                "different long password",
                "different long password",
                &receipt
            )
            .is_err()
    );
    assert!(store.authenticate(&current).is_ok());
    assert!(store.authenticate(&old).is_ok());
    assert!(store.authenticate_native(&app.credential).is_ok());
    assert_eq!(store.passkeys(user).unwrap().len(), 1);
    assert!(
        store
            .login("owner", "a changed long password", "new password")
            .is_ok()
    );
    let revoke = command();
    service.revoke_other_access(&actor, &revoke).unwrap();
    service.revoke_other_access(&actor, &revoke).unwrap();
    assert!(store.authenticate(&current).is_ok());
    assert!(store.authenticate(&old).is_err());
    assert!(store.authenticate_native(&app.credential).is_err());
    assert!(store.authenticate_native(&other_app.credential).is_ok());
    assert_eq!(
        store
            .enrollment_status(pending.id, &pending.completion_secret, false)
            .unwrap()
            .phase,
        collector_app::EnrollmentPhase::Cancelled
    );
    assert_eq!(
        store
            .enrollment_status(unbound.id, &unbound.completion_secret, false)
            .unwrap()
            .phase,
        collector_app::EnrollmentPhase::PendingApproval
    );
    assert_eq!(store.passkeys(user).unwrap().len(), 1);
    // Recovery retains the passkey but replaces all browser/app access and the code batch.
    let replacement_app = issue(user);
    let (fresh, _) = store
        .consume_recovery_code("owner", &codes[0], "recovered long password", "recovered")
        .unwrap();
    assert!(store.authenticate(&fresh).is_ok());
    assert!(store.authenticate(&current).is_err());
    assert!(
        store
            .authenticate_native(&replacement_app.credential)
            .is_err()
    );
    assert!(store.authenticate_native(&other_app.credential).is_ok());
    assert_eq!(store.passkeys(user).unwrap().len(), 1);
    assert!(
        store
            .consume_recovery_code("owner", &codes[1], "recovered long password", "replay")
            .is_err()
    );
}

#[test]
fn stopped_account_lookup_and_username_recovery_preserve_authority_without_archive() {
    let root = tempfile::tempdir().unwrap();
    let archive = root.path().join("archive");
    let state = root.path().join("state");
    assert!(Store::local_accounts(&state, None, 10).is_err());
    assert!(!state.exists());
    let store = Store::open(&archive, &state).unwrap();
    let owner = store
        .create_user("alpha", "a sufficiently long password", true)
        .unwrap();
    let disabled = store
        .create_user("beta", "a sufficiently long password", false)
        .unwrap();
    assert!(matches!(
        Store::local_accounts(&state, None, 10),
        Err(Error::Unavailable(_))
    ));
    drop(store);
    let db = rusqlite::Connection::open(state.join("private.sqlite")).unwrap();
    db.execute(
        "UPDATE users SET enabled=0 WHERE id=?",
        [disabled.to_string()],
    )
    .unwrap();
    drop(db);
    std::fs::remove_dir_all(&archive).unwrap();
    let first = Store::local_accounts(&state, None, 1).unwrap();
    assert_eq!(first.items[0].id, owner);
    assert!(first.items[0].is_default);
    assert!(first.items[0].administrator);
    let second = Store::local_accounts(&state, first.next_cursor.as_deref(), 1).unwrap();
    assert_eq!(second.items[0].id, disabled);
    assert!(!second.items[0].enabled);
    assert!(
        Store::local_recover(
            &state,
            collector_app::LocalAccountSelector::Username("missing".into()),
            "replacement long password"
        )
        .is_err()
    );
    assert_eq!(
        Store::local_recover(
            &state,
            collector_app::LocalAccountSelector::Username("beta".into()),
            "replacement long password"
        )
        .unwrap(),
        disabled
    );
    let rows = Store::local_accounts(&state, None, 10).unwrap();
    assert!(!rows.items[1].enabled);
    assert!(!rows.items[1].administrator);
    assert!(rows.items[0].is_default);
    assert!(!archive.exists());
    let db = rusqlite::Connection::open(state.join("private.sqlite")).unwrap();
    db.execute_batch("PRAGMA user_version=99").unwrap();
    drop(db);
    assert!(matches!(
        Store::local_accounts(&state, None, 10),
        Err(Error::Unavailable(_))
    ));
    assert!(matches!(
        Store::local_recover(
            &state,
            collector_app::LocalAccountSelector::Id(owner),
            "replacement long password"
        ),
        Err(Error::Unavailable(_))
    ));
}

#[test]
fn schema18_enrollment_conversion_preserves_bound_requests_native_credentials_and_grants() {
    let root = tempfile::tempdir().unwrap();
    let archive = root.path().join("archive");
    let state = root.path().join("state");
    let store = Store::open(&archive, &state).unwrap();
    let user = store
        .create_user("owner", "a sufficiently long password", true)
        .unwrap();
    let pending = store
        .start_enrollment(Some(user), "Existing pending", "/pair")
        .unwrap();
    let issued = store
        .start_enrollment(Some(user), "Existing issued", "/pair")
        .unwrap();
    store.approve_enrollment(user, &issued.user_code).unwrap();
    let credential = store
        .complete_enrollment(issued.id, &issued.completion_secret)
        .unwrap();
    drop(store);
    let db = rusqlite::Connection::open(state.join("private.sqlite")).unwrap();
    db.execute_batch("BEGIN; ALTER TABLE enrollments RENAME TO enrollments_new; DROP INDEX enrollments_active_user_code;
        CREATE TABLE enrollments (id TEXT PRIMARY KEY,user_id TEXT NOT NULL,completion_secret_hash TEXT NOT NULL,user_code TEXT NOT NULL,requested_label TEXT NOT NULL,verification_url TEXT NOT NULL,phase TEXT NOT NULL,created_at INTEGER NOT NULL,expires_at INTEGER NOT NULL,approved_at INTEGER,issued_at INTEGER,cancelled_at INTEGER);
        INSERT INTO enrollments SELECT * FROM enrollments_new;DROP TABLE enrollments_new;CREATE UNIQUE INDEX enrollments_active_user_code ON enrollments(user_code) WHERE phase IN ('pending_approval','approved');PRAGMA user_version=18;COMMIT;").unwrap();
    drop(db);
    let store = Store::open(&archive, &state).unwrap();
    assert_eq!(store.default_user_id().unwrap(), user);
    assert!(store.authenticate_native(&credential.credential).is_ok());
    assert_eq!(
        store
            .enrollment_status(pending.id, &pending.completion_secret, false)
            .unwrap()
            .user_id,
        Some(user)
    );
    store.approve_enrollment(user, &pending.user_code).unwrap();
    assert_eq!(
        store
            .complete_enrollment(pending.id, &pending.completion_secret)
            .unwrap()
            .user_id,
        user
    );
    let new = store
        .start_enrollment(None, "Unbound after conversion", "/pair")
        .unwrap();
    assert_eq!(
        store
            .enrollment_status(new.id, &new.completion_secret, false)
            .unwrap()
            .user_id,
        None
    );
}

#[test]
fn uploaded_dat_survives_restart_binds_receipts_and_keeps_snapshot_history() {
    use collector_app::{DatUploadMetadata, UploadedCatalogPort};
    let root = tempfile::tempdir().unwrap();
    let clock = Arc::new(TestClock::new(1_800_000_000));
    let archive = root.path().join("archive");
    let private = root.path().join("state");
    let store = Arc::new(Store::open_with_clock(&archive, &private, clock.clone()).unwrap());
    let user = store
        .create_user("upload-owner", "a long test password", true)
        .unwrap();
    let actor = Principal {
        user_id: user,
        administrator: true,
        device_id: None,
    };
    let collection = CollectionService::new(store.clone(), Arc::new(Registry::builtins()))
        .create_collection(
            &AuthenticatedPrincipal {
                principal: actor.clone(),
                credential_kind: CredentialKind::TestLocal,
                authenticated_at: clock.now(),
            },
            CreateCollectionCommand {
                name: "Uploaded games".into(),
                module: ModuleId("org.junk.games".into()),
            },
            &command(),
        )
        .unwrap();
    let metadata = DatUploadMetadata {
        system: "Nintendo - Game Boy".into(),
        catalog: "No-Intro".into(),
        byte_domain: "org.junk.game.raw-bytes".into(),
    };
    let bytes = |version| {
        format!(r#"<datafile><header><name>Nintendo - Game Boy</name><author>No-Intro</author><version>{version}</version></header><game name="Known Game"><rom name="game.bin" size="4" sha1="a17c9aaa61e80a1bf71d0d850af4e5baa9800bbd"/></game></datafile>"#).into_bytes()
    };
    let configure = |store: Arc<Store>| {
        let provider = Arc::new(
            LocalDatProvider::new(Default::default())
                .unwrap()
                .with_uploads(store.clone()),
        );
        CollectionService::new(store.clone(), Arc::new(Registry::builtins()))
            .with_catalog_uploads(store)
            .with_streaming_provider(ModuleId(LOCAL_DAT_PROVIDER_ID.into()), provider.clone())
            .unwrap()
            .with_catalog_parser(ModuleId(LOCAL_DAT_PROVIDER_ID.into()), provider)
            .unwrap()
    };
    let service = configure(store.clone());
    let receipt = command();
    let job = service
        .upload_dat_catalog(
            &actor,
            collection.id,
            metadata.clone(),
            &bytes("1"),
            &receipt,
        )
        .unwrap();
    drop(service);
    drop(store);
    let store = Arc::new(Store::open_with_clock(&archive, &private, clock.clone()).unwrap());
    let service = configure(store.clone());
    provider_drain(&service);
    assert_eq!(service.job(&actor, job).unwrap().state, JobState::Completed);
    let id = SnapshotId(job.0);
    let saved = service
        .catalog_snapshots(&actor, collection.id, None, 10)
        .unwrap();
    assert_eq!(saved.items.len(), 1);
    assert!(!saved.items[0].active);
    let parsed: collector_providers::DatSnapshotObservation =
        saved.items[0].snapshot.parsed_observation.parse().unwrap();
    assert_eq!(parsed.preview_entries[0].label, "Known Game");
    assert_eq!(parsed.preview_entries[0].files, ["game.bin"]);
    service
        .activate_catalog_snapshot(&actor, collection.id, id, &command())
        .unwrap();
    clock.advance(60);
    assert_eq!(
        service
            .upload_dat_catalog(
                &actor,
                collection.id,
                metadata.clone(),
                &bytes("1"),
                &receipt
            )
            .unwrap(),
        job
    );
    assert!(matches!(
        service.upload_dat_catalog(
            &actor,
            collection.id,
            metadata.clone(),
            &bytes("2"),
            &receipt
        ),
        Err(Error::IdempotencyConflict)
    ));
    let second = service
        .upload_dat_catalog(
            &actor,
            collection.id,
            metadata.clone(),
            &bytes("2"),
            &command(),
        )
        .unwrap();
    provider_drain(&service);
    service
        .activate_catalog_snapshot(&actor, collection.id, SnapshotId(second.0), &command())
        .unwrap();
    store.reindex().unwrap();
    assert_eq!(
        service
            .catalog_snapshots(&actor, collection.id, None, 10)
            .unwrap()
            .items
            .len(),
        2
    );
    let mut exact = Vec::new();
    SnapshotPort::snapshot_bytes(
        &*store,
        SnapshotQuery {
            collection_id: collection.id,
            snapshot_id: id,
        },
    )
    .unwrap()
    .read_to_end(&mut exact)
    .unwrap();
    assert_eq!(exact, bytes("1"));
    let denied = Principal {
        administrator: false,
        ..actor.clone()
    };
    assert!(
        service
            .upload_dat_catalog(
                &denied,
                collection.id,
                metadata.clone(),
                &bytes("1"),
                &command()
            )
            .is_err()
    );
    let malformed = service
        .upload_dat_catalog(
            &actor,
            collection.id,
            metadata.clone(),
            b"<datafile><game",
            &command(),
        )
        .unwrap();
    provider_drain(&service);
    assert_eq!(
        service.job(&actor, malformed).unwrap().state,
        JobState::Failed
    );
    assert_eq!(
        service
            .catalog_snapshots(&actor, collection.id, None, 10)
            .unwrap()
            .items
            .len(),
        2
    );
    let changed = service
        .upload_dat_catalog(
            &actor,
            collection.id,
            metadata.clone(),
            &bytes("3"),
            &command(),
        )
        .unwrap();
    let digest = UploadedCatalogPort::stage_uploaded_catalog(&*store, &bytes("3")).unwrap();
    let mut corrupted = bytes("3");
    corrupted[10] ^= 1;
    std::fs::write(private.join("catalog-uploads").join(digest.hex), corrupted).unwrap();
    provider_drain(&service);
    assert_eq!(
        service.job(&actor, changed).unwrap().state,
        JobState::Failed
    );
    // A new upload of the exact original can repair private staging and retry.
    UploadedCatalogPort::stage_uploaded_catalog(&*store, &bytes("3")).unwrap();
    service.retry_job(&actor, changed, &command()).unwrap();
    provider_drain(&service);
    assert_eq!(
        service.job(&actor, changed).unwrap().state,
        JobState::Completed
    );
    let cancelled = service
        .upload_dat_catalog(&actor, collection.id, metadata, &bytes("4"), &command())
        .unwrap();
    service.cancel_job(&actor, cancelled, &command()).unwrap();
    let _ = service.run_next_job("cancelled-upload").unwrap();
    assert_eq!(
        service.job(&actor, cancelled).unwrap().state,
        JobState::Cancelled
    );
    assert_eq!(
        service
            .catalog_snapshots(&actor, collection.id, None, 10)
            .unwrap()
            .items
            .len(),
        3
    );
}

fn shared_provider_input(collection: &Collection, input: &str) -> FrozenAcquisitionJob {
    FrozenAcquisitionJob {
        collection_binding: collection.binding.clone(),
        request: ProviderRequest {
            collection_id: collection.id,
            provider: ModuleId(ACCURATERIP_PROVIDER_ID.into()),
            operation: OperationId::new("org.junk.providers.accuraterip.acquire").unwrap(),
            source_fingerprint: None,
            query: RawJson::from_value(serde_json::json!({"disc":input})),
            maximum_response_bytes: 1024,
        },
        item_id: None,
        snapshot_id: SnapshotId::new(),
        created_at: chrono::Utc::now(),
    }
}
fn provider_drain(service: &CollectionService) {
    for _ in 0..40 {
        if !service.run_next_job("shared-provider-test").unwrap() {
            return;
        }
    }
    panic!("provider workflows did not settle");
}
fn completed_provider_snapshot(
    service: &CollectionService,
    actor: &Principal,
    job: JobId,
) -> SnapshotId {
    let view = service.job(actor, job).unwrap();
    assert_eq!(view.state, JobState::Completed, "{:?}", view.error);
    match view.result.unwrap() {
        collector_app::JobResultView::CatalogSnapshot { snapshot_id, .. } => snapshot_id,
        _ => panic!("expected catalog result"),
    }
}

#[test]
fn shared_provider_download_has_reserved_capacity_and_private_cross_user_ownership() {
    let (_root, store, actor, base, collection) = setup();
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let service = base
        .with_streaming_provider(
            ModuleId(ACCURATERIP_PROVIDER_ID.into()),
            Arc::new(FixtureStreamingProvider {
                calls: calls.clone(),
                bytes: b"shared response".to_vec(),
            }),
        )
        .unwrap();
    let other_id = store
        .create_user("second-provider-owner", "a long test password", false)
        .unwrap();
    Catalog::grant(store.as_ref(), collection.id, other_id, Role::Curator).unwrap();
    let other = Principal {
        user_id: other_id,
        administrator: false,
        device_id: None,
    };
    let mut processing = Vec::new();
    for _ in 0..2 {
        let id = JobId::new();
        store
            .enqueue_job(JobSpec {
                id,
                user_id: actor.user_id,
                collection_id: Some(collection.id),
                item_id: None,
                kind: "test.processing".into(),
                phase: "queued".into(),
                total_units: 1,
                request: serde_json::json!({}),
                frozen_input: None,
            })
            .unwrap();
        processing.push(
            store
                .claim_job("shared-provider-test", 30)
                .unwrap()
                .unwrap(),
        );
    }
    let first_input = shared_provider_input(&collection, "same-disc");
    let first = service
        .enqueue_catalog_acquisition(&actor, first_input.clone(), &command())
        .unwrap();
    let second = service
        .enqueue_catalog_acquisition(
            &other,
            shared_provider_input(&collection, "same-disc"),
            &command(),
        )
        .unwrap();
    assert_ne!(first, second);
    assert!(service.job(&other, first).is_err());
    assert!(service.job(&actor, second).is_err());
    assert!(
        service.run_next_job("shared-provider-test").unwrap(),
        "provider slot stays available with two same-user processing jobs"
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        service.job(&actor, first).unwrap().state,
        JobState::Queued,
        "continuation needs a processing slot"
    );
    assert!(
        store
            .claim_job("shared-provider-test", 30)
            .unwrap()
            .is_none()
    );
    for claim in processing {
        store
            .complete_job(claim.id, &claim.claim_token, serde_json::json!({}))
            .unwrap();
    }
    provider_drain(&service);
    assert_eq!(
        completed_provider_snapshot(&service, &actor, first),
        completed_provider_snapshot(&service, &other, second)
    );
    assert_eq!(
        service
            .curation_catalog_snapshots(&actor, collection.id, None, 20)
            .unwrap()
            .items
            .len(),
        1
    );
    let third = service
        .enqueue_catalog_acquisition(
            &actor,
            shared_provider_input(&collection, "same-disc"),
            &command(),
        )
        .unwrap();
    provider_drain(&service);
    assert_eq!(
        calls.load(Ordering::SeqCst),
        2,
        "deliberate refresh downloads again"
    );
    assert_eq!(
        completed_provider_snapshot(&service, &actor, third),
        first_input.snapshot_id
    );
    assert!(matches!(
        service.job(&actor, third).unwrap().provider_status,
        Some(collector_app::ProviderResponseStatus::Unchanged)
    ));
    assert_eq!(
        service
            .curation_catalog_snapshots(&actor, collection.id, None, 20)
            .unwrap()
            .items
            .len(),
        1,
        "unchanged refresh does not append a snapshot"
    );
    store.reindex().unwrap();
    assert_eq!(
        completed_provider_snapshot(&service, &other, second),
        first_input.snapshot_id
    );
}

struct BlockingSharedProvider {
    started: Arc<AtomicBool>,
    release: Arc<AtomicBool>,
}
impl StreamingProviderPort for BlockingSharedProvider {
    fn acquire_stream(
        &self,
        _request: &ProviderRequest,
        execution: &mut dyn ProcessingExecutionContext,
    ) -> Result<ProviderStreamResponse> {
        self.started.store(true, Ordering::SeqCst);
        while !self.release.load(Ordering::SeqCst) {
            if execution.cancellation_requested()? {
                return Err(Error::JobCancelled);
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        let bytes = b"owned response".to_vec();
        Ok(ProviderStreamResponse {
            media_type: "application/octet-stream".into(),
            bytes: ByteLength(bytes.len() as u64),
            digest: Digest::sha256(&bytes),
            parsed_observation: RawJson::from_value(serde_json::json!({"outcome":"data"})),
            reader: Box::new(std::io::Cursor::new(bytes)),
        })
    }
}
#[test]
fn shared_provider_later_owner_survives_cancel_and_last_owner_fences_admission() {
    let clock = Arc::new(TestClock::new(1_800_000_000));
    let (_root, store, actor, base, collection) = setup_with_clock(Some(clock.clone()));
    let started = Arc::new(AtomicBool::new(false));
    let release = Arc::new(AtomicBool::new(false));
    let service = Arc::new(
        base.with_streaming_provider(
            ModuleId(ACCURATERIP_PROVIDER_ID.into()),
            Arc::new(BlockingSharedProvider {
                started: started.clone(),
                release: release.clone(),
            }),
        )
        .unwrap(),
    );
    let first = service
        .enqueue_catalog_acquisition(
            &actor,
            shared_provider_input(&collection, "owned"),
            &command(),
        )
        .unwrap();
    let run = service.clone();
    let worker = thread::spawn(move || run.run_next_job("shared-provider-test"));
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while !started.load(Ordering::SeqCst) {
        assert!(std::time::Instant::now() < deadline);
        thread::sleep(Duration::from_millis(5));
    }
    clock.advance(2);
    let second = service
        .enqueue_catalog_acquisition(
            &actor,
            shared_provider_input(&collection, "owned"),
            &command(),
        )
        .unwrap();
    service.cancel_job(&actor, first, &command()).unwrap();
    service.reconcile_provider_acquisitions().unwrap();
    assert_eq!(service.job(&actor, second).unwrap().state, JobState::Queued);
    service.cancel_job(&actor, second, &command()).unwrap();
    assert!(
        matches!(
            service.enqueue_catalog_acquisition(
                &actor,
                shared_provider_input(&collection, "owned"),
                &command()
            ),
            Err(Error::CommandInProgress)
        ),
        "cannot resurrect last-owner cancellation before settlement"
    );
    service.reconcile_provider_acquisitions().unwrap();
    assert!(worker.join().unwrap().unwrap());
    assert!(
        service
            .curation_catalog_snapshots(&actor, collection.id, None, 20)
            .unwrap()
            .items
            .is_empty()
    );
    release.store(true, Ordering::SeqCst);
    let third = service
        .enqueue_catalog_acquisition(
            &actor,
            shared_provider_input(&collection, "owned"),
            &command(),
        )
        .unwrap();
    provider_drain(&service);
    completed_provider_snapshot(&service, &actor, third);
    assert_eq!(
        service.job(&actor, first).unwrap().state,
        JobState::Cancelled
    );
    assert_eq!(
        service.job(&actor, second).unwrap().state,
        JobState::Cancelled
    );
    assert_eq!(
        store
            .jobs(actor.user_id)
            .unwrap()
            .iter()
            .filter(|j| j.user_id != actor.user_id)
            .count(),
        0
    );
}

#[test]
fn provider_response_identity_distinguishes_changed_bytes_inputs_and_collections() {
    let (_root, store, actor, base, collection) = setup();
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let build = |bytes: &[u8]| {
        CollectionService::new(store.clone(), Arc::new(Registry::builtins()))
            .with_streaming_provider(
                ModuleId(ACCURATERIP_PROVIDER_ID.into()),
                Arc::new(FixtureStreamingProvider {
                    calls: calls.clone(),
                    bytes: bytes.to_vec(),
                }),
            )
            .unwrap()
    };
    let service = build(b"one");
    let one = service
        .enqueue_catalog_acquisition(&actor, shared_provider_input(&collection, "a"), &command())
        .unwrap();
    provider_drain(&service);
    let other = base
        .create_collection(
            &test_authentication(&actor),
            typed_collection("Other provider collection"),
            &command(),
        )
        .unwrap();
    let two = service
        .enqueue_catalog_acquisition(&actor, shared_provider_input(&collection, "b"), &command())
        .unwrap();
    let three = service
        .enqueue_catalog_acquisition(&actor, shared_provider_input(&other, "a"), &command())
        .unwrap();
    provider_drain(&service);
    assert_ne!(
        completed_provider_snapshot(&service, &actor, one),
        completed_provider_snapshot(&service, &actor, two)
    );
    assert_ne!(
        completed_provider_snapshot(&service, &actor, one),
        completed_provider_snapshot(&service, &actor, three)
    );
    let changed = build(b"two");
    let four = changed
        .enqueue_catalog_acquisition(&actor, shared_provider_input(&collection, "a"), &command())
        .unwrap();
    provider_drain(&changed);
    assert_ne!(
        completed_provider_snapshot(&changed, &actor, four),
        completed_provider_snapshot(&service, &actor, one)
    );
    assert_eq!(calls.load(Ordering::SeqCst), 4);
}

#[test]
fn snapshot_duplicate_conversion_preserves_ids_rebuilds_and_requires_verified_backup() {
    snapshot_duplicate_conversion_case(None, ACCURATERIP_PROVIDER_ID);
    snapshot_duplicate_conversion_case(
        None,
        collector_modules::music::COVER_ART_ARCHIVE_PROVIDER_ID,
    );
}
fn snapshot_duplicate_conversion_case(
    fault: Option<collector_store::PublicationFaultPoint>,
    provider: &str,
) {
    let (root, store, actor, service, collection) = setup();
    let mut ids = Vec::new();
    let payload = b"existing duplicate response";
    for index in 0..2 {
        let id = SnapshotId::new();
        ids.push(id);
        let claim = snapshot_claim(&store, collection.id);
        store
            .publish_snapshot(
                &claim,
                SnapshotRecord {
                    acquisition_key: None,
                    id,
                    collection_id: collection.id,
                    provider: ModuleId(provider.into()),
                    media_type: "application/octet-stream".into(),
                    bytes: ByteLength(payload.len() as u64),
                    digest: Digest::sha256(payload),
                    parsed_observation: RawJson::from_value(
                        serde_json::json!({"outcome":"data","query":"identical"}),
                    ),
                    created_at: chrono::DateTime::from_timestamp(
                        1_800_000_000,
                        if index == 0 { 900_000_000 } else { 100_000_000 },
                    )
                    .unwrap(),
                },
                &mut std::io::Cursor::new(payload),
                &mut || Ok(false),
            )
            .unwrap();
        store
            .complete_job(claim.id, &claim.claim_token, serde_json::json!({}))
            .unwrap();
        // Legacy acquisition inputs were private. Establish exact provenance for conversion.
        let frozen = shared_provider_input(&collection, "legacy");
        let mut frozen = frozen;
        frozen.snapshot_id = id;
        frozen.request.provider = ModuleId(provider.into());
        frozen.request.operation = OperationId::new(format!("{provider}.acquire")).unwrap();
        let private = rusqlite::Connection::open(root.path().join("state/private.sqlite")).unwrap();
        private
            .execute(
                "INSERT INTO processing_jobs(job_id,frozen_input) VALUES(?,?)",
                rusqlite::params![
                    claim.id.to_string(),
                    serde_json::to_string(&collector_app::FrozenJobInput::Acquisition(frozen))
                        .unwrap()
                ],
            )
            .unwrap();
    }
    if fault.is_none() {
        let db = rusqlite::Connection::open(root.path().join("state/private.sqlite")).unwrap();
        let (job, raw): (String, String) = db
            .query_row(
                "SELECT job_id,frozen_input FROM processing_jobs LIMIT 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        let mut unknown: serde_json::Value = serde_json::from_str(&raw).unwrap();
        unknown["input"]["future_semantics"] = serde_json::json!({"keep_original_layout":true});
        db.execute(
            "UPDATE processing_jobs SET frozen_input=? WHERE job_id=?",
            rusqlite::params![unknown.to_string(), job],
        )
        .unwrap();
        let inspected = store.inspect_snapshot_duplicates().unwrap();
        assert_eq!(inspected.skipped.len(), 1);
        assert!(inspected.duplicates.is_empty());
        db.execute(
            "UPDATE processing_jobs SET frozen_input=? WHERE job_id=?",
            rusqlite::params![raw, job],
        )
        .unwrap();
    }
    if fault.is_none() {
        let collection_path = std::fs::read_dir(root.path().join("archive/collections"))
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        let unknown = collection_path
            .join("snapshots")
            .join(ids[0].to_string())
            .join("payload/future-layout.json");
        std::fs::write(&unknown, b"unknown data requires original source.bin").unwrap();
        let inspected = store.inspect_snapshot_duplicates().unwrap();
        assert!(inspected.skipped.contains(&ids[0]));
        assert!(inspected.duplicates.is_empty());
        assert!(unknown.exists());
        std::fs::remove_file(unknown).unwrap();
    }
    let plan = store.inspect_snapshot_duplicates().unwrap();
    assert_eq!(plan.duplicates.len(), 1);
    assert_eq!(
        plan.duplicates[0].canonical, ids[1],
        "fractional timestamps precede ID tie-breaking"
    );
    assert_eq!(plan.saved_bytes, payload.len() as u64);
    assert!(
        store
            .apply_snapshot_duplicates(&plan, &root.path().join("missing-backup"))
            .is_err()
    );
    let config = root.path().join("collector.toml");
    std::fs::write(&config, "fixture configuration").unwrap();
    let backup = root.path().join("backup");
    store.backup(&backup, &config).unwrap();
    if let Some(point) = fault {
        drop(service);
        drop(store);
        let failing = Store::open_with_publication_fault(
            root.path().join("archive"),
            root.path().join("state"),
            Arc::new(FailPublicationOnce {
                point,
                failed: AtomicBool::new(false),
            }),
        )
        .unwrap();
        assert!(failing.apply_snapshot_duplicates(&plan, &backup).is_err());
        drop(failing);
        let recovered =
            Store::open(root.path().join("archive"), root.path().join("state")).unwrap();
        recovered.apply_snapshot_duplicates(&plan, &backup).unwrap();
        recovered.apply_snapshot_duplicates(&plan, &backup).unwrap();
        for id in &ids {
            let mut bytes = vec![];
            recovered
                .snapshot_bytes(SnapshotQuery {
                    collection_id: collection.id,
                    snapshot_id: *id,
                })
                .unwrap()
                .read_to_end(&mut bytes)
                .unwrap();
            assert_eq!(bytes, payload);
        }
        return;
    }
    let backed_config = std::fs::read(backup.join("collector.toml")).unwrap();
    std::fs::write(backup.join("collector.toml"), b"corrupted backup").unwrap();
    assert!(store.apply_snapshot_duplicates(&plan, &backup).is_err());
    std::fs::write(backup.join("collector.toml"), backed_config).unwrap();
    let extra = root.path().join("state/unbacked-unknown-data");
    std::fs::write(&extra, b"must preserve").unwrap();
    assert!(store.apply_snapshot_duplicates(&plan, &backup).is_err());
    std::fs::remove_file(extra).unwrap();
    store.apply_snapshot_duplicates(&plan, &backup).unwrap();
    store.apply_snapshot_duplicates(&plan, &backup).unwrap();
    let collection_path = std::fs::read_dir(root.path().join("archive/collections"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let reference = collection_path
        .join("snapshots")
        .join(plan.duplicates[0].snapshot.to_string())
        .join("payload/reference.v1.json");
    let saved_reference = std::fs::read(&reference).unwrap();
    let mut corrupt: serde_json::Value = serde_json::from_slice(&saved_reference).unwrap();
    corrupt["checksum"]["hex"] = serde_json::json!("00".repeat(32));
    std::fs::write(&reference, serde_json::to_vec(&corrupt).unwrap()).unwrap();
    assert!(
        store
            .snapshot_bytes(SnapshotQuery {
                collection_id: collection.id,
                snapshot_id: plan.duplicates[0].snapshot
            })
            .is_err()
    );
    std::fs::write(reference, saved_reference).unwrap();
    assert_eq!(
        service
            .curation_catalog_snapshots(&actor, collection.id, None, 20)
            .unwrap()
            .items
            .len(),
        2,
        "historical IDs remain"
    );
    for id in &ids {
        let mut bytes = vec![];
        store
            .snapshot_bytes(SnapshotQuery {
                collection_id: collection.id,
                snapshot_id: *id,
            })
            .unwrap()
            .read_to_end(&mut bytes)
            .unwrap();
        assert_eq!(bytes, payload);
    }
    assert!(
        store
            .inspect_snapshot_duplicates()
            .unwrap()
            .duplicates
            .is_empty()
    );
    let reuse = |store: &Store| {
        let mut incoming = store
            .snapshot(SnapshotQuery {
                collection_id: collection.id,
                snapshot_id: plan.duplicates[0].canonical,
            })
            .unwrap();
        incoming.id = SnapshotId::new();
        incoming.created_at = chrono::Utc::now();
        incoming.acquisition_key = Some(plan.duplicates[0].input_key.clone());
        let claim = snapshot_claim(store, collection.id);
        let retained = store
            .publish_snapshot(
                &claim,
                incoming,
                &mut std::io::Cursor::new(payload),
                &mut || Ok(false),
            )
            .unwrap();
        assert_eq!(
            retained, plan.duplicates[0].canonical,
            "fresh bytes reuse proven legacy canonical content"
        );
        store
            .complete_job(
                claim.id,
                &claim.claim_token,
                serde_json::json!({"snapshot_id":retained}),
            )
            .unwrap();
    };
    reuse(&store);
    drop(service);
    drop(store);
    // Restore the verified pre-conversion backup into empty roots with the
    // original private identity stopped, and prove its accounts and bytes work.
    let restored_archive = root.path().join("restored/archive");
    let restored_state = root.path().join("restored/state");
    copy_tree(&backup.join("archive"), &restored_archive);
    copy_tree(&backup.join("state"), &restored_state);
    assert_eq!(
        tree_hashes(&backup.join("archive")),
        tree_hashes(&restored_archive)
    );
    assert_eq!(
        tree_hashes(&backup.join("state")),
        tree_hashes(&restored_state)
    );
    let restored = Store::open(&restored_archive, &restored_state).unwrap();
    restored
        .login(
            "owner",
            "a long test password",
            "restored conversion backup",
        )
        .unwrap();
    assert_eq!(
        Catalog::role(&restored, actor.user_id, collection.id).unwrap(),
        Role::Owner
    );
    for id in &ids {
        let mut bytes = vec![];
        restored
            .snapshot_bytes(SnapshotQuery {
                collection_id: collection.id,
                snapshot_id: *id,
            })
            .unwrap()
            .read_to_end(&mut bytes)
            .unwrap();
        assert_eq!(bytes, payload);
    }
    drop(restored);
    let archive_only = Store::open(
        root.path().join("archive"),
        root.path().join("archive-only"),
    )
    .unwrap();
    reuse(&archive_only);
    for id in ids {
        let mut bytes = vec![];
        archive_only
            .snapshot_bytes(SnapshotQuery {
                collection_id: collection.id,
                snapshot_id: id,
            })
            .unwrap()
            .read_to_end(&mut bytes)
            .unwrap();
        assert_eq!(bytes, payload);
    }
}

#[test]
fn snapshot_duplicate_conversion_recovers_every_mutation_boundary() {
    use collector_store::PublicationFaultPoint as Point;
    for point in [
        Point::Prepared,
        Point::BytesStaged,
        Point::PayloadPublished,
        Point::ProjectionApplied,
    ] {
        snapshot_duplicate_conversion_case(Some(point), ACCURATERIP_PROVIDER_ID);
        snapshot_duplicate_conversion_case(
            Some(point),
            collector_modules::music::COVER_ART_ARCHIVE_PROVIDER_ID,
        );
    }
}

#[test]
fn same_user_parallel_tracks_publish_without_losing_capture_outputs() {
    let (_root, store, actor, service, collection) = setup();
    let imported = derivative_import(&service, &actor, collection.id);
    let service = Arc::new(
        service.with_audio_converter(Arc::new(ConcurrentDerivativeEncoder(
            std::sync::Barrier::new(2),
        ))),
    );
    let jobs: [JobId; 2] = [1, 2].map(|position| {
        service
            .prepare_music_derivative(
                &actor,
                imported.capture_id,
                derivative_command(&imported, position),
                &command(),
            )
            .unwrap()
    });
    thread::scope(|scope| {
        for _ in 0..2 {
            let service = &service;
            scope.spawn(move || service.run_next_job("parallel-tracks").unwrap());
        }
    });
    for job in jobs {
        assert_eq!(service.job(&actor, job).unwrap().state, JobState::Completed);
    }
    let capture = store.capture(imported.capture_id).unwrap();
    assert_eq!(capture.representations.len(), 3);
    store.reindex().unwrap();
    assert_eq!(
        store
            .capture(imported.capture_id)
            .unwrap()
            .representations
            .len(),
        3
    );
}

#[test]
#[ignore = "manual local performance qualification with real FLAC"]
fn processing_concurrency_local_benchmark() {
    let workers: usize = std::env::var("COLLECTOR_BENCH_WORKERS")
        .unwrap_or("1".into())
        .parse()
        .unwrap();
    assert!((1..=2).contains(&workers));
    let tool = collector_providers::ConfiguredEncoder::probe(
        collector_providers::EncoderKind::Flac,
        "flac",
    )
    .unwrap();
    let version = tool.version().to_string();
    let converter = collector_providers::ProcessAudioConverter::new(Some(tool), None).unwrap();
    let (_root, store, actor, service, collection) = setup();
    let imported = derivative_import_sectors(&service, &actor, collection.id, 0, 13500);
    let service = Arc::new(service.with_audio_converter(Arc::new(converter)));
    let jobs: Vec<_> = [1, 2]
        .into_iter()
        .map(|position| {
            let mut input = derivative_command(&imported, position);
            input.encoder_version = version.clone();
            service
                .prepare_music_derivative(&actor, imported.capture_id, input, &command())
                .unwrap()
        })
        .collect();
    let start = std::time::Instant::now();
    let mut queue_ms = std::collections::BTreeMap::new();
    thread::scope(|scope| {
        for _ in 0..workers {
            let service = &service;
            scope.spawn(move || while service.run_next_job("local-benchmark").unwrap() {});
        }
        loop {
            let mut settled = true;
            for job in &jobs {
                let view = service.job(&actor, *job).unwrap();
                if view.state != JobState::Queued {
                    queue_ms
                        .entry(*job)
                        .or_insert_with(|| start.elapsed().as_millis());
                }
                if matches!(view.state, JobState::Queued | JobState::Running) {
                    settled = false;
                }
            }
            if settled {
                break;
            }
            thread::sleep(Duration::from_millis(2));
        }
    });
    eprintln!(
        "queue_to_claim_ms={:?} (2ms polling resolution)",
        queue_ms.values().collect::<Vec<_>>()
    );
    for job in jobs {
        let view = service.job(&actor, job).unwrap();
        assert_eq!(view.state, JobState::Completed, "{:?}", view.error);
    }
    let source = store.capture(imported.capture_id).unwrap().representations[0].clone();
    let inventory = store
        .package_inventory(collection.id, PackageScope::Representation(source.id))
        .unwrap();
    let source_bytes: u64 = inventory.files.iter().map(|file| file.bytes.0).sum();
    eprintln!(
        "workers={workers} album_ms={} source_bytes={source_bytes} full_validation_bytes={} (two complete validations per track; codec input/output additional)",
        start.elapsed().as_millis(),
        source_bytes * 4
    );
}

#[test]
fn expired_processing_claim_cannot_exceed_capacity_after_older_waiters_resume() {
    let root = tempfile::tempdir().unwrap();
    let clock = Arc::new(TestClock::new(1_800_000_000));
    let store = Store::open_with_clock(
        root.path().join("archive"),
        root.path().join("state"),
        clock.clone(),
    )
    .unwrap();
    let input = |phase: &str| JobSpec {
        id: JobId::new(),
        user_id: UserId::new(),
        collection_id: None,
        item_id: None,
        kind: "test.processing".into(),
        phase: phase.into(),
        total_units: 1,
        request: serde_json::json!({}),
        frozen_input: None,
    };
    let first = input("waiting_fixture");
    let second = input("waiting_fixture");
    store.enqueue_job(first.clone()).unwrap();
    store.enqueue_job(second.clone()).unwrap();
    clock.advance(1);
    let expired = input("queued");
    store.enqueue_job(expired.clone()).unwrap();
    let old = store.claim_job("same-instance", 30).unwrap().unwrap();
    assert_eq!(old.id, expired.id);
    clock.advance(31);
    assert!(
        store.renew_job_lease(old.id, &old.claim_token, 30).is_err(),
        "expired claims cannot revive after capacity is reclaimed"
    );
    assert!(
        store
            .cancellation_requested(old.id, &old.claim_token)
            .is_err(),
        "expired worker work must stop before publication"
    );
    let frozen = FrozenAcquisitionJob {
        collection_binding: CollectionBinding::LegacyUnassigned,
        request: ProviderRequest {
            collection_id: CollectionId::new(),
            provider: ModuleId(ACCURATERIP_PROVIDER_ID.into()),
            operation: OperationId::new("org.junk.providers.accuraterip.acquire").unwrap(),
            source_fingerprint: None,
            query: RawJson::from_value(serde_json::json!({"disc":"expired"})),
            maximum_response_bytes: 1024,
        },
        item_id: None,
        snapshot_id: SnapshotId::new(),
        created_at: chrono::Utc::now(),
    };
    assert!(store.acquisition_for_job(&old, &frozen).is_err());
    assert!(store.acquisition_owners(&old).is_err());
    let db = rusqlite::Connection::open(root.path().join("state/private.sqlite")).unwrap();
    let links: u64 = db
        .query_row("SELECT COUNT(*) FROM acquisition_links", [], |r| r.get(0))
        .unwrap();
    assert_eq!(links, 0, "an expired owner cannot admit shared work");
    db.execute(
        "UPDATE jobs SET phase='queued' WHERE phase='waiting_fixture'",
        [],
    )
    .unwrap();
    let active = store.claim_job("same-instance", 30).unwrap().unwrap();
    let other = store.claim_job("same-instance", 30).unwrap().unwrap();
    assert!(
        store.claim_job("same-instance", 30).unwrap().is_none(),
        "expired execution must wait for available capacity"
    );
    store
        .complete_job(active.id, &active.claim_token, serde_json::json!({}))
        .unwrap();
    let recovered = store.claim_job("same-instance", 30).unwrap().unwrap();
    assert_eq!(recovered.id, expired.id);
    assert_ne!(recovered.claim_token, old.claim_token);
    assert!(
        store
            .complete_job(old.id, &old.claim_token, serde_json::json!({}))
            .is_err()
    );
    store
        .complete_job(recovered.id, &recovered.claim_token, serde_json::json!({}))
        .unwrap();
    store
        .complete_job(other.id, &other.claim_token, serde_json::json!({}))
        .unwrap();
}

#[test]
fn existing_queued_provider_requests_join_registry_on_upgrade_without_using_processing_capacity() {
    let (root, store, actor, service, collection) = setup();
    let first = service
        .enqueue_catalog_acquisition(
            &actor,
            shared_provider_input(&collection, "before-registry"),
            &command(),
        )
        .unwrap();
    let second = service
        .enqueue_catalog_acquisition(
            &actor,
            shared_provider_input(&collection, "before-registry"),
            &command(),
        )
        .unwrap();
    let created = service.job(&actor, first).unwrap().created_at;
    drop(service);
    drop(store);
    let db = rusqlite::Connection::open(root.path().join("state/private.sqlite")).unwrap();
    db.execute_batch("DELETE FROM acquisition_waits; DELETE FROM acquisition_links; DELETE FROM processing_jobs WHERE job_id IN (SELECT id FROM jobs WHERE kind='provider_acquisition'); DELETE FROM jobs WHERE kind='provider_acquisition'; UPDATE jobs SET phase='queued' WHERE kind='catalog_acquisition'; PRAGMA user_version=19;").unwrap();
    drop(db);
    let store =
        Arc::new(Store::open(root.path().join("archive"), root.path().join("state")).unwrap());
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let service = CollectionService::new(store.clone(), Arc::new(Registry::builtins()))
        .with_streaming_provider(
            ModuleId(ACCURATERIP_PROVIDER_ID.into()),
            Arc::new(FixtureStreamingProvider {
                calls: calls.clone(),
                bytes: b"upgraded response".to_vec(),
            }),
        )
        .unwrap();
    assert_eq!(service.job(&actor, first).unwrap().created_at, created);
    let mut claims = Vec::new();
    for _ in 0..2 {
        let id = JobId::new();
        store
            .enqueue_job(JobSpec {
                id,
                user_id: actor.user_id,
                collection_id: Some(collection.id),
                item_id: None,
                kind: "test.processing".into(),
                phase: "queued".into(),
                total_units: 1,
                request: serde_json::json!({}),
                frozen_input: None,
            })
            .unwrap();
        // Temporarily claim CPU fixtures explicitly before the older download class.
        let db = rusqlite::Connection::open(root.path().join("state/private.sqlite")).unwrap();
        db.execute(
            "UPDATE jobs SET phase='waiting_fixture' WHERE kind='provider_acquisition'",
            [],
        )
        .unwrap();
        claims.push(store.claim_job("upgrade", 30).unwrap().unwrap());
    }
    let db = rusqlite::Connection::open(root.path().join("state/private.sqlite")).unwrap();
    db.execute(
        "UPDATE jobs SET phase='queued' WHERE kind='provider_acquisition'",
        [],
    )
    .unwrap();
    drop(db);
    service.run_next_job("upgrade").unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    for claim in claims {
        store
            .complete_job(claim.id, &claim.claim_token, serde_json::json!({}))
            .unwrap();
    }
    provider_drain(&service);
    assert_eq!(
        completed_provider_snapshot(&service, &actor, first),
        completed_provider_snapshot(&service, &actor, second)
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn automatic_game_scan_recovers_later_pages_and_retains_direct_owners_and_manual_fields() {
    let clock = Arc::new(TestClock::new(1_800_000_000));
    let (root, store, actor, base, _) = setup_with_clock(Some(clock.clone()));
    let collection = base
        .create_collection(
            &AuthenticatedPrincipal {
                principal: actor.clone(),
                credential_kind: CredentialKind::TestLocal,
                authenticated_at: clock.now(),
            },
            CreateCollectionCommand {
                name: "Automatic games".into(),
                module: ModuleId("org.junk.games".into()),
            },
            &command(),
        )
        .unwrap();
    let configure = |store: Arc<Store>| {
        let provider = Arc::new(
            LocalDatProvider::new(Default::default())
                .unwrap()
                .with_uploads(store.clone()),
        );
        CollectionService::new(store.clone(), Arc::new(Registry::builtins()))
            .with_catalog_uploads(store)
            .with_catalog_inspection(Arc::new(collector_providers::StandardCatalogInspector))
            .with_streaming_provider(ModuleId(LOCAL_DAT_PROVIDER_ID.into()), provider.clone())
            .unwrap()
            .with_catalog_parser(ModuleId(LOCAL_DAT_PROVIDER_ID.into()), provider)
            .unwrap()
    };
    let service = configure(store.clone());
    let rom = b"abc";
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
                        package_name: "disc.bin".into(),
                    },
                    files: vec![UploadFile {
                        name: "disc.bin".into(),
                        bytes: ByteLength(3),
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
                clean_reimport: None,
                plan,
                analysis: collector_app::IngestAnalysisPolicy::Automatic,
                enrichment: IngestEnrichmentPolicy::None,
                prepare_music: false,
                destination: None,
            },
            &command(),
        )
        .unwrap();
    let import = accepted.imports[0].import_id;
    service
        .stage_provisional_import_chunk(&actor, import, 0, 0, rom)
        .unwrap();
    provider_drain(&service);
    let game = service
        .ingest_detail(&actor, import)
        .unwrap()
        .summary
        .target_item_id
        .unwrap();
    assert_eq!(
        service
            .metadata_review(&actor, game)
            .unwrap()
            .display
            .title
            .source,
        MetadataValueSource::Provisional
    );
    for number in 1..=101 {
        let mut empty = item(collection.id);
        empty.id = ItemId(uuid::Uuid::from_u128(number));
        empty.domain = ModuleData {
            module: ModuleId("org.junk.games".into()),
            schema_version: 1,
            payload: RawJson::from_value(serde_json::json!({})),
        };
        Catalog::save(&*store, empty, Revision(0)).unwrap();
    }
    let dat = |version: &str| {
        format!(r#"<datafile><header><name>Game Boy</name><author>No-Intro</author><version>{version}</version></header><game name="Known game"><rom name="game.gb" size="3" sha1="a9993e364706816aba3e25717850c26c9cd0d89d"/></game></datafile>"#).into_bytes()
    };
    let preview = service
        .inspect_catalog_batch(
            &actor,
            collection.id,
            vec![("fixture.dat".into(), dat("1"))],
        )
        .unwrap();
    let upload = preview.entries[0].upload.clone().unwrap();
    let job = service
        .import_uploaded_catalog(&actor, collection.id, upload, &command())
        .unwrap();
    provider_drain(&service);
    let activation = service
        .activate_catalog_snapshot(&actor, collection.id, SnapshotId(job.0), &command())
        .unwrap();
    let parent = activation.identification_job_id.unwrap();
    assert!(service.run_next_job("first-page").unwrap());
    let record = store.job_record(parent, actor.user_id).unwrap();
    assert_eq!(record.state, JobState::Queued);
    assert!(record.result.unwrap().get("cursor").is_some());
    drop(service);
    drop(base);
    drop(store);
    clock.advance(60);
    let store = Arc::new(
        Store::open_with_clock(
            root.path().join("archive"),
            root.path().join("state"),
            clock.clone(),
        )
        .unwrap(),
    );
    let service = configure(store.clone());
    assert!(service.run_next_job("later-page").unwrap());
    let direct_context = command();
    let direct = service
        .identify_game_entry(&actor, game, &direct_context)
        .unwrap();
    assert_eq!(direct.len(), 1);
    let created = service.job(&actor, direct[0]).unwrap().created_at;
    clock.advance(60);
    assert_eq!(
        service
            .identify_game_entry(&actor, game, &direct_context)
            .unwrap(),
        direct
    );
    assert_eq!(service.job(&actor, direct[0]).unwrap().created_at, created);
    service.cancel_job(&actor, parent, &command()).unwrap();
    assert_ne!(
        service.job(&actor, direct[0]).unwrap().state,
        JobState::Cancelled
    );
    let review = service.metadata_review(&actor, game).unwrap();
    service
        .update_metadata_authority(
            &actor,
            game,
            UpdateMetadataAuthorityCommand {
                expected_revision: review.revision,
                change: MetadataAuthorityChange::SetFieldOverride {
                    field: MetadataField::Title,
                    value: "My deliberate title".into(),
                    language: None,
                },
            },
            &command(),
        )
        .unwrap();
    provider_drain(&service);
    assert_eq!(
        service.job(&actor, direct[0]).unwrap().state,
        JobState::Completed
    );
    let review = service.metadata_review(&actor, game).unwrap();
    assert_eq!(review.display.title.value, "My deliberate title");
    assert_eq!(
        review.display.domain_fields["platform"].value,
        "Nintendo - Game Boy"
    );
    assert!(matches!(
        review.identification.unwrap().status,
        collector_modules::games::GameDetectionStatus::Recognized
    ));
    let preview = service
        .inspect_catalog_batch(
            &actor,
            collection.id,
            vec![("replacement.dat".into(), dat("2"))],
        )
        .unwrap();
    let replacement = service
        .import_uploaded_catalog(
            &actor,
            collection.id,
            preview.entries[0].upload.clone().unwrap(),
            &command(),
        )
        .unwrap();
    provider_drain(&service);
    let activation = service
        .activate_catalog_snapshot(&actor, collection.id, SnapshotId(replacement.0), &command())
        .unwrap();
    provider_drain(&service);
    assert_eq!(
        service
            .job(&actor, activation.identification_job_id.unwrap())
            .unwrap()
            .state,
        JobState::Completed
    );
    store.reindex().unwrap();
    assert_eq!(
        service
            .metadata_review(&actor, game)
            .unwrap()
            .display
            .title
            .value,
        "My deliberate title"
    );
    assert_eq!(
        store.active_game_snapshots(collection.id).unwrap(),
        vec![SnapshotId(replacement.0)]
    );

    // A source mutation cannot publish a successful identity even if its recorded fingerprint is unchanged.
    let next = service
        .identify_game_entry(&actor, game, &command())
        .unwrap()[0];
    let current = store.item(game).unwrap();
    let capture = &current.captures[0];
    let representation = &capture.representations[0];
    let collection_path = std::fs::read_dir(root.path().join("archive/collections"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.join(format!("captures/{}", capture.id)).exists())
        .unwrap();
    let payload = collection_path.join(format!(
        "captures/{}/payload/{}/disc.bin",
        capture.id, representation.id
    ));
    std::fs::write(&payload, b"xyz").unwrap();
    provider_drain(&service);
    assert_eq!(service.job(&actor, next).unwrap().state, JobState::Failed);
    std::fs::write(&payload, rom).unwrap();
    let curator_id = store
        .create_user("game-curator", "a long curator password", false)
        .unwrap();
    Catalog::grant(&*store, collection.id, curator_id, Role::Curator).unwrap();
    let curator = Principal {
        user_id: curator_id,
        administrator: false,
        device_id: None,
    };
    let denied = service
        .identify_game_entry(&curator, game, &command())
        .unwrap()[0];
    Catalog::grant(&*store, collection.id, curator_id, Role::Viewer).unwrap();
    provider_drain(&service);
    assert_eq!(
        store.job_record(denied, curator_id).unwrap().state,
        JobState::Failed
    );
    let preview = service
        .inspect_catalog_batch(&actor, collection.id, vec![("third.dat".into(), dat("3"))])
        .unwrap();
    let third = service
        .import_uploaded_catalog(
            &actor,
            collection.id,
            preview.entries[0].upload.clone().unwrap(),
            &command(),
        )
        .unwrap();
    provider_drain(&service);
    let stale = service
        .identify_game_entry(&actor, game, &command())
        .unwrap()[0];
    service
        .activate_catalog_snapshot(&actor, collection.id, SnapshotId(third.0), &command())
        .unwrap();
    provider_drain(&service);
    assert_eq!(service.job(&actor, stale).unwrap().state, JobState::Failed);
    let last_owner = service
        .identify_game_entry(&actor, game, &command())
        .unwrap()[0];
    service.cancel_job(&actor, last_owner, &command()).unwrap();
    assert_eq!(
        service.job(&actor, last_owner).unwrap().state,
        JobState::Cancelled
    );
    assert_eq!(
        service
            .metadata_review(&actor, game)
            .unwrap()
            .display
            .title
            .value,
        "My deliberate title"
    );
}

#[test]
fn cover_art_retention_reuses_responses_with_item_owned_assets_and_durable_retries() {
    use collector_app::{AcquireMetadataCommand, ResolveMetadataCommand};
    use collector_modules::music::{COVER_ART_ARCHIVE_PROVIDER_ID, MUSICBRAINZ_PROVIDER_ID};
    let clock = Arc::new(TestClock::new(1_800_000_000));
    let (root, store, actor, service, collection) = setup_with_clock(Some(clock.clone()));
    let items = [
        save_inventory(&service, &actor, item(collection.id), Revision(0)).unwrap(),
        save_inventory(&service, &actor, item(collection.id), Revision(0)).unwrap(),
    ];
    let release = "22222222-2222-4222-8222-222222222222";
    let mb_body = serde_json::to_vec(
        &serde_json::json!({"releases":[{"id":release,"title":"Retained album"}]}),
    )
    .unwrap();
    let mb_fixture =
        metadata_fixture_server(vec![("200 OK\r\nContent-Type: application/json", mb_body)]);
    let mb = Arc::new(
        MusicBrainzProvider::with_test_origin(
            "collector-test/0.1 (test@example.invalid)",
            &mb_fixture.origin,
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
    let image = encoded.into_inner();
    let mut changed = std::io::Cursor::new(Vec::new());
    image::RgbImage::from_pixel(24, 24, image::Rgb([56, 34, 12]))
        .write_to(&mut changed, image::ImageFormat::Png)
        .unwrap();
    let changed = changed.into_inner();
    let listing = serde_json::to_vec(&serde_json::json!({"release":format!("https://musicbrainz.org/release/{release}"),"images":[{"id":"9001","front":true,"back":false,"approved":true,"types":["Front"]}]})).unwrap();
    let mut replies = vec![
        ("200 OK\r\nContent-Type: application/json", listing.clone()),
        ("200 OK\r\nContent-Type: application/json", listing),
    ];
    replies.extend((0..4).map(|_| ("200 OK\r\nContent-Type: image/png", image.clone())));
    replies.push(("200 OK\r\nContent-Type: image/png", changed));
    replies.extend((0..2).map(|_| {
        (
            "404 Not Found\r\nContent-Type: text/html",
            b"no artwork".to_vec(),
        )
    }));
    // A pending request resumes after reopening and returns to the older, exact image.
    replies.push(("200 OK\r\nContent-Type: image/png", image.clone()));
    replies.push(("200 OK\r\nContent-Type: image/png", image.clone()));
    let fixture = metadata_fixture_server(replies);
    let caa = Arc::new(
        CoverArtArchiveProvider::with_test_origins(
            "collector-test/0.1 (test@example.invalid)",
            &fixture.origin,
            &[fixture.origin.as_str()],
            Duration::from_secs(1),
        )
        .unwrap(),
    );
    let configure = |service: CollectionService| {
        service
            .with_streaming_provider(
                ModuleId(COVER_ART_ARCHIVE_PROVIDER_ID.into()),
                Arc::new(collector_app::BoundedProviderAdapter(caa.clone())),
            )
            .unwrap()
            .with_catalog_parser(ModuleId(COVER_ART_ARCHIVE_PROVIDER_ID.into()), caa.clone())
            .unwrap()
    };
    let service = configure(
        service
            .with_streaming_provider(
                ModuleId(MUSICBRAINZ_PROVIDER_ID.into()),
                Arc::new(collector_app::BoundedProviderAdapter(mb.clone())),
            )
            .unwrap()
            .with_catalog_parser(ModuleId(MUSICBRAINZ_PROVIDER_ID.into()), mb)
            .unwrap(),
    );
    let lookup = ResolveMetadataCommand {
        lookup: AcquireMetadataCommand::MusicBrainzSearch {
            query: "retained album".into(),
            refresh: true,
        },
        acquire_artwork: false,
    };
    let text_jobs: Vec<_> = items
        .iter()
        .map(|item| {
            service
                .resolve_metadata(&actor, item.id, lookup.clone(), &command())
                .unwrap()
        })
        .collect();
    provider_drain(&service);
    for job in text_jobs {
        assert_eq!(service.job(&actor, job).unwrap().state, JobState::Completed);
    }
    let candidate = service
        .metadata_view(&actor, items[0].id)
        .unwrap()
        .candidates[0]
        .identity
        .clone();
    let listing_lookup = AcquireMetadataCommand::CoverArtListing {
        candidate: candidate.clone(),
    };
    let image_lookup = AcquireMetadataCommand::CoverArtImage {
        candidate,
        image_id: "9001".into(),
    };
    let resolve = |who: &Principal, item_id, lookup| {
        clock.advance(60);
        let job = service
            .resolve_metadata(
                who,
                item_id,
                ResolveMetadataCommand {
                    lookup,
                    acquire_artwork: false,
                },
                &command(),
            )
            .unwrap();
        provider_drain(&service);
        let view = service
            .metadata_resolutions(who, item_id)
            .unwrap()
            .into_iter()
            .find(|value| value.job.id == job)
            .unwrap();
        assert_eq!(view.job.state, JobState::Completed, "{:?}", view.job.error);
        view.result.unwrap().snapshot_id
    };
    let listing_id = resolve(&actor, items[0].id, listing_lookup.clone());
    assert_eq!(
        resolve(&actor, items[0].id, listing_lookup.clone()),
        listing_id
    );
    let image_id = resolve(&actor, items[0].id, image_lookup.clone());
    let first = service.metadata_view(&actor, items[0].id).unwrap();
    let first_asset = first.authority.artwork_choice.unwrap();
    assert_eq!(resolve(&actor, items[0].id, image_lookup.clone()), image_id);
    assert_eq!(
        service.metadata_view(&actor, items[0].id).unwrap().revision,
        first.revision
    );
    assert_eq!(resolve(&actor, items[1].id, image_lookup.clone()), image_id);
    let second_asset = service
        .metadata_view(&actor, items[1].id)
        .unwrap()
        .authority
        .artwork_choice
        .unwrap();
    assert_ne!(
        first_asset, second_asset,
        "shared provider bytes must not share item ownership"
    );
    let other_id = store
        .create_user("cover-curator", "a long test password", false)
        .unwrap();
    Catalog::grant(&*store, collection.id, other_id, Role::Curator).unwrap();
    let other = Principal {
        user_id: other_id,
        administrator: false,
        device_id: None,
    };
    assert_eq!(resolve(&other, items[0].id, image_lookup.clone()), image_id);
    assert_eq!(
        store.asset_states_for_item(items[0].id).unwrap().len(),
        1,
        "requester identity must not duplicate a cover"
    );
    let changed_id = resolve(&actor, items[0].id, image_lookup.clone());
    assert_ne!(changed_id, image_id);
    assert_eq!(
        store.asset_states_for_item(items[0].id).unwrap().len(),
        2,
        "changed artwork retains its own source and Asset"
    );
    let no_data = resolve(&actor, items[0].id, listing_lookup.clone());
    assert_ne!(no_data, listing_id);
    assert_eq!(resolve(&actor, items[0].id, listing_lookup), no_data);
    let pending_request = ResolveMetadataCommand {
        lookup: image_lookup,
        acquire_artwork: false,
    };
    let receipt = command();
    let pending = service
        .resolve_metadata(&actor, items[0].id, pending_request.clone(), &receipt)
        .unwrap();
    let cancelled = service
        .resolve_metadata(&actor, items[1].id, pending_request.clone(), &command())
        .unwrap();
    service.cancel_job(&actor, cancelled, &command()).unwrap();
    store.reindex().unwrap();
    assert_eq!(
        service
            .metadata_view(&actor, items[0].id)
            .unwrap()
            .display
            .title
            .value,
        items[0].title
    );
    drop(service);
    drop(store);
    let store = Arc::new(
        Store::open_with_clock(
            root.path().join("archive"),
            root.path().join("state"),
            clock,
        )
        .unwrap(),
    );
    let service = configure(CollectionService::new(
        store.clone(),
        Arc::new(Registry::builtins()),
    ));
    assert_eq!(
        service
            .resolve_metadata(&actor, items[0].id, pending_request.clone(), &receipt)
            .unwrap(),
        pending
    );
    provider_drain(&service);
    let view = service
        .metadata_resolutions(&actor, items[0].id)
        .unwrap()
        .into_iter()
        .find(|value| value.job.id == pending)
        .unwrap();
    assert_eq!(view.job.state, JobState::Completed, "{:?}", view.job.error);
    assert_eq!(view.result.unwrap().snapshot_id, image_id);
    assert_eq!(
        service.job(&actor, cancelled).unwrap().state,
        JobState::Cancelled
    );
    assert_eq!(
        service
            .metadata_view(&actor, items[0].id)
            .unwrap()
            .authority
            .artwork_choice,
        Some(first_asset)
    );
    assert_eq!(store.asset_states_for_item(items[0].id).unwrap().len(), 2);
    // Access loss cannot turn shared response ownership into a new write grant.
    AccessPort::set_grant(&*store, collection.id, other_id, Revision(1), None).unwrap();
    assert!(matches!(
        service.resolve_metadata(&other, items[0].id, pending_request.clone(), &command()),
        Err(Error::Forbidden)
    ));
    let payload = files_named(&root.path().join("archive"), "source.bin")
        .into_iter()
        .find(|path| {
            path.components()
                .any(|part| part.as_os_str() == image_id.to_string().as_str())
        })
        .unwrap();
    std::fs::write(payload, vec![0; image.len()]).unwrap();
    let corrupt = service
        .resolve_metadata(&actor, items[0].id, pending_request, &command())
        .unwrap();
    provider_drain(&service);
    assert_eq!(
        service.job(&actor, corrupt).unwrap().state,
        JobState::Failed,
        "corrupt retained bytes must never become an unchanged success"
    );
    assert_eq!(
        service
            .metadata_view(&actor, items[0].id)
            .unwrap()
            .authority
            .artwork_choice,
        Some(first_asset)
    );
    assert_eq!(fixture.requests.lock().unwrap().len(), 11);
    assert_eq!(
        service
            .curation_catalog_snapshots(&actor, collection.id, None, 100)
            .unwrap()
            .items
            .len(),
        5,
        "one text response and four distinct artwork responses"
    );
}

struct CoverArtReviewFixture {
    root: tempfile::TempDir,
    store: Arc<Store>,
    actor: Principal,
    service: CollectionService,
    collection: Collection,
    item: Item,
    image_command: collector_app::ResolveMetadataCommand,
    clock: Arc<TestClock>,
    _musicbrainz: MetadataFixtureServer,
    artwork: MetadataFixtureServer,
    artwork_provider: Arc<CoverArtArchiveProvider>,
}

impl CoverArtReviewFixture {
    fn reopen(self) -> Self {
        let Self {
            root,
            store,
            actor,
            service,
            collection,
            item,
            image_command,
            clock,
            _musicbrainz,
            artwork,
            artwork_provider,
        } = self;
        drop(service);
        drop(store);
        let store = Arc::new(
            Store::open_with_clock(
                root.path().join("archive"),
                root.path().join("state"),
                clock.clone(),
            )
            .unwrap(),
        );
        let service = CollectionService::new(store.clone(), Arc::new(Registry::builtins()))
            .with_streaming_provider(
                ModuleId(collector_modules::music::COVER_ART_ARCHIVE_PROVIDER_ID.into()),
                Arc::new(collector_app::BoundedProviderAdapter(
                    artwork_provider.clone(),
                )),
            )
            .unwrap()
            .with_catalog_parser(
                ModuleId(collector_modules::music::COVER_ART_ARCHIVE_PROVIDER_ID.into()),
                artwork_provider.clone(),
            )
            .unwrap();
        Self {
            root,
            store,
            actor,
            service,
            collection,
            item,
            image_command,
            clock,
            _musicbrainz,
            artwork,
            artwork_provider,
        }
    }
}

fn review_cover_image() -> Vec<u8> {
    let mut encoded = std::io::Cursor::new(Vec::new());
    image::RgbImage::from_pixel(1024, 1024, image::Rgb([12, 34, 56]))
        .write_to(&mut encoded, image::ImageFormat::Png)
        .unwrap();
    encoded.into_inner()
}

fn cover_art_review_fixture(replies: Vec<(&'static str, Vec<u8>)>) -> CoverArtReviewFixture {
    use collector_modules::music::{COVER_ART_ARCHIVE_PROVIDER_ID, MUSICBRAINZ_PROVIDER_ID};
    let clock = Arc::new(TestClock::new(1_800_000_000));
    let (root, store, actor, service, collection) = setup_with_clock(Some(clock.clone()));
    let saved = save_inventory(&service, &actor, item(collection.id), Revision(0)).unwrap();
    let musicbrainz = metadata_fixture_server(vec![(
        "200 OK\r\nContent-Type: application/json",
        br#"{"releases":[{"id":"22222222-2222-4222-8222-222222222222","title":"Retained album"}]}"#
            .to_vec(),
    )]);
    let mb = Arc::new(
        MusicBrainzProvider::with_test_origin(
            "collector-test/0.1 (test@example.invalid)",
            &musicbrainz.origin,
            Duration::from_secs(1),
            Duration::ZERO,
            Duration::from_secs(60),
            Duration::from_secs(60),
        )
        .unwrap(),
    );
    let artwork = metadata_fixture_server(replies);
    let caa = Arc::new(
        CoverArtArchiveProvider::with_test_origins(
            "collector-test/0.1 (test@example.invalid)",
            &artwork.origin,
            &[artwork.origin.as_str()],
            Duration::from_secs(1),
        )
        .unwrap(),
    );
    let service = service
        .with_streaming_provider(
            ModuleId(MUSICBRAINZ_PROVIDER_ID.into()),
            Arc::new(collector_app::BoundedProviderAdapter(mb.clone())),
        )
        .unwrap()
        .with_catalog_parser(ModuleId(MUSICBRAINZ_PROVIDER_ID.into()), mb)
        .unwrap()
        .with_streaming_provider(
            ModuleId(COVER_ART_ARCHIVE_PROVIDER_ID.into()),
            Arc::new(collector_app::BoundedProviderAdapter(caa.clone())),
        )
        .unwrap()
        .with_catalog_parser(ModuleId(COVER_ART_ARCHIVE_PROVIDER_ID.into()), caa.clone())
        .unwrap();
    let text = service
        .resolve_metadata(
            &actor,
            saved.id,
            collector_app::ResolveMetadataCommand {
                lookup: collector_app::AcquireMetadataCommand::MusicBrainzSearch {
                    query: "retained album".into(),
                    refresh: false,
                },
                acquire_artwork: false,
            },
            &command(),
        )
        .unwrap();
    provider_drain(&service);
    assert_eq!(
        service.job(&actor, text).unwrap().state,
        JobState::Completed
    );
    let candidate = service.metadata_view(&actor, saved.id).unwrap().candidates[0]
        .identity
        .clone();
    CoverArtReviewFixture {
        root,
        store,
        actor,
        service,
        collection,
        item: saved,
        image_command: collector_app::ResolveMetadataCommand {
            lookup: collector_app::AcquireMetadataCommand::CoverArtImage {
                candidate,
                image_id: "9001".into(),
            },
            acquire_artwork: false,
        },
        clock,
        _musicbrainz: musicbrainz,
        artwork,
        artwork_provider: caa,
    }
}

#[test]
fn concurrent_cover_creation_converges_across_curators_and_metadata_selection() {
    let image = review_cover_image();
    let fixture =
        cover_art_review_fixture(vec![("200 OK\r\nContent-Type: image/png", image.clone())]);
    let second_user = fixture
        .store
        .create_user("cover-review-curator", "a long test password", false)
        .unwrap();
    Catalog::grant(
        &*fixture.store,
        fixture.collection.id,
        second_user,
        Role::Curator,
    )
    .unwrap();
    let second = Principal {
        user_id: second_user,
        administrator: false,
        device_id: None,
    };
    let people = [&fixture.actor, &second];
    fixture.clock.advance(60);
    let before = fixture
        .service
        .metadata_view(&fixture.actor, fixture.item.id)
        .unwrap();
    let jobs: Vec<_> = people
        .iter()
        .map(|who| {
            fixture
                .service
                .resolve_metadata(
                    who,
                    fixture.item.id,
                    fixture.image_command.clone(),
                    &command(),
                )
                .unwrap()
        })
        .collect();
    // Both requester Jobs wait on this one acquisition. Then both processing
    // slots materialize the same cover concurrently, using one process identity.
    assert!(
        fixture
            .service
            .run_next_job("shared-cover-download")
            .unwrap()
    );
    let barrier = std::sync::Barrier::new(2);
    std::thread::scope(|scope| {
        let threads: Vec<_> = (0..2)
            .map(|_| {
                scope.spawn(|| {
                    barrier.wait();
                    fixture
                        .service
                        .run_next_job("shared-cover-workers")
                        .unwrap()
                })
            })
            .collect();
        for thread in threads {
            assert!(thread.join().unwrap());
        }
    });
    provider_drain(&fixture.service);
    for (who, job) in people.into_iter().zip(jobs) {
        let view = fixture.service.job(who, job).unwrap();
        assert_eq!(view.state, JobState::Completed, "{:?}", view.error);
    }
    let assets = fixture
        .store
        .asset_states_for_item(fixture.item.id)
        .unwrap();
    assert_eq!(assets.len(), 1);
    assert_eq!(assets[0].revision, Revision(1));
    assert_eq!(
        resource_bytes(
            fixture
                .service
                .asset_resource(
                    &fixture.actor,
                    fixture.collection.id,
                    assets[0].asset.id,
                    false
                )
                .unwrap()
        ),
        image
    );
    let after = fixture
        .service
        .metadata_view(&fixture.actor, fixture.item.id)
        .unwrap();
    assert_eq!(
        after.revision,
        Revision(before.revision.0 + 1),
        "equal selections commit one authority change"
    );
    assert_eq!(after.authority.artwork_choice, Some(assets[0].asset.id));
    assert_eq!(after.display.title.value, fixture.item.title);
    assert_eq!(fixture.artwork.requests.lock().unwrap().len(), 1);
    fixture.store.reindex().unwrap();
    assert_eq!(
        fixture
            .service
            .metadata_view(&fixture.actor, fixture.item.id)
            .unwrap()
            .authority
            .artwork_choice,
        after.authority.artwork_choice
    );
}

#[test]
fn explicit_cover_reacquisition_restores_history_but_automatic_refresh_cannot() {
    let image = review_cover_image();
    let listing = br#"{"release":"https://musicbrainz.org/release/22222222-2222-4222-8222-222222222222","images":[{"id":"9001","front":true,"back":false,"approved":true,"types":["Front"]}]}"#.to_vec();
    let mut fixture = cover_art_review_fixture(vec![
        ("200 OK\r\nContent-Type: image/png", image.clone()),
        ("200 OK\r\nContent-Type: application/json", listing),
        ("200 OK\r\nContent-Type: image/png", image.clone()),
        ("200 OK\r\nContent-Type: image/png", image.clone()),
        ("200 OK\r\nContent-Type: image/png", image.clone()),
        ("200 OK\r\nContent-Type: image/png", image.clone()),
        ("200 OK\r\nContent-Type: image/png", image.clone()),
    ]);
    let first = fixture
        .service
        .resolve_metadata(
            &fixture.actor,
            fixture.item.id,
            fixture.image_command.clone(),
            &command(),
        )
        .unwrap();
    provider_drain(&fixture.service);
    assert_eq!(
        fixture.service.job(&fixture.actor, first).unwrap().state,
        JobState::Completed
    );
    let asset = fixture
        .store
        .asset_states_for_item(fixture.item.id)
        .unwrap()
        .remove(0);
    let edited = fixture
        .service
        .update_asset(
            &fixture.actor,
            asset.asset.id,
            asset.revision,
            UpdateAssetCommand {
                visibility: Visibility::Curators,
                alt_text: "My retained cover description".into(),
            },
            &command(),
        )
        .unwrap();
    let archived = fixture
        .service
        .tombstone_asset(&fixture.actor, asset.asset.id, edited.revision, &command())
        .unwrap();
    let tombstone = files_named(&fixture.root.path().join("archive"), "record.v3.json")
        .into_iter()
        .find(|path| {
            path.components()
                .any(|part| part.as_os_str() == asset.asset.id.to_string().as_str())
                && std::fs::read_to_string(path).is_ok_and(|raw| raw.contains("\"deleted\": true"))
        })
        .unwrap();
    let tombstone_bytes = std::fs::read(&tombstone).unwrap();
    let current = fixture
        .service
        .metadata_view(&fixture.actor, fixture.item.id)
        .unwrap();
    fixture
        .service
        .update_metadata_authority(
            &fixture.actor,
            fixture.item.id,
            UpdateMetadataAuthorityCommand {
                expected_revision: current.revision,
                change: MetadataAuthorityChange::ClearArtworkChoice,
            },
            &command(),
        )
        .unwrap();
    // A selected release gives the text-first automatic workflow a concrete
    // album whose front cover it should attempt to acquire.
    let cleared = fixture
        .service
        .metadata_view(&fixture.actor, fixture.item.id)
        .unwrap();
    fixture
        .service
        .update_metadata_authority(
            &fixture.actor,
            fixture.item.id,
            UpdateMetadataAuthorityCommand {
                expected_revision: cleared.revision,
                change: MetadataAuthorityChange::SelectCandidate {
                    candidate: current.candidates[0].identity.clone(),
                },
            },
            &command(),
        )
        .unwrap();
    fixture.clock.advance(60);
    let automatic = fixture
        .service
        .resolve_metadata(
            &fixture.actor,
            fixture.item.id,
            collector_app::ResolveMetadataCommand {
                lookup: collector_app::AcquireMetadataCommand::MusicBrainzSearch {
                    query: "retained album".into(),
                    refresh: false,
                },
                acquire_artwork: true,
            },
            &command(),
        )
        .unwrap();
    provider_drain(&fixture.service);
    let automatic = fixture
        .service
        .metadata_resolutions(&fixture.actor, fixture.item.id)
        .unwrap()
        .into_iter()
        .find(|view| view.job.id == automatic)
        .unwrap();
    assert_eq!(automatic.job.state, JobState::Completed);
    assert!(
        automatic.result.unwrap().artwork_error.is_some(),
        "automatic refresh must explain why archived artwork was not selected"
    );
    assert!(fixture.store.asset_state(asset.asset.id).unwrap().deleted);
    assert_eq!(
        fixture.store.asset_state(asset.asset.id).unwrap().revision,
        archived.revision
    );
    fixture.clock.advance(60);
    let receipt = command();
    let restore = fixture
        .service
        .resolve_metadata(
            &fixture.actor,
            fixture.item.id,
            fixture.image_command.clone(),
            &receipt,
        )
        .unwrap();
    fixture = fixture.reopen();
    provider_drain(&fixture.service);
    let restored = fixture.store.asset_state(asset.asset.id).unwrap();
    assert_eq!(
        fixture.service.job(&fixture.actor, restore).unwrap().state,
        JobState::Completed
    );
    assert!(!restored.deleted);
    assert_eq!(restored.revision, Revision(archived.revision.0 + 1));
    assert_eq!(restored.asset.visibility, Visibility::Curators);
    assert_eq!(restored.asset.alt_text, "My retained cover description");
    assert_eq!(
        fixture
            .store
            .asset_states_for_item(fixture.item.id)
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        resource_bytes(
            fixture
                .service
                .asset_resource(&fixture.actor, fixture.collection.id, asset.asset.id, false)
                .unwrap()
        ),
        image
    );
    assert_eq!(std::fs::read(&tombstone).unwrap(), tombstone_bytes);
    assert_eq!(
        fixture
            .service
            .resolve_metadata(
                &fixture.actor,
                fixture.item.id,
                fixture.image_command.clone(),
                &receipt
            )
            .unwrap(),
        restore
    );
    fixture.store.reindex().unwrap();
    let authority = fixture
        .service
        .metadata_view(&fixture.actor, fixture.item.id)
        .unwrap();
    assert_eq!(authority.authority.artwork_choice, Some(asset.asset.id));
    assert_eq!(authority.display.title.value, fixture.item.title);
    // A later direct selection reuses the restored revision rather than restoring again.
    let again = fixture
        .service
        .resolve_metadata(
            &fixture.actor,
            fixture.item.id,
            fixture.image_command.clone(),
            &command(),
        )
        .unwrap();
    provider_drain(&fixture.service);
    assert_eq!(
        fixture.service.job(&fixture.actor, again).unwrap().state,
        JobState::Completed
    );
    assert_eq!(
        fixture.store.asset_state(asset.asset.id).unwrap().revision,
        restored.revision
    );
    // Keep the clock unchanged: revision ownership, not time inference, must
    // prevent an old queued lookup from undoing a newer archive decision.
    let older_lookup = fixture
        .service
        .resolve_metadata(
            &fixture.actor,
            fixture.item.id,
            fixture.image_command.clone(),
            &command(),
        )
        .unwrap();
    let later_archive = fixture
        .service
        .tombstone_asset(
            &fixture.actor,
            asset.asset.id,
            restored.revision,
            &command(),
        )
        .unwrap();
    provider_drain(&fixture.service);
    assert_eq!(
        fixture
            .service
            .job(&fixture.actor, older_lookup)
            .unwrap()
            .state,
        JobState::Failed
    );
    assert!(fixture.store.asset_state(asset.asset.id).unwrap().deleted);
    assert_eq!(
        fixture.store.asset_state(asset.asset.id).unwrap().revision,
        later_archive.revision
    );
    let fresh = fixture
        .service
        .resolve_metadata(
            &fixture.actor,
            fixture.item.id,
            fixture.image_command.clone(),
            &command(),
        )
        .unwrap();
    provider_drain(&fixture.service);
    assert_eq!(
        fixture.service.job(&fixture.actor, fresh).unwrap().state,
        JobState::Completed
    );
    assert_eq!(
        fixture.store.asset_state(asset.asset.id).unwrap().revision,
        Revision(later_archive.revision.0 + 1)
    );
}

#[test]
fn provider_cover_materialization_fences_content_restore_revision_and_job_claim() {
    use collector_app::{ItemEffectContext, ItemEffectSource};
    let clock = Arc::new(TestClock::new(1_800_000_000));
    let (_root, store, actor, service, collection) = setup_with_clock(Some(clock.clone()));
    let saved = save_inventory(&service, &actor, item(collection.id), Revision(0)).unwrap();
    let other = save_inventory(&service, &actor, item(collection.id), Revision(0)).unwrap();
    let image = review_cover_image();
    let asset = AssetId::new();
    let input = CreateAssetCommand {
        collection_id: collection.id,
        subject: AssetSubject::Item { item_id: saved.id },
        kind: AssetKind::Cover,
        visibility: Visibility::Shared,
        alt_text: "Provider cover".into(),
    };
    let context = ItemEffectContext {
        item_id: saved.id,
        generation: 0,
        source: ItemEffectSource::Command,
    };
    let created = store
        .materialize_provider_artwork(asset, input.clone(), &image, context, None)
        .unwrap();
    let mut different = input.clone();
    different.subject = AssetSubject::Item { item_id: other.id };
    assert!(matches!(
        store.materialize_provider_artwork(
            asset,
            different,
            &image,
            ItemEffectContext {
                item_id: other.id,
                ..context
            },
            None
        ),
        Err(Error::IdempotencyConflict)
    ));
    let mut changed = std::io::Cursor::new(Vec::new());
    image::RgbImage::from_pixel(24, 24, image::Rgb([56, 34, 12]))
        .write_to(&mut changed, image::ImageFormat::Png)
        .unwrap();
    assert!(matches!(
        store.materialize_provider_artwork(
            asset,
            input.clone(),
            &changed.into_inner(),
            context,
            None
        ),
        Err(Error::IdempotencyConflict)
    ));
    let archived = store.tombstone_asset(asset, created.revision).unwrap();
    assert!(matches!(
        store.materialize_provider_artwork(asset, input.clone(), &image, context, None),
        Err(Error::Conflict)
    ));
    assert!(matches!(
        store.materialize_provider_artwork(
            asset,
            input.clone(),
            &image,
            context,
            Some(created.revision)
        ),
        Err(Error::Conflict)
    ));
    assert!(matches!(
        store.materialize_provider_artwork(
            asset,
            input.clone(),
            &image,
            ItemEffectContext {
                generation: 1,
                ..context
            },
            Some(archived.revision)
        ),
        Err(Error::Conflict)
    ));
    let restored = store
        .materialize_provider_artwork(
            asset,
            input.clone(),
            &image,
            context,
            Some(archived.revision),
        )
        .unwrap();
    assert!(!restored.deleted);
    assert_eq!(restored.revision, Revision(archived.revision.0 + 1));
    // A second authorized restorer accepts the already restored effect without
    // another revision; a newer tombstone must reject the old restore revision.
    assert_eq!(
        store
            .materialize_provider_artwork(
                asset,
                input.clone(),
                &image,
                context,
                Some(archived.revision)
            )
            .unwrap()
            .revision,
        restored.revision
    );
    let archived_again = store.tombstone_asset(asset, restored.revision).unwrap();
    assert!(matches!(
        store.materialize_provider_artwork(
            asset,
            input.clone(),
            &image,
            context,
            Some(archived.revision)
        ),
        Err(Error::Conflict)
    ));
    let job = JobId::new();
    store
        .enqueue_job(JobSpec {
            id: job,
            user_id: actor.user_id,
            collection_id: Some(collection.id),
            item_id: Some(saved.id),
            kind: "test.cover".into(),
            phase: "queued".into(),
            total_units: 1,
            request: serde_json::json!({}),
            frozen_input: None,
        })
        .unwrap();
    let claim = store.claim_job("cover-claim-fence", 30).unwrap().unwrap();
    let claimed = ItemEffectContext {
        source: ItemEffectSource::Job {
            id: job,
            claim: claim.claim_token.parse().unwrap(),
        },
        ..context
    };
    clock.advance(31);
    assert!(
        store
            .materialize_provider_artwork(
                asset,
                input.clone(),
                &image,
                claimed,
                Some(archived_again.revision)
            )
            .is_err()
    );
    let current = store.claim_job("cover-claim-fence", 30).unwrap().unwrap();
    store.request_job_cancellation(job, actor.user_id).unwrap();
    let cancelled = ItemEffectContext {
        source: ItemEffectSource::Job {
            id: job,
            claim: current.claim_token.parse().unwrap(),
        },
        ..context
    };
    assert!(
        store
            .materialize_provider_artwork(
                asset,
                input,
                &image,
                cancelled,
                Some(archived_again.revision)
            )
            .is_err()
    );
    assert!(store.asset_state(asset).unwrap().deleted);
    assert_eq!(
        store.asset_state(asset).unwrap().revision,
        archived_again.revision
    );
}

#[test]
fn provider_cover_restoration_recovers_before_and_after_revision_commit() {
    use collector_app::{ItemEffectContext, ItemEffectSource};
    use collector_store::PublicationFaultPoint;
    for point in [
        PublicationFaultPoint::Prepared,
        PublicationFaultPoint::RevisionCommitted,
    ] {
        let (root, store, actor, service, collection) = setup();
        let saved = save_inventory(&service, &actor, item(collection.id), Revision(0)).unwrap();
        let image = review_cover_image();
        let asset = AssetId::new();
        let input = CreateAssetCommand {
            collection_id: collection.id,
            subject: AssetSubject::Item { item_id: saved.id },
            kind: AssetKind::Cover,
            visibility: Visibility::Shared,
            alt_text: "Restored cover".into(),
        };
        let context = ItemEffectContext {
            item_id: saved.id,
            generation: 0,
            source: ItemEffectSource::Command,
        };
        let created = store
            .materialize_provider_artwork(asset, input.clone(), &image, context, None)
            .unwrap();
        let archived = store.tombstone_asset(asset, created.revision).unwrap();
        drop(service);
        drop(store);
        let failed = Store::open_with_publication_fault(
            root.path().join("archive"),
            root.path().join("state"),
            Arc::new(FailPublicationOnce {
                point,
                failed: AtomicBool::new(false),
            }),
        )
        .unwrap();
        assert!(
            failed
                .materialize_provider_artwork(
                    asset,
                    input.clone(),
                    &image,
                    context,
                    Some(archived.revision)
                )
                .is_err()
        );
        drop(failed);
        let recovered =
            Arc::new(Store::open(root.path().join("archive"), root.path().join("state")).unwrap());
        assert_eq!(
            recovered.asset_state(asset).unwrap().deleted,
            point == PublicationFaultPoint::Prepared
        );
        let restored = recovered
            .materialize_provider_artwork(asset, input, &image, context, Some(archived.revision))
            .unwrap();
        assert!(!restored.deleted);
        assert_eq!(
            restored.revision,
            Revision(archived.revision.0 + 1),
            "recovery/retry cannot restore twice"
        );
        recovered.reindex().unwrap();
        let service = CollectionService::new(recovered, Arc::new(Registry::builtins()));
        assert_eq!(
            resource_bytes(
                service
                    .asset_resource(&actor, collection.id, asset, false)
                    .unwrap()
            ),
            image
        );
    }
}
