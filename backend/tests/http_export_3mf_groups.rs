//! Export/import 3MF des clés « par preset » regroupées par Orca
//! (`PresetBundle::full_fff_config` → `Metadata/project_settings.config`,
//! `bbs_3mf.cpp` `_add_project_config_file_to_archive` / `load_config_file_config`) :
//! `compatible_machine_expression_group`, `compatible_process_expression_group`,
//! `inherits_group` (à la place de `inherits` et `compatible_*`, effacés par
//! `full_fff_config`) et `upward_compatible_machine` (clé imprimante, telle quelle).

use std::io::{Read, Write};
use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{header, Request, StatusCode};
use axum::response::Response;
use axum::Router;
use backend::adapters::files::FileStore;
use backend::adapters::storage::sqlite::SqliteStorage;
use backend::domain::repo::{NewModel, NewProject};
use backend::domain::{
    ModelFormat, Preset, PresetId, PresetKind, PresetOrigin, ProjectId, Storage, UserId,
};
use serde_json::{json, Value};
use tower::ServiceExt;
use tower_sessions::{MemoryStore, SessionManagerLayer};

const STL: &str = "solid t
facet normal 0 0 1
 outer loop
  vertex 0 0 0
  vertex 10 0 0
  vertex 0 10 0
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
    let url = format!("sqlite://{}", dir.path().join("groups.db").display());
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

async fn register(h: &Harness, email: &str) -> (String, UserId) {
    let resp = h
        .app
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
    let session = cookie(&resp);
    let id = h
        .storage
        .users()
        .find_by_email(email)
        .await
        .unwrap()
        .unwrap()
        .id;
    (session, id)
}

