use collector_app::SnapshotRecord;
use collector_app::{FrozenJobInput, ProcessingExecutionContext, ProviderPort, ProviderRequest};
use collector_core::{
    ByteLength, CollectionId, Digest, ModuleId, OperationId, RawJson, SnapshotId,
};
use collector_providers::{
    ACCURATERIP_ACQUIRE_OPERATION, ACCURATERIP_PROVIDER_ID, AccurateRipProvider, DbarOutcome,
    DbarQuery, DbarSnapshotObservation, parse_dbar_snapshot, parse_frozen_dbar,
};
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    thread,
    time::Duration,
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

fn query() -> DbarQuery {
    DbarQuery {
        track_count: 2,
        accuraterip_id1: 0x0008_4264,
        accuraterip_id2: 0x001c_c184,
        cddb_id: 0x1911_7f03,
    }
}

fn request(query: &DbarQuery, maximum_response_bytes: u64) -> ProviderRequest {
    ProviderRequest {
        collection_id: CollectionId::new(),
        provider: ModuleId(ACCURATERIP_PROVIDER_ID.into()),
        operation: OperationId::new(ACCURATERIP_ACQUIRE_OPERATION).unwrap(),
        source_fingerprint: None,
        query: RawJson::from_value(serde_json::to_value(query).unwrap()),
        maximum_response_bytes,
    }
}

fn dbar(checksum_1: u32, checksum_2: u32) -> Vec<u8> {
    let mut body = vec![2];
    body.extend_from_slice(&0x0008_4264_u32.to_le_bytes());
    body.extend_from_slice(&0x001c_c184_u32.to_le_bytes());
    body.extend_from_slice(&0x1911_7f03_u32.to_le_bytes());
    for (confidence, checksum) in [(7, checksum_1), (9, checksum_2)] {
        body.push(confidence);
        body.extend_from_slice(&checksum.to_le_bytes());
        body.extend_from_slice(&0_u32.to_le_bytes());
    }
    body
}

struct FixtureServer {
    origin: String,
    requests: Arc<AtomicUsize>,
    paths: Arc<Mutex<Vec<String>>>,
    join: Option<thread::JoinHandle<()>>,
}

impl Drop for FixtureServer {
    fn drop(&mut self) {
        if let Some(join) = self.join.take() {
            join.join().unwrap();
        }
    }
}

fn serve(responses: Vec<(Duration, String, Vec<u8>)>) -> FixtureServer {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let requests = Arc::new(AtomicUsize::new(0));
    let request_count = Arc::clone(&requests);
    let paths = Arc::new(Mutex::new(Vec::new()));
    let request_paths = Arc::clone(&paths);
    let join = thread::spawn(move || {
        for (delay, head, body) in responses {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0_u8; 4096];
            let read = stream.read(&mut request).unwrap();
            request_count.fetch_add(1, Ordering::SeqCst);
            let path = String::from_utf8_lossy(&request[..read])
                .lines()
                .next()
                .and_then(|line| line.split_whitespace().nth(1))
                .unwrap()
                .to_owned();
            request_paths.lock().unwrap().push(path);
            thread::sleep(delay);
            let response = format!(
                "HTTP/1.1 {head}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.write_all(&body);
        }
    });
    FixtureServer {
        origin: format!("http://{address}"),
        requests,
        paths,
        join: Some(join),
    }
}

