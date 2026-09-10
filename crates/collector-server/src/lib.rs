pub mod passkeys;
use axum::{
    Json, Router,
    body::Body,
    extract::{ConnectInfo, DefaultBodyLimit, Path, Query, Request, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
};
use collector_app::{
    AccountUpdate, AccountView, AuthenticatedPrincipal, CollectionService, CommandContext,
    EnrollmentStart, EnrollmentView, GrantView, MediaResource, NativeDeviceView, PasskeyView,
    PresentedCollection, PresentedEvidence, PresentedItem, SaveItemCommand, SessionView,
};
use collector_client::Pairing;
use collector_core::*;
use collector_modules::{Availability, Registry};
use collector_store::{Store, Upload, UploadFile};
use serde::Deserialize;
use std::{
    collections::{HashMap, VecDeque},
    io::{Read, Seek, SeekFrom},
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Instant,
};
use utoipa::openapi::OpenApi;
use utoipa_axum::{
    router::{OpenApiRouter, UtoipaMethodRouterExt},
    routes,
};

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
    pub rate_limits: Arc<RateLimits>,
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
            rate_limits: Arc::new(RateLimits::default()),
            passkeys: None,
        }
    }
}

pub struct JobRuntime(Vec<tokio::task::JoinHandle<()>>);

impl Drop for JobRuntime {
    fn drop(&mut self) {
        for worker in &self.0 {
            worker.abort();
        }
    }
}

pub fn start_job_workers(service: CollectionService) -> JobRuntime {
    let worker_instance = ServerId::new().to_string();
    let workers = (0..2)
        .map(|worker_index| {
            let service = service.clone();
            let worker_instance = worker_instance.clone();
            tokio::spawn(async move {
                loop {
                    let service = service.clone();
                    let worker_instance = worker_instance.clone();
                    match tokio::task::spawn_blocking(move || {
                        service.run_next_job(&worker_instance)
                    })
                    .await
                    {
                        Ok(Ok(true)) => continue,
                        Ok(Ok(false)) => {
                            tokio::time::sleep(std::time::Duration::from_millis(250)).await
                        }
                        Ok(Err(error)) => {
                            tracing::error!(worker_index, %error, "durable job worker failed");
                            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                        }
                        Err(error) => {
                            tracing::error!(worker_index, %error, "durable job worker panicked");
                            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                        }
                    }
                }
            })
        })
        .collect();
    JobRuntime(workers)
}

pub trait RateLimitClock: Send + Sync {
    fn monotonic_millis(&self) -> u64;
}

struct SystemRateLimitClock(Instant);

impl RateLimitClock for SystemRateLimitClock {
    fn monotonic_millis(&self) -> u64 {
        self.0.elapsed().as_millis().try_into().unwrap_or(u64::MAX)
    }
}

#[derive(Default)]
struct RateLimitState {
    global: VecDeque<u64>,
    peers: HashMap<String, VecDeque<u64>>,
    accounts: HashMap<String, VecDeque<u64>>,
    enrollment_global: VecDeque<u64>,
    enrollment_peers: HashMap<String, VecDeque<u64>>,
    enrollment_accounts: HashMap<String, VecDeque<u64>>,
}

pub struct RateLimits {
    clock: Arc<dyn RateLimitClock>,
    state: Mutex<RateLimitState>,
}

impl Default for RateLimits {
    fn default() -> Self {
        Self::new(Arc::new(SystemRateLimitClock(Instant::now())))
    }
}

impl RateLimits {
    pub fn new(clock: Arc<dyn RateLimitClock>) -> Self {
        Self {
            clock,
            state: Mutex::new(RateLimitState::default()),
        }
    }

    fn check_authentication(&self, account: &str, peer: &str) -> Result<()> {
        const WINDOW_MILLIS: u64 = 60_000;
        const GLOBAL_LIMIT: usize = 100;
        const PEER_LIMIT: usize = 30;
        const ACCOUNT_LIMIT: usize = 10;
        let now = self.clock.monotonic_millis();
        let mut state = lock_rate(&self.state)?;
        purge(&mut state.global, now, WINDOW_MILLIS);
        let peer_attempts = state.peers.entry(peer.to_owned()).or_default();
        purge(peer_attempts, now, WINDOW_MILLIS);
        let account_key = Digest::sha256(account.trim().as_bytes()).hex;
        let account_attempts = state.accounts.entry(account_key.clone()).or_default();
        purge(account_attempts, now, WINDOW_MILLIS);
        if state.global.len() >= GLOBAL_LIMIT
            || state
                .peers
                .get(peer)
                .is_some_and(|values| values.len() >= PEER_LIMIT)
            || state
                .accounts
                .get(&account_key)
                .is_some_and(|values| values.len() >= ACCOUNT_LIMIT)
        {
            return Err(Error::RateLimited(60));
        }
        state.global.push_back(now);
        state
            .peers
            .get_mut(peer)
            .expect("peer bucket exists")
            .push_back(now);
        state
            .accounts
            .get_mut(&account_key)
            .expect("account bucket exists")
            .push_back(now);
        Ok(())
    }

    fn check_enrollment(&self, account: &str, peer: &str) -> Result<()> {
        const WINDOW_MILLIS: u64 = 60_000;
        const GLOBAL_LIMIT: usize = 50;
        const PEER_LIMIT: usize = 15;
        const ACCOUNT_LIMIT: usize = 8;
        let now = self.clock.monotonic_millis();
        let mut state = lock_rate(&self.state)?;
        purge(&mut state.enrollment_global, now, WINDOW_MILLIS);
        let peer_attempts = state.enrollment_peers.entry(peer.to_owned()).or_default();
        purge(peer_attempts, now, WINDOW_MILLIS);
        let account_key = Digest::sha256(account.trim().as_bytes()).hex;
        let account_attempts = state
            .enrollment_accounts
            .entry(account_key.clone())
            .or_default();
        purge(account_attempts, now, WINDOW_MILLIS);
        if state.enrollment_global.len() >= GLOBAL_LIMIT
            || state
                .enrollment_peers
                .get(peer)
                .is_some_and(|values| values.len() >= PEER_LIMIT)
            || state
                .enrollment_accounts
                .get(&account_key)
                .is_some_and(|values| values.len() >= ACCOUNT_LIMIT)
        {
            return Err(Error::RateLimited(60));
        }
        state.enrollment_global.push_back(now);
        state
            .enrollment_peers
            .get_mut(peer)
            .expect("peer bucket exists")
            .push_back(now);
        state
            .enrollment_accounts
            .get_mut(&account_key)
            .expect("account bucket exists")
            .push_back(now);
        Ok(())
    }
}

