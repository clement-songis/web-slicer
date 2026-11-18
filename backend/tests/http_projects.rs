//! Endpoints projets (T030) de bout en bout : CRUD, duplicate/rename, verrou
//! optimiste (409), et **isolation inter-comptes** (SC-008 : un projet d'autrui
//! répond 404, jamais 403 — l'existence n'est pas révélée).

use axum::body::{to_bytes, Body};
use axum::http::{header, Request, StatusCode};
use axum::response::Response;
use axum::Router;
use backend::adapters::files::FileStore;
use backend::adapters::storage::AnyStorage;
use backend::domain::{ModelFormat, ModelId, NewModel, ProjectId, Storage};
use backend::http::routes::router;
use backend::http::state::AppState;
use std::sync::Arc;
use tower::ServiceExt;
use tower_sessions::{MemoryStore, SessionManagerLayer};

async fn app() -> (tempfile::TempDir, Router) {
    let (dir, app, _, _) = app_with_storage().await;
    (dir, app)
}

async fn app_with_storage() -> (tempfile::TempDir, Router, Arc<AnyStorage>, FileStore) {
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}", dir.path().join("projects.db").display());
    let storage = Arc::new(AnyStorage::connect_test(&url).await.unwrap());
    let files = FileStore::new(dir.path().join("data"));
    let state = AppState::new(storage.clone(), files.clone());
    let session_layer = SessionManagerLayer::new(MemoryStore::default()).with_secure(false);
    (dir, router(state, session_layer), storage, files)
}

