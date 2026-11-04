//! Private operational defaults. No collection evidence is stored here.
use crate::*;

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CdLibrarySettings {
    pub revision: Revision,
    pub check_accuraterip_on_ingest: bool,
    pub maximum_offset: i32,
}
impl Default for CdLibrarySettings {
    fn default() -> Self {
        Self {
            revision: Revision(0),
            check_accuraterip_on_ingest: true,
            maximum_offset: 2939,
        }
    }
}
impl CdLibrarySettings {
    pub fn validate(&self) -> Result<()> {
        if !(0..=2939).contains(&self.maximum_offset) {
            return Err(Error::Invalid(
                "Maximum AccurateRip offset must be between 0 and 2939 stereo sample frames".into(),
            ));
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct UpdateCdLibrarySettings {
    pub expected_revision: Revision,
    pub check_accuraterip_on_ingest: bool,
    pub maximum_offset: i32,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct WorkerSettings {
    pub revision: Revision,
    pub jobs: JobSettings,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct UpdateWorkerSettings {
    pub expected_revision: Revision,
    pub jobs: JobSettings,
}
impl CollectionService {
    pub fn cd_library_settings(
        &self,
        actor: &Principal,
        collection: CollectionId,
    ) -> Result<CdLibrarySettings> {
        let eligibility = self.content_write_eligibility(actor, collection)?;
        if eligibility.owner.0 != "org.junk.music" {
            return Err(Error::Invalid(
                "CD settings require a music collection".into(),
            ));
        }
        self.care.cd_settings(collection)
    }
    pub fn update_cd_library_settings(
        &self,
        actor: &Principal,
        collection: CollectionId,
        command: UpdateCdLibrarySettings,
        context: &CommandContext,
    ) -> Result<CdLibrarySettings> {
        self.authorize(actor, collection, Action::ManageAccess)?;
        self.cd_library_settings(actor, collection)?;
        CdLibrarySettings {
            revision: command.expected_revision,
            check_accuraterip_on_ingest: command.check_accuraterip_on_ingest,
            maximum_offset: command.maximum_offset,
        }
        .validate()?;
        self.execute_command(
            actor,
            context,
            "library.cd.settings",
            &(collection, &command),
            |effects| effects.save_cd_settings(collection, &command),
        )
    }
    pub fn worker_settings(&self, actor: &Principal) -> Result<WorkerSettings> {
        self.require_administrator(actor)?;
        self.administration.worker_settings()
    }
    pub fn update_worker_settings(
        &self,
        actor: &Principal,
        command: UpdateWorkerSettings,
        context: &CommandContext,
    ) -> Result<WorkerSettings> {
        self.require_administrator(actor)?;
        command.jobs.validate()?;
        self.execute_command(actor, context, "admin.workers", &command, |effects| {
            effects.save_worker_settings(&command)
        })
    }
}

/// Private operator policy; saved preferences override the startup default.
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AudioConnectionSettings {
    pub revision: Revision,
    pub remote_media_allowed: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct UpdateAudioConnectionSettings {
    pub expected_revision: Revision,
    pub remote_media_allowed: bool,
}
impl CollectionService {
    pub fn audio_connection_settings(
        &self,
        actor: &Principal,
        default_allowed: bool,
    ) -> Result<AudioConnectionSettings> {
        self.require_administrator(actor)?;
        self.audio_connection_policy(default_allowed)
    }
    pub fn audio_connection_policy(
        &self,
        default_allowed: bool,
    ) -> Result<AudioConnectionSettings> {
        self.administration
            .audio_connection_settings(default_allowed)
    }
    pub fn update_audio_connection_settings(
        &self,
        actor: &Principal,
        default_allowed: bool,
        command: UpdateAudioConnectionSettings,
        context: &CommandContext,
    ) -> Result<AudioConnectionSettings> {
        self.require_administrator(actor)?;
        self.execute_command(
            actor,
            context,
            "admin.audio_connection",
            &command,
            |effects| effects.save_audio_connection_settings(default_allowed, &command),
        )
    }
}
