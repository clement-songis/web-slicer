//! Lancement du tranchage (T064) : création de jobs (plateau / tous), presets
//! résolus figés, avertissements moteur restitués (FR-032), gardes avant slice
//! (plateau vide, objet hors plateau), isolation inter-comptes (404, SC-008).

use axum::body::{to_bytes, Body};
use axum::http::{header, Request, StatusCode};
use axum::response::Response;
use axum::Router;
use backend::adapters::files::FileStore;
use backend::adapters::storage::sqlite::SqliteStorage;
use backend::domain::repo::NewPrinter;
use backend::domain::{NewUser, PresetKind, PresetOrigin, Role, Storage, UserId};
use backend::http::routes::router;
use backend::http::state::AppState;
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt;
use tower_sessions::{MemoryStore, SessionManagerLayer};

async fn app() -> (tempfile::TempDir, Arc<SqliteStorage>, Router) {
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}", dir.path().join("slice.db").display());
    let storage = Arc::new(SqliteStorage::connect(&url).await.unwrap());
    let files = FileStore::new(dir.path().join("data"));
    let state = AppState::new(Arc::clone(&storage) as Arc<dyn Storage>, files);
    let session_layer = SessionManagerLayer::new(MemoryStore::default()).with_secure(false);
    (dir, storage, router(state, session_layer))
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

async fn body_json(resp: Response) -> Value {
    let b = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    if b.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&b).unwrap()
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
                    json!({ "email": email, "password": "password123" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    cookie(&resp)
}

/// Crée un projet directement en base avec une scène et des presets actifs donnés.
async fn create_project(
    storage: &SqliteStorage,
    app: &Router,
    email: &str,
    scene: Value,
    active_presets: Value,
) -> (String, UserId, String) {
    let session = register(app, email).await;
    let user = storage.users().find_by_email(email).await.unwrap().unwrap();
    let project = storage
        .projects()
        .create(
            user.id,
            backend::domain::NewProject {
                name: "P".into(),
                scene,
                active_presets,
                thumbnail_path: None,
            },
        )
        .await
        .unwrap();
    (project.id.to_string(), user.id, session)
}

/// Preset process utilisateur portant une valeur hors bornes (déclenche un
/// avertissement moteur au clamp, FR-032).
async fn make_bad_process(storage: &SqliteStorage, owner: UserId) -> String {
    let preset = storage
        .presets()
        .create_user_preset(
            owner,
            backend::domain::Preset {
                id: backend::domain::PresetId::new(),
                kind: PresetKind::Process,
                name: "Test process".into(),
                origin: PresetOrigin::User,
                user_id: Some(owner),
                vendor: None,
                inherits: None,
                instantiation: true,
                setting_id: None,
                filament_id: None,
                compatible_printers: None,
                values: json!({
                    "layer_height": "-1",
                    "compatible_printers": ["Bench 0.4 nozzle"]
                }),
            },
        )
        .await
        .unwrap();
    preset.id.to_string()
}

async fn make_valid_mirror_process(storage: &SqliteStorage, owner: UserId) -> String {
    let preset = storage
        .presets()
        .create_user_preset(
            owner,
            backend::domain::Preset {
                id: backend::domain::PresetId::new(),
                kind: PresetKind::Process,
                name: "Mirror process".into(),
                origin: PresetOrigin::User,
                user_id: Some(owner),
                vendor: None,
                inherits: None,
                instantiation: true,
                setting_id: None,
                filament_id: None,
                compatible_printers: None,
                values: json!({
                    "layer_height": "0.2",
                    "use_relative_e_distances": false,
                    "compatible_printers": ["Bench 0.4 nozzle"]
                }),
            },
        )
        .await
        .unwrap();
    preset.id.to_string()
}

async fn make_filament(
    storage: &SqliteStorage,
    owner: UserId,
    name: &str,
    material: &str,
    temperature: i64,
    retraction: Option<f64>,
) -> String {
    let retraction = retraction.map_or_else(|| "nil".into(), |value| value.to_string());
    let preset = storage
        .presets()
        .create_user_preset(
            owner,
            backend::domain::Preset {
                id: backend::domain::PresetId::new(),
                kind: PresetKind::Filament,
                name: name.into(),
                origin: PresetOrigin::User,
                user_id: Some(owner),
                vendor: None,
                inherits: None,
                instantiation: true,
                setting_id: None,
                filament_id: None,
                compatible_printers: None,
                values: json!({
                    "filament_type": [material],
                    "nozzle_temperature": [temperature.to_string()],
                    "filament_retraction_length": [retraction]
                }),
            },
        )
        .await
        .unwrap();
    preset.id.to_string()
}

/// Preset machine utilisateur (config vide, se résout sans avertissement).
async fn make_machine_preset(storage: &SqliteStorage, owner: UserId) -> String {
    make_named_machine_preset(storage, owner, "Ma machine").await
}

/// Idem, avec un nom explicite (pour tester la garde par **modèle** : deux buses
/// « `<Modèle> <buse> nozzle` » partagent le même modèle).
async fn make_named_machine_preset(storage: &SqliteStorage, owner: UserId, name: &str) -> String {
    let preset = storage
        .presets()
        .create_user_preset(
            owner,
            backend::domain::Preset {
                id: backend::domain::PresetId::new(),
                kind: PresetKind::Machine,
                name: name.into(),
                origin: PresetOrigin::User,
                user_id: Some(owner),
                vendor: None,
                inherits: None,
                instantiation: true,
                setting_id: None,
                filament_id: None,
                compatible_printers: None,
                values: json!({}),
            },
        )
        .await
        .unwrap();
    preset.id.to_string()
}

/// Déclare une imprimante possédée par `owner` pointant vers ce preset machine.
async fn own_printer(storage: &SqliteStorage, owner: UserId, machine_preset_id: &str) {
    storage
        .printers()
        .create(
            owner,
            NewPrinter {
                name: "Mienne".into(),
                moonraker_url: None,
                api_key: None,
                machine_preset_id: backend::domain::PresetId(
                    uuid::Uuid::parse_str(machine_preset_id).unwrap(),
                ),
            },
        )
        .await
        .unwrap();
}

/// Réécrit les presets actifs d'un projet (helper de test).
async fn set_active_presets(storage: &SqliteStorage, owner: UserId, project: &str, active: Value) {
    let id = backend::domain::ProjectId(uuid::Uuid::parse_str(project).unwrap());
    // Version courante (verrou optimiste) : le helper peut être appelé plusieurs fois.
    let version = storage.projects().get(owner, id).await.unwrap().version;
    storage
        .projects()
        .update(owner, id, version, scene_one_object(), active, None)
        .await
        .unwrap();
}

#[tokio::test]
async fn slice_allows_a_printer_the_user_owns() {
    let (_d, storage, app) = app().await;
    let (project, owner, session) = create_project(
        &storage,
        &app,
        "boss@test.local",
        scene_one_object(),
        json!({}),
    )
    .await;
    let machine = make_machine_preset(&storage, owner).await;
    own_printer(&storage, owner, &machine).await;
    set_active_presets(&storage, owner, &project, json!({ "printer": machine })).await;

    let resp = app
        .clone()
        .oneshot(slice_req(&project, &session, json!({ "plate_index": 0 })))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let jobs = storage.jobs().list(owner).await.unwrap();
    assert_eq!(
        jobs[0].resolved_settings["printer_settings_id"],
        "Ma machine"
    );
    assert!(jobs[0].resolved_settings.get("printer_variant").is_none());
}

#[tokio::test]
async fn slice_allows_a_sibling_nozzle_of_an_owned_model() {
    // Posséder « Bench 0.4 nozzle » autorise à trancher avec « Bench 0.6 nozzle »
    // (même modèle « Bench ») sans redéclarer l'imprimante (garde par modèle).
    let (_d, storage, app) = app().await;
    let (project, owner, session) = create_project(
        &storage,
        &app,
        "boss@test.local",
        scene_one_object(),
        json!({}),
    )
    .await;
    let owned = make_named_machine_preset(&storage, owner, "Bench 0.4 nozzle").await;
    let sibling = make_named_machine_preset(&storage, owner, "Bench 0.6 nozzle").await;
    own_printer(&storage, owner, &owned).await;
    set_active_presets(&storage, owner, &project, json!({ "printer": sibling })).await;

    let resp = app
        .clone()
        .oneshot(slice_req(&project, &session, json!({ "plate_index": 0 })))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let jobs = storage.jobs().list(owner).await.unwrap();
    assert_eq!(
        jobs[0].resolved_settings["printer_settings_id"],
        "Bench 0.6 nozzle"
    );
    assert_eq!(jobs[0].resolved_settings["printer_variant"], "0.6");
}

#[tokio::test]
async fn slice_rejects_a_printer_the_user_does_not_own() {
    let (_d, storage, app) = app().await;
    let (project, owner, session) = create_project(
        &storage,
        &app,
        "boss@test.local",
        scene_one_object(),
        json!({}),
    )
    .await;
    // Le preset existe, mais `owner` n'a déclaré aucune imprimante. Même possédé
    // par un AUTRE compte, il ne compte pas (isolation, SC-008).
    let other = storage
        .users()
        .create(NewUser {
            email: "bob@test.local".into(),
            password_hash: "h".into(),
            role: Role::User,
        })
        .await
        .unwrap();
    // Le preset et l'imprimante appartiennent tous deux à l'autre compte :
    // créer une imprimante pour un preset invisible à son propriétaire est
    // désormais refusé par le dépôt, ce qui rendait l'ancienne fixture invalide.
    let machine = make_machine_preset(&storage, other.id).await;
    own_printer(&storage, other.id, &machine).await;
    set_active_presets(&storage, owner, &project, json!({ "printer": machine })).await;

    let resp = app
        .clone()
        .oneshot(slice_req(&project, &session, json!({ "plate_index": 0 })))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body_json(resp).await["code"], "printer_not_owned");
}

