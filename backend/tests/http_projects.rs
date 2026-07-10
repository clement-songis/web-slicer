//! Endpoints projets (T030) de bout en bout : CRUD, duplicate/rename, verrou
//! optimiste (409), et **isolation inter-comptes** (SC-008 : un projet d'autrui
//! répond 404, jamais 403 — l'existence n'est pas révélée).

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
    let url = format!("sqlite://{}", dir.path().join("projects.db").display());
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

/// Inscrit un compte, renvoie son cookie de session.
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

async fn create_project(app: &Router, cookie: &str, name: &str) -> serde_json::Value {
    let resp = app
        .clone()
        .oneshot(request(
            "POST",
            "/api/projects",
            serde_json::json!({ "name": name }),
            Some(cookie),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    json_body(resp).await
}

#[tokio::test]
async fn create_lists_and_opens_a_project() {
    let (_d, app) = app().await;
    let user = register(&app, "boss@test.local").await;

    let project = create_project(&app, &user, "Benchy").await;
    assert_eq!(project["name"], "Benchy");
    assert_eq!(project["version"], 1);
    assert_eq!(project["scene"]["schema_version"], 1);
    assert_eq!(project["has_thumbnail"], false);
    let id = project["id"].as_str().unwrap().to_string();

    // Bibliothèque.
    let resp = app
        .clone()
        .oneshot(request(
            "GET",
            "/api/projects",
            serde_json::json!({}),
            Some(&user),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let list = json_body(resp).await;
    assert_eq!(list.as_array().unwrap().len(), 1);

    // Ouverture.
    let resp = app
        .clone()
        .oneshot(request(
            "GET",
            &format!("/api/projects/{id}"),
            serde_json::json!({}),
            Some(&user),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(json_body(resp).await["id"], id);
}

#[tokio::test]
async fn save_uses_optimistic_locking() {
    let (_d, app) = app().await;
    let user = register(&app, "boss@test.local").await;
    let project = create_project(&app, &user, "Benchy").await;
    let id = project["id"].as_str().unwrap().to_string();

    // Sauvegarde avec la bonne version (1 à la création) → 200, version 2.
    let resp = app
        .clone()
        .oneshot(request(
            "PUT",
            &format!("/api/projects/{id}"),
            serde_json::json!({
                "expected_version": 1,
                "scene": { "schema_version": 1, "objects": [1], "plates": [] },
                "active_presets": {}
            }),
            Some(&user),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(json_body(resp).await["version"], 2);

    // Rejouer avec la version périmée → 409.
    let resp = app
        .clone()
        .oneshot(request(
            "PUT",
            &format!("/api/projects/{id}"),
            serde_json::json!({
                "expected_version": 1,
                "scene": { "schema_version": 1, "objects": [], "plates": [] },
                "active_presets": {}
            }),
            Some(&user),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn duplicate_and_rename() {
    let (_d, app) = app().await;
    let user = register(&app, "boss@test.local").await;
    let project = create_project(&app, &user, "Benchy").await;
    let id = project["id"].as_str().unwrap().to_string();

    // Duplication.
    let resp = app
        .clone()
        .oneshot(request(
            "POST",
            &format!("/api/projects/{id}/duplicate"),
            serde_json::json!({}),
            Some(&user),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let copy = json_body(resp).await;
    assert_eq!(copy["name"], "Benchy (copie)");
    assert_ne!(copy["id"], project["id"]);

    // Renommage.
    let resp = app
        .clone()
        .oneshot(request(
            "PATCH",
            &format!("/api/projects/{id}/rename"),
            serde_json::json!({ "name": "Benchy v2" }),
            Some(&user),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(json_body(resp).await["name"], "Benchy v2");
}

#[tokio::test]
async fn delete_removes_the_project() {
    let (_d, app) = app().await;
    let user = register(&app, "boss@test.local").await;
    let project = create_project(&app, &user, "Benchy").await;
    let id = project["id"].as_str().unwrap().to_string();

    let resp = app
        .clone()
        .oneshot(request(
            "DELETE",
            &format!("/api/projects/{id}"),
            serde_json::json!({}),
            Some(&user),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let resp = app
        .clone()
        .oneshot(request(
            "GET",
            &format!("/api/projects/{id}"),
            serde_json::json!({}),
            Some(&user),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

/// SC-008 : un compte ne voit ni ne touche les projets d'un autre — 404 partout,
/// jamais 403 (l'existence de la ressource ne fuite pas).
#[tokio::test]
async fn projects_are_isolated_between_accounts() {
    let (_d, app) = app().await;
    let alice = register(&app, "alice@test.local").await;
    let bob = register(&app, "bob@test.local").await;

    let project = create_project(&app, &alice, "Secret d'Alice").await;
    let id = project["id"].as_str().unwrap().to_string();

    // La bibliothèque de Bob est vide.
    let resp = app
        .clone()
        .oneshot(request(
            "GET",
            "/api/projects",
            serde_json::json!({}),
            Some(&bob),
        ))
        .await
        .unwrap();
    assert_eq!(json_body(resp).await.as_array().unwrap().len(), 0);

    // Toutes les opérations de Bob sur le projet d'Alice → 404.
    let get = request(
        "GET",
        &format!("/api/projects/{id}"),
        serde_json::json!({}),
        Some(&bob),
    );
    let del = request(
        "DELETE",
        &format!("/api/projects/{id}"),
        serde_json::json!({}),
        Some(&bob),
    );
    let dup = request(
        "POST",
        &format!("/api/projects/{id}/duplicate"),
        serde_json::json!({}),
        Some(&bob),
    );
    let ren = request(
        "PATCH",
        &format!("/api/projects/{id}/rename"),
        serde_json::json!({ "name": "x" }),
        Some(&bob),
    );
    let put = request(
        "PUT",
        &format!("/api/projects/{id}"),
        serde_json::json!({ "expected_version": 0, "scene": {}, "active_presets": {} }),
        Some(&bob),
    );
    for req in [get, del, dup, ren, put] {
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::NOT_FOUND,
            "opération inter-comptes doit répondre 404 (SC-008)"
        );
    }

    // Le projet d'Alice reste intact.
    let resp = app
        .clone()
        .oneshot(request(
            "GET",
            &format!("/api/projects/{id}"),
            serde_json::json!({}),
            Some(&alice),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(json_body(resp).await["name"], "Secret d'Alice");
}

#[tokio::test]
async fn projects_require_a_session() {
    let (_d, app) = app().await;
    let resp = app
        .oneshot(request("GET", "/api/projects", serde_json::json!({}), None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
