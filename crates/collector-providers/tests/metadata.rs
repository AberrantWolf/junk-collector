use collector_app::{FrozenJobInput, ProcessingExecutionContext, ProviderPort, ProviderRequest};
use collector_core::{CollectionId, Digest, ModuleId, OperationId, RawJson};
use collector_modules::music::{
    COVER_ART_ARCHIVE_ACQUIRE_OPERATION, COVER_ART_ARCHIVE_PROVIDER_ID, CoverArtArchiveQuery,
    MUSICBRAINZ_ACQUIRE_OPERATION, MUSICBRAINZ_PROVIDER_ID, MusicBrainzQuery,
};
use collector_providers::{CoverArtArchiveProvider, MusicBrainzProvider};
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

struct Execution {
    cancel_after: Option<usize>,
    checks: AtomicUsize,
}

impl Execution {
    fn active() -> Self {
        Self {
            cancel_after: None,
            checks: AtomicUsize::new(0),
        }
    }
}

impl ProcessingExecutionContext for Execution {
    fn frozen_job(&self) -> &FrozenJobInput {
        panic!("fixture does not inspect the frozen job")
    }

    fn report_progress(
        &mut self,
        _phase: &str,
        _completed_units: u64,
        _total_units: u64,
    ) -> collector_core::Result<()> {
        Ok(())
    }

    fn cancellation_requested(&self) -> collector_core::Result<bool> {
        let check = self.checks.fetch_add(1, Ordering::SeqCst) + 1;
        Ok(self.cancel_after.is_some_and(|limit| check >= limit))
    }
}

#[derive(Clone)]
struct Reply {
    delay: Duration,
    status_and_headers: String,
    body: Vec<u8>,
}

struct FixtureServer {
    origin: String,
    requests: Arc<Mutex<Vec<(Instant, String)>>>,
    join: Option<thread::JoinHandle<()>>,
    stop: Arc<std::sync::atomic::AtomicBool>,
}

impl Drop for FixtureServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(join) = self.join.take() {
            join.join().unwrap();
        }
    }
}

fn serve(replies: Vec<Reply>) -> FixtureServer {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    listener.set_nonblocking(true).unwrap();
    let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let stopping = stop.clone();
    let requests = Arc::new(Mutex::new(Vec::new()));
    let seen = Arc::clone(&requests);
    let join = thread::spawn(move || {
        for reply in replies {
            let (mut stream, _) = loop {
                if stopping.load(Ordering::Relaxed) {
                    return;
                }
                match listener.accept() {
                    Ok(connection) => break connection,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(10))
                    }
                    Err(error) => panic!("fixture accept: {error}"),
                }
            };
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            stream
                .set_write_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut request = vec![0_u8; 16 * 1024];
            let read = stream.read(&mut request).unwrap();
            seen.lock().unwrap().push((
                Instant::now(),
                String::from_utf8_lossy(&request[..read]).into_owned(),
            ));
            thread::sleep(reply.delay);
            let response = format!(
                "HTTP/1.1 {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                reply.status_and_headers,
                reply.body.len()
            );
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.write_all(&reply.body);
        }
    });
    FixtureServer {
        origin: format!("http://{address}"),
        requests,
        join: Some(join),
        stop,
    }
}

fn reply(status: &str, body: impl Into<Vec<u8>>) -> Reply {
    Reply {
        delay: Duration::ZERO,
        status_and_headers: status.into(),
        body: body.into(),
    }
}

fn mb_request(query: &MusicBrainzQuery, limit: u64) -> ProviderRequest {
    ProviderRequest {
        collection_id: CollectionId::new(),
        provider: ModuleId(MUSICBRAINZ_PROVIDER_ID.into()),
        operation: OperationId::new(MUSICBRAINZ_ACQUIRE_OPERATION).unwrap(),
        source_fingerprint: None,
        query: RawJson::from_value(serde_json::to_value(query).unwrap()),
        maximum_response_bytes: limit,
    }
}

fn caa_request(query: &CoverArtArchiveQuery, limit: u64) -> ProviderRequest {
    ProviderRequest {
        collection_id: CollectionId::new(),
        provider: ModuleId(COVER_ART_ARCHIVE_PROVIDER_ID.into()),
        operation: OperationId::new(COVER_ART_ARCHIVE_ACQUIRE_OPERATION).unwrap(),
        source_fingerprint: None,
        query: RawJson::from_value(serde_json::to_value(query).unwrap()),
        maximum_response_bytes: limit,
    }
}

