//! Media-only private listeners. Trust is delivered through the authenticated control origin.
use super::*;
use collector_api_types::{
    BrowserPlaybackGrant, LocalPlaybackDescriptor, LocalPlaybackEndpoint, PlaybackTarget,
};
use std::{
    net::{IpAddr, SocketAddr},
    sync::RwLock,
    time::Duration,
};
use wtransport::{Endpoint, Identity, ServerConfig};

const CHUNK: u64 = 256 * 1024;

type CachedReader = Arc<Mutex<(Instant, MediaResource)>>;

pub struct LocalPlayback {
    descriptor: RwLock<LocalPlaybackDescriptor>,
    // Hold a verified open handle across ranges: never hash a whole track per chunk.
    readers: Mutex<HashMap<String, CachedReader>>,
    slots: Arc<tokio::sync::Semaphore>,
}
impl LocalPlayback {
    pub fn descriptor(&self, remote_media_allowed: bool) -> LocalPlaybackDescriptor {
        let mut descriptor = self.descriptor.read().expect("descriptor poisoned").clone();
        descriptor.remote_media_allowed = remote_media_allowed;
        descriptor
    }
    pub fn connect_origins(&self) -> Vec<String> {
        self.descriptor(false)
            .endpoints
            .into_iter()
            .map(|e| e.https_url)
            .collect()
    }
}

pub struct Runtime {
    workers: Vec<tokio::task::JoinHandle<()>>,
    rotation: Arc<Rotation>,
    handle: axum_server::Handle<SocketAddr>,
}
impl Drop for Runtime {
    fn drop(&mut self) {
        self.rotation
            .endpoint
            .close(0u32.into(), b"server stopping");
        self.handle.graceful_shutdown(Some(Duration::from_secs(1)));
        for task in &self.workers {
            task.abort();
        }
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
// Interface names are hints for ordering, not a security boundary. Keep these
// narrow: a host LAN bridge named br0 and an ordinary 172.x LAN remain eligible.
fn container_interface(name: &str) -> bool {
    name == "docker0"
        || name == "docker_gwbridge"
        || name.strip_prefix("br-").is_some_and(|suffix| {
            suffix.len() == 12 && suffix.bytes().all(|c| c.is_ascii_hexdigit())
        })
        || name.strip_prefix("veth").is_some_and(|suffix| {
            !suffix.is_empty() && suffix.bytes().all(|c| c.is_ascii_hexdigit())
        })
}

fn preferred_source(bind: SocketAddr) -> Option<IpAddr> {
    let destination: SocketAddr = if bind.is_ipv4() {
        "192.0.2.1:9".parse().expect("IPv4 documentation address")
    } else {
        "[2001:db8::1]:9"
            .parse()
            .expect("IPv6 documentation address")
    };
    // UDP connect selects the kernel's route/source without transmitting a packet.
    // No default route is fine: interface enumeration still works on offline LANs.
    let socket = std::net::UdpSocket::bind(SocketAddr::new(bind.ip(), 0)).ok()?;
    socket.connect(destination).ok()?;
    Some(socket.local_addr().ok()?.ip())
}

fn select_addresses(
    bind: SocketAddr,
    interfaces: impl IntoIterator<Item = (String, IpAddr)>,
    preferred: Option<IpAddr>,
) -> std::io::Result<Vec<IpAddr>> {
    if !bind.ip().is_unspecified() {
        if !private_address(bind.ip()) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "local playback bind must be private or wildcard",
            ));
        }
        return Ok(vec![bind.ip()]);
    }
    let mut candidates: Vec<_> = interfaces
        .into_iter()
        .filter(|(_, ip)| private_address(*ip) && ip.is_ipv4() == bind.is_ipv4())
        .map(|(name, ip)| {
            let priority = if ip.is_loopback() {
                3
            } else if Some(ip) == preferred {
                0
            } else if container_interface(&name) {
                2
            } else {
                1
            };
            (priority, ip)
        })
        .collect();
    candidates.sort_unstable();
    let mut seen = std::collections::HashSet::new();
    Ok(candidates
        .into_iter()
        .map(|(_, ip)| ip)
        .filter(|ip| seen.insert(*ip))
        .take(8)
        .collect())
}

fn addresses(bind: SocketAddr) -> std::io::Result<Vec<IpAddr>> {
    if !bind.ip().is_unspecified() {
        return select_addresses(bind, [], None);
    }
    select_addresses(
        bind,
        if_addrs::get_if_addrs()?
            .into_iter()
            .map(|interface| (interface.name.clone(), interface.ip())),
        preferred_source(bind),
    )
}

