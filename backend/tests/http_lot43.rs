//! Lot 43 (audit 42) : objets hors du volume d'impression calculés par le
//! serveur (I1, BuildVolume / ModelInstance::calc_print_volume_state d'Orca),
//! repère des plateaux d'un 3MF à 2 plateaux, valeurs du projet conservées à
//! l'ouverture d'un 3MF (I3, load_external_preset), hauteur de couche
//! variable « Adaptatif » / « Lisser » (S1) et modèles orphelins après un
//! import interrompu (C1).

use std::io::{Cursor, Read};
use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{header, Request, StatusCode};
use axum::response::Response;
use axum::Router;
use backend::adapters::files::FileStore;
use backend::adapters::storage::AnyStorage;
use backend::domain::repo::NewPrinter;
use backend::domain::{PresetKind, ProjectId, Storage, UserId};
use backend::http::routes::router;
use backend::http::state::AppState;
use serde_json::{json, Value};
use tower::ServiceExt;
use tower_sessions::{MemoryStore, SessionManagerLayer};

async fn app() -> (tempfile::TempDir, Arc<AnyStorage>, Router) {
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}", dir.path().join("lot43.db").display());
    let storage = Arc::new(AnyStorage::connect_test(&url).await.unwrap());
    let files = FileStore::new(dir.path().join("data"));
    let state = AppState::new(Arc::clone(&storage) as Arc<dyn Storage>, files);
    let sessions = SessionManagerLayer::new(MemoryStore::default()).with_secure(false);
    (dir, storage, router(state, sessions))
}

fn worker_available() -> bool {
    let available = backend::engine::worker_binary().exists();
    if !available {
        eprintln!("engine-worker absent — test ignoré");
    }
    available
}

async fn body_json(resp: Response) -> Value {
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    }
}

async fn register(app: &Router, storage: &AnyStorage, email: &str) -> (String, UserId) {
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
    let session = resp.headers()[header::SET_COOKIE]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_string();
    let id = storage
        .users()
        .find_by_email(email)
        .await
        .unwrap()
        .unwrap()
        .id;
    (session, id)
}

async fn call(
    app: &Router,
    method: &str,
    uri: &str,
    session: &str,
    body: Option<Value>,
) -> Response {
    let builder = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::COOKIE, session);
    let req = match body {
        Some(body) => builder
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
        None => builder.body(Body::empty()).unwrap(),
    };
    app.clone().oneshot(req).await.unwrap()
}

fn multipart(uri: &str, filename: &str, bytes: &[u8], session: &str) -> Request<Body> {
    let boundary = "----lot43";
    let mut body = Vec::new();
    body.extend_from_slice(
        format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\nContent-Type: application/octet-stream\r\n\r\n"
        )
        .as_bytes(),
    );
    body.extend_from_slice(bytes);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    Request::builder()
        .method("POST")
        .uri(uri)
        .header(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={boundary}"),
        )
        .header(header::COOKIE, session)
        .body(Body::from(body))
        .unwrap()
}

fn repo_file(path: &str) -> Vec<u8> {
    std::fs::read(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(path)).unwrap()
}

async fn new_project(storage: &AnyStorage, owner: UserId, scene: Value) -> String {
    storage
        .projects()
        .create(
            owner,
            backend::domain::NewProject {
                name: "Lot 43".into(),
                scene,
                active_presets: json!({}),
                thumbnail_path: None,
            },
        )
        .await
        .unwrap()
        .id
        .to_string()
}

