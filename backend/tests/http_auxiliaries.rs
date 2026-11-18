//! Fichiers auxiliaires d'un projet (Orca : Auxiliaries/ du 3MF) : API,
//! export 3MF (store_bbs_3mf) et réimport (load_bbs_3mf).

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

/// Comme Orca (Plater::priv::load_files) : les fichiers auxiliaires d'un 3MF ne
/// sont repris qu'à l'OUVERTURE du projet (LoadModel | LoadConfig → LoadAuxiliary,
/// Plater.cpp ~5970) ; l'ajout de sa géométrie à un projet existant (« Import
/// geometry only » : LoadModel seul) ne les reprend pas et garde ceux du projet.
#[tokio::test]
async fn auxiliaries_follow_orca_on_open_vs_add() {
    if !backend::engine::worker_binary().exists() {
        eprintln!("engine-worker absent — ignoré");
        return;
    }
    let bytes = std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../engine/tests/fixtures/auxiliaries/project_aux.3mf"),
    )
    .unwrap();
    let (_d, app) = app().await;
    let user = register(&app, "auxopen@test.local").await;

    // Ouverture comme projet : les 3 fichiers auxiliaires sont repris.
    let resp = send(
        &app,
        multipart("/api/projects/import", "project_aux.3mf", &bytes, &user),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let opened = json_body(resp).await["id"].as_str().unwrap().to_string();
    let list = json_body(
        send(
            &app,
            request(
                "GET",
                &format!("/api/projects/{opened}/auxiliaries"),
                json!({}),
                Some(&user),
            ),
        )
        .await,
    )
    .await;
    let paths: Vec<&str> = list
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["path"].as_str().unwrap())
        .collect();
    assert_eq!(
        paths,
        [
            "Bill of Materials/bom.csv",
            "Model Pictures/vue.png",
            "Others/notes.txt"
        ]
    );

    // Ajout dans un projet existant : géométrie seulement, auxiliaires du projet inchangés.
    let project = json_body(
        send(
            &app,
            request(
                "POST",
                "/api/projects",
                json!({ "name": "Cible" }),
                Some(&user),
            ),
        )
        .await,
    )
    .await;
    let id = project["id"].as_str().unwrap().to_string();
    let resp = send(
        &app,
        raw(
            "PUT",
            &format!("/api/projects/{id}/auxiliaries/Others/mien.txt"),
            b"a moi".to_vec(),
            &user,
        ),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let resp = send(
        &app,
        multipart(
            &format!("/api/projects/{id}/import-3mf"),
            "project_aux.3mf",
            &bytes,
            &user,
        ),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let list = json_body(
        send(
            &app,
            request(
                "GET",
                &format!("/api/projects/{id}/auxiliaries"),
                json!({}),
                Some(&user),
            ),
        )
        .await,
    )
    .await;
    assert_eq!(list, json!([{ "path": "Others/mien.txt", "size": 5 }]));
}

/// Limite de corps des fichiers auxiliaires : 3 Mo acceptés (au-delà des 2 Mo
/// par défaut d'axum), plus de 50 Mo refusés (413 par DefaultBodyLimit).
#[tokio::test]
async fn auxiliary_body_limit() {
    let (_d, app) = app().await;
    let user = register(&app, "auxsize@test.local").await;
    let project = json_body(
        send(
            &app,
            request(
                "POST",
                "/api/projects",
                json!({ "name": "Taille" }),
                Some(&user),
            ),
        )
        .await,
    )
    .await;
    let base = format!(
        "/api/projects/{}/auxiliaries",
        project["id"].as_str().unwrap()
    );
    let resp = send(
        &app,
        raw(
            "PUT",
            &format!("{base}/Others/gros.bin"),
            vec![7u8; 3 * 1024 * 1024],
            &user,
        ),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let list = json_body(send(&app, request("GET", &base, json!({}), Some(&user))).await).await;
    assert_eq!(list[0]["size"], 3 * 1024 * 1024);
    let resp = send(
        &app,
        raw(
            "PUT",
            &format!("{base}/Others/trop.bin"),
            vec![0u8; 50 * 1024 * 1024 + 2048],
            &user,
        ),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::PAYLOAD_TOO_LARGE);
    let list = json_body(send(&app, request("GET", &base, json!({}), Some(&user))).await).await;
    assert_eq!(
        list.as_array().unwrap().len(),
        1,
        "rien écrit au-delà de la limite"
    );
}
