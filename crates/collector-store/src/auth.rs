use super::*;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier, password_hash::SaltString};
use rand::{RngCore, rngs::OsRng};
impl Store {
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
            "INSERT INTO private.users VALUES (?,?,?,?)",
            params![id.to_string(), username, password_hash, administrator],
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
    ) -> Result<(String, Principal)> {
        if password.len() > 1024 || username.len() > 128 || label.len() > 128 {
            return Err(Error::Unauthenticated);
        }
        let row: Option<(String, String, bool)> = lock(&self.db)?
            .query_row(
                "SELECT id,password_hash,administrator FROM private.users WHERE username=?",
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
    pub fn issue_session(&self, user_id: UserId, label: &str) -> Result<(String, Principal)> {
        let administrator: bool = lock(&self.db)?
            .query_row(
                "SELECT administrator FROM private.users WHERE id=?",
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
        lock(&self.db)?
            .execute(
                "INSERT INTO private.sessions VALUES (?,?,?,?,?)",
                params![
                    Digest::sha256(token.as_bytes()).hex,
                    user_id.to_string(),
                    device_id.to_string(),
                    label,
                    Utc::now().timestamp() + 30 * 86400
                ],
            )
            .map_err(storage)?;
        Ok((
            token,
            Principal {
                user_id,
                administrator,
                device_id: Some(device_id),
            },
        ))
    }
    pub fn user_by_name(&self, name: &str) -> Result<UserId> {
        let id: String = lock(&self.db)?
            .query_row(
                "SELECT id FROM private.users WHERE username=?",
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
    pub fn passkeys(&self, user: UserId) -> Result<Vec<serde_json::Value>> {
        let db = lock(&self.db)?;
        let mut stmt = db
            .prepare("SELECT record FROM private.passkeys WHERE user_id=?")
            .map_err(storage)?;
        let rows = stmt
            .query_map([user.to_string()], |r| r.get::<_, String>(0))
            .map_err(storage)?;
        rows.map(|row| serde_json::from_str(&row.map_err(storage)?).map_err(storage))
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
                "INSERT INTO private.passkeys VALUES (?,?,?)",
                params![
                    id,
                    user.to_string(),
                    serde_json::to_string(&credential).map_err(storage)?
                ],
            )
            .map_err(storage)?;
        Ok(())
    }
    pub fn update_passkey(
        &self,
        user: UserId,
        id: &str,
        credential: serde_json::Value,
    ) -> Result<()> {
        lock(&self.db)?
            .execute(
                "UPDATE private.passkeys SET record=? WHERE user_id=? AND credential_id=?",
                params![
                    serde_json::to_string(&credential).map_err(storage)?,
                    user.to_string(),
                    id
                ],
            )
            .map_err(storage)?;
        Ok(())
    }

    pub fn authenticate(&self, token: &str) -> Result<Principal> {
        if token.len() != 64 {
            return Err(Error::Unauthenticated);
        }
        let row:Option<(String,bool,String)>=lock(&self.db)?.query_row("SELECT u.id,u.administrator,s.device_id FROM private.sessions s JOIN private.users u ON u.id=s.user_id WHERE s.hash=? AND s.expires>?",params![Digest::sha256(token.as_bytes()).hex,Utc::now().timestamp()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional().map_err(storage)?;
        let (user, administrator, device) = row.ok_or(Error::Unauthenticated)?;
        Ok(Principal {
            user_id: user.parse().map_err(storage)?,
            administrator,
            device_id: Some(device.parse().map_err(storage)?),
        })
    }
    pub fn revoke_device(&self, actor: &Principal, device: DeviceId) -> Result<()> {
        lock(&self.db)?
            .execute(
                "DELETE FROM private.sessions WHERE device_id=? AND user_id=?",
                params![device.to_string(), actor.user_id.to_string()],
            )
            .map_err(storage)?;
        Ok(())
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
