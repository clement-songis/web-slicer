//! Lancement du tranchage (T064) : création de jobs (plateau / tous), presets
//! résolus figés, avertissements moteur restitués (FR-032), gardes avant slice
//! (plateau vide, objet hors plateau), isolation inter-comptes (404, SC-008).

use axum::body::{to_bytes, Body};
use axum::http::{header, Request, StatusCode};
use axum::response::Response;
use axum::Router;
use backend::adapters::files::FileStore;
use backend::adapters::storage::sqlite::SqliteStorage;
use backend::domain::{PresetKind, PresetOrigin, Storage, UserId};
use backend::http::routes::router;
use backend::http::state::AppState;
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt;
use tower_sessions::{MemoryStore, SessionManagerLayer};

async fn app() -> (tempfile::TempDir, Arc<SqliteStorage>, Router) {
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}", dir.path().join("slice.db").display());
    let storage = Arc::new(SqliteStorage::connect(&url).await.unwrap());
    let files = FileStore::new(dir.path().join("data"));
    let state = AppState::new(Arc::clone(&storage) as Arc<dyn Storage>, files);
    let session_layer = SessionManagerLayer::new(MemoryStore::default()).with_secure(false);
    (dir, storage, router(state, session_layer))
}

fn cookie(resp: &Response) -> String {
    resp.headers()
        .get(header::SET_COOKIE)
        .unwrap()
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_string()
}

async fn body_json(resp: Response) -> Value {
    let b = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    if b.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&b).unwrap()
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
                    json!({ "email": email, "password": "password123" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    cookie(&resp)
}

/// Crée un projet directement en base avec une scène et des presets actifs donnés.
async fn create_project(
    storage: &SqliteStorage,
    app: &Router,
    email: &str,
    scene: Value,
    active_presets: Value,
) -> (String, UserId, String) {
    let session = register(app, email).await;
    let user = storage.users().find_by_email(email).await.unwrap().unwrap();
    let project = storage
        .projects()
        .create(
            user.id,
            backend::domain::NewProject {
                name: "P".into(),
                scene,
                active_presets,
                thumbnail_path: None,
            },
        )
        .await
        .unwrap();
    (project.id.to_string(), user.id, session)
}

/// Preset process utilisateur portant une valeur hors bornes (déclenche un
/// avertissement moteur au clamp, FR-032).
async fn make_bad_process(storage: &SqliteStorage, owner: UserId) -> String {
    let preset = storage
        .presets()
        .create_user_preset(
            owner,
            backend::domain::Preset {
                id: backend::domain::PresetId::new(),
                kind: PresetKind::Process,
                name: "Test process".into(),
                origin: PresetOrigin::User,
                user_id: Some(owner),
                vendor: None,
                inherits: None,
                instantiation: true,
                setting_id: None,
                filament_id: None,
                compatible_printers: None,
                values: json!({ "layer_height": "-1" }),
            },
        )
        .await
        .unwrap();
    preset.id.to_string()
}

fn scene_one_object() -> Value {
    json!({
        "schema_version": 1,
        "objects": [{ "id": "o1", "modelId": "m1" }],
        "plates": [{ "id": "p1", "objectIds": ["o1"] }]
    })
}

fn slice_req(project: &str, session: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(format!("/api/projects/{project}/slice"))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::COOKIE, session)
        .body(Body::from(body.to_string()))
        .unwrap()
}

#[tokio::test]
async fn slices_a_plate_and_freezes_resolved_settings_with_warnings() {
    let (_d, storage, app) = app().await;
    let (project, owner, session) = create_project(
        &storage,
        &app,
        "boss@test.local",
        scene_one_object(),
        json!({}),
    )
    .await;
    let process = make_bad_process(&storage, owner).await;
    // Réécrit les presets actifs avec le process fautif.
    storage
        .projects()
        .update(
            owner,
            backend::domain::ProjectId(uuid::Uuid::parse_str(&project).unwrap()),
            1,
            scene_one_object(),
            json!({ "process": process }),
            None,
        )
        .await
        .unwrap();

    let resp = app
        .clone()
        .oneshot(slice_req(&project, &session, json!({ "plate_index": 0 })))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = body_json(resp).await;
    let jobs = body["jobs"].as_array().unwrap();
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0]["status"], "queued");
    assert_eq!(jobs[0]["plate_index"], 0);
    // Avertissement moteur restitué (layer_height clampé, FR-032).
    let warnings = body["warnings"].as_array().unwrap();
    assert!(
        warnings.iter().any(|w| w["key"] == "layer_height"),
        "avertissement layer_height attendu, obtenu {warnings:?}"
    );

    // Le job persiste ses resolved_settings figés.
    let jobs_db = storage.jobs().list(owner).await.unwrap();
    assert_eq!(jobs_db.len(), 1);
    assert!(jobs_db[0].resolved_settings.get("layer_height").is_some());
}

#[tokio::test]
async fn slices_all_plates_creates_one_job_each() {
    let (_d, storage, app) = app().await;
    let scene = json!({
        "schema_version": 1,
        "objects": [{ "id": "o1" }, { "id": "o2" }],
        "plates": [
            { "id": "p1", "objectIds": ["o1"] },
            { "id": "p2", "objectIds": ["o2"] }
        ]
    });
    let (project, _owner, session) =
        create_project(&storage, &app, "boss@test.local", scene, json!({})).await;

    let resp = app
        .clone()
        .oneshot(slice_req(&project, &session, json!({ "all": true })))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = body_json(resp).await;
    assert_eq!(body["jobs"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn rejects_empty_plate_before_slice() {
    let (_d, storage, app) = app().await;
    let scene = json!({
        "schema_version": 1,
        "objects": [],
        "plates": [{ "id": "p1", "objectIds": [] }]
    });
    let (project, _owner, session) =
        create_project(&storage, &app, "boss@test.local", scene, json!({})).await;

    let resp = app
        .clone()
        .oneshot(slice_req(&project, &session, json!({ "plate_index": 0 })))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn rejects_object_off_bed_before_slice() {
    let (_d, storage, app) = app().await;
    let scene = json!({
        "schema_version": 1,
        "objects": [{ "id": "o1", "off_bed": true }],
        "plates": [{ "id": "p1", "objectIds": ["o1"] }]
    });
    let (project, _owner, session) =
        create_project(&storage, &app, "boss@test.local", scene, json!({})).await;

    let resp = app
        .clone()
        .oneshot(slice_req(&project, &session, json!({ "plate_index": 0 })))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn rejects_plate_index_out_of_range() {
    let (_d, storage, app) = app().await;
    let (project, _owner, session) = create_project(
        &storage,
        &app,
        "boss@test.local",
        scene_one_object(),
        json!({}),
    )
    .await;

    let resp = app
        .clone()
        .oneshot(slice_req(&project, &session, json!({ "plate_index": 9 })))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

/// SC-008 : trancher le projet d'autrui répond 404.
#[tokio::test]
async fn slice_others_project_is_404() {
    let (_d, storage, app) = app().await;
    let (project, _owner, _sa) = create_project(
        &storage,
        &app,
        "alice@test.local",
        scene_one_object(),
        json!({}),
    )
    .await;
    let bob = register(&app, "bob@test.local").await;

    let resp = app
        .clone()
        .oneshot(slice_req(&project, &bob, json!({ "plate_index": 0 })))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