fn make_identity(
    ips: &[IpAddr],
    port: u16,
    id: ServerId,
) -> anyhow::Result<(Identity, LocalPlaybackDescriptor)> {
    let names: Vec<_> = ips.iter().map(ToString::to_string).collect();
    let now = time::OffsetDateTime::now_utc();
    let expires = now + time::Duration::days(7);
    let identity = wtransport::tls::self_signed::SelfSignedIdentityBuilder::new()
        .subject_alt_names(&names)
        .validity_period(now - time::Duration::minutes(5), expires)
        .build()?;
    let cert = &identity.certificate_chain().as_slice()[0];
    let endpoints = ips
        .iter()
        .map(|ip| {
            let authority = SocketAddr::new(*ip, port);
            LocalPlaybackEndpoint {
                https_url: format!("https://{authority}"),
                webtransport_url: format!("https://{authority}/playback"),
                certificate_pem: cert.to_pem(),
                certificate_sha256: Digest::sha256(cert.der()).hex,
                expires_at: chrono::DateTime::from_timestamp(expires.unix_timestamp(), 0)
                    .expect("current timestamp"),
            }
        })
        .collect();
    Ok((
        identity,
        LocalPlaybackDescriptor {
            server_id: id,
            endpoints,
            remote_media_allowed: false,
        },
    ))
}
fn quic_config(bind: SocketAddr, identity: Identity) -> ServerConfig {
    let mut config = ServerConfig::builder()
        .with_bind_address(bind)
        .with_identity(identity)
        .max_idle_timeout(Some(Duration::from_secs(30)))
        .expect("bounded idle")
        .build();
    let transport = Arc::get_mut(&mut config.quic_config_mut().transport).expect("unique config");
    transport.max_concurrent_bidi_streams(8u32.into());
    transport.max_concurrent_uni_streams(8u32.into());
    config
}
fn pem(identity: &Identity) -> (Vec<u8>, Vec<u8>) {
    (
        identity.certificate_chain().as_slice()[0]
            .to_pem()
            .into_bytes(),
        identity.private_key().to_secret_pem().into_bytes(),
    )
}

pub async fn start(state: &mut AppState, bind: SocketAddr) -> anyhow::Result<Runtime> {
    let tcp = std::net::TcpListener::bind(bind)?;
    tcp.set_nonblocking(true)?;
    let bind = tcp.local_addr()?;
    let ips = addresses(bind)?;
    anyhow::ensure!(
        !ips.is_empty(),
        "no private address for local playback listener"
    );
    let (identity, descriptor) = make_identity(&ips, bind.port(), state.pairing.server_id)?;
    let (cert, key) = pem(&identity);
    let tls = axum_server::tls_rustls::RustlsConfig::from_pem(cert, key).await?;
    let endpoint = Arc::new(Endpoint::server(quic_config(bind, identity))?);
    let local = Arc::new(LocalPlayback {
        descriptor: RwLock::new(descriptor),
        readers: Mutex::new(HashMap::new()),
        slots: Arc::new(tokio::sync::Semaphore::new(32)),
    });
    state.local_playback = Some(local.clone());
    let mut media_state = state.clone();
    media_state.direct_media = true;
    // Explicit composition: no library, pairing, login, curation or administration here.
    let router = Router::new()
        .route("/api/v1/identity", get(super::identity))
        .route(
            "/api/v1/items/{id}/representations/{representation}/{index}",
            get(native_media),
        )
        .with_state(media_state.clone());
    let handle = axum_server::Handle::new();
    let server = axum_server::from_tcp_rustls(tcp, tls.clone())?
        .handle(handle.clone())
        .serve(router.into_make_service());
    let mut workers = vec![tokio::spawn(async move {
        if let Err(error) = server.await {
            tracing::error!(%error,"local HTTPS listener stopped");
        }
    })];
    let accept_endpoint = endpoint.clone();
    let sessions = Arc::new(tokio::sync::Semaphore::new(32));
    workers.push(tokio::spawn(async move {
        let mut tasks=tokio::task::JoinSet::new();
        loop { tokio::select! {
            incoming=accept_endpoint.accept()=>{
                let Ok(slot)=sessions.clone().try_acquire_owned() else { incoming.refuse(); continue; };
                let state=media_state.clone();
                tasks.spawn(async move {let _slot=slot; let _=tokio::time::timeout(Duration::from_secs(600),session(incoming,state)).await;});
            }
            _=tasks.join_next(),if !tasks.is_empty()=>{}
        }}
    }));
    let rotation = Arc::new(Rotation {
        endpoint,
        local,
        tls,
        bind,
        id: state.pairing.server_id,
    });
    let rotate = rotation.clone();
    workers.push(tokio::spawn(async move {
        let mut previous = ips;
        let mut generated = Instant::now();
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;
            let updated = match addresses(bind) {
                Ok(ips) if !ips.is_empty() => ips,
                _ => continue,
            };
            if updated == previous && generated.elapsed() < Duration::from_secs(86400 * 3) {
                continue;
            }
            let result = rotate.refresh(&updated).await;
            match result {
                Ok(()) => {
                    previous = updated;
                    generated = Instant::now();
                }
                Err(error) => tracing::error!(%error,"local certificate rotation failed"),
            }
        }
    }));
    Ok(Runtime {
        workers,
        rotation,
        handle,
    })
}