fn request(
    method: &str,
    uri: &str,
    body: serde_json::Value,
    cookie: Option<&str>,
) -> Request<Body> {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(c) = cookie {
        builder = builder.header(header::COOKIE, c);
    }
    builder.body(Body::from(body.to_string())).unwrap()
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

/// Inscrit un compte, renvoie son cookie de session.
async fn register(app: &Router, email: &str) -> String {
    let resp = app
        .clone()
        .oneshot(request(
            "POST",
            "/api/auth/register",
            serde_json::json!({ "email": email, "password": "password123" }),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    session_cookie(&resp)
}

async fn create_project(app: &Router, cookie: &str, name: &str) -> serde_json::Value {
    let resp = app
        .clone()
        .oneshot(request(
            "POST",
            "/api/projects",
            serde_json::json!({ "name": name }),
            Some(cookie),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    json_body(resp).await
}

#[tokio::test]
async fn create_lists_and_opens_a_project() {
    let (_d, app) = app().await;
    let user = register(&app, "boss@test.local").await;

    let project = create_project(&app, &user, "Benchy").await;
    assert_eq!(project["name"], "Benchy");
    assert_eq!(project["version"], 1);
    assert_eq!(project["scene"]["schema_version"], 1);
    assert_eq!(project["has_thumbnail"], false);
    let id = project["id"].as_str().unwrap().to_string();

    // Bibliothèque.
    let resp = app
        .clone()
        .oneshot(request(
            "GET",
            "/api/projects",
            serde_json::json!({}),
            Some(&user),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let list = json_body(resp).await;
    assert_eq!(list.as_array().unwrap().len(), 1);

    // Ouverture.
    let resp = app
        .clone()
        .oneshot(request(
            "GET",
            &format!("/api/projects/{id}"),
            serde_json::json!({}),
            Some(&user),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(json_body(resp).await["id"], id);
}

#[tokio::test]
async fn save_uses_optimistic_locking() {
    let (_d, app) = app().await;
    let user = register(&app, "boss@test.local").await;
    let project = create_project(&app, &user, "Benchy").await;
    let id = project["id"].as_str().unwrap().to_string();

    // Sauvegarde avec la bonne version (1 à la création) → 200, version 2.
    let resp = app
        .clone()
        .oneshot(request(
            "PUT",
            &format!("/api/projects/{id}"),
            serde_json::json!({
                "expected_version": 1,
                "scene": { "schema_version": 1, "objects": [1], "plates": [] },
                "active_presets": {}
            }),
            Some(&user),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(json_body(resp).await["version"], 2);

    // Rejouer avec la version périmée → 409.
    let resp = app
        .clone()
        .oneshot(request(
            "PUT",
            &format!("/api/projects/{id}"),
            serde_json::json!({
                "expected_version": 1,
                "scene": { "schema_version": 1, "objects": [], "plates": [] },
                "active_presets": {}
            }),
            Some(&user),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn duplicate_and_rename() {
    let (_d, app) = app().await;
    let user = register(&app, "boss@test.local").await;
    let project = create_project(&app, &user, "Benchy").await;
    let id = project["id"].as_str().unwrap().to_string();

    // Duplication.
    let resp = app
        .clone()
        .oneshot(request(
            "POST",
            &format!("/api/projects/{id}/duplicate"),
            serde_json::json!({}),
            Some(&user),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let copy = json_body(resp).await;
    assert_eq!(copy["name"], "Benchy (copie)");
    assert_ne!(copy["id"], project["id"]);

    // Renommage.
    let resp = app
        .clone()
        .oneshot(request(
            "PATCH",
            &format!("/api/projects/{id}/rename"),
            serde_json::json!({ "name": "Benchy v2" }),
            Some(&user),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(json_body(resp).await["name"], "Benchy v2");
}

/// Lot 26 (phase 26 E-a) : `PATCH /api/projects/{id}` répondait 405.
#[tokio::test]
async fn patch_project_updates_only_given_fields() {
    let (_d, app) = app().await;
    let user = register(&app, "patch@test.local").await;
    let project = create_project(&app, &user, "Avant").await;
    let id = project["id"].as_str().unwrap().to_string();
    let uri = format!("/api/projects/{id}");
    let send = |body: serde_json::Value| {
        let app = app.clone();
        let uri = uri.clone();
        let user = user.clone();
        async move {
            app.oneshot(request("PATCH", &uri, body, Some(&user)))
                .await
                .unwrap()
        }
    };

    // Renommage seul.
    let resp = send(serde_json::json!({ "name": "  Après " })).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body["name"], "Après");
    let version = body["version"].as_i64().unwrap();

    // Presets seuls : la scène est conservée, la version avance.
    let scene_before = body["scene"].clone();
    let resp = send(serde_json::json!({
        "expected_version": version,
        "active_presets": { "printer": "p" }
    }))
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body["active_presets"]["printer"], "p");
    assert_eq!(body["scene"], scene_before);
    assert_eq!(body["version"].as_i64().unwrap(), version + 1);

    // Verrou optimiste : version périmée → 409 ; version absente → 422.
    let stale = send(serde_json::json!({ "expected_version": version, "scene": {} })).await;
    assert_eq!(stale.status(), StatusCode::CONFLICT);
    let missing = send(serde_json::json!({ "scene": {} })).await;
    assert_eq!(missing.status(), StatusCode::UNPROCESSABLE_ENTITY);
    // Corps vide, nom vide, champ inconnu → refusés.
    assert_eq!(
        send(serde_json::json!({})).await.status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        send(serde_json::json!({ "name": " " })).await.status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert!(send(serde_json::json!({ "nom": "x" }))
        .await
        .status()
        .is_client_error());

    // Projet d'autrui → 404.
    let other = register(&app, "patch-other@test.local").await;
    let resp = app
        .clone()
        .oneshot(request(
            "PATCH",
            &uri,
            serde_json::json!({ "name": "volé" }),
            Some(&other),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn duplicate_copies_models_files_meshes_and_remaps_the_scene() {
    let (_d, app, storage, files) = app_with_storage().await;
    let user = register(&app, "duplicate@test.local").await;
    let project = create_project(&app, &user, "Projet complet").await;
    let source_id = project["id"].as_str().unwrap();
    let project_id = ProjectId(uuid::Uuid::parse_str(source_id).unwrap());
    let user_id = storage.users().list().await.unwrap()[0].id;

    let source_bytes = b"solid source\nfacet normal 0 0 1\nendsolid\n".to_vec();
    let storage_key = ModelId::new();
    let file_path = files
        .write_model(user_id, storage_key, "stl", &source_bytes)
        .await
        .unwrap();
    let source_model = storage
        .models()
        .create(
            user_id,
            NewModel {
                project_id: Some(project_id),
                filename: "piece.stl".into(),
                format: ModelFormat::Stl,
                file_path: file_path.to_string_lossy().into_owned(),
                mesh_path: None,
                size_bytes: source_bytes.len() as i64,
                triangle_count: 1,
                repair_report: Some(serde_json::json!({"fixed": 2})),
            },
        )
        .await
        .unwrap();
    let mesh_bytes = engine::api::TriangleMesh {
        vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        indices: vec![[0, 1, 2]],
    }
    .encode_display();
    let mesh_path = files
        .write_mesh(user_id, source_model.id, &mesh_bytes)
        .await
        .unwrap();
    storage
        .models()
        .set_mesh(user_id, source_model.id, &mesh_path.to_string_lossy(), 1)
        .await
        .unwrap();

    let save = app
        .clone()
        .oneshot(request(
            "PUT",
            &format!("/api/projects/{source_id}"),
            serde_json::json!({
                "expected_version": 1,
                "scene": {
                    "schema_version": 1,
                    "objects": [{
                        "id": "object-source",
                        "modelId": source_model.id.to_string(),
                        "settings": {"wall_loops": 4}
                    }],
                    "plates": {
                        "activeId": "plate-source",
                        "plates": [{
                            "id": "plate-source",
                            "name": "Plateau A",
                            "objectIds": ["object-source"]
                        }]
                    }
                },
                "active_presets": {"process": "preset-partage"}
            }),
            Some(&user),
        ))
        .await
        .unwrap();
    assert_eq!(save.status(), StatusCode::OK);

    let response = app
        .clone()
        .oneshot(request(
            "POST",
            &format!("/api/projects/{source_id}/duplicate"),
            serde_json::json!({}),
            Some(&user),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let copy = json_body(response).await;
    let copy_id = copy["id"].as_str().unwrap();
    assert_ne!(copy_id, source_id);
    let copied_object = &copy["scene"]["objects"][0];
    let copied_model_id = copied_object["modelId"].as_str().unwrap();
    assert_ne!(copied_model_id, source_model.id.to_string());
    assert_ne!(copied_object["id"], "object-source");
    assert_eq!(copied_object["settings"]["wall_loops"], 4);
    assert_ne!(copy["scene"]["plates"]["activeId"], "plate-source");
    assert_eq!(
        copy["scene"]["plates"]["plates"][0]["objectIds"][0],
        copied_object["id"]
    );
    assert_eq!(copy["active_presets"]["process"], "preset-partage");

    let copied = storage
        .models()
        .get(
            user_id,
            ModelId(uuid::Uuid::parse_str(copied_model_id).unwrap()),
        )
        .await
        .unwrap();
    assert_ne!(copied.file_path, source_model.file_path);
    assert_ne!(
        copied.mesh_path,
        Some(mesh_path.to_string_lossy().into_owned())
    );
    assert_eq!(copied.repair_report, source_model.repair_report);
    assert_eq!(
        files
            .read(std::path::Path::new(&copied.file_path))
            .await
            .unwrap(),
        source_bytes
    );
    assert_eq!(
        files
            .read(std::path::Path::new(copied.mesh_path.as_deref().unwrap()))
            .await
            .unwrap(),
        mesh_bytes
    );

    // Les artefacts sont physiquement indépendants, pas des chemins partagés.
    tokio::fs::write(&copied.file_path, b"solid copy\nendsolid\n")
        .await
        .unwrap();
    assert_eq!(
        files
            .read(std::path::Path::new(&source_model.file_path))
            .await
            .unwrap(),
        source_bytes
    );
    let original = storage.projects().get(user_id, project_id).await.unwrap();
    assert_eq!(original.scene["objects"][0]["id"], "object-source");
    assert_eq!(
        original.scene["objects"][0]["modelId"],
        source_model.id.to_string()
    );
}

#[tokio::test]
async fn delete_removes_the_project() {
    let (_d, app) = app().await;
    let user = register(&app, "boss@test.local").await;
    let project = create_project(&app, &user, "Benchy").await;
    let id = project["id"].as_str().unwrap().to_string();

    let resp = app
        .clone()
        .oneshot(request(
            "DELETE",
            &format!("/api/projects/{id}"),
            serde_json::json!({}),
            Some(&user),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let resp = app
        .clone()
        .oneshot(request(
            "GET",
            &format!("/api/projects/{id}"),
            serde_json::json!({}),
            Some(&user),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

/// SC-008 : un compte ne voit ni ne touche les projets d'un autre — 404 partout,
/// jamais 403 (l'existence de la ressource ne fuite pas).
#[tokio::test]
async fn projects_are_isolated_between_accounts() {
    let (_d, app) = app().await;
    let alice = register(&app, "alice@test.local").await;
    let bob = register(&app, "bob@test.local").await;

    let project = create_project(&app, &alice, "Secret d'Alice").await;
    let id = project["id"].as_str().unwrap().to_string();

    // La bibliothèque de Bob est vide.
    let resp = app
        .clone()
        .oneshot(request(
            "GET",
            "/api/projects",
            serde_json::json!({}),
            Some(&bob),
        ))
        .await
        .unwrap();
    assert_eq!(json_body(resp).await.as_array().unwrap().len(), 0);

    // Toutes les opérations de Bob sur le projet d'Alice → 404.
    let get = request(
        "GET",
        &format!("/api/projects/{id}"),
        serde_json::json!({}),
        Some(&bob),
    );
    let del = request(
        "DELETE",
        &format!("/api/projects/{id}"),
        serde_json::json!({}),
        Some(&bob),
    );
    let dup = request(
        "POST",
        &format!("/api/projects/{id}/duplicate"),
        serde_json::json!({}),
        Some(&bob),
    );
    let ren = request(
        "PATCH",
        &format!("/api/projects/{id}/rename"),
        serde_json::json!({ "name": "x" }),
        Some(&bob),
    );
    let put = request(
        "PUT",
        &format!("/api/projects/{id}"),
        serde_json::json!({ "expected_version": 0, "scene": {}, "active_presets": {} }),
        Some(&bob),
    );
    for req in [get, del, dup, ren, put] {
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::NOT_FOUND,
            "opération inter-comptes doit répondre 404 (SC-008)"
        );
    }

    // Le projet d'Alice reste intact.
    let resp = app
        .clone()
        .oneshot(request(
            "GET",
            &format!("/api/projects/{id}"),
            serde_json::json!({}),
            Some(&alice),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(json_body(resp).await["name"], "Secret d'Alice");
}

#[tokio::test]
async fn projects_require_a_session() {
    let (_d, app) = app().await;
    let resp = app
        .oneshot(request("GET", "/api/projects", serde_json::json!({}), None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

/// « Split to parts » : la duplication remappe aussi le `modelId` des volumes
/// maillés vers les modèles copiés du nouveau projet.
#[tokio::test]
async fn duplicate_remaps_mesh_volume_model_ids() {
    let (_d, app, storage, files) = app_with_storage().await;
    let user = register(&app, "duplicate-parts@test.local").await;
    let project = create_project(&app, &user, "Projet pièces").await;
    let source_id = project["id"].as_str().unwrap();
    let project_id = ProjectId(uuid::Uuid::parse_str(source_id).unwrap());
    let user_id = storage.users().list().await.unwrap()[0].id;
    let mut ids = Vec::new();
    for name in ["base.stl", "piece_2.stl"] {
        let bytes = b"solid p\nendsolid\n".to_vec();
        let path = files
            .write_model(user_id, ModelId::new(), "stl", &bytes)
            .await
            .unwrap();
        let model = storage
            .models()
            .create(
                user_id,
                NewModel {
                    project_id: Some(project_id),
                    filename: name.into(),
                    format: ModelFormat::Stl,
                    file_path: path.to_string_lossy().into_owned(),
                    mesh_path: None,
                    size_bytes: bytes.len() as i64,
                    triangle_count: 1,
                    repair_report: None,
                },
            )
            .await
            .unwrap();
        ids.push(model.id.to_string());
    }
    let save = app
        .clone()
        .oneshot(request(
            "PUT",
            &format!("/api/projects/{source_id}"),
            serde_json::json!({
                "expected_version": 1,
                "scene": {
                    "schema_version": 1,
                    "objects": [{
                        "id": "o", "modelId": ids[0],
                        "volumes": [{
                            "id": "v", "type": "part", "name": "piece_2", "shape": "mesh",
                            "modelId": ids[1], "size": [10, 10, 10], "offset": [20, 0, 0],
                            "settings": {}
                        }, {
                            "id": "m", "type": "modifier", "name": "Modificateur", "shape": "box",
                            "size": [5, 5, 5], "offset": [0, 0, 0], "settings": {}
                        }]
                    }],
                    "plates": {"activeId": "p", "plates": [{"id": "p", "name": "A", "objectIds": ["o"]}]}
                },
                "active_presets": {}
            }),
            Some(&user),
        ))
        .await
        .unwrap();
    assert_eq!(save.status(), StatusCode::OK);

    let response = app
        .clone()
        .oneshot(request(
            "POST",
            &format!("/api/projects/{source_id}/duplicate"),
            serde_json::json!({}),
            Some(&user),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let copy = json_body(response).await;
    let copy_id = ProjectId(uuid::Uuid::parse_str(copy["id"].as_str().unwrap()).unwrap());
    let copied: Vec<String> = storage
        .models()
        .list(user_id, Some(copy_id))
        .await
        .unwrap()
        .iter()
        .map(|m| m.id.to_string())
        .collect();
    assert_eq!(copied.len(), 2);
    let object = &copy["scene"]["objects"][0];
    let volume = &object["volumes"][0];
    let volume_model = volume["modelId"].as_str().unwrap();
    assert_ne!(volume_model, ids[1], "modelId du volume non remappé");
    assert!(copied.iter().any(|id| id == volume_model));
    assert_ne!(volume_model, object["modelId"].as_str().unwrap());
    // Volume pavé : aucune référence de modèle ajoutée.
    assert!(object["volumes"][1].get("modelId").is_none());
}
