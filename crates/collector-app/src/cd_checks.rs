//! One resumable CD audio workflow shared by ingest, batches and the disc button.
use crate::*;
use collector_modules::music::{
    ACCURATERIP_VERIFICATION_OPERATION, AudioVerificationStatus, CD_STRUCTURE_OPERATION,
    CdProcessingResult,
};

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CheckAccurateRip {
    pub representation_id: Option<RepresentationId>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FrozenAccurateRipCheck {
    pub job_id: JobId,
    pub item_id: ItemId,
    pub generation: u64,
    pub maximum_offset: i32,
    pub sources: Vec<FrozenProcessingJob>,
    pub limitations: Vec<String>,
    pub recorded_layouts: std::collections::BTreeMap<RepresentationId, ObservationId>,
}
impl FrozenAccurateRipCheck {
    pub fn validate(&self) -> Result<()> {
        if self.sources.is_empty()
            || self.sources.len() > 99
            || self.limitations.len() > 99
            || self.limitations.iter().any(|s| s.len() > 4096)
            || !(0..=2939).contains(&self.maximum_offset)
        {
            return Err(Error::Invalid("Invalid frozen CD check".into()));
        }
        for source in &self.sources {
            source.validate()?;
        }
        Ok(())
    }
}
impl CollectionService {
    pub fn check_accuraterip(
        &self,
        actor: &Principal,
        item_id: ItemId,
        command: CheckAccurateRip,
        context: &CommandContext,
    ) -> Result<JobId> {
        let item = self.reader.item(item_id)?;
        self.content_write_eligibility(actor, item.collection_id)?;
        self.execute_command(
            actor,
            context,
            "accuraterip.check",
            &(item_id, &command),
            |effects| {
                let item = self.reader.item(item_id)?;
                let job_id =
                    deterministic_job_id(actor.user_id, "accuraterip.check", context.key());
                let frozen = self.freeze_accuraterip_check(
                    actor.user_id,
                    &item,
                    job_id,
                    command.representation_id,
                    false,
                )?;
                effects.enqueue_job(self.accuraterip_spec(actor.user_id, &item, frozen, None))
            },
        )
    }
    pub(crate) fn freeze_accuraterip_check(
        &self,
        user: UserId,
        item: &Item,
        job_id: JobId,
        selected: Option<RepresentationId>,
        unconfirmed_only: bool,
    ) -> Result<FrozenAccurateRipCheck> {
        let eligibility = self.content_write_eligibility_for_user(user, item.collection_id)?;
        if item.deleted || eligibility.owner.0 != "org.junk.music" {
            return Err(Error::Invalid(
                "AccurateRip requires a music CD original".into(),
            ));
        }
        let settings = self.care.cd_settings(item.collection_id)?;
        let mut sources = Vec::new();
        let mut limitations = Vec::new();
        let mut recorded_layouts = std::collections::BTreeMap::new();
        for representation in item
            .captures
            .iter()
            .filter(|c| c.association.item_id == Some(item.id))
            .flat_map(|c| &c.representations)
            .filter(|r| {
                r.role == RepresentationRole::PreservationMaster
                    && selected.is_none_or(|id| id == r.id)
            })
        {
            if representation.format == "original-file-audio-package"
                || self
                    .recorded_file_tracks(item.collection_id, representation.id)?
                    .is_some()
            {
                continue;
            }
            if unconfirmed_only
                && matches!(self.latest_music_result(item.collection_id, representation.id, ACCURATERIP_VERIFICATION_OPERATION)?, Some((_,CdProcessingResult::AudioVerification {verification})) if verification.status==AudioVerificationStatus::Verified)
            {
                continue;
            }
            let inventory = self.processing.package_inventory(
                item.collection_id,
                PackageScope::Representation(representation.id),
            )?;
            let operation = OperationId::new(CD_STRUCTURE_OPERATION)?;
            let recorded = self
                .latest_music_result(
                    item.collection_id,
                    representation.id,
                    CD_STRUCTURE_OPERATION,
                )?
                .or(self.latest_music_result(
                    item.collection_id,
                    representation.id,
                    "org.junk.music.import-analysis",
                )?);
            let mut selection = serde_json::json!({});
            if let Some((id, _)) = recorded {
                let record = self.processing.processing_record(id)?;
                let chosen = if record.observation.operation.as_str()
                    == "org.junk.music.import-analysis"
                {
                    record
                        .observation
                        .typed_result
                        .payload
                        .parse::<collector_modules::music::MusicImportAnalysisOutput>()?
                        .selection
                        .map(|v| serde_json::to_value(v).map_err(|e| Error::Invalid(e.to_string())))
                        .transpose()?
                } else {
                    record.inputs.as_ref().map(|v| v.selection.clone())
                };
                if let Some(anchor) = chosen.and_then(|v| v.get("anchor_file_index").cloned()) {
                    selection["anchor_file_index"] = anchor;
                }
                recorded_layouts.insert(representation.id, id);
            }
            let plan = self.modules.scan_inputs(
                &operation,
                &inventory,
                &RawJson::from_value(selection),
            )?;
            if !plan.blockers.is_empty() {
                let reason = format!(
                    "Original {} needs CD structure review: {}",
                    representation.id,
                    plan.blockers.join("; ")
                );
                if selected.is_some() {
                    return Err(Error::Unavailable(reason));
                }
                limitations.push(reason);
                continue;
            }
            sources.push(FrozenProcessingJob {
                collection_id: item.collection_id,
                collection_binding: eligibility.binding.clone(),
                operation,
                operation_version: eligibility.schema_version,
                source: inventory.scope,
                source_fingerprint: self
                    .processing
                    .recorded_source(item.collection_id, inventory.scope)?,
                snapshot_id: None,
                observation_id: ObservationId(deterministic_inventory_uuid(
                    user,
                    "accuraterip.structure",
                    &format!("{job_id}:{}", representation.id),
                )),
                selection: plan.selection,
                created_at: self.authentication.now(),
            });
        }
        let frozen = FrozenAccurateRipCheck {
            job_id,
            item_id: item.id,
            generation: item.lifecycle.generation,
            maximum_offset: settings.maximum_offset,
            sources,
            limitations,
            recorded_layouts,
        };
        if frozen.sources.is_empty() {
            return Err(Error::Unavailable(
                "No unconfirmed CD originals need checking".into(),
            ));
        }
        frozen.validate()?;
        Ok(frozen)
    }
    pub(crate) fn accuraterip_spec(
        &self,
        user: UserId,
        item: &Item,
        frozen: FrozenAccurateRipCheck,
        import: Option<ImportId>,
    ) -> JobSpec {
        JobSpec {
            id: frozen.job_id,
            user_id: user,
            collection_id: Some(item.collection_id),
            item_id: Some(item.id),
            kind: "accuraterip_check".into(),
            phase: "queued".into(),
            total_units: 0,
            request: serde_json::json!({"item_id":item.id,"import_id":import,"guided":import.is_some()}),
            frozen_input: Some(FrozenJobInput::AccurateRip(frozen)),
        }
    }
    fn current_cd_check(&self, claim: &JobClaim, frozen: &FrozenAccurateRipCheck) -> Result<()> {
        let item = self.reader.item(frozen.item_id)?;
        if item.deleted || item.lifecycle.generation != frozen.generation {
            return Err(Error::Conflict);
        }
        let current = self.content_write_eligibility_for_user(claim.user_id, item.collection_id)?;
        for source in &frozen.sources {
            self.validate_frozen_binding(&current, &source.collection_binding)?;
            if !item
                .captures
                .iter()
                .filter(|c| c.association.item_id == Some(item.id))
                .flat_map(|c| &c.representations)
                .any(|r| PackageScope::Representation(r.id) == source.source)
                || self
                    .processing
                    .recorded_source(item.collection_id, source.source)?
                    != source.source_fingerprint
            {
                return Err(Error::Conflict);
            }
        }
        if self
            .jobs
            .cancellation_requested(claim.id, &claim.claim_token)?
        {
            return Err(Error::JobCancelled);
        }
        Ok(())
    }
    pub(crate) fn run_accuraterip_check(&self, claim: &JobClaim) -> Result<serde_json::Value> {
        let Some(FrozenJobInput::AccurateRip(frozen)) = &claim.frozen_input else {
            return Err(Error::Conflict);
        };
        frozen.validate()?;
        self.current_cd_check(claim, frozen)?;
        let mut results = Vec::new();
        for source in &frozen.sources {
            self.current_cd_check(claim, frozen)?;
            let id = match source.source {
                PackageScope::Representation(id) => id,
                _ => return Err(Error::Conflict),
            };
            let observation_id = if let Some(id) = frozen.recorded_layouts.get(&id) {
                *id
            } else {
                self.run_frozen_processing(claim, source.clone())?;
                source.observation_id
            };
            let observation = self
                .processing
                .processing_record(observation_id)?
                .observation;
            if observation.collection_id != source.collection_id
                || observation.source_fingerprint != source.source_fingerprint
                || observation.typed_result.module.0 != "org.junk.music"
                || !matches!(observation.support, OperationSupport::Supported)
            {
                return Err(Error::Conflict);
            }
            let layout = recorded_cd_layout_observation(&observation)?;
            let query = collector_modules::music::accuraterip_query_from_structure(&layout)
                .map_err(|e| Error::Invalid(e.to_string()))?;
            let acquisition = FrozenAcquisitionJob {
                request: ProviderRequest {
                    collection_id: source.collection_id,
                    provider: ModuleId("org.junk.providers.accuraterip".into()),
                    operation: OperationId::new("org.junk.providers.accuraterip.acquire")?,
                    source_fingerprint: Some(source.source_fingerprint.clone()),
                    query: RawJson::from_value(
                        serde_json::to_value(query).map_err(|e| Error::Invalid(e.to_string()))?,
                    ),
                    maximum_response_bytes: 8 * 1024 * 1024,
                },
                collection_binding: source.collection_binding.clone(),
                item_id: Some(frozen.item_id),
                snapshot_id: SnapshotId(deterministic_inventory_uuid(
                    claim.user_id,
                    "accuraterip.response",
                    &format!("{}:{id}", claim.id),
                )),
                created_at: source.created_at,
            };
            let snapshot = self.acquire_snapshot_for_job(claim, &acquisition)?;
            self.current_cd_check(claim, frozen)?;
            let mut check = source.clone();
            check.operation = OperationId::new(ACCURATERIP_VERIFICATION_OPERATION)?;
            check.observation_id = ObservationId(deterministic_inventory_uuid(
                claim.user_id,
                "accuraterip.result",
                &format!("{}:{id}", claim.id),
            ));
            check.snapshot_id = Some(snapshot);
            let mut selection = source.selection.value()?;
            selection["snapshot_id"] = serde_json::json!(snapshot);
            selection["maximum_offset"] = serde_json::json!(frozen.maximum_offset);
            check.selection = RawJson::from_value(selection);
            self.run_frozen_processing(claim, check.clone())?;
            let result = self
                .processing
                .processing_record(check.observation_id)?
                .observation
                .typed_result
                .payload
                .parse::<CdProcessingResult>()?;
            let CdProcessingResult::AudioVerification { verification } = result else {
                return Err(Error::Conflict);
            };
            results.push(serde_json::json!({"source_id":id,"status":verification.status,"observation_id":check.observation_id}));
        }
        self.current_cd_check(claim, frozen)?;
        Ok(
            serde_json::json!({"confirmed":frozen.limitations.is_empty() && results.iter().all(|r|r["status"]=="verified"),"discs":results,"limitations":frozen.limitations}),
        )
    }
}

impl CollectionService {
    pub(crate) fn advance_ingest_cd_check(
        &self,
        intent: &GuidedIngestIntent,
        record: &ImportRecord,
    ) -> Result<bool> {
        let id = deterministic_job_id(
            intent.user_id,
            "guided.accuraterip",
            &intent.import_id.to_string(),
        );
        match self.jobs.job_record(id, intent.user_id) {
            Ok(job) => {
                return Ok(matches!(
                    job.state,
                    JobState::Completed | JobState::Failed | JobState::Cancelled
                ));
            }
            Err(Error::NotFound) => {}
            Err(error) => return Err(error),
        }
        let item = self.reader.item(intent.target_item_id)?;
        let mut frozen = self.freeze_accuraterip_check(
            intent.user_id,
            &item,
            id,
            Some(record.view.representation_id),
            false,
        )?;
        frozen.maximum_offset = intent
            .cd_check_settings
            .as_ref()
            .ok_or(Error::Conflict)?
            .maximum_offset;
        self.jobs.enqueue_job(self.accuraterip_spec(
            intent.user_id,
            &item,
            frozen,
            Some(intent.import_id),
        ))?;
        Ok(false)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AccurateRipDiscResult {
    pub source_id: RepresentationId,
    pub status: AudioVerificationStatus,
    pub observation_id: ObservationId,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AccurateRipCheckResult {
    pub confirmed: bool,
    pub limitations: Vec<String>,
    pub discs: Vec<AccurateRipDiscResult>,
}
#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
pub struct AccurateRipCheckView {
    pub job: JobView,
    pub sources: Vec<RepresentationId>,
    pub result: Option<AccurateRipCheckResult>,
}
impl CollectionService {
    pub fn accuraterip_checks(
        &self,
        actor: &Principal,
        item: ItemId,
    ) -> Result<Vec<AccurateRipCheckView>> {
        let record = self.reader.item(item)?;
        self.authorize(actor, record.collection_id, Action::Curate)?;
        self.care
            .cd_check_jobs(actor.user_id, item)?
            .into_iter()
            .map(|job| {
                let Some(FrozenJobInput::AccurateRip(frozen)) =
                    self.jobs.frozen_job_input(job.id, actor.user_id)?
                else {
                    return Err(Error::Conflict);
                };
                let result = job
                    .result
                    .as_ref()
                    .map(|r| r.get("completion").unwrap_or(r))
                    .filter(|r| r.get("discs").is_some())
                    .map(|r| {
                        serde_json::from_value(r.clone()).map_err(|e| Error::Storage(e.to_string()))
                    })
                    .transpose()?;
                Ok(AccurateRipCheckView {
                    sources: frozen
                        .sources
                        .iter()
                        .filter_map(|s| match s.source {
                            PackageScope::Representation(id) => Some(id),
                            _ => None,
                        })
                        .collect(),
                    result,
                    job: job.into(),
                })
            })
            .collect()
    }
}

pub(crate) fn recorded_cd_layout_observation(
    observation: &ProcessingObservation,
) -> Result<CdProcessingResult> {
    let result = match observation.operation.as_str() {
        CD_STRUCTURE_OPERATION => observation.typed_result.payload.parse()?,
        "org.junk.music.import-analysis" => {
            let output = observation
                .typed_result
                .payload
                .parse::<collector_modules::music::MusicImportAnalysisOutput>()?;
            let mut layouts = Vec::new();
            for value in output.observations {
                let result = value.parse::<CdProcessingResult>()?;
                if matches!(result, CdProcessingResult::Layout { .. }) {
                    layouts.push(result);
                }
            }
            if layouts.len() != 1 {
                return Err(Error::Invalid(
                    "Import analysis must contain one unambiguous CD layout".into(),
                ));
            }
            layouts.remove(0)
        }
        _ => {
            return Err(Error::Invalid(
                "Observation is not a CD structure check".into(),
            ));
        }
    };
    if !matches!(result, CdProcessingResult::Layout { .. }) {
        return Err(Error::Invalid("Observation has no CD layout".into()));
    }
    Ok(result)
}
