//! Plateaux tranchés en lot : « Export (all) plate sliced file » (.gcode.3mf
//! écrit par libslic3r via engine-worker) et « Send all » vers Moonraker
//! (un .gcode.3mf si `use_3mf`, sinon un G-code par plateau).
//! Les G-code des plateaux sont les références Orca de
//! engine/tests/fixtures/gcode_3mf (projet deux plateaux).
#![allow(dead_code)]

mod moonraker_mock;

use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{header, Request, StatusCode};
use axum::response::Response;
use axum::Router;
use backend::adapters::files::FileStore;
use backend::adapters::storage::sqlite::SqliteStorage;
use backend::auth::SecretBox;
use backend::domain::repo::{JobOutcome, NewGcode, NewJob};
use backend::domain::{Preset, PresetKind, PresetOrigin, ProjectId, Storage, UserId};
use moonraker_mock::MockServer;
use serde_json::{json, Value};
use tower::ServiceExt;
use tower_sessions::{MemoryStore, SessionManagerLayer};

struct Harness {
    _dir: tempfile::TempDir,
    storage: Arc<SqliteStorage>,
    files: FileStore,
    app: Router,
}

async fn harness() -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}", dir.path().join("printers.db").display());
    let storage = Arc::new(SqliteStorage::connect(&url).await.unwrap());
    let files = FileStore::new(dir.path().join("data"));
    let state = backend::http::state::AppState::new(
        Arc::clone(&storage) as Arc<dyn Storage>,
        files.clone(),
    )
    // Clé déterministe : le chiffrement au repos reste reproductible en test.
    .with_secrets(SecretBox::new([42u8; 32]));
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

/// Crée un preset machine pour satisfaire la FK `machine_preset_id`.
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

