//! Scan admission/readiness and equivalence policy. Adapters persist these decisions.
use super::*;

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ScanState {
    Ready,
    Blocked,
    Running,
    Cancelling,
    Completed,
    Failed,
    Changed,
}

#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
pub struct ScanPlan {
    pub state: ScanState,
    pub command: StartProcessingCommand,
    pub files: Vec<collector_modules::IngestAnchorChoice>,
    pub blockers: Vec<String>,
    pub job: Option<JobView>,
    pub result: Option<ProcessingSummary>,
    /// Recorded current disc identity, for choosing compatible saved responses.
    pub accuraterip_query: Option<collector_modules::music::catalog::DbarQuery>,
    pub accuraterip_structure_observation_id: Option<ObservationId>,
}

/// Input identity uses reference content; the original Snapshot identity remains
/// occurrence provenance. No private request/Job identities enter portable data.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScanIdentity {
    pub collection_id: CollectionId,
    pub operation: OperationId,
    pub operation_version: u32,
    pub source: SourceFingerprint,
    pub selection: serde_json::Value,
    pub reference: Option<ScanReference>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScanReference {
    pub provider: ModuleId,
    pub bytes: ByteLength,
    pub digest: Digest,
    pub parsed_observation: RawJson,
}
impl ScanIdentity {
    pub fn key(&self) -> Result<String> {
        Ok(Digest::sha256(
            &serde_json::to_vec(&("scan-input-v1", self))
                .map_err(|e| Error::Invalid(e.to_string()))?,
        )
        .hex)
    }
    pub fn validate(&self) -> Result<()> {
        self.source.validate()?;
        if self.operation_version == 0 || !self.selection.is_object() {
            return Err(Error::Invalid("Invalid scan identity".into()));
        }
        if let Some(reference) = &self.reference {
            reference.provider.validate()?;
            reference.digest.validate()?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProcessingResultGroup {
    pub observation: ProcessingObservation,
    pub first_checked_at: chrono::DateTime<Utc>,
    pub last_checked_at: chrono::DateTime<Utc>,
    pub occurrences: u64,
}

/// A repeated actual computation. The complete result remains in its first record.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProcessingOccurrence {
    pub id: ObservationId,
    pub collection_id: CollectionId,
    pub result_id: ObservationId,
    pub result_key: String,
    pub snapshot_id: Option<SnapshotId>,
    pub import_rebinding: Option<ImportSourceRebinding>,
    pub checked_at: chrono::DateTime<Utc>,
}

pub fn processing_result_key(record: &PortableProcessingRecord) -> Result<String> {
    let o = &record.observation;
    // Legacy records without frozen input provenance only share exact recorded
    // envelopes (including their reference ID). New identities include all settings.
    let legacy_reference = record.inputs.is_none().then_some(o.snapshot_id);
    let value = serde_json::json!({"version":1,"collection":o.collection_id,"operation":o.operation,
        "operation_version":o.operation_version,"source":o.source_fingerprint,"inputs":record.inputs,
        "legacy_reference":legacy_reference,"support":o.support,"raw":o.raw_observations,
        "result":o.typed_result,"limitations":o.limitations});
    Ok(Digest::sha256(&serde_json::to_vec(&value).map_err(|e| Error::Invalid(e.to_string()))?).hex)
}

impl CollectionService {
    pub(crate) fn scan_identity(&self, frozen: &FrozenProcessingJob) -> Result<ScanIdentity> {
        let reference = frozen
            .snapshot_id
            .map(|snapshot_id| {
                self.processing.snapshot(SnapshotQuery {
                    collection_id: frozen.collection_id,
                    snapshot_id,
                })
            })
            .transpose()?;
        scan_identity_from(frozen, reference)
    }

    pub fn processing_plan(
        &self,
        actor: &Principal,
        capture_id: CaptureId,
        mut command: StartProcessingCommand,
    ) -> Result<ScanPlan> {
        let capture = self.reader.capture(capture_id)?;
        let eligibility = self.content_write_eligibility(actor, capture.collection_id)?;
        if !capture
            .representations
            .iter()
            .any(|v| v.id == command.representation_id)
        {
            return Err(Error::NotFound);
        }
        let mut blockers = Vec::new();
        let version = match self.validate_operation_eligibility(&eligibility, &command.operation) {
            Ok(version) => version,
            Err(Error::Unavailable(message)) => {
                blockers.push(message);
                command.operation_version
            }
            Err(error) => return Err(error),
        };
        let inventory = self.processing.package_inventory(
            capture.collection_id,
            PackageScope::Representation(command.representation_id),
        )?;
        if version != command.operation_version {
            blockers.push("This operation version is not available.".into());
        }
        let planned = self
            .modules
            .scan_inputs(&command.operation, &inventory, &command.selection);
        let files = match planned {
            Ok(inputs) => {
                command.selection = inputs.selection;
                blockers.extend(inputs.blockers);
                inputs.files
            }
            Err(error) => {
                blockers.push(error.to_string());
                vec![]
            }
        };
        let selection: serde_json::Value = command
            .selection
            .parse()
            .map_err(|e| Error::Invalid(e.to_string()))?;
        let expected_provider = match command.operation.as_str() {
            "org.junk.games.catalog-verification" => Some("org.junk.providers.local-dat"),
            "org.junk.music.accuraterip-verification" => Some("org.junk.providers.accuraterip"),
            _ => None,
        };
        // Read committed structure only. Planning must never inspect/hash audio.
        // A package with several anchors needs the worker's selected-layout check;
        // a source-level layout alone cannot identify that anchor unambiguously.
        // Resolve the anchor independently of a response choice: an empty or
        // invalid snapshot ID must not hide the query needed to choose one.
        let music_layout_operation = matches!(
            command.operation.as_str(),
            collector_modules::music::ACCURATERIP_VERIFICATION_OPERATION
                | collector_modules::music::CD_STRUCTURE_OPERATION
        );
        let single_music_anchor = if music_layout_operation {
            self.modules
                .scan_inputs(
                    &OperationId::new(collector_modules::music::CD_STRUCTURE_OPERATION)?,
                    &inventory,
                    &RawJson::from_value(serde_json::json!({})),
                )
                .is_ok_and(|inputs| inputs.files.len() == 1)
        } else {
            false
        };
        let accuraterip_structure = if music_layout_operation {
            let structure = self.latest_music_result(
                capture.collection_id,
                command.representation_id,
                collector_modules::music::CD_STRUCTURE_OPERATION,
            )?;
            let structure = match structure {
                Some(value) => Some(value),
                None => self.latest_music_result(
                    capture.collection_id,
                    command.representation_id,
                    "org.junk.music.import-analysis",
                )?,
            };
            structure.and_then(|(id, value)| {
                collector_modules::music::accuraterip_query_from_structure(&value)
                    .ok()
                    .map(|query| (id, query))
            })
        } else {
            None
        };
        let accuraterip_structure_observation_id =
            accuraterip_structure.as_ref().map(|(id, _)| *id);
        let accuraterip_query = accuraterip_structure
            .filter(|_| single_music_anchor)
            .map(|(_, query)| query);
        if let Some(provider) = expected_provider {
            match command.snapshot_id {
                Some(snapshot_id) => {
                    if selection
                        .get("snapshot_id")
                        .and_then(serde_json::Value::as_str)
                        != Some(snapshot_id.to_string().as_str())
                    {
                        blockers.push("Choose the same reference snapshot for the operation and its selection.".into());
                    }
                    match self.processing.snapshot(SnapshotQuery {
                        collection_id: capture.collection_id,
                        snapshot_id,
                    }) {
                        Ok(snapshot) => {
                            if snapshot.provider.0 != provider {
                                blockers
                                    .push("The reference belongs to a different provider.".into());
                            }
                            if provider == "org.junk.providers.local-dat" {
                                let metadata: serde_json::Value = snapshot
                                    .parsed_observation
                                    .value()
                                    .map_err(|e| Error::Invalid(e.to_string()))?;
                                if metadata.get("system") != selection.get("system") {
                                    blockers.push(
                                        "The chosen DAT snapshot is for a different system.".into(),
                                    );
                                }
                            }
                            if let Some(expected) = &accuraterip_query {
                                let observation = snapshot.parsed_observation
                                    .parse::<collector_modules::music::catalog::DbarSnapshotObservation>();
                                if !observation.is_ok_and(|value| value.query == *expected) {
                                    blockers.push("This AccurateRip response belongs to a different disc. Choose a matching response or fetch one for this source.".into());
                                }
                            }
                        }
                        Err(Error::NotFound) => blockers.push(
                            "Choose an available reference snapshot from this collection.".into(),
                        ),
                        Err(error) => return Err(error),
                    }
                }
                None if provider == "org.junk.providers.accuraterip"
                    && selection.get("snapshot_id").is_none() => {}
                None => blockers.push("Choose a reference snapshot before checking.".into()),
            }
        } else if command.snapshot_id.is_some()
            && matches!(
                command.operation.as_str(),
                "org.junk.games.hash"
                    | "org.junk.games.inspect-ines"
                    | "org.junk.games.hash-tracks"
                    | "org.junk.music.cd-structure"
            )
        {
            blockers.push("This operation does not use a reference snapshot.".into());
        }
        let mut plan = ScanPlan {
            state: ScanState::Blocked,
            command,
            files,
            blockers,
            job: None,
            result: None,
            accuraterip_query,
            accuraterip_structure_observation_id,
        };
        if !plan.blockers.is_empty() {
            return Ok(plan);
        }
        let frozen = FrozenProcessingJob {
            collection_id: capture.collection_id,
            collection_binding: eligibility.binding,
            operation: plan.command.operation.clone(),
            operation_version: version,
            source: inventory.scope,
            source_fingerprint: self
                .processing
                .recorded_source(capture.collection_id, inventory.scope)?,
            snapshot_id: plan.command.snapshot_id,
            observation_id: ObservationId::new(),
            selection: plan.command.selection.clone(),
            created_at: self.authentication.now(),
        };
        let key = self.scan_identity(&frozen)?.key()?;
        let jobs = self.jobs.scan_jobs(actor.user_id, &key)?;
        let job = jobs.into_iter().next();
        let result = self
            .processing
            .scan_result(&key)?
            .map(|group| self.present_processing_group(group, &frozen.source_fingerprint))
            .transpose()?;
        plan.state = match job.as_ref().map(|v| v.state.clone()) {
            Some(JobState::Queued | JobState::Running) => ScanState::Running,
            Some(JobState::CancelRequested) => ScanState::Cancelling,
            Some(JobState::Failed) => ScanState::Failed,
            _ if result.is_some() => ScanState::Completed,
            _ => ScanState::Ready,
        };
        plan.job = job.map(Into::into);
        plan.result = result;
        if matches!(plan.state, ScanState::Ready) {
            let old = self.processing.observations(&ProcessingQuery {
                collection_id: capture.collection_id,
                source: inventory.scope,
                source_fingerprint: frozen.source_fingerprint,
                operation: Some(plan.command.operation.clone()),
                after: None,
                limit: 1,
            })?;
            if let Some(old) = old.items.into_iter().next() {
                plan.state = ScanState::Changed;
                plan.result = Some(
                    self.present_processing_group(
                        ProcessingResultGroup {
                            first_checked_at: old.created_at,
                            last_checked_at: old.created_at,
                            occurrences: 1,
                            observation: old,
                        },
                        &self
                            .processing
                            .recorded_source(capture.collection_id, inventory.scope)?,
                    )?,
                );
            }
        }
        Ok(plan)
    }
    pub(crate) fn present_processing_group(
        &self,
        group: ProcessingResultGroup,
        current: &SourceFingerprint,
    ) -> Result<ProcessingSummary> {
        let o = group.observation;
        let snapshot_observation = o
            .snapshot_id
            .and_then(|snapshot_id| {
                self.processing
                    .snapshot(SnapshotQuery {
                        collection_id: o.collection_id,
                        snapshot_id,
                    })
                    .ok()
            })
            .map(|s| s.parsed_observation);
        Ok(ProcessingSummary {
            observation_id: o.id,
            operation: o.operation,
            source: o.source_fingerprint.source,
            snapshot_id: o.snapshot_id,
            snapshot_observation,
            result: o.typed_result.payload,
            current: o.source_fingerprint.source == current.source
                && o.source_fingerprint.files == current.files,
            support: o.support,
            limitations: o.limitations,
            legacy_provenance: false,
            created_at: group.first_checked_at,
            last_checked_at: group.last_checked_at,
            occurrences: group.occurrences,
        })
    }
}

pub fn scan_identity_from(
    frozen: &FrozenProcessingJob,
    reference: Option<SnapshotRecord>,
) -> Result<ScanIdentity> {
    let mut selection: serde_json::Value = frozen
        .selection
        .parse()
        .map_err(|e| Error::Invalid(e.to_string()))?;

    if let Some(snapshot) = &reference
        && selection.get("snapshot_id").is_some()
    {
        selection["snapshot_id"] =
            serde_json::json!({"digest":snapshot.digest,"provider":snapshot.provider});
    }
    Ok(ScanIdentity {
        collection_id: frozen.collection_id,
        operation: frozen.operation.clone(),
        operation_version: frozen.operation_version,
        source: frozen.source_fingerprint.clone(),
        selection,
        reference: reference.map(|s| ScanReference {
            provider: s.provider,
            bytes: s.bytes,
            digest: s.digest,
            parsed_observation: s.parsed_observation,
        }),
    })
}

impl CollectionService {
    pub(crate) fn run_frozen_processing(
        &self,
        claim: &JobClaim,
        frozen: FrozenProcessingJob,
    ) -> Result<serde_json::Value> {
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
        self.report_processing_phase(claim, "validating_source")?;
        if self.fingerprint_inventory_checked(&inventory, &mut || {
            if self
                .jobs
                .cancellation_requested(claim.id, &claim.claim_token)?
            {
                Err(Error::JobCancelled)
            } else {
                Ok(())
            }
        })? != frozen.source_fingerprint
        {
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
        let phase = if frozen.operation.as_str()
            == collector_modules::music::ACCURATERIP_VERIFICATION_OPERATION
        {
            "checking_accuraterip"
        } else {
            "processing_files"
        };
        self.report_processing_phase(claim, phase)?;
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
        self.report_processing_phase(claim, "rechecking_source")?;
        if self.fingerprint_inventory_checked(&inventory, &mut || {
            if self
                .jobs
                .cancellation_requested(claim.id, &claim.claim_token)?
            {
                Err(Error::JobCancelled)
            } else {
                Ok(())
            }
        })? != frozen.source_fingerprint
        {
            return Err(Error::Conflict);
        }
        let inputs = Some(self.scan_identity(&frozen)?);
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
            created_at: self.authentication.now(),
        };
        observation.validate()?;
        self.report_processing_phase(claim, "recording_check")?;
        self.processing.publish_processing_record(
            claim,
            PortableProcessingRecord {
                inputs,
                observation,
                import_rebinding: None,
            },
        )?;
        Ok(serde_json::json!({"observation_id":frozen.observation_id}))
    }
}
