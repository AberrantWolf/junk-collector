use collector_core::*;
use collector_modules::*;
use std::{cell::RefCell, io::Cursor};

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
            capabilities: vec![Capability {
                id: CapabilityId::new("org.example.stamps.inspect").unwrap(),
                display_name: "Inspect scan".into(),
                availability: Availability::Available,
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
        let OperationEntity::Representation(id) = request.entity else {
            return Err(ModuleError::FailedOperation(
                "representation required".into(),
            ));
        };
        let mut reader = host.representation(id)?;
        let mut bytes = Vec::new();
        std::io::Read::read_to_end(&mut reader, &mut bytes)
            .map_err(|error| ModuleError::FailedOperation(error.to_string()))?;
        let job = host.submit_job(JobSubmission {
            operation: request.capability,
            input: request.input,
            collection_id: request.collection_id,
            entity: OperationEntity::Representation(id),
            idempotency_key: request.idempotency_key,
        })?;
        Ok(OperationOutput {
            value: RawJson::from_value(serde_json::json!({"bytes":bytes.len(),"job":job})),
        })
    }
}

#[derive(Default)]
struct FakeHost {
    jobs: RefCell<Vec<String>>,
}
impl ModuleHost for FakeHost {
    fn representation(&self, _id: RepresentationId) -> ModuleResult<Box<dyn RepresentationReader>> {
        Ok(Box::new(Cursor::new(b"scan".to_vec())))
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

    let host = FakeHost::default();
    let output = registry
        .operate(
            OperationRequest {
                capability: CapabilityId::new("org.example.stamps.inspect").unwrap(),
                input: RawJson::from_json("{}").unwrap(),
                collection_id: CollectionId::new(),
                entity: OperationEntity::Representation(RepresentationId::new()),
                idempotency_key: "request-1".into(),
            },
            &host,
        )
        .unwrap();
    assert_eq!(output.value.value().unwrap()["bytes"], 4);
    assert_eq!(&*host.jobs.borrow(), &["request-1"]);
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
    let result = registry.operate(
        OperationRequest {
            capability: CapabilityId::new("org.example.stamps.inspect.scan").unwrap(),
            input: RawJson::from_json("{}").unwrap(),
            collection_id: CollectionId::new(),
            entity: OperationEntity::Representation(RepresentationId::new()),
            idempotency_key: "probe".into(),
        },
        &FakeHost::default(),
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

struct InvalidRegistration {
    duplicate: bool,
}
impl CollectionModule for InvalidRegistration {
    fn descriptor(&self) -> ModuleDescriptor {
        let mut descriptor = Stamps {
            mode: MigrationMode::Additive,
        }
        .descriptor();
        if self.duplicate {
            descriptor
                .capabilities
                .push(descriptor.capabilities[0].clone());
        } else {
            descriptor.capabilities[0].id = CapabilityId::new("org.other.inspect").unwrap();
        }
        descriptor
    }
    fn validate_current(&self, _data: &ModuleData) -> ModuleResult<()> {
        Ok(())
    }
    fn present(&self, _data: &ModuleData) -> ModuleResult<ModulePresentation> {
        Ok(ModulePresentation::default())
    }
}
#[test]
fn capability_registration_is_atomic_and_rejects_ambiguous_ownership() {
    for duplicate in [false, true] {
        let mut registry = Registry::default();
        assert!(
            registry
                .register(Box::new(InvalidRegistration { duplicate }))
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
