//! Application-owned automatic game identification and activation recovery.
use crate::*;
use collector_modules::games::{GameIdentification, identify_game};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FrozenGameIdentification {
    pub item_id: ItemId,
    pub generation: u64,
    pub processing: FrozenProcessingJob,
    pub snapshots: Vec<SnapshotId>,
}
impl FrozenGameIdentification {
    pub fn validate(&self) -> Result<()> {
        self.processing.validate()?;
        if self.processing.operation_version != 1
            || self.snapshots.len() > 128
            || self.processing.operation.as_str() != "org.junk.games.identify"
        {
            return Err(Error::Invalid("Invalid game identification inputs".into()));
        }
        Ok(())
    }
    pub fn key(&self) -> Result<String> {
        Ok(Digest::sha256(
            &serde_json::to_vec(&(
                "game-identification-v1",
                self.item_id,
                self.generation,
                &self.processing.source_fingerprint,
                &self.snapshots,
            ))
            .map_err(|e| Error::Invalid(e.to_string()))?,
        )
        .hex)
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FrozenGameCatalogScan {
    pub collection_id: CollectionId,
    pub collection_binding: CollectionBinding,
    pub activated_snapshot: SnapshotId,
}
impl FrozenGameCatalogScan {
    pub fn validate(&self) -> Result<()> {
        self.collection_binding.validate()
    }
}

struct GameHost<'a> {
    base: ProcessingHost<'a>,
    snapshots: &'a [SnapshotId],
}
impl ModuleHost for GameHost<'_> {
    fn checkpoint(&self) -> ModuleResult<()> {
        self.base.checkpoint()
    }
    fn package_inventory(&self, access: PackageAccess) -> ModuleResult<PackageInventory> {
        self.base.package_inventory(access)
    }
    fn package_file(
        &self,
        access: PackageAccess,
        id: PackageFileId,
    ) -> ModuleResult<Box<dyn RepresentationReader>> {
        self.base.package_file(access, id)
    }
    fn representation_access(&self, id: RepresentationId) -> ModuleResult<PackageAccess> {
        self.base.representation_access(id)
    }
    fn source_fingerprint(&self, access: PackageAccess) -> ModuleResult<SourceFingerprint> {
        self.base.source_fingerprint(access)
    }
    fn submit_job(&self, request: JobSubmission) -> ModuleResult<JobId> {
        self.base.submit_job(request)
    }
    fn catalog_snapshot(&self, id: SnapshotId) -> ModuleResult<CatalogSnapshotInput> {
        if !self.snapshots.contains(&id) {
            return Err(ModuleError::AccessDenied);
        }
        ProcessingHost {
            frozen_snapshot_id: Some(id),
            port: self.base.port,
            access: self.base.access,
            source_fingerprint: self.base.source_fingerprint,
            catalog_parsers: self.base.catalog_parsers,
            jobs: self.base.jobs.clone(),
            job_id: self.base.job_id,
            claim_token: self.base.claim_token.clone(),
        }
        .catalog_snapshot(id)
    }
}
impl CollectionService {
    fn game_identification_spec(
        &self,
        user: UserId,
        item: &Item,
        representation: RepresentationId,
        id: JobId,
    ) -> Result<JobSpec> {
        let eligibility = self.content_write_eligibility_for_user(user, item.collection_id)?;
        self.validate_owned_domain(&eligibility, &item.domain)?;
        if eligibility.owner.0 != "org.junk.games" || item.deleted {
            return Err(Error::Invalid(
                "Identification requires an active game entry".into(),
            ));
        }
        let source = PackageScope::Representation(representation);
        if !item
            .captures
            .iter()
            .flat_map(|c| &c.representations)
            .any(|r| r.id == representation)
        {
            return Err(Error::NotFound);
        }
        let snapshots = self.processing.active_game_snapshots(item.collection_id)?;
        let references=snapshots.iter().map(|id| self.processing.snapshot(SnapshotQuery{collection_id:item.collection_id,snapshot_id:*id}).map(|record|serde_json::json!({"id":record.id,"digest":record.digest,"bytes":record.bytes,"observation":record.parsed_observation}))).collect::<Result<Vec<_>>>()?;
        let processing = FrozenProcessingJob {
            collection_id: item.collection_id,
            collection_binding: eligibility.binding,
            operation: OperationId::new("org.junk.games.identify")?,
            operation_version: 1,
            source,
            source_fingerprint: self
                .processing
                .recorded_source(item.collection_id, source)?,
            snapshot_id: None,
            observation_id: ObservationId(id.0),
            selection: RawJson::from_value(
                serde_json::json!({"snapshot_ids":snapshots,"references":references}),
            ),
            created_at: self.authentication.now(),
        };
        let total_units = processing
            .source_fingerprint
            .files
            .iter()
            .try_fold(0u64, |total, file| {
                total.checked_add(file.bytes.0).ok_or(Error::Conflict)
            })?;
        let frozen = FrozenGameIdentification {
            item_id: item.id,
            generation: item.lifecycle.generation,
            processing,
            snapshots,
        };
        frozen.validate()?;
        Ok(JobSpec {
            id,
            user_id: user,
            collection_id: Some(item.collection_id),
            item_id: Some(item.id),
            kind: "game_identification".into(),
            phase: "queued".into(),
            total_units,
            request: serde_json::json!({"item_id":item.id,"representation_id":representation}),
            frozen_input: Some(FrozenJobInput::GameIdentification(frozen)),
        })
    }
    pub fn identify_game_entry(
        &self,
        actor: &Principal,
        item_id: ItemId,
        context: &CommandContext,
    ) -> Result<Vec<JobId>> {
        self.execute_command(actor, context, "game.identify", &item_id, |effects| {
            let item = self.reader.item(item_id)?;
            let mut specs = vec![];
            for representation in item
                .captures
                .iter()
                .flat_map(|c| &c.representations)
                .filter(|r| r.role == collector_core::RepresentationRole::PreservationMaster)
            {
                let id = deterministic_job_id(
                    actor.user_id,
                    "game.identify",
                    &format!("{}:{}", context.key(), representation.id),
                );
                specs.push(self.game_identification_spec(
                    actor.user_id,
                    &item,
                    representation.id,
                    id,
                )?);
            }
            if specs.is_empty() {
                return Err(Error::Unavailable(
                    "Attach original files before identifying this game".into(),
                ));
            }
            effects.enqueue_game_identifications(specs)
        })
    }
    pub(crate) fn advance_guided_game_identification(
        &self,
        intent: &GuidedIngestIntent,
        record: &ImportRecord,
        item: &Item,
    ) -> Result<bool> {
        if let Some(job) = self
            .jobs
            .game_import_job(intent.user_id, intent.import_id)?
        {
            return match job.state {
                JobState::Completed => Ok(true),
                JobState::Failed => Err(Error::Unavailable(
                    job.error
                        .unwrap_or_else(|| "Game identification failed".into()),
                )),
                JobState::Cancelled => Err(Error::JobCancelled),
                _ => Ok(false),
            };
        }
        let id = deterministic_job_id(
            intent.user_id,
            "guided.game.identify",
            &intent.import_id.to_string(),
        );
        let mut spec =
            self.game_identification_spec(intent.user_id, item, record.view.representation_id, id)?;
        spec.request
            .as_object_mut()
            .unwrap()
            .insert("import_id".into(), intent.import_id.to_string().into());
        spec.request
            .as_object_mut()
            .unwrap()
            .insert("guided".into(), true.into());
        self.jobs.enqueue_job(spec)?;
        Ok(false)
    }
    pub(crate) fn run_game_catalog_scan(&self, claim: &JobClaim) -> Result<serde_json::Value> {
        let Some(FrozenJobInput::GameCatalogScan(scan)) = &claim.frozen_input else {
            return Err(Error::Conflict);
        };
        let eligibility =
            self.content_write_eligibility_for_user(claim.user_id, scan.collection_id)?;
        self.validate_frozen_binding(&eligibility, &scan.collection_binding)?;
        if !self
            .processing
            .active_game_snapshots(scan.collection_id)?
            .contains(&scan.activated_snapshot)
        {
            return Err(Error::Conflict);
        }
        let progress = self
            .jobs
            .job_record(claim.id, claim.user_id)?
            .result
            .unwrap_or_default();
        let done = progress
            .get("admissions_done")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        let cursor = progress
            .get("cursor")
            .and_then(serde_json::Value::as_str)
            .map(str::parse)
            .transpose()
            .map_err(|_| Error::Conflict)?;
        if !done {
            self.report_processing_phase(claim, "identifying_existing_games")?;
            let ids = self.jobs.game_scan_page(claim, cursor)?;
            for id in &ids {
                let item = self.reader.item(*id)?;
                for representation in item
                    .captures
                    .iter()
                    .flat_map(|c| &c.representations)
                    .filter(|r| r.role == collector_core::RepresentationRole::PreservationMaster)
                {
                    self.report_processing_phase(claim, "identifying_existing_games")?;
                    let id = deterministic_job_id(
                        claim.user_id,
                        "game.catalog.member",
                        &format!("{}:{}:{}", claim.id, item.id, representation.id),
                    );
                    // Recovery never replaces the frozen coordinates of an admitted member.
                    if self
                        .jobs
                        .game_scan_member(claim.user_id, claim.id, item.id, representation.id)?
                        .is_some()
                    {
                        continue;
                    }
                    let mut spec =
                        self.game_identification_spec(claim.user_id, &item, representation.id, id)?;
                    spec.request
                        .as_object_mut()
                        .unwrap()
                        .insert("game_parent".into(), claim.id.to_string().into());
                    spec.request
                        .as_object_mut()
                        .unwrap()
                        .insert("game_parent_claim".into(), claim.claim_token.clone().into());
                    self.jobs.enqueue_job(spec)?;
                }
            }
            self.jobs.checkpoint_game_scan(
                claim,
                ids.last().copied().or(cursor),
                ids.len() < 100,
            )?;
            return Err(Error::CommandInProgress);
        }
        let status = self.jobs.game_scan_status(claim.user_id, claim.id)?;
        if status.pending > 0 {
            self.jobs.checkpoint_game_scan(claim, cursor, true)?;
            return Err(Error::CommandInProgress);
        }
        if status.failed > 0 || status.cancelled > 0 {
            return Err(Error::Unavailable(format!(
                "{} game identification jobs failed; {} were cancelled. Review and retry their jobs before retrying this scan.",
                status.failed, status.cancelled
            )));
        }
        Ok(
            serde_json::json!({"snapshot_id":scan.activated_snapshot,"completed":status.completed,"admissions_done":true,"cursor":cursor}),
        )
    }
    pub(crate) fn run_game_identification(&self, claim: &JobClaim) -> Result<serde_json::Value> {
        let Some(FrozenJobInput::GameIdentification(frozen)) = &claim.frozen_input else {
            return Err(Error::Conflict);
        };
        frozen.validate()?;
        let check = || -> Result<Item> {
            if self
                .jobs
                .cancellation_requested(claim.id, &claim.claim_token)?
            {
                return Err(Error::JobCancelled);
            }
            let item = self.reader.item(frozen.item_id)?;
            let eligibility =
                self.content_write_eligibility_for_user(claim.user_id, item.collection_id)?;
            self.validate_frozen_binding(&eligibility, &frozen.processing.collection_binding)?;
            if item.deleted
                || item.lifecycle.generation != frozen.generation
                || !item
                    .captures
                    .iter()
                    .flat_map(|c| &c.representations)
                    .any(|r| frozen.processing.source == PackageScope::Representation(r.id))
                || self
                    .processing
                    .recorded_source(item.collection_id, frozen.processing.source)?
                    != frozen.processing.source_fingerprint
            {
                return Err(Error::Conflict);
            }
            let current = self.processing.active_game_snapshots(item.collection_id)?;
            if current != frozen.snapshots {
                return Err(Error::Conflict);
            }
            Ok(item)
        };
        let item = check()?;
        let inventory = self
            .processing
            .package_inventory(item.collection_id, frozen.processing.source)?;
        self.report_processing_phase(claim, "validating_source")?;
        if self.fingerprint_inventory_checked(&inventory, &mut || check().map(|_| ()))?
            != frozen.processing.source_fingerprint
        {
            return Err(Error::Conflict);
        }
        self.report_processing_phase(claim, "identifying_game")?;
        let host = GameHost {
            base: ProcessingHost {
                port: self.processing.as_ref(),
                access: PackageAccess {
                    collection_id: item.collection_id,
                    scope: frozen.processing.source,
                },
                source_fingerprint: &frozen.processing.source_fingerprint,
                frozen_snapshot_id: None,
                catalog_parsers: self.catalog_parsers.as_ref(),
                jobs: self.jobs.clone(),
                job_id: claim.id,
                claim_token: claim.claim_token.clone(),
            },
            snapshots: &frozen.snapshots,
        };
        let result: GameIdentification =
            identify_game(&host, host.base.access, &frozen.snapshots).map_err(Error::from)?;
        self.report_processing_phase(claim, "rechecking_source")?;
        if self.fingerprint_inventory_checked(&inventory, &mut || check().map(|_| ()))?
            != frozen.processing.source_fingerprint
        {
            return Err(Error::Conflict);
        }
        let item = check()?;
        let payload = RawJson::from_value(
            serde_json::to_value(&result).map_err(|e| Error::Invalid(e.to_string()))?,
        );
        match self
            .processing
            .processing_record(frozen.processing.observation_id)
        {
            Ok(existing) => {
                if existing.observation.typed_result.payload != payload {
                    return Err(Error::Conflict);
                }
            }
            Err(Error::NotFound) => {
                self.processing.publish_processing_record(
                    claim,
                    PortableProcessingRecord {
                        inputs: Some(ScanIdentity {
                            collection_id: item.collection_id,
                            operation: frozen.processing.operation.clone(),
                            operation_version: 1,
                            source: frozen.processing.source_fingerprint.clone(),
                            selection: frozen.processing.selection.value()?,
                            reference: None,
                        }),
                        import_rebinding: None,
                        observation: ProcessingObservation {
                            id: frozen.processing.observation_id,
                            collection_id: item.collection_id,
                            operation: frozen.processing.operation.clone(),
                            operation_version: 1,
                            source_fingerprint: frozen.processing.source_fingerprint.clone(),
                            snapshot_id: None,
                            support: OperationSupport::Supported,
                            raw_observations: vec![],
                            typed_result: ModuleData {
                                module: ModuleId("org.junk.games".into()),
                                schema_version: 1,
                                payload: payload.clone(),
                            },
                            limitations: result.limitations.clone(),
                            created_at: frozen.processing.created_at,
                        },
                    },
                )?;
            }
            Err(error) => return Err(error),
        }
        check()?;
        if !self.processing.metadata_refresh_observed(
            item.collection_id,
            item.id,
            ObservationId(claim.id.0),
        )? {
            let revision = self
                .processing
                .metadata_authority(item.collection_id, item.id)?
                .map_or(Revision(0), |r| r.revision);
            let outcome = if result.candidates.is_empty() {
                MetadataRefreshOutcome::NoMatch
            } else {
                MetadataRefreshOutcome::Candidates
            };
            self.refresh_metadata_effect(
                &item,
                &ModuleId("org.junk.games".into()),
                &RefreshMetadataCommand {
                    expected_revision: revision,
                    observation: MetadataRefreshObservation {
                        id: ObservationId(claim.id.0),
                        outcome,
                        candidates: result.candidates,
                        raw_observation: payload,
                        observed_at: frozen.processing.created_at,
                    },
                },
                "en",
                Some(claim),
                self.processing.as_ref(),
            )?;
        }
        Ok(
            serde_json::json!({"item_id":item.id,"observation_id":frozen.processing.observation_id,"status":result.status}),
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CatalogUploadEntry {
    pub name: String,
    pub detection: collector_modules::games::CatalogDetection,
    pub upload: Option<UploadedDatCatalog>,
    pub corrections: Vec<UploadedDatCatalog>,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CatalogUploadPreview {
    pub expanded_bytes: ByteLength,
    pub member_count: u32,
    pub entries: Vec<CatalogUploadEntry>,
    pub ignored: Vec<String>,
}
pub trait CatalogUploadInspectionPort: Send + Sync {
    fn inspect_upload(
        &self,
        bytes: &[u8],
        uploads: &dyn UploadedCatalogPort,
        expanded_limit: u64,
        member_limit: u32,
    ) -> Result<CatalogUploadPreview>;
}
impl CollectionService {
    pub fn with_catalog_inspection(
        mut self,
        inspector: Arc<dyn CatalogUploadInspectionPort>,
    ) -> Self {
        self.catalog_inspector = Some(inspector);
        self
    }
    pub fn authorize_game_catalog_upload(
        &self,
        actor: &Principal,
        collection: CollectionId,
    ) -> Result<()> {
        self.require_administrator(actor)?;
        let eligibility = self.content_write_eligibility(actor, collection)?;
        if eligibility.owner.0 != "org.junk.games" {
            return Err(Error::Invalid("Choose a Games collection".into()));
        }
        Ok(())
    }
    pub fn inspect_catalog_batch(
        &self,
        actor: &Principal,
        collection: CollectionId,
        files: Vec<(String, Vec<u8>)>,
    ) -> Result<CatalogUploadPreview> {
        self.authorize_game_catalog_upload(actor, collection)?;
        if files.is_empty() || files.len() > 128 {
            return Err(Error::Invalid("Select 1–128 DAT/ZIP files".into()));
        }
        let mut batch = CatalogUploadPreview {
            entries: vec![],
            ignored: vec![],
            expanded_bytes: ByteLength(0),
            member_count: 0,
        };
        for (name, bytes) in files {
            if bytes.is_empty() || bytes.len() > MAX_DAT_UPLOAD_BYTES {
                return Err(Error::Invalid(
                    "Upload DAT/ZIP files up to 64 MiB each".into(),
                ));
            }
            let mut preview = self
                .catalog_inspector
                .as_ref()
                .ok_or_else(|| Error::Unavailable("Catalog inspection is unavailable".into()))?
                .inspect_upload(
                    &bytes,
                    self.catalog_uploads
                        .as_ref()
                        .ok_or_else(|| {
                            Error::Unavailable("Catalog upload staging is unavailable".into())
                        })?
                        .as_ref(),
                    512 * 1024 * 1024 - batch.expanded_bytes.0,
                    128 - batch.member_count,
                )?;
            batch.expanded_bytes.0 = batch
                .expanded_bytes
                .0
                .checked_add(preview.expanded_bytes.0)
                .ok_or(Error::Conflict)?;
            batch.member_count = batch
                .member_count
                .checked_add(preview.member_count)
                .ok_or(Error::Conflict)?;
            if batch.expanded_bytes.0 > 512 * 1024 * 1024 || batch.member_count > 128 {
                return Err(Error::Invalid(
                    "Expanded batch exceeds 512 MiB or 128 members".into(),
                ));
            }
            for entry in &mut preview.entries {
                if entry.name == "Uploaded DAT" {
                    entry.name = name.clone();
                }
            }
            batch.entries.extend(preview.entries);
            batch.ignored.extend(preview.ignored);
        }
        Ok(batch)
    }
    pub fn inspect_catalog_upload(
        &self,
        actor: &Principal,
        collection: CollectionId,
        bytes: &[u8],
    ) -> Result<CatalogUploadPreview> {
        self.require_administrator(actor)?;
        let eligibility = self.content_write_eligibility(actor, collection)?;
        if eligibility.owner.0 != "org.junk.games" {
            return Err(Error::Invalid("Choose a Games collection".into()));
        }
        if bytes.is_empty() || bytes.len() > MAX_DAT_UPLOAD_BYTES {
            return Err(Error::Invalid(
                "Upload DAT/ZIP files up to 64 MiB each".into(),
            ));
        }
        self.catalog_inspector
            .as_ref()
            .ok_or_else(|| Error::Unavailable("Catalog inspection is unavailable".into()))?
            .inspect_upload(
                bytes,
                self.catalog_uploads
                    .as_ref()
                    .ok_or_else(|| {
                        Error::Unavailable("Catalog upload staging is unavailable".into())
                    })?
                    .as_ref(),
                512 * 1024 * 1024,
                128,
            )
    }
}

#[derive(Clone, Debug)]
pub struct GameCatalogSlot {
    pub system: String,
    pub catalog: String,
    pub byte_domain: String,
}
pub fn game_catalog_slot(record: &SnapshotRecord) -> Result<Option<GameCatalogSlot>> {
    let Ok(header) = record
        .parsed_observation
        .parse::<collector_modules::games::DatSnapshotObservation>()
    else {
        return Ok(None);
    };
    let Some(system) = collector_modules::games::standard_system(&header.system) else {
        return Ok(None);
    };
    if !matches!(header.catalog.as_str(), "No-Intro" | "Redump")
        || header.byte_domain_version != 1
        || (header.catalog == "Redump") != (system.byte_domain == "org.junk.game.cd.raw-2352-track")
        || header.byte_domain != system.byte_domain
            && !(system.id == "nes" && header.byte_domain == "org.junk.game.raw-bytes")
    {
        return Ok(None);
    }
    Ok(Some(GameCatalogSlot {
        system: system.id,
        catalog: header.catalog,
        byte_domain: header.byte_domain,
    }))
}

#[derive(Clone, Debug, Default)]
pub struct GameScanStatus {
    pub pending: u64,
    pub completed: u64,
    pub failed: u64,
    pub cancelled: u64,
}
