//! Durable intake policy. Manifest and result pages are bounded; accepted originals
//! use the same guided import effects as an individual package.
use crate::*;
use std::collections::BTreeSet;

/// A sibling disc can append preservation evidence without changing the reviewed
/// album inventory. Only references proven to belong to this intake may advance
/// its publication fence; manual inventory/metadata changes still conflict.
pub fn grouped_publication_revision_is_evidence_only(
    previous: &Item,
    current: &Item,
    previous_evidence: &std::collections::BTreeMap<EvidenceId, String>,
    current_evidence: &std::collections::BTreeMap<EvidenceId, String>,
    sibling_evidence: &BTreeSet<EvidenceId>,
) -> Result<bool> {
    if previous.id != current.id
        || previous.collection_id != current.collection_id
        || previous.revision >= current.revision
        || current.deleted
        || current_evidence.len() <= previous_evidence.len()
        || previous_evidence
            .iter()
            .any(|(id, digest)| current_evidence.get(id) != Some(digest))
        || current_evidence
            .keys()
            .any(|id| !previous_evidence.contains_key(id) && !sibling_evidence.contains(id))
    {
        return Ok(false);
    }
    let mut current = current.clone();
    let mut previous = previous.clone();
    // Captures and Assets have independent revision fences. The caller supplies
    // archive Item records, without those hydrated compatibility projections.
    current.captures.clear();
    current.assets.clear();
    previous.captures.clear();
    previous.assets.clear();
    current.revision = previous.revision;
    current.updated_at = previous.updated_at;
    Ok(
        serde_json::to_value(previous).map_err(|error| Error::Storage(error.to_string()))?
            == serde_json::to_value(current).map_err(|error| Error::Storage(error.to_string()))?,
    )
}

