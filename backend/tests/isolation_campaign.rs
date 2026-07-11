//! Campagne d'isolation automatisée (T083, SC-008 à 100 %) : pour **chaque**
//! ressource adressable par identifiant, un compte tiers (Bob) qui vise
//! directement la ressource d'un autre (Alice) reçoit **404** — jamais 403, ni
//! aucune fuite. Couvre projets, modèles, jobs, G-codes, presets et imprimantes,
//! ainsi que les identifiants inconnus/malformés (traités comme inexistants).

use std::sync::Arc;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use axum::response::Response;
use axum::Router;
use backend::adapters::files::FileStore;
use backend::adapters::storage::sqlite::SqliteStorage;
use backend::auth::SecretBox;
use backend::domain::repo::{NewGcode, NewJob, NewModel, NewPrinter, NewProject};
use backend::domain::{ModelFormat, ModelId, Preset, PresetKind, PresetOrigin, Storage, UserId};
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
    let url = format!("sqlite://{}", dir.path().join("isolation.db").display());
    let storage = Arc::new(SqliteStorage::connect(&url).await.unwrap());
    let files = FileStore::new(dir.path().join("data"));
    let state = backend::http::state::AppState::new(
        Arc::clone(&storage) as Arc<dyn Storage>,
        files.clone(),
    )
    .with_secrets(SecretBox::new([3u8; 32]));
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

/// Ressources d'Alice, toutes adressables par identifiant.
struct AliceResources {
    project: String,
    model: String,
    job: String,
    gcode: String,
    preset: String,
    printer: String,
}

