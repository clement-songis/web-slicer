//! File de tranchage (T071/US7) : liste + historique (FR-031), détail,
//! annulation (idempotente si déjà annulée, 409 si terminée), isolation
//! inter-comptes (404, SC-008).

use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{header, Request, StatusCode};
use axum::response::Response;
use axum::Router;
use backend::adapters::files::FileStore;
use backend::adapters::storage::sqlite::SqliteStorage;
use backend::domain::repo::{JobOutcome, NewGcode, NewJob, NewProject};
use backend::domain::{JobId, ProjectId, Storage, UserId};
use serde_json::{json, Value};
use tower::ServiceExt;
use tower_sessions::{MemoryStore, SessionManagerLayer};

struct Harness {
    _dir: tempfile::TempDir,
    storage: Arc<SqliteStorage>,
    app: Router,
}

async fn harness() -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}", dir.path().join("jobs.db").display());
    let storage = Arc::new(SqliteStorage::connect(&url).await.unwrap());
    let files = FileStore::new(dir.path().join("data"));
    let state =
        backend::http::state::AppState::new(Arc::clone(&storage) as Arc<dyn Storage>, files);
    let session_layer = SessionManagerLayer::new(MemoryStore::default()).with_secure(false);
    Harness {
        _dir: dir,
        storage,
        app: backend::http::routes::router(state, session_layer),
    }
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

async fn user_id(storage: &SqliteStorage, email: &str) -> UserId {
    storage
        .users()
        .find_by_email(email)
        .await
        .unwrap()
        .unwrap()
        .id
}

async fn make_project(storage: &SqliteStorage, owner: UserId) -> ProjectId {
    storage
        .projects()
        .create(
            owner,
            NewProject {
                name: "P".into(),
                scene: json!({}),
                active_presets: json!({}),
                thumbnail_path: None,
            },
        )
        .await
        .unwrap()
        .id
}

async fn enqueue(storage: &SqliteStorage, owner: UserId, project: ProjectId, plate: i64) -> JobId {
    storage
        .jobs()
        .enqueue(
            owner,
            NewJob {
                project_id: project,
                plate_index: plate,
                resolved_settings: json!({}),
            },
        )
        .await
        .unwrap()
        .id
}

async fn body_json(resp: Response) -> Value {
    let b = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    if b.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&b).unwrap()
    }
}

async fn get(app: &Router, uri: &str, session: &str) -> Response {
    send(app, "GET", uri, session).await
}

async fn send(app: &Router, method: &str, uri: &str, session: &str) -> Response {
    app.clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .header(header::COOKIE, session)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
}

#[tokio::test]
async fn lists_own_jobs_most_recent_first() {
    let h = harness().await;
    let session = register(&h.app, "boss@test.local").await;
    let owner = user_id(&h.storage, "boss@test.local").await;
    let project = make_project(&h.storage, owner).await;
    enqueue(&h.storage, owner, project, 0).await;
    enqueue(&h.storage, owner, project, 1).await;

    let resp = get(&h.app, "/api/jobs", &session).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let jobs = body_json(resp).await;
    assert_eq!(jobs.as_array().unwrap().len(), 2);
    // Plus récent d'abord.
    assert_eq!(jobs[0]["plate_index"], 1);
    assert_eq!(jobs[0]["status"], "queued");
}

#[tokio::test]
async fn gets_a_job_detail() {
    let h = harness().await;
    let session = register(&h.app, "boss@test.local").await;
    let owner = user_id(&h.storage, "boss@test.local").await;
    let project = make_project(&h.storage, owner).await;
    let job = enqueue(&h.storage, owner, project, 0).await;

    let resp = get(&h.app, &format!("/api/jobs/{job}"), &session).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["id"], job.to_string());
    assert_eq!(body["progress"], 0.0);
    assert!(body["created_at"].as_str().is_some());
}

#[tokio::test]
async fn cancels_a_queued_job() {
    let h = harness().await;
    let session = register(&h.app, "boss@test.local").await;
    let owner = user_id(&h.storage, "boss@test.local").await;
    let project = make_project(&h.storage, owner).await;
    let job = enqueue(&h.storage, owner, project, 0).await;

    let resp = send(&h.app, "POST", &format!("/api/jobs/{job}/cancel"), &session).await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(body_json(resp).await["status"], "cancelled");

    // Idempotent : annuler à nouveau reste 200 `cancelled`.
    let again = send(&h.app, "POST", &format!("/api/jobs/{job}/cancel"), &session).await;
    assert_eq!(again.status(), StatusCode::OK);
    assert_eq!(body_json(again).await["status"], "cancelled");
}

#[tokio::test]
async fn cancelling_a_finished_job_conflicts() {
    let h = harness().await;
    let session = register(&h.app, "boss@test.local").await;
    let owner = user_id(&h.storage, "boss@test.local").await;
    let project = make_project(&h.storage, owner).await;
    let job = enqueue(&h.storage, owner, project, 0).await;

    // Termine le job avec un G-code (état `succeeded`).
    let gcode = h
        .storage
        .gcodes()
        .create(
            owner,
            NewGcode {
                job_id: job,
                file_path: "x".into(),
                preview_path: String::new(),
                stats: json!({}),
                thumbnails: json!([]),
            },
        )
        .await
        .unwrap();
    h.storage
        .jobs()
        .finish(job, JobOutcome::Succeeded { gcode_id: gcode.id })
        .await
        .unwrap();

    let resp = send(&h.app, "POST", &format!("/api/jobs/{job}/cancel"), &session).await;
    assert_eq!(resp.status(), StatusCode::CONFLICT);

    // Le détail expose désormais le G-code produit.
    let detail = get(&h.app, &format!("/api/jobs/{job}"), &session).await;
    let body = body_json(detail).await;
    assert_eq!(body["status"], "succeeded");
    assert_eq!(body["gcode_id"], gcode.id.to_string());
}

#[tokio::test]
async fn other_accounts_job_is_404() {
    let h = harness().await;
    register(&h.app, "alice@test.local").await;
    let alice = user_id(&h.storage, "alice@test.local").await;
    let project = make_project(&h.storage, alice).await;
    let job = enqueue(&h.storage, alice, project, 0).await;

    let bob = register(&h.app, "bob@test.local").await;
    assert_eq!(
        get(&h.app, &format!("/api/jobs/{job}"), &bob)
            .await
            .status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        send(&h.app, "POST", &format!("/api/jobs/{job}/cancel"), &bob)
            .await
            .status(),
        StatusCode::NOT_FOUND
    );
    // La liste de Bob est vide (isolation).
    assert_eq!(
        body_json(get(&h.app, "/api/jobs", &bob).await)
            .await
            .as_array()
            .unwrap()
            .len(),
        0
    );
}
