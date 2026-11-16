//! Ouverture d'un G-code local (menu « Open G-code… ») : méta + couches WSPv
//! sans stockage, refus d'un fichier vide/non G-code, authentification requise.

use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{header, Request, StatusCode};
use axum::response::Response;
use axum::Router;
use backend::adapters::files::FileStore;
use backend::adapters::storage::sqlite::SqliteStorage;
use backend::domain::Storage;
use serde_json::Value;
use tower::ServiceExt;
use tower_sessions::{MemoryStore, SessionManagerLayer};

const SAMPLE_GCODE: &str = "\
G1 X0 Y0 Z0.2 F1200
;LAYER_CHANGE
;Z:0.2
;HEIGHT:0.2
;WIDTH:0.45
;TYPE:Outer wall
G1 X10 Y0 E0.5 F1800
G1 X10 Y10 E1.0
;LAYER_CHANGE
;Z:0.4
;TYPE:Outer wall
G1 X0 Y0 E2.0 F1800
";

async fn app() -> (tempfile::TempDir, Router) {
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}", dir.path().join("open.db").display());
    let storage = Arc::new(SqliteStorage::connect(&url).await.unwrap());
    let state = backend::http::state::AppState::new(
        storage as Arc<dyn Storage>,
        FileStore::new(dir.path().join("data")),
    );
    let layer = SessionManagerLayer::new(MemoryStore::default()).with_secure(false);
    (dir, backend::http::routes::router(state, layer))
}

async fn send(app: &Router, req: Request<Body>) -> Response {
    app.clone().oneshot(req).await.unwrap()
}

async fn register(app: &Router) -> String {
    let resp = send(
        app,
        Request::builder()
            .method("POST")
            .uri("/api/auth/register")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                serde_json::json!({ "email": "gc@test.local", "password": "password123" })
                    .to_string(),
            ))
            .unwrap(),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    resp.headers()[header::SET_COOKIE]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_string()
}

fn post(uri: &str, cookie: Option<&str>, body: &str) -> Request<Body> {
    let mut builder = Request::builder()
        .method("POST")
        .uri(uri)
        .header(header::CONTENT_TYPE, "text/plain");
    if let Some(cookie) = cookie {
        builder = builder.header(header::COOKIE, cookie);
    }
    builder.body(Body::from(body.to_string())).unwrap()
}

#[tokio::test]
async fn local_gcode_preview_meta_and_layers() {
    let (_d, app) = app().await;
    let cookie = register(&app).await;

    let resp = send(
        &app,
        post("/api/gcode/preview?meta=1", Some(&cookie), SAMPLE_GCODE),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let meta: Value =
        serde_json::from_slice(&to_bytes(resp.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(meta["layer_count"], 2);

    let resp = send(
        &app,
        post(
            "/api/gcode/preview?from=0&to=1",
            Some(&cookie),
            SAMPLE_GCODE,
        ),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    assert_eq!(&bytes[..4], b"WSPv");
}

#[tokio::test]
async fn local_gcode_preview_rejects_empty_and_non_gcode() {
    let (_d, app) = app().await;
    let cookie = register(&app).await;
    for body in ["", "hello world\nnot gcode\n"] {
        let resp = send(&app, post("/api/gcode/preview?meta=1", Some(&cookie), body)).await;
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY, "{body:?}");
    }
    let resp = send(&app, post("/api/gcode/preview?meta=1", None, SAMPLE_GCODE)).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
