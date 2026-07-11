//! Canal WebSocket `/api/ws` (T065) : bus d'événements cloisonné par compte
//! (isolation SC-008), progression de la file relayée depuis le runner, et garde
//! d'authentification (session absente → 401 avant tout upgrade).

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use backend::adapters::files::FileStore;
use backend::adapters::storage::sqlite::SqliteStorage;
use backend::domain::repo::{NewJob, NewProject, NewUser};
use backend::domain::{JobStatus, ProjectId, Role, SlicingJob, Storage, UserId};
use backend::http::routes::router;
use backend::http::state::AppState;
use backend::http::ws::EventHub;
use backend::queue::{JobContext, JobEventSink, JobRunner, Queue, QueueConfig, RunOutcome};
use serde_json::json;
use tokio::sync::broadcast;
use tower::ServiceExt;
use tower_sessions::{MemoryStore, SessionManagerLayer};

async fn storage() -> (tempfile::TempDir, Arc<SqliteStorage>) {
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}", dir.path().join("ws.db").display());
    (dir, Arc::new(SqliteStorage::connect(&url).await.unwrap()))
}

async fn make_user(s: &dyn Storage, email: &str) -> UserId {
    s.users()
        .create(NewUser {
            email: email.into(),
            password_hash: "x".into(),
            role: Role::User,
        })
        .await
        .unwrap()
        .id
}

async fn make_project(s: &dyn Storage, owner: UserId) -> ProjectId {
    s.projects()
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

/// Un événement `job.updated`/`job.finished` reçu, décodé en JSON.
fn recv_json(rx: &mut broadcast::Receiver<backend::http::dto::ServerEvent>) -> serde_json::Value {
    let event = rx.try_recv().expect("un événement attendu sur ce canal");
    serde_json::to_value(event).unwrap()
}

/// Le bus ne diffuse un événement qu'au compte propriétaire : l'abonné d'un autre
/// compte ne le voit jamais (isolation, SC-008).
#[test]
fn hub_isolates_events_per_account() {
    let hub = EventHub::new();
    let alice = UserId(uuid::Uuid::new_v4());
    let bob = UserId(uuid::Uuid::new_v4());

    let mut a_rx = hub.subscribe(alice);
    let mut b_rx = hub.subscribe(bob);

    hub.publish_model_converted(alice, "m1", "/api/models/m1/mesh");

    // Alice reçoit son événement…
    let event = recv_json(&mut a_rx);
    assert_eq!(event["event"], "model.converted");
    assert_eq!(event["model_id"], "m1");
    // …Bob ne reçoit rien (canal distinct).
    assert!(
        b_rx.try_recv().is_err(),
        "aucun événement ne doit fuir vers un autre compte"
    );
}

/// Runner qui relaie une progression puis échoue (état terminal sans G-code réel).
struct ReportingRunner;

#[async_trait]
impl JobRunner for ReportingRunner {
    async fn run(&self, _job: SlicingJob, ctx: JobContext) -> RunOutcome {
        ctx.report(0.5, "slicing").await;
        RunOutcome::Failed(json!({ "reason": "stub" }))
    }
}

/// La progression du runner remonte jusqu'à l'abonné du propriétaire (pipe
/// engine-worker relayé), et un abonné d'un autre compte n'en voit rien.
#[tokio::test]
async fn queue_relays_job_progress_to_owner_only() {
    let (_d, store) = storage().await;
    let owner = make_user(store.as_ref(), "owner@test.local").await;
    let other = make_user(store.as_ref(), "other@test.local").await;
    let project = make_project(store.as_ref(), owner).await;
    store
        .jobs()
        .enqueue(
            owner,
            NewJob {
                project_id: project,
                plate_index: 0,
                resolved_settings: json!({}),
            },
        )
        .await
        .unwrap();

    let hub = Arc::new(EventHub::new());
    let mut owner_rx = hub.subscribe(owner);
    let mut other_rx = hub.subscribe(other);

    let queue = Arc::new(
        Queue::new(
            Arc::clone(&store) as Arc<dyn Storage>,
            Arc::new(ReportingRunner),
            QueueConfig {
                workers: 1,
                poll_interval: Duration::from_millis(20),
            },
        )
        .with_event_sink(Arc::clone(&hub) as Arc<dyn JobEventSink>),
    );
    let handle = Arc::clone(&queue).start().await;

    // Attend que le job atteigne son état terminal.
    for _ in 0..200 {
        let jobs = store.jobs().list(owner).await.unwrap();
        if jobs.iter().all(|j| j.status == JobStatus::Failed) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    handle.shutdown().await;

    // Progression relayée (job.updated running 0.5) puis fin (job.updated failed).
    let running = recv_json(&mut owner_rx);
    assert_eq!(running["event"], "job.updated");
    assert_eq!(running["status"], "running");
    assert_eq!(running["progress"], 0.5);
    assert_eq!(running["phase"], "slicing");

    let failed = recv_json(&mut owner_rx);
    assert_eq!(failed["event"], "job.updated");
    assert_eq!(failed["status"], "failed");
    assert_eq!(failed["error"]["reason"], "stub");

    // Aucun événement ne fuit vers un autre compte.
    assert!(
        other_rx.try_recv().is_err(),
        "les événements de job restent cloisonnés au propriétaire"
    );
}

async fn app_async() -> (tempfile::TempDir, axum::Router) {
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}", dir.path().join("ws_http.db").display());
    let store = Arc::new(SqliteStorage::connect(&url).await.unwrap());
    let files = FileStore::new(dir.path().join("data"));
    let state = AppState::new(Arc::clone(&store) as Arc<dyn Storage>, files);
    let session_layer = SessionManagerLayer::new(MemoryStore::default()).with_secure(false);
    (dir, router(state, session_layer))
}

/// Sans session, l'ouverture du canal répond 401 (garde d'authentification), et
/// ce, avant toute négociation d'upgrade.
#[tokio::test]
async fn ws_requires_authentication() {
    let (_d, app) = app_async().await;

    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/ws")
                .header(header::CONNECTION, "upgrade")
                .header(header::UPGRADE, "websocket")
                .header("sec-websocket-version", "13")
                .header("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ==")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
