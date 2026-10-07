//! UI-independent connection trust, endpoint policy and offline cache planning.
mod library;
mod media;
pub use collector_api_types;
pub use collector_core;
use collector_core::*;
pub use media::{MediaCancellation, RangeReader};
use serde::{Deserialize, Serialize};
use url::Url;
pub use url::Url as ServerUrl;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ConnectionKey {
    pub server_id: ServerId,
    pub user_id: UserId,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct EnrolledCredential {
    #[serde(default)]
    pub user_id: Option<UserId>,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub display_name: String,
    pub server_id: ServerId,
    pub credential_id: NativeCredentialId,
    pub credential: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub pairing: Pairing,
    #[serde(default)]
    pub trust: ConnectionTrust,
}

#[derive(Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionTrust {
    #[default]
    PinnedCertificate,
    SystemHttps,
}

/// Normal HTTPS trust, including locally installed system roots. Never follows redirects.
pub fn system_https_client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .https_only(true)
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(std::time::Duration::from_secs(5))
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(client_error)
}

/// Platform clients implement this port with Keychain, Credential Manager, or
/// an equivalent OS-protected secret store. Core client code never chooses a
/// filesystem location for bearer credentials.
pub trait NativeCredentialStorage: Send + Sync {
    fn save(&self, credential: &EnrolledCredential) -> Result<()>;
    fn load(&self, key: ConnectionKey) -> Result<Option<EnrolledCredential>>;
    fn remove(&self, key: ConnectionKey) -> Result<()>;
    fn load_legacy(&self, server: ServerId) -> Result<Option<EnrolledCredential>>;
    fn remove_legacy(&self, server: ServerId) -> Result<()>;
}

pub use collector_api_types::{EnrollmentPhase, EnrollmentStart, EnrollmentView};

#[derive(Clone, Serialize, Deserialize)]
pub struct PendingEnrollment {
    base_url: Url,
    expected_server: ServerId,
    pub start: EnrollmentStart,
    trust: ConnectionTrust,
    #[serde(default)]
    pinned_pairing: Option<Pairing>,
}

#[derive(Deserialize)]
struct Identity {
    server_id: ServerId,
}

impl PendingEnrollment {
    pub fn server_id(&self) -> ServerId {
        self.expected_server
    }
    pub async fn start_system_https(
        client: &reqwest::Client,
        base_url: Url,
        device_name: &str,
    ) -> Result<Self> {
        let mut pending = Self::start(client, base_url, device_name).await?;
        pending.trust = ConnectionTrust::SystemHttps;
        Ok(pending)
    }
    pub async fn start(client: &reqwest::Client, base_url: Url, device_name: &str) -> Result<Self> {
        if base_url.scheme() != "https"
            || base_url.host_str().is_none()
            || !base_url.username().is_empty()
            || base_url.password().is_some()
            || base_url.query().is_some()
            || base_url.fragment().is_some()
            || base_url.path() != "/"
        {
            return Err(Error::Invalid(
                "enrollment requires an HTTPS server URL".into(),
            ));
        }
        let identity: Identity = library::decode_request(
            client.get(base_url.join("api/v1/identity").map_err(invalid_url)?),
        )
        .await?;
        let mut start: EnrollmentStart = library::decode_request(
            client
                .post(
                    base_url
                        .join("api/v1/enrollments/start")
                        .map_err(invalid_url)?,
                )
                .json(&collector_api_types::StartEnrollment {
                    device_name: device_name.into(),
                }),
        )
        .await?;
        let mut verification = base_url
            .join(&start.verification_url)
            .map_err(invalid_url)?;
        if verification.origin() != base_url.origin() {
            return Err(Error::Invalid(
                "Enrollment approval must stay on the trusted server".into(),
            ));
        }
        verification
            .query_pairs_mut()
            .append_pair("code", &start.user_code);
        start.verification_url = verification.to_string();
        Ok(Self {
            base_url,
            expected_server: identity.server_id,
            start,
            trust: ConnectionTrust::PinnedCertificate,
            pinned_pairing: None,
        })
    }