fn releases_json(title: &str) -> Vec<u8> {
    format!(
        r#"{{"releases":[{{"id":"00000000-0000-4000-8000-000000000001","title":{title:?},"artist-credit":[{{"name":"Example Artist","artist":{{"id":"00000000-0000-4000-8000-000000000002","name":"Example Artist"}}}}],"text-representation":{{"language":"eng","script":"Latn"}},"media":[{{"discs":[{{"id":"known-disc-id"}}]}}],"release-group":{{"id":"00000000-0000-4000-8000-000000000003"}},"unknown":null}}],"count":1}}"#
    )
    .into_bytes()
}

fn ambiguous_releases_json() -> Vec<u8> {
    r#"{"releases":[{"id":"00000000-0000-4000-8000-000000000001","title":"強い一致","score":80},{"id":"00000000-0000-4000-8000-000000000002","title":"English Name","score":100}]}"#
        .as_bytes()
        .to_vec()
}

#[test]
fn musicbrainz_normalizes_reordered_responses_and_versions_changed_raw_content() {
    let first = releases_json("Original title");
    let reordered = br#"{"count":1,"releases":[{"unknown":null,"release-group":{"id":"00000000-0000-4000-8000-000000000003"},"media":[{"discs":[{"id":"known-disc-id"}]}],"text-representation":{"script":"Latn","language":"eng"},"artist-credit":[{"artist":{"name":"Example Artist","id":"00000000-0000-4000-8000-000000000002"},"name":"Example Artist"}],"title":"Original title","id":"00000000-0000-4000-8000-000000000001"}]}"#.to_vec();
    let changed = releases_json("Changed title");
    let server = serve(vec![
        reply("200 OK", first),
        reply("200 OK", reordered),
        reply("200 OK", changed),
    ]);
    let provider = MusicBrainzProvider::with_test_origin(
        "junk-collector-test/0.1 (test@example.invalid)",
        &server.origin,
        Duration::from_secs(1),
        Duration::ZERO,
        Duration::from_secs(60),
        Duration::from_secs(60),
    )
    .unwrap();
    let query = MusicBrainzQuery::DiscId {
        disc_id: "known-disc-id".into(),
        refresh: true,
    };
    let first = provider
        .acquire(&mb_request(&query, 1024 * 1024), &mut Execution::active())
        .unwrap();
    let reordered = provider
        .acquire(&mb_request(&query, 1024 * 1024), &mut Execution::active())
        .unwrap();
    let changed = provider
        .acquire(&mb_request(&query, 1024 * 1024), &mut Execution::active())
        .unwrap();
    let first_candidates = provider.parse_response_candidates(&first, &[]).unwrap();
    let reordered_candidates = provider.parse_response_candidates(&reordered, &[]).unwrap();
    let changed_candidates = provider.parse_response_candidates(&changed, &[]).unwrap();

    assert_eq!(first_candidates[0].title.value, "Original title");
    assert_eq!(first_candidates[0].title, reordered_candidates[0].title);
    assert_ne!(
        first_candidates[0].identity.provider_record_version,
        reordered_candidates[0].identity.provider_record_version,
        "a changed raw response receives a new immutable version even when normalization is equal"
    );
    assert_ne!(
        first_candidates[0].identity.provider_record_version,
        changed_candidates[0].identity.provider_record_version
    );
    assert!(first_candidates[0].complete_identity);
    assert_eq!(first_candidates[0].provider_priority, 20);
    assert!(server.requests.lock().unwrap().iter().all(|(_, request)| {
        let request = request.to_ascii_lowercase();
        !request.contains("loan")
            && !request.contains("private")
            && !request.contains("acquisition")
            && !request.contains("filesystem")
    }));
}

