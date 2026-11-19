//! Lot 34 — ouverture d'un 3MF « comme projet » / « géométrie seule »
//! (LoadStrategy d'Orca, préférence project_load_behaviour), avertissement
//! « G-code modifié » (PresetBundle::validate_presets), métadonnées du projet
//! (GET/PUT, export store_bbs_3mf, réimport) et options d'arrangement.

use axum::body::{to_bytes, Body};
use axum::http::{header, Request, StatusCode};
use axum::response::Response;
use axum::Router;
use backend::adapters::files::FileStore;
use backend::adapters::storage::AnyStorage;
use backend::http::routes::router;
use backend::http::state::AppState;
use serde_json::json;
use std::sync::Arc;
use tower::ServiceExt;
use tower_sessions::{MemoryStore, SessionManagerLayer};

async fn app() -> (tempfile::TempDir, Router) {
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}", dir.path().join("c.db").display());
    let storage = Arc::new(AnyStorage::connect_test(&url).await.unwrap());
    let state = AppState::new(storage, FileStore::new(dir.path().join("data")));
    (
        dir,
        router(
            state,
            SessionManagerLayer::new(MemoryStore::default()).with_secure(false),
        ),
    )
}

fn request(
    method: &str,
    uri: &str,
    body: serde_json::Value,
    cookie: Option<&str>,
) -> Request<Body> {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(c) = cookie {
        builder = builder.header(header::COOKIE, c);
    }
    builder.body(Body::from(body.to_string())).unwrap()
}

fn session_cookie(resp: &Response) -> String {
    resp.headers()
        .get(header::SET_COOKIE)
        .expect("Set-Cookie présent")
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_string()
}

async fn json_body(resp: Response) -> serde_json::Value {
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    if bytes.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    }
}