/// Preset machine (système ou utilisateur) avec parent `inherits` explicite.
fn machine_preset(
    name: &str,
    origin: PresetOrigin,
    user_id: Option<UserId>,
    inherits: Option<&str>,
    instantiation: bool,
) -> backend::domain::Preset {
    backend::domain::Preset {
        id: backend::domain::PresetId::new(),
        kind: PresetKind::Machine,
        name: name.into(),
        origin,
        user_id,
        vendor: Some("Voron".into()),
        inherits: inherits.map(Into::into),
        instantiation,
        setting_id: None,
        filament_id: None,
        compatible_printers: None,
        values: json!({}),
    }
}

/// Seed système minimal : un parent abstrait et deux buses du modèle « Voron 2.4 300 ».
/// Renvoie l'id du preset système « Voron 2.4 300 0.4 nozzle ».
async fn seed_voron_system(storage: &SqliteStorage) -> String {
    let common = machine_preset(
        "fdm_klipper_common",
        PresetOrigin::System,
        None,
        None,
        false,
    );
    let v04 = machine_preset(
        "Voron 2.4 300 0.4 nozzle",
        PresetOrigin::System,
        None,
        Some("fdm_klipper_common"),
        true,
    );
    let v06 = machine_preset(
        "Voron 2.4 300 0.6 nozzle",
        PresetOrigin::System,
        None,
        Some("fdm_klipper_common"),
        true,
    );
    storage
        .presets()
        .reseed_system(vec![common, v04, v06])
        .await
        .unwrap();
    let machines = storage
        .presets()
        .list_by_kind(PresetKind::Machine, UserId::new())
        .await
        .unwrap();
    machines
        .iter()
        .find(|p| p.name == "Voron 2.4 300 0.4 nozzle")
        .unwrap()
        .id
        .to_string()
}

