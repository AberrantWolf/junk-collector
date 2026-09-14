//! Compiled, domain-owned module contracts. Modules receive host capabilities;
//! they do not construct storage, network, scheduler, or transport adapters.
pub mod games;
pub mod music;

use collector_core::*;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    io::{Read, Seek},
};

// An executable capability and its persisted operation share one validated identity.
pub use collector_core::OperationId as CapabilityId;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum Availability {
    Available,
    Unavailable { limitation: String },
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Capability {
    pub id: CapabilityId,
    pub display_name: String,
    pub availability: Availability,
    pub scope: OperationScope,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum OperationScope {
    CollectionDomain,
    SharedMedia,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DomainFieldKind {
    Text { maximum_bytes: u32 },
    Integer { minimum: i64, maximum: i64 },
    Structured,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct DomainFieldDescriptor {
    pub key: String,
    pub label: String,
    pub required: bool,
    pub editable: bool,
    pub kind: DomainFieldKind,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ModuleDescriptor {
    pub id: ModuleId,
    pub schema_version: u32,
    pub display_name: String,
    pub collection_type_label: String,
    pub fields: Vec<DomainFieldDescriptor>,
    pub capabilities: Vec<Capability>,
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ModuleError {
    #[error("unknown capability: {0}")]
    UnknownCapability(String),
    #[error("capability unavailable: {0}")]
    UnavailableCapability(String),
    #[error("unsupported schema {found}; current schema is {current}")]
    UnsupportedSchema { found: u32, current: u32 },
    #[error("module operation failed: {0}")]
    FailedOperation(String),
    #[error("invalid module data: {0}")]
    Invalid(String),
    #[error("package access denied")]
    AccessDenied,
    #[error("package file is missing: {0}")]
    MissingPackageFile(PackageFileId),
    #[error("single-file reader is ambiguous for a package with {files} files")]
    AmbiguousPackage { files: usize },
}
pub type ModuleResult<T> = std::result::Result<T, ModuleError>;
impl From<ModuleError> for Error {
    fn from(value: ModuleError) -> Self {
        let display = value.to_string();
        match value {
            ModuleError::AccessDenied => Error::Forbidden,
            ModuleError::MissingPackageFile(_) => Error::NotFound,
            ModuleError::AmbiguousPackage { .. } => Error::Invalid(display),
            ModuleError::UnavailableCapability(_)
            | ModuleError::UnsupportedSchema { .. }
            | ModuleError::FailedOperation(_) => Error::Unavailable(display),
            ModuleError::UnknownCapability(message) => {
                Error::Invalid(format!("unknown capability: {message}"))
            }
            ModuleError::Invalid(message) => Error::Invalid(message),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PresentationText {
    pub label: String,
    pub value: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PresentationBadge {
    pub label: String,
    pub tone: BadgeTone,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum BadgeTone {
    Neutral,
    Positive,
    Warning,
    Negative,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PresentationValue {
    pub label: String,
    pub value: serde_json::Value,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ArtworkReference {
    pub asset_id: AssetId,
    pub alt_text: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CapabilityAffordance {
    pub id: CapabilityId,
    pub label: String,
    pub availability: Availability,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ModulePresentation {
    pub module_id: String,
    pub schema_version: u32,
    pub available: bool,
    pub fields: Vec<PresentationText>,
    pub badges: Vec<PresentationBadge>,
    pub values: Vec<PresentationValue>,
    pub artwork: Vec<ArtworkReference>,
    pub limitations: Vec<String>,
    pub capabilities: Vec<CapabilityAffordance>,
}
impl ModulePresentation {
    pub fn validate(&self) -> ModuleResult<()> {
        let safe_text = |value: &str| {
            value.len() <= 4096
                && !value
                    .chars()
                    .any(|character| character.is_control() && !matches!(character, '\n' | '\t'))
        };
        if self.fields.len() > 128
            || self.badges.len() > 64
            || self.values.len() > 64
            || self.artwork.len() > 64
            || self.limitations.len() > 64
            || self.capabilities.len() > 128
        {
            return Err(ModuleError::Invalid(
                "presentation contribution is too large".into(),
            ));
        }
        if self
            .fields
            .iter()
            .any(|value| !safe_text(&value.label) || !safe_text(&value.value))
            || self.badges.iter().any(|value| !safe_text(&value.label))
            || self.artwork.iter().any(|value| !safe_text(&value.alt_text))
            || self.limitations.iter().any(|value| !safe_text(value))
            || self
                .capabilities
                .iter()
                .any(|value| !safe_text(&value.label))
            || self.values.iter().any(|value| {
                !safe_text(&value.label)
                    || serde_json::to_vec(&value.value).is_ok_and(|bytes| bytes.len() > 64 * 1024)
            })
        {
            return Err(ModuleError::Invalid(
                "presentation contains unsafe or oversized text/data".into(),
            ));
        }
        Ok(())
    }
}

pub trait RepresentationReader: Read + Seek + Send {}
impl<T: Read + Seek + Send> RepresentationReader for T {}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PackageAccess {
    pub collection_id: CollectionId,
    pub scope: PackageScope,
}
#[derive(Clone, Debug)]
pub enum OperationEntity {
    Work(WorkId),
    Edition(EditionId),
    Item(ItemId),
    Specimen(SpecimenId),
    Component(ComponentId),
    Carrier(CarrierId),
    Capture(CaptureId),
    Representation(RepresentationId),
    Import(ImportId),
}
#[derive(Clone, Debug)]
pub struct JobSubmission {
    pub operation: CapabilityId,
    pub input: RawJson,
    pub collection_id: CollectionId,
    pub entity: OperationEntity,
    pub idempotency_key: String,
}
pub trait ModuleHost {
    /// Cooperative cancellation / lease check during CPU-only processing loops.
    fn checkpoint(&self) -> ModuleResult<()> {
        Ok(())
    }

    fn package_inventory(&self, access: PackageAccess) -> ModuleResult<PackageInventory>;
    fn package_file(
        &self,
        access: PackageAccess,
        id: PackageFileId,
    ) -> ModuleResult<Box<dyn RepresentationReader>>;
    fn representation_access(&self, id: RepresentationId) -> ModuleResult<PackageAccess>;

    fn catalog_snapshot(&self, _id: SnapshotId) -> ModuleResult<CatalogSnapshotInput> {
        Err(ModuleError::UnavailableCapability(
            "catalog snapshot access is unavailable".into(),
        ))
    }

    fn single_package_reader(
        &self,
        access: PackageAccess,
    ) -> ModuleResult<Box<dyn RepresentationReader>> {
        let inventory = self.package_inventory(access)?;
        inventory
            .validate()
            .map_err(|error| ModuleError::Invalid(error.to_string()))?;
        if inventory.collection_id != access.collection_id || inventory.scope != access.scope {
            return Err(ModuleError::AccessDenied);
        }
        let [file] = inventory.files.as_slice() else {
            return Err(ModuleError::AmbiguousPackage {
                files: inventory.files.len(),
            });
        };
        self.package_file(access, file.id)
    }

    /// Compatibility adapter for callers that expect one Representation stream.
    /// Multi-file Representations fail explicitly instead of selecting a file.
    fn representation(&self, id: RepresentationId) -> ModuleResult<Box<dyn RepresentationReader>> {
        let access = self.representation_access(id)?;
        if access.scope != PackageScope::Representation(id) {
            return Err(ModuleError::AccessDenied);
        }
        self.single_package_reader(access)
    }

    fn submit_job(&self, request: JobSubmission) -> ModuleResult<JobId>;
}
#[derive(Clone, Debug)]
pub struct OperationRequest {
    pub capability: CapabilityId,
    pub input: RawJson,
    pub collection_id: CollectionId,
    pub entity: OperationEntity,
    pub idempotency_key: String,
}
#[derive(Clone, Debug)]
pub struct OperationOutput {
    pub value: RawJson,
}

#[derive(Clone, Debug)]
pub struct CatalogSnapshotInput {
    pub id: SnapshotId,
    pub provider: ModuleId,
    pub observation: RawJson,
    pub entries: Vec<RawJson>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegisteredOperation {
    pub owner: ModuleId,
    pub schema_version: u32,
    pub scope: OperationScope,
}

/// A bounded owner-mediated patch. The raw object preserves JSON scalar types;
/// each owner deserializes it into its own typed patch where missing and null are
/// distinct operations.
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct DomainEdit {
    pub module: ModuleId,
    #[schema(value_type = Object)]
    pub fields: RawJson,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum PatchValue<T> {
    #[default]
    Omitted,
    Clear,
    Set(T),
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for PatchValue<T> {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        Option::<T>::deserialize(deserializer).map(|value| match value {
            Some(value) => Self::Set(value),
            None => Self::Clear,
        })
    }
}

pub(crate) type RawDomainObject = BTreeMap<String, Box<serde_json::value::RawValue>>;

pub(crate) fn raw_domain_object(data: &ModuleData) -> ModuleResult<RawDomainObject> {
    serde_json::from_str(data.payload.get())
        .map_err(|error| ModuleError::Invalid(error.to_string()))
}

pub(crate) fn module_data_from_raw_object(
    current: &ModuleData,
    object: RawDomainObject,
) -> ModuleResult<ModuleData> {
    let payload =
        serde_json::to_string(&object).map_err(|error| ModuleError::Invalid(error.to_string()))?;
    Ok(ModuleData {
        module: current.module.clone(),
        schema_version: current.schema_version,
        payload: RawJson::from_json(payload)
            .map_err(|error| ModuleError::Invalid(error.to_string()))?,
    })
}

pub(crate) fn apply_patch_value<T: Serialize>(
    object: &mut RawDomainObject,
    key: &str,
    patch: PatchValue<T>,
) -> ModuleResult<()> {
    match patch {
        PatchValue::Omitted => {}
        PatchValue::Clear => {
            object.insert(
                key.into(),
                serde_json::value::to_raw_value(&Option::<()>::None)
                    .map_err(|error| ModuleError::Invalid(error.to_string()))?,
            );
        }
        PatchValue::Set(value) => {
            object.insert(
                key.into(),
                serde_json::value::to_raw_value(&value)
                    .map_err(|error| ModuleError::Invalid(error.to_string()))?,
            );
        }
    }
    Ok(())
}
#[derive(Clone, Debug)]
pub struct ModuleMigration {
    pub data: ModuleData,
    /// Immutable source and intermediate envelopes retained in version order.
    pub provenance: Vec<ModuleData>,
}

/// Additive migrations are the safe default. A schema-specific transformation
/// must opt in explicitly; the registry always retains its exact input envelope.
#[derive(Clone, Copy, Debug)]
pub enum MigrationPolicy {
    Additive,
    Transform { reason: &'static str },
}

pub trait CollectionModule: Send + Sync {
    fn migration_policy(&self, _from: u32) -> MigrationPolicy {
        MigrationPolicy::Additive
    }

    fn descriptor(&self) -> ModuleDescriptor;
    fn validate_current(&self, data: &ModuleData) -> ModuleResult<()>;
    fn apply_edit(&self, current: &ModuleData, edit: &RawJson) -> ModuleResult<ModuleData>;
    fn validate_metadata_overrides(&self, overrides: &ModuleData) -> ModuleResult<()>;
    fn validate_operation_result(
        &self,
        capability: &CapabilityId,
        result: &RawJson,
    ) -> ModuleResult<()>;
    fn migrate_adjacent(&self, from: u32, _payload: &RawJson) -> ModuleResult<RawJson> {
        Err(ModuleError::UnsupportedSchema {
            found: from,
            current: self.descriptor().schema_version,
        })
    }
    fn present(&self, data: &ModuleData) -> ModuleResult<ModulePresentation>;
    fn operate(
        &self,
        request: OperationRequest,
        host: &dyn ModuleHost,
    ) -> ModuleResult<OperationOutput> {
        let _ = (request, host);
        Err(ModuleError::UnavailableCapability(
            "operation dispatch is not implemented".into(),
        ))
    }
}

#[derive(Default)]
pub struct Registry {
    modules: BTreeMap<String, Box<dyn CollectionModule>>,
    capabilities: BTreeMap<CapabilityId, String>,
}
impl Registry {
    pub fn builtins() -> Self {
        let mut result = Self::default();
        result.register(Box::new(games::Games)).unwrap();
        result.register(Box::new(music::Music)).unwrap();
        result
    }
    pub fn register(&mut self, module: Box<dyn CollectionModule>) -> Result<()> {
        let descriptor = module.descriptor();
        descriptor.id.validate()?;
        if descriptor.schema_version == 0
            || !safe_descriptor_text(&descriptor.display_name, 120)
            || !safe_descriptor_text(&descriptor.collection_type_label, 120)
            || descriptor.fields.len() > 32
            || descriptor.capabilities.len() > 128
        {
            return Err(Error::Invalid("invalid module descriptor".into()));
        }
        if self.modules.contains_key(&descriptor.id.0) {
            return Err(Error::Invalid("duplicate module".into()));
        }
        let mut ids = std::collections::BTreeSet::new();
        let mut field_keys = std::collections::BTreeSet::new();
        for field in &descriptor.fields {
            if !valid_field_key(&field.key)
                || !field_keys.insert(field.key.clone())
                || !safe_descriptor_text(&field.label, 120)
                || matches!(field.kind, DomainFieldKind::Structured) && field.editable
                || matches!(field.kind, DomainFieldKind::Text { maximum_bytes: 0 })
                || matches!(field.kind, DomainFieldKind::Text { maximum_bytes } if maximum_bytes > 4096)
                || matches!(field.kind, DomainFieldKind::Integer { minimum, maximum } if minimum > maximum)
            {
                return Err(Error::Invalid("invalid module field descriptor".into()));
            }
        }
        for capability in &descriptor.capabilities {
            CapabilityId::new(capability.id.as_str())?;
            if !safe_descriptor_text(&capability.display_name, 120)
                || matches!(&capability.availability, Availability::Unavailable { limitation } if !safe_descriptor_text(limitation, 4096))
            {
                return Err(Error::Invalid("invalid capability descriptor".into()));
            }
            if !capability
                .id
                .as_str()
                .starts_with(&format!("{}.", descriptor.id.0))
            {
                return Err(Error::Invalid(
                    "capability must belong to its module namespace".into(),
                ));
            }
            if !ids.insert(capability.id.clone()) || self.capabilities.contains_key(&capability.id)
            {
                return Err(Error::Invalid("duplicate capability".into()));
            }
        }
        for id in ids {
            self.capabilities.insert(id, descriptor.id.0.clone());
        }
        self.modules.insert(descriptor.id.0, module);
        Ok(())
    }
    pub fn inspect(&self, data: &ModuleData) -> ModuleResult<()> {
        let module = self.modules.get(&data.module.0).ok_or_else(|| {
            ModuleError::UnavailableCapability(format!("module {} is not installed", data.module.0))
        })?;
        let current = module.descriptor().schema_version;
        if data.schema_version != current {
            return Err(ModuleError::UnsupportedSchema {
                found: data.schema_version,
                current,
            });
        }
        module.validate_current(data)
    }
    pub fn validate(&self, data: &ModuleData) -> Result<()> {
        if !self.modules.contains_key(&data.module.0) {
            // Opaque unavailable data remains storable and round-trippable. This is
            // preservation, not executable-module validation.
            return data.module.validate();
        }
        self.inspect(data).map_err(Into::into)
    }
    pub fn edit(&self, current: &ModuleData, edit: DomainEdit) -> ModuleResult<ModuleData> {
        edit.module
            .validate()
            .map_err(|error| ModuleError::Invalid(error.to_string()))?;
        if current.module != edit.module {
            return Err(ModuleError::Invalid(
                "domain edit owner does not match current module data".into(),
            ));
        }
        if edit.fields.get().len() > 64 * 1024
            || !matches!(edit.fields.value(), Ok(serde_json::Value::Object(_)))
        {
            return Err(ModuleError::Invalid(
                "domain edit must be a bounded JSON object".into(),
            ));
        }
        let module = self.modules.get(&current.module.0).ok_or_else(|| {
            ModuleError::UnavailableCapability(format!(
                "module {} is not installed",
                current.module.0
            ))
        })?;
        if current.schema_version != module.descriptor().schema_version {
            return Err(ModuleError::UnsupportedSchema {
                found: current.schema_version,
                current: module.descriptor().schema_version,
            });
        }
        let updated = module.apply_edit(current, &edit.fields)?;
        if updated.module != current.module || updated.schema_version != current.schema_version {
            return Err(ModuleError::Invalid(
                "domain edit changed its owner or schema".into(),
            ));
        }
        module.validate_current(&updated)?;
        Ok(updated)
    }
    pub fn validate_metadata_authority(
        &self,
        owner: &ModuleId,
        authority: &MetadataAuthority,
    ) -> ModuleResult<()> {
        authority
            .validate()
            .map_err(|error| ModuleError::Invalid(error.to_string()))?;
        let Some(overrides) = &authority.domain_overrides else {
            return Ok(());
        };
        if &overrides.module != owner {
            return Err(ModuleError::Invalid(
                "metadata override owner does not match collection owner".into(),
            ));
        }
        let module = self.modules.get(&owner.0).ok_or_else(|| {
            ModuleError::UnavailableCapability(format!("module {} is not installed", owner.0))
        })?;
        if overrides.schema_version != module.descriptor().schema_version {
            return Err(ModuleError::UnsupportedSchema {
                found: overrides.schema_version,
                current: module.descriptor().schema_version,
            });
        }
        module.validate_metadata_overrides(overrides)
    }
    pub fn migrate(&self, data: &ModuleData) -> ModuleResult<ModuleMigration> {
        let module = self.modules.get(&data.module.0).ok_or_else(|| {
            ModuleError::UnavailableCapability(format!("module {} is not installed", data.module.0))
        })?;
        let current = module.descriptor().schema_version;
        if data.schema_version > current {
            return Err(ModuleError::UnsupportedSchema {
                found: data.schema_version,
                current,
            });
        }
        let mut version = data.schema_version;
        let mut payload = data.payload.clone();
        let mut provenance = vec![data.clone()];
        while version < current {
            let next = module.migrate_adjacent(version, &payload)?;
            match module.migration_policy(version) {
                MigrationPolicy::Additive => ensure_additive(&payload, &next)?,
                MigrationPolicy::Transform { reason } if !reason.trim().is_empty() => {}
                MigrationPolicy::Transform { .. } => {
                    return Err(ModuleError::Invalid(
                        "schema transformation requires a rationale".into(),
                    ));
                }
            }
            payload = next;
            version += 1;
            if version < current {
                provenance.push(ModuleData {
                    module: data.module.clone(),
                    schema_version: version,
                    payload: payload.clone(),
                });
            }
        }
        let migrated = ModuleData {
            module: data.module.clone(),
            schema_version: current,
            payload,
        };
        module.validate_current(&migrated)?;
        Ok(ModuleMigration {
            data: migrated,
            provenance,
        })
    }
    pub fn presentation(&self, data: &ModuleData) -> ModulePresentation {
        let Some(module) = self.modules.get(&data.module.0) else {
            return generic_presentation(
                data,
                "Module is not installed; preserved data is not executable.",
            );
        };
        if data.schema_version != module.descriptor().schema_version {
            return generic_presentation(
                data,
                "Module schema is unsupported; preserved data is not executable.",
            );
        }
        match module
            .validate_current(data)
            .and_then(|()| module.present(data))
            .and_then(|presentation| {
                presentation.validate()?;
                Ok(presentation)
            }) {
            Ok(presentation) => presentation,
            Err(error) => generic_presentation(data, &error.to_string()),
        }
    }
    pub fn operate(
        &self,
        request: OperationRequest,
        host: &dyn ModuleHost,
    ) -> ModuleResult<OperationOutput> {
        if request.idempotency_key.is_empty()
            || request.idempotency_key.len() > 200
            || request.input.get().len() > 64 * 1024
        {
            return Err(ModuleError::Invalid(
                "operation input or idempotency identity is out of bounds".into(),
            ));
        }
        let module_name = self
            .capabilities
            .get(&request.capability)
            .ok_or_else(|| ModuleError::UnknownCapability(request.capability.as_str().into()))?;
        let module = self
            .modules
            .get(module_name)
            .ok_or_else(|| ModuleError::UnknownCapability(request.capability.as_str().into()))?;
        let capability = module
            .descriptor()
            .capabilities
            .into_iter()
            .find(|entry| entry.id == request.capability)
            .ok_or_else(|| ModuleError::UnknownCapability(request.capability.as_str().into()))?;
        if let Availability::Unavailable { limitation } = capability.availability {
            return Err(ModuleError::UnavailableCapability(limitation));
        }
        let capability_id = request.capability.clone();
        let output = module.operate(request, host)?;
        if output.value.get().len() > 1024 * 1024 {
            return Err(ModuleError::Invalid("operation result is oversized".into()));
        }
        module.validate_operation_result(&capability_id, &output.value)?;
        Ok(output)
    }
    pub fn capability_owner(&self, capability: &CapabilityId) -> ModuleResult<(ModuleId, u32)> {
        let contract = self.capability_contract(capability)?;
        Ok((contract.owner, contract.schema_version))
    }
    pub fn capability_contract(
        &self,
        capability: &CapabilityId,
    ) -> ModuleResult<RegisteredOperation> {
        let module_name = self
            .capabilities
            .get(capability)
            .ok_or_else(|| ModuleError::UnknownCapability(capability.as_str().into()))?;
        let descriptor = self
            .modules
            .get(module_name)
            .ok_or_else(|| ModuleError::UnknownCapability(capability.as_str().into()))?
            .descriptor();
        let scope = descriptor
            .capabilities
            .iter()
            .find(|registered| &registered.id == capability)
            .ok_or_else(|| ModuleError::UnknownCapability(capability.as_str().into()))?
            .scope;
        Ok(RegisteredOperation {
            owner: descriptor.id,
            schema_version: descriptor.schema_version,
            scope,
        })
    }
    pub fn descriptors(&self) -> Vec<ModuleDescriptor> {
        self.modules
            .values()
            .map(|module| module.descriptor())
            .collect()
    }

    /// A registered descriptor is the module-level availability contract. A
    /// module may intentionally expose individual operations as unavailable.
    pub fn has_module(&self, id: &ModuleId) -> bool {
        self.modules.contains_key(&id.0)
    }
}

fn safe_descriptor_text(value: &str, maximum: usize) -> bool {
    !value.trim().is_empty()
        && value.len() <= maximum
        && !value.chars().any(char::is_control)
        && !value.contains(['<', '>'])
}

fn valid_field_key(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

fn ensure_additive(source: &RawJson, target: &RawJson) -> ModuleResult<()> {
    fn visit(
        source: &serde_json::Value,
        target: &serde_json::Value,
        path: &str,
    ) -> ModuleResult<()> {
        match (source, target) {
            (serde_json::Value::Object(left), serde_json::Value::Object(right)) => {
                for (key, value) in left {
                    let next = right.get(key).ok_or_else(|| {
                        ModuleError::Invalid(format!("migration deleted {path}/{key}"))
                    })?;
                    visit(value, next, &format!("{path}/{key}"))?;
                }
                Ok(())
            }
            (serde_json::Value::Array(left), serde_json::Value::Array(right))
                if left.len() <= right.len() =>
            {
                for (index, value) in left.iter().enumerate() {
                    visit(value, &right[index], &format!("{path}/{index}"))?;
                }
                Ok(())
            }
            _ if source == target => Ok(()),
            _ => Err(ModuleError::Invalid(format!("migration overwrote {path}"))),
        }
    }
    visit(
        &source
            .value()
            .map_err(|error| ModuleError::Invalid(error.to_string()))?,
        &target
            .value()
            .map_err(|error| ModuleError::Invalid(error.to_string()))?,
        "",
    )
}

fn generic_presentation(data: &ModuleData, limitation: &str) -> ModulePresentation {
    ModulePresentation {
        module_id: data.module.0.clone(),
        schema_version: data.schema_version,
        available: false,
        fields: vec![
            PresentationText {
                label: "Module".into(),
                value: data.module.0.clone(),
            },
            PresentationText {
                label: "Schema".into(),
                value: data.schema_version.to_string(),
            },
        ],
        values: vec![PresentationValue {
            label: "Preserved raw data".into(),
            value: data.payload.value().unwrap_or(serde_json::Value::Null),
        }],
        limitations: vec![limitation.into()],
        ..Default::default()
    }
}

pub(crate) fn descriptor(
    id: &str,
    name: &str,
    fields: Vec<DomainFieldDescriptor>,
) -> ModuleDescriptor {
    let capability = |suffix: &str, display_name: &str, availability| Capability {
        id: CapabilityId::new(format!("{id}.{suffix}")).unwrap(),
        display_name: display_name.into(),
        availability,
        scope: OperationScope::CollectionDomain,
    };
    ModuleDescriptor {
        id: ModuleId(id.into()),
        schema_version: 1,
        display_name: name.into(),
        collection_type_label: name.into(),
        fields,
        capabilities: vec![
            capability(
                "import-analysis",
                "Import analysis",
                Availability::Unavailable {
                    limitation: "Built-in import analyzer migration is pending D04/D05; review may continue unidentified."
                        .into(),
                },
            ),
            capability(
                "catalog-verification",
                "Catalog verification",
                Availability::Unavailable {
                    limitation:
                        "Analyzer/provider migration pending; integrity verification is separate."
                            .into(),
                },
            ),
        ],
    }
}
pub(crate) fn basic_presentation(
    data: &ModuleData,
    descriptor: ModuleDescriptor,
) -> ModulePresentation {
    let values = data
        .payload
        .value()
        .ok()
        .and_then(|value| value.as_object().cloned())
        .unwrap_or_default();
    let mut fields = vec![PresentationText {
        label: "Collection type".into(),
        value: descriptor.collection_type_label.clone(),
    }];
    for field in &descriptor.fields {
        let Some(value) = values.get(&field.key) else {
            continue;
        };
        let rendered = match (&field.kind, value) {
            (_, serde_json::Value::Null) => continue,
            (DomainFieldKind::Text { .. }, serde_json::Value::String(value)) => value.clone(),
            (DomainFieldKind::Integer { .. }, serde_json::Value::Number(value)) => {
                value.to_string()
            }
            (DomainFieldKind::Structured, _) => "Preserved (read-only)".into(),
            _ => continue,
        };
        fields.push(PresentationText {
            label: field.label.clone(),
            value: rendered,
        });
    }
    ModulePresentation {
        module_id: data.module.0.clone(),
        schema_version: data.schema_version,
        available: true,
        fields,
        capabilities: descriptor
            .capabilities
            .into_iter()
            .map(|value| CapabilityAffordance {
                id: value.id,
                label: value.display_name,
                availability: value.availability,
            })
            .collect(),
        ..Default::default()
    }
}
/// Deterministic best guess. Identity strength precedes preferred language; manual choice survives refresh.
pub fn resolve_metadata(
    item: &mut Item,
    candidates: Vec<MetadataCandidate>,
    preferred_language: &str,
) {
    let manual = item
        .metadata
        .selected
        .as_ref()
        .and_then(|id| candidates.iter().find(|candidate| &candidate.id == id));
    let selected = manual.or_else(|| {
        candidates.iter().max_by(|left, right| {
            (left.strength, left.language.as_str() == preferred_language)
                .cmp(&(
                    right.strength,
                    right.language.as_str() == preferred_language,
                ))
                .then_with(|| right.id.cmp(&left.id))
        })
    });
    if let Some(selected) = selected {
        if !item.metadata.overrides.contains_key("title") {
            item.title = selected.title.clone();
        }
        if !item.metadata.overrides.contains_key("subtitle") {
            item.subtitle = selected.subtitle.clone();
        }
        if manual.is_none() && item.identity != IdentityState::Confirmed {
            item.identity = IdentityState::Tentative;
        }
    }
    item.metadata.candidates = candidates;
}
