use super::*;
use std::collections::{BTreeMap, BTreeSet};

struct GuidedProviderStep {
    id: JobId,
    provider: ModuleId,
    operation: OperationId,
    query: RawJson,
    maximum_response_bytes: u64,
}

fn safe_package_title(name: &str) -> Result<String> {
    if name.is_empty()
        || name.len() > 4096
        || name.contains(['/', '\\'])
        || name.chars().any(char::is_control)
    {
        return Err(Error::Invalid(
            "package name must be a direct, bounded name".into(),
        ));
    }
    let title = name
        .rsplit_once('.')
        .filter(|(_, extension)| !extension.is_empty() && extension.len() <= 16)
        .map_or(name, |(stem, _)| stem)
        .replace(['_', '-'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if title.is_empty() {
        Ok(name.to_owned())
    } else {
        Ok(title)
    }
}

fn validate_plan_files(files: &[UploadFile]) -> Result<()> {
    if files.is_empty() || files.len() > 10_000 {
        return Err(Error::Invalid(
            "package inventory must contain 1..=10000 files".into(),
        ));
    }
    let total = files
        .iter()
        .try_fold(0u64, |sum, file| sum.checked_add(file.bytes.0))
        .ok_or_else(|| Error::Invalid("package byte count overflows".into()))?;
    if total > i64::MAX as u64 {
        return Err(Error::Invalid(
            "package byte count exceeds storage limits".into(),
        ));
    }
    let mut names = BTreeSet::new();
    for file in files {
        if file.name.is_empty()
            || file.name.len() > 4096
            || file.name.starts_with('/')
            || file.name.contains('\\')
            || file
                .name
                .split('/')
                .any(|part| part.is_empty() || matches!(part, "." | ".."))
            || file.name.chars().any(char::is_control)
            || !names.insert(file.name.to_ascii_lowercase())
        {
            return Err(Error::Invalid(
                "package inventory contains an unsafe or duplicate name".into(),
            ));
        }
        if let Some(digest) = &file.sha256 {
            Digest::new(HashAlgorithm::Sha256, digest.clone())?;
        }
    }
    Ok(())
}

impl CollectionService {
    /// Reconciles accepted guided work after a process restart. Ordinary
    /// transitions also call `advance_guided_ingest`; this bounded sweep only
    /// closes the crash window between a durable transition and its next job.
    pub fn resume_guided_ingests(&self) -> Result<()> {
        let cleanup_result = self.provisional_imports.reconcile_guided_cleanup();
        let mut after = None;
        loop {
            let ids = self
                .provisional_imports
                .resumable_guided_ingest_ids(after, 256)?;
            if ids.is_empty() {
                break;
            }
            after = ids.last().copied();
            for id in ids {
                self.reconcile_guided_ingest(id)?;
            }
        }
        cleanup_result
    }

    pub fn resumable_guided_inbox_imports(
        &self,
        after: Option<ImportId>,
    ) -> Result<Vec<ImportView>> {
        Ok(self
            .provisional_imports
            .resumable_guided_inbox_records(after, 256)?
            .into_iter()
            .map(|record| record.view)
            .collect())
    }

    /// Host recovery reports a bounded, path-free source diagnostic.
    pub fn report_guided_staging_failure(&self, id: ImportId, message: &str) -> Result<()> {
        let mut intent = self
            .provisional_imports
            .guided_ingest_intent(id)?
            .ok_or(Error::NotFound)?;
        if !intent.cancelled && intent.enrichment_error.as_deref() != Some(message) {
            intent.enrichment_error = Some(message.chars().take(512).collect());
            intent.updated_at = self.authentication.now();
            self.provisional_imports.save_guided_ingest_intent(intent)?;
        }
        Ok(())
    }

    pub fn ensure_guided_inbox_staging(&self, id: ImportId, source_locator: &str) -> Result<JobId> {
        let intent = self
            .provisional_imports
            .guided_ingest_intent(id)?
            .ok_or(Error::NotFound)?;
        let record = self.provisional_imports.import_record(id)?;
        if record.user_id != intent.user_id
            || record.view.source.kind != ImportSourceKind::Inbox
            || record.view.phase != ImportPhase::Staging
        {
            return Err(Error::Conflict);
        }
        self.content_write_eligibility_for_user(intent.user_id, record.view.collection_id)?;
        let id = super::deterministic_job_id(
            intent.user_id,
            "guided.inbox.stage",
            &record.view.id.to_string(),
        );
        self.jobs.enqueue_job(JobSpec {
            id,
            user_id: intent.user_id,
            collection_id: Some(record.view.collection_id),
            item_id: None,
            kind: "import_staging".into(),
            phase: "queued".into(),
            total_units: record.view.files.iter().map(|file| file.bytes.0).sum(),
            request: serde_json::json!({
                "import_id": record.view.id,
                "source_locator": source_locator,
                "guided": true,
            }),
            frozen_input: None,
        })
    }

    pub fn plan_ingest(&self, actor: &Principal, command: PlanIngestCommand) -> Result<IngestPlan> {
        if command.packages.is_empty() || command.packages.len() > 64 {
            return Err(Error::Invalid(
                "guided intake accepts 1..=64 packages".into(),
            ));
        }
        let eligibility = self.content_write_eligibility(actor, command.collection_id)?;
        let mut packages = Vec::with_capacity(command.packages.len());
        for (position, package) in command.packages.iter().enumerate() {
            validate_plan_files(&package.files)?;
            let default_title = safe_package_title(&package.source.package_name)?;
            let readiness = self
                .modules
                .import_readiness(
                    &eligibility.owner,
                    &package
                        .files
                        .iter()
                        .map(|file| PackageFileName::new(file.name.clone()))
                        .collect::<Result<Vec<_>>>()?,
                )
                .map_err(Error::from)?;
            let anchors = readiness.anchors;
            let blockers = readiness.blockers;
            let analysis_available = self
                .modules
                .capability_contract(&CapabilityId::new(format!(
                    "{}.import-analysis",
                    eligibility.owner.0
                ))?)
                .is_ok()
                && !blockers.iter().any(|issue| issue.blocks_analysis);
            let metadata_available = if eligibility.owner.0 == "org.junk.games" {
                self.processing
                    .active_snapshot(
                        command.collection_id,
                        &ModuleId("org.junk.providers.local-dat".into()),
                    )?
                    .is_some()
            } else if eligibility.owner.0 == "org.junk.music" {
                self.streaming_providers
                    .contains_key(collector_modules::music::MUSICBRAINZ_PROVIDER_ID)
                    && self
                        .catalog_parsers
                        .contains_key(collector_modules::music::MUSICBRAINZ_PROVIDER_ID)
            } else {
                false
            };
            let artwork_available = eligibility.owner.0 == "org.junk.music"
                && self
                    .streaming_providers
                    .contains_key(collector_modules::music::COVER_ART_ARCHIVE_PROVIDER_ID)
                && self
                    .catalog_parsers
                    .contains_key(collector_modules::music::COVER_ART_ARCHIVE_PROVIDER_ID);
            packages.push(IngestPlanPackage {
                position: position
                    .try_into()
                    .map_err(|_| Error::Invalid("too many packages".into()))?,
                source: package.source.clone(),
                files: package.files.clone(),
                default_title,
                anchors,
                blockers,
                analysis_available,
                metadata_available,
                artwork_available,
            });
        }
        let token = Digest::sha256(
            &serde_json::to_vec(&(command.collection_id, &eligibility.binding, &packages))
                .map_err(|error| Error::Invalid(error.to_string()))?,
        )
        .hex;
        Ok(IngestPlan {
            collection_id: command.collection_id,
            token,
            packages,
        })
    }

    pub fn start_ingest(
        &self,
        actor: &Principal,
        command: StartIngestCommand,
        context: &CommandContext,
    ) -> Result<AcceptedIngest> {
        self.start_ingest_with_preflight(actor, command, context, |_| Ok(()))
    }

    /// The host supplies filesystem preflight, executed only for a new receipt.
    pub fn start_ingest_with_preflight(
        &self,
        actor: &Principal,
        command: StartIngestCommand,
        context: &CommandContext,
        preflight: impl FnOnce(&IngestPlan) -> Result<()>,
    ) -> Result<AcceptedIngest> {
        self.content_write_eligibility(actor, command.plan.collection_id)?;
        let views = self.execute_command(actor, context, "ingest.start", &command, |effects| {
            preflight(&command.plan)?;
            let replanned = self.plan_ingest(
                actor,
                PlanIngestCommand {
                    collection_id: command.plan.collection_id,
                    packages: command
                        .plan
                        .packages
                        .iter()
                        .map(|package| IngestPackageSelection {
                            source: package.source.clone(),
                            files: package.files.clone(),
                        })
                        .collect(),
                },
            )?;
            if replanned.token != command.plan.token
                || serde_json::to_value(&replanned.packages)
                    .map_err(|error| Error::Invalid(error.to_string()))?
                    != serde_json::to_value(&command.plan.packages)
                        .map_err(|error| Error::Invalid(error.to_string()))?
            {
                return Err(Error::Conflict);
            }
            let now = self.authentication.now();
            let mut records = Vec::new();
            let mut intents = Vec::new();
            for package in &command.plan.packages {
                let key = format!("{}:{}", context.key(), package.position);
                let import_id = ImportId(super::deterministic_inventory_uuid(
                    actor.user_id,
                    "guided-import",
                    &key,
                ));
                let target_item_id = ItemId(super::deterministic_inventory_uuid(
                    actor.user_id,
                    "guided-item",
                    &key,
                ));
                let capture_id = CaptureId(super::deterministic_inventory_uuid(
                    actor.user_id,
                    "guided-capture",
                    &key,
                ));
                let representation_id = RepresentationId(super::deterministic_inventory_uuid(
                    actor.user_id,
                    "guided-representation",
                    &key,
                ));
                let empty_digest = Digest::sha256(b"");
                let files = package
                    .files
                    .iter()
                    .map(|file| {
                        let supplied = file
                            .sha256
                            .as_ref()
                            .map(|value| Digest::new(HashAlgorithm::Sha256, value.clone()))
                            .transpose()?;
                        if file.bytes.0 == 0
                            && supplied
                                .as_ref()
                                .is_some_and(|value| value != &empty_digest)
                        {
                            return Err(Error::Invalid(
                                "declared digest does not match the empty file".into(),
                            ));
                        }
                        Ok(ImportDeclaredFile {
                            name: file.name.clone(),
                            bytes: file.bytes,
                            received_bytes: ByteLength(0),
                            digest: if file.bytes.0 == 0 {
                                Some(empty_digest.clone())
                            } else {
                                supplied
                            },
                        })
                    })
                    .collect::<Result<Vec<_>>>()?;
                records.push(ImportRecord {
                    user_id: actor.user_id,
                    view: ImportView {
                        id: import_id,
                        revision: 1,
                        collection_id: command.plan.collection_id,
                        source: package.source.clone(),
                        files,
                        capture_id,
                        representation_id,
                        association: CaptureAssociation::default(),
                        expected_item_revision: None,
                        candidates: vec![],
                        limitations: package
                            .blockers
                            .iter()
                            .map(|value| value.message.clone())
                            .collect(),
                        review: None,
                        analysis_job_id: None,
                        phase: if package.files.iter().all(|file| file.bytes.0 == 0) {
                            ImportPhase::Analyzing
                        } else {
                            ImportPhase::Staging
                        },
                        created_at: now,
                        updated_at: now,
                    },
                });
                intents.push(GuidedIngestIntent {
                    import_id,
                    user_id: actor.user_id,
                    plan_token: command.plan.token.clone(),
                    target_item_id,
                    default_title: package.default_title.clone(),
                    enrichment: command.enrichment,
                    enrichment_error: None,
                    settled: false,
                    cancelled: false,
                    created_at: now,
                    updated_at: now,
                });
            }
            effects.accept_guided_imports(records, intents)
        })?;
        for view in &views {
            self.reconcile_guided_ingest(view.id)?;
        }
        Ok(AcceptedIngest {
            imports: views
                .into_iter()
                .map(|view| self.ingest_summary(actor, view))
                .collect::<Result<Vec<_>>>()?,
            rejected: vec![],
        })
    }

    fn ensure_guided_item(
        &self,
        intent: &GuidedIngestIntent,
        record: &ImportRecord,
    ) -> Result<Item> {
        self.content_write_eligibility_for_user(intent.user_id, record.view.collection_id)?;
        let item = match self.reader.item(intent.target_item_id) {
            Ok(item) => {
                if item.deleted || item.collection_id != record.view.collection_id {
                    return Err(Error::Conflict);
                }
                self.modules.inspect(&item.domain).map_err(Error::from)?;
                item
            }
            Err(Error::NotFound) => {
                let eligibility = self.content_write_eligibility_for_user(
                    intent.user_id,
                    record.view.collection_id,
                )?;
                let item = Item {
                    id: intent.target_item_id,
                    collection_id: record.view.collection_id,
                    revision: Revision(0),
                    title: intent.default_title.clone(),
                    subtitle: String::new(),
                    identity_link: ItemIdentityLink::None,
                    domain: ModuleData {
                        module: eligibility.owner.clone(),
                        schema_version: eligibility.schema_version,
                        payload: RawJson::from_json("{}")?,
                    },
                    identity: IdentityState::Unidentified,
                    translations: vec![],
                    identifiers: vec![],
                    specimens: vec![],
                    wishlist: false,
                    captures: vec![],
                    assets: vec![],
                    metadata: MetadataState::default(),
                    deleted: false,
                    updated_at: intent.created_at,
                };
                self.modules.inspect(&item.domain).map_err(Error::from)?;
                self.writer.save(item, Revision(0))?
            }
            Err(error) => return Err(error),
        };
        if self
            .processing
            .metadata_authority(item.collection_id, item.id)?
            .is_none()
        {
            let mut authority = MetadataAuthority::default();
            if item.title != intent.default_title {
                authority.field_overrides.push(MetadataFieldOverride {
                    field: MetadataField::Title,
                    value: item.title.clone(),
                    language: None,
                });
            }
            if !item.subtitle.is_empty() {
                authority.field_overrides.push(MetadataFieldOverride {
                    field: MetadataField::Subtitle,
                    value: item.subtitle.clone(),
                    language: None,
                });
            }
            let display = resolve_metadata_display(&item, &authority, &[], "en")?;
            self.processing.publish_metadata_authority(
                TransactionId(super::deterministic_inventory_uuid(
                    intent.user_id,
                    "guided-default-authority",
                    &intent.import_id.to_string(),
                )),
                MetadataAuthorityRecord {
                    current_candidates: Some(vec![]),
                    inventory: Some((&item).into()),
                    collection_id: item.collection_id,
                    item_id: item.id,
                    revision: Revision(1),
                    authority,
                    observations: vec![],
                    display: Some(display),
                    updated_at: intent.created_at,
                },
            )?;
        }
        Ok(item)
    }

    fn enqueue_guided_analysis(
        &self,
        intent: &GuidedIngestIntent,
        record: &ImportRecord,
    ) -> Result<JobId> {
        let eligibility =
            self.content_write_eligibility_for_user(intent.user_id, record.view.collection_id)?;
        let capability = CapabilityId::new(format!("{}.import-analysis", eligibility.owner.0))?;
        let contract = self.modules.capability_contract(&capability)?;
        let inventory = self.processing.package_inventory(
            record.view.collection_id,
            PackageScope::Import(record.view.id),
        )?;
        let readiness = self
            .modules
            .import_readiness(
                &eligibility.owner,
                &inventory
                    .files
                    .iter()
                    .map(|file| file.display_name.clone())
                    .collect::<Vec<_>>(),
            )
            .map_err(Error::from)?;
        if let Some(issue) = readiness
            .blockers
            .iter()
            .find(|issue| issue.blocks_analysis)
        {
            return Err(Error::Unavailable(issue.message.clone()));
        }
        let fingerprint = self.fingerprint_inventory(&inventory)?;
        let snapshot_id = self.processing.active_snapshot(
            record.view.collection_id,
            &ModuleId("org.junk.providers.local-dat".into()),
        )?;
        let job = super::deterministic_job_id(
            intent.user_id,
            "guided.import.analysis",
            &record.view.id.to_string(),
        );
        let frozen = FrozenProcessingJob {
            collection_id: record.view.collection_id,
            collection_binding: eligibility.binding,
            operation: capability,
            operation_version: contract.schema_version,
            source: PackageScope::Import(record.view.id),
            source_fingerprint: fingerprint,
            snapshot_id,
            observation_id: ObservationId(job.0),
            selection: RawJson::from_value(serde_json::json!({
                "import_id":record.view.id,"representation_id":record.view.representation_id,
                "files":record.view.files,"snapshot_id":snapshot_id,"system":null,"domain":null
            })),
            created_at: intent.created_at,
        };
        self.jobs.enqueue_job(JobSpec {
            id: job, user_id: intent.user_id, collection_id: Some(record.view.collection_id),
            item_id: Some(intent.target_item_id), kind: "import_analysis".into(), phase: "queued".into(), total_units: 1,
            request: serde_json::json!({"import_id":record.view.id,"module":eligibility.owner,"guided":true}),
            frozen_input: Some(FrozenJobInput::Processing(frozen)),
        })
    }

    fn enqueue_guided_publication(
        &self,
        intent: &GuidedIngestIntent,
        record: &ImportRecord,
    ) -> Result<JobId> {
        let job = super::deterministic_job_id(
            intent.user_id,
            "guided.import.publish",
            &record.view.id.to_string(),
        );
        self.jobs.enqueue_job(JobSpec {
            id: job,
            user_id: intent.user_id,
            collection_id: Some(record.view.collection_id),
            item_id: Some(intent.target_item_id),
            kind: "import_publication".into(),
            phase: "queued".into(),
            total_units: record.view.files.iter().map(|file| file.bytes.0).sum(),
            request: serde_json::json!({"import_id":record.view.id,"guided":true}),
            frozen_input: None,
        })
    }

    pub(crate) fn reconcile_guided_ingest(&self, id: ImportId) -> Result<()> {
        if let Err(error) = self.advance_guided_ingest(id)
            && let Some(mut intent) = self.provisional_imports.guided_ingest_intent(id)?
        {
            intent.enrichment_error = Some(error.to_string().chars().take(512).collect());
            intent.updated_at = self.authentication.now();
            self.provisional_imports.save_guided_ingest_intent(intent)?;
        }
        Ok(())
    }

    pub(crate) fn advance_guided_ingest(&self, id: ImportId) -> Result<()> {
        let _guard = self
            .guided_transitions
            .lock()
            .map_err(|_| Error::Storage("guided transition lock poisoned".into()))?;
        let Some(mut intent) = self.provisional_imports.guided_ingest_intent(id)? else {
            return Ok(());
        };
        let mut record = self.provisional_imports.import_record(id)?;
        if record.user_id != intent.user_id
            || intent.cancelled
            || intent.settled
            || matches!(
                record.view.phase,
                ImportPhase::Staging | ImportPhase::Cancelled
            )
        {
            return Ok(());
        }
        self.content_write_eligibility_for_user(intent.user_id, record.view.collection_id)?;
        let item = self.ensure_guided_item(&intent, &record)?;
        if record.view.phase != ImportPhase::Published
            && record.view.association.item_id != Some(item.id)
        {
            record.view.association = CaptureAssociation {
                item_id: Some(item.id),
                ..Default::default()
            };
            record.view.expected_item_revision = Some(item.revision);
            let revision = record.view.revision;
            record = self.provisional_imports.save_import(record, revision)?;
        }
        match record.view.phase {
            ImportPhase::Analyzing => {
                let jobs = self
                    .provisional_imports
                    .import_job_records(id, intent.user_id)?;
                if jobs
                    .iter()
                    .any(|job| job.kind == "import_analysis" && job.state == JobState::Failed)
                {
                    record.view.review = Some(ImportReviewDecision {
                        identity: ImportIdentityDecision::Unidentified,
                        association: record.view.association,
                    });
                    record.view.phase = ImportPhase::AwaitingReview;
                    let revision = record.view.revision;
                    record = self.provisional_imports.save_import(record, revision)?;
                    self.enqueue_guided_publication(&intent, &record)?;
                    return Ok(());
                }
                if !jobs.iter().any(|job| job.kind == "import_analysis")
                    && let Err(error) = self.enqueue_guided_analysis(&intent, &record)
                {
                    let mut next = record.clone();
                    next.view
                        .limitations
                        .push(format!("Automatic analysis unavailable: {error}"));
                    next.view.review = Some(ImportReviewDecision {
                        identity: ImportIdentityDecision::Unidentified,
                        association: next.view.association,
                    });
                    next.view.phase = ImportPhase::AwaitingReview;
                    let revision = next.view.revision;
                    let next = self.provisional_imports.save_import(next, revision)?;
                    self.enqueue_guided_publication(&intent, &next)?;
                }
            }
            ImportPhase::AwaitingReview => {
                if record.view.review.is_none() {
                    record.view.review = Some(ImportReviewDecision {
                        identity: if record.view.candidates.len() == 1 {
                            ImportIdentityDecision::Candidate {
                                key: record.view.candidates[0].key.clone(),
                            }
                        } else {
                            ImportIdentityDecision::Unidentified
                        },
                        association: record.view.association,
                    });
                    record.view.expected_item_revision = Some(self.reader.item(item.id)?.revision);
                    let revision = record.view.revision;
                    record = self.provisional_imports.save_import(record, revision)?;
                }
                let jobs = self
                    .provisional_imports
                    .import_job_records(id, intent.user_id)?;
                if !jobs.iter().any(|job| job.kind == "import_publication") {
                    self.enqueue_guided_publication(&intent, &record)?;
                }
            }
            ImportPhase::Published if intent.enrichment == IngestEnrichmentPolicy::Automatic => {
                match self.advance_guided_enrichment(&intent, &record) {
                    Ok(done) => {
                        intent.settled = done;
                        intent.enrichment_error = None;
                        intent.updated_at = self.authentication.now();
                        self.provisional_imports.save_guided_ingest_intent(intent)?;
                    }
                    Err(error) => {
                        intent.settled = true;
                        intent.enrichment_error =
                            Some(error.to_string().chars().take(512).collect());
                        intent.updated_at = self.authentication.now();
                        self.provisional_imports.save_guided_ingest_intent(intent)?;
                    }
                }
            }
            ImportPhase::Published => {
                intent.settled = true;
                intent.enrichment_error = None;
                self.provisional_imports.save_guided_ingest_intent(intent)?;
            }
            _ => {}
        }
        Ok(())
    }

    fn advance_guided_enrichment(
        &self,
        intent: &GuidedIngestIntent,
        record: &ImportRecord,
    ) -> Result<bool> {
        let item = self.reader.item(intent.target_item_id)?;
        if self
            .reader
            .capture(record.view.capture_id)?
            .association
            .item_id
            != Some(item.id)
        {
            return Err(Error::Unavailable(
                "Automatic enrichment stopped because the Capture association changed".into(),
            ));
        }
        if item.domain.module.0 == "org.junk.games" {
            // DAT candidates are retained by Import analysis. Publication has
            // already rebound the exact observation; applying them automatically
            // is handled by the same metadata authority resolver in the follow-up
            // helper, never as catalog verification.
            self.apply_guided_dat_metadata(intent, record, &item)?;
            return Ok(true);
        }
        if item.domain.module.0 == "org.junk.music" {
            return self.advance_guided_music_metadata(intent, record, &item);
        }
        Ok(true)
    }

    fn apply_guided_dat_metadata(
        &self,
        intent: &GuidedIngestIntent,
        record: &ImportRecord,
        item: &Item,
    ) -> Result<()> {
        let Some(analysis) = record.view.analysis_job_id else {
            return Ok(());
        };
        let portable = self
            .processing
            .processing_record(ObservationId(analysis.0))?;
        let snapshot_id = portable.observation.snapshot_id.ok_or_else(|| {
            Error::Unavailable("No active DAT snapshot was available for automatic metadata".into())
        })?;
        self.refresh_provider_metadata_for_job(
            intent.user_id,
            item,
            snapshot_id,
            Some(portable.observation.id),
            vec![],
            ObservationId(super::deterministic_inventory_uuid(
                intent.user_id,
                "guided-dat-metadata",
                &record.view.id.to_string(),
            )),
        )?;
        Ok(())
    }

    fn advance_guided_music_metadata(
        &self,
        intent: &GuidedIngestIntent,
        record: &ImportRecord,
        item: &Item,
    ) -> Result<bool> {
        if !self
            .streaming_providers
            .contains_key(collector_modules::music::MUSICBRAINZ_PROVIDER_ID)
            || !self
                .catalog_parsers
                .contains_key(collector_modules::music::MUSICBRAINZ_PROVIDER_ID)
        {
            return Err(Error::Unavailable(
                "MusicBrainz is not configured; originals remain preserved".into(),
            ));
        }
        let jobs = self
            .provisional_imports
            .import_job_records(record.view.id, intent.user_id)?;
        let job_id = super::deterministic_job_id(
            intent.user_id,
            "guided.musicbrainz",
            &record.view.id.to_string(),
        );
        if let Some(job) = jobs.iter().find(|job| job.id == job_id) {
            return match job.state {
                JobState::Completed => {
                    let snapshot_id = job
                        .result
                        .as_ref()
                        .and_then(|result| result.get("snapshot_id"))
                        .and_then(serde_json::Value::as_str)
                        .and_then(|value| value.parse().ok())
                        .ok_or_else(|| {
                            Error::Storage("guided MusicBrainz job has no snapshot".into())
                        })?;
                    let authority = self.refresh_provider_metadata_for_job(
                        intent.user_id,
                        item,
                        snapshot_id,
                        None,
                        vec![],
                        ObservationId(super::deterministic_inventory_uuid(
                            intent.user_id,
                            "guided-music-metadata",
                            &record.view.id.to_string(),
                        )),
                    )?;
                    let Some(candidate) = authority.display.and_then(|display| display.candidate)
                    else {
                        return Ok(true);
                    };
                    self.advance_guided_music_artwork(intent, record, item, snapshot_id, candidate)
                }
                JobState::Failed => Err(Error::Unavailable(
                    job.error
                        .clone()
                        .unwrap_or_else(|| "MusicBrainz lookup failed".into()),
                )),
                _ => Ok(false),
            };
        }
        let analysis = record
            .view
            .analysis_job_id
            .ok_or_else(|| Error::Unavailable("music structure analysis is unavailable".into()))?;
        let observation = self
            .processing
            .processing_record(ObservationId(analysis.0))?
            .observation;
        let payload: collector_modules::music::MusicImportAnalysisOutput =
            observation.typed_result.payload.parse()?;
        let structure: collector_modules::music::CdProcessingResult = payload
            .observations
            .first()
            .ok_or_else(|| Error::Unavailable("music structure analysis is unavailable".into()))?
            .parse()?;
        let collector_modules::music::CdProcessingResult::Layout {
            disc_ids: Some(ids),
            ..
        } = structure
        else {
            return Err(Error::Unavailable(
                "the supported CD layout produced no MusicBrainz disc identifier".into(),
            ));
        };
        let disc_id = ids.musicbrainz;
        let query = collector_modules::music::MusicBrainzQuery::DiscId {
            disc_id,
            refresh: false,
        };
        let eligibility =
            self.content_write_eligibility_for_user(intent.user_id, item.collection_id)?;
        let frozen = FrozenAcquisitionJob {
            request: ProviderRequest {
                collection_id: item.collection_id,
                provider: ModuleId(collector_modules::music::MUSICBRAINZ_PROVIDER_ID.into()),
                operation: OperationId::new(
                    collector_modules::music::MUSICBRAINZ_ACQUIRE_OPERATION,
                )?,
                source_fingerprint: None,
                query: RawJson::from_value(
                    serde_json::to_value(query)
                        .map_err(|error| Error::Invalid(error.to_string()))?,
                ),
                maximum_response_bytes: 8 * 1024 * 1024,
            },
            collection_binding: eligibility.binding,
            item_id: Some(item.id),
            snapshot_id: SnapshotId(job_id.0),
            created_at: intent.created_at,
        };
        self.jobs.enqueue_job(JobSpec { id: job_id, user_id: intent.user_id, collection_id: Some(item.collection_id), item_id: Some(item.id), kind: "catalog_acquisition".into(), phase: "queued".into(), total_units: 8 * 1024 * 1024, request: serde_json::json!({"provider":collector_modules::music::MUSICBRAINZ_PROVIDER_ID,"snapshot_id":frozen.snapshot_id,"import_id":record.view.id,"guided":true}), frozen_input: Some(FrozenJobInput::Acquisition(frozen)) })?;
        Ok(false)
    }

    fn enqueue_guided_provider(
        &self,
        intent: &GuidedIngestIntent,
        record: &ImportRecord,
        item: &Item,
        step: GuidedProviderStep,
    ) -> Result<JobId> {
        let eligibility =
            self.content_write_eligibility_for_user(intent.user_id, item.collection_id)?;
        let frozen = FrozenAcquisitionJob {
            request: ProviderRequest {
                collection_id: item.collection_id,
                provider: step.provider.clone(),
                operation: step.operation,
                source_fingerprint: None,
                query: step.query,
                maximum_response_bytes: step.maximum_response_bytes,
            },
            collection_binding: eligibility.binding,
            item_id: Some(item.id),
            snapshot_id: SnapshotId(step.id.0),
            created_at: intent.created_at,
        };
        self.jobs.enqueue_job(JobSpec {
            id: step.id,
            user_id: intent.user_id,
            collection_id: Some(item.collection_id),
            item_id: Some(item.id),
            kind: "catalog_acquisition".into(),
            phase: "queued".into(),
            total_units: step.maximum_response_bytes,
            request: serde_json::json!({"provider":step.provider,"snapshot_id":frozen.snapshot_id,"import_id":record.view.id,"guided":true}),
            frozen_input: Some(FrozenJobInput::Acquisition(frozen)),
        })
    }

    fn advance_guided_music_artwork(
        &self,
        intent: &GuidedIngestIntent,
        record: &ImportRecord,
        item: &Item,
        musicbrainz_snapshot: SnapshotId,
        mut candidate: MetadataCandidateIdentity,
    ) -> Result<bool> {
        if !self
            .streaming_providers
            .contains_key(collector_modules::music::COVER_ART_ARCHIVE_PROVIDER_ID)
            || !self
                .catalog_parsers
                .contains_key(collector_modules::music::COVER_ART_ARCHIVE_PROVIDER_ID)
        {
            return Err(Error::Unavailable(
                "Cover Art Archive is not configured; metadata and originals remain available"
                    .into(),
            ));
        }
        let jobs = self
            .provisional_imports
            .import_job_records(record.view.id, intent.user_id)?;
        let listing_job = super::deterministic_job_id(
            intent.user_id,
            "guided.cover-art.listing",
            &record.view.id.to_string(),
        );
        let Some(listing) = jobs.iter().find(|job| job.id == listing_job) else {
            self.enqueue_guided_provider(
                intent,
                record,
                item,
                GuidedProviderStep {
                    id: listing_job,
                    provider: ModuleId(
                        collector_modules::music::COVER_ART_ARCHIVE_PROVIDER_ID.into(),
                    ),
                    operation: OperationId::new(
                        collector_modules::music::COVER_ART_ARCHIVE_ACQUIRE_OPERATION,
                    )?,
                    query: RawJson::from_value(
                        serde_json::to_value(
                            collector_modules::music::CoverArtArchiveQuery::Listing {
                                release_id: candidate.provider_record_id.clone(),
                            },
                        )
                        .map_err(|error| Error::Invalid(error.to_string()))?,
                    ),
                    maximum_response_bytes: 8 * 1024 * 1024,
                },
            )?;
            return Ok(false);
        };
        match listing.state {
            JobState::Failed => {
                return Err(Error::Unavailable(
                    listing
                        .error
                        .clone()
                        .unwrap_or_else(|| "Cover Art Archive listing failed".into()),
                ));
            }
            JobState::Completed => {}
            _ => return Ok(false),
        }
        let listing_snapshot: SnapshotId = listing
            .result
            .as_ref()
            .and_then(|result| result.get("snapshot_id"))
            .and_then(serde_json::Value::as_str)
            .and_then(|value| value.parse().ok())
            .ok_or_else(|| Error::Storage("guided artwork listing has no snapshot".into()))?;
        let snapshot = self.processing.snapshot(SnapshotQuery {
            collection_id: item.collection_id,
            snapshot_id: listing_snapshot,
        })?;
        // Resume from the acquired release, not a subsequently changed display choice.
        let listing_observation: collector_modules::music::CoverArtArchiveSnapshotObservation =
            snapshot.parsed_observation.parse()?;
        let collector_modules::music::CoverArtArchiveQuery::Listing { release_id } =
            listing_observation.query
        else {
            return Err(Error::Invalid(
                "guided artwork snapshot is not a listing".into(),
            ));
        };
        candidate.provider_record_id = release_id.clone();
        let parser = self
            .catalog_parsers
            .get(&snapshot.provider.0)
            .ok_or_else(|| Error::Unavailable("artwork provider parser is unavailable".into()))?;
        let mut reader = self.processing.snapshot_bytes(SnapshotQuery {
            collection_id: item.collection_id,
            snapshot_id: listing_snapshot,
        })?;
        let parsed = parser.parse_snapshot(&snapshot, reader.as_mut(), &mut || Ok(false))?;
        let mut options = parsed
            .entries
            .into_iter()
            .map(|raw| raw.parse::<collector_modules::music::CoverArtObservation>())
            .collect::<Result<Vec<_>>>()?;
        options.sort_by_key(|option| (!option.front, !option.approved, option.image_id.clone()));
        let Some(option) = options.first() else {
            return Ok(true);
        };
        if option.release_id != release_id {
            return Err(Error::Invalid(
                "artwork listing belongs to a different release".into(),
            ));
        }
        let image_job = super::deterministic_job_id(
            intent.user_id,
            "guided.cover-art.image",
            &record.view.id.to_string(),
        );
        let Some(image) = jobs.iter().find(|job| job.id == image_job) else {
            self.enqueue_guided_provider(
                intent,
                record,
                item,
                GuidedProviderStep {
                    id: image_job,
                    provider: ModuleId(
                        collector_modules::music::COVER_ART_ARCHIVE_PROVIDER_ID.into(),
                    ),
                    operation: OperationId::new(
                        collector_modules::music::COVER_ART_ARCHIVE_ACQUIRE_OPERATION,
                    )?,
                    query: RawJson::from_value(
                        serde_json::to_value(
                            collector_modules::music::CoverArtArchiveQuery::Image {
                                release_id: option.release_id.clone(),
                                image_id: option.image_id.clone(),
                            },
                        )
                        .map_err(|error| Error::Invalid(error.to_string()))?,
                    ),
                    maximum_response_bytes: 20 * 1024 * 1024,
                },
            )?;
            return Ok(false);
        };
        match image.state {
            JobState::Failed => {
                return Err(Error::Unavailable(
                    image
                        .error
                        .clone()
                        .unwrap_or_else(|| "Cover Art Archive image failed".into()),
                ));
            }
            JobState::Completed => {}
            _ => return Ok(false),
        }
        let image_snapshot: SnapshotId = image
            .result
            .as_ref()
            .and_then(|result| result.get("snapshot_id"))
            .and_then(serde_json::Value::as_str)
            .and_then(|value| value.parse().ok())
            .ok_or_else(|| Error::Storage("guided artwork image has no snapshot".into()))?;
        let asset_id = AssetId(super::deterministic_inventory_uuid(
            intent.user_id,
            "guided-cover-asset",
            &record.view.id.to_string(),
        ));
        let asset_missing = match self.content.asset_state(asset_id) {
            Ok(_) => false,
            Err(Error::NotFound) => true,
            Err(error) => return Err(error),
        };
        if asset_missing {
            let snapshot = self.processing.snapshot(SnapshotQuery {
                collection_id: item.collection_id,
                snapshot_id: image_snapshot,
            })?;
            let observation: collector_modules::music::CoverArtArchiveSnapshotObservation =
                snapshot.parsed_observation.parse()?;
            let (release_id, image_id) = match observation.query {
                collector_modules::music::CoverArtArchiveQuery::Image {
                    release_id,
                    image_id,
                } => (release_id, image_id),
                _ => {
                    return Err(Error::Invalid(
                        "guided artwork snapshot is not an image".into(),
                    ));
                }
            };
            if release_id != candidate.provider_record_id || image_id != option.image_id {
                return Err(Error::Invalid(
                    "artwork image does not match the selected listing".into(),
                ));
            }
            let bytes = super::metadata::read_exact_snapshot_bytes(
                self.processing.as_ref(),
                &snapshot,
                20 * 1024 * 1024,
            )?;
            self.content.create_asset(
                asset_id,
                CreateAssetCommand {
                    collection_id: item.collection_id,
                    subject: AssetSubject::Item { item_id: item.id },
                    kind: AssetKind::Cover,
                    visibility: Visibility::Shared,
                    alt_text: format!(
                        "Cover Art Archive image {image_id} for MusicBrainz release {release_id}"
                    ),
                },
                &bytes,
            )?;
        }
        self.refresh_provider_metadata_for_job(
            intent.user_id,
            &self.reader.item(item.id)?,
            musicbrainz_snapshot,
            None,
            vec![ProviderArtworkLink {
                provider_record_id: candidate.provider_record_id,
                asset_id,
            }],
            ObservationId(super::deterministic_inventory_uuid(
                intent.user_id,
                "guided-music-artwork",
                &record.view.id.to_string(),
            )),
        )?;
        Ok(true)
    }

    fn ingest_summary(&self, actor: &Principal, view: ImportView) -> Result<IngestSummary> {
        let intent = self
            .provisional_imports
            .guided_ingest_intent(view.id)?
            .ok_or(Error::NotFound)?;
        let jobs = self
            .provisional_imports
            .import_job_records(view.id, actor.user_id)?;
        let failed = jobs.iter().any(|job| job.state == JobState::Failed)
            || intent.enrichment_error.is_some();
        let active = jobs.iter().any(|job| {
            matches!(
                job.state,
                JobState::Queued | JobState::Running | JobState::CancelRequested
            )
        });
        let outcome = if view.phase == ImportPhase::Cancelled || intent.cancelled {
            IngestStepState::Cancelled
        } else if failed {
            IngestStepState::Failed
        } else if active || !intent.settled || !matches!(view.phase, ImportPhase::Published) {
            IngestStepState::Active
        } else {
            IngestStepState::Succeeded
        };
        let can_write = self
            .content_write_eligibility(actor, view.collection_id)
            .is_ok();
        let capture_revision = if view.phase == ImportPhase::Published {
            Some(self.reader.capture(view.capture_id)?.revision)
        } else {
            None
        };
        Ok(IngestSummary {
            import_id: view.id,
            collection_id: view.collection_id,
            package_name: view.source.package_name.clone(),
            file_count: view.files.len().try_into().unwrap_or(u32::MAX),
            total_bytes: ByteLength(view.files.iter().map(|file| file.bytes.0).sum()),
            target_item_id: if view.phase == ImportPhase::Published {
                self.reader.capture(view.capture_id)?.association.item_id
            } else {
                Some(intent.target_item_id)
            },
            capture_id: view.capture_id,
            capture_revision,
            phase: view.phase,
            outcome,
            enrichment: intent.enrichment,
            updated_at: view.updated_at,
            permissions: IngestPermissions {
                can_cancel: can_write
                    && !intent.cancelled
                    && (active || !intent.settled)
                    && view.phase != ImportPhase::Cancelled,
                can_discard: can_write
                    && self
                        .provisional_imports
                        .guided_cleanup_status(view.id)?
                        .is_none()
                    && !matches!(view.phase, ImportPhase::Publishing | ImportPhase::Published),
                can_correct_association: can_write && view.phase == ImportPhase::Published,
            },
        })
    }

    pub fn ingest_detail(&self, actor: &Principal, id: ImportId) -> Result<IngestDetail> {
        let mut record = self.authorized_provisional_import(actor, id)?;
        if record.view.phase == ImportPhase::Published {
            record.view.association = self.reader.capture(record.view.capture_id)?.association;
        }
        if self.provisional_imports.guided_ingest_intent(id)?.is_none() {
            return Err(Error::NotFound);
        }
        let summary = self.ingest_summary(actor, record.view.clone())?;
        let jobs = self
            .provisional_imports
            .import_job_records(id, actor.user_id)?;
        let intent = self.provisional_imports.guided_ingest_intent(id)?.unwrap();
        let mut attempts = Vec::new();
        for job in &jobs {
            if let Some(previous) = job
                .result
                .as_ref()
                .and_then(|value| value.get("prior_failures"))
                .and_then(serde_json::Value::as_array)
            {
                for failure in previous {
                    if let (Some(attempt), Some(failed_at)) = (
                        failure.get("attempt").and_then(serde_json::Value::as_u64),
                        failure.get("failed_at").and_then(serde_json::Value::as_i64),
                    ) {
                        attempts.push(IngestAttemptView {
                            job_id: job.id,
                            step: job.kind.clone(),
                            attempt: attempt as u32,
                            state: JobState::Failed,
                            started_at: None,
                            finished_at: chrono::DateTime::from_timestamp(failed_at, 0),
                            diagnostic: failure
                                .get("error")
                                .and_then(serde_json::Value::as_str)
                                .map(|value| value.chars().take(512).collect()),
                        });
                    }
                }
            }
            attempts.push(IngestAttemptView {
                job_id: job.id,
                step: job.kind.clone(),
                attempt: job.attempt,
                state: job.state.clone(),
                started_at: None,
                finished_at: matches!(
                    job.state,
                    JobState::Completed | JobState::Failed | JobState::Cancelled
                )
                .then_some(job.updated_at),
                diagnostic: job
                    .error
                    .clone()
                    .map(|value| value.chars().take(512).collect()),
            });
        }
        let mut by_kind = BTreeMap::new();
        for job in &jobs {
            by_kind.insert(job.kind.as_str(), job);
        }
        let job_step =
            |name: &str, job: Option<&JobRecord>, skipped: bool, message: Option<String>| {
                if skipped {
                    return IngestStepView {
                        step: name.into(),
                        state: IngestStepState::Skipped,
                        job_id: None,
                        attempt: None,
                        message,
                        retryable: false,
                    };
                }
                let Some(job) = job else {
                    return IngestStepView {
                        step: name.into(),
                        state: IngestStepState::Pending,
                        job_id: None,
                        attempt: None,
                        message,
                        retryable: false,
                    };
                };
                IngestStepView {
                    step: name.into(),
                    state: match job.state {
                        JobState::Queued => IngestStepState::Pending,
                        JobState::Running | JobState::CancelRequested => IngestStepState::Active,
                        JobState::Completed => IngestStepState::Succeeded,
                        JobState::Failed => IngestStepState::Failed,
                        JobState::Cancelled => IngestStepState::Cancelled,
                    },
                    job_id: Some(job.id),
                    attempt: Some(job.attempt),
                    message: job.error.clone().or(message),
                    retryable: !intent.cancelled && job.state == JobState::Failed,
                }
            };
        let owner = match self.reader.item(intent.target_item_id) {
            Ok(item) => Some(item.domain.module),
            Err(Error::NotFound) => {
                match self.reader.collection(record.view.collection_id)?.binding {
                    CollectionBinding::Assigned { module } => Some(module),
                    CollectionBinding::LegacyUnassigned => None,
                }
            }
            Err(error) => return Err(error),
        };
        let music = owner
            .as_ref()
            .is_some_and(|module| module.0 == "org.junk.music");
        let metadata_job_id = super::deterministic_job_id(
            intent.user_id,
            "guided.musicbrainz",
            &record.view.id.to_string(),
        );
        let artwork_listing_job_id = super::deterministic_job_id(
            intent.user_id,
            "guided.cover-art.listing",
            &record.view.id.to_string(),
        );
        let artwork_image_job_id = super::deterministic_job_id(
            intent.user_id,
            "guided.cover-art.image",
            &record.view.id.to_string(),
        );
        let metadata_job = jobs.iter().find(|job| job.id == metadata_job_id);
        let artwork_listing_job = jobs.iter().find(|job| job.id == artwork_listing_job_id);
        let artwork_image_job = jobs.iter().find(|job| job.id == artwork_image_job_id);
        let metadata_step = if intent.enrichment == IngestEnrichmentPolicy::None {
            job_step("metadata", None, true, None)
        } else if let Some(error) = intent.enrichment_error.clone() {
            IngestStepView {
                step: "metadata".into(),
                state: IngestStepState::Failed,
                job_id: metadata_job.map(|job| job.id),
                attempt: metadata_job.map(|job| job.attempt),
                message: Some(error),
                retryable: !intent.cancelled
                    && metadata_job.is_some_and(|job| job.state == JobState::Failed),
            }
        } else if music && metadata_job.is_some() {
            job_step("metadata", metadata_job, false, None)
        } else if music {
            job_step("metadata", None, false, None)
        } else {
            IngestStepView {
                step: "metadata".into(),
                state: if record.view.phase == ImportPhase::Published {
                    IngestStepState::Succeeded
                } else {
                    IngestStepState::Pending
                },
                job_id: None,
                attempt: None,
                message: None,
                retryable: false,
            }
        };
        let artwork_step = if intent.enrichment == IngestEnrichmentPolicy::None || !music {
            job_step("artwork", None, true, None)
        } else if let Some(error) = intent.enrichment_error.clone() {
            IngestStepView {
                step: "artwork".into(),
                state: IngestStepState::Failed,
                job_id: artwork_image_job.or(artwork_listing_job).map(|job| job.id),
                attempt: artwork_image_job
                    .or(artwork_listing_job)
                    .map(|job| job.attempt),
                message: Some(error),
                retryable: !intent.cancelled
                    && artwork_image_job
                        .or(artwork_listing_job)
                        .is_some_and(|job| job.state == JobState::Failed),
            }
        } else {
            job_step(
                "artwork",
                artwork_image_job.or(artwork_listing_job),
                intent.settled && artwork_image_job.is_none(),
                (intent.settled && artwork_image_job.is_none())
                    .then(|| "No automatic artwork was selected".into()),
            )
        };
        let staging_step = if let Some(job) = by_kind.get("import_staging") {
            job_step("staging", Some(job), false, intent.enrichment_error.clone())
        } else {
            IngestStepView {
                step: "staging".into(),
                state: if record
                    .view
                    .files
                    .iter()
                    .all(|file| file.received_bytes == file.bytes)
                {
                    IngestStepState::Succeeded
                } else {
                    IngestStepState::NeedsInput
                },
                job_id: by_kind.get("import_staging").map(|job| job.id),
                attempt: by_kind.get("import_staging").map(|job| job.attempt),
                message: (record.view.phase == ImportPhase::Staging)
                    .then(|| intent.enrichment_error.clone())
                    .flatten(),
                retryable: false,
            }
        };
        let steps = vec![
            staging_step,
            job_step(
                "identification",
                by_kind.get("import_analysis").copied(),
                record.view.phase == ImportPhase::Published
                    && !by_kind.contains_key("import_analysis"),
                record.view.limitations.first().cloned(),
            ),
            job_step(
                "preservation",
                by_kind.get("import_publication").copied(),
                false,
                None,
            ),
            metadata_step,
            artwork_step,
        ];
        Ok(IngestDetail {
            summary,
            import: record.view,
            steps,
            attempts,
            cleanup: self.provisional_imports.guided_cleanup_status(id)?,
        })
    }

    pub fn ingest_page(
        &self,
        actor: &Principal,
        collection: Option<CollectionId>,
        cursor: Option<ImportId>,
        limit: u16,
    ) -> Result<Page<IngestSummary>> {
        if let Some(collection) = collection {
            self.authorize(actor, collection, Action::Curate)?;
        }
        let page = self.provisional_imports.guided_import_records(
            actor.user_id,
            collection,
            cursor,
            limit,
        )?;
        Ok(Page {
            items: page
                .items
                .into_iter()
                .map(|record| {
                    self.authorize(actor, record.view.collection_id, Action::Curate)?;
                    self.ingest_summary(actor, record.view)
                })
                .collect::<Result<Vec<_>>>()?,
            next_cursor: page.next_cursor,
        })
    }

    pub fn discard_ingest(
        &self,
        actor: &Principal,
        id: ImportId,
        expected_revision: u64,
        context: &CommandContext,
    ) -> Result<IngestDiscardResult> {
        let _guard = self
            .guided_transitions
            .lock()
            .map_err(|_| Error::Storage("guided transition lock poisoned".into()))?;
        self.provisional_import_for_write(actor, id)?;
        self.execute_command(
            actor,
            context,
            "ingest.discard",
            &(id, expected_revision),
            |effects| effects.discard_guided_import(id, expected_revision),
        )
    }

    pub fn cancel_ingest(
        &self,
        actor: &Principal,
        id: ImportId,
        expected_revision: u64,
        context: &CommandContext,
    ) -> Result<IngestDetail> {
        let _guard = self
            .guided_transitions
            .lock()
            .map_err(|_| Error::Storage("guided transition lock poisoned".into()))?;
        self.provisional_import_for_write(actor, id)?;
        self.execute_command(
            actor,
            context,
            "ingest.cancel",
            &(id, expected_revision),
            |effects| effects.cancel_guided_import(id, expected_revision),
        )?;
        self.ingest_detail(actor, id)
    }

    pub fn retry_ingest(
        &self,
        actor: &Principal,
        id: ImportId,
        command: RetryIngestCommand,
        context: &CommandContext,
    ) -> Result<IngestDetail> {
        let (record, _) = self.provisional_import_for_write(actor, id)?;
        let mut intent = self
            .provisional_imports
            .guided_ingest_intent(id)?
            .ok_or(Error::NotFound)?;
        if let Some(job_id) = command.job_id {
            let jobs = self
                .provisional_imports
                .import_job_records(id, actor.user_id)?;
            let job = jobs
                .iter()
                .find(|job| job.id == job_id)
                .ok_or(Error::NotFound)?;
            self.execute_command(actor, context, "ingest.retry", &(id, &command), |effects| {
                if job.state != JobState::Failed {
                    return Err(Error::Conflict);
                }
                effects.retry_failed_job(job_id, actor.user_id)
            })?;
        } else {
            intent.enrichment_error = None;
            intent.settled = false;
            intent.updated_at = self.authentication.now();
            self.execute_command(actor, context, "ingest.retry", &(id, &command), |effects| {
                if intent.cancelled || record.view.phase == ImportPhase::Cancelled {
                    return Err(Error::Conflict);
                }
                effects.save_guided_ingest_intent(intent)
            })?;
            self.reconcile_guided_ingest(record.view.id)?;
        }
        self.ingest_detail(actor, id)
    }

    pub fn correct_ingest_association(
        &self,
        actor: &Principal,
        id: ImportId,
        command: CorrectIngestAssociationCommand,
        context: &CommandContext,
    ) -> Result<IngestDetail> {
        let _guard = self
            .guided_transitions
            .lock()
            .map_err(|_| Error::Storage("guided transition lock poisoned".into()))?;
        let (record, _) = self.provisional_import_for_write(actor, id)?;
        if record.view.phase != ImportPhase::Published {
            return Err(Error::Conflict);
        }
        self.execute_command(
            actor,
            context,
            "ingest.association.correct",
            &(id, &command),
            |effects| {
                let target = self
                    .validate_import_association(record.view.collection_id, command.association)?;
                if target.is_some() != command.expected_item_revision.is_some() {
                    return Err(Error::Invalid(
                        "an Item association requires its expected revision".into(),
                    ));
                }
                if let (Some(item), Some(expected)) = (target, command.expected_item_revision)
                    && item.revision != expected
                {
                    return Err(Error::Conflict);
                }
                let mut capture = self.reader.capture(record.view.capture_id)?;
                if capture.revision != command.expected_capture_revision {
                    return Err(Error::Conflict);
                }
                capture.association = command.association;
                effects.save_capture(capture, command.expected_capture_revision)
            },
        )?;
        self.ingest_detail(actor, id)
    }
}