/// Cube de 20 mm (STL du dépôt) importé dans le projet ; renvoie son id.
async fn upload_cube(app: &Router, project: &str, session: &str) -> String {
    let resp = app
        .clone()
        .oneshot(multipart(
            &format!("/api/projects/{project}/models"),
            "cube20.stl",
            &repo_file("../engine/tests/fixtures/cube20.stl"),
            session,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    body_json(resp).await["id"].as_str().unwrap().to_string()
}

fn cube(id: &str, model: &str, position: [f64; 3]) -> Value {
    json!({
        "id": id,
        "modelId": model,
        "name": format!("Cube {id}"),
        "transform": { "position": position, "rotation": [0, 0, 0], "scale": [1, 1, 1] }
    })
}

/// Scène à deux plateaux (plateau par défaut 200 × 200 × 100 du registre) :
/// dedans, à cheval sur le bord X, trop haut, et hors du plateau 2.
fn placement_scene(model: &str) -> Value {
    json!({
        "schema_version": 1,
        "objects": [
            cube("in", model, [100.0, 100.0, 10.0]),
            cube("edge", model, [195.0, 100.0, 10.0]),
            cube("tall", model, [50.0, 50.0, 95.0]),
            cube("out", model, [500.0, 100.0, 10.0]),
            cube("ok2", model, [60.0, 60.0, 10.0])
        ],
        "plates": [
            { "id": "p1", "objectIds": ["in", "edge", "tall"] },
            { "id": "p2", "objectIds": ["out", "ok2"] }
        ]
    })
}

fn state_of(objects: &Value, id: &str) -> Value {
    objects
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["object_id"] == id)
        .map(|o| o["state"].clone())
        .unwrap_or(Value::Null)
}

/// I1 : le serveur classe chaque objet à partir de la scène et du plateau,
/// sans drapeau off_bed du client (jamais posé par l'éditeur, audit 42).
#[tokio::test]
async fn object_states_are_computed_from_the_scene_and_the_bed() {
    let (_d, storage, app) = app().await;
    let (session, owner) = register(&app, &storage, "states@test.local").await;
    let project = new_project(
        &storage,
        owner,
        json!({ "schema_version": 1, "objects": [] }),
    )
    .await;
    let model = upload_cube(&app, &project, &session).await;
    let scene = placement_scene(&model);

    let resp = call(
        &app,
        "POST",
        &format!("/api/projects/{project}/object-states"),
        &session,
        Some(json!({ "scene": scene })),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    let objects = &body["objects"];
    assert_eq!(state_of(objects, "in"), "inside", "{body}");
    assert_eq!(state_of(objects, "edge"), "partly_outside", "{body}");
    assert_eq!(state_of(objects, "tall"), "partly_outside", "{body}");
    assert_eq!(state_of(objects, "out"), "outside", "{body}");
    assert_eq!(state_of(objects, "ok2"), "inside", "{body}");
    assert_eq!(body["bed"], json!([200.0, 200.0, 100.0]));
    // Plateau ciblé : ses objets seulement.
    let resp = call(
        &app,
        "POST",
        &format!("/api/projects/{project}/object-states"),
        &session,
        Some(json!({ "scene": placement_scene(&model), "plate_index": 1 })),
    )
    .await;
    let body = body_json(resp).await;
    assert_eq!(body["objects"].as_array().unwrap().len(), 2, "{body}");
}

/// I1 : POST …/slice refuse (422 lisible : ids, noms, états) un plateau dont
/// un objet est à cheval, trop haut ou dehors ; …/validate rapporte une
/// erreur par objet (message d'Orca) et l'état de chaque objet.
#[tokio::test]
async fn slice_and_validate_refuse_objects_outside_the_build_volume() {
    let (_d, storage, app) = app().await;
    let (session, owner) = register(&app, &storage, "outside@test.local").await;
    let project = new_project(
        &storage,
        owner,
        json!({ "schema_version": 1, "objects": [] }),
    )
    .await;
    let model = upload_cube(&app, &project, &session).await;
    let scene = placement_scene(&model);

    let resp = call(
        &app,
        "POST",
        &format!("/api/projects/{project}/slice"),
        &session,
        Some(json!({ "plate_index": 0, "scene": scene })),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body = body_json(resp).await;
    assert_eq!(body["details"]["reason"], "object_outside_plate", "{body}");
    assert_eq!(body["details"]["plate_index"], 0);
    let refused: Vec<(String, String)> = body["details"]["objects"]
        .as_array()
        .unwrap()
        .iter()
        .map(|o| {
            (
                o["id"].as_str().unwrap().to_string(),
                o["state"].as_str().unwrap().to_string(),
            )
        })
        .collect();
    assert_eq!(
        refused,
        vec![
            ("edge".to_string(), "partly_outside".to_string()),
            ("tall".to_string(), "partly_outside".to_string())
        ]
    );
    assert!(
        body["message"].as_str().unwrap().contains("Cube edge"),
        "{body}"
    );

    // Plateau 2 : objet entièrement dehors.
    let resp = call(
        &app,
        "POST",
        &format!("/api/projects/{project}/slice"),
        &session,
        Some(json!({ "plate_index": 1, "scene": scene })),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body = body_json(resp).await;
    assert_eq!(body["details"]["objects"][0]["id"], "out");
    assert_eq!(body["details"]["objects"][0]["state"], "outside");

    // « Tout trancher » : refusé aussi (premier plateau en faute).
    let resp = call(
        &app,
        "POST",
        &format!("/api/projects/{project}/slice"),
        &session,
        Some(json!({ "all": true, "scene": scene })),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);

    // Validation : erreurs par objet, message d'Orca, sans lancer le moteur.
    let resp = call(
        &app,
        "POST",
        &format!("/api/projects/{project}/validate"),
        &session,
        Some(json!({ "plate_index": 0, "scene": scene })),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["valid"], false, "{body}");
    let errors = body["plates"][0]["errors"].as_array().unwrap();
    assert_eq!(errors.len(), 2, "{body}");
    assert_eq!(errors[0]["kind"], "object_outside_plate");
    assert_eq!(errors[0]["object_id"], "edge");
    assert_eq!(errors[0]["placement"], "partly_outside");
    assert!(errors[0]["message"]
        .as_str()
        .unwrap()
        .starts_with("Following objects are laid over the boundary of plate"));
    assert_eq!(state_of(&body["objects"], "in"), "inside", "{body}");

    // Objets dans le volume (scène réduite) : tranchage accepté.
    let inside = json!({
        "schema_version": 1,
        "objects": [cube("in", &model, [100.0, 100.0, 10.0])],
        "plates": [{ "id": "p1", "objectIds": ["in"] }]
    });
    let resp = call(
        &app,
        "POST",
        &format!("/api/projects/{project}/slice"),
        &session,
        Some(json!({ "plate_index": 0, "scene": inside })),
    )
    .await;
    let status = resp.status();
    assert_eq!(status, StatusCode::CREATED, "{}", body_json(resp).await);

    // Objet non imprimable hors plateau : ignoré comme Orca (jamais tranché).
    let unprintable = json!({
        "schema_version": 1,
        "objects": [
            cube("in", &model, [100.0, 100.0, 10.0]),
            { "id": "off", "modelId": model, "printable": false,
              "transform": { "position": [500.0, 100.0, 10.0] } }
        ],
        "plates": [{ "id": "p1", "objectIds": ["in", "off"] }]
    });
    let resp = call(
        &app,
        "POST",
        &format!("/api/projects/{project}/slice"),
        &session,
        Some(json!({ "plate_index": 0, "scene": unprintable })),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::CREATED);
}

fn zip_entry(bytes: &[u8], name: &str) -> String {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut file = archive.by_name(name).unwrap();
    let mut out = String::new();
    file.read_to_string(&mut out).unwrap();
    out
}

/// 3MF à 2 plateaux (riche.3mf de l'audit, Voron 350) : Orca écrit le
/// plateau 2 décalé de 350 × 1,2 = 420 mm (X = 595) ; à l'ouverture l'objet
/// est ramené dans le repère de son plateau (X = 175), tous les objets sont
/// dans le volume, et l'export rétablit les positions monde d'Orca.
#[tokio::test]
async fn two_plate_3mf_is_opened_in_plate_coordinates_and_exported_back() {
    if !worker_available() {
        return;
    }
    let (_d, storage, app) = app().await;
    let (session, _owner) = register(&app, &storage, "plates@test.local").await;
    let resp = app
        .clone()
        .oneshot(multipart(
            "/api/projects/import",
            "riche.3mf",
            &repo_file("tests/fixtures/lot43/riche.3mf"),
            &session,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let project = body_json(resp).await;
    let id = project["id"].as_str().unwrap().to_string();
    let scene = &project["scene"];
    let plates = scene["plates"]["plates"].as_array().unwrap();
    assert_eq!(plates.len(), 2);
    let second = plates[1]["objectIds"][0].as_str().unwrap();
    let object = scene["objects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["id"] == second)
        .unwrap();
    let x = object["transform"]["position"][0].as_f64().unwrap();
    assert!((x - 175.0).abs() < 1e-3, "plateau 2 dans son repère : {x}");

    let resp = call(
        &app,
        "POST",
        &format!("/api/projects/{id}/object-states"),
        &session,
        Some(json!({})),
    )
    .await;
    let body = body_json(resp).await;
    let states: Vec<&Value> = body["objects"]
        .as_array()
        .unwrap()
        .iter()
        .map(|o| &o["state"])
        .collect();
    assert_eq!(states.len(), 2, "{body}");
    assert!(states.iter().all(|s| *s == "inside"), "{body}");
    assert_eq!(body["bed"][0], 350.0, "plateau de la config du projet");

    // Export : plateau 2 à nouveau à X = 595 (repère monde d'Orca).
    let resp = call(
        &app,
        "GET",
        &format!("/api/projects/{id}/export/3mf"),
        &session,
        None,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let model = zip_entry(&bytes, "3D/3dmodel.model");
    let translations: Vec<f64> = model
        .match_indices("transform=\"")
        .filter_map(|(i, _)| {
            let rest = &model[i + 11..];
            let end = rest.find('"')?;
            let parts: Vec<f64> = rest[..end]
                .split_whitespace()
                .filter_map(|v| v.parse().ok())
                .collect();
            (parts.len() == 12).then(|| parts[9])
        })
        .collect();
    assert!(
        translations.iter().any(|x| (x - 595.0).abs() < 0.01),
        "plateau 2 exporté en X ≈ 595 : {translations:?}"
    );
}

/// Profils système réels d'Orca limités aux vendeurs utiles (Voron,
/// bibliothèque de filaments) : presets « Voron 2.4 300 0.4 nozzle »,
/// « 0.20mm Standard @Voron », « Generic PLA @System ».
async fn seed_voron(storage: &AnyStorage) {
    let profiles = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../vendor/OrcaSlicer/resources/profiles");
    let outcome = engine::presets::import_profiles(&profiles).unwrap();
    let wanted: Vec<_> = outcome
        .presets
        .into_iter()
        .filter(|p| p.vendor == "Voron" || p.vendor == "OrcaFilamentLibrary")
        .collect();
    backend::domain::presets::reseed_system_presets(storage, &wanted)
        .await
        .unwrap();
}

async fn own_printer(storage: &AnyStorage, owner: UserId, name: &str) {
    let machine = storage
        .presets()
        .list_by_kind(PresetKind::Machine, owner)
        .await
        .unwrap()
        .into_iter()
        .find(|p| p.name == name)
        .expect("preset machine");
    storage
        .printers()
        .create(
            owner,
            NewPrinter {
                name: "Voron".into(),
                moonraker_url: None,
                api_key: None,
                machine_preset_id: machine.id,
            },
        )
        .await
        .unwrap();
}

/// I3 (audit 42, arbre.3mf) : « Generic PLA @System » vaut 0,98 / 10 mais le
/// projet porte filament_flow_ratio = 1 et slow_down_min_speed = 20 avec un
/// different_settings_to_system vide. Orca (load_external_preset) garde les
/// valeurs du projet (preset « modifié ») : surcharges du filament 1, et la
/// config exportée / tranchée les porte.
#[tokio::test]
async fn opening_the_audit_3mf_keeps_the_project_filament_values() {
    if !worker_available() {
        return;
    }
    let (_d, storage, app) = app().await;
    seed_voron(&storage).await;
    let (session, owner) = register(&app, &storage, "arbre@test.local").await;
    own_printer(&storage, owner, "Voron 2.4 300 0.4 nozzle").await;
    let resp = app
        .clone()
        .oneshot(multipart(
            "/api/projects/import",
            "arbre.3mf",
            &repo_file("tests/fixtures/lot43/arbre.3mf"),
            &session,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let project = body_json(resp).await;
    let active = &project["active_presets"];
    // Type de plaque du projet (option projet d'Orca) repris par le plateau.
    assert_eq!(
        project["scene"]["plates"]["plates"][0]["settings"]["curr_bed_type"],
        "Textured PEI Plate"
    );
    let filament = storage
        .presets()
        .get(backend::domain::PresetId(
            uuid::Uuid::parse_str(active["filaments"][0].as_str().unwrap()).unwrap(),
        ))
        .await
        .unwrap();
    assert_eq!(
        filament.name, "Generic PLA @System",
        "preset système retenu"
    );
    let overrides = &active["overrides"]["filaments"][0];
    assert_eq!(overrides["filament_flow_ratio"], json!([1.0]), "{active}");
    assert_eq!(overrides["slow_down_min_speed"], json!([20.0]), "{active}");
    // Différence réelle de G-code de fin (espace et ligne vide) reprise aussi.
    assert_eq!(
        overrides["filament_end_gcode"],
        json!(["; filament end gcode 

"]),
        "{overrides}"
    );
    // Valeurs identiques à la config effective : pas de surcharge.
    for same in ["filament_type", "filament_diameter", "filament_density"] {
        assert!(overrides.get(same).is_none(), "{same} : {overrides}");
    }

    // Config effective (export : project_settings.config = config éditée).
    let id = project["id"].as_str().unwrap();
    let resp = call(
        &app,
        "GET",
        &format!("/api/projects/{id}/export/3mf"),
        &session,
        None,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let settings: Value =
        serde_json::from_str(&zip_entry(&bytes, "Metadata/project_settings.config")).unwrap();
    assert_eq!(settings["filament_flow_ratio"], json!(["1"]));
    assert_eq!(settings["slow_down_min_speed"], json!(["20"]));
}

/// S1 : « Adaptatif » / « Lisser » avec les réglages résolus d'une Voron
/// (extruder_type = « Direct Drive » faisait planter le worker → 500) ; le
/// profil couvre toute la hauteur de l'objet posé (0 → 20 mm).
#[tokio::test]
async fn variable_layer_height_adaptive_and_smooth_cover_the_whole_object() {
    if !worker_available() {
        return;
    }
    let (_d, storage, app) = app().await;
    let (session, owner) = register(&app, &storage, "vlh@test.local").await;
    let project = new_project(
        &storage,
        owner,
        json!({ "schema_version": 1, "objects": [] }),
    )
    .await;
    let model = upload_cube(&app, &project, &session).await;
    // Maillage converti par le worker (sinon 409 « conversion en cours »).
    let resp = call(
        &app,
        "GET",
        &format!("/api/models/{model}/mesh?wait=60"),
        &session,
        None,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let values = json!({
        "extruder_type": ["Direct Drive"],
        "nozzle_diameter": ["0.4"],
        "layer_height": "0.2",
        "initial_layer_print_height": "0.2",
        "min_layer_height": ["0.07"],
        "max_layer_height": ["0.32"]
    });
    // Matrice de l'éditeur : cube recentré posé sur le plateau (Z = 10).
    let matrix = [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 175, 175, 10, 1];
    let resp = call(
        &app,
        "POST",
        &format!("/api/models/{model}/layer-profile"),
        &session,
        Some(json!({ "op": { "kind": "adaptive", "quality": 0.5 }, "values": values, "matrix": matrix })),
    )
    .await;
    let status = resp.status();
    let body = body_json(resp).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let adaptive: Vec<f64> = serde_json::from_value(body["profile"].clone()).unwrap();
    assert!(adaptive.len() >= 4 && adaptive.len().is_multiple_of(2));
    let top = adaptive[adaptive.len() - 2];
    assert!((top - 20.0).abs() < 1e-3, "profil jusqu'au sommet : {top}");
    assert!(adaptive
        .chunks_exact(2)
        .all(|p| p[1] > 0.0 && p[1] <= 0.32 + 1e-6));

    let resp = call(
        &app,
        "POST",
        &format!("/api/models/{model}/layer-profile"),
        &session,
        Some(json!({
            "op": { "kind": "smooth", "profile": adaptive, "radius": 5, "keep_min": false },
            "values": values,
            "matrix": matrix
        })),
    )
    .await;
    let status = resp.status();
    let body = body_json(resp).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let smooth: Vec<f64> = serde_json::from_value(body["profile"].clone()).unwrap();
    assert!(!smooth.is_empty() && smooth.len().is_multiple_of(2));
}

/// C1 : à l'ouverture, un modèle connu de la scène enregistrée
/// (knownModels) sans y être référencé est un orphelin définitif (objet
/// retiré puis projet enregistré) : supprimé avec ses fichiers. Un import
/// non enregistré (inconnu de knownModels, ex. connexion perdue après le
/// 201) est conservé et signalé referenced = false pour être repris.
#[tokio::test]
async fn opening_a_project_drops_removed_models_and_flags_interrupted_imports() {
    let (dir, storage, app) = app().await;
    let (session, owner) = register(&app, &storage, "orphans@test.local").await;
    let project = new_project(
        &storage,
        owner,
        json!({ "schema_version": 1, "objects": [] }),
    )
    .await;
    let kept = upload_cube(&app, &project, &session).await;
    let removed = upload_cube(&app, &project, &session).await;
    let interrupted = upload_cube(&app, &project, &session).await;
    let pid = ProjectId(uuid::Uuid::parse_str(&project).unwrap());
    let removed_file = storage
        .models()
        .list(owner, Some(pid))
        .await
        .unwrap()
        .into_iter()
        .find(|m| m.id.to_string() == removed)
        .unwrap()
        .file_path;
    assert!(std::path::Path::new(&removed_file).exists());
    let version = storage.projects().get(owner, pid).await.unwrap().version;
    storage
        .projects()
        .update(
            owner,
            pid,
            version,
            json!({
                "schema_version": 1,
                "objects": [cube("o1", &kept, [100.0, 100.0, 10.0])],
                "plates": [{ "id": "p1", "objectIds": ["o1"] }],
                "knownModels": { "projectId": project, "ids": [kept, removed] }
            }),
            json!({}),
            None,
        )
        .await
        .unwrap();

    let resp = call(
        &app,
        "GET",
        &format!("/api/projects/{project}/models"),
        &session,
        None,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    let listed: Vec<(String, Value)> = body
        .as_array()
        .unwrap()
        .iter()
        .map(|m| {
            (
                m["id"].as_str().unwrap().to_string(),
                m["referenced"].clone(),
            )
        })
        .collect();
    assert_eq!(listed.len(), 2, "{body}");
    assert!(listed.contains(&(kept.clone(), json!(true))), "{body}");
    assert!(
        listed.contains(&(interrupted.clone(), json!(false))),
        "{body}"
    );
    assert!(
        !std::path::Path::new(&removed_file).exists(),
        "fichier libéré"
    );
    assert_eq!(
        storage.models().list(owner, Some(pid)).await.unwrap().len(),
        2
    );
    drop(dir);
}
