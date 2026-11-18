//! Modèles pratiques d'Orca (« Add Handy models », GUI_Factories.cpp:576) :
//! catalogue, ajout au projet (un modèle par fichier), erreurs.

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

async fn create_project(app: &Router, cookie: &str) -> String {
    let resp = app
        .clone()
        .oneshot(request(
            "POST",
            "/api/projects",
            json!({ "name": "Handy" }),
            Some(cookie),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    json_body(resp).await["id"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn handy_models_catalogue_and_add_follow_orca() {
    let (_dir, app) = app().await;
    let cookie = register(&app, "handy@example.com").await;

    let resp = app
        .clone()
        .oneshot(request(
            "GET",
            "/api/handy-models",
            json!(null),
            Some(&cookie),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let list = json_body(resp).await;
    let list = list.as_array().unwrap();
    assert_eq!(list.len(), 9);
    assert_eq!(list[0]["label"], "Orca Cube");
    assert_eq!(list[0]["arrange_after_import"], true);
    let hell = list
        .iter()
        .find(|m| m["key"] == "orca-string-hell")
        .unwrap();
    assert_eq!(hell["suggestion"]["apply"]["min_width_top_surface"], "0");
    assert!(list.iter().find(|m| m["key"] == "3dbenchy").unwrap()["suggestion"].is_null());

    let project = create_project(&app, &cookie).await;
    let resp = app
        .clone()
        .oneshot(request(
            "POST",
            &format!("/api/projects/{project}/handy-models/orca-cube"),
            json!(null),
            Some(&cookie),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let added = json_body(resp).await;
    let names: Vec<_> = added["models"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["filename"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(names, ["OrcaCube_v2.drc", "OrcaPlug_v2.drc"]);
    assert_eq!(added["handy_model"]["arrange_after_import"], true);

    let resp = app
        .clone()
        .oneshot(request(
            "GET",
            &format!("/api/projects/{project}/models"),
            json!(null),
            Some(&cookie),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(json_body(resp).await.as_array().unwrap().len(), 2);

    // Clé inconnue, projet d'un autre compte : 404, rien d'ajouté.
    let resp = app
        .clone()
        .oneshot(request(
            "POST",
            &format!("/api/projects/{project}/handy-models/nope"),
            json!(null),
            Some(&cookie),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let other = register(&app, "other@example.com").await;
    let resp = app
        .clone()
        .oneshot(request(
            "POST",
            &format!("/api/projects/{project}/handy-models/3dbenchy"),
            json!(null),
            Some(&other),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    // Non authentifié.
    let resp = app
        .clone()
        .oneshot(request("GET", "/api/handy-models", json!(null), None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