async fn seed_project(h: &Harness, owner: UserId, active_presets: Value) -> ProjectId {
    let project = h
        .storage
        .projects()
        .create(
            owner,
            NewProject {
                name: format!("Groups {}", uuid::Uuid::new_v4()),
                scene: json!({}),
                active_presets,
                thumbnail_path: None,
            },
        )
        .await
        .unwrap();
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

/// Export 3MF ; un projet importé répond 409 tant que ses modèles sont en conversion.
async fn export_3mf(h: &Harness, project: &str, session: &str) -> Vec<u8> {
    for _ in 0..100 {
        let resp = h
            .app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/projects/{project}/export/3mf"))
                    .header(header::COOKIE, session)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        if resp.status() == StatusCode::CONFLICT {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            continue;
        }
        assert_eq!(resp.status(), StatusCode::OK);
        return to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap()
            .to_vec();
    }
    panic!("export 3MF toujours en conflit (conversion)");
}

fn project_settings(bytes: &[u8]) -> serde_json::Map<String, Value> {
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let mut text = String::new();
    zip.by_name("Metadata/project_settings.config")
        .unwrap()
        .read_to_string(&mut text)
        .unwrap();
    serde_json::from_str::<Value>(&text)
        .unwrap()
        .as_object()
        .unwrap()
        .clone()
}

/// Réécrit `project_settings.config` d'une archive (les autres entrées intactes).
fn with_project_settings(bytes: &[u8], settings: &serde_json::Map<String, Value>) -> Vec<u8> {
    let mut src = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let mut out = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let opts = zip::write::SimpleFileOptions::default();
    for i in 0..src.len() {
        let mut file = src.by_index(i).unwrap();
        let name = file.name().to_string();
        let mut content = Vec::new();
        file.read_to_end(&mut content).unwrap();
        if name == "Metadata/project_settings.config" {
            content = serde_json::to_vec_pretty(settings).unwrap();
        }
        out.start_file(name, opts).unwrap();
        out.write_all(&content).unwrap();
    }
    out.finish().unwrap().into_inner()
}

async fn import(h: &Harness, bytes: &[u8], session: &str) -> Value {
    let boundary = "----webslicer-groups";
    let mut body = Vec::new();
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(
        b"Content-Disposition: form-data; name=\"file\"; filename=\"groups.3mf\"\r\n",
    );
    body.extend_from_slice(b"Content-Type: model/3mf\r\n\r\n");
    body.extend_from_slice(bytes);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    let resp = h
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/projects/import")
                .header(
                    header::CONTENT_TYPE,
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .header(header::COOKIE, session)
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    serde_json::from_slice(&to_bytes(resp.into_body(), usize::MAX).await.unwrap()).unwrap()
}

fn preset(
    kind: PresetKind,
    name: &str,
    origin: PresetOrigin,
    owner: Option<UserId>,
    inherits: Option<&str>,
    values: Value,
) -> Preset {
    Preset {
        id: PresetId::new(),
        kind,
        name: name.into(),
        origin,
        user_id: owner,
        vendor: None,
        inherits: inherits.map(Into::into),
        instantiation: true,
        setting_id: None,
        filament_id: None,
        compatible_printers: None,
        values,
    }
}

const PRESET_KEYS: [&str; 5] = [
    "inherits",
    "compatible_printers",
    "compatible_printers_condition",
    "compatible_prints",
    "compatible_prints_condition",
];

/// Orca relit les groupes à l'import (`load_config_file_config` : entrée i →
/// preset i) et les réécrit à l'identique à l'export (`full_fff_config`) ;
/// `inherits` et `compatible_*` bruts du fichier sont écrasés/effacés.
#[tokio::test]
async fn preset_groups_survive_import_then_export_like_orca() {
    let h = harness().await;
    let (session, owner) = register(&h, "groups-rt@test.local").await;
    let project = seed_project(&h, owner, json!({})).await;
    let first = export_3mf(&h, &project.to_string(), &session).await;

    let mut settings = project_settings(&first);
    let groups = json!({
        "compatible_machine_expression_group": ["printer_notes=~/.*PM2.*/", "nozzle_diameter[0]==0.4"],
        "compatible_process_expression_group": ["layer_height <= 0.3"],
        "inherits_group": ["0.20mm Standard @Voron", "Generic PLA @System", "Voron 2.4 350 0.4 nozzle"],
        "upward_compatible_machine": ["Voron 2.4 300 0.4 nozzle", "Voron 2.4 250 0.4 nozzle"],
    });
    for (k, v) in groups.as_object().unwrap() {
        settings.insert(k.clone(), v.clone());
    }
    settings.insert("inherits".into(), json!("bogus parent"));
    settings.insert("compatible_printers".into(), json!(["Some printer"]));
    settings.insert(
        "compatible_printers_condition".into(),
        json!("raw_condition"),
    );
    let crafted = with_project_settings(&first, &settings);

    let imported = import(&h, &crafted, &session).await;
    // Preset d'impression importé : condition = entrée 0 du groupe (Orca).
    let process_id = imported["active_presets"]["process"].as_str().unwrap();
    let process = h
        .storage
        .presets()
        .get(PresetId(uuid::Uuid::parse_str(process_id).unwrap()))
        .await
        .unwrap();
    assert_eq!(
        process.values["compatible_printers_condition"],
        json!("printer_notes=~/.*PM2.*/")
    );
    assert!(process.values.get("inherits").is_none());

    let second = export_3mf(&h, imported["id"].as_str().unwrap(), &session).await;
    let again = project_settings(&second);
    for (k, v) in groups.as_object().unwrap() {
        assert_eq!(again.get(k), Some(v), "{k} conservé");
    }
    for k in PRESET_KEYS {
        assert!(
            again.get(k).is_none(),
            "{k} effacé par full_fff_config : {:?}",
            again.get(k)
        );
    }
}

/// Export d'un projet sur presets : groupes dérivés des presets actifs comme
/// `full_fff_config` (preset système : `inherits` vide ; preset utilisateur :
/// parent système), écrits seulement si une entrée est non vide.
#[tokio::test]
async fn preset_groups_are_derived_from_active_presets() {
    let h = harness().await;
    let (session, owner) = register(&h, "groups-derived@test.local").await;
    let process_sys = preset(
        PresetKind::Process,
        "Proc S",
        PresetOrigin::System,
        None,
        None,
        json!({ "layer_height": "0.2", "compatible_printers_condition": "" }),
    );
    let filament_sys = preset(
        PresetKind::Filament,
        "Fil S",
        PresetOrigin::System,
        None,
        None,
        json!({ "compatible_prints_condition": "layer_height <= 0.3", "compatible_printers_condition": "" }),
    );
    h.storage
        .presets()
        .reseed_system(vec![process_sys.clone(), filament_sys.clone()])
        .await
        .unwrap();
    let process_user = h
        .storage
        .presets()
        .create_user_preset(
            owner,
            preset(
                PresetKind::Process,
                "Proc U",
                PresetOrigin::User,
                Some(owner),
                Some("Proc S"),
                json!({ "compatible_printers_condition": "printer_notes=~/.*PM2.*/" }),
            ),
        )
        .await
        .unwrap();

    // Presets système seuls : aucun inherits, seule la condition process du filament.
    let system_only = seed_project(
        &h,
        owner,
        json!({
            "process": process_sys.id.to_string(), "filaments": [filament_sys.id.to_string()]
        }),
    )
    .await;
    let s = project_settings(&export_3mf(&h, &system_only.to_string(), &session).await);
    assert!(
        s.get("inherits_group").is_none(),
        "{:?}",
        s.get("inherits_group")
    );
    assert!(s.get("compatible_machine_expression_group").is_none());
    assert_eq!(
        s["compatible_process_expression_group"],
        json!(["layer_height <= 0.3"])
    );

    let derived = seed_project(
        &h,
        owner,
        json!({
            "process": process_user.id.to_string(), "filaments": [filament_sys.id.to_string()]
        }),
    )
    .await;
    let s = project_settings(&export_3mf(&h, &derived.to_string(), &session).await);
    assert_eq!(s["inherits_group"], json!(["Proc S", "", ""]));
    assert_eq!(
        s["compatible_machine_expression_group"],
        json!(["printer_notes=~/.*PM2.*/", ""])
    );
    assert_eq!(
        s["compatible_process_expression_group"],
        json!(["layer_height <= 0.3"])
    );
    for k in PRESET_KEYS {
        assert!(s.get(k).is_none(), "{k} effacé par full_fff_config");
    }
}
