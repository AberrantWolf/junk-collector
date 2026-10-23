//! Optional operator reminder policy. No backup discovery or byte validation occurs here.
use chrono::{DateTime, Utc};
use collector_core::{Error, Result};
use serde::{Deserialize, Serialize};

pub trait BackupHistoryPort: Send + Sync {
    fn backup_history(&self) -> Result<Option<BackupHistory>>;
}
impl crate::CollectionService {
    pub fn with_backup_history(mut self, port: std::sync::Arc<dyn BackupHistoryPort>) -> Self {
        self.backup_history = Some(port);
        self
    }
    pub fn backup_reminder(
        &self,
        actor: &collector_core::Principal,
        settings: &BackupReminderSettings,
    ) -> Result<BackupReminder> {
        self.require_administrator(actor)?;
        settings.validate()?;
        let history = if settings.externally_managed {
            None
        } else {
            self.backup_history
                .as_ref()
                .and_then(|port| port.backup_history().ok().flatten())
        };
        Ok(backup_reminder(
            settings,
            history.as_ref(),
            self.authentication.now(),
        ))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BackupReminderSettings {
    pub after_days: u16,
    #[serde(default)]
    pub externally_managed: bool,
    pub external_completed_at: Option<DateTime<Utc>>,
    pub external_failed_at: Option<DateTime<Utc>>,
}
impl BackupReminderSettings {
    pub fn validate(&self) -> Result<()> {
        if !(1..=3650).contains(&self.after_days)
            || (!self.externally_managed
                && (self.external_completed_at.is_some() || self.external_failed_at.is_some()))
        {
            return Err(Error::Invalid("backup reminder needs 1–3650 days; external dates require externally_managed = true".into()));
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackupAttemptOutcome {
    Started,
    Failed,
    Completed,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BackupHistory {
    pub version: u8,
    pub last_completed_at: Option<DateTime<Utc>>,
    pub attempted_at: DateTime<Utc>,
    pub outcome: BackupAttemptOutcome,
}
impl BackupHistory {
    pub fn is_consistent_at(&self, now: DateTime<Utc>) -> bool {
        self.version == 1
            && self.attempted_at <= now
            && self
                .last_completed_at
                .is_none_or(|at| at <= self.attempted_at)
            && (self.outcome != BackupAttemptOutcome::Completed
                || self.last_completed_at == Some(self.attempted_at))
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum BackupReminderStatus {
    Unknown,
    Recent,
    Due,
    Failed,
    Unconfirmed,
}
#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
pub struct BackupReminder {
    pub status: BackupReminderStatus,
    pub after_days: u16,
    pub externally_managed: bool,
    pub last_completed_at: Option<DateTime<Utc>>,
    pub checked_at: DateTime<Utc>,
}
/// `history` is absent or unreadable when no local completion can be trusted.
/// External dates remain operator reports, never observed validation claims.
pub fn backup_reminder(
    settings: &BackupReminderSettings,
    history: Option<&BackupHistory>,
    now: DateTime<Utc>,
) -> BackupReminder {
    use BackupReminderStatus::*;
    let (completed, override_status) = if settings.externally_managed {
        let completed = settings.external_completed_at;
        let failed = settings.external_failed_at;
        let status = if completed.is_some_and(|at| at > now) || failed.is_some_and(|at| at > now) {
            Some(Unknown)
        } else if failed.is_some_and(|at| completed.is_none_or(|done| at >= done)) {
            Some(Failed)
        } else {
            None
        };
        (completed, status)
    } else if let Some(history) = history {
        if !history.is_consistent_at(now) {
            (None, Some(Unknown))
        } else {
            (
                history.last_completed_at,
                match history.outcome {
                    BackupAttemptOutcome::Started => Some(Unconfirmed),
                    BackupAttemptOutcome::Failed => Some(Failed),
                    BackupAttemptOutcome::Completed => None,
                },
            )
        }
    } else {
        (None, Some(Unknown))
    };
    let status = override_status.unwrap_or_else(|| match completed {
        Some(at) if (now - at).num_seconds() >= i64::from(settings.after_days) * 86400 => Due,
        Some(_) => Recent,
        None => Unknown,
    });
    BackupReminder {
        status,
        after_days: settings.after_days,
        externally_managed: settings.externally_managed,
        last_completed_at: completed.filter(|at| *at <= now),
        checked_at: now,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn settings() -> BackupReminderSettings {
        BackupReminderSettings {
            after_days: 30,
            externally_managed: false,
            external_completed_at: None,
            external_failed_at: None,
        }
    }
    #[test]
    fn age_failure_interruption_and_invalid_history_are_distinct() {
        use BackupReminderStatus::*;
        let now = DateTime::from_timestamp(1800000000, 0).unwrap();
        let settings = settings();
        assert_eq!(backup_reminder(&settings, None, now).status, Unknown);
        let mut history = BackupHistory {
            version: 1,
            last_completed_at: Some(now),
            attempted_at: now,
            outcome: BackupAttemptOutcome::Completed,
        };
        assert_eq!(
            backup_reminder(&settings, Some(&history), now + chrono::Duration::days(29)).status,
            Recent
        );
        assert_eq!(
            backup_reminder(&settings, Some(&history), now + chrono::Duration::days(30)).status,
            Due
        );
        history.attempted_at = now + chrono::Duration::days(31);
        history.outcome = BackupAttemptOutcome::Started;
        let later = history.attempted_at + chrono::Duration::days(1);
        assert_eq!(
            backup_reminder(&settings, Some(&history), later).status,
            Unconfirmed
        );
        history.outcome = BackupAttemptOutcome::Failed;
        let failed = backup_reminder(&settings, Some(&history), later);
        assert_eq!(failed.status, Failed);
        assert_eq!(failed.last_completed_at, Some(now));
        assert_eq!(
            backup_reminder(&settings, Some(&history), now).status,
            Unknown
        );
        history.outcome = BackupAttemptOutcome::Completed; // Cannot call a prior completion the result of this attempt.
        assert_eq!(
            backup_reminder(&settings, Some(&history), later).status,
            Unknown
        );
        history.version = 2;
        assert_eq!(
            backup_reminder(&settings, Some(&history), later).status,
            Unknown
        );
    }
    #[test]
    fn external_reports_never_become_observed_local_validation() {
        let now = DateTime::from_timestamp(1800000000, 0).unwrap();
        let mut settings = settings();
        settings.external_completed_at = Some(now);
        assert!(settings.validate().is_err());
        settings.externally_managed = true;
        settings.validate().unwrap();
        let recent = backup_reminder(&settings, None, now);
        assert!(recent.externally_managed);
        assert_eq!(recent.status, BackupReminderStatus::Recent);
        settings.external_failed_at = Some(now + chrono::Duration::days(1));
        assert_eq!(
            backup_reminder(&settings, None, now + chrono::Duration::days(2)).status,
            BackupReminderStatus::Failed
        );
        assert_eq!(
            backup_reminder(&settings, None, now).status,
            BackupReminderStatus::Unknown
        );
        settings.after_days = 0;
        assert!(settings.validate().is_err());
    }
}
