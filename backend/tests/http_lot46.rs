//! Lot 46 (audit 44) : lots de préréglages au format d'Orca (PR1-a/b),
//! chemins de fichiers relatifs au répertoire de données (OLD-a).
//!
//! PR1 : presets utilisateur (héritage système, chaîne utilisateur →
//! utilisateur, import « ancien lot » à config complète) exportés en
//! `.orca_printer` par A puis importés par B : mêmes valeurs résolues, un JSON
//! par preset avec ses seules clés (`from: User`, `version`, `inherits`).

use std::io::{Cursor, Read};

use axum::body::{to_bytes, Body};
use axum::http::{header, Request, StatusCode};
use axum::Router;
use backend::adapters::files::FileStore;
use backend::adapters::storage::AnyStorage;
use backend::domain::presets::reseed_system_presets;
use backend::domain::{ModelFormat, NewModel, NewUser, Role, Storage};
use backend::http::routes::router;
use backend::http::state::AppState;
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt;
use tower_sessions::{MemoryStore, SessionManagerLayer};

async fn app() -> Option<(tempfile::TempDir, Router)> {
    let imported =
        engine::presets::import_profiles(&backend::http::state::default_profiles_dir()).ok()?;
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}", dir.path().join("lot46.db").display());
    let storage = AnyStorage::connect_test(&url).await.unwrap();
    reseed_system_presets(&storage, &imported.presets)
        .await
        .unwrap();
    let state = AppState::new(Arc::new(storage), FileStore::new(dir.path().join("data")));
    let layer = SessionManagerLayer::new(MemoryStore::default()).with_secure(false);
    Some((dir, router(state, layer)))
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
    cookie: Option<&str>,
) -> (StatusCode, Option<String>, Value) {
    let (s, c, b) = raw_call(
        app,
        method,
        uri,
        "application/json",
        body.to_string().into_bytes(),
        cookie,
    )
    .await;
    (s, c, serde_json::from_slice(&b).unwrap_or(Value::Null))
}

