//! Ouverture d'un G-code local (menu « Open G-code… ») : méta + couches WSPv
//! sans stockage, refus d'un fichier vide/non G-code, authentification requise.

use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{header, Request, StatusCode};
use axum::response::Response;
use axum::Router;
use backend::adapters::files::FileStore;
use backend::adapters::storage::AnyStorage;
use backend::domain::Storage;
use serde_json::Value;
use tower::ServiceExt;
use tower_sessions::{MemoryStore, SessionManagerLayer};

const SAMPLE_GCODE: &str = "\
G1 X0 Y0 Z0.2 F1200
;LAYER_CHANGE
;Z:0.2
;HEIGHT:0.2
;WIDTH:0.45
;TYPE:Outer wall
G1 X10 Y0 E0.5 F1800
G1 X10 Y10 E1.0
;LAYER_CHANGE
;Z:0.4
;TYPE:Outer wall
G1 X0 Y0 E2.0 F1800
";

async fn app() -> (tempfile::TempDir, Router) {
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}", dir.path().join("open.db").display());
    let storage = Arc::new(AnyStorage::connect_test(&url).await.unwrap());
    let state = backend::http::state::AppState::new(
        storage as Arc<dyn Storage>,
        FileStore::new(dir.path().join("data")),
    );
    let layer = SessionManagerLayer::new(MemoryStore::default()).with_secure(false);
    (dir, backend::http::routes::router(state, layer))
}

async fn send(app: &Router, req: Request<Body>) -> Response {
    app.clone().oneshot(req).await.unwrap()
}

async fn register(app: &Router) -> String {
    let resp = send(
        app,
        Request::builder()
            .method("POST")
            .uri("/api/auth/register")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                serde_json::json!({ "email": "gc@test.local", "password": "password123" })
                    .to_string(),
            ))
            .unwrap(),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    resp.headers()[header::SET_COOKIE]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_string()
}

fn post(uri: &str, cookie: Option<&str>, body: &str) -> Request<Body> {
    let mut builder = Request::builder()
        .method("POST")
        .uri(uri)
        .header(header::CONTENT_TYPE, "text/plain");
    if let Some(cookie) = cookie {
        builder = builder.header(header::COOKIE, cookie);
    }
    builder.body(Body::from(body.to_string())).unwrap()
}

