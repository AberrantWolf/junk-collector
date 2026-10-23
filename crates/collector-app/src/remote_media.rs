//! Scoped remote reads. The media process receives no account credentials or database.
use crate::{AuthenticatedPrincipal, CollectionService, CredentialKind};
use collector_api_types::{
    MediaFile, MediaReadTicket, MusicTrackAvailabilityState, PlaybackTarget, RemoteMediaGrant,
};
use collector_core::*;
use std::{collections::HashMap, sync::Mutex};

#[derive(Clone)]
struct Grant {
    user: UserId,
    device: DeviceId,
    kind: CredentialKind,
    origin: Option<String>,
    audience: String,
    target: Option<PlaybackTarget>,
    file: MediaFile,
    expires: chrono::DateTime<chrono::Utc>,
}
#[derive(Clone)]
struct Read {
    grant: Grant,
    deadline: chrono::DateTime<chrono::Utc>,
    heartbeat: chrono::DateTime<chrono::Utc>,
}
#[derive(Default)]
struct State {
    grants: HashMap<String, Grant>,
    reads: HashMap<String, Read>,
}
#[derive(Default)]
pub(crate) struct RemoteMediaGrants(Mutex<State>);
fn key(secret: &str) -> Result<String> {
    if secret.len() != 64 || !secret.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(Error::Unauthenticated);
    }
    Ok(Digest::sha256(secret.as_bytes()).hex)
}
impl CollectionService {
    fn remote_target(&self, actor: &Principal, target: &PlaybackTarget) -> Result<()> {
        // Include file visibility after role changes, not just collection access.
        // Opening metadata performs no payload hashing.
        self.representation(
            actor,
            target.item_id,
            target.representation_id,
            target.file_index,
        )?;
        let tracks = self.recorded_music_track_availability(actor, target.item_id)?;
        let playable = tracks.iter().any(|track| match &track.state {
            MusicTrackAvailabilityState::Available {
                representation_id,
                file_index,
                ..
            } => *representation_id == target.representation_id && *file_index == target.file_index,
            _ => false,
        });
        if !playable {
            return Err(Error::Forbidden);
        }
        Ok(())
    }
    fn remote_parent(&self, grant: &Grant) -> Result<()> {
        let (actor, _) =
            self.authentication
                .authenticate_reference(grant.kind, grant.user, grant.device)?;
        if let Some(target) = &grant.target {
            self.remote_target(&actor.principal, target)?;
        }
        Ok(())
    }
    pub fn issue_remote_media(
        &self,
        actor: &AuthenticatedPrincipal,
        target: Option<PlaybackTarget>,
        origin: Option<String>,
        audience: &str,
        server: ServerId,
        token: String,
    ) -> Result<RemoteMediaGrant> {
        let hash = key(&token)?;
        let device = actor.principal.device_id.ok_or(Error::Unauthenticated)?;
        let (_, parent_expiry) = self.authentication.authenticate_reference(
            actor.credential_kind,
            actor.principal.user_id,
            device,
        )?;
        if (actor.credential_kind == CredentialKind::BrowserSession) != origin.is_some() {
            return Err(Error::Forbidden);
        }
        let now = self.authentication.now();
        let expires = (now + chrono::Duration::minutes(if target.is_some() { 60 } else { 1 }))
            .min(parent_expiry);
        let file = if let Some(target) = &target {
            // Resolve the internal locator only after the same live target policy.
            self.remote_target(&actor.principal, target)?;
            self.content.remote_media_file(target)?
        } else {
            let text = format!(
                "collector-media-probe-v1:{server}:{}",
                Digest::sha256(token.as_bytes()).hex
            );
            MediaFile {
                relative_path: None,
                length: text.len() as u64,
                digest: Digest::sha256(text.as_bytes()),
                inline: Some(text),
                media_type: "application/octet-stream".into(),
                verify_on_transfer: false,
            }
        };
        let grant = Grant {
            user: actor.principal.user_id,
            device,
            kind: actor.credential_kind,
            origin,
            audience: audience.into(),
            target,
            file: file.clone(),
            expires,
        };
        self.remote_parent(&grant)?;
        let mut state = self
            .remote_media_grants
            .0
            .lock()
            .map_err(|_| Error::Unavailable("media authority unavailable".into()))?;
        state.grants.retain(|_, g| g.expires > now);
        state
            .reads
            .retain(|_, r| r.deadline > now && r.heartbeat + chrono::Duration::seconds(30) > now);
        if state.grants.contains_key(&hash) {
            return Err(Error::Conflict);
        }
        if state
            .grants
            .values()
            .filter(|g| g.user == grant.user)
            .count()
            >= 32
            && let Some(old) = state
                .grants
                .iter()
                .filter(|(_, g)| g.user == grant.user)
                .min_by_key(|(_, g)| g.expires)
                .map(|(k, _)| k.clone())
        {
            state.grants.remove(&old);
        }
        if state.grants.len() >= 1024 {
            return Err(Error::RateLimited(10));
        }
        state.grants.insert(hash, grant);
        Ok(RemoteMediaGrant {
            url: format!("{audience}read?grant={token}"),
            token,
            expires_at: expires,
            server_id: server,
            length: file.length,
            etag: format!("\"{}\"", file.digest.hex),
        })
    }
    pub fn authorize_remote_media(
        &self,
        token: &str,
        origin: Option<&str>,
        audience: &str,
        handle: String,
    ) -> Result<MediaReadTicket> {
        let hash = key(token)?;
        let handle_hash = key(&handle)?;
        let now = self.authentication.now();
        let grant = {
            let mut state = self
                .remote_media_grants
                .0
                .lock()
                .map_err(|_| Error::Unavailable("media authority unavailable".into()))?;
            let grant = state
                .grants
                .get(&hash)
                .cloned()
                .ok_or(Error::Unauthenticated)?;
            // Same-origin GET/HEAD may omit Origin. Permit that only when the
            // grant's browser origin is the configured media audience itself.
            // Cross-origin deployments still require the exact browser Origin.
            let same_origin_read = origin.is_none()
                && grant.origin.as_ref().is_some_and(|expected| {
                    url::Url::parse(&grant.audience)
                        .is_ok_and(|url| url.origin().ascii_serialization() == *expected)
                });
            if grant.expires <= now
                || grant.audience != audience
                || (grant.origin.as_deref() != origin && !same_origin_read)
            {
                return Err(Error::Unauthenticated);
            }
            if grant.target.is_none() {
                state.grants.remove(&hash);
            }
            grant
        };
        self.remote_parent(&grant)?;
        let mut state = self
            .remote_media_grants
            .0
            .lock()
            .map_err(|_| Error::Unavailable("media authority unavailable".into()))?;
        state
            .reads
            .retain(|_, r| r.deadline > now && r.heartbeat + chrono::Duration::seconds(30) > now);
        if state.reads.contains_key(&handle_hash) {
            return Err(Error::Conflict);
        }
        if state.reads.len() >= 64
            || state
                .reads
                .values()
                .filter(|r| r.grant.user == grant.user)
                .count()
                >= 8
        {
            return Err(Error::RateLimited(1));
        }
        let ticket = MediaReadTicket {
            handle,
            file: grant.file.clone(),
        };
        state.reads.insert(
            handle_hash,
            Read {
                grant,
                deadline: now + chrono::Duration::hours(4),
                heartbeat: now,
            },
        );
        Ok(ticket)
    }
    pub fn recheck_remote_media(&self, handle: &str, audience: &str) -> Result<()> {
        let hash = key(handle)?;
        let read = self
            .remote_media_grants
            .0
            .lock()
            .map_err(|_| Error::Unavailable("media authority unavailable".into()))?
            .reads
            .get(&hash)
            .cloned()
            .ok_or(Error::Unauthenticated)?;
        let now = self.authentication.now();
        if read.deadline <= now
            || read.heartbeat + chrono::Duration::seconds(30) <= now
            || read.grant.audience != audience
        {
            return Err(Error::Unauthenticated);
        }
        self.remote_parent(&read.grant)?;
        let now = self.authentication.now();
        let mut state = self
            .remote_media_grants
            .0
            .lock()
            .map_err(|_| Error::Unavailable("media authority unavailable".into()))?;
        let live = state.reads.get_mut(&hash).ok_or(Error::Unauthenticated)?;
        if live.deadline <= now || live.heartbeat + chrono::Duration::seconds(30) <= now {
            return Err(Error::Unauthenticated);
        }
        live.heartbeat = now;
        Ok(())
    }
    pub fn release_remote_media(&self, handle: &str) -> Result<()> {
        self.remote_media_grants
            .0
            .lock()
            .map_err(|_| Error::Unavailable("media authority unavailable".into()))?
            .reads
            .remove(&key(handle)?);
        Ok(())
    }
    pub fn remote_media_samples(
        &self,
        actor: &AuthenticatedPrincipal,
        target: &PlaybackTarget,
    ) -> Result<Vec<collector_api_types::RemoteMediaSample>> {
        use std::io::{Read, Seek, SeekFrom};
        let mut resource = self.representation(
            &actor.principal,
            target.item_id,
            target.representation_id,
            target.file_index,
        )?;
        if resource.length == 0 {
            return Err(Error::Invalid("empty audio".into()));
        }
        let mut samples = Vec::new();
        for start in [0, resource.length.saturating_sub(16)] {
            let length = (resource.length - start).min(16) as usize;
            let mut bytes = vec![0; length];
            resource
                .reader
                .seek(SeekFrom::Start(start))
                .map_err(|_| Error::Unavailable("media probe source unavailable".into()))?;
            resource
                .reader
                .read_exact(&mut bytes)
                .map_err(|_| Error::Unavailable("media probe source unavailable".into()))?;
            samples.push(collector_api_types::RemoteMediaSample {
                start,
                end: start + length as u64 - 1,
                digest: Digest::sha256(&bytes),
            });
        }
        Ok(samples)
    }
}