struct Rotation {
    endpoint: Arc<Endpoint<wtransport::endpoint::endpoint_side::Server>>,
    local: Arc<LocalPlayback>,
    tls: axum_server::tls_rustls::RustlsConfig,
    bind: SocketAddr,
    id: ServerId,
}
impl Rotation {
    async fn refresh(&self, ips: &[IpAddr]) -> anyhow::Result<()> {
        let (identity, descriptor) = make_identity(ips, self.bind.port(), self.id)?;
        let (cert, key) = pem(&identity);
        // Publish trust only after both listeners accept the new identity.
        self.endpoint
            .reload_config(quic_config(self.bind, identity), false)?;
        self.tls.reload_from_pem(cert, key).await?;
        *self.local.descriptor.write().expect("descriptor poisoned") = descriptor;
        Ok(())
    }
}

#[utoipa::path(get,path="/api/v1/playback/routes",responses((status=200,body=LocalPlaybackDescriptor),(status=401)))]
pub(crate) async fn routes(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<LocalPlaybackDescriptor>> {
    principal(&state, &headers)?;
    let descriptor = state
        .local_playback
        .as_ref()
        .map(|local| local.descriptor(state.remote_media_allowed || state.direct_media))
        .unwrap_or(LocalPlaybackDescriptor {
            server_id: state.pairing.server_id,
            endpoints: vec![],
            remote_media_allowed: state.direct_media || state.remote_media_allowed,
        });
    Ok(Json(descriptor))
}
#[utoipa::path(post,path="/api/v1/playback/grants",request_body=PlaybackTarget,responses((status=200,body=BrowserPlaybackGrant),(status=401),(status=403)))]
pub(crate) async fn browser_grant(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(target): Json<PlaybackTarget>,
) -> ApiResult<Json<BrowserPlaybackGrant>> {
    authenticated(&state, &headers)?;
    let origin = headers
        .get(header::ORIGIN)
        .and_then(|v| v.to_str().ok())
        .and_then(normalized_configured_origin)
        .filter(|origin| {
            state
                .allowed_origins
                .iter()
                .any(|o| normalized_configured_origin(o).as_ref() == Some(origin))
        })
        .ok_or(Error::Forbidden)?;
    let session = headers
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| {
            s.split(';')
                .map(str::trim)
                .find_map(|p| p.strip_prefix("collector_session="))
        })
        .ok_or(Error::Unauthenticated)?
        .to_owned();
    let token = format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    );
    Ok(Json(
        blocking(move || {
            state.service.issue_browser_playback(
                &session,
                &origin,
                target,
                token,
                chrono::Utc::now(),
            )
        })
        .await?,
    ))
}
pub(crate) async fn worker() -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, "application/javascript"),
            (header::CACHE_CONTROL, "no-cache"),
        ],
        include_str!("../../../web/dist/playback-sw.js"),
    )
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReadRequest {
    token: String,
    target: PlaybackTarget,
    method: String,
    start: Option<u64>,
    end: Option<u64>,
    etag: Option<String>,
}
#[derive(serde::Serialize)]
struct ReadResponse {
    status: u16,
    headers: HashMap<String, String>,
}

