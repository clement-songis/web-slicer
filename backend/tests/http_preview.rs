//! Prévisualisation G-code (T067, R6) : méta-données (couches/types/échelles) et
//! buffers binaires par plage de couches (format `WSPv`), isolation inter-comptes.

use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{header, Request, StatusCode};
use axum::response::Response;
use axum::Router;
use backend::adapters::files::FileStore;
use backend::adapters::storage::AnyStorage;
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
    storage: Arc<AnyStorage>,
    files: FileStore,
    app: Router,
}

async fn harness() -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}", dir.path().join("preview.db").display());
    let storage = Arc::new(AnyStorage::connect_test(&url).await.unwrap());
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

async fn user_id(storage: &AnyStorage, email: &str) -> UserId {
    storage
        .users()
        .find_by_email(email)
        .await
        .unwrap()
        .unwrap()
        .id
}

async fn make_job(storage: &AnyStorage, owner: UserId) -> JobId {
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

// ---------------------------------------------------------------------------
// Lot 31 : statistiques « tous les plateaux » (render_all_plates_stats) et
// coutures du moteur, sur deux plateaux tranchés par OrcaSlicer 2.4.1.
// ---------------------------------------------------------------------------

fn worker_built() -> bool {
    if let Ok(p) = std::env::var("ENGINE_WORKER_BIN") {
        return std::path::Path::new(&p).exists();
    }
    std::env::current_exe()
        .ok()
        .and_then(|e| e.parent()?.parent().map(|d| d.join("engine-worker")))
        .is_some_and(|b| b.exists())
}

fn orca_fixture(name: &str) -> String {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../engine/tests/fixtures")
        .join(name);
    std::fs::read_to_string(p).unwrap()
}

#[tokio::test]
async fn all_plates_stats_sum_the_plates_like_orca() {
    if !worker_built() {
        eprintln!("engine-worker absent : test ignoré (passe FFI)");
        return;
    }
    let h = harness().await;
    let email = "plates@test.local";
    let session = register(&h.app, email).await;
    let owner = user_id(&h.storage, email).await;
    let mut ids = Vec::new();
    let mut footer_time = 0u64;
    let mut footer_cost = 0f64;
    let mut footer_g = 0f64;
    // Deux plateaux du même projet (un job par plateau).
    let first_job = make_job(&h.storage, owner).await;
    let project_id = h
        .storage
        .jobs()
        .get(owner, first_job)
        .await
        .unwrap()
        .project_id;
    for (plate, name) in [
        "gcode_3mf/plate_1.orca.gcode",
        "gcode_3mf/plate_2.orca.gcode",
    ]
    .into_iter()
    .enumerate()
    {
        let text = orca_fixture(name);
        let job = if plate == 0 {
            first_job
        } else {
            h.storage
                .jobs()
                .enqueue(
                    owner,
                    NewJob {
                        project_id,
                        plate_index: plate as i64,
                        resolved_settings: json!({}),
                    },
                )
                .await
                .unwrap()
                .id
        };
        let g = backend::gcode::store_gcode(&h.files, h.storage.gcodes(), owner, job, &text)
            .await
            .unwrap();
        let stats = backend::gcode::parse_stats(&text);
        footer_time += stats.estimated_time_seconds.unwrap();
        footer_cost += stats.total_filament_cost.unwrap_or(0.0);
        footer_g += stats.total_filament_weight_g.unwrap();
        ids.push(g.id.to_string());
    }
    let resp = h
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/gcodes/plates-stats")
                .header(header::COOKIE, &session)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({ "gcode_ids": ids }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(&body_bytes(resp).await).unwrap();
    let plates = body["plates"].as_array().unwrap();
    assert_eq!(plates.len(), 2);
    let all = &body["all"];
    assert_eq!(all["plate_count"], 2);
    // Temps : somme des temps simulés des plateaux = somme des pieds d'Orca.
    let t = all["time"][0].as_f64().unwrap();
    assert!(
        (t.round() - footer_time as f64).abs() <= 2.0,
        "{t} vs {footer_time}"
    );
    let t_sum: f64 = plates.iter().map(|p| p["time"][0].as_f64().unwrap()).sum();
    assert!((t - t_sum).abs() < 1e-6);
    // Coût = PrintStatistics::total_cost figé de chaque plateau.
    assert!((all["total_cost"].as_f64().unwrap() - footer_cost).abs() < 1e-9);
    // Masse : modèle (+ support/purge/tour) cumulés = masse écrite par Orca.
    let g = all["total"]["total"]["g"].as_f64().unwrap();
    assert!(
        (g - footer_g).abs() <= 0.01 * footer_g + 0.02,
        "{g} g vs {footer_g} g"
    );
    let model_volume: f64 = plates
        .iter()
        .map(|p| {
            p["filament_table"]["rows"][0]["model"]["volume"]
                .as_f64()
                .unwrap()
        })
        .sum();
    assert!((all["rows"][0]["model"]["volume"].as_f64().unwrap() - model_volume).abs() < 1e-6);
    assert_eq!(all["columns"]["model"], true);

    // Coutures moteur du plateau 1 (une par couche pour ce cube).
    let resp = get(
        &h.app,
        &format!("/api/gcodes/{}/preview/processed/seams?from=0&to=4", ids[0]),
        &session,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let seams: Value = serde_json::from_slice(&body_bytes(resp).await).unwrap();
    assert!(!seams["seams"].as_array().unwrap().is_empty());

    // Isolation : G-code d'un autre compte → 404 ; liste vide → 422.
    let other = register(&h.app, "plates-other@test.local").await;
    for (cookie, ids, status) in [
        (&other, json!(ids), StatusCode::NOT_FOUND),
        (&session, json!([]), StatusCode::UNPROCESSABLE_ENTITY),
    ] {
        let resp = h
            .app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/gcodes/plates-stats")
                    .header(header::COOKIE, cookie)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(json!({ "gcode_ids": ids }).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), status);
    }
}