#[test]
fn musicbrainz_generated_ambiguous_fixture_keeps_stable_alternatives() {
    let fixture = ambiguous_releases_json();
    assert_eq!(fixture.len(), 175);
    assert_eq!(
        Digest::sha256(&fixture).hex,
        "c685dac16b859af9f834771ac862c4e9d421eca9ff72fa848e214f859062a7fe"
    );
    let server = serve(vec![reply("200 OK", fixture)]);
    let provider = MusicBrainzProvider::with_test_origin(
        "junk-collector-test/0.1 (test@example.invalid)",
        &server.origin,
        Duration::from_secs(1),
        Duration::ZERO,
        Duration::from_secs(60),
        Duration::from_secs(60),
    )
    .unwrap();
    let query = MusicBrainzQuery::Search {
        query: "fixture title".into(),
        refresh: true,
    };
    let response = provider
        .acquire(&mb_request(&query, 1024 * 1024), &mut Execution::active())
        .unwrap();
    let candidates = provider.parse_response_candidates(&response, &[]).unwrap();

    assert_eq!(candidates.len(), 2);
    assert_eq!(
        candidates
            .iter()
            .map(|candidate| candidate.identity.provider_record_id.as_str())
            .collect::<Vec<_>>(),
        vec![
            "00000000-0000-4000-8000-000000000001",
            "00000000-0000-4000-8000-000000000002"
        ]
    );
    assert!(candidates.iter().all(|candidate| {
        !candidate.complete_identity
            && candidate.strength == collector_core::MatchStrength::Name
            && candidate.provider_priority == 20
    }));
}

#[test]
fn musicbrainz_cache_ttls_and_explicit_refresh_still_use_shared_limiter() {
    let server = serve(vec![
        reply("200 OK", releases_json("cached")),
        reply("200 OK", releases_json("refreshed")),
        reply("200 OK", releases_json("stale success")),
        reply(
            "404 Not Found\r\nContent-Type: text/html",
            b"missing".to_vec(),
        ),
        reply(
            "404 Not Found\r\nContent-Type: text/html",
            b"still missing".to_vec(),
        ),
    ]);
    let provider = MusicBrainzProvider::with_test_origin(
        "junk-collector-test/0.1 (test@example.invalid)",
        &server.origin,
        Duration::from_secs(1),
        Duration::from_millis(20),
        Duration::from_millis(30),
        Duration::from_millis(20),
    )
    .unwrap();
    let cached = MusicBrainzQuery::DiscId {
        disc_id: "known-disc-id".into(),
        refresh: false,
    };
    provider
        .acquire(&mb_request(&cached, 1024 * 1024), &mut Execution::active())
        .unwrap();
    provider
        .acquire(&mb_request(&cached, 1024 * 1024), &mut Execution::active())
        .unwrap();
    assert_eq!(
        server.requests.lock().unwrap().len(),
        1,
        "fresh success cache"
    );

    let refresh = MusicBrainzQuery::DiscId {
        disc_id: "known-disc-id".into(),
        refresh: true,
    };
    let refreshed = provider
        .acquire(&mb_request(&refresh, 1024 * 1024), &mut Execution::active())
        .unwrap();
    let refreshed_observation: collector_modules::music::MusicBrainzSnapshotObservation =
        refreshed.parsed_observation.parse().unwrap();
    assert!(refreshed_observation.query.refresh());
    let times = server
        .requests
        .lock()
        .unwrap()
        .iter()
        .map(|value| value.0)
        .collect::<Vec<_>>();
    assert!(times[1].duration_since(times[0]) >= Duration::from_millis(18));

    let cached_after_refresh = provider
        .acquire(&mb_request(&cached, 1024 * 1024), &mut Execution::active())
        .unwrap();
    let cached_observation: collector_modules::music::MusicBrainzSnapshotObservation =
        cached_after_refresh.parsed_observation.parse().unwrap();
    assert!(
        !cached_observation.query.refresh(),
        "cached bytes remain bound to the current submitted query"
    );
    thread::sleep(Duration::from_millis(35));
    provider
        .acquire(&mb_request(&cached, 1024 * 1024), &mut Execution::active())
        .unwrap();
    assert_eq!(
        server.requests.lock().unwrap().len(),
        3,
        "stale success cache"
    );

    let missing = MusicBrainzQuery::DiscId {
        disc_id: "missing-disc-id".into(),
        refresh: false,
    };
    let missing_response = provider
        .acquire(&mb_request(&missing, 1024), &mut Execution::active())
        .unwrap();
    assert!(
        provider
            .parse_response_candidates(&missing_response, &[])
            .unwrap()
            .is_empty(),
        "an exact non-JSON 404 body remains an honest no-match observation"
    );
    provider
        .acquire(&mb_request(&missing, 1024), &mut Execution::active())
        .unwrap();
    assert_eq!(
        server.requests.lock().unwrap().len(),
        4,
        "fresh negative cache"
    );
    thread::sleep(Duration::from_millis(25));
    provider
        .acquire(&mb_request(&missing, 1024), &mut Execution::active())
        .unwrap();
    assert_eq!(
        server.requests.lock().unwrap().len(),
        5,
        "stale negative cache"
    );
}