fn lock_rate<T>(mutex: &Mutex<T>) -> Result<std::sync::MutexGuard<'_, T>> {
    mutex
        .lock()
        .map_err(|_| Error::Unavailable("authentication rate limiter unavailable".into()))
}

fn purge(values: &mut VecDeque<u64>, now: u64, window: u64) {
    while values
        .front()
        .is_some_and(|value| now.saturating_sub(*value) >= window)
    {
        values.pop_front();
    }
}

#[derive(Clone)]
pub(crate) struct DirectPeer(String);
pub struct ApiError(pub Error);
impl From<Error> for ApiError {
    fn from(e: Error) -> Self {
        Self(e)
    }
}
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        if let Error::RateLimited(seconds) = self.0 {
            return (
                StatusCode::TOO_MANY_REQUESTS,
                [(header::RETRY_AFTER, seconds.to_string())],
                Json(serde_json::json!({"error":{"code":"rate_limited","message":"Too many authentication attempts. Try again later."}})),
            )
                .into_response();
        }
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
            Error::RecentAuthenticationRequired => (
                StatusCode::FORBIDDEN,
                "recent_authentication_required",
                "Reauthenticate before changing security or access settings".into(),
            ),
            Error::LastAdministrator => (
                StatusCode::CONFLICT,
                "last_administrator",
                "At least one enabled administrator must remain".into(),
            ),
            Error::LastOwner => (
                StatusCode::CONFLICT,
                "last_owner",
                "At least one enabled owner must remain for every collection".into(),
            ),
            Error::DefaultReassignmentRequired => (
                StatusCode::CONFLICT,
                "default_reassignment_required",
                "Reassign the durable default account before disabling it".into(),
            ),
            Error::EnrollmentExpired => (
                StatusCode::GONE,
                "expired_enrollment",
                "That enrollment has expired; start a new one".into(),
            ),
            Error::EnrollmentCancelled => (
                StatusCode::GONE,
                "cancelled_enrollment",
                "That enrollment was cancelled".into(),
            ),
            Error::CredentialRevoked => (
                StatusCode::UNAUTHORIZED,
                "revoked_credential",
                "That credential is revoked or expired".into(),
            ),
            Error::Conflict => (
                StatusCode::CONFLICT,
                "revision_conflict",
                "The record or upload changed; refresh before retrying".into(),
            ),
            Error::IdempotencyConflict => (
                StatusCode::CONFLICT,
                "idempotency_conflict",
                "That Idempotency-Key was already used for a different command".into(),
            ),
            Error::CommandInProgress => (
                StatusCode::CONFLICT,
                "command_in_progress",
                "That command is already in progress; retry with the same key".into(),
            ),
            Error::JobCancelled => (
                StatusCode::CONFLICT,
                "job_cancelled",
                "That job was cancelled".into(),
            ),
            Error::CancellationTooLate => (
                StatusCode::CONFLICT,
                "cancellation_too_late",
                "Publication is already committed and will finish recovery".into(),
            ),
            Error::RateLimited(_) => unreachable!("handled above"),
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
fn authenticated(state: &AppState, headers: &HeaderMap) -> ApiResult<AuthenticatedPrincipal> {
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
    if let Some(cookie) = cookie {
        return Ok(state.service.authenticate(cookie)?);
    }
    let bearer = bearer.ok_or(Error::Unauthenticated)?;
    match state.service.authenticate_native(bearer) {
        Ok(authenticated) => Ok(authenticated),
        Err(_) => state.service.authenticate(bearer).map_err(Into::into),
    }
}
fn principal(state: &AppState, headers: &HeaderMap) -> ApiResult<Principal> {
    Ok(authenticated(state, headers)?.principal)
}
fn command_context(headers: &HeaderMap) -> ApiResult<CommandContext> {
    let key = headers
        .get("idempotency-key")
        .ok_or_else(|| Error::Invalid("Idempotency-Key is required".into()))?
        .to_str()
        .map_err(|_| Error::Invalid("Idempotency-Key must be valid ASCII".into()))?;
    Ok(CommandContext::new(key)?)
}
pub fn router(state: AppState) -> Router {
    let (client_api, _) = client_api_router().split_for_parts();
    let mut router = Router::new()
        .route(
            "/healthz",
            get(|| async { Json(serde_json::json!({"status":"ok"})) }),
        )
        .route("/", get(index))
        .route("/app.js", get(app_js))
        .route("/app.css", get(app_css))
        .route("/security.js", get(security_js))
        .merge(client_api);
    if state.exposure.allows(state.ingress, Surface::Curation) {
        let (curation_api, _) = curation_api_router().split_for_parts();
        router = router
            .route("/curate", get(index))
            .route("/curation.js", get(curation_js))
            .merge(curation_api);
    }
    if state
        .exposure
        .allows(state.ingress, Surface::Administration)
    {
        let (administration_api, _) = administration_api_router().split_for_parts();
        router = router
            .route("/admin", get(index))
            .route("/administration.js", get(administration_js))
            .merge(administration_api);
    }
    router
        .layer(DefaultBodyLimit::max(2 * 1024 * 1024))
        .layer(middleware::from_fn_with_state(state.clone(), security))
        .with_state(state)
}

fn client_api_router() -> OpenApiRouter<AppState> {
    OpenApiRouter::default()
        .routes(routes!(identity))
        .routes(routes!(features))
        .routes(routes!(login))
        .routes(routes!(recover))
        .routes(routes!(logout))
        .routes(routes!(passkeys::register_start))
        .routes(routes!(passkeys::register_finish))
        .routes(routes!(passkeys::login_start))
        .routes(routes!(passkeys::login_finish))
        .routes(routes!(me))
        .routes(routes!(profile, update_profile))
        .routes(routes!(pairing))
        .routes(routes!(revoke))
        .routes(routes!(sessions))
        .routes(routes!(replace_recovery_codes))
        .routes(routes!(list_passkeys))
        .routes(routes!(revoke_passkey))
        .routes(routes!(start_enrollment))
        .routes(routes!(approve_enrollment))
        .routes(routes!(complete_enrollment))
        .routes(routes!(cancel_enrollment))
        .routes(routes!(native_devices))
        .routes(routes!(revoke_native_device))
        .routes(routes!(collections))
        .routes(routes!(browse))
        .routes(routes!(item))
        .routes(routes!(evidence))
        .routes(routes!(activity, save_activity))
        .routes(routes!(asset))
        .routes(routes!(media))
}

