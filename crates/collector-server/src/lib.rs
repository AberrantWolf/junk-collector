use collector_api_types::{
    ApproveEnrollment, CompleteEnrollment, EnrollmentCompletion, MusicSearchHit, MusicSearchQuery,
    StartEnrollment,
};
use collector_app::{
    WorkEntry, WorkHistoryCommand, WorkJobView, WorkJobsQuery, WorkQuery, WorkTarget,
};
pub mod local_playback;
pub mod passkeys;
pub mod remote_media;
use axum::{
    Json, Router,
    body::Body,
    extract::{ConnectInfo, DefaultBodyLimit, FromRequest, Multipart, Path, Query, Request, State},
    http::{HeaderMap, HeaderValue, Method, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
};
use collector_app::{
    AccountUpdate, AccountView, AcquireAccurateRipCommand, AcquireMetadataCommand,
    ActivateCatalogSnapshotCommand, ApplyMetadataSnapshotCommand, AuthenticatedPrincipal,
    CatalogSnapshotStatus, CollectionService, CommandContext, ComponentDetails,
    CorrectIngestAssociationCommand, CreateAssetCommand, CreateCollectionCommand,
    CreateImportCommand, DisposeSpecimenCommand, EnrollmentStart, EnrollmentView, GrantView,
    ImportCatalogSnapshotCommand, ImportSourceKind, ImportSourceSummary, ImportView, IngestDetail,
    IngestDiscardResult, IngestPlan, IngestSummary, JobView, MediaResource, MetadataArtworkOption,
    MetadataCareQuery, MetadataPreferences, MetadataResolutionView, MetadataReviewView,
    MusicAlbumCoverage, MusicAlbumPreparationPlan, MusicPreparationBatchView,
    MusicTrackAvailability, NativeDeviceView, PasskeyView, PlanIngestCommand, PrepareMusicAlbum,
    PrepareMusicTrack, PresentedAsset, PresentedCollection, PresentedEvidence, PresentedItem,
    ProcessingSummary, PublishProviderArtworkCommand, ResolveMetadataCommand, RetryIngestCommand,
    ReturnLoanCommand, ReviewImportCommand, SaveItemCommand, SaveItemEditCommand, SessionView,
    SpecimenDetails, StartIngestCommand, StartLoanCommand, StartProcessingCommand,
    UpdateAssetCommand, UpdateMetadataAuthorityCommand, UpdateMetadataPreferences,
};
use collector_client::Pairing;
use collector_core::*;
use collector_modules::{ModuleDescriptor, Registry};
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

#[derive(Clone, serde::Serialize, utoipa::ToSchema)]
pub struct CatalogSourceView {
    pub id: String,
    pub system: String,
    pub catalog: String,
    pub byte_domain: String,
    pub byte_domain_version: u32,
}

#[derive(Clone)]
pub struct AppState {
    pub store: Arc<Store>,
    pub service: CollectionService,
    pub exposure: Exposure,
    pub ingress: Ingress,
    pub pairing: Pairing,
    pub multi_user: bool,
    pub local_playback: Option<Arc<local_playback::LocalPlayback>>,
    pub remote_media_allowed: bool,
    pub remote_media: Option<Arc<remote_media::Configuration>>,
    pub direct_media: bool,
    pub backup_reminder: Option<collector_app::BackupReminderSettings>,
    pub inboxes: Arc<std::collections::BTreeMap<String, std::path::PathBuf>>,
    pub catalog_sources: Arc<Vec<CatalogSourceView>>,
    pub allowed_origins: Arc<Vec<String>>,
    pub passkeys: passkeys::Configuration,
    pub rate_limits: Arc<RateLimits>,
    artwork_preview_slots: Arc<tokio::sync::Semaphore>,
    catalog_upload_slots: Arc<tokio::sync::Semaphore>,
}
impl AppState {
    pub fn new(store: Arc<Store>, pairing: Pairing) -> Self {
        Self::new_with_registry(store, pairing, Arc::new(Registry::builtins()))
    }

    pub fn new_with_registry(store: Arc<Store>, pairing: Pairing, modules: Arc<Registry>) -> Self {
        let service = CollectionService::new(store.clone(), modules)
            .with_catalog_uploads(store.clone())
            .with_backup_history(store.clone());
        Self {
            store,
            service,
            exposure: Exposure::default(),
            ingress: Ingress::Public,
            pairing,
            multi_user: false,
            local_playback: None,
            remote_media_allowed: false,
            remote_media: None,
            direct_media: false,
            backup_reminder: None,
            inboxes: Arc::new(std::collections::BTreeMap::new()),
            catalog_sources: Arc::new(Vec::new()),
            allowed_origins: Arc::new(vec![]),
            rate_limits: Arc::new(RateLimits::default()),
            artwork_preview_slots: Arc::new(tokio::sync::Semaphore::new(4)),
            catalog_upload_slots: Arc::new(tokio::sync::Semaphore::new(2)),
            passkeys: passkeys::Configuration::Disabled(passkeys::UnavailableReason::NotConfigured),
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

pub fn start_job_workers(state: AppState) -> collector_core::Result<JobRuntime> {
    let settings = state.service.job_settings()?;
    let worker_instance = ServerId::new().to_string();
    let mut workers = Vec::new();
    let recovery_state = state.clone();
    workers.push(tokio::spawn(async move {
        loop {
            let state = recovery_state.clone();
            match tokio::task::spawn_blocking(move || {
                let recovery = (|| {
                    state.service.reconcile_provider_acquisitions()?;
                    recover_guided_inbox_staging(&state)?;
                    state.service.resume_bulk_intakes()?;
                    state.service.resume_bulk_library()?;
                    state.service.resume_guided_ingests()
                })();
                // An unrelated recovery error must not suppress grouping, nor
                // may a grouping failure prevent ordinary workflow recovery.
                let grouping = state.service.reconcile_automatic_album_groups();
                recovery.and(grouping)
            })
            .await
            {
                Ok(Ok(())) => {}
                Ok(Err(error)) => tracing::error!(%error, "workflow recovery failed"),
                Err(error) => tracing::error!(%error, "workflow recovery panicked"),
            }
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }
    }));
    for worker_index in 0..settings.execution_loops() {
        let service = state.service.clone();
        let worker_instance = worker_instance.clone();
        workers.push(tokio::spawn(async move {
            loop {
                let service = service.clone();
                let instance = worker_instance.clone();
                match tokio::task::spawn_blocking(move || service.run_next_job(&instance)).await {
                    Ok(Ok(true)) => continue,
                    Ok(Ok(false)) => {
                        tokio::time::sleep(std::time::Duration::from_millis(250)).await
                    }
                    Ok(Err(error)) => {
                        tracing::error!(worker_index,%error,"job worker failed");
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    }
                    Err(error) => {
                        tracing::error!(worker_index,%error,"job worker panicked");
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    }
                }
            }
        }));
    }
    Ok(JobRuntime(workers))
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
    enrollment_polls: VecDeque<u64>,
    enrollment_poll_peers: HashMap<String, VecDeque<u64>>,
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
        purge_buckets(&mut state.peers, now, WINDOW_MILLIS);
        let account_key = Digest::sha256(account.trim().as_bytes()).hex;
        purge_buckets(&mut state.accounts, now, WINDOW_MILLIS);
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
            .entry(peer.to_owned())
            .or_default()
            .push_back(now);
        state
            .accounts
            .entry(account_key)
            .or_default()
            .push_back(now);
        Ok(())
    }

    fn check_enrollment_poll(&self, peer: &str) -> Result<()> {
        let now = self.clock.monotonic_millis();
        let mut state = lock_rate(&self.state)?;
        purge(&mut state.enrollment_polls, now, 60_000);
        purge_buckets(&mut state.enrollment_poll_peers, now, 60_000);
        if state.enrollment_polls.len() >= 600
            || state
                .enrollment_poll_peers
                .get(peer)
                .is_some_and(|v| v.len() >= 120)
        {
            return Err(Error::RateLimited(60));
        }
        state.enrollment_polls.push_back(now);
        state
            .enrollment_poll_peers
            .entry(peer.to_owned())
            .or_default()
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
        purge_buckets(&mut state.enrollment_peers, now, WINDOW_MILLIS);
        let account_key = Digest::sha256(account.trim().as_bytes()).hex;
        purge_buckets(&mut state.enrollment_accounts, now, WINDOW_MILLIS);
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
            .entry(peer.to_owned())
            .or_default()
            .push_back(now);
        state
            .enrollment_accounts
            .entry(account_key)
            .or_default()
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

fn purge_buckets(buckets: &mut HashMap<String, VecDeque<u64>>, now: u64, window: u64) {
    buckets.retain(|_, values| {
        purge(values, now, window);
        !values.is_empty()
    });
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
            Error::PreparedMediaInvalid => (
                StatusCode::CONFLICT,
                "prepared_media_invalid",
                "Prepared audio is missing, unreadable, or differs from its recorded bytes. The original is saved; ask a curator to repair this prepared track.".into(),
            ),
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
    let authenticated = if let Some(cookie) = cookie {
        state.service.authenticate(cookie)?
    } else {
        let bearer = bearer.ok_or(Error::Unauthenticated)?;
        match state.service.authenticate_native(bearer) {
            Ok(authenticated) => authenticated,
            Err(_) => state.service.authenticate(bearer)?,
        }
    };
    // This is a precondition, never an alternative source of authentication.
    // It binds a browser command to its submitting account across cookie changes.
    if let Some(expected) = headers.get("x-collector-actor") {
        let expected = expected
            .to_str()
            .ok()
            .and_then(|value| value.parse::<UserId>().ok());
        if expected != Some(authenticated.principal.user_id) {
            return Err(Error::Unauthenticated.into());
        }
    }
    Ok(authenticated)
}
fn principal(state: &AppState, headers: &HeaderMap) -> ApiResult<Principal> {
    Ok(authenticated(state, headers)?.principal)
}

// Reject untrusted large uploads before body extractors allocate or parse them.
// Collection-specific authority is still checked by each application command.
async fn authenticated_api(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> ApiResult<Response> {
    principal(&state, request.headers())?;
    Ok(next.run(request).await)
}

async fn administrator_api(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> ApiResult<Response> {
    state
        .service
        .require_administrator(&principal(&state, request.headers())?)?;
    Ok(next.run(request).await)
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
        .route("/pair", get(index))
        .route("/remote-media", get(index))
        .route("/lists", get(index))
        .route("/app.js", get(app_js))
        .route("/playback-sw.js", get(local_playback::worker))
        .route("/app.css", get(app_css))
        .route("/security.js", get(security_js))
        .merge(client_api);
    if state.exposure.allows(state.ingress, Surface::Curation) {
        let (curation_api, _) = curation_api_router().split_for_parts();
        router = router
            .route("/curate", get(index))
            .route("/curation.js", get(curation_js))
            .route("/library-care/metadata", get(index))
            .route("/library-metadata.js", get(library_metadata_js))
            .route("/imports", get(index))
            .route("/library-care/music-preparation", get(index))
            .route("/activity", get(index))
            .route("/library-preparation.js", get(library_preparation_js))
            .route("/library-activity.js", get(library_activity_js))
            .route("/library-imports.js", get(library_imports_js))
            .route("/collection-access.js", get(collection_access_js))
            .route("/work-history.js", get(work_history_js))
            .merge(curation_api.route_layer(middleware::from_fn_with_state(
                state.clone(),
                authenticated_api,
            )));
    }
    if state
        .exposure
        .allows(state.ingress, Surface::Administration)
    {
        let (administration_api, _) = administration_api_router().split_for_parts();
        router = router
            .route("/admin", get(index))
            .route("/administration.js", get(administration_js))
            .merge(
                administration_api.route_layer(middleware::from_fn_with_state(
                    state.clone(),
                    administrator_api,
                )),
            );
    }
    router
        .layer(DefaultBodyLimit::max(2 * 1024 * 1024))
        .layer(middleware::from_fn_with_state(state.clone(), security))
        .with_state(state)
}

fn client_api_router() -> OpenApiRouter<AppState> {
    OpenApiRouter::default()
        .routes(routes!(identity))
        .routes(routes!(local_playback::routes))
        .routes(routes!(local_playback::browser_grant))
        .routes(routes!(remote_media::remote_grant))
        .routes(routes!(remote_media::remote_test))
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
        .routes(routes!(metadata_preferences, update_metadata_preferences))
        .routes(routes!(pairing))
        .routes(routes!(revoke))
        .routes(routes!(sessions))
        .routes(routes!(replace_recovery_codes))
        .routes(routes!(change_password))
        .routes(routes!(revoke_other_access))
        .routes(routes!(list_passkeys))
        .routes(routes!(revoke_passkey))
        .routes(routes!(start_enrollment))
        .routes(routes!(approve_enrollment))
        .routes(routes!(preview_enrollment))
        .routes(routes!(enrollment_status))
        .routes(routes!(cancel_native_enrollment))
        .routes(routes!(complete_enrollment))
        .routes(routes!(cancel_enrollment))
        .routes(routes!(native_devices))
        .routes(routes!(revoke_native_device))
        .routes(routes!(collection_types))
        .routes(routes!(collection_search))
        .routes(routes!(collection_detail))
        .routes(routes!(collection_lists))
        .routes(routes!(collection_list_view))
        .routes(routes!(browse))
        .routes(routes!(music_search))
        .routes(routes!(client_collections))
        .routes(routes!(client_albums))
        .routes(routes!(client_album_playback))
        .routes(routes!(client_music_tracks))
        .routes(routes!(item))
        .routes(routes!(evidence))
        .routes(routes!(activity, save_activity))
        .routes(routes!(item_asset))
        .routes(routes!(asset_metadata))
        .routes(routes!(asset_original))
        .routes(routes!(asset_text_preview))
        .routes(routes!(asset_thumbnail))
        .routes(routes!(music_track_availability))
        .routes(routes!(music_album_coverage))
        .routes(routes!(media))
}

fn curation_api_router() -> OpenApiRouter<AppState> {
    OpenApiRouter::default()
        .routes(routes!(create_collection_list))
        .routes(routes!(update_collection_list))
        .routes(routes!(collection_grants))
        .routes(routes!(
            get_collection_grant,
            set_collection_grant,
            remove_collection_grant
        ))
        .routes(routes!(save_item))
        .routes(routes!(remove_item))
        .routes(routes!(restore_item))
        .routes(routes!(create_specimen))
        .routes(routes!(update_specimen))
        .routes(routes!(create_component))
        .routes(routes!(update_component, remove_component))
        .routes(routes!(start_loan))
        .routes(routes!(return_loan))
        .routes(routes!(dispose_specimen))
        .routes(routes!(verify))
        .routes(routes!(start_processing_operation))
        .routes(routes!(processing_plan))
        .routes(routes!(acquire_accuraterip_snapshot))
        .routes(routes!(capture_processing_summaries))
        .routes(routes!(curation_catalog_snapshots))
        .routes(routes!(create_upload))
        .routes(routes!(create_provisional_import))
        .routes(routes!(create_intake, list_intakes))
        .routes(routes!(intake_detail))
        .routes(
            routes!(append_intake, intake_members).layer(DefaultBodyLimit::max(16 * 1024 * 1024)),
        )
        .routes(routes!(repair_intake_destination))
        .routes(routes!(start_intake))
        .routes(routes!(cancel_intake))
        .routes(routes!(plan_ingest))
        .routes(routes!(discover_ingest))
        .routes(routes!(start_ingest, list_ingests))
        .routes(routes!(ingest_detail))
        .routes(routes!(cleanup_ingest_inbox))
        .routes(routes!(discard_ingest))
        .routes(routes!(cancel_ingest))
        .routes(routes!(retry_ingest))
        .routes(routes!(correct_ingest_association))
        .routes(routes!(provisional_import))
        .routes(routes!(analyze_provisional_import))
        .routes(routes!(review_provisional_import))
        .routes(routes!(publish_provisional_import))
        .routes(
            routes!(stage_provisional_import_chunk).layer(DefaultBodyLimit::max(8 * 1024 * 1024)),
        )
        .routes(routes!(collection_inboxes))
        .routes(routes!(inbox_packages))
        .routes(routes!(stage_inbox_import))
        .routes(routes!(upload, cancel_upload))
        .routes(routes!(commit_upload))
        .routes(routes!(jobs))
        .routes(routes!(work_history, change_work_history))
        .routes(routes!(work_history_entry))
        .routes(routes!(work_jobs))
        .routes(routes!(job))
        .routes(routes!(cancel_job))
        .routes(routes!(retry_job))
        .routes(routes!(prepare_music_track))
        .routes(routes!(update_digital_purchase).layer(DefaultBodyLimit::max(16 * 1024)))
        .routes(routes!(music_album_preparation_plan))
        .routes(routes!(prepare_music_album, music_preparation_batches))
        .routes(routes!(music_preparation_batch))
        .routes(routes!(cancel_music_preparation_batch))
        .routes(routes!(retry_music_preparation_batch))
        .routes(routes!(metadata_review, update_metadata_authority))
        .routes(routes!(identify_game_entry))
        .routes(routes!(resolve_metadata, metadata_resolutions))
        .routes(routes!(metadata_care))
        .routes(routes!(preparation_care))
        .routes(routes!(discover_bulk_intake).layer(DefaultBodyLimit::max(64 * 1024 * 1024)))
        .routes(routes!(intake_discovery_page))
        .routes(routes!(preview_bulk_library))
        .routes(routes!(bulk_library_page))
        .routes(routes!(bulk_library_detail))
        .routes(routes!(bulk_library_members))
        .routes(routes!(start_bulk_library))
        .routes(routes!(cancel_bulk_library))
        .routes(routes!(retry_bulk_library))
        .routes(routes!(album_group_proposals))
        .routes(routes!(preview_album_group))
        .routes(routes!(confirm_album_group))
        .routes(routes!(album_group_undo, undo_album_group))
        .routes(routes!(acquire_metadata))
        .routes(routes!(complete_metadata_acquisition))
        .routes(routes!(apply_metadata_snapshot))
        .routes(routes!(metadata_artwork_options))
        .routes(routes!(metadata_artwork_preview))
        .routes(routes!(publish_metadata_artwork))
        .routes(routes!(create_asset).layer(DefaultBodyLimit::max(20 * 1024 * 1024)))
        .routes(routes!(update_asset, tombstone_asset))
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
        .routes(routes!(catalog_sources))
        .routes(
            routes!(upload_catalog_snapshot)
                .layer(DefaultBodyLimit::max(collector_app::MAX_DAT_UPLOAD_BYTES)),
        )
        .routes(
            routes!(inspect_catalog_upload)
                .layer(DefaultBodyLimit::max(512 * 1024 * 1024 + 1024 * 1024)),
        )
        .routes(routes!(accept_catalog_upload))
        .routes(routes!(catalog_snapshots, import_catalog_snapshot))
        .routes(routes!(activate_catalog_snapshot))
}

pub fn openapi() -> OpenApi {
    let mut document = client_api_router()
        .merge(curation_api_router())
        .merge(administration_api_router())
        .into_openapi();
    document.info.title = "Junk Collector API".into();
    document.info.version = "1.0.0".into();
    for (path, item) in &mut document.paths.paths {
        let limit = if path.ends_with("/catalog-upload/inspect") {
            512 * 1024 * 1024 + 1024 * 1024
        } else if path.ends_with("/catalog-upload") {
            collector_app::MAX_DAT_UPLOAD_BYTES
        } else if path.ends_with("/cover") || path.ends_with("/assets") {
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
        || (path.starts_with("/api/v1/enrollments/")
            && (path.ends_with("/complete")
                || path.ends_with("/status")
                || path.ends_with("/cancel")));
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
    let mut local_origins = state
        .local_playback
        .as_ref()
        .map(|local| local.connect_origins())
        .unwrap_or_default();
    let media_origin = state
        .remote_media
        .as_ref()
        .and_then(|remote| url::Url::parse(&remote.base_url).ok())
        .map(|url| url.origin().ascii_serialization())
        .unwrap_or_default();
    if !media_origin.is_empty() {
        local_origins.push(media_origin.clone());
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
    if !local_origins.is_empty() {
        let csp = format!(
            "default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' blob:; media-src 'self' blob: {}; connect-src 'self' {}; frame-ancestors 'none'; base-uri 'none'; form-action 'self'",
            media_origin,
            local_origins.join(" ")
        );
        if let Ok(value) = HeaderValue::from_str(&csp) {
            response
                .headers_mut()
                .insert("content-security-policy", value);
        }
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
async fn library_preparation_js() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/javascript")],
        include_str!("../../../web/dist/library-preparation.js"),
    )
}
async fn library_activity_js() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/javascript")],
        include_str!("../../../web/dist/library-activity.js"),
    )
}
async fn library_imports_js() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/javascript")],
        include_str!("../../../web/dist/library-imports.js"),
    )
}

async fn library_metadata_js() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
        include_str!("../../../web/dist/library-metadata.js"),
    )
}
async fn curation_js() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
        include_str!("../../../web/dist/curation.js"),
    )
}
async fn collection_access_js() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
        include_str!("../../../web/dist/collection-access.js"),
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
pub struct FeaturesResponse {
    pub default_username: Option<String>,
    pub multi_user: bool,
    pub curation: bool,
    pub administration: bool,
    pub passkeys: bool,
    pub passkeys_unavailable_reason: Option<passkeys::UnavailableReason>,
}

#[utoipa::path(get, path = "/api/v1/identity", responses((status = 200, body = IdentityResponse)))]
async fn identity(State(state): State<AppState>) -> Json<IdentityResponse> {
    Json(IdentityResponse {
        server_id: state.pairing.server_id,
        protocol_version: 1,
    })
}
#[utoipa::path(get, path = "/api/v1/features", responses((status = 200, body = FeaturesResponse)))]
async fn features(State(state): State<AppState>) -> ApiResult<Json<FeaturesResponse>> {
    Ok(Json(FeaturesResponse {
        default_username: if state.multi_user {
            None
        } else {
            Some(state.service.default_login_username()?)
        },
        multi_user: state.multi_user,
        curation: state.exposure.allows(state.ingress, Surface::Curation),
        administration: state
            .exposure
            .allows(state.ingress, Surface::Administration),
        passkeys: state.passkeys.get().is_some(),
        passkeys_unavailable_reason: state.passkeys.unavailable_reason(),
    }))
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

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
struct ChangePassword {
    expected_revision: Revision,
    new_password: String,
    confirmation: String,
}
#[utoipa::path(put, path = "/api/v1/security/password", request_body = ChangePassword, responses((status = 200, body = Revision), (status = 401), (status = 403), (status = 409)))]
async fn change_password(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<ChangePassword>,
) -> ApiResult<Json<Revision>> {
    let actor = authenticated(&state, &headers)?;
    let context = command_context(&headers)?;
    Ok(Json(
        blocking(move || {
            state.service.change_password(
                &actor,
                input.expected_revision,
                &input.new_password,
                &input.confirmation,
                &context,
            )
        })
        .await?,
    ))
}
#[utoipa::path(post, path = "/api/v1/security/revoke-other-access", responses((status = 204), (status = 401), (status = 403)))]
async fn revoke_other_access(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<StatusCode> {
    let actor = authenticated(&state, &headers)?;
    let context = command_context(&headers)?;
    blocking(move || state.service.revoke_other_access(&actor, &context)).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(get, path = "/api/v1/me/profile", responses((status = 200, body = AccountView), (status = 401)))]
async fn profile(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<AccountView>> {
    let actor = authenticated(&state, &headers)?;
    Ok(Json(state.service.profile(&actor)?))
}
#[utoipa::path(get, path = "/api/v1/me/metadata-preferences", responses((status = 200, body = MetadataPreferences), (status = 401)))]
async fn metadata_preferences(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<MetadataPreferences>> {
    let actor = principal(&state, &headers)?;
    Ok(Json(
        blocking(move || state.service.metadata_preferences(&actor)).await?,
    ))
}
#[utoipa::path(put, path = "/api/v1/me/metadata-preferences", params(("Idempotency-Key" = String, Header)), request_body = UpdateMetadataPreferences, responses((status = 200, body = MetadataPreferences), (status = 401), (status = 409)))]
async fn update_metadata_preferences(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(command): Json<UpdateMetadataPreferences>,
) -> ApiResult<Json<MetadataPreferences>> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    Ok(Json(
        blocking(move || {
            state
                .service
                .update_metadata_preferences(&actor, command, &context)
        })
        .await?,
    ))
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
#[derive(Deserialize, utoipa::IntoParams)]
struct SecurityPageQuery {
    cursor: Option<String>,
    limit: Option<u16>,
}

#[utoipa::path(get, path = "/api/v1/security/sessions", params(SecurityPageQuery), responses((status = 200, body = Page<SessionView>), (status = 401)))]
async fn sessions(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<SecurityPageQuery>,
) -> ApiResult<Json<Page<SessionView>>> {
    let actor = authenticated(&state, &headers)?;
    Ok(Json(state.service.sessions_page(
        &actor,
        query.cursor.as_deref(),
        query.limit.unwrap_or(50),
    )?))
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

#[utoipa::path(get, path = "/api/v1/security/passkeys", params(SecurityPageQuery), responses((status = 200, body = Page<PasskeyView>), (status = 401)))]
async fn list_passkeys(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<SecurityPageQuery>,
) -> ApiResult<Json<Page<PasskeyView>>> {
    let actor = principal(&state, &headers)?;
    Ok(Json(state.service.passkeys_page(
        &actor,
        query.cursor.as_deref(),
        query.limit.unwrap_or(50),
    )?))
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

#[utoipa::path(post, path = "/api/v1/enrollments/start", request_body = StartEnrollment, responses((status = 200, body = EnrollmentStart), (status = 401), (status = 429)))]
async fn start_enrollment(
    State(state): State<AppState>,
    peer: Option<axum::Extension<DirectPeer>>,
    Json(input): Json<StartEnrollment>,
) -> ApiResult<Json<EnrollmentStart>> {
    let peer = peer
        .as_ref()
        .map(|peer| peer.0.0.as_str())
        .unwrap_or("unknown-direct-peer");
    state.rate_limits.check_enrollment("unbound", peer)?;
    Ok(Json(
        blocking(move || state.service.start_enrollment(&input.device_name, "/pair")).await?,
    ))
}

#[utoipa::path(post, path = "/api/v1/enrollments/preview", request_body = ApproveEnrollment, responses((status = 200, body = EnrollmentView), (status = 401), (status = 400)))]
async fn preview_enrollment(
    State(state): State<AppState>,
    peer: Option<axum::Extension<DirectPeer>>,
    headers: HeaderMap,
    Json(input): Json<ApproveEnrollment>,
) -> ApiResult<Json<EnrollmentView>> {
    let actor = authenticated(&state, &headers)?;
    let peer = peer
        .as_ref()
        .map(|p| p.0.0.as_str())
        .unwrap_or("unknown-direct-peer");
    state
        .rate_limits
        .check_enrollment(&actor.principal.user_id.to_string(), peer)?;
    Ok(Json(
        blocking(move || state.service.preview_enrollment(&actor, &input.user_code)).await?,
    ))
}

#[utoipa::path(post, path = "/api/v1/enrollments/approve", request_body = ApproveEnrollment, responses((status = 200, body = EnrollmentView), (status = 401), (status = 403), (status = 410)))]
async fn approve_enrollment(
    State(state): State<AppState>,
    peer: Option<axum::Extension<DirectPeer>>,
    headers: HeaderMap,
    Json(input): Json<ApproveEnrollment>,
) -> ApiResult<Json<EnrollmentView>> {
    let actor = authenticated(&state, &headers)?;
    let peer = peer
        .as_ref()
        .map(|p| p.0.0.as_str())
        .unwrap_or("unknown-direct-peer");
    state
        .rate_limits
        .check_enrollment(&actor.principal.user_id.to_string(), peer)?;
    Ok(Json(
        blocking(move || state.service.approve_enrollment(&actor, &input.user_code)).await?,
    ))
}

#[utoipa::path(post, path = "/api/v1/enrollments/{id}/complete", params(("id" = EnrollmentId, Path)), request_body = CompleteEnrollment, responses((status = 200, body = EnrollmentCompletion<Pairing>), (status = 401), (status = 410), (status = 429)))]
async fn complete_enrollment(
    State(state): State<AppState>,
    peer: Option<axum::Extension<DirectPeer>>,
    Path(id): Path<EnrollmentId>,
    Json(input): Json<CompleteEnrollment>,
) -> ApiResult<Json<EnrollmentCompletion<Pairing>>> {
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
    Ok(Json(EnrollmentCompletion {
        issue,
        pairing: state.pairing,
    }))
}

#[utoipa::path(post, path = "/api/v1/enrollments/{id}/status", params(("id" = EnrollmentId, Path)), request_body = CompleteEnrollment, responses((status = 200, body = EnrollmentView), (status = 401), (status = 429)))]
async fn enrollment_status(
    State(state): State<AppState>,
    peer: Option<axum::Extension<DirectPeer>>,
    Path(id): Path<EnrollmentId>,
    Json(input): Json<CompleteEnrollment>,
) -> ApiResult<Json<EnrollmentView>> {
    let peer = peer
        .as_ref()
        .map(|peer| peer.0.0.as_str())
        .unwrap_or("unknown-direct-peer");
    state.rate_limits.check_enrollment_poll(peer)?;
    Ok(Json(
        blocking(move || {
            state
                .service
                .enrollment_status(id, &input.completion_secret, false)
        })
        .await?,
    ))
}
#[utoipa::path(post, path = "/api/v1/enrollments/{id}/cancel", params(("id" = EnrollmentId, Path)), request_body = CompleteEnrollment, responses((status = 200, body = EnrollmentView), (status = 401), (status = 429)))]
async fn cancel_native_enrollment(
    State(state): State<AppState>,
    peer: Option<axum::Extension<DirectPeer>>,
    Path(id): Path<EnrollmentId>,
    Json(input): Json<CompleteEnrollment>,
) -> ApiResult<Json<EnrollmentView>> {
    let peer = peer
        .as_ref()
        .map(|peer| peer.0.0.as_str())
        .unwrap_or("unknown-direct-peer");
    state.rate_limits.check_enrollment_poll(peer)?;
    Ok(Json(
        blocking(move || {
            state
                .service
                .enrollment_status(id, &input.completion_secret, true)
        })
        .await?,
    ))
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

#[utoipa::path(get, path = "/api/v1/security/native-devices", params(SecurityPageQuery), responses((status = 200, body = Page<NativeDeviceView>), (status = 401)))]
async fn native_devices(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<SecurityPageQuery>,
) -> ApiResult<Json<Page<NativeDeviceView>>> {
    let actor = authenticated(&state, &headers)?;
    Ok(Json(state.service.native_devices_page(
        &actor,
        query.cursor.as_deref(),
        query.limit.unwrap_or(50),
    )?))
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
#[utoipa::path(get, path = "/api/v1/collection-types", responses((status = 200, body = Vec<ModuleDescriptor>), (status = 401)))]
async fn collection_types(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<ModuleDescriptor>>> {
    let actor = principal(&state, &headers)?;
    Ok(Json(state.service.collection_type_descriptors(&actor)))
}
#[derive(Deserialize, utoipa::IntoParams)]
struct CollectionSearchQuery {
    search: Option<String>,
    cursor: Option<String>,
    limit: Option<u16>,
    min_role: Option<u8>,
    module: Option<String>,
}

#[utoipa::path(get, path = "/api/v1/collection-search", params(CollectionSearchQuery), responses((status = 200, body = Page<PresentedCollection>), (status = 400), (status = 401)))]
async fn collection_search(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<CollectionSearchQuery>,
) -> ApiResult<Json<Page<PresentedCollection>>> {
    let actor = principal(&state, &headers)?;
    Ok(Json(
        blocking(move || {
            state.service.collections_page(
                &actor,
                query.search.as_deref().unwrap_or(""),
                query.cursor.as_deref(),
                query.limit.unwrap_or(50),
                query.min_role,
                query.module.as_deref(),
            )
        })
        .await?,
    ))
}

#[utoipa::path(get, path = "/api/v1/collections/{id}", params(("id" = CollectionId, Path)), responses((status = 200, body = PresentedCollection), (status = 401), (status = 403), (status = 404)))]
async fn collection_detail(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<CollectionId>,
) -> ApiResult<Json<PresentedCollection>> {
    let actor = principal(&state, &headers)?;
    Ok(Json(
        blocking(move || state.service.presented_collection(&actor, id)).await?,
    ))
}
#[utoipa::path(get,path="/api/v1/client/collections",responses((status=200,body=Vec<collector_api_types::ClientCollection>)))]
async fn client_collections(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<collector_api_types::ClientCollection>>> {
    let actor = principal(&state, &headers)?;
    let curation = state.exposure.allows(state.ingress, Surface::Curation);
    let mut values = blocking(move || state.service.listening_collections(&actor)).await?;
    for collection in &mut values {
        collection.can_edit_playlists &= curation;
    }
    Ok(Json(values))
}
#[utoipa::path(get,path="/api/v1/client/albums",params(("collection_id"=Option<CollectionId>,Query),("search"=Option<String>,Query),("cursor"=Option<String>,Query),("limit"=Option<u16>,Query)),responses((status=200,body=Page<collector_api_types::ClientAlbum>)))]
async fn client_albums(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<BrowseQuery>,
) -> ApiResult<Json<Page<collector_api_types::ClientAlbum>>> {
    let actor = principal(&state, &headers)?;
    Ok(Json(
        blocking(move || state.service.listening_albums(&actor, query)).await?,
    ))
}
#[utoipa::path(get,path="/api/v1/items/{item}/client-tracks",params(("item"=ItemId,Path)),responses((status=200,body=Vec<collector_api_types::MusicTrackAvailability>),(status=403)))]
async fn client_music_tracks(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(item): Path<ItemId>,
) -> ApiResult<Json<Vec<collector_api_types::MusicTrackAvailability>>> {
    let actor = principal(&state, &headers)?;
    Ok(Json(
        blocking(move || {
            state
                .service
                .recorded_music_track_availability(&actor, item)
        })
        .await?,
    ))
}
#[derive(Deserialize)]
struct AlbumPlaybackQuery {
    source: Option<RepresentationId>,
}
#[utoipa::path(get,path="/api/v1/items/{item}/client-playback",params(("item"=ItemId,Path),("source"=Option<RepresentationId>,Query)),responses((status=200,body=collector_api_types::AlbumPlaybackSnapshot),(status=403)))]
async fn client_album_playback(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(item): Path<ItemId>,
    Query(query): Query<AlbumPlaybackQuery>,
) -> ApiResult<Json<collector_api_types::AlbumPlaybackSnapshot>> {
    let actor = principal(&state, &headers)?;
    Ok(Json(
        blocking(move || {
            state
                .service
                .album_playback_snapshot(&actor, item, query.source)
        })
        .await?,
    ))
}
#[utoipa::path(get, path = "/api/v1/collections/{collection}/music-search", params(("collection" = CollectionId, Path), ("search" = Option<String>, Query), ("cursor" = Option<String>, Query), ("limit" = Option<u16>, Query)), responses((status=200,body=Page<MusicSearchHit>),(status=403)))]
async fn music_search(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(collection): Path<CollectionId>,
    Query(query): Query<MusicSearchQuery>,
) -> ApiResult<Json<Page<MusicSearchHit>>> {
    let actor = principal(&state, &headers)?;
    Ok(Json(
        blocking(move || state.service.music_search(&actor, collection, query)).await?,
    ))
}
#[utoipa::path(get, path = "/api/v1/items", params(("sort" = inline(Option<BrowseSort>), Query), ("filter" = inline(Option<BrowseFilter>), Query), ("collection_id" = Option<CollectionId>, Query), ("search" = Option<String>, Query), ("cursor" = Option<String>, Query), ("limit" = Option<u16>, Query)), responses((status = 200, body = Page<PresentedItem>), (status = 401)))]
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

#[derive(serde::Serialize, utoipa::ToSchema)]
struct JobAccepted {
    job_id: JobId,
}

#[utoipa::path(get,path="/api/v1/items/{id}/album-group-proposals",params(("id"=ItemId,Path),("cursor"=Option<String>,Query),("limit"=Option<u16>,Query)),responses((status=200,body=Page<PresentedItem>),(status=403)))]
async fn album_group_proposals(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<ItemId>,
    Query(query): Query<IntakePageQuery>,
) -> ApiResult<Json<Page<PresentedItem>>> {
    let actor = principal(&state, &headers)?;
    let after = query
        .cursor
        .map(|v| v.parse())
        .transpose()
        .map_err(|_| Error::Invalid("invalid grouping cursor".into()))?;
    Ok(Json(
        blocking(move || {
            state
                .service
                .album_group_proposals(&actor, id, after, query.limit.unwrap_or(25))
        })
        .await?,
    ))
}

#[utoipa::path(post, path="/api/v1/album-groups/preview", request_body=collector_app::PreviewAlbumGroup, responses((status=200,body=collector_app::AlbumGroupPreview),(status=403),(status=409)))]
async fn preview_album_group(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(command): Json<collector_app::PreviewAlbumGroup>,
) -> ApiResult<Json<collector_app::AlbumGroupPreview>> {
    let actor = principal(&state, &headers)?;
    Ok(Json(
        blocking(move || state.service.preview_album_group(&actor, command)).await?,
    ))
}
#[utoipa::path(post, path="/api/v1/album-groups", params(("Idempotency-Key"=String,Header)),request_body=collector_app::ConfirmAlbumGroup, responses((status=200,body=collector_app::AlbumGroupResult),(status=403),(status=409)))]
async fn confirm_album_group(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(command): Json<collector_app::ConfirmAlbumGroup>,
) -> ApiResult<Json<collector_app::AlbumGroupResult>> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    Ok(Json(
        blocking(move || state.service.confirm_album_group(&actor, command, &context)).await?,
    ))
}
#[utoipa::path(get,path="/api/v1/album-groups/{id}/undo",params(("id"=TransactionId,Path)),responses((status=200,body=collector_app::AlbumGroupUndo),(status=403)))]
async fn album_group_undo(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<TransactionId>,
) -> ApiResult<Json<collector_app::AlbumGroupUndo>> {
    let actor = principal(&state, &headers)?;
    Ok(Json(
        blocking(move || state.service.album_group_undo(&actor, id)).await?,
    ))
}
#[utoipa::path(post,path="/api/v1/album-groups/{id}/undo",params(("id"=TransactionId,Path),("Idempotency-Key"=String,Header)),responses((status=200,body=collector_app::AlbumGroupResult),(status=403),(status=409)))]
async fn undo_album_group(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<TransactionId>,
) -> ApiResult<Json<collector_app::AlbumGroupResult>> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    Ok(Json(
        blocking(move || state.service.undo_album_group(&actor, id, &context)).await?,
    ))
}

#[utoipa::path(get, path = "/api/v1/library-care/metadata", params(("collection_id" = Option<CollectionId>, Query), ("search" = Option<String>, Query), ("reason" = Option<collector_app::MetadataReviewReason>, Query), ("needs_attention" = Option<bool>, Query), ("cursor" = Option<String>, Query), ("limit" = Option<u16>, Query)), responses((status = 200, body = Page<PresentedItem>), (status = 401), (status = 403)))]
async fn metadata_care(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<MetadataCareQuery>,
) -> ApiResult<Json<Page<PresentedItem>>> {
    let actor = principal(&state, &headers)?;
    Ok(Json(
        blocking(move || state.service.metadata_care(&actor, query)).await?,
    ))
}

#[utoipa::path(get, path = "/api/v1/items/{id}/metadata/resolutions", params(("id" = ItemId, Path)), responses((status = 200, body = Vec<MetadataResolutionView>), (status = 401), (status = 403)))]
async fn metadata_resolutions(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<ItemId>,
) -> ApiResult<Json<Vec<MetadataResolutionView>>> {
    let actor = principal(&state, &headers)?;
    Ok(Json(
        blocking(move || state.service.metadata_resolutions(&actor, id)).await?,
    ))
}

#[utoipa::path(post, path = "/api/v1/items/{id}/metadata/resolutions", params(("id" = ItemId, Path), ("Idempotency-Key" = String, Header)), request_body = ResolveMetadataCommand, responses((status = 202, body = JobAccepted), (status = 401), (status = 403), (status = 409)))]
async fn resolve_metadata(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<ItemId>,
    Json(command): Json<ResolveMetadataCommand>,
) -> ApiResult<(StatusCode, Json<JobAccepted>)> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    let job_id = blocking(move || {
        state
            .service
            .resolve_metadata(&actor, id, command, &context)
    })
    .await?;
    Ok((StatusCode::ACCEPTED, Json(JobAccepted { job_id })))
}

#[utoipa::path(get, path = "/api/v1/items/{id}/metadata", params(("id" = ItemId, Path)), responses((status = 200, body = MetadataReviewView), (status = 403), (status = 404)))]
async fn metadata_review(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<ItemId>,
) -> ApiResult<Json<MetadataReviewView>> {
    let actor = principal(&state, &headers)?;
    Ok(Json(
        blocking(move || state.service.metadata_review(&actor, id)).await?,
    ))
}

#[utoipa::path(put, path = "/api/v1/items/{id}/metadata", params(("id" = ItemId, Path), ("Idempotency-Key" = String, Header)), request_body = UpdateMetadataAuthorityCommand, responses((status = 200, body = MetadataReviewView), (status = 403), (status = 404), (status = 409), (status = 422)))]
async fn update_metadata_authority(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<ItemId>,
    Json(command): Json<UpdateMetadataAuthorityCommand>,
) -> ApiResult<Json<MetadataReviewView>> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    Ok(Json(
        blocking(move || {
            state
                .service
                .update_metadata_authority(&actor, id, command, &context)?;
            state.service.metadata_review(&actor, id)
        })
        .await?,
    ))
}

#[utoipa::path(post, path = "/api/v1/items/{id}/metadata/acquisitions", params(("id" = ItemId, Path), ("Idempotency-Key" = String, Header)), request_body = AcquireMetadataCommand, responses((status = 202, body = JobAccepted), (status = 403), (status = 404), (status = 409), (status = 422), (status = 503)))]
async fn acquire_metadata(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<ItemId>,
    Json(command): Json<AcquireMetadataCommand>,
) -> ApiResult<(StatusCode, Json<JobAccepted>)> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    let job_id = blocking(move || {
        state
            .service
            .acquire_metadata(&actor, id, command, &context)
    })
    .await?;
    Ok((StatusCode::ACCEPTED, Json(JobAccepted { job_id })))
}

#[utoipa::path(post, path = "/api/v1/items/{id}/metadata/acquisitions/complete", params(("id" = ItemId, Path), ("Idempotency-Key" = String, Header)), request_body = collector_app::CompleteMetadataAcquisitionCommand, responses((status = 200, body = collector_app::CompletedMetadataAcquisition), (status = 403), (status = 404), (status = 409), (status = 422)))]
async fn complete_metadata_acquisition(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<ItemId>,
    Json(command): Json<collector_app::CompleteMetadataAcquisitionCommand>,
) -> ApiResult<Json<collector_app::CompletedMetadataAcquisition>> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    Ok(Json(
        blocking(move || {
            state
                .service
                .complete_metadata_acquisition(&actor, id, command, &context)
        })
        .await?,
    ))
}

#[utoipa::path(post, path = "/api/v1/items/{id}/metadata/refresh", params(("id" = ItemId, Path), ("Idempotency-Key" = String, Header)), request_body = ApplyMetadataSnapshotCommand, responses((status = 200, body = MetadataReviewView), (status = 403), (status = 404), (status = 409), (status = 422)))]
async fn apply_metadata_snapshot(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<ItemId>,
    Json(command): Json<ApplyMetadataSnapshotCommand>,
) -> ApiResult<Json<MetadataReviewView>> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    Ok(Json(
        blocking(move || {
            state
                .service
                .apply_metadata_snapshot(&actor, id, command, &context)
        })
        .await?,
    ))
}

#[derive(Deserialize)]
struct MetadataArtworkQuery {
    snapshot_id: SnapshotId,
}

#[utoipa::path(get, path = "/api/v1/items/{id}/metadata/artwork", params(("id" = ItemId, Path), ("snapshot_id" = SnapshotId, Query)), responses((status = 200, body = Vec<MetadataArtworkOption>), (status = 403), (status = 404), (status = 422)))]
async fn metadata_artwork_options(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<ItemId>,
    Query(query): Query<MetadataArtworkQuery>,
) -> ApiResult<Json<Vec<MetadataArtworkOption>>> {
    let actor = principal(&state, &headers)?;
    Ok(Json(
        blocking(move || {
            state
                .service
                .metadata_artwork_options(&actor, id, query.snapshot_id)
        })
        .await?,
    ))
}

#[utoipa::path(get, path = "/api/v1/items/{id}/metadata/artwork/{snapshot_id}/{image_id}/thumbnail", params(("id" = ItemId, Path), ("snapshot_id" = SnapshotId, Path), ("image_id" = String, Path)), responses((status = 200, content_type = "image/jpeg"), (status = 403), (status = 404), (status = 503)))]
async fn metadata_artwork_preview(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((id, snapshot_id, image_id)): Path<(ItemId, SnapshotId, String)>,
) -> ApiResult<Response> {
    let actor = principal(&state, &headers)?;
    struct CancelOnDrop(Arc<std::sync::atomic::AtomicBool>);
    impl Drop for CancelOnDrop {
        fn drop(&mut self) {
            self.0.store(true, std::sync::atomic::Ordering::Relaxed);
        }
    }
    let cancelled = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let _cancel = CancelOnDrop(cancelled.clone());
    let permit = state
        .artwork_preview_slots
        .acquire_owned()
        .await
        .map_err(|_| Error::Unavailable("Artwork preview service stopped".into()))?;
    let preview = blocking(move || {
        // Keep the permit until the blocking read actually exits, even when the
        // HTTP future disconnects. Never release concurrency bounds prematurely.
        let _permit = permit;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
        state
            .service
            .metadata_artwork_preview(&actor, id, snapshot_id, &image_id, &|| {
                Ok(cancelled.load(std::sync::atomic::Ordering::Relaxed)
                    || std::time::Instant::now() >= deadline)
            })
    })
    .await?;
    Ok((
        [
            (header::CONTENT_TYPE, preview.media_type),
            (header::CACHE_CONTROL, "private, no-store".into()),
        ],
        preview.bytes,
    )
        .into_response())
}

#[utoipa::path(post, path = "/api/v1/items/{id}/metadata/artwork", params(("id" = ItemId, Path), ("Idempotency-Key" = String, Header)), request_body = PublishProviderArtworkCommand, responses((status = 200, body = PresentedAsset), (status = 403), (status = 404), (status = 409), (status = 422)))]
async fn publish_metadata_artwork(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<ItemId>,
    Json(command): Json<PublishProviderArtworkCommand>,
) -> ApiResult<Json<PresentedAsset>> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    Ok(Json(
        blocking(move || {
            let asset = state
                .service
                .publish_provider_artwork(&actor, id, command, &context)?;
            state
                .service
                .asset_presented(&actor, asset.collection_id, asset.asset.id)
        })
        .await?,
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
struct SaveItemEditCommandEnvelope {
    edit: SaveItemEditCommand,
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
    Edit(SaveItemEditCommandEnvelope),
    Wave1(Box<Wave1SaveItemEnvelope>),
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
        match (
            object.contains_key("command"),
            object.contains_key("edit"),
            object.contains_key("item"),
        ) {
            (true, false, false) => serde_json::from_str(raw.get())
                .map(SaveItemEnvelope::Command)
                .map_err(serde::de::Error::custom),
            (false, true, false) => serde_json::from_str(raw.get())
                .map(SaveItemEnvelope::Edit)
                .map_err(serde::de::Error::custom),
            (false, false, true) => serde_json::from_str(raw.get())
                .map(SaveItemEnvelope::Wave1)
                .map_err(serde::de::Error::custom),
            _ => Err(serde::de::Error::custom(
                "item save must contain exactly one of command, edit or item",
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
            let saved = match input {
                SaveItemEnvelope::Command(input) => state.service.save_item(
                    &actor,
                    id,
                    input.command,
                    input.expected_revision,
                    &context,
                )?,
                SaveItemEnvelope::Edit(input) => state.service.save_item_edit(
                    &actor,
                    id,
                    input.edit,
                    input.expected_revision,
                    &context,
                )?,
                SaveItemEnvelope::Wave1(input) => {
                    let command = state.service.adapt_wave1_item_save(
                        &actor,
                        id,
                        input.item,
                        input.expected_revision,
                    )?;
                    state.service.save_item(
                        &actor,
                        id,
                        command,
                        input.expected_revision,
                        &context,
                    )?
                }
            };
            state.service.item_presented(&actor, saved.id)
        })
        .await?,
    ))
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
struct ItemLifecycleCommand {
    expected_revision: Revision,
}

#[utoipa::path(post, path = "/api/v1/items/{id}/remove", params(("id" = ItemId, Path), ("Idempotency-Key" = String, Header)), request_body = ItemLifecycleCommand, responses((status = 200, body = PresentedItem), (status = 403), (status = 404), (status = 409)))]
async fn remove_item(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<ItemId>,
    Json(input): Json<ItemLifecycleCommand>,
) -> ApiResult<Json<PresentedItem>> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    Ok(Json(
        blocking(move || {
            state
                .service
                .set_item_removed(&actor, id, input.expected_revision, true, &context)
        })
        .await?,
    ))
}

#[utoipa::path(post, path = "/api/v1/items/{id}/restore", params(("id" = ItemId, Path), ("Idempotency-Key" = String, Header)), request_body = ItemLifecycleCommand, responses((status = 200, body = PresentedItem), (status = 403), (status = 404), (status = 409)))]
async fn restore_item(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<ItemId>,
    Json(input): Json<ItemLifecycleCommand>,
) -> ApiResult<Json<PresentedItem>> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    Ok(Json(
        blocking(move || {
            state
                .service
                .set_item_removed(&actor, id, input.expected_revision, false, &context)
        })
        .await?,
    ))
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
struct SpecimenChange {
    expected_revision: Revision,
    details: SpecimenDetails,
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
struct ComponentChange {
    expected_revision: Revision,
    details: ComponentDetails,
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
struct StartLoan {
    expected_revision: Revision,
    command: StartLoanCommand,
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
struct ReturnLoan {
    expected_revision: Revision,
    command: ReturnLoanCommand,
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
struct DisposeSpecimen {
    expected_revision: Revision,
    command: DisposeSpecimenCommand,
}

#[utoipa::path(post, path = "/api/v1/items/{id}/specimens", params(("id" = ItemId, Path), ("Idempotency-Key" = String, Header)), request_body = SpecimenChange, responses((status = 200, body = PresentedItem), (status = 403), (status = 404), (status = 409), (status = 422)))]
async fn create_specimen(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<ItemId>,
    Json(input): Json<SpecimenChange>,
) -> ApiResult<Json<PresentedItem>> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    Ok(Json(
        blocking(move || {
            let saved = state.service.create_specimen(
                &actor,
                id,
                input.expected_revision,
                input.details,
                &context,
            )?;
            state.service.item_presented(&actor, saved.id)
        })
        .await?,
    ))
}

#[utoipa::path(put, path = "/api/v1/items/{id}/specimens/{specimen_id}", params(("id" = ItemId, Path), ("specimen_id" = SpecimenId, Path), ("Idempotency-Key" = String, Header)), request_body = SpecimenChange, responses((status = 200, body = PresentedItem), (status = 403), (status = 404), (status = 409), (status = 422)))]
async fn update_specimen(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((id, specimen_id)): Path<(ItemId, SpecimenId)>,
    Json(input): Json<SpecimenChange>,
) -> ApiResult<Json<PresentedItem>> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    Ok(Json(
        blocking(move || {
            let saved = state.service.update_specimen(
                &actor,
                id,
                specimen_id,
                input.expected_revision,
                input.details,
                &context,
            )?;
            state.service.item_presented(&actor, saved.id)
        })
        .await?,
    ))
}

#[utoipa::path(post, path = "/api/v1/items/{id}/specimens/{specimen_id}/components", params(("id" = ItemId, Path), ("specimen_id" = SpecimenId, Path), ("Idempotency-Key" = String, Header)), request_body = ComponentChange, responses((status = 200, body = PresentedItem), (status = 403), (status = 404), (status = 409), (status = 422)))]
async fn create_component(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((id, specimen_id)): Path<(ItemId, SpecimenId)>,
    Json(input): Json<ComponentChange>,
) -> ApiResult<Json<PresentedItem>> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    Ok(Json(
        blocking(move || {
            let saved = state.service.create_component(
                &actor,
                id,
                specimen_id,
                input.expected_revision,
                input.details,
                &context,
            )?;
            state.service.item_presented(&actor, saved.id)
        })
        .await?,
    ))
}

#[utoipa::path(put, path = "/api/v1/items/{id}/specimens/{specimen_id}/components/{component_id}", params(("id" = ItemId, Path), ("specimen_id" = SpecimenId, Path), ("component_id" = ComponentId, Path), ("Idempotency-Key" = String, Header)), request_body = ComponentChange, responses((status = 200, body = PresentedItem), (status = 403), (status = 404), (status = 409), (status = 422)))]
async fn update_component(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((id, specimen_id, component_id)): Path<(ItemId, SpecimenId, ComponentId)>,
    Json(input): Json<ComponentChange>,
) -> ApiResult<Json<PresentedItem>> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    Ok(Json(
        blocking(move || {
            let saved = state.service.update_component(
                &actor,
                (id, specimen_id, component_id),
                input.expected_revision,
                input.details,
                &context,
            )?;
            state.service.item_presented(&actor, saved.id)
        })
        .await?,
    ))
}

#[utoipa::path(delete, path = "/api/v1/items/{id}/specimens/{specimen_id}/components/{component_id}", params(("id" = ItemId, Path), ("specimen_id" = SpecimenId, Path), ("component_id" = ComponentId, Path), ExpectedRevision, ("Idempotency-Key" = String, Header)), responses((status = 200, body = PresentedItem), (status = 403), (status = 404), (status = 409)))]
async fn remove_component(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((id, specimen_id, component_id)): Path<(ItemId, SpecimenId, ComponentId)>,
    Query(query): Query<ExpectedRevision>,
) -> ApiResult<Json<PresentedItem>> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    Ok(Json(
        blocking(move || {
            let saved = state.service.remove_component(
                &actor,
                id,
                specimen_id,
                component_id,
                query.expected_revision,
                &context,
            )?;
            state.service.item_presented(&actor, saved.id)
        })
        .await?,
    ))
}

#[utoipa::path(post, path = "/api/v1/items/{id}/specimens/{specimen_id}/loans", params(("id" = ItemId, Path), ("specimen_id" = SpecimenId, Path), ("Idempotency-Key" = String, Header)), request_body = StartLoan, responses((status = 200, body = PresentedItem), (status = 403), (status = 404), (status = 409), (status = 422)))]
async fn start_loan(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((id, specimen_id)): Path<(ItemId, SpecimenId)>,
    Json(input): Json<StartLoan>,
) -> ApiResult<Json<PresentedItem>> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    Ok(Json(
        blocking(move || {
            let saved = state.service.start_loan(
                &actor,
                id,
                specimen_id,
                input.expected_revision,
                input.command,
                &context,
            )?;
            state.service.item_presented(&actor, saved.id)
        })
        .await?,
    ))
}

#[utoipa::path(post, path = "/api/v1/items/{id}/specimens/{specimen_id}/loans/{loan_id}/return", params(("id" = ItemId, Path), ("specimen_id" = SpecimenId, Path), ("loan_id" = LoanId, Path), ("Idempotency-Key" = String, Header)), request_body = ReturnLoan, responses((status = 200, body = PresentedItem), (status = 403), (status = 404), (status = 409), (status = 422)))]
async fn return_loan(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((id, specimen_id, loan_id)): Path<(ItemId, SpecimenId, LoanId)>,
    Json(input): Json<ReturnLoan>,
) -> ApiResult<Json<PresentedItem>> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    Ok(Json(
        blocking(move || {
            let saved = state.service.return_loan(
                &actor,
                (id, specimen_id, loan_id),
                input.expected_revision,
                input.command,
                &context,
            )?;
            state.service.item_presented(&actor, saved.id)
        })
        .await?,
    ))
}

#[utoipa::path(post, path = "/api/v1/items/{id}/specimens/{specimen_id}/dispose", params(("id" = ItemId, Path), ("specimen_id" = SpecimenId, Path), ("Idempotency-Key" = String, Header)), request_body = DisposeSpecimen, responses((status = 200, body = PresentedItem), (status = 403), (status = 404), (status = 409), (status = 422)))]
async fn dispose_specimen(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((id, specimen_id)): Path<(ItemId, SpecimenId)>,
    Json(input): Json<DisposeSpecimen>,
) -> ApiResult<Json<PresentedItem>> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    Ok(Json(
        blocking(move || {
            let saved = state.service.dispose_specimen(
                &actor,
                id,
                specimen_id,
                input.expected_revision,
                input.command,
                &context,
            )?;
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
async fn item_asset(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((id, asset)): Path<(ItemId, AssetId)>,
    Query(query): Query<AssetQuery>,
) -> ApiResult<Response> {
    let actor = principal(&state, &headers)?;
    let resource =
        blocking(move || state.service.asset(&actor, id, asset, query.thumbnail)).await?;
    file_response(resource, headers, false).await
}

#[utoipa::path(get, path = "/api/v1/collections/{collection}/assets/{asset}", params(("collection" = CollectionId, Path), ("asset" = AssetId, Path)), responses((status = 200, body = PresentedAsset), (status = 403), (status = 404)))]
async fn asset_metadata(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((collection, asset)): Path<(CollectionId, AssetId)>,
) -> ApiResult<Json<PresentedAsset>> {
    let actor = principal(&state, &headers)?;
    Ok(Json(
        blocking(move || state.service.asset_presented(&actor, collection, asset)).await?,
    ))
}

#[utoipa::path(get, path = "/api/v1/collections/{collection}/assets/{asset}/original", params(("collection" = CollectionId, Path), ("asset" = AssetId, Path)), responses((status = 200, content_type = "application/octet-stream"), (status = 206, content_type = "application/octet-stream"), (status = 403), (status = 404), (status = 416)))]
async fn asset_original(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((collection, asset)): Path<(CollectionId, AssetId)>,
) -> ApiResult<Response> {
    let actor = principal(&state, &headers)?;
    let resource = blocking(move || {
        state
            .service
            .asset_resource(&actor, collection, asset, false)
    })
    .await?;
    file_response(resource, headers, false).await
}

#[utoipa::path(get,path="/api/v1/collections/{collection}/assets/{asset}/text-preview",params(("collection"=CollectionId,Path),("asset"=AssetId,Path)),responses((status=200,body=collector_app::DocumentTextPreview),(status=400),(status=403),(status=404)))]
async fn asset_text_preview(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((collection, asset)): Path<(CollectionId, AssetId)>,
) -> ApiResult<Json<collector_app::DocumentTextPreview>> {
    let actor = principal(&state, &headers)?;
    Ok(Json(
        blocking(move || {
            state
                .service
                .document_text_preview(&actor, collection, asset)
        })
        .await?,
    ))
}

#[utoipa::path(get, path = "/api/v1/collections/{collection}/assets/{asset}/thumbnail", params(("collection" = CollectionId, Path), ("asset" = AssetId, Path)), responses((status = 200, content_type = "image/png"), (status = 206, content_type = "image/png"), (status = 403), (status = 404), (status = 416)))]
async fn asset_thumbnail(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((collection, asset)): Path<(CollectionId, AssetId)>,
) -> ApiResult<Response> {
    let actor = principal(&state, &headers)?;
    let resource = blocking(move || {
        state
            .service
            .asset_resource(&actor, collection, asset, true)
    })
    .await?;
    file_response(resource, headers, false).await
}

#[derive(Clone, Copy, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
enum AssetSubjectKind {
    Work,
    Edition,
    Item,
    Specimen,
    Component,
    Capture,
}

#[derive(Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
struct CreateAssetQuery {
    #[param(inline)]
    subject_kind: AssetSubjectKind,
    subject_id: String,
    #[param(inline)]
    kind: AssetKind,
    #[param(inline)]
    visibility: Visibility,
    alt_text: Option<String>,
}

impl CreateAssetQuery {
    fn subject(&self) -> Result<AssetSubject> {
        let invalid = || Error::Invalid("asset subject ID is invalid".into());
        Ok(match self.subject_kind {
            AssetSubjectKind::Work => AssetSubject::Work {
                work_id: self.subject_id.parse().map_err(|_| invalid())?,
            },
            AssetSubjectKind::Edition => AssetSubject::Edition {
                edition_id: self.subject_id.parse().map_err(|_| invalid())?,
            },
            AssetSubjectKind::Item => AssetSubject::Item {
                item_id: self.subject_id.parse().map_err(|_| invalid())?,
            },
            AssetSubjectKind::Specimen => AssetSubject::Specimen {
                specimen_id: self.subject_id.parse().map_err(|_| invalid())?,
            },
            AssetSubjectKind::Component => AssetSubject::Component {
                component_id: self.subject_id.parse().map_err(|_| invalid())?,
            },
            AssetSubjectKind::Capture => AssetSubject::Capture {
                capture_id: self.subject_id.parse().map_err(|_| invalid())?,
            },
        })
    }
}

#[utoipa::path(post, path = "/api/v1/collections/{collection}/assets", params(("collection" = CollectionId, Path), CreateAssetQuery, ("Idempotency-Key" = String, Header)), request_body(content = String, content_type = "application/octet-stream"), responses((status = 200, body = PresentedAsset), (status = 403), (status = 404), (status = 409), (status = 413), (status = 422), (status = 503)))]
async fn create_asset(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(collection): Path<CollectionId>,
    Query(query): Query<CreateAssetQuery>,
    body: axum::body::Bytes,
) -> ApiResult<Json<PresentedAsset>> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    let subject = query.subject()?;
    Ok(Json(
        blocking(move || {
            state.service.create_asset(
                &actor,
                CreateAssetCommand {
                    collection_id: collection,
                    subject,
                    kind: query.kind,
                    visibility: query.visibility,
                    alt_text: query.alt_text.unwrap_or_default(),
                },
                &body,
                &context,
            )
        })
        .await?,
    ))
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
struct UpdateAssetEnvelope {
    expected_revision: Revision,
    command: UpdateAssetCommand,
}

#[utoipa::path(put, path = "/api/v1/collections/{collection}/assets/{asset}/metadata", params(("collection" = CollectionId, Path), ("asset" = AssetId, Path), ("Idempotency-Key" = String, Header)), request_body = UpdateAssetEnvelope, responses((status = 200, body = PresentedAsset), (status = 403), (status = 404), (status = 409), (status = 422)))]
async fn update_asset(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((collection, asset)): Path<(CollectionId, AssetId)>,
    Json(input): Json<UpdateAssetEnvelope>,
) -> ApiResult<Json<PresentedAsset>> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    let current = state.service.asset_presented(&actor, collection, asset)?;
    if current.visibility.is_none() {
        return Err(Error::Forbidden.into());
    }
    Ok(Json(
        blocking(move || {
            state.service.update_asset(
                &actor,
                asset,
                input.expected_revision,
                input.command,
                &context,
            )
        })
        .await?,
    ))
}

#[utoipa::path(post, path = "/api/v1/collections/{collection}/assets/{asset}/archive", params(("collection" = CollectionId, Path), ("asset" = AssetId, Path), ExpectedRevision, ("Idempotency-Key" = String, Header)), responses((status = 200, body = PresentedAsset), (status = 403), (status = 404), (status = 409)))]
async fn tombstone_asset(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((collection, asset)): Path<(CollectionId, AssetId)>,
    Query(query): Query<ExpectedRevision>,
) -> ApiResult<Json<PresentedAsset>> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    let current = state.service.asset_presented(&actor, collection, asset)?;
    if current.visibility.is_none() {
        return Err(Error::Forbidden.into());
    }
    Ok(Json(
        blocking(move || {
            state
                .service
                .tombstone_asset(&actor, asset, query.expected_revision, &context)
        })
        .await?,
    ))
}
#[utoipa::path(get, path = "/api/v1/items/{id}/representations/{representation}/{index}", params(("id" = ItemId, Path), ("representation" = RepresentationId, Path), ("index" = usize, Path)), responses((status = 200, content_type = "application/octet-stream"), (status = 206, content_type = "application/octet-stream"), (status = 403), (status = 404), (status = 416)))]
async fn media(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
    Path((id, representation, index)): Path<(ItemId, RepresentationId, usize)>,
) -> ApiResult<Response> {
    if !state.direct_media && !state.remote_media_allowed {
        return Err(Error::Unavailable(
            "Local playback is required; public media fallback is disabled".into(),
        )
        .into());
    }
    let actor = principal(&state, &headers)?;
    let resource = blocking(move || {
        state
            .service
            .representation(&actor, id, representation, index)
    })
    .await?;
    file_response(resource, headers, method != Method::HEAD).await
}

#[utoipa::path(get, path = "/api/v1/items/{id}/music-tracks", params(("id" = ItemId, Path)), responses((status = 200, body = Vec<MusicTrackAvailability>), (status = 403), (status = 404)))]
async fn music_track_availability(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<ItemId>,
) -> ApiResult<Json<Vec<MusicTrackAvailability>>> {
    let actor = principal(&state, &headers)?;
    Ok(Json(
        blocking(move || state.service.music_track_availability(&actor, id)).await?,
    ))
}

#[utoipa::path(get, path = "/api/v1/items/{id}/music-disc-coverage", params(("id" = ItemId, Path)), responses((status = 200, body = MusicAlbumCoverage), (status = 403), (status = 404)))]
async fn music_album_coverage(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<ItemId>,
) -> ApiResult<Json<MusicAlbumCoverage>> {
    let actor = principal(&state, &headers)?;
    Ok(Json(
        blocking(move || state.service.music_album_coverage(&actor, id)).await?,
    ))
}

#[utoipa::path(post, path = "/api/v1/items/{item}/captures/{capture}/music-tracks/{track}/prepare", params(("item" = ItemId, Path), ("capture" = CaptureId, Path), ("track" = u16, Path), ("Idempotency-Key" = String, Header)), request_body = PrepareMusicTrack, responses((status = 202, body = serde_json::Value), (status = 400), (status = 403), (status = 404), (status = 409), (status = 503)))]
async fn prepare_music_track(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((item, capture, track)): Path<(ItemId, CaptureId, u16)>,
    Json(command): Json<PrepareMusicTrack>,
) -> ApiResult<Response> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    let job_id = blocking(move || {
        state
            .service
            .prepare_configured_music_track(&actor, item, capture, command, track, &context)
    })
    .await?;
    Ok((
        StatusCode::ACCEPTED,
        Json(serde_json::json!({"job_id": job_id})),
    )
        .into_response())
}

#[utoipa::path(get, path = "/api/v1/items/{item}/music-preparation-plan", params(("item" = ItemId, Path)), responses((status = 200, body = MusicAlbumPreparationPlan), (status = 403), (status = 404)))]
async fn music_album_preparation_plan(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(item): Path<ItemId>,
) -> ApiResult<Json<MusicAlbumPreparationPlan>> {
    let actor = principal(&state, &headers)?;
    Ok(Json(
        blocking(move || state.service.music_album_preparation_plan(&actor, item)).await?,
    ))
}

#[utoipa::path(post, path = "/api/v1/items/{item}/music-preparation-batches", params(("item" = ItemId, Path), ("Idempotency-Key" = String, Header)), request_body = PrepareMusicAlbum, responses((status = 202, body = MusicPreparationBatchView), (status = 400), (status = 403), (status = 404), (status = 409), (status = 503)))]
async fn prepare_music_album(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(item): Path<ItemId>,
    Json(command): Json<PrepareMusicAlbum>,
) -> ApiResult<Response> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    let batch = blocking(move || {
        state
            .service
            .prepare_music_album(&actor, item, command, &context)
    })
    .await?;
    Ok((StatusCode::ACCEPTED, Json(batch)).into_response())
}

#[utoipa::path(get, path = "/api/v1/items/{item}/music-preparation-batches", params(("item" = ItemId, Path)), responses((status = 200, body = Vec<MusicPreparationBatchView>), (status = 403), (status = 404)))]
async fn music_preparation_batches(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(item): Path<ItemId>,
) -> ApiResult<Json<Vec<MusicPreparationBatchView>>> {
    let actor = principal(&state, &headers)?;
    Ok(Json(
        blocking(move || state.service.music_preparation_batches(&actor, item)).await?,
    ))
}

#[utoipa::path(get, path = "/api/v1/music-preparation-batches/{batch}", params(("batch" = JobId, Path)), responses((status = 200, body = MusicPreparationBatchView), (status = 403), (status = 404)))]
async fn music_preparation_batch(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(batch): Path<JobId>,
) -> ApiResult<Json<MusicPreparationBatchView>> {
    let actor = principal(&state, &headers)?;
    Ok(Json(
        blocking(move || state.service.music_preparation_batch(&actor, batch)).await?,
    ))
}

#[utoipa::path(post, path = "/api/v1/music-preparation-batches/{batch}/cancel", params(("batch" = JobId, Path), ("Idempotency-Key" = String, Header)), responses((status = 200, body = MusicPreparationBatchView), (status = 403), (status = 404), (status = 409)))]
async fn cancel_music_preparation_batch(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(batch): Path<JobId>,
) -> ApiResult<Json<MusicPreparationBatchView>> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    Ok(Json(
        blocking(move || {
            state
                .service
                .cancel_music_preparation_batch(&actor, batch, &context)
        })
        .await?,
    ))
}

#[utoipa::path(post, path = "/api/v1/music-preparation-batches/{batch}/retry", params(("batch" = JobId, Path), ("Idempotency-Key" = String, Header)), responses((status = 200, body = MusicPreparationBatchView), (status = 403), (status = 404), (status = 409), (status = 503)))]
async fn retry_music_preparation_batch(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(batch): Path<JobId>,
) -> ApiResult<Json<MusicPreparationBatchView>> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    Ok(Json(
        blocking(move || {
            state
                .service
                .retry_music_preparation_batch(&actor, batch, &context)
        })
        .await?,
    ))
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
        limits.check_authentication("recent", "192.0.2.3").unwrap();
        clock.0.store(60_000, Ordering::SeqCst);
        limits.check_authentication("owner", "192.0.2.2").unwrap();
        for _ in 0..9 {
            limits.check_authentication("recent", "192.0.2.3").unwrap();
        }
        assert!(matches!(
            limits.check_authentication("recent", "192.0.2.4"),
            Err(Error::RateLimited(60))
        ));
    }

    #[test]
    fn authentication_alias_flood_is_throttled_without_retaining_rejected_identities() {
        let clock = Arc::new(TestRateClock(AtomicU64::new(0)));
        let limits = RateLimits::new(clock.clone());
        for index in 0..30 {
            limits
                .check_authentication(&format!("account-{index}"), "busy-peer")
                .unwrap();
        }
        assert!(matches!(
            limits.check_authentication("another-account", "busy-peer"),
            Err(Error::RateLimited(60))
        ));
        for index in 30..100 {
            limits
                .check_authentication(&format!("account-{index}"), &format!("peer-{index}"))
                .unwrap();
        }
        for index in 0..1000 {
            assert!(matches!(
                limits.check_authentication(
                    &format!("rejected-account-{index}"),
                    &format!("rejected-peer-{index}")
                ),
                Err(Error::RateLimited(60))
            ));
        }
        {
            let state = limits.state.lock().unwrap();
            assert_eq!(state.global.len(), 100);
            assert_eq!(state.accounts.len(), 100);
            assert!(state.peers.len() <= 100);
        }
        clock.0.store(59_999, Ordering::SeqCst);
        assert!(matches!(
            limits.check_authentication("new-account", "new-peer"),
            Err(Error::RateLimited(60))
        ));
        clock.0.store(60_000, Ordering::SeqCst);
        limits
            .check_authentication("new-account", "new-peer")
            .unwrap();
        let state = limits.state.lock().unwrap();
        assert_eq!(state.global.len(), 1);
        assert_eq!(state.accounts.len(), 1);
        assert_eq!(state.peers.len(), 1);
    }

    #[test]
    fn enrollment_and_poll_alias_floods_bound_state_and_reclaim_expired_buckets() {
        let clock = Arc::new(TestRateClock(AtomicU64::new(0)));
        let limits = RateLimits::new(clock.clone());
        for index in 0..8 {
            limits
                .check_enrollment("account", &format!("peer-{index}"))
                .unwrap();
        }
        assert!(matches!(
            limits.check_enrollment("account", "different-peer"),
            Err(Error::RateLimited(60))
        ));
        for index in 0..15 {
            limits
                .check_enrollment(&format!("account-{index}"), "busy-peer")
                .unwrap();
        }
        assert!(matches!(
            limits.check_enrollment("another-account", "busy-peer"),
            Err(Error::RateLimited(60))
        ));
        for index in 23..50 {
            limits
                .check_enrollment(&format!("account-{index}"), &format!("peer-{index}"))
                .unwrap();
        }
        for _ in 0..120 {
            limits.check_enrollment_poll("busy-poll-peer").unwrap();
        }
        assert!(matches!(
            limits.check_enrollment_poll("busy-poll-peer"),
            Err(Error::RateLimited(60))
        ));
        for index in 120..600 {
            limits
                .check_enrollment_poll(&format!("poll-peer-{index}"))
                .unwrap();
        }
        for index in 0..1000 {
            assert!(matches!(
                limits.check_enrollment(
                    &format!("rejected-account-{index}"),
                    &format!("rejected-peer-{index}")
                ),
                Err(Error::RateLimited(60))
            ));
            assert!(matches!(
                limits.check_enrollment_poll(&format!("rejected-poll-peer-{index}")),
                Err(Error::RateLimited(60))
            ));
        }
        {
            let state = limits.state.lock().unwrap();
            assert_eq!(state.enrollment_global.len(), 50);
            assert!(state.enrollment_peers.len() <= 50);
            assert!(state.enrollment_accounts.len() <= 50);
            assert_eq!(state.enrollment_polls.len(), 600);
            assert!(state.enrollment_poll_peers.len() <= 600);
        }
        clock.0.store(60_000, Ordering::SeqCst);
        limits.check_enrollment("new-account", "new-peer").unwrap();
        limits.check_enrollment_poll("new-poll-peer").unwrap();
        let state = limits.state.lock().unwrap();
        assert_eq!(state.enrollment_global.len(), 1);
        assert_eq!(state.enrollment_peers.len(), 1);
        assert_eq!(state.enrollment_accounts.len(), 1);
        assert_eq!(state.enrollment_polls.len(), 1);
        assert_eq!(state.enrollment_poll_peers.len(), 1);
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
fn verify_transfer_bytes(
    reader: &mut dyn collector_app::MediaReader,
    length: u64,
    digest: &Digest,
) -> Result<()> {
    use sha2::Digest as _;
    let mut hash = sha2::Sha256::new();
    let mut buffer = [0u8; 1024 * 1024];
    let mut read_bytes = 0u64;
    loop {
        let count = reader.read(&mut buffer).map_err(|error| {
            tracing::error!(%error, "prepared media read failed");
            Error::PreparedMediaInvalid
        })?;
        if count == 0 {
            break;
        }
        read_bytes = read_bytes
            .checked_add(count as u64)
            .ok_or(Error::Conflict)?;
        if read_bytes > length {
            return Err(Error::PreparedMediaInvalid);
        }
        hash.update(&buffer[..count]);
    }
    if read_bytes != length
        || digest.algorithm != HashAlgorithm::Sha256
        || hex::encode(hash.finalize()) != digest.hex
    {
        return Err(Error::PreparedMediaInvalid);
    }
    reader.seek(SeekFrom::Start(0)).map_err(|error| {
        tracing::error!(%error, "prepared media rewind failed");
        Error::PreparedMediaInvalid
    })?;
    Ok(())
}

#[cfg(test)]
mod derivative_transfer_tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn transfer_checks_derivative_bytes_before_serving_them() {
        let expected = b"published FLAC bytes";
        let digest = Digest::sha256(expected);
        let mut intact = Cursor::new(expected.to_vec());
        verify_transfer_bytes(&mut intact, expected.len() as u64, &digest).unwrap();
        assert_eq!(intact.position(), 0);

        let mut changed = Cursor::new(b"published fLAC bytes".to_vec());
        assert!(verify_transfer_bytes(&mut changed, expected.len() as u64, &digest).is_err());

        let mut truncated = Cursor::new(expected[..expected.len() - 1].to_vec());
        assert!(verify_transfer_bytes(&mut truncated, expected.len() as u64, &digest).is_err());
    }
}

async fn file_response(
    resource: MediaResource,
    headers: HeaderMap,
    verify_bytes: bool,
) -> ApiResult<Response> {
    let length = resource.length;
    let etag = format!("\"{}\"", resource.digest.hex);
    let media_type = resource.media_type;
    let mut reader = resource.reader;
    if verify_bytes && resource.verify_on_transfer {
        let digest = resource.digest;
        reader = blocking(move || {
            verify_transfer_bytes(reader.as_mut(), length, &digest)?;
            Ok(reader)
        })
        .await?;
    }
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
            return Response::builder()
                .status(StatusCode::RANGE_NOT_SATISFIABLE)
                .header(header::CONTENT_RANGE, format!("bytes */{length}"))
                .header(header::ACCEPT_RANGES, "bytes")
                .header(header::ETAG, etag)
                .header(header::CONTENT_LENGTH, 0)
                .body(Body::empty())
                .map_err(|error| Error::Storage(error.to_string()).into());
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

#[utoipa::path(post, path = "/api/v1/ingests/discover", request_body = collector_app::DiscoverIngestCommand, responses((status = 200, body = collector_app::DiscoveredIngestPlan), (status = 403), (status = 422)))]
async fn discover_ingest(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(command): Json<collector_app::DiscoverIngestCommand>,
) -> ApiResult<Json<collector_app::DiscoveredIngestPlan>> {
    let actor = principal(&state, &headers)?;
    Ok(Json(
        blocking(move || state.service.discover_ingest(&actor, command)).await?,
    ))
}

#[utoipa::path(get,path="/api/v1/library-care/music-preparation",params(("collection_id"=Option<CollectionId>,Query),("search"=Option<String>,Query),("needs_attention"=Option<bool>,Query),("cursor"=Option<String>,Query),("limit"=Option<u16>,Query)),responses((status=200,body=Page<collector_app::PreparationCareEntry>),(status=403)))]
async fn preparation_care(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<MetadataCareQuery>,
) -> ApiResult<Json<Page<collector_app::PreparationCareEntry>>> {
    let actor = principal(&state, &headers)?;
    Ok(Json(
        blocking(move || state.service.preparation_care(&actor, query)).await?,
    ))
}
#[utoipa::path(post,path="/api/v1/bulk-library/preview",params(("Idempotency-Key"=String,Header)),request_body=collector_app::PreviewBulkLibrary,responses((status=200,body=collector_app::BulkLibrarySummary),(status=403),(status=409)))]
async fn preview_bulk_library(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(command): Json<collector_app::PreviewBulkLibrary>,
) -> ApiResult<Json<collector_app::BulkLibrarySummary>> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    Ok(Json(
        blocking(move || {
            state
                .service
                .preview_bulk_library(&actor, command, &context)
        })
        .await?,
    ))
}
#[utoipa::path(get,path="/api/v1/bulk-library",params(("collection_id"=Option<CollectionId>,Query),("cursor"=Option<String>,Query),("limit"=Option<u16>,Query)),responses((status=200,body=Page<collector_app::BulkLibrarySummary>),(status=403)))]
async fn bulk_library_page(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<IntakePageQuery>,
) -> ApiResult<Json<Page<collector_app::BulkLibrarySummary>>> {
    let actor = principal(&state, &headers)?;
    let after = query
        .cursor
        .map(|v| v.parse())
        .transpose()
        .map_err(|_| Error::Invalid("invalid batch cursor".into()))?;
    Ok(Json(
        blocking(move || {
            state.service.bulk_library_page(
                &actor,
                query.collection_id,
                after,
                query.limit.unwrap_or(50),
            )
        })
        .await?,
    ))
}
#[utoipa::path(get,path="/api/v1/bulk-library/{id}",params(("id"=JobId,Path)),responses((status=200,body=collector_app::BulkLibrarySummary),(status=403)))]
async fn bulk_library_detail(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<JobId>,
) -> ApiResult<Json<collector_app::BulkLibrarySummary>> {
    let actor = principal(&state, &headers)?;
    Ok(Json(
        blocking(move || state.service.bulk_library(&actor, id)).await?,
    ))
}
#[utoipa::path(get,path="/api/v1/bulk-library/{id}/members",params(("id"=JobId,Path),("cursor"=Option<String>,Query),("limit"=Option<u16>,Query)),responses((status=200,body=Page<collector_app::BulkLibraryMemberView>),(status=403)))]
async fn bulk_library_members(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<JobId>,
    Query(query): Query<IntakePageQuery>,
) -> ApiResult<Json<Page<collector_app::BulkLibraryMemberView>>> {
    let actor = principal(&state, &headers)?;
    let after = query
        .cursor
        .map(|v| v.parse())
        .transpose()
        .map_err(|_| Error::Invalid("invalid batch member cursor".into()))?;
    Ok(Json(
        blocking(move || {
            state
                .service
                .bulk_library_members(&actor, id, after, query.limit.unwrap_or(50))
        })
        .await?,
    ))
}
#[utoipa::path(post,path="/api/v1/bulk-library/{id}/start",params(("id"=JobId,Path),("Idempotency-Key"=String,Header)),responses((status=200,body=collector_app::BulkLibrarySummary),(status=403),(status=409)))]
async fn start_bulk_library(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<JobId>,
) -> ApiResult<Json<collector_app::BulkLibrarySummary>> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    Ok(Json(
        blocking(move || state.service.start_bulk_library(&actor, id, &context)).await?,
    ))
}
#[utoipa::path(post,path="/api/v1/bulk-library/{id}/cancel",params(("id"=JobId,Path),("Idempotency-Key"=String,Header)),responses((status=200,body=collector_app::BulkLibrarySummary),(status=403)))]
async fn cancel_bulk_library(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<JobId>,
) -> ApiResult<Json<collector_app::BulkLibrarySummary>> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    Ok(Json(
        blocking(move || {
            state
                .service
                .change_bulk_library(&actor, id, true, &context)
        })
        .await?,
    ))
}
#[utoipa::path(post,path="/api/v1/bulk-library/{id}/retry",params(("id"=JobId,Path),("Idempotency-Key"=String,Header)),responses((status=200,body=collector_app::BulkLibrarySummary),(status=403),(status=409)))]
async fn retry_bulk_library(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<JobId>,
) -> ApiResult<Json<collector_app::BulkLibrarySummary>> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    Ok(Json(
        blocking(move || {
            state
                .service
                .change_bulk_library(&actor, id, false, &context)
        })
        .await?,
    ))
}

