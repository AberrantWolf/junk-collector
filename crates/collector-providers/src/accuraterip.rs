use collector_app::{
    CatalogParserPort, ParsedCatalogSnapshot, ProcessingExecutionContext, ProviderPort,
    ProviderRequest, ProviderResponse, SnapshotRecord,
};
use collector_core::{Digest, Error, RawJson, Result};
use reqwest::{
    StatusCode, Url,
    blocking::{Client, Response},
    header::{LOCATION, RETRY_AFTER},
};
use std::{
    io::Read,
    net::IpAddr,
    sync::Mutex,
    thread,
    time::{Duration, Instant, SystemTime},
};

pub const ACCURATERIP_PROVIDER_ID: &str = "org.junk.providers.accuraterip";
pub const ACCURATERIP_ACQUIRE_OPERATION: &str = "org.junk.providers.accuraterip.acquire";
pub const MAX_DBAR_BYTES: u64 = 8 * 1024 * 1024;
const MAX_ATTEMPTS: usize = 3;
const MAX_REDIRECTS: usize = 3;
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(15);
const DEFAULT_RETRY_DELAY: Duration = Duration::from_secs(1);
const DEFAULT_MINIMUM_INTERVAL: Duration = Duration::from_secs(1);
const MAX_SERVER_RETRY_DELAY: Duration = Duration::from_secs(60);
const READ_CHUNK_BYTES: usize = 64 * 1024;
const DBAR_MEDIA_TYPE: &str = "application/vnd.accuraterip.dbar";

pub use collector_modules::music::catalog::{
    DBAR_SNAPSHOT_SCHEMA_VERSION, DbarOutcome, DbarQuery, DbarSnapshotObservation,
    ParsedDbarResponse, ParsedDbarSnapshot, ParsedDbarTrack,
};

#[derive(Clone)]
struct EndpointPolicy {
    origin: Url,
}

impl EndpointPolicy {
    fn request_url(&self, query: &DbarQuery) -> Result<Url> {
        query.validate()?;
        let id1 = format!("{:08x}", query.accuraterip_id1);
        let id2 = format!("{:08x}", query.accuraterip_id2);
        let cddb = format!("{:08x}", query.cddb_id);
        let mut reverse = id1.chars().rev();
        let a = reverse.next().expect("eight hexadecimal digits");
        let b = reverse.next().expect("eight hexadecimal digits");
        let c = reverse.next().expect("eight hexadecimal digits");
        self.origin
            .join(&format!(
                "/accuraterip/{a}/{b}/{c}/dBAR-{:03}-{id1}-{id2}-{cddb}.bin",
                query.track_count
            ))
            .map_err(|error| Error::Invalid(format!("invalid AccurateRip URL: {error}")))
    }

    fn allows(&self, url: &Url) -> bool {
        url.scheme() == self.origin.scheme()
            && url.host_str() == self.origin.host_str()
            && url.port_or_known_default() == self.origin.port_or_known_default()
            && url.username().is_empty()
            && url.password().is_none()
    }
}

pub struct AccurateRipProvider {
    client: Option<Client>,
    endpoint: EndpointPolicy,
    minimum_interval: Duration,
    next_request: Mutex<Option<Instant>>,
}

impl AccurateRipProvider {
    pub fn new(user_agent: &str) -> Result<Self> {
        let origin = Url::parse("https://www.accuraterip.com")
            .map_err(|error| Error::Invalid(error.to_string()))?;
        Self::build(
            user_agent,
            origin,
            DEFAULT_TIMEOUT,
            DEFAULT_MINIMUM_INTERVAL,
        )
    }

