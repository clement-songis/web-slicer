//! Import de projets 3MF : géométrie, occurrences, plateaux, configuration et
//! conversions asynchrones. Les fixtures sont lues comme par le endpoint public,
//! sans appeler directement le lecteur ni injecter de scène dans la réponse.

use std::io::{Cursor, Write};
use std::sync::Arc;
use std::time::Duration;

use axum::body::{to_bytes, Body};
use axum::http::{header, Request, StatusCode};
use axum::response::Response;
use axum::Router;
use backend::adapters::files::FileStore;
use backend::adapters::storage::AnyStorage;
use backend::http::routes::router;
use backend::http::state::AppState;
use tower::ServiceExt;
use tower_sessions::{MemoryStore, SessionManagerLayer};
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

async fn app() -> (tempfile::TempDir, Router) {
    let dir = tempfile::tempdir().unwrap();
    let url = format!(
        "sqlite://{}",
        dir.path().join("project-import.db").display()
    );
    let storage = Arc::new(AnyStorage::connect_test(&url).await.unwrap());
    let files = FileStore::new(dir.path().join("data"));
    let state = AppState::new(storage, files);
    let sessions = SessionManagerLayer::new(MemoryStore::default()).with_secure(false);
    (dir, router(state, sessions))
}

fn session_cookie(response: &Response) -> String {
    response.headers()[header::SET_COOKIE]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_string()
}

async fn register(app: &Router) -> String {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/register")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::json!({"email":"import@test.local","password":"password123"})
                        .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    session_cookie(&response)
}

