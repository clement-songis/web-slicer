//! Exclusion d'objet en cours d'impression (Klipper EXCLUDE_OBJECT ; G-code
//! d'Orca avec « Exclude objects » : EXCLUDE_OBJECT_DEFINE, GCode.cpp:8104)
//! contre le Moonraker simulé du harness (harness/moonraker-mock.mjs, lancé
//! avec node ; test ignoré proprement si node est absent).

use std::sync::Arc;
use std::time::Duration;

use axum::body::{to_bytes, Body};
use axum::http::{header, Request, StatusCode};
use axum::response::Response;
use axum::Router;
use backend::adapters::files::FileStore;
use backend::adapters::storage::AnyStorage;
use backend::auth::SecretBox;
use backend::domain::repo::{NewGcode, NewJob, NewProject};
use backend::domain::{Preset, PresetKind, PresetOrigin, Storage, UserId};
use serde_json::{json, Value};
use tower::ServiceExt;
use tower_sessions::{MemoryStore, SessionManagerLayer};

struct Harness {
    _dir: tempfile::TempDir,
    storage: Arc<AnyStorage>,
    files: FileStore,
    app: Router,
}

async fn harness() -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}", dir.path().join("integration.db").display());
    let storage = Arc::new(AnyStorage::connect_test(&url).await.unwrap());
    let files = FileStore::new(dir.path().join("data"));
    let state = backend::http::state::AppState::new(
        Arc::clone(&storage) as Arc<dyn Storage>,
        files.clone(),
    )
    .with_secrets(SecretBox::new([7u8; 32]));
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

async fn user_id(storage: &AnyStorage, email: &str) -> UserId {
    storage
        .users()
        .find_by_email(email)
        .await
        .unwrap()
        .unwrap()
        .id
}

