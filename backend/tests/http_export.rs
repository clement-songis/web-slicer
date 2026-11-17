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
    let model_xml = all_model_xml(&mut zip);
    // Le triangle du STL est présent (3 sommets, 1 triangle).
    assert_eq!(model_xml.matches("<vertex ").count(), 3);
    assert_eq!(model_xml.matches("<triangle ").count(), 1);
}

/// Contenu de toutes les parties `.model` : le projet est écrit par libslic3r
/// comme « Save project » d'Orca (SplitModel : maillages dans 3D/Objects/).
fn all_model_xml(zip: &mut zip::ZipArchive<std::io::Cursor<&[u8]>>) -> String {
    let mut out = String::new();
    for i in 0..zip.len() {
        let mut file = zip.by_index(i).unwrap();
        if file.name().ends_with(".model") {
            file.read_to_string(&mut out).unwrap();
        }
    }
    out
}

#[tokio::test]
async fn export_preserves_the_printable_flag_and_reimports_it() {
    let h = harness().await;
    let session = register(&h.app, "printable@test.local").await;
    let owner = user_id(&h.storage, "printable@test.local").await;
    let project = seed_project(&h, owner).await;
    let models = h.storage.models().list(owner, Some(project)).await.unwrap();
    let scene = json!({
        "schemaVersion": 1,
        "objects": [{
            "id": "o1", "modelId": models[0].id.to_string(), "printable": false
        }],
        "plates": { "plates": [{ "id": "p1", "objectIds": ["o1"] }] }
    });
    h.storage
        .projects()
        .update(owner, project, 1, scene, json!({}), None)
        .await
        .unwrap();
    let resp = get(
        &h.app,
        &format!("/api/projects/{project}/export/3mf"),
        &session,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes.as_ref())).unwrap();
    let mut xml = String::new();
    zip.by_name("3D/3dmodel.model")
        .unwrap()
        .read_to_string(&mut xml)
        .unwrap();
    assert!(xml.contains("printable=\"0\""), "{xml}");
    drop(zip);
    let imported = backend::models::threemf::read_project(&bytes)
        .await
        .unwrap();
    assert_eq!(imported.objects.len(), 1);
    assert!(!imported.objects[0].printable);
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

#[tokio::test]
async fn exports_a_converted_non_stl_model() {
    let h = harness().await;
    let session = register(&h.app, "converted@test.local").await;
    let owner = user_id(&h.storage, "converted@test.local").await;
    let project = h
        .storage
        .projects()
        .create(
            owner,
            NewProject {
                name: "Converted".into(),
                scene: json!({}),
                active_presets: json!({}),
                thumbnail_path: None,
            },
        )
        .await
        .unwrap();
    let mesh = engine::api::TriangleMesh {
        vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        indices: vec![[0, 1, 2]],
    };
    let mesh_path = h
        .files
        .write_mesh(
            owner,
            backend::domain::ModelId::new(),
            &mesh.encode_display(),
        )
        .await
        .unwrap();
    h.storage
        .models()
        .create(
            owner,
            NewModel {
                project_id: Some(project.id),
                filename: "part.obj".into(),
                format: ModelFormat::Obj,
                // Le chemin source ne doit plus être nécessaire une fois le WSMh écrit.
                file_path: "source-indisponible.obj".into(),
                mesh_path: Some(mesh_path.to_string_lossy().into_owned()),
                size_bytes: 0,
                triangle_count: 1,
                repair_report: None,
            },
        )
        .await
        .unwrap();

    let resp = get(
        &h.app,
        &format!("/api/projects/{}/export/3mf", project.id),
        &session,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes.as_ref())).unwrap();
    let model_xml = all_model_xml(&mut zip);
    assert_eq!(model_xml.matches("<vertex ").count(), 3);
    assert_eq!(model_xml.matches("<triangle ").count(), 1);
}

