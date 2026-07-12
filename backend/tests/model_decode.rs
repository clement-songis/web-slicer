//! Gate d'intégration du **pipeline de décodage serveur** (T128, retournement
//! R7) : upload HTTP d'un modèle → le backend lance le process `engine-worker`
//! (T124 spawn) → conversion moteur → `GET …/mesh` sert le WSMh (200). Un
//! fichier corrompu → conversion en échec → `/mesh` 422 et le backend **reste
//! debout** (isolation moteur, constitution).
//!
//! Exercice réel du worker : le binaire `engine-worker` (`required-features =
//! ["ffi"]`) doit être bâti (`cargo build -p engine --bin engine-worker
//! --features ffi`). Absent, le test s'ignore proprement — même logique que les
//! tests moteur `#[cfg(feature = "ffi")]`.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use axum::body::{to_bytes, Body};
use axum::http::{header, Request, StatusCode};
use axum::response::Response;
use axum::Router;
use tower::ServiceExt;
use tower_sessions::{MemoryStore, SessionManagerLayer};

use backend::adapters::files::FileStore;
use backend::adapters::storage::sqlite::SqliteStorage;
use backend::http::routes::router;
use backend::http::state::AppState;

/// Chemin du binaire `engine-worker` s'il est bâti : `ENGINE_WORKER_BIN` sinon
/// à côté du binaire de test (`target/<profil>/engine-worker`).
fn worker_bin() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("ENGINE_WORKER_BIN") {
        let p = PathBuf::from(p);
        return p.exists().then_some(p);
    }
    let profile_dir = std::env::current_exe()
        .ok()?
        .parent()?
        .parent()?
        .to_path_buf();
    let bin = profile_dir.join("engine-worker");
    bin.exists().then_some(bin)
}

/// Récupère le binaire ou ignore le test s'il n'est pas bâti.
macro_rules! worker_or_skip {
    () => {
        match worker_bin() {
            Some(bin) => bin,
            None => {
                eprintln!(
                    "engine-worker absent — test ignoré \
                     (bâtir : cargo build -p engine --bin engine-worker --features ffi)"
                );
                return;
            }
        }
    };
}

/// Monte l'app avec un convertisseur réel : `AppState::new` lit
/// `ENGINE_WORKER_BIN`, positionné sur le binaire résolu.
async fn app_with_worker(bin: &PathBuf) -> (tempfile::TempDir, Router) {
    // Le convertisseur (WorkerMeshDecoder) résout le binaire à la construction.
    std::env::set_var("ENGINE_WORKER_BIN", bin);
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}", dir.path().join("decode.db").display());
    let storage = Arc::new(SqliteStorage::connect(&url).await.unwrap());
    let files = FileStore::new(dir.path().join("data"));
    let state = AppState::new(storage, files);
    let session_layer = SessionManagerLayer::new(MemoryStore::default()).with_secure(false);
    (dir, router(state, session_layer))
}

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../engine/tests/fixtures"
    ))
    .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("fixture {name} : {e}"))
}

fn multipart(filename: &str, content: &[u8]) -> (String, Vec<u8>) {
    let boundary = "----webslicerdecodeboundary";
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
    resp.headers()
        .get(header::SET_COOKIE)
        .expect("Set-Cookie")
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_string()
}

async fn json_body(resp: Response) -> serde_json::Value {
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
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
                    serde_json::json!({ "name": "decode" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    json_body(resp).await["id"].as_str().unwrap().to_string()
}

async fn upload(app: &Router, project: &str, cookie: &str, name: &str, content: &[u8]) -> String {
    let (ct, body) = multipart(name, content);
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/projects/{project}/models"))
                .header(header::CONTENT_TYPE, ct)
                .header(header::COOKIE, cookie)
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED, "upload {name}");
    json_body(resp).await["id"].as_str().unwrap().to_string()
}

/// Interroge `/mesh` jusqu'à ce que la conversion aboutisse ou échoue (sortie du
/// 409 « en cours »), avec un plafond de temps. Renvoie (statut, corps).
async fn poll_mesh(app: &Router, cookie: &str, model: &str) -> (StatusCode, Vec<u8>) {
    for _ in 0..200 {
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/models/{model}/mesh"))
                    .header(header::COOKIE, cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = resp.status();
        if status != StatusCode::CONFLICT {
            let body = to_bytes(resp.into_body(), usize::MAX)
                .await
                .unwrap()
                .to_vec();
            return (status, body);
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("/mesh reste en 409 : la conversion n'a jamais abouti");
}

async fn health(app: &Router) -> StatusCode {
    app.clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
        .status()
}

#[tokio::test]
async fn decodes_all_formats_via_worker_to_wsmh() {
    let bin = worker_or_skip!();
    let (_d, app) = app_with_worker(&bin).await;
    let cookie = register(&app, "boss@test.local").await;
    let project = create_project(&app, &cookie).await;

    for (name, content) in [
        ("cube20.stl", fixture("cube20.stl")),
        ("cube20.obj", fixture("cube20.obj")),
        ("orca_project.3mf", fixture("orca_project.3mf")),
        ("cube20.step", fixture("cube20.step")),
    ] {
        let model = upload(&app, &project, &cookie, name, &content).await;
        let (status, body) = poll_mesh(&app, &cookie, &model).await;
        assert_eq!(status, StatusCode::OK, "{name} : conversion → 200");
        assert_eq!(&body[0..4], b"WSMh", "{name} : en-tête WSMh");
        // index_count (u32 à l'offset 12) > 0 → maillage non vide.
        let index_count = u32::from_le_bytes([body[12], body[13], body[14], body[15]]);
        assert!(index_count > 0, "{name} : maillage non vide");
    }
}

#[tokio::test]
async fn corrupt_file_is_unprocessable_and_backend_survives() {
    let bin = worker_or_skip!();
    let (_d, app) = app_with_worker(&bin).await;
    let cookie = register(&app, "boss@test.local").await;
    let project = create_project(&app, &cookie).await;

    // OBJ syntaxiquement valide (UTF-8) mais **sans géométrie** : passe la
    // validation d'upload (léger contrôle de contenu) puis fait échouer le
    // décodage moteur (aucun maillage) — exerce le chemin de conversion, pas le
    // rejet à l'upload.
    let model = upload(
        &app,
        &project,
        &cookie,
        "broken.obj",
        b"# OBJ sans aucune geometrie\no vide\n",
    )
    .await;

    let (status, _body) = poll_mesh(&app, &cookie, &model).await;
    assert_eq!(
        status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "décodage échoué → 422"
    );
    // Le crash moteur est contenu : le backend répond toujours.
    assert_eq!(
        health(&app).await,
        StatusCode::OK,
        "backend debout après échec"
    );
}
