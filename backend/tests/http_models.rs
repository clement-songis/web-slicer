//! Upload de modèles (T048) de bout en bout : formats acceptés, détection,
//! contrôle de contenu (corrompu → 422), limite de taille (413), isolation
//! inter-comptes (projet d'autrui → 404, SC-008).

use axum::body::{to_bytes, Body};
use axum::http::{header, Request, StatusCode};
use axum::response::Response;
use axum::Router;
use backend::adapters::files::FileStore;
use backend::adapters::storage::sqlite::SqliteStorage;
use backend::http::routes::router;
use backend::http::state::AppState;
use std::sync::Arc;
use tower::ServiceExt;
use tower_sessions::{MemoryStore, SessionManagerLayer};

async fn app() -> (tempfile::TempDir, Router) {
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}", dir.path().join("models.db").display());
    let storage = Arc::new(SqliteStorage::connect(&url).await.unwrap());
    let files = FileStore::new(dir.path().join("data"));
    let state = AppState::new(storage, files);
    let session_layer = SessionManagerLayer::new(MemoryStore::default()).with_secure(false);
    (dir, router(state, session_layer))
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

/// Construit un corps multipart avec un unique champ fichier.
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

fn upload_req(project: &str, cookie: &str, filename: &str, content: &[u8]) -> Request<Body> {
    let (ct, body) = multipart(filename, content);
    Request::builder()
        .method("POST")
        .uri(format!("/api/projects/{project}/models"))
        .header(header::CONTENT_TYPE, ct)
        .header(header::COOKIE, cookie)
        .body(Body::from(body))
        .unwrap()
}

/// Requête d'import de projet (`POST /api/projects/import`, T090).
fn import_req(cookie: &str, filename: &str, content: &[u8]) -> Request<Body> {
    let (ct, body) = multipart(filename, content);
    Request::builder()
        .method("POST")
        .uri("/api/projects/import")
        .header(header::CONTENT_TYPE, ct)
        .header(header::COOKIE, cookie)
        .body(Body::from(body))
        .unwrap()
}

/// STL binaire : en-tête 80 o + u32 (n triangles) + n×50 o.
fn binary_stl(n: u32) -> Vec<u8> {
    let mut v = vec![0u8; 80];
    v.extend_from_slice(&n.to_le_bytes());
    v.extend(std::iter::repeat(0u8).take(n as usize * 50));
    v
}