async fn session(
    incoming: wtransport::endpoint::IncomingSession,
    state: AppState,
) -> anyhow::Result<()> {
    let request = tokio::time::timeout(Duration::from_secs(5), incoming).await??;
    let origin = request.origin().and_then(normalized_configured_origin);
    let Some(origin) = origin.filter(|o| {
        state
            .allowed_origins
            .iter()
            .any(|allowed| normalized_configured_origin(allowed).as_ref() == Some(o))
    }) else {
        request.forbidden().await;
        return Ok(());
    };
    if request.path() != "/playback" {
        request.not_found().await;
        return Ok(());
    }
    let connection = request.accept().await?;
    let mut tasks = tokio::task::JoinSet::new();
    let cache_key = uuid::Uuid::new_v4().to_string();
    loop {
        tokio::select! {
            stream=connection.accept_bi()=>{
                let (mut send,mut receive)=stream?;
                let state=state.clone(); let origin=origin.clone(); let key=cache_key.clone();
                if tasks.len()>=8 {let _ = send.reset(1u32.into());continue;}
                tasks.spawn(async move {
                    let work=async {
                        let mut input=Vec::new(); let mut buffer=[0u8;1024];
                        while let Some(n)=receive.read(&mut buffer).await? { if input.len()+n>8192 {anyhow::bail!("request too large");} input.extend_from_slice(&buffer[..n]); }
                        let request:ReadRequest=serde_json::from_slice(&input)?;
                        let result=tokio::select! {result=read_browser(&state,&origin,&key,request)=>result,_=send.stopped()=>return Ok(())};
                        let (response,body)=match result {Ok(value)=>value,Err(error)=>{
                            let response=error.into_response();let status=response.status().as_u16();
                            let body=axum::body::to_bytes(response.into_body(),8192).await?.to_vec();
                            (ReadResponse{status,headers:HashMap::from([("content-type".into(),"application/json".into()),("content-length".into(),body.len().to_string())])},body)
                        }};
                        let mut header=serde_json::to_vec(&response)?; header.push(b'\n');
                        send.write_all(&header).await?; send.write_all(&body).await?; send.finish().await?;
                        Ok::<_,anyhow::Error>(())
                    };
                    let _=tokio::time::timeout(Duration::from_secs(20),work).await;
                });
            }
            _=tasks.join_next(),if !tasks.is_empty()=>{}
        }
    }
}
async fn read_browser(
    state: &AppState,
    origin: &str,
    key: &str,
    request: ReadRequest,
) -> ApiResult<(ReadResponse, Vec<u8>)> {
    let local = state
        .local_playback
        .as_ref()
        .ok_or(Error::NotFound)?
        .clone();
    let _slot = local
        .slots
        .clone()
        .try_acquire_owned()
        .map_err(|_| Error::RateLimited(1))?;
    if !matches!(request.method.as_str(), "GET" | "HEAD") {
        return Err(Error::Forbidden.into());
    }
    let service = state.service.clone();
    let origin = origin.to_owned();
    let target = request.target.clone();
    let resource = blocking(move || {
        service.browser_playback_resource(&request.token, &origin, &target, chrono::Utc::now())
    })
    .await?;
    bounded_read(
        local,
        format!(
            "{key}:{}:{}:{}",
            request.target.item_id, request.target.representation_id, request.target.file_index
        ),
        resource,
        request.method == "HEAD",
        request.start,
        request.end,
        request.etag,
    )
    .await
}

async fn native_media(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
    Path((id, representation, index)): Path<(ItemId, RepresentationId, usize)>,
) -> ApiResult<Response> {
    // Native credentials only; browser cookies are never sent to private addresses.
    let token = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(Error::Unauthenticated)?
        .to_owned();
    let service = state.service.clone();
    let credential = token.clone();
    let resource = blocking(move || {
        let actor = service.authenticate_native(&credential)?.principal;
        service.representation(&actor, id, representation, index)
    })
    .await?;
    let local = state
        .local_playback
        .as_ref()
        .ok_or(Error::NotFound)?
        .clone();
    let _slot = local
        .slots
        .clone()
        .try_acquire_owned()
        .map_err(|_| Error::RateLimited(1))?;
    let range = headers.get(header::RANGE).and_then(|v| v.to_str().ok());
    let parsed = range.and_then(|r| parse_range(r, resource.length));
    if method != Method::HEAD && parsed.is_none() {
        return Ok(StatusCode::RANGE_NOT_SATISFIABLE.into_response());
    }
    let key = format!(
        "{}:{id}:{representation}:{index}",
        Digest::sha256(token.as_bytes()).hex
    );
    let (response, body) = bounded_read(
        local,
        key,
        resource,
        method == Method::HEAD,
        parsed.map(|r| r.0),
        parsed.map(|r| r.1),
        headers
            .get(header::IF_RANGE)
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned),
    )
    .await?;
    let mut builder = Response::builder().status(response.status);
    for (key, value) in response.headers {
        builder = builder.header(key, value);
    }
    Ok(builder
        .body(Body::from(body))
        .map_err(|e| Error::Storage(e.to_string()))?)
}

