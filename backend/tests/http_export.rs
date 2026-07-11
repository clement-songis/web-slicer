//! Export de projet 3MF (T072, FR-044) : archive OPC compatible OrcaSlicer
//! (géométrie STL + configuration figée), pièce jointe, isolation inter-comptes.

use std::io::Read;
use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{header, Request, StatusCode};
use axum::response::Response;
use axum::Router;
use backend::adapters::files::FileStore;
use backend::adapters::storage::sqlite::SqliteStorage;
use backend::domain::repo::{NewModel, NewProject};
use backend::domain::{ModelFormat, ProjectId, Storage, UserId};
use serde_json::json;
use tower::ServiceExt;
use tower_sessions::{MemoryStore, SessionManagerLayer};

/// STL ASCII : un seul triangle.
const STL: &str = "solid t
facet normal 0 0 1
 outer loop
  vertex 0 0 0
  vertex 1 0 0
  vertex 0 1 0
 endloop
endfacet
endsolid t
";

struct Harness {
    _dir: tempfile::TempDir,
    storage: Arc<SqliteStorage>,
    files: FileStore,
    app: Router,
}

async fn harness() -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}", dir.path().join("export.db").display());
    let storage = Arc::new(SqliteStorage::connect(&url).await.unwrap());
    let files = FileStore::new(dir.path().join("data"));
    let state = backend::http::state::AppState::new(
        Arc::clone(&storage) as Arc<dyn Storage>,
        files.clone(),
    );
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

/// Crée un projet avec un modèle STL stocké ; renvoie l'id projet.
async fn seed_project(h: &Harness, owner: UserId) -> ProjectId {
    let project = h
        .storage
        .projects()
        .create(
            owner,
            NewProject {
                name: "My Part".into(),
                scene: json!({}),
                active_presets: json!({}),
                thumbnail_path: None,
            },
        )
        .await
        .unwrap();
    // Clé de stockage propre pour nommer le fichier ; `file_path` fait foi.
    let path = h
        .files
        .write_model(
            owner,
            backend::domain::ModelId::new(),
            "stl",
            STL.as_bytes(),
        )
        .await
        .unwrap();
    h.storage
        .models()
        .create(
            owner,
            NewModel {
                project_id: Some(project.id),
                filename: "part.stl".into(),
                format: ModelFormat::Stl,
                file_path: path.to_string_lossy().into_owned(),
                mesh_path: None,
                size_bytes: STL.len() as i64,
                triangle_count: 1,
                repair_report: None,
            },
        )
        .await
        .unwrap();
    project.id
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

#[tokio::test]
async fn exports_a_valid_3mf_project() {
    let h = harness().await;
    let session = register(&h.app, "boss@test.local").await;
    let owner = user_id(&h.storage, "boss@test.local").await;
    let project = seed_project(&h, owner).await;

    let resp = get(
        &h.app,
        &format!("/api/projects/{project}/export/3mf"),
        &session,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Pièce jointe nommée d'après le projet (nom assaini).
    let disposition = resp
        .headers()
        .get(header::CONTENT_DISPOSITION)
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    assert!(disposition.contains("My_Part.3mf"), "{disposition}");

    // L'archive ZIP contient les entrées OPC standard + géométrie.
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes.as_ref())).unwrap();
    let names: Vec<String> = (0..zip.len())
        .map(|i| zip.by_index(i).unwrap().name().to_string())
        .collect();
    for expected in ["[Content_Types].xml", "_rels/.rels", "3D/3dmodel.model"] {
        assert!(
            names.iter().any(|n| n == expected),
            "manque {expected} dans {names:?}"
        );
    }
    let mut model_xml = String::new();
    zip.by_name("3D/3dmodel.model")
        .unwrap()
        .read_to_string(&mut model_xml)
        .unwrap();
    // Le triangle du STL est présent (3 sommets, 1 triangle).
    assert_eq!(model_xml.matches("<vertex ").count(), 3);
    assert_eq!(model_xml.matches("<triangle ").count(), 1);
}

#[tokio::test]
async fn other_accounts_project_export_is_404() {
    let h = harness().await;
    register(&h.app, "alice@test.local").await;
    let alice = user_id(&h.storage, "alice@test.local").await;
    let project = seed_project(&h, alice).await;

    let bob = register(&h.app, "bob@test.local").await;
    let resp = get(&h.app, &format!("/api/projects/{project}/export/3mf"), &bob).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
