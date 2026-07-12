//! Route `GET /api/models/{id}/mesh` servie depuis le maillage **stocké** par la
//! conversion moteur (T125) : 200 (WSMh prêt), 409 (conversion en cours), 422
//! (conversion échouée), 404 (modèle d'autrui). Les modèles sont créés et
//! ensemencés directement via le dépôt — indépendant du process worker et de
//! toute course avec la conversion asynchrone déclenchée par l'upload.

use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{header, Request, StatusCode};
use axum::response::Response;
use axum::Router;
use tower::ServiceExt;
use tower_sessions::{MemoryStore, SessionManagerLayer};

use backend::adapters::files::FileStore;
use backend::adapters::storage::sqlite::SqliteStorage;
use backend::domain::{ModelFormat, NewModel, NewProject, Storage, UserId};
use backend::http::routes::router;
use backend::http::state::AppState;

/// Harnais exposant le stockage concret (pour ensemencer) et le routeur HTTP.
async fn harness() -> (tempfile::TempDir, Arc<SqliteStorage>, FileStore, Router) {
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}", dir.path().join("models.db").display());
    let storage = Arc::new(SqliteStorage::connect(&url).await.unwrap());
    let files = FileStore::new(dir.path().join("data"));
    let state = AppState::new(storage.clone(), files.clone());
    let session_layer = SessionManagerLayer::new(MemoryStore::default()).with_secure(false);
    let app = router(state, session_layer);
    (dir, storage, files, app)
}

/// Enregistre un compte via HTTP et renvoie (cookie de session, id utilisateur).
async fn register(app: &Router, storage: &SqliteStorage, email: &str) -> (String, UserId) {
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
    let cookie = resp
        .headers()
        .get(header::SET_COOKIE)
        .expect("Set-Cookie")
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_string();
    let user = storage
        .users()
        .find_by_email(email)
        .await
        .unwrap()
        .expect("compte créé");
    (cookie, user.id)
}

/// Crée un modèle rattaché à un nouveau projet du compte, non converti.
async fn create_model(
    storage: &SqliteStorage,
    user: UserId,
    format: ModelFormat,
) -> backend::domain::ModelId {
    let project = storage
        .projects()
        .create(
            user,
            NewProject {
                name: "P".into(),
                scene: serde_json::json!({}),
                active_presets: serde_json::json!({}),
                thumbnail_path: None,
            },
        )
        .await
        .unwrap();
    let model = storage
        .models()
        .create(
            user,
            NewModel {
                project_id: Some(project.id),
                filename: "piece.step".into(),
                format,
                file_path: "/data/piece.step".into(),
                mesh_path: None,
                size_bytes: 1,
                triangle_count: 0,
                repair_report: None,
            },
        )
        .await
        .unwrap();
    model.id
}

/// WSMh valide (1 triangle) via le codec moteur.
fn one_triangle_wsmh() -> Vec<u8> {
    engine::api::TriangleMesh {
        vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        indices: vec![[0, 1, 2]],
    }
    .encode_display()
}

async fn get_mesh(app: &Router, cookie: &str, model: &str) -> Response {
    app.clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/models/{model}/mesh"))
                .header(header::COOKIE, cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
}

#[tokio::test]
async fn ready_mesh_is_served_as_wsmh() {
    let (_d, storage, files, app) = harness().await;
    let (cookie, user) = register(&app, &storage, "boss@test.local").await;
    let model_id = create_model(&storage, user, ModelFormat::Step).await;

    // Ensemence le maillage converti : fichier WSMh + `mesh_path`.
    let wsmh = one_triangle_wsmh();
    let path = files.write_mesh(user, model_id, &wsmh).await.unwrap();
    storage
        .models()
        .set_mesh(user, model_id, &path.to_string_lossy(), 1)
        .await
        .unwrap();

    let resp = get_mesh(&app, &cookie, &model_id.to_string()).await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        resp.headers().get(header::CONTENT_TYPE).unwrap(),
        "application/octet-stream"
    );
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    assert_eq!(&bytes[0..4], b"WSMh", "en-tête WSMh");
    assert_eq!(bytes.as_ref(), wsmh.as_slice(), "octets du mesh stocké");
}

#[tokio::test]
async fn pending_conversion_is_conflict() {
    let (_d, storage, _files, app) = harness().await;
    let (cookie, user) = register(&app, &storage, "boss@test.local").await;
    // Modèle créé sans mesh ni erreur : conversion en cours.
    let model_id = create_model(&storage, user, ModelFormat::Step).await;

    let resp = get_mesh(&app, &cookie, &model_id.to_string()).await;
    assert_eq!(resp.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn failed_conversion_is_unprocessable() {
    let (_d, storage, _files, app) = harness().await;
    let (cookie, user) = register(&app, &storage, "boss@test.local").await;
    let model_id = create_model(&storage, user, ModelFormat::Step).await;
    storage
        .models()
        .mark_conversion_failed(user, model_id, "fichier illisible")
        .await
        .unwrap();

    let resp = get_mesh(&app, &cookie, &model_id.to_string()).await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["code"], "conversion_failed");
}

#[tokio::test]
async fn mesh_of_others_model_is_not_found() {
    let (_d, storage, files, app) = harness().await;
    let (_alice_cookie, alice) = register(&app, &storage, "alice@test.local").await;
    let (bob_cookie, _bob) = register(&app, &storage, "bob@test.local").await;
    let model_id = create_model(&storage, alice, ModelFormat::Step).await;
    let path = files
        .write_mesh(alice, model_id, &one_triangle_wsmh())
        .await
        .unwrap();
    storage
        .models()
        .set_mesh(alice, model_id, &path.to_string_lossy(), 1)
        .await
        .unwrap();

    // Bob demande le maillage d'Alice : 404 (isolation SC-008), même prêt.
    let resp = get_mesh(&app, &bob_cookie, &model_id.to_string()).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