#[derive(Deserialize)]
struct IntakePageQuery {
    collection_id: Option<CollectionId>,
    cursor: Option<String>,
    limit: Option<u16>,
}
#[utoipa::path(post,path="/api/v1/intake-discoveries",params(("Idempotency-Key"=String,Header)),request_body=collector_app::DiscoverIngestCommand,responses((status=200,body=collector_app::IntakeDiscovery),(status=403),(status=422)))]
async fn discover_bulk_intake(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(command): Json<collector_app::DiscoverIngestCommand>,
) -> ApiResult<Json<collector_app::IntakeDiscovery>> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    Ok(Json(
        blocking(move || {
            state
                .service
                .discover_bulk_intake(&actor, command, &context)
        })
        .await?,
    ))
}
#[derive(Deserialize)]
struct DiscoveryPageQuery {
    cursor: Option<u32>,
    unassigned_cursor: Option<u32>,
}
#[utoipa::path(get,path="/api/v1/intake-discoveries/{id}",params(("id"=JobId,Path),("cursor"=Option<u32>,Query),("unassigned_cursor"=Option<u32>,Query)),responses((status=200,body=collector_app::IntakeDiscoveryPage),(status=403)))]
async fn intake_discovery_page(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<JobId>,
    Query(q): Query<DiscoveryPageQuery>,
) -> ApiResult<Json<collector_app::IntakeDiscoveryPage>> {
    let actor = principal(&state, &headers)?;
    Ok(Json(
        blocking(move || {
            state
                .service
                .intake_discovery_page(&actor, id, q.cursor, q.unassigned_cursor)
        })
        .await?,
    ))
}
#[utoipa::path(post,path="/api/v1/intakes",params(("Idempotency-Key"=String,Header)),request_body=collector_app::CreateIntake,responses((status=200,body=collector_app::IntakeSession),(status=403)))]
async fn create_intake(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(command): Json<collector_app::CreateIntake>,
) -> ApiResult<Json<collector_app::IntakeSession>> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    Ok(Json(
        blocking(move || state.service.create_intake(&actor, command, &context)).await?,
    ))
}
#[utoipa::path(get,path="/api/v1/intakes",params(("collection_id"=Option<CollectionId>,Query),("cursor"=Option<String>,Query),("limit"=Option<u16>,Query)),responses((status=200,body=Page<collector_app::IntakeSession>),(status=403)))]
async fn list_intakes(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<IntakePageQuery>,
) -> ApiResult<Json<Page<collector_app::IntakeSession>>> {
    let actor = principal(&state, &headers)?;
    let after = query
        .cursor
        .map(|v| v.parse())
        .transpose()
        .map_err(|_| Error::Invalid("invalid intake cursor".into()))?;
    Ok(Json(
        blocking(move || {
            state.service.intake_sessions(
                &actor,
                query.collection_id,
                after,
                query.limit.unwrap_or(50),
            )
        })
        .await?,
    ))
}
#[utoipa::path(get,path="/api/v1/intakes/{id}",params(("id"=JobId,Path)),responses((status=200,body=collector_app::IntakeSession),(status=403)))]
async fn intake_detail(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<JobId>,
) -> ApiResult<Json<collector_app::IntakeSession>> {
    let actor = principal(&state, &headers)?;
    Ok(Json(
        blocking(move || state.service.intake(&actor, id)).await?,
    ))
}
#[utoipa::path(post,path="/api/v1/intakes/{id}/members",params(("id"=JobId,Path),("Idempotency-Key"=String,Header)),request_body=collector_app::AppendIntake,responses((status=200,body=collector_app::IntakeSession),(status=403),(status=409),(status=422)))]
async fn append_intake(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<JobId>,
    Json(command): Json<collector_app::AppendIntake>,
) -> ApiResult<Json<collector_app::IntakeSession>> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    Ok(Json(
        blocking(move || {
            state
                .service
                .append_intake_with_preflight(&actor, id, command, &context, |plan| {
                    for package in &plan.packages {
                        if package.source.kind == ImportSourceKind::Inbox {
                            let (_, files) = resolve_inbox_package(&state, &package.source)?;
                            if !same_upload_inventory(&files, &package.files) {
                                return Err(Error::Conflict);
                            }
                        }
                    }
                    Ok(())
                })
        })
        .await?,
    ))
}
#[utoipa::path(get,path="/api/v1/intakes/{id}/members",params(("id"=JobId,Path),("cursor"=Option<String>,Query),("limit"=Option<u16>,Query)),responses((status=200,body=Page<collector_app::IntakeMemberView>),(status=403)))]
async fn intake_members(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<JobId>,
    Query(query): Query<IntakePageQuery>,
) -> ApiResult<Json<Page<collector_app::IntakeMemberView>>> {
    let actor = principal(&state, &headers)?;
    let after = query
        .cursor
        .map(|v| v.parse())
        .transpose()
        .map_err(|_| Error::Invalid("invalid member cursor".into()))?;
    Ok(Json(
        blocking(move || {
            state
                .service
                .intake_members(&actor, id, after, query.limit.unwrap_or(32))
        })
        .await?,
    ))
}
#[utoipa::path(post,path="/api/v1/intakes/{id}/members/{position}/destination",params(("id"=JobId,Path),("position"=u32,Path),("Idempotency-Key"=String,Header)),request_body=collector_app::RepairIntakeDestination,responses((status=200,body=collector_app::IntakeSession),(status=403),(status=409)))]
async fn repair_intake_destination(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((id, position)): Path<(JobId, u32)>,
    Json(command): Json<collector_app::RepairIntakeDestination>,
) -> ApiResult<Json<collector_app::IntakeSession>> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    Ok(Json(
        blocking(move || {
            state
                .service
                .repair_intake_destination(&actor, id, position, command, &context)
        })
        .await?,
    ))
}
#[utoipa::path(post,path="/api/v1/intakes/{id}/start",params(("id"=JobId,Path),("Idempotency-Key"=String,Header)),request_body=collector_app::StartIntake,responses((status=200,body=collector_app::IntakeSession),(status=403),(status=409)))]
async fn start_intake(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<JobId>,
    Json(command): Json<collector_app::StartIntake>,
) -> ApiResult<Json<collector_app::IntakeSession>> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    Ok(Json(
        blocking(move || state.service.start_intake(&actor, id, command, &context)).await?,
    ))
}
#[utoipa::path(post,path="/api/v1/intakes/{id}/cancel",params(("id"=JobId,Path),("Idempotency-Key"=String,Header)),responses((status=200,body=collector_app::IntakeSession),(status=403)))]
async fn cancel_intake(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<JobId>,
) -> ApiResult<Json<collector_app::IntakeSession>> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    Ok(Json(
        blocking(move || state.service.cancel_intake(&actor, id, &context)).await?,
    ))
}

