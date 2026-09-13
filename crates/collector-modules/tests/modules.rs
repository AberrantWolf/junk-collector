use collector_core::*;
use collector_modules::{games::*, music::*, *};
use std::{cell::RefCell, collections::BTreeMap, io::Cursor};

#[derive(Clone, Copy)]
enum MigrationMode {
    Additive,
    Rename,
    Delete,
    ReplaceScalar,
    ReplaceArray,
}
struct Stamps {
    mode: MigrationMode,
}
#[derive(Default, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
struct StampPatch {
    denomination: PatchValue<u64>,
    unit: PatchValue<String>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct StampOperationResult {
    bytes: usize,
    job: JobId,
}

impl CollectionModule for Stamps {
    fn migration_policy(&self, from: u32) -> MigrationPolicy {
        if from == 1 && matches!(self.mode, MigrationMode::Rename) {
            MigrationPolicy::Transform {
                reason: "Replace the legacy string denomination with a numeric field",
            }
        } else {
            MigrationPolicy::Additive
        }
    }

    fn descriptor(&self) -> ModuleDescriptor {
        ModuleDescriptor {
            id: ModuleId("org.example.stamps".into()),
            schema_version: 2,
            display_name: "Stamps".into(),
            collection_type_label: "Postage stamps".into(),
            fields: vec![
                DomainFieldDescriptor {
                    key: "denomination".into(),
                    label: "Denomination".into(),
                    required: true,
                    editable: true,
                    kind: DomainFieldKind::Integer {
                        minimum: 0,
                        maximum: 1_000_000,
                    },
                },
                DomainFieldDescriptor {
                    key: "unit".into(),
                    label: "Unit".into(),
                    required: true,
                    editable: true,
                    kind: DomainFieldKind::Text { maximum_bytes: 32 },
                },
                DomainFieldDescriptor {
                    key: "ordered".into(),
                    label: "Legacy ordering".into(),
                    required: false,
                    editable: false,
                    kind: DomainFieldKind::Structured,
                },
            ],
            capabilities: vec![Capability {
                id: CapabilityId::new("org.example.stamps.inspect").unwrap(),
                display_name: "Inspect scan".into(),
                availability: Availability::Available,
                scope: OperationScope::CollectionDomain,
            }],
        }
    }
    fn validate_current(&self, data: &ModuleData) -> ModuleResult<()> {
        let value = data
            .payload
            .value()
            .map_err(|error| ModuleError::Invalid(error.to_string()))?;
        if value["denomination"].is_number() && value["unit"].is_string() {
            Ok(())
        } else {
            Err(ModuleError::Invalid(
                "denomination and unit required".into(),
            ))
        }
    }
    fn apply_edit(&self, current: &ModuleData, edit: &RawJson) -> ModuleResult<ModuleData> {
        let patch: StampPatch = edit
            .parse()
            .map_err(|error| ModuleError::Invalid(error.to_string()))?;
        let mut value = current
            .payload
            .value()
            .map_err(|error| ModuleError::Invalid(error.to_string()))?;
        let object = value
            .as_object_mut()
            .ok_or_else(|| ModuleError::Invalid("stamp data must be an object".into()))?;
        match patch.denomination {
            PatchValue::Omitted => {}
            PatchValue::Clear => {
                object.insert("denomination".into(), serde_json::Value::Null);
            }
            PatchValue::Set(value) => {
                object.insert("denomination".into(), serde_json::json!(value));
            }
        }
        match patch.unit {
            PatchValue::Omitted => {}
            PatchValue::Clear => {
                object.insert("unit".into(), serde_json::Value::Null);
            }
            PatchValue::Set(value) => {
                object.insert("unit".into(), serde_json::json!(value));
            }
        }
        Ok(ModuleData {
            module: current.module.clone(),
            schema_version: current.schema_version,
            payload: RawJson::from_value(value),
        })
    }
    fn validate_metadata_overrides(&self, overrides: &ModuleData) -> ModuleResult<()> {
        let value = overrides
            .payload
            .value()
            .map_err(|error| ModuleError::Invalid(error.to_string()))?;
        if value.as_object().is_some_and(serde_json::Map::is_empty) {
            Ok(())
        } else {
            Err(ModuleError::Invalid(
                "stamps define no metadata overrides".into(),
            ))
        }
    }
    fn validate_operation_result(
        &self,
        _capability: &CapabilityId,
        result: &RawJson,
    ) -> ModuleResult<()> {
        let result: StampOperationResult = result
            .parse()
            .map_err(|error| ModuleError::Invalid(error.to_string()))?;
        let _ = (result.bytes, result.job);
        Ok(())
    }
    fn migrate_adjacent(&self, from: u32, payload: &RawJson) -> ModuleResult<RawJson> {
        if from != 1 {
            return Err(ModuleError::UnsupportedSchema {
                found: from,
                current: 2,
            });
        }
        let mut value = payload
            .value()
            .map_err(|error| ModuleError::Invalid(error.to_string()))?;
        match self.mode {
            MigrationMode::Additive => {}
            MigrationMode::Rename => {
                let previous = value
                    .as_object_mut()
                    .unwrap()
                    .remove("legacy_denomination")
                    .unwrap();
                value["denomination"] =
                    serde_json::json!(previous.as_str().unwrap().parse::<u64>().unwrap());
            }
            MigrationMode::Delete => {
                value.as_object_mut().unwrap().remove("denomination");
            }
            MigrationMode::ReplaceScalar => {
                value["denomination"] = serde_json::json!(21);
            }
            MigrationMode::ReplaceArray => {
                value["ordered"] = serde_json::json!([2, 1]);
            }
        }
        value
            .as_object_mut()
            .unwrap()
            .insert("unit".into(), serde_json::json!("cents"));
        Ok(RawJson::from_value(value))
    }
    fn present(&self, data: &ModuleData) -> ModuleResult<ModulePresentation> {
        self.validate_current(data)?;
        let value = data.payload.value().unwrap();
        Ok(ModulePresentation {
            module_id: data.module.0.clone(),
            schema_version: 2,
            available: true,
            fields: vec![PresentationText {
                label: "Denomination".into(),
                value: value["denomination"].to_string(),
            }],
            badges: vec![PresentationBadge {
                label: "Stamp".into(),
                tone: BadgeTone::Neutral,
            }],
            ..Default::default()
        })
    }
    fn operate(
        &self,
        request: OperationRequest,
        host: &dyn ModuleHost,
    ) -> ModuleResult<OperationOutput> {
        let (scope, entity) = match request.entity {
            OperationEntity::Representation(id) => (
                PackageScope::Representation(id),
                OperationEntity::Representation(id),
            ),
            OperationEntity::Import(id) => (PackageScope::Import(id), OperationEntity::Import(id)),
            _ => {
                return Err(ModuleError::FailedOperation(
                    "package-bearing entity required".into(),
                ));
            }
        };
        let access = PackageAccess {
            collection_id: request.collection_id,
            scope,
        };
        let inventory = host.package_inventory(access)?;
        let mut bytes = 0usize;
        for file in inventory.files {
            let mut reader = host.package_file(access, file.id)?;
            bytes += std::io::Read::read_to_end(&mut reader, &mut Vec::new())
                .map_err(|error| ModuleError::FailedOperation(error.to_string()))?;
        }
        let job = host.submit_job(JobSubmission {
            operation: request.capability,
            input: request.input,
            collection_id: request.collection_id,
            entity,
            idempotency_key: request.idempotency_key,
        })?;
        Ok(OperationOutput {
            value: RawJson::from_value(serde_json::json!({"bytes":bytes,"job":job})),
        })
    }
}

struct FakeHost {
    collection_id: CollectionId,
    scope: PackageScope,
    files: BTreeMap<PackageFileId, Vec<u8>>,
    jobs: RefCell<Vec<String>>,
}
impl Default for FakeHost {
    fn default() -> Self {
        let file = PackageFileId::new();
        Self {
            collection_id: CollectionId::new(),
            scope: PackageScope::Representation(RepresentationId::new()),
            files: BTreeMap::from([(file, b"scan".to_vec())]),
            jobs: RefCell::default(),
        }
    }
}
impl ModuleHost for FakeHost {
    fn package_inventory(&self, access: PackageAccess) -> ModuleResult<PackageInventory> {
        if access.collection_id != self.collection_id || access.scope != self.scope {
            return Err(ModuleError::AccessDenied);
        }
        PackageInventory::new(
            access.collection_id,
            access.scope,
            self.files
                .iter()
                .enumerate()
                .map(|(index, (&id, bytes))| PackageInventoryFile {
                    id,
                    display_name: PackageFileName::new(format!("scan-{index}.bin")).unwrap(),
                    bytes: ByteLength(bytes.len() as u64),
                })
                .collect(),
        )
        .map_err(|error| ModuleError::Invalid(error.to_string()))
    }
    fn package_file(
        &self,
        access: PackageAccess,
        id: PackageFileId,
    ) -> ModuleResult<Box<dyn RepresentationReader>> {
        self.package_inventory(access)?;
        self.files
            .get(&id)
            .cloned()
            .map(|bytes| Box::new(Cursor::new(bytes)) as Box<dyn RepresentationReader>)
            .ok_or(ModuleError::MissingPackageFile(id))
    }
    fn representation_access(&self, id: RepresentationId) -> ModuleResult<PackageAccess> {
        if self.scope == PackageScope::Representation(id) {
            Ok(PackageAccess {
                collection_id: self.collection_id,
                scope: self.scope,
            })
        } else {
            Err(ModuleError::AccessDenied)
        }
    }
    fn submit_job(&self, request: JobSubmission) -> ModuleResult<JobId> {
        self.jobs.borrow_mut().push(request.idempotency_key);
        Ok(JobId::new())
    }
}

fn data(schema_version: u32, json: &str) -> ModuleData {
    ModuleData {
        module: ModuleId("org.example.stamps".into()),
        schema_version,
        payload: RawJson::from_json(json).unwrap(),
    }
}

#[test]
fn third_module_migrates_operates_and_presents_without_a_core_enum() {
    let mut registry = Registry::builtins();
    registry
        .register(Box::new(Stamps {
            mode: MigrationMode::Additive,
        }))
        .unwrap();
    let source = data(1, r#"{"denomination":20,"ordered":1e+02,"nullable":null}"#);
    let migrated = registry.migrate(&source).unwrap();
    assert_eq!(migrated.data.schema_version, 2);
    assert_eq!(migrated.provenance[0].payload.get(), source.payload.get());
    assert_eq!(
        source.payload.get(),
        r#"{"denomination":20,"ordered":1e+02,"nullable":null}"#
    );
    assert!(registry.inspect(&migrated.data).is_ok());
    assert!(registry.presentation(&migrated.data).available);

    let first = PackageFileId::new();
    let second = PackageFileId::new();
    let collection_id = CollectionId::new();
    let import_id = ImportId::new();
    let host = FakeHost {
        collection_id,
        scope: PackageScope::Import(import_id),
        files: BTreeMap::from([(first, b"front".to_vec()), (second, b"back".to_vec())]),
        jobs: RefCell::default(),
    };
    let output = registry
        .operate(
            OperationRequest {
                capability: CapabilityId::new("org.example.stamps.inspect").unwrap(),
                input: RawJson::from_json("{}").unwrap(),
                collection_id,
                entity: OperationEntity::Import(import_id),
                idempotency_key: "request-1".into(),
            },
            &host,
        )
        .unwrap();
    assert_eq!(output.value.value().unwrap()["bytes"], 9);
    assert_eq!(&*host.jobs.borrow(), &["request-1"]);
}

#[test]
fn package_access_is_scoped_and_single_reader_rejects_ambiguity() {
    let first = PackageFileId::new();
    let second = PackageFileId::new();
    let host = FakeHost {
        collection_id: CollectionId::new(),
        scope: PackageScope::Import(ImportId::new()),
        files: BTreeMap::from([(first, b"one".to_vec()), (second, b"two".to_vec())]),
        jobs: RefCell::default(),
    };
    let access = PackageAccess {
        collection_id: host.collection_id,
        scope: host.scope,
    };
    assert!(matches!(
        host.representation(RepresentationId::new()),
        Err(ModuleError::AccessDenied)
    ));
    assert!(matches!(
        host.package_file(access, PackageFileId::new()),
        Err(ModuleError::MissingPackageFile(_))
    ));
    assert!(matches!(
        host.single_package_reader(access),
        Err(ModuleError::AmbiguousPackage { files: 2 })
    ));
    assert!(matches!(
        host.package_inventory(PackageAccess {
            collection_id: CollectionId::new(),
            scope: host.scope
        }),
        Err(ModuleError::AccessDenied)
    ));
    assert!(matches!(
        host.package_inventory(PackageAccess {
            collection_id: host.collection_id,
            scope: PackageScope::Import(ImportId::new())
        }),
        Err(ModuleError::AccessDenied)
    ));
}

#[test]
fn destructive_and_unsupported_migrations_are_distinct() {
    for mode in [
        MigrationMode::Delete,
        MigrationMode::ReplaceScalar,
        MigrationMode::ReplaceArray,
    ] {
        let mut registry = Registry::default();
        registry.register(Box::new(Stamps { mode })).unwrap();
        assert!(matches!(
            registry.migrate(&data(1, r#"{"denomination":20,"ordered":[1,2]}"#)),
            Err(ModuleError::Invalid(_))
        ));
    }
    let mut registry = Registry::default();
    registry
        .register(Box::new(Stamps {
            mode: MigrationMode::Additive,
        }))
        .unwrap();
    assert!(matches!(
        registry.migrate(&data(0, r#"{"denomination":20}"#)),
        Err(ModuleError::UnsupportedSchema { .. })
    ));
    assert!(matches!(
        registry.inspect(&data(3, "{}")),
        Err(ModuleError::UnsupportedSchema { .. })
    ));
    let unknown = ModuleData {
        module: ModuleId("org.example.absent".into()),
        schema_version: 99,
        payload: RawJson::from_json(r#"{"z":1e2,"a":null}"#).unwrap(),
    };
    assert!(matches!(
        registry.inspect(&unknown),
        Err(ModuleError::UnavailableCapability(_))
    ));
    let presentation = registry.presentation(&unknown);
    assert!(!presentation.available);
    assert!(presentation.limitations[0].contains("not executable"));
    assert_eq!(unknown.payload.get(), r#"{"z":1e2,"a":null}"#);
}

struct NestedCapability;
impl CollectionModule for NestedCapability {
    fn descriptor(&self) -> ModuleDescriptor {
        let mut descriptor = Stamps {
            mode: MigrationMode::Additive,
        }
        .descriptor();
        descriptor.capabilities[0].id =
            CapabilityId::new("org.example.stamps.inspect.scan").unwrap();
        descriptor
    }
    fn validate_current(&self, data: &ModuleData) -> ModuleResult<()> {
        Stamps {
            mode: MigrationMode::Additive,
        }
        .validate_current(data)
    }
    fn apply_edit(&self, current: &ModuleData, edit: &RawJson) -> ModuleResult<ModuleData> {
        Stamps {
            mode: MigrationMode::Additive,
        }
        .apply_edit(current, edit)
    }
    fn validate_metadata_overrides(&self, overrides: &ModuleData) -> ModuleResult<()> {
        Stamps {
            mode: MigrationMode::Additive,
        }
        .validate_metadata_overrides(overrides)
    }
    fn validate_operation_result(
        &self,
        capability: &CapabilityId,
        result: &RawJson,
    ) -> ModuleResult<()> {
        Stamps {
            mode: MigrationMode::Additive,
        }
        .validate_operation_result(capability, result)
    }
    fn present(&self, data: &ModuleData) -> ModuleResult<ModulePresentation> {
        Stamps {
            mode: MigrationMode::Additive,
        }
        .present(data)
    }
    fn operate(
        &self,
        request: OperationRequest,
        host: &dyn ModuleHost,
    ) -> ModuleResult<OperationOutput> {
        Stamps {
            mode: MigrationMode::Additive,
        }
        .operate(request, host)
    }
}
#[test]
fn review_registered_capability_can_be_dispatched() {
    let mut registry = Registry::default();
    registry.register(Box::new(NestedCapability)).unwrap();
    let host = FakeHost::default();
    let PackageScope::Representation(representation_id) = host.scope else {
        unreachable!()
    };
    let result = registry.operate(
        OperationRequest {
            capability: CapabilityId::new("org.example.stamps.inspect.scan").unwrap(),
            input: RawJson::from_json("{}").unwrap(),
            collection_id: host.collection_id,
            entity: OperationEntity::Representation(representation_id),
            idempotency_key: "probe".into(),
        },
        &host,
    );
    assert!(
        result.is_ok(),
        "registered capability cannot dispatch: {result:?}"
    );
}

#[test]
fn explicit_schema_transform_retains_exact_source_and_unknown_fields() {
    let mut registry = Registry::default();
    registry
        .register(Box::new(Stamps {
            mode: MigrationMode::Rename,
        }))
        .unwrap();
    let source = data(
        1,
        r#"{"legacy_denomination":"20","unknown":{"z":1e+02,"a":null}}"#,
    );
    let migrated = registry.migrate(&source).unwrap();
    assert_eq!(migrated.provenance[0].payload.get(), source.payload.get());
    let value = migrated.data.payload.value().unwrap();
    assert_eq!(value["denomination"], 20);
    assert!(value.get("legacy_denomination").is_none());
    assert_eq!(value["unknown"], source.payload.value().unwrap()["unknown"]);
    assert!(registry.inspect(&migrated.data).is_ok());
}

#[derive(Clone, Copy)]
enum InvalidRegistrationKind {
    DuplicateCapability,
    ForeignCapability,
    InvalidFieldKey,
    UnsafeLabel,
    TooManyFields,
}
struct InvalidRegistration(InvalidRegistrationKind);
impl CollectionModule for InvalidRegistration {
    fn descriptor(&self) -> ModuleDescriptor {
        let mut descriptor = Stamps {
            mode: MigrationMode::Additive,
        }
        .descriptor();
        match self.0 {
            InvalidRegistrationKind::DuplicateCapability => descriptor
                .capabilities
                .push(descriptor.capabilities[0].clone()),
            InvalidRegistrationKind::ForeignCapability => {
                descriptor.capabilities[0].id = CapabilityId::new("org.other.inspect").unwrap();
            }
            InvalidRegistrationKind::InvalidFieldKey => {
                descriptor.fields[0].key = "Not-A-Safe-Key".into();
            }
            InvalidRegistrationKind::UnsafeLabel => {
                descriptor.fields[0].label = "<script>".into();
            }
            InvalidRegistrationKind::TooManyFields => {
                let field = descriptor.fields[0].clone();
                descriptor.fields = (0..33)
                    .map(|index| DomainFieldDescriptor {
                        key: format!("field_{index}"),
                        ..field.clone()
                    })
                    .collect();
            }
        }
        descriptor
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
        Ok(ModulePresentation::default())
    }
}
#[test]
fn capability_registration_is_atomic_and_rejects_ambiguous_ownership() {
    for kind in [
        InvalidRegistrationKind::ForeignCapability,
        InvalidRegistrationKind::DuplicateCapability,
        InvalidRegistrationKind::InvalidFieldKey,
        InvalidRegistrationKind::UnsafeLabel,
        InvalidRegistrationKind::TooManyFields,
    ] {
        let mut registry = Registry::default();
        assert!(
            registry
                .register(Box::new(InvalidRegistration(kind)))
                .is_err()
        );
        registry
            .register(Box::new(Stamps {
                mode: MigrationMode::Additive,
            }))
            .unwrap();
        assert_eq!(registry.descriptors().len(), 1);
    }
    assert!(CapabilityId::new("org..inspect").is_err());
}

#[test]
fn compatibility_reader_rejects_a_misdirected_host_binding() {
    struct MisdirectedHost(FakeHost);
    impl ModuleHost for MisdirectedHost {
        fn package_inventory(&self, access: PackageAccess) -> ModuleResult<PackageInventory> {
            self.0.package_inventory(access)
        }
        fn package_file(
            &self,
            access: PackageAccess,
            id: PackageFileId,
        ) -> ModuleResult<Box<dyn RepresentationReader>> {
            self.0.package_file(access, id)
        }
        fn representation_access(&self, _id: RepresentationId) -> ModuleResult<PackageAccess> {
            Ok(PackageAccess {
                collection_id: self.0.collection_id,
                scope: self.0.scope,
            })
        }
        fn submit_job(&self, request: JobSubmission) -> ModuleResult<JobId> {
            self.0.submit_job(request)
        }
    }
    let host = MisdirectedHost(FakeHost::default());
    assert!(matches!(
        host.representation(RepresentationId::new()),
        Err(ModuleError::AccessDenied)
    ));
}

#[test]
fn persisted_operations_use_registry_capability_identity_validation() {
    let id: collector_core::OperationId = CapabilityId::new("org.example.stamps.inspect").unwrap();
    let capability: CapabilityId =
        serde_json::from_str(&serde_json::to_string(&id).unwrap()).unwrap();
    assert_eq!(capability.as_str(), id.as_str());
    let mut registry = Registry::default();
    registry
        .register(Box::new(Stamps {
            mode: MigrationMode::Additive,
        }))
        .unwrap();
    assert_eq!(
        registry.capability_contract(&capability).unwrap().scope,
        OperationScope::CollectionDomain
    );
    assert!(serde_json::from_str::<CapabilityId>(r#""INVALID/operation""#).is_err());
}

#[test]
fn builtin_descriptors_publish_bounded_scalar_fields_and_read_only_tracks() {
    let descriptors = Registry::builtins().descriptors();
    let games = descriptors
        .iter()
        .find(|descriptor| descriptor.id.0 == "org.junk.games")
        .unwrap();
    assert_eq!(games.collection_type_label, "Video games");
    assert!(games.fields.iter().all(|field| field.editable));
    assert!(matches!(
        games
            .fields
            .iter()
            .find(|field| field.key == "release_year")
            .unwrap()
            .kind,
        DomainFieldKind::Integer {
            minimum: 0,
            maximum: 9999
        }
    ));

    let music = descriptors
        .iter()
        .find(|descriptor| descriptor.id.0 == "org.junk.music")
        .unwrap();
    let tracks = music
        .fields
        .iter()
        .find(|field| field.key == "tracks")
        .unwrap();
    assert!(!tracks.editable);
    assert_eq!(tracks.kind, DomainFieldKind::Structured);
}

#[test]
fn owner_edit_distinguishes_omission_and_null_and_preserves_structured_unknown_data() {
    let registry = Registry::builtins();
    let current = ModuleData {
        module: ModuleId("org.junk.music".into()),
        schema_version: 1,
        payload: RawJson::from_json(
            r#"{"artist":"Original","label":"Keep","release_year":1999,"tracks":[{"position":1,"title":"Track","start":0,"length":10}],"future":{"number":1e+02,"null":null}}"#,
        )
        .unwrap(),
    };
    let updated = registry
        .edit(
            &current,
            DomainEdit {
                module: current.module.clone(),
                fields: RawJson::from_json(r#"{"artist":null,"release_year":2001}"#).unwrap(),
            },
        )
        .unwrap();
    let value = updated.payload.value().unwrap();
    assert!(value["artist"].is_null(), "explicit null clears the field");
    assert_eq!(value["label"], "Keep", "omission preserves the field");
    assert_eq!(value["release_year"], 2001, "integer stays numeric");
    assert!(value["release_year"].is_number());
    assert_eq!(
        value["tracks"],
        current.payload.value().unwrap()["tracks"],
        "scalar edits do not overwrite structured fields"
    );
    assert_eq!(value["future"], current.payload.value().unwrap()["future"]);
    let presentation = registry.presentation(&updated);
    assert_eq!(presentation.fields[0].value, "Music CDs");
    assert!(
        presentation
            .fields
            .iter()
            .any(|field| field.label == "Release year" && field.value == "2001")
    );
    assert!(
        presentation
            .fields
            .iter()
            .any(|field| { field.label == "Tracks" && field.value == "Preserved (read-only)" })
    );
    assert!(
        updated
            .payload
            .get()
            .contains(r#""future":{"number":1e+02,"null":null}"#)
    );
    assert!(
        updated
            .payload
            .get()
            .contains(r#""tracks":[{"position":1,"title":"Track","start":0,"length":10}]"#)
    );

    for fields in [
        r#"{"tracks":[]}"#,
        r#"{"unknown":1}"#,
        r#"{"release_year":"2001"}"#,
    ] {
        assert!(matches!(
            registry.edit(
                &current,
                DomainEdit {
                    module: current.module.clone(),
                    fields: RawJson::from_json(fields).unwrap(),
                }
            ),
            Err(ModuleError::Invalid(_))
        ));
    }
}

#[test]
fn domain_metadata_overrides_are_typed_and_bound_to_the_collection_owner() {
    let registry = Registry::builtins();
    let mut authority = MetadataAuthority {
        domain_overrides: Some(ModuleData {
            module: ModuleId("org.junk.games".into()),
            schema_version: 1,
            payload: RawJson::from_json(r#"{"platform":"Mega Drive"}"#).unwrap(),
        }),
        ..Default::default()
    };
    registry
        .validate_metadata_authority(&ModuleId("org.junk.games".into()), &authority)
        .unwrap();
    assert!(matches!(
        registry.validate_metadata_authority(&ModuleId("org.junk.music".into()), &authority),
        Err(ModuleError::Invalid(_))
    ));
    authority.domain_overrides.as_mut().unwrap().payload =
        RawJson::from_json(r#"{"artist":"wrong schema"}"#).unwrap();
    assert!(matches!(
        registry.validate_metadata_authority(&ModuleId("org.junk.games".into()), &authority),
        Err(ModuleError::Invalid(_))
    ));
}

struct MalformedOutputStamps;
impl CollectionModule for MalformedOutputStamps {
    fn descriptor(&self) -> ModuleDescriptor {
        Stamps {
            mode: MigrationMode::Additive,
        }
        .descriptor()
    }
    fn validate_current(&self, data: &ModuleData) -> ModuleResult<()> {
        Stamps {
            mode: MigrationMode::Additive,
        }
        .validate_current(data)
    }
    fn apply_edit(&self, current: &ModuleData, edit: &RawJson) -> ModuleResult<ModuleData> {
        Stamps {
            mode: MigrationMode::Additive,
        }
        .apply_edit(current, edit)
    }
    fn validate_metadata_overrides(&self, overrides: &ModuleData) -> ModuleResult<()> {
        Stamps {
            mode: MigrationMode::Additive,
        }
        .validate_metadata_overrides(overrides)
    }
    fn validate_operation_result(
        &self,
        capability: &CapabilityId,
        result: &RawJson,
    ) -> ModuleResult<()> {
        Stamps {
            mode: MigrationMode::Additive,
        }
        .validate_operation_result(capability, result)
    }
    fn present(&self, data: &ModuleData) -> ModuleResult<ModulePresentation> {
        Stamps {
            mode: MigrationMode::Additive,
        }
        .present(data)
    }
    fn operate(
        &self,
        _request: OperationRequest,
        _host: &dyn ModuleHost,
    ) -> ModuleResult<OperationOutput> {
        Ok(OperationOutput {
            value: RawJson::from_value(serde_json::json!({"bytes":"nine"})),
        })
    }
}

#[test]
fn registry_rejects_malformed_success_output_before_publication() {
    let mut registry = Registry::default();
    registry.register(Box::new(MalformedOutputStamps)).unwrap();
    let host = FakeHost::default();
    let PackageScope::Representation(representation_id) = host.scope else {
        unreachable!()
    };
    assert!(matches!(
        registry.operate(
            OperationRequest {
                capability: CapabilityId::new("org.example.stamps.inspect").unwrap(),
                input: RawJson::from_json("{}").unwrap(),
                collection_id: host.collection_id,
                entity: OperationEntity::Representation(representation_id),
                idempotency_key: "malformed".into(),
            },
            &host,
        ),
        Err(ModuleError::Invalid(_))
    ));
}

#[test]
fn moved_game_and_music_result_wire_shapes_and_signed_units_are_unchanged() {
    let music = CdProcessingResult::AudioVerification {
        verification: AudioVerificationResult {
            status: AudioVerificationStatus::Verified,
            selected_offset: Some(StereoSampleFrameOffset(-6)),
            evaluated_offset_start: StereoSampleFrameOffset(-2939),
            evaluated_offset_end: StereoSampleFrameOffset(2939),
            tracks: vec![AudioTrackVerification {
                position: 1,
                status: AudioVerificationStatus::Verified,
                checksum_version: Some(AudioChecksumVersion::AccurateRipV2),
                confidence: Some(12),
            }],
            limitations: vec![],
        },
    };
    if let CdProcessingResult::AudioVerification { verification } = &music {
        verification.validate().unwrap();
    }
    assert_eq!(
        serde_json::to_string(&music).unwrap(),
        r#"{"result":"audio_verification","verification":{"status":"verified","selected_offset":-6,"evaluated_offset_start":-2939,"evaluated_offset_end":2939,"tracks":[{"position":1,"status":"verified","checksum_version":"accurate_rip_v2","confidence":12}],"limitations":[]}}"#
    );
    let game = GameProcessingResult::Structure {
        format: "header".into(),
        fields: BTreeMap::from([("mapper".into(), GameStructureValue::Unsigned(0))]),
    };
    assert_eq!(
        serde_json::to_string(&game).unwrap(),
        r#"{"result":"structure","format":"header","fields":{"mapper":{"type":"unsigned","value":0}}}"#
    );
}

#[test]
fn game_primitives_map_shared_hash_domains_without_claiming_catalog_verification() {
    let file_id = PackageFileId::new();
    let mut raw = Cursor::new(b"123456789".to_vec());
    let result = hash_game_file(file_id, &mut raw, GameHashProfile::Raw).unwrap();
    let GameProcessingResult::Hashes { files } = &result else {
        panic!("expected hashes")
    };
    assert_eq!(files[0].file_id, file_id);
    assert_eq!(files[0].domain.name, "org.junk.game.raw-bytes");
    assert_eq!(files[0].domain.version, 1);
    assert!(files[0].digests.iter().any(|digest| {
        digest.algorithm == HashAlgorithm::Sha256
            && digest.hex == "15e2b0d3c33891ebb0f1ef609ec419420c20e320ce94c65fbc8c3312448eb225"
    }));
    Games
        .validate_operation_result(
            &CapabilityId::new("org.junk.games.hash").unwrap(),
            &RawJson::from_value(serde_json::to_value(result).unwrap()),
        )
        .unwrap();

    let payload: Vec<u8> = (0..16 * 1024)
        .map(|index| ((index * 17 + 3) % 256) as u8)
        .collect();
    let mut rom = vec![0; 16];
    rom[..4].copy_from_slice(b"NES\x1a");
    rom[4] = 1;
    rom.extend_from_slice(&payload);
    let hashes = hash_game_file(
        file_id,
        &mut Cursor::new(rom.clone()),
        GameHashProfile::InesPayload,
    )
    .unwrap();
    let structure = inspect_ines_structure(&mut Cursor::new(rom)).unwrap();
    assert!(matches!(hashes, GameProcessingResult::Hashes { .. }));
    assert!(matches!(
        structure,
        GameProcessingResult::Structure { ref format, .. } if format == "ines"
    ));
    assert!(!matches!(hashes, GameProcessingResult::CatalogMatch { .. }));
}

#[test]
fn game_cue_uses_frozen_inventory_and_hashes_every_ordered_raw_track() {
    let collection_id = CollectionId::new();
    let representation_id = RepresentationId::new();
    let cue_id = PackageFileId::new();
    let bin_id = PackageFileId::new();
    let bin = [vec![0x11; 2352], vec![0x22; 2352]].concat();
    let inventory = PackageInventory::new(
        collection_id,
        PackageScope::Representation(representation_id),
        vec![
            PackageInventoryFile {
                id: cue_id,
                display_name: PackageFileName::new("disc/game.cue").unwrap(),
                bytes: ByteLength(106),
            },
            PackageInventoryFile {
                id: bin_id,
                display_name: PackageFileName::new("disc/game.bin").unwrap(),
                bytes: ByteLength(bin.len() as u64),
            },
        ],
    )
    .unwrap();
    let cue = "FILE \"game.bin\" BINARY\n\
               TRACK 01 MODE1/2352\nINDEX 01 00:00:00\n\
               TRACK 02 AUDIO\nINDEX 01 00:00:01\n";
    let result = hash_cue_package(cue, cue_id, &inventory, |id| {
        if id == bin_id {
            Ok(Box::new(Cursor::new(bin.clone())))
        } else {
            Err(ModuleError::MissingPackageFile(id))
        }
    })
    .unwrap();
    let GameProcessingResult::TrackHashes { tracks } = &result else {
        panic!("expected ordered track hashes")
    };
    assert_eq!(tracks.len(), 2);
    assert_eq!(tracks[0].source_file_id, bin_id);
    assert_eq!(tracks[1].source_file_id, bin_id);
    assert_ne!(tracks[0].digests[0], tracks[1].digests[0]);
    Games
        .validate_operation_result(
            &CapabilityId::new("org.junk.games.hash-tracks").unwrap(),
            &RawJson::from_value(serde_json::to_value(&result).unwrap()),
        )
        .unwrap();
    for offset in [0, (u64::MAX / 2352) * 2352] {
        let mut malformed = result.clone();
        if let GameProcessingResult::TrackHashes { tracks } = &mut malformed {
            tracks[1].source_byte_offset = offset;
        }
        assert!(
            Games
                .validate_operation_result(
                    &CapabilityId::new("org.junk.games.hash-tracks").unwrap(),
                    &RawJson::from_value(serde_json::to_value(malformed).unwrap()),
                )
                .is_err()
        );
    }
}

#[test]
fn unqualified_game_shapes_fail_closed() {
    let file_id = PackageFileId::new();
    let mut trainer = vec![0; 16];
    trainer[..4].copy_from_slice(b"NES\x1a");
    trainer[4] = 1;
    trainer[6] = 0x04;
    trainer.extend(vec![0; 512 + 16 * 1024]);
    assert!(matches!(
        hash_game_file(
            file_id,
            &mut Cursor::new(trainer),
            GameHashProfile::InesPayload
        ),
        Err(ModuleError::UnavailableCapability(_))
    ));

    let collection_id = CollectionId::new();
    let representation_id = RepresentationId::new();
    let cue_id = PackageFileId::new();
    let bin_id = PackageFileId::new();
    let inventory = PackageInventory::new(
        collection_id,
        PackageScope::Representation(representation_id),
        vec![
            PackageInventoryFile {
                id: cue_id,
                display_name: PackageFileName::new("disc.cue").unwrap(),
                bytes: ByteLength(1),
            },
            PackageInventoryFile {
                id: bin_id,
                display_name: PackageFileName::new("disc.iso").unwrap(),
                bytes: ByteLength(2048),
            },
        ],
    )
    .unwrap();
    let cooked = "FILE \"disc.iso\" BINARY\nTRACK 01 MODE1/2048\nINDEX 01 00:00:00\n";
    assert!(matches!(
        hash_cue_package(cooked, cue_id, &inventory, |_| {
            Ok(Box::new(Cursor::new(vec![0; 2048])))
        }),
        Err(ModuleError::UnavailableCapability(_))
    ));
}

#[test]
fn catalog_results_require_distinct_candidates_and_file_identities() {
    let capability = "org.junk.games.catalog-verification".parse().unwrap();
    let candidate = CatalogCandidate {
        provider: ModuleId("org.example.dat".into()),
        provider_record_id: "one".into(),
        label: "One".into(),
        matched_files: vec![PackageFileId::new()],
        matched_digests: vec![Digest::sha256(b"data")],
    };
    let check = |catalog| {
        Games.validate_operation_result(
            &capability,
            &RawJson::from_value(
                serde_json::to_value(GameProcessingResult::CatalogMatch { catalog }).unwrap(),
            ),
        )
    };
    assert!(
        check(CatalogMatchResult::AmbiguousCompleteMatches {
            candidates: vec![candidate.clone()]
        })
        .is_err()
    );
    assert!(
        check(CatalogMatchResult::AmbiguousCompleteMatches {
            candidates: vec![candidate.clone(), candidate.clone()]
        })
        .is_err()
    );
    let mut other = candidate.clone();
    other.provider_record_id = "two".into();
    assert!(
        check(CatalogMatchResult::AmbiguousCompleteMatches {
            candidates: vec![candidate.clone(), other]
        })
        .is_ok()
    );
    let mut duplicated_file = candidate;
    duplicated_file
        .matched_files
        .push(duplicated_file.matched_files[0]);
    duplicated_file
        .matched_digests
        .push(duplicated_file.matched_digests[0].clone());
    assert!(
        check(CatalogMatchResult::UniqueCompleteMatch {
            candidate: duplicated_file
        })
        .is_err()
    );
}

struct DescriptorFixture(ModuleDescriptor);
impl CollectionModule for DescriptorFixture {
    fn descriptor(&self) -> ModuleDescriptor {
        self.0.clone()
    }
    fn validate_current(&self, _: &ModuleData) -> ModuleResult<()> {
        unreachable!()
    }
    fn apply_edit(&self, _: &ModuleData, _: &RawJson) -> ModuleResult<ModuleData> {
        unreachable!()
    }
    fn validate_metadata_overrides(&self, _: &ModuleData) -> ModuleResult<()> {
        unreachable!()
    }
    fn validate_operation_result(&self, _: &CapabilityId, _: &RawJson) -> ModuleResult<()> {
        unreachable!()
    }
    fn present(&self, _: &ModuleData) -> ModuleResult<ModulePresentation> {
        unreachable!()
    }
}

#[test]
fn descriptor_registration_bounds_all_client_visible_contracts() {
    let valid = Stamps {
        mode: MigrationMode::Additive,
    }
    .descriptor();
    let mut invalid = Vec::new();
    let mut value = valid.clone();
    value.fields[0].key = "bad-key".into();
    invalid.push(value);
    let mut value = valid.clone();
    value.fields.push(value.fields[0].clone());
    invalid.push(value);
    let mut value = valid.clone();
    value.display_name = "unsafe\nlabel".into();
    invalid.push(value);
    let mut value = valid.clone();
    value.capabilities[0].availability = Availability::Unavailable {
        limitation: "x".repeat(4097),
    };
    invalid.push(value);
    let mut value = valid.clone();
    value.capabilities = (0..129)
        .map(|index| {
            let mut c = valid.capabilities[0].clone();
            c.id = CapabilityId::new(format!("org.example.stamps.op{index}")).unwrap();
            c
        })
        .collect();
    invalid.push(value);
    for descriptor in invalid {
        let mut registry = Registry::default();
        assert!(
            registry
                .register(Box::new(DescriptorFixture(descriptor)))
                .is_err()
        );
        assert!(registry.descriptors().is_empty());
        registry
            .register(Box::new(DescriptorFixture(valid.clone())))
            .unwrap();
    }
}

#[test]
fn empty_raw_hash_remains_a_valid_integrity_observation() {
    let result = hash_game_file(
        PackageFileId::new(),
        &mut Cursor::new(Vec::<u8>::new()),
        GameHashProfile::Raw,
    )
    .unwrap();
    Games
        .validate_operation_result(
            &CapabilityId::new("org.junk.games.hash").unwrap(),
            &RawJson::from_value(serde_json::to_value(result).unwrap()),
        )
        .unwrap();
}
