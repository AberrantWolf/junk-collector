//! Requester-private work presentation and lifecycle policy.
use super::*;

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum WorkTarget {
    Import(ImportId),
    Job(JobId),
}
impl WorkTarget {
    pub fn key(&self) -> String {
        match self {
            Self::Import(id) => format!("import:{id}"),
            Self::Job(id) => format!("job:{id}"),
        }
    }
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, utoipa::ToSchema, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
#[serde(default, deny_unknown_fields)]
pub struct WorkQuery {
    pub collection_id: Option<CollectionId>,
    pub item_id: Option<ItemId>,
    pub unattached: bool,
    pub search: String,
    pub status: String,
    pub archived: bool,
    pub after: Option<i64>,
    pub before: Option<i64>,
    pub cursor: Option<String>,
    pub limit: Option<u16>,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct WorkEntry {
    pub target: WorkTarget,
    pub collection_id: Option<CollectionId>,
    pub collection_name: String,
    pub item_id: Option<ItemId>,
    pub item_title: Option<String>,
    pub name: String,
    pub kind: String,
    pub phase: String,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub archived: bool,
    pub clear_pending: bool,
    pub clear_error: Option<String>,
    pub received_bytes: u64,
    pub reclaimed_bytes: u64,
    pub pending_bytes: u64,
    pub cleanup_error: Option<String>,
    pub revision: Option<u64>,
    pub guided: bool,
    pub active: bool,
    pub shared: bool,
    #[serde(default)]
    pub actions: Vec<WorkHistoryAction>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum WorkHistoryAction {
    Dismiss,
    Restore,
    Discard,
    CancelAndClear,
    RetryCleanup,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkHistoryCommand {
    pub target: WorkTarget,
    pub action: WorkHistoryAction,
}
/// Re-evaluated under the adapter transaction as well as before receipt admission.
pub fn work_action_allowed(entry: &WorkEntry, action: WorkHistoryAction) -> Result<()> {
    let allowed = match action {
        WorkHistoryAction::Restore => entry.archived && !entry.clear_pending,
        WorkHistoryAction::Dismiss => {
            !entry.active
                && !entry.clear_pending
                && entry.pending_bytes == 0
                && !matches!(entry.status.as_str(), "needs_input")
        }
        WorkHistoryAction::Discard => {
            matches!(entry.target, WorkTarget::Import(_))
                && !entry.shared
                && !entry.active
                && entry.phase != "published"
                && entry.phase != "publishing"
        }
        WorkHistoryAction::CancelAndClear => {
            !entry.shared && !entry.archived && !entry.clear_pending && entry.status != "completed"
        }
        WorkHistoryAction::RetryCleanup => {
            entry.clear_error.is_some() || entry.cleanup_error.is_some()
        }
    };
    if allowed {
        Ok(())
    } else {
        Err(Error::Invalid(
            "This action is not eligible in the current state. Refresh the work entry.".into(),
        ))
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkJobsQuery {
    pub target: WorkTarget,
    pub cursor: Option<JobId>,
    pub limit: Option<u16>,
}
#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
pub struct WorkJobView {
    pub job: JobView,
    pub prior_failures: Vec<IngestAttemptView>,
}
impl CollectionService {
    pub fn work_jobs(&self, actor: &Principal, query: WorkJobsQuery) -> Result<Page<WorkJobView>> {
        self.work_history_entry(actor, &query.target)?;
        if !(1..=100).contains(&query.limit.unwrap_or(25)) {
            return Err(Error::Invalid("Invalid work job limit".into()));
        }
        let page = self.jobs.work_job_records(actor.user_id, &query)?;
        Ok(Page {
            items: page
                .items
                .into_iter()
                .map(|job| {
                    let prior_failures = job
                        .result
                        .as_ref()
                        .and_then(|v| v.get("prior_failures"))
                        .and_then(serde_json::Value::as_array)
                        .into_iter()
                        .flatten()
                        .filter_map(|failure| {
                            Some(IngestAttemptView {
                                job_id: job.id,
                                step: job.kind.clone(),
                                attempt: failure.get("attempt")?.as_u64()?.try_into().ok()?,
                                state: JobState::Failed,
                                started_at: None,
                                finished_at: chrono::DateTime::from_timestamp(
                                    failure.get("failed_at")?.as_i64()?,
                                    0,
                                ),
                                diagnostic: failure
                                    .get("error")
                                    .and_then(serde_json::Value::as_str)
                                    .map(|v| v.chars().take(512).collect()),
                            })
                        })
                        .collect();
                    WorkJobView {
                        job: job.into(),
                        prior_failures,
                    }
                })
                .collect(),
            next_cursor: page.next_cursor,
        })
    }
    pub fn work_history(&self, actor: &Principal, query: WorkQuery) -> Result<Page<WorkEntry>> {
        if query.search.len() > 200
            || query.cursor.as_ref().is_some_and(|c| c.len() > 512)
            || ![
                "",
                "active",
                "needs_input",
                "completed",
                "failed",
                "cancelled",
            ]
            .contains(&query.status.as_str())
            || !(1..=100).contains(&query.limit.unwrap_or(50))
            || query.after.zip(query.before).is_some_and(|(a, b)| a > b)
        {
            return Err(Error::Invalid("Invalid work history filters".into()));
        }
        if let Some(id) = query.collection_id {
            self.authorize(actor, id, Action::Curate)?;
        }
        let mut page = self.jobs.work_page(actor.user_id, &query)?;
        for entry in &mut page.items {
            self.present_work_actions(actor, entry);
        }
        Ok(page)
    }
    pub fn work_history_entry(&self, actor: &Principal, target: &WorkTarget) -> Result<WorkEntry> {
        let mut entry = self.jobs.work_entry(actor.user_id, target)?;
        if let Some(id) = entry.collection_id {
            self.authorize(actor, id, Action::Curate)?;
        }
        self.present_work_actions(actor, &mut entry);
        Ok(entry)
    }
    fn present_work_actions(&self, actor: &Principal, entry: &mut WorkEntry) {
        if entry
            .collection_id
            .is_some_and(|id| self.content_write_eligibility(actor, id).is_err())
        {
            entry
                .actions
                .retain(|a| matches!(a, WorkHistoryAction::Dismiss | WorkHistoryAction::Restore));
        }
    }
    pub fn change_work_history(
        &self,
        actor: &Principal,
        command: WorkHistoryCommand,
        context: &CommandContext,
    ) -> Result<()> {
        let entry = self.work_history_entry(actor, &command.target)?;
        if matches!(
            command.action,
            WorkHistoryAction::Discard
                | WorkHistoryAction::CancelAndClear
                | WorkHistoryAction::RetryCleanup
        ) && let Some(id) = entry.collection_id
        {
            self.content_write_eligibility(actor, id)?;
        }
        // Eligibility is checked inside the effect so a committed receipt can replay.
        self.execute_command(actor, context, "work.history", &command, |effects| {
            effects.change_work_history(actor.user_id, &command)
        })
    }
    pub(crate) fn settle_work_clear(&self, user: UserId, target: &WorkTarget) -> Result<bool> {
        let entry = self.jobs.work_entry(user, target)?;
        if entry.shared {
            return Err(Error::Unavailable("Independent requests still need this work. Cancel those requests before clearing staging.".into()));
        }
        match target {
            WorkTarget::Job(id) => {
                let job = self.jobs.job_record(*id, user)?;
                if matches!(job.state, JobState::Queued | JobState::Running) {
                    if job.kind == "music_derivative_batch" {
                        self.jobs.cancel_preparation_batch(*id, user)?;
                    } else {
                        self.jobs.request_job_cancellation(*id, user)?;
                    }
                }
                Ok(!matches!(
                    self.jobs.job_record(*id, user)?.state,
                    JobState::Queued | JobState::Running | JobState::CancelRequested
                ))
            }
            WorkTarget::Import(id) => {
                let record = self.provisional_imports.import_record(*id)?;
                if record.view.phase == ImportPhase::Published
                    || self.reader.capture(record.view.capture_id).is_ok()
                {
                    if entry.guided {
                        self.provisional_imports
                            .cancel_guided_import(*id, record.view.revision)?;
                    }
                    return Ok(!self.jobs.work_entry(user, target)?.active);
                }
                if self
                    .provisional_imports
                    .guided_cleanup_status(*id)?
                    .is_none()
                {
                    self.provisional_imports
                        .discard_guided_import(*id, record.view.revision)?;
                }
                self.provisional_imports.cleanup_import(*id)?;
                Ok(self
                    .provisional_imports
                    .guided_cleanup_status(*id)?
                    .is_some_and(|v| !v.cleanup_pending))
            }
        }
    }
}
