//! Explicit source-removal policy; filesystem identity/copy mechanisms belong to Store.
use crate::*;

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CleanupInboxCommand {
    pub source_is_idle: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct InboxCleanupView {
    pub job_id: JobId,
    pub state: JobState,
    pub removed_bytes: ByteLength,
    pub total_bytes: ByteLength,
    pub completed: bool,
    pub retained_files: bool,
    pub recovery_directory: Option<String>,
    pub error: Option<String>,
}

impl CollectionService {
    pub fn request_inbox_cleanup(
        &self,
        actor: &Principal,
        id: ImportId,
        command: CleanupInboxCommand,
        context: &CommandContext,
        resolve: impl FnOnce(&ImportSourceSummary) -> Result<String>,
    ) -> Result<JobId> {
        let record = self.authorized_provisional_import(actor, id)?;
        self.content_write_eligibility(actor, record.view.collection_id)?;
        self.execute_command(
            actor,
            context,
            "ingest.inbox_cleanup",
            &(id, &command),
            |effects| {
                if !command.source_is_idle {
                    return Err(Error::Invalid(
                        "Confirm that copying and editing the inbox source have finished.".into(),
                    ));
                }
                let summary = self.ingest_summary(actor, record.view.clone())?;
                if record.view.source.kind != ImportSourceKind::Inbox
                    || !summary.permissions.can_clean_inbox
                {
                    return Err(Error::Invalid(
                        "Finish preservation and wait for requested processing to settle before removing inbox files."
                            .into(),
                    ));
                }
                let previous = self.provisional_imports.inbox_cleanup_status(id)?;
                if let Some(cleanup) = &previous
                    && (cleanup.completed || matches!(cleanup.state,JobState::Queued|JobState::Running|JobState::CancelRequested))
                { return Ok(cleanup.job_id); }
                let path = if previous.is_some_and(|value| value.recovery_directory.is_some()) {
                    String::new() // Resume the frozen quarantine, never resolve a new package.
                } else {
                    resolve(&record.view.source)?
                };
                effects.queue_inbox_cleanup(
                    id,
                    path,
                    JobSpec {
                        id: deterministic_job_id(
                            actor.user_id,
                            "ingest.inbox_cleanup",
                            context.key(),
                        ),
                        user_id: actor.user_id,
                        collection_id: Some(record.view.collection_id),
                        item_id: None,
                        kind: "inbox_cleanup".into(),
                        phase: "queued".into(),
                        total_units: summary.total_bytes.0,
                        request: serde_json::json!({"import_id": id}),
                        frozen_input: None,
                    },
                )
            },
        )
    }

    pub(crate) fn run_inbox_cleanup(&self, claim: &JobClaim) -> Result<serde_json::Value> {
        let id = claim
            .request
            .get("import_id")
            .and_then(serde_json::Value::as_str)
            .ok_or(Error::Conflict)?
            .parse()
            .map_err(|_| Error::Conflict)?;
        let record = self.provisional_imports.import_record(id)?;
        let guard = &mut || {
            self.content_write_eligibility_for_user(claim.user_id, record.view.collection_id)?;
            if self
                .jobs
                .cancellation_requested(claim.id, &claim.claim_token)?
            {
                return Err(Error::JobCancelled);
            }
            Ok(())
        };
        guard()?;
        let result = self
            .provisional_imports
            .clean_inbox_source(id, claim, guard)?;
        serde_json::to_value(result).map_err(|error| Error::Storage(error.to_string()))
    }
}
