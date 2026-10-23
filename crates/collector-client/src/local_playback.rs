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
    pub remote: bool,
    pub grant: Option<collector_api_types::RemoteMediaGrant>,
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
        let credential = if self.remote {
            if url.path() != self.base.join("read").map_err(|_| Error::Forbidden)?.path()
                || url.query().is_some()
                || url.fragment().is_some()
                || !url.username().is_empty()
                || url.password().is_some()
            {
                return Err(Error::Forbidden);
            }
            self.grant
                .as_ref()
                .filter(|g| g.expires_at > chrono::Utc::now())
                .map(|g| g.token.as_str())
                .ok_or(Error::CredentialRevoked)?
        } else {
            &owner.enrolled.credential
        };
        Ok(self.client.request(method, url).bearer_auth(credential))
    }
    pub async fn target(
        &mut self,
        owner: &EnrolledClient,
        target: collector_api_types::PlaybackTarget,
    ) -> Result<Url> {
        if !self.remote {
            return self
                .base
                .join(&format!(
                    "api/v1/items/{}/representations/{}/{}",
                    target.item_id, target.representation_id, target.file_index
                ))
                .map_err(|_| Error::Invalid("invalid media URL".into()));
        }
        let response = owner
            .request(
                Method::POST,
                owner.api_url("api/v1/playback/remote-grants")?,
            )?
            .json(&collector_api_types::RemoteGrantRequest {
                target: Some(target),
            })
            .send()
            .await
            .map_err(|_| Error::Unavailable("direct media grant unavailable".into()))?;
        if !response.status().is_success() {
            return Err(Error::Forbidden);
        }
        let grant: collector_api_types::RemoteMediaGrant =
            crate::bounded_json(response, 16384).await?;
        let mut url = Url::parse(&grant.url)
            .map_err(|_| Error::Invalid("invalid direct media URL".into()))?;
        if url.origin() != self.base.origin()
            || url.path() != self.base.join("read").map_err(|_| Error::Forbidden)?.path()
            || grant.server_id != owner.enrolled.server_id
            || grant.expires_at <= chrono::Utc::now()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.fragment().is_some()
            || grant.token.len() != 64
            || !grant.token.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err(Error::Forbidden);
        }
        // Native uses only a scoped Authorization header, never a secret-bearing URL.
        url.set_query(None);
        self.grant = Some(grant);
        Ok(url)
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
                    remote: false,
                    grant: None,
                })
            });
        }
        while let Some(result) = probes.join_next().await {
            if let Ok(Some(route)) = result {
                return Ok(route);
            }
        }
        if let Some(endpoint) = descriptor.remote_media {
            let base = Url::parse(&endpoint.base_url)
                .map_err(|_| Error::Invalid("invalid direct media endpoint".into()))?;
            if base.scheme() != "https"
                || !base.username().is_empty()
                || base.password().is_some()
                || base.query().is_some()
                || base.fragment().is_some()
                || !base.path().ends_with('/')
                || endpoint.server_id != self.enrolled.server_id
            {
                return Err(Error::Forbidden);
            }
            // This client uses system roots, independently of the saved control certificate.
            let client = crate::system_https_client()?;
            let identity = client
                .get(base.join("api/v1/identity").map_err(|_| Error::Forbidden)?)
                .send()
                .await
                .map_err(|_| Error::Unavailable("direct HTTPS media is unreachable".into()))?;
            let identity: crate::Identity = crate::bounded_json(identity, 4096).await?;
            if identity.server_id != self.enrolled.server_id {
                return Err(Error::Forbidden);
            }
            return Ok(MediaRoute {
                client,
                base,
                local: false,
                remote: true,
                grant: None,
            });
        }
        if descriptor.remote_media_allowed {
            return Ok(MediaRoute {
                client: self.client.clone(),
                base: self.api_url("")?,
                local: false,
                remote: false,
                grant: None,
            });
        }
        Err(Error::Unavailable("No trusted local playback route is reachable. Public media fallback is disabled. Check local network access and the server firewall.".into()))
    }
}
