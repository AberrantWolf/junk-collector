use super::*;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier, password_hash::SaltString};
use rand::{RngCore, rngs::OsRng};
use rusqlite::TransactionBehavior;
use std::path::Path;
impl Store {
    pub fn start_enrollment(
        &self,
        user: UserId,
        label: &str,
        verification_url: &str,
    ) -> Result<EnrollmentStart> {
        let label = label.trim();
        if label.is_empty() || label.len() > 128 || verification_url.len() > 2048 {
            return Err(Error::Invalid(
                "device label and verification URL are required and bounded".into(),
            ));
        }
        let mut secret = [0u8; 32];
        OsRng.fill_bytes(&mut secret);
        let completion_secret = hex::encode(secret);
        let id = EnrollmentId::new();
        let now = self.clock.now();
        let expires_at = now + chrono::Duration::minutes(10);
        let mut db = lock(&self.db)?;
        let tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage)?;
        let enabled: bool = tx
            .query_row(
                "SELECT enabled FROM private.users WHERE id=?",
                [user.to_string()],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage)?
            .ok_or(Error::Unauthenticated)?;
        if !enabled {
            return Err(Error::Unauthenticated);
        }
        expire_enrollments(&tx, now.timestamp())?;
        let user_code = loop {
            const ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
            let mut bytes = [0u8; 8];
            OsRng.fill_bytes(&mut bytes);
            let code: String = bytes
                .iter()
                .map(|byte| ALPHABET[*byte as usize % ALPHABET.len()] as char)
                .collect();
            let exists: bool = tx
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM private.enrollments WHERE user_code=? AND phase IN ('pending_approval','approved'))",
                    [&code],
                    |row| row.get(0),
                )
                .map_err(storage)?;
            if !exists {
                break code;
            }
        };
        tx.execute(
            "INSERT INTO private.enrollments(id,user_id,completion_secret_hash,user_code,requested_label,verification_url,phase,created_at,expires_at,approved_at,issued_at,cancelled_at) VALUES(?,?,?,?,?,?,'pending_approval',?,?,NULL,NULL,NULL)",
            params![
                id.to_string(),
                user.to_string(),
                Digest::sha256(completion_secret.as_bytes()).hex,
                user_code,
                label,
                verification_url,
                now.timestamp(),
                expires_at.timestamp()
            ],
        )
        .map_err(storage)?;
        insert_security_audit(
            &tx,
            Some(user),
            "enrollment_started",
            serde_json::json!({"enrollment_id": id}),
            now.timestamp(),
        )?;
        tx.commit().map_err(storage)?;
        Ok(EnrollmentStart {
            id,
            completion_secret,
            user_code,
            verification_url: verification_url.into(),
            expires_at,
        })
    }

    pub fn approve_enrollment(&self, user: UserId, user_code: &str) -> Result<EnrollmentView> {
        let code = user_code.trim().to_ascii_uppercase();
        let now = self.clock.now().timestamp();
        let mut db = lock(&self.db)?;
        let tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage)?;
        expire_enrollments(&tx, now)?;
        let enrollment: Option<(String, String, String, i64)> = tx
            .query_row(
                "SELECT id,requested_label,phase,expires_at FROM private.enrollments WHERE user_id=? AND user_code=?",
                params![user.to_string(), code],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .optional()
            .map_err(storage)?;
        let (id, requested_label, phase, expires_at) = enrollment.ok_or(Error::Unauthenticated)?;
        collector_app::security::enrollment_transition(
            parse_enrollment_phase(&phase)?,
            collector_app::security::EnrollmentAction::Approve,
        )?;
        if tx
            .execute(
                "UPDATE private.enrollments SET phase='approved',approved_at=? WHERE id=? AND user_id=? AND phase='pending_approval'",
                params![now, id, user.to_string()],
            )
            .map_err(storage)?
            != 1
        {
            return Err(Error::Unauthenticated);
        }
        let id: EnrollmentId = id.parse().map_err(storage)?;
        insert_security_audit(
            &tx,
            Some(user),
            "enrollment_approved",
            serde_json::json!({"enrollment_id": id}),
            now,
        )?;
        tx.commit().map_err(storage)?;
        Ok(EnrollmentView {
            id,
            requested_label,
            phase: collector_app::EnrollmentPhase::Approved,
            expires_at: chrono::DateTime::from_timestamp(expires_at, 0)
                .ok_or_else(|| Error::Storage("invalid enrollment expiry".into()))?,
        })
    }

    pub fn cancel_enrollment(&self, user: UserId, enrollment: EnrollmentId) -> Result<()> {
        let now = self.clock.now().timestamp();
        let mut db = lock(&self.db)?;
        let tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage)?;
        expire_enrollments(&tx, now)?;
        let phase: Option<String> = tx
            .query_row(
                "SELECT phase FROM private.enrollments WHERE id=? AND user_id=?",
                params![enrollment.to_string(), user.to_string()],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage)?;
        collector_app::security::enrollment_transition(
            parse_enrollment_phase(&phase.ok_or(Error::Unauthenticated)?)?,
            collector_app::security::EnrollmentAction::Cancel,
        )?;
        tx.execute(
            "UPDATE private.enrollments SET phase='cancelled',cancelled_at=? WHERE id=? AND user_id=? AND phase IN ('pending_approval','approved')",
            params![now, enrollment.to_string(), user.to_string()],
        )
        .map_err(storage)?;
        insert_security_audit(
            &tx,
            Some(user),
            "enrollment_cancelled",
            serde_json::json!({"enrollment_id": enrollment}),
            now,
        )?;
        tx.commit().map_err(storage)
    }

    pub fn complete_enrollment(
        &self,
        enrollment: EnrollmentId,
        completion_secret: &str,
    ) -> Result<NativeCredentialIssue> {
        if completion_secret.len() != 64
            || !completion_secret
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(Error::Unauthenticated);
        }
        let mut credential_bytes = [0u8; 32];
        OsRng.fill_bytes(&mut credential_bytes);
        let credential = hex::encode(credential_bytes);
        let credential_id = NativeCredentialId::new();
        let now = self.clock.now();
        let expires_at = now + chrono::Duration::days(30);
        let mut db = lock(&self.db)?;
        let tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage)?;
        expire_enrollments(&tx, now.timestamp())?;
        let record: Option<(String, String, String, String)> = tx
            .query_row(
                "SELECT user_id,completion_secret_hash,requested_label,phase FROM private.enrollments WHERE id=?",
                [enrollment.to_string()],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .optional()
            .map_err(storage)?;
        let (user, secret_hash, label, phase) = record.ok_or(Error::Unauthenticated)?;
        collector_app::security::enrollment_transition(
            parse_enrollment_phase(&phase)?,
            collector_app::security::EnrollmentAction::Complete,
        )?;
        if Digest::sha256(completion_secret.to_ascii_lowercase().as_bytes()).hex != secret_hash {
            return Err(Error::Unauthenticated);
        }
        if tx
            .execute(
                "UPDATE private.enrollments SET phase='issued',issued_at=? WHERE id=? AND phase='approved'",
                params![now.timestamp(), enrollment.to_string()],
            )
            .map_err(storage)?
            != 1
        {
            return Err(Error::Unauthenticated);
        }
        tx.execute(
            "INSERT INTO private.native_credentials(id,user_id,label,revision,credential_hash,created_at,expires_at,revoked_at) VALUES(?,?,?,1,?,?,?,NULL)",
            params![
                credential_id.to_string(),
                user,
                label,
                Digest::sha256(credential.as_bytes()).hex,
                now.timestamp(),
                expires_at.timestamp()
            ],
        )
        .map_err(storage)?;
        let user_id: UserId = user.parse().map_err(storage)?;
        insert_security_audit(
            &tx,
            Some(user_id),
            "native_credential_issued",
            serde_json::json!({"credential_id": credential_id, "enrollment_id": enrollment}),
            now.timestamp(),
        )?;
        tx.commit().map_err(storage)?;
        Ok(NativeCredentialIssue {
            device_id: credential_id,
            credential,
            expires_at,
        })
    }

    pub fn authenticate_native(&self, credential: &str) -> Result<AuthenticatedPrincipal> {
        if credential.len() != 64 {
            return Err(Error::CredentialRevoked);
        }
        let row: Option<(String, bool, String)> = lock(&self.db)?
            .query_row(
                "SELECT u.id,u.administrator,n.id FROM private.native_credentials n JOIN private.users u ON u.id=n.user_id WHERE n.credential_hash=? AND n.revoked_at IS NULL AND n.expires_at>? AND u.enabled=1",
                params![Digest::sha256(credential.as_bytes()).hex, self.clock.now().timestamp()],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()
            .map_err(storage)?;
        let (user, administrator, id) = row.ok_or(Error::CredentialRevoked)?;
        let credential_id: NativeCredentialId = id.parse().map_err(storage)?;
        Ok(AuthenticatedPrincipal {
            principal: Principal {
                user_id: user.parse().map_err(storage)?,
                administrator,
                device_id: Some(DeviceId(credential_id.0)),
            },
            credential_kind: CredentialKind::NativeCredential,
            authenticated_at: chrono::DateTime::UNIX_EPOCH,
        })
    }

    pub fn native_devices(&self, user: UserId) -> Result<Vec<NativeDeviceView>> {
        let db = lock(&self.db)?;
        let mut statement = db
            .prepare("SELECT id,label,revision,created_at,expires_at FROM private.native_credentials WHERE user_id=? AND revoked_at IS NULL ORDER BY created_at,id")
            .map_err(storage)?;
        let rows = statement
            .query_map([user.to_string()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, u64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                ))
            })
            .map_err(storage)?;
        rows.map(|row| {
            let (id, label, revision, created_at, expires_at) = row.map_err(storage)?;
            Ok(NativeDeviceView {
                id: id.parse().map_err(storage)?,
                label,
                revision: Revision(revision),
                created_at: chrono::DateTime::from_timestamp(created_at, 0)
                    .ok_or_else(|| Error::Storage("invalid native credential creation".into()))?,
                expires_at: chrono::DateTime::from_timestamp(expires_at, 0)
                    .ok_or_else(|| Error::Storage("invalid native credential expiry".into()))?,
            })
        })
        .collect()
    }

    pub fn revoke_native_device(
        &self,
        user: UserId,
        credential: NativeCredentialId,
        expected: Revision,
    ) -> Result<()> {
        let now = self.clock.now().timestamp();
        let mut db = lock(&self.db)?;
        let tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage)?;
        if tx
            .execute(
                "UPDATE private.native_credentials SET revoked_at=?,revision=revision+1 WHERE id=? AND user_id=? AND revision=? AND revoked_at IS NULL",
                params![now, credential.to_string(), user.to_string(), expected.0],
            )
            .map_err(storage)?
            != 1
        {
            return Err(Error::Conflict);
        }
        insert_security_audit(
            &tx,
            Some(user),
            "native_credential_revoked",
            serde_json::json!({"credential_id": credential}),
            now,
        )?;
        self.record_command(&tx, &())?;
        tx.commit().map_err(storage)
    }

    pub fn local_recover(
        private_state: impl AsRef<Path>,
        user: UserId,
        password: &str,
    ) -> Result<()> {
        if !(12..=1024).contains(&password.len()) {
            return Err(Error::Invalid("password must be 12–1024 bytes".into()));
        }
        let state = private_state.as_ref();
        let database = state.join("private.sqlite");
        if !database.is_file() {
            return Err(Error::NotFound);
        }
        let private_lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(state.join(".private.lock"))
            .map_err(storage)?;
        private_lock
            .try_lock_exclusive()
            .map_err(|_| Error::Unavailable("private state is already in use".into()))?;
        let mut connection = Connection::open_in_memory().map_err(storage)?;
        connection
            .execute("ATTACH DATABASE ? AS private", [database.to_string_lossy()])
            .map_err(storage)?;
        let version: u32 = connection
            .query_row("PRAGMA private.user_version", [], |row| row.get(0))
            .map_err(storage)?;
        if !matches!(version, 5..=14) {
            return Err(Error::Unavailable(format!(
                "private state schema {version} must be upgraded by this server before local recovery"
            )));
        }
        let salt = SaltString::generate(&mut OsRng);
        let password_hash = Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map_err(storage)?
            .to_string();
        let now = Utc::now().timestamp();
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage)?;
        if tx
            .execute(
                "UPDATE private.users SET password_hash=?,revision=revision+1 WHERE id=?",
                params![password_hash, user.to_string()],
            )
            .map_err(storage)?
            != 1
        {
            return Err(Error::NotFound);
        }
        apply_credential_effects(
            &tx,
            user,
            now,
            collector_app::security::credential_effects(
                collector_app::security::CredentialChange::LocalRecovery,
            ),
        )?;
        insert_security_audit(
            &tx,
            Some(user),
            "local_account_recovered",
            serde_json::json!({"sessions_revoked": true, "passkeys_revoked": true}),
            now,
        )?;
        tx.commit().map_err(storage)?;
        private_lock.unlock().map_err(storage)
    }

    pub fn accounts(&self) -> Result<Vec<AccountView>> {
        let db = lock(&self.db)?;
        let mut statement = db
            .prepare("SELECT id,username,display_name,administrator,enabled,revision FROM private.users ORDER BY username,id")
            .map_err(storage)?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, bool>(3)?,
                    row.get::<_, bool>(4)?,
                    row.get::<_, u64>(5)?,
                ))
            })
            .map_err(storage)?;
        rows.map(|row| {
            let (id, username, display_name, administrator, enabled, revision) =
                row.map_err(storage)?;
            Ok(AccountView {
                id: id.parse().map_err(storage)?,
                username,
                display_name,
                administrator,
                enabled,
                revision: Revision(revision),
            })
        })
        .collect()
    }

    pub fn account(&self, user: UserId) -> Result<AccountView> {
        self.accounts()?
            .into_iter()
            .find(|account| account.id == user)
            .ok_or(Error::NotFound)
    }

    pub fn update_account(
        &self,
        user: UserId,
        expected: Revision,
        display_name: &str,
        enabled: bool,
        administrator: bool,
    ) -> Result<AccountView> {
        let display_name = display_name.trim();
        if display_name.is_empty() || display_name.len() > 256 {
            return Err(Error::Invalid("display name must be 1–256 bytes".into()));
        }
        let now = self.clock.now().timestamp();
        let mut db = lock(&self.db)?;
        let tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage)?;
        let current: Option<(String, bool, bool, u64)> = tx
            .query_row(
                "SELECT username,administrator,enabled,revision FROM private.users WHERE id=?",
                [user.to_string()],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .optional()
            .map_err(storage)?;
        let (username, was_administrator, was_enabled, revision) =
            current.ok_or(Error::NotFound)?;

        let is_default: bool = tx
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM private.settings WHERE key='default_user_id' AND value=?)",
                    [user.to_string()],
                    |row| row.get(0),
                )
                .map_err(storage)?;
        let would_remove_last_owner: bool = tx
            .query_row(
                "SELECT EXISTS(
                       SELECT 1 FROM private.grants owned
                       WHERE owned.user_id=? AND owned.role=?
                       AND NOT EXISTS(
                         SELECT 1 FROM private.grants other
                         JOIN private.users u ON u.id=other.user_id
                         WHERE other.collection_id=owned.collection_id
                           AND other.user_id<>owned.user_id
                           AND other.role=? AND u.enabled=1
                       )
                     )",
                params![user.to_string(), Role::Owner as i64, Role::Owner as i64],
                |row| row.get(0),
            )
            .map_err(storage)?;

        let other_administrators: u64 = tx
            .query_row(
                "SELECT COUNT(*) FROM private.users WHERE id<>? AND enabled=1 AND administrator=1",
                [user.to_string()],
                |row| row.get(0),
            )
            .map_err(storage)?;
        let effects = collector_app::security::account_update(
            &collector_app::security::AccountAuthority {
                revision: Revision(revision),
                enabled: was_enabled,
                administrator: was_administrator,
                is_default,
                sole_owner_of_any_collection: would_remove_last_owner,
                other_enabled_administrators: other_administrators,
            },
            expected,
            enabled,
            administrator,
        )?;
        let next = Revision(expected.0 + 1);
        tx.execute(
            "UPDATE private.users SET display_name=?,enabled=?,administrator=?,revision=? WHERE id=? AND revision=?",
            params![display_name, enabled, administrator, next.0, user.to_string(), expected.0],
        )
        .map_err(storage)?;
        apply_credential_effects(&tx, user, now, effects)?;
        insert_security_audit(
            &tx,
            Some(user),
            "account_updated",
            serde_json::json!({"enabled": enabled, "administrator": administrator}),
            now,
        )?;
        let result = AccountView {
            id: user,
            username,
            display_name: display_name.into(),
            administrator,
            enabled,
            revision: next,
        };
        self.record_command(&tx, &result)?;
        tx.commit().map_err(storage)?;
        Ok(result)
    }

    pub fn reassign_default(&self, user: UserId) -> Result<()> {
        let mut db = lock(&self.db)?;
        let tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage)?;
        let enabled: bool = tx
            .query_row(
                "SELECT enabled FROM private.users WHERE id=?",
                [user.to_string()],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage)?
            .ok_or(Error::NotFound)?;
        collector_app::security::default_account(enabled)?;
        tx.execute(
            "INSERT INTO private.settings(key,value) VALUES('default_user_id',?) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            [user.to_string()],
        )
        .map_err(storage)?;
        insert_security_audit(
            &tx,
            Some(user),
            "default_account_reassigned",
            serde_json::json!({}),
            self.clock.now().timestamp(),
        )?;
        self.record_command(&tx, &())?;
        tx.commit().map_err(storage)
    }

    pub fn create_user(
        &self,
        username: &str,
        password: &str,
        administrator: bool,
    ) -> Result<UserId> {
        if username.trim().is_empty()
            || username.len() > 128
            || password.len() < 12
            || password.len() > 1024
        {
            return Err(Error::Invalid(
                "username required; password must be 12–1024 bytes".into(),
            ));
        }
        let salt = SaltString::generate(&mut OsRng);
        let password_hash = Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map_err(storage)?
            .to_string();
        let id = UserId::new();
        let mut db = lock(&self.db)?;
        let tx = db.transaction().map_err(storage)?;
        tx.execute(
            "INSERT INTO private.users(id,username,password_hash,administrator,display_name,enabled,revision) VALUES (?,?,?,?,?,1,1)",
            params![id.to_string(), username, password_hash, administrator, username],
        )
        .map_err(storage)?;
        let count: u64 = tx
            .query_row("SELECT COUNT(*) FROM private.users", [], |row| row.get(0))
            .map_err(storage)?;
        if count == 1 {
            tx.execute(
                "INSERT INTO private.settings(key,value) VALUES('default_user_id',?) ON CONFLICT(key) DO NOTHING",
                [id.to_string()],
            )
            .map_err(storage)?;
        }
        self.record_command(&tx, &id)?;
        tx.commit().map_err(storage)?;
        Ok(id)
    }
    /// The durable default UserId is selected once. Account order and multi-user
    /// presentation changes cannot silently select another profile.
    pub fn default_username(&self) -> Result<String> {
        lock(&self.db)?
            .query_row(
                "SELECT u.username FROM private.settings s JOIN private.users u ON u.id=s.value WHERE s.key='default_user_id'",
                [],
                |r| r.get(0),
            )
            .optional()
            .map_err(storage)?
            .ok_or_else(|| Error::Unavailable("default user is missing or deleted; assign it explicitly through the private recovery workflow".into()))
    }
    pub fn user_count(&self) -> Result<u64> {
        lock(&self.db)?
            .query_row("SELECT COUNT(*) FROM private.users", [], |r| r.get(0))
            .map_err(storage)
    }
    pub fn login(
        &self,
        username: &str,
        password: &str,
        label: &str,
    ) -> Result<(String, AuthenticatedPrincipal)> {
        if password.len() > 1024 || username.len() > 128 || label.len() > 128 {
            return Err(Error::Unauthenticated);
        }
        let row: Option<(String, String, bool)> = lock(&self.db)?
            .query_row(
                "SELECT id,password_hash,administrator FROM private.users WHERE username=? AND enabled=1",
                [username],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()
            .map_err(storage)?;
        let (id, hash, _administrator) = match row {
            Some(row) => row,
            None => {
                // Match the dominant verification cost without disclosing whether an account exists.
                let salt = SaltString::encode_b64(b"collector-dummy-salt").map_err(storage)?;
                let _ = Argon2::default().hash_password(password.as_bytes(), &salt);
                return Err(Error::Unauthenticated);
            }
        };
        let hash = PasswordHash::new(&hash).map_err(storage)?;
        Argon2::default()
            .verify_password(password.as_bytes(), &hash)
            .map_err(|_| Error::Unauthenticated)?;
        self.issue_session(id.parse().map_err(storage)?, label)
    }
    pub fn issue_session(
        &self,
        user_id: UserId,
        label: &str,
    ) -> Result<(String, AuthenticatedPrincipal)> {
        let administrator: bool = lock(&self.db)?
            .query_row(
                "SELECT administrator FROM private.users WHERE id=? AND enabled=1",
                [user_id.to_string()],
                |r| r.get(0),
            )
            .optional()
            .map_err(storage)?
            .ok_or(Error::Unauthenticated)?;
        let device_id = DeviceId::new();
        let mut random = [0u8; 32];
        OsRng.fill_bytes(&mut random);
        let token = hex::encode(random);
        let authenticated_at = self.clock.now();
        lock(&self.db)?
            .execute(
                "INSERT INTO private.sessions(hash,user_id,session_id,label,expires,authenticated_at,created_at) VALUES (?,?,?,?,?,?,?)",
                params![
                    Digest::sha256(token.as_bytes()).hex,
                    user_id.to_string(),
                    device_id.to_string(),
                    label,
                    authenticated_at.timestamp() + 30 * 86400,
                    authenticated_at.timestamp(),
                    authenticated_at.timestamp()
                ],
            )
            .map_err(storage)?;
        Ok((
            token,
            AuthenticatedPrincipal {
                principal: Principal {
                    user_id,
                    administrator,
                    device_id: Some(device_id),
                },
                credential_kind: CredentialKind::BrowserSession,
                authenticated_at,
            },
        ))
    }
    pub fn user_by_name(&self, name: &str) -> Result<UserId> {
        let id: String = lock(&self.db)?
            .query_row(
                "SELECT id FROM private.users WHERE username=? AND enabled=1",
                [name],
                |r| r.get(0),
            )
            .optional()
            .map_err(storage)?
            .ok_or(Error::Unauthenticated)?;
        id.parse().map_err(storage)
    }
    pub fn username(&self, id: UserId) -> Result<String> {
        lock(&self.db)?
            .query_row(
                "SELECT username FROM private.users WHERE id=?",
                [id.to_string()],
                |r| r.get(0),
            )
            .optional()
            .map_err(storage)?
            .ok_or(Error::NotFound)
    }
    pub fn passkeys(&self, user: UserId) -> Result<Vec<StoredPasskey>> {
        let db = lock(&self.db)?;
        let mut stmt = db
            .prepare("SELECT passkey_id,credential_id,label,revision,record,created_at FROM private.passkeys WHERE user_id=? AND revoked_at IS NULL ORDER BY created_at,passkey_id")
            .map_err(storage)?;
        let rows = stmt
            .query_map([user.to_string()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, u64>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, i64>(5)?,
                ))
            })
            .map_err(storage)?;
        rows.map(|row| {
            let (id, credential_id, label, revision, record, created_at) = row.map_err(storage)?;
            Ok(StoredPasskey {
                view: collector_app::PasskeyView {
                    id: id.parse().map_err(storage)?,
                    label,
                    revision: Revision(revision),
                    created_at: chrono::DateTime::from_timestamp(created_at, 0).ok_or_else(
                        || Error::Storage("invalid passkey creation timestamp".into()),
                    )?,
                },
                credential_id,
                record: serde_json::from_str(&record).map_err(storage)?,
            })
        })
        .collect()
    }
    pub fn insert_passkey(
        &self,
        user: UserId,
        id: &str,
        credential: serde_json::Value,
    ) -> Result<()> {
        lock(&self.db)?
            .execute(
                "INSERT INTO private.passkeys(passkey_id,credential_id,user_id,label,revision,record,created_at,revoked_at) VALUES (?,?,?,'Passkey',1,?,?,NULL)",
                params![
                    PasskeyId::new().to_string(),
                    id,
                    user.to_string(),
                    serde_json::to_string(&credential).map_err(storage)?,
                    self.clock.now().timestamp()
                ],
            )
            .map_err(storage)?;
        Ok(())
    }
    pub fn complete_passkey_authentication(
        &self,
        user: UserId,
        passkey: PasskeyId,
        expected_revision: Revision,
        expected_record: serde_json::Value,
        credential: serde_json::Value,
        label: &str,
    ) -> Result<(String, AuthenticatedPrincipal)> {
        if label.len() > 128 {
            return Err(Error::Unauthenticated);
        }
        let expected_record = serde_json::to_string(&expected_record).map_err(storage)?;
        let credential = serde_json::to_string(&credential).map_err(storage)?;
        let mut random = [0u8; 32];
        OsRng.fill_bytes(&mut random);
        let token = hex::encode(random);
        let session_id = DeviceId::new();
        let now = self.clock.now();
        let mut db = lock(&self.db)?;
        let tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage)?;
        let administrator: bool = tx
            .query_row(
                "SELECT administrator FROM private.users WHERE id=? AND enabled=1",
                [user.to_string()],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage)?
            .ok_or(Error::Unauthenticated)?;
        let changed = tx
            .execute(
                "UPDATE private.passkeys SET record=?,revision=revision+1 WHERE passkey_id=? AND user_id=? AND revision=? AND record=? AND revoked_at IS NULL",
                params![
                    credential,
                    passkey.to_string(),
                    user.to_string(),
                    expected_revision.0,
                    expected_record
                ],
            )
            .map_err(storage)?;
        if changed != 1 {
            return Err(Error::Unauthenticated);
        }
        tx.execute(
            "INSERT INTO private.sessions(hash,user_id,session_id,label,expires,authenticated_at,created_at) VALUES(?,?,?,?,?,?,?)",
            params![
                Digest::sha256(token.as_bytes()).hex,
                user.to_string(),
                session_id.to_string(),
                label,
                now.timestamp() + 30 * 86400,
                now.timestamp(),
                now.timestamp()
            ],
        )
        .map_err(storage)?;
        insert_security_audit(
            &tx,
            Some(user),
            "passkey_authenticated",
            serde_json::json!({"passkey_id": passkey}),
            now.timestamp(),
        )?;
        tx.commit().map_err(storage)?;
        Ok((
            token,
            AuthenticatedPrincipal {
                principal: Principal {
                    user_id: user,
                    administrator,
                    device_id: Some(session_id),
                },
                credential_kind: CredentialKind::BrowserSession,
                authenticated_at: now,
            },
        ))
    }

    pub fn revoke_passkey(
        &self,
        user: UserId,
        passkey: PasskeyId,
        expected: Revision,
    ) -> Result<()> {
        let now = self.clock.now().timestamp();
        let mut db = lock(&self.db)?;
        let tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage)?;
        if tx
            .execute(
                "UPDATE private.passkeys SET revoked_at=?,revision=revision+1 WHERE passkey_id=? AND user_id=? AND revision=? AND revoked_at IS NULL",
                params![now, passkey.to_string(), user.to_string(), expected.0],
            )
            .map_err(storage)?
            != 1
        {
            return Err(Error::Conflict);
        }
        insert_security_audit(
            &tx,
            Some(user),
            "passkey_revoked",
            serde_json::json!({"passkey_id": passkey}),
            now,
        )?;
        self.record_command(&tx, &())?;
        tx.commit().map_err(storage)
    }

    pub fn authenticate(&self, token: &str) -> Result<AuthenticatedPrincipal> {
        if token.len() != 64 {
            return Err(Error::Unauthenticated);
        }
        let row:Option<(String,bool,String,i64)>=lock(&self.db)?.query_row("SELECT u.id,u.administrator,s.session_id,s.authenticated_at FROM private.sessions s JOIN private.users u ON u.id=s.user_id WHERE s.hash=? AND s.expires>? AND u.enabled=1",params![Digest::sha256(token.as_bytes()).hex,self.clock.now().timestamp()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).optional().map_err(storage)?;
        let (user, administrator, device, authenticated_at) = row.ok_or(Error::Unauthenticated)?;
        Ok(AuthenticatedPrincipal {
            principal: Principal {
                user_id: user.parse().map_err(storage)?,
                administrator,
                device_id: Some(device.parse().map_err(storage)?),
            },
            credential_kind: CredentialKind::BrowserSession,
            authenticated_at: chrono::DateTime::from_timestamp(authenticated_at, 0)
                .ok_or_else(|| Error::Storage("invalid session authentication timestamp".into()))?,
        })
    }
    pub(crate) fn revoke_session_record(&self, user: UserId, device: DeviceId) -> Result<()> {
        let mut db = lock(&self.db)?;
        let tx = db.transaction().map_err(storage)?;
        tx.execute(
            "DELETE FROM private.sessions WHERE session_id=? AND user_id=?",
            params![device.to_string(), user.to_string()],
        )
        .map_err(storage)?;
        self.record_command(&tx, &())?;
        tx.commit().map_err(storage)
    }

    pub fn sessions(&self, user: UserId, current: Option<DeviceId>) -> Result<Vec<SessionView>> {
        let now = self.clock.now().timestamp();
        let db = lock(&self.db)?;
        let mut statement = db
            .prepare(
                "SELECT session_id,label,created_at,expires FROM private.sessions WHERE user_id=? AND expires>? ORDER BY created_at DESC,session_id",
            )
            .map_err(storage)?;
        let rows = statement
            .query_map(params![user.to_string(), now], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            })
            .map_err(storage)?;
        rows.map(|row| {
            let (id, label, created_at, expires_at) = row.map_err(storage)?;
            let parsed_device: DeviceId = id.parse().map_err(storage)?;
            Ok(SessionView {
                id: id.parse().map_err(storage)?,
                label,
                created_at: chrono::DateTime::from_timestamp(created_at, 0)
                    .ok_or_else(|| Error::Storage("invalid session creation timestamp".into()))?,
                expires_at: chrono::DateTime::from_timestamp(expires_at, 0)
                    .ok_or_else(|| Error::Storage("invalid session expiry timestamp".into()))?,
                current: current == Some(parsed_device),
            })
        })
        .collect()
    }

    pub fn replace_recovery_codes(&self, user: UserId) -> Result<Vec<String>> {
        let now = self.clock.now().timestamp();
        let batch = TransactionId::new().to_string();
        let mut codes = Vec::with_capacity(10);
        for _ in 0..10 {
            let mut random = [0u8; 20];
            OsRng.fill_bytes(&mut random);
            codes.push(hex::encode(random));
        }
        let mut db = lock(&self.db)?;
        let tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage)?;
        let enabled: bool = tx
            .query_row(
                "SELECT enabled FROM private.users WHERE id=?",
                [user.to_string()],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage)?
            .ok_or(Error::NotFound)?;
        if !enabled {
            return Err(Error::Forbidden);
        }
        apply_credential_effects(
            &tx,
            user,
            now,
            collector_app::security::credential_effects(
                collector_app::security::CredentialChange::ReplaceRecoveryCodes,
            ),
        )?;
        tx.execute(
            "INSERT INTO private.recovery_batches(id,user_id,created_at,invalidated_at) VALUES(?,?,?,NULL)",
            params![batch, user.to_string(), now],
        )
        .map_err(storage)?;
        for code in &codes {
            tx.execute(
                "INSERT INTO private.recovery_codes(batch_id,code_hash,consumed_at) VALUES(?,?,NULL)",
                params![batch, Digest::sha256(code.as_bytes()).hex],
            )
            .map_err(storage)?;
        }
        insert_security_audit(
            &tx,
            Some(user),
            "recovery_codes_replaced",
            serde_json::json!({}),
            now,
        )?;
        tx.commit().map_err(storage)?;
        Ok(codes)
    }

    pub fn consume_recovery_code(
        &self,
        username: &str,
        code: &str,
        new_password: &str,
        label: &str,
    ) -> Result<(String, AuthenticatedPrincipal)> {
        if username.len() > 128
            || label.len() > 128
            || code.len() != 40
            || !code.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(Error::Unauthenticated);
        }
        if !(12..=1024).contains(&new_password.len()) {
            return Err(Error::Invalid("password must be 12–1024 bytes".into()));
        }
        let salt = SaltString::generate(&mut OsRng);
        let password_hash = Argon2::default()
            .hash_password(new_password.as_bytes(), &salt)
            .map_err(storage)?
            .to_string();
        let mut token_random = [0u8; 32];
        OsRng.fill_bytes(&mut token_random);
        let token = hex::encode(token_random);
        let session_id = DeviceId::new();
        let now = self.clock.now();
        let mut db = lock(&self.db)?;
        let tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage)?;
        let account: Option<(String, bool)> = tx
            .query_row(
                "SELECT id,administrator FROM private.users WHERE username=? AND enabled=1",
                [username],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(storage)?;
        let (user, administrator) = account.ok_or(Error::Unauthenticated)?;
        let code_hash = Digest::sha256(code.to_ascii_lowercase().as_bytes()).hex;
        let batch: Option<String> = tx
            .query_row(
                "SELECT b.id FROM private.recovery_batches b JOIN private.recovery_codes c ON c.batch_id=b.id WHERE b.user_id=? AND b.invalidated_at IS NULL AND c.code_hash=? AND c.consumed_at IS NULL",
                params![user, code_hash],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage)?;
        let batch = batch.ok_or(Error::Unauthenticated)?;
        if tx
            .execute(
                "UPDATE private.recovery_codes SET consumed_at=? WHERE batch_id=? AND code_hash=? AND consumed_at IS NULL",
                params![now.timestamp(), batch, code_hash],
            )
            .map_err(storage)?
            != 1
        {
            return Err(Error::Unauthenticated);
        }
        tx.execute(
            "UPDATE private.recovery_batches SET invalidated_at=? WHERE id=? AND invalidated_at IS NULL",
            params![now.timestamp(), batch],
        )
        .map_err(storage)?;
        tx.execute(
            "UPDATE private.users SET password_hash=?,revision=revision+1 WHERE id=? AND enabled=1",
            params![password_hash, user],
        )
        .map_err(storage)?;
        apply_credential_effects(
            &tx,
            user.parse().map_err(storage)?,
            now.timestamp(),
            collector_app::security::credential_effects(
                collector_app::security::CredentialChange::RecoverPassword,
            ),
        )?;
        tx.execute(
            "INSERT INTO private.sessions(hash,user_id,session_id,label,expires,authenticated_at,created_at) VALUES(?,?,?,?,?,?,?)",
            params![
                Digest::sha256(token.as_bytes()).hex,
                user,
                session_id.to_string(),
                label,
                now.timestamp() + 30 * 86400,
                now.timestamp(),
                now.timestamp()
            ],
        )
        .map_err(storage)?;
        let user_id: UserId = user.parse().map_err(storage)?;
        insert_security_audit(
            &tx,
            Some(user_id),
            "account_recovered",
            serde_json::json!({"sessions_revoked": true}),
            now.timestamp(),
        )?;
        tx.commit().map_err(storage)?;
        Ok((
            token,
            AuthenticatedPrincipal {
                principal: Principal {
                    user_id,
                    administrator,
                    device_id: Some(session_id),
                },
                credential_kind: CredentialKind::BrowserSession,
                authenticated_at: now,
            },
        ))
    }
    pub fn reset_password(&self, username: &str, password: &str) -> Result<()> {
        if password.len() < 12 || password.len() > 1024 {
            return Err(Error::Invalid("password must be 12–1024 bytes".into()));
        }
        let salt = SaltString::generate(&mut OsRng);
        let hash = Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map_err(storage)?
            .to_string();
        let mut db = lock(&self.db)?;
        let tx = db.transaction().map_err(storage)?;
        if tx
            .execute(
                "UPDATE private.users SET password_hash=? WHERE username=?",
                params![hash, username],
            )
            .map_err(storage)?
            == 0
        {
            return Err(Error::NotFound);
        }
        tx.execute("DELETE FROM private.sessions WHERE user_id IN (SELECT id FROM private.users WHERE username=?)",[username]).map_err(storage)?;
        tx.commit().map_err(storage)
    }
}

