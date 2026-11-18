//! Contrat HTTP du harness (`harness/CONTRACT.md` §1).
//!
//! Les tests de slicing utilisent un faux `engine-worker` : ils vérifient le
//! pipeline multipart → décodage moteur → requête de slice → réponse, sans
//! imposer une compilation de libslic3r à la suite backend pure Rust.

use std::path::Path;
use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{header, Request, StatusCode};
use axum::response::Response;
use axum::Router;
use backend::adapters::files::FileStore;
use backend::adapters::storage::AnyStorage;
use backend::domain::presets::reseed_system_presets;
use backend::domain::{PresetOrigin, Storage};
use backend::http::routes::router;
use backend::http::state::AppState;
use engine::presets::{ImportedPreset, PresetKind as EnginePresetKind};
use serde_json::{json, Value};
use tower::ServiceExt;
use tower_sessions::{MemoryStore, SessionManagerLayer};

/// Les drapeaux du harness et le chemin du worker sont des variables de process.
/// Ce verrou évite toute course entre les tests de ce binaire d'intégration.
// Verrou asynchrone : il est tenu à travers des `.await` (variables d'env).
static ENV_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

const PRINTER: &str = "Test Printer 0.4 nozzle";
const FILAMENT: &str = "Generic Test PLA";
const PROCESS: &str = "0.20mm Test Standard";

async fn app(harness_enabled: bool) -> (tempfile::TempDir, Arc<AnyStorage>, Router) {
    if harness_enabled {
        std::env::set_var("HARNESS_API", "1");
    } else {
        std::env::remove_var("HARNESS_API");
    }
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}", dir.path().join("harness.db").display());
    let storage = Arc::new(AnyStorage::connect_test(&url).await.unwrap());
    let files = FileStore::new(dir.path().join("data"));
    let state = AppState::new(Arc::clone(&storage) as Arc<dyn Storage>, files);
    let session_layer = SessionManagerLayer::new(MemoryStore::default()).with_secure(false);
    let app = router(state, session_layer);
    std::env::remove_var("HARNESS_API");
    (dir, storage, app)
}

fn imported(name: &str, kind: EnginePresetKind) -> ImportedPreset {
    ImportedPreset {
        vendor: "TestVendor".into(),
        kind,
        name: name.into(),
        sub_path: String::new(),
        inherits: None,
        from: None,
        setting_id: None,
        filament_id: None,
        instantiation: true,
        compatible_printers: vec![],
        values: serde_json::Map::new(),
    }
}

async fn seed_system_presets(storage: &AnyStorage) {
    reseed_system_presets(
        storage,
        &[
            imported(PRINTER, EnginePresetKind::Machine),
            imported(FILAMENT, EnginePresetKind::Filament),
            imported(PROCESS, EnginePresetKind::Process),
        ],
    )
    .await
    .unwrap();
}

fn bootstrap_request(email: &str) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri("/api/_harness/bootstrap")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "email": email,
                "password": "harness-password-123",
                "printer": PRINTER,
                "filament": FILAMENT,
                "process": PROCESS,
            })
            .to_string(),
        ))
        .unwrap()
}

async fn response_bytes(response: Response) -> Vec<u8> {
    to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap()
        .to_vec()
}

async fn response_json(response: Response) -> Value {
    serde_json::from_slice(&response_bytes(response).await).unwrap()
}

fn cookie(response: &Response) -> String {
    response
        .headers()
        .get(header::SET_COOKIE)
        .expect("cookie de session")
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_string()
}

#[tokio::test]
async fn routes_are_absent_unless_harness_api_is_one() {
    let _guard = ENV_LOCK.lock().await;
    let (_dir, _storage, app) = app(false).await;

    for (uri, content_type, body) in [
        ("/api/_harness/bootstrap", "application/json", "{}".into()),
        (
            "/api/_harness/slice",
            "application/octet-stream",
            Vec::new(),
        ),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(uri)
                    .header(header::CONTENT_TYPE, content_type)
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            response.status(),
            StatusCode::NOT_FOUND,
            "route active : {uri}"
        );
    }
}