#[test]
fn known_match_response_is_exact_bounded_and_replayable_offline() {
    let body = dbar(0x1111_1111, 0x2222_2222);
    let server = serve(vec![(Duration::ZERO, "200 OK".into(), body.clone())]);
    let provider = AccurateRipProvider::with_test_origin(
        "junk-collector-test/0.1",
        &server.origin,
        Duration::from_secs(1),
    )
    .unwrap();
    let response = provider
        .acquire(&request(&query(), 1024), &mut Execution::active())
        .unwrap();

    assert_eq!(response.bytes, body);
    assert_eq!(response.digest, Digest::sha256(&response.bytes));
    let observation: DbarSnapshotObservation = response.parsed_observation.parse().unwrap();
    assert_eq!(observation.outcome, DbarOutcome::Data);
    assert_eq!(observation.response_count, 1);
    assert_eq!(
        server.paths.lock().unwrap()[0],
        "/accuraterip/4/6/2/dBAR-002-00084264-001cc184-19117f03.bin"
    );
    let parsed = parse_frozen_dbar(&response).unwrap().unwrap();
    assert_eq!(parsed.responses[0].tracks[1].checksum, 0x2222_2222);

    let record = SnapshotRecord {
        id: SnapshotId::new(),
        collection_id: CollectionId::new(),
        provider: ModuleId(ACCURATERIP_PROVIDER_ID.into()),
        media_type: response.media_type.clone(),
        bytes: ByteLength(response.bytes.len() as u64),
        digest: response.digest.clone(),
        parsed_observation: response.parsed_observation.clone(),
        created_at: "2026-09-14T00:00:00Z".parse().unwrap(),
    };
    assert!(
        parse_dbar_snapshot(&record, &mut response.bytes.as_slice(), &mut || Ok(false))
            .unwrap()
            .is_some()
    );
    let mut changed = response.bytes.clone();
    *changed.last_mut().unwrap() ^= 1;
    assert!(parse_dbar_snapshot(&record, &mut changed.as_slice(), &mut || Ok(false)).is_err());
    let mut truncated = &response.bytes[..response.bytes.len() - 1];
    assert!(parse_dbar_snapshot(&record, &mut truncated, &mut || Ok(false)).is_err());
    let mut trailing = response.bytes.clone();
    trailing.push(0);
    assert!(parse_dbar_snapshot(&record, &mut trailing.as_slice(), &mut || Ok(false)).is_err());
    assert!(matches!(
        parse_dbar_snapshot(&record, &mut response.bytes.as_slice(), &mut || Ok(true)),
        Err(collector_core::Error::JobCancelled)
    ));
}

#[test]
fn no_data_is_distinct_from_malformed_unavailable_and_oversize() {
    let not_found = serve(vec![(Duration::ZERO, "404 Not Found".into(), vec![])]);
    let provider = AccurateRipProvider::with_test_origin(
        "junk-collector-test/0.1",
        &not_found.origin,
        Duration::from_secs(1),
    )
    .unwrap();
    let response = provider
        .acquire(&request(&query(), 1024), &mut Execution::active())
        .unwrap();
    let observation: DbarSnapshotObservation = response.parsed_observation.parse().unwrap();
    assert_eq!(observation.outcome, DbarOutcome::NoData);
    assert!(parse_frozen_dbar(&response).unwrap().is_none());

    let malformed = serve(vec![(Duration::ZERO, "200 OK".into(), vec![2, 0, 0])]);
    let provider = AccurateRipProvider::with_test_origin(
        "junk-collector-test/0.1",
        &malformed.origin,
        Duration::from_secs(1),
    )
    .unwrap();
    assert!(matches!(
        provider.acquire(&request(&query(), 1024), &mut Execution::active()),
        Err(collector_core::Error::Unavailable(_))
    ));

    let mut wrong_identity = dbar(0x1111_1111, 0x2222_2222);
    wrong_identity[1..5].copy_from_slice(&0x0008_4265_u32.to_le_bytes());
    let misdirected = serve(vec![(Duration::ZERO, "200 OK".into(), wrong_identity)]);
    let provider = AccurateRipProvider::with_test_origin(
        "junk-collector-test/0.1",
        &misdirected.origin,
        Duration::from_secs(1),
    )
    .unwrap();
    assert!(matches!(
        provider.acquire(&request(&query(), 1024), &mut Execution::active()),
        Err(collector_core::Error::Unavailable(_))
    ));

    let oversized = serve(vec![(
        Duration::ZERO,
        "200 OK".into(),
        dbar(0x1111_1111, 0x2222_2222),
    )]);
    let provider = AccurateRipProvider::with_test_origin(
        "junk-collector-test/0.1",
        &oversized.origin,
        Duration::from_secs(1),
    )
    .unwrap();
    assert!(matches!(
        provider.acquire(&request(&query(), 16), &mut Execution::active()),
        Err(collector_core::Error::Unavailable(_))
    ));
}

