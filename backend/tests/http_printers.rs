//! Endpoints imprimantes Moonraker (T075/US8, contrat http-api.md) : CRUD avec
//! clé API chiffrée au repos, test de connexion, envoi, statut et contrôles
//! relayés vers l'instance simulée, échec réseau propre (502) et isolation
//! inter-comptes (404, SC-008).

mod moonraker_mock;

use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{header, Request, StatusCode};
use axum::response::Response;
use axum::Router;
use backend::adapters::files::FileStore;
use backend::adapters::storage::sqlite::SqliteStorage;
use backend::auth::SecretBox;
use backend::domain::repo::{NewGcode, NewJob, NewProject};
use backend::domain::{Preset, PresetKind, PresetOrigin, PrinterId, ProjectId, Storage, UserId};
use moonraker_mock::MockServer;
use serde_json::{json, Value};
use tower::ServiceExt;
use tower_sessions::{MemoryStore, SessionManagerLayer};

struct Harness {
    _dir: tempfile::TempDir,
    storage: Arc<SqliteStorage>,
    files: FileStore,
    app: Router,
}

async fn harness() -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}", dir.path().join("printers.db").display());
    let storage = Arc::new(SqliteStorage::connect(&url).await.unwrap());
    let files = FileStore::new(dir.path().join("data"));
    let state = backend::http::state::AppState::new(
        Arc::clone(&storage) as Arc<dyn Storage>,
        files.clone(),
    )
    // Clé déterministe : le chiffrement au repos reste reproductible en test.
    .with_secrets(SecretBox::new([42u8; 32]));
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

/// Crée un preset machine pour satisfaire la FK `machine_preset_id`.
async fn seed_machine_preset(storage: &SqliteStorage, owner: UserId) -> String {
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
                values: json!({}),
            },
        )
        .await
        .unwrap();
    preset.id.to_string()
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