#[tokio::test]
async fn bootstrap_prepares_a_session_an_owned_printer_and_an_empty_project() {
    let _guard = ENV_LOCK.lock().await;
    let (_dir, storage, app) = app(true).await;
    seed_system_presets(&storage).await;

    let response = app
        .clone()
        .oneshot(bootstrap_request("bootstrap@harness.local"))
        .await
        .unwrap();
    if response.status() != StatusCode::OK {
        let status = response.status();
        panic!(
            "bootstrap a répondu {status} : {:?}",
            response_json(response).await
        );
    }
    let session = cookie(&response);
    let project_id = response_json(response).await["project_id"]
        .as_str()
        .unwrap()
        .to_string();

    let user = storage
        .users()
        .find_by_email("bootstrap@harness.local")
        .await
        .unwrap()
        .expect("utilisateur créé");
    backend::auth::authenticate(
        storage.as_ref(),
        "bootstrap@harness.local",
        "harness-password-123",
    )
    .await
    .expect("mot de passe initial utilisable");

    let printers = storage.printers().list(user.id).await.unwrap();
    assert_eq!(
        printers.len(),
        1,
        "le wizard est terminé par cette imprimante"
    );
    let machine = storage
        .presets()
        .get(printers[0].machine_preset_id)
        .await
        .unwrap();
    assert_eq!(machine.name, PRINTER);
    assert_eq!(machine.origin, PresetOrigin::System);

    let project = storage
        .projects()
        .get(
            user.id,
            backend::domain::ProjectId(uuid::Uuid::parse_str(&project_id).unwrap()),
        )
        .await
        .unwrap();
    assert_eq!(project.scene["objects"], json!([]));
    assert_eq!(project.active_presets["printer"], machine.id.to_string());
    let filament = storage
        .presets()
        .get(preset_id(&project.active_presets, "filament"))
        .await
        .unwrap();
    let process = storage
        .presets()
        .get(preset_id(&project.active_presets, "process"))
        .await
        .unwrap();
    assert_eq!(filament.name, FILAMENT);
    assert_eq!(process.name, PROCESS);
    assert_eq!(
        project.active_presets["filaments"],
        json!([filament.id.to_string()])
    );

    // Le cookie ouvert par bootstrap donne immédiatement accès au projet.
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/projects/{project_id}"))
                .header(header::COOKIE, &session)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Retrouver un compte ne le duplique pas et ouvre une nouvelle session.
    let response = app
        .clone()
        .oneshot(bootstrap_request("bootstrap@harness.local"))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.headers().contains_key(header::SET_COOKIE));
    assert_ne!(response_json(response).await["project_id"], project_id);
    assert_eq!(storage.users().count().await.unwrap(), 1);
}

fn preset_id(active: &Value, key: &str) -> backend::domain::PresetId {
    let raw = if key == "filament" {
        active["filaments"][0].as_str().unwrap()
    } else {
        active[key].as_str().unwrap()
    };
    backend::domain::PresetId(uuid::Uuid::parse_str(raw).unwrap())
}

#[tokio::test]
async fn bootstrap_rejects_an_unknown_system_preset() {
    let _guard = ENV_LOCK.lock().await;
    let (_dir, storage, app) = app(true).await;
    seed_system_presets(&storage).await;
    let mut request = bootstrap_request("unknown-preset@harness.local");
    let body = json!({
        "email": "unknown-preset@harness.local",
        "password": "harness-password-123",
        "printer": "Missing Printer",
        "filament": FILAMENT,
        "process": PROCESS,
    });
    *request.body_mut() = Body::from(body.to_string());

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert!(response_json(response).await["message"]
        .as_str()
        .unwrap()
        .contains("Missing Printer"));
}

