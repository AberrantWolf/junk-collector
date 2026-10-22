//! Browser media authority. No listener, HTTP cookie, network or persistence mechanisms.
use crate::{CollectionService, MediaResource};
use collector_api_types::{BrowserPlaybackGrant, PlaybackTarget};
use collector_core::*;
use std::{collections::HashMap, sync::Mutex};

#[derive(Clone)]
struct Grant {
    session: String,
    user: UserId,
    origin: String,
    target: PlaybackTarget,
    expires: chrono::DateTime<chrono::Utc>,
}

#[derive(Default)]
pub(crate) struct BrowserMediaGrants(Mutex<HashMap<String, Grant>>);

impl CollectionService {
    /// The adapter supplies an unpredictable token and a normalized allowed origin.
    pub fn issue_browser_playback(
        &self,
        session: &str,
        origin: &str,
        target: PlaybackTarget,
        token: String,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<BrowserPlaybackGrant> {
        if origin.is_empty()
            || origin.len() > 512
            || token.len() != 64
            || !token.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err(Error::Invalid("invalid media enrollment".into()));
        }
        let actor = self.authenticate(session)?.principal;
        // Admission and every subsequent read both enforce the actual current role.
        self.representation(
            &actor,
            target.item_id,
            target.representation_id,
            target.file_index,
        )?;
        let expires = now + chrono::Duration::seconds(120);
        let mut grants = self
            .browser_media_grants
            .0
            .lock()
            .map_err(|_| Error::Unavailable("media authority unavailable".into()))?;
        grants.retain(|_, grant| grant.expires > now);
        let owned: Vec<_> = grants
            .iter()
            .filter(|(_, g)| g.user == actor.user_id)
            .map(|(key, g)| (key.clone(), g.expires))
            .collect();
        if owned.len() >= 8
            && let Some((key, _)) = owned.into_iter().min_by_key(|(_, expiry)| *expiry)
        {
            grants.remove(&key);
        }
        if grants.len() >= 1024 {
            return Err(Error::RateLimited(10));
        }
        let key = Digest::sha256(token.as_bytes()).hex;
        if grants.contains_key(&key) {
            return Err(Error::Conflict);
        }
        grants.insert(
            key,
            Grant {
                session: session.into(),
                user: actor.user_id,
                origin: origin.into(),
                target,
                expires,
            },
        );
        Ok(BrowserPlaybackGrant {
            token,
            expires_at: expires,
        })
    }

    pub fn browser_playback_resource(
        &self,
        token: &str,
        origin: &str,
        target: &PlaybackTarget,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<MediaResource> {
        if token.len() != 64 {
            return Err(Error::Unauthenticated);
        }
        let grant = self
            .browser_media_grants
            .0
            .lock()
            .map_err(|_| Error::Unavailable("media authority unavailable".into()))?
            .get(&Digest::sha256(token.as_bytes()).hex)
            .cloned()
            .ok_or(Error::Unauthenticated)?;
        if grant.expires <= now || grant.origin != origin || grant.target != *target {
            return Err(Error::Unauthenticated);
        }
        let actor = self.authenticate(&grant.session)?.principal;
        if actor.user_id != grant.user {
            return Err(Error::Unauthenticated);
        }
        self.representation(
            &actor,
            target.item_id,
            target.representation_id,
            target.file_index,
        )
    }
}
