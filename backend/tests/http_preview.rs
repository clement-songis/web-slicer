//! Prévisualisation G-code (T067, R6) : méta-données (couches/types/échelles) et
//! buffers binaires par plage de couches (format `WSPv`), isolation inter-comptes.

use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{header, Request, StatusCode};
use axum::response::Response;
use axum::Router;
use backend::adapters::files::FileStore;
use backend::adapters::storage::sqlite::SqliteStorage;
use backend::domain::repo::{NewJob, NewProject};
use backend::domain::{JobId, Storage, UserId};
use serde_json::{json, Value};
use tower::ServiceExt;
use tower_sessions::{MemoryStore, SessionManagerLayer};

/// Deux couches typées (mode E absolu).
const SAMPLE_GCODE: &str = "\
G1 X0 Y0 Z0.2 F1200
;LAYER_CHANGE
;Z:0.2
;HEIGHT:0.2
;WIDTH:0.45
;TYPE:Outer wall
G1 X10 Y0 E0.5 F1800
G1 X10 Y10 E1.0
;TYPE:Sparse infill
G1 X0 Y10 E1.5 F3000
;LAYER_CHANGE
;Z:0.4
;TYPE:Outer wall
G1 X0 Y0 E2.0 F1800
";

struct Harness {
    _dir: tempfile::TempDir,
    storage: Arc<SqliteStorage>,
    files: FileStore,
    app: Router,
}

async fn harness() -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}", dir.path().join("preview.db").display());
    let storage = Arc::new(SqliteStorage::connect(&url).await.unwrap());
    let files = FileStore::new(dir.path().join("data"));
    let state = backend::http::state::AppState::new(
        Arc::clone(&storage) as Arc<dyn Storage>,
        files.clone(),
    );
    let session_layer = SessionManagerLayer::new(MemoryStore::default()).with_secure(false);
    let app = backend::http::routes::router(state, session_layer);
    Harness {
        _dir: dir,
        storage,
        files,
        app,
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

async fn make_job(storage: &SqliteStorage, owner: UserId) -> JobId {
    let project = storage
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
    storage
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
        .unwrap()
        .id
}

async fn body_bytes(resp: Response) -> Vec<u8> {
    to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap()
        .to_vec()
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

/// Crée un G-code appartenant à `email` et renvoie (session, gcode_id).
async fn seed_gcode(h: &Harness, email: &str) -> (String, String) {
    let session = register(&h.app, email).await;
    let owner = user_id(&h.storage, email).await;
    let job = make_job(&h.storage, owner).await;
    let gcode = backend::gcode::store_gcode(&h.files, h.storage.gcodes(), owner, job, SAMPLE_GCODE)
        .await
        .unwrap();
    (session, gcode.id.to_string())
}

#[tokio::test]
async fn meta_reports_layers_types_and_scales() {
    let h = harness().await;
    let (session, id) = seed_gcode(&h, "boss@test.local").await;

    let resp = get(&h.app, &format!("/api/gcodes/{id}/preview/meta"), &session).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let meta: Value = serde_json::from_slice(&body_bytes(resp).await).unwrap();

    assert_eq!(meta["layer_count"], 2);
    assert_eq!(meta["layers"].as_array().unwrap().len(), 2);
    // z est stocké en f32 puis élargi en f64 : comparaison avec tolérance.
    assert!((meta["layers"][0]["z"].as_f64().unwrap() - 0.2).abs() < 1e-4);
    assert_eq!(meta["layers"][0]["segment_count"], 3);
    // Types présents nommés (légende, FR-041).
    let names: Vec<&str> = meta["types_present"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"Outer wall"), "{names:?}");
    assert!(names.contains(&"Sparse infill"), "{names:?}");
    assert_eq!(meta["feedrate_max"], 3000.0);
    assert_eq!(meta["segment_record_bytes"], 40);
}

#[tokio::test]
async fn layers_buffer_has_wspv_header_and_records() {
    let h = harness().await;
    let (session, id) = seed_gcode(&h, "boss@test.local").await;

    // Couche 0 seule : 3 segments.
    let resp = get(
        &h.app,
        &format!("/api/gcodes/{id}/preview/layers?from=0&to=0"),
        &session,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        resp.headers().get(header::CONTENT_TYPE).unwrap(),
        "application/octet-stream"
    );
    let buf = body_bytes(resp).await;
    assert_eq!(&buf[0..4], b"WSPv");
    let count = u32::from_le_bytes([buf[14], buf[15], buf[16], buf[17]]);
    assert_eq!(count, 3);
    assert_eq!(buf.len(), 18 + 3 * 40);
}

#[tokio::test]
async fn layers_without_range_returns_all_layers() {
    let h = harness().await;
    let (session, id) = seed_gcode(&h, "boss@test.local").await;

    let resp = get(
        &h.app,
        &format!("/api/gcodes/{id}/preview/layers"),
        &session,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let buf = body_bytes(resp).await;
    // 3 segments (couche 0) + 1 segment (couche 1) = 4.
    let count = u32::from_le_bytes([buf[14], buf[15], buf[16], buf[17]]);
    assert_eq!(count, 4);
    let to = u32::from_le_bytes([buf[10], buf[11], buf[12], buf[13]]);
    assert_eq!(to, 1);
}

#[tokio::test]
async fn other_accounts_preview_is_404() {
    let h = harness().await;
    let (_alice, id) = seed_gcode(&h, "alice@test.local").await;
    let bob = register(&h.app, "bob@test.local").await;

    for suffix in ["preview/meta", "preview/layers"] {
        let resp = get(&h.app, &format!("/api/gcodes/{id}/{suffix}"), &bob).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND, "{suffix}");
    }
}