async fn send(app: &Router, method: &str, uri: &str, session: &str, body: Value) -> Response {
    app.clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .header(header::COOKIE, session)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap()
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

async fn json_body(resp: Response) -> Value {
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

async fn seed_machine_preset(storage: &AnyStorage, owner: UserId) -> String {
    let preset = storage
        .presets()
        .create_user_preset(
            owner,
            Preset {
                id: backend::domain::PresetId::new(),
                kind: PresetKind::Machine,
                name: "Test machine".into(),
                origin: PresetOrigin::User,
                user_id: Some(owner),
                vendor: None,
                inherits: None,
                instantiation: true,
                setting_id: None,
                filament_id: None,
                compatible_printers: None,
                renamed_from: Vec::new(),
                values: json!({}),
            },
        )
        .await
        .unwrap();
    preset.id.to_string()
}

/// Écrit un fichier G-code et enregistre sa ligne pour `owner`.
async fn seed_gcode(h: &Harness, owner: UserId, data: &[u8]) -> String {
    let project = h
        .storage
        .projects()
        .create(
            owner,
            NewProject {
                name: "P".into(),
                scene: json!({}),
                active_presets: json!({}),
                thumbnail_path: None,
            },
        )
        .await
        .unwrap();
    let job = h
        .storage
        .jobs()
        .enqueue(
            owner,
            NewJob {
                project_id: project.id,
                plate_index: 0,
                resolved_settings: json!({}),
            },
        )
        .await
        .unwrap();
    let path = h
        .files
        .write_model(owner, backend::domain::ModelId::new(), "gcode", data)
        .await
        .unwrap();
    let gcode = h
        .storage
        .gcodes()
        .create(
            owner,
            NewGcode {
                job_id: job.id,
                file_path: path.to_string_lossy().into_owned(),
                preview_path: String::new(),
                stats: json!({}),
                thumbnails: json!([]),
            },
        )
        .await
        .unwrap();
    gcode.id.to_string()
}

async fn declare_printer(h: &Harness, session: &str, preset: &str, url: &str) -> String {
    let resp = send(
        &h.app,
        "POST",
        "/api/printers",
        session,
        json!({
            "name": "Klipper",
            "moonraker_url": url,
            "api_key": null,
            "machine_preset_id": preset,
        }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    json_body(resp).await["id"].as_str().unwrap().to_string()
}

/// Lance harness/moonraker-mock.mjs sur un port libre ; None si node manque.
struct NodeMock {
    child: std::process::Child,
    url: String,
}

impl Drop for NodeMock {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

async fn node_mock() -> Option<NodeMock> {
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .ok()?
        .local_addr()
        .ok()?
        .port();
    let script = concat!(env!("CARGO_MANIFEST_DIR"), "/../harness/moonraker-mock.mjs");
    let child = std::process::Command::new("node")
        .arg(script)
        .arg(port.to_string())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .ok()?;
    let mock = NodeMock {
        child,
        url: format!("http://127.0.0.1:{port}"),
    };
    let http = reqwest::Client::new();
    for _ in 0..100 {
        if http
            .get(format!("{}/server/info", mock.url))
            .send()
            .await
            .is_ok_and(|r| r.status().is_success())
        {
            return Some(mock);
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    None
}

/// En-tête d'objets tel qu'Orca l'émet (flavor Klipper, exclude_object=1).
const LABELED_GCODE: &str = "; generated by OrcaSlicer 2.4.1\n\
EXCLUDE_OBJECT_DEFINE NAME=cube_id_0_copy_0 CENTER=100,100 POLYGON=[[90,90],[110,90],[110,110],[90,110],[90,90]]\n\
EXCLUDE_OBJECT_DEFINE NAME=cylinder_id_1_copy_0 CENTER=150,100 POLYGON=[[140,90],[160,90],[160,110],[140,110],[140,90]]\n\
G28\nEXCLUDE_OBJECT_START NAME=cube_id_0_copy_0\nG1 X100 Y100\nEXCLUDE_OBJECT_END NAME=cube_id_0_copy_0\n";

#[tokio::test]
async fn exclude_object_during_print_like_klipper() {
    let Some(mock) = node_mock().await else {
        eprintln!("node absent — test ignoré (harness/moonraker-mock.mjs)");
        return;
    };
    let h = harness().await;
    let session = register(&h.app, "excl@test.local").await;
    let owner = user_id(&h.storage, "excl@test.local").await;
    let preset = seed_machine_preset(&h.storage, owner).await;
    let printer = declare_printer(&h, &session, &preset, &mock.url).await;

    // Pas d'impression : liste vide, exclusion refusée (409).
    let resp = get(
        &h.app,
        &format!("/api/printers/{printer}/objects"),
        &session,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(json_body(resp).await["objects"], json!([]));
    let resp = send(
        &h.app,
        "POST",
        &format!("/api/printers/{printer}/objects/exclude"),
        &session,
        json!({ "name": "cube_id_0_copy_0" }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::CONFLICT);

    // Envoi + démarrage : les objets du G-code apparaissent.
    let gcode = seed_gcode(&h, owner, LABELED_GCODE.as_bytes()).await;
    let resp = send(
        &h.app,
        "POST",
        &format!("/api/printers/{printer}/upload"),
        &session,
        json!({ "gcode_id": gcode, "start_now": true }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let resp = get(
        &h.app,
        &format!("/api/printers/{printer}/objects"),
        &session,
    )
    .await;
    let st = json_body(resp).await;
    let names: Vec<_> = st["objects"]
        .as_array()
        .unwrap()
        .iter()
        .map(|o| o["name"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(names, ["CUBE_ID_0_COPY_0", "CYLINDER_ID_1_COPY_0"]);
    assert_eq!(st["objects"][0]["center"], json!([100.0, 100.0]));
    assert_eq!(st["objects"][0]["polygon"].as_array().unwrap().len(), 5);
    assert_eq!(st["current_object"], "CUBE_ID_0_COPY_0");

    // Nom invalide (injection) : 422 ; inconnu : 409.
    let resp = send(
        &h.app,
        "POST",
        &format!("/api/printers/{printer}/objects/exclude"),
        &session,
        json!({ "name": "cube\nM112" }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let resp = send(
        &h.app,
        "POST",
        &format!("/api/printers/{printer}/objects/exclude"),
        &session,
        json!({ "name": "nope" }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::CONFLICT);

    // Exclusion : commande Klipper reçue, état mis à jour, objet courant suivant.
    let resp = send(
        &h.app,
        "POST",
        &format!("/api/printers/{printer}/objects/exclude"),
        &session,
        json!({ "name": "cube_id_0_copy_0" }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let st = json_body(resp).await;
    assert_eq!(st["excluded_objects"], json!(["CUBE_ID_0_COPY_0"]));
    assert_eq!(st["current_object"], "CYLINDER_ID_1_COPY_0");
    let log: Value = reqwest::get(format!("{}/mock/gcode-log", mock.url))
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(
        log["result"]
            .as_array()
            .unwrap()
            .iter()
            .any(|l| l == "EXCLUDE_OBJECT NAME=CUBE_ID_0_COPY_0"),
        "{log}"
    );
    // Déjà exclu : 409.
    let resp = send(
        &h.app,
        "POST",
        &format!("/api/printers/{printer}/objects/exclude"),
        &session,
        json!({ "name": "CUBE_ID_0_COPY_0" }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::CONFLICT);
}
