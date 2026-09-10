//! Portable collection contracts. No filesystem, database, transport or UI dependencies.
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Deserializer, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    fmt,
    str::FromStr,
};
use uuid::Uuid;

macro_rules! identity {
    ($($name:ident),+ $(,)?) => {$ (
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, utoipa::ToSchema)]
        #[serde(transparent)]
        pub struct $name(pub Uuid);
        impl $name { pub fn new() -> Self { Self(Uuid::now_v7()) } }
        impl Default for $name { fn default() -> Self { Self::new() } }
        impl std::fmt::Display for $name { fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { self.0.fmt(f) } }
        impl std::str::FromStr for $name { type Err = uuid::Error; fn from_str(s: &str) -> std::result::Result<Self, Self::Err> { Ok(Self(s.parse()?)) } }
    )+};
}
identity!(
    ServerId,
    CollectionId,
    ItemId,
    WorkId,
    EditionId,
    SpecimenId,
    ComponentId,
    CarrierId,
    CaptureId,
    RepresentationId,
    AssetId,
    EvidenceId,
    UserId,
    DeviceId,
    SessionId,
    PasskeyId,
    EnrollmentId,
    NativeCredentialId,
    JobId,
    ImportId,
    TransactionId,
    LoanId
);

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    utoipa::ToSchema,
)]
#[serde(transparent)]
pub struct Revision(pub u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(transparent)]
pub struct ByteLength(pub u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(transparent)]
pub struct SampleFrame(pub u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(transparent)]
pub struct DiscLba(pub i32);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, utoipa::ToSchema)]
pub struct Digest {
    pub algorithm: HashAlgorithm,
    pub hex: String,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum HashAlgorithm {
    Sha256,
    Sha1,
    Md5,
    Crc32,
    AccurateRipV1,
    AccurateRipV2,
}
impl Digest {
    pub fn sha256(bytes: &[u8]) -> Self {
        use sha2::{Digest as _, Sha256};
        Self {
            algorithm: HashAlgorithm::Sha256,
            hex: hex::encode(Sha256::digest(bytes)),
        }
    }
    pub fn validate(&self) -> Result<()> {
        let length = match self.algorithm {
            HashAlgorithm::Sha256 => 64,
            HashAlgorithm::Sha1 => 40,
            HashAlgorithm::Md5 => 32,
            _ => 8,
        };
        if self.hex.len() != length || !self.hex.bytes().all(|c| c.is_ascii_hexdigit()) {
            return Err(Error::Invalid("invalid digest encoding".into()));
        }
        Ok(())
    }

    pub fn new(algorithm: HashAlgorithm, value: impl Into<String>) -> Result<Self> {
        let digest = Self {
            algorithm,
            hex: value.into().to_ascii_lowercase(),
        };
        digest.validate()?;
        Ok(digest)
    }
}

impl<'de> Deserialize<'de> for Digest {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Wire {
            algorithm: HashAlgorithm,
            hex: String,
        }
        let wire = Wire::deserialize(deserializer)?;
        Digest::new(wire.algorithm, wire.hex).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(transparent)]
pub struct ModuleId(pub String);
impl ModuleId {
    pub fn validate(&self) -> Result<()> {
        if self.0.is_empty()
            || self.0.len() > 80
            || !self
                .0
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'.' || b == b'-')
        {
            return Err(Error::Invalid("invalid module namespace".into()));
        }
        Ok(())
    }
}

/// A lossless JSON envelope. Deserializing and serializing this value does not
/// rebuild a `Value`, so unknown key order, number spelling, and explicit nulls survive.
pub struct RawJson(Box<serde_json::value::RawValue>);
impl RawJson {
    pub fn from_json(json: impl Into<String>) -> Result<Self> {
        serde_json::value::RawValue::from_string(json.into())
            .map(Self)
            .map_err(|e| Error::Invalid(format!("invalid raw JSON: {e}")))
    }
    pub fn from_value(value: serde_json::Value) -> Self {
        Self::from_json(value.to_string()).expect("serde_json::Value always serializes as JSON")
    }
    pub fn get(&self) -> &str {
        self.0.get()
    }
    pub fn parse<T: serde::de::DeserializeOwned>(&self) -> Result<T> {
        serde_json::from_str(self.get()).map_err(|e| Error::Invalid(e.to_string()))
    }
    pub fn value(&self) -> Result<serde_json::Value> {
        self.parse()
    }
}
impl Clone for RawJson {
    fn clone(&self) -> Self {
        Self::from_json(self.get()).expect("stored RawJson is valid")
    }
}
impl fmt::Debug for RawJson {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_tuple("RawJson").field(&self.get()).finish()
    }
}
impl PartialEq for RawJson {
    fn eq(&self, other: &Self) -> bool {
        self.get() == other.get()
    }
}
impl Eq for RawJson {}
impl Serialize for RawJson {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for RawJson {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        Box::<serde_json::value::RawValue>::deserialize(deserializer).map(Self)
    }
}