#[utoipa::path(post, path = "/api/v1/ingests/plan", request_body = PlanIngestCommand, responses((status = 200, body = IngestPlan), (status = 403), (status = 409), (status = 422)))]
async fn plan_ingest(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(mut command): Json<PlanIngestCommand>,
) -> ApiResult<Json<IngestPlan>> {
    let actor = principal(&state, &headers)?;
    Ok(Json(
        blocking(move || {
            state
                .service
                .authorize(&actor, command.collection_id, Action::Curate)?;
            for package in &mut command.packages {
                if package.source.kind == ImportSourceKind::Inbox {
                    let (_, mut files) = resolve_inbox_package(&state, &package.source)?;
                    if !package.files.is_empty() {
                        if !same_upload_inventory(&files, &package.files) {
                            return Err(Error::Conflict);
                        }
                        files = package.files.clone();
                    }
                    package.files = files;
                }
            }
            state.service.plan_ingest(&actor, command)
        })
        .await?,
    ))
}

#[utoipa::path(post, path = "/api/v1/ingests", params(("Idempotency-Key" = String, Header)), request_body = StartIngestCommand, responses((status = 202, body = collector_app::AcceptedIngest), (status = 403), (status = 409), (status = 422)))]
async fn start_ingest(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(command): Json<StartIngestCommand>,
) -> ApiResult<Response> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    let accepted = blocking(move || {
        let mut inbox_paths = Vec::new();
        let accepted =
            state
                .service
                .start_ingest_with_preflight(&actor, command, &context, |plan| {
                    for package in &plan.packages {
                        if package.source.kind == ImportSourceKind::Inbox {
                            let (path, current_files) =
                                resolve_inbox_package(&state, &package.source)?;
                            if !same_upload_inventory(&current_files, &package.files) {
                                return Err(Error::Conflict);
                            }
                            inbox_paths.push((package.position, path));
                        }
                    }
                    Ok(())
                })?;
        // Best-effort dispatch uses the already validated path. Receipt replay
        // does not resolve mutable inbox configuration; the sweep closes crash gaps.
        for (position, path) in inbox_paths {
            if let (Some(import), Some(path)) =
                (accepted.imports.get(position as usize), path.to_str())
                && let Err(error) = state
                    .service
                    .ensure_guided_inbox_staging(import.import_id, path)
            {
                tracing::warn!(%error, "accepted inbox staging deferred to recovery");
            }
        }
        Ok(accepted)
    })
    .await?;
    Ok((StatusCode::ACCEPTED, Json(accepted)).into_response())
}

