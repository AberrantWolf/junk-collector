use super::*;
use collector_app::{
    CdLibrarySettings, JobSettings, UpdateCdLibrarySettings, UpdateWorkerSettings, WorkerSettings,
};

pub(crate) fn read_workers(db: &Connection, defaults: JobSettings) -> Result<WorkerSettings> {
    let raw: Option<String> = db
        .query_row(
            "SELECT value FROM private.settings WHERE key='worker_settings'",
            [],
            |r| r.get(0),
        )
        .optional()
        .map_err(storage)?;
    let settings = raw
        .map(|s| serde_json::from_str::<WorkerSettings>(&s).map_err(storage))
        .transpose()?
        .unwrap_or(WorkerSettings {
            revision: Revision(0),
            jobs: defaults,
        });
    settings.jobs.validate()?;
    Ok(settings)
}
pub(crate) fn read_cd_settings(
    db: &Connection,
    collection: CollectionId,
) -> Result<CdLibrarySettings> {
    let raw: Option<String> = db
        .query_row(
            "SELECT value FROM private.settings WHERE key=?",
            [format!("cd_settings:{collection}")],
            |r| r.get(0),
        )
        .optional()
        .map_err(storage)?;
    let settings = raw
        .map(|s| serde_json::from_str::<CdLibrarySettings>(&s).map_err(storage))
        .transpose()?
        .unwrap_or_default();
    settings.validate()?;
    Ok(settings)
}
impl Store {
    pub(crate) fn worker_settings_record(&self) -> Result<WorkerSettings> {
        let defaults = lock(&self.job_dispatch)?.settings;
        read_workers(&*lock(&self.db)?, defaults)
    }
    pub(crate) fn save_worker_settings_record(
        &self,
        command: &UpdateWorkerSettings,
    ) -> Result<WorkerSettings> {
        command.jobs.validate()?;
        let defaults = lock(&self.job_dispatch)?.settings;
        let mut db = lock(&self.db)?;
        let tx = db.transaction().map_err(storage)?;
        let current = read_workers(&tx, defaults)?;
        if current.revision != command.expected_revision {
            return Err(Error::Conflict);
        }
        let result = WorkerSettings {
            revision: Revision(current.revision.0.checked_add(1).ok_or(Error::Conflict)?),
            jobs: command.jobs,
        };
        tx.execute("INSERT INTO private.settings(key,value) VALUES('worker_settings',?) ON CONFLICT(key) DO UPDATE SET value=excluded.value",[serde_json::to_string(&result).map_err(storage)?]).map_err(storage)?;
        self.record_command(&tx, &result)?;
        tx.commit().map_err(storage)?;
        Ok(result)
    }
    pub(crate) fn save_cd_settings_record(
        &self,
        collection: CollectionId,
        command: &UpdateCdLibrarySettings,
    ) -> Result<CdLibrarySettings> {
        let mut db = lock(&self.db)?;
        let tx = db.transaction().map_err(storage)?;
        let current = read_cd_settings(&tx, collection)?;
        if current.revision != command.expected_revision {
            return Err(Error::Conflict);
        }
        let result = CdLibrarySettings {
            revision: Revision(current.revision.0.checked_add(1).ok_or(Error::Conflict)?),
            check_accuraterip_on_ingest: command.check_accuraterip_on_ingest,
            maximum_offset: command.maximum_offset,
        };
        result.validate()?;
        tx.execute("INSERT INTO private.settings(key,value) VALUES(?,?) ON CONFLICT(key) DO UPDATE SET value=excluded.value",params![format!("cd_settings:{collection}"),serde_json::to_string(&result).map_err(storage)?]).map_err(storage)?;
        self.record_command(&tx, &result)?;
        tx.commit().map_err(storage)?;
        Ok(result)
    }
}
