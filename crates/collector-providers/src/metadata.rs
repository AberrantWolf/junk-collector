use collector_app::{
    CatalogParserPort, ParsedCatalogSnapshot, ProcessingExecutionContext, ProviderPort,
    ProviderRequest, ProviderResponse, ProviderStreamResponse, SnapshotRecord,
    StreamingProviderPort,
};
use collector_core::{ByteLength, Digest, Error, RawJson, Result};
use collector_modules::music::{
    COVER_ART_ARCHIVE_ACQUIRE_OPERATION, COVER_ART_ARCHIVE_PROVIDER_ID,
    COVER_ART_ARCHIVE_SNAPSHOT_SCHEMA_VERSION, CoverArtArchiveQuery,
    CoverArtArchiveSnapshotObservation, CoverArtObservation, MUSICBRAINZ_ACQUIRE_OPERATION,
    MUSICBRAINZ_PROVIDER_ID, MUSICBRAINZ_SNAPSHOT_SCHEMA_VERSION, MetadataProviderOutcome,
    MusicBrainzQuery, MusicBrainzReleaseObservation, MusicBrainzSnapshotObservation,
    musicbrainz_metadata_candidates,
};
use reqwest::{
    StatusCode, Url,
    blocking::{Client, Response},
    header::{CONTENT_TYPE, LOCATION, RETRY_AFTER},
};
use serde::Deserialize;
use sha2::{Digest as _, Sha256};
use std::{
    collections::{BTreeMap, HashMap},
    io::{Read, Seek, Write},
    net::IpAddr,
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant, SystemTime},
};

const MAX_JSON_BYTES: u64 = 8 * 1024 * 1024;
const MAX_IMAGE_BYTES: u64 = 20 * 1024 * 1024;
const MAX_ATTEMPTS: usize = 3;
const MAX_REDIRECTS: usize = 3;
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(15);
const MUSICBRAINZ_MINIMUM_INTERVAL: Duration = Duration::from_secs(1);
const SUCCESS_CACHE_TTL: Duration = Duration::from_secs(24 * 60 * 60);
const NEGATIVE_CACHE_TTL: Duration = Duration::from_secs(60 * 60);
const DEFAULT_RETRY_DELAY: Duration = Duration::from_secs(1);
const MAX_RETRY_DELAY: Duration = Duration::from_secs(60);
const READ_CHUNK_BYTES: usize = 64 * 1024;

#[derive(Clone)]
struct OriginPolicy {
    allowed: Vec<Url>,
    allow_private_for_tests: bool,
}

impl OriginPolicy {
    fn allows(&self, url: &Url) -> bool {
        url.username().is_empty()
            && url.password().is_none()
            && (self.allowed.iter().any(|origin| same_origin(origin, url))
                || (!self.allow_private_for_tests
                    && self
                        .allowed
                        .iter()
                        .any(|origin| origin.host_str() == Some("archive.org"))
                    && url.scheme() == "https"
                    && url.port_or_known_default() == Some(443)
                    && url.host_str().is_some_and(|host| {
                        host == "s3.us.archive.org"
                            || archive_storage_host(host, "ia", "us")
                            || archive_storage_host(host, "dn", "ca")
                    })))
    }

    fn validate_destination(&self, url: &Url) -> Result<()> {
        if !self.allows(url) {
            return Err(Error::Unavailable(
                "provider redirect left the pinned origin set".into(),
            ));
        }
        if self.allow_private_for_tests {
            return Ok(());
        }
        if url.scheme() != "https" {
            return Err(Error::Unavailable(
                "production provider destinations require HTTPS".into(),
            ));
        }
        // DNS is validated by the resolver used by the actual connection,
        // under reqwest's request timeout; never resolve twice or use a proxy.
        Ok(())
    }
}

fn archive_storage_host(host: &str, prefix: &str, region: &str) -> bool {
    host.strip_suffix(&format!(".{region}.archive.org"))
        .and_then(|label| label.strip_prefix(prefix))
        .is_some_and(|digits| {
            !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())
        })
}

#[derive(Clone)]
struct CachedResponse {
    status: StatusCode,
    bytes: Vec<u8>,
    media_type: String,
    expires_at: SystemTime,
}

pub struct MusicBrainzProvider {
    client: Option<Client>,
    origin: Url,
    policy: OriginPolicy,
    minimum_interval: Duration,
    next_request: Mutex<Option<Instant>>,
    success_cache_ttl: Duration,
    negative_cache_ttl: Duration,
    cache: Mutex<HashMap<String, CachedResponse>>,
}

impl MusicBrainzProvider {
    pub fn new(user_agent: &str) -> Result<Self> {
        let origin = Url::parse("https://musicbrainz.org")
            .map_err(|error| Error::Invalid(error.to_string()))?;
        Self::build(
            user_agent,
            origin,
            DEFAULT_TIMEOUT,
            MUSICBRAINZ_MINIMUM_INTERVAL,
            SUCCESS_CACHE_TTL,
            NEGATIVE_CACHE_TTL,
            false,
        )
    }

    /// Test-only loopback composition. Production construction always pins HTTPS
    /// MusicBrainz and performs public-address checks before every request.
    #[doc(hidden)]
    pub fn with_test_origin(
        user_agent: &str,
        origin: &str,
        timeout: Duration,
        minimum_interval: Duration,
        success_cache_ttl: Duration,
        negative_cache_ttl: Duration,
    ) -> Result<Self> {
        let origin = test_origin(origin)?;
        Self::build(
            user_agent,
            origin,
            timeout,
            minimum_interval,
            success_cache_ttl,
            negative_cache_ttl,
            true,
        )
    }

    fn build(
        user_agent: &str,
        origin: Url,
        timeout: Duration,
        minimum_interval: Duration,
        success_cache_ttl: Duration,
        negative_cache_ttl: Duration,
        allow_private_for_tests: bool,
    ) -> Result<Self> {
        validate_http_configuration(user_agent, timeout)?;
        if success_cache_ttl.is_zero()
            || success_cache_ttl > SUCCESS_CACHE_TTL
            || negative_cache_ttl.is_zero()
            || negative_cache_ttl > NEGATIVE_CACHE_TTL
            || minimum_interval > MUSICBRAINZ_MINIMUM_INTERVAL
        {
            return Err(Error::Invalid(
                "invalid MusicBrainz cache or rate policy".into(),
            ));
        }
        let client = build_client(user_agent, timeout, "MusicBrainz")?;
        Ok(Self {
            client: Some(client),
            policy: OriginPolicy {
                allowed: vec![origin.clone()],
                allow_private_for_tests,
            },
            origin,
            minimum_interval,
            next_request: Mutex::new(None),
            success_cache_ttl,
            negative_cache_ttl,
            cache: Mutex::new(HashMap::new()),
        })
    }