#[derive(Deserialize)]
struct IngestPageQuery {
    collection_id: Option<CollectionId>,
    cursor: Option<String>,
    limit: Option<u16>,
}

#[utoipa::path(get, path = "/api/v1/ingests", params(("collection_id" = Option<CollectionId>, Query), ("cursor" = Option<String>, Query), ("limit" = Option<u16>, Query)), responses((status = 200, body = Page<IngestSummary>), (status = 403), (status = 422)))]
async fn list_ingests(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<IngestPageQuery>,
) -> ApiResult<Json<Page<IngestSummary>>> {
    let actor = principal(&state, &headers)?;
    let cursor = query
        .cursor
        .map(|value| value.parse())
        .transpose()
        .map_err(|_| Error::Invalid("invalid Import cursor".into()))?;
    Ok(Json(
        blocking(move || {
            state.service.ingest_page(
                &actor,
                query.collection_id,
                cursor,
                query.limit.unwrap_or(50),
            )
        })
        .await?,
    ))
}

#[utoipa::path(get, path = "/api/v1/ingests/{id}", params(("id" = ImportId, Path)), responses((status = 200, body = IngestDetail), (status = 403), (status = 404)))]
async fn ingest_detail(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<ImportId>,
) -> ApiResult<Json<IngestDetail>> {
    let actor = principal(&state, &headers)?;
    Ok(Json(
        blocking(move || state.service.ingest_detail(&actor, id)).await?,
    ))
}