#[test]
fn musicbrainz_timeout_retry_after_cancellation_and_body_failures_are_bounded() {
    assert!(MusicBrainzProvider::new("JunkCollector/0.1 (missing-contact)").is_err());
    assert!(MusicBrainzProvider::new("JunkCollector (test@example.invalid)").is_err());

    let timed_out = serve(
        (0..3)
            .map(|_| Reply {
                delay: Duration::from_millis(60),
                status_and_headers: "200 OK".into(),
                body: releases_json("late"),
            })
            .collect(),
    );
    let provider = MusicBrainzProvider::with_test_origin(
        "junk-collector-test/0.1 (test@example.invalid)",
        &timed_out.origin,
        Duration::from_millis(15),
        Duration::ZERO,
        Duration::from_secs(1),
        Duration::from_secs(1),
    )
    .unwrap();
    let query = MusicBrainzQuery::Search {
        query: "safe title".into(),
        refresh: true,
    };
    assert!(
        provider
            .acquire(&mb_request(&query, 1024), &mut Execution::active())
            .is_err()
    );
    assert_eq!(timed_out.requests.lock().unwrap().len(), 3);

    let rate_limited = serve(vec![reply(
        "429 Too Many Requests\r\nRetry-After: 1",
        Vec::new(),
    )]);
    let provider = MusicBrainzProvider::with_test_origin(
        "junk-collector-test/0.1 (test@example.invalid)",
        &rate_limited.origin,
        Duration::from_secs(1),
        Duration::ZERO,
        Duration::from_secs(1),
        Duration::from_secs(1),
    )
    .unwrap();
    let mut cancelled = Execution {
        cancel_after: Some(7),
        checks: AtomicUsize::new(0),
    };
    assert!(matches!(
        provider.acquire(&mb_request(&query, 1024), &mut cancelled),
        Err(collector_core::Error::JobCancelled)
    ));
    assert_eq!(rate_limited.requests.lock().unwrap().len(), 1);

    let retried = serve(vec![
        reply("429 Too Many Requests\r\nRetry-After: 1", Vec::new()),
        reply("200 OK", releases_json("after rate limit")),
    ]);
    let provider = MusicBrainzProvider::with_test_origin(
        "junk-collector-test/0.1 (test@example.invalid)",
        &retried.origin,
        Duration::from_secs(2),
        Duration::ZERO,
        Duration::from_secs(1),
        Duration::from_secs(1),
    )
    .unwrap();
    let started = Instant::now();
    provider
        .acquire(&mb_request(&query, 1024 * 1024), &mut Execution::active())
        .unwrap();
    assert!(started.elapsed() >= Duration::from_millis(900));
    assert_eq!(retried.requests.lock().unwrap().len(), 2);

    let malformed = serve(vec![reply("200 OK", b"{not-json".to_vec())]);
    let provider = MusicBrainzProvider::with_test_origin(
        "junk-collector-test/0.1 (test@example.invalid)",
        &malformed.origin,
        Duration::from_secs(1),
        Duration::ZERO,
        Duration::from_secs(1),
        Duration::from_secs(1),
    )
    .unwrap();
    assert!(
        provider
            .acquire(&mb_request(&query, 1024), &mut Execution::active())
            .is_err()
    );

    let oversized = serve(vec![reply("200 OK", releases_json("oversized"))]);
    let provider = MusicBrainzProvider::with_test_origin(
        "junk-collector-test/0.1 (test@example.invalid)",
        &oversized.origin,
        Duration::from_secs(1),
        Duration::ZERO,
        Duration::from_secs(1),
        Duration::from_secs(1),
    )
    .unwrap();
    assert!(
        provider
            .acquire(&mb_request(&query, 16), &mut Execution::active())
            .is_err()
    );
}

