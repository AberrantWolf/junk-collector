//! Private operational defaults. No collection evidence is stored here.
use crate::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum OpusImportScope {
    #[default]
    Off,
    Cd,
    CdAndDigital,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct OpusImportDefault {
    pub scope: OpusImportScope,
    pub bitrate_kbps: u16,
}

impl Default for OpusImportDefault {
    fn default() -> Self {
        Self {
            scope: OpusImportScope::Off,
            bitrate_kbps: 128,
        }
    }
}

impl OpusImportDefault {
    pub(crate) fn for_package(self, package: &IngestPlanPackage) -> ImportAudioPreparation {
        if package.music_actions_available
            && match self.scope {
                OpusImportScope::Off => false,
                OpusImportScope::Cd => package.anchors.iter().any(|anchor| anchor.selected),
                OpusImportScope::CdAndDigital => true,
            }
        {
            ImportAudioPreparation::Opus {
                bitrate_kbps: self.bitrate_kbps,
            }
        } else {
            ImportAudioPreparation::None
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(tag = "codec", rename_all = "snake_case", deny_unknown_fields)]
pub enum ImportAudioPreparation {
    None,
    Flac,
    Opus { bitrate_kbps: u16 },
}

impl ImportAudioPreparation {
    pub(crate) fn profile(self) -> Result<Option<ConversionProfile>> {
        let profile = match self {
            Self::None => return Ok(None),
            Self::Flac => ConversionProfile::Flac,
            Self::Opus { bitrate_kbps } => ConversionProfile::Opus { bitrate_kbps },
        };
        profile.validate()?;
        Ok(Some(profile))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CdLibrarySettings {
    pub revision: Revision,
    pub check_accuraterip_on_ingest: bool,
    pub maximum_offset: i32,
    #[serde(default)]
    pub opus_import: OpusImportDefault,
}
impl Default for CdLibrarySettings {
    fn default() -> Self {
        Self {
            revision: Revision(0),
            check_accuraterip_on_ingest: true,
            maximum_offset: 2939,
            opus_import: OpusImportDefault::default(),
        }
    }
}
impl CdLibrarySettings {
    pub fn validate(&self) -> Result<()> {
        ConversionProfile::Opus {
            bitrate_kbps: self.opus_import.bitrate_kbps,
        }
        .validate()?;
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opus_import: Option<OpusImportDefault>,
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
            opus_import: command.opus_import.unwrap_or_default(),
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