/// Copie utilisateur d'un preset machine (nom libre, `inherits` = parent).
async fn derive_machine(
    storage: &SqliteStorage,
    owner: UserId,
    name: &str,
    parent: &str,
) -> String {
    storage
        .presets()
        .create_user_preset(
            owner,
            machine_preset(name, PresetOrigin::User, Some(owner), Some(parent), true),
        )
        .await
        .unwrap()
        .id
        .to_string()
}

#[tokio::test]
async fn slice_allows_a_derived_machine_of_an_owned_model() {
    // « Voron 2.4 300 0.4 nozzle (copie) » (et une copie de cette copie) ne suit
    // pas la convention de nom Orca : le modèle est retrouvé en remontant
    // `inherits` jusqu'au preset système, que l'utilisateur possède.
    let (_d, storage, app) = app().await;
    let (project, owner, session) = create_project(
        &storage,
        &app,
        "boss@test.local",
        scene_one_object(),
        json!({}),
    )
    .await;
    let system = seed_voron_system(&storage).await;
    own_printer(&storage, owner, &system).await;
    let copy = derive_machine(
        &storage,
        owner,
        "Voron 2.4 300 0.4 nozzle (copie)",
        "Voron 2.4 300 0.4 nozzle",
    )
    .await;
    let copy_of_copy = derive_machine(
        &storage,
        owner,
        "Mon Voron réglé",
        "Voron 2.4 300 0.4 nozzle (copie)",
    )
    .await;

    for (preset, name) in [
        (&copy, "Voron 2.4 300 0.4 nozzle (copie)"),
        (&copy_of_copy, "Mon Voron réglé"),
    ] {
        set_active_presets(&storage, owner, &project, json!({ "printer": preset })).await;
        let resp = app
            .clone()
            .oneshot(slice_req(&project, &session, json!({ "plate_index": 0 })))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED, "{name}");
        let jobs = storage.jobs().list(owner).await.unwrap();
        assert!(jobs
            .iter()
            .any(|j| j.resolved_settings["printer_settings_id"] == name));
    }
}

#[tokio::test]
async fn slice_rejects_a_derived_machine_of_a_model_not_owned() {
    // Isolation conservée : dériver d'un preset système ne suffit pas, il faut
    // posséder son modèle — et l'imprimante d'un AUTRE compte ne compte pas.
    let (_d, storage, app) = app().await;
    let (project, owner, session) = create_project(
        &storage,
        &app,
        "boss@test.local",
        scene_one_object(),
        json!({}),
    )
    .await;
    let system = seed_voron_system(&storage).await;
    let other = storage
        .users()
        .create(NewUser {
            email: "bob@test.local".into(),
            password_hash: "h".into(),
            role: Role::User,
        })
        .await
        .unwrap();
    own_printer(&storage, other.id, &system).await;
    let copy = derive_machine(
        &storage,
        owner,
        "Voron 2.4 300 0.4 nozzle (copie)",
        "Voron 2.4 300 0.4 nozzle",
    )
    .await;
    // Dérivation uniquement depuis le parent abstrait : pas de modèle reconnu.
    let abstract_copy = derive_machine(
        &storage,
        owner,
        "Voron 2.4 300 0.4 nozzle (abstrait)",
        "fdm_klipper_common",
    )
    .await;
    // La copie de l'autre compte est invisible, même si `owner` possède le modèle.
    let foreign_copy = derive_machine(
        &storage,
        other.id,
        "Voron 2.4 300 0.4 nozzle (copie bob)",
        "Voron 2.4 300 0.4 nozzle",
    )
    .await;

    for preset in [&copy, &abstract_copy] {
        set_active_presets(&storage, owner, &project, json!({ "printer": preset })).await;
        let resp = app
            .clone()
            .oneshot(slice_req(&project, &session, json!({ "plate_index": 0 })))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(body_json(resp).await["code"], "printer_not_owned");
    }

    own_printer(&storage, owner, &system).await;
    set_active_presets(
        &storage,
        owner,
        &project,
        json!({ "printer": abstract_copy }),
    )
    .await;
    let resp = app
        .clone()
        .oneshot(slice_req(&project, &session, json!({ "plate_index": 0 })))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    set_active_presets(
        &storage,
        owner,
        &project,
        json!({ "printer": foreign_copy }),
    )
    .await;
    let resp = app
        .clone()
        .oneshot(slice_req(&project, &session, json!({ "plate_index": 0 })))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body_json(resp).await["code"], "printer_not_owned");
    assert!(storage.jobs().list(owner).await.unwrap().is_empty());
}

fn scene_one_object() -> Value {
    json!({
        "schema_version": 1,
        "objects": [{ "id": "o1", "modelId": "m1" }],
        "plates": [{ "id": "p1", "objectIds": ["o1"] }]
    })
}

fn slice_req(project: &str, session: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(format!("/api/projects/{project}/slice"))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::COOKIE, session)
        .body(Body::from(body.to_string()))
        .unwrap()
}

