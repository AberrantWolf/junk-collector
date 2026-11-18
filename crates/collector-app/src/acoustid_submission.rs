//! Explicit contributions from a current, user-reviewed digital album match.
use crate::*;
use collector_modules::music::AcoustIdQuery;

pub const ACOUSTID_SUBMISSION_COMMAND: &str = "music.acoustid.submit";

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SubmitAcoustIdFingerprints {
    pub job_id: JobId,
    pub release_id: String,
    pub track_assignments: Vec<DigitalTrackAssignment>,
    #[schema(write_only)]
    pub user_key: String,
}

pub struct AcoustIdContribution {
    pub recording_id: String,
    pub query: AcoustIdQuery,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum AcoustIdSubmissionOutcome {
    Accepted { submission_ids: Vec<u64> },
    Unknown,
}

pub trait AcoustIdSubmissionPort: Send + Sync {
    /// Once delivery is uncertain, return Unknown so replay cannot resubmit.
    fn submit(
        &self,
        user_key: &str,
        tracks: &[AcoustIdContribution],
        before_send: &mut dyn FnMut() -> Result<()>,
    ) -> Result<AcoustIdSubmissionOutcome>;
}

impl CollectionService {
    pub fn with_acoustid_submitter(mut self, submitter: Arc<dyn AcoustIdSubmissionPort>) -> Self {
        self.acoustid_submitter = Some(submitter);
        self
    }

    pub fn submit_acoustid_fingerprints(
        &self,
        actor: &Principal,
        id: ItemId,
        mut command: SubmitAcoustIdFingerprints,
        context: &CommandContext,
    ) -> Result<AcoustIdSubmissionOutcome> {
        let item = self.reader.item(id)?;
        self.authorize(actor, item.collection_id, Action::Curate)?;
        if !junk_libs_music::valid_mbid(&command.release_id)
            || command.track_assignments.is_empty()
            || command.track_assignments.len() > junk_libs_music::MAX_TRACKS
            || command
                .track_assignments
                .iter()
                .any(|assignment| assignment.key.len() > 73)
        {
            return Err(Error::Invalid("Invalid reviewed track assignments".into()));
        }
        if command.user_key.is_empty()
            || command.user_key.len() > 128
            || !command
                .user_key
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_".contains(&b))
        {
            return Err(Error::Invalid("Enter your AcoustID user API key".into()));
        }
        command.track_assignments.sort_by(|a, b| a.key.cmp(&b.key));
        // The personal key authorizes the request. The reviewed mapping identifies its effect.
        let request = (
            id,
            command.job_id,
            &command.release_id,
            &command.track_assignments,
        );
        self.execute_command(
            actor,
            context,
            ACOUSTID_SUBMISSION_COMMAND,
            &request,
            |_| {
                let submitter = self.acoustid_submitter.as_ref().ok_or_else(|| {
                    Error::Unavailable("AcoustID fingerprint submission is unavailable".into())
                })?;
                let item = self.reader.item(id)?;
                self.authorize(actor, item.collection_id, Action::Curate)?;
                let job = self.jobs.job_record(command.job_id, actor.user_id)?;
                if job.state != JobState::Completed {
                    return Err(Error::Conflict);
                }
                let Some(FrozenJobInput::DigitalMatching(frozen)) =
                    self.jobs.frozen_job_input(command.job_id, actor.user_id)?
                else {
                    return Err(Error::Conflict);
                };
                let (_, recordings) = self.resolve_digital_assignments(
                    actor.user_id,
                    &item,
                    command.job_id,
                    &command.release_id,
                    Some(&command.track_assignments),
                )?;
                let checkpoint = self
                    .jobs
                    .digital_checkpoint(command.job_id, actor.user_id)?
                    .ok_or(Error::Conflict)?;
                let audio = crate::digital_pipeline::acoustid_submission_audio(&checkpoint)?;
                let mut contributions = Vec::new();
                let mut submitted_files = Vec::new();
                for (key, fingerprint) in audio {
                    let Some(recording_id) = recordings.get(&key) else {
                        continue;
                    };
                    let (representation, _) = key.split_once(':').ok_or(Error::Conflict)?;
                    let source: RepresentationId =
                        representation.parse().map_err(|_| Error::Conflict)?;
                    submitted_files.push((source, fingerprint.file_index));
                    let query = AcoustIdQuery {
                        refresh: false,
                        duration: fingerprint.audio.duration_ms.div_ceil(1000),
                        fingerprint: fingerprint.audio.fingerprint,
                    };
                    query.validate()?;
                    contributions.push(AcoustIdContribution {
                        recording_id: recording_id.clone(),
                        query,
                    });
                }
                if contributions.is_empty() {
                    return Err(Error::Invalid(
                        "No reviewed tracks need an AcoustID contribution".into(),
                    ));
                }
                submitter.submit(&command.user_key, &contributions, &mut || {
                    let item = self.reader.item(id)?;
                    self.authorize(actor, item.collection_id, Action::Curate)?;
                    if !self.digital_matching_inputs_current(&item, &frozen)? {
                        return Err(Error::Conflict);
                    }
                    // Recorded fingerprints describe file bytes, so check current
                    // visibility separately after waiting for provider admission.
                    for (source, index) in &submitted_files {
                        let file = item
                            .captures
                            .iter()
                            .flat_map(|capture| &capture.representations)
                            .find(|representation| representation.id == *source)
                            .and_then(|representation| representation.files.get(*index))
                            .ok_or(Error::Conflict)?;
                        if file.visibility != Visibility::Shared {
                            return Err(Error::Forbidden);
                        }
                    }
                    Ok(())
                })
            },
        )
    }
}
