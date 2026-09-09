use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use collector_client::Pairing;
use collector_core::*;
use collector_server::*;
use collector_store::Store;
use std::sync::Arc;
use tower::ServiceExt;
fn state() -> (tempfile::TempDir, AppState) {
    let root = tempfile::tempdir().unwrap();
    let store =
        Arc::new(Store::open(root.path().join("archive"), root.path().join("state")).unwrap());
    let id = store.server_id().unwrap();
    let mut state = AppState::new(
        store,
        Pairing {
            server_id: id,
            certificate_pem: String::new(),
            certificate_sha256: String::new(),
            endpoints: vec![],
        },
    );
    state.allowed_origins = Arc::new(vec!["https://collector.example".into()]);
    (root, state)
}
#[tokio::test]
async fn public_surface_matrix_ignores_spoofed_proxy_headers() {
    for curation in [false, true] {
        for admin in [false, true] {
            let (_root, mut state) = state();
            state.exposure = Exposure {
                public_curation: curation,
                public_administration: admin,
            };
            let app = router(state);
            for (path, enabled) in [
                ("/curate", curation || admin),
                ("/curation.js", curation || admin),
                ("/admin", admin),
                ("/administration.js", admin),
            ] {
                let response = app
                    .clone()
                    .oneshot(
                        Request::builder()
                            .uri(path)
                            .header("x-forwarded-for", "127.0.0.1")
                            .header("forwarded", "for=127.0.0.1")
                            .body(Body::empty())
                            .unwrap(),
                    )
                    .await
                    .unwrap();
                assert_eq!(
                    response.status(),
                    if enabled {
                        StatusCode::OK
                    } else {
                        StatusCode::NOT_FOUND
                    },
                    "{path} curation={curation}, admin={admin}"
                );
            }
            let response = app
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/v1/admin/collections")
                        .header("content-type", "application/json")
                        .body(Body::from("{\"name\":\"Test\"}"))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(
                response.status(),
                if admin {
                    StatusCode::UNAUTHORIZED
                } else {
                    StatusCode::NOT_FOUND
                }
            );
        }
    }
}
#[tokio::test]
async fn private_surface_still_requires_authentication_and_csrf_origin() {
    let (_root, mut state) = state();
    state.ingress = Ingress::Private;
    let viewer = state
        .store
        .create_user("viewer", "a long viewer password", false)
        .unwrap();
    let (viewer_token, _) = state.store.issue_session(viewer, "browser").unwrap();
    let app = router(state);
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/admin/settings")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let body = axum::body::to_bytes(response.into_body(), 8192)
        .await
        .unwrap();
    let error: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(error["error"]["code"], "unauthenticated");
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/admin/settings")
                .header("authorization", format!("Bearer {viewer_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let body = axum::body::to_bytes(response.into_body(), 8192)
        .await
        .unwrap();
    let error: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(error["error"]["code"], "forbidden");
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header("origin", "https://attacker.example")
                .header("content-type", "application/json")
                .body(Body::from(
                    "{\"username\":\"owner\",\"password\":\"secret\"}",
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let body = axum::body::to_bytes(response.into_body(), 8192)
        .await
        .unwrap();
    let error: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(error["error"]["code"], "origin_not_allowed");
}
#[test]
fn range_boundaries() {
    assert_eq!(parse_range("bytes=0-0", 10), Some((0, 0)));
    assert_eq!(parse_range("bytes=5-", 10), Some((5, 9)));
    assert_eq!(parse_range("bytes=-3", 10), Some((7, 9)));
    assert_eq!(parse_range("bytes=0-99", 10), Some((0, 9)));
    assert_eq!(parse_range("bytes=10-", 10), None);
    assert_eq!(parse_range("bytes=0-0,2-2", 10), None);
    assert_eq!(parse_range("bytes=-0", 10), None);
    assert_eq!(parse_range("bytes=0-0", 0), None);
}

#[tokio::test]
async fn unavailable_module_is_preserved_and_presented_as_non_executable() {
    let (_root, mut state) = state();
    state.ingress = Ingress::Private;
    let user = state
        .store
        .create_user("owner", "a long owner password", true)
        .unwrap();
    let actor = Principal {
        user_id: user,
        administrator: true,
        device_id: None,
    };
    let collection = state
        .service
        .create_collection(&actor, "Unknown data")
        .unwrap();
    let item = Item {
        id: ItemId::new(),
        collection_id: collection.id,
        revision: Revision(0),
        title: "Opaque record".into(),
        subtitle: String::new(),
        identity_link: ItemIdentityLink::None,
        domain: ModuleData {
            module: ModuleId("org.example.absent".into()),
            schema_version: 8,
            payload: RawJson::from_json(r#"{"z":1e+02,"a":null}"#).unwrap(),
        },
        identity: IdentityState::Unidentified,
        translations: vec![],
        identifiers: vec![],
        specimens: vec![],
        wishlist: true,
        captures: vec![],
        assets: vec![],
        metadata: MetadataState::default(),
        deleted: false,
        updated_at: chrono::Utc::now(),
    };
    let saved = state.service.save(&actor, item, Revision(0)).unwrap();
    let (token, _) = state.store.issue_session(user, "test").unwrap();
    let response = router(state)
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/items/{}", saved.id))
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 64 * 1024)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["presentation"]["available"], false);
    assert!(
        body["presentation"]["limitations"][0]
            .as_str()
            .unwrap()
            .contains("not executable")
    );
    assert!(
        String::from_utf8(bytes.to_vec())
            .unwrap()
            .contains(r#""z":1e+02,"a":null"#)
    );
}