    pub fn pin(&mut self, mut pairing: Pairing) -> Result<()> {
        pairing.server_id = self.expected_server;
        pairing.lan_client()?;
        self.pinned_pairing = Some(pairing);
        Ok(())
    }
    pub fn client(&self) -> Result<reqwest::Client> {
        match self.trust {
            ConnectionTrust::SystemHttps => system_https_client(),
            ConnectionTrust::PinnedCertificate => self
                .pinned_pairing
                .as_ref()
                .ok_or_else(|| Error::Invalid("owner-provided certificate missing".into()))?
                .lan_client(),
        }
    }
    pub async fn status(&self, client: &reqwest::Client, cancel: bool) -> Result<EnrollmentView> {
        let action = if cancel { "cancel" } else { "status" };
        library::decode_request(
            client
                .post(
                    self.base_url
                        .join(&format!("api/v1/enrollments/{}/{action}", self.start.id))
                        .map_err(invalid_url)?,
                )
                .json(&collector_api_types::CompleteEnrollment {
                    completion_secret: self.start.completion_secret.clone(),
                }),
        )
        .await
    }
    pub async fn complete(self, client: &reqwest::Client) -> Result<EnrolledCredential> {
        let completed: collector_api_types::EnrollmentCompletion<Pairing> =
            library::decode_request(
                client
                    .post(
                        self.base_url
                            .join(&format!("api/v1/enrollments/{}/complete", self.start.id))
                            .map_err(invalid_url)?,
                    )
                    .json(&collector_api_types::CompleteEnrollment {
                        completion_secret: self.start.completion_secret.clone(),
                    }),
            )
            .await?;
        if completed.pairing.server_id != self.expected_server {
            return Err(Error::Invalid(
                "server identity changed during enrollment".into(),
            ));
        }
        if matches!(self.trust, ConnectionTrust::PinnedCertificate) {
            completed.pairing.lan_client()?;
            if self
                .pinned_pairing
                .as_ref()
                .is_some_and(|p| p.certificate_sha256 != completed.pairing.certificate_sha256)
            {
                return Err(Error::Invalid("Server certificate changed during enrollment; obtain fresh owner-provided trust".into()));
            }
        }
        let mut pairing = completed.pairing;
        // Enrollment authenticated this origin. Keep it first: a private curation
        // listener must not silently turn into the advertised public listener.
        let route = pairing
            .endpoints
            .iter()
            .find(|v| v.url.origin() == self.base_url.origin())
            .map_or(Route::Lan, |v| v.route);
        pairing
            .endpoints
            .retain(|v| v.url.origin() != self.base_url.origin());
        pairing.endpoints.insert(
            0,
            Endpoint {
                url: self.base_url.clone(),
                route,
                latency_ms: None,
                healthy: true,
            },
        );
        // Normal HTTPS enrollment authorizes this origin only. Advertised LAN
        // routes may use a different certificate and must not receive its token.
        if matches!(self.trust, ConnectionTrust::SystemHttps) {
            pairing.endpoints.truncate(1);
        }
        let enrolled = EnrolledCredential {
            user_id: Some(completed.issue.user_id),
            username: completed.issue.username,
            display_name: completed.issue.display_name,
            server_id: pairing.server_id,
            credential_id: completed.issue.device_id,
            credential: completed.issue.credential,
            expires_at: completed.issue.expires_at,
            pairing,
            trust: self.trust,
        };
        Ok(enrolled)
    }
}

#[derive(Clone)]
pub struct EnrolledClient {
    client: reqwest::Client,
    enrolled: EnrolledCredential,
}

impl EnrolledClient {
    pub fn from_storage(storage: &dyn NativeCredentialStorage, key: ConnectionKey) -> Result<Self> {
        let enrolled = storage.load(key)?.ok_or(Error::Unauthenticated)?;
        if enrolled.user_id != Some(key.user_id) {
            return Err(Error::Invalid(
                "stored credential belongs to another account".into(),
            ));
        }
        Self::from_credential(enrolled, key.server_id)
    }
    /// Used only for the restartable conversion of actual server-keyed credentials.
    pub fn from_legacy_storage(
        storage: &dyn NativeCredentialStorage,
        server: ServerId,
    ) -> Result<Self> {
        Self::from_credential(
            storage.load_legacy(server)?.ok_or(Error::Unauthenticated)?,
            server,
        )
    }
    pub fn credential(&self) -> &EnrolledCredential {
        &self.enrolled
    }
    fn from_credential(enrolled: EnrolledCredential, server: ServerId) -> Result<Self> {
        if enrolled.server_id != server || enrolled.pairing.server_id != server {
            return Err(Error::Invalid(
                "stored credential belongs to another server".into(),
            ));
        }
        if enrolled.expires_at <= chrono::Utc::now() {
            return Err(Error::CredentialRevoked);
        }
        let client = match enrolled.trust {
            ConnectionTrust::PinnedCertificate => enrolled.pairing.lan_client()?,
            ConnectionTrust::SystemHttps => system_https_client()?,
        };
        Ok(Self { client, enrolled })
    }

    pub fn get(&self, url: Url) -> Result<reqwest::RequestBuilder> {
        self.request(reqwest::Method::GET, url)
    }

    pub fn request(&self, method: reqwest::Method, url: Url) -> Result<reqwest::RequestBuilder> {
        if self.enrolled.expires_at <= chrono::Utc::now() {
            return Err(Error::CredentialRevoked);
        }
        if url.scheme() != "https" || !url.username().is_empty() || url.password().is_some() {
            return Err(Error::Invalid(
                "endpoint URL cannot contain credentials".into(),
            ));
        }

        let allowed = self.enrolled.pairing.endpoints.iter().any(|endpoint| {
            endpoint.healthy
                && endpoint.url.scheme() == url.scheme()
                && endpoint.url.host_str() == url.host_str()
                && endpoint.url.port_or_known_default() == url.port_or_known_default()
        });
        if !allowed {
            return Err(Error::Invalid(
                "credentials may only be sent to an enrolled server endpoint".into(),
            ));
        }
        Ok(self
            .client
            .request(method, url)
            .bearer_auth(&self.enrolled.credential))
    }
}