/// JSON is the versioned module boundary; known modules parse borrowed/owned views
/// while the original envelope remains the provenance record.
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ModuleData {
    pub module: ModuleId,
    pub schema_version: u32,
    #[schema(value_type = serde_json::Value)]
    pub payload: RawJson,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, utoipa::ToSchema)]
#[serde(transparent)]
pub struct CurrencyCode(String);
impl CurrencyCode {
    pub fn new(value: impl AsRef<str>) -> Result<Self> {
        let value = value.as_ref().to_ascii_uppercase();
        if value.len() != 3 || !value.bytes().all(|byte| byte.is_ascii_uppercase()) {
            return Err(Error::Invalid(
                "currency must be a three-letter ISO-style code".into(),
            ));
        }
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl fmt::Display for CurrencyCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl FromStr for CurrencyCode {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        Self::new(s)
    }
}
impl<'de> Deserialize<'de> for CurrencyCode {
    fn deserialize<D: Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        Self::new(String::deserialize(d)?).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, utoipa::ToSchema)]
#[serde(transparent)]
pub struct LanguageTag(String);
impl LanguageTag {
    pub fn new(value: impl AsRef<str>) -> Result<Self> {
        // Validate syntax without restricting tags to a bundled, potentially stale
        // IANA registry. The parser also handles extensions and private-use tags.
        let parsed = language_tags::LanguageTag::parse(value.as_ref())
            .map_err(|error| Error::Invalid(format!("invalid BCP 47 language tag: {error}")))?;
        let mut variants = HashSet::new();
        let mut extensions = HashSet::new();
        if parsed
            .variant_subtags()
            .any(|v| !variants.insert(v.to_ascii_lowercase()))
            || parsed
                .extension_subtags()
                .any(|(key, _)| !extensions.insert(key.to_ascii_lowercase()))
        {
            return Err(Error::Invalid(
                "duplicate language variant or extension".into(),
            ));
        }
        Ok(Self(parsed.into_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl fmt::Display for LanguageTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl FromStr for LanguageTag {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        Self::new(s)
    }
}
impl<'de> Deserialize<'de> for LanguageTag {
    fn deserialize<D: Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        Self::new(String::deserialize(d)?).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, utoipa::ToSchema)]
pub struct GeoCoordinate {
    pub latitude: f64,
    pub longitude: f64,
}
impl GeoCoordinate {
    pub fn new(latitude: f64, longitude: f64) -> Result<Self> {
        if !latitude.is_finite()
            || !longitude.is_finite()
            || !(-90.0..=90.0).contains(&latitude)
            || !(-180.0..=180.0).contains(&longitude)
        {
            return Err(Error::Invalid(
                "geographic coordinates are not finite or in range".into(),
            ));
        }
        Ok(Self {
            latitude,
            longitude,
        })
    }
}
impl<'de> Deserialize<'de> for GeoCoordinate {
    fn deserialize<D: Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Wire {
            latitude: f64,
            longitude: f64,
        }
        let wire = Wire::deserialize(d)?;
        Self::new(wire.latitude, wire.longitude).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Money {
    pub minor_units: i64,
    pub currency: CurrencyCode,
}
impl Money {
    pub fn validate(&self) -> Result<()> {
        CurrencyCode::new(self.currency.as_str()).map(|_| ())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct LocalizedText {
    pub language: LanguageTag,
    pub text: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ProviderIdentity {
    pub provider: ModuleId,
    pub value: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Collection {
    pub id: CollectionId,
    pub revision: Revision,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Work {
    pub id: WorkId,
    pub collection_id: CollectionId,
    pub revision: Revision,
    pub provisional: bool,
    pub title: String,
    pub translations: Vec<LocalizedText>,
    pub identifiers: Vec<ProviderIdentity>,
    #[serde(default)]
    pub migration_provenance: Vec<LegacyRelationshipProvenance>,
    pub deleted: bool,
    pub updated_at: DateTime<Utc>,
}
impl Work {
    pub fn validate(&self) -> Result<()> {
        if !self.provisional && self.title.trim().is_empty() {
            return Err(Error::Invalid(
                "a non-provisional work requires a title".into(),
            ));
        }
        if self.title.len() > 4096 {
            return Err(Error::Invalid("work title is too long".into()));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Edition {
    pub id: EditionId,
    pub collection_id: CollectionId,
    pub revision: Revision,
    /// `None` is permitted only while `provisional` is true.
    pub work_id: Option<WorkId>,
    pub provisional: bool,
    pub title: String,
    #[serde(default)]
    pub expected_components: Vec<ExpectedComponent>,
    pub identifiers: Vec<ProviderIdentity>,
    #[serde(default)]
    pub migration_provenance: Vec<LegacyRelationshipProvenance>,
    pub deleted: bool,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct LegacyRelationshipProvenance {
    pub item_id: ItemId,
    pub legacy_field: String,
}
impl Edition {
    pub fn validate(&self) -> Result<()> {
        if self.work_id.is_none() && !self.provisional {
            return Err(Error::Invalid(
                "a non-provisional edition must identify its work".into(),
            ));
        }
        let mut positions = BTreeSet::new();
        let mut previous = None;
        for expected in &self.expected_components {
            if !positions.insert(expected.position) {
                return Err(Error::Invalid(
                    "duplicate expected component position".into(),
                ));
            }
            if previous.is_some_and(|value| expected.position <= value) {
                return Err(Error::Invalid(
                    "expected component slots must be ordered".into(),
                ));
            }
            previous = Some(expected.position);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ExpectedComponent {
    pub position: u16,
    pub label: String,
    pub kind: ComponentKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ItemIdentityLink {
    None,
    Work { work_id: WorkId },
    Edition { edition_id: EditionId },
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Item {
    pub id: ItemId,
    pub collection_id: CollectionId,
    pub revision: Revision,
    pub title: String,
    pub subtitle: String,
    pub identity_link: ItemIdentityLink,
    pub domain: ModuleData,
    pub identity: IdentityState,
    pub translations: Vec<LocalizedText>,
    pub identifiers: Vec<ProviderIdentity>,
    pub specimens: Vec<Specimen>,
    pub wishlist: bool,
    pub captures: Vec<Capture>,
    pub assets: Vec<Asset>,
    pub metadata: MetadataState,
    pub deleted: bool,
    pub updated_at: DateTime<Utc>,
}
impl Item {
    pub fn validate(&self) -> Result<()> {
        self.domain.module.validate()?;
        if self.title.trim().is_empty() || self.title.len() > 4096 || self.subtitle.len() > 4096 {
            return Err(Error::Invalid(
                "title is required and text must be at most 4096 bytes".into(),
            ));
        }
        if self.domain.schema_version == 0 {
            return Err(Error::Invalid("module schema version is required".into()));
        }
        if self.specimens.len() > 1000 || self.assets.len() > 10000 {
            return Err(Error::Invalid("item is too large".into()));
        }
        let mut ids = HashSet::new();
        let mut component_ids = HashSet::new();
        let mut carrier_ids = HashSet::new();
        for specimen in &self.specimens {
            if !ids.insert(specimen.id) {
                return Err(Error::Invalid("duplicate specimen identity".into()));
            }
            if let Some(money) = specimen.acquisition.as_ref().and_then(|a| a.cost.as_ref()) {
                money.validate()?;
            }
            let mut positions = BTreeSet::new();
            let mut previous_position = None;
            for component in &specimen.components {
                if !component_ids.insert(component.id) {
                    return Err(Error::Invalid("duplicate component identity".into()));
                }
                if !positions.insert(component.position) {
                    return Err(Error::Invalid(
                        "duplicate component position in specimen".into(),
                    ));
                }
                if previous_position.is_some_and(|value| component.position <= value) {
                    return Err(Error::Invalid(
                        "specimen components must be ordered by position".into(),
                    ));
                }
                previous_position = Some(component.position);
                if let Some(carrier) = &component.carrier {
                    if !component.kind.data_bearing() {
                        return Err(Error::Invalid(
                            "a non-data-bearing component cannot have a carrier".into(),
                        ));
                    }
                    if !carrier_ids.insert(carrier.id) {
                        return Err(Error::Invalid("duplicate carrier identity".into()));
                    }
                    if matches!(carrier.kind, CarrierKind::Case | CarrierKind::Booklet) {
                        return Err(Error::Invalid(
                            "cases and booklets cannot have carrier identity".into(),
                        ));
                    }
                }
            }
            if specimen
                .loans
                .iter()
                .filter(|l| l.returned_at.is_none())
                .count()
                > 1
            {
                return Err(Error::Invalid(
                    "a specimen cannot have multiple active loans".into(),
                ));
            }
        }
        for translation in &self.translations {
            LanguageTag::new(translation.language.as_str())?;
        }
        for capture in &self.captures {
            capture.validate()?;
            if capture.collection_id != self.collection_id {
                return Err(Error::Invalid("capture crosses collection boundary".into()));
            }
            if let Some(id) = capture.association.item_id
                && id != self.id
            {
                return Err(Error::Invalid(
                    "embedded compatibility capture names another item".into(),
                ));
            }
            if let Some(id) = capture.association.specimen_id
                && !ids.contains(&id)
            {
                return Err(Error::Invalid(
                    "capture references a specimen outside its item".into(),
                ));
            }
            if let Some(id) = capture.association.component_id
                && !component_ids.contains(&id)
            {
                return Err(Error::Invalid(
                    "capture references a component outside its item".into(),
                ));
            }
            if let Some(id) = capture.association.carrier_id
                && !carrier_ids.contains(&id)
            {
                return Err(Error::Invalid(
                    "capture references a carrier outside its item".into(),
                ));
            }
        }
        let capture_ids = self
            .captures
            .iter()
            .map(|capture| capture.id)
            .collect::<HashSet<_>>();
        for asset in &self.assets {
            asset.file.digest.validate()?;
            if let Some(thumbnail) = &asset.thumbnail {
                thumbnail.digest.validate()?;
            }
            let valid = match asset.subject {
                AssetSubject::Item { item_id } => item_id == self.id,
                AssetSubject::Specimen { specimen_id } => ids.contains(&specimen_id),
                AssetSubject::Component { component_id } => component_ids.contains(&component_id),
                AssetSubject::Capture { capture_id } => capture_ids.contains(&capture_id),
                AssetSubject::Work { .. } | AssetSubject::Edition { .. } => true,
            };
            if !valid {
                return Err(Error::Invalid(
                    "asset subject is outside its compatibility item view".into(),
                ));
            }
        }
        Ok(())
    }
    pub fn redact_for(&mut self, role: Role) {
        if role == Role::Viewer {
            for specimen in &mut self.specimens {
                specimen.acquisition = None;
                specimen.private_notes = None;
                specimen.loans.clear();
            }
            self.assets.retain(|a| a.visibility == Visibility::Shared);
            // Provider observations and manual overrides can contain private source data.
            self.metadata = MetadataState::default();
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum IdentityState {
    Unidentified,
    Tentative,
    Selected,
    Confirmed,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Specimen {
    pub id: SpecimenId,
    pub label: String,
    pub condition: Condition,
    pub ownership: Ownership,
    pub possession: Possession,
    pub components: Vec<Component>,
    pub acquisition: Option<Acquisition>,
    pub private_notes: Option<String>,
    pub loans: Vec<Loan>,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Condition {
    Unknown,
    Sealed,
    Excellent,
    Good,
    Fair,
    Poor,
    Incomplete,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum Ownership {
    Owned,
    Borrowed,
    Unknown,
    Disposed { date: NaiveDate, reason: String },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Possession {
    Present,
    Lent,
    Elsewhere,
    Unknown,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Acquisition {
    pub date: Option<NaiveDate>,
    pub cost: Option<Money>,
    pub source: Option<String>,
    #[serde(default)]
    pub coordinates: Option<GeoCoordinate>,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Loan {
    pub id: LoanId,
    pub contact: String,
    pub lent_at: NaiveDate,
    pub due_at: Option<NaiveDate>,
    pub returned_at: Option<NaiveDate>,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Component {
    pub id: ComponentId,
    pub label: String,
    pub kind: ComponentKind,
    pub position: u16,
    pub carrier: Option<Carrier>,
    #[serde(default)]
    pub migration_provenance: Option<LegacyComponentProvenance>,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct LegacyComponentProvenance {
    pub legacy_carrier_id: CarrierId,
    pub legacy_kind: CarrierKind,
    pub legacy_position: Option<u16>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ComponentKind {
    Cartridge,
    OpticalDisc,
    Tape,
    MemoryCard,
    Case,
    Booklet,
    Insert,
    OtherData,
    OtherPhysical,
}
impl ComponentKind {
    pub fn data_bearing(self) -> bool {
        matches!(
            self,
            Self::Cartridge | Self::OpticalDisc | Self::Tape | Self::MemoryCard | Self::OtherData
        )
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Carrier {
    pub id: CarrierId,
    pub label: String,
    pub kind: CarrierKind,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CarrierKind {
    Cartridge,
    OpticalDisc,
    Tape,
    Card,
    Booklet,
    Case,
    Other,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Capture {
    pub id: CaptureId,
    pub collection_id: CollectionId,
    pub revision: Revision,
    pub association: CaptureAssociation,
    pub created_at: DateTime<Utc>,
    pub representations: Vec<Representation>,
}
impl Capture {
    pub fn validate(&self) -> Result<()> {
        let ids = self
            .representations
            .iter()
            .map(|r| r.id)
            .collect::<HashSet<_>>();
        if ids.len() != self.representations.len() {
            return Err(Error::Invalid("duplicate representation identity".into()));
        }
        for representation in &self.representations {
            for file in &representation.files {
                file.digest.validate()?;
            }
            let mut seen = HashSet::new();
            let mut source = representation.source;
            while let Some(id) = source {
                if !ids.contains(&id) || !seen.insert(id) || id == representation.id {
                    return Err(Error::Invalid(
                        "representation sources must be local and acyclic".into(),
                    ));
                }
                source = self
                    .representations
                    .iter()
                    .find(|r| r.id == id)
                    .and_then(|r| r.source);
            }
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CaptureAssociation {
    pub item_id: Option<ItemId>,
    pub specimen_id: Option<SpecimenId>,
    pub component_id: Option<ComponentId>,
    pub carrier_id: Option<CarrierId>,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Representation {
    pub id: RepresentationId,
    pub role: RepresentationRole,
    pub format: String,
    pub files: Vec<StoredFile>,
    pub source: Option<RepresentationId>,
    pub profile: Option<ConversionProfile>,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum RepresentationRole {
    PreservationMaster,
    LosslessDerivative,
    LossyDerivative,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct StoredFile {
    pub name: String,
    pub bytes: ByteLength,
    pub digest: Digest,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(tag = "codec", rename_all = "snake_case")]
pub enum ConversionProfile {
    Flac,
    Opus { bitrate_kbps: u16 },
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Asset {
    pub id: AssetId,
    pub subject: AssetSubject,
    pub kind: AssetKind,
    pub visibility: Visibility,
    pub media_type: String,
    pub file: StoredFile,
    pub thumbnail: Option<StoredFile>,
    pub alt_text: String,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AssetSubject {
    Item { item_id: ItemId },
    Specimen { specimen_id: SpecimenId },
    Component { component_id: ComponentId },
    Capture { capture_id: CaptureId },
    Work { work_id: WorkId },
    Edition { edition_id: EditionId },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AssetKind {
    Cover,
    Photo,
    Scan,
    Document,
    Other,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    Shared,
    Curators,
}

/// Canonical collection-level aggregate used by schema-2 migration and reindex.
/// `Item.captures`/`Item.assets` remain compatibility projections for the existing
/// HTTP shape; new persistence treats these vectors as views of these top-level rows.
#[derive(Clone, Debug, Default, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CollectionGraph {
    pub works: Vec<Work>,
    pub editions: Vec<Edition>,
    pub items: Vec<Item>,
    pub captures: Vec<Capture>,
    pub assets: Vec<Asset>,
}
impl CollectionGraph {
    /// The archive can contain several collections, but durable entity IDs cannot
    /// be reused between them. Storage supplies the other collection snapshots.
    pub fn validate_disjoint(&self, other: &Self) -> Result<()> {
        fn ids(graph: &CollectionGraph) -> HashSet<(&'static str, Uuid)> {
            let mut ids = HashSet::new();
            ids.extend(graph.works.iter().map(|v| ("work", v.id.0)));
            ids.extend(graph.editions.iter().map(|v| ("edition", v.id.0)));
            ids.extend(graph.items.iter().map(|v| ("item", v.id.0)));
            ids.extend(graph.assets.iter().map(|v| ("asset", v.id.0)));
            for item in &graph.items {
                for specimen in &item.specimens {
                    ids.insert(("specimen", specimen.id.0));
                    ids.extend(specimen.loans.iter().map(|v| ("loan", v.id.0)));
                    for component in &specimen.components {
                        ids.insert(("component", component.id.0));
                        if let Some(carrier) = &component.carrier {
                            ids.insert(("carrier", carrier.id.0));
                        }
                    }
                }
            }
            for capture in &graph.captures {
                ids.insert(("capture", capture.id.0));
                ids.extend(
                    capture
                        .representations
                        .iter()
                        .map(|v| ("representation", v.id.0)),
                );
            }
            ids
        }
        if !ids(self).is_disjoint(&ids(other)) {
            return Err(Error::Invalid(
                "entity identity is reused across collections".into(),
            ));
        }
        Ok(())
    }
    pub fn validate(&self, collection_id: CollectionId) -> Result<()> {
        let unique = |values: Vec<Uuid>, label: &str| -> Result<()> {
            let count = values.len();
            if values.into_iter().collect::<HashSet<_>>().len() != count {
                return Err(Error::Invalid(format!("duplicate {label} identity")));
            }
            Ok(())
        };
        unique(self.works.iter().map(|value| value.id.0).collect(), "work")?;
        unique(
            self.editions.iter().map(|value| value.id.0).collect(),
            "edition",
        )?;
        unique(self.items.iter().map(|value| value.id.0).collect(), "item")?;
        unique(
            self.captures.iter().map(|value| value.id.0).collect(),
            "capture",
        )?;
        unique(
            self.items
                .iter()
                .flat_map(|item| &item.specimens)
                .map(|value| value.id.0)
                .collect(),
            "specimen",
        )?;
        unique(
            self.items
                .iter()
                .flat_map(|item| &item.specimens)
                .flat_map(|s| &s.components)
                .map(|value| value.id.0)
                .collect(),
            "component",
        )?;
        unique(
            self.items
                .iter()
                .flat_map(|item| &item.specimens)
                .flat_map(|s| &s.components)
                .filter_map(|c| c.carrier.as_ref())
                .map(|value| value.id.0)
                .collect(),
            "carrier",
        )?;
        unique(
            self.items
                .iter()
                .flat_map(|item| &item.specimens)
                .flat_map(|s| &s.loans)
                .map(|value| value.id.0)
                .collect(),
            "loan",
        )?;
        unique(
            self.captures
                .iter()
                .flat_map(|c| &c.representations)
                .map(|value| value.id.0)
                .collect(),
            "representation",
        )?;
        unique(
            self.assets.iter().map(|value| value.id.0).collect(),
            "asset",
        )?;
        let works = self
            .works
            .iter()
            .map(|work| work.id)
            .collect::<HashSet<_>>();
        let editions = self
            .editions
            .iter()
            .map(|edition| (edition.id, edition.work_id))
            .collect::<BTreeMap<_, _>>();
        let items = self
            .items
            .iter()
            .map(|item| item.id)
            .collect::<HashSet<_>>();
        let specimens = self
            .items
            .iter()
            .flat_map(|item| item.specimens.iter())
            .map(|value| value.id)
            .collect::<HashSet<_>>();
        let components = self
            .items
            .iter()
            .flat_map(|item| item.specimens.iter())
            .flat_map(|value| value.components.iter())
            .map(|value| value.id)
            .collect::<HashSet<_>>();
        let carriers = self
            .items
            .iter()
            .flat_map(|item| item.specimens.iter())
            .flat_map(|value| value.components.iter())
            .filter_map(|value| value.carrier.as_ref())
            .map(|value| value.id)
            .collect::<HashSet<_>>();
        let specimen_owner = self
            .items
            .iter()
            .flat_map(|item| {
                item.specimens
                    .iter()
                    .map(move |specimen| (specimen.id, item.id))
            })
            .collect::<BTreeMap<_, _>>();
        let component_owner = self
            .items
            .iter()
            .flat_map(|item| item.specimens.iter())
            .flat_map(|specimen| {
                specimen
                    .components
                    .iter()
                    .map(move |component| (component.id, specimen.id))
            })
            .collect::<BTreeMap<_, _>>();
        let carrier_owner = self
            .items
            .iter()
            .flat_map(|item| item.specimens.iter())
            .flat_map(|specimen| specimen.components.iter())
            .filter_map(|component| {
                component
                    .carrier
                    .as_ref()
                    .map(|carrier| (carrier.id, component.id))
            })
            .collect::<BTreeMap<_, _>>();
        for work in &self.works {
            work.validate()?;
            if work.collection_id != collection_id {
                return Err(Error::Invalid("work crosses collection boundary".into()));
            }
        }
        for edition in &self.editions {
            edition.validate()?;
            if edition.collection_id != collection_id {
                return Err(Error::Invalid("edition crosses collection boundary".into()));
            }
            if let Some(work) = edition.work_id
                && !works.contains(&work)
            {
                return Err(Error::Invalid("edition references an absent work".into()));
            }
        }
        for item in &self.items {
            item.validate()?;
            if item.collection_id != collection_id {
                return Err(Error::Invalid("item crosses collection boundary".into()));
            }
            match item.identity_link {
                ItemIdentityLink::None => {}
                ItemIdentityLink::Work { work_id } if works.contains(&work_id) => {}
                ItemIdentityLink::Edition { edition_id } if editions.contains_key(&edition_id) => {}
                _ => {
                    return Err(Error::Invalid(
                        "item identity references an absent collection record".into(),
                    ));
                }
            }
        }
        for capture in &self.captures {
            capture.validate()?;
            if capture.collection_id != collection_id {
                return Err(Error::Invalid("capture crosses collection boundary".into()));
            }
            let association = capture.association;
            if association.item_id.is_some_and(|id| !items.contains(&id))
                || association
                    .specimen_id
                    .is_some_and(|id| !specimens.contains(&id))
                || association
                    .component_id
                    .is_some_and(|id| !components.contains(&id))
                || association
                    .carrier_id
                    .is_some_and(|id| !carriers.contains(&id))
            {
                return Err(Error::Invalid(
                    "capture association references an absent collection record".into(),
                ));
            }
            let component = association.component_id.or_else(|| {
                association
                    .carrier_id
                    .and_then(|id| carrier_owner.get(&id).copied())
            });
            let specimen = association
                .specimen_id
                .or_else(|| component.and_then(|id| component_owner.get(&id).copied()));
            if association
                .carrier_id
                .is_some_and(|id| carrier_owner.get(&id).copied() != component)
                || component.is_some_and(|id| component_owner.get(&id).copied() != specimen)
                || association.item_id.is_some_and(|item| {
                    specimen.is_some_and(|id| specimen_owner.get(&id) != Some(&item))
                })
            {
                return Err(Error::Invalid(
                    "capture physical links do not form one ownership chain".into(),
                ));
            }
        }
        for asset in &self.assets {
            asset.file.digest.validate()?;
            if let Some(thumbnail) = &asset.thumbnail {
                thumbnail.digest.validate()?;
            }
            let exists = match asset.subject {
                AssetSubject::Work { work_id } => works.contains(&work_id),
                AssetSubject::Edition { edition_id } => editions.contains_key(&edition_id),
                AssetSubject::Item { item_id } => items.contains(&item_id),
                AssetSubject::Specimen { specimen_id } => specimens.contains(&specimen_id),
                AssetSubject::Component { component_id } => components.contains(&component_id),
                AssetSubject::Capture { capture_id } => {
                    self.captures.iter().any(|c| c.id == capture_id)
                }
            };
            if !exists {
                return Err(Error::Invalid(
                    "asset subject is absent from the collection".into(),
                ));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Evidence {
    pub id: EvidenceId,
    pub item_id: ItemId,
    pub representation_id: RepresentationId,
    pub kind: EvidenceKind,
    pub outcome: EvidenceOutcome,
    pub inputs: Vec<Digest>,
    pub implementation: String,
    pub implementation_version: String,
    pub catalog: Option<CatalogSnapshot>,
    pub details: serde_json::Value,
    pub created_at: DateTime<Utc>,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    Integrity,
    Structure,
    GameCatalog,
    AccurateRip,
    Reconstruction,
    DerivativeValidation,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceOutcome {
    Passed,
    Failed,
    Inconclusive,
    Unsupported,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CatalogSnapshot {
    pub provider: ModuleId,
    pub digest: Digest,
    pub retrieved_at: DateTime<Utc>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, utoipa::ToSchema)]
pub struct MetadataState {
    pub candidates: Vec<MetadataCandidate>,
    pub selected: Option<String>,
    pub overrides: BTreeMap<String, serde_json::Value>,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct MetadataCandidate {
    pub id: String,
    pub provider: ModuleId,
    pub title: String,
    pub subtitle: String,
    pub language: LanguageTag,
    pub strength: MatchStrength,
    #[schema(value_type = serde_json::Value)]
    pub observation: RawJson,
}
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, utoipa::ToSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum MatchStrength {
    Name,
    Identifier,
    DiscLayout,
    CompleteHashes,
}

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, utoipa::ToSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Viewer,
    Curator,
    Owner,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Browse,
    Curate,
    ManageAccess,
}
impl Role {
    pub fn allows(self, action: Action) -> bool {
        self >= match action {
            Action::Browse => Role::Viewer,
            Action::Curate => Role::Curator,
            Action::ManageAccess => Role::Owner,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Principal {
    pub user_id: UserId,
    pub administrator: bool,
    pub device_id: Option<DeviceId>,
}
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Exposure {
    #[serde(default)]
    pub public_curation: bool,
    #[serde(default)]
    pub public_administration: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Surface {
    Client,
    Curation,
    Administration,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ingress {
    Public,
    Private,
}
impl Exposure {
    pub fn allows(self, ingress: Ingress, surface: Surface) -> bool {
        ingress == Ingress::Private
            || match surface {
                Surface::Client => true,
                Surface::Curation => self.public_curation || self.public_administration,
                Surface::Administration => self.public_administration,
            }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<String>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, utoipa::ToSchema)]
pub struct BrowseQuery {
    pub collection_id: Option<CollectionId>,
    pub search: Option<String>,
    pub cursor: Option<String>,
    pub limit: Option<u16>,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Job {
    pub id: JobId,
    pub user_id: UserId,
    pub item_id: ItemId,
    pub kind: String,
    pub state: JobState,
    pub completed_bytes: u64,
    pub total_bytes: u64,
    pub error: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    Queued,
    Running,
    CancelRequested,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Invalid(String),
    #[error("not found")]
    NotFound,
    #[error("authentication required")]
    Unauthenticated,
    #[error("permission denied")]
    Forbidden,
    #[error("recent authentication required")]
    RecentAuthenticationRequired,
    #[error("the last enabled administrator cannot be removed")]
    LastAdministrator,
    #[error("the last enabled collection owner cannot be removed")]
    LastOwner,
    #[error("the durable default account must be reassigned first")]
    DefaultReassignmentRequired,
    #[error("enrollment expired")]
    EnrollmentExpired,
    #[error("enrollment was cancelled")]
    EnrollmentCancelled,
    #[error("credential was revoked or expired")]
    CredentialRevoked,
    #[error("revision conflict")]
    Conflict,
    #[error("idempotency key was reused with a different request")]
    IdempotencyConflict,
    #[error("command is already in progress")]
    CommandInProgress,
    #[error("job cancellation requested")]
    JobCancelled,
    #[error("publication is already committed; cancellation is too late")]
    CancellationTooLate,
    #[error("rate limited; retry after {0} seconds")]
    RateLimited(u64),
    #[error("{0}")]
    Unavailable(String),
    #[error("storage failure: {0}")]
    Storage(String),
}
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validated_values_canonicalize_during_deserialization() {
        let language: LanguageTag = serde_json::from_str("\"ZH-hant-tw\"").unwrap();
        let currency: CurrencyCode = serde_json::from_str("\"jpy\"").unwrap();
        let digest: Digest = serde_json::from_str(
            r#"{"algorithm":"sha1","hex":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"}"#,
        )
        .unwrap();
        assert_eq!(language.as_str(), "zh-Hant-TW");
        assert_eq!(currency.as_str(), "JPY");
        assert_eq!(digest.hex, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        assert!(serde_json::from_str::<GeoCoordinate>(r#"{"latitude":91,"longitude":0}"#).is_err());
        assert!(serde_json::from_str::<Digest>(r#"{"algorithm":"md5","hex":"abc"}"#).is_err());
    }

    #[test]
    fn raw_json_retains_unknown_lexical_form() {
        let raw: RawJson =
            serde_json::from_str(r#"{"z":1e+02,"a":null,"nested":{"b":2,"a":1}}"#).unwrap();
        assert_eq!(raw.get(), r#"{"z":1e+02,"a":null,"nested":{"b":2,"a":1}}"#);
        assert_eq!(serde_json::to_string(&raw).unwrap(), raw.get());
    }

    #[test]
    fn collection_graph_accepts_two_disc_copies_and_unattached_capture() {
        let collection = CollectionId::new();
        let item_id = ItemId::new();
        let component = |position| Component {
            id: ComponentId::new(),
            label: format!("Disc {position}"),
            kind: ComponentKind::OpticalDisc,
            position,
            carrier: Some(Carrier {
                id: CarrierId::new(),
                label: format!("Disc {position}"),
                kind: CarrierKind::OpticalDisc,
            }),
            migration_provenance: None,
        };
        let item = Item {
            id: item_id,
            collection_id: collection,
            revision: Revision(1),
            title: "Set".into(),
            subtitle: String::new(),
            identity_link: ItemIdentityLink::None,
            domain: ModuleData {
                module: ModuleId("org.example.unknown".into()),
                schema_version: 9,
                payload: RawJson::from_json("{}").unwrap(),
            },
            identity: IdentityState::Unidentified,
            translations: vec![],
            identifiers: vec![],
            specimens: vec![
                Specimen {
                    id: SpecimenId::new(),
                    label: "Copy 1".into(),
                    condition: Condition::Good,
                    ownership: Ownership::Owned,
                    possession: Possession::Present,
                    components: vec![component(1), component(2)],
                    acquisition: None,
                    private_notes: None,
                    loans: vec![],
                },
                Specimen {
                    id: SpecimenId::new(),
                    label: "Copy 2".into(),
                    condition: Condition::Fair,
                    ownership: Ownership::Owned,
                    possession: Possession::Elsewhere,
                    components: vec![],
                    acquisition: None,
                    private_notes: None,
                    loans: vec![],
                },
            ],
            wishlist: false,
            captures: vec![],
            assets: vec![],
            metadata: MetadataState::default(),
            deleted: false,
            updated_at: Utc::now(),
        };
        let capture = Capture {
            id: CaptureId::new(),
            collection_id: collection,
            revision: Revision(1),
            association: CaptureAssociation::default(),
            created_at: Utc::now(),
            representations: vec![],
        };
        CollectionGraph {
            items: vec![item],
            captures: vec![capture],
            ..Default::default()
        }
        .validate(collection)
        .unwrap();
    }

    #[test]
    fn physical_and_representation_invariants_fail_closed() {
        let invalid_component = Component {
            id: ComponentId::new(),
            label: "Case".into(),
            kind: ComponentKind::Case,
            position: 1,
            carrier: Some(Carrier {
                id: CarrierId::new(),
                label: "impossible".into(),
                kind: CarrierKind::Case,
            }),
            migration_provenance: None,
        };
        assert!(!invalid_component.kind.data_bearing());
        let representation = RepresentationId::new();
        let capture = Capture {
            id: CaptureId::new(),
            collection_id: CollectionId::new(),
            revision: Revision(1),
            association: CaptureAssociation::default(),
            created_at: Utc::now(),
            representations: vec![Representation {
                id: representation,
                role: RepresentationRole::PreservationMaster,
                format: "bad".into(),
                files: vec![],
                source: Some(representation),
                profile: None,
            }],
        };
        assert!(capture.validate().is_err());
    }
}

#[cfg(test)]
mod language_regressions {
    use super::*;
    #[test]
    fn private_use_extensions_and_grandfathered_tags_are_preserved() {
        for value in ["x-archive", "en-u-ca-gregory", "i-klingon", "zh-Hant-TW"] {
            let tag = LanguageTag::new(value).unwrap();
            assert_eq!(LanguageTag::new(tag.as_str()).unwrap(), tag);
        }
        for value in ["en-a", "en-1234-1234", "en--US"] {
            assert!(LanguageTag::new(value).is_err(), "{value}");
        }
    }
}
