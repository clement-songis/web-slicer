//! Endpoints d'administration (T029) de bout en bout : lecture/écriture des
//! réglages d'instance, création de comptes + reset password, invitations,
//! placeholder reseed. Vérifie surtout la réservation aux admins (403 pour un
//! compte simple, 401 sans session).

use axum::body::{to_bytes, Body};
use axum::http::{header, Request, StatusCode};
use axum::response::Response;
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
    let url = format!("sqlite://{}", dir.path().join("admin.db").display());
    let storage = Arc::new(SqliteStorage::connect(&url).await.unwrap());
    let files = FileStore::new(dir.path().join("data"));
    let state = AppState::new(storage, files);
    let session_layer = SessionManagerLayer::new(MemoryStore::default()).with_secure(false);
    (dir, router(state, session_layer))
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

/// Inscrit un compte et renvoie son cookie de session + le corps de réponse.
async fn register(app: &Router, email: &str, password: &str) -> (String, serde_json::Value) {
    let resp = app
        .clone()
        .oneshot(request(
            "POST",
            "/api/auth/register",
            serde_json::json!({ "email": email, "password": password }),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let cookie = session_cookie(&resp);
    (cookie, json_body(resp).await)
}

#[tokio::test]
async fn admin_reads_and_updates_instance_settings() {
    let (_d, app) = app().await;
    let (admin, _) = register(&app, "boss@test.local", "password123").await;

    // Défauts semés : open / 500 Mo.
    let resp = app
        .clone()
        .oneshot(request(
            "GET",
            "/api/admin/instance",
            serde_json::json!({}),
            Some(&admin),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body["registration_policy"], "open");
    assert_eq!(body["upload_limit_bytes"], 524_288_000i64);

    // Mise à jour partielle : politique + limite.
    let resp = app
        .clone()
        .oneshot(request(
            "PATCH",
            "/api/admin/instance",
            serde_json::json!({ "registration_policy": "invite", "upload_limit_bytes": 1000 }),
            Some(&admin),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body["registration_policy"], "invite");
    assert_eq!(body["upload_limit_bytes"], 1000);

    // Valeur d'enum invalide → 422.
    let resp = app
        .clone()
        .oneshot(request(
            "PATCH",
            "/api/admin/instance",
            serde_json::json!({ "registration_policy": "banana" }),
            Some(&admin),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn admin_routes_are_reserved_to_admins() {
    let (_d, app) = app().await;
    // 1er compte = admin ; 2e = simple utilisateur (politique open par défaut).
    let (_admin, _) = register(&app, "boss@test.local", "password123").await;
    let (user, body) = register(&app, "member@test.local", "password123").await;
    assert_eq!(body["role"], "user");

    // Utilisateur simple → 403.
    let resp = app
        .clone()
        .oneshot(request(
            "GET",
            "/api/admin/instance",
            serde_json::json!({}),
            Some(&user),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    // Sans session → 401.
    let resp = app
        .clone()
        .oneshot(request(
            "GET",
            "/api/admin/instance",
            serde_json::json!({}),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn admin_creates_a_user_then_resets_its_password() {
    let (_d, app) = app().await;
    let (admin, _) = register(&app, "boss@test.local", "password123").await;

    // Création d'un compte géré.
    let resp = app
        .clone()
        .oneshot(request(
            "POST",
            "/api/admin/users",
            serde_json::json!({ "email": "managed@test.local", "password": "initial-pass", "role": "user" }),
            Some(&admin),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let created = json_body(resp).await;
    assert_eq!(created["role"], "user");
    let user_id = created["id"].as_str().unwrap().to_string();

    // Le compte peut se connecter avec le mot de passe initial.
    let resp = app
        .clone()
        .oneshot(request(
            "POST",
            "/api/auth/login",
            serde_json::json!({ "email": "managed@test.local", "password": "initial-pass" }),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Reset par l'admin.
    let resp = app
        .clone()
        .oneshot(request(
            "POST",
            &format!("/api/admin/users/{user_id}/reset-password"),
            serde_json::json!({ "new_password": "brand-new-pass" }),
            Some(&admin),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // L'ancien mot de passe ne marche plus, le nouveau oui.
    let resp = app
        .clone()
        .oneshot(request(
            "POST",
            "/api/auth/login",
            serde_json::json!({ "email": "managed@test.local", "password": "initial-pass" }),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    let resp = app
        .clone()
        .oneshot(request(
            "POST",
            "/api/auth/login",
            serde_json::json!({ "email": "managed@test.local", "password": "brand-new-pass" }),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Reset sur un id inconnu → 404.
    let ghost = uuid::Uuid::new_v4();
    let resp = app
        .clone()
        .oneshot(request(
            "POST",
            &format!("/api/admin/users/{ghost}/reset-password"),
            serde_json::json!({ "new_password": "whatever-pass" }),
            Some(&admin),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn admin_issues_an_invitation_usable_under_invite_policy() {
    let (_d, app) = app().await;
    let (admin, _) = register(&app, "boss@test.local", "password123").await;

    // Passe l'instance en politique invite.
    let resp = app
        .clone()
        .oneshot(request(
            "PATCH",
            "/api/admin/instance",
            serde_json::json!({ "registration_policy": "invite" }),
            Some(&admin),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Émet une invitation.
    let resp = app
        .clone()
        .oneshot(request(
            "POST",
            "/api/admin/invitations",
            serde_json::json!({}),
            Some(&admin),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let inv = json_body(resp).await;
    let token = inv["token"].as_str().unwrap().to_string();
    assert!(!token.is_empty());

    // Inscription sans token refusée (403), avec token acceptée.
    let resp = app
        .clone()
        .oneshot(request(
            "POST",
            "/api/auth/register",
            serde_json::json!({ "email": "invited@test.local", "password": "password123" }),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    let resp = app
        .clone()
        .oneshot(request(
            "POST",
            "/api/auth/register",
            serde_json::json!({ "email": "invited@test.local", "password": "password123", "invite_token": token }),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn reseed_presets_is_not_yet_implemented() {
    let (_d, app) = app().await;
    let (admin, _) = register(&app, "boss@test.local", "password123").await;
    let resp = app
        .oneshot(request(
            "POST",
            "/api/admin/presets/reseed",
            serde_json::json!({}),
            Some(&admin),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_IMPLEMENTED);
}