fn curation_api_router() -> OpenApiRouter<AppState> {
    OpenApiRouter::default()
        .routes(routes!(collection_grants))
        .routes(routes!(set_collection_grant, remove_collection_grant))
        .routes(routes!(save_item))
        .routes(routes!(verify))
        .routes(routes!(create_upload))
        .routes(routes!(upload, cancel_upload))
        .routes(routes!(commit_upload))
        .routes(routes!(jobs))
        .routes(routes!(chunk).layer(DefaultBodyLimit::max(8 * 1024 * 1024)))
        .routes(routes!(cover).layer(DefaultBodyLimit::max(20 * 1024 * 1024)))
}

fn administration_api_router() -> OpenApiRouter<AppState> {
    OpenApiRouter::default()
        .routes(routes!(settings))
        .routes(routes!(inboxes))
        .routes(routes!(import_inbox))
        .routes(routes!(create_user))
        .routes(routes!(list_users))
        .routes(routes!(update_user))
        .routes(routes!(reassign_default_user))
        .routes(routes!(create_collection))
        .routes(routes!(grant))
        .routes(routes!(reindex))
}

pub fn openapi() -> OpenApi {
    let mut document = client_api_router()
        .merge(curation_api_router())
        .merge(administration_api_router())
        .into_openapi();
    document.info.title = "Junk Collector API".into();
    document.info.version = "1.0.0".into();
    for (path, item) in &mut document.paths.paths {
        let limit = if path.ends_with("/cover") {
            20 * 1024 * 1024
        } else if path.ends_with("/files/{index}") {
            8 * 1024 * 1024
        } else {
            2 * 1024 * 1024
        };
        for operation in [
            &mut item.get,
            &mut item.put,
            &mut item.post,
            &mut item.delete,
            &mut item.patch,
        ]
        .into_iter()
        .flatten()
        {
            if let Some(body) = &mut operation.request_body {
                body.extensions = Some(utoipa::openapi::extensions::Extensions::from_iter([(
                    "x-body-limit-bytes",
                    limit,
                )]));
            }
        }
    }
    document
}
async fn security(State(state): State<AppState>, request: Request, next: Next) -> Response {
    let modifying = !matches!(
        *request.method(),
        axum::http::Method::GET | axum::http::Method::HEAD | axum::http::Method::OPTIONS
    );
    let path = request.uri().path();
    let enrollment_bootstrap = path == "/api/v1/enrollments/start"
        || (path.starts_with("/api/v1/enrollments/") && path.ends_with("/complete"));
    let origin = request
        .headers()
        .get(header::ORIGIN)
        .and_then(|value| value.to_str().ok());
    let accepted_origin = origin.is_some_and(|candidate| {
        normalized_origin_header(candidate).is_some_and(|candidate| {
            state.allowed_origins.iter().any(|allowed| {
                normalized_configured_origin(allowed).as_deref() == Some(candidate.as_str())
            })
        })
    });
    // Static ESM requests also carry Origin. They must load on an unconfigured
    // address so the UI can explain the rejected login. Router composition still
    // controls which management assets exist; APIs retain exact-Origin checks.
    if origin.is_some() && !accepted_origin && (modifying || path.starts_with("/api/")) {
        return (StatusCode::FORBIDDEN, Json(serde_json::json!({"error": {"code": "origin_not_allowed", "message": "This browser address is not enabled in the server configuration. On the server, run doctor; for local management, stop the server, run setup, then restart and use the printed localhost URL. Remote HTTPS addresses must be added to allowed_origins."}}))).into_response();
    }
    let native_authenticated = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .is_some_and(|credential| state.service.authenticate_native(credential).is_ok());
    if modifying && !accepted_origin && !enrollment_bootstrap && !native_authenticated {
        return (StatusCode::FORBIDDEN, Json(serde_json::json!({"error": {"code": "origin_not_allowed", "message": "This browser address is not enabled in the server configuration. On the server, run doctor; for local management, stop the server, run setup, then restart and use the printed localhost URL. Remote HTTPS addresses must be added to allowed_origins."}}))).into_response();
    }
    let mut request = request;
    let peer = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|peer| peer.0.ip().to_string())
        .unwrap_or_else(|| "unknown-direct-peer".into());
    request.extensions_mut().insert(DirectPeer(peer));
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

fn normalized_configured_origin(value: &str) -> Option<String> {
    let parsed = url::Url::parse(value).ok()?;
    if !matches!(parsed.scheme(), "http" | "https")
        || parsed.host().is_none()
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.query().is_some()
        || parsed.fragment().is_some()
        || parsed.path() != "/"
    {
        return None;
    }
    Some(parsed.origin().ascii_serialization())
}