async fn seed_alice(h: &Harness, owner: UserId) -> AliceResources {
    let project = h
        .storage
        .projects()
        .create(
            owner,
            NewProject {
                name: "Alice project".into(),
                scene: json!({}),
                active_presets: json!({}),
                thumbnail_path: None,
            },
        )
        .await
        .unwrap();

    let model = h
        .storage
        .models()
        .create(
            owner,
            NewModel {
                project_id: Some(project.id),
                filename: "part.stl".into(),
                format: ModelFormat::Stl,
                file_path: "/dev/null".into(),
                mesh_path: None,
                size_bytes: 1,
                triangle_count: 1,
                repair_report: None,
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
    // G-code avec un fichier réel (les endpoints qui lisent le fichier doivent
    // tout de même renvoyer 404 avant toute lecture pour un autre compte).
    let path = h
        .files
        .write_model(owner, ModelId::new(), "gcode", b"; g\nG28\n")
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

    let preset = h
        .storage
        .presets()
        .create_user_preset(
            owner,
            Preset {
                id: backend::domain::PresetId::new(),
                kind: PresetKind::Machine,
                name: "Alice machine".into(),
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

    let printer = h
        .storage
        .printers()
        .create(
            owner,
            NewPrinter {
                name: "Alice printer".into(),
                moonraker_url: "http://alice.local".into(),
                api_key: None,
                machine_preset_id: preset.id,
            },
        )
        .await
        .unwrap();

    AliceResources {
        project: project.id.to_string(),
        model: model.id.to_string(),
        job: job.id.to_string(),
        gcode: gcode.id.to_string(),
        preset: preset.id.to_string(),
        printer: printer.id.to_string(),
    }
}

async fn attempt(
    app: &Router,
    method: &str,
    uri: &str,
    session: &str,
    body: Option<Value>,
) -> Response {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::COOKIE, session);
    let body = match body {
        Some(v) => {
            builder = builder.header(header::CONTENT_TYPE, "application/json");
            Body::from(v.to_string())
        }
        None => Body::empty(),
    };
    app.clone()
        .oneshot(builder.body(body).unwrap())
        .await
        .unwrap()
}

#[tokio::test]
async fn every_cross_account_resource_access_is_404_never_403() {
    let h = harness().await;
    register(&h.app, "alice@test.local").await;
    let alice = user_id(&h.storage, "alice@test.local").await;
    let r = seed_alice(&h, alice).await;

    let bob = register(&h.app, "bob@test.local").await;
    let random = uuid::Uuid::new_v4().to_string();

    // (méthode, URI, corps JSON valide éventuel) — chaque appel vise une
    // ressource d'Alice depuis le compte de Bob.
    let save_body = json!({ "expected_version": 0, "scene": {}, "active_presets": {} });
    let arrange_body =
        json!({ "bed_width": 200.0, "bed_depth": 200.0, "spacing": 2.0, "items": [] });
    let cases: Vec<(&str, String, Option<Value>)> = vec![
        // Projets et sous-ressources projet.
        ("GET", format!("/api/projects/{}", r.project), None),
        (
            "PUT",
            format!("/api/projects/{}", r.project),
            Some(save_body.clone()),
        ),
        ("DELETE", format!("/api/projects/{}", r.project), None),
        (
            "POST",
            format!("/api/projects/{}/duplicate", r.project),
            None,
        ),
        (
            "PATCH",
            format!("/api/projects/{}/rename", r.project),
            Some(json!({ "name": "x" })),
        ),
        (
            "GET",
            format!("/api/projects/{}/thumbnail", r.project),
            None,
        ),
        (
            "POST",
            format!("/api/projects/{}/arrange", r.project),
            Some(arrange_body),
        ),
        (
            "POST",
            format!("/api/projects/{}/orient", r.project),
            Some(json!({ "model_id": r.model })),
        ),
        (
            "POST",
            format!("/api/projects/{}/slice", r.project),
            Some(json!({ "all": true })),
        ),
        (
            "GET",
            format!("/api/projects/{}/export/3mf", r.project),
            None,
        ),
        // Modèles.
        ("GET", format!("/api/models/{}/mesh", r.model), None),
        ("POST", format!("/api/models/{}/repair", r.model), None),
        // Jobs.
        ("GET", format!("/api/jobs/{}", r.job), None),
        ("POST", format!("/api/jobs/{}/cancel", r.job), None),
        // G-codes.
        ("GET", format!("/api/gcodes/{}/download", r.gcode), None),
        ("GET", format!("/api/gcodes/{}/stats", r.gcode), None),
        ("GET", format!("/api/gcodes/{}/preview/meta", r.gcode), None),
        (
            "GET",
            format!("/api/gcodes/{}/preview/layers", r.gcode),
            None,
        ),
        // Presets.
        ("GET", format!("/api/presets/{}", r.preset), None),
        (
            "PUT",
            format!("/api/presets/{}", r.preset),
            Some(json!({ "name": "x" })),
        ),
        ("DELETE", format!("/api/presets/{}", r.preset), None),
        ("GET", format!("/api/presets/{}/resolved", r.preset), None),
        ("GET", format!("/api/presets/{}/export", r.preset), None),
        // Imprimantes.
        ("GET", format!("/api/printers/{}", r.printer), None),
        (
            "PUT",
            format!("/api/printers/{}", r.printer),
            Some(
                json!({ "name": "x", "moonraker_url": "http://h", "api_key": null, "machine_preset_id": random }),
            ),
        ),
        ("DELETE", format!("/api/printers/{}", r.printer), None),
        ("POST", format!("/api/printers/{}/test", r.printer), None),
        (
            "POST",
            format!("/api/printers/{}/upload", r.printer),
            Some(json!({ "gcode_id": r.gcode, "start_now": false })),
        ),
        ("GET", format!("/api/printers/{}/status", r.printer), None),
        ("POST", format!("/api/printers/{}/pause", r.printer), None),
        ("POST", format!("/api/printers/{}/resume", r.printer), None),
        ("POST", format!("/api/printers/{}/cancel", r.printer), None),
    ];

    for (method, uri, body) in &cases {
        let resp = attempt(&h.app, method, uri, &bob, body.clone()).await;
        assert_eq!(
            resp.status(),
            StatusCode::NOT_FOUND,
            "{method} {uri} devrait renvoyer 404 pour un autre compte (SC-008)"
        );
        assert_ne!(
            resp.status(),
            StatusCode::FORBIDDEN,
            "{method} {uri} ne doit jamais révéler l'existence via 403 (SC-008)"
        );
    }

    // Identifiants inconnus (bien formés mais inexistants) : mêmes 404.
    for (method, uri) in [
        ("GET", format!("/api/projects/{random}")),
        ("GET", format!("/api/models/{random}/mesh")),
        ("GET", format!("/api/jobs/{random}")),
        ("GET", format!("/api/gcodes/{random}/stats")),
        ("GET", format!("/api/presets/{random}")),
        ("GET", format!("/api/printers/{random}")),
    ] {
        let resp = attempt(&h.app, method, &uri, &bob, None).await;
        assert_eq!(
            resp.status(),
            StatusCode::NOT_FOUND,
            "{method} {uri} (inconnu)"
        );
    }

    // Identifiants malformés (pas des UUID) : 404 également, jamais 500.
    for (method, uri) in [
        ("GET", "/api/projects/not-a-uuid"),
        ("GET", "/api/gcodes/xyz/stats"),
        ("GET", "/api/printers/%20/status"),
    ] {
        let resp = attempt(&h.app, method, uri, &bob, None).await;
        assert_eq!(
            resp.status(),
            StatusCode::NOT_FOUND,
            "{method} {uri} (malformé)"
        );
    }

    // Contrôle positif : Alice, elle, accède bien à sa ressource (pas de faux 404).
    let alice_session = register_existing(&h.app, "alice@test.local").await;
    let ok = attempt(
        &h.app,
        "GET",
        &format!("/api/projects/{}", r.project),
        &alice_session,
        None,
    )
    .await;
    assert_eq!(ok.status(), StatusCode::OK, "le propriétaire garde l'accès");
}

/// Reconnecte un compte existant (login) et renvoie son cookie de session.
async fn register_existing(app: &Router, email: &str) -> String {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({ "email": email, "password": "password123" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    cookie(&resp)
}