async fn register(app: &Router, email: &str) -> String {
    let (status, cookie, body) = call(
        app,
        "POST",
        "/api/auth/register",
        json!({ "email": email, "password": "password123" }),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    cookie.unwrap()
}

async fn system_id(app: &Router, cookie: &str, kind: &str, name: &str) -> String {
    let (status, _, list) = call(
        app,
        "GET",
        &format!("/api/presets?kind={kind}"),
        json!({}),
        Some(cookie),
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

async fn create(
    app: &Router,
    cookie: &str,
    kind: &str,
    name: &str,
    inherits: &str,
    values: Value,
) -> String {
    let (status, _, body) = call(
        app,
        "POST",
        "/api/presets",
        json!({ "kind": kind, "name": name, "inherits": inherits, "values": values }),
        Some(cookie),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    body["id"].as_str().unwrap().to_string()
}

async fn resolved_values(app: &Router, cookie: &str, id: &str) -> Value {
    let (status, _, body) = call(
        app,
        "GET",
        &format!("/api/presets/{id}/resolved"),
        json!({}),
        Some(cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let mut values = body["values"].clone();
    for key in [
        "print_settings_id",
        "printer_settings_id",
        "filament_settings_id",
        "inherits",
    ] {
        values.as_object_mut().unwrap().remove(key);
    }
    values
}

async fn user_preset(app: &Router, cookie: &str, kind: &str, name: &str) -> String {
    let (_, _, list) = call(
        app,
        "GET",
        &format!("/api/presets?kind={kind}"),
        json!({}),
        Some(cookie),
    )
    .await;
    list.as_array()
        .unwrap()
        .iter()
        .find(|p| p["name"] == name && p["origin"] == "user")
        .unwrap_or_else(|| panic!("preset utilisateur {kind} {name} absent"))["id"]
        .as_str()
        .unwrap()
        .to_string()
}

fn unzip(bytes: &[u8]) -> Vec<(String, Value)> {
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut out = Vec::new();
    for i in 0..zip.len() {
        let mut f = zip.by_index(i).unwrap();
        let mut text = String::new();
        f.read_to_string(&mut text).unwrap();
        out.push((f.name().to_string(), serde_json::from_str(&text).unwrap()));
    }
    out
}

#[tokio::test]
async fn preset_bundle_round_trips_with_same_resolved_values() {
    let Some((_d, app)) = app().await else {
        return; // profils absents
    };
    let a = register(&app, "lot46-a@test.local").await;
    let machine = system_id(&app, &a, "machine", "Voron 2.4 350 0.4 nozzle").await;
    let process = system_id(&app, &a, "process", "0.20mm Standard @Voron").await;
    let filament = system_id(&app, &a, "filament", "Generic PLA @System").await;

    // Presets de l'éditeur (inherits = id du parent) ; procédé « petit-fils ».
    let m = create(
        &app,
        &a,
        "machine",
        "Voron Audit46",
        &machine,
        json!({ "retraction_length": ["1.2"], "z_hop": ["0.3"] }),
    )
    .await;
    let p = create(&app, &a, "process", "Audit46 Proc 016", &process,
        json!({ "layer_height": "0.16", "sparse_infill_density": "25%", "wall_loops": "4", "top_shell_layers": "6" })).await;
    let p2 = create(
        &app,
        &a,
        "process",
        "Audit46 Proc 016 B",
        &p,
        json!({ "wall_loops": "5", "exclude_object": "1" }),
    )
    .await;
    let f = create(
        &app,
        &a,
        "filament",
        "Audit46 PETG Bleu",
        &filament,
        json!({ "filament_flow_ratio": ["0.95"], "filament_type": ["PETG"] }),
    )
    .await;

    // Export d'un preset seul : format de fichier Orca.
    let (status, _, file) = call(
        &app,
        "GET",
        &format!("/api/presets/{p2}/export"),
        json!({}),
        Some(&a),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(file["from"], "User");
    assert_eq!(file["version"], "2.4.1.0");
    assert_eq!(
        file["inherits"], "0.20mm Standard @Voron",
        "relatif à l'ancêtre système"
    );
    assert_eq!(file["print_settings_id"], "Audit46 Proc 016 B");
    assert_eq!(file["wall_loops"], "5");
    assert_eq!(
        file["layer_height"], "0.16",
        "valeur du parent utilisateur aplatie"
    );
    assert!(file.get("retraction_length").is_none() && file.get("nozzle_diameter").is_none());
    assert!(
        file.get("sparse_infill_pattern").is_none(),
        "valeur égale au parent système omise"
    );

    // Lot .orca_printer.
    let (status, _, bytes) = raw_call(
        &app,
        "POST",
        "/api/presets/export-bundle",
        "application/json",
        json!({ "presets": [{ "id": m }, { "id": f }, { "id": p }, { "id": p2 }, { "id": machine }] })
            .to_string()
            .into_bytes(),
        Some(&a),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "{}",
        String::from_utf8_lossy(&bytes)
    );
    let files = unzip(&bytes);
    let names: Vec<&str> = files.iter().map(|(n, _)| n.as_str()).collect();
    assert!(names.contains(&"printer/Voron Audit46.json"), "{names:?}");
    assert!(
        names.contains(&"filament/Audit46 PETG Bleu.json"),
        "{names:?}"
    );
    assert!(
        names.contains(&"process/Audit46 Proc 016.json"),
        "{names:?}"
    );
    assert!(names.contains(&"bundle_structure.json"), "{names:?}");
    assert_eq!(files.len(), 5, "preset système non exporté");
    let structure = &files
        .iter()
        .find(|(n, _)| n == "bundle_structure.json")
        .unwrap()
        .1;
    assert_eq!(structure["bundle_type"], "printer config bundle");
    assert_eq!(structure["printer_preset_name"], "Voron Audit46");
    let printer = &files
        .iter()
        .find(|(n, _)| n == "printer/Voron Audit46.json")
        .unwrap()
        .1;
    assert_eq!(printer["inherits"], "Voron 2.4 350 0.4 nozzle");
    assert_eq!(printer["retraction_length"], json!(["1.2"]));
    assert!(printer.get("wall_loops").is_none() && printer.get("layer_height").is_none());
    assert_eq!(printer["from"], "User");

    // Import par un autre compte.
    let b = register(&app, "lot46-b@test.local").await;
    let (status, _, body) = raw_call(
        &app,
        "POST",
        "/api/presets/import-bundle?filename=Voron%20Audit46.orca_printer",
        "application/octet-stream",
        bytes.clone(),
        Some(&b),
    )
    .await;
    let body: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["imported"].as_array().unwrap().len(), 4, "{body}");
    assert!(body["skipped"].as_array().unwrap().is_empty(), "{body}");

    for (kind, name, id_a) in [
        ("machine", "Voron Audit46", &m),
        ("process", "Audit46 Proc 016", &p),
        ("process", "Audit46 Proc 016 B", &p2),
        ("filament", "Audit46 PETG Bleu", &f),
    ] {
        let id_b = user_preset(&app, &b, kind, name).await;
        let va = resolved_values(&app, &a, id_a).await;
        let vb = resolved_values(&app, &b, &id_b).await;
        assert_eq!(va, vb, "{name} : valeurs résolues identiques");
    }
    let id_b = user_preset(&app, &b, "process", "Audit46 Proc 016 B").await;
    let vb = resolved_values(&app, &b, &id_b).await;
    assert_eq!(vb["layer_height"], json!(0.16));
    assert_eq!(vb["wall_loops"], json!(5));

    // Réimport (même nom) : remplacé, pas dupliqué.
    let (status, _, _) = raw_call(
        &app,
        "POST",
        "/api/presets/import-bundle",
        "application/octet-stream",
        bytes,
        Some(&b),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (_, _, list) = call(
        &app,
        "GET",
        "/api/presets?kind=process",
        json!({}),
        Some(&b),
    )
    .await;
    let count = list
        .as_array()
        .unwrap()
        .iter()
        .filter(|p| p["name"] == "Audit46 Proc 016 B")
        .count();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn old_full_config_import_keeps_only_own_keys() {
    let Some((_d, app)) = app().await else {
        return;
    };
    let a = register(&app, "lot46-c@test.local").await;
    // JSON d'un ancien lot web-slicer : config complète, valeurs typées.
    let (status, _, created) = call(
        &app,
        "POST",
        "/api/presets/import",
        json!({
            "kind": "machine",
            "name": "Ancien lot machine",
            "inherits": "Voron 2.4 350 0.4 nozzle",
            "values": {
                "retraction_length": [1.2],
                "z_hop": [0.3],
                "wall_loops": 2,
                "layer_height": 0.2,
                "sparse_infill_density": 20,
                "skirt_loops": 1,
                "exclude_object": false,
                "printer_settings_id": "Ancien lot machine"
            }
        }),
        Some(&a),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    let values = created["values"].as_object().unwrap();
    assert_eq!(values.get("retraction_length"), Some(&json!(["1.2"])));
    assert_eq!(values.get("z_hop"), Some(&json!(["0.3"])));
    for foreign in [
        "wall_loops",
        "layer_height",
        "sparse_infill_density",
        "skirt_loops",
        "exclude_object",
    ] {
        assert!(
            !values.contains_key(foreign),
            "{foreign} : clé étrangère au type machine"
        );
    }

    // Fichier Orca sans « type » (Preset::save n'en écrit pas) : type déduit
    // de *_settings_id comme import_json_presets.
    let (status, _, created) = call(
        &app,
        "POST",
        "/api/presets/import",
        json!({
            "kind": "process",
            "name": "Orca sans type",
            "inherits": "0.20mm Standard @Voron",
            "values": { "from": "User", "version": "2.4.1.0", "print_settings_id": "Orca sans type", "wall_loops": "5", "z_hop": ["0.9"] }
        }),
        Some(&a),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    assert_eq!(created["kind"], "process");
    assert_eq!(created["values"], json!({ "wall_loops": "5" }));
}

#[tokio::test]
async fn relocated_data_dir_paths_are_rewritten_relative() {
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}", dir.path().join("lot46c.db").display());
    let old = AnyStorage::connect_test(&url).await.unwrap();
    let user = old
        .users()
        .create(NewUser {
            email: "old@test.local".into(),
            password_hash: "x".into(),
            role: Role::User,
        })
        .await
        .unwrap();
    let uid = user.id.to_string();
    let rel = format!("users/{uid}/models/m.stl");
    let model = old
        .models()
        .create(
            user.id,
            NewModel {
                project_id: None,
                filename: "m.stl".into(),
                format: ModelFormat::Stl,
                file_path: format!("/var/lib/ancienne-racine/data/{rel}"),
                mesh_path: Some(format!(
                    "/var/lib/ancienne-racine/data/users/{uid}/meshes/absent.wsm"
                )),
                size_bytes: 1,
                triangle_count: 0,
                repair_report: None,
            },
        )
        .await
        .unwrap();
    // Répertoire de données déplacé : le fichier existe sous la nouvelle racine.
    let root = dir.path().join("data");
    std::fs::create_dir_all(root.join(format!("users/{uid}/models"))).unwrap();
    std::fs::write(root.join(&rel), b"solid").unwrap();

    let moved = AnyStorage::connect_test(&url).await.unwrap();
    assert_eq!(
        moved.set_data_root(&root).await.unwrap(),
        1,
        "un chemin rattrapé"
    );
    let read = moved.models().get(user.id, model.id).await.unwrap();
    assert_eq!(read.file_path, root.join(&rel).to_string_lossy());
    // Maillage absent sous la nouvelle racine : valeur d'origine conservée.
    assert!(read
        .mesh_path
        .unwrap()
        .starts_with("/var/lib/ancienne-racine/"));
    // En base (codec neutre) : chemin relatif.
    let raw = old.models().get(user.id, model.id).await.unwrap();
    assert_eq!(raw.file_path, rel);
    // Idempotent ; un nouvel enregistrement est stocké relatif.
    assert_eq!(moved.set_data_root(&root).await.unwrap(), 0);
}

/// DEV1-a : G-code d'une autre imprimante (printer_model différent) refusé
/// en 409 lisible avant toute connexion à l'hôte.
#[tokio::test]
async fn upload_refuses_gcode_of_another_printer_model() {
    let imported =
        match engine::presets::import_profiles(&backend::http::state::default_profiles_dir()) {
            Ok(i) => i,
            Err(_) => return,
        };
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}", dir.path().join("lot46d.db").display());
    let storage = Arc::new(AnyStorage::connect_test(&url).await.unwrap());
    reseed_system_presets(storage.as_ref(), &imported.presets)
        .await
        .unwrap();
    let state = AppState::new(storage.clone(), FileStore::new(dir.path().join("data")));
    let layer = SessionManagerLayer::new(MemoryStore::default()).with_secure(false);
    let app = router(state, layer);
    let cookie = register(&app, "lot46-e@test.local").await;
    let (_, _, me) = call(&app, "GET", "/api/auth/me", json!({}), Some(&cookie)).await;
    let user: backend::domain::UserId = uuid::Uuid::parse_str(me["id"].as_str().unwrap())
        .unwrap()
        .into();
    let voron = system_id(&app, &cookie, "machine", "Voron 2.4 350 0.4 nozzle").await;
    let (_, _, resolved) = call(
        &app,
        "GET",
        &format!("/api/presets/{voron}/resolved"),
        json!({}),
        Some(&cookie),
    )
    .await;
    let voron_model = resolved["values"]["printer_model"]
        .as_str()
        .unwrap()
        .to_string();
    let printer = storage
        .printers()
        .create(
            user,
            backend::domain::NewPrinter {
                name: "Voron".into(),
                moonraker_url: Some("http://127.0.0.1:9".into()),
                api_key: None,
                machine_preset_id: uuid::Uuid::parse_str(&voron).unwrap().into(),
            },
        )
        .await
        .unwrap();
    let project = storage
        .projects()
        .create(
            user,
            backend::domain::NewProject {
                name: "p".into(),
                scene: json!({}),
                active_presets: json!({}),
                thumbnail_path: None,
            },
        )
        .await
        .unwrap();
    let gcode = |model: &str, id: &str| {
        format!(
            "G1 X0\n; CONFIG_BLOCK_START\n; printer_model = {model}\n; printer_settings_id = {id}\n; CONFIG_BLOCK_END\n"
        )
    };
    let mut ids = Vec::new();
    for (i, text) in [
        gcode("Bambu Lab X1 Carbon", "Bambu Lab X1 Carbon 0.4 nozzle"),
        gcode(&voron_model, "Voron 2.4 350 0.4 nozzle"),
    ]
    .iter()
    .enumerate()
    {
        let job = storage
            .jobs()
            .enqueue(
                user,
                backend::domain::NewJob {
                    project_id: project.id,
                    plate_index: 0,
                    resolved_settings: json!({}),
                },
            )
            .await
            .unwrap();
        let path = dir.path().join(format!("g{i}.gcode"));
        std::fs::write(&path, text).unwrap();
        let g = storage
            .gcodes()
            .create(
                user,
                backend::domain::NewGcode {
                    job_id: job.id,
                    file_path: path.to_string_lossy().into_owned(),
                    preview_path: String::new(),
                    stats: json!({}),
                    thumbnails: json!([]),
                },
            )
            .await
            .unwrap();
        ids.push(g.id.to_string());
    }
    let uri = format!("/api/printers/{}/upload", printer.id);
    let (status, _, body) = call(
        &app,
        "POST",
        &uri,
        json!({ "gcode_id": ids[0], "start_now": false }),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(body["details"]["reason"], "printer_incompatible", "{body}");
    assert_eq!(
        body["details"]["gcode_printer_model"],
        "Bambu Lab X1 Carbon"
    );
    assert_eq!(body["details"]["printer_model"], voron_model.as_str());
    // Même modèle : le contrôle passe (l'hôte injoignable échoue ensuite).
    let (_, _, body) = call(
        &app,
        "POST",
        &uri,
        json!({ "gcode_id": ids[1], "start_now": false }),
        Some(&cookie),
    )
    .await;
    assert_ne!(body["details"]["reason"], "printer_incompatible", "{body}");
}

/// Lot 46c : presets système actifs (cas courant de « Fichier › Exporter ›
/// Lot de préréglages ») exportés en presets autonomes (`from: User`,
/// `inherits: ""`, config complète du type) dans un `Presets.zip` plat ;
/// réimport : ignorés tant que le preset système homonyme existe (Orca), et
/// importés sous un autre nom (installation sans ce preset) avec les mêmes
/// valeurs résolues que le preset système.
#[tokio::test]
async fn system_presets_export_as_standalone_and_round_trip() {
    let Some((_d, app)) = app().await else {
        return; // profils absents
    };
    let a = register(&app, "lot46c-a@test.local").await;
    let machine = system_id(&app, &a, "machine", "Voron 2.4 350 0.4 nozzle").await;
    let process = system_id(&app, &a, "process", "0.20mm Standard @Voron").await;
    let filament = system_id(&app, &a, "filament", "Generic PLA @System").await;
    let (status, _, bytes) = raw_call(
        &app,
        "POST",
        "/api/presets/export-bundle",
        "application/json",
        json!({ "presets": [{ "id": machine }, { "id": filament }, { "id": process }] })
            .to_string()
            .into_bytes(),
        Some(&a),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "{}",
        String::from_utf8_lossy(&bytes)
    );
    if let Ok(dir) = std::env::var("LOT46C_DUMP") {
        std::fs::write(format!("{dir}/system.zip"), &bytes).unwrap();
    }
    assert!(
        bytes.windows(12).any(|w| w == b"layer_height"),
        "entrées non compressées (JSON lisible dans l'archive)"
    );
    let files = unzip(&bytes);
    let mut names: Vec<&str> = files.iter().map(|(n, _)| n.as_str()).collect();
    names.sort();
    assert_eq!(
        names,
        [
            "0.20mm Standard @Voron.json",
            "Generic PLA @System.json",
            "Voron 2.4 350 0.4 nozzle.json"
        ],
        "ZIP plat, sans bundle_structure.json"
    );
    let doc = |name: &str| &files.iter().find(|(n, _)| n == name).unwrap().1;
    let m = doc("Voron 2.4 350 0.4 nozzle.json");
    let p = doc("0.20mm Standard @Voron.json");
    let f = doc("Generic PLA @System.json");
    for (d, ty, name) in [
        (m, "machine", "Voron 2.4 350 0.4 nozzle"),
        (p, "process", "0.20mm Standard @Voron"),
        (f, "filament", "Generic PLA @System"),
    ] {
        assert_eq!(d["type"], ty);
        assert_eq!(d["name"], name);
        assert_eq!(d["from"], "User");
        assert_eq!(d["inherits"], "");
        assert_eq!(d["version"], "2.4.1.0");
        assert!(d.as_object().unwrap().len() > 40, "{ty} : config complète");
    }
    assert_eq!(m["printer_settings_id"], "Voron 2.4 350 0.4 nozzle");
    assert!(m.get("nozzle_diameter").is_some() && m.get("layer_height").is_none());
    assert!(m.get("filament_type").is_none());
    assert!(p.get("layer_height").is_some() && p.get("wall_loops").is_some());
    assert!(p.get("nozzle_diameter").is_none() && p.get("retraction_length").is_none());
    assert_eq!(f["filament_settings_id"], json!(["Generic PLA @System"]));
    // Vecteur au défaut Orca [""] jamais écrit vide (CLI : set_at sur vecteur vide).
    assert_eq!(f["filament_notes"], json!([""]));
    assert!(f.get("filament_type").is_some() && f.get("layer_height").is_none());
    assert!(f.get("printer_settings_id").is_none() && f.get("print_settings_id").is_none());

    // Réimport tel quel : le preset système homonyme bloque (Orca).
    let b = register(&app, "lot46c-b@test.local").await;
    let (status, _, body) = raw_call(
        &app,
        "POST",
        "/api/presets/import-bundle?filename=Presets.zip",
        "application/octet-stream",
        bytes.clone(),
        Some(&b),
    )
    .await;
    let body: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(body["imported"].as_array().unwrap().is_empty(), "{body}");
    assert_eq!(body["skipped"].as_array().unwrap().len(), 3, "{body}");

    // Installation sans ces presets système : mêmes fichiers sous un autre nom.
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (file, d) in &files {
        let mut d = d.clone();
        let name = format!("{} (export)", d["name"].as_str().unwrap());
        d["name"] = json!(name);
        for key in ["printer_settings_id", "print_settings_id"] {
            if d.get(key).is_some() {
                d[key] = json!(name);
            }
        }
        if d.get("filament_settings_id").is_some() {
            d["filament_settings_id"] = json!([name]);
        }
        zip.start_file(file.as_str(), zip::write::SimpleFileOptions::default())
            .unwrap();
        std::io::Write::write_all(&mut zip, d.to_string().as_bytes()).unwrap();
    }
    let renamed = zip.finish().unwrap().into_inner();
    let (status, _, body) = raw_call(
        &app,
        "POST",
        "/api/presets/import-bundle?filename=Presets.zip",
        "application/octet-stream",
        renamed,
        Some(&b),
    )
    .await;
    let body: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["imported"].as_array().unwrap().len(), 3, "{body}");
    for (kind, name, system) in [
        ("machine", "Voron 2.4 350 0.4 nozzle", &machine),
        ("process", "0.20mm Standard @Voron", &process),
        ("filament", "Generic PLA @System", &filament),
    ] {
        let id_b = user_preset(&app, &b, kind, &format!("{name} (export)")).await;
        let va = resolved_values(&app, &a, system).await;
        let vb = resolved_values(&app, &b, &id_b).await;
        let (va, vb) = (va.as_object().unwrap(), vb.as_object().unwrap());
        let diff: Vec<String> = va
            .keys()
            .chain(vb.keys())
            .filter(|k| va.get(*k) != vb.get(*k))
            .map(|k| format!("{k}: {:?} != {:?}", va.get(k), vb.get(k)))
            .collect();
        assert!(
            diff.is_empty(),
            "{name} : valeurs résolues différentes {diff:#?}"
        );
    }
}
