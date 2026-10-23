//! Restricted HTTP delivery. No Collector application, store, provider or account database.
use axum::{
    Router,
    body::{Body, Bytes},
    extract::{Query, Request, State},
    http::{HeaderMap, HeaderValue, Method, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
};
use collector_api_types::{MediaAuthorizeRequest, MediaFile, MediaHandleRequest, MediaReadTicket};
use collector_core::Digest;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sha2::{Digest as _, Sha256};
use std::{
    collections::HashMap,
    io::{Read, Seek, SeekFrom},
    path::Path,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant, SystemTime},
};

const CHUNK: usize = 256 * 1024;
#[derive(Clone)]
struct Authority {
    client: reqwest::Client,
    key: Arc<String>,
}
impl Authority {
    async fn call<T: Serialize, R: DeserializeOwned>(
        &self,
        path: &str,
        input: &T,
    ) -> Result<R, StatusCode> {
        let mut response = self
            .client
            .post(format!("http://authority{path}"))
            .bearer_auth(self.key.as_str())
            .header("content-type", "application/json")
            .body(serde_json::to_vec(input).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?)
            .send()
            .await
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        if !response.status().is_success() {
            return Err(match response.status().as_u16() {
                401 => StatusCode::UNAUTHORIZED,
                403 | 404 => StatusCode::FORBIDDEN,
                429 => StatusCode::TOO_MANY_REQUESTS,
                _ => StatusCode::SERVICE_UNAVAILABLE,
            });
        }
        if response.status() == reqwest::StatusCode::NO_CONTENT {
            return serde_json::from_str("null").map_err(|_| StatusCode::BAD_GATEWAY);
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| StatusCode::BAD_GATEWAY)?
        {
            if bytes.len() + chunk.len() > 16384 {
                return Err(StatusCode::BAD_GATEWAY);
            }
            bytes.extend_from_slice(&chunk);
        }
        serde_json::from_slice(&bytes).map_err(|_| StatusCode::BAD_GATEWAY)
    }
    async fn recheck(&self, handle: &str) -> Result<(), StatusCode> {
        self.call(
            "/recheck",
            &MediaHandleRequest {
                handle: handle.into(),
            },
        )
        .await
    }
    async fn release(&self, handle: &str) {
        let _: Result<(), _> = self
            .call(
                "/release",
                &MediaHandleRequest {
                    handle: handle.into(),
                },
            )
            .await;
    }
}
struct FileHandle {
    invalid: AtomicBool,
    file: Mutex<std::fs::File>,
    length: u64,
    modified: SystemTime,
    validated: tokio::sync::Mutex<bool>,
}
impl FileHandle {
    async fn bytes(self: Arc<Self>, position: u64, count: usize) -> Result<Vec<u8>, StatusCode> {
        tokio::task::spawn_blocking(move || {
            let mut file = self
                .file
                .lock()
                .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
            let metadata = file.metadata().map_err(|_| StatusCode::CONFLICT)?;
            if metadata.len() != self.length || metadata.modified().ok() != Some(self.modified) {
                self.invalid.store(true, Ordering::Release);
                return Err(StatusCode::CONFLICT);
            }
            file.seek(SeekFrom::Start(position))
                .map_err(|_| StatusCode::CONFLICT)?;
            let mut bytes = vec![0; count];
            file.read_exact(&mut bytes)
                .map_err(|_| StatusCode::CONFLICT)?;
            Ok(bytes)
        })
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
    }
}
#[derive(Clone)]
pub struct MediaState {
    root: Arc<cap_std::fs::Dir>,
    authority: Authority,
    origins: Arc<Vec<String>>,
    files: Arc<Mutex<HashMap<String, Arc<FileHandle>>>>,
    slots: Arc<tokio::sync::Semaphore>,
    admissions: Arc<tokio::sync::Semaphore>,
}
impl MediaState {
    pub fn new(
        archive: &Path,
        socket: &Path,
        key: String,
        origins: Vec<String>,
    ) -> anyhow::Result<Self> {
        anyhow::ensure!(
            key.len() == 64 && key.bytes().all(|b| b.is_ascii_hexdigit()),
            "invalid service key"
        );
        Ok(Self {
            root: Arc::new(cap_std::fs::Dir::open_ambient_dir(
                archive,
                cap_std::ambient_authority(),
            )?),
            authority: Authority {
                client: reqwest::Client::builder()
                    .no_proxy()
                    .unix_socket(socket)
                    .redirect(reqwest::redirect::Policy::none())
                    .timeout(Duration::from_secs(2))
                    .build()?,
                key: Arc::new(key),
            },
            origins: Arc::new(origins),
            files: Arc::new(Mutex::new(HashMap::new())),
            slots: Arc::new(tokio::sync::Semaphore::new(32)),
            admissions: Arc::new(tokio::sync::Semaphore::new(8)),
        })
    }
    async fn file(&self, metadata: &MediaFile) -> Result<Option<Arc<FileHandle>>, StatusCode> {
        if metadata.inline.is_some() {
            return if metadata.relative_path.is_none()
                && metadata.inline.as_ref().is_some_and(|s| {
                    s.len() <= 4096
                        && s.len() as u64 == metadata.length
                        && Digest::sha256(s.as_bytes()) == metadata.digest
                }) {
                Ok(None)
            } else {
                Err(StatusCode::BAD_GATEWAY)
            };
        }
        let path = metadata
            .relative_path
            .as_ref()
            .filter(|s| s.len() <= 4096 && !s.is_empty())
            .ok_or(StatusCode::BAD_GATEWAY)?;
        if Path::new(path).is_absolute()
            || Path::new(path)
                .components()
                .any(|c| !matches!(c, std::path::Component::Normal(_)))
        {
            return Err(StatusCode::FORBIDDEN);
        }
        let cache_key = format!("{}:{}:{}", path, metadata.length, metadata.digest.hex);
        let cached = self
            .files
            .lock()
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
            .get(&cache_key)
            .cloned();
        if let Some(file) = cached {
            // HEAD checks metadata too. Evict changed handles so a repaired file can recover.
            if !file.invalid.load(Ordering::Acquire) && file.clone().bytes(0, 0).await.is_ok() {
                return Ok(Some(file));
            }
            self.files
                .lock()
                .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
                .remove(&cache_key);
        }
        let root = self.root.clone();
        let path = path.clone();
        let length = metadata.length;
        let file = tokio::task::spawn_blocking(move || {
            use cap_std::fs::OpenOptionsExt;
            let mut options = cap_std::fs::OpenOptions::new();
            options.read(true).custom_flags(libc::O_NONBLOCK);
            let file = root
                .open_with(path, &options)
                .map_err(|_| StatusCode::NOT_FOUND)?
                .into_std();
            let meta = file.metadata().map_err(|_| StatusCode::CONFLICT)?;
            if !meta.is_file() || meta.len() != length {
                return Err(StatusCode::CONFLICT);
            }
            Ok(Arc::new(FileHandle {
                invalid: AtomicBool::new(false),
                file: Mutex::new(file),
                length,
                modified: meta.modified().map_err(|_| StatusCode::CONFLICT)?,
                validated: tokio::sync::Mutex::new(false),
            }))
        })
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)??;
        let mut files = self
            .files
            .lock()
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        if files.len() >= 128 {
            if let Some(old) = files
                .iter()
                .find(|(_, v)| Arc::strong_count(v) == 1)
                .map(|(k, _)| k.clone())
            {
                files.remove(&old);
            } else {
                return Err(StatusCode::TOO_MANY_REQUESTS);
            }
        }
        Ok(Some(files.entry(cache_key).or_insert(file).clone()))
    }
}
pub fn router(state: MediaState) -> Router {
    Router::new()
        .route("/read", get(read).options(preflight))
        .route("/api/v1/identity", get(identity))
        .layer(middleware::from_fn_with_state(state.clone(), limits))
        .with_state(state)
}
async fn limits(State(state): State<MediaState>, request: Request, next: Next) -> Response {
    if request.uri().to_string().len() > 1024
        || request.headers().len() > 32
        || request.headers().iter().any(|(_, v)| v.len() > 4096)
        || request.headers().contains_key(header::COOKIE)
        || request.headers().contains_key(header::TRANSFER_ENCODING)
        || request
            .headers()
            .get(header::CONTENT_LENGTH)
            .is_some_and(|v| v != "0")
    {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let Ok(_permit) = state.admissions.clone().try_acquire_owned() else {
        return StatusCode::TOO_MANY_REQUESTS.into_response();
    };
    match tokio::time::timeout(Duration::from_secs(30), next.run(request)).await {
        Ok(mut response) => {
            response.headers_mut().insert(
                header::CACHE_CONTROL,
                HeaderValue::from_static("private, no-store"),
            );
            response.headers_mut().insert(
                header::X_CONTENT_TYPE_OPTIONS,
                HeaderValue::from_static("nosniff"),
            );
            response
        }
        Err(_) => StatusCode::GATEWAY_TIMEOUT.into_response(),
    }
}
async fn identity(State(state): State<MediaState>, headers: HeaderMap) -> Response {
    let origin = headers.get(header::ORIGIN).and_then(|v| v.to_str().ok());
    if origin.is_some_and(|o| !state.origins.iter().any(|x| x == o)) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let response = state
        .authority
        .client
        .get("http://authority/identity")
        .bearer_auth(state.authority.key.as_str())
        .send()
        .await;
    match response {
        Ok(response) if response.status().is_success() => {
            let mut response = response;
            let mut bytes = Vec::new();
            loop {
                let chunk = match response.chunk().await {
                    Ok(Some(chunk)) => chunk,
                    Ok(None) => break,
                    Err(_) => return StatusCode::BAD_GATEWAY.into_response(),
                };
                if bytes.len() + chunk.len() > 4096 {
                    return StatusCode::BAD_GATEWAY.into_response();
                }
                bytes.extend_from_slice(&chunk);
            }
            let mut output = ([(header::CONTENT_TYPE, "application/json")], bytes).into_response();
            cors(output.headers_mut(), origin);
            output
        }
        _ => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}
fn cors(headers: &mut HeaderMap, origin: Option<&str>) {
    if let Some(origin) = origin.and_then(|v| HeaderValue::from_str(v).ok()) {
        headers.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, origin);
        headers.insert(header::VARY, HeaderValue::from_static("Origin"));
        headers.insert(
            header::ACCESS_CONTROL_EXPOSE_HEADERS,
            HeaderValue::from_static("ETag, Content-Range, Content-Length, Accept-Ranges"),
        );
    }
}
async fn preflight(State(state): State<MediaState>, headers: HeaderMap) -> Response {
    let origin = headers.get(header::ORIGIN).and_then(|v| v.to_str().ok());
    if !origin.is_some_and(|o| state.origins.iter().any(|x| x == o)) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let mut response = StatusCode::NO_CONTENT.into_response();
    cors(response.headers_mut(), origin);
    response.headers_mut().insert(
        header::ACCESS_CONTROL_ALLOW_METHODS,
        HeaderValue::from_static("GET, HEAD"),
    );
    response.headers_mut().insert(
        header::ACCESS_CONTROL_ALLOW_HEADERS,
        HeaderValue::from_static("Authorization, Range, If-Range"),
    );
    response
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Capability {
    grant: Option<String>,
}
/// RFC single ranges only. A missing range and mismatched If-Range use full GET.
pub fn range(value: Option<&str>, length: u64) -> Result<Option<(u64, u64)>, StatusCode> {
    let Some(value) = value else { return Ok(None) };
    let (first, last) = value
        .strip_prefix("bytes=")
        .and_then(|s| s.split_once('-'))
        .ok_or(StatusCode::RANGE_NOT_SATISFIABLE)?;
    if length == 0 {
        return Err(StatusCode::RANGE_NOT_SATISFIABLE);
    }
    let number = |value: &str| {
        if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
            return Err(StatusCode::RANGE_NOT_SATISFIABLE);
        }
        value
            .parse::<u64>()
            .map_err(|_| StatusCode::RANGE_NOT_SATISFIABLE)
    };
    let (start, end) = if first.is_empty() {
        let n = number(last)
            .ok()
            .filter(|n| *n > 0)
            .ok_or(StatusCode::RANGE_NOT_SATISFIABLE)?;
        (length.saturating_sub(n), length - 1)
    } else {
        let start = number(first)?;
        let end = if last.is_empty() {
            length - 1
        } else {
            number(last)?.min(length - 1)
        };
        (start, end)
    };
    if start > end || start >= length {
        return Err(StatusCode::RANGE_NOT_SATISFIABLE);
    }
    Ok(Some((start, end)))
}
async fn read(
    State(state): State<MediaState>,
    method: Method,
    headers: HeaderMap,
    Query(query): Query<Capability>,
) -> Response {
    let origin = headers
        .get(header::ORIGIN)
        .and_then(|v| v.to_str().ok())
        .filter(|o| state.origins.iter().any(|x| x == o))
        .map(str::to_owned);
    let mut response = match read_inner(state, method, headers, query).await {
        Ok(response) => response,
        Err(status) => status.into_response(),
    };
    cors(response.headers_mut(), origin.as_deref());
    response
}
struct Stream {
    state: MediaState,
    ticket: MediaReadTicket,
    file: Option<Arc<FileHandle>>,
    position: u64,
    end: u64,
    _permit: tokio::sync::OwnedSemaphorePermit,
    deadline: Instant,
    revoked: tokio::sync::watch::Receiver<bool>,
    monitor: tokio::task::JoinHandle<()>,
}
impl Drop for Stream {
    fn drop(&mut self) {
        self.monitor.abort();
        let authority = self.state.authority.clone();
        let handle = self.ticket.handle.clone();
        tokio::spawn(async move {
            authority.release(&handle).await;
        });
    }
}
async fn read_inner(
    state: MediaState,
    method: Method,
    headers: HeaderMap,
    query: Capability,
) -> Result<Response, StatusCode> {
    let bearer = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));
    if bearer.is_some() && query.grant.is_some() {
        return Err(StatusCode::BAD_REQUEST);
    }
    let token = bearer
        .or(query.grant.as_deref())
        .filter(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()))
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let origin = headers
        .get(header::ORIGIN)
        .map(|v| v.to_str().map(str::to_owned))
        .transpose()
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    if origin.as_ref().is_some_and(|o| !state.origins.contains(o)) {
        return Err(StatusCode::FORBIDDEN);
    }
    let permit = state
        .slots
        .clone()
        .try_acquire_owned()
        .map_err(|_| StatusCode::TOO_MANY_REQUESTS)?;
    let ticket: MediaReadTicket = state
        .authority
        .call(
            "/authorize",
            &MediaAuthorizeRequest {
                token: token.into(),
                origin: origin.clone(),
            },
        )
        .await?;
    let (denied, revoked) = tokio::sync::watch::channel(false);
    let authority = state.authority.clone();
    let handle = ticket.handle.clone();
    let monitor = tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(5)).await;
            if authority.recheck(&handle).await.is_err() {
                let _ = denied.send(true);
                break;
            }
        }
    });
    // RAII covers metadata/validation errors as well as a detached response body.
    let mut stream = Stream {
        state: state.clone(),
        ticket,
        file: None,
        position: 0,
        end: 0,
        _permit: permit,
        deadline: Instant::now() + Duration::from_secs(4 * 3600),
        revoked,
        monitor,
    };
    let info = stream.ticket.file.clone();
    let etag = format!("\"{}\"", info.digest.hex);
    stream.file = state.file(&info).await?;
    let requested = if headers
        .get(header::IF_RANGE)
        .is_some_and(|v| v.to_str().ok() != Some(&etag))
    {
        None
    } else {
        headers
            .get(header::RANGE)
            .map(|v| v.to_str().map_err(|_| StatusCode::RANGE_NOT_SATISFIABLE))
            .transpose()?
    };
    let selected = match range(
        if method == Method::HEAD {
            None
        } else {
            requested
        },
        info.length,
    ) {
        Ok(value) => value,
        Err(status) => {
            let mut response = status.into_response();
            response.headers_mut().insert(
                header::CONTENT_RANGE,
                HeaderValue::from_str(&format!("bytes */{}", info.length))
                    .map_err(|_| StatusCode::BAD_GATEWAY)?,
            );
            cors(response.headers_mut(), origin.as_deref());
            return Ok(response);
        }
    };
    let (start, end) = selected.unwrap_or((0, info.length.saturating_sub(1)));
    let count = if info.length == 0 { 0 } else { end - start + 1 };
    if method != Method::HEAD
        && info.verify_on_transfer
        && let Some(file) = stream.file.clone()
    {
        let mut validated = file.validated.lock().await;
        if !*validated {
            let mut hash = Sha256::new();
            let mut position = 0;
            while position < info.length {
                state.authority.recheck(&stream.ticket.handle).await?;
                let bytes = file
                    .clone()
                    .bytes(
                        position,
                        ((info.length - position).min(CHUNK as u64)) as usize,
                    )
                    .await?;
                hash.update(&bytes);
                position += bytes.len() as u64;
            }
            if hex::encode(hash.finalize()) != info.digest.hex {
                file.invalid.store(true, Ordering::Release);
                return Err(StatusCode::CONFLICT);
            }
            *validated = true;
        }
    }
    stream.position = start;
    stream.end = if info.length == 0 { 0 } else { end + 1 };
    let body = if method == Method::HEAD {
        Body::empty()
    } else {
        Body::from_stream(futures_util::stream::try_unfold(
            stream,
            |mut stream| async move {
                if stream.position >= stream.end {
                    return Ok::<_, std::io::Error>(None);
                }
                if *stream.revoked.borrow() || Instant::now() > stream.deadline {
                    return Err(std::io::Error::other("media deadline"));
                }
                stream
                    .state
                    .authority
                    .recheck(&stream.ticket.handle)
                    .await
                    .map_err(|_| std::io::Error::other("media authorization unavailable"))?;
                let count = (stream.end - stream.position).min(CHUNK as u64) as usize;
                let bytes = if let Some(file) = &stream.file {
                    file.clone()
                        .bytes(stream.position, count)
                        .await
                        .map_err(|_| std::io::Error::other("media changed"))?
                } else {
                    let inline = stream
                        .ticket
                        .file
                        .inline
                        .as_ref()
                        .ok_or_else(|| std::io::Error::other("invalid media source"))?;
                    inline.as_bytes()[stream.position as usize..stream.position as usize + count]
                        .to_vec()
                };
                stream.position += count as u64;
                Ok(Some((Bytes::from(bytes), stream)))
            },
        ))
    };
    let mut response = Response::new(body);
    *response.status_mut() = if selected.is_some() {
        StatusCode::PARTIAL_CONTENT
    } else {
        StatusCode::OK
    };
    let output = response.headers_mut();
    output.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(&info.media_type).map_err(|_| StatusCode::BAD_GATEWAY)?,
    );
    output.insert(
        header::CONTENT_LENGTH,
        HeaderValue::from_str(
            &if method == Method::HEAD {
                info.length
            } else {
                count
            }
            .to_string(),
        )
        .map_err(|_| StatusCode::BAD_GATEWAY)?,
    );
    output.insert(
        header::ETAG,
        HeaderValue::from_str(&etag).map_err(|_| StatusCode::BAD_GATEWAY)?,
    );
    output.insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));
    if selected.is_some() {
        output.insert(
            header::CONTENT_RANGE,
            HeaderValue::from_str(&format!("bytes {start}-{end}/{}", info.length))
                .map_err(|_| StatusCode::BAD_GATEWAY)?,
        );
    }
    cors(output, origin.as_deref());
    Ok(response)
}