#[test]
fn provider_redirects_reject_foreign_credentials_private_targets_and_excess_hops() {
    for location in [
        "http://127.0.0.1:9/private",
        "http://user:secret@127.0.0.1:9/private",
        "http://169.254.169.254/latest/meta-data",
    ] {
        let server = serve(
            (0..3)
                .map(|_| reply(&format!("302 Found\r\nLocation: {location}"), Vec::new()))
                .collect(),
        );
        let provider = MusicBrainzProvider::with_test_origin(
            "junk-collector-test/0.1 (test@example.invalid)",
            &server.origin,
            Duration::from_secs(1),
            Duration::ZERO,
            Duration::from_secs(1),
            Duration::from_secs(1),
        )
        .unwrap();
        let query = MusicBrainzQuery::Search {
            query: "safe".into(),
            refresh: true,
        };
        assert!(
            provider
                .acquire(&mb_request(&query, 1024), &mut Execution::active())
                .is_err()
        );
    }

    let server = serve(vec![
        reply("302 Found\r\nLocation: /1", Vec::new()),
        reply("302 Found\r\nLocation: /2", Vec::new()),
        reply("302 Found\r\nLocation: /3", Vec::new()),
        reply("302 Found\r\nLocation: /4", Vec::new()),
    ]);
    let provider = MusicBrainzProvider::with_test_origin(
        "junk-collector-test/0.1 (test@example.invalid)",
        &server.origin,
        Duration::from_secs(1),
        Duration::ZERO,
        Duration::from_secs(1),
        Duration::from_secs(1),
    )
    .unwrap();
    let query = MusicBrainzQuery::Search {
        query: "safe".into(),
        refresh: true,
    };
    assert!(
        provider
            .acquire(&mb_request(&query, 1024), &mut Execution::active())
            .is_err()
    );
    assert_eq!(server.requests.lock().unwrap().len(), 4);
}

#[test]
fn cover_art_listing_and_image_are_bounded_pinned_and_request_no_private_item_data() {
    let png = vec![0x89, b'P', b'N', b'G', 13, 10, 26, 10];
    let listing = br#"{"release":"https://musicbrainz.org/release/00000000-0000-4000-8000-000000000001","images":[{"id":12345,"front":true,"back":false,"approved":true,"types":["Front"],"comment":"scan","edit":999,"image":"http://untrusted.invalid/image.jpg","thumbnails":{"250":"http://untrusted.invalid/thumb.jpg"}},{"id":"67890","front":false,"back":true,"approved":true,"types":["Back"],"comment":"back scan"}]}"#.to_vec();
    let server = serve(vec![
        reply("200 OK", listing),
        reply(
            "307 Temporary Redirect\r\nLocation: /cdn/image.png",
            Vec::new(),
        ),
        reply("200 OK\r\nContent-Type: image/png", png.clone()),
    ]);
    let provider = CoverArtArchiveProvider::with_test_origins(
        "junk-collector-test/0.1 (test@example.invalid)",
        &server.origin,
        &[server.origin.as_str()],
        Duration::from_secs(1),
    )
    .unwrap();
    let release_id = "00000000-0000-4000-8000-000000000001".to_owned();
    let listing_response = provider
        .acquire(
            &caa_request(
                &CoverArtArchiveQuery::Listing {
                    release_id: release_id.clone(),
                },
                1024 * 1024,
            ),
            &mut Execution::active(),
        )
        .unwrap();
    let images = provider.parse_listing(&listing_response).unwrap();
    assert_eq!(
        images
            .iter()
            .map(|image| image.image_id.as_str())
            .collect::<Vec<_>>(),
        ["12345", "67890"]
    );
    assert!(images[0].front);

    let image_response = provider
        .acquire(
            &caa_request(
                &CoverArtArchiveQuery::Image {
                    release_id,
                    image_id: "12345".into(),
                },
                20 * 1024 * 1024,
            ),
            &mut Execution::active(),
        )
        .unwrap();
    assert_eq!(image_response.bytes, png);
    assert_eq!(image_response.media_type, "image/png");
    let requests = server.requests.lock().unwrap();
    assert!(requests.iter().all(|(_, request)| {
        let request = request.to_ascii_lowercase();
        request.contains("user-agent: junk-collector-test/0.1 (test@example.invalid)")
            && !request.contains("loan")
            && !request.contains("private")
            && !request.contains("acquisition")
            && !request.contains("filesystem")
    }));
}