#[tokio::test]
async fn slices_a_plate_and_freezes_resolved_settings_with_warnings() {
    let (_d, storage, app) = app().await;
    let (project, owner, session) = create_project(
        &storage,
        &app,
        "boss@test.local",
        scene_one_object(),
        json!({}),
    )
    .await;
    let process = make_bad_process(&storage, owner).await;
    // Réécrit les presets actifs avec le process fautif.
    storage
        .projects()
        .update(
            owner,
            backend::domain::ProjectId(uuid::Uuid::parse_str(&project).unwrap()),
            1,
            scene_one_object(),
            json!({ "process": process }),
            None,
        )
        .await
        .unwrap();

    let resp = app
        .clone()
        .oneshot(slice_req(&project, &session, json!({ "plate_index": 0 })))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = body_json(resp).await;
    let jobs = body["jobs"].as_array().unwrap();
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0]["status"], "queued");
    assert_eq!(jobs[0]["plate_index"], 0);
    // Avertissement moteur restitué (layer_height clampé, FR-032).
    let warnings = body["warnings"].as_array().unwrap();
    assert!(
        warnings.iter().any(|w| w["key"] == "layer_height"),
        "avertissement layer_height attendu, obtenu {warnings:?}"
    );

    // Le job persiste ses resolved_settings figés.
    let jobs_db = storage.jobs().list(owner).await.unwrap();
    assert_eq!(jobs_db.len(), 1);
    assert!(jobs_db[0].resolved_settings.get("layer_height").is_some());
    assert_eq!(
        jobs_db[0].resolved_settings["print_settings_id"],
        "Test process"
    );
    assert_eq!(
        jobs_db[0].resolved_settings["print_compatible_printers"],
        json!(["Bench 0.4 nozzle"])
    );
}

#[tokio::test]
async fn freezes_multi_filament_vectors_and_scene_at_enqueue() {
    let (_d, storage, app) = app().await;
    let original_scene = scene_one_object();
    let (project, owner, session) = create_project(
        &storage,
        &app,
        "multi@test.local",
        original_scene.clone(),
        json!({}),
    )
    .await;
    let pla = make_filament(&storage, owner, "PLA", "PLA", 205, Some(0.8)).await;
    let petg = make_filament(&storage, owner, "PETG", "PETG", 235, None).await;
    storage
        .projects()
        .update(
            owner,
            backend::domain::ProjectId(uuid::Uuid::parse_str(&project).unwrap()),
            1,
            original_scene.clone(),
            json!({"filaments": [pla, petg]}),
            None,
        )
        .await
        .unwrap();

    let resp = app
        .clone()
        .oneshot(slice_req(&project, &session, json!({"plate_index": 0})))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    // Toute modification postérieure du projet ne doit pas altérer le job.
    let current_scene = json!({
        "schema_version": 1,
        "objects": [{"id": "new", "modelId": "different-model"}],
        "plates": [{"id": "p1", "objectIds": ["new"]}]
    });
    storage
        .projects()
        .update(
            owner,
            backend::domain::ProjectId(uuid::Uuid::parse_str(&project).unwrap()),
            2,
            current_scene,
            json!({"filaments": []}),
            None,
        )
        .await
        .unwrap();

    let jobs = storage.jobs().list(owner).await.unwrap();
    assert_eq!(jobs.len(), 1);
    let job = &jobs[0];
    assert_eq!(job.scene, Some(original_scene));
    assert_eq!(
        job.resolved_settings["filament_type"],
        json!(["PLA", "PETG"])
    );
    assert_eq!(
        job.resolved_settings["nozzle_temperature"],
        json!([205, 235])
    );
    assert_eq!(
        job.resolved_settings["filament_retraction_length"],
        json!([0.8, null])
    );
    assert_eq!(
        job.resolved_settings["filament_settings_id"],
        json!(["PLA", "PETG"])
    );
}

#[tokio::test]
async fn slices_all_plates_creates_one_job_each() {
    let (_d, storage, app) = app().await;
    let scene = json!({
        "schema_version": 1,
        "objects": [{ "id": "o1" }, { "id": "o2" }],
        "plates": [
            { "id": "p1", "objectIds": ["o1"] },
            { "id": "p2", "objectIds": ["o2"] }
        ]
    });
    let (project, _owner, session) =
        create_project(&storage, &app, "boss@test.local", scene, json!({})).await;

    let resp = app
        .clone()
        .oneshot(slice_req(&project, &session, json!({ "all": true })))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = body_json(resp).await;
    assert_eq!(body["jobs"].as_array().unwrap().len(), 2);
}

