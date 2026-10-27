//! Frozen library selections and rolling admission for metadata and FLAC care.
use crate::*;
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum BulkLibraryKind {
    Metadata,
    MusicPreparation,
    AccurateRip,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(tag = "selection", rename_all = "snake_case", deny_unknown_fields)]
pub enum LibrarySelection {
    Explicit {
        items: Vec<ItemId>,
    },
    AllMatching {
        query: MetadataCareQuery,
        exclusions: Vec<ItemId>,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PreviewBulkLibrary {
    pub kind: BulkLibraryKind,
    pub selection: LibrarySelection,
    #[serde(default)]
    pub sources: BTreeMap<ItemId, Vec<RepresentationId>>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum BulkLibraryState {
    Preview,
    Accepted,
    Completed,
    Cancelling,
    Cancelled,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum BulkMemberState {
    Ready,
    Excluded,
    Queued,
    Running,
    Succeeded,
    Failed,
    Review,
    Cancelled,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct BulkLibrarySummary {
    pub id: JobId,
    pub kind: BulkLibraryKind,
    pub state: BulkLibraryState,
    pub selected: u32,
    pub tracks: u32,
    pub already_ready: u32,
    pub excluded: u32,
    pub succeeded: u32,
    pub failed: u32,
    pub pending: u32,
    pub source_bytes: ByteLength,
    pub can_start: bool,
    pub can_cancel: bool,
    pub can_retry: bool,
    pub admission_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct BulkLibraryMemberView {
    pub position: u32,
    pub item_id: ItemId,
    pub collection_id: CollectionId,
    pub title: String,
    pub artwork_asset_id: Option<AssetId>,
    pub state: BulkMemberState,
    pub reason: Option<String>,
    pub job_id: Option<JobId>,
    pub tracks: u16,
    pub already_ready: u16,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FrozenBulkLibraryMember {
    pub view: BulkLibraryMemberView,
    pub item: Item,
    pub input: Option<FrozenJobInput>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PrivateBulkLibrary {
    pub user_id: UserId,
    pub summary: BulkLibrarySummary,
}
#[derive(Clone, Copy, Debug)]
pub enum BulkMemberFilter {
    All,
    Admission,
    Unsettled,
}
pub trait BulkLibraryPort: Send + Sync {
    fn freeze_bulk_library(
        &self,
        record: PrivateBulkLibrary,
        members: Vec<FrozenBulkLibraryMember>,
    ) -> Result<BulkLibrarySummary>;
    fn bulk_library(&self, user: UserId, id: JobId) -> Result<PrivateBulkLibrary>;
    fn bulk_library_page(
        &self,
        user: UserId,
        collection: Option<CollectionId>,
        after: Option<JobId>,
        limit: u16,
    ) -> Result<Page<BulkLibrarySummary>>;
    fn bulk_library_members(
        &self,
        user: UserId,
        id: JobId,
        after: Option<u32>,
        limit: u16,
        filter: BulkMemberFilter,
    ) -> Result<Page<FrozenBulkLibraryMember>>;
    fn accept_bulk_library(&self, user: UserId, id: JobId) -> Result<BulkLibrarySummary>;
    fn admit_bulk_library_member(
        &self,
        claim: &JobClaim,
        position: u32,
        spec: JobSpec,
    ) -> Result<JobId>;
    fn bulk_library_capacity(&self, user: UserId, kind: BulkLibraryKind) -> Result<u32>;
    fn yield_bulk_library(&self, claim: &JobClaim) -> Result<()>;
    fn change_bulk_library(
        &self,
        user: UserId,
        id: JobId,
        cancel: bool,
    ) -> Result<BulkLibrarySummary>;
    fn settle_bulk_library_member(
        &self,
        user: UserId,
        id: JobId,
        position: u32,
        state: BulkMemberState,
        reason: Option<String>,
    ) -> Result<()>;
    fn unfinished_bulk_library(
        &self,
        after: Option<JobId>,
        limit: u16,
    ) -> Result<Vec<PrivateBulkLibrary>>;
    fn reconcile_bulk_library(&self, user: UserId, id: JobId) -> Result<BulkLibrarySummary>;
}
impl CollectionService {
    pub fn preview_bulk_library(
        &self,
        actor: &Principal,
        command: PreviewBulkLibrary,
        context: &CommandContext,
    ) -> Result<BulkLibrarySummary> {
        self.execute_command(actor, context, "library.bulk.preview", &command, |effects| {
            let id = deterministic_job_id(actor.user_id, "library.bulk.preview", context.key());
            let mut items = Vec::new();
            match &command.selection {
                LibrarySelection::Explicit { items: ids } => {
                    if ids.is_empty() || ids.len() > 10_000 {return Err(Error::Invalid("Select 1–10,000 entries.".into()));}
                    let mut seen = BTreeSet::new();
                    for id in ids {
                        if !seen.insert(*id) {return Err(Error::Invalid("Duplicate selected entry.".into()));}
                        let item = self.reader.item(*id)?;
                        self.authorize(actor, item.collection_id, Action::Curate)?;
                        items.push((item.id,item.revision));
                    }
                }
                LibrarySelection::AllMatching { query, exclusions } => {
                    if query.cursor.is_some() || exclusions.len()>10_000 {return Err(Error::Invalid("Select all matching results from the start of a filter, with at most 10,000 exclusions.".into()));}
                    let exclusions=exclusions.iter().copied().collect::<BTreeSet<_>>();
                    let mut query=query.clone();query.limit=Some(200);
                    loop {
                        let page=if command.kind==BulkLibraryKind::MusicPreparation {self.care.preparation_care_items(actor.user_id,&query)?}else{self.care.metadata_care_items(actor.user_id,&query)?};
                        items.extend(page.items.into_iter().filter(|i|!exclusions.contains(&i.id)).map(|i|(i.id,i.revision)));
                        if items.len()>10_000 {return Err(Error::Invalid("This selection exceeds 10,000 entries. Narrow the collection or search and submit another batch.".into()));}
                        let Some(cursor)=page.next_cursor else {break;};query.cursor=Some(cursor);
                    }
                }
            }
            if items.is_empty(){return Err(Error::Invalid("No entries match this selection.".into()));}
            let now=self.authentication.now();
            let mut summary=BulkLibrarySummary{id,kind:command.kind,state:BulkLibraryState::Preview,selected:items.len() as u32,tracks:0,already_ready:0,excluded:0,succeeded:0,failed:0,pending:0,source_bytes:ByteLength(0),can_start:false,can_cancel:false,can_retry:false,admission_error:None,created_at:now,updated_at:now};
            let mut members=Vec::new();let mut manifest_bytes=0usize;
            for (position,(selected_id,selected_revision)) in items.into_iter().enumerate() {
                let item=self.reader.item(selected_id)?;
                if item.revision!=selected_revision{return Err(Error::Conflict);}
                let child=deterministic_job_id(actor.user_id,"library.bulk.member",&format!("{id}:{position}"));
                let frozen=self.freeze_bulk_member(actor,&item,&command,child);
                let (input,reason)=match frozen {Ok(input)=>(Some(input),None),Err(error)=>(None,Some(error.to_string()))};
                let (tracks,ready)=if let Some(FrozenJobInput::DerivativeBatch(batch))=&input {(batch.members.len() as u16,batch.members.iter().filter(|m|matches!(m,FrozenPreparationMember::AlreadyAvailable{..})).count() as u16)}else{(0,0)};
                summary.tracks+=u32::from(tracks);summary.already_ready+=u32::from(ready);
                if input.is_none(){summary.excluded+=1;}else{summary.pending+=1;}
                if input.is_some(){for file in item.captures.iter().flat_map(|c|&c.representations).filter(|r|r.role==RepresentationRole::PreservationMaster).flat_map(|r|&r.files){summary.source_bytes.0=summary.source_bytes.0.saturating_add(file.bytes.0);}}
                let presented=self.item_presented(actor,item.id)?;
                let member=FrozenBulkLibraryMember {view:BulkLibraryMemberView{position:position as u32,item_id:item.id,collection_id:item.collection_id,title:presented.title,artwork_asset_id:presented.primary_artwork,state:if input.is_some(){BulkMemberState::Ready}else{BulkMemberState::Excluded},reason,job_id:None,tracks,already_ready:ready},item,input};
                manifest_bytes=manifest_bytes.saturating_add(serde_json::to_vec(&member).map_err(|e|Error::Invalid(e.to_string()))?.len());
                if manifest_bytes>256*1024*1024{return Err(Error::Invalid("This frozen selection exceeds 256 MiB of inventory and processing metadata. Submit smaller batches; no work has been started.".into()));}
                members.push(member);
            }
            summary.can_start=summary.pending>0;
            effects.freeze_bulk_library(PrivateBulkLibrary{user_id:actor.user_id,summary},members)
        })
    }
    fn freeze_bulk_member(
        &self,
        actor: &Principal,
        item: &Item,
        command: &PreviewBulkLibrary,
        child: JobId,
    ) -> Result<FrozenJobInput> {
        self.content_write_eligibility(actor, item.collection_id)?;
        if item.deleted {
            return Err(Error::Conflict);
        }
        match command.kind {
            BulkLibraryKind::AccurateRip => Ok(FrozenJobInput::AccurateRip(
                self.freeze_accuraterip_check(actor.user_id, item, child, None, true)?,
            )),
            BulkLibraryKind::MusicPreparation => {
                let plan = self.music_album_preparation_plan(actor, item.id)?;
                let chosen = command.sources.get(&item.id);
                let sources = plan
                    .discs
                    .iter()
                    .flat_map(|disc| {
                        let explicit = disc
                            .sources
                            .iter()
                            .filter(|source| {
                                chosen.is_some_and(|ids| {
                                    ids.contains(&source.source_representation_id)
                                })
                            })
                            .map(|source| source.source_representation_id)
                            .collect::<Vec<_>>();
                        if explicit.is_empty() {
                            disc.sources
                                .iter()
                                .filter(|source| source.default_selected)
                                .map(|source| source.source_representation_id)
                                .collect()
                        } else {
                            explicit
                        }
                    })
                    .collect::<Vec<_>>();
                if chosen.is_some_and(|ids| ids.iter().any(|id| !sources.contains(id))) {
                    return Err(Error::Invalid(
                        "A selected source is no longer available for this album.".into(),
                    ));
                }
                if plan.discs.iter().any(|disc| {
                    disc.ambiguous
                        && !disc
                            .sources
                            .iter()
                            .any(|source| sources.contains(&source.source_representation_id))
                }) {
                    return Err(Error::Unavailable(
                        "Choose a preservation source for every disc with alternatives.".into(),
                    ));
                }
                Ok(FrozenJobInput::DerivativeBatch(
                    self.freeze_music_album_batch(actor.user_id, item, &sources, child)?,
                ))
            }
            BulkLibraryKind::Metadata => {
                if item.domain.module.0 == "org.junk.games" {
                    return self.freeze_existing_dat_metadata(actor.user_id, item, child);
                }
                let review = self.metadata_review(actor, item.id)?;
                let choice = review
                    .lookup_choices
                    .iter()
                    .find(|v| v.kind == MetadataLookupKind::DiscId && v.id.starts_with("layout:"))
                    .or_else(|| {
                        review
                            .lookup_choices
                            .iter()
                            .find(|v| v.kind == MetadataLookupKind::ReleaseId)
                    })
                    .or_else(|| {
                        review
                            .lookup_choices
                            .iter()
                            .find(|v| v.kind == MetadataLookupKind::Search)
                    })
                    .ok_or_else(|| {
                        Error::Unavailable("No metadata lookup is available for this entry.".into())
                    })?;
                let lookup = match choice.kind {
                    MetadataLookupKind::DiscId => AcquireMetadataCommand::MusicBrainzDisc {
                        disc_id: choice.value.clone(),
                        refresh: true,
                    },
                    MetadataLookupKind::ReleaseId => AcquireMetadataCommand::MusicBrainzRelease {
                        release_id: choice.value.clone(),
                        refresh: true,
                    },
                    MetadataLookupKind::Search => AcquireMetadataCommand::MusicBrainzSearch {
                        query: choice.value.clone(),
                        refresh: true,
                    },
                };
                Ok(FrozenJobInput::MetadataResolution(
                    self.freeze_metadata_resolution(
                        actor.user_id,
                        item,
                        ResolveMetadataCommand {
                            lookup,
                            acquire_artwork: true,
                        },
                        SnapshotId(child.0),
                    )?,
                ))
            }
        }
    }
    pub fn bulk_library(&self, actor: &Principal, id: JobId) -> Result<BulkLibrarySummary> {
        let record = self.bulk_port.bulk_library(actor.user_id, id)?;
        Ok(record.summary)
    }
    pub fn bulk_library_page(
        &self,
        actor: &Principal,
        collection: Option<CollectionId>,
        after: Option<JobId>,
        limit: u16,
    ) -> Result<Page<BulkLibrarySummary>> {
        if let Some(collection) = collection {
            self.authorize(actor, collection, Action::Curate)?;
        }
        self.bulk_port
            .bulk_library_page(actor.user_id, collection, after, limit)
    }
    pub fn bulk_library_members(
        &self,
        actor: &Principal,
        id: JobId,
        after: Option<u32>,
        limit: u16,
    ) -> Result<Page<BulkLibraryMemberView>> {
        let page = self.bulk_port.bulk_library_members(
            actor.user_id,
            id,
            after,
            limit,
            BulkMemberFilter::All,
        )?;
        Ok(Page {
            items: page.items.into_iter().map(|member| member.view).collect(),
            next_cursor: page.next_cursor,
        })
    }
    pub fn start_bulk_library(
        &self,
        actor: &Principal,
        id: JobId,
        context: &CommandContext,
    ) -> Result<BulkLibrarySummary> {
        self.execute_command(actor, context, "library.bulk.start", &id, |effects| {
            effects.accept_bulk_library(actor.user_id, id)
        })
    }
    pub fn change_bulk_library(
        &self,
        actor: &Principal,
        id: JobId,
        cancel: bool,
        context: &CommandContext,
    ) -> Result<BulkLibrarySummary> {
        self.execute_command(
            actor,
            context,
            if cancel {
                "library.bulk.cancel"
            } else {
                "library.bulk.retry"
            },
            &id,
            |effects| effects.change_bulk_library(actor.user_id, id, cancel),
        )
    }
    pub(crate) fn run_bulk_library(&self, claim: &JobClaim) -> Result<serde_json::Value> {
        let record = self.bulk_port.bulk_library(claim.user_id, claim.id)?;
        let capacity = self
            .bulk_port
            .bulk_library_capacity(claim.user_id, record.summary.kind)?;
        if capacity == 0 {
            return Ok(serde_json::json!({"yielded":true}));
        }
        let page = self.bulk_port.bulk_library_members(
            claim.user_id,
            claim.id,
            None,
            capacity.min(16) as u16,
            BulkMemberFilter::Admission,
        )?;
        for member in page.items {
            let outcome = (|| -> Result<()> {
                let item = self.reader.item(member.item.id)?;
                self.content_write_eligibility_for_user(claim.user_id, item.collection_id)?;
                if serde_json::to_value(&item).map_err(|e| Error::Storage(e.to_string()))?
                    != serde_json::to_value(&member.item)
                        .map_err(|e| Error::Storage(e.to_string()))?
                {
                    return Err(Error::Conflict);
                }
                let input = member.input.ok_or(Error::Conflict)?;
                let (id, kind, units) = match &input {
                    FrozenJobInput::AccurateRip(v) => (v.job_id, "accuraterip_check", 0),
                    FrozenJobInput::DerivativeBatch(v) => {
                        (v.batch_id, "music_derivative_batch", v.members.len() as u64)
                    }
                    FrozenJobInput::MetadataResolution(v) => (
                        JobId(v.acquisition.snapshot_id.0),
                        "metadata_resolution",
                        36 * 1024 * 1024,
                    ),
                    FrozenJobInput::ExistingMetadata(v) => {
                        (v.job_id, "metadata_existing", 8 * 1024 * 1024)
                    }
                    _ => return Err(Error::Conflict),
                };
                self.bulk_port.admit_bulk_library_member(
                    claim,
                    member.view.position,
                    JobSpec {
                        id,
                        user_id: claim.user_id,
                        collection_id: Some(item.collection_id),
                        item_id: Some(item.id),
                        kind: kind.into(),
                        phase: "queued".into(),
                        total_units: units,
                        request: serde_json::json!({"item_id":item.id,"bulk_parent":claim.id}),
                        frozen_input: Some(input),
                    },
                )?;
                Ok(())
            })();
            if let Err(error) = outcome {
                if matches!(error, Error::JobCancelled) {
                    return Err(error);
                }
                if matches!(error, Error::Unavailable(_))
                    && self
                        .bulk_port
                        .bulk_library_capacity(claim.user_id, record.summary.kind)?
                        == 0
                {
                    break;
                }
                self.bulk_port.settle_bulk_library_member(
                    claim.user_id,
                    claim.id,
                    member.view.position,
                    BulkMemberState::Review,
                    Some(error.to_string()),
                )?;
            }
        }
        if !self
            .bulk_port
            .bulk_library_members(
                claim.user_id,
                claim.id,
                None,
                1,
                BulkMemberFilter::Admission,
            )?
            .items
            .is_empty()
        {
            Ok(serde_json::json!({"yielded":true}))
        } else {
            Ok(serde_json::json!({"batch_id":claim.id}))
        }
    }
    pub fn resume_bulk_library(&self) -> Result<()> {
        let mut after = None;
        loop {
            let page = self.bulk_port.unfinished_bulk_library(after, 100)?;
            if page.is_empty() {
                break;
            }
            for record in page {
                after = Some(record.summary.id);
                let outcome = (|| -> Result<()> {
                    let mut cursor = None;
                    loop {
                        let page = self.bulk_port.bulk_library_members(
                            record.user_id,
                            record.summary.id,
                            cursor,
                            64,
                            BulkMemberFilter::Unsettled,
                        )?;
                        for member in page.items {
                            if let Some(job) = member.view.job_id {
                                let actor = Principal {
                                    user_id: record.user_id,
                                    administrator: false,
                                    device_id: None,
                                };
                                let state = if record.summary.kind
                                    == BulkLibraryKind::MusicPreparation
                                {
                                    let view = self.music_preparation_batch(&actor, job)?;
                                    if view.queued + view.running > 0
                                        || matches!(
                                            view.admission_state,
                                            JobState::Queued
                                                | JobState::Running
                                                | JobState::CancelRequested
                                        )
                                    {
                                        None
                                    } else if view.failed + view.ineligible > 0 {
                                        Some((BulkMemberState::Failed,view.admission_error.or(Some("Some tracks could not be prepared. Open this album for individual results.".into()))))
                                    } else if view.cancelled > 0 {
                                        Some((BulkMemberState::Cancelled, None))
                                    } else {
                                        Some((BulkMemberState::Succeeded, None))
                                    }
                                } else {
                                    let job = self.jobs.job_record(job, record.user_id)?;
                                    match job.state {
                                        JobState::Completed
                                            if record.summary.kind
                                                == BulkLibraryKind::AccurateRip =>
                                        {
                                            let confirmed = job
                                                .result
                                                .as_ref()
                                                .and_then(|r| {
                                                    r.get("completion")
                                                        .unwrap_or(r)
                                                        .get("confirmed")
                                                })
                                                .and_then(|r| r.as_bool())
                                                .unwrap_or(false);
                                            Some(if confirmed {
                                                (BulkMemberState::Succeeded, None)
                                            } else {
                                                (BulkMemberState::Review,Some("Audio check finished without confirming every disc. Open the album for AccurateRip results.".into()))
                                            })
                                        }
                                        JobState::Completed => {
                                            let partial = job
                                                .result
                                                .as_ref()
                                                .and_then(|r| {
                                                    r.get("completion")
                                                        .unwrap_or(r)
                                                        .get("artwork_error")
                                                })
                                                .and_then(|r| r.as_str())
                                                .map(str::to_string);
                                            if partial.is_some() {
                                                Some((BulkMemberState::Failed, partial))
                                            } else {
                                                let review = self
                                                    .metadata_review(&actor, member.view.item_id)?;
                                                let reasons=review.review_reasons.into_iter().filter(|r|matches!(r,MetadataReviewReason::AlbumAmbiguity|MetadataReviewReason::IncompleteProviderInformation|MetadataReviewReason::NoMatch|MetadataReviewReason::ManualConflict)).collect::<Vec<_>>();
                                                if reasons.is_empty() {
                                                    Some((BulkMemberState::Succeeded, None))
                                                } else {
                                                    Some((BulkMemberState::Review,Some("Metadata lookup finished; review the album alternatives, missing information or manual conflicts.".into())))
                                                }
                                            }
                                        }
                                        JobState::Failed => {
                                            Some((BulkMemberState::Failed, job.error))
                                        }
                                        JobState::Cancelled => {
                                            Some((BulkMemberState::Cancelled, None))
                                        }
                                        _ => None,
                                    }
                                };
                                if let Some((state, reason)) = state {
                                    self.bulk_port.settle_bulk_library_member(
                                        record.user_id,
                                        record.summary.id,
                                        member.view.position,
                                        state,
                                        reason,
                                    )?;
                                }
                            }
                        }
                        let Some(next) = page.next_cursor else {
                            break;
                        };
                        cursor = Some(
                            next.parse()
                                .map_err(|_| Error::Storage("invalid bulk cursor".into()))?,
                        );
                    }
                    self.bulk_port
                        .reconcile_bulk_library(record.user_id, record.summary.id)?;
                    Ok(())
                })();
                if let Err(error) = outcome
                    && !matches!(
                        error,
                        Error::Forbidden | Error::Unauthenticated | Error::NotFound
                    )
                {
                    return Err(error);
                }
            }
        }
        Ok(())
    }
}
