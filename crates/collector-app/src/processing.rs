use chrono::{DateTime, Utc};
use collector_core::{
    ByteLength, CollectionBinding, CollectionId, Digest, ImportId, ImportSourceRebinding, ModuleId,
    ObservationId, OperationId, PackageFileId, PackageInventory, PackageScope,
    ProcessingObservation, RawJson, RecipeId, SnapshotId, SourceFingerprint,
};
use serde::{Deserialize, Serialize};

use collector_core::Result;

const MAX_PARSED_OBSERVATION_BYTES: usize = 1024 * 1024;
const MAX_PROVIDER_LIMITATIONS: usize = 128;
const MAX_PROVIDER_LIMITATION_BYTES: usize = 4096;

fn legacy_unassigned_binding() -> CollectionBinding {
    CollectionBinding::LegacyUnassigned
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProcessingQuery {
    pub collection_id: CollectionId,
    pub source: PackageScope,
    pub source_fingerprint: SourceFingerprint,
    pub operation: Option<OperationId>,
    pub after: Option<ObservationId>,
    pub limit: u16,
}

impl ProcessingQuery {
    pub fn validate(&self) -> Result<()> {
        if !(1..=200).contains(&self.limit) {
            return Err(collector_core::Error::Invalid(
                "processing page limit must be 1..=200".into(),
            ));
        }
        self.source_fingerprint.validate()?;
        if self.source_fingerprint.source != self.source {
            return Err(collector_core::Error::Invalid(
                "processing query source does not match its fingerprint".into(),
            ));
        }
        Ok(())
    }
}

/// All identities and decision inputs required for one processing attempt are
/// allocated before execution. W4.2 persists this value without changing it.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrozenProcessingJob {
    pub collection_id: CollectionId,
    #[serde(default = "legacy_unassigned_binding")]
    pub collection_binding: CollectionBinding,
    pub operation: OperationId,
    pub operation_version: u32,
    pub source: PackageScope,
    pub source_fingerprint: SourceFingerprint,
    pub snapshot_id: Option<SnapshotId>,
    pub observation_id: ObservationId,
    pub selection: RawJson,
    pub created_at: DateTime<Utc>,
}