    /// Local-loopback composition for deterministic HTTP fixtures. Production
    /// composition uses [`Self::new`] and cannot replace the pinned HTTPS origin.
    #[doc(hidden)]
    pub fn with_test_origin(user_agent: &str, origin: &str, timeout: Duration) -> Result<Self> {
        let origin = Url::parse(origin)
            .map_err(|error| Error::Invalid(format!("invalid test origin: {error}")))?;
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
                "test AccurateRip origin must be an uncredentialed loopback HTTP origin".into(),
            ));
        }
        Self::build(user_agent, origin, timeout, Duration::ZERO)
    }

    fn build(
        user_agent: &str,
        origin: Url,
        timeout: Duration,
        minimum_interval: Duration,
    ) -> Result<Self> {
        if user_agent.trim().is_empty()
            || user_agent.len() > 255
            || user_agent.chars().any(char::is_control)
            || timeout.is_zero()
            || timeout > DEFAULT_TIMEOUT
        {
            return Err(Error::Invalid(
                "invalid AccurateRip client configuration".into(),
            ));
        }
        let user_agent = user_agent.to_owned();
        let client = std::thread::spawn(move || {
            Client::builder()
                .user_agent(user_agent)
                .timeout(timeout)
                .redirect(reqwest::redirect::Policy::none())
                .build()
        })
        .join()
        .map_err(|_| Error::Unavailable("AccurateRip client initialization panicked".into()))?
        .map_err(|error| Error::Unavailable(format!("AccurateRip client: {error}")))?;
        Ok(Self {
            client: Some(client),
            endpoint: EndpointPolicy { origin },
            minimum_interval,
            next_request: Mutex::new(None),
        })
    }

    fn acquire_query(
        &self,
        query: DbarQuery,
        limit: u64,
        execution: &mut dyn ProcessingExecutionContext,
    ) -> Result<ProviderResponse> {
        let initial_url = self.endpoint.request_url(&query)?;
        let limit = limit.min(MAX_DBAR_BYTES);
        let mut last_error = None;
        for attempt in 0..MAX_ATTEMPTS {
            checkpoint(execution)?;
            let response =
                match self.send_following_allowed_redirects(initial_url.clone(), execution) {
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
            if status == StatusCode::OK || status == StatusCode::NOT_FOUND {
                let bytes = read_bounded(response, limit, execution)?;
                return frozen_response(query, status, bytes);
            }
            if status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error() {
                let retry_after = retry_after(response.headers()).unwrap_or(DEFAULT_RETRY_DELAY);
                let _ = read_bounded(response, limit, execution)?;
                last_error = Some(Error::Unavailable(format!(
                    "AccurateRip provider returned HTTP {status}"
                )));
                if attempt + 1 < MAX_ATTEMPTS {
                    if retry_after > MAX_SERVER_RETRY_DELAY {
                        return Err(Error::RateLimited(retry_after.as_secs()));
                    }
                    wait_cancellable(retry_after, execution)?;
                    continue;
                }
                break;
            }
            let _ = read_bounded(response, limit, execution)?;
            return Err(Error::Unavailable(format!(
                "AccurateRip provider returned HTTP {status}"
            )));
        }
        Err(last_error.unwrap_or_else(|| Error::Unavailable("AccurateRip provider outage".into())))
    }

    fn wait_for_rate_limit(&self, execution: &mut dyn ProcessingExecutionContext) -> Result<()> {
        let delay = {
            let mut next = self
                .next_request
                .lock()
                .map_err(|_| Error::Unavailable("AccurateRip limiter lock failed".into()))?;
            let now = Instant::now();
            let scheduled = next.map_or(now, |instant| instant.max(now));
            let delay = scheduled
                .checked_duration_since(now)
                .unwrap_or(Duration::ZERO);
            *next = Some(scheduled + self.minimum_interval);
            delay
        };
        wait_cancellable(delay, execution)
    }

    fn send_following_allowed_redirects(
        &self,
        mut url: Url,
        execution: &mut dyn ProcessingExecutionContext,
    ) -> Result<Response> {
        for redirects in 0..=MAX_REDIRECTS {
            checkpoint(execution)?;
            if !self.endpoint.allows(&url) {
                return Err(Error::Unavailable(
                    "AccurateRip redirect left the pinned provider origin".into(),
                ));
            }
            self.wait_for_rate_limit(execution)?;
            let response = self
                .client
                .as_ref()
                .expect("AccurateRip client exists until provider drop")
                .get(url.clone())
                .send()
                .map_err(|error| Error::Unavailable(format!("AccurateRip request: {error}")))?;
            if !response.status().is_redirection() {
                return Ok(response);
            }
            if redirects == MAX_REDIRECTS {
                return Err(Error::Unavailable(
                    "AccurateRip redirect limit exceeded".into(),
                ));
            }
            let location = response
                .headers()
                .get(LOCATION)
                .and_then(|value| value.to_str().ok())
                .ok_or_else(|| Error::Unavailable("AccurateRip redirect has no location".into()))?;
            url = url
                .join(location)
                .map_err(|_| Error::Unavailable("invalid AccurateRip redirect".into()))?;
        }
        unreachable!("redirect loop is bounded")
    }
}

