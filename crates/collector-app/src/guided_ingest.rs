use super::*;
use std::collections::{BTreeMap, BTreeSet};

fn completed_snapshot_id(result: Option<&serde_json::Value>) -> Option<SnapshotId> {
    let result = result?;
    result
        .get("completion")
        .unwrap_or(result)
        .get("snapshot_id")
        .and_then(serde_json::Value::as_str)
        .and_then(|value| value.parse().ok())
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

/// Narrow policy for new intake only. Unknown and other hidden files are originals.
/// Adapters pass `directory` when deciding whether to descend into a directory.
pub fn is_intake_housekeeping(name: &str, directory: bool) -> bool {
    let parts = name.split('/').collect::<Vec<_>>();
    parts.iter().enumerate().any(|(index, part)| {
        matches!(*part, "#recycle" | "@eaDir") && (directory || index + 1 < parts.len())
    }) || (!directory && parts.last() == Some(&".DS_Store"))
}

/// Reserved direct inbox recovery directory; never a generic package-file filter.
pub fn is_inbox_cleanup_directory(name: &str) -> bool {
    name.strip_prefix(".collector-cleanup-")
        .is_some_and(|id| id.parse::<ImportId>().is_ok())
}

fn validate_plan_files(files: &[UploadFile]) -> Result<()> {
    validate_intake_files(files, 10_000)
}
pub(crate) fn validate_intake_files(files: &[UploadFile], limit: usize) -> Result<()> {
    if files.is_empty() || files.len() > limit {
        return Err(Error::Invalid(format!(
            "Inventory must contain 1–{limit} files. Split larger selections into another intake."
        )));
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
        self.provisional_imports.reconcile_removed_item_work()?;
        let cleanup_result = self.provisional_imports.reconcile_guided_cleanup();
        self.jobs
            .reconcile_work_history(&mut |user, target| self.settle_work_clear(user, target))?;
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

    pub fn discover_ingest(
        &self,
        actor: &Principal,
        mut command: DiscoverIngestCommand,
    ) -> Result<DiscoveredIngestPlan> {
        let eligibility = self.content_write_eligibility(actor, command.collection_id)?;
        validate_plan_files(&command.files)?;
        command
            .files
            .retain(|file| !is_intake_housekeeping(&file.name, false));
        let names = command
            .files
            .iter()
            .map(|file| PackageFileName::new(file.name.clone()))
            .collect::<Result<Vec<_>>>()?;
        let discovered = self.modules.discover_imports(&eligibility.owner, &names)?;
        let mut used = BTreeSet::new();
        let mut packages = Vec::new();
        for package in discovered {
            let mut files = Vec::new();
            for index in package.file_indices {
                let file = command.files.get(index).ok_or_else(|| {
                    Error::Invalid("module returned an invalid package member".into())
                })?;
                if !used.insert(index) {
                    return Err(Error::Invalid(
                        "module returned overlapping packages".into(),
                    ));
                }
                files.push(file.clone());
            }
            packages.push(IngestPackageSelection {
                purchase: None,
                source: ImportSourceSummary {
                    kind: ImportSourceKind::Browser,
                    label: None,
                    package_name: package.label,
                },
                files,
                anchor_file_index: None,
            });
        }
        let unassigned_files = command
            .files
            .iter()
            .enumerate()
            .filter(|(index, _)| !used.contains(index))
            .map(|(_, file)| file.name.clone())
            .collect();
        let plan = if packages.is_empty() {
            None
        } else {
            Some(self.plan_ingest(
                actor,
                PlanIngestCommand {
                    collection_id: command.collection_id,
                    packages,
                },
            )?)
        };
        Ok(DiscoveredIngestPlan {
            plan,
            unassigned_files,
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
            let mut package = package.clone();
            validate_plan_files(&package.files)?;
            if let Some(selected) = package.anchor_file_index {
                let selected_name = package
                    .files
                    .get(selected)
                    .ok_or_else(|| Error::Invalid("Invalid anchor file index".into()))?
                    .name
                    .clone();
                package
                    .files
                    .retain(|file| !is_intake_housekeeping(&file.name, false));
                package.anchor_file_index = Some(
                    package
                        .files
                        .iter()
                        .position(|file| file.name == selected_name)
                        .ok_or_else(|| {
                            Error::Invalid(
                                "A housekeeping file cannot be an analysis anchor".into(),
                            )
                        })?,
                );
            } else {
                package
                    .files
                    .retain(|file| !is_intake_housekeeping(&file.name, false));
            }
            let digital = eligibility.owner.0 == "org.junk.music"
                && collector_modules::music::files::is_file_package(
                    &package
                        .files
                        .iter()
                        .map(|f| PackageFileName::new(f.name.clone()))
                        .collect::<Result<Vec<_>>>()?,
                );
            for file in &mut package.files {
                if file.visibility.is_none() {
                    file.visibility = Some(default_import_visibility(digital, &file.name));
                }
            }
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
                    package.anchor_file_index,
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
                !self
                    .processing
                    .active_game_snapshots(command.collection_id)?
                    .is_empty()
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
            if let Some(purchase) = &package.purchase {
                purchase.validate()?;
            }
            let file_audio = eligibility.owner.0 == "org.junk.music"
                && collector_modules::music::files::is_file_package(
                    &package
                        .files
                        .iter()
                        .map(|f| PackageFileName::new(f.name.clone()))
                        .collect::<Result<Vec<_>>>()?,
                );
            if package.purchase.is_some() && !file_audio {
                return Err(Error::Invalid(
                    "Purchase facts apply to digital music packages.".into(),
                ));
            }
            let analysis_blocked = blockers.iter().any(|issue| issue.blocks_analysis);
            let (preparation_available, preparation_reason) = if eligibility.owner.0
                == "org.junk.music"
            {
                if file_audio {
                    (false, Some("Purchased audio plays directly after import; preparation is unnecessary.".into()))
                } else if analysis_blocked {
                    (
                        false,
                        Some("Choose a usable CD anchor before preparing this package.".into()),
                    )
                } else {
                    match self.encoder_availability(&ConversionProfile::Flac) {
                        EncoderAvailability::Available { .. } => (true, None),
                        EncoderAvailability::Unavailable { reason } => (false, Some(reason)),
                    }
                }
            } else {
                (
                    false,
                    Some("Optional album preparation applies only to music collections.".into()),
                )
            };
            packages.push(IngestPlanPackage {
                purchase: package.purchase.clone(),
                music_actions_available: eligibility.owner.0 == "org.junk.music",
                position: position
                    .try_into()
                    .map_err(|_| Error::Invalid("too many packages".into()))?,
                source: package.source.clone(),
                files: package.files.clone(),
                default_title,
                anchors,
                blockers,
                analysis_available,
                metadata_available: file_audio || metadata_available,
                artwork_available: file_audio || artwork_available,
                preparation_available,
                preparation_reason,
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
        self.authorize(actor, command.plan.collection_id, Action::Curate)?;
        if let Some(purchase) = &command.purchase {
            purchase.validate()?;
        }
        if command.purchase.is_some()
            && command.plan.packages.iter().any(|package| {
                !package.music_actions_available
                    || package.anchors.iter().any(|a| a.selected)
                    || !collector_modules::music::files::is_file_package(
                        &package
                            .files
                            .iter()
                            .map(|f| PackageFileName::new(f.name.clone()))
                            .collect::<Result<Vec<_>>>()
                            .unwrap_or_default(),
                    )
            })
        {
            return Err(Error::Invalid(
                "Purchase facts apply to digital music packages.".into(),
            ));
        }

        if command.destination.is_some() && command.plan.packages.len() != 1 {
            return Err(Error::Invalid(
                "an existing destination accepts one package at a time".into(),
            ));
        }
        if command.analysis == IngestAnalysisPolicy::PreserveOnly
            && (command.enrichment != IngestEnrichmentPolicy::None || command.prepare_music)
        {
            return Err(Error::Invalid(
                "preservation without analysis cannot request enrichment or music preparation"
                    .into(),
            ));
        }
        if command.clean_reimport.is_some()
            && (command.destination.is_none() || command.plan.packages.len() != 1)
        {
            return Err(Error::Invalid(
                "clean reimport requires one package and an existing destination".into(),
            ));
        }
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
                            purchase: package.purchase.clone(),
                            source: package.source.clone(),
                            files: package.files.clone(),
                            anchor_file_index: package
                                .anchors
                                .iter()
                                .find(|anchor| anchor.selected)
                                .map(|anchor| anchor.file_index),
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
            if command.prepare_music
                && let Some(package) = replanned
                    .packages
                    .iter()
                    .find(|package| !package.preparation_available)
            {
                return Err(Error::Unavailable(
                    package.preparation_reason.clone().unwrap_or_else(|| {
                        "Optional music preparation is unavailable for this package.".into()
                    }),
                ));
            }
            let (records, intents) =
                self.build_guided_import_records(actor.user_id, &command, context.key())?;
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

    pub(crate) fn build_guided_import_records(
        &self,
        user: UserId,
        command: &StartIngestCommand,
        key: &str,
    ) -> Result<(Vec<ImportRecord>, Vec<GuidedIngestIntent>)> {
        let eligibility =
            self.content_write_eligibility_for_user(user, command.plan.collection_id)?;
        let now = self.authentication.now();
        if let Some(destination) = &command.destination {
            let item = self.reader.item(destination.item_id)?;
            if item.deleted
                || item.collection_id != command.plan.collection_id
                || item.domain.module != eligibility.owner
                || (command.clean_reimport.is_none()
                    && item.revision != destination.expected_revision)
            {
                return Err(Error::Conflict);
            }
        }
        let mut records = Vec::new();
        let mut intents = Vec::new();
        for package in &command.plan.packages {
            let key = format!("{}:{}", key, package.position);
            let import_id = ImportId(super::deterministic_inventory_uuid(
                user,
                "guided-import",
                &key,
            ));
            let target_item_id = command
                .destination
                .as_ref()
                .map(|value| value.item_id)
                .unwrap_or_else(|| {
                    ItemId(super::deterministic_inventory_uuid(
                        user,
                        "guided-item",
                        &key,
                    ))
                });
            let capture_id = CaptureId(super::deterministic_inventory_uuid(
                user,
                "guided-capture",
                &key,
            ));
            let representation_id = RepresentationId(super::deterministic_inventory_uuid(
                user,
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
                        visibility: file.visibility.unwrap_or(Visibility::Shared),
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
                user_id: user,
                view: ImportView {
                    id: import_id,
                    revision: 1,
                    collection_id: command.plan.collection_id,
                    source: package.source.clone(),
                    files,
                    capture_id,
                    representation_id,
                    association: CaptureAssociation {
                        item_id: command.destination.as_ref().map(|value| value.item_id),
                        ..Default::default()
                    },
                    expected_item_revision: command
                        .destination
                        .as_ref()
                        .map(|value| value.expected_revision),
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
                cd_check_settings: if eligibility.owner.0 == "org.junk.music"
                    && command.analysis != IngestAnalysisPolicy::PreserveOnly
                    && package.anchors.iter().any(|anchor| anchor.selected)
                    && package.purchase.is_none()
                    && command.purchase.is_none()
                {
                    let settings = self.care.cd_settings(command.plan.collection_id)?;
                    settings.check_accuraterip_on_ingest.then_some(settings)
                } else {
                    None
                },
                cd_check_error: None,
                purchase: package
                    .purchase
                    .clone()
                    .or_else(|| command.purchase.clone()),
                clean_reimport: command.clean_reimport.clone(),
                bulk_context: None,
                import_id,
                user_id: user,
                plan_token: command.plan.token.clone(),
                target_item_id,
                created_item: command.destination.is_none(),
                default_title: package.default_title.clone(),
                anchor_file_index: package
                    .anchors
                    .iter()
                    .find(|anchor| anchor.selected)
                    .map(|anchor| anchor.file_index),
                analysis: command.analysis,
                enrichment: command.enrichment,
                enrichment_error: None,
                prepare_music: command.prepare_music,
                preparation_batch_id: None,
                preparation_error: None,
                settled: false,
                cancelled: false,
                created_at: now,
                updated_at: now,
            });
        }
        Ok((records, intents))
    }

    fn ensure_guided_item(
        &self,
        intent: &GuidedIngestIntent,
        record: &ImportRecord,
    ) -> Result<Item> {
        let eligibility =
            self.content_write_eligibility_for_user(intent.user_id, record.view.collection_id)?;
        let digital = eligibility.owner.0 == "org.junk.music"
            && record
                .view
                .files
                .iter()
                .any(|f| collector_modules::music::files::is_audio_name(&f.name))
            && !record.view.files.iter().any(|f| {
                f.name.to_ascii_lowercase().ends_with(".cue")
                    || f.name.to_ascii_lowercase().ends_with(".toc")
            });
        let mut item = match self.reader.item(intent.target_item_id) {
            Ok(item) => {
                if item.deleted || item.collection_id != record.view.collection_id {
                    return Err(Error::Conflict);
                }
                self.modules.inspect(&item.domain).map_err(Error::from)?;
                item
            }
            Err(Error::NotFound) => {
                if !intent.created_item {
                    return Err(Error::NotFound);
                }
                let eligibility = self.content_write_eligibility_for_user(
                    intent.user_id,
                    record.view.collection_id,
                )?;
                let mut item = Item {
                    lifecycle: Default::default(),
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
                    specimens: vec![Specimen {
                        id: intent
                            .bulk_context
                            .as_ref()
                            .map(|context| context.copy_id)
                            .unwrap_or_else(|| {
                                SpecimenId(super::deterministic_inventory_uuid(
                                    intent.user_id,
                                    "guided-import-copy",
                                    &intent.import_id.to_string(),
                                ))
                            }),
                        label: intent
                            .bulk_context
                            .as_ref()
                            .map(|context| context.copy_label.clone())
                            .unwrap_or_else(|| "Imported copy".into()),
                        condition: Condition::Unknown,
                        ownership: Ownership::Owned,
                        possession: Possession::Present,
                        components: vec![],
                        acquisition: None,
                        private_notes: None,
                        loans: vec![],
                    }],
                    wishlist: false,
                    captures: vec![],
                    assets: vec![],
                    metadata: MetadataState::default(),
                    deleted: false,
                    updated_at: intent.created_at,
                };
                if digital {
                    item.specimens.clear();
                }
                self.modules.inspect(&item.domain).map_err(Error::from)?;
                self.writer.save(item, Revision(0))?
            }
            Err(error) => return Err(error),
        };
        if let Some(context) = &intent.bulk_context
            && !digital
            && !item.specimens.iter().any(|copy| copy.id == context.copy_id)
        {
            let expected = item.revision;
            item.specimens.push(Specimen {
                id: context.copy_id,
                label: context.copy_label.clone(),
                condition: Condition::Unknown,
                ownership: Ownership::Owned,
                possession: Possession::Present,
                components: vec![],
                acquisition: None,
                private_notes: None,
                loans: vec![],
            });
            item = self.writer.save(item, expected)?;
        }
        if intent.created_item
            && self
                .processing
                .metadata_authority(item.collection_id, item.id)?
                .is_none()
        {
            let mut authority = MetadataAuthority::default();
            if item.title != intent.default_title
                || intent
                    .bulk_context
                    .as_ref()
                    .and_then(|context| context.manual_title.as_ref())
                    .is_some()
            {
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
                    resolution_policy: 4,
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
                None,
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
                intent.anchor_file_index,
            )
            .map_err(Error::from)?;
        if let Some(issue) = readiness
            .blockers
            .iter()
            .find(|issue| issue.blocks_analysis)
        {
            return Err(Error::Unavailable(issue.message.clone()));
        }
        let fingerprint = self
            .processing
            .recorded_source(record.view.collection_id, inventory.scope)?;
        let snapshot_id = self.processing.active_snapshot(
            record.view.collection_id,
            &ModuleId("org.junk.providers.local-dat".into()),
        )?;
        let job = super::deterministic_job_id(
            intent.user_id,
            "guided.import.analysis",
            &record.view.id.to_string(),
        );
        let mut selection = serde_json::json!({
            "import_id":record.view.id,"representation_id":record.view.representation_id,
            "files":record.view.files,"snapshot_id":snapshot_id,"system":null,"domain":null
        });
        if let Some(anchor_file_index) = intent.anchor_file_index {
            selection
                .as_object_mut()
                .expect("guided analysis selection is an object")
                .insert("anchor_file_index".into(), anchor_file_index.into());
        }
        let selected = self
            .modules
            .scan_inputs(&capability, &inventory, &RawJson::from_value(selection))
            .map_err(Error::from)?;
        if !selected.blockers.is_empty() {
            return Err(Error::Invalid(selected.blockers.join(" ")));
        }
        let frozen = FrozenProcessingJob {
            collection_id: record.view.collection_id,
            collection_binding: eligibility.binding,
            operation: capability,
            operation_version: contract.schema_version,
            source: PackageScope::Import(record.view.id),
            source_fingerprint: fingerprint,
            snapshot_id,
            observation_id: ObservationId(job.0),
            selection: selected.selection,
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
        if let Some(intent) = self.provisional_imports.guided_ingest_intent(id)? {
            self.retire_reassigned_placeholder(&intent)?;
        }
        Ok(())
    }

    /// The committed association and original intent are the durable cleanup
    /// request. Reconciliation retries the gap after the association receipt.
    fn retire_reassigned_placeholder(&self, intent: &GuidedIngestIntent) -> Result<()> {
        if !intent.created_item || (!intent.settled && !intent.cancelled) {
            return Ok(());
        }
        let record = self.provisional_imports.import_record(intent.import_id)?;
        if record.view.phase != ImportPhase::Published {
            return Ok(());
        }
        let capture = self.reader.capture(record.view.capture_id)?;
        if capture.association.item_id.is_none()
            || capture.association.item_id == Some(intent.target_item_id)
        {
            return Ok(());
        }
        if self
            .content_write_eligibility_for_user(intent.user_id, record.view.collection_id)
            .is_err()
        {
            return Ok(());
        }
        let item = self.reader.item(intent.target_item_id)?;
        let automatic_copy = item.specimens.len() == 1
            && item.specimens[0].id
                == SpecimenId(super::deterministic_inventory_uuid(
                    intent.user_id,
                    "guided-import-copy",
                    &intent.import_id.to_string(),
                ))
            && item.specimens[0].label == "Imported copy"
            && matches!(item.specimens[0].condition, Condition::Unknown)
            && matches!(item.specimens[0].ownership, Ownership::Owned)
            && item.specimens[0].possession == Possession::Present
            && item.specimens[0].components.is_empty()
            && item.specimens[0].acquisition.is_none()
            && item.specimens[0].private_notes.is_none()
            && item.specimens[0].loans.is_empty();
        if item.deleted
            || !item.captures.is_empty()
            || (!item.specimens.is_empty() && !automatic_copy)
            || item.wishlist
            || !item.assets.is_empty()
            || item.title != intent.default_title
            || !item.subtitle.is_empty()
            || item.identity_link != ItemIdentityLink::None
            || item.identity != IdentityState::Unidentified
            || !item.translations.is_empty()
            || !item.identifiers.is_empty()
            || item.domain.payload.parse::<serde_json::Value>()? != serde_json::json!({})
            || serde_json::to_value(&item.metadata).map_err(|e| Error::Storage(e.to_string()))?
                != serde_json::to_value(MetadataState::default())
                    .map_err(|e| Error::Storage(e.to_string()))?
        {
            return Ok(());
        }
        let authority = self
            .processing
            .metadata_authority(item.collection_id, item.id)?;
        if let Some(record) = &authority {
            let authority = metadata::metadata_authority_for_inventory(&item, record)?;
            if serde_json::to_value(authority).map_err(|e| Error::Storage(e.to_string()))?
                != serde_json::to_value(MetadataAuthority::default())
                    .map_err(|e| Error::Storage(e.to_string()))?
            {
                return Ok(());
            }
        }
        match self
            .writer
            .retire_item_if_unchanged(item, authority.map(|value| value.revision))
        {
            Ok(()) | Err(Error::Conflict) => Ok(()),
            Err(error) => Err(error),
        }
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
        if matches!(
            record.view.phase,
            ImportPhase::Staging | ImportPhase::Analyzing
        ) && !intent.cancelled
            && let Some(representation) = intent
                .clean_reimport
                .as_ref()
                .and_then(|options| options.retained_representation)
        {
            self.content_write_eligibility_for_user(intent.user_id, record.view.collection_id)?;
            self.jobs.enqueue_job(JobSpec {
                    id: deterministic_job_id(intent.user_id, "guided.retained.stage", &id.to_string()),
                    user_id: intent.user_id, collection_id: Some(record.view.collection_id), item_id: Some(intent.target_item_id),
                    kind: "import_staging".into(), phase: "queued".into(), total_units: record.view.files.iter().map(|file|file.bytes.0).sum(),
                    request: serde_json::json!({"import_id":id,"retained_representation_id":representation,"guided":true}), frozen_input: None,
                })?;
        }
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
        if intent
            .clean_reimport
            .as_ref()
            .and_then(|options| options.retained_representation)
            .is_some()
        {
            let staging =
                deterministic_job_id(intent.user_id, "guided.retained.stage", &id.to_string());
            if self.jobs.job_record(staging, intent.user_id)?.state != JobState::Completed {
                return Ok(());
            }
        }
        self.content_write_eligibility_for_user(intent.user_id, record.view.collection_id)?;
        let item = self.ensure_guided_item(&intent, &record)?;
        if record.view.phase != ImportPhase::Published
            && record.view.association.item_id != Some(item.id)
        {
            record.view.association = CaptureAssociation {
                item_id: Some(item.id),
                specimen_id: intent
                    .bulk_context
                    .as_ref()
                    .filter(|context| item.specimens.iter().any(|copy| copy.id == context.copy_id))
                    .map(|context| context.copy_id),
                ..Default::default()
            };
            if intent.created_item || intent.bulk_context.is_some() {
                record.view.expected_item_revision = Some(item.revision);
            }
            let revision = record.view.revision;
            record = self.provisional_imports.save_import(record, revision)?;
        }
        if let Some(context) = &intent.bulk_context
            && item.specimens.iter().any(|copy| copy.id == context.copy_id)
            && record.view.association.specimen_id != Some(context.copy_id)
            && record.view.phase != ImportPhase::Published
        {
            record.view.association.specimen_id = Some(context.copy_id);
            record.view.expected_item_revision = Some(item.revision);
            let revision = record.view.revision;
            record = self.provisional_imports.save_import(record, revision)?;
        }
        match record.view.phase {
            ImportPhase::Analyzing => {
                if intent.analysis == IngestAnalysisPolicy::PreserveOnly
                    || (item.domain.module.0 == "org.junk.games"
                        && !record
                            .view
                            .files
                            .iter()
                            .any(|file| file.name.to_ascii_lowercase().ends_with(".cue")))
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
                    if intent.created_item || intent.bulk_context.is_some() {
                        record.view.expected_item_revision =
                            Some(self.reader.item(item.id)?.revision);
                    }
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
            ImportPhase::Published => {
                if let Some(context) = &intent.bulk_context
                    && !self.intake_port.intake_album_preservation_settled(
                        intent.user_id,
                        context.intake_id,
                        intent.target_item_id,
                    )?
                {
                    return Ok(());
                }
                let current_item = self.reader.item(intent.target_item_id)?;
                if current_item.domain.module.0 == "org.junk.music"
                    && intent.enrichment_error.is_none()
                {
                    self.apply_file_tags(&intent, &record, &current_item)?;
                }
                let enrichment_done = if intent.enrichment == IngestEnrichmentPolicy::None
                    || intent.enrichment_error.is_some()
                {
                    true
                } else {
                    match self.advance_guided_enrichment(&intent, &record) {
                        Ok(done) => done,
                        Err(error) => {
                            intent.enrichment_error =
                                Some(error.to_string().chars().take(512).collect());
                            true
                        }
                    }
                };
                let preparation_done =
                    if !intent.prepare_music || intent.preparation_error.is_some() {
                        true
                    } else {
                        if intent.preparation_batch_id.is_none() {
                            match self.enqueue_guided_music_preparation(
                                intent.user_id,
                                intent.target_item_id,
                                intent.import_id,
                            ) {
                                Ok(batch) => intent.preparation_batch_id = Some(batch),
                                Err(error) => {
                                    intent.preparation_error =
                                        Some(error.to_string().chars().take(512).collect());
                                }
                            }
                        }
                        match intent.preparation_batch_id {
                            Some(batch) => {
                                let (done, error) =
                                    self.guided_music_preparation_outcome(intent.user_id, batch)?;
                                if done {
                                    intent.preparation_error = error;
                                }
                                done
                            }
                            None => true,
                        }
                    };
                let check_done =
                    if intent.cd_check_settings.is_none() || intent.cd_check_error.is_some() {
                        true
                    } else {
                        match self.advance_ingest_cd_check(&intent, &record) {
                            Ok(done) => done,
                            Err(error) => {
                                intent.cd_check_error =
                                    Some(error.to_string().chars().take(512).collect());
                                true
                            }
                        }
                    };
                intent.settled = enrichment_done && preparation_done && check_done;
                intent.updated_at = self.authentication.now();
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
            return self.advance_guided_game_identification(intent, record, &item);
        }
        if item.domain.module.0 == "org.junk.music" {
            return self.advance_guided_music_metadata(intent, record, &item);
        }
        Ok(true)
    }

    fn advance_guided_music_metadata(
        &self,
        intent: &GuidedIngestIntent,
        record: &ImportRecord,
        item: &Item,
    ) -> Result<bool> {
        let job_id = super::deterministic_job_id(
            intent.user_id,
            "guided.musicbrainz",
            &record.view.id.to_string(),
        );
        match self.jobs.job_record(job_id, intent.user_id) {
            Ok(job) => {
                return match job.state {
                    JobState::Completed => Ok(true),
                    JobState::Failed => Err(Error::Unavailable(job.error.unwrap_or_else(|| {
                        "Metadata resolution failed; originals remain preserved".into()
                    }))),
                    JobState::Cancelled => Err(Error::JobCancelled),
                    _ => Ok(false),
                };
            }
            Err(Error::NotFound) => {}
            Err(error) => return Err(error),
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
        let lookup = if let Some(manifest) = &payload.file_tracks {
            if !self
                .streaming_providers
                .contains_key(collector_modules::music::MUSICBRAINZ_PROVIDER_ID)
            {
                return Ok(true);
            }
            let capture = self.reader.capture(record.view.capture_id)?;
            let source = capture
                .representations
                .iter()
                .find(|r| r.id == record.view.representation_id)
                .ok_or(Error::Conflict)?;
            let ids = manifest
                .tracks
                .iter()
                .filter(|t| {
                    source
                        .files
                        .get(t.file_index)
                        .is_some_and(|f| f.visibility == Visibility::Shared)
                })
                .filter_map(|t| t.audio.tag("musicbrainz_release_id"))
                .collect::<BTreeSet<_>>();
            if ids.len() != 1 {
                return Ok(true);
            }
            AcquireMetadataCommand::MusicBrainzRelease {
                release_id: ids.first().unwrap().to_string(),
                refresh: false,
            }
        } else {
            let structure: collector_modules::music::CdProcessingResult = payload
                .observations
                .first()
                .ok_or_else(|| {
                    Error::Unavailable("music structure analysis is unavailable".into())
                })?
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
            AcquireMetadataCommand::MusicBrainzDisc {
                disc_id: ids.musicbrainz,
                refresh: false,
            }
        };
        let frozen = self.freeze_metadata_resolution(
            intent.user_id,
            item,
            ResolveMetadataCommand {
                lookup,
                acquire_artwork: true,
            },
            SnapshotId(job_id.0),
        )?;
        self.jobs.enqueue_job(JobSpec {id:job_id,user_id:intent.user_id,collection_id:Some(item.collection_id),item_id:Some(item.id),kind:"metadata_resolution".into(),phase:"queued".into(),total_units:36*1024*1024,request:serde_json::json!({"import_id":record.view.id,"guided":true,"metadata_resolution":true}),frozen_input:Some(FrozenJobInput::MetadataResolution(frozen))})?;
        Ok(false)
    }

    pub(crate) fn ingest_summary(
        &self,
        actor: &Principal,
        view: ImportView,
    ) -> Result<IngestSummary> {
        let intent = self
            .provisional_imports
            .guided_ingest_intent(view.id)?
            .ok_or(Error::NotFound)?;
        let jobs = self
            .provisional_imports
            .import_job_records(view.id, actor.user_id)?;
        let failed = jobs
            .iter()
            .filter(|job| job.kind != "inbox_cleanup")
            .any(|job| {
                job.state == JobState::Failed
                    || super::metadata_artwork_error(&job.kind, job.result.as_ref()).is_some()
            })
            || intent.enrichment_error.is_some()
            || intent.preparation_error.is_some()
            || intent.cd_check_error.is_some();
        let active = jobs
            .iter()
            .filter(|job| job.kind != "inbox_cleanup")
            .any(|job| {
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
        } else if jobs.iter().any(|j| {
            j.kind == "accuraterip_check"
                && j.result
                    .as_ref()
                    .and_then(|r| r.get("completion").unwrap_or(r).get("confirmed"))
                    .and_then(|r| r.as_bool())
                    == Some(false)
        }) {
            IngestStepState::NeedsInput
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
        let mut work = self
            .provisional_imports
            .import_work_records(view.id, actor.user_id)?
            .into_iter()
            .filter_map(|job| {
                let state = match job.state {
                    JobState::CancelRequested => IngestWorkState::Cancelling,
                    JobState::Queued | JobState::Running
                        if job.phase.starts_with("waiting_")
                            || matches!(job.phase.as_str(), "awaiting_source" | "staging") =>
                    {
                        IngestWorkState::Waiting
                    }
                    JobState::Running => IngestWorkState::Running,
                    JobState::Queued => IngestWorkState::Queued,
                    _ => return None,
                };
                Some(IngestWork {
                    shared: job.shared,
                    state,
                    kind: job.kind,
                    phase: job.phase,
                    jobs: job.jobs,
                })
            })
            .collect::<Vec<_>>();
        work.sort_by_key(|entry| match entry.state {
            IngestWorkState::Running => 0,
            IngestWorkState::Cancelling => 1,
            IngestWorkState::Waiting => 2,
            IngestWorkState::Queued => 3,
        });
        Ok(IngestSummary {
            work,
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
            prepare_music: intent.prepare_music,
            updated_at: view.updated_at,
            permissions: IngestPermissions {
                can_clean_inbox: can_write
                    && view.source.kind == ImportSourceKind::Inbox
                    && view.phase == ImportPhase::Published
                    && intent.settled
                    && !intent.cancelled
                    && !active,
                can_resume_upload: can_write
                    && !intent.cancelled
                    && view.phase == ImportPhase::Staging
                    && view.source.kind == ImportSourceKind::Browser
                    && intent
                        .clean_reimport
                        .as_ref()
                        .and_then(|options| options.retained_representation)
                        .is_none(),
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
                        state: if message.is_some() {
                            IngestStepState::Failed
                        } else {
                            IngestStepState::Pending
                        },
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
                    retryable: !intent.cancelled
                        && (job.state == JobState::Failed
                            || super::metadata_artwork_error(&job.kind, job.result.as_ref())
                                .is_some()),
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
        let game_job = if owner
            .as_ref()
            .is_some_and(|module| module.0 == "org.junk.games")
        {
            self.jobs.game_import_job(intent.user_id, record.view.id)?
        } else {
            None
        };
        let metadata_job = if music {
            jobs.iter().find(|job| job.id == metadata_job_id)
        } else {
            game_job.as_ref()
        };
        let metadata_message = metadata_job
            .filter(|job| job.state == JobState::Completed)
            .and_then(|job| completed_snapshot_id(job.result.as_ref()))
            .and_then(|snapshot_id| {
                self.processing
                    .snapshot(SnapshotQuery {
                        collection_id: record.view.collection_id,
                        snapshot_id,
                    })
                    .ok()
            })
            .and_then(|snapshot| {
                snapshot
                    .parsed_observation
                    .parse::<collector_modules::music::MusicBrainzSnapshotObservation>()
                    .ok()
            })
            .map(|observation| match observation.outcome {
                collector_modules::music::MetadataProviderOutcome::NoMatch => {
                    "No MusicBrainz album matched this disc ID. Original files are kept.".into()
                }
                collector_modules::music::MetadataProviderOutcome::Candidates
                    if observation.release_count > 1 =>
                {
                    format!(
                        "{} MusicBrainz albums matched. Review the choices in Metadata.",
                        observation.release_count
                    )
                }
                collector_modules::music::MetadataProviderOutcome::Candidates => {
                    "Album metadata was found using the disc ID.".into()
                }
            });
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
            job_step("metadata", metadata_job, false, metadata_message)
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
        let artwork_step = if metadata_job.is_some_and(|job| job.kind == "metadata_resolution") {
            let error = metadata_job
                .and_then(|job| job.result.as_ref())
                .and_then(|value| {
                    value
                        .get("completion")
                        .unwrap_or(value)
                        .get("artwork_error")
                })
                .and_then(serde_json::Value::as_str);
            let mut step = job_step("artwork", metadata_job, false, error.map(str::to_string));
            if error.is_some() {
                step.state = IngestStepState::Failed;
            }
            step
        } else if intent.enrichment == IngestEnrichmentPolicy::None || !music {
            job_step("artwork", None, true, None)
        } else {
            job_step(
                "artwork",
                artwork_image_job.or(artwork_listing_job),
                intent.settled && artwork_image_job.is_none() && intent.enrichment_error.is_none(),
                intent.enrichment_error.clone(),
            )
        };
        let preparation_step = if !intent.prepare_music {
            job_step("preparation", None, true, None)
        } else if let Some(error) = intent.preparation_error.clone() {
            IngestStepView {
                step: "preparation".into(),
                state: IngestStepState::Failed,
                job_id: intent.preparation_batch_id,
                attempt: intent
                    .preparation_batch_id
                    .and_then(|id| jobs.iter().find(|job| job.id == id).map(|job| job.attempt)),
                message: Some(error),
                retryable: !intent.cancelled && intent.preparation_batch_id.is_some(),
            }
        } else if let Some(batch) = intent.preparation_batch_id {
            let (done, error) = self.guided_music_preparation_outcome(intent.user_id, batch)?;
            IngestStepView {
                step: "preparation".into(),
                state: if done {
                    if error.is_some() {
                        IngestStepState::Failed
                    } else {
                        IngestStepState::Succeeded
                    }
                } else {
                    IngestStepState::Active
                },
                job_id: Some(batch),
                attempt: jobs
                    .iter()
                    .find(|job| job.id == batch)
                    .map(|job| job.attempt),
                message: error,
                retryable: false,
            }
        } else {
            job_step("preparation", None, false, None)
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
        let mut steps = vec![
            staging_step,
            job_step(
                "identification",
                game_job
                    .as_ref()
                    .or_else(|| by_kind.get("import_analysis").copied()),
                game_job.is_none()
                    && record.view.phase == ImportPhase::Published
                    && !by_kind.contains_key("import_analysis"),
                if intent.analysis == IngestAnalysisPolicy::PreserveOnly {
                    Some("Save originals only was selected. Files were not analyzed or matched to an album or edition.".into())
                } else {
                    record.view.limitations.first().cloned()
                },
            ),
            job_step(
                "preservation",
                by_kind.get("import_publication").copied(),
                false,
                None,
            ),
            metadata_step,
            artwork_step,
            preparation_step,
            {
                let job = jobs.iter().find(|j| j.kind == "accuraterip_check");
                let mut step = job_step(
                    "accuraterip",
                    job,
                    intent.cd_check_settings.is_none(),
                    intent.cd_check_error.clone(),
                );
                if job.is_some_and(|j| {
                    j.state == JobState::Completed
                        && j.result
                            .as_ref()
                            .and_then(|r| r.get("completion").unwrap_or(r).get("confirmed"))
                            .and_then(|r| r.as_bool())
                            == Some(false)
                }) {
                    step.state = IngestStepState::NeedsInput;
                    step.message=Some("Audio check finished without confirming this disc. See its AccurateRip result in the album.".into());
                }
                step
            },
        ];
        if !music {
            steps.retain(|step| {
                step.step != "preparation" && step.step != "metadata" && step.step != "accuraterip"
            });
        }
        Ok(IngestDetail {
            inbox_cleanup: self.provisional_imports.inbox_cleanup_status(id)?,
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
                if job.kind == "music_derivative_batch" {
                    return effects
                        .retry_preparation_batch(job_id, actor.user_id)
                        .map(|()| job_id);
                }
                if job.state != JobState::Failed
                    && !(job.state == JobState::Completed
                        && super::metadata_artwork_error(&job.kind, job.result.as_ref()).is_some())
                {
                    return Err(Error::Conflict);
                }
                effects.retry_failed_job(job_id, actor.user_id)
            })?;
        } else {
            intent.enrichment_error = None;
            intent.cd_check_error = None;
            if intent.preparation_batch_id.is_none() {
                intent.preparation_error = None;
            }
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
        if let Some(intent) = self.provisional_imports.guided_ingest_intent(id)? {
            self.retire_reassigned_placeholder(&intent)?;
        }
        self.ingest_detail(actor, id)
    }
}