#[test]
fn timeout_and_transient_outage_are_bounded_to_three_attempts() {
    let responses = (0..3)
        .map(|_| (Duration::from_millis(80), "200 OK".into(), dbar(1, 2)))
        .collect();
    let server = serve(responses);
    let provider = AccurateRipProvider::with_test_origin(
        "junk-collector-test/0.1",
        &server.origin,
        Duration::from_millis(20),
    )
    .unwrap();
    assert!(matches!(
        provider.acquire(&request(&query(), 1024), &mut Execution::active()),
        Err(collector_core::Error::Unavailable(_))
    ));
    assert_eq!(server.requests.load(Ordering::SeqCst), 3);
}

#[test]
fn cancellation_stops_before_success_or_retry_publication() {
    let server = serve(vec![(
        Duration::ZERO,
        "503 Unavailable\r\nRetry-After: 1".into(),
        vec![],
    )]);
    let provider = AccurateRipProvider::with_test_origin(
        "junk-collector-test/0.1",
        &server.origin,
        Duration::from_secs(1),
    )
    .unwrap();
    let mut execution = Execution {
        cancel_after: Some(6),
        checks: AtomicUsize::new(0),
    };
    assert!(matches!(
        provider.acquire(&request(&query(), 1024), &mut execution),
        Err(collector_core::Error::JobCancelled)
    ));
    assert_eq!(server.requests.load(Ordering::SeqCst), 1);
}

#[test]
fn frozen_cache_survives_outage_and_later_refresh_is_distinct() {
    let first_body = dbar(0x1111_1111, 0x2222_2222);
    let first = serve(vec![(Duration::ZERO, "200 OK".into(), first_body)]);
    let provider = AccurateRipProvider::with_test_origin(
        "junk-collector-test/0.1",
        &first.origin,
        Duration::from_secs(1),
    )
    .unwrap();
    let frozen = provider
        .acquire(&request(&query(), 1024), &mut Execution::active())
        .unwrap();
    drop(first);

    assert!(matches!(
        provider.acquire(&request(&query(), 1024), &mut Execution::active()),
        Err(collector_core::Error::Unavailable(_))
    ));
    assert!(parse_frozen_dbar(&frozen).unwrap().is_some());

    let second_body = dbar(0x3333_3333, 0x4444_4444);
    let second = serve(vec![(Duration::ZERO, "200 OK".into(), second_body)]);
    let refreshed_provider = AccurateRipProvider::with_test_origin(
        "junk-collector-test/0.1",
        &second.origin,
        Duration::from_secs(1),
    )
    .unwrap();
    let refreshed = refreshed_provider
        .acquire(&request(&query(), 1024), &mut Execution::active())
        .unwrap();
    assert_ne!(frozen.digest, refreshed.digest);
    assert_eq!(
        parse_frozen_dbar(&frozen).unwrap().unwrap().responses[0].tracks[0].checksum,
        0x1111_1111
    );
    assert_eq!(
        parse_frozen_dbar(&refreshed).unwrap().unwrap().responses[0].tracks[0].checksum,
        0x3333_3333
    );
}

#[test]
fn redirect_policy_rejects_other_origins_and_supports_same_origin_redirects() {
    let server = serve(vec![
        (
            Duration::ZERO,
            "302 Found\r\nLocation: /redirected.bin".into(),
            vec![],
        ),
        (Duration::ZERO, "200 OK".into(), dbar(1, 2)),
    ]);
    let provider =
        AccurateRipProvider::with_test_origin("test/1", &server.origin, Duration::from_secs(1))
            .unwrap();
    assert!(
        provider
            .acquire(&request(&query(), 1024), &mut Execution::active())
            .is_ok()
    );
    assert_eq!(server.paths.lock().unwrap()[1], "/redirected.bin");
    for location in [
        "http://192.0.2.1/data",
        "http://user:password@127.0.0.1/data",
    ] {
        let server = serve(
            (0..3)
                .map(|_| {
                    (
                        Duration::ZERO,
                        format!("302 Found\r\nLocation: {location}"),
                        vec![],
                    )
                })
                .collect(),
        );
        let provider =
            AccurateRipProvider::with_test_origin("test/1", &server.origin, Duration::from_secs(1))
                .unwrap();
        assert!(
            provider
                .acquire(&request(&query(), 1024), &mut Execution::active())
                .is_err()
        );
    }
    assert!(
        AccurateRipProvider::with_test_origin(
            "test/1",
            "http://[::1]:1234",
            Duration::from_secs(1)
        )
        .is_ok()
    );
}