#[test]
fn cached_bounds_candidate_versions_and_malformed_successes_are_checked() {
    let server = serve(vec![
        reply("200 OK", releases_json("Cached")),
        reply("200 OK", b"{}".to_vec()),
        reply("200 OK", Vec::new()),
    ]);
    let provider = MusicBrainzProvider::with_test_origin(
        "junk-collector-test/0.1 (test@example.invalid)",
        &server.origin,
        Duration::from_secs(1),
        Duration::ZERO,
        Duration::from_secs(60),
        Duration::from_secs(60),
    )
    .unwrap();
    let query = MusicBrainzQuery::DiscId {
        disc_id: "known-disc-id".into(),
        refresh: false,
    };
    let response = provider
        .acquire(&mb_request(&query, 1024 * 1024), &mut Execution::active())
        .unwrap();
    assert!(
        provider
            .acquire(&mb_request(&query, 1), &mut Execution::active())
            .is_err()
    );
    assert_eq!(server.requests.lock().unwrap().len(), 1);
    let plain = provider.parse_response_candidates(&response, &[]).unwrap();
    let art = provider
        .parse_response_candidates(
            &response,
            &[(
                plain[0].identity.provider_record_id.clone(),
                collector_core::AssetId::new(),
            )],
        )
        .unwrap();
    assert_ne!(
        plain[0].identity, art[0].identity,
        "attaching art must not mutate a frozen version"
    );
    assert_eq!(plain[0].title.language.as_ref().unwrap().as_str(), "en");
    let mut different_query = response.clone();
    let mut snapshot: collector_modules::music::MusicBrainzSnapshotObservation =
        response.parsed_observation.parse().unwrap();
    snapshot.query = MusicBrainzQuery::Search {
        query: "Cached".into(),
        refresh: false,
    };
    different_query.parsed_observation =
        RawJson::from_value(serde_json::to_value(snapshot).unwrap());
    let search = provider
        .parse_response_candidates(&different_query, &[])
        .unwrap();
    assert_ne!(
        plain[0].identity, search[0].identity,
        "different evidence strength needs a different version"
    );
    for query in ["malformed", "empty"] {
        assert!(
            provider
                .acquire(
                    &mb_request(
                        &MusicBrainzQuery::Search {
                            query: query.into(),
                            refresh: true
                        },
                        1024
                    ),
                    &mut Execution::active()
                )
                .is_err()
        );
    }
}

#[test]
fn artwork_previews_use_small_endpoint_and_bound_redirects_bytes_types_and_cancellation() {
    use collector_app::ArtworkPreviewPort;
    let release = "00000000-0000-4000-8000-000000000001";
    let server = serve(vec![reply("200 OK\r\nContent-Type: image/png", b"preview")]);
    let provider = CoverArtArchiveProvider::with_test_origins(
        "collector-test/1 (test@example.invalid)",
        &server.origin,
        &[],
        Duration::from_secs(1),
    )
    .unwrap();
    let preview = provider.preview(release, "12345", &|| Ok(false)).unwrap();
    assert_eq!(preview.bytes, b"preview");
    assert_eq!(preview.media_type, "image/png");
    assert!(
        server.requests.lock().unwrap()[0]
            .1
            .contains(&format!("GET /release/{release}/12345-250 "))
    );
    assert!(
        provider
            .preview(release, "../private", &|| Ok(false))
            .is_err()
    );
    assert!(matches!(
        provider.preview(release, "12345", &|| Ok(true)),
        Err(collector_core::Error::JobCancelled)
    ));
    for response in [
        reply(
            "200 OK\r\nContent-Type: image/png",
            vec![0; collector_app::MAX_ARTWORK_PREVIEW_BYTES as usize + 1],
        ),
        reply("200 OK\r\nContent-Type: text/html", b"not an image"),
        reply(
            "307 Temporary Redirect\r\nLocation: http://untrusted.invalid/image",
            Vec::new(),
        ),
        reply("404 Not Found", Vec::new()),
    ] {
        let server = serve(vec![response]);
        let provider = CoverArtArchiveProvider::with_test_origins(
            "collector-test/1 (test@example.invalid)",
            &server.origin,
            &[],
            Duration::from_secs(1),
        )
        .unwrap();
        assert!(provider.preview(release, "12345", &|| Ok(false)).is_err());
    }
    let server = serve(vec![reply(
        "200 OK\r\nContent-Type: image/png",
        vec![0; 128 * 1024],
    )]);
    let provider = CoverArtArchiveProvider::with_test_origins(
        "collector-test/1 (test@example.invalid)",
        &server.origin,
        &[],
        Duration::from_secs(1),
    )
    .unwrap();
    let checks = AtomicUsize::new(0);
    assert!(matches!(
        provider.preview(release, "12345", &|| Ok(checks
            .fetch_add(1, Ordering::SeqCst)
            >= 2)),
        Err(collector_core::Error::JobCancelled)
    ));
}
