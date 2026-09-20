//! Shared use cases. Every adapter and frontend crosses this authorization boundary.
mod album_preparation;
pub use album_preparation::*;
mod collection_types;
pub use collection_types::*;
mod derivatives;
pub use derivatives::*;
mod guided_ingest;
mod metadata;
pub use metadata::*;
mod processing;
pub mod security;

use chrono::{DateTime, Utc};
use collector_core::*;
use collector_modules::{
    CapabilityId, CatalogSnapshotInput, DomainEdit, JobSubmission, ModuleDescriptor, ModuleError,
    ModuleHost, ModulePresentation, ModuleResult, OperationEntity, OperationRequest, PackageAccess,
    Registry, RepresentationReader,
};
pub use processing::*;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sha2::Digest as _;
use std::io::{Read, Seek};
use std::sync::Arc;

/// Transitional combined Wave 1 adapter. Application services consume the
/// narrow ports below; A04.2 removes this compatibility surface after moving
/// the remaining command families behind application policy.
pub trait Catalog: Send + Sync {
    fn collections(&self, user: UserId) -> Result<Vec<Collection>>;
    fn collection(&self, id: CollectionId) -> Result<Collection>;
    fn create_collection(&self, user: UserId, collection: Collection) -> Result<Collection>;
    fn role(&self, user: UserId, collection: CollectionId) -> Result<Role>;
    fn grant(&self, collection: CollectionId, user: UserId, role: Role) -> Result<()>;
    fn browse(&self, user: UserId, query: BrowseQuery) -> Result<Page<Item>>;
    fn item(&self, id: ItemId) -> Result<Item>;
    fn save(&self, item: Item, expected: Revision) -> Result<Item>;
    fn work(&self, id: WorkId) -> Result<Work>;
    fn save_work(&self, work: Work, expected: Revision) -> Result<Work>;
    fn edition(&self, id: EditionId) -> Result<Edition>;
    fn save_edition(&self, edition: Edition, expected: Revision) -> Result<Edition>;
    fn capture(&self, id: CaptureId) -> Result<Capture>;
    fn save_capture(&self, capture: Capture, expected: Revision) -> Result<Capture>;
    fn evidence(&self, item: ItemId) -> Result<Vec<Evidence>>;
}

/// Mechanism-only reads. A read adapter never receives a Principal or decides
/// what a role is allowed to observe.
pub trait CollectionReadPort: Send + Sync {
    fn collections(&self, user: UserId) -> Result<Vec<Collection>>;
    fn collection(&self, id: CollectionId) -> Result<Collection>;
    fn role(&self, user: UserId, collection: CollectionId) -> Result<Role>;
    fn browse(&self, user: UserId, query: BrowseQuery) -> Result<Page<Item>>;
    fn item(&self, id: ItemId) -> Result<Item>;
    fn work(&self, id: WorkId) -> Result<Work>;
    fn edition(&self, id: EditionId) -> Result<Edition>;
    fn capture(&self, id: CaptureId) -> Result<Capture>;
    fn evidence(&self, item: ItemId) -> Result<Vec<Evidence>>;
}

impl<T: Catalog + ?Sized> CollectionReadPort for T {
    fn collections(&self, user: UserId) -> Result<Vec<Collection>> {
        Catalog::collections(self, user)
    }
    fn collection(&self, id: CollectionId) -> Result<Collection> {
        Catalog::collection(self, id)
    }
    fn role(&self, user: UserId, collection: CollectionId) -> Result<Role> {
        Catalog::role(self, user, collection)
    }
    fn browse(&self, user: UserId, query: BrowseQuery) -> Result<Page<Item>> {
        Catalog::browse(self, user, query)
    }
    fn item(&self, id: ItemId) -> Result<Item> {
        Catalog::item(self, id)
    }
    fn work(&self, id: WorkId) -> Result<Work> {
        Catalog::work(self, id)
    }
    fn edition(&self, id: EditionId) -> Result<Edition> {
        Catalog::edition(self, id)
    }
    fn capture(&self, id: CaptureId) -> Result<Capture> {
        Catalog::capture(self, id)
    }
    fn evidence(&self, item: ItemId) -> Result<Vec<Evidence>> {
        Catalog::evidence(self, item)
    }
}

pub trait CollectionWritePort: Send + Sync {
    fn create_collection(&self, user: UserId, collection: Collection) -> Result<Collection>;
    fn save(&self, item: Item, expected: Revision) -> Result<Item>;
    fn save_work(&self, work: Work, expected: Revision) -> Result<Work>;
    fn save_edition(&self, edition: Edition, expected: Revision) -> Result<Edition>;
    fn save_capture(&self, capture: Capture, expected: Revision) -> Result<Capture>;
}

pub trait AccessPort: Send + Sync {
    fn grant_for_user(&self, collection: CollectionId, user: UserId) -> Result<Option<GrantView>>;
    fn grants(&self, collection: CollectionId) -> Result<Vec<GrantView>>;
    fn set_grant(
        &self,
        collection: CollectionId,
        user: UserId,
        expected: Revision,
        role: Option<Role>,
    ) -> Result<Option<GrantView>>;
}

pub trait MediaReader: Read + Seek + Send {}
impl<T: Read + Seek + Send> MediaReader for T {}

pub struct MediaResource {
    pub reader: Box<dyn MediaReader>,
    pub length: u64,
    pub media_type: String,
    pub digest: Digest,
    pub download_name: String,
}