/// Déclare une imprimante pointant vers `moonraker_url` et renvoie son id.
async fn declare_printer(
    h: &Harness,
    session: &str,
    preset_id: &str,
    moonraker_url: &str,
    api_key: Option<&str>,
) -> String {
    let resp = send(
        &h.app,
        "POST",
        "/api/printers",
        session,
        json!({
            "name": "Klipper",
            "moonraker_url": moonraker_url,
            "api_key": api_key,
            "machine_preset_id": preset_id,
        }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = json_body(resp).await;
    body["id"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn crud_never_exposes_the_api_key_and_stores_it_encrypted() {
    let h = harness().await;
    let session = register(&h.app, "boss@test.local").await;
    let owner = user_id(&h.storage, "boss@test.local").await;
    let preset = seed_machine_preset(&h.storage, owner).await;

    // Création : la clé API n'est jamais renvoyée, seul `has_api_key` la signale.
    let id = declare_printer(
        &h,
        &session,
        &preset,
        "http://printer.local",
        Some("s3cr3t"),
    )
    .await;
    let printer = json_body(get(&h.app, &format!("/api/printers/{id}"), &session).await).await;
    assert_eq!(printer["has_api_key"], true);
    assert!(printer.get("api_key").is_none(), "la clé ne fuit jamais");

    // Chiffrée au repos : la valeur stockée n'est pas le clair.
    let pid = PrinterId(uuid::Uuid::parse_str(&id).unwrap());
    let stored = h.storage.printers().get(owner, pid).await.unwrap();
    let at_rest = stored.api_key.unwrap();
    assert_ne!(at_rest, "s3cr3t", "la clé API est chiffrée au repos");
    assert!(!at_rest.contains("s3cr3t"));

    // Liste.
    let list = json_body(get(&h.app, "/api/printers", &session).await).await;
    assert_eq!(list.as_array().unwrap().len(), 1);

    // Mise à jour du nom.
    let resp = send(
        &h.app,
        "PUT",
        &format!("/api/printers/{id}"),
        &session,
        json!({
            "name": "Klipper V2",
            "moonraker_url": "http://printer.local",
            "api_key": null,
            "machine_preset_id": preset,
        }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let updated = json_body(resp).await;
    assert_eq!(updated["name"], "Klipper V2");

    // Suppression.
    let resp = send(
        &h.app,
        "DELETE",
        &format!("/api/printers/{id}"),
        &session,
        json!({}),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    let after = get(&h.app, &format!("/api/printers/{id}"), &session).await;
    assert_eq!(after.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_endpoint_relays_server_info_with_decrypted_key() {
    let mock = MockServer::start_with_key("live-key").await;
    let h = harness().await;
    let session = register(&h.app, "boss@test.local").await;
    let owner = user_id(&h.storage, "boss@test.local").await;
    let preset = seed_machine_preset(&h.storage, owner).await;
    let id = declare_printer(&h, &session, &preset, &mock.base_url(), Some("live-key")).await;

    let resp = send(
        &h.app,
        "POST",
        &format!("/api/printers/{id}/test"),
        &session,
        json!({}),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body["connected"], true);
    assert_eq!(body["klippy_state"], "ready");
}

#[tokio::test]
async fn upload_sends_owned_gcode_to_the_printer() {
    let mock = MockServer::start().await;
    let h = harness().await;
    let session = register(&h.app, "boss@test.local").await;
    let owner = user_id(&h.storage, "boss@test.local").await;
    let preset = seed_machine_preset(&h.storage, owner).await;
    let id = declare_printer(&h, &session, &preset, &mock.base_url(), None).await;

    // Un projet + job + G-code du compte, avec un fichier réel.
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
    let gcode_id = seed_gcode(&h, owner, project.id).await;

    let resp = send(
        &h.app,
        "POST",
        &format!("/api/printers/{id}/upload"),
        &session,
        json!({ "gcode_id": gcode_id, "start_now": true }),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body["print_started"], true);

    let uploads = mock.uploads();
    assert_eq!(uploads.len(), 1);
    assert!(uploads[0].start_now);
    // Le démarrage immédiat lance aussi l'impression côté hôte.
    assert_eq!(mock.actions(), vec!["print".to_string()]);
}

#[tokio::test]
async fn status_and_controls_reach_the_host() {
    let mock = MockServer::start().await;
    let h = harness().await;
    let session = register(&h.app, "boss@test.local").await;
    let owner = user_id(&h.storage, "boss@test.local").await;
    let preset = seed_machine_preset(&h.storage, owner).await;
    let id = declare_printer(&h, &session, &preset, &mock.base_url(), None).await;

    let status =
        json_body(get(&h.app, &format!("/api/printers/{id}/status"), &session).await).await;
    assert_eq!(status["state"], "printing");
    assert_eq!(status["filename"], "benchy.gcode");

    for action in ["pause", "resume", "cancel"] {
        let resp = send(
            &h.app,
            "POST",
            &format!("/api/printers/{id}/{action}"),
            &session,
            json!({}),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::NO_CONTENT, "{action}");
    }
    assert_eq!(
        mock.actions(),
        vec![
            "pause".to_string(),
            "resume".to_string(),
            "cancel".to_string()
        ]
    );
}

#[tokio::test]
async fn unreachable_host_yields_clean_502() {
    let h = harness().await;
    let session = register(&h.app, "boss@test.local").await;
    let owner = user_id(&h.storage, "boss@test.local").await;
    let preset = seed_machine_preset(&h.storage, owner).await;
    // Port fermé : l'hôte est injoignable (FR-062).
    let id = declare_printer(&h, &session, &preset, "http://127.0.0.1:1", None).await;

    let resp = send(
        &h.app,
        "POST",
        &format!("/api/printers/{id}/test"),
        &session,
        json!({}),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::BAD_GATEWAY);
    let body = json_body(resp).await;
    assert_eq!(body["code"], "printer_unreachable");
}

#[tokio::test]
async fn other_accounts_printer_is_404_everywhere() {
    let mock = MockServer::start().await;
    let h = harness().await;
    let alice = register(&h.app, "alice@test.local").await;
    let alice_id = user_id(&h.storage, "alice@test.local").await;
    let preset = seed_machine_preset(&h.storage, alice_id).await;
    let id = declare_printer(&h, &alice, &preset, &mock.base_url(), None).await;

    let bob = register(&h.app, "bob@test.local").await;
    // Toutes les sous-ressources de l'imprimante d'Alice sont invisibles pour Bob.
    assert_eq!(
        get(&h.app, &format!("/api/printers/{id}"), &bob)
            .await
            .status(),
        StatusCode::NOT_FOUND
    );
    for action in ["test", "pause", "resume", "cancel", "upload"] {
        let resp = send(
            &h.app,
            "POST",
            &format!("/api/printers/{id}/{action}"),
            &bob,
            json!({ "gcode_id": uuid::Uuid::new_v4().to_string(), "start_now": false }),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND, "{action}");
    }
    assert_eq!(
        get(&h.app, &format!("/api/printers/{id}/status"), &bob)
            .await
            .status(),
        StatusCode::NOT_FOUND
    );
    // Le mock n'a rien reçu de Bob.
    assert!(mock.actions().is_empty());
    assert!(mock.uploads().is_empty());
}

/// Écrit un fichier G-code réel et l'enregistre pour le compte donné.
async fn seed_gcode(h: &Harness, owner: UserId, project: ProjectId) -> String {
    let job = h
        .storage
        .jobs()
        .enqueue(
            owner,
            NewJob {
                project_id: project,
                plate_index: 0,
                resolved_settings: json!({}),
            },
        )
        .await
        .unwrap();
    let path = h
        .files
        .write_model(
            owner,
            backend::domain::ModelId::new(),
            "gcode",
            b"; G-code\nG28\n",
        )
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