/// Requête multipart d'import de projet 3MF (`POST /api/projects/import`).
fn import_request(bytes: &[u8], session: &str) -> Request<Body> {
    let boundary = "----webslicer-export-roundtrip";
    let mut body = Vec::new();
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(
        b"Content-Disposition: form-data; name=\"file\"; filename=\"roundtrip.3mf\"\r\n",
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
        .header(header::COOKIE, session)
        .body(Body::from(body))
        .unwrap()
}

#[tokio::test]
async fn object_and_plate_settings_survive_export_then_import() {
    let h = harness().await;
    let session = register(&h.app, "roundtrip@test.local").await;
    let owner = user_id(&h.storage, "roundtrip@test.local").await;
    let project = seed_project(&h, owner).await;
    let model_id = h.storage.models().list(owner, Some(project)).await.unwrap()[0]
        .id
        .to_string();
    let current = h.storage.projects().get(owner, project).await.unwrap();
    let scene = json!({
        "objects": [{
            "id": "o1",
            "modelId": model_id,
            "transform": {"position": [20.0, 30.0, 0.0], "rotation": [0.0, 0.0, 0.0],
                          "scale": [1.0, 1.0, 1.0]},
            "extruder": 2,
            "settings": {"wall_loops": 5},
            "layerProfile": [0.0, 0.2, 0.5, 0.12, 1.0, 0.28]
        }],
        "plates": {"activeId": "p1", "plates": [{
            "id": "p1", "name": "Plaque A", "plateType": "Engineering Plate",
            "objectIds": ["o1"], "settings": {"print_sequence": "by object"}
        }]}
    });
    h.storage
        .projects()
        .update(owner, project, current.version, scene, json!({}), None)
        .await
        .unwrap();

    let resp = get(
        &h.app,
        &format!("/api/projects/{project}/export/3mf"),
        &session,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();

    // L'archive porte les réglages au format Orca.
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes.as_ref())).unwrap();
    let mut settings_xml = String::new();
    zip.by_name("Metadata/model_settings.config")
        .unwrap()
        .read_to_string(&mut settings_xml)
        .unwrap();
    for expected in [
        r#"<metadata key="wall_loops" value="5"/>"#,
        r#"<metadata key="extruder" value="2"/>"#,
        r#"<metadata key="plater_name" value="Plaque A"/>"#,
        r#"<metadata key="bed_type" value="Engineering Plate"/>"#,
        r#"<metadata key="print_sequence" value="by object"/>"#,
    ] {
        assert!(
            settings_xml.contains(expected),
            "manque {expected} : {settings_xml}"
        );
    }
    assert!(zip.by_name("Metadata/layer_heights_profile.txt").is_ok());
    drop(zip);

    // Réimport : surcharge d'objet, profil et plateau retrouvés dans la scène.
    let resp = h
        .app
        .clone()
        .oneshot(import_request(&bytes, &session))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let imported: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let objects = imported["scene"]["objects"].as_array().unwrap();
    assert_eq!(objects.len(), 1);
    assert_eq!(objects[0]["settings"]["wall_loops"], 5);
    // Le fichier porte extruder = 2 (vérifié plus haut) ; à la lecture, Orca
    // (load_bbs_3mf, bbs_3mf.cpp ~2228) ramène à 1 tout extrudeur au-delà du
    // nombre de filaments du projet (ici un seul) : même résultat qu'Orca.
    assert_eq!(objects[0]["extruder"], 1);
    assert_eq!(
        objects[0]["layerProfile"],
        json!([0.0, 0.2, 0.5, 0.12, 1.0, 0.28])
    );
    let plates = imported["scene"]["plates"]["plates"].as_array().unwrap();
    assert_eq!(plates.len(), 1);
    assert_eq!(plates[0]["name"], "Plaque A");
    assert_eq!(plates[0]["plateType"], "Engineering Plate");
    assert_eq!(plates[0]["objectIds"][0], objects[0]["id"]);
}

