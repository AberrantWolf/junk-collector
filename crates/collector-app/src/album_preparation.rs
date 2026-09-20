//! Application policy for durable, bounded album preparation over track Jobs.
use crate::derivatives::profile_for_recipe;
use crate::{
    CollectionService, CommandContext, EncoderAvailability, FrozenDerivativeJob, FrozenJobInput,
    JobClaim, JobSpec, MusicTrackAvailability, MusicTrackAvailabilityState, PrepareMusicDerivative,
    Principal,
};
use collector_core::{
    CaptureId, CollectionBinding, CollectionId, ConversionProfile, ConversionTrack, Error, JobId,
    PackageScope, RepresentationId, Result,
};
use serde::{Deserialize, Serialize};

pub const MAX_ACTIVE_MUSIC_PREPARATION_BATCHES: u32 = 32;
const MAX_ALBUM_PREPARATION_MEMBERS: usize = 256;

#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
pub struct MusicAlbumSourcePlan {
    pub capture_id: CaptureId,
    pub source_representation_id: RepresentationId,
    pub track_count: u16,
    pub default_selected: bool,
    pub already_available: u16,
    pub preparing: u16,
    pub eligible: u16,
}

#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
pub struct MusicAlbumDiscPlan {
    /// Stable representative capture for this disc group; sources retain their actual capture IDs.
    pub capture_id: CaptureId,
    pub sources: Vec<MusicAlbumSourcePlan>,
    pub ambiguous: bool,
    pub message: Option<String>,
}

