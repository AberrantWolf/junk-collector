pub mod passkeys;
use axum::{
    Json, Router,
    body::Body,
    extract::{DefaultBodyLimit, Path, Query, Request, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
};
use collector_app::{CollectionService, PresentedItem};
use collector_client::Pairing;
use collector_core::*;
use collector_modules::{Availability, Registry};
use collector_store::{Store, UploadFile};
use serde::Deserialize;
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::io::{AsyncReadExt, AsyncSeekExt};

#[derive(Clone)]
pub struct AppState {
    pub store: Arc<Store>,
    pub service: CollectionService,
    pub exposure: Exposure,
    pub ingress: Ingress,
    pub pairing: Pairing,
    pub multi_user: bool,
    pub default_username: String,
    pub inboxes: Arc<std::collections::BTreeMap<String, std::path::PathBuf>>,
    pub allowed_origins: Arc<Vec<String>>,
    pub passkeys: Option<Arc<passkeys::Passkeys>>,
    pub login_attempts: Arc<Mutex<VecDeque<Instant>>>,
    pub jobs: Arc<tokio::sync::Semaphore>,
}
impl AppState {
    pub fn new(store: Arc<Store>, pairing: Pairing) -> Self {
        let service = CollectionService::new(store.clone(), Arc::new(Registry::builtins()));
        Self {
            store,
            service,
            exposure: Exposure::default(),
            ingress: Ingress::Public,
            pairing,
            multi_user: false,
            default_username: "owner".into(),
            inboxes: Arc::new(std::collections::BTreeMap::new()),
            allowed_origins: Arc::new(vec![]),
            login_attempts: Arc::new(Mutex::new(VecDeque::new())),
            passkeys: None,
            jobs: Arc::new(tokio::sync::Semaphore::new(2)),
        }
    }
}
pub struct ApiError(pub Error);
impl From<Error> for ApiError {
    fn from(e: Error) -> Self {
        Self(e)
    }
}
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self.0 {
            Error::Invalid(m) => (StatusCode::BAD_REQUEST, "invalid_request", m.clone()),
            Error::NotFound => (StatusCode::NOT_FOUND, "not_found", "Not found".into()),
            Error::Unauthenticated => (
                StatusCode::UNAUTHORIZED,
                "unauthenticated",
                "Sign in to continue".into(),
            ),
            Error::Forbidden => (
                StatusCode::FORBIDDEN,
                "forbidden",
                "You do not have access to this resource".into(),
            ),
            Error::Conflict => (
                StatusCode::CONFLICT,
                "revision_conflict",
                "The record or upload changed; refresh before retrying".into(),
            ),
            Error::Unavailable(m) => (StatusCode::SERVICE_UNAVAILABLE, "unavailable", m.clone()),
            Error::Storage(_) => {
                tracing::error!(error=%self.0,"storage operation failed");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "storage_error",
                    "Storage operation failed; check the server log".into(),
                )
            }
        };
        (
            status,
            Json(serde_json::json!({"error":{"code":code,"message":message}})),
        )
            .into_response()
    }
}
type ApiResult<T> = std::result::Result<T, ApiError>;
async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> collector_core::Result<T> + Send + 'static,
) -> ApiResult<T> {
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|_| ApiError(Error::Unavailable("worker stopped".into())))?
        .map_err(Into::into)
}
fn principal(state: &AppState, headers: &HeaderMap) -> ApiResult<Principal> {
    let bearer = headers
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "));
    let cookie = headers
        .get(header::COOKIE)
        .and_then(|h| h.to_str().ok())
        .and_then(|s| {
            s.split(';')
                .map(str::trim)
                .find_map(|p| p.strip_prefix("collector_session="))
        });
    Ok(state
        .store
        .authenticate(bearer.or(cookie).ok_or(Error::Unauthenticated)?)?)
}
fn administrator(actor: &Principal) -> ApiResult<()> {
    if actor.administrator {
        Ok(())
    } else {
        Err(Error::Forbidden.into())
    }
}

