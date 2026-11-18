//! Export Draco de bout en bout (worker réel `export-drc`, `store_drc` d'Orca) :
//! fichier fusionné, ZIP d'un `.drc` par objet, sélection d'objets ; chaque
//! `.drc` commence par « DRACO » et se relit via le moteur (`load-model drc`).

use axum::body::{to_bytes, Body};
use axum::http::{header, Request, StatusCode};
use axum::response::Response;
use axum::Router;
use backend::adapters::files::FileStore;
use backend::adapters::storage::AnyStorage;
use backend::http::routes::router;
use backend::http::state::AppState;
use std::sync::Arc;
use tower::ServiceExt;
use tower_sessions::{MemoryStore, SessionManagerLayer};

async fn app() -> (tempfile::TempDir, Router, AppState) {
    let _ = tracing_subscriber::fmt().with_test_writer().try_init();
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}", dir.path().join("scene.db").display());
    let storage = Arc::new(AnyStorage::connect_test(&url).await.unwrap());
    let files = FileStore::new(dir.path().join("data"));
    let state = AppState::new(storage, files);
    let session_layer = SessionManagerLayer::new(MemoryStore::default()).with_secure(false);
    let app = router(state.clone(), session_layer);
    (dir, app, state)
}

fn worker_available() -> bool {
    backend::engine::worker_binary().exists()
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

async fn setup() -> Option<(tempfile::TempDir, Router, String, String)> {
    if !worker_available() {
        eprintln!("engine-worker absent — test d'export Draco réel ignoré");
        return None;
    }
    let (dir, app, _state) = app().await;
    let user = register(&app, "drc@test.local").await;
    let project = create_project(&app, &user).await;
    let model = upload_model(
        &app,
        &project,
        &user,
        "cube.stl",
        &cubes_stl(&[([0.0; 3], 20.0)]),
    )
    .await;
    wait_for_mesh(&app, &model, &user).await;
    // Deux instances du cube, déplacées (et l'une en miroir X) dans la scène.
    let scene = serde_json::json!({
        "schemaVersion": 1,
        "objects": [
            {"id": "a", "modelId": model, "name": "Cube A",
             "transform": {"position": [0.0, 0.0, 10.0], "rotation": [0.0, 0.0, 0.0], "scale": [1.0, 1.0, 1.0]}},
            {"id": "b", "modelId": model, "name": "Cube B",
             "transform": {"position": [50.0, 0.0, 10.0], "rotation": [0.0, 0.0, 0.0], "scale": [-1.0, 1.0, 1.0]}}
        ],
        "plates": {"activeId": "p1", "plates": [{"id": "p1", "objectIds": ["a", "b"]}]}
    });
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/projects/{project}"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &user)
                .body(Body::from(
                    serde_json::json!({"expected_version": 1, "scene": scene, "active_presets": {}})
                        .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK, "sauvegarde de la scène");
    Some((dir, app, user, project))
}

async fn export(app: &Router, cookie: &str, project: &str, body: serde_json::Value) -> Response {
    app.clone()
        .oneshot(post(
            &format!("/api/projects/{project}/export/drc"),
            cookie,
            body,
        ))
        .await
        .unwrap()
}

/// Relit un `.drc` via le moteur (`load-model <fichier> drc` → WSMh).
fn decode_drc(bytes: &[u8]) -> engine::api::DisplayMesh {
    assert!(bytes.starts_with(b"DRACO"), "en-tête Draco attendu");
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("export.drc");
    std::fs::write(&path, bytes).unwrap();
    let output = std::process::Command::new(backend::engine::worker_binary())
        .args(["load-model", path.to_str().unwrap(), "drc"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "relecture Draco : {stdout}");
    let wsmh = stdout
        .lines()
        .find_map(|line| line.strip_prefix("R "))
        .expect("ligne R du worker");
    let decoded = engine::api::DisplayMesh::decode(&std::fs::read(wsmh.trim()).unwrap()).unwrap();
    let _ = std::fs::remove_file(wsmh.trim());
    decoded
}

fn x_range(mesh: &engine::api::DisplayMesh) -> (f32, f32) {
    mesh.positions
        .chunks_exact(3)
        .fold((f32::MAX, f32::MIN), |(lo, hi), p| {
            (lo.min(p[0]), hi.max(p[0]))
        })
}

fn zip_entries(bytes: &[u8]) -> Vec<(String, Vec<u8>)> {
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

#[tokio::test]
async fn one_drc_merges_all_objects_in_plate_coordinates() {
    let Some((_dir, app, user, project)) = setup().await else {
        return;
    };
    let resp = export(&app, &user, &project, serde_json::json!({})).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let disposition = resp.headers()[header::CONTENT_DISPOSITION]
        .to_str()
        .unwrap()
        .to_string();
    assert!(disposition.ends_with(".drc\""), "{disposition}");
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let mesh = decode_drc(&bytes);
    assert_eq!(mesh.indices.len() / 3, 24, "deux cubes de 12 facettes");
    // Cube A centré en x = 0, cube B en x = 50 : transformations appliquées.
    let (lo, hi) = x_range(&mesh);
    assert!(
        (lo + 10.0).abs() < 1e-3 && (hi - 60.0).abs() < 1e-3,
        "x ∈ [{lo}, {hi}]"
    );
}

#[tokio::test]
async fn split_drcs_zip_one_file_per_object_and_selection() {
    let Some((_dir, app, user, project)) = setup().await else {
        return;
    };
    let resp = export(&app, &user, &project, serde_json::json!({"split": true})).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let entries = zip_entries(&bytes);
    assert_eq!(entries.len(), 2, "un .drc par objet");
    assert!(entries.iter().all(|(name, _)| name.ends_with(".drc")));
    for (_, data) in &entries {
        assert_eq!(decode_drc(data).indices.len() / 3, 12);
    }

    // Sélection (menu objet) : seul l'objet « b » est exporté.
    let resp = export(
        &app,
        &user,
        &project,
        serde_json::json!({"split": true, "object_ids": ["b"]}),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let entries = zip_entries(&to_bytes(resp.into_body(), usize::MAX).await.unwrap());
    assert_eq!(entries.len(), 1);
    let (lo, hi) = x_range(&decode_drc(&entries[0].1));
    assert!(
        (lo - 40.0).abs() < 1e-3 && (hi - 60.0).abs() < 1e-3,
        "x ∈ [{lo}, {hi}]"
    );

    // Sélection vide de sens → 422 ; projet d'autrui → 404.
    let resp = export(
        &app,
        &user,
        &project,
        serde_json::json!({"object_ids": ["zzz"]}),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let other = register(&app, "other-drc@test.local").await;
    let resp = export(&app, &other, &project, serde_json::json!({})).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

// --- Export STL (`POST /export/stl`) : mêmes maillages libslic3r que le DRC
// (`Plater::export_stl` → `store_stl`), encodés en STL binaire.

async fn export_stl(
    app: &Router,
    cookie: &str,
    project: &str,
    body: serde_json::Value,
) -> Response {
    app.clone()
        .oneshot(post(
            &format!("/api/projects/{project}/export/stl"),
            cookie,
            body,
        ))
        .await
        .unwrap()
}

/// STL binaire : (nombre de facettes, plage x des sommets).
fn decode_stl(bytes: &[u8]) -> (usize, (f32, f32)) {
    assert!(bytes.len() >= 84, "STL binaire trop court");
    let count = u32::from_le_bytes(bytes[80..84].try_into().unwrap()) as usize;
    assert_eq!(bytes.len(), 84 + 50 * count, "taille STL binaire");
    let mut range = (f32::MAX, f32::MIN);
    for facet in bytes[84..].chunks_exact(50) {
        for v in 0..3 {
            let off = 12 + v * 12;
            let x = f32::from_le_bytes(facet[off..off + 4].try_into().unwrap());
            range = (range.0.min(x), range.1.max(x));
        }
    }
    (count, range)
}

#[tokio::test]
async fn one_stl_merges_all_objects_in_plate_coordinates() {
    let Some((_dir, app, user, project)) = setup().await else {
        return;
    };
    let resp = export_stl(&app, &user, &project, serde_json::json!({})).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let disposition = resp.headers()[header::CONTENT_DISPOSITION]
        .to_str()
        .unwrap()
        .to_string();
    assert!(disposition.ends_with(".stl\""), "{disposition}");
    let (count, (lo, hi)) = decode_stl(&to_bytes(resp.into_body(), usize::MAX).await.unwrap());
    assert_eq!(count, 24, "deux cubes de 12 facettes");
    assert!(
        (lo + 10.0).abs() < 1e-3 && (hi - 60.0).abs() < 1e-3,
        "x ∈ [{lo}, {hi}]"
    );
}

#[tokio::test]
async fn split_stls_zip_one_file_per_object_and_selection() {
    let Some((_dir, app, user, project)) = setup().await else {
        return;
    };
    let resp = export_stl(&app, &user, &project, serde_json::json!({"split": true})).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let entries = zip_entries(&to_bytes(resp.into_body(), usize::MAX).await.unwrap());
    assert_eq!(entries.len(), 2, "un .stl par objet");
    assert!(entries.iter().all(|(name, _)| name.ends_with(".stl")));
    for (_, data) in &entries {
        assert_eq!(decode_stl(data).0, 12);
    }
    // Menu objet « Export as one STL » sur « b » : fichier nommé d'après l'objet
    // (nom du ModelObject = fichier importé, comme Orca).
    let resp = export_stl(
        &app,
        &user,
        &project,
        serde_json::json!({"object_ids": ["b"]}),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let disposition = resp.headers()[header::CONTENT_DISPOSITION]
        .to_str()
        .unwrap()
        .to_string();
    assert!(
        disposition.ends_with("filename=\"cube.stl\""),
        "{disposition}"
    );
    let (count, (lo, hi)) = decode_stl(&to_bytes(resp.into_body(), usize::MAX).await.unwrap());
    assert_eq!(count, 12);
    assert!(
        (lo - 40.0).abs() < 1e-3 && (hi - 60.0).abs() < 1e-3,
        "x ∈ [{lo}, {hi}]"
    );
}
