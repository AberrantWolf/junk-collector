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
        user_id: UserId::new(),
        server_id: server,
        representation_id: RepresentationId::new(),
        file_id: PackageFileId::new(),
        digest: Digest::sha256(b"cache fixture"),
        profile: Some(ConversionProfile::Flac),
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
        user_id: UserId::new(),
        server_id: ServerId::new(),
        representation_id: RepresentationId::new(),
        file_id: PackageFileId::new(),
        digest: Digest::sha256(b"cache fixture"),
        profile: Some(ConversionProfile::Flac),
        bytes: 100,
        pinned: true,
        last_used: 0,
        complete: true,
    };
    let lossy = CacheEntry {
        profile: Some(ConversionProfile::Opus { bitrate_kbps: 128 }),
        pinned: false,
        ..base.clone()
    };
    let remote = CacheEntry {
        server_id: ServerId::new(),
        pinned: false,
        ..base.clone()
    };
    let other_file = CacheEntry {
        file_id: PackageFileId::new(),
        profile: None,
        ..base.clone()
    };
    let replacement = CacheEntry {
        digest: Digest::sha256(b"replacement"),
        ..base.clone()
    };
    let other_user = CacheEntry {
        user_id: UserId::new(),
        ..base.clone()
    };
    assert_ne!(base.key(), other_user.key());
    assert_ne!(base.key(), other_file.key());
    assert_ne!(base.key(), replacement.key());
    let evicted = eviction_candidates(&[base.clone(), lossy.clone(), remote.clone()], 100);
    assert_eq!(evicted, vec![lossy.key(), remote.key()]);
    assert!(!evicted.contains(&base.key()));
}

#[test]
fn credential_storage_cannot_return_another_server_or_expired_authority() {
    use collector_client::{EnrolledClient, EnrolledCredential, NativeCredentialStorage};
    use collector_core::{Error, NativeCredentialId, Result};
    struct Stored(EnrolledCredential);
    impl NativeCredentialStorage for Stored {
        fn save(&self, _: &EnrolledCredential) -> Result<()> {
            Ok(())
        }
        fn load(&self, _: ConnectionKey) -> Result<Option<EnrolledCredential>> {
            Ok(Some(self.0.clone()))
        }
        fn load_legacy(&self, _: ServerId) -> Result<Option<EnrolledCredential>> {
            Ok(None)
        }
        fn remove_legacy(&self, _: ServerId) -> Result<()> {
            Ok(())
        }
        fn remove(&self, _: ConnectionKey) -> Result<()> {
            Ok(())
        }
    }
    let server = ServerId::new();
    let user = UserId::new();
    let key = ConnectionKey {
        server_id: server,
        user_id: user,
    };
    let mut credential = EnrolledCredential {
        user_id: Some(user),
        username: "owner".into(),
        display_name: "Owner".into(),
        trust: Default::default(),
        server_id: server,
        pairing: Pairing {
            server_id: server,
            certificate_pem: String::new(),
            certificate_sha256: String::new(),
            endpoints: vec![],
        },
        credential_id: NativeCredentialId::new(),
        credential: "test-only-secret".into(),
        expires_at: chrono::Utc::now() + chrono::Duration::hours(1),
    };
    assert!(matches!(
        EnrolledClient::from_storage(
            &Stored(credential.clone()),
            ConnectionKey {
                server_id: ServerId::new(),
                user_id: user
            }
        ),
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        EnrolledClient::from_storage(
            &Stored(credential.clone()),
            ConnectionKey {
                server_id: server,
                user_id: UserId::new()
            }
        ),
        Err(Error::Invalid(_))
    ));
    credential.pairing.server_id = ServerId::new();
    assert!(matches!(
        EnrolledClient::from_storage(&Stored(credential.clone()), key),
        Err(Error::Invalid(_))
    ));
    credential.pairing.server_id = server;
    credential.expires_at = chrono::Utc::now() - chrono::Duration::seconds(1);
    assert!(matches!(
        EnrolledClient::from_storage(&Stored(credential), key),
        Err(Error::CredentialRevoked)
    ));
}

#[test]
fn system_https_credentials_reconnect_without_a_local_certificate_and_fence_origins() {
    struct Stored(EnrolledCredential);
    impl NativeCredentialStorage for Stored {
        fn save(&self, _: &EnrolledCredential) -> Result<()> {
            Ok(())
        }
        fn load(&self, _: ConnectionKey) -> Result<Option<EnrolledCredential>> {
            Ok(Some(self.0.clone()))
        }
        fn load_legacy(&self, _: ServerId) -> Result<Option<EnrolledCredential>> {
            Ok(None)
        }
        fn remove_legacy(&self, _: ServerId) -> Result<()> {
            Ok(())
        }
        fn remove(&self, _: ConnectionKey) -> Result<()> {
            Ok(())
        }
    }
    let server = ServerId::new();
    let user = UserId::new();
    let key = ConnectionKey {
        server_id: server,
        user_id: user,
    };
    let stored = Stored(EnrolledCredential {
        server_id: server,
        user_id: Some(user),
        username: "owner".into(),
        display_name: "Owner".into(),
        credential_id: NativeCredentialId::new(),
        credential: "test-only-secret".into(),
        expires_at: chrono::Utc::now() + chrono::Duration::hours(1),
        trust: collector_client::ConnectionTrust::SystemHttps,
        pairing: Pairing {
            server_id: server,
            certificate_pem: String::new(),
            certificate_sha256: String::new(),
            endpoints: vec![Endpoint {
                url: "https://collector.example/".parse().unwrap(),
                route: Route::Public,
                latency_ms: None,
                healthy: true,
            }],
        },
    });
    let client = EnrolledClient::from_storage(&stored, key).unwrap();
    assert!(
        client
            .get("https://collector.example/api/v1/me".parse().unwrap())
            .is_ok()
    );
    for url in [
        "http://collector.example/api/v1/me",
        "https://attacker.example/api/v1/me",
        "https://collector.example:8443/api/v1/me",
        "https://user@collector.example/api/v1/me",
    ] {
        assert!(client.get(url.parse().unwrap()).is_err(), "{url}");
    }
}