fn invalid_url(error: url::ParseError) -> Error {
    Error::Invalid(error.to_string())
}

fn client_error(error: reqwest::Error) -> Error {
    if error.is_connect() {
        Error::Unavailable("Could not establish an HTTPS connection to Collector. Check that the address resolves, the server is running, and its certificate is trusted. For a local self-signed certificate, see Phono’s Self-signed certificate help.".into())
    } else if error.is_timeout() {
        Error::Unavailable(
            "Collector did not respond in time. Check the connection and retry.".into(),
        )
    } else if error.status() == Some(reqwest::StatusCode::UNAUTHORIZED) {
        Error::Unauthenticated
    } else if error.status() == Some(reqwest::StatusCode::FORBIDDEN) {
        Error::Forbidden
    } else if error.status() == Some(reqwest::StatusCode::CONFLICT) {
        Error::Conflict
    } else if error.status() == Some(reqwest::StatusCode::BAD_REQUEST)
        || error.status() == Some(reqwest::StatusCode::UNPROCESSABLE_ENTITY)
    {
        Error::Invalid("Collector rejected the submitted command".into())
    } else if error.status() == Some(reqwest::StatusCode::NOT_FOUND) {
        Error::NotFound
    } else {
        Error::Unavailable("Collector request failed".into())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Pairing {
    pub server_id: ServerId,
    pub certificate_pem: String,
    pub certificate_sha256: String,
    pub endpoints: Vec<Endpoint>,
}
#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Endpoint {
    #[schema(value_type = String)]
    pub url: Url,
    pub route: Route,
    pub latency_ms: Option<u32>,
    pub healthy: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Route {
    Lan,
    Tailnet,
    Public,
}
impl Pairing {
    /// Trust must arrive through authenticated enrollment or out-of-band fingerprint verification.
    pub fn lan_client(&self) -> Result<reqwest::Client> {
        if Digest::sha256(self.certificate_pem.as_bytes()).hex != self.certificate_sha256 {
            return Err(Error::Invalid(
                "pairing certificate fingerprint mismatch".into(),
            ));
        }
        let certificate = reqwest::Certificate::from_pem(self.certificate_pem.as_bytes())
            .map_err(|e| Error::Invalid(e.to_string()))?;
        reqwest::Client::builder()
            .https_only(true)
            .tls_built_in_root_certs(false)
            .add_root_certificate(certificate)
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(std::time::Duration::from_secs(5))
            .timeout(std::time::Duration::from_secs(20))
            .build()
            .map_err(|e| Error::Unavailable(e.to_string()))
    }
    pub fn best_endpoint(&self, current: Option<&Url>) -> Option<&Endpoint> {
        let rank = |e: &Endpoint| (e.route != Route::Lan, e.latency_ms.unwrap_or(u32::MAX));
        let best = self
            .endpoints
            .iter()
            .filter(|e| e.healthy && e.url.scheme() == "https")
            .min_by_key(|e| rank(e))?;
        if let Some(current) = current.and_then(|url| {
            self.endpoints
                .iter()
                .find(|e| &e.url == url && e.healthy && e.url.scheme() == "https")
        }) && current.route == best.route
            && current
                .latency_ms
                .unwrap_or(u32::MAX)
                .saturating_sub(best.latency_ms.unwrap_or(u32::MAX))
                < 25
        {
            return Some(current);
        }
        Some(best)
    }
    pub fn add_verified_endpoint(&mut self, server_id: ServerId, endpoint: Endpoint) -> Result<()> {
        if server_id != self.server_id
            || endpoint.url.scheme() != "https"
            || endpoint.url.host_str().is_none()
            || !endpoint.url.username().is_empty()
            || endpoint.url.password().is_some()
        {
            return Err(Error::Invalid(
                "endpoint identity or scheme mismatch".into(),
            ));
        }
        if let Some(existing) = self.endpoints.iter_mut().find(|e| e.url == endpoint.url) {
            *existing = endpoint;
        } else {
            self.endpoints.push(endpoint);
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CacheEntry {
    pub server_id: ServerId,
    pub representation_id: RepresentationId,
    pub profile: ConversionProfile,
    pub bytes: u64,
    pub pinned: bool,
    pub last_used: u64,
    pub complete: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CacheKey {
    pub server_id: ServerId,
    pub representation_id: RepresentationId,
    pub profile: ConversionProfile,
}
impl CacheEntry {
    pub fn key(&self) -> CacheKey {
        CacheKey {
            server_id: self.server_id,
            representation_id: self.representation_id,
            profile: self.profile.clone(),
        }
    }
}
pub fn eviction_candidates(entries: &[CacheEntry], budget: u64) -> Vec<CacheKey> {
    let mut total = entries.iter().map(|e| e.bytes as u128).sum::<u128>();
    let mut candidates = entries.iter().filter(|e| !e.pinned).collect::<Vec<_>>();
    candidates.sort_by_key(|e| e.last_used);
    let mut result = Vec::new();
    for entry in candidates {
        if total <= budget as u128 {
            break;
        }
        total -= entry.bytes as u128;
        result.push(entry.key());
    }
    result
}