#[utoipa::path(post, path="/api/v1/ingests/{id}/inbox-cleanup", params(("id"=ImportId,Path),("Idempotency-Key"=String,Header)), request_body=collector_app::CleanupInboxCommand, responses((status=202,body=JobId),(status=403),(status=404),(status=409),(status=422)))]
async fn cleanup_ingest_inbox(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<ImportId>,
    Json(command): Json<collector_app::CleanupInboxCommand>,
) -> ApiResult<Response> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    let job = blocking(move || {
        state
            .service
            .request_inbox_cleanup(&actor, id, command, &context, |source| {
                let (path, _) = resolve_inbox_package(&state, source)?;
                path.into_os_string()
                    .into_string()
                    .map_err(|_| Error::Invalid("Inbox path must be UTF-8".into()))
            })
    })
    .await?;
    Ok((StatusCode::ACCEPTED, Json(job)).into_response())
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
struct DiscardIngestCommand {
    expected_revision: u64,
}

#[utoipa::path(post, path = "/api/v1/ingests/{id}/discard", params(("id" = ImportId, Path), ("Idempotency-Key" = String, Header)), request_body = DiscardIngestCommand, responses((status = 200, body = IngestDiscardResult), (status = 403), (status = 404), (status = 409)))]
async fn discard_ingest(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<ImportId>,
    Json(command): Json<DiscardIngestCommand>,
) -> ApiResult<Json<IngestDiscardResult>> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    Ok(Json(
        blocking(move || {
            state
                .service
                .discard_ingest(&actor, id, command.expected_revision, &context)
        })
        .await?,
    ))
}

