//! Outils moteur de scène (T054) de bout en bout : arrangement (FR-013),
//! rapport de réparation (FR-012), suggestion d'orientation, et isolation
//! inter-comptes (404, SC-008).

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
    let url = format!("sqlite://{}", dir.path().join("scene.db").display());
    let storage = Arc::new(SqliteStorage::connect(&url).await.unwrap());
    let files = FileStore::new(dir.path().join("data"));
    let state = AppState::new(storage, files);
    let session_layer = SessionManagerLayer::new(MemoryStore::default()).with_secure(false);
    (dir, router(state, session_layer))
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
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/register")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::json!({ "email": email, "password": "password123" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    session_cookie(&resp)
}

async fn create_project(app: &Router, cookie: &str) -> String {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/projects")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie)
                .body(Body::from(
                    serde_json::json!({ "name": "Projet" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    json_body(resp).await["id"].as_str().unwrap().to_string()
}

/// STL binaire d'un unique triangle aux sommets donnés.
fn binary_stl_tri(verts: [[f32; 3]; 3]) -> Vec<u8> {
    let mut v = vec![0u8; 80];
    v.extend_from_slice(&1u32.to_le_bytes());
    for f in [0.0f32, 0.0, 1.0] {
        v.extend_from_slice(&f.to_le_bytes());
    }
    for vert in verts {
        for c in vert {
            v.extend_from_slice(&c.to_le_bytes());
        }
    }
    v.extend_from_slice(&0u16.to_le_bytes());
    v
}

fn multipart(filename: &str, content: &[u8]) -> (String, Vec<u8>) {
    let boundary = "----webslicertestboundary";
    let mut body = Vec::new();
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(
        format!("Content-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\n")
            .as_bytes(),
    );
    body.extend_from_slice(b"Content-Type: application/octet-stream\r\n\r\n");
    body.extend_from_slice(content);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    (format!("multipart/form-data; boundary={boundary}"), body)
}

async fn upload_model(
    app: &Router,
    project: &str,
    cookie: &str,
    name: &str,
    body: &[u8],
) -> String {
    let (ct, payload) = multipart(name, body);
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/projects/{project}/models"))
                .header(header::CONTENT_TYPE, ct)
                .header(header::COOKIE, cookie)
                .body(Body::from(payload))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    json_body(resp).await["id"].as_str().unwrap().to_string()
}

fn post(uri: &str, cookie: &str, body: serde_json::Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::COOKIE, cookie)
        .body(Body::from(body.to_string()))
        .unwrap()
}

#[tokio::test]
async fn arrange_places_objects_without_collision() {
    let (_d, app) = app().await;
    let user = register(&app, "boss@test.local").await;
    let project = create_project(&app, &user).await;

    let resp = app
        .clone()
        .oneshot(post(
            &format!("/api/projects/{project}/arrange"),
            &user,
            serde_json::json!({
                "bed_width": 100.0, "bed_depth": 100.0, "spacing": 5.0,
                "items": [
                    { "id": "a", "width": 20.0, "depth": 20.0 },
                    { "id": "b", "width": 20.0, "depth": 20.0 }
                ]
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    let placements = body["placements"].as_array().unwrap();
    assert_eq!(placements.len(), 2);
    assert_eq!(placements[0]["id"], "a");
    assert_eq!(placements[0]["x"], 15.0);
    assert_eq!(placements[1]["x"], 40.0);
}

#[tokio::test]
async fn arrange_rejects_invalid_bed() {
    let (_d, app) = app().await;
    let user = register(&app, "boss@test.local").await;
    let project = create_project(&app, &user).await;

    let resp = app
        .clone()
        .oneshot(post(
            &format!("/api/projects/{project}/arrange"),
            &user,
            serde_json::json!({ "bed_width": 0.0, "bed_depth": 100.0, "spacing": 5.0, "items": [] }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

/// SC-008 : arranger dans le projet d'autrui répond 404.
#[tokio::test]
async fn arrange_in_others_project_is_404() {
    let (_d, app) = app().await;
    let alice = register(&app, "alice@test.local").await;
    let bob = register(&app, "bob@test.local").await;
    let project = create_project(&app, &alice).await;

    let resp = app
        .clone()
        .oneshot(post(
            &format!("/api/projects/{project}/arrange"),
            &bob,
            serde_json::json!({ "bed_width": 100.0, "bed_depth": 100.0, "spacing": 5.0, "items": [] }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn arrange_requires_a_session() {
    let (_d, app) = app().await;
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/projects/00000000-0000-0000-0000-000000000000/arrange")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::json!({ "bed_width": 1.0, "bed_depth": 1.0, "spacing": 0.0, "items": [] })
                        .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn repair_reports_open_triangle() {
    let (_d, app) = app().await;
    let user = register(&app, "boss@test.local").await;
    let project = create_project(&app, &user).await;
    let stl = binary_stl_tri([[0.0, 0.0, 0.0], [10.0, 0.0, 0.0], [0.0, 10.0, 0.0]]);
    let model = upload_model(&app, &project, &user, "tri.stl", &stl).await;

    let resp = app
        .clone()
        .oneshot(post(
            &format!("/api/models/{model}/repair"),
            &user,
            serde_json::json!({}),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body["triangles"], 1);
    assert_eq!(body["open_edges"], 3);
    assert_eq!(body["watertight"], false);
}

/// SC-008 : réparer le modèle d'autrui répond 404.
#[tokio::test]
async fn repair_of_others_model_is_404() {
    let (_d, app) = app().await;
    let alice = register(&app, "alice@test.local").await;
    let bob = register(&app, "bob@test.local").await;
    let project = create_project(&app, &alice).await;
    let stl = binary_stl_tri([[0.0, 0.0, 0.0], [10.0, 0.0, 0.0], [0.0, 10.0, 0.0]]);
    let model = upload_model(&app, &project, &alice, "tri.stl", &stl).await;

    let resp = app
        .clone()
        .oneshot(post(
            &format!("/api/models/{model}/repair"),
            &bob,
            serde_json::json!({}),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn orient_suggests_rotation_for_stl() {
    let (_d, app) = app().await;
    let user = register(&app, "boss@test.local").await;
    let project = create_project(&app, &user).await;
    // Facette dans le plan XY (normale +Z) → orientation la ramène vers le bas.
    let stl = binary_stl_tri([[0.0, 0.0, 5.0], [10.0, 0.0, 5.0], [0.0, 10.0, 5.0]]);
    let model = upload_model(&app, &project, &user, "tri.stl", &stl).await;

    let resp = app
        .clone()
        .oneshot(post(
            &format!("/api/projects/{project}/orient"),
            &user,
            serde_json::json!({ "model_id": model }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    let rotation = body["rotation"].as_array().unwrap();
    assert_eq!(rotation.len(), 3);
}