    fn request_url(&self, query: &MusicBrainzQuery) -> Result<Url> {
        query.validate()?;
        let (path, parameters) = match query {
            MusicBrainzQuery::ReleaseId { release_id, .. } => (
                format!("/ws/2/release/{release_id}"),
                vec![
                    ("fmt", "json".to_owned()),
                    ("inc", "artist-credits+recordings+release-groups".to_owned()),
                ],
            ),
            MusicBrainzQuery::DiscId { disc_id, .. } => (
                format!("/ws/2/discid/{disc_id}"),
                vec![
                    ("fmt", "json".to_owned()),
                    ("inc", "artist-credits+recordings+release-groups".to_owned()),
                ],
            ),
            MusicBrainzQuery::Search { query, .. } => (
                "/ws/2/release/".into(),
                vec![
                    ("fmt", "json".to_owned()),
                    ("limit", "25".to_owned()),
                    ("query", query.clone()),
                ],
            ),
        };
        let mut url = self
            .origin
            .join(&path)
            .map_err(|error| Error::Invalid(format!("invalid MusicBrainz URL: {error}")))?;
        url.query_pairs_mut().extend_pairs(parameters);
        Ok(url)
    }

    fn acquire_query(
        &self,
        query: MusicBrainzQuery,
        limit: u64,
        execution: &mut dyn ProcessingExecutionContext,
    ) -> Result<ProviderResponse> {
        let url = self.request_url(&query)?;
        let cache_key = url.as_str().to_owned();
        if !query.refresh()
            && let Some(response) = self.cached(&cache_key)?
        {
            checkpoint(execution)?;
            if response.bytes.len() as u64 > limit {
                return Err(Error::Invalid(
                    "cached provider response exceeds requested bound".into(),
                ));
            }
            return freeze_musicbrainz(query, response.status, response.bytes, response.media_type);
        }
        let (status, bytes, media_type) = self.fetch_with_retries(
            url,
            limit.min(MAX_JSON_BYTES),
            execution,
            true,
            "MusicBrainz",
        )?;
        let response = freeze_musicbrainz(query, status, bytes, media_type)?;
        if matches!(status, StatusCode::OK | StatusCode::NOT_FOUND) {
            let observation: MusicBrainzSnapshotObservation =
                response.parsed_observation.parse()?;
            let ttl = if observation.outcome == MetadataProviderOutcome::Candidates {
                self.success_cache_ttl
            } else {
                self.negative_cache_ttl
            };
            let mut cache = self
                .cache
                .lock()
                .map_err(|_| Error::Unavailable("MusicBrainz cache lock failed".into()))?;
            cache.retain(|_, entry| entry.expires_at > SystemTime::now());
            // 32 MiB total plus at most 128 keys, including negative lookups.
            while cache.len() >= 128
                || cache.values().map(|entry| entry.bytes.len()).sum::<usize>()
                    + response.bytes.len()
                    > 32 * 1024 * 1024
            {
                let Some(oldest) = cache
                    .iter()
                    .min_by_key(|(_, entry)| entry.expires_at)
                    .map(|(key, _)| key.clone())
                else {
                    break;
                };
                cache.remove(&oldest);
            }
            cache.insert(
                cache_key,
                CachedResponse {
                    status,
                    bytes: response.bytes.clone(),
                    media_type: response.media_type.clone(),
                    expires_at: SystemTime::now() + ttl,
                },
            );
        }
        Ok(response)
    }

    fn cached(&self, key: &str) -> Result<Option<CachedResponse>> {
        let mut cache = self
            .cache
            .lock()
            .map_err(|_| Error::Unavailable("MusicBrainz cache lock failed".into()))?;
        cache.retain(|_, entry| entry.expires_at > SystemTime::now());
        let fresh = cache
            .get(key)
            .filter(|entry| entry.expires_at > SystemTime::now())
            .cloned();
        if fresh.is_none() {
            cache.remove(key);
        }
        Ok(fresh)
    }

    fn fetch_with_retries(
        &self,
        url: Url,
        limit: u64,
        execution: &mut dyn ProcessingExecutionContext,
        rate_limited: bool,
        label: &str,
    ) -> Result<(StatusCode, Vec<u8>, String)> {
        let mut last_error = None;
        for attempt in 0..MAX_ATTEMPTS {
            checkpoint(execution)?;
            let response = match self.send_redirects(url.clone(), execution, rate_limited, label) {
                Ok(response) => response,
                Err(Error::JobCancelled) => return Err(Error::JobCancelled),
                Err(error) => {
                    last_error = Some(error);
                    if attempt + 1 < MAX_ATTEMPTS {
                        wait_cancellable(DEFAULT_RETRY_DELAY, execution)?;
                        continue;
                    }
                    break;
                }
            };
            let status = response.status();
            let media_type = content_type(&response);
            if matches!(status, StatusCode::OK | StatusCode::NOT_FOUND) {
                let bytes = read_bounded(response, limit, execution, label)?;
                return Ok((status, bytes, media_type));
            }
            if status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error() {
                let delay = retry_after(response.headers()).unwrap_or(DEFAULT_RETRY_DELAY);
                let deadline = Instant::now()
                    .checked_add(delay)
                    .ok_or_else(|| Error::Unavailable("invalid provider retry deadline".into()))?;
                let mut next = self
                    .next_request
                    .lock()
                    .map_err(|_| Error::Unavailable("MusicBrainz limiter lock failed".into()))?;
                *next = Some(next.map_or(deadline, |old| old.max(deadline)));
                drop(next);
                let _ = read_bounded(response, limit, execution, label)?;
                last_error = Some(Error::Unavailable(format!(
                    "{label} provider returned HTTP {status}"
                )));
                if attempt + 1 < MAX_ATTEMPTS {
                    if delay > MAX_RETRY_DELAY {
                        return Err(Error::RateLimited(delay.as_secs()));
                    }
                    wait_cancellable(delay, execution)?;
                    continue;
                }
                break;
            }
            let _ = read_bounded(response, limit, execution, label)?;
            return Err(Error::Unavailable(format!(
                "{label} provider returned HTTP {status}"
            )));
        }
        Err(last_error.unwrap_or_else(|| Error::Unavailable(format!("{label} provider outage"))))
    }

    fn send_redirects(
        &self,
        mut url: Url,
        execution: &dyn ProcessingExecutionContext,
        rate_limited: bool,
        label: &str,
    ) -> Result<Response> {
        for redirects in 0..=MAX_REDIRECTS {
            checkpoint(execution)?;
            self.policy.validate_destination(&url)?;
            let mut admission = if rate_limited {
                Some(self.wait_for_rate_limit(execution)?)
            } else {
                None
            };
            let response = self
                .client
                .as_ref()
                .expect("provider client exists until drop")
                .get(url.clone())
                .send()
                .map_err(|error| Error::Unavailable(format!("{label} request: {error}")));
            // Keep admission through request dispatch/headers, so delayed worker
            // scheduling cannot bunch actual network starts after reservations.
            if let Some(next) = admission.as_mut() {
                **next = Some(Instant::now() + self.minimum_interval);
            }
            drop(admission);
            let response = response?;
            if !response.status().is_redirection() {
                return Ok(response);
            }
            if redirects == MAX_REDIRECTS {
                return Err(Error::Unavailable(format!(
                    "{label} redirect limit exceeded"
                )));
            }
            let location = response
                .headers()
                .get(LOCATION)
                .and_then(|value| value.to_str().ok())
                .ok_or_else(|| Error::Unavailable(format!("{label} redirect has no location")))?;
            url = url
                .join(location)
                .map_err(|_| Error::Unavailable(format!("invalid {label} redirect")))?;
        }
        unreachable!("redirect loop is bounded")
    }

