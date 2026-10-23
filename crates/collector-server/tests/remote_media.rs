//! Real core store + socket authority + restricted delivery, with independent audio bytes.
use axum::response::IntoResponse;
use axum::{body::Body, http::Request};
use collector_api_types::{
    MediaAuthorizeRequest, MediaHandleRequest, MediaReadTicket, PlaybackTarget, RemoteGrantRequest,
    RemoteMediaGrant,
};
use collector_app::*;
use collector_core::*;
use collector_server::{AppState, remote_media};
use collector_store::{Clock, Store, UploadFile};
use std::{
    sync::{
        Arc,
        atomic::{AtomicI64, Ordering},
    },
    time::Duration,
};
use tower::ServiceExt;

struct Time(AtomicI64);
impl Clock for Time {
    fn now(&self) -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::from_timestamp(self.0.load(Ordering::SeqCst), 0).unwrap()
    }
    fn monotonic_millis(&self) -> u64 {
        self.0.load(Ordering::SeqCst) as u64 * 1000
    }
}
fn command() -> CommandContext {
    CommandContext::new(TransactionId::new().to_string()).unwrap()
}
fn nonce() -> String {
    format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
}
struct Task(tokio::task::JoinHandle<()>);
impl Drop for Task {
    fn drop(&mut self) {
        self.0.abort();
    }
}

