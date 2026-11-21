//! Lot 40 (audit D1) : réglages modifiés non enregistrés en preset, portés par
//! le projet (`active_presets.overrides`) comme le preset « (modified) »
//! d'Orca : enregistrement / relecture, tranchage sans brouillon, brouillon
//! qui les remplace, export 3MF (`project_settings.config`) et ouverture d'un
//! 3MF Orca (valeurs différentes des presets nommés → surcharges).

use std::io::{Cursor, Read, Write};
use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{header, Request, StatusCode};
use axum::response::Response;
use axum::Router;
use backend::adapters::files::FileStore;
use backend::adapters::storage::AnyStorage;
use backend::domain::repo::NewPrinter;
use backend::domain::{PresetKind, PresetOrigin, Storage, UserId};
use backend::http::routes::router;
use backend::http::state::AppState;
use serde_json::{json, Value};
use tower::ServiceExt;
use tower_sessions::{MemoryStore, SessionManagerLayer};

async fn app() -> (tempfile::TempDir, Arc<AnyStorage>, Router) {
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}", dir.path().join("overrides.db").display());
    let storage = Arc::new(AnyStorage::connect_test(&url).await.unwrap());
    let files = FileStore::new(dir.path().join("data"));
    let state = AppState::new(Arc::clone(&storage) as Arc<dyn Storage>, files);
    let sessions = SessionManagerLayer::new(MemoryStore::default()).with_secure(false);
    (dir, storage, router(state, sessions))
}

async fn body_bytes(resp: Response) -> Vec<u8> {
    to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap()
        .to_vec()
}

async fn body_json(resp: Response) -> Value {
    serde_json::from_slice(&body_bytes(resp).await).unwrap()
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
    resp.headers()[header::SET_COOKIE]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_string()
}

fn request(method: &str, uri: &str, session: &str, body: Option<Value>) -> Request<Body> {
    let builder = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::COOKIE, session);
    match body {
        Some(body) => builder
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
        None => builder.body(Body::empty()).unwrap(),
    }
}

async fn user_preset(
    storage: &AnyStorage,
    owner: UserId,
    kind: PresetKind,
    name: &str,
    values: Value,
) -> String {
    storage
        .presets()
        .create_user_preset(
            owner,
            backend::domain::Preset {
                id: backend::domain::PresetId::new(),
                kind,
                name: name.into(),
                origin: PresetOrigin::User,
                user_id: Some(owner),
                vendor: None,
                inherits: None,
                instantiation: true,
                setting_id: None,
                filament_id: None,
                compatible_printers: None,
                renamed_from: Vec::new(),
                values,
            },
        )
        .await
        .unwrap()
        .id
        .to_string()
}

/// Compte + imprimante possédée « Bench 0.4 nozzle », process « Mon process »
/// (3 parois) et filament « Mon PLA » ; renvoie (session, owner, presets).
async fn account(storage: &AnyStorage, app: &Router, email: &str) -> (String, UserId, Value) {
    let session = register(app, email).await;
    let owner = storage
        .users()
        .find_by_email(email)
        .await
        .unwrap()
        .unwrap()
        .id;
    let machine = user_preset(
        storage,
        owner,
        PresetKind::Machine,
        "Bench 0.4 nozzle",
        json!({}),
    )
    .await;
    storage
        .printers()
        .create(
            owner,
            NewPrinter {
                name: "Bench".into(),
                moonraker_url: None,
                api_key: None,
                machine_preset_id: backend::domain::PresetId(
                    uuid::Uuid::parse_str(&machine).unwrap(),
                ),
            },
        )
        .await
        .unwrap();
    let process = user_preset(
        storage,
        owner,
        PresetKind::Process,
        "Mon process",
        json!({ "layer_height": "0.2", "wall_loops": "3" }),
    )
    .await;
    let filament = user_preset(
        storage,
        owner,
        PresetKind::Filament,
        "Mon PLA",
        json!({ "filament_type": ["PLA"], "nozzle_temperature": ["210"] }),
    )
    .await;
    (
        session,
        owner,
        json!({ "printer": machine, "process": process, "filaments": [filament] }),
    )
}