    fn wait_for_rate_limit<'a>(
        &'a self,
        execution: &dyn ProcessingExecutionContext,
    ) -> Result<std::sync::MutexGuard<'a, Option<Instant>>> {
        // Hold admission until the actual start time; reservations made before
        // sleeping can bunch requests when threads wake late or are cancelled.
        let mut next = loop {
            checkpoint(execution)?;
            match self.next_request.try_lock() {
                Ok(guard) => break guard,
                Err(std::sync::TryLockError::WouldBlock) => {
                    wait_cancellable(Duration::from_millis(10), execution)?
                }
                Err(_) => return Err(Error::Unavailable("MusicBrainz limiter lock failed".into())),
            }
        };
        if let Some(deadline) = *next {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining > MAX_RETRY_DELAY {
                return Err(Error::RateLimited(remaining.as_secs().saturating_add(1)));
            }
            wait_cancellable(remaining, execution)?;
        }
        *next = Some(Instant::now() + self.minimum_interval);
        Ok(next)
    }

    pub fn parse_response_candidates(
        &self,
        response: &ProviderResponse,
        artwork: &[(String, collector_core::AssetId)],
    ) -> Result<Vec<collector_core::MetadataCandidateObservation>> {
        response.validate(MAX_JSON_BYTES)?;
        let snapshot: MusicBrainzSnapshotObservation = response.parsed_observation.parse()?;
        snapshot.validate()?;
        let releases = if snapshot.http_status == 404 {
            Vec::new()
        } else {
            normalize_musicbrainz_body(&snapshot.query, &response.bytes)?
        };
        let artwork = artwork.iter().cloned().collect::<BTreeMap<_, _>>();
        musicbrainz_metadata_candidates(&response.digest, &snapshot, &releases, &artwork)
    }
}

impl Drop for MusicBrainzProvider {
    fn drop(&mut self) {
        if let Some(client) = self.client.take() {
            let _ = thread::spawn(move || drop(client)).join();
        }
    }
}

impl ProviderPort for MusicBrainzProvider {
    fn acquire(
        &self,
        request: &ProviderRequest,
        execution: &mut dyn ProcessingExecutionContext,
    ) -> Result<ProviderResponse> {
        request.validate()?;
        if request.provider.0 != MUSICBRAINZ_PROVIDER_ID
            || request.operation.as_str() != MUSICBRAINZ_ACQUIRE_OPERATION
            || request.maximum_response_bytes > MAX_JSON_BYTES
        {
            return Err(Error::Invalid(
                "unsupported or oversized MusicBrainz request".into(),
            ));
        }
        let query: MusicBrainzQuery = request.query.parse()?;
        self.acquire_query(query, request.maximum_response_bytes, execution)
    }
}

impl CatalogParserPort for MusicBrainzProvider {
    fn parse_snapshot(
        &self,
        record: &SnapshotRecord,
        reader: &mut dyn Read,
        cancelled: &mut dyn FnMut() -> Result<bool>,
    ) -> Result<ParsedCatalogSnapshot> {
        let bytes = read_snapshot(record, reader, cancelled, MAX_JSON_BYTES)?;
        if record.provider.0 != MUSICBRAINZ_PROVIDER_ID || record.media_type != "application/json" {
            return Err(Error::Invalid("snapshot is not MusicBrainz JSON".into()));
        }
        let observation: MusicBrainzSnapshotObservation = record.parsed_observation.parse()?;
        observation.validate()?;
        let entries = if observation.http_status == 404 {
            Vec::new()
        } else {
            normalize_musicbrainz_body(&observation.query, &bytes)?
        };
        if entries.len() != usize::from(observation.release_count) {
            return Err(Error::Invalid(
                "MusicBrainz snapshot body disagrees with its observation".into(),
            ));
        }
        let parsed = ParsedCatalogSnapshot {
            observation: record.parsed_observation.clone(),
            entries,
        };
        parsed.validate()?;
        Ok(parsed)
    }
}

pub struct CoverArtArchiveProvider {
    client: Option<Client>,
    origin: Url,
    policy: OriginPolicy,
}

impl CoverArtArchiveProvider {
    pub fn new(user_agent: &str) -> Result<Self> {
        let origin = Url::parse("https://coverartarchive.org")
            .map_err(|error| Error::Invalid(error.to_string()))?;
        let cdn =
            Url::parse("https://archive.org").map_err(|error| Error::Invalid(error.to_string()))?;
        Self::build(user_agent, origin, vec![cdn], DEFAULT_TIMEOUT, false)
    }

    /// Test-only loopback origins for deterministic redirect and image fixtures.
    #[doc(hidden)]
    pub fn with_test_origins(
        user_agent: &str,
        origin: &str,
        cdn_origins: &[&str],
        timeout: Duration,
    ) -> Result<Self> {
        let origin = test_origin(origin)?;
        let cdns = cdn_origins
            .iter()
            .map(|value| test_origin(value))
            .collect::<Result<Vec<_>>>()?;
        Self::build(user_agent, origin, cdns, timeout, true)
    }

    fn build(
        user_agent: &str,
        origin: Url,
        cdn_origins: Vec<Url>,
        timeout: Duration,
        allow_private_for_tests: bool,
    ) -> Result<Self> {
        validate_http_configuration(user_agent, timeout)?;
        let client = build_client(user_agent, timeout, "Cover Art Archive")?;
        let mut allowed = vec![origin.clone()];
        allowed.extend(cdn_origins);
        Ok(Self {
            client: Some(client),
            origin,
            policy: OriginPolicy {
                allowed,
                allow_private_for_tests,
            },
        })
    }

    fn request_url(&self, query: &CoverArtArchiveQuery) -> Result<Url> {
        query.validate()?;
        let path = match query {
            CoverArtArchiveQuery::Listing { release_id } => format!("/release/{release_id}"),
            CoverArtArchiveQuery::Image {
                release_id,
                image_id,
            } => format!("/release/{release_id}/{image_id}"),
        };
        self.origin
            .join(&path)
            .map_err(|error| Error::Invalid(format!("invalid Cover Art Archive URL: {error}")))
    }