impl Drop for AccurateRipProvider {
    fn drop(&mut self) {
        // reqwest's blocking client owns a helper runtime that cannot be dropped
        // from an async worker. Server composition is async, so release the last
        // handle on a plain thread during orderly shutdown.
        if let Some(client) = self.client.take() {
            let _ = std::thread::spawn(move || drop(client)).join();
        }
    }
}

impl ProviderPort for AccurateRipProvider {
    fn acquire(
        &self,
        request: &ProviderRequest,
        execution: &mut dyn ProcessingExecutionContext,
    ) -> Result<ProviderResponse> {
        request.validate()?;
        if request.provider.0 != ACCURATERIP_PROVIDER_ID
            || request.operation.as_str() != ACCURATERIP_ACQUIRE_OPERATION
            || request.maximum_response_bytes > MAX_DBAR_BYTES
        {
            return Err(Error::Invalid(
                "unsupported or oversized AccurateRip provider request".into(),
            ));
        }
        let query: DbarQuery = request.query.parse()?;
        query.validate()?;
        self.acquire_query(query, request.maximum_response_bytes, execution)
    }
}

impl CatalogParserPort for AccurateRipProvider {
    fn parse_snapshot(
        &self,
        record: &SnapshotRecord,
        reader: &mut dyn Read,
        cancelled: &mut dyn FnMut() -> Result<bool>,
    ) -> Result<ParsedCatalogSnapshot> {
        let dbar = parse_dbar_snapshot(record, reader, cancelled)?.unwrap_or_default();
        let parsed = ParsedDbarSnapshot {
            responses: dbar
                .responses
                .into_iter()
                .map(|response| ParsedDbarResponse {
                    track_count: response.track_count,
                    accuraterip_id1: response.ar_id1,
                    accuraterip_id2: response.ar_id2,
                    cddb_id: response.cddb_id,
                    tracks: response
                        .tracks
                        .into_iter()
                        .map(|track| ParsedDbarTrack {
                            confidence: track.confidence,
                            checksum: track.checksum,
                            checksum_450: track.checksum_450,
                        })
                        .collect(),
                })
                .collect(),
        };
        let result = ParsedCatalogSnapshot {
            observation: record.parsed_observation.clone(),
            // One envelope is retained even for a checked 404/no-data snapshot.
            entries: vec![RawJson::from_value(
                serde_json::to_value(parsed).map_err(|error| Error::Invalid(error.to_string()))?,
            )],
        };
        result.validate()?;
        Ok(result)
    }
}

pub fn parse_frozen_dbar(
    response: &ProviderResponse,
) -> Result<Option<junk_libs_accuraterip::DbarFile>> {
    response.validate(MAX_DBAR_BYTES)?;
    if response.media_type != DBAR_MEDIA_TYPE {
        return Err(Error::Invalid(
            "frozen AccurateRip response has the wrong media type".into(),
        ));
    }
    let observation: DbarSnapshotObservation = response.parsed_observation.parse()?;
    observation.validate()?;
    match observation.outcome {
        DbarOutcome::NoData => Ok(None),
        DbarOutcome::Data => {
            let dbar = junk_libs_accuraterip::DbarFile::parse(&response.bytes)
                .map_err(|error| Error::Unavailable(format!("malformed dBAR response: {error}")))?;
            dbar.validate_request(
                observation.query.track_count,
                observation.query.accuraterip_id1,
                observation.query.accuraterip_id2,
                observation.query.cddb_id,
            )
            .map_err(|error| Error::Unavailable(format!("misdirected dBAR response: {error}")))?;
            if dbar.responses.len() != observation.response_count as usize {
                return Err(Error::Invalid(
                    "dBAR response count does not match its frozen observation".into(),
                ));
            }
            Ok(Some(dbar))
        }
    }
}

