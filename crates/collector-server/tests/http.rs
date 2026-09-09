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