#[tokio::test]
async fn isolated_media_enforces_grants_exact_bytes_and_socket_revocation() {
    let root = tempfile::Builder::new()
        .prefix("jc-remote-")
        .tempdir_in("/tmp")
        .unwrap();
    let clock = Arc::new(Time(AtomicI64::new(chrono::Utc::now().timestamp())));
    let store = Arc::new(
        Store::open_with_clock(
            root.path().join("archive"),
            root.path().join("state"),
            clock.clone(),
        )
        .unwrap(),
    );
    let user = store
        .create_user("owner", "remote test password", true)
        .unwrap();
    let (session, actor) = store
        .login("owner", "remote test password", "fixture")
        .unwrap();
    let mut state = AppState::new(
        store.clone(),
        collector_client::Pairing {
            server_id: store.server_id().unwrap(),
            certificate_pem: String::new(),
            certificate_sha256: String::new(),
            endpoints: vec![],
        },
    );
    let origin = "https://control.example";
    let audience = "https://media.example/media/";
    state.allowed_origins = Arc::new(vec![origin.into()]);
    let config = remote_media::Configuration {
        base_url: audience.into(),
        socket: root.path().join("auth/socket"),
    };
    state.remote_media = Some(Arc::new(config.clone()));
    let collection = state
        .service
        .create_collection(
            &actor,
            CreateCollectionCommand {
                name: "Audio".into(),
                module: ModuleId("org.junk.music".into()),
            },
            &command(),
        )
        .unwrap();
    let audio = include_bytes!("../../collector-store/tests/fixtures/digital-audio/01.flac");
    let plan = state
        .service
        .plan_ingest(
            &actor.principal,
            PlanIngestCommand {
                collection_id: collection.id,
                packages: vec![IngestPackageSelection {
                    purchase: Some(DigitalPurchase::default()),
                    source: ImportSourceSummary {
                        kind: ImportSourceKind::Browser,
                        label: None,
                        package_name: "Audio".into(),
                    },
                    anchor_file_index: None,
                    files: vec![UploadFile {
                        name: "01.flac".into(),
                        bytes: ByteLength(audio.len() as u64),
                        sha256: None,
                        visibility: Some(Visibility::Curators),
                    }],
                }],
            },
        )
        .unwrap();
    let accepted = state
        .service
        .start_ingest(
            &actor.principal,
            StartIngestCommand {
                purchase: None,
                clean_reimport: None,
                plan,
                analysis: IngestAnalysisPolicy::Automatic,
                enrichment: IngestEnrichmentPolicy::Automatic,
                prepare_music: false,
                destination: None,
            },
            &command(),
        )
        .unwrap();
    let import = accepted.imports[0].import_id;
    state
        .service
        .stage_provisional_import_chunk(&actor.principal, import, 0, 0, audio)
        .unwrap();
    for _ in 0..32 {
        if !state.service.run_next_job("remote-fixture").unwrap() {
            break;
        }
    }
    state.service.resume_guided_ingests().unwrap();
    let item = state
        .service
        .ingest_detail(&actor.principal, import)
        .unwrap()
        .summary
        .target_item_id
        .unwrap();
    let tracks = state
        .service
        .recorded_music_track_availability(&actor.principal, item)
        .unwrap();
    let target = match tracks[0].state {
        collector_api_types::MusicTrackAvailabilityState::Available {
            representation_id,
            file_index,
            ..
        } => PlaybackTarget {
            item_id: item,
            representation_id,
            file_index,
        },
        ref value => panic!("{value:?}"),
    };
    let authority = remote_media::start(state.clone(), &config).await.unwrap();
    let key = std::fs::read_to_string(config.socket.parent().unwrap().join("service-key")).unwrap();
    let socket = reqwest::Client::builder()
        .unix_socket(config.socket.as_path())
        .timeout(Duration::from_secs(3))
        .build()
        .unwrap();
    let response = socket
        .post("http://authority/authorize")
        .json(&MediaAuthorizeRequest {
            token: nonce(),
            origin: None,
        })
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 401);
    let response = socket
        .post("http://authority/api/v1/admin/settings")
        .bearer_auth(&key)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 404);
    let media = collector_media::MediaState::new(
        &root.path().join("archive"),
        &config.socket,
        key.clone(),
        vec![origin.into()],
    )
    .unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let router = collector_media::router(media);
    let task = Task(tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    }));
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .unwrap();
    for path in [
        "/",
        "/pair",
        "/api/v1/auth/login",
        "/api/v1/admin/settings",
        "/authorize",
    ] {
        assert_eq!(
            client
                .get(format!("{base}{path}"))
                .send()
                .await
                .unwrap()
                .status(),
            404
        );
    }
    assert_eq!(
        client
            .get(format!("{base}/read"))
            .header("Cookie", format!("collector_session={session}"))
            .send()
            .await
            .unwrap()
            .status(),
        400
    );
    assert_eq!(
        client
            .get(format!("{base}/read"))
            .bearer_auth(&session)
            .send()
            .await
            .unwrap()
            .status(),
        401
    );
    let control = collector_server::router(state.clone());
    let response = control
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/playback/remote-grants")
                .header("origin", origin)
                .header("cookie", format!("collector_session={session}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&RemoteGrantRequest {
                        target: Some(target.clone()),
                    })
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let grant: RemoteMediaGrant = serde_json::from_slice(
        &axum::body::to_bytes(response.into_body(), 16384)
            .await
            .unwrap(),
    )
    .unwrap();
    let url = format!("{base}/read?grant={}", grant.token);
    assert_eq!(
        client.get(&url).send().await.unwrap().status(),
        401,
        "browser grant accepted without its origin"
    );
    assert_eq!(
        client
            .get(&url)
            .header("origin", "https://other.example")
            .send()
            .await
            .unwrap()
            .status(),
        403
    );
    let head = client
        .head(&url)
        .header("origin", origin)
        .send()
        .await
        .unwrap();
    assert_eq!(head.status(), 200);
    assert_eq!(head.headers()["content-length"], audio.len().to_string());
    for (first, last) in [(0, 15), (audio.len() - 16, audio.len() - 1)] {
        let response = client
            .get(&url)
            .header("origin", origin)
            .header("range", format!("bytes={first}-{last}"))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 206);
        assert_eq!(response.headers()["etag"], grant.etag);
        assert_eq!(
            response.bytes().await.unwrap().as_ref(),
            &audio[first..=last]
        );
    }
    let response = client
        .get(&url)
        .header("origin", origin)
        .header("range", "bytes=-1")
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 206);
    assert_eq!(
        response.bytes().await.unwrap().as_ref(),
        &audio[audio.len() - 1..]
    );
    let response = client
        .get(&url)
        .header("origin", origin)
        .header("range", "bytes=0-1,2-3")
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 416);
    let response = client
        .get(&url)
        .header("origin", origin)
        .header("range", "bytes=0-1")
        .header("if-range", "\"different\"")
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(response.bytes().await.unwrap().as_ref(), audio);
    let response = control
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/v1/items/{item}/representations/{}/{}",
                    target.representation_id, target.file_index
                ))
                .header("cookie", format!("collector_session={session}"))
                .header("forwarded", "for=127.0.0.1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 503, "control origin served audio");
    // A grant never becomes a collection-wide or service-secret authorization.
    let other = store
        .create_user("listener", "another remote test password", false)
        .unwrap();
    let (_, other_actor) = store
        .login("listener", "another remote test password", "other browser")
        .unwrap();
    assert!(
        state
            .service
            .issue_remote_media(
                &other_actor,
                Some(target.clone()),
                Some(origin.into()),
                audience,
                state.pairing.server_id,
                nonce()
            )
            .is_err()
    );
    store
        .set_grant(collection.id, other, Revision(0), Some(Role::Curator))
        .unwrap();
    let scoped = state
        .service
        .issue_remote_media(
            &other_actor,
            Some(target.clone()),
            Some(origin.into()),
            audience,
            state.pairing.server_id,
            nonce(),
        )
        .unwrap();
    let admitted = state
        .service
        .authorize_remote_media(&scoped.token, Some(origin), audience, nonce())
        .unwrap();
    store
        .set_grant(collection.id, other, Revision(1), Some(Role::Viewer))
        .unwrap();
    assert!(
        state
            .service
            .recheck_remote_media(&admitted.handle, audience)
            .is_err(),
        "curator-only audio survived demotion"
    );
    store
        .set_grant(collection.id, other, Revision(2), None)
        .unwrap();
    assert!(
        state
            .service
            .recheck_remote_media(&admitted.handle, audience)
            .is_err()
    );
    assert!(
        state
            .service
            .authorize_remote_media(
                &grant.token,
                Some(origin),
                "https://wrong.example/",
                nonce()
            )
            .is_err()
    );
    assert!(
        state
            .service
            .recheck_remote_media(&nonce(), audience)
            .is_err()
    );
    let mut wrong = target.clone();
    wrong.file_index += 1;
    assert!(
        state
            .service
            .issue_remote_media(
                &actor,
                Some(wrong),
                Some(origin.into()),
                audience,
                state.pairing.server_id,
                nonce()
            )
            .is_err()
    );
    assert!(
        state
            .service
            .issue_remote_media(
                &actor,
                None,
                Some(origin.into()),
                audience,
                state.pairing.server_id,
                grant.token.clone()
            )
            .is_err()
    );
    let probe = state
        .service
        .issue_remote_media(
            &actor,
            None,
            Some(origin.into()),
            audience,
            state.pairing.server_id,
            nonce(),
        )
        .unwrap();
    let probe_url = format!("{base}/read?grant={}", probe.token);
    let response = client
        .get(&probe_url)
        .header("origin", origin)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(
        response.text().await.unwrap(),
        format!(
            "collector-media-probe-v1:{}:{}",
            state.pairing.server_id,
            Digest::sha256(probe.token.as_bytes()).hex
        )
    );
    assert_eq!(
        client
            .get(&probe_url)
            .header("origin", origin)
            .send()
            .await
            .unwrap()
            .status(),
        401
    );
    let same_origin_audience = format!("{origin}/media/");
    let same_origin_grant = state
        .service
        .issue_remote_media(
            &actor,
            Some(target.clone()),
            Some(origin.into()),
            &same_origin_audience,
            state.pairing.server_id,
            nonce(),
        )
        .unwrap();
    assert!(
        state
            .service
            .authorize_remote_media(
                &same_origin_grant.token,
                None,
                &same_origin_audience,
                nonce()
            )
            .is_ok()
    );
    assert!(
        state
            .service
            .authorize_remote_media(
                &same_origin_grant.token,
                Some("https://other.example"),
                &same_origin_audience,
                nonce()
            )
            .is_err()
    );
    native_round_trip(&state, root.path(), &target, user).await;
    clock.0.fetch_add(3590, Ordering::SeqCst);
    let ticket: MediaReadTicket = socket
        .post("http://authority/authorize")
        .bearer_auth(&key)
        .json(&MediaAuthorizeRequest {
            token: grant.token.clone(),
            origin: Some(origin.into()),
        })
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    // Expiry prevents new requests but does not revoke an already admitted stream.
    clock.0.fetch_add(11, Ordering::SeqCst);
    assert_eq!(
        socket
            .post("http://authority/recheck")
            .bearer_auth(&key)
            .json(&MediaHandleRequest {
                handle: ticket.handle.clone()
            })
            .send()
            .await
            .unwrap()
            .status(),
        204
    );
    assert_eq!(
        client
            .get(&url)
            .header("origin", origin)
            .send()
            .await
            .unwrap()
            .status(),
        401
    );
    // A crashed media process cannot hold every user's admission slot for four hours.
    let lost = state
        .service
        .issue_remote_media(
            &actor,
            Some(target.clone()),
            Some(origin.into()),
            audience,
            state.pairing.server_id,
            nonce(),
        )
        .unwrap();
    let abandoned = state
        .service
        .authorize_remote_media(&lost.token, Some(origin), audience, nonce())
        .unwrap();
    let mut occupied = vec![abandoned.handle];
    loop {
        match state
            .service
            .authorize_remote_media(&lost.token, Some(origin), audience, nonce())
        {
            Ok(read) => occupied.push(read.handle),
            Err(Error::RateLimited(_)) => break,
            Err(error) => panic!("unexpected admission failure: {error}"),
        }
        assert!(
            occupied.len() <= 8,
            "per-user stream bound was not enforced"
        );
    }
    clock.0.fetch_add(31, Ordering::SeqCst);
    for handle in occupied {
        assert!(
            state
                .service
                .recheck_remote_media(&handle, audience)
                .is_err()
        );
    }
    // Keep the revocation assertion about a currently live handle, not an idle expiry.
    let ticket = state
        .service
        .authorize_remote_media(&lost.token, Some(origin), audience, nonce())
        .unwrap();
    AuthenticationPort::revoke_session(store.as_ref(), user, actor.principal.device_id.unwrap())
        .unwrap();
    assert_eq!(
        socket
            .post("http://authority/recheck")
            .bearer_auth(&key)
            .json(&MediaHandleRequest {
                handle: ticket.handle
            })
            .send()
            .await
            .unwrap()
            .status(),
        401
    );
    drop(authority);
    assert_eq!(
        client
            .get(format!("{base}/api/v1/identity"))
            .send()
            .await
            .unwrap()
            .status(),
        503
    );
    drop(task);
}