    fn fetch(
        &self,
        query: CoverArtArchiveQuery,
        limit: u64,
        execution: &mut dyn ProcessingExecutionContext,
    ) -> Result<ProviderResponse> {
        let is_listing = matches!(query, CoverArtArchiveQuery::Listing { .. });
        let allowed_limit = if is_listing {
            MAX_JSON_BYTES
        } else {
            MAX_IMAGE_BYTES
        };
        if limit > allowed_limit {
            return Err(Error::Invalid(
                "Cover Art Archive request exceeds its response limit".into(),
            ));
        }
        let response = self.fetch_response(&query, limit, execution)?;
        let status = response.status();
        let media_type = content_type(&response);
        let bytes = read_bounded(response, limit, execution, "Cover Art Archive")?;
        freeze_cover_art(query, status, bytes, media_type)
    }

    fn fetch_response(
        &self,
        query: &CoverArtArchiveQuery,
        limit: u64,
        execution: &mut dyn ProcessingExecutionContext,
    ) -> Result<Response> {
        let mut last_error = None;
        for attempt in 0..MAX_ATTEMPTS {
            let response = match self.send_redirects(self.request_url(query)?, &|| {
                execution.cancellation_requested()
            }) {
                Ok(response) => response,
                Err(Error::JobCancelled) => return Err(Error::JobCancelled),
                Err(error) => {
                    last_error = Some(error);
                    if attempt + 1 < MAX_ATTEMPTS {
                        wait_cancellable(DEFAULT_RETRY_DELAY, execution)?;
                        continue;
                    }
                    break;
                }
            };
            let status = response.status();
            if matches!(status, StatusCode::OK | StatusCode::NOT_FOUND) {
                return Ok(response);
            }
            if status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error() {
                let delay = retry_after(response.headers()).unwrap_or(DEFAULT_RETRY_DELAY);
                let _ = read_bounded(
                    response,
                    limit.min(MAX_JSON_BYTES),
                    execution,
                    "Cover Art Archive",
                )?;
                last_error = Some(Error::Unavailable(format!(
                    "Cover Art Archive returned HTTP {status}"
                )));
                if attempt + 1 < MAX_ATTEMPTS {
                    if delay > MAX_RETRY_DELAY {
                        return Err(Error::RateLimited(delay.as_secs()));
                    }
                    wait_cancellable(delay, execution)?;
                    continue;
                }
                break;
            }
            let _ = read_bounded(
                response,
                limit.min(MAX_JSON_BYTES),
                execution,
                "Cover Art Archive",
            )?;
            return Err(Error::Unavailable(format!(
                "Cover Art Archive returned HTTP {status}"
            )));
        }
        Err(last_error.unwrap_or_else(|| Error::Unavailable("Cover Art Archive outage".into())))
    }

    fn send_redirects(&self, url: Url, cancelled: &dyn Fn() -> Result<bool>) -> Result<Response> {
        self.send_redirects_with_method(url, reqwest::Method::GET, cancelled)
    }

    fn send_redirects_with_method(
        &self,
        mut url: Url,
        method: reqwest::Method,
        cancelled: &dyn Fn() -> Result<bool>,
    ) -> Result<Response> {
        for redirects in 0..=MAX_REDIRECTS {
            if cancelled()? {
                return Err(Error::JobCancelled);
            }
            self.policy.validate_destination(&url)?;
            let response = self
                .client
                .as_ref()
                .expect("provider client exists until drop")
                .request(method.clone(), url.clone())
                .send()
                .map_err(|error| {
                    Error::Unavailable(format!("Cover Art Archive request: {error}"))
                })?;
            if !response.status().is_redirection() {
                return Ok(response);
            }
            if redirects == MAX_REDIRECTS {
                return Err(Error::Unavailable(
                    "Cover Art Archive redirect limit exceeded".into(),
                ));
            }
            let location = response
                .headers()
                .get(LOCATION)
                .and_then(|value| value.to_str().ok())
                .ok_or_else(|| {
                    Error::Unavailable("Cover Art Archive redirect has no location".into())
                })?;
            url = url
                .join(location)
                .map_err(|_| Error::Unavailable("invalid Cover Art Archive redirect".into()))?;
        }
        unreachable!("redirect loop is bounded")
    }

    pub fn parse_listing(&self, response: &ProviderResponse) -> Result<Vec<CoverArtObservation>> {
        response.validate(MAX_JSON_BYTES)?;
        let snapshot: CoverArtArchiveSnapshotObservation = response.parsed_observation.parse()?;
        snapshot.validate()?;
        let entries = if snapshot.http_status == 404 {
            Vec::new()
        } else {
            normalize_cover_art_listing(&snapshot.query, &response.bytes)?
        };
        entries.iter().map(RawJson::parse).collect()
    }
}

impl Drop for CoverArtArchiveProvider {
    fn drop(&mut self) {
        if let Some(client) = self.client.take() {
            let _ = thread::spawn(move || drop(client)).join();
        }
    }
}

impl collector_app::ArtworkSourcePort for CoverArtArchiveProvider {
    fn original_size(
        &self,
        release_id: &str,
        image_id: &str,
        cancelled: &dyn Fn() -> Result<bool>,
    ) -> Result<Option<u64>> {
        let query = CoverArtArchiveQuery::Image {
            release_id: release_id.into(),
            image_id: image_id.into(),
        };
        let response = self.send_redirects_with_method(
            self.request_url(&query)?,
            reqwest::Method::HEAD,
            cancelled,
        )?;
        if cancelled()? {
            return Err(Error::JobCancelled);
        }
        match response.status() {
            StatusCode::OK => response
                .headers()
                .get(reqwest::header::CONTENT_LENGTH)
                .map(|value| {
                    value
                        .to_str()
                        .ok()
                        .and_then(|value| value.parse::<u64>().ok())
                        .ok_or_else(|| Error::Unavailable("invalid artwork Content-Length".into()))
                })
                .transpose(),
            StatusCode::NOT_FOUND => Err(Error::NotFound),
            StatusCode::METHOD_NOT_ALLOWED | StatusCode::NOT_IMPLEMENTED => Ok(None),
            status => Err(Error::Unavailable(format!(
                "Cover Art Archive size inspection returned HTTP {status}"
            ))),
        }
    }

    fn preview(
        &self,
        release_id: &str,
        image_id: &str,
        cancelled: &dyn Fn() -> Result<bool>,
    ) -> Result<collector_app::ArtworkPreview> {
        let query = CoverArtArchiveQuery::Image {
            release_id: release_id.into(),
            image_id: image_id.into(),
        };
        let mut url = self.request_url(&query)?;
        url.set_path(&format!("/release/{release_id}/{image_id}-250"));
        let response = self.send_redirects(url, cancelled)?;
        if response.status() == StatusCode::NOT_FOUND {
            return Err(Error::NotFound);
        }
        if response.status() != StatusCode::OK {
            return Err(Error::Unavailable(format!(
                "Artwork preview returned HTTP {}",
                response.status()
            )));
        }
        let media_type = content_type(&response);
        if !matches!(
            media_type.as_str(),
            "image/png" | "image/jpeg" | "image/webp"
        ) {
            return Err(Error::Unavailable(
                "Unsupported artwork preview type".into(),
            ));
        }
        let bytes = read_bounded_with_progress(
            response,
            collector_app::MAX_ARTWORK_PREVIEW_BYTES,
            &mut |_| {
                if cancelled()? {
                    Err(Error::JobCancelled)
                } else {
                    Ok(())
                }
            },
            "Artwork preview",
        )?;
        Ok(collector_app::ArtworkPreview { bytes, media_type })
    }
}

