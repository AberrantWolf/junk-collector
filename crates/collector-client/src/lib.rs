//! UI-independent connection trust, endpoint policy and offline cache planning.
use collector_core::*;
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EnrolledCredential {
    pub server_id: ServerId,
    pub credential_id: NativeCredentialId,
    pub credential: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub pairing: Pairing,
}

/// Platform clients implement this port with Keychain, Credential Manager, or
/// an equivalent OS-protected secret store. Core client code never chooses a
/// filesystem location for bearer credentials.
pub trait NativeCredentialStorage: Send + Sync {
    fn save(&self, credential: &EnrolledCredential) -> Result<()>;
    fn load(&self, server: ServerId) -> Result<Option<EnrolledCredential>>;
    fn remove(&self, server: ServerId) -> Result<()>;
}

#[derive(Clone, Debug, Deserialize)]
pub struct EnrollmentStart {
    pub id: EnrollmentId,
    pub completion_secret: String,
    pub user_code: String,
    pub verification_url: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

pub struct PendingEnrollment {
    base_url: Url,
    expected_server: ServerId,
    pub start: EnrollmentStart,
}

#[derive(Deserialize)]
struct Identity {
    server_id: ServerId,
}

#[derive(Deserialize)]
struct CompletedEnrollment {
    device_id: NativeCredentialId,
    credential: String,
    expires_at: chrono::DateTime<chrono::Utc>,
    pairing: Pairing,
}

impl PendingEnrollment {
    pub async fn start(
        client: &reqwest::Client,
        base_url: Url,
        username: Option<&str>,
        device_name: &str,
    ) -> Result<Self> {
        if base_url.scheme() != "https" || base_url.host_str().is_none() {
            return Err(Error::Invalid(
                "enrollment requires an HTTPS server URL".into(),
            ));
        }
        let identity: Identity = client
            .get(base_url.join("api/v1/identity").map_err(invalid_url)?)
            .send()
            .await
            .map_err(client_error)?
            .error_for_status()
            .map_err(client_error)?
            .json()
            .await
            .map_err(client_error)?;
        let start = client
            .post(
                base_url
                    .join("api/v1/enrollments/start")
                    .map_err(invalid_url)?,
            )
            .json(&serde_json::json!({"username": username, "device_name": device_name}))
            .send()
            .await
            .map_err(client_error)?
            .error_for_status()
            .map_err(client_error)?
            .json()
            .await
            .map_err(client_error)?;
        Ok(Self {
            base_url,
            expected_server: identity.server_id,
            start,
        })
    }

    pub async fn complete(
        self,
        client: &reqwest::Client,
        storage: &dyn NativeCredentialStorage,
    ) -> Result<EnrolledCredential> {
        let completed: CompletedEnrollment = client
            .post(
                self.base_url
                    .join(&format!("api/v1/enrollments/{}/complete", self.start.id))
                    .map_err(invalid_url)?,
            )
            .json(&serde_json::json!({"completion_secret": self.start.completion_secret}))
            .send()
            .await
            .map_err(client_error)?
            .error_for_status()
            .map_err(client_error)?
            .json()
            .await
            .map_err(client_error)?;
        if completed.pairing.server_id != self.expected_server {
            return Err(Error::Invalid(
                "server identity changed during enrollment".into(),
            ));
        }
        completed.pairing.lan_client()?;
        let enrolled = EnrolledCredential {
            server_id: completed.pairing.server_id,
            credential_id: completed.device_id,
            credential: completed.credential,
            expires_at: completed.expires_at,
            pairing: completed.pairing,
        };
        storage.save(&enrolled)?;
        Ok(enrolled)
    }
}

pub struct EnrolledClient {
    client: reqwest::Client,
    enrolled: EnrolledCredential,
}

impl EnrolledClient {
    pub fn from_storage(storage: &dyn NativeCredentialStorage, server: ServerId) -> Result<Self> {
        let enrolled = storage.load(server)?.ok_or(Error::Unauthenticated)?;
        if enrolled.expires_at <= chrono::Utc::now() {
            return Err(Error::CredentialRevoked);
        }
        let client = enrolled.pairing.lan_client()?;
        Ok(Self { client, enrolled })
    }

    pub fn get(&self, url: Url) -> Result<reqwest::RequestBuilder> {
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
        Ok(self.client.get(url).bearer_auth(&self.enrolled.credential))
    }
}

fn invalid_url(error: url::ParseError) -> Error {
    Error::Invalid(error.to_string())
}

fn client_error(error: reqwest::Error) -> Error {
    if error.status() == Some(reqwest::StatusCode::UNAUTHORIZED) {
        Error::Unauthenticated
    } else {
        Error::Unavailable(error.to_string())
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