fn normalized_origin_header(value: &str) -> Option<String> {
    let normalized = normalized_configured_origin(value)?;
    (value == normalized).then_some(normalized)
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
async fn security_js() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
        include_str!("../../../web/dist/security.js"),
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

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct IdentityResponse {
    pub server_id: ServerId,
    pub protocol_version: u16,
}

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct FeatureCapability {
    pub name: String,
    pub available: bool,
    pub limitation: Option<String>,
    pub id: collector_modules::CapabilityId,
    pub display_name: String,
    pub availability: Availability,
}

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct ModuleFeature {
    pub id: ModuleId,
    pub schema_version: u32,
    pub display_name: String,
    pub capabilities: Vec<FeatureCapability>,
}

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct FeaturesResponse {
    pub default_username: Option<String>,
    pub multi_user: bool,
    pub curation: bool,
    pub administration: bool,
    pub passkeys: bool,
    pub modules: Vec<ModuleFeature>,
}

#[utoipa::path(get, path = "/api/v1/identity", responses((status = 200, body = IdentityResponse)))]
async fn identity(State(state): State<AppState>) -> Json<IdentityResponse> {
    Json(IdentityResponse {
        server_id: state.pairing.server_id,
        protocol_version: 1,
    })
}
#[utoipa::path(get, path = "/api/v1/features", responses((status = 200, body = FeaturesResponse)))]
async fn features(State(state): State<AppState>) -> Json<FeaturesResponse> {
    let modules = Registry::builtins()
        .descriptors()
        .into_iter()
        .map(|module| {
            let capabilities = module
                .capabilities
                .into_iter()
                .map(|capability| {
                    let (available, limitation) = match &capability.availability {
                        Availability::Available => (true, None),
                        Availability::Unavailable { limitation } => {
                            (false, Some(limitation.clone()))
                        }
                    };
                    FeatureCapability {
                        name: capability
                            .id
                            .as_str()
                            .rsplit('.')
                            .next()
                            .unwrap_or_default()
                            .replace('-', "_"),
                        available,
                        limitation,
                        id: capability.id,
                        display_name: capability.display_name,
                        availability: capability.availability,
                    }
                })
                .collect::<Vec<_>>();
            ModuleFeature {
                id: module.id,
                schema_version: module.schema_version,
                display_name: module.display_name,
                capabilities,
            }
        })
        .collect::<Vec<_>>();
    Json(FeaturesResponse {
        default_username: (!state.multi_user).then(|| state.default_username.clone()),
        multi_user: state.multi_user,
        curation: state.exposure.allows(state.ingress, Surface::Curation),
        administration: state
            .exposure
            .allows(state.ingress, Surface::Administration),
        passkeys: state.passkeys.is_some(),
        modules,
    })
}
#[derive(Deserialize, utoipa::ToSchema)]
struct Login {
    username: String,
    password: String,
    #[serde(default)]
    device_name: String,
}
#[utoipa::path(post, path = "/api/v1/auth/login", request_body = Login, responses((status = 200, body = serde_json::Value), (status = 401)))]
async fn login(
    State(state): State<AppState>,
    peer: Option<axum::Extension<DirectPeer>>,
    headers: HeaderMap,
    Json(input): Json<Login>,
) -> ApiResult<Response> {
    let peer = peer
        .as_ref()
        .map(|peer| peer.0.0.as_str())
        .unwrap_or("unknown-direct-peer");
    state
        .rate_limits
        .check_authentication(&input.username, peer)?;
    let (token, actor) = blocking(move || {
        state
            .service
            .login(&input.username, &input.password, &input.device_name)
    })
    .await?;
    session_response(token, actor, &headers)
}
fn session_response(
    token: String,
    actor: AuthenticatedPrincipal,
    _headers: &HeaderMap,
) -> ApiResult<Response> {
    let actor = actor.principal;
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
#[utoipa::path(post, path = "/api/v1/auth/logout", responses((status = 204), (status = 401)))]
async fn logout(State(state): State<AppState>, headers: HeaderMap) -> ApiResult<Response> {
    let actor = principal(&state, &headers)?;
    if let Some(device) = actor.device_id {
        state.service.logout_session(&actor, device)?;
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
#[utoipa::path(get, path = "/api/v1/me", responses((status = 200, body = Principal), (status = 401)))]
async fn me(State(state): State<AppState>, headers: HeaderMap) -> ApiResult<Json<Principal>> {
    Ok(Json(principal(&state, &headers)?))
}
#[derive(Deserialize, utoipa::ToSchema)]
struct ProfileChange {
    expected_revision: Revision,
    display_name: String,
}

#[utoipa::path(put, path = "/api/v1/me/profile", params(("Idempotency-Key" = String, Header)), request_body = ProfileChange, responses((status = 200, body = AccountView), (status = 401), (status = 403), (status = 409)))]
async fn update_profile(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<ProfileChange>,
) -> ApiResult<Json<AccountView>> {
    let actor = authenticated(&state, &headers)?;
    let context = command_context(&headers)?;
    Ok(Json(
        blocking(move || {
            state.service.update_profile(
                &actor,
                input.expected_revision,
                &input.display_name,
                &context,
            )
        })
        .await?,
    ))
}

#[utoipa::path(get, path = "/api/v1/me/profile", responses((status = 200, body = AccountView), (status = 401)))]
async fn profile(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<AccountView>> {
    let actor = authenticated(&state, &headers)?;
    Ok(Json(state.service.profile(&actor)?))
}
#[derive(Deserialize, utoipa::ToSchema)]
struct RecoverAccount {
    username: String,
    code: String,
    new_password: String,
    #[serde(default)]
    device_name: String,
}

#[utoipa::path(post, path = "/api/v1/auth/recovery", request_body = RecoverAccount, responses((status = 200, body = serde_json::Value), (status = 401), (status = 429)))]
async fn recover(
    State(state): State<AppState>,
    peer: Option<axum::Extension<DirectPeer>>,
    headers: HeaderMap,
    Json(input): Json<RecoverAccount>,
) -> ApiResult<Response> {
    let peer = peer
        .as_ref()
        .map(|peer| peer.0.0.as_str())
        .unwrap_or("unknown-direct-peer");
    state
        .rate_limits
        .check_authentication(&input.username, peer)?;
    let (token, actor) = blocking(move || {
        state.service.recover_account(
            &input.username,
            &input.code,
            &input.new_password,
            &input.device_name,
        )
    })
    .await?;
    session_response(token, actor, &headers)
}
#[utoipa::path(get, path = "/api/v1/pairing", responses((status = 200, body = Pairing), (status = 401)))]
async fn pairing(State(state): State<AppState>, headers: HeaderMap) -> ApiResult<Json<Pairing>> {
    principal(&state, &headers)?;
    Ok(Json(state.pairing))
}
#[utoipa::path(delete, path = "/api/v1/devices/{id}", params(("id" = DeviceId, Path), ("Idempotency-Key" = String, Header)), responses((status = 204), (status = 401), (status = 409)))]
async fn revoke(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<DeviceId>,
) -> ApiResult<StatusCode> {
    let actor = authenticated(&state, &headers)?;
    let context = command_context(&headers)?;
    state.service.revoke_session(&actor, id, &context)?;
    Ok(StatusCode::NO_CONTENT)
}
#[utoipa::path(get, path = "/api/v1/security/sessions", responses((status = 200, body = Vec<SessionView>), (status = 401)))]
async fn sessions(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<SessionView>>> {
    let actor = authenticated(&state, &headers)?;
    Ok(Json(state.service.sessions(&actor)?))
}

#[utoipa::path(post, path = "/api/v1/security/recovery-codes", responses((status = 200, body = Vec<String>), (status = 401), (status = 403)))]
async fn replace_recovery_codes(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<String>>> {
    let actor = authenticated(&state, &headers)?;
    Ok(Json(
        blocking(move || state.service.replace_recovery_codes(&actor)).await?,
    ))
}

#[utoipa::path(get, path = "/api/v1/security/passkeys", responses((status = 200, body = Vec<PasskeyView>), (status = 401)))]
async fn list_passkeys(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<PasskeyView>>> {
    let actor = principal(&state, &headers)?;
    let passkeys = state
        .service
        .passkeys(actor.user_id)?
        .into_iter()
        .map(|stored| stored.view)
        .collect();
    Ok(Json(passkeys))
}

#[derive(Deserialize, utoipa::IntoParams)]
struct ExpectedRevision {
    expected_revision: Revision,
}

#[utoipa::path(delete, path = "/api/v1/security/passkeys/{id}", params(("id" = PasskeyId, Path), ExpectedRevision, ("Idempotency-Key" = String, Header)), responses((status = 204), (status = 401), (status = 403), (status = 409)))]
async fn revoke_passkey(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<PasskeyId>,
    Query(query): Query<ExpectedRevision>,
) -> ApiResult<StatusCode> {
    let actor = authenticated(&state, &headers)?;
    let context = command_context(&headers)?;
    state
        .service
        .revoke_passkey(&actor, id, query.expected_revision, &context)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize, utoipa::ToSchema)]
struct StartEnrollment {
    username: Option<String>,
    device_name: String,
}

#[utoipa::path(post, path = "/api/v1/enrollments/start", request_body = StartEnrollment, responses((status = 200, body = EnrollmentStart), (status = 401), (status = 429)))]
async fn start_enrollment(
    State(state): State<AppState>,
    peer: Option<axum::Extension<DirectPeer>>,
    Json(input): Json<StartEnrollment>,
) -> ApiResult<Json<EnrollmentStart>> {
    let username = if state.multi_user {
        input
            .username
            .filter(|value| !value.trim().is_empty())
            .ok_or(Error::Unauthenticated)?
    } else {
        state.default_username.clone()
    };
    let peer = peer
        .as_ref()
        .map(|peer| peer.0.0.as_str())
        .unwrap_or("unknown-direct-peer");
    state.rate_limits.check_enrollment(&username, peer)?;
    Ok(Json(
        blocking(move || {
            state
                .service
                .start_enrollment(&username, &input.device_name, "/pair")
        })
        .await?,
    ))
}

#[derive(Deserialize, utoipa::ToSchema)]
struct ApproveEnrollment {
    user_code: String,
}

#[utoipa::path(post, path = "/api/v1/enrollments/approve", request_body = ApproveEnrollment, responses((status = 200, body = EnrollmentView), (status = 401), (status = 403), (status = 410)))]
async fn approve_enrollment(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<ApproveEnrollment>,
) -> ApiResult<Json<EnrollmentView>> {
    let actor = authenticated(&state, &headers)?;
    Ok(Json(
        blocking(move || state.service.approve_enrollment(&actor, &input.user_code)).await?,
    ))
}

#[derive(Deserialize, utoipa::ToSchema)]
struct CompleteEnrollment {
    completion_secret: String,
}

#[utoipa::path(post, path = "/api/v1/enrollments/{id}/complete", params(("id" = EnrollmentId, Path)), request_body = CompleteEnrollment, responses((status = 200, body = serde_json::Value), (status = 401), (status = 410), (status = 429)))]
async fn complete_enrollment(
    State(state): State<AppState>,
    peer: Option<axum::Extension<DirectPeer>>,
    Path(id): Path<EnrollmentId>,
    Json(input): Json<CompleteEnrollment>,
) -> ApiResult<Json<serde_json::Value>> {
    let peer = peer
        .as_ref()
        .map(|peer| peer.0.0.as_str())
        .unwrap_or("unknown-direct-peer");
    state.rate_limits.check_enrollment(&id.to_string(), peer)?;
    let issue = blocking({
        let state = state.clone();
        move || {
            state
                .service
                .complete_enrollment(id, &input.completion_secret)
        }
    })
    .await?;
    Ok(Json(serde_json::json!({
        "device_id": issue.device_id,
        "credential": issue.credential,
        "expires_at": issue.expires_at,
        "pairing": state.pairing,
    })))
}

#[utoipa::path(delete, path = "/api/v1/enrollments/{id}", params(("id" = EnrollmentId, Path)), responses((status = 204), (status = 401), (status = 410)))]
async fn cancel_enrollment(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<EnrollmentId>,
) -> ApiResult<StatusCode> {
    let actor = authenticated(&state, &headers)?;
    blocking(move || state.service.cancel_enrollment(&actor, id)).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(get, path = "/api/v1/security/native-devices", responses((status = 200, body = Vec<NativeDeviceView>), (status = 401)))]
async fn native_devices(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<NativeDeviceView>>> {
    let actor = authenticated(&state, &headers)?;
    Ok(Json(state.service.native_devices(&actor)?))
}

#[utoipa::path(delete, path = "/api/v1/security/native-devices/{id}", params(("id" = NativeCredentialId, Path), ExpectedRevision, ("Idempotency-Key" = String, Header)), responses((status = 204), (status = 401), (status = 403), (status = 409)))]
async fn revoke_native_device(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<NativeCredentialId>,
    Query(query): Query<ExpectedRevision>,
) -> ApiResult<StatusCode> {
    let actor = authenticated(&state, &headers)?;
    let context = command_context(&headers)?;
    state
        .service
        .revoke_native_device(&actor, id, query.expected_revision, &context)?;
    Ok(StatusCode::NO_CONTENT)
}
#[utoipa::path(get, path = "/api/v1/collections", responses((status = 200, body = Vec<PresentedCollection>), (status = 401)))]
async fn collections(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<PresentedCollection>>> {
    let actor = principal(&state, &headers)?;
    Ok(Json(
        blocking(move || state.service.collections(&actor)).await?,
    ))
}
#[utoipa::path(get, path = "/api/v1/items", params(("collection_id" = Option<CollectionId>, Query), ("search" = Option<String>, Query), ("cursor" = Option<String>, Query), ("limit" = Option<u16>, Query)), responses((status = 200, body = Page<PresentedItem>), (status = 401)))]
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
#[utoipa::path(get, path = "/api/v1/items/{id}", params(("id" = ItemId, Path)), responses((status = 200, body = PresentedItem), (status = 403), (status = 404)))]
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
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
struct SaveItemCommandEnvelope {
    command: SaveItemCommand,
    expected_revision: Revision,
}
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
struct Wave1SaveItemEnvelope {
    item: Item,
    expected_revision: Revision,
}
#[derive(utoipa::ToSchema)]
#[serde(untagged)]
enum SaveItemEnvelope {
    Command(SaveItemCommandEnvelope),
    Wave1(Wave1SaveItemEnvelope),
}
impl<'de> serde::Deserialize<'de> for SaveItemEnvelope {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        let raw = Box::<serde_json::value::RawValue>::deserialize(deserializer)?;
        let shape: serde_json::Value =
            serde_json::from_str(raw.get()).map_err(serde::de::Error::custom)?;
        let object = shape
            .as_object()
            .ok_or_else(|| serde::de::Error::custom("item save must be an object"))?;
        match (object.contains_key("command"), object.contains_key("item")) {
            (true, false) => serde_json::from_str(raw.get())
                .map(SaveItemEnvelope::Command)
                .map_err(serde::de::Error::custom),
            (false, true) => serde_json::from_str(raw.get())
                .map(SaveItemEnvelope::Wave1)
                .map_err(serde::de::Error::custom),
            _ => Err(serde::de::Error::custom(
                "item save must contain exactly one of command or item",
            )),
        }
    }
}
#[utoipa::path(put, path = "/api/v1/items/{id}", params(("id" = ItemId, Path), ("Idempotency-Key" = String, Header)), request_body = SaveItemEnvelope, responses((status = 200, body = PresentedItem), (status = 409), (status = 422)))]
async fn save_item(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<ItemId>,
    Json(input): Json<SaveItemEnvelope>,
) -> ApiResult<Json<PresentedItem>> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    Ok(Json(
        blocking(move || {
            let (command, expected_revision) = match input {
                SaveItemEnvelope::Command(input) => (input.command, input.expected_revision),
                SaveItemEnvelope::Wave1(input) => {
                    let command = state.service.adapt_wave1_item_save(
                        &actor,
                        id,
                        input.item,
                        input.expected_revision,
                    )?;
                    (command, input.expected_revision)
                }
            };
            let saved =
                state
                    .service
                    .save_item(&actor, id, command, expected_revision, &context)?;
            state.service.item_presented(&actor, saved.id)
        })
        .await?,
    ))
}
#[utoipa::path(get, path = "/api/v1/items/{id}/evidence", params(("id" = ItemId, Path)), responses((status = 200, body = Vec<PresentedEvidence>), (status = 403), (status = 404)))]
async fn evidence(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<ItemId>,
) -> ApiResult<Json<Vec<PresentedEvidence>>> {
    let actor = principal(&state, &headers)?;
    Ok(Json(
        blocking(move || state.service.evidence(&actor, id)).await?,
    ))
}
#[utoipa::path(get, path = "/api/v1/items/{id}/activity", params(("id" = ItemId, Path)), responses((status = 200, body = serde_json::Value), (status = 403)))]
async fn activity(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<ItemId>,
) -> ApiResult<Json<serde_json::Value>> {
    let actor = principal(&state, &headers)?;
    Ok(Json(
        blocking(move || state.service.activity(&actor, id)).await?,
    ))
}
#[utoipa::path(put, path = "/api/v1/items/{id}/activity", params(("id" = ItemId, Path), ("Idempotency-Key" = String, Header)), request_body = serde_json::Value, responses((status = 204), (status = 403), (status = 409)))]
async fn save_activity(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<ItemId>,
    Json(value): Json<serde_json::Value>,
) -> ApiResult<StatusCode> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    blocking(move || state.service.save_activity(&actor, id, value, &context)).await?;
    Ok(StatusCode::NO_CONTENT)
}
#[derive(Deserialize, utoipa::ToSchema)]
struct AssetQuery {
    #[serde(default)]
    thumbnail: bool,
}
#[utoipa::path(get, path = "/api/v1/items/{id}/assets/{asset}", params(("id" = ItemId, Path), ("asset" = AssetId, Path), ("thumbnail" = Option<bool>, Query)), responses((status = 200, content_type = "application/octet-stream"), (status = 403), (status = 404)))]
async fn asset(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((id, asset)): Path<(ItemId, AssetId)>,
    Query(query): Query<AssetQuery>,
) -> ApiResult<Response> {
    let actor = principal(&state, &headers)?;
    let resource =
        blocking(move || state.service.asset(&actor, id, asset, query.thumbnail)).await?;
    file_response(resource, headers).await
}
#[utoipa::path(get, path = "/api/v1/items/{id}/representations/{representation}/{index}", params(("id" = ItemId, Path), ("representation" = RepresentationId, Path), ("index" = usize, Path)), responses((status = 200, content_type = "application/octet-stream"), (status = 206, content_type = "application/octet-stream"), (status = 403), (status = 404), (status = 416)))]
async fn media(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((id, representation, index)): Path<(ItemId, RepresentationId, usize)>,
) -> ApiResult<Response> {
    let actor = principal(&state, &headers)?;
    let resource = blocking(move || {
        state
            .service
            .representation(&actor, id, representation, index)
    })
    .await?;
    file_response(resource, headers).await
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

#[cfg(test)]
mod security_tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    struct TestRateClock(AtomicU64);

    impl RateLimitClock for TestRateClock {
        fn monotonic_millis(&self) -> u64 {
            self.0.load(Ordering::SeqCst)
        }
    }

    #[test]
    fn authentication_buckets_have_deterministic_window_boundaries() {
        let clock = Arc::new(TestRateClock(AtomicU64::new(0)));
        let limits = RateLimits::new(clock.clone());
        for _ in 0..10 {
            limits.check_authentication("owner", "192.0.2.1").unwrap();
        }
        assert!(matches!(
            limits.check_authentication("owner", "192.0.2.2"),
            Err(Error::RateLimited(60))
        ));
        clock.0.store(59_999, Ordering::SeqCst);
        assert!(matches!(
            limits.check_authentication("owner", "192.0.2.2"),
            Err(Error::RateLimited(60))
        ));
        clock.0.store(60_000, Ordering::SeqCst);
        limits.check_authentication("owner", "192.0.2.2").unwrap();
    }

    #[test]
    fn origin_normalization_rejects_non_origin_serializations() {
        assert_eq!(
            normalized_configured_origin("https://EXAMPLE.com:443"),
            Some("https://example.com".into())
        );
        assert_eq!(
            normalized_origin_header("https://example.com"),
            Some("https://example.com".into())
        );
        assert_eq!(normalized_origin_header("https://example.com/"), None);
        assert_eq!(normalized_origin_header("null"), None);
    }
}
async fn file_response(resource: MediaResource, headers: HeaderMap) -> ApiResult<Response> {
    let length = resource.length;
    let etag = format!("\"{}\"", resource.digest.hex);
    let media_type = resource.media_type;
    let mut reader = resource.reader;
    let range = headers
        .get(header::RANGE)
        .and_then(|v| v.to_str().ok())
        .filter(|_| {
            headers
                .get(header::IF_RANGE)
                .is_none_or(|v| v.to_str().ok() == Some(&etag))
        });
    let mut builder = Response::builder()
        .header(header::CONTENT_TYPE, &media_type)
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
        reader = blocking(move || {
            reader
                .seek(SeekFrom::Start(start))
                .map_err(|error| Error::Storage(error.to_string()))?;
            Ok(reader)
        })
        .await?;
        builder = builder.status(StatusCode::PARTIAL_CONTENT).header(
            header::CONTENT_RANGE,
            format!("bytes {start}-{end}/{length}"),
        );
        end - start + 1
    } else {
        length
    };
    let stream =
        futures_util::stream::try_unfold((reader, count), |(mut reader, remaining)| async move {
            if remaining == 0 {
                return Ok::<_, std::io::Error>(None);
            }
            tokio::task::spawn_blocking(move || {
                let mut bytes = vec![0; remaining.min(1024 * 1024) as usize];
                let read = reader.read(&mut bytes)?;
                if read == 0 {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::UnexpectedEof,
                        "media ended before its recorded length",
                    ));
                }
                bytes.truncate(read);
                Ok(Some((
                    axum::body::Bytes::from(bytes),
                    (reader, remaining - read as u64),
                )))
            })
            .await
            .map_err(|error| std::io::Error::other(error.to_string()))?
        });
    builder
        .header(header::CONTENT_LENGTH, count)
        .body(Body::from_stream(stream))
        .map_err(|e| Error::Storage(e.to_string()).into())
}
#[derive(Deserialize, utoipa::ToSchema)]
struct CoverQuery {
    expected_revision: u64,
    #[serde(default)]
    alt_text: String,
}
#[utoipa::path(post, path = "/api/v1/items/{id}/cover", params(("id" = ItemId, Path), ("expected_revision" = u64, Query), ("alt_text" = Option<String>, Query), ("Idempotency-Key" = String, Header)), request_body(content = String, content_type = "application/octet-stream"), responses((status = 200, body = Item), (status = 409), (status = 413)))]
async fn cover(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<ItemId>,
    Query(query): Query<CoverQuery>,
    body: axum::body::Bytes,
) -> ApiResult<Json<Item>> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    Ok(Json(
        blocking(move || {
            state.service.publish_cover(
                &actor,
                id,
                Revision(query.expected_revision),
                &body,
                query.alt_text,
                &context,
            )
        })
        .await?,
    ))
}
#[derive(Deserialize, utoipa::ToSchema)]
struct NewUpload {
    item_id: ItemId,
    expected_revision: Revision,
    files: Vec<UploadFile>,
}
#[utoipa::path(post, path = "/api/v1/uploads", params(("Idempotency-Key" = String, Header)), request_body = NewUpload, responses((status = 200, body = Upload), (status = 409)))]
async fn create_upload(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<NewUpload>,
) -> ApiResult<Json<collector_store::Upload>> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    Ok(Json(
        blocking(move || {
            state.service.create_upload(
                &actor,
                input.item_id,
                input.expected_revision,
                input.files,
                &context,
            )
        })
        .await?,
    ))
}
#[utoipa::path(get, path = "/api/v1/uploads/{id}", params(("id" = JobId, Path)), responses((status = 200, body = Upload), (status = 403), (status = 404)))]
async fn upload(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<JobId>,
) -> ApiResult<Json<collector_store::Upload>> {
    let actor = principal(&state, &headers)?;
    Ok(Json(
        blocking(move || state.service.upload(&actor, id)).await?,
    ))
}
#[derive(Deserialize, utoipa::ToSchema)]
struct Offset {
    offset: u64,
}
#[utoipa::path(put, path = "/api/v1/uploads/{id}/files/{index}", params(("id" = JobId, Path), ("index" = usize, Path), ("offset" = u64, Query)), request_body(content = String, content_type = "application/octet-stream"), responses((status = 200, body = Upload), (status = 409), (status = 413)))]
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
                .service
                .upload_chunk(&actor, id, index, query.offset, &body)
        })
        .await?,
    ))
}
#[utoipa::path(delete, path = "/api/v1/uploads/{id}", params(("id" = JobId, Path), ("Idempotency-Key" = String, Header)), responses((status = 204), (status = 409)))]
async fn cancel_upload(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<JobId>,
) -> ApiResult<StatusCode> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    blocking(move || state.service.cancel_upload(&actor, id, &context)).await?;
    Ok(StatusCode::NO_CONTENT)
}
#[utoipa::path(post, path = "/api/v1/uploads/{id}/commit", params(("id" = JobId, Path), ("Idempotency-Key" = String, Header)), responses((status = 202, body = serde_json::Value), (status = 409)))]
async fn commit_upload(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<JobId>,
) -> ApiResult<Response> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    state.service.upload(&actor, id)?;
    blocking(move || state.service.enqueue_upload_commit(&actor, id, &context)).await?;
    Ok((StatusCode::ACCEPTED, Json(serde_json::json!({"job_id":id}))).into_response())
}
#[utoipa::path(post, path = "/api/v1/items/{id}/verify", params(("id" = ItemId, Path), ("Idempotency-Key" = String, Header)), responses((status = 202, body = serde_json::Value), (status = 409)))]
async fn verify(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<ItemId>,
) -> ApiResult<Response> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    let job_id = blocking(move || {
        state
            .service
            .enqueue_integrity_verification(&actor, id, &context)
    })
    .await?;
    Ok((
        StatusCode::ACCEPTED,
        Json(serde_json::json!({"job_id":job_id})),
    )
        .into_response())
}
#[utoipa::path(get, path = "/api/v1/jobs", responses((status = 200, body = Vec<Job>), (status = 401)))]
async fn jobs(State(state): State<AppState>, headers: HeaderMap) -> ApiResult<Json<Vec<Job>>> {
    let actor = principal(&state, &headers)?;
    Ok(Json(blocking(move || state.service.jobs(&actor)).await?))
}
#[utoipa::path(get, path = "/api/v1/admin/settings", responses((status = 200, body = serde_json::Value), (status = 403)))]
async fn settings(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<serde_json::Value>> {
    state
        .service
        .require_administrator(&principal(&state, &headers)?)?;
    Ok(Json(
        serde_json::json!({"exposure":state.exposure,"multi_user":state.multi_user,"server_id":state.pairing.server_id,"ingress":format!("{:?}",state.ingress)}),
    ))
}
#[derive(Deserialize, utoipa::ToSchema)]
struct NewUser {
    username: String,
    password: String,
    #[serde(default)]
    administrator: bool,
}
#[utoipa::path(post, path = "/api/v1/admin/users", params(("Idempotency-Key" = String, Header)), request_body = NewUser, responses((status = 200, body = serde_json::Value), (status = 403), (status = 409)))]
async fn create_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<NewUser>,
) -> ApiResult<Json<serde_json::Value>> {
    let actor = authenticated(&state, &headers)?;
    let context = command_context(&headers)?;
    let id = blocking(move || {
        state.service.create_user(
            &actor,
            &input.username,
            &input.password,
            input.administrator,
            &context,
        )
    })
    .await?;
    Ok(Json(serde_json::json!({"user_id":id})))
}
#[utoipa::path(get, path = "/api/v1/admin/users", responses((status = 200, body = Vec<AccountView>), (status = 401), (status = 403)))]
async fn list_users(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<AccountView>>> {
    let actor = authenticated(&state, &headers)?;
    Ok(Json(state.service.accounts(&actor)?))
}

#[derive(Deserialize, utoipa::ToSchema)]
struct AccountChange {
    expected_revision: Revision,
    display_name: String,
    enabled: bool,
    administrator: bool,
}

#[utoipa::path(put, path = "/api/v1/admin/users/{id}", params(("id" = UserId, Path), ("Idempotency-Key" = String, Header)), request_body = AccountChange, responses((status = 200, body = AccountView), (status = 401), (status = 403), (status = 409)))]
async fn update_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<UserId>,
    Json(input): Json<AccountChange>,
) -> ApiResult<Json<AccountView>> {
    let actor = authenticated(&state, &headers)?;
    let context = command_context(&headers)?;
    Ok(Json(
        blocking(move || {
            state.service.update_account(
                &actor,
                AccountUpdate {
                    user: id,
                    expected_revision: input.expected_revision,
                    display_name: input.display_name,
                    enabled: input.enabled,
                    administrator: input.administrator,
                },
                &context,
            )
        })
        .await?,
    ))
}