#[tokio::test]
async fn local_gcode_preview_meta_and_layers() {
    let (_d, app) = app().await;
    let cookie = register(&app).await;

    let resp = send(
        &app,
        post("/api/gcode/preview?meta=1", Some(&cookie), SAMPLE_GCODE),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let meta: Value =
        serde_json::from_slice(&to_bytes(resp.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(meta["layer_count"], 2);

    let resp = send(
        &app,
        post(
            "/api/gcode/preview?from=0&to=1",
            Some(&cookie),
            SAMPLE_GCODE,
        ),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    assert_eq!(&bytes[..4], b"WSPv");
}

#[tokio::test]
async fn local_gcode_preview_rejects_empty_and_non_gcode() {
    let (_d, app) = app().await;
    let cookie = register(&app).await;
    for body in ["", "hello world\nnot gcode\n"] {
        let resp = send(&app, post("/api/gcode/preview?meta=1", Some(&cookie), body)).await;
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY, "{body:?}");
    }
    let resp = send(&app, post("/api/gcode/preview?meta=1", None, SAMPLE_GCODE)).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

// ---------------------------------------------------------------------------
// Lot 31 : simulation moteur (GCodeProcessor) d'un G-code externe ouvert.
// ---------------------------------------------------------------------------

/// `engine-worker` bâti (passe FFI) ; sinon les tests moteur sont ignorés,
/// comme dans tests/engine_worker.rs.
fn worker_built() -> bool {
    if let Ok(p) = std::env::var("ENGINE_WORKER_BIN") {
        return std::path::Path::new(&p).exists();
    }
    std::env::current_exe()
        .ok()
        .and_then(|e| e.parent()?.parent().map(|d| d.join("engine-worker")))
        .is_some_and(|b| b.exists())
}

fn orca_fixture(name: &str) -> String {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../engine/tests/fixtures")
        .join(name);
    std::fs::read_to_string(p).unwrap()
}

async fn register_as(app: &Router, email: &str) -> String {
    let resp = send(
        app,
        Request::builder()
            .method("POST")
            .uri("/api/auth/register")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                serde_json::json!({ "email": email, "password": "password123" }).to_string(),
            ))
            .unwrap(),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    resp.headers()[header::SET_COOKIE]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_string()
}

fn get_req(uri: &str, cookie: &str) -> Request<Body> {
    Request::builder()
        .uri(uri)
        .header(header::COOKIE, cookie)
        .body(Body::empty())
        .unwrap()
}

async fn json_of(resp: Response) -> Value {
    serde_json::from_slice(&to_bytes(resp.into_body(), usize::MAX).await.unwrap()).unwrap()
}

#[tokio::test]
async fn opened_gcode_is_simulated_by_gcode_processor() {
    if !worker_built() {
        eprintln!("engine-worker absent : test ignoré (passe FFI)");
        return;
    }
    let (_d, app) = app().await;
    let cookie = register(&app).await;
    // G-code Orca 2.4.1 : pause z=10 + deux G-code personnalisés (z=5, z=15).
    let text = orca_fixture("custom_gcode/cube_custom.orca.gcode");
    let resp = send(&app, post("/api/gcode/process", Some(&cookie), &text)).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_of(resp).await;
    let token = body["token"].as_str().unwrap().to_string();
    assert_eq!(token.len(), 32);
    // Temps / couches simulés = pied écrit par Orca (9m 55s, 100 couches).
    assert_eq!(body["stats"]["source"], "gcode_processor");
    let t = body["stats"]["estimated_time_seconds"].as_u64().unwrap();
    assert!(t.abs_diff(595) <= 1, "{t}");
    assert_eq!(body["footer"]["estimated_time_seconds"], 595);
    assert_eq!(body["stats"]["layer_count"], 100);
    assert_eq!(body["footer"]["layer_count"], 100);
    let g = body["stats"]["filament_used_g"][0].as_f64().unwrap();
    assert!((g - 3.89).abs() < 0.02, "{g} g vs pied 3.89 g");
    let mm = body["stats"]["filament_used_mm"][0].as_f64().unwrap();
    assert!((mm - 1304.61).abs() < 0.01 * 1304.61, "{mm} mm");
    // Types réels (temps par rôle) et vue Couleur.
    assert_eq!(body["meta"]["layer_count"], 100);
    assert!(body["meta"]["role_times"].as_array().unwrap().len() >= 20);
    assert_eq!(
        body["meta"]["color_print"]["markers"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    assert!(body["meta"]["seam_count"].as_u64().unwrap() >= 100);

    // Lecture paresseuse sans renvoyer le fichier.
    let resp = send(
        &app,
        get_req(&format!("/api/gcode/opened/{token}/meta"), &cookie),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(json_of(resp).await["layer_count"], 100);
    let resp = send(
        &app,
        get_req(
            &format!("/api/gcode/opened/{token}/moves?from=3&to=4"),
            &cookie,
        ),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    assert_eq!(&bytes[..4], b"WSPm");
    let resp = send(
        &app,
        get_req(
            &format!("/api/gcode/opened/{token}/seams?from=3&to=3"),
            &cookie,
        ),
    )
    .await;
    let seams = json_of(resp).await;
    assert_eq!(seams["seams"].as_array().unwrap().len(), 1);
    assert_eq!(seams["seams"][0]["layer"], 3);

    // Réouverture du même fichier : même jeton (empreinte), cache réutilisé.
    let resp = send(&app, post("/api/gcode/process", Some(&cookie), &text)).await;
    assert_eq!(json_of(resp).await["token"], token.as_str());

    // Isolation : jeton d'un autre compte ou invalide → 404.
    let other = register_as(&app, "other@test.local").await;
    let resp = send(
        &app,
        get_req(&format!("/api/gcode/opened/{token}/meta"), &other),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    for bad in ["..%2F..%2Fx", "zz", "0123456789abcdef0123456789abcdeZ"] {
        let resp = send(
            &app,
            get_req(&format!("/api/gcode/opened/{bad}/meta"), &cookie),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND, "{bad}");
    }
}

#[tokio::test]
async fn opened_gcode_process_requires_auth_and_content() {
    let (_d, app) = app().await;
    let cookie = register(&app).await;
    let resp = send(&app, post("/api/gcode/process", Some(&cookie), "")).await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let resp = send(&app, post("/api/gcode/process", None, SAMPLE_GCODE)).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn opened_gcodes_keep_only_the_latest_files() {
    let dir = tempfile::tempdir().unwrap();
    let files = FileStore::new(dir.path());
    let user = backend::domain::UserId::new();
    let tokens: Vec<String> = (0..6).map(|i| format!("{i:032x}")).collect();
    for t in &tokens {
        let p = files
            .write_opened_gcode(user, t, b"G1 X1\n", 4)
            .await
            .unwrap();
        std::fs::write(format!("{}.wspg", p.display()), b"cache").unwrap();
        // Dates de modification distinctes (ordre de rétention).
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    let kept: Vec<bool> = tokens
        .iter()
        .map(|t| files.opened_gcode_path(user, t).unwrap().exists())
        .collect();
    assert_eq!(kept, [false, false, true, true, true, true]);
    let p0 = files.opened_gcode_path(user, &tokens[0]).unwrap();
    assert!(!std::path::Path::new(&format!("{}.wspg", p0.display())).exists());
    assert!(files.opened_gcode_path(user, "../etc/passwd").is_err());
}
