//! Portable collection contracts. No filesystem, database, transport or UI dependencies.
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;

macro_rules! identity {
    ($($name:ident),+ $(,)?) => {$ (
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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
    CarrierId,
    CaptureId,
    RepresentationId,
    AssetId,
    EvidenceId,
    UserId,
    DeviceId,
    JobId,
    TransactionId,
    LoanId
);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Revision(pub u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ByteLength(pub u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SampleFrame(pub u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DiscLba(pub i32);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Digest {
    pub algorithm: HashAlgorithm,
    pub hex: String,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
/// JSON is only the versioned module boundary; built-in modules deserialize into owned types.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModuleData {
    pub module: ModuleId,
    pub schema_version: u32,
    pub payload: serde_json::Value,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Money {
    pub minor_units: i64,
    pub currency: String,
}
impl Money {
    pub fn validate(&self) -> Result<()> {
        if self.currency.len() != 3 || !self.currency.bytes().all(|b| b.is_ascii_uppercase()) {
            return Err(Error::Invalid(
                "currency must be a three-letter uppercase code".into(),
            ));
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LocalizedText {
    pub language: String,
    pub text: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProviderIdentity {
    pub provider: ModuleId,
    pub value: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Collection {
    pub id: CollectionId,
    pub revision: Revision,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Item {
    pub id: ItemId,
    pub collection_id: CollectionId,
    pub revision: Revision,
    pub title: String,
    pub subtitle: String,
    pub work_id: Option<WorkId>,
    pub edition_id: Option<EditionId>,
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
        let mut ids = std::collections::HashSet::new();
        for specimen in &self.specimens {
            if !ids.insert(specimen.id) {
                return Err(Error::Invalid("duplicate specimen identity".into()));
            }
            if let Some(money) = specimen.acquisition.as_ref().and_then(|a| a.cost.as_ref()) {
                money.validate()?;
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
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdentityState {
    Unidentified,
    Tentative,
    Selected,
    Confirmed,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Specimen {
    pub id: SpecimenId,
    pub label: String,
    pub condition: Condition,
    pub ownership: Ownership,
    pub possession: Possession,
    pub carriers: Vec<Carrier>,
    pub acquisition: Option<Acquisition>,
    pub private_notes: Option<String>,
    pub loans: Vec<Loan>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
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
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum Ownership {
    Owned,
    Borrowed,
    Unknown,
    Disposed { date: NaiveDate, reason: String },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Possession {
    Present,
    Lent,
    Elsewhere,
    Unknown,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Acquisition {
    pub date: Option<NaiveDate>,
    pub cost: Option<Money>,
    pub source: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Loan {
    pub id: LoanId,
    pub contact: String,
    pub lent_at: NaiveDate,
    pub due_at: Option<NaiveDate>,
    pub returned_at: Option<NaiveDate>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Carrier {
    pub id: CarrierId,
    pub label: String,
    pub kind: CarrierKind,
    pub position: Option<u16>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
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
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Capture {
    pub id: CaptureId,
    pub carrier_id: Option<CarrierId>,
    pub created_at: DateTime<Utc>,
    pub representations: Vec<Representation>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Representation {
    pub id: RepresentationId,
    pub role: RepresentationRole,
    pub format: String,
    pub files: Vec<StoredFile>,
    pub source: Option<RepresentationId>,
    pub profile: Option<ConversionProfile>,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepresentationRole {
    PreservationMaster,
    LosslessDerivative,
    LossyDerivative,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StoredFile {
    pub name: String,
    pub bytes: ByteLength,
    pub digest: Digest,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "codec", rename_all = "snake_case")]
pub enum ConversionProfile {
    Flac,
    Opus { bitrate_kbps: u16 },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Asset {
    pub id: AssetId,
    pub kind: AssetKind,
    pub visibility: Visibility,
    pub media_type: String,
    pub file: StoredFile,
    pub thumbnail: Option<StoredFile>,
    pub alt_text: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetKind {
    Cover,
    Photo,
    Scan,
    Document,
    Other,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    Shared,
    Curators,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
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
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    Integrity,
    Structure,
    GameCatalog,
    AccurateRip,
    Reconstruction,
    DerivativeValidation,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceOutcome {
    Passed,
    Failed,
    Inconclusive,
    Unsupported,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CatalogSnapshot {
    pub provider: ModuleId,
    pub digest: Digest,
    pub retrieved_at: DateTime<Utc>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MetadataState {
    pub candidates: Vec<MetadataCandidate>,
    pub selected: Option<String>,
    pub overrides: BTreeMap<String, serde_json::Value>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetadataCandidate {
    pub id: String,
    pub provider: ModuleId,
    pub title: String,
    pub subtitle: String,
    pub language: String,
    pub strength: MatchStrength,
    pub observation: serde_json::Value,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchStrength {
    Name,
    Identifier,
    DiscLayout,
    CompleteHashes,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
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
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Principal {
    pub user_id: UserId,
    pub administrator: bool,
    pub device_id: Option<DeviceId>,
}
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
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
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<String>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct BrowseQuery {
    pub collection_id: Option<CollectionId>,
    pub search: Option<String>,
    pub cursor: Option<String>,
    pub limit: Option<u16>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
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
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    Queued,
    Running,
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
    #[error("revision conflict")]
    Conflict,
    #[error("{0}")]
    Unavailable(String),
    #[error("storage failure: {0}")]
    Storage(String),
}
pub type Result<T> = std::result::Result<T, Error>;
