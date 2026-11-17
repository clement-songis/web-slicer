//! Outils moteur de scène (T054) de bout en bout : arrangement (FR-013),
//! rapport de réparation (FR-012), simplification quadrique (T055),
//! suggestion d'orientation, et isolation inter-comptes (404, SC-008).

use axum::body::{to_bytes, Body};
use axum::http::{header, Request, StatusCode};
use axum::response::Response;
use axum::Router;
use backend::adapters::files::FileStore;
use backend::adapters::storage::sqlite::SqliteStorage;
use backend::http::routes::router;
use backend::http::state::AppState;
use std::sync::Arc;
use tower::ServiceExt;
use tower_sessions::{MemoryStore, SessionManagerLayer};

async fn app() -> (tempfile::TempDir, Router, AppState) {
    let _ = tracing_subscriber::fmt().with_test_writer().try_init();
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}", dir.path().join("scene.db").display());
    let storage = Arc::new(SqliteStorage::connect(&url).await.unwrap());
    let files = FileStore::new(dir.path().join("data"));
    let state = AppState::new(storage, files);
    let session_layer = SessionManagerLayer::new(MemoryStore::default()).with_secure(false);
    let app = router(state.clone(), session_layer);
    (dir, app, state)
}

fn worker_available() -> bool {
    backend::engine::worker_binary().exists()
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
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/register")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::json!({ "email": email, "password": "password123" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    session_cookie(&resp)
}