async fn register(app: &Router, email: &str) -> String {
    let resp = app
        .clone()
        .oneshot(request(
            "POST",
            "/api/auth/register",
            serde_json::json!({ "email": email, "password": "password123" }),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    session_cookie(&resp)
}

async fn send(app: &Router, req: Request<Body>) -> Response {
    app.clone().oneshot(req).await.unwrap()
}

fn multipart(uri: &str, filename: &str, bytes: &[u8], cookie: &str) -> Request<Body> {
    let boundary = "----webslicer-aux";
    let mut body = Vec::new();
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(
        format!("Content-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\n")
            .as_bytes(),
    );
    body.extend_from_slice(b"Content-Type: model/3mf\r\n\r\n");
    body.extend_from_slice(bytes);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    Request::builder()
        .method("POST")
        .uri(uri)
        .header(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={boundary}"),
        )
        .header(header::COOKIE, cookie)
        .body(Body::from(body))
        .unwrap()
}

fn fixture() -> Vec<u8> {
    std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../engine/tests/fixtures/auxiliaries/project_aux.3mf"),
    )
    .unwrap()
}

async fn get(app: &Router, uri: &str, user: &str) -> (StatusCode, serde_json::Value) {
    let resp = send(app, request("GET", uri, json!({}), Some(user))).await;
    let status = resp.status();
    (status, json_body(resp).await)
}

async fn put(
    app: &Router,
    uri: &str,
    body: serde_json::Value,
    user: &str,
) -> (StatusCode, serde_json::Value) {
    let resp = send(app, request("PUT", uri, body, Some(user))).await;
    let status = resp.status();
    (status, json_body(resp).await)
}

#[tokio::test]
async fn open_as_geometry_vs_project_and_preference() {
    if !backend::engine::worker_binary().exists() {
        eprintln!("engine-worker absent — ignoré");
        return;
    }
    let bytes = fixture();
    let (_d, app) = app().await;
    let user = register(&app, "lot34load@test.local").await;

    // Géométrie seule : ni config/presets, ni auxiliaires, ni métadonnées.
    let resp = send(
        &app,
        multipart("/api/projects/import?load=geometry", "p.3mf", &bytes, &user),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = json_body(resp).await;
    assert_eq!(body["import"]["load"], "geometry");
    assert!(body["import"].get("gcode_warning").is_none());
    assert_eq!(body["import"]["has_metadata"], false);
    let id = body["id"].as_str().unwrap().to_string();
    let (_, project) = get(&app, &format!("/api/projects/{id}"), &user).await;
    assert_eq!(project["active_presets"], json!({}));
    let (_, aux) = get(&app, &format!("/api/projects/{id}/auxiliaries"), &user).await;
    assert_eq!(aux, json!([]));

    // Projet (défaut) : config + auxiliaires.
    let resp = send(
        &app,
        multipart("/api/projects/import", "p.3mf", &bytes, &user),
    )
    .await;
    let body = json_body(resp).await;
    assert_eq!(body["import"]["load"], "project");
    if let Some(warning) = body["import"].get("gcode_warning") {
        let kind = warning["kind"].as_str().unwrap();
        assert!(
            kind == "modified_gcodes" || kind == "customized_presets",
            "{warning}"
        );
        assert!(!warning["items"].as_array().unwrap().is_empty());
    }
    let id = body["id"].as_str().unwrap().to_string();
    let (_, project) = get(&app, &format!("/api/projects/{id}"), &user).await;
    assert!(project["active_presets"].get("process").is_some());

    // Préférence du compte : géométrie seule par défaut, plus d'avertissement.
    let (status, _) = put(
        &app,
        "/api/me/preferences",
        json!({ "project_load_behaviour": "load_geometry_only", "no_warn_when_modified_gcodes": true }),
        &user,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let body = json_body(
        send(
            &app,
            multipart("/api/projects/import", "p.3mf", &bytes, &user),
        )
        .await,
    )
    .await;
    assert_eq!(body["import"]["load"], "geometry");
    // Paramètre explicite prioritaire ; avertissement coupé par la préférence.
    let body = json_body(
        send(
            &app,
            multipart("/api/projects/import?load=project", "p.3mf", &bytes, &user),
        )
        .await,
    )
    .await;
    assert_eq!(body["import"]["load"], "project");
    assert!(body["import"].get("gcode_warning").is_none());
}

#[tokio::test]
async fn metadata_get_put_export_and_reopen() {
    if !backend::engine::worker_binary().exists() {
        eprintln!("engine-worker absent — ignoré");
        return;
    }
    let (_d, app) = app().await;
    let user = register(&app, "lot34meta@test.local").await;
    let body = json_body(
        send(
            &app,
            multipart("/api/projects/import", "p.3mf", &fixture(), &user),
        )
        .await,
    )
    .await;
    let id = body["id"].as_str().unwrap().to_string();
    let uri = format!("/api/projects/{id}/metadata");

    let (status, meta) = get(&app, &uri, &user).await;
    assert_eq!(status, StatusCode::OK);
    assert!(meta["title"].is_string());

    // Validation : licence d'Orca, couverture parmi « Model Pictures ».
    let (status, _) = put(&app, &uri, json!({ "title": "T", "license": "MIT" }), &user).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let (status, _) = put(
        &app,
        &uri,
        json!({ "title": "T", "cover": "absent.png" }),
        &user,
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    let edit = json!({
        "title": "Support de casque",
        "designer": "Alice",
        "license": "BY-SA",
        "description": "Imprimé en PETG",
        "cover": "vue.png"
    });
    let (status, saved) = put(&app, &uri, edit.clone(), &user).await;
    assert_eq!(status, StatusCode::OK, "{saved}");
    assert_eq!(saved["display_author"], "Alice");
    let (_, again) = get(&app, &uri, &user).await;
    for key in ["title", "designer", "license", "description", "cover"] {
        assert_eq!(again[key], edit[key], "{key}");
    }

    // Export (store_bbs_3mf) puis réouverture : métadonnées relues.
    let resp = send(
        &app,
        request(
            "GET",
            &format!("/api/projects/{id}/export/3mf"),
            json!({}),
            Some(&user),
        ),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let exported = to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap()
        .to_vec();
    let body = json_body(
        send(
            &app,
            multipart("/api/projects/import", "re.3mf", &exported, &user),
        )
        .await,
    )
    .await;
    assert_eq!(body["import"]["has_metadata"], true);
    let reopened = body["id"].as_str().unwrap().to_string();
    let (_, meta) = get(&app, &format!("/api/projects/{reopened}/metadata"), &user).await;
    for key in ["title", "designer", "license", "description", "cover"] {
        assert_eq!(meta[key], edit[key], "{key} après réouverture");
    }

    // Duplication : copie des métadonnées.
    let resp = send(
        &app,
        request(
            "POST",
            &format!("/api/projects/{id}/duplicate"),
            json!({}),
            Some(&user),
        ),
    )
    .await;
    let copy = json_body(resp).await["id"].as_str().unwrap().to_string();
    let (_, meta) = get(&app, &format!("/api/projects/{copy}/metadata"), &user).await;
    assert_eq!(meta["title"], "Support de casque");
}

#[tokio::test]
async fn arrange_options_defaults_and_validation() {
    let (_d, app) = app().await;
    let user = register(&app, "lot34arr@test.local").await;
    let project = json_body(
        send(
            &app,
            request("POST", "/api/projects", json!({ "name": "A" }), Some(&user)),
        )
        .await,
    )
    .await;
    let id = project["id"].as_str().unwrap().to_string();
    let (status, info) = get(&app, &format!("/api/projects/{id}/arrange/options"), &user).await;
    assert_eq!(status, StatusCode::OK, "{info}");
    assert_eq!(info["defaults"]["spacing"], 0.0);
    assert_eq!(info["defaults"]["enable_rotation"], false);
    assert_eq!(
        info["defaults"]["allow_multi_materials_on_same_plate"],
        true
    );
    assert_eq!(info["defaults"]["avoid_extrusion_cali_region"], true);
    assert_eq!(info["show_avoid_extrusion_cali_region"], false);

    let resp = send(
        &app,
        request(
            "POST",
            &format!("/api/projects/{id}/arrange"),
            json!({ "bed_width": 200.0, "bed_depth": 200.0, "spacing": 0.0, "items": [],
                    "options": { "spacing": -1.0 } }),
            Some(&user),
        ),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}
