//! Security policy evaluated over snapshots held stable by the persistence adapter.
//! Adapters load these facts and apply returned effects in the same transaction.
use collector_core::{Error, Result, Revision, Role};

/// Mail intake must not persist data for a disabled or missing account.
/// The store evaluates this policy against facts held stable through commit.
pub fn mail_ingestion(account_enabled: bool) -> Result<()> {
    if account_enabled {
        Ok(())
    } else {
        Err(Error::Forbidden)
    }
}

pub struct AccountAuthority {
    pub revision: Revision,
    pub enabled: bool,
    pub administrator: bool,
    pub is_default: bool,
    pub sole_owner_of_any_collection: bool,
    pub other_enabled_administrators: u64,
}

pub fn account_update(
    snapshot: &AccountAuthority,
    expected: Revision,
    enabled: bool,
    administrator: bool,
) -> Result<CredentialEffects> {
    if snapshot.revision != expected {
        return Err(Error::Conflict);
    }
    if snapshot.enabled && !enabled {
        if snapshot.is_default {
            return Err(Error::DefaultReassignmentRequired);
        }
        if snapshot.sole_owner_of_any_collection {
            return Err(Error::LastOwner);
        }
    }
    if snapshot.enabled
        && snapshot.administrator
        && (!enabled || !administrator)
        && snapshot.other_enabled_administrators == 0
    {
        return Err(Error::LastAdministrator);
    }
    Ok(if enabled {
        CredentialEffects::default()
    } else {
        credential_effects(CredentialChange::Disable)
    })
}

pub struct GrantAuthority {
    pub account_enabled: bool,
    pub current_role: Option<Role>,
    pub current_revision: Revision,
    pub other_enabled_owners: u64,
}

/// Returns whether an explicit local recovery needs to publish a first Owner.
/// Existing owners, even disabled ones, must use ordinary access/account recovery.
pub fn recover_initial_owner(
    account_enabled: bool,
    current_role: Option<Role>,
    has_owner: bool,
) -> Result<bool> {
    if !account_enabled {
        return Err(Error::Invalid(
            "disabled accounts cannot receive collection grants".into(),
        ));
    }
    if current_role == Some(Role::Owner) {
        return Ok(false);
    }
    if has_owner {
        return Err(Error::Invalid("collection already has an Owner; use its normal access controls or recover that account".into()));
    }
    Ok(true)
}

pub fn grant_update(
    snapshot: &GrantAuthority,
    expected: Revision,
    role: Option<Role>,
) -> Result<()> {
    if role.is_some() && !snapshot.account_enabled {
        return Err(Error::Invalid(
            "disabled accounts cannot receive collection grants".into(),
        ));
    }
    if snapshot.current_revision != expected {
        return Err(Error::Conflict);
    }
    if snapshot.current_role == Some(Role::Owner)
        && role != Some(Role::Owner)
        && snapshot.other_enabled_owners == 0
    {
        return Err(Error::LastOwner);
    }
    Ok(())
}

pub fn default_account(enabled: bool) -> Result<()> {
    if enabled {
        Ok(())
    } else {
        Err(Error::Invalid(
            "the durable default account must be enabled".into(),
        ))
    }
}

#[derive(Clone, Copy)]
pub enum CredentialChange {
    Disable,
    ReplaceRecoveryCodes,
    RecoverPassword,
    LocalRecovery,
    RevokeOtherAccess,
}
#[derive(Default)]
pub struct CredentialEffects {
    pub browser_sessions: bool,
    pub native_credentials: bool,
    pub enrollments: bool,
    pub recovery_batches: bool,
    pub passkeys: bool,
}
pub fn credential_effects(change: CredentialChange) -> CredentialEffects {
    use CredentialChange::*;
    match change {
        Disable => CredentialEffects {
            browser_sessions: true,
            native_credentials: true,
            enrollments: true,
            recovery_batches: true,
            passkeys: false,
        },
        ReplaceRecoveryCodes => CredentialEffects {
            recovery_batches: true,
            ..Default::default()
        },
        RecoverPassword => CredentialEffects {
            browser_sessions: true,
            native_credentials: true,
            enrollments: true,
            recovery_batches: true,
            ..Default::default()
        },
        RevokeOtherAccess => CredentialEffects {
            native_credentials: true,
            enrollments: true,
            ..Default::default()
        },
        LocalRecovery => CredentialEffects {
            browser_sessions: true,
            native_credentials: true,
            enrollments: true,
            recovery_batches: true,
            passkeys: true,
        },
    }
}

#[derive(Clone, Copy)]
pub enum EnrollmentAction {
    Approve,
    Cancel,
    Complete,
}

/// Caller identity is matched before loading the snapshot. Persistence applies
/// the selected transition with CAS while the snapshot remains locked.
pub fn enrollment_transition(
    phase: crate::EnrollmentPhase,
    action: EnrollmentAction,
) -> Result<crate::EnrollmentPhase> {
    use crate::EnrollmentPhase::*;
    match (phase, action) {
        (Expired, _) => Err(Error::EnrollmentExpired),
        (Cancelled, _) => Err(Error::EnrollmentCancelled),
        (PendingApproval, EnrollmentAction::Approve) => Ok(Approved),
        (PendingApproval | Approved, EnrollmentAction::Cancel) => Ok(Cancelled),
        (Approved, EnrollmentAction::Complete) => Ok(Issued),
        _ => Err(Error::Unauthenticated),
    }
}