impl FrozenProcessingJob {
    pub fn validate(&self) -> Result<()> {
        self.collection_binding.validate()?;
        if !matches!(self.collection_binding, CollectionBinding::Assigned { .. }) {
            return Err(collector_core::Error::Invalid(
                "processing jobs require an assigned collection owner".into(),
            ));
        }
        if self.operation_version == 0 {
            return Err(collector_core::Error::Invalid(
                "processing operation version must be non-zero".into(),
            ));
        }
        self.source_fingerprint.validate()?;
        if self.source_fingerprint.source != self.source {
            return Err(collector_core::Error::Invalid(
                "frozen job source does not match its fingerprint".into(),
            ));
        }
        if self.selection.get().len() > 64 * 1024 {
            return Err(collector_core::Error::Invalid(
                "processing selection is oversized".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SnapshotQuery {
    pub collection_id: CollectionId,
    pub snapshot_id: SnapshotId,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SnapshotRecord {
    pub id: SnapshotId,
    pub collection_id: CollectionId,
    pub provider: ModuleId,
    pub media_type: String,
    pub bytes: ByteLength,
    pub digest: Digest,
    #[schema(value_type = Object)]
    pub parsed_observation: RawJson,
    pub created_at: DateTime<Utc>,
}

impl SnapshotRecord {
    /// Validate metadata before publishing; the writer separately hashes the
    /// streamed bytes and compares them with this digest and length.
    pub fn validate(&self) -> Result<()> {
        self.provider.validate()?;
        self.digest.validate()?;
        if self.digest.algorithm != collector_core::HashAlgorithm::Sha256
            || self.media_type.is_empty()
            || self.media_type.len() > 255
            || self.media_type.chars().any(char::is_control)
            || self.bytes.0 > 512 * 1024 * 1024
            || self.parsed_observation.get().len() > MAX_PARSED_OBSERVATION_BYTES
        {
            return Err(collector_core::Error::Invalid(
                "invalid snapshot metadata".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderRequest {
    pub collection_id: CollectionId,
    pub provider: ModuleId,
    pub operation: OperationId,
    pub source_fingerprint: Option<SourceFingerprint>,
    pub query: RawJson,
    pub maximum_response_bytes: u64,
}

impl ProviderRequest {
    pub fn validate(&self) -> Result<()> {
        self.provider.validate()?;
        if self.maximum_response_bytes == 0
            || self.maximum_response_bytes > 512 * 1024 * 1024
            || self.query.get().len() > 64 * 1024
        {
            return Err(collector_core::Error::Invalid(
                "provider request is invalid or oversized".into(),
            ));
        }
        if let Some(fingerprint) = &self.source_fingerprint {
            fingerprint.validate()?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderResponse {
    pub media_type: String,
    pub bytes: Vec<u8>,
    pub digest: Digest,
    pub parsed_observation: RawJson,
    pub limitations: Vec<String>,
}

impl ProviderResponse {
    pub fn validate(&self, maximum_response_bytes: u64) -> Result<()> {
        if maximum_response_bytes == 0
            || maximum_response_bytes > 8 * 1024 * 1024
            || self.media_type.is_empty()
            || self.media_type.len() > 255
            || self.media_type.chars().any(char::is_control)
            || self.bytes.len() as u64 > maximum_response_bytes
            || self.parsed_observation.get().len() > MAX_PARSED_OBSERVATION_BYTES
            || self.limitations.len() > MAX_PROVIDER_LIMITATIONS
            || self.limitations.iter().any(|limitation| {
                limitation.len() > MAX_PROVIDER_LIMITATION_BYTES
                    || limitation.chars().any(char::is_control)
            })
        {
            return Err(collector_core::Error::Invalid(
                "provider response is invalid or oversized".into(),
            ));
        }
        self.digest.validate()?;
        if self.digest != Digest::sha256(&self.bytes) {
            return Err(collector_core::Error::Invalid(
                "provider response digest mismatch".into(),
            ));
        }
        Ok(())
    }
}

pub trait ProcessingReadPort: Send + Sync {
    fn observations(
        &self,
        query: &ProcessingQuery,
    ) -> Result<collector_core::Page<collector_core::ProcessingObservation>>;
}

pub trait SnapshotPort: Send + Sync {
    fn snapshot(&self, query: SnapshotQuery) -> Result<SnapshotRecord>;
    /// The adapter enforces the same collection scope for metadata and bytes.
    /// Large DAT snapshots must not become a Vec in the shared application DTO.
    fn snapshot_bytes(&self, query: SnapshotQuery) -> Result<Box<dyn std::io::Read + Send>>;
    fn snapshots(
        &self,
        collection_id: CollectionId,
        after: Option<SnapshotId>,
        limit: u16,
    ) -> Result<collector_core::Page<SnapshotRecord>>;
    fn active_snapshot(
        &self,
        collection_id: CollectionId,
        provider: &ModuleId,
    ) -> Result<Option<SnapshotId>>;
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CatalogSnapshotStatus {
    pub snapshot: SnapshotRecord,
    pub active: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ImportCatalogSnapshotCommand {
    pub collection_id: CollectionId,
    pub provider: ModuleId,
    pub source: String,
}

impl ImportCatalogSnapshotCommand {
    pub fn validate(&self) -> Result<()> {
        self.provider.validate()?;
        if self.source.is_empty()
            || self.source.len() > 120
            || !self
                .source
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
        {
            return Err(collector_core::Error::Invalid(
                "invalid configured catalog source".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ActivateCatalogSnapshotCommand {
    pub snapshot_id: SnapshotId,
}

/// JSON/dBAR responses are capped at 8 MiB. Large catalogs use StreamingProviderPort.
pub trait ProviderPort: Send + Sync {
    fn acquire(
        &self,
        request: &ProviderRequest,
        execution: &mut dyn ProcessingExecutionContext,
    ) -> Result<ProviderResponse>;
}

pub trait ProcessingExecutionContext {
    fn frozen_job(&self) -> &FrozenJobInput;
    fn report_progress(
        &mut self,
        phase: &str,
        completed_units: u64,
        total_units: u64,
    ) -> Result<()>;
    fn cancellation_requested(&self) -> Result<bool>;
}

/// Acquisition has frozen query/output identity but need not have a file source.
/// The executor checkpoints the actual response before processing it or retrying.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrozenAcquisitionJob {
    pub request: ProviderRequest,
    #[serde(default = "legacy_unassigned_binding")]
    pub collection_binding: CollectionBinding,
    pub snapshot_id: SnapshotId,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", content = "input", rename_all = "snake_case")]
pub enum FrozenJobInput {
    Processing(FrozenProcessingJob),
    Acquisition(FrozenAcquisitionJob),
}
impl FrozenJobInput {
    pub fn validate(&self) -> Result<()> {
        match self {
            Self::Processing(job) => job.validate(),
            Self::Acquisition(job) => {
                job.collection_binding.validate()?;
                if !matches!(job.collection_binding, CollectionBinding::Assigned { .. }) {
                    return Err(collector_core::Error::Invalid(
                        "acquisition jobs require an assigned collection owner".into(),
                    ));
                }
                job.request.validate()
            }
        }
    }
}

/// Reader is bounded by the adapter to the validated request limit. The snapshot
/// writer hashes what it consumes and enforces the same limit before publishing.
/// Content is never trusted solely from provider-declared length/digest headers.
pub struct ProviderStreamResponse {
    pub media_type: String,
    pub bytes: ByteLength,
    pub digest: Digest,
    pub parsed_observation: RawJson,
    pub reader: Box<dyn std::io::Read + Send>,
}

pub trait StreamingProviderPort: Send + Sync {
    fn acquire_stream(
        &self,
        request: &ProviderRequest,
        execution: &mut dyn ProcessingExecutionContext,
    ) -> Result<ProviderStreamResponse>;
}

/// Adapt a bounded response into the single acquisition/publication pipeline.
/// This adapter owns no registry, scheduling or provider policy.
pub struct BoundedProviderAdapter(pub std::sync::Arc<dyn ProviderPort>);

impl StreamingProviderPort for BoundedProviderAdapter {
    fn acquire_stream(
        &self,
        request: &ProviderRequest,
        execution: &mut dyn ProcessingExecutionContext,
    ) -> Result<ProviderStreamResponse> {
        let response = self.0.acquire(request, execution)?;
        response.validate(request.maximum_response_bytes)?;
        Ok(ProviderStreamResponse {
            media_type: response.media_type,
            bytes: ByteLength(response.bytes.len() as u64),
            digest: response.digest,
            parsed_observation: response.parsed_observation,
            reader: Box::new(std::io::Cursor::new(response.bytes)),
        })
    }
}

#[derive(Clone, Debug)]
pub struct ParsedCatalogSnapshot {
    pub observation: RawJson,
    pub entries: Vec<RawJson>,
}

impl ParsedCatalogSnapshot {
    pub fn validate(&self) -> Result<()> {
        if self.observation.get().len() > MAX_PARSED_OBSERVATION_BYTES
            || self.entries.is_empty()
            || self.entries.len() > 1_000_000
        {
            return Err(collector_core::Error::Invalid(
                "parsed catalog snapshot is invalid or oversized".into(),
            ));
        }
        let mut bytes = 0_usize;
        for entry in &self.entries {
            bytes = bytes.saturating_add(entry.get().len());
            if entry.get().len() > 64 * 1024 || bytes > 64 * 1024 * 1024 {
                return Err(collector_core::Error::Invalid(
                    "parsed catalog index exceeds its resource bound".into(),
                ));
            }
        }
        Ok(())
    }
}

/// Rebuildable parser/index adapter for exact archived provider bytes. The
/// returned rows are neutral raw observations; module policy interprets them.
pub trait CatalogParserPort: Send + Sync {
    fn parse_snapshot(
        &self,
        record: &SnapshotRecord,
        reader: &mut dyn std::io::Read,
        cancelled: &mut dyn FnMut() -> Result<bool>,
    ) -> Result<ParsedCatalogSnapshot>;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PortableProcessingRecord {
    pub observation: ProcessingObservation,
    pub import_rebinding: Option<ImportSourceRebinding>,
}

impl PortableProcessingRecord {
    pub fn validate(&self) -> Result<()> {
        self.observation.validate()?;
        if !matches!(
            self.observation.source_fingerprint.source,
            PackageScope::Representation(_)
        ) {
            return Err(collector_core::Error::Invalid(
                "portable observations require a Representation source".into(),
            ));
        }
        if let Some(rebinding) = &self.import_rebinding {
            rebinding.validate()?;
            if self.observation.source_fingerprint != rebinding.committed_fingerprint {
                return Err(collector_core::Error::Invalid(
                    "observation does not use its committed Representation fingerprint".into(),
                ));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ProcessingSummary {
    pub observation_id: ObservationId,
    pub operation: OperationId,
    pub source: PackageScope,
    pub snapshot_id: Option<SnapshotId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<Object>)]
    pub snapshot_observation: Option<RawJson>,
    #[schema(value_type = Object)]
    pub result: RawJson,
    pub current: bool,
    pub support: collector_core::OperationSupport,
    pub limitations: Vec<String>,
    pub legacy_provenance: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct StartProcessingCommand {
    pub operation: OperationId,
    pub operation_version: u32,
    pub representation_id: collector_core::RepresentationId,
    pub snapshot_id: Option<SnapshotId>,
    #[schema(value_type = Object)]
    pub selection: RawJson,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AcquireAccurateRipCommand {
    pub representation_id: collector_core::RepresentationId,
    pub structure_observation_id: ObservationId,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetadataAuthorityRecord {
    pub collection_id: CollectionId,
    pub item_id: collector_core::ItemId,
    pub revision: collector_core::Revision,
    pub authority: collector_core::MetadataAuthority,
    pub updated_at: DateTime<Utc>,
}

impl MetadataAuthorityRecord {
    pub fn validate(&self) -> Result<()> {
        if self.revision.0 == 0 {
            return Err(collector_core::Error::Invalid(
                "metadata authority revision is required".into(),
            ));
        }
        self.authority.validate()
    }
}

/// Mechanism-only persistence and package access. Authorization and currency
/// policy remain in CollectionService.
pub trait ProcessingPort: ProcessingReadPort + SnapshotPort + Send + Sync {
    fn package_inventory(
        &self,
        collection: CollectionId,
        source: PackageScope,
    ) -> Result<PackageInventory>;
    /// Current committed file identities/digests, without reading payload bytes.
    fn recorded_source(
        &self,
        collection: CollectionId,
        source: PackageScope,
    ) -> Result<SourceFingerprint>;
    fn package_file(
        &self,
        collection: CollectionId,
        source: PackageScope,
        file: PackageFileId,
    ) -> Result<Box<dyn crate::MediaReader>>;
    fn save_provisional_observation(
        &self,
        import: ImportId,
        job: collector_core::JobId,
        record: &ProcessingObservation,
    ) -> Result<()>;
    fn provisional_observations(&self, import: ImportId) -> Result<Vec<ProcessingObservation>>;
    fn publish_processing_record(
        &self,
        job: collector_core::JobId,
        claim_token: &str,
        record: PortableProcessingRecord,
    ) -> Result<()>;
    fn publish_snapshot(
        &self,
        job: collector_core::JobId,
        claim_token: &str,
        record: SnapshotRecord,
        reader: &mut dyn std::io::Read,
        cancelled: &mut dyn FnMut() -> Result<bool>,
    ) -> Result<()>;
    fn processing_record(&self, id: ObservationId) -> Result<PortableProcessingRecord>;
    fn recipe(&self, id: RecipeId) -> Result<collector_core::ConversionRecipe>;
    fn publish_recipe(
        &self,
        job: collector_core::JobId,
        collection: CollectionId,
        recipe: collector_core::ConversionRecipe,
    ) -> Result<()>;
    fn publish_metadata_authority(
        &self,
        transaction: collector_core::TransactionId,
        record: MetadataAuthorityRecord,
    ) -> Result<()>;
}