#[utoipa::path(post, path = "/api/v1/ingests/{id}/cancel", params(("id" = ImportId, Path), ("Idempotency-Key" = String, Header)), request_body = DiscardIngestCommand, responses((status = 200, body = IngestDetail), (status = 403), (status = 404), (status = 409)))]
async fn cancel_ingest(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<ImportId>,
    Json(command): Json<DiscardIngestCommand>,
) -> ApiResult<Json<IngestDetail>> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    Ok(Json(
        blocking(move || {
            state
                .service
                .cancel_ingest(&actor, id, command.expected_revision, &context)
        })
        .await?,
    ))
}

#[utoipa::path(post, path = "/api/v1/ingests/{id}/retry", params(("id" = ImportId, Path), ("Idempotency-Key" = String, Header)), request_body = RetryIngestCommand, responses((status = 200, body = IngestDetail), (status = 403), (status = 404), (status = 409)))]
async fn retry_ingest(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<ImportId>,
    Json(command): Json<RetryIngestCommand>,
) -> ApiResult<Json<IngestDetail>> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    Ok(Json(
        blocking(move || state.service.retry_ingest(&actor, id, command, &context)).await?,
    ))
}

#[utoipa::path(put, path = "/api/v1/ingests/{id}/association", params(("id" = ImportId, Path), ("Idempotency-Key" = String, Header)), request_body = CorrectIngestAssociationCommand, responses((status = 200, body = IngestDetail), (status = 403), (status = 404), (status = 409), (status = 422)))]
async fn correct_ingest_association(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<ImportId>,
    Json(command): Json<CorrectIngestAssociationCommand>,
) -> ApiResult<Json<IngestDetail>> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    Ok(Json(
        blocking(move || {
            state
                .service
                .correct_ingest_association(&actor, id, command, &context)
        })
        .await?,
    ))
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
struct NewProvisionalImport {
    collection_id: CollectionId,
    package_name: String,
    files: Vec<UploadFile>,
}

#[utoipa::path(post, path = "/api/v1/imports", params(("Idempotency-Key" = String, Header)), request_body = NewProvisionalImport, responses((status = 200, body = ImportView), (status = 403), (status = 409)))]
async fn create_provisional_import(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<NewProvisionalImport>,
) -> ApiResult<Json<ImportView>> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    Ok(Json(
        blocking(move || {
            state.service.create_provisional_import(
                &actor,
                CreateImportCommand {
                    collection_id: input.collection_id,
                    source: ImportSourceSummary {
                        kind: ImportSourceKind::Browser,
                        label: None,
                        package_name: input.package_name,
                    },
                    files: input.files,
                    association: CaptureAssociation::default(),
                    expected_item_revision: None,
                },
                &context,
            )
        })
        .await?,
    ))
}

#[utoipa::path(get, path = "/api/v1/imports/{id}", params(("id" = ImportId, Path)), responses((status = 200, body = ImportView), (status = 404)))]
async fn provisional_import(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<ImportId>,
) -> ApiResult<Json<ImportView>> {
    let actor = principal(&state, &headers)?;
    Ok(Json(
        blocking(move || state.service.provisional_import(&actor, id)).await?,
    ))
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
struct AnalyzeProvisionalImport {
    module: ModuleId,
}

#[utoipa::path(post, path = "/api/v1/captures/{id}/operations/plan", params(("id" = CaptureId, Path)), request_body = StartProcessingCommand, responses((status = 200, body = collector_app::ScanPlan), (status = 400), (status = 403), (status = 404)))]
async fn processing_plan(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<CaptureId>,
    Json(input): Json<StartProcessingCommand>,
) -> ApiResult<Json<collector_app::ScanPlan>> {
    let actor = principal(&state, &headers)?;
    Ok(Json(
        blocking(move || state.service.processing_plan(&actor, id, input)).await?,
    ))
}

#[utoipa::path(post, path = "/api/v1/captures/{id}/operations", params(("id" = CaptureId, Path), ("Idempotency-Key" = String, Header)), request_body = StartProcessingCommand, responses((status = 202, body = serde_json::Value), (status = 400), (status = 403), (status = 404), (status = 409)))]
async fn start_processing_operation(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<CaptureId>,
    Json(input): Json<StartProcessingCommand>,
) -> ApiResult<Response> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    let job_id = blocking(move || {
        state
            .service
            .enqueue_processing_operation(&actor, id, input, &context)
    })
    .await?;
    Ok((
        StatusCode::ACCEPTED,
        Json(serde_json::json!({"job_id":job_id})),
    )
        .into_response())
}

#[utoipa::path(post, path = "/api/v1/captures/{id}/accuraterip-snapshots", params(("id" = CaptureId, Path), ("Idempotency-Key" = String, Header)), request_body = AcquireAccurateRipCommand, responses((status = 202, body = serde_json::Value), (status = 400), (status = 403), (status = 404), (status = 409)))]
async fn acquire_accuraterip_snapshot(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<CaptureId>,
    Json(input): Json<AcquireAccurateRipCommand>,
) -> ApiResult<Response> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    let job_id = blocking(move || {
        state
            .service
            .enqueue_accuraterip_acquisition(&actor, id, input, &context)
    })
    .await?;
    Ok((
        StatusCode::ACCEPTED,
        Json(serde_json::json!({"job_id":job_id})),
    )
        .into_response())
}

#[derive(Deserialize)]
struct ProcessingPageQuery {
    representation_id: RepresentationId,
    cursor: Option<String>,
    limit: Option<u16>,
}

#[utoipa::path(get, path = "/api/v1/captures/{id}/operations", params(("id" = CaptureId, Path), ("representation_id" = RepresentationId, Query), ("cursor" = Option<String>, Query), ("limit" = Option<u16>, Query)), responses((status = 200, body = Page<ProcessingSummary>), (status = 400), (status = 403), (status = 404)))]
async fn capture_processing_summaries(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<CaptureId>,
    Query(query): Query<ProcessingPageQuery>,
) -> ApiResult<Json<Page<ProcessingSummary>>> {
    let actor = principal(&state, &headers)?;
    let after = query
        .cursor
        .map(|value| value.parse())
        .transpose()
        .map_err(|_| Error::Invalid("invalid processing cursor".into()))?;
    Ok(Json(
        blocking(move || {
            state.service.capture_processing_summaries(
                &actor,
                id,
                query.representation_id,
                after,
                query.limit.unwrap_or(50),
            )
        })
        .await?,
    ))
}

#[utoipa::path(post, path = "/api/v1/imports/{id}/analysis", params(("id" = ImportId, Path), ("Idempotency-Key" = String, Header)), request_body = AnalyzeProvisionalImport, responses((status = 202, body = serde_json::Value), (status = 404), (status = 409)))]
async fn analyze_provisional_import(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<ImportId>,
    Json(input): Json<AnalyzeProvisionalImport>,
) -> ApiResult<Response> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    let job_id = blocking(move || {
        state
            .service
            .enqueue_import_analysis(&actor, id, input.module, &context)
    })
    .await?;
    Ok((
        StatusCode::ACCEPTED,
        Json(serde_json::json!({"job_id":job_id})),
    )
        .into_response())
}

#[utoipa::path(put, path = "/api/v1/imports/{id}/review", params(("id" = ImportId, Path), ("Idempotency-Key" = String, Header)), request_body = ReviewImportCommand, responses((status = 200, body = ImportView), (status = 404), (status = 409)))]
async fn review_provisional_import(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<ImportId>,
    Json(command): Json<ReviewImportCommand>,
) -> ApiResult<Json<ImportView>> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    Ok(Json(
        blocking(move || {
            state
                .service
                .review_provisional_import(&actor, id, command, &context)
        })
        .await?,
    ))
}

#[utoipa::path(post, path = "/api/v1/imports/{id}/publish", params(("id" = ImportId, Path), ("Idempotency-Key" = String, Header)), responses((status = 202, body = serde_json::Value), (status = 404), (status = 409)))]
async fn publish_provisional_import(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<ImportId>,
) -> ApiResult<Response> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    let job_id = blocking(move || {
        state
            .service
            .enqueue_import_publication(&actor, id, &context)
    })
    .await?;
    Ok((
        StatusCode::ACCEPTED,
        Json(serde_json::json!({"job_id":job_id})),
    )
        .into_response())
}

#[utoipa::path(put, path = "/api/v1/imports/{id}/files/{index}", params(("id" = ImportId, Path), ("index" = usize, Path), ("offset" = u64, Query)), request_body(content = String, content_type = "application/octet-stream"), responses((status = 200, body = ImportView), (status = 404), (status = 409), (status = 413)))]
async fn stage_provisional_import_chunk(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((id, index)): Path<(ImportId, usize)>,
    Query(query): Query<Offset>,
    body: axum::body::Bytes,
) -> ApiResult<Json<ImportView>> {
    let actor = principal(&state, &headers)?;
    Ok(Json(
        blocking(move || {
            state
                .service
                .stage_provisional_import_chunk(&actor, id, index, query.offset, &body)
        })
        .await?,
    ))
}

#[derive(serde::Serialize, utoipa::ToSchema)]
struct InboxSourceView {
    label: String,
    packages: Vec<String>,
}

#[derive(Deserialize)]
struct InboxListQuery {
    packages: Option<bool>,
}

#[derive(Deserialize)]
struct InboxPackagesQuery {
    search: Option<String>,
    cursor: Option<String>,
    limit: Option<u16>,
}

#[derive(serde::Serialize, utoipa::ToSchema)]
struct InboxPackagePage {
    items: Vec<String>,
    next_cursor: Option<String>,
}

fn paged_inbox_packages(
    root: &std::path::Path,
    query: InboxPackagesQuery,
) -> Result<InboxPackagePage> {
    let root = std::fs::canonicalize(root).map_err(|error| Error::Storage(error.to_string()))?;
    let limit = query.limit.unwrap_or(50).clamp(1, 100) as usize;
    let search = query.search.unwrap_or_default().to_lowercase();
    let mut items = Vec::with_capacity(limit + 1);
    for entry in std::fs::read_dir(root).map_err(|error| Error::Storage(error.to_string()))? {
        let entry = entry.map_err(|error| Error::Storage(error.to_string()))?;
        let metadata = std::fs::symlink_metadata(entry.path())
            .map_err(|error| Error::Storage(error.to_string()))?;
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        if !metadata.is_dir()
            || collector_app::is_intake_housekeeping(&name, true)
            || collector_app::is_inbox_cleanup_directory(&name)
            || metadata.file_type().is_symlink()
            || name.is_empty()
            || name.contains(['/', '\\'])
            || matches!(name.as_str(), "." | "..")
            || query.cursor.as_ref().is_some_and(|cursor| &name <= cursor)
            || !name.to_lowercase().contains(&search)
        {
            continue;
        }
        items.push(name);
        items.sort_unstable();
        if items.len() > limit + 1 {
            items.pop();
        }
    }
    let next_cursor = if items.len() > limit {
        items.truncate(limit);
        items.last().cloned()
    } else {
        None
    };
    Ok(InboxPackagePage { items, next_cursor })
}

fn direct_inbox_packages(root: &std::path::Path) -> Result<Vec<String>> {
    let root = std::fs::canonicalize(root).map_err(|error| Error::Storage(error.to_string()))?;
    let mut packages = Vec::new();
    for entry in std::fs::read_dir(root).map_err(|error| Error::Storage(error.to_string()))? {
        let entry = entry.map_err(|error| Error::Storage(error.to_string()))?;
        let metadata = std::fs::symlink_metadata(entry.path())
            .map_err(|error| Error::Storage(error.to_string()))?;
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        if metadata.is_dir()
            && !collector_app::is_intake_housekeeping(&name, true)
            && !collector_app::is_inbox_cleanup_directory(&name)
            && !metadata.file_type().is_symlink()
            && !name.is_empty()
            && !name.contains(['/', '\\'])
            && !matches!(name.as_str(), "." | "..")
        {
            packages.push(name);
        }
    }
    packages.sort();
    Ok(packages)
}