impl ProviderPort for CoverArtArchiveProvider {
    fn acquire(
        &self,
        request: &ProviderRequest,
        execution: &mut dyn ProcessingExecutionContext,
    ) -> Result<ProviderResponse> {
        request.validate()?;
        if request.provider.0 != COVER_ART_ARCHIVE_PROVIDER_ID
            || request.operation.as_str() != COVER_ART_ARCHIVE_ACQUIRE_OPERATION
        {
            return Err(Error::Invalid(
                "unsupported Cover Art Archive request".into(),
            ));
        }
        let query: CoverArtArchiveQuery = request.query.parse()?;
        self.fetch(query, request.maximum_response_bytes, execution)
    }
}

impl StreamingProviderPort for CoverArtArchiveProvider {
    fn acquire_stream(
        &self,
        request: &ProviderRequest,
        execution: &mut dyn ProcessingExecutionContext,
    ) -> Result<ProviderStreamResponse> {
        request.validate()?;
        if request.provider.0 != COVER_ART_ARCHIVE_PROVIDER_ID
            || request.operation.as_str() != COVER_ART_ARCHIVE_ACQUIRE_OPERATION
            || request.maximum_response_bytes > collector_app::MAX_PROVIDER_ARTWORK_BYTES as u64
        {
            return Err(Error::Invalid(
                "unsupported or oversized Cover Art Archive request".into(),
            ));
        }
        let query: CoverArtArchiveQuery = request.query.parse()?;
        query.validate()?;
        if matches!(query, CoverArtArchiveQuery::Listing { .. }) {
            // JSON is small; retain the same parser and publication pipeline.
            let response = self.acquire(request, execution)?;
            return Ok(ProviderStreamResponse {
                bytes: ByteLength(response.bytes.len() as u64),
                digest: response.digest,
                media_type: response.media_type,
                parsed_observation: response.parsed_observation,
                reader: Box::new(std::io::Cursor::new(response.bytes)),
            });
        }
        let mut response =
            self.fetch_response(&query, request.maximum_response_bytes, execution)?;
        let status = response.status();
        let media_type = content_type(&response);
        // Validate the status/type envelope before reading an image payload.
        let envelope = freeze_cover_art(query, status, Vec::new(), media_type)?;
        let limit = request.maximum_response_bytes;
        if let Some(length) = response.content_length()
            && length > limit
        {
            return Err(cover_size_error(length, limit));
        }
        let mut spool = tempfile::tempfile().map_err(cover_io)?;
        let mut hash = Sha256::new();
        let mut bytes = 0_u64;
        let mut buffer = [0_u8; READ_CHUNK_BYTES];
        loop {
            checkpoint(execution)?;
            let count = response.read(&mut buffer).map_err(cover_io)?;
            if count == 0 {
                break;
            }
            bytes += count as u64;
            if bytes > limit {
                return Err(cover_size_error(bytes, limit));
            }
            spool.write_all(&buffer[..count]).map_err(cover_io)?;
            hash.update(&buffer[..count]);
            execution.report_progress("downloading_provider_response", bytes, limit)?;
        }
        checkpoint(execution)?;
        spool.rewind().map_err(cover_io)?;
        Ok(ProviderStreamResponse {
            media_type: envelope.media_type,
            bytes: ByteLength(bytes),
            digest: Digest {
                algorithm: collector_core::HashAlgorithm::Sha256,
                hex: hex::encode(hash.finalize()),
            },
            parsed_observation: envelope.parsed_observation,
            reader: Box::new(spool),
        })
    }
}

fn cover_size_error(bytes: u64, limit: u64) -> Error {
    Error::Unavailable(format!(
        "Cover Art Archive original is too large ({bytes} bytes; limit {limit} bytes). Choose a smaller front cover in Find cover art, or upload a smaller image."
    ))
}

fn cover_io(error: std::io::Error) -> Error {
    Error::Unavailable(format!("Cover Art Archive transfer: {error}"))
}

impl CatalogParserPort for CoverArtArchiveProvider {
    fn parse_snapshot(
        &self,
        record: &SnapshotRecord,
        reader: &mut dyn Read,
        cancelled: &mut dyn FnMut() -> Result<bool>,
    ) -> Result<ParsedCatalogSnapshot> {
        let observation: CoverArtArchiveSnapshotObservation = record.parsed_observation.parse()?;
        observation.validate()?;
        if !matches!(observation.query, CoverArtArchiveQuery::Listing { .. }) {
            return Err(Error::Invalid(
                "cover image snapshots are not JSON listings".into(),
            ));
        }
        if record.provider.0 != COVER_ART_ARCHIVE_PROVIDER_ID
            || record.media_type != "application/json"
        {
            return Err(Error::Invalid("snapshot is not CAA JSON".into()));
        }
        let bytes = read_snapshot(record, reader, cancelled, MAX_JSON_BYTES)?;
        let entries = if observation.http_status == 404 {
            Vec::new()
        } else {
            normalize_cover_art_listing(&observation.query, &bytes)?
        };
        if entries.len() != usize::from(observation.image_count) {
            return Err(Error::Invalid(
                "Cover Art Archive body disagrees with its observation".into(),
            ));
        }
        let parsed = ParsedCatalogSnapshot {
            observation: record.parsed_observation.clone(),
            entries,
        };
        parsed.validate()?;
        Ok(parsed)
    }
}

fn freeze_musicbrainz(
    query: MusicBrainzQuery,
    status: StatusCode,
    bytes: Vec<u8>,
    media_type: String,
) -> Result<ProviderResponse> {
    if status == StatusCode::OK && media_type != "application/json" && !media_type.is_empty() {
        return Err(Error::Unavailable(
            "MusicBrainz returned a non-JSON response".into(),
        ));
    }
    let releases = if status == StatusCode::NOT_FOUND {
        Vec::new()
    } else {
        normalize_musicbrainz_body(&query, &bytes)?
    };
    let count = u16::try_from(releases.len())
        .map_err(|_| Error::Unavailable("too many MusicBrainz releases".into()))?;
    let observation = MusicBrainzSnapshotObservation {
        http_status: status.as_u16(),
        schema_version: MUSICBRAINZ_SNAPSHOT_SCHEMA_VERSION,
        query,
        outcome: if releases.is_empty() {
            MetadataProviderOutcome::NoMatch
        } else {
            MetadataProviderOutcome::Candidates
        },
        release_count: count,
    };
    observation.validate()?;
    let response = ProviderResponse {
        media_type: "application/json".into(),
        digest: Digest::sha256(&bytes),
        bytes,
        parsed_observation: RawJson::from_value(
            serde_json::to_value(observation).map_err(|error| Error::Invalid(error.to_string()))?,
        ),
        limitations: Vec::new(),
    };
    response.validate(MAX_JSON_BYTES)?;
    Ok(response)
}