fn insert_security_audit(
    tx: &rusqlite::Transaction<'_>,
    user: Option<UserId>,
    event: &str,
    metadata: serde_json::Value,
    created_at: i64,
) -> Result<()> {
    tx.execute(
        "INSERT INTO private.security_audit(id,user_id,event,metadata,created_at) VALUES(?,?,?,?,?)",
        params![
            TransactionId::new().to_string(),
            user.map(|value| value.to_string()),
            event,
            serde_json::to_string(&metadata).map_err(storage)?,
            created_at
        ],
    )
    .map_err(storage)?;
    Ok(())
}

fn expire_enrollments(tx: &rusqlite::Transaction<'_>, now: i64) -> Result<()> {
    tx.execute(
        "UPDATE private.enrollments SET phase='expired' WHERE expires_at<=? AND phase IN ('pending_approval','approved')",
        [now],
    )
    .map_err(storage)?;
    Ok(())
}

/// Apply the application policy's revocation set in the caller's transaction.
fn apply_credential_effects(
    db: &Connection,
    user: UserId,
    now: i64,
    effects: collector_app::security::CredentialEffects,
) -> Result<()> {
    if effects.browser_sessions {
        db.execute(
            "DELETE FROM private.sessions WHERE user_id=?",
            [user.to_string()],
        )
        .map_err(storage)?;
    }
    if effects.recovery_batches {
        db.execute("UPDATE private.recovery_batches SET invalidated_at=? WHERE user_id=? AND invalidated_at IS NULL", params![now,user.to_string()]).map_err(storage)?;
    }
    if effects.passkeys {
        db.execute("UPDATE private.passkeys SET revoked_at=?,revision=revision+1 WHERE user_id=? AND revoked_at IS NULL", params![now,user.to_string()]).map_err(storage)?;
    }
    if effects.native_credentials || effects.enrollments {
        let version: u32 = db
            .query_row("PRAGMA private.user_version", [], |row| row.get(0))
            .map_err(storage)?;
        if version >= 6 {
            if effects.native_credentials {
                db.execute("UPDATE private.native_credentials SET revoked_at=?,revision=revision+1 WHERE user_id=? AND revoked_at IS NULL",params![now,user.to_string()]).map_err(storage)?;
            }
            if effects.enrollments {
                db.execute("UPDATE private.enrollments SET phase='cancelled',cancelled_at=? WHERE user_id=? AND phase IN ('pending_approval','approved')",params![now,user.to_string()]).map_err(storage)?;
            }
        }
    }
    Ok(())
}

fn parse_enrollment_phase(value: &str) -> Result<collector_app::EnrollmentPhase> {
    use collector_app::EnrollmentPhase::*;
    match value {
        "pending_approval" => Ok(PendingApproval),
        "approved" => Ok(Approved),
        "issued" => Ok(Issued),
        "cancelled" => Ok(Cancelled),
        "expired" => Ok(Expired),
        _ => Err(Error::Storage("unknown enrollment phase".into())),
    }
}