// Separate process isolates fixture system roots from the test runner and OS trust store.
#[test]
#[ignore = "subprocess fixture invoked by isolated_media_enforces_grants_exact_bytes_and_socket_revocation"]
fn native_remote_reader_fixture() {
    use collector_client::{ConnectionKey, EnrolledCredential, NativeCredentialStorage};
    use std::io::{Read, Seek, SeekFrom};
    struct Memory(EnrolledCredential);
    impl NativeCredentialStorage for Memory {
        fn save(&self, _: &EnrolledCredential) -> Result<()> {
            unreachable!()
        }
        fn load(&self, _: ConnectionKey) -> Result<Option<EnrolledCredential>> {
            Ok(Some(self.0.clone()))
        }
        fn remove(&self, _: ConnectionKey) -> Result<()> {
            unreachable!()
        }
        fn load_legacy(&self, _: ServerId) -> Result<Option<EnrolledCredential>> {
            unreachable!()
        }
        fn remove_legacy(&self, _: ServerId) -> Result<()> {
            unreachable!()
        }
    }
    let fixture: serde_json::Value = serde_json::from_slice(
        &std::fs::read(std::env::var("COLLECTOR_NATIVE_MEDIA_FIXTURE").unwrap()).unwrap(),
    )
    .unwrap();
    let credential: EnrolledCredential =
        serde_json::from_value(fixture["credential"].clone()).unwrap();
    let key = ConnectionKey {
        server_id: credential.server_id,
        user_id: credential.user_id.unwrap(),
    };
    let client = collector_client::EnrolledClient::from_storage(&Memory(credential), key).unwrap();
    let target: PlaybackTarget = serde_json::from_value(fixture["target"].clone()).unwrap();
    let expected = include_bytes!("../../collector-store/tests/fixtures/digital-audio/01.flac");
    let cancellation = collector_client::MediaCancellation::default();
    let mut reader = client
        .media_reader(
            target.item_id,
            target.representation_id,
            target.file_index,
            cancellation.clone(),
        )
        .unwrap();
    assert!(!reader.is_local());
    assert_eq!(reader.length(), expected.len() as u64);
    let mut bytes = [0; 16];
    reader.read_exact(&mut bytes).unwrap();
    assert_eq!(&bytes, &expected[..16]);
    reader.seek(SeekFrom::End(-16)).unwrap();
    reader.read_exact(&mut bytes).unwrap();
    assert_eq!(&bytes, &expected[expected.len() - 16..]);
    cancellation.cancel();
    assert_eq!(
        reader.seek(SeekFrom::Start(0)).unwrap_err().kind(),
        std::io::ErrorKind::Interrupted
    );
}