#[tokio::test]
async fn uploads_binary_stl_and_counts_triangles() {
    let (_d, app) = app().await;
    let user = register(&app, "boss@test.local").await;
    let project = create_project(&app, &user).await;

    let resp = app
        .clone()
        .oneshot(upload_req(&project, &user, "cube.stl", &binary_stl(12)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let model = json_body(resp).await;
    assert_eq!(model["format"], "stl");
    assert_eq!(model["triangle_count"], 12);
    // T124 : tout format passe par la conversion moteur (source unique).
    assert_eq!(model["conversion_pending"], true);
    assert_eq!(model["project_id"], project);
}

#[tokio::test]
async fn imports_model_as_new_project() {
    let (_d, app) = app().await;
    let user = register(&app, "boss@test.local").await;

    let resp = app
        .clone()
        .oneshot(import_req(&user, "Benchy.stl", &binary_stl(4)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let project = json_body(resp).await;
    // Nom dérivé du nom de fichier (sans extension).
    assert_eq!(project["name"], "Benchy");
    let pid = project["id"].as_str().unwrap().to_string();

    // Le projet apparaît dans la bibliothèque du compte.
    let list = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/projects")
                .header(header::COOKIE, &user)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let projects = json_body(list).await;
    assert!(projects.as_array().unwrap().iter().any(|p| p["id"] == pid));
}

#[tokio::test]
async fn import_rejects_unsupported_format() {
    let (_d, app) = app().await;
    let user = register(&app, "boss@test.local").await;
    let resp = app
        .clone()
        .oneshot(import_req(&user, "notes.gcode", b"G28\n"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn import_requires_authentication() {
    let (_d, app) = app().await;
    let resp = app
        .clone()
        .oneshot(import_req("", "part.stl", &binary_stl(1)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn uploads_obj_and_step() {
    let (_d, app) = app().await;
    let user = register(&app, "boss@test.local").await;
    let project = create_project(&app, &user).await;

    let obj = app
        .clone()
        .oneshot(upload_req(
            &project,
            &user,
            "part.obj",
            b"v 0 0 0\nf 1 1 1\n",
        ))
        .await
        .unwrap();
    assert_eq!(obj.status(), StatusCode::CREATED);
    assert_eq!(json_body(obj).await["format"], "obj");

    // STEP : accepté, en attente de conversion mesh (R7).
    let step = app
        .clone()
        .oneshot(upload_req(
            &project,
            &user,
            "part.step",
            b"ISO-10303-21;\nHEADER;\nENDSEC;\n",
        ))
        .await
        .unwrap();
    assert_eq!(step.status(), StatusCode::CREATED);
    let model = json_body(step).await;
    assert_eq!(model["format"], "step");
    assert_eq!(model["conversion_pending"], true);
    assert_eq!(model["has_mesh"], false);
}

#[tokio::test]
async fn rejects_unsupported_and_corrupt() {
    let (_d, app) = app().await;
    let user = register(&app, "boss@test.local").await;
    let project = create_project(&app, &user).await;

    // Extension non supportée.
    let gif = app
        .clone()
        .oneshot(upload_req(&project, &user, "img.gif", b"GIF89a"))
        .await
        .unwrap();
    assert_eq!(gif.status(), StatusCode::UNPROCESSABLE_ENTITY);

    // 3MF corrompu (pas une archive ZIP).
    let bad3mf = app
        .clone()
        .oneshot(upload_req(&project, &user, "broken.3mf", b"not a zip"))
        .await
        .unwrap();
    assert_eq!(bad3mf.status(), StatusCode::UNPROCESSABLE_ENTITY);

    // Fichier vide.
    let empty = app
        .clone()
        .oneshot(upload_req(&project, &user, "empty.stl", b""))
        .await
        .unwrap();
    assert_eq!(empty.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn rejects_oversized_beyond_instance_limit() {
    let (_d, app) = app().await;
    let admin = register(&app, "admin@test.local").await; // 1er compte = admin
    let project = create_project(&app, &admin).await;

    // Abaisse la limite d'upload de l'instance à 10 octets.
    let patch = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/api/admin/instance")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &admin)
                .body(Body::from(
                    serde_json::json!({ "upload_limit_bytes": 10 }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(patch.status(), StatusCode::OK);

    let resp = app
        .clone()
        .oneshot(upload_req(&project, &admin, "cube.stl", &binary_stl(12)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

/// SC-008 : Bob ne peut pas déposer dans le projet d'Alice (404, jamais 403).
#[tokio::test]
async fn upload_into_others_project_is_404() {
    let (_d, app) = app().await;
    let alice = register(&app, "alice@test.local").await;
    let bob = register(&app, "bob@test.local").await;
    let project = create_project(&app, &alice).await;

    let resp = app
        .clone()
        .oneshot(upload_req(&project, &bob, "cube.stl", &binary_stl(1)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

async fn upload_model(
    app: &Router,
    project: &str,
    cookie: &str,
    name: &str,
    body: &[u8],
) -> String {
    let resp = app
        .clone()
        .oneshot(upload_req(project, cookie, name, body))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    json_body(resp).await["id"].as_str().unwrap().to_string()
}

/// Un STL binaire avec une facette réelle (pour un maillage non vide).
fn binary_stl_triangle() -> Vec<u8> {
    let mut v = vec![0u8; 80];
    v.extend_from_slice(&1u32.to_le_bytes());
    for f in [0.0f32, 0.0, 1.0] {
        v.extend_from_slice(&f.to_le_bytes());
    }
    for f in [0.0f32, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0] {
        v.extend_from_slice(&f.to_le_bytes());
    }
    v.extend_from_slice(&0u16.to_le_bytes());
    v
}

#[tokio::test]
async fn serves_stl_mesh_as_compact_binary() {
    let (_d, app) = app().await;
    let user = register(&app, "boss@test.local").await;
    let project = create_project(&app, &user).await;
    let model = upload_model(&app, &project, &user, "cube.stl", &binary_stl_triangle()).await;

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/models/{model}/mesh"))
                .header(header::COOKIE, &user)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        resp.headers().get(header::CONTENT_TYPE).unwrap(),
        "application/octet-stream"
    );
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    // En-tête « WSMh » + version 1 + 3 sommets + 3 indices.
    assert_eq!(&bytes[0..4], b"WSMh");
    let vertex_count = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
    let index_count = u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]);
    assert_eq!(vertex_count, 3);
    assert_eq!(index_count, 3);
    assert_eq!(bytes.len(), 16 + 9 * 4 + 9 * 4 + 3 * 4);
}

#[tokio::test]
async fn step_mesh_is_conflict_until_converted() {
    let (_d, app) = app().await;
    let user = register(&app, "boss@test.local").await;
    let project = create_project(&app, &user).await;
    let model = upload_model(
        &app,
        &project,
        &user,
        "part.step",
        b"ISO-10303-21;\nHEADER;\nENDSEC;\n",
    )
    .await;

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/models/{model}/mesh"))
                .header(header::COOKIE, &user)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CONFLICT);
}

/// SC-008 : le maillage d'un modèle d'autrui répond 404.
#[tokio::test]
async fn mesh_of_others_model_is_404() {
    let (_d, app) = app().await;
    let alice = register(&app, "alice@test.local").await;
    let bob = register(&app, "bob@test.local").await;
    let project = create_project(&app, &alice).await;
    let model = upload_model(&app, &project, &alice, "cube.stl", &binary_stl_triangle()).await;

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/models/{model}/mesh"))
                .header(header::COOKIE, &bob)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

/// T092 : la scène est repeuplée à l'ouverture — le projet expose la liste de
/// ses modèles.
#[tokio::test]
async fn lists_models_of_a_project() {
    let (_d, app) = app().await;
    let user = register(&app, "boss@test.local").await;
    let project = create_project(&app, &user).await;
    upload_model(&app, &project, &user, "a.stl", &binary_stl_triangle()).await;
    upload_model(&app, &project, &user, "b.obj", b"v 0 0 0\nf 1 1 1\n").await;

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/projects/{project}/models"))
                .header(header::COOKIE, &user)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let list = json_body(resp).await;
    let arr = list.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    assert!(arr.iter().all(|m| m["project_id"] == project));
}

/// SC-008 : la liste des modèles d'un projet d'autrui répond 404.
#[tokio::test]
async fn list_models_of_others_project_is_404() {
    let (_d, app) = app().await;
    let alice = register(&app, "alice@test.local").await;
    let bob = register(&app, "bob@test.local").await;
    let project = create_project(&app, &alice).await;

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/projects/{project}/models"))
                .header(header::COOKIE, &bob)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

/// T092 : le fichier source brut est servi pour un aperçu client (OBJ/3MF…).
#[tokio::test]
async fn downloads_raw_model_file() {
    let (_d, app) = app().await;
    let user = register(&app, "boss@test.local").await;
    let project = create_project(&app, &user).await;
    let obj = b"v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n";
    let model = upload_model(&app, &project, &user, "part.obj", obj).await;

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/models/{model}/file"))
                .header(header::COOKIE, &user)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    assert_eq!(bytes.as_ref(), obj.as_ref());
}

/// SC-008 : le fichier d'un modèle d'autrui répond 404.
#[tokio::test]
async fn download_file_of_others_model_is_404() {
    let (_d, app) = app().await;
    let alice = register(&app, "alice@test.local").await;
    let bob = register(&app, "bob@test.local").await;
    let project = create_project(&app, &alice).await;
    let model = upload_model(&app, &project, &alice, "cube.stl", &binary_stl_triangle()).await;

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/models/{model}/file"))
                .header(header::COOKIE, &bob)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn upload_requires_a_session() {
    let (_d, app) = app().await;
    let (ct, body) = multipart("cube.stl", &binary_stl(1));
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/projects/00000000-0000-0000-0000-000000000000/models")
                .header(header::CONTENT_TYPE, ct)
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
