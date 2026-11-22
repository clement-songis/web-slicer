//! Lot 49 (audit 47) : « Disposer » en multi-filament (ARR-a), 3MF projet
//! complet et config du 3MF prioritaire à l'ouverture (RT-a), presets du
//! projet ouvert dans la réponse d'import (PROJ-a).

use std::io::{Cursor, Read};
use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{header, Request, StatusCode};
use axum::Router;
use backend::adapters::files::FileStore;
use backend::adapters::storage::AnyStorage;
use backend::domain::presets::reseed_system_presets;
use backend::domain::{Storage, UserId};
use backend::http::routes::router;
use backend::http::state::AppState;
use serde_json::{json, Value};
use tower::ServiceExt;
use tower_sessions::{MemoryStore, SessionManagerLayer};

async fn app() -> Option<(tempfile::TempDir, Arc<AnyStorage>, Router)> {
    let imported =
        engine::presets::import_profiles(&backend::http::state::default_profiles_dir()).ok()?;
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}", dir.path().join("lot49.db").display());
    let storage = Arc::new(AnyStorage::connect_test(&url).await.unwrap());
    reseed_system_presets(storage.as_ref(), &imported.presets)
        .await
        .unwrap();
    let state = AppState::new(
        Arc::clone(&storage) as Arc<dyn Storage>,
        FileStore::new(dir.path().join("data")),
    );
    let layer = SessionManagerLayer::new(MemoryStore::default()).with_secure(false);
    Some((dir, storage, router(state, layer)))
}

fn worker_available() -> bool {
    let available = backend::engine::worker_binary().exists();
    if !available {
        eprintln!("engine-worker absent — test ignoré");
    }
    available
}

async fn raw_call(
    app: &Router,
    method: &str,
    uri: &str,
    content_type: &str,
    body: Vec<u8>,
    cookie: Option<&str>,
) -> (StatusCode, Option<String>, Vec<u8>) {
    let mut b = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, content_type);
    if let Some(c) = cookie {
        b = b.header(header::COOKIE, c);
    }
    let resp = app
        .clone()
        .oneshot(b.body(Body::from(body)).unwrap())
        .await
        .unwrap();
    let status = resp.status();
    let cookie = resp
        .headers()
        .get(header::SET_COOKIE)
        .map(|v| v.to_str().unwrap().split(';').next().unwrap().to_string());
    let bytes = to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap()
        .to_vec();
    (status, cookie, bytes)
}

async fn call(
    app: &Router,
    method: &str,
    uri: &str,
    body: Value,
    cookie: &str,
) -> (StatusCode, Value) {
    let (s, _, b) = raw_call(
        app,
        method,
        uri,
        "application/json",
        body.to_string().into_bytes(),
        Some(cookie),
    )
    .await;
    (s, serde_json::from_slice(&b).unwrap_or(Value::Null))
}

