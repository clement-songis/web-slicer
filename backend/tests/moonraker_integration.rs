//! Intégration Moonraker de bout en bout (T078/US8, SC-009, dépendance CI) :
//! contre l'instance simulée complète, un G-code de **50 Mo** est envoyé via la
//! pile HTTP réelle (`POST /api/printers/{id}/upload`) en bien moins de 2 min,
//! l'hôte le reçoit intégralement, l'impression démarre, et le suivi d'état est
//! correct — via l'endpoint `status` **et** le flux WebSocket direct.

mod moonraker_mock;

use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::body::{to_bytes, Body};
use axum::http::{header, Request, StatusCode};
use axum::response::Response;
use axum::Router;
use backend::adapters::files::FileStore;
use backend::adapters::moonraker::MoonrakerClient;
use backend::adapters::storage::sqlite::SqliteStorage;
use backend::auth::SecretBox;
use backend::domain::repo::{NewGcode, NewJob, NewProject};
use backend::domain::{Preset, PresetKind, PresetOrigin, Storage, UserId};
use moonraker_mock::MockServer;
use serde_json::{json, Value};
use tower::ServiceExt;
use tower_sessions::{MemoryStore, SessionManagerLayer};

/// Taille du G-code de test : 50 Mo (SC-009).
const FIFTY_MB: usize = 50 * 1024 * 1024;

struct Harness {
    _dir: tempfile::TempDir,
    storage: Arc<SqliteStorage>,
    files: FileStore,
    app: Router,
}

async fn harness() -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}", dir.path().join("integration.db").display());
    let storage = Arc::new(SqliteStorage::connect(&url).await.unwrap());
    let files = FileStore::new(dir.path().join("data"));
    let state = backend::http::state::AppState::new(
        Arc::clone(&storage) as Arc<dyn Storage>,
        files.clone(),
    )
    .with_secrets(SecretBox::new([7u8; 32]));
    let session_layer = SessionManagerLayer::new(MemoryStore::default()).with_secure(false);
    Harness {
        _dir: dir,
        storage,
        files,
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

async fn send(app: &Router, method: &str, uri: &str, session: &str, body: Value) -> Response {
    app.clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .header(header::COOKIE, session)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap()
}

async fn get(app: &Router, uri: &str, session: &str) -> Response {
    app.clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(uri)
                .header(header::COOKIE, session)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
}

async fn json_body(resp: Response) -> Value {
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

async fn seed_machine_preset(storage: &SqliteStorage, owner: UserId) -> String {
    let preset = storage
        .presets()
        .create_user_preset(
            owner,
            Preset {
                id: backend::domain::PresetId::new(),
                kind: PresetKind::Machine,
                name: "Test machine".into(),
                origin: PresetOrigin::User,
                user_id: Some(owner),
                vendor: None,
                inherits: None,
                instantiation: true,
                setting_id: None,
                filament_id: None,
                compatible_printers: None,
                values: json!({}),
            },
        )
        .await
        .unwrap();
    preset.id.to_string()
}

/// Écrit un fichier G-code de `size` octets et enregistre sa ligne pour `owner`.
async fn seed_large_gcode(h: &Harness, owner: UserId, size: usize) -> String {
    let project = h
        .storage
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
        .unwrap();
    let job = h
        .storage
        .jobs()
        .enqueue(
            owner,
            NewJob {
                project_id: project.id,
                plate_index: 0,
                resolved_settings: json!({}),
            },
        )
        .await
        .unwrap();
    // Contenu volumineux mais valide (préambule G-code + remplissage).
    let mut data = b"; web-slicer 50MB payload\nG28\n".to_vec();
    data.resize(size, b'\n');
    let path = h
        .files
        .write_model(owner, backend::domain::ModelId::new(), "gcode", &data)
        .await
        .unwrap();
    let gcode = h
        .storage
        .gcodes()
        .create(
            owner,
            NewGcode {
                job_id: job.id,
                file_path: path.to_string_lossy().into_owned(),
                preview_path: String::new(),
                stats: json!({}),
                thumbnails: json!([]),
            },
        )
        .await
        .unwrap();
    gcode.id.to_string()
}

async fn declare_printer(h: &Harness, session: &str, preset: &str, url: &str) -> String {
    let resp = send(
        &h.app,
        "POST",
        "/api/printers",
        session,
        json!({
            "name": "Klipper",
            "moonraker_url": url,
            "api_key": null,
            "machine_preset_id": preset,
        }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    json_body(resp).await["id"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn fifty_megabyte_upload_completes_and_is_tracked() {
    let mock = MockServer::start().await;
    let h = harness().await;
    let session = register(&h.app, "boss@test.local").await;
    let owner = user_id(&h.storage, "boss@test.local").await;
    let preset = seed_machine_preset(&h.storage, owner).await;
    let printer = declare_printer(&h, &session, &preset, &mock.base_url()).await;
    let gcode = seed_large_gcode(&h, owner, FIFTY_MB).await;

    // Envoi 50 Mo avec démarrage immédiat, chronométré (SC-009 : < 2 min).
    let started = Instant::now();
    let resp = send(
        &h.app,
        "POST",
        &format!("/api/printers/{printer}/upload"),
        &session,
        json!({ "gcode_id": gcode, "start_now": true }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let elapsed = started.elapsed();
    assert!(
        elapsed < Duration::from_secs(120),
        "SC-009 : envoi 50 Mo en {elapsed:?} (< 2 min attendu)"
    );

    let body = json_body(resp).await;
    assert_eq!(body["print_started"], true);

    // L'hôte a reçu exactement 50 Mo et lancé l'impression.
    let uploads = mock.uploads();
    assert_eq!(uploads.len(), 1);
    assert_eq!(uploads[0].size, FIFTY_MB, "l'hôte reçoit l'intégralité");
    assert!(uploads[0].start_now);
    assert_eq!(mock.actions(), vec!["print".to_string()]);

    // Suivi d'état correct via l'endpoint REST (100 % des cas, SC-009).
    let status =
        json_body(get(&h.app, &format!("/api/printers/{printer}/status"), &session).await).await;
    assert_eq!(status["state"], "printing");
    assert_eq!(status["filename"], "benchy.gcode");
    assert!((status["progress"].as_f64().unwrap() - 0.25).abs() < 1e-9);
    assert!((status["extruder_temp"].as_f64().unwrap() - 210.0).abs() < 1e-9);
    assert!((status["bed_target"].as_f64().unwrap() - 60.0).abs() < 1e-9);
}

#[tokio::test]
async fn live_status_stream_tracks_the_print() {
    // Suivi en direct par le canal WebSocket Moonraker (FR-061) : deux
    // instantanés successifs, progression mise à jour de 0.25 → 0.50.
    let mock = MockServer::start().await;
    let client = MoonrakerClient::new(mock.base_url(), None);
    let mut subscription = client.subscribe().await.unwrap();

    let first = tokio::time::timeout(Duration::from_secs(5), subscription.next())
        .await
        .expect("premier état à temps")
        .expect("flux ouvert");
    assert_eq!(first.state, "printing");
    assert!((first.progress - 0.25).abs() < 1e-9);

    let second = tokio::time::timeout(Duration::from_secs(5), subscription.next())
        .await
        .expect("second état à temps")
        .expect("flux ouvert");
    assert!((second.progress - 0.5).abs() < 1e-9);
    assert_eq!(second.state, "printing");

    subscription.close();
}