pub fn router(state: AppState) -> Router {
    let mut router = Router::new()
        .route(
            "/healthz",
            get(|| async { Json(serde_json::json!({"status":"ok"})) }),
        )
        .route("/", get(index))
        .route("/app.js", get(app_js))
        .route("/app.css", get(app_css))
        .route("/api/v1/identity", get(identity))
        .route("/api/v1/features", get(features))
        .route("/api/v1/auth/login", post(login))
        .route("/api/v1/auth/logout", post(logout))
        .route(
            "/api/v1/auth/passkeys/register/start",
            post(passkeys::register_start),
        )
        .route(
            "/api/v1/auth/passkeys/register/finish",
            post(passkeys::register_finish),
        )
        .route(
            "/api/v1/auth/passkeys/login/start",
            post(passkeys::login_start),
        )
        .route(
            "/api/v1/auth/passkeys/login/finish",
            post(passkeys::login_finish),
        )
        .route("/api/v1/me", get(me))
        .route("/api/v1/pairing", get(pairing))
        .route("/api/v1/devices/{id}", delete(revoke))
        .route("/api/v1/collections", get(collections))
        .route("/api/v1/items", get(browse))
        .route("/api/v1/items/{id}", get(item))
        .route("/api/v1/items/{id}/evidence", get(evidence))
        .route(
            "/api/v1/items/{id}/activity",
            get(activity).put(save_activity),
        )
        .route("/api/v1/items/{id}/assets/{asset}", get(asset))
        .route(
            "/api/v1/items/{id}/representations/{representation}/{index}",
            get(media),
        );
    if state.exposure.allows(state.ingress, Surface::Curation) {
        router = router
            .route("/curate", get(index))
            .route("/curation.js", get(curation_js))
            .route("/api/v1/items/{id}", put(save_item))
            .route(
                "/api/v1/items/{id}/cover",
                post(cover).layer(DefaultBodyLimit::max(20 * 1024 * 1024)),
            )
            .route("/api/v1/items/{id}/verify", post(verify))
            .route("/api/v1/uploads", post(create_upload))
            .route("/api/v1/uploads/{id}", get(upload).delete(cancel_upload))
            .route(
                "/api/v1/uploads/{id}/files/{index}",
                put(chunk).layer(DefaultBodyLimit::max(8 * 1024 * 1024)),
            )
            .route("/api/v1/uploads/{id}/commit", post(commit_upload))
            .route("/api/v1/jobs", get(jobs));
    }
    if state
        .exposure
        .allows(state.ingress, Surface::Administration)
    {
        router = router
            .route("/admin", get(index))
            .route("/administration.js", get(administration_js))
            .route("/api/v1/admin/settings", get(settings))
            .route("/api/v1/admin/inboxes", get(inboxes))
            .route("/api/v1/admin/inboxes/{name}/import", post(import_inbox))
            .route("/api/v1/admin/users", post(create_user))
            .route("/api/v1/admin/collections", post(create_collection))
            .route("/api/v1/admin/grants", post(grant))
            .route("/api/v1/admin/reindex", post(reindex));
    }
    router
        .layer(DefaultBodyLimit::max(2 * 1024 * 1024))
        .layer(middleware::from_fn_with_state(state.clone(), security))
        .with_state(state)
}
async fn security(State(state): State<AppState>, request: Request, next: Next) -> Response {
    let modifying = !matches!(
        *request.method(),
        axum::http::Method::GET | axum::http::Method::HEAD | axum::http::Method::OPTIONS
    );
    if modifying {
        let origin = request
            .headers()
            .get(header::ORIGIN)
            .and_then(|v| v.to_str().ok());
        if let Some(origin) = origin {
            if !state
                .allowed_origins
                .iter()
                .any(|allowed| allowed == origin)
            {
                return (StatusCode::FORBIDDEN, Json(serde_json::json!({"error": {"code": "origin_not_allowed", "message": "This browser address is not enabled in the server configuration. On the server, run doctor; for local management, stop the server, run setup, then restart and use the printed localhost URL. Remote HTTPS addresses must be added to allowed_origins."}}))).into_response();
            }
        } else if request.headers().contains_key(header::COOKIE) {
            return ApiError(Error::Forbidden).into_response();
        }
    }
    let mut response = next.run(request).await;
    for (key, value) in [
        ("x-content-type-options", "nosniff"),
        ("referrer-policy", "no-referrer"),
        ("x-frame-options", "DENY"),
        ("cache-control", "private, no-store"),
        (
            "content-security-policy",
            "default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' blob:; media-src 'self' blob:; connect-src 'self'; frame-ancestors 'none'; base-uri 'none'; form-action 'self'",
        ),
    ] {
        response
            .headers_mut()
            .insert(key, HeaderValue::from_static(value));
    }
    response
}
async fn index() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        include_str!("../../../web/dist/index.html"),
    )
}
async fn app_js() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
        include_str!("../../../web/dist/app.js"),
    )
}
async fn app_css() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        include_str!("../../../web/dist/app.css"),
    )
}
async fn curation_js() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
        include_str!("../../../web/dist/curation.js"),
    )
}
async fn administration_js() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
        include_str!("../../../web/dist/administration.js"),
    )
}
async fn identity(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(serde_json::json!({"server_id":state.pairing.server_id,"protocol_version":1}))
}
async fn features(State(state): State<AppState>) -> Json<serde_json::Value> {
    let modules = Registry::builtins()
        .descriptors()
        .into_iter()
        .map(|module| {
            let capabilities = module.capabilities.into_iter().map(|capability| {
                let (available, limitation) = match &capability.availability {
                    Availability::Available => (true, None),
                    Availability::Unavailable { limitation } => (false, Some(limitation.clone())),
                };
                serde_json::json!({
                    "name": capability.id.as_str().rsplit('.').next().unwrap_or_default().replace('-', "_"),
                    "available": available,
                    "limitation": limitation,
                    "id": capability.id,
                    "display_name": capability.display_name,
                    "availability": capability.availability,
                })
            }).collect::<Vec<_>>();
            serde_json::json!({"id":module.id,"schema_version":module.schema_version,"display_name":module.display_name,"capabilities":capabilities})
        })
        .collect::<Vec<_>>();
    Json(
        serde_json::json!({"default_username": if state.multi_user { None } else { Some(&state.default_username) },"multi_user":state.multi_user,"curation":state.exposure.allows(state.ingress,Surface::Curation),"administration":state.exposure.allows(state.ingress,Surface::Administration),"passkeys":state.passkeys.is_some(),"modules":modules}),
    )
}
#[derive(Deserialize)]
struct Login {
    username: String,
    password: String,
    #[serde(default)]
    device_name: String,
}
async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<Login>,
) -> ApiResult<Response> {
    {
        let mut attempts = state
            .login_attempts
            .lock()
            .map_err(|_| Error::Unavailable("authentication lock unavailable".into()))?;
        let now = Instant::now();
        while attempts
            .front()
            .is_some_and(|t| now.duration_since(*t) > Duration::from_secs(60))
        {
            attempts.pop_front();
        }
        if attempts.len() >= 20 {
            return Ok((StatusCode::TOO_MANY_REQUESTS,[(header::RETRY_AFTER,"60")],Json(serde_json::json!({"error":{"code":"rate_limited","message":"Too many sign-in attempts. Try again in one minute."}}))).into_response());
        }
        attempts.push_back(now);
    }
    let (token, actor) = blocking(move || {
        state
            .store
            .login(&input.username, &input.password, &input.device_name)
    })
    .await?;
    session_response(token, actor, &headers)
}
fn session_response(token: String, actor: Principal, headers: &HeaderMap) -> ApiResult<Response> {
    let native = headers
        .get("x-collector-client")
        .and_then(|v| v.to_str().ok())
        == Some("native");
    if native {
        return Ok(Json(serde_json::json!({"token":token,"principal":actor})).into_response());
    }
    let mut response = Json(serde_json::json!({"principal":actor})).into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        HeaderValue::from_str(&format!(
            "collector_session={token}; HttpOnly; Secure; SameSite=Strict; Path=/; Max-Age=2592000"
        ))
        .map_err(|_| Error::Invalid("invalid session".into()))?,
    );
    Ok(response)
}
async fn logout(State(state): State<AppState>, headers: HeaderMap) -> ApiResult<Response> {
    let actor = principal(&state, &headers)?;
    if let Some(device) = actor.device_id {
        state.store.revoke_device(&actor, device)?;
    }
    Ok((
        [(
            header::SET_COOKIE,
            "collector_session=; HttpOnly; Secure; SameSite=Strict; Path=/; Max-Age=0",
        )],
        StatusCode::NO_CONTENT,
    )
        .into_response())
}
async fn me(State(state): State<AppState>, headers: HeaderMap) -> ApiResult<Json<Principal>> {
    Ok(Json(principal(&state, &headers)?))
}
async fn pairing(State(state): State<AppState>, headers: HeaderMap) -> ApiResult<Json<Pairing>> {
    principal(&state, &headers)?;
    Ok(Json(state.pairing))
}
async fn revoke(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<DeviceId>,
) -> ApiResult<StatusCode> {
    let actor = principal(&state, &headers)?;
    state.store.revoke_device(&actor, id)?;
    Ok(StatusCode::NO_CONTENT)
}
async fn collections(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<Collection>>> {
    let actor = principal(&state, &headers)?;
    Ok(Json(
        blocking(move || state.service.collections(&actor)).await?,
    ))
}
async fn browse(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<BrowseQuery>,
) -> ApiResult<Json<Page<PresentedItem>>> {
    let actor = principal(&state, &headers)?;
    Ok(Json(
        blocking(move || state.service.browse_presented(&actor, query)).await?,
    ))
}
async fn item(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<ItemId>,
) -> ApiResult<Json<PresentedItem>> {
    let actor = principal(&state, &headers)?;
    Ok(Json(
        blocking(move || state.service.item_presented(&actor, id)).await?,
    ))
}
#[derive(Deserialize)]
struct SaveItem {
    item: Item,
    expected_revision: Revision,
}
async fn save_item(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<ItemId>,
    Json(input): Json<SaveItem>,
) -> ApiResult<Json<PresentedItem>> {
    let actor = principal(&state, &headers)?;
    if id != input.item.id {
        return Err(Error::Invalid("item ID mismatch".into()).into());
    }
    Ok(Json(
        blocking(move || {
            let saved = state
                .service
                .save(&actor, input.item, input.expected_revision)?;
            state.service.item_presented(&actor, saved.id)
        })
        .await?,
    ))
}
async fn evidence(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<ItemId>,
) -> ApiResult<Json<Vec<Evidence>>> {
    let actor = principal(&state, &headers)?;
    Ok(Json(
        blocking(move || state.service.evidence(&actor, id)).await?,
    ))
}
async fn activity(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<ItemId>,
) -> ApiResult<Json<serde_json::Value>> {
    let actor = principal(&state, &headers)?;
    Ok(Json(
        blocking(move || state.store.activity(&actor, id)).await?,
    ))
}
async fn save_activity(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<ItemId>,
    Json(value): Json<serde_json::Value>,
) -> ApiResult<StatusCode> {
    let actor = principal(&state, &headers)?;
    blocking(move || state.store.save_activity(&actor, id, value)).await?;
    Ok(StatusCode::NO_CONTENT)
}
#[derive(Deserialize)]
struct AssetQuery {
    #[serde(default)]
    thumbnail: bool,
}
async fn asset(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((id, asset)): Path<(ItemId, AssetId)>,
    Query(query): Query<AssetQuery>,
) -> ApiResult<Response> {
    let actor = principal(&state, &headers)?;
    let (path, mime, digest) =
        blocking(move || state.store.asset_file(&actor, id, asset, query.thumbnail)).await?;
    file_response(path, &mime, &digest, headers).await
}
async fn media(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((id, representation, index)): Path<(ItemId, RepresentationId, usize)>,
) -> ApiResult<Response> {
    let actor = principal(&state, &headers)?;
    let (path, file) = blocking(move || {
        state
            .store
            .representation_file(&actor, id, representation, index)
    })
    .await?;
    let mime = match path
        .extension()
        .and_then(|s| s.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("flac") => "audio/flac",
        Some("mp3") => "audio/mpeg",
        Some("wav") => "audio/wav",
        Some("opus") | Some("ogg") => "audio/ogg",
        _ => "application/octet-stream",
    };
    file_response(path, mime, &file.digest, headers).await
}
pub fn parse_range(value: &str, length: u64) -> Option<(u64, u64)> {
    if length == 0 {
        return None;
    }
    let value = value.strip_prefix("bytes=")?;
    if value.contains(',') {
        return None;
    }
    let (start, end) = value.split_once('-')?;
    if start.is_empty() {
        let suffix = end.parse::<u64>().ok()?;
        if suffix == 0 {
            return None;
        }
        return Some((length.saturating_sub(suffix), length - 1));
    }
    let start = start.parse::<u64>().ok()?;
    let end = if end.is_empty() {
        length - 1
    } else {
        end.parse::<u64>().ok()?.min(length - 1)
    };
    if start >= length || start > end {
        None
    } else {
        Some((start, end))
    }
}
async fn file_response(
    path: std::path::PathBuf,
    mime: &str,
    digest: &Digest,
    headers: HeaderMap,
) -> ApiResult<Response> {
    let mut file = tokio::fs::File::open(path)
        .await
        .map_err(|e| Error::Storage(e.to_string()))?;
    let length = file
        .metadata()
        .await
        .map_err(|e| Error::Storage(e.to_string()))?
        .len();
    let etag = format!("\"{}\"", digest.hex);
    let range = headers
        .get(header::RANGE)
        .and_then(|v| v.to_str().ok())
        .filter(|_| {
            headers
                .get(header::IF_RANGE)
                .is_none_or(|v| v.to_str().ok() == Some(&etag))
        });
    let mut builder = Response::builder()
        .header(header::CONTENT_TYPE, mime)
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::ETAG, &etag);
    let count = if let Some(range) = range {
        let Some((start, end)) = parse_range(range, length) else {
            return Ok((
                StatusCode::RANGE_NOT_SATISFIABLE,
                [(header::CONTENT_RANGE, format!("bytes */{length}"))],
            )
                .into_response());
        };
        file.seek(std::io::SeekFrom::Start(start))
            .await
            .map_err(|e| Error::Storage(e.to_string()))?;
        builder = builder.status(StatusCode::PARTIAL_CONTENT).header(
            header::CONTENT_RANGE,
            format!("bytes {start}-{end}/{length}"),
        );
        end - start + 1
    } else {
        length
    };
    builder
        .header(header::CONTENT_LENGTH, count)
        .body(Body::from_stream(tokio_util::io::ReaderStream::new(
            file.take(count),
        )))
        .map_err(|e| Error::Storage(e.to_string()).into())
}
#[derive(Deserialize)]
struct CoverQuery {
    expected_revision: u64,
    #[serde(default)]
    alt_text: String,
}
async fn cover(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<ItemId>,
    Query(query): Query<CoverQuery>,
    body: axum::body::Bytes,
) -> ApiResult<Json<Item>> {
    let actor = principal(&state, &headers)?;
    Ok(Json(
        blocking(move || {
            state.store.add_cover(
                &actor,
                id,
                Revision(query.expected_revision),
                &body,
                query.alt_text,
            )
        })
        .await?,
    ))
}
#[derive(Deserialize)]
struct NewUpload {
    item_id: ItemId,
    expected_revision: Revision,
    files: Vec<UploadFile>,
}
async fn create_upload(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<NewUpload>,
) -> ApiResult<Json<collector_store::Upload>> {
    let actor = principal(&state, &headers)?;
    Ok(Json(
        blocking(move || {
            state
                .store
                .create_upload(&actor, input.item_id, input.expected_revision, input.files)
        })
        .await?,
    ))
}
async fn upload(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<JobId>,
) -> ApiResult<Json<collector_store::Upload>> {
    let actor = principal(&state, &headers)?;
    Ok(Json(
        blocking(move || state.store.upload(&actor, id)).await?,
    ))
}
#[derive(Deserialize)]
struct Offset {
    offset: u64,
}
async fn chunk(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((id, index)): Path<(JobId, usize)>,
    Query(query): Query<Offset>,
    body: axum::body::Bytes,
) -> ApiResult<Json<collector_store::Upload>> {
    let actor = principal(&state, &headers)?;
    Ok(Json(
        blocking(move || {
            state
                .store
                .upload_chunk(&actor, id, index, query.offset, &body)
        })
        .await?,
    ))
}
async fn cancel_upload(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<JobId>,
) -> ApiResult<StatusCode> {
    let actor = principal(&state, &headers)?;
    blocking(move || state.store.cancel_upload(&actor, id)).await?;
    Ok(StatusCode::NO_CONTENT)
}
async fn commit_upload(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<JobId>,
) -> ApiResult<Response> {
    let actor = principal(&state, &headers)?;
    state.store.upload(&actor, id)?;
    let permit = state
        .jobs
        .clone()
        .try_acquire_owned()
        .map_err(|_| Error::Unavailable("import workers are busy; retry shortly".into()))?;
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        if let Err(error) = state.store.commit_upload(&actor, id) {
            tracing::error!(%error,%id,"import failed");
            let _ = state.store.fail_upload(
                &actor,
                id,
                "Archive publication failed. Check server logs and retry the retained upload.",
            );
        }
    });
    Ok((StatusCode::ACCEPTED, Json(serde_json::json!({"job_id":id}))).into_response())
}
async fn verify(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<ItemId>,
) -> ApiResult<Json<Item>> {
    let actor = principal(&state, &headers)?;
    let _permit = state
        .jobs
        .clone()
        .try_acquire_owned()
        .map_err(|_| Error::Unavailable("workers busy".into()))?;
    Ok(Json(
        blocking(move || state.store.verify_integrity(&actor, id)).await?,
    ))
}
async fn jobs(State(state): State<AppState>, headers: HeaderMap) -> ApiResult<Json<Vec<Job>>> {
    let actor = principal(&state, &headers)?;
    Ok(Json(blocking(move || state.store.jobs(&actor)).await?))
}
async fn settings(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<serde_json::Value>> {
    administrator(&principal(&state, &headers)?)?;
    Ok(Json(
        serde_json::json!({"exposure":state.exposure,"multi_user":state.multi_user,"server_id":state.pairing.server_id,"ingress":format!("{:?}",state.ingress)}),
    ))
}
#[derive(Deserialize)]
struct NewUser {
    username: String,
    password: String,
    #[serde(default)]
    administrator: bool,
}
async fn create_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<NewUser>,
) -> ApiResult<Json<serde_json::Value>> {
    administrator(&principal(&state, &headers)?)?;
    let id = blocking(move || {
        state
            .store
            .create_user(&input.username, &input.password, input.administrator)
    })
    .await?;
    Ok(Json(serde_json::json!({"user_id":id})))
}
#[derive(Deserialize)]
struct NewCollection {
    name: String,
}
async fn create_collection(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<NewCollection>,
) -> ApiResult<Json<Collection>> {
    let actor = principal(&state, &headers)?;
    Ok(Json(
        blocking(move || state.service.create_collection(&actor, &input.name)).await?,
    ))
}
#[derive(Deserialize)]
struct Grant {
    collection_id: CollectionId,
    user_id: UserId,
    role: Role,
}
async fn grant(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<Grant>,
) -> ApiResult<StatusCode> {
    let actor = principal(&state, &headers)?;
    blocking(move || {
        state
            .service
            .grant(&actor, input.collection_id, input.user_id, input.role)
    })
    .await?;
    Ok(StatusCode::NO_CONTENT)
}
async fn reindex(State(state): State<AppState>, headers: HeaderMap) -> ApiResult<StatusCode> {
    administrator(&principal(&state, &headers)?)?;
    blocking(move || state.store.reindex()).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn inboxes(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<String>>> {
    administrator(&principal(&state, &headers)?)?;
    Ok(Json(state.inboxes.keys().cloned().collect()))
}
#[derive(Deserialize)]
struct InboxImport {
    item_id: ItemId,
    expected_revision: Revision,
    package: String,
}
async fn import_inbox(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(name): Path<String>,
    Json(input): Json<InboxImport>,
) -> ApiResult<Response> {
    let actor = principal(&state, &headers)?;
    administrator(&actor)?;
    if input.package.is_empty()
        || input.package.contains(['/', '\\'])
        || matches!(input.package.as_str(), "." | "..")
    {
        return Err(Error::Invalid("select a direct inbox package directory".into()).into());
    }
    let root = state.inboxes.get(&name).ok_or(Error::NotFound)?.clone();
    let source = root.join(&input.package);
    let source = blocking(move || {
        let root = std::fs::canonicalize(root).map_err(|e| Error::Storage(e.to_string()))?;
        let path = std::fs::canonicalize(&source).map_err(|e| Error::Storage(e.to_string()))?;
        if !path.starts_with(root)
            || std::fs::symlink_metadata(source)
                .map_err(|e| Error::Storage(e.to_string()))?
                .file_type()
                .is_symlink()
        {
            return Err(Error::Forbidden);
        }
        Ok(path)
    })
    .await?;
    let files = {
        let source = source.clone();
        blocking(move || Store::inventory_directory(&source)).await?
    };
    let upload =
        state
            .store
            .create_upload(&actor, input.item_id, input.expected_revision, files)?;
    let id = upload.id;
    let permit = state
        .jobs
        .clone()
        .try_acquire_owned()
        .map_err(|_| Error::Unavailable("import workers busy".into()))?;
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        if let Err(error) = state.store.stage_directory(&actor, id, &source) {
            tracing::error!(%error,%id,"inbox import failed");
            let _ = state.store.fail_upload(
                &actor,
                id,
                "Inbox import failed. Sources are retained; check the server log.",
            );
        }
    });
    Ok((StatusCode::ACCEPTED, Json(serde_json::json!({"job_id":id}))).into_response())
}