async fn get_bytes(app: &Router, uri: &str, cookie: &str) -> (StatusCode, Vec<u8>) {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(uri)
                .header(header::COOKIE, cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    (
        status,
        to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap()
            .to_vec(),
    )
}

async fn register(app: &Router, storage: &AnyStorage, email: &str) -> (String, UserId) {
    let (status, cookie, body) = raw_call(
        app,
        "POST",
        "/api/auth/register",
        "application/json",
        json!({ "email": email, "password": "password123" })
            .to_string()
            .into_bytes(),
        None,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::CREATED,
        "{}",
        String::from_utf8_lossy(&body)
    );
    let owner = storage
        .users()
        .find_by_email(email)
        .await
        .unwrap()
        .unwrap()
        .id;
    (cookie.unwrap(), owner)
}

async fn system_id(app: &Router, cookie: &str, kind: &str, name: &str) -> String {
    let (status, list) = call(
        app,
        "GET",
        &format!("/api/presets?kind={kind}"),
        json!({}),
        cookie,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    list.as_array()
        .unwrap()
        .iter()
        .find(|p| p["name"] == name && p["origin"] == "system")
        .unwrap_or_else(|| panic!("{kind} {name} absent"))["id"]
        .as_str()
        .unwrap()
        .to_string()
}

/// Déclare une imprimante possédée (page Imprimantes) : la garde
/// `printer_not_owned` de `resolve_active_config` (tranchage comme
/// arrangement) exige que le modèle du preset imprimante actif soit déclaré
/// par le compte. La création ne teste pas la connexion.
async fn declare_printer(app: &Router, cookie: &str, machine_preset_id: &str, name: &str) {
    let (status, body) = call(
        app,
        "POST",
        "/api/printers",
        json!({
            "name": format!("{name} (lot49)"),
            "moonraker_url": "http://127.0.0.1:7125",
            "machine_preset_id": machine_preset_id,
        }),
        cookie,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
}

fn multipart(filename: &str, bytes: &[u8]) -> (String, Vec<u8>) {
    let boundary = "----webslicer-lot49";
    let mut body = Vec::new();
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(
        format!("Content-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\n")
            .as_bytes(),
    );
    body.extend_from_slice(b"Content-Type: application/octet-stream\r\n\r\n");
    body.extend_from_slice(bytes);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    (format!("multipart/form-data; boundary={boundary}"), body)
}

async fn upload(app: &Router, project: &str, cookie: &str, name: &str, bytes: &[u8]) -> String {
    let (ct, body) = multipart(name, bytes);
    let (status, _, out) = raw_call(
        app,
        "POST",
        &format!("/api/projects/{project}/models"),
        &ct,
        body,
        Some(cookie),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::CREATED,
        "{}",
        String::from_utf8_lossy(&out)
    );
    let id = serde_json::from_slice::<Value>(&out).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();
    wait_mesh(app, &id, cookie).await;
    id
}

async fn wait_mesh(app: &Router, model: &str, cookie: &str) {
    for _ in 0..1200 {
        let (status, _) = get_bytes(app, &format!("/api/models/{model}/mesh"), cookie).await;
        if status != StatusCode::CONFLICT {
            assert_eq!(status, StatusCode::OK, "conversion de {model}");
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    panic!("conversion de {model} non terminée");
}

/// Pavé "size" centré à l'origine (STL binaire).
fn box_stl(size: [f32; 3]) -> Vec<u8> {
    let [x, y, z] = size.map(|s| s / 2.0);
    let v = [
        [-x, -y, -z],
        [x, -y, -z],
        [x, y, -z],
        [-x, y, -z],
        [-x, -y, z],
        [x, -y, z],
        [x, y, z],
        [-x, y, z],
    ];
    let faces: [[usize; 3]; 12] = [
        [0, 2, 1],
        [0, 3, 2],
        [4, 5, 6],
        [4, 6, 7],
        [0, 1, 5],
        [0, 5, 4],
        [1, 2, 6],
        [1, 6, 5],
        [2, 3, 7],
        [2, 7, 6],
        [3, 0, 4],
        [3, 4, 7],
    ];
    let mut out = vec![0u8; 80];
    out.extend_from_slice(&(faces.len() as u32).to_le_bytes());
    for f in faces {
        out.extend_from_slice(&[0u8; 12]);
        for i in f {
            for c in v[i] {
                out.extend_from_slice(&c.to_le_bytes());
            }
        }
        out.extend_from_slice(&[0u8; 2]);
    }
    out
}

async fn new_project(app: &Router, cookie: &str, active: Value) -> String {
    let (status, body) = call(
        app,
        "POST",
        "/api/projects",
        json!({ "name": "lot49", "active_presets": active }),
        cookie,
    )
    .await;
    assert!(status.is_success(), "{status} {body}");
    body["id"].as_str().unwrap().to_string()
}

async fn put_scene(app: &Router, project: &str, cookie: &str, scene: Value, active: Value) {
    let (_, current) = call(
        app,
        "GET",
        &format!("/api/projects/{project}"),
        json!({}),
        cookie,
    )
    .await;
    let (status, body) = call(
        app,
        "PUT",
        &format!("/api/projects/{project}"),
        json!({ "expected_version": current["version"], "scene": scene, "active_presets": active }),
        cookie,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
}

fn two_objects_scene(cube: &str, bar: &str) -> Value {
    json!({
        "schemaVersion": 1,
        "objects": [
            { "id": "a", "modelId": cube, "transform": { "position": [60.0, 60.0, 10.0] } },
            { "id": "b", "modelId": bar, "transform": { "position": [62.0, 62.0, 5.0] } }
        ],
        "plates": { "plates": [{ "id": "p", "objectIds": ["a", "b"] }], "activeId": "p" }
    })
}

async fn arrange(app: &Router, project: &str, cookie: &str, bed: f64) -> (StatusCode, Value) {
    call(
        app,
        "POST",
        &format!("/api/projects/{project}/arrange"),
        json!({
            "bed_width": bed, "bed_depth": bed, "spacing": 0.0, "items": [],
            "bed_shape": [[0, 0], [bed, 0], [bed, bed], [0, bed]], "excluded": [],
            "max_height": 200.0,
            "options": {
                "spacing": 0.0, "enable_rotation": false, "allow_multi_materials_on_same_plate": true,
                "avoid_extrusion_cali_region": false, "align_to_y_axis": false
            },
            "scope": "all", "plate_index": 0
        }),
        cookie,
    )
    .await
}

/// ARR-a : « Disposer » avec N filaments (A1 mini 2, Voron 3, X1C 4, et le
/// témoin A1 mini 1) — 200 et tous les objets placés, au lieu du 500
/// (assertion ConfigOptionVector::apply_override dans le worker).
#[tokio::test]
async fn arrange_succeeds_with_several_filaments() {
    if !worker_available() {
        return;
    }
    let Some((_d, storage, app)) = app().await else {
        eprintln!("profils Orca absents — test ignoré");
        return;
    };
    let (cookie, _) = register(&app, &storage, "arrange49@test.local").await;
    let cases = [
        (
            "Bambu Lab A1 mini 0.4 nozzle",
            "0.20mm Standard @BBL A1M",
            "Bambu PLA Basic @BBL A1M",
            1usize,
            180.0,
        ),
        (
            "Bambu Lab A1 mini 0.4 nozzle",
            "0.20mm Standard @BBL A1M",
            "Bambu PLA Basic @BBL A1M",
            2,
            180.0,
        ),
        (
            "Voron 2.4 350 0.4 nozzle",
            "0.20mm Standard @Voron",
            "Generic PLA @System",
            3,
            350.0,
        ),
        (
            "Bambu Lab X1 Carbon 0.4 nozzle",
            "0.20mm Standard @BBL X1C",
            "Bambu PLA Basic @BBL X1C",
            4,
            256.0,
        ),
    ];
    let mut declared: Vec<&str> = Vec::new();
    for (printer, process, filament, count, bed) in cases {
        let machine = system_id(&app, &cookie, "machine", printer).await;
        // Garde « imprimante possédée », au niveau du modèle : une
        // déclaration couvre toutes les variantes de buse.
        if !declared.contains(&printer) {
            declare_printer(&app, &cookie, &machine, printer).await;
            declared.push(printer);
        }
        let active = json!({
            "printer": machine,
            "process": system_id(&app, &cookie, "process", process).await,
            "filaments": vec![system_id(&app, &cookie, "filament", filament).await; count],
        });
        let project = new_project(&app, &cookie, active.clone()).await;
        let cube = upload(
            &app,
            &project,
            &cookie,
            "cube.stl",
            &box_stl([20.0, 20.0, 20.0]),
        )
        .await;
        let bar = upload(
            &app,
            &project,
            &cookie,
            "bar.stl",
            &box_stl([40.0, 10.0, 10.0]),
        )
        .await;
        put_scene(
            &app,
            &project,
            &cookie,
            two_objects_scene(&cube, &bar),
            active,
        )
        .await;
        let (status, body) = arrange(&app, &project, &cookie, bed).await;
        assert_eq!(status, StatusCode::OK, "{printer} × {count} : {body}");
        assert_eq!(
            body["placements"].as_array().map(Vec::len),
            Some(2),
            "{printer} × {count} : {body}"
        );
    }
}

fn zip_entry(bytes: &[u8], name: &str) -> Option<String> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).ok()?;
    let mut file = archive.by_name(name).ok()?;
    let mut out = String::new();
    file.read_to_string(&mut out).ok()?;
    Some(out)
}

async fn import_3mf(app: &Router, cookie: &str, name: &str, bytes: &[u8]) -> Value {
    let (ct, body) = multipart(name, bytes);
    let (status, _, out) = raw_call(
        app,
        "POST",
        "/api/projects/import?load=project",
        &ct,
        body,
        Some(cookie),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::CREATED,
        "{}",
        String::from_utf8_lossy(&out)
    );
    serde_json::from_slice(&out).unwrap()
}

async fn wait_scene_meshes(app: &Router, cookie: &str, project: &Value) {
    for object in project["scene"]["objects"].as_array().unwrap() {
        if let Some(model) = object["modelId"].as_str() {
            wait_mesh(app, model, cookie).await;
        }
    }
}

/// ARR-a, parcours réel : projet COMBO (Voron, 2 filaments, plateau 2 « par
/// objet ») exporté par le web puis rouvert — « Disposer » répond 200.
#[tokio::test]
async fn arrange_succeeds_in_a_reopened_two_filament_project() {
    if !worker_available() {
        return;
    }
    let Some((_d, storage, app)) = app().await else {
        return;
    };
    let (cookie, _) = register(&app, &storage, "combo49@test.local").await;
    let bytes = std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/lot49/combo_voron_2f.3mf"),
    )
    .unwrap();
    let project = import_3mf(&app, &cookie, "combo_voron_2f.3mf", &bytes).await;
    wait_scene_meshes(&app, &cookie, &project).await;
    let id = project["id"].as_str().unwrap();
    let (status, body) = arrange(&app, id, &cookie, 350.0).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        body["placements"].as_array().map(Vec::len),
        Some(project["scene"]["objects"].as_array().unwrap().len()),
        "{body}"
    );
}

/// Clés à valeur vide d'Orca 2.4.1 (FullPrintConfig::defaults()) que le
/// project_settings.config du web omettait (audit 47 RT-a).
const EMPTY_DEFAULT_KEYS: &[&str] = &[
    "time_lapse_gcode",
    "post_process",
    "printing_by_object_gcode",
    "template_custom_gcode",
    "change_extrusion_role_gcode",
    "process_change_extrusion_role_gcode",
    "printer_notes",
    "notes",
    "wrapping_exclude_area",
    "silent_mode",
    "wall_maximum_deviation",
];

async fn job_settings(storage: &AnyStorage, owner: UserId) -> Value {
    let mut jobs = storage.jobs().list(owner).await.unwrap();
    jobs.sort_by_key(|j| j.created_at);
    jobs.last().unwrap().resolved_settings.clone()
}

async fn slice(app: &Router, project: &str, cookie: &str) {
    let (status, body) = call(
        app,
        "POST",
        &format!("/api/projects/{project}/slice"),
        json!({ "plate_index": 0 }),
        cookie,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
}

/// RT-a : le 3MF exporté porte la config complète (clés vides comprises,
/// comme store_bbs_3mf avec full_fff_config) ; rouvert dans un compte dont
/// l'interface garde une P1S, la config du 3MF fait foi pour toutes les clés
/// (Plater::load_files : défauts + config du 3MF) — aucun G-code de
/// timelapse Bambu ne passe dans le projet Voron, la config tranchée est
/// celle de l'original.
#[tokio::test]
async fn exported_project_is_complete_and_wins_over_the_previous_printer() {
    if !worker_available() {
        return;
    }
    let Some((_d, storage, app)) = app().await else {
        return;
    };
    let (cookie, owner) = register(&app, &storage, "rt49@test.local").await;
    let generic = system_id(&app, &cookie, "filament", "Generic PLA @System").await;
    let voron_machine = system_id(&app, &cookie, "machine", "Voron 2.4 350 0.4 nozzle").await;
    declare_printer(&app, &cookie, &voron_machine, "Voron 2.4 350").await;
    let voron = json!({
        "printer": voron_machine,
        "process": system_id(&app, &cookie, "process", "0.20mm Standard @Voron").await,
        "filaments": [generic.clone(), generic.clone()],
    });
    let project = new_project(&app, &cookie, voron.clone()).await;
    let cube = upload(
        &app,
        &project,
        &cookie,
        "cube.stl",
        &box_stl([20.0, 20.0, 20.0]),
    )
    .await;
    let scene = json!({
        "schemaVersion": 1,
        "objects": [{ "id": "a", "modelId": cube, "transform": { "position": [175.0, 175.0, 10.0] } }],
        "plates": { "plates": [{ "id": "p", "objectIds": ["a"] }], "activeId": "p" }
    });
    put_scene(&app, &project, &cookie, scene, voron).await;
    slice(&app, &project, &cookie).await;
    let original = job_settings(&storage, owner).await;
    // Voron (Klipper) : la chaîne de presets ne porte pas de timelapse — la
    // clé est absente (Null) ou vide, jamais un G-code Bambu.
    assert!(
        original
            .get("time_lapse_gcode")
            .and_then(Value::as_str)
            .unwrap_or("")
            .is_empty(),
        "{}",
        original["time_lapse_gcode"]
    );

    let (status, exported) = get_bytes(
        &app,
        &format!("/api/projects/{project}/export/3mf"),
        &cookie,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let settings: Value =
        serde_json::from_str(&zip_entry(&exported, "Metadata/project_settings.config").unwrap())
            .unwrap();
    let missing: Vec<&&str> = EMPTY_DEFAULT_KEYS
        .iter()
        .filter(|k| settings.get(**k).is_none())
        .collect();
    assert!(missing.is_empty(), "clés absentes du 3MF : {missing:?}");
    assert_eq!(settings["time_lapse_gcode"], "");

    // Chemin A — même compte (possède le Voron) : la réouverture pose les
    // presets du 3MF (PROJ-a ; Orca load_config_file_config) et tranche
    // exactement comme l'original, sans y toucher.
    let reopened = import_3mf(&app, &cookie, "voron.3mf", &exported).await;
    let presets = &reopened["import"]["presets"];
    assert_eq!(
        presets["printer"]["name"], "Voron 2.4 350 0.4 nozzle",
        "{presets}"
    );
    assert_eq!(presets["printer"]["active"], true, "{presets}");
    assert_eq!(
        presets["process"]["name"], "0.20mm Standard @Voron",
        "{presets}"
    );
    assert_eq!(presets["process"]["active"], true, "{presets}");
    assert_eq!(
        presets["filaments"].as_array().map(Vec::len),
        Some(2),
        "{presets}"
    );
    assert_eq!(
        presets["filaments"][0]["name"], "Generic PLA @System",
        "{presets}"
    );
    assert_eq!(presets["filaments"][0]["active"], true, "{presets}");
    let reopened_id = reopened["id"].as_str().unwrap().to_string();
    slice(&app, &reopened_id, &cookie).await;
    assert_resolved_like_original(&job_settings(&storage, owner).await, &original);

    // Chemin B — repro exacte de l'audit RT-a : un compte qui ne possède que
    // la P1S rouvre le 3MF Voron et son interface garde la P1S active (état
    // PROJ-a d'avant le lot). La config complète du 3MF fait foi à la lecture
    // (load_config_file_config) : aucun timelapse Bambu ne passe dans le
    // G-code Voron, la config tranchée reste celle de l'original.
    let (cookie_b, owner_b) = register(&app, &storage, "rt49b@test.local").await;
    let p1s = system_id(&app, &cookie_b, "machine", "Bambu Lab P1S 0.4 nozzle").await;
    declare_printer(&app, &cookie_b, &p1s, "Bambu Lab P1S").await;
    let reopened_b = import_3mf(&app, &cookie_b, "voron.3mf", &exported).await;
    let presets_b = &reopened_b["import"]["presets"];
    assert_eq!(
        presets_b["printer"]["name"], "Voron 2.4 350 0.4 nozzle",
        "{presets_b}"
    );
    // Non possédée → non posée, mais résolue et nommée pour l'interface.
    assert_eq!(presets_b["printer"]["active"], false, "{presets_b}");
    assert!(presets_b["printer"]["id"].is_string(), "{presets_b}");
    let id_b = reopened_b["id"].as_str().unwrap().to_string();
    let mut active_b = reopened_b["active_presets"].clone();
    active_b["printer"] = json!(p1s);
    put_scene(
        &app,
        &id_b,
        &cookie_b,
        reopened_b["scene"].clone(),
        active_b,
    )
    .await;
    slice(&app, &id_b, &cookie_b).await;
    let after_b = job_settings(&storage, owner_b).await;
    assert!(
        after_b
            .get("time_lapse_gcode")
            .and_then(Value::as_str)
            .unwrap_or("")
            .is_empty(),
        "timelapse Bambu repris : {}",
        after_b["time_lapse_gcode"]
    );
    assert_resolved_like_original(&after_b, &original);
}

/// La config tranchée après réouverture est celle de l'original : mêmes
/// valeurs pour toutes les clés (identifiants de presets et marqueurs
/// Orca exclus ; couleurs comparées sans la casse, l'écriture 3MF les
/// met en minuscules).
fn assert_resolved_like_original(after: &Value, original: &Value) {
    let mut differences = Vec::new();
    for (key, value) in original.as_object().unwrap() {
        if matches!(
            key.as_str(),
            "print_settings_id"
                | "printer_settings_id"
                | "filament_settings_id"
                | "inherits_group"
                | "different_settings_to_system"
                | "compatible_printers_condition"
                | "compatible_prints_condition"
                | "compatible_printers"
                | "inherits"
                | "compatible_machine_expression_group"
                | "compatible_process_expression_group"
        ) {
            continue;
        }
        let other = after.get(key).unwrap_or(&Value::Null);
        let same = if key.contains("colour") || key.contains("color") {
            value.to_string().to_lowercase() == other.to_string().to_lowercase()
        } else {
            value == other
        };
        if !same {
            differences.push(format!("{key}: {value} → {other}"));
        }
    }
    assert!(
        differences.is_empty(),
        "config tranchée ≠ original :\n{}",
        differences.join("\n")
    );
}