/// Reconstruct a first-class offline dBAR input from the exact archived snapshot
/// bytes. Metadata alone is never enough to attribute results to a SnapshotId.
pub fn parse_dbar_snapshot(
    record: &SnapshotRecord,
    reader: &mut dyn Read,
    cancelled: &mut dyn FnMut() -> Result<bool>,
) -> Result<Option<junk_libs_accuraterip::DbarFile>> {
    record.validate()?;
    if record.provider.0 != ACCURATERIP_PROVIDER_ID
        || record.media_type != DBAR_MEDIA_TYPE
        || record.bytes.0 > MAX_DBAR_BYTES
    {
        return Err(Error::Invalid(
            "snapshot is not a bounded AccurateRip dBAR response".into(),
        ));
    }
    let capacity = usize::try_from(record.bytes.0)
        .map_err(|_| Error::Invalid("dBAR snapshot length does not fit memory".into()))?;
    let mut bytes = Vec::with_capacity(capacity);
    let mut buffer = [0_u8; READ_CHUNK_BYTES];
    loop {
        if cancelled()? {
            return Err(Error::JobCancelled);
        }
        let read = reader
            .read(&mut buffer)
            .map_err(|error| Error::Unavailable(format!("dBAR snapshot read: {error}")))?;
        if read == 0 {
            break;
        }
        if bytes.len().saturating_add(read) as u64 > record.bytes.0 {
            return Err(Error::Invalid(
                "dBAR snapshot contains trailing bytes".into(),
            ));
        }
        bytes.extend_from_slice(&buffer[..read]);
    }
    if bytes.len() as u64 != record.bytes.0 || Digest::sha256(&bytes) != record.digest {
        return Err(Error::Invalid(
            "dBAR snapshot length or digest mismatch".into(),
        ));
    }
    parse_frozen_dbar(&ProviderResponse {
        media_type: record.media_type.clone(),
        bytes,
        digest: record.digest.clone(),
        parsed_observation: record.parsed_observation.clone(),
        limitations: Vec::new(),
    })
}

fn frozen_response(
    query: DbarQuery,
    status: StatusCode,
    bytes: Vec<u8>,
) -> Result<ProviderResponse> {
    let (outcome, response_count) = if status == StatusCode::NOT_FOUND {
        (DbarOutcome::NoData, 0)
    } else {
        let dbar = junk_libs_accuraterip::DbarFile::parse(&bytes)
            .map_err(|error| Error::Unavailable(format!("malformed dBAR response: {error}")))?;
        dbar.validate_request(
            query.track_count,
            query.accuraterip_id1,
            query.accuraterip_id2,
            query.cddb_id,
        )
        .map_err(|error| Error::Unavailable(format!("misdirected dBAR response: {error}")))?;
        if dbar.responses.is_empty() {
            return Err(Error::Unavailable(
                "empty successful dBAR response is malformed".into(),
            ));
        }
        (
            DbarOutcome::Data,
            u32::try_from(dbar.responses.len())
                .map_err(|_| Error::Unavailable("too many dBAR responses".into()))?,
        )
    };
    let observation = DbarSnapshotObservation {
        schema_version: DBAR_SNAPSHOT_SCHEMA_VERSION,
        query,
        outcome,
        response_count,
    };
    observation.validate()?;
    let response = ProviderResponse {
        media_type: DBAR_MEDIA_TYPE.into(),
        digest: Digest::sha256(&bytes),
        bytes,
        parsed_observation: RawJson::from_value(
            serde_json::to_value(observation).map_err(|error| Error::Invalid(error.to_string()))?,
        ),
        limitations: Vec::new(),
    };
    response.validate(MAX_DBAR_BYTES)?;
    Ok(response)
}

fn read_bounded(
    mut response: Response,
    limit: u64,
    execution: &mut dyn ProcessingExecutionContext,
) -> Result<Vec<u8>> {
    if response
        .content_length()
        .is_some_and(|length| length > limit)
    {
        return Err(Error::Unavailable(
            "AccurateRip response exceeds its body limit".into(),
        ));
    }
    let capacity = response
        .content_length()
        .unwrap_or(0)
        .min(limit)
        .try_into()
        .unwrap_or(0);
    let mut bytes = Vec::with_capacity(capacity);
    let mut buffer = [0_u8; READ_CHUNK_BYTES];
    loop {
        checkpoint(execution)?;
        let read = response
            .read(&mut buffer)
            .map_err(|error| Error::Unavailable(format!("AccurateRip response read: {error}")))?;
        if read == 0 {
            break;
        }
        if bytes.len().saturating_add(read) as u64 > limit {
            return Err(Error::Unavailable(
                "AccurateRip response exceeds its body limit".into(),
            ));
        }
        bytes.extend_from_slice(&buffer[..read]);
        execution.report_progress("downloading_provider_response", bytes.len() as u64, limit)?;
    }
    Ok(bytes)
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