/// Shared connection mechanism for public delivery and private authority sockets.
/// Header deadlines also bound clients that trickle bytes instead of staying idle.
pub async fn connection<I>(io: I, router: Router)
where
    I: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    let mut io = tokio_io_timeout::TimeoutStream::new(io);
    io.set_read_timeout(Some(Duration::from_secs(30)));
    io.set_write_timeout(Some(Duration::from_secs(30)));
    let mut builder = hyper::server::conn::http1::Builder::new();
    builder
        .timer(hyper_util::rt::TokioTimer::new())
        .header_read_timeout(Duration::from_secs(10))
        .max_headers(32)
        .max_buf_size(16 * 1024);
    let _ = tokio::time::timeout(
        Duration::from_secs(4 * 3600),
        builder.serve_connection(
            hyper_util::rt::TokioIo::new(Box::pin(io)),
            hyper_util::service::TowerToHyperService::new(router),
        ),
    )
    .await;
}

/// Deployment guard: require the archive and every visible nested mount to be read-only.
/// Host administrators can still change mounts; the media process cannot (no capabilities).
#[cfg(target_os = "linux")]
pub fn require_read_only_archive(archive: &Path) -> anyhow::Result<()> {
    let archive = std::fs::canonicalize(archive)?;
    let mut bytes = Vec::new();
    std::fs::File::open("/proc/self/mountinfo")?
        .take(1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    anyhow::ensure!(bytes.len() <= 1024 * 1024, "mount table exceeds limit");
    read_only_mounts(&archive, std::str::from_utf8(&bytes)?)
}
#[cfg(not(target_os = "linux"))]
pub fn require_read_only_archive(_archive: &Path) -> anyhow::Result<()> {
    anyhow::bail!("read-only archive qualification requires Linux mount namespaces")
}
#[cfg(any(target_os = "linux", test))]
fn read_only_mounts(archive: &Path, mountinfo: &str) -> anyhow::Result<()> {
    let mut parent: Option<(usize, bool)> = None;
    for line in mountinfo.lines() {
        let fields: Vec<_> = line.split_whitespace().collect();
        anyhow::ensure!(
            fields.len() >= 10 && fields.contains(&"-"),
            "malformed mount table"
        );
        let mut mount = fields[4].to_owned();
        // proc(5) escapes whitespace and backslashes in mount paths.
        for (encoded, value) in [
            ("\\040", " "),
            ("\\011", "\t"),
            ("\\012", "\n"),
            ("\\134", "\\"),
        ] {
            mount = mount.replace(encoded, value);
        }
        let mount = Path::new(&mount);
        anyhow::ensure!(mount.is_absolute(), "invalid mount path");
        let readonly = fields[5].split(',').any(|flag| flag == "ro");
        if mount.starts_with(archive) {
            anyhow::ensure!(
                readonly,
                "archive contains a writable mount: {}",
                mount.display()
            );
        }
        if archive.starts_with(mount) {
            let depth = mount.components().count();
            if parent.is_none_or(|(old, _)| depth >= old) {
                parent = Some((depth, readonly));
            }
        }
    }
    anyhow::ensure!(
        parent.is_some_and(|(_, readonly)| readonly),
        "archive filesystem is writable"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ranges_are_exact_and_fail_closed() {
        assert_eq!(range(Some("bytes=-1"), 1), Ok(Some((0, 0))));
        assert_eq!(range(Some("bytes=1-"), 3), Ok(Some((1, 2))));
        assert_eq!(range(Some("bytes=0-99"), 3), Ok(Some((0, 2))));
        for value in [
            "bytes=-0",
            "bytes=+1-2",
            "bytes=0-+2",
            "bytes=3-",
            "bytes=2-1",
            "bytes=0-1,2-3",
            "bytes=18446744073709551616-",
        ] {
            assert!(range(Some(value), 3).is_err());
        }
        assert!(range(Some("bytes=0-0"), 0).is_err());
    }
    #[tokio::test]
    async fn archive_handles_reject_escape_special_files_and_changed_payloads() {
        use std::os::unix::fs::symlink;
        let root = tempfile::tempdir().unwrap();
        let archive = root.path().join("archive");
        std::fs::create_dir(&archive).unwrap();
        std::fs::write(root.path().join("secret"), b"outside").unwrap();
        std::fs::write(archive.join("track"), b"original").unwrap();
        symlink(root.path().join("secret"), archive.join("escape")).unwrap();
        let state = MediaState::new(
            &archive,
            &root.path().join("socket"),
            "0".repeat(64),
            vec![],
        )
        .unwrap();
        let mut info = MediaFile {
            relative_path: Some("track".into()),
            inline: None,
            length: 8,
            digest: Digest::sha256(b"original"),
            media_type: "audio/flac".into(),
            verify_on_transfer: false,
        };
        let file = state.file(&info).await.unwrap().unwrap();
        assert_eq!(file.clone().bytes(0, 8).await.unwrap(), b"original");
        std::fs::write(archive.join("track"), b"truncated").unwrap();
        assert!(file.clone().bytes(0, 8).await.is_err());
        assert!(
            state.file(&info).await.is_err(),
            "HEAD retained changed metadata"
        );
        std::fs::write(archive.join("track"), b"original").unwrap();
        assert_eq!(
            state
                .file(&info)
                .await
                .unwrap()
                .unwrap()
                .bytes(0, 8)
                .await
                .unwrap(),
            b"original"
        );
        for path in ["../secret", "/etc/passwd", "escape", ".", ""] {
            info.relative_path = Some(path.into());
            assert!(state.file(&info).await.is_err(), "accepted {path}");
        }
        let fifo = archive.join("pipe");
        let name = std::ffi::CString::new(fifo.as_os_str().as_encoded_bytes()).unwrap();
        // The fixture is private to this test; no host device is opened.
        assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
        info.relative_path = Some("pipe".into());
        info.length = 0;
        assert!(
            tokio::time::timeout(Duration::from_secs(1), state.file(&info))
                .await
                .unwrap()
                .is_err(),
            "FIFO blocked or opened"
        );
        info.relative_path = None;
        info.inline = Some("probe".into());
        info.length = 5;
        assert!(
            state.file(&info).await.is_err(),
            "unverified inline bytes accepted"
        );
    }
    #[test]
    fn mount_guard_checks_effective_parent_and_nested_mounts() {
        let root = "1 0 0:1 / / ro - overlay overlay rw\n";
        let archive = "2 1 0:2 / /media ro - ext4 /dev/a rw\n";
        let nested = "3 2 0:3 / /media/nested rw - ext4 /dev/b rw\n";
        assert!(read_only_mounts(Path::new("/media"), &format!("{root}{archive}")).is_ok());
        assert!(
            read_only_mounts(Path::new("/media"), &format!("{root}{archive}{nested}")).is_err()
        );
        assert!(
            read_only_mounts(
                Path::new("/media"),
                &archive.replace("/media ro", "/media rw")
            )
            .is_err()
        );
        assert!(read_only_mounts(Path::new("/media"), "malformed").is_err());
        assert!(
            read_only_mounts(
                Path::new("/media library"),
                &format!("{root}2 1 0:2 / /media\\040library ro - ext4 /dev/a rw\n")
            )
            .is_ok()
        );
    }
}