#[utoipa::path(put, path = "/api/v1/admin/default-user/{id}", params(("id" = UserId, Path), ("Idempotency-Key" = String, Header)), responses((status = 204), (status = 401), (status = 403), (status = 409)))]
async fn reassign_default_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<UserId>,
) -> ApiResult<StatusCode> {
    let actor = authenticated(&state, &headers)?;
    let context = command_context(&headers)?;
    blocking(move || state.service.reassign_default(&actor, id, &context)).await?;
    Ok(StatusCode::NO_CONTENT)
}
#[derive(Deserialize, utoipa::ToSchema)]
struct NewCollection {
    name: String,
}
#[utoipa::path(post, path = "/api/v1/admin/collections", params(("Idempotency-Key" = String, Header)), request_body = NewCollection, responses((status = 200, body = Collection), (status = 403), (status = 409)))]
async fn create_collection(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<NewCollection>,
) -> ApiResult<Json<Collection>> {
    let actor = authenticated(&state, &headers)?;
    let context = command_context(&headers)?;
    Ok(Json(
        blocking(move || {
            state
                .service
                .create_collection(&actor, &input.name, &context)
        })
        .await?,
    ))
}
#[derive(Deserialize, utoipa::ToSchema)]
struct Grant {
    collection_id: CollectionId,
    user_id: UserId,
    role: Role,
}
#[utoipa::path(post, path = "/api/v1/admin/grants", params(("Idempotency-Key" = String, Header)), request_body = Grant, responses((status = 204), (status = 403), (status = 409)))]
async fn grant(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<Grant>,
) -> ApiResult<StatusCode> {
    let actor = authenticated(&state, &headers)?;
    let context = command_context(&headers)?;
    blocking(move || {
        state.service.grant(
            &actor,
            input.collection_id,
            input.user_id,
            input.role,
            &context,
        )
    })
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(get, path = "/api/v1/collections/{id}/grants", params(("id" = CollectionId, Path)), responses((status = 200, body = Vec<GrantView>), (status = 401), (status = 403)))]
async fn collection_grants(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<CollectionId>,
) -> ApiResult<Json<Vec<GrantView>>> {
    let actor = principal(&state, &headers)?;
    Ok(Json(state.service.grants(&actor, id)?))
}

