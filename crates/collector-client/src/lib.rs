//! UI-independent connection trust, endpoint policy and offline cache planning.
use collector_core::*;
use serde::{Deserialize, Serialize};
use url::Url;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Pairing {
    pub server_id: ServerId,
    pub certificate_pem: String,
    pub certificate_sha256: String,
    pub endpoints: Vec<Endpoint>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Endpoint {
    pub url: Url,
    pub route: Route,
    pub latency_ms: Option<u32>,
    pub healthy: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