fn resolve_inbox_package(
    state: &AppState,
    source: &ImportSourceSummary,
) -> Result<(std::path::PathBuf, Vec<UploadFile>)> {
    resolve_inbox_package_retaining(state, source, None)
}

fn resolve_inbox_package_retaining(
    state: &AppState,
    source: &ImportSourceSummary,
    retained: Option<&[String]>,
) -> Result<(std::path::PathBuf, Vec<UploadFile>)> {
    let label = source
        .label
        .as_ref()
        .ok_or_else(|| Error::Invalid("an inbox package requires an inbox label".into()))?;
    if source.package_name.is_empty()
        || source.package_name.contains(['/', '\\'])
        || matches!(source.package_name.as_str(), "." | "..")
        || (retained.is_none()
            && (collector_app::is_intake_housekeeping(&source.package_name, true)
                || collector_app::is_inbox_cleanup_directory(&source.package_name)))
    {
        return Err(Error::Invalid(
            "select a direct inbox package directory".into(),
        ));
    }
    let root = state.inboxes.get(label).ok_or(Error::NotFound)?;
    let canonical_root =
        std::fs::canonicalize(root).map_err(|error| Error::Storage(error.to_string()))?;
    let unresolved = root.join(&source.package_name);
    let metadata = std::fs::symlink_metadata(&unresolved)
        .map_err(|error| Error::Storage(error.to_string()))?;
    let path =
        std::fs::canonicalize(&unresolved).map_err(|error| Error::Storage(error.to_string()))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() || !path.starts_with(canonical_root)
    {
        return Err(Error::Forbidden);
    }
    let files = Store::inventory_directory_retaining(&path, retained.unwrap_or_default())?;
    Ok((path, files))
}

fn recover_guided_inbox_staging(state: &AppState) -> Result<()> {
    let mut after = None;
    loop {
        let imports = state.service.resumable_guided_inbox_imports(after)?;
        if imports.is_empty() {
            break;
        }
        after = imports.last().map(|value| value.id);
        for import in imports {
            let retained = import
                .files
                .iter()
                .map(|file| file.name.clone())
                .collect::<Vec<_>>();
            let (path, current) = match resolve_inbox_package_retaining(
                state,
                &import.source,
                Some(&retained),
            ) {
                Ok(value) => value,
                Err(error) => {
                    state.service.report_guided_staging_failure(import.id, "Configured inbox package is unavailable; restore the source or discard this import.")?;
                    tracing::warn!(import_id=%import.id, %error, "guided inbox source is unavailable during recovery");
                    continue;
                }
            };
            let by_name = current
                .iter()
                .map(|file| (file.name.as_str(), file))
                .collect::<std::collections::BTreeMap<_, _>>();
            if current.len() != import.files.len()
                || import.files.iter().any(|declared| {
                    by_name.get(declared.name.as_str()).is_none_or(|source| {
                        source.bytes != declared.bytes
                            || source
                                .sha256
                                .as_ref()
                                .zip(declared.digest.as_ref())
                                .is_some_and(|(source, declared)| source != &declared.hex)
                    })
                })
            {
                state.service.report_guided_staging_failure(import.id, "Inbox inventory changed after acceptance; restore the original package or discard and preview it again.")?;
                tracing::warn!(import_id=%import.id, "guided inbox inventory changed before recovery");
                continue;
            }
            let source = path
                .to_str()
                .ok_or_else(|| Error::Invalid("inbox source path must be UTF-8".into()))?;
            if let Err(error) = state.service.ensure_guided_inbox_staging(import.id, source) {
                tracing::warn!(import_id=%import.id, %error, "guided inbox staging recovery was deferred");
            }
        }
    }
    Ok(())
}

#[utoipa::path(get, path = "/api/v1/collections/{id}/inboxes", params(("id" = CollectionId, Path), ("packages" = Option<bool>, Query)), responses((status = 200, body = Vec<InboxSourceView>), (status = 403)))]
async fn collection_inboxes(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<CollectionId>,
    Query(query): Query<InboxListQuery>,
) -> ApiResult<Json<Vec<InboxSourceView>>> {
    let actor = principal(&state, &headers)?;
    state.service.authorize(&actor, id, Action::Curate)?;
    let inboxes = state.inboxes.clone();
    Ok(Json(
        blocking(move || {
            inboxes
                .iter()
                .map(|(label, root)| {
                    Ok(InboxSourceView {
                        label: label.clone(),
                        packages: if query.packages == Some(false) {
                            Vec::new()
                        } else {
                            direct_inbox_packages(root)?
                        },
                    })
                })
                .collect::<Result<Vec<_>>>()
        })
        .await?,
    ))
}

#[utoipa::path(get, path = "/api/v1/collections/{id}/inboxes/{label}/packages", params(("id" = CollectionId, Path), ("label" = String, Path), ("search" = Option<String>, Query), ("cursor" = Option<String>, Query), ("limit" = Option<u16>, Query)), responses((status = 200, body = InboxPackagePage), (status = 403), (status = 404)))]
async fn inbox_packages(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((id, label)): Path<(CollectionId, String)>,
    Query(query): Query<InboxPackagesQuery>,
) -> ApiResult<Json<InboxPackagePage>> {
    let actor = principal(&state, &headers)?;
    state.service.authorize(&actor, id, Action::Curate)?;
    let root = state.inboxes.get(&label).ok_or(Error::NotFound)?.clone();
    Ok(Json(
        blocking(move || paged_inbox_packages(&root, query)).await?,
    ))
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
struct StageInboxImport {
    package: String,
}

#[derive(serde::Serialize, utoipa::ToSchema)]
struct StagedInboxImport {
    import: ImportView,
    job_id: JobId,
}

#[utoipa::path(post, path = "/api/v1/collections/{collection}/inboxes/{label}/stage", params(("collection" = CollectionId, Path), ("label" = String, Path), ("Idempotency-Key" = String, Header)), request_body = StageInboxImport, responses((status = 202, body = StagedInboxImport), (status = 403), (status = 409)))]
async fn stage_inbox_import(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((collection, label)): Path<(CollectionId, String)>,
    Json(input): Json<StageInboxImport>,
) -> ApiResult<Response> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    state
        .service
        .authorize(&actor, collection, Action::Curate)?;
    if input.package.is_empty()
        || input.package.contains(['/', '\\'])
        || matches!(input.package.as_str(), "." | "..")
        || collector_app::is_intake_housekeeping(&input.package, true)
    {
        return Err(Error::Invalid("select a direct inbox package directory".into()).into());
    }
    let root = state.inboxes.get(&label).ok_or(Error::NotFound)?.clone();
    let source = root.join(&input.package);
    let package = input.package;
    let (source, files) = blocking(move || {
        let root = std::fs::canonicalize(root).map_err(|e| Error::Storage(e.to_string()))?;
        let metadata =
            std::fs::symlink_metadata(&source).map_err(|e| Error::Storage(e.to_string()))?;
        let path = std::fs::canonicalize(&source).map_err(|e| Error::Storage(e.to_string()))?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() || !path.starts_with(root) {
            return Err(Error::Forbidden);
        }
        let files = Store::inventory_directory(&path)?;
        Ok((path, files))
    })
    .await?;
    let import = state.service.create_provisional_import(
        &actor,
        CreateImportCommand {
            collection_id: collection,
            source: ImportSourceSummary {
                kind: ImportSourceKind::Inbox,
                label: Some(label),
                package_name: package,
            },
            files,
            association: CaptureAssociation::default(),
            expected_item_revision: None,
        },
        &context,
    )?;
    let source_locator = source
        .to_str()
        .ok_or_else(|| Error::Invalid("inbox source path must be UTF-8".into()))?;
    let job_id =
        state
            .service
            .enqueue_inbox_import_staging(&actor, import.id, source_locator, &context)?;
    Ok((
        StatusCode::ACCEPTED,
        Json(StagedInboxImport { import, job_id }),
    )
        .into_response())
}

fn compatibility_upload(user_id: UserId, import: ImportView) -> Result<Upload> {
    let item_id = import
        .association
        .item_id
        .ok_or_else(|| Error::Storage("legacy upload Import lost its Item association".into()))?;
    let expected_revision = import
        .expected_item_revision
        .ok_or_else(|| Error::Storage("legacy upload Import lost its Item revision".into()))?;
    let state = match import.phase {
        collector_app::ImportPhase::Staging
            if import.files.iter().all(|file| file.received_bytes.0 == 0) =>
        {
            JobState::Queued
        }
        collector_app::ImportPhase::Staging
        | collector_app::ImportPhase::Analyzing
        | collector_app::ImportPhase::AwaitingReview
        | collector_app::ImportPhase::Publishing => JobState::Running,
        collector_app::ImportPhase::Published => JobState::Completed,
        collector_app::ImportPhase::Cancelled => JobState::Cancelled,
    };
    Ok(Upload {
        id: JobId(import.id.0),
        user_id,
        item_id,
        expected_revision,
        representation_id: import.representation_id,
        capture_id: import.capture_id,
        offsets: import
            .files
            .iter()
            .map(|file| file.received_bytes.0)
            .collect(),
        files: import
            .files
            .into_iter()
            .map(|file| UploadFile {
                visibility: Default::default(),
                name: file.name,
                bytes: file.bytes,
                sha256: file.digest.map(|digest| digest.hex),
            })
            .collect(),
        state,
    })
}