pub const INTAKE_PACKAGE_LIMIT: u32 = 10_000;
pub const INTAKE_FILE_LIMIT: u32 = 100_000;
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum IntakeState {
    Draft,
    Accepted,
    Completed,
    Cancelling,
    Cancelled,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct IntakeSession {
    pub id: JobId,
    pub collection_id: CollectionId,
    pub revision: Revision,
    pub state: IntakeState,
    pub package_count: u32,
    pub file_count: u32,
    pub expected_packages: u32,
    pub expected_files: u32,
    pub admitted: u32,
    pub rejected: u32,
    pub preserved: u32,
    pub needs_attention: u32,
    pub automatic_metadata: bool,
    pub prepare_music: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateIntake {
    pub collection_id: CollectionId,
    pub expected_packages: u32,
    pub expected_files: u32,
    #[serde(default = "automatic_metadata_default")]
    pub automatic_metadata: bool,
    #[serde(default)]
    pub prepare_music: bool,
}
fn automatic_metadata_default() -> bool {
    true
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(tag = "destination", rename_all = "snake_case")]
pub enum IntakeDestination {
    Separate,
    AlbumGroup {
        key: String,
        #[serde(default)]
        title: Option<String>,
        copy_key: String,
    },
    Existing {
        item_id: ItemId,
        expected_revision: Revision,
        copy_key: String,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct IntakePackage {
    pub package: IngestPackageSelection,
    pub destination: IntakeDestination,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AppendIntake {
    pub expected_revision: Revision,
    pub packages: Vec<IntakePackage>,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct StartIntake {
    pub expected_revision: Revision,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IntakePackageRecord {
    pub position: u32,
    pub plan: IngestPlanPackage,
    pub plan_token: String,
    pub destination: IntakeDestination,
    pub import_id: Option<ImportId>,
    pub error: Option<String>,
}
#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
pub struct IntakeMemberView {
    pub position: u32,
    pub plan: IngestPlanPackage,
    pub destination: IntakeDestination,
    pub import_id: Option<ImportId>,
    pub import: Option<IngestSummary>,
    pub error: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PrivateIntake {
    pub user_id: UserId,
    pub binding: CollectionBinding,
    pub session: IntakeSession,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct IntakeDiscovery {
    pub id: JobId,
    pub collection_id: CollectionId,
    pub package_count: u32,
    pub unassigned_count: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct IntakeDiscoveryPage {
    pub discovery: IntakeDiscovery,
    pub packages: Page<IngestPlanPackage>,
    pub unassigned_files: Page<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RepairIntakeDestination {
    pub expected_revision: Revision,
    pub destination: IntakeDestination,
}
pub trait BulkIntakePort: Send + Sync {
    fn reject_intake_member(&self, claim: &JobClaim, position: u32, reason: &str) -> Result<()>;
    fn repair_intake_destination(
        &self,
        user: UserId,
        id: JobId,
        position: u32,
        command: RepairIntakeDestination,
    ) -> Result<IntakeSession>;

    fn freeze_intake_discovery(
        &self,
        user: UserId,
        discovery: IntakeDiscovery,
        packages: Vec<IngestPlanPackage>,
        unassigned: Vec<String>,
    ) -> Result<IntakeDiscovery>;
    fn intake_discovery_page(
        &self,
        user: UserId,
        id: JobId,
        after: Option<u32>,
        unassigned_after: Option<u32>,
    ) -> Result<IntakeDiscoveryPage>;

    fn create_intake(&self, record: PrivateIntake) -> Result<IntakeSession>;
    fn intake(&self, user: UserId, id: JobId) -> Result<PrivateIntake>;
    fn intake_sessions(
        &self,
        user: UserId,
        collection: Option<CollectionId>,
        after: Option<JobId>,
        limit: u16,
    ) -> Result<Page<IntakeSession>>;
    fn append_intake(
        &self,
        user: UserId,
        id: JobId,
        expected: Revision,
        packages: Vec<IntakePackageRecord>,
    ) -> Result<IntakeSession>;
    fn accept_intake(&self, user: UserId, id: JobId, expected: Revision) -> Result<IntakeSession>;
    fn intake_members(
        &self,
        user: UserId,
        id: JobId,
        after: Option<u32>,
        limit: u16,
        pending: bool,
    ) -> Result<Page<IntakePackageRecord>>;
    fn admit_intake_members(
        &self,
        claim: &JobClaim,
        positions: &[u32],
        records: Vec<ImportRecord>,
        intents: Vec<GuidedIngestIntent>,
    ) -> Result<()>;
    fn cancel_intake(&self, user: UserId, id: JobId) -> Result<IntakeSession>;
    fn unfinished_intakes(&self, after: Option<JobId>, limit: u16) -> Result<Vec<PrivateIntake>>;
    fn reconcile_intake_state(&self, user: UserId, id: JobId) -> Result<IntakeSession>;
    fn intake_album_preservation_settled(
        &self,
        user: UserId,
        id: JobId,
        item: ItemId,
    ) -> Result<bool>;
}
impl CollectionService {
    pub fn discover_bulk_intake(
        &self,
        actor: &Principal,
        command: DiscoverIngestCommand,
        context: &CommandContext,
    ) -> Result<IntakeDiscovery> {
        self.execute_command(actor,context,"intake.discover",&command,|effects| {
            let eligibility=self.content_write_eligibility(actor,command.collection_id)?;
            crate::guided_ingest::validate_intake_files(&command.files,INTAKE_FILE_LIMIT as usize)?;
            let files=command.files.iter().filter(|file| !is_intake_housekeeping(&file.name,false)).cloned().collect::<Vec<_>>();
            let names=files.iter().map(|f|PackageFileName::new(f.name.clone())).collect::<Result<Vec<_>>>()?;
            let found=self.modules.discover_imports(&eligibility.owner,&names)?;
            if found.len()>INTAKE_PACKAGE_LIMIT as usize {return Err(Error::Invalid("This folder contains more than 10,000 packages. Select a smaller folder or split it into separate intakes.".into()));}
            let mut used=BTreeSet::new();let mut selections=Vec::new();
            for p in found {let files=p.file_indices.into_iter().map(|index|{if !used.insert(index){return Err(Error::Invalid("Overlapping discovered packages".into()));}files.get(index).cloned().ok_or_else(||Error::Invalid("Invalid discovered file".into()))}).collect::<Result<Vec<_>>>()?;selections.push(IngestPackageSelection{purchase:None,source:ImportSourceSummary{kind:ImportSourceKind::Browser,label:None,package_name:p.label},files,anchor_file_index:None});}
            let mut packages=Vec::new();for page in selections.chunks(64){packages.extend(self.plan_ingest(actor,PlanIngestCommand{collection_id:command.collection_id,packages:page.to_vec()})?.packages);}
            let unassigned=files.iter().enumerate().filter(|(i,_)|!used.contains(i)).map(|(_,f)|f.name.clone()).collect::<Vec<_>>();
            effects.freeze_intake_discovery(actor.user_id,IntakeDiscovery{id:deterministic_job_id(actor.user_id,"intake.discover",context.key()),collection_id:command.collection_id,package_count:packages.len() as u32,unassigned_count:unassigned.len() as u32},packages,unassigned)
        })
    }
    pub fn intake_discovery_page(
        &self,
        actor: &Principal,
        id: JobId,
        after: Option<u32>,
        unassigned_after: Option<u32>,
    ) -> Result<IntakeDiscoveryPage> {
        let page =
            self.intake_port
                .intake_discovery_page(actor.user_id, id, after, unassigned_after)?;
        self.authorize(actor, page.discovery.collection_id, Action::Curate)?;
        Ok(page)
    }
    pub fn create_intake(
        &self,
        actor: &Principal,
        command: CreateIntake,
        context: &CommandContext,
    ) -> Result<IntakeSession> {
        self.execute_command(actor, context, "intake.create", &command, |effects| {
            if command.expected_packages == 0 || command.expected_packages > INTAKE_PACKAGE_LIMIT
                || command.expected_files > INTAKE_FILE_LIMIT {
                return Err(Error::Invalid("Plan 1–10,000 packages and at most 100,000 files per intake. Split larger selections into separate intakes.".into()));
            }
            let eligibility = self.content_write_eligibility(actor, command.collection_id)?;
            let now = self.authentication.now();
            effects.create_intake(PrivateIntake {
                user_id: actor.user_id,
                binding: eligibility.binding,
                session: IntakeSession {
                    id: deterministic_job_id(actor.user_id, "intake.create", context.key()),
                    collection_id: command.collection_id,
                    revision: Revision(1),
                    state: IntakeState::Draft,
                    package_count: 0,
                    file_count: 0,
                    expected_packages: command.expected_packages,
                    expected_files: command.expected_files,
                    admitted: 0,
                    rejected: 0,
                    preserved: 0,
                    needs_attention: 0,
                    automatic_metadata: command.automatic_metadata,
                    prepare_music: command.prepare_music,
                    created_at: now,
                    updated_at: now,
                },
            })
        })
    }
    pub fn intake(&self, actor: &Principal, id: JobId) -> Result<IntakeSession> {
        let record = self.intake_port.intake(actor.user_id, id)?;
        self.authorize(actor, record.session.collection_id, Action::Curate)?;
        Ok(record.session)
    }
    pub fn intake_sessions(
        &self,
        actor: &Principal,
        collection: Option<CollectionId>,
        after: Option<JobId>,
        limit: u16,
    ) -> Result<Page<IntakeSession>> {
        if let Some(collection) = collection {
            self.authorize(actor, collection, Action::Curate)?;
        }
        self.intake_port
            .intake_sessions(actor.user_id, collection, after, limit)
    }
    pub fn append_intake_with_preflight(
        &self,
        actor: &Principal,
        id: JobId,
        command: AppendIntake,
        context: &CommandContext,
        preflight: impl FnOnce(&IngestPlan) -> Result<()>,
    ) -> Result<IntakeSession> {
        self.execute_command(
            actor,
            context,
            "intake.append",
            &(id, &command),
            |effects| {
                let intake = self.intake_port.intake(actor.user_id, id)?;
                let eligibility =
                    self.content_write_eligibility(actor, intake.session.collection_id)?;
                self.validate_frozen_binding(&eligibility, &intake.binding)?;
                if command.packages.is_empty() || command.packages.len() > 64 {
                    return Err(Error::Invalid(
                        "Add 1–64 packages per manifest page.".into(),
                    ));
                }
                if command.packages.iter().map(|p|p.package.files.len()).sum::<usize>()>10000 {return Err(Error::Invalid("Each manifest page accepts at most 10,000 files. Add fewer packages per page; the session supports 100,000 files.".into()));}
                for package in &command.packages {
                    match &package.destination {
                        IntakeDestination::AlbumGroup {
                            key,
                            title,
                            copy_key,
                        } => {
                            validate_intake_label(key)?;
                            if let Some(title) = title { validate_intake_label(title)?; }
                            validate_intake_label(copy_key)?;
                        }
                        IntakeDestination::Existing {
                            item_id,
                            expected_revision,
                            copy_key,
                        } => {
                            validate_intake_label(copy_key)?;
                            let item = self.reader.item(*item_id)?;
                            if item.collection_id != intake.session.collection_id
                                || item.deleted
                                || item.revision != *expected_revision
                            {
                                return Err(Error::Conflict);
                            }
                        }
                        IntakeDestination::Separate => {}
                    }
                }
                let plan = self.plan_ingest(
                    actor,
                    PlanIngestCommand {
                        collection_id: intake.session.collection_id,
                        packages: command
                            .packages
                            .iter()
                            .map(|value| value.package.clone())
                            .collect(),
                    },
                )?;
                preflight(&plan)?;
                let packages = plan
                    .packages
                    .into_iter()
                    .zip(&command.packages)
                    .map(|(plan, input)| {
                        Ok(IntakePackageRecord {
                            position: 0,
                            plan,
                            plan_token: plan_token_for_package(
                                intake.session.collection_id,
                                &intake.binding,
                                &input.package,
                            )?,
                            destination: input.destination.clone(),
                            import_id: None,
                            error: None,
                        })
                    })
                    .collect::<Result<Vec<_>>>()?;
                effects.append_intake(actor.user_id, id, command.expected_revision, packages)
            },
        )
    }
    pub fn repair_intake_destination(
        &self,
        actor: &Principal,
        id: JobId,
        position: u32,
        command: RepairIntakeDestination,
        context: &CommandContext,
    ) -> Result<IntakeSession> {
        self.execute_command(
            actor,
            context,
            "intake.destination.repair",
            &(id, position, &command),
            |effects| {
                let intake = self.intake_port.intake(actor.user_id, id)?;
                self.content_write_eligibility(actor, intake.session.collection_id)?;
                match &command.destination {
                    IntakeDestination::Separate => {}
                    IntakeDestination::AlbumGroup {
                        key,
                        title,
                        copy_key,
                    } => {
                        validate_intake_label(key)?;
                        if let Some(title) = title {
                            validate_intake_label(title)?;
                        }
                        validate_intake_label(copy_key)?;
                    }
                    IntakeDestination::Existing {
                        item_id,
                        expected_revision,
                        copy_key,
                    } => {
                        validate_intake_label(copy_key)?;
                        let item = self.reader.item(*item_id)?;
                        if item.collection_id != intake.session.collection_id
                            || item.deleted
                            || item.revision != *expected_revision
                        {
                            return Err(Error::Conflict);
                        }
                    }
                }
                effects.repair_intake_destination(actor.user_id, id, position, command.clone())
            },
        )
    }
    pub fn start_intake(
        &self,
        actor: &Principal,
        id: JobId,
        command: StartIntake,
        context: &CommandContext,
    ) -> Result<IntakeSession> {
        self.execute_command(actor, context, "intake.start", &(id, &command), |effects| {
            let intake = self.intake_port.intake(actor.user_id, id)?;
            let eligibility =
                self.content_write_eligibility(actor, intake.session.collection_id)?;
            self.validate_frozen_binding(&eligibility, &intake.binding)?;
            effects.accept_intake(actor.user_id, id, command.expected_revision)
        })
    }
    pub fn intake_members(
        &self,
        actor: &Principal,
        id: JobId,
        after: Option<u32>,
        limit: u16,
    ) -> Result<Page<IntakeMemberView>> {
        self.intake(actor, id)?;
        let page = self
            .intake_port
            .intake_members(actor.user_id, id, after, limit, false)?;
        let items = page
            .items
            .into_iter()
            .map(|record| {
                let import = record
                    .import_id
                    .map(|id| self.ingest_detail(actor, id).map(|detail| detail.summary))
                    .transpose()?;
                Ok(IntakeMemberView {
                    position: record.position,
                    plan: record.plan,
                    destination: record.destination,
                    import_id: record.import_id,
                    import,
                    error: record.error,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Page {
            items,
            next_cursor: page.next_cursor,
        })
    }
    pub fn cancel_intake(
        &self,
        actor: &Principal,
        id: JobId,
        context: &CommandContext,
    ) -> Result<IntakeSession> {
        self.intake(actor, id)?;
        self.execute_command(actor, context, "intake.cancel", &id, |effects| {
            effects.cancel_intake(actor.user_id, id)
        })
    }
    pub(crate) fn run_bulk_intake(&self, claim: &JobClaim) -> Result<serde_json::Value> {
        let intake = self.intake_port.intake(claim.user_id, claim.id)?;
        let eligibility =
            self.content_write_eligibility_for_user(claim.user_id, intake.session.collection_id)?;
        self.validate_frozen_binding(&eligibility, &intake.binding)?;
        loop {
            if self
                .jobs
                .cancellation_requested(claim.id, &claim.claim_token)?
            {
                return Err(Error::JobCancelled);
            }
            let page = self
                .intake_port
                .intake_members(claim.user_id, claim.id, None, 32, true)?;
            if page.items.is_empty() {
                break;
            }
            let mut records = Vec::new();
            let mut intents = Vec::new();
            let mut positions = Vec::new();
            for member in page.items {
                let mut package = member.plan.clone();
                package.position = 0;
                let destination = match &member.destination {
                    IntakeDestination::Existing {
                        item_id,
                        expected_revision,
                        ..
                    } => Some(IngestDestination {
                        item_id: *item_id,
                        expected_revision: *expected_revision,
                    }),
                    _ => None,
                };
                let command = StartIngestCommand {
                    purchase: None,
                    clean_reimport: None,
                    plan: IngestPlan {
                        collection_id: intake.session.collection_id,
                        token: member.plan_token,
                        packages: vec![package],
                    },
                    analysis: IngestAnalysisPolicy::Automatic,
                    enrichment: if intake.session.automatic_metadata {
                        IngestEnrichmentPolicy::Automatic
                    } else {
                        IngestEnrichmentPolicy::None
                    },
                    prepare_music: intake.session.prepare_music
                        && member.plan.preparation_available,
                    destination,
                };
                let (mut child_records, mut child_intents) = match self.build_guided_import_records(
                    claim.user_id,
                    &command,
                    &format!("{}:{}", claim.id, member.position),
                ) {
                    Ok(records) => records,
                    Err(error @ (Error::Conflict | Error::Invalid(_) | Error::Unavailable(_))) => {
                        self.intake_port.reject_intake_member(
                            claim,
                            member.position,
                            &error.to_string(),
                        )?;
                        continue;
                    }
                    Err(error) => return Err(error),
                };
                let record = child_records.first_mut().ok_or(Error::Conflict)?;
                let intent = child_intents.first_mut().ok_or(Error::Conflict)?;
                let (copy_key, title) = match &member.destination {
                    IntakeDestination::AlbumGroup {
                        key,
                        title,
                        copy_key,
                    } => {
                        intent.target_item_id = ItemId(deterministic_inventory_uuid(
                            claim.user_id,
                            "intake.album",
                            &format!("{}:{key}", claim.id),
                        ));
                        (copy_key.clone(), title.clone())
                    }
                    IntakeDestination::Existing { copy_key, .. } => (copy_key.clone(), None),
                    IntakeDestination::Separate => (member.position.to_string(), None),
                };
                let manual_title = title.clone();
                if let Some(title) = title {
                    intent.default_title = title;
                }
                intent.bulk_context = Some(GuidedBulkContext {
                    intake_id: claim.id,
                    manual_title,
                    copy_id: SpecimenId(deterministic_inventory_uuid(
                        claim.user_id,
                        "intake.copy",
                        &format!("{}:{}:{copy_key}", claim.id, intent.target_item_id),
                    )),
                    copy_label: format!("Imported copy {copy_key}"),
                });
                if intent.created_item {
                    record.view.association.item_id = None;
                }
                positions.push(member.position);
                records.extend(child_records);
                intents.extend(child_intents);
            }
            if !records.is_empty() {
                match self.intake_port.admit_intake_members(
                    claim,
                    &positions,
                    records.clone(),
                    intents.clone(),
                ) {
                    Ok(()) => {}
                    Err(Error::Conflict) => {
                        for ((position, record), intent) in
                            positions.into_iter().zip(records).zip(intents)
                        {
                            match self.intake_port.admit_intake_members(claim,&[position],vec![record],vec![intent]){Ok(())=>{},Err(Error::Conflict)=>self.intake_port.reject_intake_member(claim,position,"The destination changed after review. Choose its current revision or preserve this package as a separate entry.")?,Err(error)=>return Err(error)}
                        }
                    }
                    Err(error) => return Err(error),
                }
            }
            let current = self.intake_port.intake(claim.user_id, claim.id)?.session;
            self.jobs.heartbeat_job(
                claim.id,
                &claim.claim_token,
                "admitting",
                u64::from(current.admitted + current.rejected),
                30,
            )?;
        }
        Ok(serde_json::json!({"intake_id":claim.id}))
    }
    pub fn resume_bulk_intakes(&self) -> Result<()> {
        let mut after = None;
        loop {
            let records = self.intake_port.unfinished_intakes(after, 128)?;
            if records.is_empty() {
                break;
            }
            after = records.last().map(|record| record.session.id);
            for record in records {
                match self.intake_port.intake(record.user_id, record.session.id) {
                    Ok(_) => {}
                    Err(Error::Forbidden | Error::NotFound | Error::Unauthenticated) => continue,
                    Err(error) => return Err(error),
                }
                if matches!(record.session.state, IntakeState::Cancelling) {
                    let mut cursor = None;
                    loop {
                        let page = self.intake_port.intake_members(
                            record.user_id,
                            record.session.id,
                            cursor,
                            64,
                            false,
                        )?;
                        cursor = page.items.last().map(|member| member.position);
                        for member in page.items {
                            if let Some(id) = member.import_id {
                                let import = self.provisional_imports.import_record(id)?;
                                if !matches!(
                                    import.view.phase,
                                    ImportPhase::Cancelled | ImportPhase::Published
                                ) {
                                    self.provisional_imports
                                        .cancel_guided_import(id, import.view.revision)?;
                                }
                            }
                        }
                        if page.next_cursor.is_none() {
                            break;
                        }
                    }
                }
                self.intake_port
                    .reconcile_intake_state(record.user_id, record.session.id)?;
            }
        }
        Ok(())
    }
}
fn validate_intake_label(value: &str) -> Result<()> {
    if value.trim().is_empty() || value.len() > 512 || value.chars().any(char::is_control) {
        return Err(Error::Invalid(
            "Album group titles, group keys and copy names must contain 1–512 characters.".into(),
        ));
    }
    Ok(())
}
fn plan_token_for_package(
    collection: CollectionId,
    binding: &CollectionBinding,
    package: &IngestPackageSelection,
) -> Result<String> {
    Ok(Digest::sha256(
        &serde_json::to_vec(&(collection, binding, package))
            .map_err(|error| Error::Invalid(error.to_string()))?,
    )
    .hex)
}
