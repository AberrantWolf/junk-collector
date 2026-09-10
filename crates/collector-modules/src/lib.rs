//! Compiled, domain-owned module contracts. Modules receive host capabilities;
//! they do not construct storage, network, scheduler, or transport adapters.
use collector_core::*;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    io::{Read, Seek},
};

#[derive(
    Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, utoipa::ToSchema,
)]
#[serde(transparent)]
pub struct CapabilityId(String);
impl CapabilityId {
    pub fn new(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        if value.len() > 120
            || value.split('.').count() < 3
            || value.split('.').any(str::is_empty)
            || value.starts_with('.')
            || value.ends_with('.')
            || !value.bytes().all(|byte| {
                byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'-')
            })
        {
            return Err(Error::Invalid(
                "capability must be a namespaced lowercase identifier".into(),
            ));
        }
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

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
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ModuleDescriptor {
    pub id: ModuleId,
    pub schema_version: u32,
    pub display_name: String,
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
}
pub type ModuleResult<T> = std::result::Result<T, ModuleError>;
impl From<ModuleError> for Error {
    fn from(value: ModuleError) -> Self {
        let display = value.to_string();
        match value {
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
    fn representation(&self, id: RepresentationId) -> ModuleResult<Box<dyn RepresentationReader>>;
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
        result.register(Box::new(Games)).unwrap();
        result.register(Box::new(Music)).unwrap();
        result
    }
    pub fn register(&mut self, module: Box<dyn CollectionModule>) -> Result<()> {
        let descriptor = module.descriptor();
        descriptor.id.validate()?;
        if descriptor.schema_version == 0 {
            return Err(Error::Invalid("module schema version is required".into()));
        }
        if self.modules.contains_key(&descriptor.id.0) {
            return Err(Error::Invalid("duplicate module".into()));
        }
        let mut ids = std::collections::BTreeSet::new();
        for capability in &descriptor.capabilities {
            CapabilityId::new(capability.id.as_str())?;
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
        module.operate(request, host)
    }
    pub fn descriptors(&self) -> Vec<ModuleDescriptor> {
        self.modules
            .values()
            .map(|module| module.descriptor())
            .collect()
    }
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

#[derive(Clone, Debug, Default, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct GameEdition {
    pub platform: Option<String>,
    pub region: Option<String>,
    pub publisher: Option<String>,
    pub release_year: Option<u16>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct MusicEdition {
    pub artist: Option<String>,
    pub label: Option<String>,
    pub release_year: Option<u16>,
    #[serde(default)]
    pub tracks: Vec<Track>,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Track {
    pub position: u16,
    pub title: String,
    pub start: SampleFrame,
    pub length: SampleFrame,
}
pub struct Games;
pub struct Music;

fn descriptor(id: &str, name: &str) -> ModuleDescriptor {
    let capability = |suffix: &str, display_name: &str, availability| Capability {
        id: CapabilityId::new(format!("{id}.{suffix}")).unwrap(),
        display_name: display_name.into(),
        availability,
    };
    ModuleDescriptor {
        id: ModuleId(id.into()),
        schema_version: 1,
        display_name: name.into(),
        capabilities: vec![
            capability("inventory", "Inventory", Availability::Available),
            capability("preservation", "Preservation", Availability::Available),
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
fn basic_presentation(data: &ModuleData, descriptor: ModuleDescriptor) -> ModulePresentation {
    ModulePresentation {
        module_id: data.module.0.clone(),
        schema_version: data.schema_version,
        available: true,
        fields: vec![PresentationText {
            label: "Format".into(),
            value: descriptor.display_name,
        }],
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
impl CollectionModule for Games {
    fn descriptor(&self) -> ModuleDescriptor {
        descriptor("org.junk.games", "Video games")
    }
    fn validate_current(&self, data: &ModuleData) -> ModuleResult<()> {
        data.payload
            .parse::<GameEdition>()
            .map(|_| ())
            .map_err(|error| ModuleError::Invalid(error.to_string()))
    }
    fn present(&self, data: &ModuleData) -> ModuleResult<ModulePresentation> {
        self.validate_current(data)?;
        Ok(basic_presentation(data, self.descriptor()))
    }
}
impl CollectionModule for Music {
    fn descriptor(&self) -> ModuleDescriptor {
        descriptor("org.junk.music", "Music CDs")
    }
    fn validate_current(&self, data: &ModuleData) -> ModuleResult<()> {
        let music: MusicEdition = data
            .payload
            .parse()
            .map_err(|error| ModuleError::Invalid(error.to_string()))?;
        for track in music.tracks {
            track
                .start
                .0
                .checked_add(track.length.0)
                .ok_or_else(|| ModuleError::Invalid("track span overflow".into()))?;
        }
        Ok(())
    }
    fn present(&self, data: &ModuleData) -> ModuleResult<ModulePresentation> {
        self.validate_current(data)?;
        Ok(basic_presentation(data, self.descriptor()))
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