#[utoipa::path(post, path = "/api/v1/uploads", params(("Idempotency-Key" = String, Header)), request_body = NewUpload, responses((status = 200, body = Upload), (status = 409)))]
async fn create_upload(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<NewUpload>,
) -> ApiResult<Json<collector_store::Upload>> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    let user_id = actor.user_id;
    Ok(Json(compatibility_upload(
        user_id,
        blocking(move || {
            state.service.create_provisional_import(
                &actor,
                CreateImportCommand {
                    collection_id: state
                        .service
                        .item_presented(&actor, input.item_id)?
                        .collection_id,
                    source: ImportSourceSummary {
                        kind: ImportSourceKind::Browser,
                        label: None,
                        package_name: "legacy-browser-upload".into(),
                    },
                    files: input.files,
                    association: CaptureAssociation {
                        item_id: Some(input.item_id),
                        ..Default::default()
                    },
                    expected_item_revision: Some(input.expected_revision),
                },
                &context,
            )
        })
        .await?,
    )?))
}
#[utoipa::path(get, path = "/api/v1/uploads/{id}", params(("id" = JobId, Path)), responses((status = 200, body = Upload), (status = 403), (status = 404)))]
async fn upload(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<JobId>,
) -> ApiResult<Json<collector_store::Upload>> {
    let actor = principal(&state, &headers)?;
    let user_id = actor.user_id;
    Ok(Json(
        blocking(
            move || match state.service.provisional_import(&actor, ImportId(id.0)) {
                Ok(import) => compatibility_upload(user_id, import),
                Err(Error::NotFound) => state.service.upload(&actor, id),
                Err(error) => Err(error),
            },
        )
        .await?,
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
    let user_id = actor.user_id;
    Ok(Json(
        blocking(move || {
            match state.service.stage_provisional_import_chunk(
                &actor,
                ImportId(id.0),
                index,
                query.offset,
                &body,
            ) {
                Ok(import) => compatibility_upload(user_id, import),
                Err(Error::NotFound) => {
                    state
                        .service
                        .upload_chunk(&actor, id, index, query.offset, &body)
                }
                Err(error) => Err(error),
            }
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
    blocking(move || {
        match state
            .service
            .cancel_provisional_import(&actor, ImportId(id.0), &context)
        {
            Ok(_) => Ok(()),
            Err(Error::NotFound) => state.service.cancel_upload(&actor, id, &context),
            Err(error) => Err(error),
        }
    })
    .await?;
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
    let import_id = ImportId(id.0);
    let mut import = match state.service.provisional_import(&actor, import_id) {
        Ok(import) => import,
        Err(Error::NotFound) => {
            let job_id =
                blocking(move || state.service.enqueue_upload_commit(&actor, id, &context)).await?;
            return Ok((
                StatusCode::ACCEPTED,
                Json(serde_json::json!({"job_id":job_id})),
            )
                .into_response());
        }
        Err(error) => return Err(error.into()),
    };
    if import.phase == collector_app::ImportPhase::Analyzing {
        import = state.service.review_provisional_import(
            &actor,
            import_id,
            ReviewImportCommand {
                expected_revision: import.revision,
                identity: collector_app::ImportIdentityDecision::Unidentified,
                association: import.association,
                expected_item_revision: import.expected_item_revision,
            },
            &context,
        )?;
    }
    let _ = import;
    let job_id = blocking(move || {
        state
            .service
            .enqueue_import_publication(&actor, import_id, &context)
    })
    .await?;
    Ok((
        StatusCode::ACCEPTED,
        Json(serde_json::json!({"job_id":job_id})),
    )
        .into_response())
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
#[derive(Deserialize, utoipa::IntoParams)]
struct JobQuery {
    cursor: Option<JobId>,
    limit: Option<u16>,
}

#[utoipa::path(get, path = "/api/v1/jobs", params(JobQuery), responses((status = 200, body = Page<JobView>), (status = 401)))]
async fn jobs(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<JobQuery>,
) -> ApiResult<Json<Page<JobView>>> {
    let actor = principal(&state, &headers)?;
    Ok(Json(
        blocking(move || state.service.job_page(&actor, query.cursor, query.limit)).await?,
    ))
}

#[utoipa::path(get, path = "/api/v1/jobs/{id}", params(("id" = JobId, Path)), responses((status = 200, body = JobView), (status = 401), (status = 404)))]
async fn job(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<JobId>,
) -> ApiResult<Json<JobView>> {
    let actor = principal(&state, &headers)?;
    Ok(Json(blocking(move || state.service.job(&actor, id)).await?))
}

#[utoipa::path(post, path = "/api/v1/jobs/{id}/cancel", params(("id" = JobId, Path), ("Idempotency-Key" = String, Header)), responses((status = 200, body = JobView), (status = 401), (status = 404), (status = 409)))]
async fn cancel_job(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<JobId>,
) -> ApiResult<Json<JobView>> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    blocking({
        let state = state.clone();
        let actor = actor.clone();
        move || state.service.cancel_job(&actor, id, &context)
    })
    .await?;
    Ok(Json(blocking(move || state.service.job(&actor, id)).await?))
}

#[utoipa::path(post, path = "/api/v1/jobs/{id}/retry", params(("id" = JobId, Path), ("Idempotency-Key" = String, Header)), responses((status = 202, body = JobView), (status = 401), (status = 404), (status = 409)))]
async fn retry_job(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<JobId>,
) -> ApiResult<Response> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    blocking({
        let state = state.clone();
        let actor = actor.clone();
        move || state.service.retry_job(&actor, id, &context)
    })
    .await?;
    let view = blocking(move || state.service.job(&actor, id)).await?;
    Ok((StatusCode::ACCEPTED, Json(view)).into_response())
}
#[derive(serde::Serialize, utoipa::ToSchema)]
struct ServerSettingsView {
    default_user_id: UserId,
    current_user_id: UserId,
    exposure: Exposure,
    multi_user: bool,
    server_id: ServerId,
    ingress: String,
    backup_reminder: Option<collector_app::BackupReminder>,
}
#[utoipa::path(get, path = "/api/v1/admin/settings", responses((status = 200, body = ServerSettingsView), (status = 403)))]
async fn settings(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<ServerSettingsView>> {
    let actor = principal(&state, &headers)?;
    let default_user_id = state.service.default_user_id(&actor)?;
    let reminder = if let Some(settings) = &state.backup_reminder {
        let settings = settings.clone();
        let service = state.service.clone();
        let actor = actor.clone();
        Some(blocking(move || service.backup_reminder(&actor, &settings)).await?)
    } else {
        None
    };

    Ok(Json(ServerSettingsView {
        default_user_id,
        current_user_id: actor.user_id,
        exposure: state.exposure,
        multi_user: state.multi_user,
        server_id: state.pairing.server_id,
        ingress: format!("{:?}", state.ingress),
        backup_reminder: reminder,
    }))
}

#[utoipa::path(get, path = "/api/v1/admin/catalog-sources", responses((status = 200, body = Vec<CatalogSourceView>), (status = 403)))]
async fn catalog_sources(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<CatalogSourceView>>> {
    let actor = principal(&state, &headers)?;
    state.service.require_administrator(&actor)?;
    Ok(Json(state.catalog_sources.as_ref().clone()))
}

#[derive(Deserialize)]
struct SnapshotPageQuery {
    cursor: Option<String>,
    limit: Option<u16>,
}

#[utoipa::path(get, path = "/api/v1/collections/{id}/catalog-snapshots", params(("id" = CollectionId, Path), ("cursor" = Option<String>, Query), ("limit" = Option<u16>, Query)), responses((status = 200, body = Page<CatalogSnapshotStatus>), (status = 400), (status = 403)))]
async fn curation_catalog_snapshots(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<CollectionId>,
    Query(query): Query<SnapshotPageQuery>,
) -> ApiResult<Json<Page<CatalogSnapshotStatus>>> {
    let actor = principal(&state, &headers)?;
    let after = query
        .cursor
        .map(|value| value.parse())
        .transpose()
        .map_err(|_| Error::Invalid("invalid snapshot cursor".into()))?;
    Ok(Json(
        blocking(move || {
            state
                .service
                .curation_catalog_snapshots(&actor, id, after, query.limit.unwrap_or(50))
        })
        .await?,
    ))
}

#[utoipa::path(get, path = "/api/v1/admin/collections/{id}/catalog-snapshots", params(("id" = CollectionId, Path), ("cursor" = Option<String>, Query), ("limit" = Option<u16>, Query)), responses((status = 200, body = Page<CatalogSnapshotStatus>), (status = 400), (status = 403)))]
async fn catalog_snapshots(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<CollectionId>,
    Query(query): Query<SnapshotPageQuery>,
) -> ApiResult<Json<Page<CatalogSnapshotStatus>>> {
    let actor = principal(&state, &headers)?;
    let after = query
        .cursor
        .map(|value| value.parse())
        .transpose()
        .map_err(|_| Error::Invalid("invalid snapshot cursor".into()))?;
    Ok(Json(
        blocking(move || {
            state
                .service
                .catalog_snapshots(&actor, id, after, query.limit.unwrap_or(50))
        })
        .await?,
    ))
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
struct CatalogImportRequest {
    source: String,
}

#[utoipa::path(post, path = "/api/v1/admin/collections/{id}/catalog-snapshots", params(("id" = CollectionId, Path), ("Idempotency-Key" = String, Header)), request_body = CatalogImportRequest, responses((status = 202, body = serde_json::Value), (status = 400), (status = 403), (status = 409)))]
async fn import_catalog_snapshot(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<CollectionId>,
    Json(input): Json<CatalogImportRequest>,
) -> ApiResult<Response> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    let job_id = blocking(move || {
        state.service.import_catalog_snapshot(
            &actor,
            ImportCatalogSnapshotCommand {
                collection_id: id,
                provider: ModuleId(collector_providers::LOCAL_DAT_PROVIDER_ID.into()),
                source: input.source,
            },
            &context,
        )
    })
    .await?;
    Ok((
        StatusCode::ACCEPTED,
        Json(serde_json::json!({"job_id":job_id})),
    )
        .into_response())
}

#[utoipa::path(post, path = "/api/v1/admin/collections/{id}/catalog-upload", params(("id" = CollectionId, Path), ("Idempotency-Key" = String, Header), collector_app::DatUploadMetadata), request_body(content = String, content_type = "application/octet-stream"), responses((status = 202, body = serde_json::Value), (status = 400), (status = 403), (status = 409), (status = 413)))]
async fn upload_catalog_snapshot(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<CollectionId>,
    Query(metadata): Query<collector_app::DatUploadMetadata>,
    request: Request,
) -> ApiResult<Response> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    state.service.validate_dat_upload(&actor, id, &metadata)?;
    let _slot = state
        .catalog_upload_slots
        .clone()
        .try_acquire_owned()
        .map_err(|_| {
            Error::Unavailable("Other DAT uploads are in progress; try again shortly.".into())
        })?;
    let bytes = match axum::body::to_bytes(request.into_body(), collector_app::MAX_DAT_UPLOAD_BYTES).await {
        Ok(bytes) => bytes,
        Err(_) => return Ok((StatusCode::PAYLOAD_TOO_LARGE, Json(serde_json::json!({"error":{"code":"invalid","message":"Upload an extracted DAT file no larger than 64 MiB."}}))).into_response()),
    };
    let job_id = blocking(move || {
        state
            .service
            .upload_dat_catalog(&actor, id, metadata, &bytes, &context)
    })
    .await?;
    Ok((
        StatusCode::ACCEPTED,
        Json(serde_json::json!({"job_id":job_id})),
    )
        .into_response())
}

#[utoipa::path(post, path = "/api/v1/items/{id}/identify-game", params(("id" = ItemId, Path), ("Idempotency-Key" = String, Header)), responses((status = 202, body = Vec<JobId>), (status = 403), (status = 409)))]
async fn identify_game_entry(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<ItemId>,
) -> ApiResult<(StatusCode, Json<Vec<JobId>>)> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    Ok((
        StatusCode::ACCEPTED,
        Json(blocking(move || state.service.identify_game_entry(&actor, id, &context)).await?),
    ))
}

/// Browser batch of binary DAT/ZIP uploads; parsing uses bounded multipart chunks.
#[derive(utoipa::ToSchema)]
#[allow(dead_code)]
struct CatalogUploadForm {
    #[schema(value_type=Vec<String>, format=Binary)]
    files: Vec<String>,
}

#[utoipa::path(post, path = "/api/v1/admin/collections/{id}/catalog-upload/inspect", params(("id" = CollectionId, Path)), request_body(content = CatalogUploadForm, content_type = "multipart/form-data"), responses((status = 200, body = collector_app::CatalogUploadPreview), (status = 400), (status = 403), (status = 413)))]
async fn inspect_catalog_upload(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<CollectionId>,
    request: Request,
) -> ApiResult<Json<collector_app::CatalogUploadPreview>> {
    let actor = principal(&state, &headers)?;
    let service = state.service.clone();
    let authorized = actor.clone();
    blocking(move || service.authorize_game_catalog_upload(&authorized, id)).await?;
    let _slot = state
        .catalog_upload_slots
        .clone()
        .try_acquire_owned()
        .map_err(|_| Error::Unavailable("Other catalog uploads are in progress".into()))?;
    let multipart = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.starts_with("multipart/form-data"));
    let mut files = vec![];
    if multipart {
        let mut form = Multipart::from_request(request, &state)
            .await
            .map_err(|_| Error::Invalid("Invalid catalog batch".into()))?;
        let mut total = 0usize;
        while let Some(mut field) = form
            .next_field()
            .await
            .map_err(|_| Error::Invalid("Invalid catalog batch field".into()))?
        {
            if files.len() >= 128 {
                return Err(Error::Invalid("Batch exceeds 128 uploaded files".into()).into());
            }
            let name = field.file_name().unwrap_or("Uploaded DAT").to_owned();
            let mut data = Vec::new();
            while let Some(chunk) = field
                .chunk()
                .await
                .map_err(|_| Error::Invalid("Catalog upload read failed".into()))?
            {
                total = total.checked_add(chunk.len()).ok_or(Error::Conflict)?;
                if data.len() + chunk.len() > collector_app::MAX_DAT_UPLOAD_BYTES
                    || total > 512 * 1024 * 1024
                {
                    return Err(Error::Invalid("Upload file or batch limit exceeded".into()).into());
                }
                data.extend_from_slice(&chunk);
            }
            files.push((name, data));
        }
    } else {
        let bytes = axum::body::to_bytes(request.into_body(), collector_app::MAX_DAT_UPLOAD_BYTES)
            .await
            .map_err(|_| Error::Invalid("Upload files up to 64 MiB each".into()))?;
        files.push(("Uploaded DAT".into(), bytes.to_vec()));
    }
    Ok(Json(
        blocking(move || {
            let _slot = _slot;
            state.service.inspect_catalog_batch(&actor, id, files)
        })
        .await?,
    ))
}
#[utoipa::path(post, path = "/api/v1/admin/collections/{id}/catalog-upload/accept", params(("id" = CollectionId, Path), ("Idempotency-Key" = String, Header)), request_body = collector_app::UploadedDatCatalog, responses((status = 202, body = serde_json::Value), (status = 400), (status = 403)))]
async fn accept_catalog_upload(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<CollectionId>,
    Json(upload): Json<collector_app::UploadedDatCatalog>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    let job = blocking(move || {
        state
            .service
            .import_uploaded_catalog(&actor, id, upload, &context)
    })
    .await?;
    Ok((
        StatusCode::ACCEPTED,
        Json(serde_json::json!({"job_id":job})),
    ))
}

#[utoipa::path(put, path = "/api/v1/admin/collections/{id}/catalog-snapshots/{snapshot_id}/active", params(("id" = CollectionId, Path), ("snapshot_id" = SnapshotId, Path), ("Idempotency-Key" = String, Header)), request_body = ActivateCatalogSnapshotCommand, responses((status = 200, body = CatalogSnapshotStatus), (status = 400), (status = 403), (status = 404), (status = 409)))]
async fn activate_catalog_snapshot(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((id, snapshot_id)): Path<(CollectionId, SnapshotId)>,
    Json(input): Json<ActivateCatalogSnapshotCommand>,
) -> ApiResult<Json<CatalogSnapshotStatus>> {
    if input.snapshot_id != snapshot_id {
        return Err(Error::Invalid("snapshot path and body identities differ".into()).into());
    }
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    Ok(Json(
        blocking(move || {
            state
                .service
                .activate_catalog_snapshot(&actor, id, snapshot_id, &context)
        })
        .await?,
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
#[derive(Deserialize, utoipa::IntoParams)]
struct AccountsQuery {
    search: Option<String>,
    cursor: Option<String>,
    limit: Option<u16>,
}

#[utoipa::path(get, path = "/api/v1/admin/users", params(AccountsQuery), responses((status = 200, body = Page<AccountView>), (status = 401), (status = 403)))]
async fn list_users(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<AccountsQuery>,
) -> ApiResult<Json<Page<AccountView>>> {
    let actor = authenticated(&state, &headers)?;
    Ok(Json(state.service.accounts(
        &actor,
        query.search.as_deref().unwrap_or(""),
        query.cursor.as_deref(),
        query.limit.unwrap_or(50),
    )?))
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
#[utoipa::path(post, path = "/api/v1/admin/collections", params(("Idempotency-Key" = String, Header)), request_body = CreateCollectionCommand, responses((status = 200, body = Collection), (status = 403), (status = 409)))]
async fn create_collection(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<CreateCollectionCommand>,
) -> ApiResult<Json<Collection>> {
    let actor = authenticated(&state, &headers)?;
    let context = command_context(&headers)?;
    Ok(Json(
        blocking(move || state.service.create_collection(&actor, input, &context)).await?,
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

#[derive(Deserialize, utoipa::IntoParams)]
struct GrantsQuery {
    search: Option<String>,
    cursor: Option<String>,
    limit: Option<u16>,
}

#[utoipa::path(get, path = "/api/v1/collections/{id}/grants", params(("id" = CollectionId, Path), GrantsQuery), responses((status = 200, body = Page<GrantView>), (status = 401), (status = 403)))]
async fn collection_grants(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<CollectionId>,
    Query(query): Query<GrantsQuery>,
) -> ApiResult<Json<Page<GrantView>>> {
    let actor = principal(&state, &headers)?;
    Ok(Json(state.service.grants(
        &actor,
        id,
        query.search.as_deref().unwrap_or(""),
        query.cursor.as_deref(),
        query.limit.unwrap_or(50),
    )?))
}

#[derive(Deserialize, utoipa::ToSchema)]
struct GrantChange {
    expected_revision: Revision,
    role: Role,
}

#[utoipa::path(get, path = "/api/v1/collections/{id}/grants/{username}", params(("id" = CollectionId, Path), ("username" = String, Path)), responses((status = 200, body = Option<GrantView>), (status = 401), (status = 403), (status = 404)))]
async fn get_collection_grant(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((id, username)): Path<(CollectionId, String)>,
) -> ApiResult<Json<Option<GrantView>>> {
    let actor = principal(&state, &headers)?;
    Ok(Json(
        state.service.grant_for_username(&actor, id, &username)?,
    ))
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
        || collector_app::is_intake_housekeeping(&input.package, true)
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
    let collection_id = state
        .service
        .item_presented(&actor, input.item_id)?
        .collection_id;
    let import = state.service.create_provisional_import(
        &actor,
        CreateImportCommand {
            collection_id,
            source: ImportSourceSummary {
                kind: ImportSourceKind::Inbox,
                label: Some(name),
                package_name: input.package,
            },
            files,
            association: CaptureAssociation {
                item_id: Some(input.item_id),
                ..Default::default()
            },
            expected_item_revision: Some(input.expected_revision),
        },
        &context,
    )?;
    let source_locator = source
        .to_str()
        .ok_or_else(|| Error::Invalid("inbox source path must be UTF-8".into()))?;
    let job_id =
        state
            .service
            .enqueue_inbox_import_staging(&actor, import.id, source_locator, &context)?;
    Ok((
        StatusCode::ACCEPTED,
        Json(serde_json::json!({"import_id":import.id,"job_id":job_id})),
    )
        .into_response())
}

#[utoipa::path(get,path="/api/v1/work",params(WorkQuery),responses((status=200,body=Page<WorkEntry>),(status=403),(status=422)))]
async fn work_history(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<WorkQuery>,
) -> ApiResult<Json<Page<WorkEntry>>> {
    let actor = principal(&state, &headers)?;
    Ok(Json(
        blocking(move || state.service.work_history(&actor, query)).await?,
    ))
}
#[utoipa::path(post,path="/api/v1/work/entry",request_body=WorkTarget,responses((status=200,body=WorkEntry),(status=403),(status=404)))]
async fn work_history_entry(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(target): Json<WorkTarget>,
) -> ApiResult<Json<WorkEntry>> {
    let actor = principal(&state, &headers)?;
    Ok(Json(
        blocking(move || state.service.work_history_entry(&actor, &target)).await?,
    ))
}
#[utoipa::path(post,path="/api/v1/work",request_body=WorkHistoryCommand,params(("Idempotency-Key"=String,Header)),responses((status=204),(status=403),(status=409),(status=422)))]
async fn change_work_history(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(command): Json<WorkHistoryCommand>,
) -> ApiResult<StatusCode> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    blocking(move || state.service.change_work_history(&actor, command, &context)).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn work_history_js() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
        include_str!("../../../web/dist/work-history.js"),
    )
}

#[utoipa::path(post,path="/api/v1/work/jobs",request_body=WorkJobsQuery,responses((status=200,body=Page<WorkJobView>),(status=403),(status=404)))]
async fn work_jobs(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(query): Json<WorkJobsQuery>,
) -> ApiResult<Json<Page<WorkJobView>>> {
    let actor = principal(&state, &headers)?;
    Ok(Json(
        blocking(move || state.service.work_jobs(&actor, query)).await?,
    ))
}

#[utoipa::path(get,path="/api/v1/collections/{collection}/lists",params(("collection"=CollectionId,Path),("search"=Option<String>,Query),("cursor"=Option<CollectionListId>,Query),("limit"=Option<u16>,Query)),responses((status=200,body=Page<collector_app::CollectionListSummary>),(status=403)))]
async fn collection_lists(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(collection): Path<CollectionId>,
    Query(query): Query<collector_app::CollectionListsQuery>,
) -> ApiResult<Json<Page<collector_app::CollectionListSummary>>> {
    let actor = principal(&state, &headers)?;
    Ok(Json(
        blocking(move || state.service.collection_lists(&actor, collection, query)).await?,
    ))
}
#[utoipa::path(get,path="/api/v1/collections/{collection}/lists/{list}",params(("collection"=CollectionId,Path),("list"=CollectionListId,Path),("cursor"=Option<String>,Query),("limit"=Option<u16>,Query)),responses((status=200,body=collector_app::CollectionListView),(status=403),(status=404),(status=409)))]
async fn collection_list_view(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((collection, list)): Path<(CollectionId, CollectionListId)>,
    Query(query): Query<collector_app::ListEntriesQuery>,
) -> ApiResult<Json<collector_app::CollectionListView>> {
    let actor = principal(&state, &headers)?;
    Ok(Json(
        blocking(move || {
            state
                .service
                .collection_list_view(&actor, collection, list, query)
        })
        .await?,
    ))
}
#[utoipa::path(post,path="/api/v1/collections/{collection}/lists",params(("collection"=CollectionId,Path),("Idempotency-Key"=String,Header)),request_body=collector_app::CreateCollectionList,responses((status=200,body=collector_app::CollectionListSummary),(status=400),(status=403),(status=409)))]
async fn create_collection_list(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(collection): Path<CollectionId>,
    Json(command): Json<collector_app::CreateCollectionList>,
) -> ApiResult<Json<collector_app::CollectionListSummary>> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    Ok(Json(
        blocking(move || {
            state
                .service
                .create_collection_list(&actor, collection, command, &context)
        })
        .await?,
    ))
}
#[utoipa::path(put,path="/api/v1/collections/{collection}/lists/{list}",params(("collection"=CollectionId,Path),("list"=CollectionListId,Path),("Idempotency-Key"=String,Header)),request_body=collector_app::UpdateCollectionList,responses((status=200,body=collector_app::CollectionListSummary),(status=400),(status=403),(status=404),(status=409)))]
async fn update_collection_list(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((collection, list)): Path<(CollectionId, CollectionListId)>,
    Json(command): Json<collector_app::UpdateCollectionList>,
) -> ApiResult<Json<collector_app::CollectionListSummary>> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    Ok(Json(
        blocking(move || {
            state
                .service
                .update_collection_list(&actor, collection, list, command, &context)
        })
        .await?,
    ))
}

#[utoipa::path(put, path = "/api/v1/items/{item}/captures/{capture}/purchase", params(("item" = ItemId, Path), ("capture" = CaptureId, Path), ("Idempotency-Key" = String, Header)), request_body = collector_app::UpdateDigitalPurchase, responses((status = 200, body = collector_app::PresentedCapture), (status = 400), (status = 403), (status = 404), (status = 409)))]
async fn update_digital_purchase(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((item, capture)): Path<(ItemId, CaptureId)>,
    Json(command): Json<collector_app::UpdateDigitalPurchase>,
) -> ApiResult<Json<collector_app::PresentedCapture>> {
    let actor = principal(&state, &headers)?;
    let context = command_context(&headers)?;
    Ok(Json(
        blocking(move || {
            state
                .service
                .update_digital_purchase(&actor, item, capture, command, &context)
        })
        .await?,
    ))
}

fn same_upload_inventory(
    actual: &[collector_app::UploadFile],
    reviewed: &[collector_app::UploadFile],
) -> bool {
    let by_name = actual
        .iter()
        .map(|file| (file.name.as_str(), file))
        .collect::<std::collections::BTreeMap<_, _>>();
    actual.len() == reviewed.len()
        && reviewed.iter().all(|file| {
            by_name
                .get(file.name.as_str())
                .is_some_and(|actual| actual.bytes == file.bytes && actual.sha256 == file.sha256)
        })
}