impl MediaResource {
    /// Consume an already-authorized representation as an opaque audio source.
    /// Archive paths remain confined to the storage adapter.
    pub fn into_audio_source(self) -> junk_libs_audio::ByteSource {
        let extension = self
            .download_name
            .rsplit_once('.')
            .map(|(_, extension)| extension);
        junk_libs_audio::ByteSource::new(
            self.reader,
            junk_libs_audio::MediaHint::new(Some(self.length), extension, Some(&self.media_type)),
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AssetState {
    pub collection_id: CollectionId,
    pub revision: Revision,
    pub asset: Asset,
    pub deleted: bool,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
pub struct PresentedAsset {
    pub id: AssetId,
    pub revision: Revision,
    pub subject: AssetSubject,
    pub kind: AssetKind,
    pub media_type: String,
    pub alt_text: String,
    pub updated_at: DateTime<Utc>,
    pub archived: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visibility: Option<Visibility>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original: Option<StoredFile>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail: Option<StoredFile>,
}

impl PresentedAsset {
    fn from_state(state: AssetState, role: Role) -> Self {
        let private = role != Role::Viewer;
        Self {
            id: state.asset.id,
            revision: state.revision,
            subject: state.asset.subject,
            kind: state.asset.kind,
            media_type: state.asset.media_type,
            alt_text: state.asset.alt_text,
            updated_at: state.updated_at,
            archived: state.deleted,
            visibility: private.then_some(state.asset.visibility),
            original: private.then_some(state.asset.file),
            thumbnail: private.then_some(state.asset.thumbnail).flatten(),
        }
    }
}

/// Persistence and file mechanisms after application policy has authorized an
/// effect. No method receives a Principal or Role.
pub trait ContentPort: Send + Sync {
    fn activity(&self, user: UserId, item: ItemId) -> Result<serde_json::Value>;
    fn save_activity(&self, user: UserId, item: ItemId, value: serde_json::Value) -> Result<()>;
    fn representation(
        &self,
        item: ItemId,
        representation: RepresentationId,
        index: usize,
    ) -> Result<MediaResource>;
    fn asset_resource(&self, asset: AssetId, thumbnail: bool) -> Result<MediaResource>;
    fn asset_state(&self, asset: AssetId) -> Result<AssetState>;
    fn asset_states_for_item(&self, item: ItemId) -> Result<Vec<AssetState>>;
    fn asset_subject_collection(&self, subject: AssetSubject) -> Result<CollectionId>;
    fn create_asset(
        &self,
        asset: AssetId,
        command: CreateAssetCommand,
        bytes: &[u8],
    ) -> Result<AssetState>;
    fn update_asset(
        &self,
        asset: AssetId,
        expected: Revision,
        visibility: Visibility,
        alt_text: String,
    ) -> Result<AssetState>;
    fn tombstone_asset(&self, asset: AssetId, expected: Revision) -> Result<AssetState>;
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct UploadFile {
    pub name: String,
    pub bytes: ByteLength,
    pub sha256: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Upload {
    pub id: JobId,
    pub user_id: UserId,
    pub item_id: ItemId,
    pub expected_revision: Revision,
    pub representation_id: RepresentationId,
    pub capture_id: CaptureId,
    pub files: Vec<UploadFile>,
    pub offsets: Vec<u64>,
    pub state: JobState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ImportPhase {
    Staging,
    Analyzing,
    AwaitingReview,
    Publishing,
    Published,
    Cancelled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ImportSourceKind {
    Browser,
    Inbox,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ImportSourceSummary {
    pub kind: ImportSourceKind,
    pub label: Option<String>,
    pub package_name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ImportDeclaredFile {
    pub name: String,
    pub bytes: ByteLength,
    pub received_bytes: ByteLength,
    pub digest: Option<Digest>,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ImportCandidate {
    pub key: String,
    pub module: ModuleId,
    pub label: String,
    #[schema(value_type = serde_json::Value)]
    pub observation: RawJson,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(tag = "identity", rename_all = "snake_case")]
pub enum ImportIdentityDecision {
    Candidate { key: String },
    Unidentified,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ImportReviewDecision {
    pub identity: ImportIdentityDecision,
    pub association: CaptureAssociation,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ReviewImportCommand {
    pub expected_revision: u64,
    pub identity: ImportIdentityDecision,
    pub association: CaptureAssociation,
    pub expected_item_revision: Option<Revision>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CreateImportCommand {
    pub collection_id: CollectionId,
    pub source: ImportSourceSummary,
    pub files: Vec<UploadFile>,
    pub association: CaptureAssociation,
    pub expected_item_revision: Option<Revision>,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ImportView {
    pub id: ImportId,
    pub revision: u64,
    pub collection_id: CollectionId,
    pub source: ImportSourceSummary,
    pub files: Vec<ImportDeclaredFile>,
    pub capture_id: CaptureId,
    pub representation_id: RepresentationId,
    pub association: CaptureAssociation,
    pub expected_item_revision: Option<Revision>,
    pub candidates: Vec<ImportCandidate>,
    pub limitations: Vec<String>,
    pub review: Option<ImportReviewDecision>,
    pub analysis_job_id: Option<JobId>,
    pub phase: ImportPhase,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImportRecord {
    pub user_id: UserId,
    pub view: ImportView,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum IngestEnrichmentPolicy {
    Automatic,
    None,
}

pub use collector_modules::{IngestAnchorChoice, IngestBlocker, IngestBlockerKind};

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct IngestPlanPackage {
    pub position: u16,
    pub source: ImportSourceSummary,
    pub files: Vec<UploadFile>,
    pub default_title: String,
    pub anchors: Vec<IngestAnchorChoice>,
    pub blockers: Vec<IngestBlocker>,
    pub analysis_available: bool,
    pub metadata_available: bool,
    pub artwork_available: bool,
    pub preparation_available: bool,
    pub preparation_reason: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PlanIngestCommand {
    pub collection_id: CollectionId,
    pub packages: Vec<IngestPackageSelection>,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct IngestPackageSelection {
    pub source: ImportSourceSummary,
    pub files: Vec<UploadFile>,
    #[serde(default)]
    pub anchor_file_index: Option<usize>,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct IngestPlan {
    pub collection_id: CollectionId,
    pub token: String,
    pub packages: Vec<IngestPlanPackage>,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct StartIngestCommand {
    pub plan: IngestPlan,
    pub enrichment: IngestEnrichmentPolicy,
    #[serde(default)]
    pub prepare_music: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GuidedIngestIntent {
    pub import_id: ImportId,
    pub user_id: UserId,
    pub plan_token: String,
    pub target_item_id: ItemId,
    pub default_title: String,
    pub anchor_file_index: Option<usize>,
    pub enrichment: IngestEnrichmentPolicy,
    pub enrichment_error: Option<String>,
    pub prepare_music: bool,
    pub preparation_batch_id: Option<JobId>,
    pub preparation_error: Option<String>,
    pub settled: bool,
    pub cancelled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AcceptedIngest {
    pub imports: Vec<IngestSummary>,
    pub rejected: Vec<IngestRejectedPackage>,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct IngestRejectedPackage {
    pub position: u16,
    pub reason: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum IngestStepState {
    Pending,
    Active,
    NeedsInput,
    Succeeded,
    Failed,
    Skipped,
    Cancelled,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct IngestStepView {
    pub step: String,
    pub state: IngestStepState,
    pub job_id: Option<JobId>,
    pub attempt: Option<u32>,
    pub message: Option<String>,
    pub retryable: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct IngestAttemptView {
    pub job_id: JobId,
    pub step: String,
    pub attempt: u32,
    pub state: JobState,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub diagnostic: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct IngestPermissions {
    pub can_cancel: bool,
    pub can_discard: bool,
    pub can_correct_association: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct IngestSummary {
    pub import_id: ImportId,
    pub collection_id: CollectionId,
    pub package_name: String,
    pub file_count: u32,
    pub total_bytes: ByteLength,
    pub target_item_id: Option<ItemId>,
    pub capture_id: CaptureId,
    pub capture_revision: Option<Revision>,
    pub phase: ImportPhase,
    pub outcome: IngestStepState,
    pub enrichment: IngestEnrichmentPolicy,
    pub prepare_music: bool,
    pub updated_at: DateTime<Utc>,
    pub permissions: IngestPermissions,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct IngestDetail {
    pub summary: IngestSummary,
    pub import: ImportView,
    pub steps: Vec<IngestStepView>,
    pub attempts: Vec<IngestAttemptView>,
    pub cleanup: Option<IngestDiscardResult>,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct IngestDiscardResult {
    pub import_id: ImportId,
    pub reclaimed_bytes: ByteLength,
    pub pending_bytes: ByteLength,
    pub cleanup_pending: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CorrectIngestAssociationCommand {
    pub expected_capture_revision: Revision,
    pub association: CaptureAssociation,
    pub expected_item_revision: Option<Revision>,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RetryIngestCommand {
    pub job_id: Option<JobId>,
}

pub trait ProvisionalImportPort: Send + Sync {
    fn create_import(&self, record: ImportRecord) -> Result<ImportRecord>;
    fn import_record(&self, id: ImportId) -> Result<ImportRecord>;
    fn save_import(&self, record: ImportRecord, expected_revision: u64) -> Result<ImportRecord>;
    fn stage_import_chunk(
        &self,
        id: ImportId,
        index: usize,
        offset: u64,
        bytes: &[u8],
    ) -> Result<ImportRecord>;
    fn stage_inbox_import(
        &self,
        id: ImportId,
        job: JobId,
        claim_token: &str,
        source_locator: &str,
    ) -> Result<ImportRecord>;
    /// Recheck application policy after staging, immediately before a new Capture
    /// commits. Do not call the guard when reconciling an existing commit.
    fn publish_import_for_job(
        &self,
        id: ImportId,
        job: JobId,
        claim_token: &str,
        before_publish: &mut dyn FnMut() -> Result<()>,
    ) -> Result<ImportRecord>;
    fn cancel_import(&self, id: ImportId) -> Result<ImportRecord>;
    fn accept_guided_imports(
        &self,
        records: Vec<ImportRecord>,
        intents: Vec<GuidedIngestIntent>,
    ) -> Result<Vec<ImportView>>;
    fn guided_ingest_intent(&self, id: ImportId) -> Result<Option<GuidedIngestIntent>>;
    fn resumable_guided_ingest_ids(
        &self,
        after: Option<ImportId>,
        limit: u16,
    ) -> Result<Vec<ImportId>>;
    fn resumable_guided_inbox_records(
        &self,
        after: Option<ImportId>,
        limit: u16,
    ) -> Result<Vec<ImportRecord>>;
    fn reconcile_guided_cleanup(&self) -> Result<()>;
    fn guided_cleanup_status(&self, id: ImportId) -> Result<Option<IngestDiscardResult>>;
    fn save_guided_ingest_intent(&self, intent: GuidedIngestIntent) -> Result<()>;
    fn guided_import_records(
        &self,
        user: UserId,
        collection: Option<CollectionId>,
        cursor: Option<ImportId>,
        limit: u16,
    ) -> Result<Page<ImportRecord>>;
    fn import_job_records(&self, id: ImportId, user: UserId) -> Result<Vec<JobRecord>>;
    fn discard_guided_import(
        &self,
        id: ImportId,
        expected_revision: u64,
    ) -> Result<IngestDiscardResult>;
    fn cancel_guided_import(&self, id: ImportId, expected_revision: u64) -> Result<ImportRecord>;
}

pub trait ImportPort: Send + Sync {
    fn create_upload(
        &self,
        user: UserId,
        item: ItemId,
        expected: Revision,
        files: Vec<UploadFile>,
    ) -> Result<Upload>;
    fn upload(&self, id: JobId) -> Result<Upload>;
    fn upload_chunk(&self, id: JobId, index: usize, offset: u64, bytes: &[u8]) -> Result<Upload>;
    fn cancel_upload(&self, id: JobId) -> Result<()>;
    fn commit_upload(&self, id: JobId) -> Result<Item>;
    /// Recheck application policy before a new Capture commit, never during
    /// completion of an already committed publication.
    fn commit_upload_for_job(
        &self,
        id: JobId,
        claim_token: &str,
        before_publish: &mut dyn FnMut() -> Result<()>,
    ) -> Result<Item>;
    fn queue_upload(&self, id: JobId) -> Result<JobId>;
    fn queue_inbox_upload(&self, id: JobId, source_locator: &str) -> Result<JobId>;
    fn retry_upload(&self, id: JobId) -> Result<JobId>;
    fn fail_upload(&self, id: JobId, message: &str) -> Result<()>;
    fn verify_integrity(&self, item: ItemId) -> Result<Item>;
    fn verify_integrity_for_job(
        &self,
        item: ItemId,
        job: JobId,
        claim_token: &str,
        created_at: DateTime<Utc>,
        progress: &mut dyn FnMut(u64) -> Result<()>,
    ) -> Result<Item>;
    fn jobs(&self, user: UserId) -> Result<Vec<Job>>;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JobSpec {
    pub id: JobId,
    pub user_id: UserId,
    pub collection_id: Option<CollectionId>,
    pub item_id: Option<ItemId>,
    pub kind: String,
    pub phase: String,
    pub total_units: u64,
    pub request: serde_json::Value,
    pub frozen_input: Option<FrozenJobInput>,
}

#[derive(Clone, Debug)]
pub struct JobClaim {
    pub id: JobId,
    pub user_id: UserId,
    pub collection_id: Option<CollectionId>,
    pub item_id: Option<ItemId>,
    pub kind: String,
    pub request: serde_json::Value,
    pub claim_token: String,
    pub attempt: u32,
    pub phase: String,
    pub completed_units: u64,
    pub created_at: chrono::DateTime<Utc>,
    pub frozen_input: Option<FrozenJobInput>,
}

#[derive(Clone, Debug)]
pub struct JobRecord {
    pub id: JobId,
    pub user_id: UserId,
    pub collection_id: Option<CollectionId>,
    pub item_id: Option<ItemId>,
    pub kind: String,
    pub state: JobState,
    pub phase: String,
    pub completed_units: u64,
    pub total_units: u64,
    pub attempt: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub error: Option<String>,
    pub limitation: Option<String>,
    pub result: Option<serde_json::Value>,
}

#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum JobResultView {
    CatalogSnapshot { snapshot_id: SnapshotId },
}

#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
pub struct JobView {
    pub id: JobId,
    pub collection_id: Option<CollectionId>,
    pub item_id: Option<ItemId>,
    pub kind: String,
    pub state: JobState,
    pub phase: String,
    pub completed_units: u64,
    pub total_units: u64,
    pub attempt: u32,
    pub cancellable: bool,
    pub retryable: bool,
    pub error: Option<String>,
    pub limitation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<JobResultView>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<JobRecord> for JobView {
    fn from(record: JobRecord) -> Self {
        let cancellable = matches!(record.state, JobState::Queued | JobState::Running)
            && record.phase != "revision_committed";
        let retryable = record.state == JobState::Failed
            && matches!(
                record.kind.as_str(),
                "integrity_verification"
                    | "preservation_import"
                    | "import_staging"
                    | "import_analysis"
                    | "import_publication"
                    | "catalog_acquisition"
                    | "processing_operation"
                    | "music_derivative"
                    | "music_derivative_batch"
            );
        let result = safe_job_result(&record.kind, record.result.as_ref());
        Self {
            id: record.id,
            collection_id: record.collection_id,
            item_id: record.item_id,
            kind: record.kind,
            state: record.state,
            phase: record.phase,
            completed_units: record.completed_units.min(record.total_units),
            total_units: record.total_units,
            attempt: record.attempt,
            cancellable,
            retryable,
            error: record.error.map(|value| value.chars().take(512).collect()),
            limitation: record
                .limitation
                .map(|value| value.chars().take(512).collect()),
            result,
            created_at: record.created_at,
            updated_at: record.updated_at,
        }
    }
}

fn safe_job_result(kind: &str, result: Option<&serde_json::Value>) -> Option<JobResultView> {
    if kind != "catalog_acquisition" {
        return None;
    }
    let result = result.and_then(|value| value.get("completion").or(Some(value)))?;
    let snapshot_id = result.get("snapshot_id")?.as_str()?.parse().ok()?;
    Some(JobResultView::CatalogSnapshot { snapshot_id })
}

pub trait JobPort: Send + Sync {
    fn enqueue_job(&self, spec: JobSpec) -> Result<JobId>;
    fn enqueue_preparation_member(
        &self,
        batch: JobId,
        claim_token: &str,
        spec: JobSpec,
    ) -> Result<JobId>;
    fn claim_job(&self, worker_instance: &str, lease_seconds: i64) -> Result<Option<JobClaim>>;
    fn renew_job_lease(&self, id: JobId, claim_token: &str, lease_seconds: i64) -> Result<()>;
    fn heartbeat_job(
        &self,
        id: JobId,
        claim_token: &str,
        phase: &str,
        completed_units: u64,
        lease_seconds: i64,
    ) -> Result<()>;
    fn complete_job(&self, id: JobId, claim_token: &str, result: serde_json::Value) -> Result<()>;
    fn fail_job(&self, id: JobId, claim_token: &str, message: &str) -> Result<()>;
    fn cancel_claimed_job(&self, id: JobId, claim_token: &str) -> Result<()>;
    fn cancellation_requested(&self, id: JobId, claim_token: &str) -> Result<bool>;
    fn request_job_cancellation(&self, id: JobId, user: UserId) -> Result<()>;
    fn job_record(&self, id: JobId, user: UserId) -> Result<JobRecord>;
    fn job_records(
        &self,
        user: UserId,
        cursor: Option<JobId>,
        limit: u16,
    ) -> Result<Page<JobRecord>>;
    fn retry_failed_job(&self, id: JobId, user: UserId) -> Result<JobId>;
    /// Active derivative inputs are private mechanism records. Application code
    /// reduces them to a viewer-safe preparing state after collection access is
    /// authorized; job identity, submitter and diagnostics never cross the DTO.
    fn active_derivative_jobs(&self, capture: CaptureId) -> Result<Vec<ActiveDerivativeJob>>;
    fn frozen_job_input(&self, id: JobId, user: UserId) -> Result<Option<FrozenJobInput>>;
    fn preparation_batch_records(
        &self,
        user: UserId,
        item: ItemId,
        limit: u16,
    ) -> Result<Vec<JobRecord>>;
    fn cancel_preparation_batch(&self, id: JobId, user: UserId) -> Result<()>;
    fn retry_preparation_batch(&self, id: JobId, user: UserId) -> Result<()>;
}

pub trait AdministrationPort: Send + Sync {
    fn create_user(&self, username: &str, password: &str, administrator: bool) -> Result<UserId>;
    fn reindex(&self) -> Result<()>;
    fn accounts(&self) -> Result<Vec<AccountView>>;
    fn account(&self, user: UserId) -> Result<AccountView>;
    fn update_account(
        &self,
        user: UserId,
        expected: Revision,
        display_name: &str,
        enabled: bool,
        administrator: bool,
    ) -> Result<AccountView>;
    fn reassign_default(&self, user: UserId) -> Result<()>;
    fn activate_catalog_snapshot(
        &self,
        collection: CollectionId,
        provider: &ModuleId,
        snapshot: SnapshotId,
    ) -> Result<CatalogSnapshotStatus>;
}

pub trait AuthenticationPort: Send + Sync {
    fn now(&self) -> chrono::DateTime<Utc>;
    fn login(
        &self,
        username: &str,
        password: &str,
        label: &str,
    ) -> Result<(String, AuthenticatedPrincipal)>;
    fn authenticate(&self, token: &str) -> Result<AuthenticatedPrincipal>;
    fn authenticate_native(&self, credential: &str) -> Result<AuthenticatedPrincipal>;
    fn revoke_session(&self, user: UserId, device: DeviceId) -> Result<()>;
    fn sessions(&self, user: UserId, current: Option<DeviceId>) -> Result<Vec<SessionView>>;
    fn replace_recovery_codes(&self, user: UserId) -> Result<Vec<String>>;
    fn consume_recovery_code(
        &self,
        username: &str,
        code: &str,
        new_password: &str,
        label: &str,
    ) -> Result<(String, AuthenticatedPrincipal)>;
    fn username(&self, user: UserId) -> Result<String>;
    fn user_by_name(&self, username: &str) -> Result<UserId>;
    fn passkeys(&self, user: UserId) -> Result<Vec<StoredPasskey>>;
    fn insert_passkey(
        &self,
        user: UserId,
        credential_id: &str,
        credential: serde_json::Value,
    ) -> Result<()>;
    fn complete_passkey_authentication(
        &self,
        user: UserId,
        passkey: PasskeyId,
        expected_revision: Revision,
        expected_record: serde_json::Value,
        credential: serde_json::Value,
        label: &str,
    ) -> Result<(String, AuthenticatedPrincipal)>;
    fn revoke_passkey(&self, user: UserId, passkey: PasskeyId, expected: Revision) -> Result<()>;
    fn issue_session(&self, user: UserId, label: &str) -> Result<(String, AuthenticatedPrincipal)>;
}

pub trait EnrollmentPort: Send + Sync {
    fn start_enrollment(
        &self,
        user: UserId,
        label: &str,
        verification_url: &str,
    ) -> Result<EnrollmentStart>;
    fn approve_enrollment(&self, user: UserId, user_code: &str) -> Result<EnrollmentView>;
    fn cancel_enrollment(&self, user: UserId, enrollment: EnrollmentId) -> Result<()>;
    fn complete_enrollment(
        &self,
        enrollment: EnrollmentId,
        completion_secret: &str,
    ) -> Result<NativeCredentialIssue>;
    fn native_devices(&self, user: UserId) -> Result<Vec<NativeDeviceView>>;
    fn revoke_native_device(
        &self,
        user: UserId,
        credential: NativeCredentialId,
        expected: Revision,
    ) -> Result<()>;
}

#[derive(Clone, Debug)]
pub struct CommandContext {
    key: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CredentialKind {
    BrowserSession,
    NativeCredential,
    TestLocal,
}

#[derive(Clone, Debug)]
pub struct AuthenticatedPrincipal {
    pub principal: Principal,
    pub credential_kind: CredentialKind,
    pub authenticated_at: chrono::DateTime<Utc>,
}

impl AuthenticatedPrincipal {
    pub fn require_recent_at(&self, now: chrono::DateTime<Utc>) -> Result<()> {
        const RECENT_SECONDS: i64 = 5 * 60;
        let age = now
            .signed_duration_since(self.authenticated_at)
            .num_seconds();
        if (0..RECENT_SECONDS).contains(&age) {
            Ok(())
        } else {
            Err(Error::RecentAuthenticationRequired)
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AccountView {
    pub id: UserId,
    pub username: String,
    pub display_name: String,
    pub administrator: bool,
    pub enabled: bool,
    pub revision: Revision,
}

#[derive(Clone, Debug, Serialize)]
pub struct AccountUpdate {
    pub user: UserId,
    pub expected_revision: Revision,
    pub display_name: String,
    pub enabled: bool,
    pub administrator: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct GrantView {
    pub collection_id: CollectionId,
    pub user_id: UserId,
    pub username: String,
    pub role: Role,
    pub revision: Revision,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SessionView {
    pub id: SessionId,
    pub label: String,
    pub created_at: chrono::DateTime<Utc>,
    pub expires_at: chrono::DateTime<Utc>,
    pub current: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PasskeyView {
    pub id: PasskeyId,
    pub label: String,
    pub revision: Revision,
    pub created_at: chrono::DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct StoredPasskey {
    pub view: PasskeyView,
    pub credential_id: String,
    pub record: serde_json::Value,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum EnrollmentPhase {
    PendingApproval,
    Approved,
    Issued,
    Cancelled,
    Expired,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct EnrollmentStart {
    pub id: EnrollmentId,
    pub completion_secret: String,
    pub user_code: String,
    pub verification_url: String,
    pub expires_at: chrono::DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct EnrollmentView {
    pub id: EnrollmentId,
    pub requested_label: String,
    pub phase: EnrollmentPhase,
    pub expires_at: chrono::DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct NativeCredentialIssue {
    pub device_id: NativeCredentialId,
    pub credential: String,
    pub expires_at: chrono::DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct NativeDeviceView {
    pub id: NativeCredentialId,
    pub label: String,
    pub revision: Revision,
    pub created_at: chrono::DateTime<Utc>,
    pub expires_at: chrono::DateTime<Utc>,
}
impl CommandContext {
    pub fn new(key: impl Into<String>) -> Result<Self> {
        let key = key.into();
        if key.is_empty() || key.len() > 128 || !key.bytes().all(|byte| byte.is_ascii_graphic()) {
            return Err(Error::Invalid(
                "Idempotency-Key must be 1–128 visible ASCII characters".into(),
            ));
        }
        Ok(Self { key })
    }
    pub fn key(&self) -> &str {
        &self.key
    }
}

pub enum ReceiptDecision {
    Execute,
    Replay(serde_json::Value),
}

/// A per-command mechanism scope. Every durable effect must commit its receipt
/// atomically, or prepare an outcome tied to an immutable archive commit point.
/// No ambient thread-local state or authorization decisions live in this scope.
pub trait CommandEffects:
    CollectionWritePort
    + ContentPort
    + ImportPort
    + AdministrationPort
    + AuthenticationPort
    + EnrollmentPort
    + AccessPort
    + JobPort
    + ProvisionalImportPort
    + ProcessingPort
{
}
impl<T> CommandEffects for T where
    T: CollectionWritePort
        + ContentPort
        + ImportPort
        + AdministrationPort
        + AuthenticationPort
        + EnrollmentPort
        + AccessPort
        + JobPort
        + ProvisionalImportPort
        + ProcessingPort
{
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommandReceipt {
    pub user: UserId,
    pub api_major: u16,
    pub operation: String,
    pub key: String,
    pub request_sha256: String,
}

pub trait IdempotencyPort: Send + Sync {
    fn command_effects(&self, receipt: CommandReceipt) -> Box<dyn CommandEffects>;

    fn begin_command(
        &self,
        user: UserId,
        api_major: u16,
        operation: &str,
        key: &str,
        request_sha256: &str,
    ) -> Result<ReceiptDecision>;
    fn complete_command(
        &self,
        user: UserId,
        api_major: u16,
        operation: &str,
        key: &str,
        request_sha256: &str,
        result: serde_json::Value,
    ) -> Result<()>;
    fn abandon_command(
        &self,
        user: UserId,
        api_major: u16,
        operation: &str,
        key: &str,
        request_sha256: &str,
    ) -> Result<()>;
}

impl<T: Catalog + ?Sized> CollectionWritePort for T {
    fn create_collection(&self, user: UserId, collection: Collection) -> Result<Collection> {
        Catalog::create_collection(self, user, collection)
    }
    fn save(&self, item: Item, expected: Revision) -> Result<Item> {
        Catalog::save(self, item, expected)
    }
    fn save_work(&self, work: Work, expected: Revision) -> Result<Work> {
        Catalog::save_work(self, work, expected)
    }
    fn save_edition(&self, edition: Edition, expected: Revision) -> Result<Edition> {
        Catalog::save_edition(self, edition, expected)
    }
    fn save_capture(&self, capture: Capture, expected: Revision) -> Result<Capture> {
        Catalog::save_capture(self, capture, expected)
    }
}

#[derive(Clone)]
pub struct CollectionService {
    reader: Arc<dyn CollectionReadPort>,
    writer: Arc<dyn CollectionWritePort>,
    guided_transitions: Arc<std::sync::Mutex<()>>,
    access: Arc<dyn AccessPort>,
    content: Arc<dyn ContentPort>,
    imports: Arc<dyn ImportPort>,
    provisional_imports: Arc<dyn ProvisionalImportPort>,
    administration: Arc<dyn AdministrationPort>,
    authentication: Arc<dyn AuthenticationPort>,
    enrollments: Arc<dyn EnrollmentPort>,
    idempotency: Arc<dyn IdempotencyPort>,
    jobs: Arc<dyn JobPort>,
    processing: Arc<dyn ProcessingPort>,
    audio_converter: Option<Arc<dyn AudioConversionPort>>,
    streaming_providers: Arc<std::collections::BTreeMap<String, Arc<dyn StreamingProviderPort>>>,
    catalog_parsers: Arc<std::collections::BTreeMap<String, Arc<dyn CatalogParserPort>>>,
    modules: Arc<Registry>,
}

#[derive(Clone, Debug)]
struct ContentWriteEligibility {
    binding: CollectionBinding,
    owner: ModuleId,
    schema_version: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ImportAnalysisCandidateOutput {
    key: String,
    label: String,
    observation: RawJson,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ImportAnalysisOutput {
    #[serde(default)]
    candidates: Vec<ImportAnalysisCandidateOutput>,
    #[serde(default)]
    limitations: Vec<String>,
    #[serde(default, rename = "observations")]
    _observations: Vec<RawJson>,
    #[serde(default, rename = "selection")]
    _selection: Option<RawJson>,
}

struct ProcessingHost<'a> {
    port: &'a dyn ProcessingPort,
    access: PackageAccess,
    source_fingerprint: &'a SourceFingerprint,
    frozen_snapshot_id: Option<SnapshotId>,
    catalog_parsers: &'a std::collections::BTreeMap<String, Arc<dyn CatalogParserPort>>,
    jobs: Arc<dyn JobPort>,
    job_id: JobId,
    claim_token: String,
}

struct DurableProcessingContext {
    frozen: FrozenJobInput,
    jobs: Arc<dyn JobPort>,
    id: JobId,
    claim_token: String,
}

struct JobHeartbeat {
    stop: Option<std::sync::mpsc::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl JobHeartbeat {
    fn stop(mut self) -> Result<()> {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let Some(thread) = self.thread.take() {
            thread
                .join()
                .map_err(|_| Error::Unavailable("job heartbeat worker panicked".into()))?;
        }
        Ok(())
    }
}

impl Drop for JobHeartbeat {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl ProcessingExecutionContext for DurableProcessingContext {
    fn frozen_job(&self) -> &FrozenJobInput {
        &self.frozen
    }

    fn report_progress(
        &mut self,
        phase: &str,
        completed_units: u64,
        total_units: u64,
    ) -> Result<()> {
        if total_units == 0 || completed_units > total_units {
            return Err(Error::Invalid("processing progress is invalid".into()));
        }
        self.jobs
            .heartbeat_job(self.id, &self.claim_token, phase, completed_units, 30)
    }

    fn cancellation_requested(&self) -> Result<bool> {
        self.jobs.cancellation_requested(self.id, &self.claim_token)
    }
}

fn processing_module_error(error: Error) -> ModuleError {
    match error {
        Error::Forbidden => ModuleError::AccessDenied,
        Error::NotFound => ModuleError::Invalid("package source is missing".into()),
        error => ModuleError::FailedOperation(error.to_string()),
    }
}

struct ProcessingReader {
    reader: Box<dyn RepresentationReader>,
    jobs: Arc<dyn JobPort>,
    job_id: JobId,
    claim_token: String,
}

impl std::io::Read for ProcessingReader {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        if self
            .jobs
            .cancellation_requested(self.job_id, &self.claim_token)
            .map_err(std::io::Error::other)?
        {
            return Err(std::io::Error::other("processing cancelled"));
        }
        self.reader.read(buffer)
    }
}

impl std::io::Seek for ProcessingReader {
    fn seek(&mut self, position: std::io::SeekFrom) -> std::io::Result<u64> {
        self.reader.seek(position)
    }
}

impl ModuleHost for ProcessingHost<'_> {
    fn checkpoint(&self) -> ModuleResult<()> {
        if self
            .jobs
            .cancellation_requested(self.job_id, &self.claim_token)
            .map_err(processing_module_error)?
        {
            return Err(ModuleError::FailedOperation("processing cancelled".into()));
        }
        Ok(())
    }

    fn package_inventory(&self, access: PackageAccess) -> ModuleResult<PackageInventory> {
        if access != self.access {
            return Err(ModuleError::AccessDenied);
        }
        self.port
            .package_inventory(access.collection_id, access.scope)
            .map_err(processing_module_error)
    }

    fn package_file(
        &self,
        access: PackageAccess,
        id: PackageFileId,
    ) -> ModuleResult<Box<dyn RepresentationReader>> {
        if access != self.access {
            return Err(ModuleError::AccessDenied);
        }
        self.port
            .package_file(access.collection_id, access.scope, id)
            .map(|reader| {
                Box::new(ProcessingReader {
                    reader: Box::new(reader),
                    jobs: self.jobs.clone(),
                    job_id: self.job_id,
                    claim_token: self.claim_token.clone(),
                }) as Box<dyn RepresentationReader>
            })
            .map_err(processing_module_error)
    }

    fn representation_access(&self, id: RepresentationId) -> ModuleResult<PackageAccess> {
        if self.access.scope == PackageScope::Representation(id) {
            Ok(self.access)
        } else {
            Err(ModuleError::AccessDenied)
        }
    }

    fn source_fingerprint(&self, access: PackageAccess) -> ModuleResult<SourceFingerprint> {
        if access != self.access {
            return Err(ModuleError::AccessDenied);
        }
        Ok(self.source_fingerprint.clone())
    }

    fn catalog_snapshot(&self, id: SnapshotId) -> ModuleResult<CatalogSnapshotInput> {
        if self.frozen_snapshot_id != Some(id) {
            return Err(ModuleError::Invalid(
                "operation requested a catalog snapshot that was not frozen with its job".into(),
            ));
        }
        let query = SnapshotQuery {
            collection_id: self.access.collection_id,
            snapshot_id: id,
        };
        let record = self.port.snapshot(query).map_err(processing_module_error)?;
        let parser = self
            .catalog_parsers
            .get(&record.provider.0)
            .ok_or_else(|| {
                ModuleError::UnavailableCapability(format!(
                    "catalog parser {} is unavailable",
                    record.provider.0
                ))
            })?;
        let mut reader = self
            .port
            .snapshot_bytes(query)
            .map_err(processing_module_error)?;
        let jobs = self.jobs.clone();
        let job_id = self.job_id;
        let claim_token = self.claim_token.clone();
        let parsed = parser
            .parse_snapshot(&record, reader.as_mut(), &mut || {
                jobs.cancellation_requested(job_id, &claim_token)
            })
            .map_err(processing_module_error)?;
        parsed.validate().map_err(processing_module_error)?;
        Ok(CatalogSnapshotInput {
            id,
            provider: record.provider,
            digest: record.digest,
            observation: parsed.observation,
            entries: parsed.entries,
        })
    }

    fn submit_job(&self, _request: JobSubmission) -> ModuleResult<JobId> {
        Err(ModuleError::Invalid(
            "import analysis cannot recursively submit jobs".into(),
        ))
    }
}

#[derive(Clone, Copy, Debug, Serialize, utoipa::ToSchema)]
pub struct EffectivePermissions {
    pub role: Role,
    pub can_browse: bool,
    pub can_curate: bool,
    pub can_manage_access: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ContentAvailabilityStatus {
    Available,
    LegacyUnassigned,
    OwnerUnavailable,
    ItemSchemaUnsupported,
}

#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
pub struct ContentPermissions {
    pub can_edit: bool,
    pub status: ContentAvailabilityStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub administrator_commands: Vec<String>,
}
impl From<Role> for EffectivePermissions {
    fn from(role: Role) -> Self {
        Self {
            role,
            can_browse: role.allows(Action::Browse),
            can_curate: role.allows(Action::Curate),
            can_manage_access: role.allows(Action::ManageAccess),
        }
    }
}

#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
pub struct PresentedCollection {
    pub id: CollectionId,
    pub revision: Revision,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub binding: CollectionBinding,
    pub effective_permissions: EffectivePermissions,
    pub content_permissions: ContentPermissions,
}

/// Typed creation input reserved for CT.2 runtime integration. Keeping the
/// module mandatory here prevents `LegacyUnassigned` from becoming a valid new
/// collection request while CT.1 leaves persisted Collection unchanged.
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CreateCollectionCommand {
    pub name: String,
    pub module: ModuleId,
}

impl CreateCollectionCommand {
    pub fn validate(&self) -> Result<()> {
        if self.name.trim().is_empty() || self.name.len() > 256 {
            return Err(Error::Invalid("collection name must be 1–256 bytes".into()));
        }
        self.module.validate()
    }
}

#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
pub struct PresentedModuleData {
    pub module: ModuleId,
    pub schema_version: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<serde_json::Value>)]
    pub payload: Option<RawJson>,
}

#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
pub struct PresentedSpecimen {
    pub id: SpecimenId,
    pub label: String,
    pub condition: Condition,
    pub ownership: Ownership,
    pub possession: Possession,
    pub components: Vec<Component>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acquisition: Option<Acquisition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub private_notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loans: Option<Vec<Loan>>,
}

impl PresentedSpecimen {
    fn from_stored(specimen: Specimen, private: bool) -> Self {
        Self {
            id: specimen.id,
            label: specimen.label,
            condition: specimen.condition,
            ownership: specimen.ownership,
            possession: specimen.possession,
            components: specimen.components,
            acquisition: private.then_some(specimen.acquisition).flatten(),
            private_notes: private.then_some(specimen.private_notes).flatten(),
            loans: private.then_some(specimen.loans),
        }
    }
}

#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
pub struct PresentedCapture {
    pub id: CaptureId,
    pub collection_id: CollectionId,
    pub revision: Revision,
    pub association: CaptureAssociation,
    pub created_at: DateTime<Utc>,
    pub representations: Vec<Representation>,
}

impl From<Capture> for PresentedCapture {
    fn from(capture: Capture) -> Self {
        Self {
            id: capture.id,
            collection_id: capture.collection_id,
            revision: capture.revision,
            association: capture.association,
            created_at: capture.created_at,
            representations: capture.representations,
        }
    }
}

/// Explicit response shape: unrestricted stored Item records are never
/// flattened into a transport response.
#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
pub struct PresentedItem {
    pub id: ItemId,
    pub collection_id: CollectionId,
    pub revision: Revision,
    pub title: String,
    pub subtitle: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inventory_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inventory_subtitle: Option<String>,
    pub identity_link: ItemIdentityLink,
    pub domain: PresentedModuleData,
    pub identity: IdentityState,
    pub translations: Vec<LocalizedText>,
    pub identifiers: Vec<ProviderIdentity>,
    pub specimens: Vec<PresentedSpecimen>,
    pub wishlist: bool,
    pub captures: Vec<PresentedCapture>,
    pub assets: Vec<PresentedAsset>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_artwork: Option<AssetId>,
    pub effective_metadata: ResolvedMetadataDisplay,
    pub deleted: bool,
    pub updated_at: DateTime<Utc>,
    pub presentation: ModulePresentation,
    pub effective_permissions: EffectivePermissions,
    pub content_permissions: ContentPermissions,
}

#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
pub struct PresentedEvidence {
    pub id: EvidenceId,
    pub item_id: ItemId,
    pub representation_id: RepresentationId,
    pub kind: EvidenceKind,
    pub outcome: EvidenceOutcome,
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inputs: Option<Vec<Digest>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub implementation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub implementation_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub catalog: Option<CatalogSnapshot>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    pub provenance_limitation: Option<String>,
}

/// Editable inventory fields. Revision, timestamps, deletion state, metadata
/// observations, Evidence, Assets, and Capture representations are server-owned.
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SaveItemCommand {
    pub collection_id: CollectionId,
    pub title: String,
    pub subtitle: String,
    pub identity_link: ItemIdentityLink,
    pub domain: ModuleData,
    pub identity: IdentityState,
    pub translations: Vec<LocalizedText>,
    pub identifiers: Vec<ProviderIdentity>,
    pub wishlist: bool,
}

/// Item form input whose domain changes are applied by the owning module. The
/// client can edit declared scalar fields without replacing preserved raw data.
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SaveItemEditCommand {
    pub collection_id: CollectionId,
    pub title: String,
    pub subtitle: String,
    pub identity_link: ItemIdentityLink,
    pub domain_edit: DomainEdit,
    pub identity: IdentityState,
    pub translations: Vec<LocalizedText>,
    pub identifiers: Vec<ProviderIdentity>,
    pub wishlist: bool,
}

impl From<Item> for SaveItemCommand {
    fn from(item: Item) -> Self {
        Self {
            collection_id: item.collection_id,
            title: item.title,
            subtitle: item.subtitle,
            identity_link: item.identity_link,
            domain: item.domain,
            identity: item.identity,
            translations: item.translations,
            identifiers: item.identifiers,
            wishlist: item.wishlist,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ActiveOwnership {
    Owned,
    Borrowed,
    Unknown,
}

impl From<ActiveOwnership> for Ownership {
    fn from(value: ActiveOwnership) -> Self {
        match value {
            ActiveOwnership::Owned => Self::Owned,
            ActiveOwnership::Borrowed => Self::Borrowed,
            ActiveOwnership::Unknown => Self::Unknown,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReturnedPossession {
    Present,
    Elsewhere,
    Unknown,
}

impl From<ReturnedPossession> for Possession {
    fn from(value: ReturnedPossession) -> Self {
        match value {
            ReturnedPossession::Present => Self::Present,
            ReturnedPossession::Elsewhere => Self::Elsewhere,
            ReturnedPossession::Unknown => Self::Unknown,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SpecimenDetails {
    pub label: String,
    pub condition: Condition,
    pub ownership: ActiveOwnership,
    pub possession: Possession,
    pub acquisition: Option<Acquisition>,
    pub private_notes: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CarrierDetails {
    pub label: String,
    pub kind: CarrierKind,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ComponentDetails {
    pub label: String,
    pub kind: ComponentKind,
    pub position: u16,
    pub carrier: Option<CarrierDetails>,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct StartLoanCommand {
    pub contact: String,
    pub lent_at: chrono::NaiveDate,
    pub due_at: Option<chrono::NaiveDate>,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ReturnLoanCommand {
    pub returned_at: chrono::NaiveDate,
    pub possession: ReturnedPossession,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct DisposeSpecimenCommand {
    pub date: chrono::NaiveDate,
    pub reason: String,
    pub possession: ReturnedPossession,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateAssetCommand {
    pub collection_id: CollectionId,
    pub subject: AssetSubject,
    pub kind: AssetKind,
    pub visibility: Visibility,
    pub alt_text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct UpdateAssetCommand {
    pub visibility: Visibility,
    pub alt_text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SaveWorkCommand {
    pub collection_id: CollectionId,
    pub provisional: bool,
    pub title: String,
    pub translations: Vec<LocalizedText>,
    pub identifiers: Vec<ProviderIdentity>,
}
impl From<Work> for SaveWorkCommand {
    fn from(work: Work) -> Self {
        Self {
            collection_id: work.collection_id,
            provisional: work.provisional,
            title: work.title,
            translations: work.translations,
            identifiers: work.identifiers,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SaveEditionCommand {
    pub collection_id: CollectionId,
    pub work_id: Option<WorkId>,
    pub provisional: bool,
    pub title: String,
    pub expected_components: Vec<ExpectedComponent>,
    pub identifiers: Vec<ProviderIdentity>,
}
impl From<Edition> for SaveEditionCommand {
    fn from(edition: Edition) -> Self {
        Self {
            collection_id: edition.collection_id,
            work_id: edition.work_id,
            provisional: edition.provisional,
            title: edition.title,
            expected_components: edition.expected_components,
            identifiers: edition.identifiers,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SaveCaptureCommand {
    pub collection_id: CollectionId,
    pub association: CaptureAssociation,
}
impl From<Capture> for SaveCaptureCommand {
    fn from(capture: Capture) -> Self {
        Self {
            collection_id: capture.collection_id,
            association: capture.association,
        }
    }
}

impl CollectionService {
    pub fn new<C>(catalog: Arc<C>, modules: Arc<Registry>) -> Self
    where
        C: CollectionReadPort
            + CollectionWritePort
            + AccessPort
            + ContentPort
            + ImportPort
            + AdministrationPort
            + AuthenticationPort
            + EnrollmentPort
            + IdempotencyPort
            + JobPort
            + ProvisionalImportPort
            + ProcessingPort
            + 'static,
    {
        let reader: Arc<dyn CollectionReadPort> = catalog.clone();
        let writer: Arc<dyn CollectionWritePort> = catalog.clone();
        let content: Arc<dyn ContentPort> = catalog.clone();
        let imports: Arc<dyn ImportPort> = catalog.clone();
        let provisional_imports: Arc<dyn ProvisionalImportPort> = catalog.clone();
        let administration: Arc<dyn AdministrationPort> = catalog.clone();
        let authentication: Arc<dyn AuthenticationPort> = catalog.clone();
        let enrollments: Arc<dyn EnrollmentPort> = catalog.clone();
        let idempotency: Arc<dyn IdempotencyPort> = catalog.clone();
        let jobs: Arc<dyn JobPort> = catalog.clone();
        let access: Arc<dyn AccessPort> = catalog.clone();
        let processing: Arc<dyn ProcessingPort> = catalog.clone();
        Self {
            reader,
            writer,
            guided_transitions: Arc::new(std::sync::Mutex::new(())),
            access,
            content,
            imports,
            provisional_imports,
            administration,
            authentication,
            enrollments,
            idempotency,
            jobs,
            processing,
            audio_converter: None,
            streaming_providers: Arc::new(Default::default()),
            catalog_parsers: Arc::new(Default::default()),
            modules,
        }
    }
    pub fn with_streaming_provider(
        mut self,
        id: ModuleId,
        provider: Arc<dyn StreamingProviderPort>,
    ) -> Result<Self> {
        id.validate()?;
        let mut providers = (*self.streaming_providers).clone();
        if providers.insert(id.0, provider).is_some() {
            return Err(Error::Invalid("duplicate streaming provider".into()));
        }
        self.streaming_providers = Arc::new(providers);
        Ok(self)
    }

    pub fn with_catalog_parser(
        mut self,
        id: ModuleId,
        parser: Arc<dyn CatalogParserPort>,
    ) -> Result<Self> {
        id.validate()?;
        let mut parsers = (*self.catalog_parsers).clone();
        if parsers.insert(id.0, parser).is_some() {
            return Err(Error::Invalid("duplicate catalog parser".into()));
        }
        self.catalog_parsers = Arc::new(parsers);
        Ok(self)
    }
    fn execute_command<Request, Response>(
        &self,
        actor: &Principal,
        context: &CommandContext,
        operation: &str,
        request: &Request,
        effect: impl FnOnce(&dyn CommandEffects) -> Result<Response>,
    ) -> Result<Response>
    where
        Request: Serialize,
        Response: Serialize + DeserializeOwned,
    {
        let request = serde_json::to_vec(&(operation, request))
            .map_err(|error| Error::Invalid(error.to_string()))?;
        let request_sha256 = Digest::sha256(&request).hex;
        match self.idempotency.begin_command(
            actor.user_id,
            1,
            operation,
            context.key(),
            &request_sha256,
        )? {
            ReceiptDecision::Replay(value) => serde_json::from_value(value)
                .map_err(|error| Error::Storage(format!("invalid command receipt: {error}"))),
            ReceiptDecision::Execute => match effect(
                self.idempotency
                    .command_effects(CommandReceipt {
                        user: actor.user_id,
                        api_major: 1,
                        operation: operation.into(),
                        key: context.key().into(),
                        request_sha256: request_sha256.clone(),
                    })
                    .as_ref(),
            ) {
                Ok(result) => {
                    let value = serde_json::to_value(&result)
                        .map_err(|error| Error::Storage(error.to_string()))?;
                    self.idempotency.complete_command(
                        actor.user_id,
                        1,
                        operation,
                        context.key(),
                        &request_sha256,
                        value,
                    )?;
                    Ok(result)
                }
                Err(error) => {
                    let _ = self.idempotency.abandon_command(
                        actor.user_id,
                        1,
                        operation,
                        context.key(),
                        &request_sha256,
                    );
                    Err(error)
                }
            },
        }
    }
    pub fn collections(&self, actor: &Principal) -> Result<Vec<PresentedCollection>> {
        self.reader
            .collections(actor.user_id)?
            .into_iter()
            .map(|collection| {
                let role = self.reader.role(actor.user_id, collection.id)?;
                let (effective_permissions, content_permissions) =
                    self.presented_content_permissions(role, &collection, None);
                Ok(PresentedCollection {
                    id: collection.id,
                    revision: collection.revision,
                    name: collection.name,
                    created_at: collection.created_at,
                    binding: collection.binding,
                    effective_permissions,
                    content_permissions,
                })
            })
            .collect()
    }

    pub fn create_collection(
        &self,
        authenticated: &AuthenticatedPrincipal,
        command: CreateCollectionCommand,
        context: &CommandContext,
    ) -> Result<Collection> {
        authenticated.require_recent_at(self.authentication.now())?;
        let actor = &authenticated.principal;
        if !actor.administrator {
            return Err(Error::Forbidden);
        }
        command.validate()?;
        if !self.modules.has_module(&command.module) {
            return Err(Error::Unavailable(format!(
                "collection module {} is not registered",
                command.module.0
            )));
        }
        let name = command.name.trim().to_owned();
        let collection = Collection {
            id: deterministic_collection_id(actor.user_id, context.key()),
            revision: Revision(1),
            name: name.clone(),
            created_at: self.authentication.now(),
            binding: CollectionBinding::Assigned {
                module: command.module.clone(),
            },
        };
        let command = CreateCollectionCommand {
            name,
            module: command.module,
        };
        self.execute_command(actor, context, "collection.create", &command, |effects| {
            effects.create_collection(actor.user_id, collection.clone())
        })
    }
    pub fn authorize(
        &self,
        actor: &Principal,
        collection: CollectionId,
        action: Action,
    ) -> Result<Role> {
        let role = self.reader.role(actor.user_id, collection)?;
        if !role.allows(action) {
            return Err(Error::Forbidden);
        }
        Ok(role)
    }
    /// Presentation is calculated only after role-based entity redaction. It is a
    /// read model; module commands remain behind A04.
    pub fn browse_presented(
        &self,
        actor: &Principal,
        query: BrowseQuery,
    ) -> Result<Page<PresentedItem>> {
        if let Some(collection) = query.collection_id {
            self.authorize(actor, collection, Action::Browse)?;
        }
        let page = self.reader.browse(actor.user_id, query)?;
        let mut items = Vec::with_capacity(page.items.len());
        for mut item in page.items {
            let role = self.authorize(actor, item.collection_id, Action::Browse)?;
            let effective_metadata = self.effective_metadata_for_item(&item)?;
            item.redact_for(role);
            items.push(self.presented(item, role, effective_metadata)?);
        }
        Ok(Page {
            items,
            next_cursor: page.next_cursor,
        })
    }
    pub fn item_presented(&self, actor: &Principal, id: ItemId) -> Result<PresentedItem> {
        let mut item = self.reader.item(id)?;
        let role = self.authorize(actor, item.collection_id, Action::Browse)?;
        let effective_metadata = self.effective_metadata_for_item(&item)?;
        item.redact_for(role);
        self.presented(item, role, effective_metadata)
    }
    fn presented(
        &self,
        item: Item,
        role: Role,
        mut effective_metadata: ResolvedMetadataDisplay,
    ) -> Result<PresentedItem> {
        let collection = self.reader.collection(item.collection_id)?;
        let (effective_permissions, content_permissions) =
            self.presented_content_permissions(role, &collection, Some(&item.domain));
        let mut presentation = self.modules.presentation(&item.domain);
        let mut asset_states = self
            .content
            .asset_states_for_item(item.id)?
            .into_iter()
            .filter(|state| {
                !state.deleted
                    && (role != Role::Viewer || state.asset.visibility == Visibility::Shared)
            })
            .collect::<Vec<_>>();
        presentation.artwork.retain(|reference| {
            asset_states
                .iter()
                .any(|state| state.asset.id == reference.asset_id)
        });
        let module_artwork = presentation.artwork.first().map(|value| value.asset_id);
        let mut fallback = asset_states
            .iter()
            .filter(|state| {
                state.asset.kind == AssetKind::Cover
                    && matches!(
                        state.asset.subject,
                        AssetSubject::Item { .. }
                            | AssetSubject::Edition { .. }
                            | AssetSubject::Work { .. }
                    )
            })
            .collect::<Vec<_>>();
        fallback.sort_by(|left, right| {
            right
                .updated_at
                .cmp(&left.updated_at)
                .then_with(|| left.asset.id.cmp(&right.asset.id))
        });
        let primary_artwork =
            module_artwork.or_else(|| fallback.first().map(|state| state.asset.id));
        if role == Role::Viewer && !presentation.available {
            presentation.values.clear();
        }
        let private = role != Role::Viewer;
        asset_states.sort_by_key(|state| state.asset.id);
        effective_metadata.artwork = effective_metadata
            .artwork
            .filter(|asset_id| asset_states.iter().any(|state| state.asset.id == *asset_id));
        let primary_artwork = effective_metadata.artwork.or(primary_artwork);
        let inventory_title = item.title;
        let inventory_subtitle = item.subtitle;
        Ok(PresentedItem {
            id: item.id,
            collection_id: item.collection_id,
            revision: item.revision,
            title: effective_metadata.title.value.clone(),
            subtitle: effective_metadata
                .subtitle
                .as_ref()
                .map(|value| value.value.clone())
                .unwrap_or_default(),
            inventory_title: private.then_some(inventory_title),
            inventory_subtitle: private.then_some(inventory_subtitle),
            identity_link: item.identity_link,
            domain: PresentedModuleData {
                module: item.domain.module,
                schema_version: item.domain.schema_version,
                payload: private.then_some(item.domain.payload),
            },
            identity: item.identity,
            translations: item.translations,
            identifiers: item.identifiers,
            specimens: item
                .specimens
                .into_iter()
                .map(|specimen| PresentedSpecimen::from_stored(specimen, private))
                .collect(),
            wishlist: item.wishlist,
            captures: item.captures.into_iter().map(Into::into).collect(),
            assets: asset_states
                .into_iter()
                .map(|state| PresentedAsset::from_state(state, role))
                .collect(),
            primary_artwork,
            effective_metadata,
            deleted: item.deleted,
            updated_at: item.updated_at,
            presentation,
            effective_permissions,
            content_permissions,
        })
    }

    pub fn save_item_edit(
        &self,
        actor: &Principal,
        id: ItemId,
        command: SaveItemEditCommand,
        expected: Revision,
        context: &CommandContext,
    ) -> Result<Item> {
        let request = (id, expected, command.clone());
        let eligibility = self.content_write_eligibility(actor, command.collection_id)?;
        if command.domain_edit.module != eligibility.owner {
            return Err(Error::Invalid(format!(
                "domain edit owner {} conflicts with collection owner {}",
                command.domain_edit.module.0, eligibility.owner.0
            )));
        }
        let current_domain = match self.reader.item(id) {
            Ok(previous) => {
                if previous.collection_id != command.collection_id {
                    return Err(Error::Invalid(
                        "moving items across collections is not supported".into(),
                    ));
                }
                self.validate_owned_domain(&eligibility, &previous.domain)?;
                previous.domain
            }
            Err(Error::NotFound) if expected == Revision(0) => ModuleData {
                module: eligibility.owner.clone(),
                schema_version: eligibility.schema_version,
                payload: RawJson::from_json("{}")?,
            },
            Err(error) => return Err(error),
        };
        let domain = self.modules.edit(&current_domain, command.domain_edit)?;
        self.save_item_with_receipt(
            actor,
            id,
            SaveItemCommand {
                collection_id: command.collection_id,
                title: command.title,
                subtitle: command.subtitle,
                identity_link: command.identity_link,
                domain,
                identity: command.identity,
                translations: command.translations,
                identifiers: command.identifiers,
                wishlist: command.wishlist,
            },
            expected,
            context,
            ("item.edit", &request),
        )
    }
    pub fn save_item(
        &self,
        actor: &Principal,
        id: ItemId,
        command: SaveItemCommand,
        expected: Revision,
        context: &CommandContext,
    ) -> Result<Item> {
        let request = (id, expected, command.clone());
        self.save_item_with_receipt(
            actor,
            id,
            command,
            expected,
            context,
            ("item.save", &request),
        )
    }

    fn save_item_with_receipt<Request: Serialize>(
        &self,
        actor: &Principal,
        id: ItemId,
        command: SaveItemCommand,
        expected: Revision,
        context: &CommandContext,
        receipt: (&str, &Request),
    ) -> Result<Item> {
        let eligibility = self.content_write_eligibility(actor, command.collection_id)?;
        self.validate_owned_domain(&eligibility, &command.domain)?;
        let (specimens, captures, assets, metadata, deleted, revision, updated_at) =
            match self.reader.item(id) {
                Ok(previous) => {
                    if previous.collection_id != command.collection_id {
                        return Err(Error::Invalid(
                            "moving items across collections is not supported".into(),
                        ));
                    }
                    self.validate_owned_domain(&eligibility, &previous.domain)?;
                    (
                        previous.specimens,
                        previous.captures,
                        previous.assets,
                        previous.metadata,
                        previous.deleted,
                        previous.revision,
                        previous.updated_at,
                    )
                }
                Err(Error::NotFound) => (
                    vec![],
                    vec![],
                    vec![],
                    MetadataState::default(),
                    false,
                    Revision(0),
                    Utc::now(),
                ),
                Err(error) => return Err(error),
            };
        let item = Item {
            id,
            collection_id: command.collection_id,
            revision,
            title: command.title,
            subtitle: command.subtitle,
            identity_link: command.identity_link,
            domain: command.domain,
            identity: command.identity,
            translations: command.translations,
            identifiers: command.identifiers,
            specimens,
            wishlist: command.wishlist,
            captures,
            assets,
            metadata,
            deleted,
            updated_at,
        };
        item.validate()?;
        self.execute_command(actor, context, receipt.0, receipt.1, |effects| {
            effects.save(item, expected)
        })
    }

    fn update_inventory<Request: Serialize>(
        &self,
        actor: &Principal,
        target: (ItemId, Revision),
        context: &CommandContext,
        operation: &str,
        request: &Request,
        change: impl FnOnce(&mut Item) -> Result<()>,
    ) -> Result<Item> {
        let (id, expected) = target;
        let mut item = self.reader.item(id)?;
        let eligibility = self.content_write_eligibility(actor, item.collection_id)?;
        self.validate_owned_domain(&eligibility, &item.domain)?;
        self.execute_command(actor, context, operation, request, move |effects| {
            change(&mut item)?;
            item.validate()?;
            effects.save(item, expected)
        })
    }

    pub fn create_specimen(
        &self,
        actor: &Principal,
        id: ItemId,
        expected: Revision,
        details: SpecimenDetails,
        context: &CommandContext,
    ) -> Result<Item> {
        validate_specimen_details(&details)?;
        if details.possession == Possession::Lent {
            return Err(Error::Invalid(
                "a new specimen cannot be lent without starting a loan".into(),
            ));
        }
        let specimen_id = deterministic_specimen_id(actor.user_id, context.key());
        let request = (id, expected, specimen_id, details.clone());
        self.update_inventory(
            actor,
            (id, expected),
            context,
            "specimen.create",
            &request,
            move |item| {
                item.specimens.push(Specimen {
                    id: specimen_id,
                    label: details.label,
                    condition: details.condition,
                    ownership: details.ownership.into(),
                    possession: details.possession,
                    components: vec![],
                    acquisition: details.acquisition,
                    private_notes: details.private_notes,
                    loans: vec![],
                });
                Ok(())
            },
        )
    }

    pub fn update_specimen(
        &self,
        actor: &Principal,
        id: ItemId,
        specimen_id: SpecimenId,
        expected: Revision,
        details: SpecimenDetails,
        context: &CommandContext,
    ) -> Result<Item> {
        validate_specimen_details(&details)?;
        let request = (id, specimen_id, expected, details.clone());
        self.update_inventory(
            actor,
            (id, expected),
            context,
            "specimen.update",
            &request,
            move |item| {
                let specimen = specimen_mut(item, specimen_id)?;
                if matches!(specimen.ownership, Ownership::Disposed { .. }) {
                    return Err(Error::Invalid(
                        "disposed specimens are corrected through the disposal command".into(),
                    ));
                }
                if specimen.loans.iter().any(|loan| loan.returned_at.is_none())
                    && details.possession != Possession::Lent
                {
                    return Err(Error::Invalid(
                        "an active loan keeps possession lent until it is returned".into(),
                    ));
                }
                if !specimen.loans.iter().any(|loan| loan.returned_at.is_none())
                    && details.possession == Possession::Lent
                {
                    return Err(Error::Invalid(
                        "start a loan before setting possession to lent".into(),
                    ));
                }
                specimen.label = details.label;
                specimen.condition = details.condition;
                specimen.ownership = details.ownership.into();
                specimen.possession = details.possession;
                specimen.acquisition = details.acquisition;
                specimen.private_notes = details.private_notes;
                Ok(())
            },
        )
    }

    pub fn create_component(
        &self,
        actor: &Principal,
        id: ItemId,
        specimen_id: SpecimenId,
        expected: Revision,
        details: ComponentDetails,
        context: &CommandContext,
    ) -> Result<Item> {
        validate_component_details(&details)?;
        let component_id = deterministic_component_id(actor.user_id, context.key());
        let carrier_id = details
            .carrier
            .as_ref()
            .map(|_| deterministic_carrier_id(actor.user_id, context.key()));
        let request = (
            id,
            specimen_id,
            component_id,
            carrier_id,
            expected,
            details.clone(),
        );
        self.update_inventory(
            actor,
            (id, expected),
            context,
            "component.create",
            &request,
            move |item| {
                let specimen = specimen_mut(item, specimen_id)?;
                specimen.components.push(Component {
                    id: component_id,
                    label: details.label,
                    kind: details.kind,
                    position: details.position,
                    carrier: details.carrier.map(|carrier| Carrier {
                        id: carrier_id.expect("carrier identity was derived"),
                        label: carrier.label,
                        kind: carrier.kind,
                    }),
                    migration_provenance: None,
                });
                specimen
                    .components
                    .sort_by_key(|component| component.position);
                Ok(())
            },
        )
    }

    pub fn update_component(
        &self,
        actor: &Principal,
        target: (ItemId, SpecimenId, ComponentId),
        expected: Revision,
        details: ComponentDetails,
        context: &CommandContext,
    ) -> Result<Item> {
        let (id, specimen_id, component_id) = target;
        validate_component_details(&details)?;
        let new_carrier_id = deterministic_carrier_id(actor.user_id, context.key());
        let request = (id, specimen_id, component_id, expected, details.clone());
        self.update_inventory(
            actor,
            (id, expected),
            context,
            "component.update",
            &request,
            move |item| {
                let current = item
                    .specimens
                    .iter()
                    .find(|specimen| specimen.id == specimen_id)
                    .and_then(|specimen| {
                        specimen
                            .components
                            .iter()
                            .find(|component| component.id == component_id)
                    })
                    .cloned()
                    .ok_or(Error::NotFound)?;
                if current.carrier.is_some()
                    && details.carrier.is_none()
                    && item.captures.iter().any(|capture| {
                        capture.association.carrier_id
                            == current.carrier.as_ref().map(|carrier| carrier.id)
                    })
                {
                    return Err(Error::Conflict);
                }
                let specimen = specimen_mut(item, specimen_id)?;
                let component = specimen
                    .components
                    .iter_mut()
                    .find(|component| component.id == component_id)
                    .ok_or(Error::NotFound)?;
                let carrier_id = component
                    .carrier
                    .as_ref()
                    .map(|carrier| carrier.id)
                    .unwrap_or(new_carrier_id);
                component.label = details.label;
                component.kind = details.kind;
                component.position = details.position;
                component.carrier = details.carrier.map(|carrier| Carrier {
                    id: carrier_id,
                    label: carrier.label,
                    kind: carrier.kind,
                });
                specimen
                    .components
                    .sort_by_key(|component| component.position);
                Ok(())
            },
        )
    }

    pub fn remove_component(
        &self,
        actor: &Principal,
        id: ItemId,
        specimen_id: SpecimenId,
        component_id: ComponentId,
        expected: Revision,
        context: &CommandContext,
    ) -> Result<Item> {
        let request = (id, specimen_id, component_id, expected);
        self.update_inventory(
            actor,
            (id, expected),
            context,
            "component.remove",
            &request,
            move |item| {
                let component = item
                    .specimens
                    .iter()
                    .find(|specimen| specimen.id == specimen_id)
                    .and_then(|specimen| {
                        specimen
                            .components
                            .iter()
                            .find(|component| component.id == component_id)
                    })
                    .ok_or(Error::NotFound)?;
                let carrier_id = component.carrier.as_ref().map(|carrier| carrier.id);
                if item.captures.iter().any(|capture| {
                    capture.association.component_id == Some(component_id)
                        || carrier_id.is_some_and(|carrier_id| {
                            capture.association.carrier_id == Some(carrier_id)
                        })
                }) || item
                    .assets
                    .iter()
                    .any(|asset| asset.subject == AssetSubject::Component { component_id })
                {
                    return Err(Error::Conflict);
                }
                let specimen = specimen_mut(item, specimen_id)?;
                specimen
                    .components
                    .retain(|component| component.id != component_id);
                Ok(())
            },
        )
    }

    pub fn start_loan(
        &self,
        actor: &Principal,
        id: ItemId,
        specimen_id: SpecimenId,
        expected: Revision,
        command: StartLoanCommand,
        context: &CommandContext,
    ) -> Result<Item> {
        if command.contact.trim().is_empty()
            || command.contact.len() > 4096
            || command.due_at.is_some_and(|due| due < command.lent_at)
        {
            return Err(Error::Invalid("loan details are invalid".into()));
        }
        let loan_id = deterministic_loan_id(actor.user_id, context.key());
        let request = (id, specimen_id, loan_id, expected, command.clone());
        self.update_inventory(
            actor,
            (id, expected),
            context,
            "loan.start",
            &request,
            move |item| {
                let specimen = specimen_mut(item, specimen_id)?;
                if matches!(specimen.ownership, Ownership::Disposed { .. })
                    || specimen.loans.iter().any(|loan| loan.returned_at.is_none())
                {
                    return Err(Error::Conflict);
                }
                specimen.loans.push(Loan {
                    id: loan_id,
                    contact: command.contact,
                    lent_at: command.lent_at,
                    due_at: command.due_at,
                    returned_at: None,
                });
                specimen.possession = Possession::Lent;
                Ok(())
            },
        )
    }

    pub fn return_loan(
        &self,
        actor: &Principal,
        target: (ItemId, SpecimenId, LoanId),
        expected: Revision,
        command: ReturnLoanCommand,
        context: &CommandContext,
    ) -> Result<Item> {
        let (id, specimen_id, loan_id) = target;
        let request = (id, specimen_id, loan_id, expected, command.clone());
        self.update_inventory(
            actor,
            (id, expected),
            context,
            "loan.return",
            &request,
            move |item| {
                let specimen = specimen_mut(item, specimen_id)?;
                let loan = specimen
                    .loans
                    .iter_mut()
                    .find(|loan| loan.id == loan_id)
                    .ok_or(Error::NotFound)?;
                if loan.returned_at.is_some() || command.returned_at < loan.lent_at {
                    return Err(Error::Conflict);
                }
                loan.returned_at = Some(command.returned_at);
                specimen.possession = command.possession.into();
                Ok(())
            },
        )
    }

    pub fn dispose_specimen(
        &self,
        actor: &Principal,
        id: ItemId,
        specimen_id: SpecimenId,
        expected: Revision,
        command: DisposeSpecimenCommand,
        context: &CommandContext,
    ) -> Result<Item> {
        if command.reason.trim().is_empty() || command.reason.len() > 4096 {
            return Err(Error::Invalid(
                "disposal reason is required and bounded".into(),
            ));
        }
        let request = (id, specimen_id, expected, command.clone());
        self.update_inventory(
            actor,
            (id, expected),
            context,
            "specimen.dispose",
            &request,
            move |item| {
                let specimen = specimen_mut(item, specimen_id)?;
                if specimen.loans.iter().any(|loan| loan.returned_at.is_none()) {
                    return Err(Error::Conflict);
                }
                specimen.ownership = Ownership::Disposed {
                    date: command.date,
                    reason: command.reason,
                };
                specimen.possession = command.possession.into();
                Ok(())
            },
        )
    }
    pub fn adapt_wave1_item_save(
        &self,
        actor: &Principal,
        path_id: ItemId,
        item: Item,
        expected: Revision,
    ) -> Result<SaveItemCommand> {
        if path_id != item.id {
            return Err(Error::Invalid("item ID mismatch".into()));
        }
        let eligibility = self.content_write_eligibility(actor, item.collection_id)?;
        self.validate_owned_domain(&eligibility, &item.domain)?;
        match self.reader.item(path_id) {
            Ok(current) => {
                let current_eligibility =
                    self.content_write_eligibility(actor, current.collection_id)?;
                self.validate_owned_domain(&current_eligibility, &current.domain)?;
                let submitted_owned = serde_json::to_value((
                    item.revision,
                    &item.specimens,
                    &item.captures,
                    &item.assets,
                    &item.metadata,
                    item.deleted,
                    item.updated_at,
                ))
                .map_err(|error| Error::Invalid(error.to_string()))?;
                let canonical_owned = serde_json::to_value((
                    current.revision,
                    &current.specimens,
                    &current.captures,
                    &current.assets,
                    &current.metadata,
                    current.deleted,
                    current.updated_at,
                ))
                .map_err(|error| Error::Storage(error.to_string()))?;
                if item.collection_id != current.collection_id
                    || expected != current.revision
                    || submitted_owned != canonical_owned
                {
                    return Err(Error::Invalid(
                        "the Wave 1 item envelope changed server-owned fields; refresh and send only editable command fields"
                            .into(),
                    ));
                }
            }
            Err(Error::NotFound)
                if expected == Revision(0)
                    && item.revision == Revision(0)
                    && item.specimens.is_empty()
                    && item.captures.is_empty()
                    && item.assets.is_empty()
                    && !item.deleted
                    && serde_json::to_value(&item.metadata)
                        .map_err(|error| Error::Invalid(error.to_string()))?
                        == serde_json::to_value(MetadataState::default())
                            .map_err(|error| Error::Storage(error.to_string()))? => {}
            Err(Error::NotFound) => {
                return Err(Error::Invalid(
                    "a new Wave 1 item envelope contains server-owned state".into(),
                ));
            }
            Err(error) => return Err(error),
        }
        Ok(item.into())
    }
    pub fn capture(&self, actor: &Principal, id: CaptureId) -> Result<Capture> {
        let capture = self.reader.capture(id)?;
        self.authorize(actor, capture.collection_id, Action::Browse)?;
        Ok(capture)
    }
    pub fn work(&self, actor: &Principal, id: WorkId) -> Result<Work> {
        let work = self.reader.work(id)?;
        self.authorize(actor, work.collection_id, Action::Browse)?;
        Ok(work)
    }
    pub fn save_work(
        &self,
        actor: &Principal,
        id: WorkId,
        command: SaveWorkCommand,
        expected: Revision,
        context: &CommandContext,
    ) -> Result<Work> {
        let request = (id, expected, command.clone());
        self.content_write_eligibility(actor, command.collection_id)?;
        let (revision, migration_provenance, deleted, updated_at) = match self.reader.work(id) {
            Ok(previous) => {
                self.content_write_eligibility(actor, previous.collection_id)?;
                if previous.collection_id != command.collection_id {
                    return Err(Error::Invalid("work cannot move across collections".into()));
                }
                (
                    previous.revision,
                    previous.migration_provenance,
                    previous.deleted,
                    previous.updated_at,
                )
            }
            Err(Error::NotFound) => (Revision(0), vec![], false, Utc::now()),
            Err(error) => return Err(error),
        };
        let work = Work {
            id,
            collection_id: command.collection_id,
            revision,
            provisional: command.provisional,
            title: command.title,
            translations: command.translations,
            identifiers: command.identifiers,
            migration_provenance,
            deleted,
            updated_at,
        };
        work.validate()?;
        self.execute_command(actor, context, "work.save", &request, |effects| {
            effects.save_work(work, expected)
        })
    }
    pub fn edition(&self, actor: &Principal, id: EditionId) -> Result<Edition> {
        let edition = self.reader.edition(id)?;
        self.authorize(actor, edition.collection_id, Action::Browse)?;
        Ok(edition)
    }
    pub fn save_edition(
        &self,
        actor: &Principal,
        id: EditionId,
        command: SaveEditionCommand,
        expected: Revision,
        context: &CommandContext,
    ) -> Result<Edition> {
        let request = (id, expected, command.clone());
        self.content_write_eligibility(actor, command.collection_id)?;
        let (revision, migration_provenance, deleted, updated_at) = match self.reader.edition(id) {
            Ok(previous) => {
                self.content_write_eligibility(actor, previous.collection_id)?;
                if previous.collection_id != command.collection_id {
                    return Err(Error::Invalid(
                        "edition cannot move across collections".into(),
                    ));
                }
                (
                    previous.revision,
                    previous.migration_provenance,
                    previous.deleted,
                    previous.updated_at,
                )
            }
            Err(Error::NotFound) => (Revision(0), vec![], false, Utc::now()),
            Err(error) => return Err(error),
        };
        let edition = Edition {
            id,
            collection_id: command.collection_id,
            revision,
            work_id: command.work_id,
            provisional: command.provisional,
            title: command.title,
            expected_components: command.expected_components,
            identifiers: command.identifiers,
            migration_provenance,
            deleted,
            updated_at,
        };
        edition.validate()?;
        self.execute_command(actor, context, "edition.save", &request, |effects| {
            effects.save_edition(edition, expected)
        })
    }
    pub fn save_capture(
        &self,
        actor: &Principal,
        id: CaptureId,
        command: SaveCaptureCommand,
        expected: Revision,
        context: &CommandContext,
    ) -> Result<Capture> {
        let request = (id, expected, command.clone());
        let (revision, created_at, representations) = match self.reader.capture(id) {
            Ok(previous) => {
                self.content_write_eligibility(actor, previous.collection_id)?;
                if previous.collection_id != command.collection_id {
                    return Err(Error::Invalid(
                        "capture cannot move across collections".into(),
                    ));
                }
                (
                    previous.revision,
                    previous.created_at,
                    previous.representations,
                )
            }
            Err(Error::NotFound) => (Revision(0), Utc::now(), vec![]),
            Err(error) => return Err(error),
        };
        self.content_write_eligibility(actor, command.collection_id)?;
        let capture = Capture {
            id,
            collection_id: command.collection_id,
            revision,
            association: command.association,
            created_at,
            representations,
        };
        capture.validate()?;
        self.execute_command(actor, context, "capture.save", &request, |effects| {
            effects.save_capture(capture, expected)
        })
    }
    pub fn evidence(&self, actor: &Principal, id: ItemId) -> Result<Vec<PresentedEvidence>> {
        let item = self.reader.item(id)?;
        let role = self.authorize(actor, item.collection_id, Action::Browse)?;
        let private = role != Role::Viewer;
        Ok(self
            .reader
            .evidence(id)?
            .into_iter()
            .map(|evidence| PresentedEvidence {
                id: evidence.id,
                item_id: evidence.item_id,
                representation_id: evidence.representation_id,
                kind: evidence.kind,
                outcome: evidence.outcome,
                created_at: evidence.created_at,
                inputs: private.then_some(evidence.inputs),
                implementation: private.then_some(evidence.implementation),
                implementation_version: private.then_some(evidence.implementation_version),
                catalog: if private { evidence.catalog } else { None },
                details: private.then_some(evidence.details),
                provenance_limitation: Some(
                    "Legacy schema-3 Evidence has limited provenance and is not current source-bound processing proof."
                        .into(),
                ),
            })
            .collect())
    }
    pub fn processing_summaries(
        &self,
        actor: &Principal,
        query: ProcessingQuery,
    ) -> Result<Page<ProcessingSummary>> {
        query.validate()?;
        self.authorize(actor, query.collection_id, Action::Curate)?;
        let current_source = self
            .processing
            .recorded_source(query.collection_id, query.source)?;
        let page = self.processing.observations(&query)?;
        Ok(Page {
            items: page
                .items
                .into_iter()
                .map(|observation| {
                    let snapshot_observation = observation
                        .snapshot_id
                        .and_then(|snapshot_id| {
                            self.processing
                                .snapshot(SnapshotQuery {
                                    collection_id: query.collection_id,
                                    snapshot_id,
                                })
                                .ok()
                        })
                        .map(|snapshot| snapshot.parsed_observation);
                    ProcessingSummary {
                        observation_id: observation.id,
                        operation: observation.operation,
                        source: observation.source_fingerprint.source,
                        snapshot_id: observation.snapshot_id,
                        snapshot_observation,
                        result: observation.typed_result.payload,
                        current: observation.source_fingerprint.source == current_source.source
                            && observation.source_fingerprint.files == current_source.files,
                        support: observation.support,
                        limitations: observation.limitations,
                        legacy_provenance: false,
                        created_at: observation.created_at,
                    }
                })
                .collect(),
            next_cursor: page.next_cursor,
        })
    }

    pub fn capture_processing_summaries(
        &self,
        actor: &Principal,
        capture_id: CaptureId,
        representation_id: RepresentationId,
        after: Option<ObservationId>,
        limit: u16,
    ) -> Result<Page<ProcessingSummary>> {
        let capture = self.reader.capture(capture_id)?;
        if !capture
            .representations
            .iter()
            .any(|representation| representation.id == representation_id)
        {
            return Err(Error::NotFound);
        }
        self.authorize(actor, capture.collection_id, Action::Curate)?;
        let source = PackageScope::Representation(representation_id);
        let source_fingerprint = self
            .processing
            .recorded_source(capture.collection_id, source)?;
        self.processing_summaries(
            actor,
            ProcessingQuery {
                collection_id: capture.collection_id,
                source,
                source_fingerprint,
                operation: None,
                after,
                limit,
            },
        )
    }
    pub fn activity(&self, actor: &Principal, id: ItemId) -> Result<serde_json::Value> {
        let item = self.reader.item(id)?;
        self.authorize(actor, item.collection_id, Action::Browse)?;
        self.content.activity(actor.user_id, id)
    }
    pub fn save_activity(
        &self,
        actor: &Principal,
        id: ItemId,
        value: serde_json::Value,
        context: &CommandContext,
    ) -> Result<()> {
        let item = self.reader.item(id)?;
        self.authorize(actor, item.collection_id, Action::Browse)?;
        if serde_json::to_vec(&value)
            .map_err(|error| Error::Invalid(error.to_string()))?
            .len()
            > 8192
        {
            return Err(Error::Invalid("activity too large".into()));
        }
        let request = (id, value.clone());
        self.execute_command(actor, context, "activity.save", &request, |effects| {
            effects.save_activity(actor.user_id, id, value)
        })
    }
    pub fn representation(
        &self,
        actor: &Principal,
        item_id: ItemId,
        representation: RepresentationId,
        index: usize,
    ) -> Result<MediaResource> {
        let item = self.reader.item(item_id)?;
        self.authorize(actor, item.collection_id, Action::Browse)?;
        if item.deleted {
            return Err(Error::NotFound);
        }
        self.content.representation(item_id, representation, index)
    }
    pub fn asset(
        &self,
        actor: &Principal,
        item_id: ItemId,
        asset_id: AssetId,
        thumbnail: bool,
    ) -> Result<MediaResource> {
        let item = self.reader.item(item_id)?;
        let role = self.authorize(actor, item.collection_id, Action::Browse)?;
        if item.deleted {
            return Err(Error::NotFound);
        }
        let state = self
            .content
            .asset_states_for_item(item_id)?
            .into_iter()
            .find(|state| state.asset.id == asset_id && !state.deleted)
            .ok_or(Error::NotFound)?;
        if role == Role::Viewer && state.asset.visibility == Visibility::Curators {
            return Err(Error::Forbidden);
        }
        self.content.asset_resource(asset_id, thumbnail)
    }
    pub fn asset_presented(
        &self,
        actor: &Principal,
        collection_id: CollectionId,
        asset_id: AssetId,
    ) -> Result<PresentedAsset> {
        let role = self.authorize(actor, collection_id, Action::Browse)?;
        let state = self.content.asset_state(asset_id)?;
        if state.collection_id != collection_id || state.deleted {
            return Err(Error::NotFound);
        }
        if role == Role::Viewer && state.asset.visibility == Visibility::Curators {
            return Err(Error::Forbidden);
        }
        Ok(PresentedAsset::from_state(state, role))
    }
    pub fn asset_resource(
        &self,
        actor: &Principal,
        collection_id: CollectionId,
        asset_id: AssetId,
        thumbnail: bool,
    ) -> Result<MediaResource> {
        let role = self.authorize(actor, collection_id, Action::Browse)?;
        let state = self.content.asset_state(asset_id)?;
        if state.collection_id != collection_id || state.deleted {
            return Err(Error::NotFound);
        }
        if role == Role::Viewer && state.asset.visibility == Visibility::Curators {
            return Err(Error::Forbidden);
        }
        self.content.asset_resource(asset_id, thumbnail)
    }
    pub fn create_asset(
        &self,
        actor: &Principal,
        command: CreateAssetCommand,
        bytes: &[u8],
        context: &CommandContext,
    ) -> Result<PresentedAsset> {
        validate_asset_upload(&command.kind, &command.alt_text, bytes)?;
        let role = self.authorize(actor, command.collection_id, Action::Curate)?;
        self.content_write_eligibility(actor, command.collection_id)?;
        let subject_collection = self.content.asset_subject_collection(command.subject)?;
        if subject_collection != command.collection_id {
            return Err(Error::Invalid(
                "asset subject is outside the selected collection".into(),
            ));
        }
        let asset_id = deterministic_new_asset_id(actor.user_id, context.key());
        let request = (asset_id, command.clone(), Digest::sha256(bytes));
        let state = self.execute_command(actor, context, "asset.create", &request, |effects| {
            effects.create_asset(asset_id, command, bytes)
        })?;
        Ok(PresentedAsset::from_state(state, role))
    }
    pub fn update_asset(
        &self,
        actor: &Principal,
        asset_id: AssetId,
        expected: Revision,
        command: UpdateAssetCommand,
        context: &CommandContext,
    ) -> Result<PresentedAsset> {
        if command.alt_text.len() > 4096 {
            return Err(Error::Invalid("alt text exceeds upload limit".into()));
        }
        let current = self.content.asset_state(asset_id)?;
        let role = self.authorize(actor, current.collection_id, Action::Curate)?;
        self.content_write_eligibility(actor, current.collection_id)?;
        if current.deleted {
            return Err(Error::NotFound);
        }
        let request = (asset_id, expected, command.clone());
        let state = self.execute_command(actor, context, "asset.update", &request, |effects| {
            effects.update_asset(asset_id, expected, command.visibility, command.alt_text)
        })?;
        Ok(PresentedAsset::from_state(state, role))
    }
    pub fn tombstone_asset(
        &self,
        actor: &Principal,
        asset_id: AssetId,
        expected: Revision,
        context: &CommandContext,
    ) -> Result<PresentedAsset> {
        let current = self.content.asset_state(asset_id)?;
        let role = self.authorize(actor, current.collection_id, Action::Curate)?;
        self.content_write_eligibility(actor, current.collection_id)?;
        if current.deleted {
            return Err(Error::NotFound);
        }
        let request = (asset_id, expected);
        let state =
            self.execute_command(actor, context, "asset.tombstone", &request, |effects| {
                effects.tombstone_asset(asset_id, expected)
            })?;
        Ok(PresentedAsset::from_state(state, role))
    }
    pub fn publish_cover(
        &self,
        actor: &Principal,
        item_id: ItemId,
        expected: Revision,
        bytes: &[u8],
        alt_text: String,
        context: &CommandContext,
    ) -> Result<Item> {
        let item = self.reader.item(item_id)?;
        let eligibility = self.content_write_eligibility(actor, item.collection_id)?;
        self.validate_owned_domain(&eligibility, &item.domain)?;
        if item.revision != expected {
            return Err(Error::Conflict);
        }
        let asset_id = deterministic_asset_id(actor.user_id, context.key());
        validate_asset_upload(&AssetKind::Cover, &alt_text, bytes)?;
        let request = (item_id, expected, Digest::sha256(bytes), alt_text.clone());
        self.execute_command(
            actor,
            context,
            "asset.create.compat-cover",
            &request,
            |effects| {
                effects.create_asset(
                    asset_id,
                    CreateAssetCommand {
                        collection_id: item.collection_id,
                        subject: AssetSubject::Item { item_id },
                        kind: AssetKind::Cover,
                        visibility: Visibility::Shared,
                        alt_text,
                    },
                    bytes,
                )
            },
        )?;
        self.reader.item(item_id)
    }
    fn authorized_provisional_import(
        &self,
        actor: &Principal,
        id: ImportId,
    ) -> Result<ImportRecord> {
        let record = self.provisional_imports.import_record(id)?;
        if record.user_id != actor.user_id {
            return Err(Error::NotFound);
        }
        self.authorize(actor, record.view.collection_id, Action::Curate)?;
        Ok(record)
    }
    fn provisional_import_for_write(
        &self,
        actor: &Principal,
        id: ImportId,
    ) -> Result<(ImportRecord, ContentWriteEligibility)> {
        let record = self.authorized_provisional_import(actor, id)?;
        let eligibility = self.content_write_eligibility(actor, record.view.collection_id)?;
        Ok((record, eligibility))
    }
    pub fn create_provisional_import(
        &self,
        actor: &Principal,
        command: CreateImportCommand,
        context: &CommandContext,
    ) -> Result<ImportView> {
        let CreateImportCommand {
            collection_id,
            source,
            files,
            association,
            expected_item_revision,
        } = command;
        let eligibility = self.content_write_eligibility(actor, collection_id)?;
        let item = self.validate_import_association(collection_id, association)?;
        if let Some(item) = &item {
            self.validate_owned_domain(&eligibility, &item.domain)?;
        }
        if item.is_some() != expected_item_revision.is_some() {
            return Err(Error::Invalid(
                "an Item association requires its expected revision".into(),
            ));
        }
        if let (Some(item), Some(expected)) = (item, expected_item_revision)
            && item.revision != expected
        {
            return Err(Error::Conflict);
        }
        let request = (
            collection_id,
            source.clone(),
            files.clone(),
            association,
            expected_item_revision,
        );
        let empty_digest = Digest::sha256(b"");
        let declared_files = files
            .into_iter()
            .map(|file| {
                let supplied_digest = file
                    .sha256
                    .map(|value| Digest::new(HashAlgorithm::Sha256, value))
                    .transpose()?;
                let digest = if file.bytes.0 == 0 {
                    if supplied_digest
                        .as_ref()
                        .is_some_and(|digest| digest != &empty_digest)
                    {
                        return Err(Error::Invalid(
                            "declared digest does not match the empty file".into(),
                        ));
                    }
                    Some(empty_digest.clone())
                } else {
                    supplied_digest
                };
                Ok(ImportDeclaredFile {
                    name: file.name,
                    bytes: file.bytes,
                    received_bytes: ByteLength(0),
                    digest,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let phase = if declared_files.iter().all(|file| file.bytes.0 == 0) {
            ImportPhase::Analyzing
        } else {
            ImportPhase::Staging
        };
        let now = self.authentication.now();
        let record = ImportRecord {
            user_id: actor.user_id,
            view: ImportView {
                id: ImportId(deterministic_inventory_uuid(
                    actor.user_id,
                    "import-create",
                    context.key(),
                )),
                revision: 1,
                collection_id,
                source,
                files: declared_files,
                capture_id: CaptureId(deterministic_inventory_uuid(
                    actor.user_id,
                    "import-capture",
                    context.key(),
                )),
                representation_id: RepresentationId(deterministic_inventory_uuid(
                    actor.user_id,
                    "import-representation",
                    context.key(),
                )),
                association,
                expected_item_revision,
                candidates: vec![],
                limitations: vec![],
                review: None,
                analysis_job_id: None,
                phase,
                created_at: now,
                updated_at: now,
            },
        };
        self.execute_command(actor, context, "import.create", &request, |effects| {
            effects.create_import(record).map(|record| record.view)
        })
    }
    pub fn provisional_import(&self, actor: &Principal, id: ImportId) -> Result<ImportView> {
        Ok(self.authorized_provisional_import(actor, id)?.view)
    }
    pub fn stage_provisional_import_chunk(
        &self,
        actor: &Principal,
        id: ImportId,
        index: usize,
        offset: u64,
        bytes: &[u8],
    ) -> Result<ImportView> {
        self.provisional_import_for_write(actor, id)?;
        self.provisional_imports
            .stage_import_chunk(id, index, offset, bytes)?;
        self.reconcile_guided_ingest(id)?;
        self.provisional_imports
            .import_record(id)
            .map(|record| record.view)
    }
    pub fn enqueue_inbox_import_staging(
        &self,
        actor: &Principal,
        id: ImportId,
        source_locator: &str,
        context: &CommandContext,
    ) -> Result<JobId> {
        let (record, _) = self.provisional_import_for_write(actor, id)?;
        if record.view.source.kind != ImportSourceKind::Inbox {
            return Err(Error::Invalid(
                "only configured inbox Imports can stage from an inbox source".into(),
            ));
        }
        let job = JobId::new();
        let total_units = record.view.files.iter().map(|file| file.bytes.0).sum();
        let spec = JobSpec {
            id: job,
            user_id: actor.user_id,
            collection_id: Some(record.view.collection_id),
            item_id: None,
            kind: "import_staging".into(),
            phase: "queued".into(),
            total_units,
            request: serde_json::json!({
                "import_id": id,
                "source_locator": source_locator,
            }),
            frozen_input: None,
        };
        self.execute_command(
            actor,
            context,
            "import.inbox.stage.enqueue",
            &(id, source_locator),
            |effects| effects.enqueue_job(spec),
        )
    }
    pub fn enqueue_import_analysis(
        &self,
        actor: &Principal,
        id: ImportId,
        module: ModuleId,
        context: &CommandContext,
    ) -> Result<JobId> {
        module.validate()?;
        let (record, eligibility) = self.provisional_import_for_write(actor, id)?;
        let job = deterministic_job_id(actor.user_id, "import.analyze", context.key());
        let capability = CapabilityId::new(format!("{}.import-analysis", module.0))?;
        let contract = self.modules.capability_contract(&capability)?;
        if contract.scope != collector_modules::OperationScope::CollectionDomain
            || contract.owner != eligibility.owner
            || module != eligibility.owner
        {
            return Err(Error::Invalid(
                "import identity analysis must be owned by the collection module".into(),
            ));
        }
        let operation_version = contract.schema_version;
        let inventory = self
            .processing
            .package_inventory(record.view.collection_id, PackageScope::Import(id))?;
        let source_fingerprint = self.fingerprint_inventory(&inventory)?;
        let provider = ModuleId("org.junk.providers.local-dat".into());
        let snapshot_id = self
            .processing
            .active_snapshot(record.view.collection_id, &provider)?;
        let domain = record
            .view
            .association
            .item_id
            .map(|item| self.reader.item(item).map(|item| item.domain))
            .transpose()?;
        let selection = RawJson::from_value(serde_json::json!({
            "import_id": id,
            "representation_id": record.view.representation_id,
            "files": record.view.files,
            "snapshot_id": snapshot_id,
            "system": null,
            "domain": domain,
        }));
        let frozen = FrozenProcessingJob {
            collection_id: record.view.collection_id,
            collection_binding: eligibility.binding,
            operation: capability,
            operation_version,
            source: PackageScope::Import(id),
            source_fingerprint,
            snapshot_id,
            observation_id: ObservationId(job.0),
            selection,
            created_at: self.authentication.now(),
        };
        frozen.validate()?;
        let spec = JobSpec {
            id: job,
            user_id: actor.user_id,
            collection_id: Some(record.view.collection_id),
            item_id: record.view.association.item_id,
            kind: "import_analysis".into(),
            phase: "queued".into(),
            total_units: 1,
            request: serde_json::json!({"import_id":id,"module":module}),
            frozen_input: Some(FrozenJobInput::Processing(frozen)),
        };
        self.execute_command(
            actor,
            context,
            "import.analyze.enqueue",
            &(id, module),
            |effects| {
                if effects.import_record(id)?.view.phase != ImportPhase::Analyzing {
                    return Err(Error::Conflict);
                }
                effects.enqueue_job(spec)
            },
        )
    }
    fn analyze_import_for_job(&self, claim: &JobClaim) -> Result<ImportView> {
        let id: ImportId = claim
            .request
            .get("import_id")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| Error::Storage("analysis job has no Import identity".into()))?
            .parse()
            .map_err(|error| Error::Storage(format!("invalid Import identity: {error}")))?;
        let frozen = match claim.frozen_input.as_ref() {
            Some(FrozenJobInput::Processing(value)) => value,
            _ => {
                return Err(Error::Storage(
                    "analysis job has no frozen processing input".into(),
                ));
            }
        };
        frozen.validate()?;
        let (module, operation_version) = self.modules.capability_owner(&frozen.operation)?;
        if operation_version != frozen.operation_version
            || frozen.source != PackageScope::Import(id)
        {
            return Err(Error::Conflict);
        }
        if self
            .jobs
            .cancellation_requested(claim.id, &claim.claim_token)?
        {
            return Err(Error::JobCancelled);
        }
        let mut record = self.provisional_imports.import_record(id)?;
        if record.user_id != claim.user_id || record.view.phase != ImportPhase::Analyzing {
            return Err(Error::Conflict);
        }
        if record.view.collection_id != frozen.collection_id
            || !self
                .reader
                .role(claim.user_id, frozen.collection_id)?
                .allows(Action::Curate)
        {
            return Err(Error::Forbidden);
        }
        let eligibility =
            self.content_write_eligibility_for_user(claim.user_id, frozen.collection_id)?;
        self.validate_frozen_binding(&eligibility, &frozen.collection_binding)?;
        let contract = self.modules.capability_contract(&frozen.operation)?;
        if contract.scope != collector_modules::OperationScope::CollectionDomain
            || contract.owner != eligibility.owner
            || contract.owner != module
        {
            return Err(Error::Conflict);
        }
        let inventory = self
            .processing
            .package_inventory(frozen.collection_id, frozen.source)?;
        frozen.source_fingerprint.validate_inventory(&inventory)?;
        if self.fingerprint_inventory(&inventory)? != frozen.source_fingerprint {
            return Err(Error::Conflict);
        }
        let host = ProcessingHost {
            port: self.processing.as_ref(),
            access: PackageAccess {
                collection_id: frozen.collection_id,
                scope: frozen.source,
            },
            source_fingerprint: &frozen.source_fingerprint,
            frozen_snapshot_id: frozen.snapshot_id,
            catalog_parsers: self.catalog_parsers.as_ref(),
            jobs: self.jobs.clone(),
            job_id: claim.id,
            claim_token: claim.claim_token.clone(),
        };
        let analyzed = self.modules.operate(
            OperationRequest {
                capability: frozen.operation.clone(),
                input: frozen.selection.clone(),
                collection_id: frozen.collection_id,
                entity: OperationEntity::Import(id),
                idempotency_key: claim.id.to_string(),
            },
            &host,
        );
        if self
            .jobs
            .cancellation_requested(claim.id, &claim.claim_token)?
        {
            return Err(Error::JobCancelled);
        }
        let (support, typed_result, limitations) = match analyzed {
            Ok(output) => {
                let parsed: ImportAnalysisOutput = output.value.parse()?;
                record.view.candidates = parsed
                    .candidates
                    .into_iter()
                    .map(|candidate| ImportCandidate {
                        key: candidate.key,
                        module: module.clone(),
                        label: candidate.label,
                        observation: candidate.observation,
                    })
                    .collect();
                record.view.limitations = parsed.limitations.clone();
                (
                    OperationSupport::Supported,
                    ModuleData {
                        module: module.clone(),
                        schema_version: operation_version,
                        payload: output.value,
                    },
                    parsed.limitations,
                )
            }
            Err(ModuleError::UnavailableCapability(limitation)) => {
                record.view.candidates.clear();
                record.view.limitations = vec![limitation.clone()];
                (
                    OperationSupport::UnavailableDependency {
                        dependency: module.0.clone(),
                        reason: limitation.clone(),
                    },
                    ModuleData {
                        module: module.clone(),
                        schema_version: operation_version,
                        payload: RawJson::from_value(serde_json::json!({"unavailable":limitation})),
                    },
                    vec![limitation],
                )
            }
            Err(error) => return Err(error.into()),
        };
        if self
            .jobs
            .cancellation_requested(claim.id, &claim.claim_token)?
        {
            return Err(Error::JobCancelled);
        }
        let current =
            self.content_write_eligibility_for_user(claim.user_id, frozen.collection_id)?;
        self.validate_frozen_binding(&current, &frozen.collection_binding)?;
        let current_contract = self.modules.capability_contract(&frozen.operation)?;
        if current_contract.scope != collector_modules::OperationScope::CollectionDomain
            || current_contract.owner != current.owner
            || current_contract.owner != module
            || current_contract.schema_version != frozen.operation_version
        {
            return Err(Error::Conflict);
        }
        if self.fingerprint_inventory(&inventory)? != frozen.source_fingerprint {
            return Err(Error::Conflict);
        }
        let observation = ProcessingObservation {
            id: frozen.observation_id,
            collection_id: frozen.collection_id,
            operation: frozen.operation.clone(),
            operation_version: frozen.operation_version,
            source_fingerprint: frozen.source_fingerprint.clone(),
            snapshot_id: frozen.snapshot_id,
            support,
            raw_observations: vec![],
            typed_result,
            limitations,
            created_at: frozen.created_at,
        };
        observation.validate()?;
        self.processing
            .save_provisional_observation(id, claim.id, &observation)?;
        record.view.analysis_job_id = Some(claim.id);
        record.view.phase = ImportPhase::AwaitingReview;
        let revision = record.view.revision;
        Ok(self.provisional_imports.save_import(record, revision)?.view)
    }

    fn fingerprint_inventory(&self, inventory: &PackageInventory) -> Result<SourceFingerprint> {
        self.fingerprint_inventory_checked(inventory, &mut || Ok(()))
    }
    fn fingerprint_inventory_checked(
        &self,
        inventory: &PackageInventory,
        checkpoint: &mut dyn FnMut() -> Result<()>,
    ) -> Result<SourceFingerprint> {
        inventory.validate()?;
        let mut files = Vec::with_capacity(inventory.files.len());
        for file in &inventory.files {
            checkpoint()?;
            let mut reader =
                self.processing
                    .package_file(inventory.collection_id, inventory.scope, file.id)?;
            let mut bytes = 0_u64;
            let mut hasher = sha2::Sha256::new();
            let mut buffer = vec![0; 1024 * 1024];
            loop {
                checkpoint()?;
                let read = reader
                    .read(&mut buffer)
                    .map_err(|error| Error::Storage(error.to_string()))?;
                if read == 0 {
                    break;
                }
                bytes = bytes
                    .checked_add(read as u64)
                    .ok_or_else(|| Error::Invalid("package length overflow".into()))?;
                if bytes > file.bytes.0 {
                    return Err(Error::Conflict);
                }
                use sha2::Digest as _;
                hasher.update(&buffer[..read]);
            }
            if bytes != file.bytes.0 {
                return Err(Error::Conflict);
            }
            files.push(PackageFileFingerprint {
                id: file.id,
                bytes: file.bytes,
                sha256: Digest::new(HashAlgorithm::Sha256, hex::encode(hasher.finalize()))?,
            });
        }
        SourceFingerprint::calculate(inventory.scope, files, None, Default::default())
    }
    fn validate_import_association(
        &self,
        collection: CollectionId,
        association: CaptureAssociation,
    ) -> Result<Option<Item>> {
        let Some(item_id) = association.item_id else {
            if association.specimen_id.is_some()
                || association.component_id.is_some()
                || association.carrier_id.is_some()
            {
                return Err(Error::Invalid(
                    "physical Import association requires its enclosing Item".into(),
                ));
            }
            return Ok(None);
        };
        let item = self.reader.item(item_id)?;
        if item.collection_id != collection {
            return Err(Error::NotFound);
        }
        let specimen = association
            .specimen_id
            .map(|id| {
                item.specimens
                    .iter()
                    .find(|specimen| specimen.id == id)
                    .ok_or(Error::NotFound)
            })
            .transpose()?;
        if association.component_id.is_some() && specimen.is_none() {
            return Err(Error::Invalid(
                "component association requires its enclosing specimen".into(),
            ));
        }
        let component = association
            .component_id
            .map(|id| {
                specimen
                    .and_then(|specimen| {
                        specimen
                            .components
                            .iter()
                            .find(|component| component.id == id)
                    })
                    .ok_or(Error::NotFound)
            })
            .transpose()?;
        if association.carrier_id.is_some() && component.is_none() {
            return Err(Error::Invalid(
                "carrier association requires its enclosing component".into(),
            ));
        }
        if let Some(carrier_id) = association.carrier_id
            && component
                .and_then(|value| value.carrier.as_ref())
                .map(|value| value.id)
                != Some(carrier_id)
        {
            return Err(Error::NotFound);
        }
        Ok(Some(item))
    }
    pub fn review_provisional_import(
        &self,
        actor: &Principal,
        id: ImportId,
        command: ReviewImportCommand,
        context: &CommandContext,
    ) -> Result<ImportView> {
        let (current, eligibility) = self.provisional_import_for_write(actor, id)?;
        if !matches!(
            current.view.phase,
            ImportPhase::Analyzing | ImportPhase::AwaitingReview
        ) {
            return Err(Error::Conflict);
        }
        let item =
            self.validate_import_association(current.view.collection_id, command.association)?;
        if let Some(item) = &item {
            self.validate_owned_domain(&eligibility, &item.domain)?;
        }
        if item.is_some() != command.expected_item_revision.is_some() {
            return Err(Error::Invalid(
                "an Item association requires its expected revision".into(),
            ));
        }
        if let (Some(item), Some(expected)) = (&item, command.expected_item_revision)
            && item.revision != expected
        {
            if current.view.revision == command.expected_revision {
                let mut preserved = current;
                preserved.view.association = command.association;
                preserved.view.expected_item_revision = Some(expected);
                preserved.view.review = Some(ImportReviewDecision {
                    identity: command.identity.clone(),
                    association: command.association,
                });
                preserved.view.phase = ImportPhase::AwaitingReview;
                let revision = preserved.view.revision;
                self.provisional_imports.save_import(preserved, revision)?;
            }
            return Err(Error::Conflict);
        }
        if let ImportIdentityDecision::Candidate { key } = &command.identity {
            let candidate = current
                .view
                .candidates
                .iter()
                .find(|candidate| &candidate.key == key)
                .ok_or_else(|| Error::Invalid("selected import candidate is unavailable".into()))?;
            if candidate.module != eligibility.owner {
                return Err(Error::Invalid(
                    "selected import candidate conflicts with the collection owner".into(),
                ));
            }
        }
        self.execute_command(actor, context, "import.review", &(id, &command), |effects| {
            let mut record = effects.import_record(id)?;
            if record.view.revision != command.expected_revision
                || !matches!(
                    record.view.phase,
                    ImportPhase::Analyzing | ImportPhase::AwaitingReview
                )
            {
                return Err(Error::Conflict);
            }
            if record.view.phase == ImportPhase::Analyzing && record.view.limitations.is_empty() {
                record.view.limitations.push(
                    "No analyzer was selected; review continued unidentified or by manual association."
                        .into(),
                );
            }
            record.view.association = command.association;
            record.view.expected_item_revision = command.expected_item_revision;
            record.view.review = Some(ImportReviewDecision {
                identity: command.identity.clone(),
                association: command.association,
            });
            record.view.phase = ImportPhase::AwaitingReview;
            effects
                .save_import(record, command.expected_revision)
                .map(|record| record.view)
        })
    }
    pub fn enqueue_import_publication(
        &self,
        actor: &Principal,
        id: ImportId,
        context: &CommandContext,
    ) -> Result<JobId> {
        let (record, _) = self.provisional_import_for_write(actor, id)?;
        let job = deterministic_job_id(actor.user_id, "import.publish", context.key());
        let spec = JobSpec {
            id: job,
            user_id: actor.user_id,
            collection_id: Some(record.view.collection_id),
            item_id: record.view.association.item_id,
            kind: "import_publication".into(),
            phase: "queued".into(),
            total_units: record.view.files.iter().map(|file| file.bytes.0).sum(),
            request: serde_json::json!({"import_id":id}),
            frozen_input: None,
        };
        self.execute_command(actor, context, "import.publish.enqueue", &id, |effects| {
            let current = effects.import_record(id)?;
            if current.view.phase != ImportPhase::AwaitingReview || current.view.review.is_none() {
                return Err(Error::Conflict);
            }
            effects.enqueue_job(spec)
        })
    }
    fn authorized_upload(&self, actor: &Principal, id: JobId) -> Result<Upload> {
        let upload = self.imports.upload(id)?;
        if upload.user_id != actor.user_id {
            return Err(Error::Forbidden);
        }
        let item = self.reader.item(upload.item_id)?;
        self.authorize(actor, item.collection_id, Action::Curate)?;
        Ok(upload)
    }
    fn upload_for_write(
        &self,
        actor: &Principal,
        id: JobId,
    ) -> Result<(Upload, ContentWriteEligibility)> {
        let upload = self.authorized_upload(actor, id)?;
        let item = self.reader.item(upload.item_id)?;
        let eligibility = self.content_write_eligibility(actor, item.collection_id)?;
        self.validate_owned_domain(&eligibility, &item.domain)?;
        Ok((upload, eligibility))
    }
    pub fn create_upload(
        &self,
        actor: &Principal,
        item_id: ItemId,
        expected: Revision,
        files: Vec<UploadFile>,
        context: &CommandContext,
    ) -> Result<Upload> {
        let item = self.reader.item(item_id)?;
        let eligibility = self.content_write_eligibility(actor, item.collection_id)?;
        self.validate_owned_domain(&eligibility, &item.domain)?;
        let request = (item_id, expected, files.clone());
        self.execute_command(actor, context, "upload.create", &request, |effects| {
            effects.create_upload(actor.user_id, item_id, expected, files)
        })
    }
    pub fn authorize_import(
        &self,
        actor: &Principal,
        item_id: ItemId,
        expected: Revision,
    ) -> Result<()> {
        self.require_administrator(actor)?;
        let item = self.reader.item(item_id)?;
        let eligibility = self.content_write_eligibility(actor, item.collection_id)?;
        self.validate_owned_domain(&eligibility, &item.domain)?;
        if item.revision != expected {
            return Err(Error::Conflict);
        }
        Ok(())
    }
    pub fn upload(&self, actor: &Principal, id: JobId) -> Result<Upload> {
        self.authorized_upload(actor, id)
    }
    pub fn upload_chunk(
        &self,
        actor: &Principal,
        id: JobId,
        index: usize,
        offset: u64,
        bytes: &[u8],
    ) -> Result<Upload> {
        self.upload_for_write(actor, id)?;
        self.imports.upload_chunk(id, index, offset, bytes)
    }
    pub fn cancel_upload(
        &self,
        actor: &Principal,
        id: JobId,
        context: &CommandContext,
    ) -> Result<()> {
        self.authorized_upload(actor, id)?;
        self.execute_command(actor, context, "upload.cancel", &id, |effects| {
            effects.request_job_cancellation(id, actor.user_id)
        })
    }
    pub fn commit_upload(
        &self,
        actor: &Principal,
        id: JobId,
        context: &CommandContext,
    ) -> Result<Item> {
        self.upload_for_write(actor, id)?;
        let result = self.execute_command(actor, context, "upload.commit", &id, |effects| {
            effects.commit_upload(id)
        });
        if let Err(error) = &result
            && !matches!(error, Error::IdempotencyConflict | Error::CommandInProgress)
        {
            let _ = self.imports.fail_upload(
                id,
                "Archive publication failed. Check server logs and retry the retained upload.",
            );
        }
        result
    }
    pub fn enqueue_upload_commit(
        &self,
        actor: &Principal,
        id: JobId,
        context: &CommandContext,
    ) -> Result<JobId> {
        self.upload_for_write(actor, id)?;
        self.execute_command(actor, context, "upload.commit.enqueue", &id, |effects| {
            effects.queue_upload(id)
        })
    }
    pub fn retry_upload_job(
        &self,
        actor: &Principal,
        id: JobId,
        context: &CommandContext,
    ) -> Result<JobId> {
        self.upload_for_write(actor, id)?;
        self.execute_command(actor, context, "upload.retry", &id, |effects| {
            effects.retry_upload(id)
        })
    }
    pub fn enqueue_inbox_upload(
        &self,
        actor: &Principal,
        id: JobId,
        source_locator: &str,
        context: &CommandContext,
    ) -> Result<JobId> {
        self.require_administrator(actor)?;
        self.upload_for_write(actor, id)?;
        let request = (id, source_locator);
        self.execute_command(
            actor,
            context,
            "upload.inbox.enqueue",
            &request,
            |effects| effects.queue_inbox_upload(id, source_locator),
        )
    }
    pub fn cancel_provisional_import(
        &self,
        actor: &Principal,
        id: ImportId,
        context: &CommandContext,
    ) -> Result<ImportView> {
        self.authorized_provisional_import(actor, id)?;
        self.execute_command(actor, context, "import.cancel", &id, |effects| {
            effects.cancel_import(id).map(|record| record.view)
        })
    }
    pub fn fail_upload(&self, actor: &Principal, id: JobId, message: &str) -> Result<()> {
        self.upload_for_write(actor, id)?;
        self.imports.fail_upload(id, message)
    }
    pub fn verify_integrity(
        &self,
        actor: &Principal,
        item_id: ItemId,
        context: &CommandContext,
    ) -> Result<Item> {
        let item = self.reader.item(item_id)?;
        let eligibility = self.content_write_eligibility(actor, item.collection_id)?;
        self.validate_owned_domain(&eligibility, &item.domain)?;
        self.execute_command(actor, context, "integrity.verify", &item_id, |effects| {
            effects.verify_integrity(item_id)
        })
    }

    pub fn enqueue_integrity_verification(
        &self,
        actor: &Principal,
        item_id: ItemId,
        context: &CommandContext,
    ) -> Result<JobId> {
        let item = self.reader.item(item_id)?;
        let eligibility = self.content_write_eligibility(actor, item.collection_id)?;
        self.validate_owned_domain(&eligibility, &item.domain)?;
        let id = deterministic_job_id(actor.user_id, "integrity.verify", context.key());
        let spec = JobSpec {
            id,
            user_id: actor.user_id,
            collection_id: Some(item.collection_id),
            item_id: Some(item_id),
            kind: "integrity_verification".into(),
            phase: "queued".into(),
            total_units: item
                .captures
                .iter()
                .flat_map(|capture| &capture.representations)
                .flat_map(|representation| &representation.files)
                .count() as u64,
            request: serde_json::json!({"item_id":item_id}),
            frozen_input: None,
        };
        if spec.total_units == 0 {
            return Err(Error::Invalid(
                "integrity verification requires at least one stored file".into(),
            ));
        }
        self.execute_command(
            actor,
            context,
            "job.integrity.enqueue",
            &item_id,
            |effects| effects.enqueue_job(spec),
        )
    }

    pub fn enqueue_catalog_acquisition(
        &self,
        actor: &Principal,
        frozen: FrozenAcquisitionJob,
        context: &CommandContext,
    ) -> Result<JobId> {
        let id = deterministic_job_id(actor.user_id, "catalog.acquire", context.key());
        let spec = self.catalog_acquisition_spec(actor, &frozen, id)?;
        self.execute_command(
            actor,
            context,
            "catalog.acquire.enqueue",
            &frozen,
            |effects| effects.enqueue_job(spec),
        )
    }

    fn catalog_acquisition_spec(
        &self,
        actor: &Principal,
        frozen: &FrozenAcquisitionJob,
        id: JobId,
    ) -> Result<JobSpec> {
        frozen.request.validate()?;
        let eligibility = self.content_write_eligibility(actor, frozen.request.collection_id)?;
        self.validate_frozen_binding(&eligibility, &frozen.collection_binding)?;
        if let Some(item_id) = frozen.item_id {
            let item = self.reader.item(item_id)?;
            if item.deleted || item.collection_id != frozen.request.collection_id {
                return Err(Error::Invalid(
                    "catalog acquisition item does not belong to its collection".into(),
                ));
            }
        }
        let input = FrozenJobInput::Acquisition(frozen.clone());
        input.validate()?;
        let spec = JobSpec {
            id,
            user_id: actor.user_id,
            collection_id: Some(frozen.request.collection_id),
            item_id: frozen.item_id,
            kind: "catalog_acquisition".into(),
            phase: "queued".into(),
            total_units: frozen.request.maximum_response_bytes,
            request: serde_json::json!({
                "provider": frozen.request.provider,
                "snapshot_id": frozen.snapshot_id,
            }),
            frozen_input: Some(input),
        };
        Ok(spec)
    }

    pub fn enqueue_accuraterip_acquisition(
        &self,
        actor: &Principal,
        capture_id: CaptureId,
        command: AcquireAccurateRipCommand,
        context: &CommandContext,
    ) -> Result<JobId> {
        let capture = self.reader.capture(capture_id)?;
        let eligibility = self.content_write_eligibility(actor, capture.collection_id)?;
        if eligibility.owner.0 != "org.junk.music"
            || !capture
                .representations
                .iter()
                .any(|value| value.id == command.representation_id)
        {
            return Err(Error::NotFound);
        }
        let structure = self
            .processing
            .processing_record(command.structure_observation_id)?
            .observation;
        if structure.collection_id != capture.collection_id
            || structure.operation.as_str() != collector_modules::music::CD_STRUCTURE_OPERATION
            || structure.operation_version != eligibility.schema_version
            || structure.typed_result.module != eligibility.owner
            || structure.typed_result.schema_version != eligibility.schema_version
            || structure.source_fingerprint.source
                != PackageScope::Representation(command.representation_id)
            || !matches!(structure.support, OperationSupport::Supported)
        {
            return Err(Error::Invalid(
                "AccurateRip acquisition requires structural evidence for this representation"
                    .into(),
            ));
        }
        let inventory = self.processing.package_inventory(
            capture.collection_id,
            PackageScope::Representation(command.representation_id),
        )?;
        structure
            .source_fingerprint
            .validate_inventory(&inventory)?;
        if self.fingerprint_inventory(&inventory)? != structure.source_fingerprint {
            return Err(Error::Conflict);
        }
        let result: collector_modules::music::CdProcessingResult =
            structure.typed_result.payload.parse()?;
        let query = collector_modules::music::accuraterip_query_from_structure(&result)
            .map_err(|error| Error::Invalid(error.to_string()))?;
        let job = deterministic_job_id(actor.user_id, "catalog.acquire", context.key());
        let frozen = FrozenAcquisitionJob {
            request: ProviderRequest {
                collection_id: capture.collection_id,
                provider: ModuleId("org.junk.providers.accuraterip".into()),
                operation: OperationId::new("org.junk.providers.accuraterip.acquire")?,
                source_fingerprint: Some(structure.source_fingerprint),
                query: RawJson::from_value(
                    serde_json::to_value(query)
                        .map_err(|error| Error::Invalid(error.to_string()))?,
                ),
                maximum_response_bytes: 8 * 1024 * 1024,
            },
            collection_binding: eligibility.binding,
            item_id: None,
            snapshot_id: SnapshotId(job.0),
            created_at: self.authentication.now(),
        };
        self.enqueue_catalog_acquisition(actor, frozen, context)
    }

    pub fn import_catalog_snapshot(
        &self,
        actor: &Principal,
        command: ImportCatalogSnapshotCommand,
        context: &CommandContext,
    ) -> Result<JobId> {
        self.require_administrator(actor)?;
        command.validate()?;
        if command.provider.0 != "org.junk.providers.local-dat" {
            return Err(Error::Invalid(
                "the configured DAT import route requires the local DAT provider".into(),
            ));
        }
        let eligibility = self.content_write_eligibility(actor, command.collection_id)?;
        if eligibility.owner.0 != "org.junk.games" {
            return Err(Error::Invalid(
                "DAT catalogs can only be imported for a Games collection".into(),
            ));
        }
        let job = deterministic_job_id(actor.user_id, "catalog.acquire", context.key());
        let frozen = FrozenAcquisitionJob {
            request: ProviderRequest {
                collection_id: command.collection_id,
                provider: command.provider,
                operation: OperationId::new("org.junk.providers.local-dat.acquire")?,
                source_fingerprint: None,
                query: RawJson::from_value(serde_json::json!({"source":command.source})),
                maximum_response_bytes: 512 * 1024 * 1024,
            },
            collection_binding: eligibility.binding,
            item_id: None,
            snapshot_id: SnapshotId(job.0),
            created_at: self.authentication.now(),
        };
        self.enqueue_catalog_acquisition(actor, frozen, context)
    }

    pub fn catalog_snapshots(
        &self,
        actor: &Principal,
        collection_id: CollectionId,
        after: Option<SnapshotId>,
        limit: u16,
    ) -> Result<Page<CatalogSnapshotStatus>> {
        self.require_administrator(actor)?;
        self.authorize(actor, collection_id, Action::Curate)?;
        self.catalog_snapshot_page(collection_id, after, limit)
    }

    pub fn curation_catalog_snapshots(
        &self,
        actor: &Principal,
        collection_id: CollectionId,
        after: Option<SnapshotId>,
        limit: u16,
    ) -> Result<Page<CatalogSnapshotStatus>> {
        self.authorize(actor, collection_id, Action::Curate)?;
        self.catalog_snapshot_page(collection_id, after, limit)
    }

    fn catalog_snapshot_page(
        &self,
        collection_id: CollectionId,
        after: Option<SnapshotId>,
        limit: u16,
    ) -> Result<Page<CatalogSnapshotStatus>> {
        if !(1..=200).contains(&limit) {
            return Err(Error::Invalid("snapshot page limit must be 1..=200".into()));
        }
        let page = self.processing.snapshots(collection_id, after, limit)?;
        let mut items = Vec::with_capacity(page.items.len());
        for snapshot in page.items {
            let active = self
                .processing
                .active_snapshot(collection_id, &snapshot.provider)?
                == Some(snapshot.id);
            items.push(CatalogSnapshotStatus { snapshot, active });
        }
        Ok(Page {
            items,
            next_cursor: page.next_cursor,
        })
    }

    pub fn activate_catalog_snapshot(
        &self,
        actor: &Principal,
        collection_id: CollectionId,
        snapshot_id: SnapshotId,
        context: &CommandContext,
    ) -> Result<CatalogSnapshotStatus> {
        self.require_administrator(actor)?;
        self.content_write_eligibility(actor, collection_id)?;
        let snapshot = self.processing.snapshot(SnapshotQuery {
            collection_id,
            snapshot_id,
        })?;
        let provider = snapshot.provider.clone();
        self.execute_command(
            actor,
            context,
            "catalog.snapshot.activate",
            &(collection_id, snapshot_id),
            |effects| effects.activate_catalog_snapshot(collection_id, &provider, snapshot_id),
        )
    }

    pub fn enqueue_processing_operation(
        &self,
        actor: &Principal,
        capture_id: CaptureId,
        command: StartProcessingCommand,
        context: &CommandContext,
    ) -> Result<JobId> {
        let capture = self.reader.capture(capture_id)?;
        let eligibility = self.content_write_eligibility(actor, capture.collection_id)?;
        if !capture
            .representations
            .iter()
            .any(|value| value.id == command.representation_id)
        {
            return Err(Error::NotFound);
        }
        let registered_version =
            self.validate_operation_eligibility(&eligibility, &command.operation)?;
        if command.operation_version != registered_version {
            return Err(Error::Invalid("operation version is not registered".into()));
        }
        if let Some(snapshot_id) = command.snapshot_id {
            self.processing.snapshot(SnapshotQuery {
                collection_id: capture.collection_id,
                snapshot_id,
            })?;
        }
        let source = PackageScope::Representation(command.representation_id);
        let inventory = self
            .processing
            .package_inventory(capture.collection_id, source)?;
        let fingerprint = self.fingerprint_inventory(&inventory)?;
        let id = deterministic_job_id(actor.user_id, "processing.operation", context.key());
        let frozen = FrozenProcessingJob {
            collection_id: capture.collection_id,
            collection_binding: eligibility.binding,
            operation: command.operation.clone(),
            operation_version: command.operation_version,
            source,
            source_fingerprint: fingerprint,
            snapshot_id: command.snapshot_id,
            observation_id: ObservationId(id.0),
            selection: command.selection.clone(),
            created_at: self.authentication.now(),
        };
        frozen.validate()?;
        let spec = JobSpec {
            id,
            user_id: actor.user_id,
            collection_id: Some(capture.collection_id),
            item_id: capture.association.item_id,
            kind: "processing_operation".into(),
            phase: "queued".into(),
            total_units: inventory.files.iter().map(|file| file.bytes.0).sum(),
            request: serde_json::json!({"capture_id":capture_id,"observation_id":frozen.observation_id}),
            frozen_input: Some(FrozenJobInput::Processing(frozen)),
        };
        self.execute_command(
            actor,
            context,
            "processing.operation.enqueue",
            &(capture_id, command),
            |effects| effects.enqueue_job(spec),
        )
    }

    pub fn run_next_job(&self, worker_instance: &str) -> Result<bool> {
        let Some(claim) = self.jobs.claim_job(worker_instance, 30)? else {
            return Ok(false);
        };
        self.jobs.heartbeat_job(
            claim.id,
            &claim.claim_token,
            &claim.phase,
            claim.completed_units,
            30,
        )?;
        let (stop_heartbeat, heartbeat_stop) = std::sync::mpsc::channel();
        let heartbeat_jobs = self.jobs.clone();
        let heartbeat_id = claim.id;
        let heartbeat_token = claim.claim_token.clone();
        let heartbeat = std::thread::Builder::new()
            .name(format!("collector-job-heartbeat-{heartbeat_id}"))
            .spawn(move || {
                loop {
                    match heartbeat_stop.recv_timeout(std::time::Duration::from_secs(5)) {
                        Ok(()) | Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                            if heartbeat_jobs
                                .renew_job_lease(heartbeat_id, &heartbeat_token, 30)
                                .is_err()
                            {
                                break;
                            }
                        }
                    }
                }
            })
            .map_err(|error| Error::Unavailable(error.to_string()))?;
        let heartbeat = JobHeartbeat {
            stop: Some(stop_heartbeat),
            thread: Some(heartbeat),
        };
        let guided_import_id = claim
            .request
            .get("import_id")
            .and_then(serde_json::Value::as_str)
            .and_then(|value| value.parse::<ImportId>().ok())
            .filter(|id| {
                self.provisional_imports
                    .guided_ingest_intent(*id)
                    .ok()
                    .flatten()
                    .is_some()
            });
        let result = match claim.kind.as_str() {
            "integrity_verification" => (|| {
                if claim.phase == "revision_committed" {
                    let item = self.reader.item(claim.item_id.ok_or_else(|| {
                        Error::Storage("integrity job has no Item identity".into())
                    })?)?;
                    return Ok(serde_json::json!({"item_id":item.id,"revision":item.revision}));
                }
                self.content_write_eligibility_for_job(
                    claim.user_id,
                    claim.collection_id,
                    claim.item_id,
                )?;
                let item_id = claim
                    .item_id
                    .ok_or_else(|| Error::Storage("integrity job has no Item identity".into()))?;
                let progress_jobs = self.jobs.clone();
                let progress_token = claim.claim_token.clone();
                let mut progress = |completed_units| {
                    self.content_write_eligibility_for_job(
                        claim.user_id,
                        claim.collection_id,
                        claim.item_id,
                    )?;
                    if progress_jobs.cancellation_requested(claim.id, &progress_token)? {
                        return Err(Error::JobCancelled);
                    }
                    progress_jobs.heartbeat_job(
                        claim.id,
                        &progress_token,
                        "verifying_integrity",
                        completed_units,
                        30,
                    )
                };
                self.imports
                    .verify_integrity_for_job(
                        item_id,
                        claim.id,
                        &claim.claim_token,
                        claim.created_at,
                        &mut progress,
                    )
                    .map(|item| serde_json::json!({"item_id":item.id,"revision":item.revision}))
            })(),
            "preservation_import" => (|| {
                if claim.phase != "revision_committed" {
                    self.content_write_eligibility_for_job(
                        claim.user_id,
                        claim.collection_id,
                        claim.item_id,
                    )?;
                }
                self.imports
                    .commit_upload_for_job(claim.id, &claim.claim_token, &mut || {
                        self.content_write_eligibility_for_job(
                            claim.user_id,
                            claim.collection_id,
                            claim.item_id,
                        )
                        .map(|_| ())
                    })
                    .map(|item| serde_json::json!({"item_id":item.id,"revision":item.revision}))
            })(),
            "import_staging" => (|| {
                self.content_write_eligibility_for_job(
                    claim.user_id,
                    claim.collection_id,
                    claim.item_id,
                )?;
                let import_id = claim
                    .request
                    .get("import_id")
                    .and_then(serde_json::Value::as_str)
                    .ok_or_else(|| Error::Storage("staging job has no Import identity".into()))?
                    .parse()
                    .map_err(|error| Error::Storage(format!("invalid Import identity: {error}")))?;
                let source_locator = claim
                    .request
                    .get("source_locator")
                    .and_then(serde_json::Value::as_str)
                    .ok_or_else(|| Error::Storage("staging job has no source locator".into()))?;
                let record = self.provisional_imports.import_record(import_id)?;
                if claim.collection_id != Some(record.view.collection_id) {
                    return Err(Error::Conflict);
                }
                self.provisional_imports
                    .stage_inbox_import(import_id, claim.id, &claim.claim_token, source_locator)
                    .map(|record| serde_json::json!({"import_id":record.view.id}))
            })(),
            "import_analysis" => self
                .analyze_import_for_job(&claim)
                .map(|view| serde_json::json!({"import_id":view.id,"revision":view.revision})),
            "import_publication" => (|| {
                let import_id = claim
                    .request
                    .get("import_id")
                    .and_then(serde_json::Value::as_str)
                    .ok_or_else(|| Error::Storage("publication job has no Import identity".into()))?
                    .parse()
                    .map_err(|error| Error::Storage(format!("invalid Import identity: {error}")))?;
                if claim.phase != "revision_committed" {
                    let record = self.provisional_imports.import_record(import_id)?;
                    let collection_id = record.view.collection_id;
                    if claim.collection_id != Some(collection_id) {
                        return Err(Error::Conflict);
                    }
                    if !self
                        .reader
                        .role(claim.user_id, collection_id)?
                        .allows(Action::Curate)
                    {
                        return Err(Error::Forbidden);
                    }
                    let eligibility = self.content_write_eligibility_for_job(
                        claim.user_id,
                        claim.collection_id,
                        record.view.association.item_id,
                    )?;
                    if let Some(ImportReviewDecision {
                        identity: ImportIdentityDecision::Candidate { key },
                        ..
                    }) = &record.view.review
                    {
                        let candidate = record
                            .view
                            .candidates
                            .iter()
                            .find(|candidate| &candidate.key == key)
                            .ok_or_else(|| {
                                Error::Invalid("selected import candidate is unavailable".into())
                            })?;
                        if candidate.module != eligibility.owner {
                            return Err(Error::Invalid(
                                "selected import candidate conflicts with the collection owner"
                                    .into(),
                            ));
                        }
                    }
                }
                self.provisional_imports
                    .publish_import_for_job(import_id, claim.id, &claim.claim_token, &mut || {
                        let record = self.provisional_imports.import_record(import_id)?;
                        self.content_write_eligibility_for_job(
                            claim.user_id,
                            claim.collection_id,
                            record.view.association.item_id,
                        )
                        .map(|_| ())
                    })
                    .map(|record| {
                        serde_json::json!({
                            "import_id":record.view.id,
                            "capture_id":record.view.capture_id,
                            "representation_id":record.view.representation_id
                        })
                    })
            })(),
            "catalog_acquisition" => (|| {
                let frozen = match claim.frozen_input.clone() {
                    Some(FrozenJobInput::Acquisition(value)) => value,
                    _ => return Err(Error::Storage("acquisition job has no frozen input".into())),
                };
                if let Ok(existing) = self.processing.snapshot(SnapshotQuery {
                    collection_id: frozen.request.collection_id,
                    snapshot_id: frozen.snapshot_id,
                }) {
                    return Ok(serde_json::json!({"snapshot_id":existing.id}));
                }
                FrozenJobInput::Acquisition(frozen.clone()).validate()?;
                let eligibility = self.content_write_eligibility_for_job(
                    claim.user_id,
                    claim.collection_id,
                    claim.item_id,
                )?;
                self.validate_frozen_binding(&eligibility, &frozen.collection_binding)?;
                let mut execution = DurableProcessingContext {
                    frozen: FrozenJobInput::Acquisition(frozen.clone()),
                    jobs: self.jobs.clone(),
                    id: claim.id,
                    claim_token: claim.claim_token.clone(),
                };
                if execution.cancellation_requested()? {
                    return Err(Error::JobCancelled);
                }
                let provider = self
                    .streaming_providers
                    .get(&frozen.request.provider.0)
                    .ok_or_else(|| {
                        Error::Unavailable("configured provider is unavailable".into())
                    })?;
                let mut response = provider.acquire_stream(&frozen.request, &mut execution)?;
                let snapshot = SnapshotRecord {
                    id: frozen.snapshot_id,
                    collection_id: frozen.request.collection_id,
                    provider: frozen.request.provider.clone(),
                    media_type: response.media_type,
                    bytes: response.bytes,
                    digest: response.digest,
                    parsed_observation: response.parsed_observation,
                    created_at: frozen.created_at,
                };
                snapshot.validate()?;
                if snapshot.bytes.0 > frozen.request.maximum_response_bytes {
                    return Err(Error::Invalid(
                        "provider stream exceeds its frozen request limit".into(),
                    ));
                }
                self.processing.publish_snapshot(
                    claim.id,
                    &claim.claim_token,
                    snapshot,
                    response.reader.as_mut(),
                    &mut || {
                        let current = self.content_write_eligibility_for_job(
                            claim.user_id,
                            claim.collection_id,
                            claim.item_id,
                        )?;
                        self.validate_frozen_binding(&current, &frozen.collection_binding)?;
                        execution.cancellation_requested()
                    },
                )?;
                Ok(serde_json::json!({"snapshot_id":frozen.snapshot_id}))
            })(),
            "music_derivative" => self.run_music_derivative(&claim),
            "music_derivative_batch" => self.run_music_derivative_batch(&claim),
            "processing_operation" => (|| {
                let frozen = match claim.frozen_input.clone() {
                    Some(FrozenJobInput::Processing(value)) => value,
                    _ => return Err(Error::Storage("processing job has no frozen input".into())),
                };
                if self
                    .processing
                    .processing_record(frozen.observation_id)
                    .is_ok()
                {
                    return Ok(serde_json::json!({"observation_id":frozen.observation_id}));
                }
                frozen.validate()?;
                let eligibility = self.content_write_eligibility_for_job(
                    claim.user_id,
                    claim.collection_id,
                    claim.item_id,
                )?;
                self.validate_frozen_binding(&eligibility, &frozen.collection_binding)?;
                let registered_version =
                    self.validate_operation_eligibility(&eligibility, &frozen.operation)?;
                if registered_version != frozen.operation_version {
                    return Err(Error::Conflict);
                }
                let inventory = self
                    .processing
                    .package_inventory(frozen.collection_id, frozen.source)?;
                frozen.source_fingerprint.validate_inventory(&inventory)?;
                if self.fingerprint_inventory(&inventory)? != frozen.source_fingerprint {
                    return Err(Error::Conflict);
                }
                if self
                    .jobs
                    .cancellation_requested(claim.id, &claim.claim_token)?
                {
                    return Err(Error::JobCancelled);
                }
                let host = ProcessingHost {
                    port: self.processing.as_ref(),
                    access: PackageAccess {
                        collection_id: frozen.collection_id,
                        scope: frozen.source,
                    },
                    source_fingerprint: &frozen.source_fingerprint,
                    frozen_snapshot_id: frozen.snapshot_id,
                    catalog_parsers: self.catalog_parsers.as_ref(),
                    jobs: self.jobs.clone(),
                    job_id: claim.id,
                    claim_token: claim.claim_token.clone(),
                };
                let (module, version) = self.modules.capability_owner(&frozen.operation)?;
                let operated = self.modules.operate(
                    OperationRequest {
                        capability: frozen.operation.clone(),
                        input: frozen.selection.clone(),
                        collection_id: frozen.collection_id,
                        entity: OperationEntity::Representation(match frozen.source {
                            PackageScope::Representation(id) => id,
                            _ => {
                                return Err(Error::Invalid(
                                    "durable processing operation requires a Representation".into(),
                                ));
                            }
                        }),
                        idempotency_key: claim.id.to_string(),
                    },
                    &host,
                );
                if self
                    .jobs
                    .cancellation_requested(claim.id, &claim.claim_token)?
                {
                    return Err(Error::JobCancelled);
                }
                let (support, payload, limitations) = match operated {
                    Ok(output) => (OperationSupport::Supported, output.value, vec![]),
                    Err(ModuleError::UnavailableCapability(reason)) => (
                        OperationSupport::UnavailableDependency {
                            dependency: module.0.clone(),
                            reason: reason.clone(),
                        },
                        RawJson::from_value(serde_json::json!({"unavailable":reason})),
                        vec![reason],
                    ),
                    Err(error) => return Err(error.into()),
                };
                if self
                    .jobs
                    .cancellation_requested(claim.id, &claim.claim_token)?
                {
                    return Err(Error::JobCancelled);
                }
                let current = self.content_write_eligibility_for_job(
                    claim.user_id,
                    claim.collection_id,
                    claim.item_id,
                )?;
                self.validate_frozen_binding(&current, &frozen.collection_binding)?;
                self.validate_operation_eligibility(&current, &frozen.operation)?;
                if self.fingerprint_inventory(&inventory)? != frozen.source_fingerprint {
                    return Err(Error::Conflict);
                }
                let observation = ProcessingObservation {
                    id: frozen.observation_id,
                    collection_id: frozen.collection_id,
                    operation: frozen.operation,
                    operation_version: frozen.operation_version,
                    source_fingerprint: frozen.source_fingerprint,
                    snapshot_id: frozen.snapshot_id,
                    support,
                    raw_observations: vec![],
                    typed_result: ModuleData {
                        module,
                        schema_version: version,
                        payload,
                    },
                    limitations,
                    created_at: frozen.created_at,
                };
                observation.validate()?;
                self.processing.publish_processing_record(
                    claim.id,
                    &claim.claim_token,
                    PortableProcessingRecord {
                        observation,
                        import_rebinding: None,
                    },
                )?;
                Ok(serde_json::json!({"observation_id":frozen.observation_id}))
            })(),
            _ => Err(Error::Unavailable(format!(
                "job kind {} has no registered executor",
                claim.kind
            ))),
        };
        heartbeat.stop()?;
        match result {
            Ok(result) => {
                self.jobs
                    .complete_job(claim.id, &claim.claim_token, result)?;
            }
            Err(Error::JobCancelled) => {
                if claim.kind.starts_with("import_")
                    && let Some(import_id) = claim
                        .request
                        .get("import_id")
                        .and_then(serde_json::Value::as_str)
                        .and_then(|value| value.parse().ok())
                {
                    let _ = self.provisional_imports.cancel_import(import_id);
                }
                self.jobs.cancel_claimed_job(claim.id, &claim.claim_token)?
            }
            Err(error) => {
                if claim.kind == "music_derivative"
                    && self.jobs.job_record(claim.id, claim.user_id)?.phase == "revision_committed"
                {
                    // Keep committed work recoverable; it must never become a fresh encoding attempt.
                    return Err(error);
                }
                if claim.kind == "preservation_import" {
                    let _ = self.imports.fail_upload(claim.id, &error.to_string());
                }
                self.jobs.fail_job(
                    claim.id,
                    &claim.claim_token,
                    &error.to_string().chars().take(512).collect::<String>(),
                )?;
            }
        }
        if let Some(id) = guided_import_id {
            self.reconcile_guided_ingest(id)?;
        }
        Ok(true)
    }
    pub fn jobs(&self, actor: &Principal) -> Result<Vec<Job>> {
        let jobs = self.imports.jobs(actor.user_id)?;
        if jobs.iter().any(|job| job.user_id != actor.user_id) {
            return Err(Error::Storage(
                "job adapter returned another user's private record".into(),
            ));
        }
        Ok(jobs)
    }
    pub fn job(&self, actor: &Principal, id: JobId) -> Result<JobView> {
        self.jobs.job_record(id, actor.user_id).map(Into::into)
    }
    pub fn job_page(
        &self,
        actor: &Principal,
        cursor: Option<JobId>,
        limit: Option<u16>,
    ) -> Result<Page<JobView>> {
        let page = self
            .jobs
            .job_records(actor.user_id, cursor, limit.unwrap_or(50).min(100))?;
        Ok(Page {
            items: page.items.into_iter().map(Into::into).collect(),
            next_cursor: page.next_cursor,
        })
    }
    pub fn cancel_job(&self, actor: &Principal, id: JobId, context: &CommandContext) -> Result<()> {
        let job = self.jobs.job_record(id, actor.user_id)?;
        self.execute_command(actor, context, "job.cancel", &id, |effects| {
            if job.kind == "music_derivative_batch" {
                effects.cancel_preparation_batch(id, actor.user_id)
            } else {
                effects.request_job_cancellation(id, actor.user_id)
            }
        })
    }
    pub fn retry_job(
        &self,
        actor: &Principal,
        id: JobId,
        context: &CommandContext,
    ) -> Result<JobId> {
        let job = self.jobs.job_record(id, actor.user_id)?;
        if matches!(
            job.kind.as_str(),
            "preservation_import"
                | "integrity_verification"
                | "import_staging"
                | "import_analysis"
                | "import_publication"
                | "catalog_acquisition"
                | "processing_operation"
                | "music_derivative"
                | "music_derivative_batch"
        ) {
            self.content_write_eligibility_for_job(actor.user_id, job.collection_id, job.item_id)?;
        }
        self.execute_command(actor, context, "job.retry", &id, |effects| {
            if job.state != JobState::Failed {
                return Err(Error::Conflict);
            }
            match job.kind.as_str() {
                "preservation_import" => effects.retry_upload(id),
                "integrity_verification"
                | "import_staging"
                | "import_analysis"
                | "import_publication"
                | "catalog_acquisition"
                | "music_derivative"
                | "processing_operation" => effects.retry_failed_job(id, actor.user_id),
                "music_derivative_batch" => effects
                    .retry_preparation_batch(id, actor.user_id)
                    .map(|()| id),
                _ => Err(Error::Unavailable("this job kind cannot be retried".into())),
            }
        })
    }
    pub fn require_administrator(&self, actor: &Principal) -> Result<()> {
        if actor.administrator {
            Ok(())
        } else {
            Err(Error::Forbidden)
        }
    }
    pub fn require_recent_authentication(
        &self,
        authenticated: &AuthenticatedPrincipal,
    ) -> Result<()> {
        authenticated.require_recent_at(self.authentication.now())
    }
    pub fn create_user(
        &self,
        authenticated: &AuthenticatedPrincipal,
        username: &str,
        password: &str,
        administrator: bool,
        context: &CommandContext,
    ) -> Result<UserId> {
        self.require_recent_authentication(authenticated)?;
        let actor = &authenticated.principal;
        self.require_administrator(actor)?;
        let request = (username, Digest::sha256(password.as_bytes()), administrator);
        self.execute_command(actor, context, "account.create", &request, |effects| {
            effects.create_user(username, password, administrator)
        })
    }
    pub fn accounts(&self, authenticated: &AuthenticatedPrincipal) -> Result<Vec<AccountView>> {
        self.require_administrator(&authenticated.principal)?;
        self.administration.accounts()
    }
    pub fn update_account(
        &self,
        authenticated: &AuthenticatedPrincipal,
        request: AccountUpdate,
        context: &CommandContext,
    ) -> Result<AccountView> {
        self.require_recent_authentication(authenticated)?;
        self.require_administrator(&authenticated.principal)?;
        self.execute_command(
            &authenticated.principal,
            context,
            "account.update",
            &request,
            |effects| {
                effects.update_account(
                    request.user,
                    request.expected_revision,
                    &request.display_name,
                    request.enabled,
                    request.administrator,
                )
            },
        )
    }
    pub fn update_profile(
        &self,
        authenticated: &AuthenticatedPrincipal,
        expected: Revision,
        display_name: &str,
        context: &CommandContext,
    ) -> Result<AccountView> {
        self.require_recent_authentication(authenticated)?;
        let current = self
            .administration
            .account(authenticated.principal.user_id)?;
        let request = (expected, display_name);
        self.execute_command(
            &authenticated.principal,
            context,
            "profile.update",
            &request,
            |effects| {
                effects.update_account(
                    current.id,
                    expected,
                    display_name,
                    current.enabled,
                    current.administrator,
                )
            },
        )
    }
    pub fn profile(&self, authenticated: &AuthenticatedPrincipal) -> Result<AccountView> {
        self.administration.account(authenticated.principal.user_id)
    }
    pub fn reassign_default(
        &self,
        authenticated: &AuthenticatedPrincipal,
        user: UserId,
        context: &CommandContext,
    ) -> Result<()> {
        self.require_recent_authentication(authenticated)?;
        self.require_administrator(&authenticated.principal)?;
        self.execute_command(
            &authenticated.principal,
            context,
            "account.default.reassign",
            &user,
            |effects| effects.reassign_default(user),
        )
    }
    pub fn reindex(
        &self,
        authenticated: &AuthenticatedPrincipal,
        context: &CommandContext,
    ) -> Result<()> {
        self.require_recent_authentication(authenticated)?;
        let actor = &authenticated.principal;
        self.require_administrator(actor)?;
        self.execute_command(actor, context, "projection.reindex", &(), |effects| {
            effects.reindex()
        })
    }
    pub fn login(
        &self,
        username: &str,
        password: &str,
        label: &str,
    ) -> Result<(String, AuthenticatedPrincipal)> {
        self.authentication.login(username, password, label)
    }
    pub fn authenticate(&self, token: &str) -> Result<AuthenticatedPrincipal> {
        self.authentication.authenticate(token)
    }
    pub fn authenticate_native(&self, credential: &str) -> Result<AuthenticatedPrincipal> {
        self.authentication.authenticate_native(credential)
    }
    pub fn revoke_session(
        &self,
        authenticated: &AuthenticatedPrincipal,
        device: DeviceId,
        context: &CommandContext,
    ) -> Result<()> {
        self.require_recent_authentication(authenticated)?;
        let actor = &authenticated.principal;
        self.execute_command(actor, context, "session.revoke", &device, |effects| {
            effects.revoke_session(actor.user_id, device)
        })
    }
    pub fn sessions(&self, authenticated: &AuthenticatedPrincipal) -> Result<Vec<SessionView>> {
        self.authentication.sessions(
            authenticated.principal.user_id,
            authenticated.principal.device_id,
        )
    }
    pub fn replace_recovery_codes(
        &self,
        authenticated: &AuthenticatedPrincipal,
    ) -> Result<Vec<String>> {
        self.require_recent_authentication(authenticated)?;
        self.authentication
            .replace_recovery_codes(authenticated.principal.user_id)
    }
    pub fn recover_account(
        &self,
        username: &str,
        code: &str,
        new_password: &str,
        label: &str,
    ) -> Result<(String, AuthenticatedPrincipal)> {
        self.authentication
            .consume_recovery_code(username, code, new_password, label)
    }
    pub fn start_enrollment(
        &self,
        username: &str,
        label: &str,
        verification_url: &str,
    ) -> Result<EnrollmentStart> {
        let user = self.authentication.user_by_name(username)?;
        self.enrollments
            .start_enrollment(user, label, verification_url)
    }
    pub fn approve_enrollment(
        &self,
        authenticated: &AuthenticatedPrincipal,
        user_code: &str,
    ) -> Result<EnrollmentView> {
        self.require_recent_authentication(authenticated)?;
        self.enrollments
            .approve_enrollment(authenticated.principal.user_id, user_code)
    }
    pub fn cancel_enrollment(
        &self,
        authenticated: &AuthenticatedPrincipal,
        enrollment: EnrollmentId,
    ) -> Result<()> {
        self.enrollments
            .cancel_enrollment(authenticated.principal.user_id, enrollment)
    }
    pub fn complete_enrollment(
        &self,
        enrollment: EnrollmentId,
        completion_secret: &str,
    ) -> Result<NativeCredentialIssue> {
        self.enrollments
            .complete_enrollment(enrollment, completion_secret)
    }
    pub fn native_devices(
        &self,
        authenticated: &AuthenticatedPrincipal,
    ) -> Result<Vec<NativeDeviceView>> {
        self.enrollments
            .native_devices(authenticated.principal.user_id)
    }
    pub fn revoke_native_device(
        &self,
        authenticated: &AuthenticatedPrincipal,
        credential: NativeCredentialId,
        expected: Revision,
        context: &CommandContext,
    ) -> Result<()> {
        self.require_recent_authentication(authenticated)?;
        let actor = &authenticated.principal;
        self.execute_command(
            actor,
            context,
            "native_credential.revoke",
            &(credential, expected),
            |effects| effects.revoke_native_device(actor.user_id, credential, expected),
        )
    }
    pub fn logout_session(&self, actor: &Principal, device: DeviceId) -> Result<()> {
        self.authentication.revoke_session(actor.user_id, device)
    }
    pub fn username(&self, actor: &Principal) -> Result<String> {
        self.authentication.username(actor.user_id)
    }
    pub fn user_by_name_for_authentication(&self, username: &str) -> Result<UserId> {
        self.authentication.user_by_name(username)
    }
    pub fn passkeys(&self, user: UserId) -> Result<Vec<StoredPasskey>> {
        self.authentication.passkeys(user)
    }
    pub fn register_passkey(
        &self,
        authenticated: &AuthenticatedPrincipal,
        credential_id: &str,
        credential: serde_json::Value,
    ) -> Result<()> {
        self.require_recent_authentication(authenticated)?;
        self.authentication.insert_passkey(
            authenticated.principal.user_id,
            credential_id,
            credential,
        )
    }
    pub fn complete_verified_passkey_authentication(
        &self,
        user: UserId,
        passkey: PasskeyId,
        expected_revision: Revision,
        expected_record: serde_json::Value,
        credential: serde_json::Value,
        label: &str,
    ) -> Result<(String, AuthenticatedPrincipal)> {
        self.authentication.complete_passkey_authentication(
            user,
            passkey,
            expected_revision,
            expected_record,
            credential,
            label,
        )
    }
    pub fn revoke_passkey(
        &self,
        authenticated: &AuthenticatedPrincipal,
        passkey: PasskeyId,
        expected: Revision,
        context: &CommandContext,
    ) -> Result<()> {
        self.require_recent_authentication(authenticated)?;
        let actor = &authenticated.principal;
        self.execute_command(
            actor,
            context,
            "passkey.revoke",
            &(passkey, expected),
            |effects| effects.revoke_passkey(actor.user_id, passkey, expected),
        )
    }
    pub fn grant(
        &self,
        authenticated: &AuthenticatedPrincipal,
        collection: CollectionId,
        user: UserId,
        role: Role,
        context: &CommandContext,
    ) -> Result<()> {
        self.require_recent_authentication(authenticated)?;
        let actor = &authenticated.principal;
        self.authorize(actor, collection, Action::ManageAccess)?;
        let current = self.access.grant_for_user(collection, user)?;
        let expected = current.map_or(Revision(0), |grant| grant.revision);
        let request = (collection, user, role);
        self.execute_command(actor, context, "grant.set", &request, |effects| {
            effects.set_grant(collection, user, expected, Some(role))
        })
        .map(|_| ())
    }
    pub fn grants(&self, actor: &Principal, collection: CollectionId) -> Result<Vec<GrantView>> {
        self.authorize(actor, collection, Action::ManageAccess)?;
        self.access.grants(collection)
    }
    pub fn set_grant(
        &self,
        authenticated: &AuthenticatedPrincipal,
        collection: CollectionId,
        username: &str,
        expected: Revision,
        role: Option<Role>,
        context: &CommandContext,
    ) -> Result<Option<GrantView>> {
        self.require_recent_authentication(authenticated)?;
        let actor = &authenticated.principal;
        self.authorize(actor, collection, Action::ManageAccess)?;
        let user = self.authentication.user_by_name(username)?;
        let request = (collection, user, expected, role);
        self.execute_command(actor, context, "grant.update", &request, |effects| {
            effects.set_grant(collection, user, expected, role)
        })
    }
}

fn deterministic_collection_id(user: UserId, key: &str) -> CollectionId {
    use sha2::{Digest as _, Sha256};
    let mut bytes: [u8; 16] =
        Sha256::digest(format!("junk-collector:collection-create:{user}:{key}"))[..16]
            .try_into()
            .expect("SHA-256 prefix has fixed length");
    bytes[6] = (bytes[6] & 0x0f) | 0x50;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    CollectionId(uuid::Uuid::from_bytes(bytes))
}

fn deterministic_inventory_uuid(user: UserId, kind: &str, key: &str) -> uuid::Uuid {
    use sha2::{Digest as _, Sha256};
    let mut bytes: [u8; 16] = Sha256::digest(format!("junk-collector:{kind}:{user}:{key}"))[..16]
        .try_into()
        .expect("SHA-256 prefix has fixed length");
    bytes[6] = (bytes[6] & 0x0f) | 0x50;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    uuid::Uuid::from_bytes(bytes)
}

fn deterministic_specimen_id(user: UserId, key: &str) -> SpecimenId {
    SpecimenId(deterministic_inventory_uuid(user, "specimen-create", key))
}

fn deterministic_component_id(user: UserId, key: &str) -> ComponentId {
    ComponentId(deterministic_inventory_uuid(user, "component-create", key))
}

fn deterministic_carrier_id(user: UserId, key: &str) -> CarrierId {
    CarrierId(deterministic_inventory_uuid(user, "carrier-create", key))
}

fn deterministic_loan_id(user: UserId, key: &str) -> LoanId {
    LoanId(deterministic_inventory_uuid(user, "loan-start", key))
}

fn validate_bounded_text(value: &str, label: &str, required: bool) -> Result<()> {
    if (required && value.trim().is_empty()) || value.len() > 4096 {
        return Err(Error::Invalid(format!(
            "{label} {} and must be at most 4096 bytes",
            if required {
                "is required"
            } else {
                "is optional"
            }
        )));
    }
    Ok(())
}

fn validate_specimen_details(details: &SpecimenDetails) -> Result<()> {
    validate_bounded_text(&details.label, "specimen label", true)?;
    if let Some(notes) = &details.private_notes {
        validate_bounded_text(notes, "private notes", false)?;
    }
    if let Some(acquisition) = &details.acquisition {
        if let Some(source) = &acquisition.source {
            validate_bounded_text(source, "acquisition source", false)?;
        }
        if let Some(cost) = &acquisition.cost {
            cost.validate()?;
        }
    }
    Ok(())
}

fn validate_component_details(details: &ComponentDetails) -> Result<()> {
    validate_bounded_text(&details.label, "component label", true)?;
    if let Some(carrier) = &details.carrier {
        validate_bounded_text(&carrier.label, "carrier label", true)?;
        if !details.kind.data_bearing() {
            return Err(Error::Invalid(
                "a non-data-bearing component cannot have a carrier".into(),
            ));
        }
        if matches!(carrier.kind, CarrierKind::Case | CarrierKind::Booklet) {
            return Err(Error::Invalid(
                "cases and booklets cannot have carrier identity".into(),
            ));
        }
    }
    Ok(())
}

fn specimen_mut(item: &mut Item, specimen_id: SpecimenId) -> Result<&mut Specimen> {
    item.specimens
        .iter_mut()
        .find(|specimen| specimen.id == specimen_id)
        .ok_or(Error::NotFound)
}

fn deterministic_asset_id(user: UserId, key: &str) -> AssetId {
    use sha2::{Digest as _, Sha256};
    let mut bytes: [u8; 16] = Sha256::digest(format!("junk-collector:cover-publish:{user}:{key}"))
        [..16]
        .try_into()
        .expect("SHA-256 prefix has fixed length");
    bytes[6] = (bytes[6] & 0x0f) | 0x50;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    AssetId(uuid::Uuid::from_bytes(bytes))
}

fn deterministic_new_asset_id(user: UserId, key: &str) -> AssetId {
    AssetId(deterministic_inventory_uuid(user, "asset-create", key))
}

fn validate_asset_upload(kind: &AssetKind, alt_text: &str, bytes: &[u8]) -> Result<()> {
    match kind {
        AssetKind::Cover | AssetKind::Photo | AssetKind::Scan => {}
        AssetKind::Document => {
            return Err(Error::Unavailable(
                "document uploads are not implemented in Wave 3".into(),
            ));
        }
        AssetKind::Other => {
            return Err(Error::Invalid(
                "asset kind must be cover, photo, or scan".into(),
            ));
        }
    }
    if bytes.len() > 20 * 1024 * 1024 || alt_text.len() > 4096 {
        return Err(Error::Invalid("image exceeds upload limit".into()));
    }
    Ok(())
}

fn deterministic_job_id(user: UserId, operation: &str, key: &str) -> JobId {
    use sha2::{Digest as _, Sha256};
    let mut bytes: [u8; 16] = Sha256::digest(format!("{user}:{operation}:{key}").as_bytes())[..16]
        .try_into()
        .expect("SHA-256 prefix");
    bytes[6] = (bytes[6] & 0x0f) | 0x50;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    JobId(uuid::Uuid::from_bytes(bytes))
}