fn freeze_cover_art(
    query: CoverArtArchiveQuery,
    status: StatusCode,
    bytes: Vec<u8>,
    media_type: String,
) -> Result<ProviderResponse> {
    let listing = matches!(query, CoverArtArchiveQuery::Listing { .. });
    if listing
        && status == StatusCode::OK
        && media_type != "application/json"
        && !media_type.is_empty()
    {
        return Err(Error::Unavailable(
            "Cover Art Archive returned non-JSON listing".into(),
        ));
    }
    let entries = if status == StatusCode::NOT_FOUND || !listing {
        Vec::new()
    } else {
        normalize_cover_art_listing(&query, &bytes)?
    };
    let image_count = u16::try_from(entries.len())
        .map_err(|_| Error::Unavailable("too many Cover Art Archive images".into()))?;
    if !listing
        && status == StatusCode::OK
        && !matches!(
            media_type.as_str(),
            "image/png" | "image/jpeg" | "image/webp"
        )
    {
        return Err(Error::Unavailable(
            "Cover Art Archive returned an unsupported image type".into(),
        ));
    }
    let observation = CoverArtArchiveSnapshotObservation {
        http_status: status.as_u16(),
        schema_version: COVER_ART_ARCHIVE_SNAPSHOT_SCHEMA_VERSION,
        query,
        outcome: if status == StatusCode::NOT_FOUND || (listing && entries.is_empty()) {
            MetadataProviderOutcome::NoMatch
        } else {
            MetadataProviderOutcome::Candidates
        },
        image_count,
        media_type: (!listing && status == StatusCode::OK).then_some(media_type.clone()),
    };
    observation.validate()?;
    let response = ProviderResponse {
        media_type: if listing {
            "application/json".into()
        } else {
            media_type
        },
        digest: Digest::sha256(&bytes),
        bytes,
        parsed_observation: RawJson::from_value(
            serde_json::to_value(observation).map_err(|error| Error::Invalid(error.to_string()))?,
        ),
        limitations: Vec::new(),
    };
    response.validate(if listing {
        MAX_JSON_BYTES
    } else {
        MAX_IMAGE_BYTES
    })?;
    Ok(response)
}

#[derive(Deserialize)]
struct MusicBrainzEnvelope {
    releases: Vec<MusicBrainzRelease>,
}

#[derive(Deserialize)]
struct MusicBrainzRelease {
    id: String,
    title: String,
    #[serde(default)]
    disambiguation: Option<String>,
    #[serde(default, rename = "text-representation")]
    text_representation: Option<TextRepresentation>,
    #[serde(default, rename = "artist-credit")]
    artist_credit: Vec<ArtistCredit>,
    #[serde(default)]
    media: Vec<Medium>,
    #[serde(default, rename = "release-group")]
    release_group: Option<ReleaseGroup>,
}

#[derive(Deserialize)]
struct TextRepresentation {
    #[serde(default)]
    language: Option<String>,
    #[serde(default)]
    script: Option<String>,
}

#[derive(Deserialize)]
struct ArtistCredit {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    joinphrase: Option<String>,
    #[serde(default)]
    artist: Option<Artist>,
}

#[derive(Deserialize)]
struct Artist {
    id: String,
    name: String,
}

#[derive(Deserialize)]
struct Medium {
    #[serde(default)]
    position: Option<u16>,
    #[serde(default)]
    discs: Vec<Disc>,
    #[serde(default)]
    tracks: Vec<MusicBrainzTrack>,
}

#[derive(Deserialize)]
struct MusicBrainzTrack {
    position: u16,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    recording: Option<MusicBrainzRecording>,
}

#[derive(Deserialize)]
struct MusicBrainzRecording {
    title: String,
}

#[derive(Deserialize)]
struct Disc {
    id: String,
}

#[derive(Deserialize)]
struct ReleaseGroup {
    id: String,
}

fn normalize_musicbrainz_body(query: &MusicBrainzQuery, bytes: &[u8]) -> Result<Vec<RawJson>> {
    let value: serde_json::Value = serde_json::from_slice(bytes)
        .map_err(|error| Error::Unavailable(format!("malformed MusicBrainz JSON: {error}")))?;
    let releases = if matches!(query, MusicBrainzQuery::ReleaseId { .. }) {
        vec![serde_json::from_value(value).map_err(|error| {
            Error::Unavailable(format!("malformed MusicBrainz release: {error}"))
        })?]
    } else {
        serde_json::from_value::<MusicBrainzEnvelope>(value)
            .map_err(|error| {
                Error::Unavailable(format!("malformed MusicBrainz response: {error}"))
            })?
            .releases
    };
    if releases.len() > 128 {
        return Err(Error::Unavailable(
            "MusicBrainz response contains too many releases".into(),
        ));
    }
    let mut normalized = releases
        .into_iter()
        .map(normalize_musicbrainz_release)
        .collect::<Result<Vec<_>>>()?;
    normalized.sort_by(|left, right| left.get().cmp(right.get()));
    Ok(normalized)
}

fn normalize_musicbrainz_release(release: MusicBrainzRelease) -> Result<RawJson> {
    let mut artist_credit = String::new();
    let mut artist_ids = Vec::new();
    for credit in release.artist_credit {
        if let Some(artist) = &credit.artist {
            artist_ids.push(artist.id.clone());
        }
        let name = credit
            .name
            .or_else(|| credit.artist.as_ref().map(|artist| artist.name.clone()))
            .unwrap_or_default();
        artist_credit.push_str(&name);
        artist_credit.push_str(credit.joinphrase.as_deref().unwrap_or_default());
    }
    artist_ids.sort();
    artist_ids.dedup();
    let media = release
        .media
        .into_iter()
        .enumerate()
        .map(
            |(index, medium)| collector_modules::music::MusicBrainzMediumObservation {
                position: medium.position.unwrap_or((index + 1) as u16),
                disc_ids: medium.discs.into_iter().map(|disc| disc.id).collect(),
                tracks: medium
                    .tracks
                    .into_iter()
                    .filter_map(|track| {
                        track
                            .title
                            .or_else(|| track.recording.map(|value| value.title))
                            .filter(|title| !title.trim().is_empty())
                            .map(
                                |title| collector_modules::music::MusicBrainzTrackObservation {
                                    position: track.position,
                                    title,
                                },
                            )
                    })
                    .collect(),
            },
        )
        .collect::<Vec<_>>();
    let mut disc_ids = media
        .iter()
        .flat_map(|medium| medium.disc_ids.iter().cloned())
        .collect::<Vec<_>>();
    disc_ids.sort();
    disc_ids.dedup();
    let observation = MusicBrainzReleaseObservation {
        release_id: release.id,
        title: release.title,
        disambiguation: release.disambiguation.filter(|value| !value.is_empty()),
        language: release
            .text_representation
            .as_ref()
            .and_then(|value| value.language.clone()),
        script: release.text_representation.and_then(|value| value.script),
        artist_credit: (!artist_credit.is_empty()).then_some(artist_credit),
        artist_ids,
        release_group_id: release.release_group.map(|value| value.id),
        disc_ids,
        media,
    };
    observation.validate()?;
    Ok(RawJson::from_value(
        serde_json::to_value(observation).map_err(|error| Error::Invalid(error.to_string()))?,
    ))
}