async fn bounded_read(
    local: Arc<LocalPlayback>,
    key: String,
    resource: MediaResource,
    head: bool,
    start: Option<u64>,
    end: Option<u64>,
    expected: Option<String>,
) -> ApiResult<(ReadResponse, Vec<u8>)> {
    let length = resource.length;
    let etag = format!("\"{}\"", resource.digest.hex);
    let mut headers = HashMap::from([
        ("content-type".into(), resource.media_type.clone()),
        ("etag".into(), etag.clone()),
        ("accept-ranges".into(), "bytes".into()),
        ("content-length".into(), length.to_string()),
    ]);
    if head {
        return Ok((
            ReadResponse {
                status: 200,
                headers,
            },
            vec![],
        ));
    }
    let (start, end) = match (start, end) {
        (Some(start), Some(end)) if start <= end && end < length && end - start < CHUNK => {
            (start, end)
        }
        _ => {
            headers.insert("content-range".into(), format!("bytes */{length}"));
            headers.insert("content-length".into(), "0".into());
            return Ok((
                ReadResponse {
                    status: 416,
                    headers,
                },
                vec![],
            ));
        }
    };
    if expected.as_ref().is_some_and(|v| v != &etag) {
        return Err(Error::Conflict.into());
    }
    headers.insert("content-length".into(), (end - start + 1).to_string());
    headers.insert(
        "content-range".into(),
        format!("bytes {start}-{end}/{length}"),
    );
    let reader = {
        let mut readers = local
            .readers
            .lock()
            .map_err(|_| Error::Unavailable("media readers unavailable".into()))?;
        readers.retain(|_, reader| {
            reader
                .try_lock()
                .map_or(true, |r| r.0.elapsed() < Duration::from_secs(120))
        });
        if !readers.contains_key(&key) && readers.len() >= 32 {
            let oldest = readers
                .iter()
                .filter_map(|(key, reader)| reader.try_lock().ok().map(|r| (key.clone(), r.0)))
                .min_by_key(|(_, time)| *time)
                .map(|(key, _)| key);
            if let Some(oldest) = oldest {
                readers.remove(&oldest);
            } else {
                return Err(Error::RateLimited(1).into());
            }
        }
        readers
            .entry(key)
            .or_insert_with(|| Arc::new(Mutex::new((Instant::now(), resource))))
            .clone()
    };
    struct Cancel(std::sync::Arc<std::sync::atomic::AtomicBool>);
    impl Drop for Cancel {
        fn drop(&mut self) {
            self.0.store(true, std::sync::atomic::Ordering::Release);
        }
    }
    let cancelled = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let _cancel = Cancel(cancelled.clone());
    let body = blocking(move || {
        let mut cached = reader
            .lock()
            .map_err(|_| Error::Unavailable("media reader unavailable".into()))?;
        if cancelled.load(std::sync::atomic::Ordering::Acquire) {
            return Err(Error::JobCancelled);
        }
        cached.0 = Instant::now();
        let resource = &mut cached.1;
        if resource.length != length || format!("\"{}\"", resource.digest.hex) != etag {
            return Err(Error::Conflict);
        }
        if resource.verify_on_transfer {
            verify_open_reader(
                resource.reader.as_mut(),
                length,
                &resource.digest,
                &cancelled,
            )?;
            resource.verify_on_transfer = false;
        }
        resource
            .reader
            .seek(SeekFrom::Start(start))
            .map_err(|e| Error::Storage(e.to_string()))?;
        let mut bytes = vec![0; (end - start + 1) as usize];
        resource
            .reader
            .read_exact(&mut bytes)
            .map_err(|e| Error::Storage(e.to_string()))?;
        Ok(bytes)
    })
    .await?;
    Ok((
        ReadResponse {
            status: 206,
            headers,
        },
        body,
    ))
}

