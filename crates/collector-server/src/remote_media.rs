//! Public grant issuance and a separately composed, socket-only authority adapter.
use super::*;
use axum::routing::post;
use collector_api_types::{
    MediaAuthorizeRequest, MediaHandleRequest, MediaReadTicket, RemoteGrantRequest,
    RemoteMediaGrant,
};
use std::{
    io::Write,
    os::unix::fs::{FileTypeExt, PermissionsExt},
    path::PathBuf,
};

#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Configuration {
    pub base_url: String,
    pub socket: PathBuf,
}
impl Configuration {
    pub fn validate(&self) -> anyhow::Result<()> {
        let url = url::Url::parse(&self.base_url)?;
        anyhow::ensure!(
            url.scheme() == "https"
                && url.host_str().is_some()
                && url.username().is_empty()
                && url.password().is_none()
                && url.query().is_none()
                && url.fragment().is_none()
                && self.base_url.ends_with('/')
                && !url.path().contains(".."),
            "remote media requires a fixed HTTPS base URL ending in / without credentials, query or fragment"
        );
        anyhow::ensure!(
            self.socket.is_absolute() && self.socket.file_name().is_some(),
            "media authority requires an absolute socket path in a dedicated directory"
        );
        Ok(())
    }
}
pub struct Runtime {
    task: tokio::task::JoinHandle<()>,
    socket: PathBuf,
}
impl Drop for Runtime {
    fn drop(&mut self) {
        self.task.abort();
        let _ = std::fs::remove_file(&self.socket);
    }
}
pub async fn start(state: AppState, config: &Configuration) -> anyhow::Result<Runtime> {
    config.validate()?;
    let parent = config
        .socket
        .parent()
        .ok_or_else(|| anyhow::anyhow!("socket directory missing"))?;
    use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
    if !parent.exists() {
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o750)
            .create(parent)?;
    }
    let metadata = std::fs::symlink_metadata(parent)?;
    anyhow::ensure!(
        metadata.file_type().is_dir() && metadata.permissions().mode() & 0o022 == 0,
        "media authority directory must be a real dedicated directory without group/other write access"
    );
    let key_path = parent.join("service-key");
    if !key_path.exists() {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o640)
            .open(&key_path)?;
        file.set_permissions(std::fs::Permissions::from_mode(0o640))?;
        file.write_all(secret().as_bytes())?;
        file.sync_all()?;
    }
    let metadata = std::fs::symlink_metadata(&key_path)?;
    anyhow::ensure!(
        metadata.file_type().is_file() && metadata.permissions().mode() & 0o027 == 0,
        "media service key must be a regular owner-writable, optionally group-readable file"
    );
    let key = std::fs::read_to_string(key_path)?;
    anyhow::ensure!(
        key.len() == 64 && key.bytes().all(|b| b.is_ascii_hexdigit()),
        "invalid media service key"
    );
    if let Ok(metadata) = std::fs::symlink_metadata(&config.socket) {
        anyhow::ensure!(
            metadata.file_type().is_socket(),
            "refusing to replace non-socket authority path"
        );
        anyhow::ensure!(
            tokio::net::UnixStream::connect(&config.socket)
                .await
                .is_err(),
            "media authority socket already in use"
        );
        std::fs::remove_file(&config.socket)?;
    }
    let listener = tokio::net::UnixListener::bind(&config.socket)?;
    std::fs::set_permissions(&config.socket, std::fs::Permissions::from_mode(0o660))?;
    let digest = Digest::sha256(key.as_bytes()).hex;
    let slots = Arc::new(tokio::sync::Semaphore::new(32));
    let router = Router::new()
        .route("/authorize", post(authorize))
        .route("/recheck", post(recheck))
        .route("/release", post(release))
        .route("/identity", get(identity))
        .layer(DefaultBodyLimit::max(4096))
        .layer(middleware::from_fn(move |request: Request, next: Next| {
            let digest = digest.clone();
            let slots = slots.clone();
            async move {
                let supplied = request
                    .headers()
                    .get(header::AUTHORIZATION)
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.strip_prefix("Bearer "));
                let valid = supplied
                    .filter(|s| s.len() == 64)
                    .map(|s| Digest::sha256(s.as_bytes()).hex)
                    .is_some_and(|actual| {
                        actual
                            .bytes()
                            .zip(digest.bytes())
                            .fold(0u8, |value, (a, b)| value | (a ^ b))
                            == 0
                    });
                if !valid {
                    return StatusCode::UNAUTHORIZED.into_response();
                }
                let Ok(_permit) = slots.try_acquire_owned() else {
                    return StatusCode::TOO_MANY_REQUESTS.into_response();
                };
                next.run(request).await
            }
        }))
        .with_state(state);
    let task = tokio::spawn(async move {
        let mut tasks = tokio::task::JoinSet::new();
        loop {
            tokio::select! {
                incoming=listener.accept()=>match incoming {Ok((stream,_))=>{if tasks.len()<64 {tasks.spawn(collector_media::connection(stream,router.clone()));}},Err(error)=>{tracing::error!(%error,"media authority stopped");break;}},
                _=tasks.join_next(),if !tasks.is_empty()=>{},
            }
        }
    });
    Ok(Runtime {
        task,
        socket: config.socket.clone(),
    })
}
fn secret() -> String {
    format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
}
#[utoipa::path(post,path="/api/v1/playback/remote-test",request_body=RemoteGrantRequest,responses((status=200,body=collector_api_types::RemoteMediaTest),(status=401),(status=403)))]
pub(crate) async fn remote_test(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<RemoteGrantRequest>,
) -> ApiResult<Json<collector_api_types::RemoteMediaTest>> {
    let actor = authenticated(&state, &headers)?;
    let target = request.target.clone();
    let grant = issue_grant(&state, &headers, actor.clone(), request).await?;
    let samples = if let Some(target) = target {
        blocking(move || state.service.remote_media_samples(&actor, &target)).await?
    } else {
        Vec::new()
    };
    Ok(Json(collector_api_types::RemoteMediaTest {
        grant,
        samples,
    }))
}
#[utoipa::path(post,path="/api/v1/playback/remote-grants",request_body=RemoteGrantRequest,responses((status=200,body=RemoteMediaGrant),(status=401),(status=403),(status=503)))]
pub(crate) async fn remote_grant(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<RemoteGrantRequest>,
) -> ApiResult<Json<RemoteMediaGrant>> {
    let actor = authenticated(&state, &headers)?;
    Ok(Json(issue_grant(&state, &headers, actor, request).await?))
}
async fn issue_grant(
    state: &AppState,
    headers: &HeaderMap,
    actor: collector_app::AuthenticatedPrincipal,
    request: RemoteGrantRequest,
) -> ApiResult<RemoteMediaGrant> {
    let remote = state.remote_media.as_ref().ok_or(Error::Unavailable(
        "Direct HTTPS media is not configured".into(),
    ))?;
    let origin = if actor.credential_kind == collector_app::CredentialKind::BrowserSession {
        Some(
            headers
                .get(header::ORIGIN)
                .and_then(|v| v.to_str().ok())
                .and_then(normalized_configured_origin)
                .filter(|o| state.allowed_origins.contains(o))
                .ok_or(Error::Forbidden)?,
        )
    } else {
        None
    };
    let service = state.service.clone();
    let audience = remote.base_url.clone();
    let server = state.pairing.server_id;
    blocking(move || {
        service.issue_remote_media(&actor, request.target, origin, &audience, server, secret())
    })
    .await
}
async fn authorize(
    State(state): State<AppState>,
    Json(request): Json<MediaAuthorizeRequest>,
) -> ApiResult<Json<MediaReadTicket>> {
    let audience = state
        .remote_media
        .as_ref()
        .ok_or(Error::Forbidden)?
        .base_url
        .clone();
    let service = state.service.clone();
    Ok(Json(
        blocking(move || {
            service.authorize_remote_media(
                &request.token,
                request.origin.as_deref(),
                &audience,
                secret(),
            )
        })
        .await?,
    ))
}
async fn recheck(
    State(state): State<AppState>,
    Json(request): Json<MediaHandleRequest>,
) -> ApiResult<StatusCode> {
    let audience = state
        .remote_media
        .as_ref()
        .ok_or(Error::Forbidden)?
        .base_url
        .clone();
    let service = state.service.clone();
    blocking(move || service.recheck_remote_media(&request.handle, &audience)).await?;
    Ok(StatusCode::NO_CONTENT)
}
async fn release(
    State(state): State<AppState>,
    Json(request): Json<MediaHandleRequest>,
) -> ApiResult<StatusCode> {
    blocking(move || state.service.release_remote_media(&request.handle)).await?;
    Ok(StatusCode::NO_CONTENT)
}