fn scene_one_object() -> Value {
    json!({
        "schema_version": 1,
        "objects": [{ "id": "o1", "modelId": "m1" }],
        "plates": [{ "id": "p1", "objectIds": ["o1"] }]
    })
}

async fn new_project(storage: &AnyStorage, owner: UserId, scene: Value, active: Value) -> String {
    storage
        .projects()
        .create(
            owner,
            backend::domain::NewProject {
                name: "P".into(),
                scene,
                active_presets: active,
                thumbnail_path: None,
            },
        )
        .await
        .unwrap()
        .id
        .to_string()
}

#[tokio::test]
async fn saved_overrides_round_trip_and_are_sliced_unless_a_draft_replaces_them() {
    let (_d, storage, app) = app().await;
    let (session, owner, active) = account(&storage, &app, "ov@test.local").await;
    let project = new_project(&storage, owner, scene_one_object(), active.clone()).await;

    // PUT : surcharges vérifiées puis compactées (objet vide retiré).
    let mut with_overrides = active.clone();
    with_overrides["overrides"] = json!({
        "process": { "wall_loops": "5" },
        "printer": {},
        "filaments": [null]
    });
    let resp = app
        .clone()
        .oneshot(request(
            "PUT",
            &format!("/api/projects/{project}"),
            &session,
            Some(json!({
                "expected_version": 1,
                "scene": scene_one_object(),
                "active_presets": with_overrides
            })),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let saved = body_json(resp).await;
    assert_eq!(
        saved["active_presets"]["overrides"],
        json!({ "process": { "wall_loops": "5" } })
    );
    let reopened = body_json(
        app.clone()
            .oneshot(request(
                "GET",
                &format!("/api/projects/{project}"),
                &session,
                None,
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(reopened["active_presets"], saved["active_presets"]);

    // Valeur illisible : 422, rien d'enregistré.
    let mut bad = active.clone();
    bad["overrides"] = json!({ "process": { "wall_loops": "beaucoup" } });
    let resp = app
        .clone()
        .oneshot(request(
            "PATCH",
            &format!("/api/projects/{project}"),
            &session,
            Some(json!({ "expected_version": 2, "active_presets": bad })),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);

    // Tranchage sans brouillon : config effective = presets + surcharges.
    let resp = app
        .clone()
        .oneshot(request(
            "POST",
            &format!("/api/projects/{project}/slice"),
            &session,
            Some(json!({ "plate_index": 0 })),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    // Brouillon de l'éditeur : il remplace les surcharges (état édité complet).
    let resp = app
        .clone()
        .oneshot(request(
            "POST",
            &format!("/api/projects/{project}/slice"),
            &session,
            Some(json!({ "plate_index": 0, "draft": { "process": { "layer_height": "0.16" } } })),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let mut jobs = storage.jobs().list(owner).await.unwrap();
    jobs.sort_by_key(|j| j.created_at);
    assert_eq!(jobs.len(), 2);
    assert_eq!(jobs[0].resolved_settings["wall_loops"], 5);
    assert_eq!(jobs[1].resolved_settings["wall_loops"], 3);
    assert_eq!(jobs[1].resolved_settings["layer_height"], 0.16);
}

fn read_zip_entry(bytes: &[u8], name: &str) -> String {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut file = archive.by_name(name).unwrap();
    let mut out = String::new();
    file.read_to_string(&mut out).unwrap();
    out
}

#[tokio::test]
async fn exported_project_settings_carry_the_overrides() {
    let (_d, storage, app) = app().await;
    let (session, owner, mut active) = account(&storage, &app, "export@test.local").await;
    active["overrides"] = json!({
        "process": { "wall_loops": "5" },
        "printer": { "z_offset": "0.1" }
    });
    let project = new_project(
        &storage,
        owner,
        json!({ "schema_version": 1, "objects": [], "plates": [] }),
        active,
    )
    .await;
    let resp = app
        .clone()
        .oneshot(request(
            "GET",
            &format!("/api/projects/{project}/export/3mf"),
            &session,
            None,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = body_bytes(resp).await;
    let config: Value =
        serde_json::from_str(&read_zip_entry(&bytes, "Metadata/project_settings.config")).unwrap();
    assert_eq!(config["wall_loops"], "5");
    assert_eq!(config["z_offset"], "0.1");
    assert_eq!(config["print_settings_id"], "Mon process");
    // [process, filament 1, imprimante] : presets « modifiés » comme Orca.
    assert_eq!(
        config["different_settings_to_system"],
        json!(["wall_loops", "", "z_offset"])
    );
}

/// 3MF Orca minimal : un triangle, presets nommés et valeurs du projet.
fn orca_project(config: &Value) -> Vec<u8> {
    let model = r#"<?xml version="1.0"?><model unit="millimeter"><resources>
      <object id="1" type="model"><mesh><vertices>
      <vertex x="0" y="0" z="0"/><vertex x="1" y="1" z="0"/><vertex x="0" y="0" z="1"/>
      </vertices><triangles><triangle v1="0" v2="1" v3="2"/></triangles></mesh></object>
      </resources><build><item objectid="1" transform="1 0 0 0 1 0 0 0 1 10 20 0"/></build></model>"#;
    let content_types = r#"<?xml version="1.0" encoding="UTF-8"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="model" ContentType="application/vnd.ms-package.3dmanufacturing-3dmodel+xml"/></Types>"#;
    let rels = r#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Target="/3D/3dmodel.model" Id="rel-1" Type="http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel"/></Relationships>"#;
    let config = config.to_string();
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default();
    for (name, content) in [
        ("[Content_Types].xml", content_types),
        ("_rels/.rels", rels),
        ("3D/3dmodel.model", model),
        ("Metadata/project_settings.config", config.as_str()),
    ] {
        zip.start_file(name, options).unwrap();
        zip.write_all(content.as_bytes()).unwrap();
    }
    zip.finish().unwrap().into_inner()
}

fn import_request(bytes: &[u8], session: &str) -> Request<Body> {
    let boundary = "----lot40-overrides";
    let mut body = Vec::new();
    body.extend_from_slice(
        format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"modifie.3mf\"\r\nContent-Type: model/3mf\r\n\r\n"
        )
        .as_bytes(),
    );
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

/// Orca (load_external_preset, LoadAndSelect::Always) : le process et
/// l'imprimante nommés par le projet sont sélectionnés et les valeurs qui en
/// diffèrent deviennent des modifications — ici des surcharges, sans preset
/// « (configuration 3MF) » créé.
#[tokio::test]
async fn opening_an_orca_project_turns_differences_into_overrides() {
    let (_d, storage, app) = app().await;
    let (session, owner, active) = account(&storage, &app, "import@test.local").await;
    let before = storage
        .presets()
        .list_by_kind(PresetKind::Process, owner)
        .await
        .unwrap()
        .len();
    let archive = orca_project(&json!({
        "printer_settings_id": "Bench 0.4 nozzle",
        "print_settings_id": "Mon process",
        "filament_settings_id": ["Mon PLA"],
        "filament_colour": ["#FF0000"],
        "filament_type": ["PLA"],
        "layer_height": "0.2",
        "wall_loops": "5",
        "z_offset": "0.1",
        "different_settings_to_system": ["wall_loops", "", "z_offset"]
    }));
    let resp = app
        .clone()
        .oneshot(import_request(&archive, &session))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let project = body_json(resp).await;
    let imported = &project["active_presets"];
    assert_eq!(imported["process"], active["process"]);
    assert_eq!(imported["printer"], active["printer"]);
    assert_eq!(imported["filaments"], active["filaments"]);
    assert_eq!(imported["overrides"]["process"], json!({ "wall_loops": 5 }));
    assert_eq!(imported["overrides"]["printer"], json!({ "z_offset": 0.1 }));
    let after = storage
        .presets()
        .list_by_kind(PresetKind::Process, owner)
        .await
        .unwrap()
        .len();
    assert_eq!(after, before, "aucun preset process importé créé");
}