#[derive(Deserialize, utoipa::ToSchema)]
struct GrantChange {
    expected_revision: Revision,
    role: Role,
}

#[utoipa::path(put, path = "/api/v1/collections/{id}/grants/{username}", params(("id" = CollectionId, Path), ("username" = String, Path), ("Idempotency-Key" = String, Header)), request_body = GrantChange, responses((status = 200, body = GrantView), (status = 401), (status = 403), (status = 409)))]
async fn set_collection_grant(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((id, username)): Path<(CollectionId, String)>,
    Json(input): Json<GrantChange>,
) -> ApiResult<Json<GrantView>> {
    let actor = authenticated(&state, &headers)?;
    let context = command_context(&headers)?;
    let grant = blocking(move || {
        state.service.set_grant(
            &actor,
            id,
            &username,
            input.expected_revision,
            Some(input.role),
            &context,
        )
    })
    .await?
    .ok_or(Error::Storage("grant update returned no record".into()))?;
    Ok(Json(grant))
}

#[utoipa::path(delete, path = "/api/v1/collections/{id}/grants/{username}", params(("id" = CollectionId, Path), ("username" = String, Path), ExpectedRevision, ("Idempotency-Key" = String, Header)), responses((status = 204), (status = 401), (status = 403), (status = 409)))]
async fn remove_collection_grant(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((id, username)): Path<(CollectionId, String)>,
    Query(query): Query<ExpectedRevision>,
) -> ApiResult<StatusCode> {
    let actor = authenticated(&state, &headers)?;
    let context = command_context(&headers)?;
    blocking(move || {
        state.service.set_grant(
            &actor,
            id,
            &username,
            query.expected_revision,
            None,
            &context,
        )
    })
    .await?;
    Ok(StatusCode::NO_CONTENT)
}
#[utoipa::path(post, path = "/api/v1/admin/reindex", params(("Idempotency-Key" = String, Header)), responses((status = 204), (status = 403), (status = 409)))]
async fn reindex(State(state): State<AppState>, headers: HeaderMap) -> ApiResult<StatusCode> {
    let actor = authenticated(&state, &headers)?;
    let context = command_context(&headers)?;
    blocking(move || state.service.reindex(&actor, &context)).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(get, path = "/api/v1/admin/inboxes", responses((status = 200, body = Vec<String>), (status = 403)))]