#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
pub struct MusicAlbumPreparationPlan {
    pub item_id: collector_core::ItemId,
    pub profile: ConversionProfile,
    pub discs: Vec<MusicAlbumDiscPlan>,
    pub total_tracks: u16,
    pub blockers: Vec<String>,
    pub can_submit: bool,
    pub maximum_members: u16,
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PrepareMusicAlbum {
    pub source_representation_ids: Vec<RepresentationId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum MusicPreparationMemberState {
    AlreadyAvailable,
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
    Ineligible,
}

#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
pub struct MusicPreparationMemberView {
    pub member_id: JobId,
    pub capture_id: CaptureId,
    pub source_representation_id: RepresentationId,
    pub track_position: u16,
    pub title: String,
    pub state: MusicPreparationMemberState,
    pub job_id: Option<JobId>,
    pub attempt: Option<u32>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
pub struct MusicPreparationBatchView {
    pub batch_id: JobId,
    pub item_id: collector_core::ItemId,
    pub profile: ConversionProfile,
    pub members: Vec<MusicPreparationMemberView>,
    pub queued: u16,
    pub running: u16,
    pub succeeded: u16,
    pub failed: u16,
    pub cancelled: u16,
    pub ineligible: u16,
    pub cancelled_by_user: bool,
    pub admission_state: crate::JobState,
    pub admission_error: Option<String>,
    pub can_cancel: bool,
    pub can_retry: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "disposition", rename_all = "snake_case")]
pub enum FrozenPreparationMember {
    AlreadyAvailable {
        member_id: JobId,
        capture_id: CaptureId,
        source_representation_id: RepresentationId,
        track_position: u16,
        title: String,
        representation_id: RepresentationId,
    },
    Prepare {
        member_id: JobId,
        job_id: JobId,
        title: String,
        input: Box<FrozenDerivativeJob>,
    },
    Ineligible {
        member_id: JobId,
        capture_id: CaptureId,
        source_representation_id: RepresentationId,
        track_position: u16,
        title: String,
        reason: String,
    },
}

impl FrozenPreparationMember {
    fn member_id(&self) -> JobId {
        match self {
            Self::AlreadyAvailable { member_id, .. }
            | Self::Prepare { member_id, .. }
            | Self::Ineligible { member_id, .. } => *member_id,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrozenDerivativeBatchJob {
    pub batch_id: JobId,
    pub user_id: collector_core::UserId,
    pub collection_id: CollectionId,
    pub collection_binding: CollectionBinding,
    pub item_id: collector_core::ItemId,
    pub profile: ConversionProfile,
    pub members: Vec<FrozenPreparationMember>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl FrozenDerivativeBatchJob {
    pub fn validate(&self) -> Result<()> {
        self.collection_binding.validate()?;
        if self.collection_binding
            != (CollectionBinding::Assigned {
                module: collector_core::ModuleId("org.junk.music".into()),
            })
            || self.profile != ConversionProfile::Flac
            || self.members.is_empty()
            || self.members.len() > MAX_ALBUM_PREPARATION_MEMBERS
        {
            return Err(Error::Invalid("invalid music preparation batch".into()));
        }
        let mut members = std::collections::BTreeSet::new();
        for member in &self.members {
            if !members.insert(member.member_id()) {
                return Err(Error::Invalid("duplicate preparation batch member".into()));
            }
            let (capture, source, position, title) = match member {
                FrozenPreparationMember::Prepare { input, title, .. } => {
                    input.validate()?;
                    if input.collection_binding != self.collection_binding {
                        return Err(Error::Invalid(
                            "invalid preparation collection binding".into(),
                        ));
                    }
                    let PackageScope::Representation(source) =
                        input.recipe.source_fingerprint.source
                    else {
                        return Err(Error::Conflict);
                    };
                    (
                        input.capture_id,
                        source,
                        input.recipe.tracks[0].position,
                        title,
                    )
                }
                FrozenPreparationMember::AlreadyAvailable {
                    capture_id,
                    source_representation_id,
                    track_position,
                    title,
                    ..
                }
                | FrozenPreparationMember::Ineligible {
                    capture_id,
                    source_representation_id,
                    track_position,
                    title,
                    ..
                } => (
                    *capture_id,
                    *source_representation_id,
                    *track_position,
                    title,
                ),
            };
            if member.member_id() != preparation_member_id(self.batch_id, capture, source, position)
                || title.len() > 8192
            {
                return Err(Error::Invalid("invalid preparation member identity".into()));
            }
            if let FrozenPreparationMember::Prepare { input, .. } = member
                && (input.collection_id != self.collection_id
                    || profile_for_recipe(&input.recipe) != self.profile)
            {
                return Err(Error::Invalid(
                    "batch member disagrees with batch scope".into(),
                ));
            }
        }
        Ok(())
    }
}

impl CollectionService {
    pub fn music_album_preparation_plan(
        &self,
        actor: &Principal,
        item_id: collector_core::ItemId,
    ) -> Result<MusicAlbumPreparationPlan> {
        let item = self.reader.item(item_id)?;
        self.authorize(actor, item.collection_id, crate::Action::Browse)?;
        let rows = self.music_track_availability_for_item(actor.user_id, &item)?;
        Ok(album_plan_for_item(&item, &rows))
    }

    pub fn prepare_music_album(
        &self,
        actor: &Principal,
        item_id: collector_core::ItemId,
        command: PrepareMusicAlbum,
        context: &CommandContext,
    ) -> Result<MusicPreparationBatchView> {
        let item = self.reader.item(item_id)?;
        self.authorize(actor, item.collection_id, crate::Action::Curate)?;
        validate_album_sources(&command.source_representation_ids)?;
        let batch_id = self.execute_command(
            actor,
            context,
            "music.album.prepare",
            &(item_id, &command),
            |effects| {
                let id = crate::deterministic_job_id(
                    actor.user_id,
                    "music.album.prepare",
                    context.key(),
                );
                let frozen = self.freeze_music_album_batch(
                    actor.user_id,
                    &item,
                    &command.source_representation_ids,
                    id,
                )?;
                Self::enqueue_music_album_batch(effects, frozen, None)
            },
        )?;
        self.music_preparation_batch(actor, batch_id)
    }

    fn freeze_music_album_batch(
        &self,
        user_id: collector_core::UserId,
        item: &collector_core::Item,
        selected_sources: &[RepresentationId],
        batch_id: JobId,
    ) -> Result<FrozenDerivativeBatchJob> {
        let eligibility = self.content_write_eligibility_for_job(
            user_id,
            Some(item.collection_id),
            Some(item.id),
        )?;
        if eligibility.owner.0 != "org.junk.music" {
            return Err(Error::Invalid("music collection required".into()));
        }
        let rows = self.music_track_availability_for_item(user_id, item)?;
        let plan = album_plan_for_item(item, &rows);
        let selected = selected_sources
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>();
        let mut selected_capture = std::collections::BTreeSet::new();
        for source in selected_sources {
            let matches = plan
                .discs
                .iter()
                .filter(|disc| {
                    disc.sources
                        .iter()
                        .any(|candidate| candidate.source_representation_id == *source)
                })
                .collect::<Vec<_>>();
            if matches.len() != 1 || !selected_capture.insert(matches[0].capture_id) {
                return Err(Error::Invalid(
                    "select at most one supported source for each captured disc".into(),
                ));
            }
        }
        if selected.is_empty() {
            return Err(Error::Invalid("select at least one captured disc".into()));
        }
        let selected_rows = rows.iter().filter(|row| {
            row.profile == ConversionProfile::Flac
                && selected.contains(&row.source_representation_id)
        });
        if selected_rows.clone().count() > MAX_ALBUM_PREPARATION_MEMBERS {
            return Err(Error::Invalid(format!(
                "Select at most {MAX_ALBUM_PREPARATION_MEMBERS} tracks per album batch."
            )));
        }
        if !selected_rows.clone().any(|row| {
            row.can_prepare
                || matches!(
                    &row.state,
                    MusicTrackAvailabilityState::Available { .. }
                        | MusicTrackAvailabilityState::Preparing
                )
        }) {
            let reason = selected_rows
                .filter_map(|row| match &row.state {
                    MusicTrackAvailabilityState::Unavailable { reason } => Some(reason.clone()),
                    _ => None,
                })
                .next()
                .unwrap_or_else(|| "The selected sources have no eligible FLAC tracks.".into());
            return Err(Error::Unavailable(reason));
        }
        let active = item
            .captures
            .iter()
            .map(|capture| self.jobs.active_derivative_jobs(capture.id))
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        let mut members = Vec::new();
        for mut row in rows.into_iter().filter(|row| {
            row.profile == ConversionProfile::Flac
                && selected.contains(&row.source_representation_id)
        }) {
            // Another actor's Job is private. Admit our own request, converging on
            // the immutable output at publication without exposing their Job.
            if matches!(row.state, MusicTrackAvailabilityState::Preparing)
                && !active.iter().any(|job| {
                    job.user_id == user_id
                        && job.input.recipe.source_fingerprint.source
                            == PackageScope::Representation(row.source_representation_id)
                        && job
                            .input
                            .recipe
                            .tracks
                            .first()
                            .is_some_and(|track| track.position == row.track_position)
                        && profile_for_recipe(&job.input.recipe) == ConversionProfile::Flac
                })
            {
                row.state = MusicTrackAvailabilityState::Unavailable {
                    reason: "Another user is preparing this output.".into(),
                };
                row.can_prepare = true;
            }
            let member_id = preparation_member_id(
                batch_id,
                row.capture_id,
                row.source_representation_id,
                row.track_position,
            );
            match row.state {
                MusicTrackAvailabilityState::Available {
                    representation_id, ..
                } => members.push(FrozenPreparationMember::AlreadyAvailable {
                    member_id,
                    capture_id: row.capture_id,
                    source_representation_id: row.source_representation_id,
                    track_position: row.track_position,
                    title: row.title,
                    representation_id,
                }),
                MusicTrackAvailabilityState::Preparing => {
                    let track = ConversionTrack {
                        position: row.track_position,
                        start: row.start_frame,
                        length: row.frame_count,
                    };
                    let active = active
                        .iter()
                        .find(|job| {
                            job.user_id == user_id
                                && job.input.recipe.source_fingerprint.source
                                    == PackageScope::Representation(row.source_representation_id)
                                && job.input.recipe.tracks.as_slice() == [track.clone()]
                                && profile_for_recipe(&job.input.recipe) == ConversionProfile::Flac
                        })
                        .ok_or_else(|| {
                            Error::Storage("preparing track has no active derivative Job".into())
                        })?;
                    members.push(FrozenPreparationMember::Prepare {
                        member_id,
                        job_id: active.id,
                        title: row.title,
                        input: Box::new(active.input.clone()),
                    });
                }
                MusicTrackAvailabilityState::Unavailable { reason: _ } if row.can_prepare => {
                    let capture = self.reader.capture(row.capture_id)?;
                    let selection = self.detect_music_selection(
                        capture.collection_id,
                        row.source_representation_id,
                    )?;
                    let version = match self.encoder_availability(&ConversionProfile::Flac) {
                        EncoderAvailability::Available { version, .. } => version,
                        EncoderAvailability::Unavailable { reason } => {
                            members.push(FrozenPreparationMember::Ineligible {
                                member_id,
                                capture_id: row.capture_id,
                                source_representation_id: row.source_representation_id,
                                track_position: row.track_position,
                                title: row.title,
                                reason,
                            });
                            continue;
                        }
                    };
                    let frozen = self.freeze_music_derivative(
                        &capture,
                        eligibility.binding.clone(),
                        &PrepareMusicDerivative {
                            representation_id: row.source_representation_id,
                            selection,
                            track_position: row.track_position,
                            profile: ConversionProfile::Flac,
                            encoder_version: version,
                        },
                    )?;
                    let job_id = crate::deterministic_job_id(
                        user_id,
                        "music.derivative.output",
                        &frozen.representation_id.to_string(),
                    );
                    members.push(FrozenPreparationMember::Prepare {
                        member_id,
                        job_id,
                        title: row.title,
                        input: Box::new(frozen),
                    });
                }
                MusicTrackAvailabilityState::Unavailable { reason } => {
                    members.push(FrozenPreparationMember::Ineligible {
                        member_id,
                        capture_id: row.capture_id,
                        source_representation_id: row.source_representation_id,
                        track_position: row.track_position,
                        title: row.title,
                        reason,
                    });
                }
            }
        }
        if members.is_empty() {
            return Err(Error::Invalid(
                "the selected sources have no supported audio tracks".into(),
            ));
        }
        let frozen = FrozenDerivativeBatchJob {
            batch_id,
            user_id,
            collection_id: item.collection_id,
            collection_binding: eligibility.binding,
            item_id: item.id,
            profile: ConversionProfile::Flac,
            members,
            created_at: self.authentication.now(),
        };
        frozen.validate()?;
        Ok(frozen)
    }

    fn enqueue_music_album_batch(
        effects: &(impl crate::JobPort + ?Sized),
        frozen: FrozenDerivativeBatchJob,
        import_id: Option<collector_core::ImportId>,
    ) -> Result<JobId> {
        let id = frozen.batch_id;
        effects.enqueue_job(JobSpec {
            id,
            user_id: frozen.user_id,
            collection_id: Some(frozen.collection_id),
            item_id: Some(frozen.item_id),
            kind: "music_derivative_batch".into(),
            phase: "queued".into(),
            total_units: frozen.members.len() as u64,
            request: serde_json::json!({
                "item_id": frozen.item_id,
                "import_id": import_id,
            }),
            frozen_input: Some(FrozenJobInput::DerivativeBatch(frozen)),
        })
    }

    pub(crate) fn enqueue_guided_music_preparation(
        &self,
        user_id: collector_core::UserId,
        item_id: collector_core::ItemId,
        import_id: collector_core::ImportId,
    ) -> Result<JobId> {
        let item = self.reader.item(item_id)?;
        self.content_write_eligibility_for_job(user_id, Some(item.collection_id), Some(item.id))?;
        let batch_id =
            crate::deterministic_job_id(user_id, "guided.music.prepare", &import_id.to_string());
        match self.jobs.job_record(batch_id, user_id) {
            Ok(_) => return Ok(batch_id),
            Err(Error::NotFound) => {}
            Err(error) => return Err(error),
        }
        let import = self.provisional_imports.import_record(import_id)?;
        if self
            .reader
            .capture(import.view.capture_id)?
            .association
            .item_id
            != Some(item_id)
        {
            return Err(Error::Unavailable("Automatic preparation stopped because the Capture association changed. Open the associated album to prepare it explicitly.".into()));
        }
        let rows = self.music_track_availability_for_item(user_id, &item)?;
        let plan = album_plan_for_item(&item, &rows);
        if plan.discs.iter().any(|disc| disc.ambiguous) {
            return Err(Error::Unavailable(
                "Album preparation needs a source choice because this import is associated with alternative disc sources. Use Prepare album to choose explicitly."
                    .into(),
            ));
        }
        let sources = plan
            .discs
            .iter()
            .flat_map(|disc| disc.sources.iter())
            .filter(|source| source.default_selected)
            .map(|source| source.source_representation_id)
            .collect::<Vec<_>>();
        validate_album_sources(&sources)?;
        let frozen = self.freeze_music_album_batch(user_id, &item, &sources, batch_id)?;
        Self::enqueue_music_album_batch(self.jobs.as_ref(), frozen, Some(import_id))
    }

    pub(crate) fn guided_music_preparation_outcome(
        &self,
        user_id: collector_core::UserId,
        batch_id: JobId,
    ) -> Result<(bool, Option<String>)> {
        let actor = Principal {
            user_id,
            administrator: false,
            device_id: None,
        };
        let view = self.music_preparation_batch(&actor, batch_id)?;
        let pending = view.queued > 0
            || view.running > 0
            || matches!(
                view.admission_state,
                crate::JobState::Queued
                    | crate::JobState::Running
                    | crate::JobState::CancelRequested
            );
        let failures = view
            .members
            .iter()
            .filter_map(|member| {
                member.error.clone().or_else(|| {
                    (member.state == MusicPreparationMemberState::Cancelled)
                        .then(|| "Track preparation was cancelled.".into())
                })
            })
            .chain(view.admission_error)
            .collect::<Vec<_>>();
        Ok((
            !pending,
            (!failures.is_empty()).then(|| failures.join(" ").chars().take(512).collect()),
        ))
    }

    pub fn music_preparation_batches(
        &self,
        actor: &Principal,
        item_id: collector_core::ItemId,
    ) -> Result<Vec<MusicPreparationBatchView>> {
        let item = self.reader.item(item_id)?;
        self.authorize(actor, item.collection_id, crate::Action::Browse)?;
        self.jobs
            .preparation_batch_records(actor.user_id, item_id, 50)?
            .into_iter()
            .map(|record| self.music_preparation_batch_from_record(actor, record))
            .collect()
    }

    pub fn music_preparation_batch(
        &self,
        actor: &Principal,
        batch_id: JobId,
    ) -> Result<MusicPreparationBatchView> {
        let record = self.jobs.job_record(batch_id, actor.user_id)?;
        self.music_preparation_batch_from_record(actor, record)
    }

    fn music_preparation_batch_from_record(
        &self,
        actor: &Principal,
        record: crate::JobRecord,
    ) -> Result<MusicPreparationBatchView> {
        if record.kind != "music_derivative_batch" {
            return Err(Error::NotFound);
        }
        let Some(FrozenJobInput::DerivativeBatch(frozen)) =
            self.jobs.frozen_job_input(record.id, actor.user_id)?
        else {
            return Err(Error::Storage(
                "preparation batch has no frozen input".into(),
            ));
        };
        frozen.validate()?;
        let item = self.reader.item(frozen.item_id)?;
        self.authorize(actor, item.collection_id, crate::Action::Browse)?;
        if frozen.user_id != actor.user_id
            || record.collection_id != Some(frozen.collection_id)
            || record.item_id != Some(frozen.item_id)
        {
            return Err(Error::Conflict);
        }
        let cancelled_by_user = matches!(
            record.state,
            crate::JobState::Cancelled | crate::JobState::CancelRequested
        ) || record.phase == "batch_cancelled";
        let mut updated_at = record.updated_at;
        let mut members = Vec::with_capacity(frozen.members.len());
        for member in frozen.members {
            let view = match member {
                FrozenPreparationMember::AlreadyAvailable {
                    member_id,
                    capture_id,
                    source_representation_id,
                    track_position,
                    title,
                    representation_id,
                } => {
                    let capture = self.reader.capture(capture_id)?;
                    let state = if capture
                        .representations
                        .iter()
                        .any(|value| value.id == representation_id)
                    {
                        MusicPreparationMemberState::AlreadyAvailable
                    } else {
                        MusicPreparationMemberState::Failed
                    };
                    MusicPreparationMemberView {
                        member_id,
                        capture_id,
                        source_representation_id,
                        track_position,
                        title,
                        state,
                        job_id: None,
                        attempt: None,
                        error: (state == MusicPreparationMemberState::Failed).then(|| {
                            "A previously available output is no longer referenced.".into()
                        }),
                    }
                }
                FrozenPreparationMember::Ineligible {
                    member_id,
                    capture_id,
                    source_representation_id,
                    track_position,
                    title,
                    reason,
                } => MusicPreparationMemberView {
                    member_id,
                    capture_id,
                    source_representation_id,
                    track_position,
                    title,
                    state: MusicPreparationMemberState::Ineligible,
                    job_id: None,
                    attempt: None,
                    error: Some(reason),
                },
                FrozenPreparationMember::Prepare {
                    member_id,
                    job_id,
                    title,
                    input,
                    ..
                } => {
                    let published = self
                        .reader
                        .capture(input.capture_id)?
                        .representations
                        .iter()
                        .any(|value| value.id == input.representation_id);
                    let job = self.jobs.job_record(job_id, actor.user_id);
                    let (state, attempt, error) = if published {
                        (MusicPreparationMemberState::Succeeded, None, None)
                    } else {
                        match job {
                            Ok(job) => {
                                updated_at = updated_at.max(job.updated_at);
                                let state = match job.state {
                                    crate::JobState::Queued => MusicPreparationMemberState::Queued,
                                    crate::JobState::Running | crate::JobState::CancelRequested => {
                                        MusicPreparationMemberState::Running
                                    }
                                    crate::JobState::Completed => {
                                        MusicPreparationMemberState::Failed
                                    }
                                    crate::JobState::Failed => MusicPreparationMemberState::Failed,
                                    crate::JobState::Cancelled => {
                                        MusicPreparationMemberState::Cancelled
                                    }
                                };
                                let error = if job.state == crate::JobState::Completed {
                                    Some(
                                        "Track preparation completed without publishing its output."
                                            .into(),
                                    )
                                } else {
                                    job.error
                                };
                                (state, Some(job.attempt), error)
                            }
                            Err(Error::NotFound) if cancelled_by_user => {
                                (MusicPreparationMemberState::Cancelled, None, None)
                            }
                            Err(Error::NotFound) if matches!(record.state, crate::JobState::Failed | crate::JobState::Completed) => {
                                (MusicPreparationMemberState::Failed, None, Some(record.error.clone().unwrap_or_else(|| "Batch admission did not create this track Job; retry the batch.".into())))
                            }
                            Err(Error::NotFound) => (MusicPreparationMemberState::Queued, None, None),
                            Err(error) => return Err(error),
                        }
                    };
                    MusicPreparationMemberView {
                        member_id,
                        capture_id: input.capture_id,
                        source_representation_id: match input.recipe.source_fingerprint.source {
                            PackageScope::Representation(id) => id,
                            _ => return Err(Error::Conflict),
                        },
                        track_position: input.recipe.tracks[0].position,
                        title,
                        state,
                        job_id: Some(job_id),
                        attempt,
                        error,
                    }
                }
            };
            members.push(view);
        }
        let count = |state| {
            members
                .iter()
                .filter(|member| member.state == state)
                .count()
                .try_into()
                .unwrap_or(u16::MAX)
        };
        let succeeded = count(MusicPreparationMemberState::Succeeded)
            .saturating_add(count(MusicPreparationMemberState::AlreadyAvailable));
        let queued = count(MusicPreparationMemberState::Queued);
        let running = count(MusicPreparationMemberState::Running);
        let failed = count(MusicPreparationMemberState::Failed);
        let cancelled = count(MusicPreparationMemberState::Cancelled);
        let ineligible = count(MusicPreparationMemberState::Ineligible);
        let can_write = self
            .content_write_eligibility_for_job(
                actor.user_id,
                Some(item.collection_id),
                Some(item.id),
            )
            .is_ok();
        let can_stop = self
            .authorize(actor, item.collection_id, crate::Action::Curate)
            .is_ok();
        Ok(MusicPreparationBatchView {
            batch_id: frozen.batch_id,
            item_id: frozen.item_id,
            profile: frozen.profile,
            members,
            queued,
            running,
            succeeded,
            failed,
            cancelled,
            ineligible,
            cancelled_by_user,
            admission_state: record.state.clone(),
            admission_error: record.error,
            can_cancel: can_stop && !cancelled_by_user && (queued > 0 || running > 0),
            can_retry: can_write
                && (failed > 0 || cancelled > 0 || record.state == crate::JobState::Failed),
            created_at: frozen.created_at,
            updated_at,
        })
    }

    pub fn cancel_music_preparation_batch(
        &self,
        actor: &Principal,
        batch_id: JobId,
        context: &CommandContext,
    ) -> Result<MusicPreparationBatchView> {
        let record = self.jobs.job_record(batch_id, actor.user_id)?;
        self.authorize(
            actor,
            record.collection_id.ok_or(Error::NotFound)?,
            crate::Action::Curate,
        )?;
        self.execute_command(actor, context, "music.album.cancel", &batch_id, |effects| {
            effects.cancel_preparation_batch(batch_id, actor.user_id)
        })?;
        self.music_preparation_batch(actor, batch_id)
    }

    pub fn retry_music_preparation_batch(
        &self,
        actor: &Principal,
        batch_id: JobId,
        context: &CommandContext,
    ) -> Result<MusicPreparationBatchView> {
        let record = self.jobs.job_record(batch_id, actor.user_id)?;
        let item = record.item_id.ok_or(Error::NotFound)?;
        self.content_write_eligibility_for_job(actor.user_id, record.collection_id, Some(item))?;
        self.execute_command(actor, context, "music.album.retry", &batch_id, |effects| {
            effects.retry_preparation_batch(batch_id, actor.user_id)
        })?;
        self.music_preparation_batch(actor, batch_id)
    }

    pub(crate) fn run_music_derivative_batch(&self, claim: &JobClaim) -> Result<serde_json::Value> {
        let Some(FrozenJobInput::DerivativeBatch(frozen)) = claim.frozen_input.as_ref() else {
            return Err(Error::Storage(
                "preparation batch has no frozen input".into(),
            ));
        };
        frozen.validate()?;
        if claim.id != frozen.batch_id
            || claim.user_id != frozen.user_id
            || claim.collection_id != Some(frozen.collection_id)
            || claim.item_id != Some(frozen.item_id)
        {
            return Err(Error::Conflict);
        }
        let eligibility = self.content_write_eligibility_for_job(
            claim.user_id,
            claim.collection_id,
            claim.item_id,
        )?;
        self.validate_frozen_binding(&eligibility, &frozen.collection_binding)?;
        let mut admitted = 0_u64;
        for member in &frozen.members {
            if self
                .jobs
                .cancellation_requested(claim.id, &claim.claim_token)?
            {
                return Err(Error::JobCancelled);
            }
            if let FrozenPreparationMember::Prepare { job_id, input, .. } = member {
                self.jobs.enqueue_preparation_member(
                    claim.id,
                    &claim.claim_token,
                    Self::derivative_job_spec(claim.user_id, *job_id, input.as_ref().clone()),
                )?;
                admitted += 1;
            }
            self.jobs.heartbeat_job(
                claim.id,
                &claim.claim_token,
                "admitting_tracks",
                admitted.min(frozen.members.len() as u64),
                30,
            )?;
        }
        Ok(serde_json::json!({"batch_id": frozen.batch_id, "admitted": admitted}))
    }
}

fn validate_album_sources(sources: &[RepresentationId]) -> Result<()> {
    if sources.is_empty() || sources.len() > 64 {
        return Err(Error::Invalid(
            "album preparation requires 1–64 selected disc sources".into(),
        ));
    }
    let unique = sources
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    if unique.len() != sources.len() {
        return Err(Error::Invalid(
            "album preparation sources must be unique".into(),
        ));
    }
    Ok(())
}

fn album_plan_for_item(
    item: &collector_core::Item,
    rows: &[MusicTrackAvailability],
) -> MusicAlbumPreparationPlan {
    let mut plan = album_plan_from_rows(item.id, rows);
    let mut carriers = std::collections::BTreeMap::new();
    for capture in &item.captures {
        if let Some(carrier) = capture.association.carrier_id {
            carriers
                .entry(carrier)
                .or_insert_with(Vec::new)
                .push(capture.id);
        }
    }
    for captures in carriers.values() {
        let indices = plan
            .discs
            .iter()
            .enumerate()
            .filter_map(|(index, disc)| captures.contains(&disc.capture_id).then_some(index))
            .collect::<Vec<_>>();
        if indices.len() < 2 {
            continue;
        }
        let first = indices[0];
        for index in indices.into_iter().skip(1).rev() {
            let alternate = plan.discs.remove(index);
            plan.discs[first].sources.extend(alternate.sources);
        }
        let disc = &mut plan.discs[first];
        disc.ambiguous = true;
        disc.message = Some(
            "Multiple captures belong to the same physical carrier. Choose one source.".into(),
        );
        for source in &mut disc.sources {
            source.default_selected = false;
        }
    }
    plan.total_tracks = plan
        .discs
        .iter()
        .flat_map(|disc| &disc.sources)
        .filter(|source| source.default_selected)
        .map(|source| source.track_count)
        .fold(0, u16::saturating_add);
    plan
}

fn album_plan_from_rows(
    item_id: collector_core::ItemId,
    rows: &[MusicTrackAvailability],
) -> MusicAlbumPreparationPlan {
    let mut grouped = std::collections::BTreeMap::<
        CaptureId,
        std::collections::BTreeMap<RepresentationId, Vec<&MusicTrackAvailability>>,
    >::new();
    for row in rows
        .iter()
        .filter(|row| row.profile == ConversionProfile::Flac)
    {
        grouped
            .entry(row.capture_id)
            .or_default()
            .entry(row.source_representation_id)
            .or_default()
            .push(row);
    }
    let mut total_tracks = 0usize;
    let mut blockers = Vec::new();
    let mut discs = Vec::new();
    for (capture_id, sources) in grouped {
        let ambiguous = sources.len() > 1;
        if ambiguous {
            blockers.push(format!(
                "Captured disc {capture_id} has alternative preservation sources; choose one before preparing it."
            ));
        }
        let mut source_plans = Vec::new();
        for (source_representation_id, tracks) in sources {
            if !ambiguous {
                total_tracks += tracks.len();
            }
            let count = |predicate: fn(&MusicTrackAvailabilityState) -> bool| {
                tracks
                    .iter()
                    .filter(|row| predicate(&row.state))
                    .count()
                    .try_into()
                    .unwrap_or(u16::MAX)
            };
            source_plans.push(MusicAlbumSourcePlan {
                capture_id,
                source_representation_id,
                track_count: tracks.len().try_into().unwrap_or(u16::MAX),
                default_selected: !ambiguous,
                already_available: count(|state| {
                    matches!(state, MusicTrackAvailabilityState::Available { .. })
                }),
                preparing: count(|state| matches!(state, MusicTrackAvailabilityState::Preparing)),
                eligible: tracks
                    .iter()
                    .filter(|row| row.can_prepare)
                    .count()
                    .try_into()
                    .unwrap_or(u16::MAX),
            });
        }
        discs.push(MusicAlbumDiscPlan {
            capture_id,
            sources: source_plans,
            ambiguous,
            message: ambiguous.then(|| {
                "Select the preservation source that represents this disc; alternatives are never encoded as duplicate discs.".into()
            }),
        });
    }
    if discs.is_empty() {
        blockers.push(
            "No supported music-disc source is associated with this album. Preserve and analyze a complete supported package first."
                .into(),
        );
    }
    if total_tracks > MAX_ALBUM_PREPARATION_MEMBERS {
        blockers.push(format!(
            "The default scope has {total_tracks} tracks; one batch admits at most {MAX_ALBUM_PREPARATION_MEMBERS}."
        ));
    }
    let can_submit = discs.iter().any(|disc| {
        disc.sources.iter().any(|source| {
            source.already_available > 0 || source.preparing > 0 || source.eligible > 0
        })
    });
    if !can_submit && !discs.is_empty() {
        let reasons = rows
            .iter()
            .filter_map(|row| match &row.state {
                MusicTrackAvailabilityState::Unavailable { reason } if !row.can_prepare => {
                    Some(reason.clone())
                }
                _ => None,
            })
            .collect::<std::collections::BTreeSet<_>>();
        blockers.extend(reasons.into_iter().take(3));
    }
    MusicAlbumPreparationPlan {
        item_id,
        profile: ConversionProfile::Flac,
        discs,
        total_tracks: total_tracks.try_into().unwrap_or(u16::MAX),
        blockers,
        can_submit,
        maximum_members: MAX_ALBUM_PREPARATION_MEMBERS as u16,
    }
}

fn preparation_member_id(
    batch: JobId,
    capture: CaptureId,
    source: RepresentationId,
    track: u16,
) -> JobId {
    use sha2::{Digest as _, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(b"collector music preparation member v1");
    hasher.update(batch.0.as_bytes());
    hasher.update(capture.0.as_bytes());
    hasher.update(source.0.as_bytes());
    hasher.update(track.to_be_bytes());
    let mut bytes: [u8; 16] = hasher.finalize()[..16].try_into().expect("SHA-256 prefix");
    bytes[6] = (bytes[6] & 0x0f) | 0x50;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    JobId(uuid::Uuid::from_bytes(bytes))
}

/// Evaluated with transactionally loaded ownership facts by the storage adapter.
pub fn may_cancel_preparation_child(direct_request: bool, other_batch_requests: bool) -> bool {
    !direct_request && !other_batch_requests
}

#[cfg(test)]
mod tests {
    use super::*;
    use collector_core::PcmFrame;
    #[test]
    fn album_plan_never_guesses_between_sources_for_one_captured_disc() {
        let item = collector_core::ItemId::new();
        let capture = CaptureId::new();
        let first = RepresentationId::new();
        let second = RepresentationId::new();
        let row = |source_representation_id| MusicTrackAvailability {
            capture_id: capture,
            source_representation_id,
            track_position: 1,
            start_frame: PcmFrame(0),
            frame_count: PcmFrame(588),
            title: "Track 01".into(),
            profile: ConversionProfile::Flac,
            state: MusicTrackAvailabilityState::Unavailable {
                reason: "not prepared".into(),
            },
            can_prepare: true,
        };
        let plan = album_plan_from_rows(item, &[row(first), row(second)]);
        assert_eq!(plan.discs.len(), 1);
        assert!(plan.discs[0].ambiguous);
        assert_eq!(plan.discs[0].sources.len(), 2);
        assert!(
            plan.discs[0]
                .sources
                .iter()
                .all(|source| !source.default_selected)
        );
        assert_eq!(plan.total_tracks, 0);
        assert!(plan.blockers[0].contains("choose one"));

        let source = RepresentationId::new();
        let oversized = (1..=257)
            .map(|track_position| MusicTrackAvailability {
                capture_id: CaptureId::new(),
                source_representation_id: source,
                track_position,
                start_frame: PcmFrame(u64::from(track_position - 1) * 588),
                frame_count: PcmFrame(588),
                title: format!("Track {track_position:02}"),
                profile: ConversionProfile::Flac,
                state: MusicTrackAvailabilityState::Unavailable {
                    reason: "not prepared".into(),
                },
                can_prepare: true,
            })
            .collect::<Vec<_>>();
        let oversized = album_plan_from_rows(item, &oversized);
        assert_eq!(oversized.maximum_members, 256);
        assert!(
            oversized
                .blockers
                .iter()
                .any(|blocker| blocker.contains("at most 256"))
        );
    }
}