async fn create_project(app: &Router, cookie: &str) -> String {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/projects")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie)
                .body(Body::from(
                    serde_json::json!({ "name": "Projet" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    json_body(resp).await["id"].as_str().unwrap().to_string()
}

async fn user_id(app: &Router, cookie: &str) -> backend::domain::UserId {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/auth/me")
                .header(header::COOKIE, cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let id = json_body(resp).await["id"].as_str().unwrap().to_string();
    backend::domain::UserId(uuid::Uuid::parse_str(&id).unwrap())
}

async fn wait_for_mesh(app: &Router, model: &str, cookie: &str) {
    for _ in 0..100 {
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/models/{model}/mesh"))
                    .header(header::COOKIE, cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        if resp.status() == StatusCode::OK {
            return;
        }
        assert_eq!(
            resp.status(),
            StatusCode::CONFLICT,
            "conversion encore en cours"
        );
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    panic!("conversion du modèle non terminée");
}

/// STL binaire d'un unique triangle aux sommets donnés.
fn binary_stl_tri(verts: [[f32; 3]; 3]) -> Vec<u8> {
    let mut v = vec![0u8; 80];
    v.extend_from_slice(&1u32.to_le_bytes());
    for f in [0.0f32, 0.0, 1.0] {
        v.extend_from_slice(&f.to_le_bytes());
    }
    for vert in verts {
        for c in vert {
            v.extend_from_slice(&c.to_le_bytes());
        }
    }
    v.extend_from_slice(&0u16.to_le_bytes());
    v
}

fn multipart(filename: &str, content: &[u8]) -> (String, Vec<u8>) {
    let boundary = "----webslicertestboundary";
    let mut body = Vec::new();
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(
        format!("Content-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\n")
            .as_bytes(),
    );
    body.extend_from_slice(b"Content-Type: application/octet-stream\r\n\r\n");
    body.extend_from_slice(content);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    (format!("multipart/form-data; boundary={boundary}"), body)
}

async fn upload_model(
    app: &Router,
    project: &str,
    cookie: &str,
    name: &str,
    body: &[u8],
) -> String {
    let (ct, payload) = multipart(name, body);
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/projects/{project}/models"))
                .header(header::CONTENT_TYPE, ct)
                .header(header::COOKIE, cookie)
                .body(Body::from(payload))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    json_body(resp).await["id"].as_str().unwrap().to_string()
}

fn post(uri: &str, cookie: &str, body: serde_json::Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::COOKIE, cookie)
        .body(Body::from(body.to_string()))
        .unwrap()
}

#[tokio::test]
async fn arrange_places_objects_without_collision() {
    if !worker_available() {
        eprintln!("engine-worker absent — test d'arrangement réel ignoré");
        return;
    }
    let (_d, app, _state) = app().await;
    let user = register(&app, "boss@test.local").await;
    let project = create_project(&app, &user).await;

    let resp = app
        .clone()
        .oneshot(post(
            &format!("/api/projects/{project}/arrange"),
            &user,
            serde_json::json!({
                "bed_width": 100.0, "bed_depth": 100.0, "spacing": 5.0,
                "items": [
                    { "id": "a", "width": 20.0, "depth": 20.0 },
                    { "id": "b", "width": 20.0, "depth": 20.0 }
                ]
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    let placements = body["placements"].as_array().unwrap();
    assert_eq!(placements.len(), 2);
    assert_eq!(placements[0]["id"], "a");
    assert!(placements[0]["x"].as_f64().unwrap().is_finite());
    assert!(placements[1]["x"].as_f64().unwrap().is_finite());
    assert_eq!(placements[0]["plate_index"], 0);
    assert_eq!(placements[0]["matrix"].as_array().unwrap().len(), 16);
    let a = placements[0]["matrix"].as_array().unwrap();
    let b = placements[1]["matrix"].as_array().unwrap();
    let world_box = |m: &Vec<serde_json::Value>| {
        let mut lo = [f64::INFINITY; 2];
        let mut hi = [f64::NEG_INFINITY; 2];
        for (x, y, z) in [
            (-10.0, -10.0, 0.0),
            (10.0, -10.0, 0.0),
            (10.0, 10.0, 0.0),
            (-10.0, 10.0, 0.0),
            (-10.0, -10.0, 1.0),
            (10.0, -10.0, 1.0),
            (10.0, 10.0, 1.0),
            (-10.0, 10.0, 1.0),
        ] {
            for axis in 0..2 {
                let value = m[axis].as_f64().unwrap() * x
                    + m[4 + axis].as_f64().unwrap() * y
                    + m[8 + axis].as_f64().unwrap() * z
                    + m[12 + axis].as_f64().unwrap();
                lo[axis] = lo[axis].min(value);
                hi[axis] = hi[axis].max(value);
            }
        }
        (lo, hi)
    };
    let (lo_a, hi_a) = world_box(a);
    let (lo_b, hi_b) = world_box(b);
    let overlap_x = hi_a[0].min(hi_b[0]) - lo_a[0].max(lo_b[0]);
    let overlap_y = hi_a[1].min(hi_b[1]) - lo_a[1].max(lo_b[1]);
    assert!(overlap_x <= 1e-3 || overlap_y <= 1e-3, "{body}");
}

/// `POST /api/flush-volumes` : matrice de purge calculée par libslic3r depuis
/// les couleurs (FlushVolCalculator d'Orca) ; valeurs de two_objects.3mf
/// (#F2754E → #FF0000, minimum 0, jeu de données 0).
#[tokio::test]
async fn flush_volumes_from_colours() {
    if !worker_available() {
        eprintln!("engine-worker absent — ignoré");
        return;
    }
    let (_d, app, _state) = app().await;
    let user = register(&app, "flush@test.local").await;
    let resp = app
        .clone()
        .oneshot(post(
            "/api/flush-volumes",
            &user,
            serde_json::json!({ "colours": ["#F2754E", "#FF0000"], "multiplier": 1.5 }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    let matrix: Vec<f64> = serde_json::from_value(body["matrix"].clone()).unwrap();
    assert_eq!(matrix.len(), 4);
    assert_eq!((matrix[0], matrix[3]), (0.0, 0.0));
    assert!(matrix[1] > 0.0 && matrix[2] > 0.0, "{body}");
    assert_eq!(body["multiplier"], 1.5);
    // Couleur invalide / liste vide → 422 (l'interface sonde la route ainsi).
    for colours in [serde_json::json!(["rouge"]), serde_json::json!([])] {
        let resp = app
            .clone()
            .oneshot(post(
                "/api/flush-volumes",
                &user,
                serde_json::json!({ "colours": colours }),
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }
}

/// « Remplir le plateau » (FillBedJob d'Orca) : copies de l'objet ciblé
/// placées par libslic3r autour de l'objet fixe, sans chevauchement.
#[tokio::test]
async fn fill_bed_adds_instances_around_fixed_objects() {
    if !worker_available() {
        eprintln!("engine-worker absent — test de remplissage réel ignoré");
        return;
    }
    let (_d, app, _state) = app().await;
    let user = register(&app, "fill@test.local").await;
    let project = create_project(&app, &user).await;
    let resp = app
        .clone()
        .oneshot(post(
            &format!("/api/projects/{project}/arrange"),
            &user,
            serde_json::json!({
                "bed_width": 100.0, "bed_depth": 100.0, "spacing": 0.0,
                "items": [
                    { "id": "a", "width": 20.0, "depth": 20.0 },
                    { "id": "fixe", "width": 30.0, "depth": 30.0 }
                ],
                "fill_object_id": "a"
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    let placements = body["placements"].as_array().unwrap();
    assert!(placements.len() > 4, "{body}");
    assert_eq!(placements[0]["id"], "a");
    assert!(
        placements[1..]
            .iter()
            .all(|p| p["id"].as_str().unwrap().starts_with("a#fill")),
        "{body}"
    );
    for i in 0..placements.len() {
        for j in i + 1..placements.len() {
            let (p, q) = (&placements[i], &placements[j]);
            let dx = (p["x"].as_f64().unwrap() - q["x"].as_f64().unwrap()).abs();
            let dy = (p["y"].as_f64().unwrap() - q["y"].as_f64().unwrap()).abs();
            assert!(
                dx >= 20.0 - 1e-3 || dy >= 20.0 - 1e-3,
                "copies {i}/{j} superposées : {body}"
            );
        }
    }
}

/// Arrangement séquentiel (objet par objet) : config du projet + dégagement
/// extrudeur → arrangement libslic3r du CLI Orca, `print_order` renvoyé, objets
/// espacés d'au moins `extruder_clearance_radius`.
#[tokio::test]
async fn sequential_arrange_spaces_objects_by_the_extruder_clearance() {
    if !worker_available() {
        eprintln!("engine-worker absent — test d'arrangement réel ignoré");
        return;
    }
    let (_d, app, _state) = app().await;
    let user = register(&app, "seq@test.local").await;
    let project = create_project(&app, &user).await;
    let items = serde_json::json!([
        { "id": "a", "width": 20.0, "depth": 20.0 },
        { "id": "b", "width": 20.0, "depth": 20.0 },
        { "id": "c", "width": 20.0, "depth": 20.0 }
    ]);
    let arrange = |seq: bool| {
        post(
            &format!("/api/projects/{project}/arrange"),
            &user,
            serde_json::json!({
                "bed_width": 350.0, "bed_depth": 350.0, "spacing": 5.0,
                "items": items, "is_seq_print": seq, "extruder_clearance_radius": 65.0
            }),
        )
    };
    let resp = app.clone().oneshot(arrange(true)).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    let placements = body["placements"].as_array().unwrap();
    let mut orders: Vec<u64> = placements
        .iter()
        .map(|p| p["print_order"].as_u64().expect("print_order"))
        .collect();
    orders.sort();
    assert_eq!(orders, vec![1, 2, 3], "{body}");
    for i in 0..3 {
        for j in i + 1..3 {
            let (a, b) = (&placements[i], &placements[j]);
            let dx = (a["x"].as_f64().unwrap() - b["x"].as_f64().unwrap()).abs() - 20.0;
            let dy = (a["y"].as_f64().unwrap() - b["y"].as_f64().unwrap()).abs() - 20.0;
            assert!(
                dx.max(dy) >= 65.0 - 1e-3,
                "objets {i}/{j} trop proches : {body}"
            );
        }
    }

    // Couche par couche : pas d'ordre, écart standard.
    let resp = app.clone().oneshot(arrange(false)).await.unwrap();
    let body = json_body(resp).await;
    assert!(
        body["placements"]
            .as_array()
            .unwrap()
            .iter()
            .all(|p| p.get("print_order").is_none()),
        "{body}"
    );
}

#[tokio::test]
async fn arrange_rejects_invalid_bed() {
    let (_d, app, _state) = app().await;
    let user = register(&app, "boss@test.local").await;
    let project = create_project(&app, &user).await;

    let resp = app
        .clone()
        .oneshot(post(
            &format!("/api/projects/{project}/arrange"),
            &user,
            serde_json::json!({ "bed_width": 0.0, "bed_depth": 100.0, "spacing": 5.0, "items": [] }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn arrange_preserves_scene_transforms_and_plates() {
    if !worker_available() {
        eprintln!("engine-worker absent — test de scène réel ignoré");
        return;
    }
    let (_d, app, _state) = app().await;
    let user = register(&app, "scene@test.local").await;
    let project = create_project(&app, &user).await;
    let cube = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../engine/tests/fixtures/cube20.stl"
    ))
    .unwrap();
    let model = upload_model(&app, &project, &user, "cube.stl", &cube).await;
    wait_for_mesh(&app, &model, &user).await;

    let transform = |scale| {
        serde_json::json!({
            "position": [10.0, 10.0, 0.0], "rotation": [0.0, 0.0, 15.0],
            "scale": scale
        })
    };
    let scene = serde_json::json!({
        "schemaVersion": 1,
        "objects": [
            { "id": "a", "modelId": model, "transform": transform([1.5, 1.0, 1.0]) },
            { "id": "b", "modelId": model, "transform": transform([1.0, 1.0, 1.0]) }
        ],
        "plates": { "plates": [
            { "objectIds": ["a"] }, { "objectIds": ["b"] }
        ], "activeId": null }
    });
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/projects/{project}"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &user)
                .body(Body::from(
                    serde_json::json!({
                        "expected_version": 1, "scene": scene, "active_presets": {}
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let resp = app
        .clone()
        .oneshot(post(
            &format!("/api/projects/{project}/arrange"),
            &user,
            serde_json::json!({
                "bed_width": 100.0, "bed_depth": 100.0, "spacing": 2.0,
                "items": [
                    { "id": "a", "width": 30.0, "depth": 20.0 },
                    { "id": "b", "width": 20.0, "depth": 20.0 }
                ]
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    let placements = body["placements"].as_array().unwrap();
    assert_eq!(placements[0]["plate_index"], 0);
    assert_eq!(placements[1]["plate_index"], 1);
    let matrix = placements[0]["matrix"].as_array().unwrap();
    let scale_x = (matrix[0].as_f64().unwrap().powi(2)
        + matrix[1].as_f64().unwrap().powi(2)
        + matrix[2].as_f64().unwrap().powi(2))
    .sqrt();
    assert!((scale_x - 1.5).abs() < 0.06, "scale conservé : {matrix:?}");
}

#[tokio::test]
async fn arrange_passes_exclusions_to_real_nesting() {
    if !worker_available() {
        eprintln!("engine-worker absent — test d'exclusion réel ignoré");
        return;
    }
    let (_d, app, _state) = app().await;
    let user = register(&app, "excluded@test.local").await;
    let project = create_project(&app, &user).await;
    let resp = app
        .clone()
        .oneshot(post(
            &format!("/api/projects/{project}/arrange"),
            &user,
            serde_json::json!({
                "bed_width": 100.0, "bed_depth": 100.0, "spacing": 2.0,
                "excluded": [[[40.0, 40.0], [60.0, 40.0], [60.0, 60.0], [40.0, 60.0]]],
                "items": [
                    { "id": "a", "width": 20.0, "depth": 20.0 },
                    { "id": "b", "width": 20.0, "depth": 20.0 },
                    { "id": "c", "width": 20.0, "depth": 20.0 },
                    { "id": "d", "width": 20.0, "depth": 20.0 }
                ]
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    for placement in body["placements"].as_array().unwrap() {
        let x = placement["x"].as_f64().unwrap();
        let y = placement["y"].as_f64().unwrap();
        let overlap_x = (x + 10.0).min(60.0) - (x - 10.0).max(40.0);
        let overlap_y = (y + 10.0).min(60.0) - (y - 10.0).max(40.0);
        assert!(
            overlap_x <= 1e-3 || overlap_y <= 1e-3,
            "objet dans la zone exclue : {placement}"
        );
    }
}

/// SC-008 : arranger dans le projet d'autrui répond 404.
#[tokio::test]
async fn arrange_in_others_project_is_404() {
    let (_d, app, _state) = app().await;
    let alice = register(&app, "alice@test.local").await;
    let bob = register(&app, "bob@test.local").await;
    let project = create_project(&app, &alice).await;

    let resp = app
        .clone()
        .oneshot(post(
            &format!("/api/projects/{project}/arrange"),
            &bob,
            serde_json::json!({ "bed_width": 100.0, "bed_depth": 100.0, "spacing": 5.0, "items": [] }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn arrange_requires_a_session() {
    let (_d, app, _state) = app().await;
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/projects/00000000-0000-0000-0000-000000000000/arrange")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::json!({ "bed_width": 1.0, "bed_depth": 1.0, "spacing": 0.0, "items": [] })
                        .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn repair_fixes_broken_cube_and_persists_result() {
    if !worker_available() {
        eprintln!("engine-worker absent — test de réparation réel ignoré");
        return;
    }
    let (_d, app, state) = app().await;
    let user = register(&app, "boss@test.local").await;
    let owner = user_id(&app, &user).await;
    let project = create_project(&app, &user).await;
    let mut stl = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../engine/tests/fixtures/cube20.stl"
    ))
    .unwrap();
    stl[80..84].copy_from_slice(&11u32.to_le_bytes());
    stl.truncate(stl.len() - 50);
    let model = upload_model(&app, &project, &user, "broken.stl", &stl).await;
    wait_for_mesh(&app, &model, &user).await;

    let resp = app
        .clone()
        .oneshot(post(
            &format!("/api/models/{model}/repair"),
            &user,
            serde_json::json!({}),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body["triangles"], 12);
    assert_eq!(body["open_edges"], 0);
    assert_eq!(body["watertight"], true);
    assert!(body["repairs"].is_object());

    let persisted = state
        .storage
        .models()
        .get(
            owner,
            backend::domain::ModelId(uuid::Uuid::parse_str(&model).unwrap()),
        )
        .await
        .unwrap();
    assert_eq!(persisted.repair_report, Some(body.clone()));
    assert_eq!(persisted.triangle_count, 12);
    let mesh_path = persisted.mesh_path.expect("maillage réparé persisté");
    let wsmh = state
        .files
        .read(std::path::Path::new(&mesh_path))
        .await
        .unwrap();
    assert_eq!(&wsmh[0..4], b"WSMh");
}

/// SC-008 : réparer le modèle d'autrui répond 404.
#[tokio::test]
async fn repair_of_others_model_is_404() {
    let (_d, app, _state) = app().await;
    let alice = register(&app, "alice@test.local").await;
    let bob = register(&app, "bob@test.local").await;
    let project = create_project(&app, &alice).await;
    let stl = binary_stl_tri([[0.0, 0.0, 0.0], [10.0, 0.0, 0.0], [0.0, 10.0, 0.0]]);
    let model = upload_model(&app, &project, &alice, "tri.stl", &stl).await;

    let resp = app
        .clone()
        .oneshot(post(
            &format!("/api/models/{model}/repair"),
            &bob,
            serde_json::json!({}),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

/// STL binaire d'un cube de côté `size` mm dont chaque face est subdivisée en
/// `n`×`n` carrés (2 triangles chacun) : `12·n²` triangles, normales sortantes.
fn subdivided_cube_stl(size: f32, n: u32) -> Vec<u8> {
    // (normale, triangle) : des normales réelles, comme tout exporteur STL.
    // admesh (Orca) ne reconnaît un STL binaire que si un octet > 127 figure
    // dans les 128 octets suivant l'en-tête ; des normales nulles et de petites
    // coordonnées le feraient lire comme ASCII (refus identique dans Orca).
    let mut tris: Vec<([f32; 3], [[f32; 3]; 3])> = Vec::new();
    let step = size / n as f32;
    // (axe normal, signe) pour les 6 faces ; (u, v) = deux autres axes.
    for (axis, positive) in [
        (0, false),
        (0, true),
        (1, false),
        (1, true),
        (2, false),
        (2, true),
    ] {
        let (u, v) = ((axis + 1) % 3, (axis + 2) % 3);
        let fixed = if positive { size } else { 0.0 };
        let mut normal = [0.0f32; 3];
        normal[axis] = if positive { 1.0 } else { -1.0 };
        let point = |i: u32, j: u32| {
            let mut p = [0.0f32; 3];
            p[axis] = fixed;
            p[u] = i as f32 * step;
            p[v] = j as f32 * step;
            p
        };
        for i in 0..n {
            for j in 0..n {
                let (a, b, c, d) = (
                    point(i, j),
                    point(i + 1, j),
                    point(i + 1, j + 1),
                    point(i, j + 1),
                );
                // u × v = +axe : ordre direct pour la face positive.
                if positive {
                    tris.push((normal, [a, b, c]));
                    tris.push((normal, [a, c, d]));
                } else {
                    tris.push((normal, [a, c, b]));
                    tris.push((normal, [a, d, c]));
                }
            }
        }
    }
    let mut out = vec![0u8; 80];
    out.extend_from_slice(&(tris.len() as u32).to_le_bytes());
    for (normal, tri) in &tris {
        for c in normal {
            out.extend_from_slice(&c.to_le_bytes());
        }
        for vertex in tri {
            for c in vertex {
                out.extend_from_slice(&c.to_le_bytes());
            }
        }
        out.extend_from_slice(&[0u8; 2]);
    }
    out
}

async fn persisted_model(
    state: &AppState,
    owner: backend::domain::UserId,
    model: &str,
) -> backend::domain::Model {
    state
        .storage
        .models()
        .get(
            owner,
            backend::domain::ModelId(uuid::Uuid::parse_str(model).unwrap()),
        )
        .await
        .unwrap()
}

/// Simplification réelle (libslic3r `its_quadric_edge_collapse`) d'un cube
/// subdivisé de 768 triangles : le nombre de triangles baisse, le WSMh
/// persisté et servi reflète le maillage simplifié, la géométrie est conservée.
#[tokio::test]
async fn simplify_ratio_reduces_subdivided_cube_and_persists_mesh() {
    if !worker_available() {
        eprintln!("engine-worker absent — test de simplification réel ignoré");
        return;
    }
    let (_d, app, state) = app().await;
    let user = register(&app, "boss@test.local").await;
    let owner = user_id(&app, &user).await;
    let project = create_project(&app, &user).await;
    let stl = subdivided_cube_stl(20.0, 8);
    let model = upload_model(&app, &project, &user, "cube8.stl", &stl).await;
    wait_for_mesh(&app, &model, &user).await;
    assert_eq!(
        persisted_model(&state, owner, &model).await.triangle_count,
        768
    );

    let resp = app
        .clone()
        .oneshot(post(
            &format!("/api/models/{model}/simplify"),
            &user,
            serde_json::json!({ "mode": "ratio", "decimate_ratio": 50.0 }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body["triangles_before"], 768);
    let after = body["triangles_after"].as_u64().unwrap();
    assert!(after > 0 && after <= 384, "réduction attendue : {body}");
    assert!(body["vertices_after"].as_u64().unwrap() < body["vertices_before"].as_u64().unwrap());
    // Les faces planes se décimment sans perte : le cube reste fermé.
    assert_eq!(body["watertight"], true, "{body}");

    let persisted = persisted_model(&state, owner, &model).await;
    assert_eq!(persisted.triangle_count as u64, after);
    let mesh_path = persisted.mesh_path.expect("maillage simplifié persisté");
    let wsmh = state
        .files
        .read(std::path::Path::new(&mesh_path))
        .await
        .unwrap();
    let display = engine::api::DisplayMesh::decode(&wsmh).expect("WSMh valide");
    assert_eq!(display.indices.len() as u64, after * 3);
    // Boîte englobante conservée (sommets des coins jamais déplacés).
    let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
    for p in display.positions.chunks_exact(3) {
        for k in 0..3 {
            lo[k] = lo[k].min(p[k]);
            hi[k] = hi[k].max(p[k]);
        }
    }
    for k in 0..3 {
        assert!(
            (hi[k] - lo[k] - 20.0).abs() < 1e-3,
            "étendue {k} : {lo:?} {hi:?}"
        );
    }

    // Le maillage servi au client est bien le maillage simplifié.
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/models/{model}/mesh"))
                .header(header::COOKIE, &user)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let served = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    assert_eq!(served.as_ref(), wsmh.as_slice());
}

/// Mode « niveau de détail » d'Orca : l'erreur quadrique bornée suffit à
/// fusionner les faces coplanaires du cube subdivisé.
#[tokio::test]
async fn simplify_detail_level_reduces_subdivided_cube() {
    if !worker_available() {
        eprintln!("engine-worker absent — test de simplification réel ignoré");
        return;
    }
    let (_d, app, _state) = app().await;
    let user = register(&app, "boss@test.local").await;
    let project = create_project(&app, &user).await;
    let stl = subdivided_cube_stl(20.0, 6);
    let model = upload_model(&app, &project, &user, "cube6.stl", &stl).await;
    wait_for_mesh(&app, &model, &user).await;

    let resp = app
        .clone()
        .oneshot(post(
            &format!("/api/models/{model}/simplify"),
            &user,
            serde_json::json!({ "mode": "detail", "level": 2 }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body["triangles_before"], 432);
    let after = body["triangles_after"].as_u64().unwrap();
    assert!((12..432).contains(&after), "réduction attendue : {body}");
    assert!(body["max_error"].as_f64().unwrap() <= 0.1 + 1e-6);
}

/// Paramètres invalides : 422 sans lancer le worker ni toucher au maillage.
#[tokio::test]
async fn simplify_rejects_invalid_parameters() {
    let (_d, app, _state) = app().await;
    let user = register(&app, "boss@test.local").await;
    let project = create_project(&app, &user).await;
    let stl = binary_stl_tri([[0.0, 0.0, 0.0], [10.0, 0.0, 0.0], [0.0, 10.0, 0.0]]);
    let model = upload_model(&app, &project, &user, "tri.stl", &stl).await;

    for body in [
        serde_json::json!({ "mode": "ratio", "decimate_ratio": 0.0 }),
        serde_json::json!({ "mode": "ratio", "decimate_ratio": 100.0 }),
        serde_json::json!({ "mode": "ratio", "decimate_ratio": -5.0 }),
        serde_json::json!({ "mode": "detail", "level": 5 }),
        serde_json::json!({ "mode": "grid", "cell": 1.0 }),
        serde_json::json!({ "decimate_ratio": 50.0 }),
    ] {
        let resp = app
            .clone()
            .oneshot(post(
                &format!("/api/models/{model}/simplify"),
                &user,
                body.clone(),
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    }
}

/// Un maillage déjà minimal (1 triangle) ne peut pas être simplifié : 422.
#[tokio::test]
async fn simplify_of_too_small_mesh_is_422() {
    if !worker_available() {
        eprintln!("engine-worker absent — conversion réelle requise");
        return;
    }
    let (_d, app, _state) = app().await;
    let user = register(&app, "boss@test.local").await;
    let project = create_project(&app, &user).await;
    // Cube minimal (12 triangles, lisible par admesh) : 90 % de réduction vise
    // 1 triangle, sous le minimum de 4 → refus explicite avant le worker.
    let stl = subdivided_cube_stl(20.0, 1);
    let model = upload_model(&app, &project, &user, "cube1.stl", &stl).await;
    wait_for_mesh(&app, &model, &user).await;

    let resp = app
        .clone()
        .oneshot(post(
            &format!("/api/models/{model}/simplify"),
            &user,
            serde_json::json!({ "mode": "ratio", "decimate_ratio": 90.0 }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

/// SC-008 : simplifier le modèle d'autrui répond 404.
#[tokio::test]
async fn simplify_of_others_model_is_404() {
    let (_d, app, _state) = app().await;
    let alice = register(&app, "alice@test.local").await;
    let bob = register(&app, "bob@test.local").await;
    let project = create_project(&app, &alice).await;
    let stl = binary_stl_tri([[0.0, 0.0, 0.0], [10.0, 0.0, 0.0], [0.0, 10.0, 0.0]]);
    let model = upload_model(&app, &project, &alice, "tri.stl", &stl).await;

    let resp = app
        .clone()
        .oneshot(post(
            &format!("/api/models/{model}/simplify"),
            &bob,
            serde_json::json!({ "mode": "ratio", "decimate_ratio": 50.0 }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn orient_suggests_rotation_for_stl() {
    let (_d, app, _state) = app().await;
    let user = register(&app, "boss@test.local").await;
    let project = create_project(&app, &user).await;
    // Facette dans le plan XY (normale +Z) → orientation la ramène vers le bas.
    let stl = binary_stl_tri([[0.0, 0.0, 5.0], [10.0, 0.0, 5.0], [0.0, 10.0, 5.0]]);
    let model = upload_model(&app, &project, &user, "tri.stl", &stl).await;

    let resp = app
        .clone()
        .oneshot(post(
            &format!("/api/projects/{project}/orient"),
            &user,
            serde_json::json!({ "model_id": model }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    let rotation = body["rotation"].as_array().unwrap();
    assert_eq!(rotation.len(), 3);
}