async fn inboxes(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<String>>> {
    state
        .service
        .require_administrator(&principal(&state, &headers)?)?;
    Ok(Json(state.inboxes.keys().cloned().collect()))
}
#[derive(Deserialize, utoipa::ToSchema)]
struct InboxImport {
    item_id: ItemId,
    expected_revision: Revision,
    package: String,
}
#[utoipa::path(post, path = "/api/v1/admin/inboxes/{name}/import", params(("name" = String, Path), ("Idempotency-Key" = String, Header)), request_body = InboxImport, responses((status = 202, body = serde_json::Value), (status = 403), (status = 409)))]
async fn import_inbox(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(name): Path<String>,
    Json(input): Json<InboxImport>,
) -> ApiResult<Response> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    state
        .service
        .authorize_import(&actor, input.item_id, input.expected_revision)?;
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
    let upload = state.service.create_upload(
        &actor,
        input.item_id,
        input.expected_revision,
        files,
        &context,
    )?;
    let id = upload.id;
    let source_locator = source
        .to_str()
        .ok_or_else(|| Error::Invalid("inbox source path must be UTF-8".into()))?;
    state
        .service
        .enqueue_inbox_upload(&actor, id, source_locator, &context)?;
    Ok((StatusCode::ACCEPTED, Json(serde_json::json!({"job_id":id}))).into_response())
}