#[derive(Deserialize)]
struct CoverArtEnvelope {
    release: String,
    images: Vec<CoverArtImage>,
}

#[derive(Deserialize)]
struct CoverArtImage {
    id: CoverArtImageId,
    #[serde(default)]
    front: bool,
    #[serde(default)]
    back: bool,
    #[serde(default)]
    approved: bool,
    #[serde(default)]
    types: Vec<String>,
    #[serde(default)]
    comment: String,
    #[serde(default, rename = "edit")]
    edit_id: Option<u64>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum CoverArtImageId {
    Text(String),
    Number(u64),
}

impl CoverArtImageId {
    fn into_string(self) -> String {
        match self {
            Self::Text(value) => value,
            Self::Number(value) => value.to_string(),
        }
    }
}

fn normalize_cover_art_listing(query: &CoverArtArchiveQuery, bytes: &[u8]) -> Result<Vec<RawJson>> {
    let release_id = match query {
        CoverArtArchiveQuery::Listing { release_id } => release_id,
        CoverArtArchiveQuery::Image { .. } => {
            return Err(Error::Invalid("cover image is not a JSON listing".into()));
        }
    };
    let envelope: CoverArtEnvelope = serde_json::from_slice(bytes).map_err(|error| {
        Error::Unavailable(format!("malformed Cover Art Archive JSON: {error}"))
    })?;
    let actual_release = envelope
        .release
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or_default();
    if actual_release != release_id {
        return Err(Error::Unavailable(
            "Cover Art Archive response is misdirected".into(),
        ));
    }
    if envelope.images.len() > 128 {
        return Err(Error::Unavailable(
            "Cover Art Archive response contains too many images".into(),
        ));
    }
    let mut entries = envelope
        .images
        .into_iter()
        .map(|image| {
            let observation = CoverArtObservation {
                release_id: release_id.clone(),
                image_id: image.id.into_string(),
                front: image.front,
                back: image.back,
                approved: image.approved,
                types: image.types,
                comment: image.comment,
                edit_id: image.edit_id,
            };
            observation.validate()?;
            Ok(RawJson::from_value(
                serde_json::to_value(observation)
                    .map_err(|error| Error::Invalid(error.to_string()))?,
            ))
        })
        .collect::<Result<Vec<_>>>()?;
    entries.sort_by(|left, right| left.get().cmp(right.get()));
    Ok(entries)
}

fn read_snapshot(
    record: &SnapshotRecord,
    reader: &mut dyn Read,
    cancelled: &mut dyn FnMut() -> Result<bool>,
    maximum: u64,
) -> Result<Vec<u8>> {
    record.validate()?;
    if record.bytes.0 > maximum {
        return Err(Error::Invalid("provider snapshot is oversized".into()));
    }
    let capacity = usize::try_from(record.bytes.0)
        .map_err(|_| Error::Invalid("provider snapshot length does not fit memory".into()))?;
    let mut bytes = Vec::with_capacity(capacity);
    let mut buffer = [0_u8; READ_CHUNK_BYTES];
    loop {
        if cancelled()? {
            return Err(Error::JobCancelled);
        }
        let read = reader
            .read(&mut buffer)
            .map_err(|error| Error::Unavailable(format!("provider snapshot read: {error}")))?;
        if read == 0 {
            break;
        }
        if bytes.len().saturating_add(read) as u64 > record.bytes.0 {
            return Err(Error::Invalid(
                "provider snapshot has trailing bytes".into(),
            ));
        }
        bytes.extend_from_slice(&buffer[..read]);
    }
    if bytes.len() as u64 != record.bytes.0 || Digest::sha256(&bytes) != record.digest {
        return Err(Error::Invalid(
            "provider snapshot length or digest mismatch".into(),
        ));
    }
    Ok(bytes)
}

fn read_bounded(
    response: Response,
    limit: u64,
    execution: &mut dyn ProcessingExecutionContext,
    label: &str,
) -> Result<Vec<u8>> {
    read_bounded_with_progress(
        response,
        limit,
        &mut |completed| {
            checkpoint(execution)?;
            execution.report_progress("downloading_provider_response", completed, limit)
        },
        label,
    )
}

fn read_bounded_with_progress(
    mut response: Response,
    limit: u64,
    progress: &mut dyn FnMut(u64) -> Result<()>,
    label: &str,
) -> Result<Vec<u8>> {
    if response
        .content_length()
        .is_some_and(|length| length > limit)
    {
        return Err(Error::Unavailable(format!(
            "{label} response exceeds its body limit"
        )));
    }
    let mut bytes = Vec::with_capacity(
        response
            .content_length()
            .unwrap_or_default()
            .min(limit)
            .try_into()
            .unwrap_or_default(),
    );
    let mut buffer = [0_u8; READ_CHUNK_BYTES];
    loop {
        progress(bytes.len() as u64)?;
        let read = response
            .read(&mut buffer)
            .map_err(|error| Error::Unavailable(format!("{label} response read: {error}")))?;
        if read == 0 {
            break;
        }
        if bytes.len().saturating_add(read) as u64 > limit {
            return Err(Error::Unavailable(format!(
                "{label} response exceeds its body limit"
            )));
        }
        bytes.extend_from_slice(&buffer[..read]);
        progress(bytes.len() as u64)?;
    }
    Ok(bytes)
}

/// Validates precisely the addresses handed to the HTTP connector. This avoids
/// a preflight DNS check followed by a second, potentially different resolution.
struct PublicDns;
impl reqwest::dns::Resolve for PublicDns {
    fn resolve(&self, name: reqwest::dns::Name) -> reqwest::dns::Resolving {
        let host = name.as_str().to_owned();
        Box::pin(async move {
            let addresses = tokio::net::lookup_host((host, 0))
                .await?
                .take(65)
                .collect::<Vec<_>>();
            if addresses.is_empty()
                || addresses.len() > 64
                || addresses
                    .iter()
                    .any(|address| forbidden_destination(address.ip()))
            {
                return Err(
                    std::io::Error::other("provider DNS returned a forbidden address").into(),
                );
            }
            Ok(Box::new(addresses.into_iter()) as reqwest::dns::Addrs)
        })
    }
}

fn build_client(user_agent: &str, timeout: Duration, label: &str) -> Result<Client> {
    let user_agent = user_agent.to_owned();
    let label = label.to_owned();
    thread::spawn(move || {
        Client::builder()
            .user_agent(user_agent)
            .timeout(timeout)
            .no_proxy()
            .dns_resolver(Arc::new(PublicDns))
            .redirect(reqwest::redirect::Policy::none())
            .build()
    })
    .join()
    .map_err(|_| Error::Unavailable(format!("{label} client initialization panicked")))?
    .map_err(|error| Error::Unavailable(format!("{label} client: {error}")))
}

fn validate_http_configuration(user_agent: &str, timeout: Duration) -> Result<()> {
    let meaningful_contact = user_agent
        .split_once('(')
        .and_then(|(product, contact)| {
            let contact = contact.strip_suffix(')')?.trim();
            let (name, version) = product.trim().split_once('/')?;
            Some(
                !name.is_empty()
                    && !version.trim().is_empty()
                    && (contact.contains('@')
                        || contact.starts_with("https://")
                        || contact.starts_with("http://")),
            )
        })
        .unwrap_or(false);
    if user_agent.trim().is_empty()
        || user_agent.len() > 255
        || user_agent.chars().any(char::is_control)
        || !meaningful_contact
        || timeout.is_zero()
        || timeout > DEFAULT_TIMEOUT
    {
        return Err(Error::Invalid(
            "provider User-Agent must name a version and contact, with a timeout up to 15 seconds"
                .into(),
        ));
    }
    Ok(())
}

fn test_origin(value: &str) -> Result<Url> {
    let origin = Url::parse(value)
        .map_err(|error| Error::Invalid(format!("invalid test provider origin: {error}")))?;
    let loopback = origin
        .host_str()
        .and_then(|host| host.trim_matches(['[', ']']).parse::<IpAddr>().ok())
        .is_some_and(|address| address.is_loopback());
    if origin.scheme() != "http"
        || !loopback
        || !origin.username().is_empty()
        || origin.password().is_some()
        || origin.path() != "/"
        || origin.query().is_some()
        || origin.fragment().is_some()
    {
        return Err(Error::Invalid(
            "test provider origin must be an uncredentialed loopback HTTP origin".into(),
        ));
    }
    Ok(origin)
}

fn same_origin(left: &Url, right: &Url) -> bool {
    left.scheme() == right.scheme()
        && left.host_str() == right.host_str()
        && left.port_or_known_default() == right.port_or_known_default()
}

fn forbidden_destination(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(value) => {
            value.is_private()
                || (value.octets()[0] == 100 && (64..=127).contains(&value.octets()[1]))
                || value.is_broadcast()
                || value.is_documentation()
                || value.octets()[0] >= 240
                || value.is_loopback()
                || value.is_link_local()
                || value.is_multicast()
                || value.is_unspecified()
                || value.octets()[0] == 0
        }
        IpAddr::V6(value) => {
            value
                .to_ipv4_mapped()
                .is_some_and(|address| forbidden_destination(IpAddr::V4(address)))
                || value.is_loopback()
                || value.is_unspecified()
                || value.is_multicast()
                || (value.segments()[0] & 0xfe00) == 0xfc00
                || (value.segments()[0] & 0xffc0) == 0xfe80
        }
    }
}

