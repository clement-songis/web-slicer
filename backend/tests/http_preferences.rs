//! Préférences de l'interface par compte (GET/PUT /api/me/preferences) :
//! défaut, remplacement, isolation, validation, authentification.

use axum::body::{to_bytes, Body};
use axum::http::{header, Request, StatusCode};
use axum::response::Response;
use axum::Router;
use backend::adapters::files::FileStore;
use backend::adapters::storage::AnyStorage;
use backend::http::routes::router;
use backend::http::state::AppState;
use serde_json::json;
use std::sync::Arc;
use tower::ServiceExt;
use tower_sessions::{MemoryStore, SessionManagerLayer};

async fn app() -> (tempfile::TempDir, Router) {
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}", dir.path().join("c.db").display());
    let storage = Arc::new(AnyStorage::connect_test(&url).await.unwrap());
    let state = AppState::new(storage, FileStore::new(dir.path().join("data")));
    (
        dir,
        router(
            state,
            SessionManagerLayer::new(MemoryStore::default()).with_secure(false),
        ),
    )
}

fn request(
    method: &str,
    uri: &str,
    body: serde_json::Value,
    cookie: Option<&str>,
) -> Request<Body> {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(c) = cookie {
        builder = builder.header(header::COOKIE, c);
    }
    builder.body(Body::from(body.to_string())).unwrap()
}

fn session_cookie(resp: &Response) -> String {
    resp.headers()
        .get(header::SET_COOKIE)
        .expect("Set-Cookie présent")
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_string()
}

async fn json_body(resp: Response) -> serde_json::Value {
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    if bytes.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    }
}

async fn register(app: &Router, email: &str) -> String {
    let resp = app
        .clone()
        .oneshot(request(
            "POST",
            "/api/auth/register",
            serde_json::json!({ "email": email, "password": "password123" }),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    session_cookie(&resp)
}

#[tokio::test]
async fn preferences_are_per_account_and_validated() {
    let (_dir, app) = app().await;
    let resp = app
        .clone()
        .oneshot(request("GET", "/api/me/preferences", json!(null), None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let a = register(&app, "prefs-a@example.com").await;
    let b = register(&app, "prefs-b@example.com").await;

    let get = |cookie: String| {
        let app = app.clone();
        async move {
            let resp = app
                .oneshot(request(
                    "GET",
                    "/api/me/preferences",
                    json!(null),
                    Some(&cookie),
                ))
                .await
                .unwrap();
            assert_eq!(resp.status(), StatusCode::OK);
            json_body(resp).await
        }
    };
    assert_eq!(get(a.clone()).await, json!({}));

    let prefs =
        json!({ "camera": { "style": "free", "zoom_to_mouse": true }, "auto_arrange": false });
    let resp = app
        .clone()
        .oneshot(request(
            "PUT",
            "/api/me/preferences",
            prefs.clone(),
            Some(&a),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(json_body(resp).await, prefs);
    assert_eq!(get(a.clone()).await, prefs);
    assert_eq!(get(b.clone()).await, json!({}), "isolation entre comptes");

    // Pas un objet → 422 ; trop gros → 413 ; rien n'est modifié.
    let resp = app
        .clone()
        .oneshot(request(
            "PUT",
            "/api/me/preferences",
            json!([1, 2]),
            Some(&a),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let big = json!({ "blob": "x".repeat(70 * 1024) });
    let resp = app
        .clone()
        .oneshot(request("PUT", "/api/me/preferences", big, Some(&a)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(get(a.clone()).await, prefs);
}