/// Matrice renvoyée par le faux `arrange` : rotation Z de 90° et translation
/// arbitraire, que le harness doit remplacer par le centrage post-rotation.
fn fake_arranged_matrix() -> [f64; 16] {
    let mut matrix = glam::DMat4::from_rotation_z(std::f64::consts::FRAC_PI_2);
    matrix.w_axis = glam::DVec4::new(12.0, 34.0, 56.0, 1.0);
    matrix.to_cols_array()
}

/// Écrit un faux worker qui conserve les requêtes d'arrangement et de slice et
/// produit un G-code. Les chemins sont issus de `tempfile` et ne contiennent
/// pas d'apostrophe.
fn fake_worker(dir: &Path, fail_validation: bool) -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt;

    let mesh = engine::api::TriangleMesh {
        vertices: vec![
            [0.0, 0.0, 0.0],
            [20.0, 0.0, 0.0],
            [0.0, 20.0, 0.0],
            [0.0, 0.0, 20.0],
        ],
        indices: vec![[0, 1, 2], [0, 1, 3], [0, 2, 3], [1, 2, 3]],
    };
    let mesh_path = dir.join("mesh.wsmh");
    std::fs::write(&mesh_path, mesh.encode_display()).unwrap();
    let request_path = dir.join("captured-request.json");
    let arrange_request_path = dir.join("captured-arrange.json");
    let arranged = json!({
        "placements": [{ "id": "0:0", "plate_index": 0, "matrix": fake_arranged_matrix() }]
    });
    let gcode_path = dir.join("harness.gcode");
    let result = json!({
        "gcode_path": gcode_path,
        "stats": {
            "estimated_time_s": 1.0,
            "filament_mm": [1.0],
            "filament_g": [0.1],
            "layer_count": 1,
            "tool_changes": 0,
        },
        "thumbnails": [],
    });
    let slice_action = if fail_validation {
        "printf 'E engine_crashed configuration invalide : layer_height\\n' >&1; exit 1".to_string()
    } else {
        format!(
            "cp \"$2\" '{}'; printf '; fake harness\\nG90\\n' > '{}'; printf 'R %s\\n' '{}'",
            request_path.display(),
            gcode_path.display(),
            result.to_string().replace('\'', "'\\''"),
        )
    };
    let script = format!(
        "#!/bin/sh\ncase \"$1\" in\n  load-model) printf 'R %s\\n' '{}' ;;\n  arrange) cp \"$2\" '{}'; printf 'R %s\\n' '{}' ;;\n  slice) [ $# -eq 2 ] || exit 3; {slice_action} ;;\n  *) exit 2 ;;\nesac\n",
        mesh_path.display(),
        arrange_request_path.display(),
        arranged,
    );
    let path = dir.join("fake-engine-worker");
    std::fs::write(&path, script).unwrap();
    let mut permissions = std::fs::metadata(&path).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&path, permissions).unwrap();
    path
}

fn multipart(model_name: &str, model: &[u8], config: Value) -> (String, Vec<u8>) {
    let boundary = "----webslicerharnessboundary";
    let mut body = Vec::new();
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(
        format!(
            "Content-Disposition: form-data; name=\"model\"; filename=\"{model_name}\"\r\n\
             Content-Type: application/octet-stream\r\n\r\n"
        )
        .as_bytes(),
    );
    body.extend_from_slice(model);
    body.extend_from_slice(format!("\r\n--{boundary}\r\n").as_bytes());
    body.extend_from_slice(
        b"Content-Disposition: form-data; name=\"config\"\r\nContent-Type: application/json\r\n\r\n",
    );
    body.extend_from_slice(config.to_string().as_bytes());
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    (format!("multipart/form-data; boundary={boundary}"), body)
}

async fn post_slice(app: Router, content_type: String, body: Vec<u8>) -> Response {
    app.oneshot(
        Request::builder()
            .method("POST")
            .uri("/api/_harness/slice")
            .header(header::CONTENT_TYPE, content_type)
            .body(Body::from(body))
            .unwrap(),
    )
    .await
    .unwrap()
}