fn content_type(response: &Response) -> String {
    response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
}

fn checkpoint(execution: &dyn ProcessingExecutionContext) -> Result<()> {
    if execution.cancellation_requested()? {
        Err(Error::JobCancelled)
    } else {
        Ok(())
    }
}

fn wait_cancellable(delay: Duration, execution: &dyn ProcessingExecutionContext) -> Result<()> {
    let deadline = Instant::now() + delay;
    loop {
        checkpoint(execution)?;
        let now = Instant::now();
        if now >= deadline {
            return Ok(());
        }
        thread::sleep((deadline - now).min(Duration::from_millis(50)));
    }
}

fn retry_after(headers: &reqwest::header::HeaderMap) -> Option<Duration> {
    let value = headers.get(RETRY_AFTER)?.to_str().ok()?;
    if let Ok(seconds) = value.parse::<u64>() {
        return Some(Duration::from_secs(seconds));
    }
    let date = httpdate::parse_http_date(value).ok()?;
    Some(
        date.duration_since(SystemTime::now())
            .unwrap_or(Duration::ZERO),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn musicbrainz_tracks_remain_bound_to_their_medium() {
        let query = MusicBrainzQuery::ReleaseId {
            release_id: "00000000-0000-0000-0000-000000000001".into(),
            refresh: false,
        };
        let body = br#"{"id":"00000000-0000-0000-0000-000000000001","title":"Album","media":[{"position":1,"discs":[{"id":"disc-one"}],"tracks":[{"position":1,"title":"First"}]},{"position":2,"discs":[{"id":"disc-two"}],"tracks":[{"position":1,"title":"Second"}]}]}"#;
        let releases = normalize_musicbrainz_body(&query, body).unwrap();
        let release: MusicBrainzReleaseObservation = releases[0].parse().unwrap();
        assert_eq!(release.media.len(), 2);
        assert_eq!(release.media[0].disc_ids, ["disc-one"]);
        assert_eq!(release.media[0].tracks[0].title, "First");
        assert_eq!(release.media[1].disc_ids, ["disc-two"]);
        assert_eq!(release.media[1].tracks[0].title, "Second");
    }

    #[test]
    fn production_destinations_include_archive_storage_but_reject_private_addresses() {
        let policy = OriginPolicy {
            allowed: vec![Url::parse("https://archive.org").unwrap()],
            allow_private_for_tests: false,
        };
        for url in [
            "https://s3.us.archive.org/x",
            "https://ia800123.us.archive.org/x",
            "https://dn721500.ca.archive.org/x",
        ] {
            assert!(policy.allows(&Url::parse(url).unwrap()));
        }
        for url in [
            "https://ia800123.us.archive.org.evil.test/x",
            "https://dn721500.ca.archive.org.evil.test/x",
            "https://dn721500.us.archive.org/x",
            "https://ia800123.ca.archive.org/x",
            "https://evil.archive.org/x",
            "http://archive.org/x",
            "https://user@archive.org/x",
        ] {
            assert!(!policy.allows(&Url::parse(url).unwrap()));
        }
        for address in [
            "127.0.0.1",
            "100.64.0.1",
            "169.254.169.254",
            "10.0.0.1",
            "::ffff:127.0.0.1",
            "fc00::1",
            "fe80::1",
        ] {
            assert!(forbidden_destination(address.parse().unwrap()), "{address}");
        }
        assert!(!forbidden_destination("8.8.8.8".parse().unwrap()));
    }
}
