use collector_app::{AuthenticatedPrincipal, CreateCollectionCommand, CredentialKind};
use collector_client::{
    ConnectionKey, Endpoint, EnrolledClient, EnrolledCredential, NativeCredentialStorage, Pairing,
    PendingEnrollment, Route,
};
use collector_core::*;
use collector_server::{AppState, router};
use collector_store::{Store, UploadFile};
use std::sync::Arc;

#[derive(Default)]
struct TestCredentialStorage(std::sync::Mutex<Option<EnrolledCredential>>);

impl NativeCredentialStorage for TestCredentialStorage {
    fn save(&self, credential: &EnrolledCredential) -> Result<()> {
        *self.0.lock().unwrap() = Some(credential.clone());
        Ok(())
    }

    fn load(&self, key: ConnectionKey) -> Result<Option<EnrolledCredential>> {
        Ok(self.0.lock().unwrap().clone().filter(|credential| {
            credential.server_id == key.server_id && credential.user_id == Some(key.user_id)
        }))
    }

    fn load_legacy(&self, _: ServerId) -> Result<Option<EnrolledCredential>> {
        Ok(None)
    }
    fn remove_legacy(&self, _: ServerId) -> Result<()> {
        Ok(())
    }
    fn remove(&self, key: ConnectionKey) -> Result<()> {
        let mut stored = self.0.lock().unwrap();
        if stored.as_ref().is_some_and(|value| {
            value.server_id == key.server_id && value.user_id == Some(key.user_id)
        }) {
            *stored = None;
        }
        Ok(())
    }
}
fn command() -> collector_app::CommandContext {
    collector_app::CommandContext::new(TransactionId::new().to_string()).unwrap()
}
fn typed_collection(name: &str) -> CreateCollectionCommand {
    CreateCollectionCommand {
        name: name.into(),
        module: ModuleId("org.junk.games".into()),
    }
}
fn test_authentication(principal: &Principal) -> AuthenticatedPrincipal {
    AuthenticatedPrincipal {
        principal: principal.clone(),
        credential_kind: CredentialKind::TestLocal,
        authenticated_at: chrono::Utc::now(),
    }
}
#[tokio::test]
async fn paired_lan_authentication_and_ranged_download_work_without_a_public_route() {
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    let root = tempfile::tempdir().unwrap();
    let store =
        Arc::new(Store::open(root.path().join("archive"), root.path().join("state")).unwrap());
    let owner = Principal {
        user_id: store
            .create_user("owner", "long test password", true)
            .unwrap(),
        administrator: true,
        device_id: None,
    };
    let cert =
        rcgen::generate_simple_self_signed(vec!["localhost".into(), "127.0.0.1".into()]).unwrap();
    let config = axum_server::tls_rustls::RustlsConfig::from_pem(
        cert.cert.pem().into_bytes(),
        cert.key_pair.serialize_pem().into_bytes(),
    )
    .await
    .unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let base = format!("https://{address}");
    let handle = axum_server::Handle::new();
    let stop = handle.clone();
    let id = store.server_id().unwrap();
    let pem = cert.cert.pem();
    let pairing = Pairing {
        server_id: id,
        certificate_sha256: Digest::sha256(pem.as_bytes()).hex,
        certificate_pem: pem,
        endpoints: vec![Endpoint {
            url: base.parse().unwrap(),
            route: Route::Lan,
            latency_ms: None,
            healthy: true,
        }],
    };
    let mut state = AppState::new(store.clone(), pairing.clone());
    state.ingress = Ingress::Private;
    let browser_origin = format!("https://{address}");
    state.allowed_origins = Arc::new(vec![browser_origin.clone()]);
    let collection = state
        .service
        .create_collection(
            &test_authentication(&owner),
            typed_collection("Local archive"),
            &command(),
        )
        .unwrap();
    let item = Item {
        lifecycle: Default::default(),
        id: ItemId::new(),
        collection_id: collection.id,
        revision: Revision(0),
        title: "Offline LAN fixture".into(),
        subtitle: String::new(),
        identity_link: ItemIdentityLink::None,
        domain: ModuleData {
            module: ModuleId("org.junk.games".into()),
            schema_version: 1,
            payload: RawJson::from_value(serde_json::json!({})),
        },
        identity: IdentityState::Unidentified,
        translations: vec![],
        identifiers: vec![],
        specimens: vec![],
        wishlist: false,
        captures: vec![],
        assets: vec![],
        metadata: MetadataState::default(),
        deleted: false,
        updated_at: chrono::Utc::now(),
    };
    let item = state
        .service
        .save_item(&owner, item.id, item.into(), Revision(0), &command())
        .unwrap();
    let data = b"0123456789";
    let upload = state
        .service
        .create_upload(
            &owner,
            item.id,
            item.revision,
            vec![UploadFile {
                visibility: Default::default(),
                name: "original.bin".into(),
                bytes: ByteLength(data.len() as u64),
                sha256: None,
            }],
            &command(),
        )
        .unwrap();
    state
        .service
        .upload_chunk(&owner, upload.id, 0, 0, data)
        .unwrap();
    state
        .service
        .commit_upload(&owner, upload.id, &command())
        .unwrap();
    let app = router(state);
    let task = tokio::spawn(async move {
        axum_server::from_tcp_rustls(listener, config)
            .unwrap()
            .handle(handle)
            .serve(app.into_make_service())
            .await
            .unwrap();
    });
    // Normal HTTPS must reject a freshly generated, untrusted self-signed identity.
    let system_client = collector_client::system_https_client().unwrap();
    assert!(matches!(
        PendingEnrollment::start_system_https(
            &system_client,
            base.parse().unwrap(),
            "Untrusted test"
        )
        .await,
        Err(Error::Unavailable(_))
    ));
    let client = pairing.lan_client().unwrap();
    let response = client
        .get(format!("{base}/api/v1/identity"))
        .send()
        .await
        .unwrap();
    assert_eq!(
        response.json::<serde_json::Value>().await.unwrap()["server_id"],
        id.to_string()
    );
    let pending = PendingEnrollment::start(&client, base.parse().unwrap(), "LAN test")
        .await
        .unwrap();
    let approval_code = pending.start.user_code.clone();
    let login = client
        .post(format!("{base}/api/v1/auth/login"))
        .header("origin", &browser_origin)
        .json(&serde_json::json!({"username":"owner","password":"long test password","device_name":"LAN test"}))
        .send()
        .await
        .unwrap();
    assert!(login.status().is_success());
    let cookie = login
        .headers()
        .get("set-cookie")
        .unwrap()
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    let approval = client
        .post(format!("{base}/api/v1/enrollments/approve"))
        .header("origin", &browser_origin)
        .header("cookie", cookie)
        .json(&serde_json::json!({"user_code": approval_code}))
        .send()
        .await
        .unwrap();
    assert!(approval.status().is_success());
    let storage = TestCredentialStorage::default();
    let enrolled = pending.complete(&client).await.unwrap();
    storage.save(&enrolled).unwrap();
    let connection_key = ConnectionKey {
        server_id: id,
        user_id: enrolled.user_id.unwrap(),
    };
    assert_eq!(enrolled.server_id, id);
    let native = EnrolledClient::from_storage(&storage, connection_key).unwrap();
    assert!(
        native
            .get("https://attacker.example/data".parse().unwrap())
            .is_err()
    );
    let response = native
        .get(
            format!(
                "{base}/api/v1/items/{}/representations/{}/0",
                item.id, upload.representation_id
            )
            .parse()
            .unwrap(),
        )
        .unwrap()
        .header("range", "bytes=2-5")
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest_status_partial());
    assert_eq!(response.bytes().await.unwrap().as_ref(), b"2345");
    // Exercise the actual blocking decoder adapter against the trusted listener.
    let saved = enrolled.clone();
    let item_id = item.id;
    let representation = upload.representation_id;
    tokio::task::spawn_blocking(move || {
        use std::io::{Read, Seek, SeekFrom};
        let storage = TestCredentialStorage(std::sync::Mutex::new(Some(saved)));
        let client = EnrolledClient::from_storage(&storage, connection_key).unwrap();
        let cancellation = collector_client::MediaCancellation::default();
        let mut reader = client
            .media_reader(item_id, representation, 0, cancellation.clone())
            .unwrap();
        assert_eq!(reader.length(), 10);
        reader.seek(SeekFrom::Start(2)).unwrap();
        let mut bytes = [0; 4];
        reader.read_exact(&mut bytes).unwrap();
        assert_eq!(&bytes, b"2345");
        reader.seek(SeekFrom::End(-2)).unwrap();
        let mut last = Vec::new();
        reader.read_to_end(&mut last).unwrap();
        assert_eq!(last, b"89");
        assert!(reader.seek(SeekFrom::Start(11)).is_err());
        cancellation.cancel();
        assert_eq!(
            reader.read(&mut bytes).unwrap_err().kind(),
            std::io::ErrorKind::Interrupted
        );
    })
    .await
    .unwrap();
    let list_input = collector_client::collector_api_types::CreateCollectionList {
        name: "LAN playlist".into(),
    };
    let key = TransactionId::new().to_string();
    let created = native
        .create_list(collection.id, &list_input, &key)
        .await
        .unwrap();
    let replay = native
        .create_list(collection.id, &list_input, &key)
        .await
        .unwrap();
    assert_eq!(created.id, replay.id);
    let snapshot = native
        .list_snapshot(collection.id, created.id)
        .await
        .unwrap();
    assert_eq!(snapshot.list.name, "LAN playlist");
    let rename = collector_client::collector_api_types::UpdateCollectionList {
        expected_revision: created.revision,
        change: collector_client::collector_api_types::CollectionListChange::Rename {
            name: "Renamed".into(),
        },
    };
    let updated = native
        .update_list(
            collection.id,
            created.id,
            &rename,
            &TransactionId::new().to_string(),
        )
        .await
        .unwrap();
    assert_eq!(updated.name, "Renamed");
    assert!(matches!(
        native
            .update_list(
                collection.id,
                created.id,
                &rename,
                &TransactionId::new().to_string()
            )
            .await,
        Err(Error::Conflict)
    ));
    let unrelated = rcgen::generate_simple_self_signed(vec!["127.0.0.1".into()])
        .unwrap()
        .cert
        .pem();
    let wrong = Pairing {
        certificate_sha256: Digest::sha256(unrelated.as_bytes()).hex,
        certificate_pem: unrelated,
        ..pairing
    };
    assert!(
        wrong
            .lan_client()
            .unwrap()
            .get(format!("{base}/api/v1/identity"))
            .send()
            .await
            .is_err()
    );
    // A reader admitted before revocation must fail on its next uncached read.
    let device = store
        .native_devices(owner.user_id)
        .unwrap()
        .into_iter()
        .find(|device| device.id == enrolled.credential_id)
        .unwrap();
    let saved = enrolled.clone();
    tokio::task::spawn_blocking(move || {
        use std::io::Read;
        let storage = TestCredentialStorage(std::sync::Mutex::new(Some(saved)));
        let client = EnrolledClient::from_storage(&storage, connection_key).unwrap();
        let mut reader = client
            .media_reader(
                item_id,
                representation,
                0,
                collector_client::MediaCancellation::default(),
            )
            .unwrap();
        store
            .revoke_native_device(owner.user_id, device.id, device.revision)
            .unwrap();
        assert_eq!(
            reader.read(&mut [0; 1]).unwrap_err().kind(),
            std::io::ErrorKind::PermissionDenied
        );
        assert!(matches!(
            client.media_reader(
                item_id,
                representation,
                0,
                collector_client::MediaCancellation::default()
            ),
            Err(Error::CredentialRevoked)
        ));
    })
    .await
    .unwrap();
    stop.shutdown();
    task.await.unwrap();
}
fn reqwest_status_partial() -> u16 {
    206
}
