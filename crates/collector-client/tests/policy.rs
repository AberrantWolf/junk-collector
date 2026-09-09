use collector_client::*;
use collector_core::*;
#[test]
fn direct_lan_wins_and_same_class_hysteresis_prevents_flapping() {
    let id = ServerId::new();
    let local = Endpoint {
        url: "https://collector.local:9843".parse().unwrap(),
        route: Route::Lan,
        latency_ms: Some(4),
        healthy: true,
    };
    let public = Endpoint {
        url: "https://collector.example.com".parse().unwrap(),
        route: Route::Public,
        latency_ms: Some(1),
        healthy: true,
    };
    let mut pairing = Pairing {
        server_id: id,
        certificate_pem: String::new(),
        certificate_sha256: String::new(),
        endpoints: vec![public.clone(), local.clone()],
    };
    assert_eq!(
        pairing.best_endpoint(Some(&public.url)).unwrap().url,
        local.url
    );
    assert!(
        pairing
            .add_verified_endpoint(ServerId::new(), public)
            .is_err()
    );
    pairing.endpoints[1].healthy = false;
    assert_eq!(pairing.best_endpoint(None).unwrap().route, Route::Public);
}
#[test]
fn pinned_offline_content_survives_eviction() {
    let server = ServerId::new();
    let pinned = CacheEntry {
        server_id: server,
        representation_id: RepresentationId::new(),
        profile: ConversionProfile::Flac,
        bytes: 100,
        pinned: true,
        last_used: 0,
        complete: true,
    };
    let other = CacheEntry {
        pinned: false,
        representation_id: RepresentationId::new(),
        ..pinned.clone()
    };
    assert_eq!(
        eviction_candidates(&[pinned.clone(), other.clone()], 50),
        vec![other.key()]
    );
}

#[test]
fn cache_identity_includes_server_and_encoding_profile() {
    let base = CacheEntry {
        server_id: ServerId::new(),
        representation_id: RepresentationId::new(),
        profile: ConversionProfile::Flac,
        bytes: 100,
        pinned: true,
        last_used: 0,
        complete: true,
    };
    let lossy = CacheEntry {
        profile: ConversionProfile::Opus { bitrate_kbps: 128 },
        pinned: false,
        ..base.clone()
    };
    let remote = CacheEntry {
        server_id: ServerId::new(),
        pinned: false,
        ..base.clone()
    };
    let evicted = eviction_candidates(&[base.clone(), lossy.clone(), remote.clone()], 100);
    assert_eq!(evicted, vec![lossy.key(), remote.key()]);
    assert!(!evicted.contains(&base.key()));
}
