//! Endpoints d'authentification (T028) de bout en bout : register → session →
//! me, absence de session → 401, logout. Requêtes via `oneshot`, cookie de
//! session propagé manuellement.

use axum::body::{to_bytes, Body};
use axum::http::{header, Request, StatusCode};
use axum::Router;
use backend::adapters::files::FileStore;
use backend::adapters::storage::sqlite::SqliteStorage;
use backend::http::routes::router;
use backend::http::state::AppState;
use std::sync::Arc;
use tower::ServiceExt;
use tower_sessions::{MemoryStore, SessionManagerLayer};

async fn app() -> (tempfile::TempDir, Router) {
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}", dir.path().join("http.db").display());
    let storage = Arc::new(SqliteStorage::connect(&url).await.unwrap());
    let files = FileStore::new(dir.path().join("data"));
    let state = AppState::new(storage, files);
    let session_layer = SessionManagerLayer::new(MemoryStore::default()).with_secure(false);
    (dir, router(state, session_layer))
}

fn json_request(
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

/// Extrait `name=value` du premier Set-Cookie d'une réponse.
fn session_cookie(headers: &axum::http::HeaderMap) -> String {
    let raw = headers
        .get(header::SET_COOKIE)
        .expect("Set-Cookie présent")
        .to_str()
        .unwrap();
    raw.split(';').next().unwrap().to_string()
}

#[tokio::test]
async fn register_opens_a_session_then_me_returns_the_account() {
    let (_d, app) = app().await;

    let resp = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/auth/register",
            serde_json::json!({ "email": "boss@test.local", "password": "password123" }),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let cookie = session_cookie(resp.headers());
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let user: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(user["email"], "boss@test.local");
    assert_eq!(user["role"], "admin", "premier compte = admin");
    assert!(user.get("password_hash").is_none(), "jamais de hash exposé");

    // /me avec le cookie → le compte.
    let resp = app
        .clone()
        .oneshot(json_request(
            "GET",
            "/api/auth/me",
            serde_json::json!({}),
            Some(&cookie),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let me: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(me["email"], "boss@test.local");
}

#[tokio::test]
async fn me_without_session_is_401() {
    let (_d, app) = app().await;
    let resp = app
        .oneshot(json_request(
            "GET",
            "/api/auth/me",
            serde_json::json!({}),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn logout_clears_the_session() {
    let (_d, app) = app().await;
    let resp = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/auth/register",
            serde_json::json!({ "email": "u@test.local", "password": "password123" }),
            None,
        ))
        .await
        .unwrap();
    let cookie = session_cookie(resp.headers());

    let resp = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/auth/logout",
            serde_json::json!({}),
            Some(&cookie),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Après logout, le même cookie ne donne plus accès.
    let resp = app
        .oneshot(json_request(
            "GET",
            "/api/auth/me",
            serde_json::json!({}),
            Some(&cookie),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

/// T031 : `DELETE /api/auth/me` exige la confirmation par mot de passe, puis
/// supprime le compte (cascade) et détruit la session. La garde « dernier
/// administrateur » bloque l'auto-suppression du seul admin.
#[tokio::test]
async fn delete_me_requires_the_password_then_removes_the_account() {
    let (_d, app) = app().await;

    // 1er compte = admin ; 2e = simple utilisateur (politique open par défaut).
    let resp = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/auth/register",
            serde_json::json!({ "email": "boss@test.local", "password": "password123" }),
            None,
        ))
        .await
        .unwrap();
    let admin = session_cookie(resp.headers());
    let resp = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/auth/register",
            serde_json::json!({ "email": "member@test.local", "password": "password123" }),
            None,
        ))
        .await
        .unwrap();
    let member = session_cookie(resp.headers());

    // Le seul admin ne peut pas s'auto-supprimer (garde dernier admin) → 403.
    let resp = app
        .clone()
        .oneshot(json_request(
            "DELETE",
            "/api/auth/me",
            serde_json::json!({ "password": "password123" }),
            Some(&admin),
        ))
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "dernier administrateur protégé"
    );

    // Mauvais mot de passe → 403, compte intact.
    let resp = app
        .clone()
        .oneshot(json_request(
            "DELETE",
            "/api/auth/me",
            serde_json::json!({ "password": "wrong-password" }),
            Some(&member),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    let resp = app
        .clone()
        .oneshot(json_request(
            "GET",
            "/api/auth/me",
            serde_json::json!({}),
            Some(&member),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK, "compte toujours là");

    // Bon mot de passe → 204, session détruite (cookie inutilisable).
    let resp = app
        .clone()
        .oneshot(json_request(
            "DELETE",
            "/api/auth/me",
            serde_json::json!({ "password": "password123" }),
            Some(&member),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let resp = app
        .clone()
        .oneshot(json_request(
            "GET",
            "/api/auth/me",
            serde_json::json!({}),
            Some(&member),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // Le compte ne peut plus se reconnecter.
    let resp = app
        .oneshot(json_request(
            "POST",
            "/api/auth/login",
            serde_json::json!({ "email": "member@test.local", "password": "password123" }),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