#[tokio::test]
async fn slice_decodes_step_and_returns_plain_gcode_with_the_flat_orca_config() {
    let _guard = ENV_LOCK.lock().await;
    let (_dir, _storage, app) = app(true).await;
    let worker_dir = tempfile::tempdir().unwrap();
    let worker = fake_worker(worker_dir.path(), false);
    std::env::set_var("ENGINE_WORKER_BIN", &worker);

    let (content_type, body) = multipart(
        "cube.step",
        b"ISO-10303-21;\nHEADER;\nENDSEC;",
        json!({
            "layer_height": "0.2",
            "nozzle_diameter": ["0.4"],
            "filament_type": ["PLA"],
            "sparse_infill_density": "20%",
            "printable_area": ["0x0", "350x0", "350x350", "0x350"],
            "unknown_vendor_key": "ignored-as-orca-does"
        }),
    );
    let response = post_slice(app, content_type, body).await;
    std::env::remove_var("ENGINE_WORKER_BIN");

    if response.status() != StatusCode::OK {
        let status = response.status();
        panic!(
            "slice a répondu {status} : {:?}",
            response_json(response).await
        );
    }
    assert_eq!(
        response.headers()[header::CONTENT_TYPE],
        "text/plain; charset=utf-8"
    );
    assert_eq!(response_bytes(response).await, b"; fake harness\nG90\n");

    let captured: Value = serde_json::from_slice(
        &std::fs::read(worker_dir.path().join("captured-request.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(captured["plate_index"], 0);
    assert_eq!(captured["config"]["layer_height"], 0.2);
    assert_eq!(captured["config"]["nozzle_diameter"], json!([0.4]));
    assert_eq!(captured["config"]["filament_type"], json!(["PLA"]));
    assert_eq!(captured["model"]["objects"].as_array().unwrap().len(), 1);
    let matrix: [f64; 16] =
        serde_json::from_value(captured["model"]["objects"][0]["instances"][0]["matrix"].clone())
            .unwrap();
    // Rotation de l'arrangeur conservée ; translation recalculée sur la boîte
    // APRÈS rotation : x' = -y ∈ [-20, 0], y' = x ∈ [0, 20], lit centré en 175.
    let expected = fake_arranged_matrix();
    assert_eq!(&matrix[..12], &expected[..12]);
    assert!((matrix[12] - 185.0).abs() < 1e-9, "x = {}", matrix[12]);
    assert!((matrix[13] - 165.0).abs() < 1e-9, "y = {}", matrix[13]);
    assert!(matrix[14].abs() < 1e-9, "z = {}", matrix[14]);
    assert!(captured["config"].get("unknown_vendor_key").is_none());

    // Le vrai arrange moteur est sollicité comme le CLI Orca : rotations
    // autorisées, dégagement nul, contour printable_area seul.
    let arrange: Value = serde_json::from_slice(
        &std::fs::read(worker_dir.path().join("captured-arrange.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(arrange["params"]["allow_rotations"], true);
    assert_eq!(arrange["params"]["clearance"], 0.0);
    assert_eq!(
        arrange["bed"]["bed_shape"],
        json!([[0.0, 0.0], [350.0, 0.0], [350.0, 350.0], [0.0, 350.0]])
    );
    assert_eq!(arrange["bed"]["excluded"], json!([]));
    assert_eq!(arrange["instance_ids"], json!(["0:0"]));
}

#[tokio::test]
async fn slice_maps_print_validate_failure_to_422_error() {
    let _guard = ENV_LOCK.lock().await;
    let (_dir, _storage, app) = app(true).await;
    let worker_dir = tempfile::tempdir().unwrap();
    let worker = fake_worker(worker_dir.path(), true);
    std::env::set_var("ENGINE_WORKER_BIN", &worker);

    let (content_type, body) = multipart(
        "cube.stl",
        b"solid cube\nendsolid cube\n",
        json!({ "layer_height": "5" }),
    );
    let response = post_slice(app, content_type, body).await;
    std::env::remove_var("ENGINE_WORKER_BIN");

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body = response_json(response).await;
    assert!(body["error"]
        .as_str()
        .unwrap()
        .contains("configuration invalide"));
}