/// « Slice all » d'Orca saute les plateaux vides ; un plateau vide ciblé
/// explicitement reste refusé, comme un projet sans aucun objet.
#[tokio::test]
async fn slice_all_skips_empty_plates_like_orca() {
    let (_d, storage, app) = app().await;
    let scene = json!({
        "schema_version": 1,
        "objects": [{ "id": "o1" }, { "id": "o3" }],
        "plates": [
            { "id": "p1", "objectIds": ["o1"] },
            { "id": "p2", "objectIds": [] },
            { "id": "p3", "objectIds": ["o3"] }
        ]
    });
    let (project, _owner, session) =
        create_project(&storage, &app, "boss@test.local", scene, json!({})).await;

    let resp = app
        .clone()
        .oneshot(slice_req(&project, &session, json!({ "all": true })))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = body_json(resp).await;
    let plates: Vec<i64> = body["jobs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|j| j["plate_index"].as_i64().unwrap())
        .collect();
    assert_eq!(plates, vec![0, 2], "plateau 2 (vide) sauté");

    let resp = app
        .clone()
        .oneshot(slice_req(&project, &session, json!({ "plate_index": 1 })))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn slice_all_rejects_a_project_without_objects() {
    let (_d, storage, app) = app().await;
    let scene = json!({
        "schema_version": 1,
        "objects": [],
        "plates": [{ "id": "p1", "objectIds": [] }, { "id": "p2", "objectIds": [] }]
    });
    let (project, _owner, session) =
        create_project(&storage, &app, "boss@test.local", scene, json!({})).await;
    let resp = app
        .clone()
        .oneshot(slice_req(&project, &session, json!({ "all": true })))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn rejects_empty_plate_before_slice() {
    let (_d, storage, app) = app().await;
    let scene = json!({
        "schema_version": 1,
        "objects": [],
        "plates": [{ "id": "p1", "objectIds": [] }]
    });
    let (project, _owner, session) =
        create_project(&storage, &app, "boss@test.local", scene, json!({})).await;

    let resp = app
        .clone()
        .oneshot(slice_req(&project, &session, json!({ "plate_index": 0 })))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn rejects_object_off_bed_before_slice() {
    let (_d, storage, app) = app().await;
    let scene = json!({
        "schema_version": 1,
        "objects": [{ "id": "o1", "off_bed": true }],
        "plates": [{ "id": "p1", "objectIds": ["o1"] }]
    });
    let (project, _owner, session) =
        create_project(&storage, &app, "boss@test.local", scene, json!({})).await;

    let resp = app
        .clone()
        .oneshot(slice_req(&project, &session, json!({ "plate_index": 0 })))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn rejects_plate_index_out_of_range() {
    let (_d, storage, app) = app().await;
    let (project, _owner, session) = create_project(
        &storage,
        &app,
        "boss@test.local",
        scene_one_object(),
        json!({}),
    )
    .await;

    let resp = app
        .clone()
        .oneshot(slice_req(&project, &session, json!({ "plate_index": 9 })))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

/// SC-008 : trancher le projet d'autrui répond 404.
#[tokio::test]
async fn slice_others_project_is_404() {
    let (_d, storage, app) = app().await;
    let (project, _owner, _sa) = create_project(
        &storage,
        &app,
        "alice@test.local",
        scene_one_object(),
        json!({}),
    )
    .await;
    let bob = register(&app, "bob@test.local").await;

    let resp = app
        .clone()
        .oneshot(slice_req(&project, &bob, json!({ "plate_index": 0 })))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

/// STL ASCII d'un prisme en L centré (40 × 40 mm, bras de 10 mm, hauteur 6 mm).
fn l_prism_stl() -> Vec<u8> {
    let outline = [
        [0., 0.],
        [40., 0.],
        [40., 10.],
        [10., 10.],
        [10., 40.],
        [0., 40.],
    ];
    // Reconstruit explicitement les sommets 3D des faces (bas, haut, côtés).
    let mut out = String::from("solid L\n");
    let mut facet = |a: [f32; 3], b: [f32; 3], c: [f32; 3]| {
        out.push_str("facet normal 0 0 0\nouter loop\n");
        for mut v in [a, b, c] {
            v[0] -= 20.;
            v[1] -= 20.;
            out.push_str(&format!("vertex {} {} {}\n", v[0], v[1], v[2]));
        }
        out.push_str("endloop\nendfacet\n");
    };
    for i in 1..5 {
        let p = |i: usize, z: f32| [outline[i][0], outline[i][1], z];
        facet(p(0, 0.), p(i + 1, 0.), p(i, 0.));
        facet(p(0, 6.), p(i, 6.), p(i + 1, 6.));
    }
    for i in 0..6 {
        let j = (i + 1) % 6;
        let p = |i: usize, z: f32| [outline[i][0], outline[i][1], z];
        facet(p(i, 0.), p(j, 0.), p(j, 6.));
        facet(p(i, 0.), p(j, 6.), p(i, 6.));
    }
    out.push_str("endsolid L\n");
    out.into_bytes()
}

async fn upload_l(app: &Router, project: &str, session: &str) -> String {
    let boundary = "mirror-test-boundary";
    let mut body = format!("--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"L.stl\"\r\nContent-Type: application/sla\r\n\r\n").into_bytes();
    body.extend_from_slice(&l_prism_stl());
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/projects/{project}/models"))
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
    body_json(resp).await["id"].as_str().unwrap().to_string()
}

/// Tranche le L via HTTP + file + vrai worker, puis retourne le G-code stocké.
async fn slice_l_over_http(scale_x: f64) -> String {
    use backend::http::state::AppState;
    use backend::queue::{Queue, QueueConfig};
    use backend::slicing::SliceRunner;

    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}", dir.path().join("mirror.db").display());
    let storage = Arc::new(SqliteStorage::connect(&url).await.unwrap());
    let files = FileStore::new(dir.path().join("data"));
    let state = AppState::new(Arc::clone(&storage) as Arc<dyn Storage>, files);
    let session_layer = SessionManagerLayer::new(MemoryStore::default()).with_secure(false);
    let app = router(state.clone(), session_layer);
    let (project, owner, session) =
        create_project(&storage, &app, "mirror@test.local", json!({}), json!({})).await;
    let model = upload_l(&app, &project, &session).await;
    let machine = make_machine_preset(&storage, owner).await;
    own_printer(&storage, owner, &machine).await;
    let process = make_valid_mirror_process(&storage, owner).await;
    let filament = make_filament(&storage, owner, "PLA", "PLA", 205, Some(0.8)).await;
    let scene = json!({
        "schema_version": 1,
        "objects": [{"id": "L", "modelId": model, "transform": {
            "position": [100.0, 100.0, 3.0], "rotation": [0.0, 0.0, 0.0], "scale": [scale_x, 1.0, 1.0]
        }}],
        "plates": [{"id": "p1", "objectIds": ["L"]}]
    });
    storage
        .projects()
        .update(
            owner,
            backend::domain::ProjectId(uuid::Uuid::parse_str(&project).unwrap()),
            1,
            scene,
            json!({"printer": machine, "process": process, "filaments": [filament]}),
            None,
        )
        .await
        .unwrap();
    let resp = app
        .oneshot(slice_req(&project, &session, json!({"plate_index": 0})))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let job_id = storage.jobs().list(owner).await.unwrap()[0].id;

    let queue = Queue::new(
        Arc::clone(&storage) as Arc<dyn Storage>,
        Arc::new(SliceRunner::new(state)),
        QueueConfig {
            workers: 1,
            poll_interval: std::time::Duration::from_millis(10),
        },
    );
    let handle = Arc::new(queue).start().await;
    let job = loop {
        let job = storage.jobs().get(owner, job_id).await.unwrap();
        if !matches!(
            job.status,
            backend::domain::JobStatus::Queued | backend::domain::JobStatus::Running
        ) {
            break job;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    };
    handle.shutdown().await;
    assert_eq!(
        job.status,
        backend::domain::JobStatus::Succeeded,
        "erreur : {:?}",
        job.error
    );
    let gcode = storage
        .gcodes()
        .get(owner, job.gcode_id.unwrap())
        .await
        .unwrap();
    std::fs::read_to_string(gcode.file_path).unwrap()
}

fn outer_wall_layers(gcode: &str) -> (usize, Vec<[f64; 2]>) {
    let (mut layers, mut wall, mut out) = (0, false, Vec::new());
    let (mut x, mut y) = (0., 0.);
    for line in gcode.lines() {
        if line.starts_with(";LAYER_CHANGE") {
            layers += 1;
        } else if let Some(kind) = line.strip_prefix(";TYPE:") {
            wall = kind == "Outer wall";
        } else if line.starts_with("G1 ") {
            let mut extrusion = false;
            for word in line.split(';').next().unwrap_or("").split_whitespace() {
                let (key, value) = word.split_at(1);
                match key {
                    "X" => x = value.parse().unwrap_or(x),
                    "Y" => y = value.parse().unwrap_or(y),
                    "E" => extrusion = value.parse::<f64>().is_ok_and(|e| e > 0.),
                    _ => {}
                }
            }
            if wall && extrusion && layers > 0 {
                out.push([x, y]);
            }
        }
    }
    (layers, out)
}

#[tokio::test]
async fn http_slice_of_negative_scale_is_the_mirror_image() {
    // Nécessite le worker FFI déjà bâti ; ne lance pas de compilation imbriquée.
    let worker = std::env::var_os("ENGINE_WORKER_BIN")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::env::current_exe()
                .unwrap()
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join("engine-worker")
        });
    if !worker.exists() {
        eprintln!("engine-worker absent — test miroir HTTP ignoré");
        return;
    }
    let original = slice_l_over_http(1.).await;
    let mirrored = slice_l_over_http(-1.).await;
    let (layers, a) = outer_wall_layers(&original);
    let (mirror_layers, b) = outer_wall_layers(&mirrored);
    assert_eq!(mirror_layers, layers);
    assert!(layers >= 20 && !a.is_empty() && !b.is_empty());
    let bounds = |points: &[[f64; 2]]| {
        points.iter().fold([f64::MAX, f64::MIN], |[lo, hi], p| {
            [lo.min(p[0]), hi.max(p[0])]
        })
    };
    let [lo, hi] = bounds(&a);
    let [mlo, mhi] = bounds(&b);
    assert!(
        (mlo - (200. - hi)).abs() < 0.3,
        "min {mlo}, attendu {}",
        200. - hi
    );
    assert!(
        (mhi - (200. - lo)).abs() < 0.3,
        "max {mhi}, attendu {}",
        200. - lo
    );
    let arm = |points: &[[f64; 2]]| {
        bounds(
            &points
                .iter()
                .copied()
                .filter(|p| p[1] > 105.)
                .collect::<Vec<_>>(),
        )
    };
    assert!(arm(&a)[1] < 95., "bras original à gauche : {:?}", arm(&a));
    assert!(arm(&b)[0] > 105., "bras miroir à droite : {:?}", arm(&b));
}

/// Deux cubes de 10 mm disjoints (x ∈ [0,10] et [20,30]), STL ASCII.
fn two_cubes_stl() -> Vec<u8> {
    let mut out = String::from("solid pair\n");
    for x0 in [0.0f32, 20.0] {
        let p = |i: usize| {
            [
                x0 + if i & 1 != 0 { 10. } else { 0. },
                if i & 2 != 0 { 10. } else { 0. },
                if i & 4 != 0 { 10. } else { 0. },
            ]
        };
        for [a, b, c, d] in [
            [0, 2, 3, 1],
            [4, 5, 7, 6],
            [0, 1, 5, 4],
            [2, 6, 7, 3],
            [0, 4, 6, 2],
            [1, 3, 7, 5],
        ] {
            for [u, v, w] in [[a, b, c], [a, c, d]] {
                out.push_str("facet normal 0 0 0\n outer loop\n");
                for k in [u, v, w] {
                    let [x, y, z] = p(k);
                    out.push_str(&format!("  vertex {x} {y} {z}\n"));
                }
                out.push_str(" endloop\nendfacet\n");
            }
        }
    }
    out.push_str("endsolid pair\n");
    out.into_bytes()
}

/// « Split to parts » (Orca `split_volume`) : un objet dont la 2e composante est
/// un volume pièce maillé (`shape: "mesh"`, `modelId`) se tranche avec les deux
/// pièces et s'exporte en 3MF avec deux volumes `normal_part`.
#[tokio::test]
async fn split_to_parts_object_slices_both_mesh_volumes_and_exports_them() {
    use backend::queue::{Queue, QueueConfig};
    use backend::slicing::SliceRunner;
    use std::io::Read;

    let worker = std::env::var_os("ENGINE_WORKER_BIN")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::env::current_exe()
                .unwrap()
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join("engine-worker")
        });
    if !worker.exists() {
        eprintln!("engine-worker absent — test Split to parts ignoré");
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}", dir.path().join("parts.db").display());
    let storage = Arc::new(SqliteStorage::connect(&url).await.unwrap());
    let files = FileStore::new(dir.path().join("data"));
    let state = AppState::new(Arc::clone(&storage) as Arc<dyn Storage>, files);
    let session_layer = SessionManagerLayer::new(MemoryStore::default()).with_secure(false);
    let app = router(state.clone(), session_layer);
    let (project, owner, session) =
        create_project(&storage, &app, "parts@test.local", json!({}), json!({})).await;

    // Import du STL à deux composantes.
    let boundary = "parts-test-boundary";
    let mut body = format!("--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"pair.stl\"\r\nContent-Type: application/sla\r\n\r\n").into_bytes();
    body.extend_from_slice(&two_cubes_stl());
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/projects/{project}/models"))
                .header(
                    header::CONTENT_TYPE,
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .header(header::COOKIE, &session)
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let source = body_json(resp).await["id"].as_str().unwrap().to_string();

    // Split → deux pièces (modèles), centres dans le repère recentré de la source.
    // 409 tant que la conversion de l'import n'est pas terminée.
    let mut attempts = 0;
    let resp = loop {
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/models/{source}/split"))
                    .header(header::COOKIE, &session)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        attempts += 1;
        if resp.status() != StatusCode::CONFLICT || attempts > 600 {
            break resp;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    };
    assert_eq!(resp.status(), StatusCode::CREATED);
    let parts = body_json(resp).await["parts"].as_array().unwrap().clone();
    assert_eq!(parts.len(), 2);
    let model = |i: usize| parts[i]["model"]["id"].as_str().unwrap().to_string();
    let center = |i: usize| -> Vec<f64> {
        (0..3)
            .map(|a| parts[i]["center"][a].as_f64().unwrap())
            .collect()
    };
    let (c0, c1) = (center(0), center(1));
    let offset: Vec<f64> = (0..3).map(|a| c1[a] - c0[a]).collect();
    assert!((offset[0].abs() - 20.0).abs() < 1e-3, "décalage {offset:?}");

    let machine = make_machine_preset(&storage, owner).await;
    own_printer(&storage, owner, &machine).await;
    let process = make_valid_mirror_process(&storage, owner).await;
    let filament = make_filament(&storage, owner, "PLA", "PLA", 205, Some(0.8)).await;
    let scene = json!({
        "schema_version": 1,
        "objects": [{"id": "o", "modelId": model(0), "transform": {
            "position": [100.0 + c0[0], 100.0, 5.0], "rotation": [0.0, 0.0, 0.0],
            "scale": [1.0, 1.0, 1.0]
        }, "volumes": [{
            "id": "v1", "type": "part", "name": "pair_2", "shape": "mesh",
            "modelId": model(1), "size": [10.0, 10.0, 10.0], "offset": offset,
            "settings": {}
        }]}],
        "plates": [{"id": "p1", "objectIds": ["o"]}]
    });
    let project_id = backend::domain::ProjectId(uuid::Uuid::parse_str(&project).unwrap());
    storage
        .projects()
        .update(
            owner,
            project_id,
            1,
            scene,
            json!({"printer": machine, "process": process, "filaments": [filament]}),
            None,
        )
        .await
        .unwrap();

    // Tranchage : les deux pièces sont imprimées (parois sur ~30 mm en X).
    let resp = app
        .clone()
        .oneshot(slice_req(&project, &session, json!({"plate_index": 0})))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let job_id = storage.jobs().list(owner).await.unwrap()[0].id;
    let queue = Queue::new(
        Arc::clone(&storage) as Arc<dyn Storage>,
        Arc::new(SliceRunner::new(state)),
        QueueConfig {
            workers: 1,
            poll_interval: std::time::Duration::from_millis(10),
        },
    );
    let handle = Arc::new(queue).start().await;
    let job = loop {
        let job = storage.jobs().get(owner, job_id).await.unwrap();
        if !matches!(
            job.status,
            backend::domain::JobStatus::Queued | backend::domain::JobStatus::Running
        ) {
            break job;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    };
    handle.shutdown().await;
    assert_eq!(
        job.status,
        backend::domain::JobStatus::Succeeded,
        "erreur : {:?}",
        job.error
    );
    let gcode = storage
        .gcodes()
        .get(owner, job.gcode_id.unwrap())
        .await
        .unwrap();
    let gcode = std::fs::read_to_string(gcode.file_path).unwrap();
    let (layers, walls) = outer_wall_layers(&gcode);
    assert!(layers >= 20 && !walls.is_empty());
    let (lo, hi) = walls.iter().fold((f64::MAX, f64::MIN), |(lo, hi), p| {
        (lo.min(p[0]), hi.max(p[0]))
    });
    assert!(hi - lo > 28.0, "parois sur {lo}..{hi} : une pièce manque");

    // Export 3MF : deux volumes pièce dans l'objet, 24 triangles au total.
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/projects/{project}/export/3mf"))
                .header(header::COOKIE, &session)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes.as_ref())).unwrap();
    let mut settings = String::new();
    zip.by_name("Metadata/model_settings.config")
        .unwrap()
        .read_to_string(&mut settings)
        .unwrap();
    assert_eq!(
        settings.matches(r#"subtype="normal_part""#).count(),
        2,
        "{settings}"
    );
    let mut triangles = 0;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).unwrap();
        if entry.name().ends_with(".model") {
            let mut xml = String::new();
            entry.read_to_string(&mut xml).unwrap();
            triangles += xml.matches("<triangle ").count();
        }
    }
    assert_eq!(triangles, 24);
    drop(zip);

    // Réimport du 3MF : un objet, sa 2e pièce relue comme volume maillé.
    let boundary = "parts-reimport-boundary";
    let mut body = format!("--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"parts.3mf\"\r\nContent-Type: application/vnd.ms-package.3dmanufacturing-3dmodel+xml\r\n\r\n").into_bytes();
    body.extend_from_slice(&bytes);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/projects/import")
                .header(
                    header::CONTENT_TYPE,
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .header(header::COOKIE, &session)
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let imported = body_json(resp).await;
    let objects = imported["scene"]["objects"].as_array().unwrap();
    assert_eq!(objects.len(), 1, "{imported}");
    let volumes = objects[0]["volumes"].as_array().unwrap();
    let parts: Vec<&Value> = volumes
        .iter()
        .filter(|v| v["type"] == "part" && v["shape"] == "mesh")
        .collect();
    assert_eq!(parts.len(), 1, "volumes relus : {volumes:?}");
    assert!(parts[0]["modelId"].is_string());
    let dx = parts[0]["offset"][0].as_f64().unwrap();
    assert!((dx.abs() - 20.0).abs() < 1e-3, "décalage relu {dx}");
}

/// Hauteur de plage (Orca `layer_config_ranges`) : le prisme en L de 6 mm, avec
/// `layer_height = 0.1` sur [0, 3] mm, gagne ~15 couches par rapport au même
/// objet sans plage (0.2 mm partout), et la plage se retrouve dans l'export 3MF.
#[tokio::test]
async fn layer_config_ranges_change_the_layer_heights_of_their_z_range() {
    use backend::queue::{Queue, QueueConfig};
    use backend::slicing::SliceRunner;

    let worker = std::env::var_os("ENGINE_WORKER_BIN")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::env::current_exe()
                .unwrap()
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join("engine-worker")
        });
    if !worker.exists() {
        eprintln!("engine-worker absent — test layer_config_ranges ignoré");
        return;
    }

    async fn slice(ranges: Value) -> (String, Vec<u8>) {
        let dir = tempfile::tempdir().unwrap();
        let url = format!("sqlite://{}", dir.path().join("ranges.db").display());
        let storage = Arc::new(SqliteStorage::connect(&url).await.unwrap());
        let files = FileStore::new(dir.path().join("data"));
        let state = AppState::new(Arc::clone(&storage) as Arc<dyn Storage>, files);
        let session_layer = SessionManagerLayer::new(MemoryStore::default()).with_secure(false);
        let app = router(state.clone(), session_layer);
        let (project, owner, session) =
            create_project(&storage, &app, "ranges@test.local", json!({}), json!({})).await;
        let model = upload_l(&app, &project, &session).await;
        let machine = make_machine_preset(&storage, owner).await;
        own_printer(&storage, owner, &machine).await;
        let process = make_valid_mirror_process(&storage, owner).await;
        let filament = make_filament(&storage, owner, "PLA", "PLA", 205, Some(0.8)).await;
        let scene = json!({
            "schema_version": 1,
            "objects": [{"id": "L", "modelId": model, "transform": {
                "position": [100.0, 100.0, 3.0], "rotation": [0.0, 0.0, 0.0], "scale": [1.0, 1.0, 1.0]
            }, "layerRanges": ranges}],
            "plates": [{"id": "p1", "objectIds": ["L"]}]
        });
        storage
            .projects()
            .update(
                owner,
                backend::domain::ProjectId(uuid::Uuid::parse_str(&project).unwrap()),
                1,
                scene,
                json!({"printer": machine, "process": process, "filaments": [filament]}),
                None,
            )
            .await
            .unwrap();
        let resp = app
            .clone()
            .oneshot(slice_req(&project, &session, json!({"plate_index": 0})))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let job_id = storage.jobs().list(owner).await.unwrap()[0].id;
        let queue = Queue::new(
            Arc::clone(&storage) as Arc<dyn Storage>,
            Arc::new(SliceRunner::new(state)),
            QueueConfig {
                workers: 1,
                poll_interval: std::time::Duration::from_millis(10),
            },
        );
        let handle = Arc::new(queue).start().await;
        let job = loop {
            let job = storage.jobs().get(owner, job_id).await.unwrap();
            if !matches!(
                job.status,
                backend::domain::JobStatus::Queued | backend::domain::JobStatus::Running
            ) {
                break job;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        };
        handle.shutdown().await;
        assert_eq!(
            job.status,
            backend::domain::JobStatus::Succeeded,
            "erreur : {:?}",
            job.error
        );
        let gcode = storage
            .gcodes()
            .get(owner, job.gcode_id.unwrap())
            .await
            .unwrap();
        let gcode = std::fs::read_to_string(gcode.file_path).unwrap();
        let resp = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/projects/{project}/export/3mf"))
                    .header(header::COOKIE, &session)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap()
            .to_vec();
        (gcode, bytes)
    }

    let (plain, _) = slice(json!([])).await;
    let (ranged, archive) = slice(json!([
        {"min": 0.0, "max": 3.0, "settings": {"layer_height": 0.1, "extruder": 0}}
    ]))
    .await;
    let layers = |g: &str| g.lines().filter(|l| l.starts_with(";LAYER_CHANGE")).count();
    let (base, fine) = (layers(&plain), layers(&ranged));
    assert!(
        fine >= base + 12,
        "couches sans plage {base}, avec plage fine {fine}"
    );
    let heights: Vec<f64> = ranged
        .lines()
        .filter_map(|l| l.strip_prefix(";HEIGHT:"))
        .filter_map(|h| h.trim().parse().ok())
        .collect();
    assert!(
        heights.iter().any(|h| (h - 0.1).abs() < 1e-3),
        "aucune couche de 0.1 mm"
    );

    // Export 3MF : la plage est écrite (Orca `layer_config_ranges.xml`).
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(archive)).unwrap();
    let mut xml = String::new();
    {
        use std::io::Read;
        zip.by_name("Metadata/layer_config_ranges.xml")
            .expect("layer_config_ranges.xml absent")
            .read_to_string(&mut xml)
            .unwrap();
    }
    assert!(xml.contains("layer_height"), "{xml}");
}
