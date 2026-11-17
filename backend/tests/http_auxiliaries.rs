//! Fichiers auxiliaires d'un projet (Orca : Auxiliaries/ du 3MF) : API,
//! export 3MF (store_bbs_3mf) et réimport (load_bbs_3mf).

use axum::body::{to_bytes, Body};
use axum::http::{header, Request, StatusCode};
use axum::response::Response;
use axum::Router;
use backend::adapters::files::FileStore;
use backend::adapters::storage::sqlite::SqliteStorage;
use backend::http::routes::router;
use backend::http::state::AppState;
use serde_json::json;
use std::sync::Arc;
use tower::ServiceExt;
use tower_sessions::{MemoryStore, SessionManagerLayer};

async fn app() -> (tempfile::TempDir, Router) {
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}", dir.path().join("c.db").display());
    let storage = Arc::new(SqliteStorage::connect(&url).await.unwrap());
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

fn raw(method: &str, uri: &str, body: Vec<u8>, cookie: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(header::COOKIE, cookie)
        .body(Body::from(body))
        .unwrap()
}

#[tokio::test]
async fn auxiliaries_api_and_3mf_round_trip() {
    let (_d, app) = app().await;
    let user = register(&app, "aux@test.local").await;
    let project = json_body(
        send(
            &app,
            request(
                "POST",
                "/api/projects",
                json!({ "name": "Aux" }),
                Some(&user),
            ),
        )
        .await,
    )
    .await;
    let id = project["id"].as_str().unwrap().to_string();
    let base = format!("/api/projects/{id}/auxiliaries");

    let resp = send(
        &app,
        raw(
            "PUT",
            &format!("{base}/Others/notes.txt"),
            b"vis M3".to_vec(),
            &user,
        ),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let resp = send(
        &app,
        raw(
            "PUT",
            &format!("{base}/Model%20Pictures/vue.png"),
            b"\x89PNG".to_vec(),
            &user,
        ),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let resp = send(
        &app,
        raw(
            "PUT",
            &format!("{base}/Others/notes.txt"),
            b"vis M3 x 8".to_vec(),
            &user,
        ),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT, "remplacement");
    // Dossiers d'Orca seulement, pas de remontée.
    for bad in ["Divers/x.txt", "Others/..", "Others/.cache"] {
        let resp = send(
            &app,
            raw("PUT", &format!("{base}/{bad}"), b"x".to_vec(), &user),
        )
        .await;
        assert!(
            matches!(
                resp.status(),
                StatusCode::UNPROCESSABLE_ENTITY | StatusCode::NOT_FOUND | StatusCode::BAD_REQUEST
            ),
            "{bad} : {}",
            resp.status()
        );
    }
    let list = json_body(send(&app, request("GET", &base, json!({}), Some(&user))).await).await;
    assert_eq!(
        list,
        json!([{ "path": "Model Pictures/vue.png", "size": 4 }, { "path": "Others/notes.txt", "size": 10 }])
    );
    let resp = send(
        &app,
        request(
            "GET",
            &format!("{base}/Others/notes.txt"),
            json!({}),
            Some(&user),
        ),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap()
            .as_ref(),
        b"vis M3 x 8"
    );

    // Isolation.
    let other = register(&app, "aux2@test.local").await;
    let resp = send(&app, request("GET", &base, json!({}), Some(&other))).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    if backend::engine::worker_binary().exists() {
        // Export 3MF : entrées Auxiliaries/ ; réimport comme nouveau projet : fichiers restitués.
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
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes.to_vec())).unwrap();
        let mut names: Vec<String> = zip
            .file_names()
            .filter(|n| n.starts_with("Auxiliaries/"))
            .map(String::from)
            .collect();
        names.sort();
        assert_eq!(
            names,
            [
                "Auxiliaries/Model Pictures/vue.png",
                "Auxiliaries/Others/notes.txt"
            ]
        );
        let mut notes = Vec::new();
        std::io::Read::read_to_end(
            &mut zip.by_name("Auxiliaries/Others/notes.txt").unwrap(),
            &mut notes,
        )
        .unwrap();
        assert_eq!(notes, b"vis M3 x 8");
    }

    let resp = send(
        &app,
        request(
            "DELETE",
            &format!("{base}/Others/notes.txt"),
            json!({}),
            Some(&user),
        ),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    let list = json_body(send(&app, request("GET", &base, json!({}), Some(&user))).await).await;
    assert_eq!(list.as_array().unwrap().len(), 1);
}
