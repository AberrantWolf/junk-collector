use collector_client::Pairing;
use collector_core::*;
use collector_server::{AppState, router};
use collector_store::{Store, UploadFile};
use std::sync::Arc;
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
    let handle = axum_server::Handle::new();
    let stop = handle.clone();
    let id = store.server_id().unwrap();
    let pem = cert.cert.pem();
    let pairing = Pairing {
        server_id: id,
        certificate_sha256: Digest::sha256(pem.as_bytes()).hex,
        certificate_pem: pem,
        endpoints: vec![],
    };
    let state = AppState::new(store.clone(), pairing.clone());
    let collection = state
        .service
        .create_collection(&owner, "Local archive")
        .unwrap();
    let item = Item {
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
    let item = state.service.save(&owner, item, Revision(0)).unwrap();
    let data = b"0123456789";
    let upload = store
        .create_upload(
            &owner,
            item.id,
            item.revision,
            vec![UploadFile {
                name: "original.bin".into(),
                bytes: ByteLength(data.len() as u64),
                sha256: None,
            }],
        )
        .unwrap();
    store.upload_chunk(&owner, upload.id, 0, 0, data).unwrap();
    store.commit_upload(&owner, upload.id).unwrap();
    let app = router(state);
    let task = tokio::spawn(async move {
        axum_server::from_tcp_rustls(listener, config)
            .unwrap()
            .handle(handle)
            .serve(app.into_make_service())
            .await
            .unwrap();
    });
    let client = pairing.lan_client().unwrap();
    let base = format!("https://{address}");
    let response = client
        .get(format!("{base}/api/v1/identity"))
        .send()
        .await
        .unwrap();
    assert_eq!(
        response.json::<serde_json::Value>().await.unwrap()["server_id"],
        id.to_string()
    );
    let login=client.post(format!("{base}/api/v1/auth/login")).header("x-collector-client","native").json(&serde_json::json!({"username":"owner","password":"long test password","device_name":"LAN test"})).send().await.unwrap().json::<serde_json::Value>().await.unwrap();
    let token = login["token"].as_str().unwrap();
    let response = client
        .get(format!(
            "{base}/api/v1/items/{}/representations/{}/0",
            item.id, upload.representation_id
        ))
        .bearer_auth(token)
        .header("range", "bytes=2-5")
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest_status_partial());
    assert_eq!(response.bytes().await.unwrap().as_ref(), b"2345");
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
    stop.shutdown();
    task.await.unwrap();
}
fn reqwest_status_partial() -> u16 {
    206
}