/// Déclare une imprimante pointant vers `moonraker_url` et renvoie son id.
async fn declare_printer(
    h: &Harness,
    session: &str,
    preset_id: &str,
    moonraker_url: &str,
    api_key: Option<&str>,
) -> String {
    let resp = send(
        &h.app,
        "POST",
        "/api/printers",
        session,
        json!({
            "name": "Klipper",
            "moonraker_url": moonraker_url,
            "api_key": api_key,
            "machine_preset_id": preset_id,
        }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = json_body(resp).await;
    body["id"].as_str().unwrap().to_string()
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

async fn wait_for_mesh(app: &Router, model: &str, cookie: &str) {
    for _ in 0..100 {
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/models/{model}/mesh"))
                    .header(header::COOKIE, cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        if resp.status() == StatusCode::OK {
            return;
        }
        assert_eq!(
            resp.status(),
            StatusCode::CONFLICT,
            "conversion encore en cours"
        );
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    panic!("conversion du modèle non terminée");
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

/// STL binaire (normales réelles, en-tête ≠ « solid ») de cubes alignés.
fn cubes_stl(cubes: &[([f32; 3], f32)]) -> Vec<u8> {
    let mut facets: Vec<[[f32; 3]; 3]> = Vec::new();
    for &([x, y, z], s) in cubes {
        let p = |i: usize| -> [f32; 3] {
            [
                x + if i & 1 != 0 { s } else { 0.0 },
                y + if i & 2 != 0 { s } else { 0.0 },
                z + if i & 4 != 0 { s } else { 0.0 },
            ]
        };
        // Faces orientées vers l'extérieur (sommets indexés en binaire xyz).
        for [a, b, c, d] in [
            [0, 2, 3, 1],
            [4, 5, 7, 6],
            [0, 1, 5, 4],
            [2, 6, 7, 3],
            [0, 4, 6, 2],
            [1, 3, 7, 5],
        ] {
            facets.push([p(a), p(b), p(c)]);
            facets.push([p(a), p(c), p(d)]);
        }
    }
    let mut out = vec![b' '; 80];
    out[..9].copy_from_slice(b"test-cube");
    out.extend_from_slice(&(facets.len() as u32).to_le_bytes());
    for [a, b, c] in facets {
        let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let w = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let n = [
            u[1] * w[2] - u[2] * w[1],
            u[2] * w[0] - u[0] * w[2],
            u[0] * w[1] - u[1] * w[0],
        ];
        let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
        for v in [n[0] / len, n[1] / len, n[2] / len]
            .iter()
            .chain(&a)
            .chain(&b)
            .chain(&c)
        {
            out.extend_from_slice(&v.to_le_bytes());
        }
        out.extend_from_slice(&0u16.to_le_bytes());
    }
    out
}

// --- Tests propres aux plateaux tranchés -----------------------------------

const FIX: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../engine/tests/fixtures/gcode_3mf"
);

fn worker_available() -> bool {
    backend::engine::worker_binary().exists()
}

struct Sliced {
    h: Harness,
    session: String,
    owner: UserId,
    project: String,
    jobs: Vec<String>,
}

/// Projet deux plateaux (cube « a » sur le plateau 1, « b » sur le plateau 2),
/// un job réussi par plateau dont le G-code est la référence Orca du plateau.
async fn sliced_project(resolved: Value) -> Option<Sliced> {
    if !worker_available() {
        eprintln!("engine-worker absent — test ignoré");
        return None;
    }
    let _ = tracing_subscriber::fmt().with_test_writer().try_init();
    let h = harness().await;
    let session = register(&h.app, "sliced@test.local").await;
    let owner = user_id(&h.storage, "sliced@test.local").await;
    let project = create_project(&h.app, &session).await;
    let model = upload_model(
        &h.app,
        &project,
        &session,
        "cube.stl",
        &cubes_stl(&[([0.0; 3], 20.0)]),
    )
    .await;
    wait_for_mesh(&h.app, &model, &session).await;
    let scene = json!({
        "schemaVersion": 1,
        "objects": [
            {"id": "a", "modelId": model, "name": "Cube A",
             "transform": {"position": [175.0, 175.0, 10.0], "rotation": [0.0, 0.0, 0.0], "scale": [1.0, 1.0, 1.0]}},
            {"id": "b", "modelId": model, "name": "Cube B",
             "transform": {"position": [175.0, 175.0, 10.0], "rotation": [0.0, 0.0, 0.0], "scale": [1.0, 1.0, 1.0]}}
        ],
        "plates": {"activeId": "p1", "plates": [
            {"id": "p1", "objectIds": ["a"]}, {"id": "p2", "objectIds": ["b"]}
        ]}
    });
    let resp = send(
        &h.app,
        "PUT",
        &format!("/api/projects/{project}"),
        &session,
        json!({"expected_version": 1, "scene": scene, "active_presets": {}}),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK, "sauvegarde de la scène");
    let project_id = ProjectId(uuid::Uuid::parse_str(&project).unwrap());
    let mut jobs = Vec::new();
    for plate in 0..2i64 {
        let job = h
            .storage
            .jobs()
            .enqueue(
                owner,
                NewJob {
                    project_id,
                    plate_index: plate,
                    resolved_settings: resolved.clone(),
                },
            )
            .await
            .unwrap();
        let bytes = std::fs::read(format!("{FIX}/plate_{}.orca.gcode", plate + 1)).unwrap();
        let path = h
            .files
            .write_model(owner, backend::domain::ModelId::new(), "gcode", &bytes)
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
        h.storage
            .jobs()
            .finish(job.id, JobOutcome::Succeeded { gcode_id: gcode.id })
            .await
            .unwrap();
        jobs.push(job.id.to_string());
    }
    Some(Sliced {
        h,
        session,
        owner,
        project,
        jobs,
    })
}

fn zip_entries(bytes: &[u8]) -> std::collections::BTreeMap<String, Vec<u8>> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    (0..archive.len())
        .map(|i| {
            let mut file = archive.by_index(i).unwrap();
            let mut data = Vec::new();
            std::io::Read::read_to_end(&mut file, &mut data).unwrap();
            (file.name().to_string(), data)
        })
        .collect()
}

fn predictions(slice_info: &[u8]) -> Vec<String> {
    let text = String::from_utf8_lossy(slice_info);
    text.split("<metadata key=\"prediction\" value=\"")
        .skip(1)
        .map(|s| s.split('"').next().unwrap().to_string())
        .collect()
}

async fn export(s: &Sliced, body: Value) -> Response {
    send(
        &s.h.app,
        "POST",
        &format!("/api/projects/{}/export/sliced", s.project),
        &s.session,
        body,
    )
    .await
}

#[tokio::test]
async fn export_all_plates_sliced_file_embeds_each_plate_gcode() {
    let Some(s) = sliced_project(json!({})).await else {
        return;
    };
    let resp = export(&s, json!({ "job_ids": s.jobs })).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let disposition = resp.headers()[header::CONTENT_DISPOSITION]
        .to_str()
        .unwrap()
        .to_string();
    assert_eq!(disposition, "attachment; filename=\"projet.gcode.3mf\"");
    let entries = zip_entries(&to_bytes(resp.into_body(), usize::MAX).await.unwrap());
    for n in [1, 2] {
        let expected = std::fs::read(format!("{FIX}/plate_{n}.orca.gcode")).unwrap();
        assert_eq!(
            entries.get(&format!("Metadata/plate_{n}.gcode")),
            Some(&expected),
            "plateau {n}"
        );
        assert!(entries.contains_key(&format!("Metadata/plate_{n}.gcode.md5")));
    }
    // Temps estimés de chaque plateau (mode Normal) = ceux d'Orca.
    assert_eq!(
        predictions(&entries["Metadata/slice_info.config"]),
        vec!["595", "521"]
    );
}

#[tokio::test]
async fn export_one_plate_sliced_file_and_validation_errors() {
    let Some(s) = sliced_project(json!({})).await else {
        return;
    };
    let resp = export(&s, json!({ "job_ids": s.jobs, "plate_index": 1 })).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let disposition = resp.headers()[header::CONTENT_DISPOSITION]
        .to_str()
        .unwrap()
        .to_string();
    assert!(
        disposition.ends_with("projet_plateau-2.gcode.3mf\""),
        "{disposition}"
    );
    let entries = zip_entries(&to_bytes(resp.into_body(), usize::MAX).await.unwrap());
    assert!(!entries.contains_key("Metadata/plate_1.gcode"));
    assert!(entries.contains_key("Metadata/plate_2.gcode"));
    assert_eq!(
        predictions(&entries["Metadata/slice_info.config"]),
        vec!["521"]
    );

    // Aucun job → 422 ; plateau non tranché demandé → 422.
    assert_eq!(
        export(&s, json!({ "job_ids": [] })).await.status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let resp = export(&s, json!({ "job_ids": [s.jobs[0]], "plate_index": 1 })).await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    // Job non réussi → 422.
    let project_id = ProjectId(uuid::Uuid::parse_str(&s.project).unwrap());
    let queued =
        s.h.storage
            .jobs()
            .enqueue(
                s.owner,
                NewJob {
                    project_id,
                    plate_index: 0,
                    resolved_settings: json!({}),
                },
            )
            .await
            .unwrap();
    let resp = export(&s, json!({ "job_ids": [queued.id.to_string()] })).await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    // Projet ou jobs d'un autre compte → 404.
    let other = register(&s.h.app, "other-sliced@test.local").await;
    let resp = send(
        &s.h.app,
        "POST",
        &format!("/api/projects/{}/export/sliced", s.project),
        &other,
        json!({ "job_ids": s.jobs }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn send_all_uploads_one_gcode_per_plate() {
    let Some(s) = sliced_project(json!({})).await else {
        return;
    };
    let mock = MockServer::start().await;
    let preset = seed_machine_preset(&s.h.storage, s.owner).await;
    let printer = declare_printer(&s.h, &s.session, &preset, &mock.base_url(), None).await;
    let uri = format!("/api/printers/{printer}/upload-sliced");
    // Plusieurs fichiers : pas de démarrage immédiat.
    let body = json!({ "project_id": s.project, "job_ids": s.jobs, "start_now": true });
    assert_eq!(
        send(&s.h.app, "POST", &uri, &s.session, body)
            .await
            .status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert!(mock.uploads().is_empty(), "aucun envoi partiel");

    let body = json!({ "project_id": s.project, "job_ids": s.jobs });
    let resp = send(&s.h.app, "POST", &uri, &s.session, body).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let json = json_body(resp).await;
    assert_eq!(json["format"], "gcode");
    let uploads = mock.uploads();
    let names: Vec<&str> = uploads.iter().map(|u| u.filename.as_str()).collect();
    assert_eq!(
        names,
        // Noms Orca relus dans le G-code (filename_format par défaut).
        vec!["Projet_PLA_9m55s.gcode", "Projet_PLA_8m41s.gcode"]
    );
    let sizes: Vec<usize> = uploads.iter().map(|u| u.size).collect();
    let expected: Vec<usize> = [1, 2]
        .iter()
        .map(|n| {
            std::fs::metadata(format!("{FIX}/plate_{n}.orca.gcode"))
                .unwrap()
                .len() as usize
        })
        .collect();
    assert_eq!(sizes, expected);
}

#[tokio::test]
async fn send_all_uploads_a_single_gcode_3mf_when_the_printer_uses_3mf() {
    let Some(s) = sliced_project(json!({ "use_3mf": "1" })).await else {
        return;
    };
    let mock = MockServer::start().await;
    let preset = seed_machine_preset(&s.h.storage, s.owner).await;
    let printer = declare_printer(&s.h, &s.session, &preset, &mock.base_url(), None).await;
    let body = json!({ "project_id": s.project, "job_ids": s.jobs, "start_now": true });
    let resp = send(
        &s.h.app,
        "POST",
        &format!("/api/printers/{printer}/upload-sliced"),
        &s.session,
        body,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let json = json_body(resp).await;
    assert_eq!(json["format"], "gcode_3mf");
    let uploads = mock.uploads();
    assert_eq!(uploads.len(), 1);
    // Nom d'export du premier plateau, extension remplacée par .gcode.3mf.
    assert_eq!(uploads[0].filename, "Projet_PLA_9m55s.gcode.3mf");
    assert!(uploads[0].start_now);
}