async fn native_round_trip(
    state: &AppState,
    root: &std::path::Path,
    target: &PlaybackTarget,
    user: UserId,
) {
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    let control_cert = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
    let media_cert = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
    let control_listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let media_listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    control_listener.set_nonblocking(true).unwrap();
    media_listener.set_nonblocking(true).unwrap();
    let control_url = format!(
        "https://localhost:{}/",
        control_listener.local_addr().unwrap().port()
    );
    let media_url = format!(
        "https://localhost:{}/",
        media_listener.local_addr().unwrap().port()
    );
    let config = remote_media::Configuration {
        base_url: media_url,
        socket: root.join("native-auth/socket"),
    };
    let mut control = state.clone();
    control.remote_media = Some(Arc::new(config.clone()));
    let _authority = remote_media::start(control.clone(), &config).await.unwrap();
    let key = std::fs::read_to_string(config.socket.parent().unwrap().join("service-key")).unwrap();
    let media =
        collector_media::MediaState::new(&root.join("archive"), &config.socket, key, vec![])
            .unwrap();
    let control_tls = axum_server::tls_rustls::RustlsConfig::from_pem(
        control_cert.cert.pem().into_bytes(),
        control_cert.key_pair.serialize_pem().into_bytes(),
    )
    .await
    .unwrap();
    let media_tls = axum_server::tls_rustls::RustlsConfig::from_pem(
        media_cert.cert.pem().into_bytes(),
        media_cert.key_pair.serialize_pem().into_bytes(),
    )
    .await
    .unwrap();
    let _control_task = Task(tokio::spawn(async move {
        axum_server::from_tcp_rustls(control_listener, control_tls)
            .unwrap()
            .serve(collector_server::router(control).into_make_service())
            .await
            .unwrap();
    }));
    // Simulate losing one admission lease while keeping real core authorization/file delivery.
    // The next request must present a freshly issued capability, not retry the rejected token.
    let rejected = Arc::new(std::sync::Mutex::new(None::<String>));
    let seen = rejected.clone();
    let media_router = collector_media::router(media).layer(axum::middleware::from_fn(
        move |request: axum::extract::Request, next: axum::middleware::Next| {
            let seen = seen.clone();
            async move {
                if request.method() == axum::http::Method::GET && request.uri().path() == "/read" {
                    let token = request
                        .headers()
                        .get("authorization")
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .to_owned();
                    let reject = {
                        let mut old = seen.lock().unwrap();
                        match old.as_ref() {
                            Some(old) => *old == token,
                            None => {
                                *old = Some(token);
                                true
                            }
                        }
                    };
                    if reject {
                        return axum::http::StatusCode::UNAUTHORIZED.into_response();
                    }
                }
                next.run(request).await
            }
        },
    ));
    let _media_task = Task(tokio::spawn(async move {
        axum_server::from_tcp_rustls(media_listener, media_tls)
            .unwrap()
            .serve(media_router.into_make_service())
            .await
            .unwrap();
    }));
    let start = state
        .store
        .start_enrollment(Some(user), "Native fixture", &control_url)
        .unwrap();
    state
        .store
        .approve_enrollment(user, &start.user_code)
        .unwrap();
    let issue = state
        .store
        .complete_enrollment(start.id, &start.completion_secret)
        .unwrap();
    let pairing = collector_client::Pairing {
        server_id: state.pairing.server_id,
        certificate_pem: control_cert.cert.pem(),
        certificate_sha256: Digest::sha256(control_cert.cert.pem().as_bytes()).hex,
        endpoints: vec![collector_client::Endpoint {
            url: control_url.parse().unwrap(),
            route: collector_client::Route::Public,
            latency_ms: None,
            healthy: true,
        }],
    };
    let credential = collector_client::EnrolledCredential {
        user_id: Some(user),
        username: issue.username,
        display_name: issue.display_name,
        server_id: pairing.server_id,
        credential_id: issue.device_id,
        credential: issue.credential,
        expires_at: issue.expires_at,
        pairing,
        trust: collector_client::ConnectionTrust::PinnedCertificate,
    };
    let fixture = root.join("native-fixture.json");
    let trust = root.join("media-ca.pem");
    let empty = root.join("empty-roots");
    std::fs::create_dir(&empty).unwrap();
    std::fs::write(&trust, media_cert.cert.pem()).unwrap();
    std::fs::write(
        &fixture,
        serde_json::to_vec(&serde_json::json!({"credential":credential,"target":target})).unwrap(),
    )
    .unwrap();
    let output = tokio::time::timeout(
        Duration::from_secs(30),
        tokio::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "native_remote_reader_fixture",
                "--ignored",
                "--nocapture",
            ])
            .env("COLLECTOR_NATIVE_MEDIA_FIXTURE", &fixture)
            .env("SSL_CERT_FILE", &trust)
            .env("SSL_CERT_DIR", &empty)
            .kill_on_drop(true)
            .output(),
    )
    .await
    .unwrap()
    .unwrap();
    assert!(
        rejected.lock().unwrap().is_some(),
        "native lease-loss fixture was not exercised"
    );
    assert!(
        output.status.success(),
        "native fixture failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