async fn json_body(response: Response) -> serde_json::Value {
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

fn multipart(filename: &str, bytes: &[u8], cookie: &str) -> Request<Body> {
    let boundary = "----webslicer-project-import";
    let mut body = Vec::new();
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(
        format!("Content-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\n")
            .as_bytes(),
    );
    body.extend_from_slice(b"Content-Type: model/3mf\r\n\r\n");
    body.extend_from_slice(bytes);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    Request::builder()
        .method("POST")
        .uri("/api/projects/import")
        .header(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={boundary}"),
        )
        .header(header::COOKIE, cookie)
        .body(Body::from(body))
        .unwrap()
}

async fn import_project(app: &Router, cookie: &str, filename: &str, bytes: &[u8]) -> Response {
    app.clone()
        .oneshot(multipart(filename, bytes, cookie))
        .await
        .unwrap()
}

async fn get_json(app: &Router, cookie: &str, uri: &str) -> serde_json::Value {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(uri)
                .header(header::COOKIE, cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    json_body(response).await
}

fn triangle_object(id: u32, offset: f32) -> String {
    format!(
        r#"<object id="{id}" type="model"><mesh><vertices>
           <vertex x="{offset}" y="0" z="0"/><vertex x="{}" y="1" z="0"/><vertex x="{offset}" y="0" z="1"/>
           </vertices><triangles><triangle v1="0" v2="1" v3="2"/></triangles></mesh></object>"#,
        offset + 1.0
    )
}

fn two_plate_project() -> Vec<u8> {
    let model = format!(
        r#"<?xml version="1.0"?><model unit="millimeter"><resources>{}{}</resources>
        <build><item objectid="1" transform="1 0 0 0 1 0 0 0 1 10 20 0"/>
        <item objectid="2" transform="1 0 0 0 1 0 0 0 1 30 40 0"/></build></model>"#,
        triangle_object(1, 0.0),
        triangle_object(2, 5.0)
    );
    let metadata = r#"<?xml version="1.0"?><config>
      <object id="1"><metadata key="name" value="Gauche"/><metadata key="extruder" value="1"/>
        <metadata key="wall_loops" value="7"/></object>
      <object id="2"><metadata key="name" value="Droite"/><metadata key="extruder" value="2"/></object>
      <plate><metadata key="plater_id" value="1"/><metadata key="plater_name" value="Premier"/>
        <metadata key="bed_type" value="Cool Plate"/><model_instance>
        <metadata key="object_id" value="1"/><metadata key="instance_id" value="0"/></model_instance></plate>
      <plate><metadata key="plater_id" value="2"/><metadata key="plater_name" value="Second"/>
        <metadata key="bed_type" value="Engineering Plate"/><model_instance>
        <metadata key="object_id" value="2"/><metadata key="instance_id" value="0"/></model_instance></plate>
    </config>"#;
    let config = r#"{"layer_height":"0.16","wall_loops":"5","filament_type":["PLA"],"unknown_vendor_key":"ignored"}"#;

    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default();
    // Paquet 3MF complet, tel qu'Orca l'exige (_rels/.rels) ; plateaux au
    // format bbs_3mf (bed_type) et profil de couches dans
    // layer_heights_profile.txt (index 1-based de l'objet).
    let content_types = r#"<?xml version="1.0" encoding="UTF-8"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="model" ContentType="application/vnd.ms-package.3dmanufacturing-3dmodel+xml"/></Types>"#;
    let rels = r#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Target="/3D/3dmodel.model" Id="rel-1" Type="http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel"/></Relationships>"#;
    // Orca exige au moins trois paires (z, h) par profil.
    let profile = "object_id=2|0;0.2;0.5;0.12;1;0.12\n";
    for (name, content) in [
        ("[Content_Types].xml", content_types.as_bytes()),
        ("_rels/.rels", rels.as_bytes()),
        ("3D/3dmodel.model", model.as_bytes()),
        ("Metadata/model_settings.config", metadata.as_bytes()),
        ("Metadata/project_settings.config", config.as_bytes()),
        ("Metadata/layer_heights_profile.txt", profile.as_bytes()),
    ] {
        zip.start_file(name, options).unwrap();
        zip.write_all(content).unwrap();
    }
    zip.finish().unwrap().into_inner()
}

#[tokio::test]
async fn imports_geometry_objects_plates_config_and_converts_each_object() {
    let (_dir, app) = app().await;
    let cookie = register(&app).await;
    let archive = two_plate_project();
    let response = import_project(&app, &cookie, "atelier.3mf", &archive).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let project = json_body(response).await;
    let id = project["id"].as_str().unwrap().to_string();

    assert_eq!(project["name"], "atelier");
    assert_eq!(project["scene"]["objects"].as_array().unwrap().len(), 2);
    let plates = project["scene"]["plates"]["plates"].as_array().unwrap();
    assert_eq!(plates.len(), 2);
    assert_eq!(plates[0]["name"], "Premier");
    assert_eq!(plates[1]["name"], "Second");
    assert_eq!(
        plates[0]["objectIds"][0],
        project["scene"]["objects"][0]["id"]
    );
    assert_eq!(
        plates[1]["objectIds"][0],
        project["scene"]["objects"][1]["id"]
    );
    // Réglages d'objet typés selon le schéma des options (int → nombre).
    assert_eq!(project["scene"]["objects"][0]["settings"]["wall_loops"], 7);
    assert_eq!(project["scene"]["objects"][1]["layerProfile"][2], 0.5);
    assert_eq!(project["scene"]["objects"][1]["extruder"], 2);

    let process = project["active_presets"]["process"].as_str().unwrap();
    let preset = get_json(&app, &cookie, &format!("/api/presets/{process}")).await;
    assert_eq!(preset["kind"], "process");
    assert_eq!(preset["values"]["layer_height"], "0.16");
    assert_eq!(preset["values"]["wall_loops"], "5");
    assert!(preset["values"].get("unknown_vendor_key").is_none());

    let mut models = Vec::new();
    for _ in 0..100 {
        models = get_json(&app, &cookie, &format!("/api/projects/{id}/models"))
            .await
            .as_array()
            .unwrap()
            .clone();
        if models.len() == 2 && models.iter().all(|model| model["has_mesh"] == true) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert_eq!(models.len(), 2);
    assert!(models.iter().all(|model| model["has_mesh"] == true));
    assert!(models
        .iter()
        .all(|model| model["conversion_pending"] == false));
    assert!(models.iter().all(|model| model["triangle_count"] == 1));
    assert_ne!(models[0]["id"], models[1]["id"]);

    for model in models {
        let model_id = model["id"].as_str().unwrap();
        let mesh = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/models/{model_id}/mesh"))
                    .header(header::COOKIE, &cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(mesh.status(), StatusCode::OK);
        let bytes = to_bytes(mesh.into_body(), usize::MAX).await.unwrap();
        let mesh = engine::api::DisplayMesh::decode(&bytes).unwrap();
        assert_eq!(mesh.positions.len(), 9);
        assert_eq!(mesh.indices.len(), 3);

        let source = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/models/{model_id}/file"))
                    .header(header::COOKIE, &cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(source.status(), StatusCode::OK);
        let source_bytes = to_bytes(source.into_body(), usize::MAX).await.unwrap();
        // Chaque enregistrement possède un 3MF autonome : une duplication ou
        // conversion ultérieure ne peut pas réintroduire les autres objets.
        let standalone = backend::models::threemf::read_project(&source_bytes)
            .await
            .unwrap();
        assert_eq!(standalone.objects.len(), 1);
        assert_eq!(standalone.objects[0].mesh.indices.len(), 1);
    }
}

#[tokio::test]
async fn imports_real_orca_project_with_external_component() {
    let (_dir, app) = app().await;
    let cookie = register(&app).await;
    let archive = include_bytes!("../../engine/tests/fixtures/orca_project.3mf");
    let response = import_project(&app, &cookie, "orca.3mf", archive).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let project = json_body(response).await;
    assert_eq!(project["scene"]["objects"].as_array().unwrap().len(), 1);
    assert_eq!(
        project["scene"]["plates"]["plates"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert!(project["active_presets"]["process"].is_string());
    let id = project["id"].as_str().unwrap().to_string();

    let mut converted = false;
    for _ in 0..100 {
        let models = get_json(&app, &cookie, &format!("/api/projects/{id}/models")).await;
        if models[0]["has_mesh"] == true {
            assert!(models[0]["triangle_count"].as_i64().unwrap() > 0);
            converted = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(converted, "le composant externe doit produire un maillage");
}

#[tokio::test]
async fn rejects_zip_without_model_without_creating_a_project() {
    let (_dir, app) = app().await;
    let cookie = register(&app).await;
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    zip.start_file("README", SimpleFileOptions::default())
        .unwrap();
    zip.write_all(b"not a 3MF project").unwrap();
    let bytes = zip.finish().unwrap().into_inner();

    let response = import_project(&app, &cookie, "broken.3mf", &bytes).await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let projects = get_json(&app, &cookie, "/api/projects").await;
    assert_eq!(projects.as_array().unwrap().len(), 0);
}

/// F7 : 3MF à trois objets produit par OrcaSlicer 2.4.1 (`--export-3mf` de
/// cube.stl, pyramide.stl, pont.stl) → trois objets nommés, comme Orca, à
/// l'import d'un projet comme à l'ajout dans un projet existant.
#[tokio::test]
async fn orca_three_object_3mf_gives_three_named_objects() {
    let (_dir, app) = app().await;
    let cookie = register(&app).await;
    let archive = include_bytes!("fixtures/trois-orca.3mf");

    let response = import_project(&app, &cookie, "trois-orca.3mf", archive).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let project = json_body(response).await;
    let objects = project["scene"]["objects"].as_array().unwrap();
    let mut names: Vec<&str> = objects
        .iter()
        .map(|o| o["name"].as_str().unwrap())
        .collect();
    names.sort();
    assert_eq!(names, ["cube.stl", "pont.stl", "pyramide.stl"]);
    let id = project["id"].as_str().unwrap().to_string();

    // Ajout dans le projet existant : trois nouveaux objets et trois modèles.
    let mut request = multipart("trois-orca.3mf", archive, &cookie);
    *request.uri_mut() = format!("/api/projects/{id}/import-3mf").parse().unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let added = json_body(response).await;
    let objects = added["objects"].as_array().unwrap();
    assert_eq!(objects.len(), 3);
    assert_eq!(added["models"].as_array().unwrap().len(), 3);
    let distinct: std::collections::HashSet<_> = objects
        .iter()
        .map(|o| o["transform"]["position"].to_string())
        .collect();
    assert_eq!(
        distinct.len(),
        3,
        "positions d'Orca conservées : {objects:?}"
    );
    for o in objects {
        assert!(o["name"].is_string() && o["modelId"].is_string(), "{o}");
    }
    let models = get_json(&app, &cookie, &format!("/api/projects/{id}/models")).await;
    assert_eq!(models.as_array().unwrap().len(), 6);
}

/// F21 : la sauvegarde rend une vignette de la scène pour la bibliothèque.
#[tokio::test]
async fn saving_a_project_renders_its_library_thumbnail() {
    let (_dir, app) = app().await;
    let cookie = register(&app).await;
    let archive = include_bytes!("fixtures/trois-orca.3mf");
    let project = json_body(import_project(&app, &cookie, "trois-orca.3mf", archive).await).await;
    let id = project["id"].as_str().unwrap().to_string();
    for _ in 0..200 {
        let models = get_json(&app, &cookie, &format!("/api/projects/{id}/models")).await;
        if models
            .as_array()
            .unwrap()
            .iter()
            .all(|m| m["has_mesh"] == true)
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let body = serde_json::json!({
        "expected_version": project["version"],
        "scene": project["scene"],
        "active_presets": project["active_presets"],
    });
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/projects/{id}"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &cookie)
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(response.status().is_success(), "{}", response.status());
    let mut png = Vec::new();
    for _ in 0..200 {
        let r = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/projects/{id}/thumbnail"))
                    .header(header::COOKIE, &cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        if r.status() == StatusCode::OK {
            png = to_bytes(r.into_body(), usize::MAX).await.unwrap().to_vec();
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n", "vignette PNG servie");
    let list = get_json(&app, &cookie, "/api/projects").await;
    let entry = list
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == id.as_str())
        .unwrap();
    assert_eq!(entry["has_thumbnail"], true);
}

/// Affectation filament → buse du plateau (H2D, FilamentMapDialog d'Orca) :
/// `filament_map_mode` / `filament_maps` du <plate> relus comme réglages de
/// plateau, donc transmis au tranchage (with_plate_settings) comme le CLI
/// (OrcaSlicer.cpp ~5980). Fixture : scène bambu_h2d, plateau Manual « 1 1 »
/// (référence Orca et contrôle moteur : .work/staging/parity-audit/lot21).
#[tokio::test]
async fn plate_filament_map_is_read_as_a_plate_setting() {
    let archive = include_bytes!("fixtures/threemf/h2d_filament_map_manual.3mf");
    let project = backend::models::threemf::read_project(archive)
        .await
        .unwrap();
    let settings = &project.plates[0].settings;
    assert_eq!(
        settings.get("filament_map_mode").and_then(|v| v.as_str()),
        Some("Manual"),
        "{settings:?}"
    );
    let map = settings.get("filament_map").expect("filament_map relu");
    let text = map
        .as_str()
        .map(str::to_string)
        .unwrap_or_else(|| map.to_string());
    assert!(
        text.contains('1') && !text.contains('2'),
        "carte 1,1 : {text}"
    );
}