// Validation runs once for the retained open handle, never on metadata polling or
// every range, and stops promptly when the network consumer goes away.
fn verify_open_reader(
    reader: &mut dyn collector_app::MediaReader,
    length: u64,
    digest: &Digest,
    cancel: &std::sync::atomic::AtomicBool,
) -> collector_core::Result<()> {
    use sha2::{Digest as _, Sha256};
    reader
        .seek(SeekFrom::Start(0))
        .map_err(|_| Error::PreparedMediaInvalid)?;
    let mut hash = Sha256::new();
    let mut seen = 0u64;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        if cancel.load(std::sync::atomic::Ordering::Acquire) {
            return Err(Error::JobCancelled);
        }
        let n = reader
            .read(&mut buffer)
            .map_err(|_| Error::PreparedMediaInvalid)?;
        if n == 0 {
            break;
        }
        seen = seen
            .checked_add(n as u64)
            .ok_or(Error::PreparedMediaInvalid)?;
        if seen > length {
            return Err(Error::PreparedMediaInvalid);
        }
        hash.update(&buffer[..n]);
    }
    if seen != length || hex::encode(hash.finalize()) != digest.hex {
        return Err(Error::PreparedMediaInvalid);
    }
    reader
        .seek(SeekFrom::Start(0))
        .map_err(|_| Error::PreparedMediaInvalid)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn discovery_never_treats_public_or_link_local_addresses_as_private() {
        for ip in [
            "192.168.1.2",
            "10.0.0.2",
            "100.70.1.2",
            "fd00::2",
            "127.0.0.1",
        ] {
            assert!(private_address(ip.parse().unwrap()));
        }
        for ip in [
            "8.8.8.8",
            "169.254.1.2",
            "100.128.0.1",
            "fe80::1",
            "2001:4860:4860::8888",
        ] {
            assert!(!private_address(ip.parse().unwrap()));
        }
    }
    fn interface(name: &str, ip: &str) -> (String, IpAddr) {
        (name.into(), ip.parse().unwrap())
    }

    #[test]
    fn docker_bridges_cannot_crowd_out_lan_descriptor_or_certificate() {
        use rustls::client::danger::ServerCertVerifier;
        use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
        let mut interfaces: Vec<_> = (17..=30)
            .map(|n| interface(&format!("br-{n:012x}"), &format!("172.{n}.0.1")))
            .collect();
        interfaces.extend([
            interface("docker0", "172.16.0.1"),
            interface("ovs_eth0", "192.168.1.106"),
            interface("lo", "127.0.0.1"),
        ]);
        let ips = select_addresses("0.0.0.0:9845".parse().unwrap(), interfaces, None).unwrap();
        let lan: IpAddr = "192.168.1.106".parse().unwrap();
        assert_eq!(ips[0], lan);
        assert_eq!(ips.len(), 8);
        let (identity, descriptor) = make_identity(&ips, 9845, ServerId::new()).unwrap();
        let endpoint = descriptor
            .endpoints
            .iter()
            .find(|endpoint| endpoint.https_url == "https://192.168.1.106:9845")
            .expect("the browser/native descriptor must retain the NAS LAN route");
        assert_eq!(
            endpoint.webtransport_url,
            "https://192.168.1.106:9845/playback"
        );
        let leaf = CertificateDer::from(identity.certificate_chain().as_slice()[0].der().to_vec());
        assert_eq!(
            endpoint.certificate_sha256,
            Digest::sha256(leaf.as_ref()).hex
        );
        let mut roots = rustls::RootCertStore::empty();
        roots.add(leaf.clone()).unwrap();
        let verifier = rustls::client::WebPkiServerVerifier::builder(Arc::new(roots))
            .build()
            .unwrap();
        verifier
            .verify_server_cert(
                &leaf,
                &[],
                &ServerName::IpAddress(lan.into()),
                &[],
                UnixTime::now(),
            )
            .expect("the pinned certificate must actually validate for the advertised LAN address");
        assert!(
            verifier
                .verify_server_cert(
                    &leaf,
                    &[],
                    &ServerName::IpAddress("192.168.1.107".parse::<IpAddr>().unwrap().into()),
                    &[],
                    UnixTime::now(),
                )
                .is_err()
        );
    }

    #[test]
    fn kernel_source_survives_the_limit_even_for_unrecognized_interface_names() {
        let mut interfaces: Vec<_> = (17..=30)
            .map(|n| interface(&format!("custom{n}"), &format!("172.{n}.0.1")))
            .collect();
        let lan = interface("custom-uplink", "192.168.1.106");
        interfaces.push(lan.clone());
        let selected = select_addresses(
            "0.0.0.0:0".parse().unwrap(),
            interfaces.clone(),
            Some(lan.1),
        )
        .unwrap();
        assert_eq!(selected[0], lan.1);
        assert_eq!(selected.len(), 8);
        interfaces.reverse();
        assert_eq!(
            select_addresses("0.0.0.0:0".parse().unwrap(), interfaces, Some(lan.1)).unwrap(),
            selected
        );
    }

    #[test]
    fn deduplication_precedes_the_bound_and_uses_the_best_interface_priority() {
        let mut interfaces = vec![interface("docker0", "10.0.0.1"); 16];
        interfaces.extend([
            interface("eth0", "10.0.0.1"),
            interface("eth1", "192.168.1.106"),
            interface("lo", "127.0.0.1"),
        ]);
        let ips = select_addresses("0.0.0.0:0".parse().unwrap(), interfaces, None).unwrap();
        assert_eq!(
            ips,
            ["10.0.0.1", "192.168.1.106", "127.0.0.1"].map(|ip| ip.parse::<IpAddr>().unwrap())
        );
    }

    #[test]
    fn offline_private_routes_preserve_host_bridges_cgnat_and_address_family() {
        let interfaces = vec![
            interface("eth0", "172.20.1.2"),
            interface("br0", "192.168.2.1"),
            interface("overlay0", "100.70.1.2"),
            interface("vethdeadbeef", "10.0.0.2"),
            interface("docker_gwbridge", "10.0.0.3"),
            interface("eth1", "fd00::2"),
            interface("lo", "127.0.0.1"),
            interface("lo", "::1"),
            interface("public", "8.8.8.8"),
            interface("public6", "2001:4860:4860::8888"),
            interface("local", "169.254.1.2"),
            interface("local6", "fe80::1"),
        ];
        let ips = select_addresses("0.0.0.0:0".parse().unwrap(), interfaces.clone(), None).unwrap();
        assert_eq!(
            ips,
            [
                "100.70.1.2",
                "172.20.1.2",
                "192.168.2.1",
                "10.0.0.2",
                "10.0.0.3",
                "127.0.0.1"
            ]
            .map(|ip| ip.parse::<IpAddr>().unwrap())
        );
        assert_eq!(
            select_addresses("[::]:0".parse().unwrap(), interfaces, None).unwrap(),
            ["fd00::2", "::1"].map(|ip| ip.parse::<IpAddr>().unwrap())
        );
        // A route hint cannot manufacture an interface or admit a public address.
        assert!(
            select_addresses(
                "0.0.0.0:0".parse().unwrap(),
                [],
                Some("8.8.8.8".parse().unwrap())
            )
            .unwrap()
            .is_empty()
        );
    }

    #[test]
    fn explicit_bind_still_overrides_discovery_and_rejects_public_addresses() {
        assert_eq!(
            select_addresses("192.168.1.106:9845".parse().unwrap(), [], None).unwrap(),
            ["192.168.1.106".parse::<IpAddr>().unwrap()]
        );
        assert!(select_addresses("8.8.8.8:9845".parse().unwrap(), [], None).is_err());
    }

    #[tokio::test]
    async fn ranges_reuse_one_verified_open_handle_and_metadata_never_hashes() {
        use std::io::Cursor;
        use std::sync::atomic::{AtomicUsize, Ordering};
        struct Counted {
            source: Cursor<Vec<u8>>,
            count: Arc<AtomicUsize>,
        }
        impl Read for Counted {
            fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
                let n = self.source.read(out)?;
                self.count.fetch_add(n, Ordering::Relaxed);
                Ok(n)
            }
        }
        impl Seek for Counted {
            fn seek(&mut self, from: SeekFrom) -> std::io::Result<u64> {
                self.source.seek(from)
            }
        }
        let data = vec![0x43; CHUNK as usize * 2];
        let count = Arc::new(AtomicUsize::new(0));
        let resource = || MediaResource {
            visibility: Visibility::default(),
            reader: Box::new(Counted {
                source: Cursor::new(data.clone()),
                count: count.clone(),
            }),
            length: data.len() as u64,
            media_type: "audio/wav".into(),
            digest: Digest::sha256(&data),
            download_name: "fixture.wav".into(),
            verify_on_transfer: true,
        };
        let local = Arc::new(LocalPlayback {
            descriptor: RwLock::new(LocalPlaybackDescriptor {
                server_id: ServerId::new(),
                endpoints: vec![],
                remote_media_allowed: false,
            }),
            readers: Mutex::new(HashMap::new()),
            slots: Arc::new(tokio::sync::Semaphore::new(1)),
        });
        let head = bounded_read(
            local.clone(),
            "track".into(),
            resource(),
            true,
            None,
            None,
            None,
        )
        .await
        .map_err(|error| error.0)
        .unwrap();
        assert_eq!(head.0.status, 200);
        assert_eq!(count.load(Ordering::Relaxed), 0);
        let first = bounded_read(
            local.clone(),
            "track".into(),
            resource(),
            false,
            Some(0),
            Some(CHUNK - 1),
            None,
        )
        .await
        .map_err(|error| error.0)
        .unwrap();
        assert_eq!(first.1.len(), CHUNK as usize);
        assert_eq!(count.load(Ordering::Relaxed), data.len() + CHUNK as usize);
        let second = bounded_read(
            local.clone(),
            "track".into(),
            resource(),
            false,
            Some(CHUNK),
            Some(CHUNK * 2 - 1),
            None,
        )
        .await
        .map_err(|error| error.0)
        .unwrap();
        assert_eq!(second.1.len(), CHUNK as usize);
        assert_eq!(
            count.load(Ordering::Relaxed),
            data.len() + CHUNK as usize * 2
        );
        let invalid = bounded_read(
            local,
            "track".into(),
            resource(),
            false,
            Some(0),
            Some(CHUNK),
            None,
        )
        .await
        .map_err(|error| error.0)
        .unwrap();
        assert_eq!(invalid.0.status, 416);
        assert_eq!(
            count.load(Ordering::Relaxed),
            data.len() + CHUNK as usize * 2
        );
    }
    #[test]
    fn prepared_media_validation_stops_and_rejects_changed_or_truncated_bytes() {
        use std::io::Cursor;
        use std::sync::atomic::AtomicBool;
        let bytes = b"immutable publication";
        let digest = Digest::sha256(bytes);
        assert!(
            verify_open_reader(
                &mut Cursor::new(bytes),
                bytes.len() as u64,
                &digest,
                &AtomicBool::new(false)
            )
            .is_ok()
        );
        assert!(matches!(
            verify_open_reader(
                &mut Cursor::new(bytes),
                bytes.len() as u64,
                &digest,
                &AtomicBool::new(true)
            ),
            Err(Error::JobCancelled)
        ));
        assert!(matches!(
            verify_open_reader(
                &mut Cursor::new(b"changed"),
                bytes.len() as u64,
                &digest,
                &AtomicBool::new(false)
            ),
            Err(Error::PreparedMediaInvalid)
        ));
    }
    #[tokio::test]
    async fn certificate_rollover_refreshes_both_listeners_and_rejects_old_trust() {
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
        let root = tempfile::tempdir().unwrap();
        let store =
            Arc::new(Store::open(root.path().join("archive"), root.path().join("state")).unwrap());
        let id = store.server_id().unwrap();
        let mut state = AppState::new(
            store,
            Pairing {
                server_id: id,
                certificate_pem: String::new(),
                certificate_sha256: String::new(),
                endpoints: vec![],
            },
        );
        let origin = "https://collector.example";
        state.allowed_origins = Arc::new(vec![origin.into()]);
        let runtime = start(&mut state, "127.0.0.1:0".parse().unwrap())
            .await
            .unwrap();
        let local = state.local_playback.as_ref().unwrap();
        let before = local.descriptor(false).endpoints.remove(0);
        fn client(endpoint: &LocalPlaybackEndpoint) -> reqwest::Client {
            reqwest::Client::builder()
                .https_only(true)
                .tls_built_in_root_certs(false)
                .add_root_certificate(
                    reqwest::Certificate::from_pem(endpoint.certificate_pem.as_bytes()).unwrap(),
                )
                .pool_max_idle_per_host(0)
                .timeout(Duration::from_secs(3))
                .build()
                .unwrap()
        }
        async fn connect(endpoint: &LocalPlaybackEndpoint, origin: &str) -> bool {
            let hash = wtransport::tls::Sha256Digest::new(
                hex::decode(&endpoint.certificate_sha256)
                    .unwrap()
                    .try_into()
                    .unwrap(),
            );
            let client = Endpoint::client(
                wtransport::ClientConfig::builder()
                    .with_bind_default()
                    .with_server_certificate_hashes([hash])
                    .build(),
            )
            .unwrap();
            let result = tokio::time::timeout(
                Duration::from_secs(3),
                client.connect(
                    wtransport::endpoint::ConnectOptions::builder(&endpoint.webtransport_url)
                        .add_header("origin", origin)
                        .build(),
                ),
            )
            .await;
            result.is_ok_and(|result| result.is_ok())
        }
        let old_client = client(&before);
        assert!(
            old_client
                .get(format!("{}/api/v1/identity", before.https_url))
                .send()
                .await
                .unwrap()
                .status()
                .is_success()
        );
        assert!(connect(&before, origin).await);
        runtime
            .rotation
            .refresh(&["127.0.0.1".parse().unwrap()])
            .await
            .unwrap();
        let after = local.descriptor(false).endpoints.remove(0);
        assert_ne!(before.certificate_sha256, after.certificate_sha256);
        assert_eq!(before.https_url, after.https_url);
        assert!(
            old_client
                .get(format!("{}/api/v1/identity", before.https_url))
                .send()
                .await
                .is_err()
        );
        assert!(!connect(&before, origin).await);
        assert!(
            client(&after)
                .get(format!("{}/api/v1/identity", after.https_url))
                .send()
                .await
                .unwrap()
                .status()
                .is_success()
        );
        assert!(connect(&after, origin).await);
        let response = client(&after)
            .get(format!("{}/api/v1/admin/collections", after.https_url))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 404);
    }
}