/// Plages de hauteur (`layerRanges` → Orca `layer_config_ranges`) : écrites dans
/// `Metadata/layer_config_ranges.xml` à l'export, relues à l'import.
#[tokio::test]
async fn layer_config_ranges_survive_export_then_import() {
    let h = harness().await;
    let session = register(&h.app, "ranges-roundtrip@test.local").await;
    let owner = user_id(&h.storage, "ranges-roundtrip@test.local").await;
    let project = seed_project(&h, owner).await;
    let model_id = h.storage.models().list(owner, Some(project)).await.unwrap()[0]
        .id
        .to_string();
    let current = h.storage.projects().get(owner, project).await.unwrap();
    let scene = json!({
        "objects": [{
            "id": "o1",
            "modelId": model_id,
            "transform": {"position": [20.0, 30.0, 0.0], "rotation": [0.0, 0.0, 0.0],
                          "scale": [1.0, 1.0, 1.0]},
            "layerRanges": [
                {"min": 0.0, "max": 2.0, "settings": {"layer_height": 0.12, "extruder": 0}},
                {"min": 2.0, "max": 4.5, "settings": {"wall_loops": 4}}
            ]
        }],
        "plates": {"activeId": "p1", "plates": [{"id": "p1", "name": "A", "objectIds": ["o1"]}]}
    });
    h.storage
        .projects()
        .update(owner, project, current.version, scene, json!({}), None)
        .await
        .unwrap();

    let resp = get(
        &h.app,
        &format!("/api/projects/{project}/export/3mf"),
        &session,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes.as_ref())).unwrap();
    let mut xml = String::new();
    zip.by_name("Metadata/layer_config_ranges.xml")
        .unwrap()
        .read_to_string(&mut xml)
        .unwrap();
    assert!(xml.contains(r#"<range min_z="0" max_z="2">"#), "{xml}");
    assert!(
        xml.contains(r#"<option opt_key="layer_height">0.12</option>"#),
        "{xml}"
    );
    drop(zip);

    let resp = h
        .app
        .clone()
        .oneshot(import_request(&bytes, &session))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let imported: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let ranges = imported["scene"]["objects"][0]["layerRanges"]
        .as_array()
        .unwrap();
    assert_eq!(ranges.len(), 2, "{imported}");
    assert_eq!(ranges[0]["min"], 0.0);
    assert_eq!(ranges[0]["max"], 2.0);
    assert_eq!(ranges[0]["settings"]["layer_height"], 0.12);
    assert_eq!(ranges[1]["min"], 2.0);
    assert_eq!(ranges[1]["max"], 4.5);
    assert_eq!(ranges[1]["settings"]["wall_loops"], 4);
}

/// `meshName` (volume du maillage propre, Orca `<nom>_1` après un split) :
/// écrit comme nom du volume pièce dans `Metadata/model_settings.config`, puis
/// relu à l'import ; absent quand il égale le nom de l'objet.
#[tokio::test]
async fn mesh_name_survives_export_then_import() {
    let h = harness().await;
    let session = register(&h.app, "mesh-name@test.local").await;
    let owner = user_id(&h.storage, "mesh-name@test.local").await;
    let project = seed_project(&h, owner).await;
    let model_id = h.storage.models().list(owner, Some(project)).await.unwrap()[0]
        .id
        .to_string();
    let current = h.storage.projects().get(owner, project).await.unwrap();
    let scene = json!({
        "objects": [{
            "id": "o1",
            "modelId": model_id,
            "meshName": "pair.stl_1",
            "transform": {"position": [20.0, 30.0, 0.0], "rotation": [0.0, 0.0, 0.0],
                          "scale": [1.0, 1.0, 1.0]}
        }],
        "plates": {"activeId": "p1", "plates": [{"id": "p1", "name": "A", "objectIds": ["o1"]}]}
    });
    h.storage
        .projects()
        .update(owner, project, current.version, scene, json!({}), None)
        .await
        .unwrap();

    let resp = get(
        &h.app,
        &format!("/api/projects/{project}/export/3mf"),
        &session,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes.as_ref())).unwrap();
    let mut xml = String::new();
    zip.by_name("Metadata/model_settings.config")
        .unwrap()
        .read_to_string(&mut xml)
        .unwrap();
    let part = &xml[xml.find("<part ").expect("<part> écrit")..];
    let part = &part[..part.find("</part>").unwrap()];
    assert!(part.contains(r#"subtype="normal_part""#), "{part}");
    assert!(
        part.contains(r#"<metadata key="name" value="pair.stl_1"/>"#),
        "{part}"
    );
    drop(zip);

    let resp = h
        .app
        .clone()
        .oneshot(import_request(&bytes, &session))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let imported: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        imported["scene"]["objects"][0]["meshName"], "pair.stl_1",
        "{imported}"
    );
}
