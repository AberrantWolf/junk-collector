//! Authenticated route discovery; media routes never alter the saved control identity.
use crate::EnrolledClient;
use collector_api_types::LocalPlaybackDescriptor;
use collector_core::{Digest, Error, Result, ServerId};
use reqwest::{Client, Method, RequestBuilder};
use std::{net::IpAddr, time::Duration};
use url::Url;

#[derive(Clone)]
pub(crate) struct MediaRoute {
    pub client: Client,
    pub base: Url,
    pub local: bool,
}
impl MediaRoute {
    pub fn request(
        &self,
        owner: &EnrolledClient,
        method: Method,
        url: Url,
    ) -> Result<RequestBuilder> {
        if url.origin() != self.base.origin() || owner.enrolled.expires_at <= chrono::Utc::now() {
            return Err(Error::CredentialRevoked);
        }
        Ok(self
            .client
            .request(method, url)
            .bearer_auth(&owner.enrolled.credential))
    }
}
fn private_address(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            ip.is_private()
                || ip.is_loopback()
                || (ip.octets()[0] == 100 && (64..128).contains(&ip.octets()[1]))
        }
        IpAddr::V6(ip) => ip.is_unique_local() || ip.is_loopback(),
    }
}
impl EnrolledClient {
    pub(crate) async fn media_route(&self) -> Result<MediaRoute> {
        tokio::time::timeout(Duration::from_secs(5), self.discover_media_route())
            .await
            .map_err(|_| Error::Unavailable("local playback discovery timed out".into()))?
    }
    async fn discover_media_route(&self) -> Result<MediaRoute> {
        let response = self
            .request(Method::GET, self.api_url("api/v1/playback/routes")?)?
            .send()
            .await
            .map_err(|_| Error::Unavailable("local playback discovery failed".into()))?;
        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(Error::CredentialRevoked);
        }
        if response.status() == reqwest::StatusCode::FORBIDDEN {
            return Err(Error::Forbidden);
        }
        if !response.status().is_success() {
            return Err(Error::Unavailable(
                "server does not advertise secure playback routes".into(),
            ));
        }
        if response.content_length().is_some_and(|n| n > 128 * 1024) {
            return Err(Error::Invalid("oversized playback descriptor".into()));
        }
        let mut response = response;
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| Error::Unavailable("playback discovery interrupted".into()))?
        {
            if bytes.len() + chunk.len() > 128 * 1024 {
                return Err(Error::Invalid("oversized playback descriptor".into()));
            }
            bytes.extend_from_slice(&chunk);
        }
        let descriptor: LocalPlaybackDescriptor = serde_json::from_slice(&bytes)
            .map_err(|_| Error::Invalid("invalid playback descriptor".into()))?;
        if descriptor.server_id != self.enrolled.server_id || descriptor.endpoints.len() > 8 {
            return Err(Error::Invalid("playback server identity mismatch".into()));
        }
        let mut probes = tokio::task::JoinSet::new();
        for endpoint in descriptor.endpoints {
            let id = self.enrolled.server_id;
            probes.spawn(async move {
                let base = Url::parse(&endpoint.https_url).ok()?;
                let ip = base
                    .host_str()?
                    .trim_matches(['[', ']'])
                    .parse::<IpAddr>()
                    .ok()?;
                if base.scheme() != "https"
                    || !private_address(ip)
                    || !base.username().is_empty()
                    || base.password().is_some()
                    || base.path() != "/"
                    || base.query().is_some()
                    || base.fragment().is_some()
                    || endpoint.expires_at <= chrono::Utc::now()
                    || endpoint.expires_at > chrono::Utc::now() + chrono::Duration::days(8)
                {
                    return None;
                }
                let certificates = rustls_pemfile::certs(&mut endpoint.certificate_pem.as_bytes())
                    .collect::<std::io::Result<Vec<_>>>()
                    .ok()?;
                if certificates.len() != 1
                    || Digest::sha256(certificates[0].as_ref()).hex != endpoint.certificate_sha256
                {
                    return None;
                }
                let client = Client::builder()
                    .no_proxy()
                    .https_only(true)
                    .tls_built_in_root_certs(false)
                    .add_root_certificate(
                        reqwest::Certificate::from_pem(endpoint.certificate_pem.as_bytes()).ok()?,
                    )
                    .redirect(reqwest::redirect::Policy::none())
                    .connect_timeout(Duration::from_secs(2))
                    .timeout(Duration::from_secs(10))
                    .build()
                    .ok()?;
                // No bearer secret until the pinned listener proves the enrolled server identity.
                #[derive(serde::Deserialize)]
                struct Identity {
                    server_id: ServerId,
                }
                let response = tokio::time::timeout(
                    Duration::from_secs(2),
                    client.get(base.join("api/v1/identity").ok()?).send(),
                )
                .await
                .ok()?
                .ok()?;
                if !response.status().is_success()
                    || response.content_length().is_some_and(|n| n > 4096)
                {
                    return None;
                }
                let mut response = response;
                let mut bytes = Vec::new();
                while let Some(chunk) = response.chunk().await.ok()? {
                    if bytes.len() + chunk.len() > 4096 {
                        return None;
                    }
                    bytes.extend_from_slice(&chunk);
                }
                let identity: Identity = serde_json::from_slice(&bytes).ok()?;
                (identity.server_id == id).then_some(MediaRoute {
                    client,
                    base,
                    local: true,
                })
            });
        }
        while let Some(result) = probes.join_next().await {
            if let Ok(Some(route)) = result {
                return Ok(route);
            }
        }
        if descriptor.remote_media_allowed {
            return Ok(MediaRoute {
                client: self.client.clone(),
                base: self.api_url("")?,
                local: false,
            });
        }
        Err(Error::Unavailable("No trusted local playback route is reachable. Public media fallback is disabled. Check local network access and the server firewall.".into()))
    }
}
