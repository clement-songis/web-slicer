//! Découpe de maillage de bout en bout (worker réel `mesh-op`, split.cpp) :
//! split en objets (`TriangleMesh::split`), coupe plane (`cut_mesh`) et
//! booléen mcut, sur de vrais STL binaires à normales réelles.

use axum::body::{to_bytes, Body};
use axum::http::{header, Request, StatusCode};
use axum::response::Response;
use axum::Router;
use backend::adapters::files::FileStore;
use backend::adapters::storage::AnyStorage;
use backend::http::routes::router;
use backend::http::state::AppState;
use std::sync::Arc;
use tower::ServiceExt;
use tower_sessions::{MemoryStore, SessionManagerLayer};

async fn app() -> (tempfile::TempDir, Router, AppState) {
    let _ = tracing_subscriber::fmt().with_test_writer().try_init();
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}", dir.path().join("scene.db").display());
    let storage = Arc::new(AnyStorage::connect_test(&url).await.unwrap());
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

/// STL binaire (normales réelles, en-tête ≠ « solid ») de cubes alignés.
fn cubes_stl(cubes: &[([f32; 3], f32)]) -> Vec<u8> {
    let mut facets: Vec<[[f32; 3]; 3]> = Vec::new();
    for &([x, y, z], s) in cubes {
        let p = |i: usize| -> [f32; 3] {
            [
                x + if i & 1 != 0 { s } else { 0.0 },
                y + if i & 2 != 0 { s } else { 0.0 },
                z + if i & 4 != 0 { s } else { 0.0 },
            ]
        };
        // Faces orientées vers l'extérieur (sommets indexés en binaire xyz).
        for [a, b, c, d] in [
            [0, 2, 3, 1],
            [4, 5, 7, 6],
            [0, 1, 5, 4],
            [2, 6, 7, 3],
            [0, 4, 6, 2],
            [1, 3, 7, 5],
        ] {
            facets.push([p(a), p(b), p(c)]);
            facets.push([p(a), p(c), p(d)]);
        }
    }
    let mut out = vec![b' '; 80];
    out[..9].copy_from_slice(b"test-cube");
    out.extend_from_slice(&(facets.len() as u32).to_le_bytes());
    for [a, b, c] in facets {
        let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let w = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let n = [
            u[1] * w[2] - u[2] * w[1],
            u[2] * w[0] - u[0] * w[2],
            u[0] * w[1] - u[1] * w[0],
        ];
        let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
        for v in [n[0] / len, n[1] / len, n[2] / len]
            .iter()
            .chain(&a)
            .chain(&b)
            .chain(&c)
        {
            out.extend_from_slice(&v.to_le_bytes());
        }
        out.extend_from_slice(&0u16.to_le_bytes());
    }
    out
}

struct Ctx {
    _dir: tempfile::TempDir,
    app: Router,
    state: AppState,
    user: String,
    project: String,
    model: String,
}

async fn setup(stl: &[u8]) -> Option<Ctx> {
    if !worker_available() {
        eprintln!("engine-worker absent — test de découpe réel ignoré");
        return None;
    }
    let (dir, app, state) = app().await;
    let user = register(&app, "cutter@test.local").await;
    let project = create_project(&app, &user).await;
    let model = upload_model(&app, &project, &user, "pieces.stl", stl).await;
    wait_for_mesh(&app, &model, &user).await;
    Some(Ctx {
        _dir: dir,
        app,
        state,
        user,
        project,
        model,
    })
}

fn approx(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}

async fn send(app: &Router, req: Request<Body>) -> (StatusCode, serde_json::Value) {
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    (status, json_body(resp).await)
}

fn get(uri: &str, cookie: &str) -> Request<Body> {
    Request::builder()
        .uri(uri)
        .header(header::COOKIE, cookie)
        .body(Body::empty())
        .unwrap()
}

#[tokio::test]
async fn split_two_disjoint_cubes_creates_two_ready_models() {
    let stl = cubes_stl(&[([0.0, 0.0, 0.0], 10.0), ([20.0, 0.0, 0.0], 10.0)]);
    let Some(ctx) = setup(&stl).await else {
        return;
    };
    let (status, body) = send(
        &ctx.app,
        post(
            &format!("/api/models/{}/split", ctx.model),
            &ctx.user,
            serde_json::json!({}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    let parts = body["parts"].as_array().unwrap();
    assert_eq!(parts.len(), 2, "{body}");
    let uid = user_id(&ctx.app, &ctx.user).await;
    let mut centers_x: Vec<f64> = Vec::new();
    for part in parts {
        assert_eq!(part["triangles"], 12, "{part}");
        assert!(
            approx(part["volume_mm3"].as_f64().unwrap(), 1000.0, 1e-3),
            "{part}"
        );
        assert_eq!(part["watertight"], true);
        assert_eq!(part["model"]["project_id"], ctx.project.as_str());
        assert_eq!(part["model"]["has_mesh"], true);
        assert!(part["model"]["filename"]
            .as_str()
            .unwrap()
            .starts_with("pieces_"));
        // Repère recentré de la source (centre x = 15) : pièces à ±10.
        centers_x.push(part["center"][0].as_f64().unwrap());
        assert!(approx(part["center"][1].as_f64().unwrap(), 0.0, 1e-4));
        assert!(approx(part["center"][2].as_f64().unwrap(), 0.0, 1e-4));

        let id = part["model"]["id"].as_str().unwrap();
        let resp = ctx
            .app
            .clone()
            .oneshot(get(&format!("/api/models/{id}/mesh"), &ctx.user))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        // Fichier source : STL binaire réel (normales unitaires).
        let stored = ctx
            .state
            .storage
            .models()
            .get(
                uid,
                backend::domain::ModelId(uuid::Uuid::parse_str(id).unwrap()),
            )
            .await
            .unwrap();
        let bytes = std::fs::read(&stored.file_path).unwrap();
        assert!(!bytes.starts_with(b"solid"));
        assert_eq!(u32::from_le_bytes(bytes[80..84].try_into().unwrap()), 12);
        let n: Vec<f32> = (0..3)
            .map(|i| f32::from_le_bytes(bytes[84 + i * 4..88 + i * 4].try_into().unwrap()))
            .collect();
        assert!(approx(
            f64::from(n[0] * n[0] + n[1] * n[1] + n[2] * n[2]),
            1.0,
            1e-4
        ));
    }
    centers_x.sort_by(f64::total_cmp);
    assert!(
        approx(centers_x[0], -10.0, 1e-4) && approx(centers_x[1], 10.0, 1e-4),
        "{centers_x:?}"
    );

    // Les pièces sont des modèles du projet ; la source est conservée.
    let (status, list) = send(
        &ctx.app,
        get(&format!("/api/projects/{}/models", ctx.project), &ctx.user),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list.as_array().unwrap().len(), 3);
}

#[tokio::test]
async fn split_single_part_is_refused() {
    let Some(ctx) = setup(&cubes_stl(&[([0.0, 0.0, 0.0], 10.0)])).await else {
        return;
    };
    let (status, _) = send(
        &ctx.app,
        post(
            &format!("/api/models/{}/split", ctx.model),
            &ctx.user,
            serde_json::json!({}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

/// « Subdivision mesh (Lost color) » : nouveau modèle ×4 triangles, fermé.
#[tokio::test]
async fn smooth_subdivides_cube_into_closed_mesh() {
    let Some(ctx) = setup(&cubes_stl(&[([0.0, 0.0, 0.0], 10.0)])).await else {
        return;
    };
    let (status, body) = send(
        &ctx.app,
        post(
            &format!("/api/models/{}/smooth", ctx.model),
            &ctx.user,
            serde_json::json!({}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    let parts = body["parts"].as_array().unwrap();
    assert_eq!(parts.len(), 1, "{body}");
    let part = &parts[0];
    assert_eq!(part["label"], "result");
    // Le STL du cube (12 facettes, sommets fusionnés par admesh) → 48.
    assert_eq!(part["triangles"], 48, "{body}");
    assert_eq!(part["watertight"], true, "{body}");
    let volume = part["volume_mm3"].as_f64().unwrap();
    assert!(volume > 0.0 && volume < 1000.0, "volume lissé {volume}");
    // Le nouveau modèle est servi immédiatement par /mesh.
    let new_model = part["model"]["id"].as_str().unwrap();
    let resp = ctx
        .app
        .clone()
        .oneshot(get(&format!("/api/models/{new_model}/mesh"), &ctx.user))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn horizontal_cut_produces_two_closed_halves() {
    let Some(ctx) = setup(&cubes_stl(&[([0.0, 0.0, 0.0], 20.0)])).await else {
        return;
    };
    let uri = format!("/api/models/{}/cut", ctx.model);
    let (status, body) = send(
        &ctx.app,
        post(
            &uri,
            &ctx.user,
            serde_json::json!({ "plane": { "point": [0, 0, 2.5], "normal": [0, 0, 1] } }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    let parts = body["parts"].as_array().unwrap();
    assert_eq!(parts.len(), 2, "{body}");
    let upper = parts.iter().find(|p| p["label"] == "upper").unwrap();
    let lower = parts.iter().find(|p| p["label"] == "lower").unwrap();
    // Cube 20 mm recentré : z ∈ [-10, 10], coupe à 2.5 → 7.5 / 12.5 mm.
    assert!(
        approx(upper["volume_mm3"].as_f64().unwrap(), 3000.0, 0.5),
        "{upper}"
    );
    assert!(
        approx(lower["volume_mm3"].as_f64().unwrap(), 5000.0, 0.5),
        "{lower}"
    );
    assert_eq!(upper["watertight"], true, "section bouchée : {upper}");
    assert_eq!(lower["watertight"], true, "section bouchée : {lower}");
    assert!(
        approx(upper["center"][2].as_f64().unwrap(), 6.25, 1e-3),
        "{upper}"
    );
    assert!(
        approx(lower["center"][2].as_f64().unwrap(), -3.75, 1e-3),
        "{lower}"
    );
    assert!(upper["model"]["filename"]
        .as_str()
        .unwrap()
        .ends_with("_A.stl"));
    assert!(lower["model"]["filename"]
        .as_str()
        .unwrap()
        .ends_with("_B.stl"));

    // Moitié unique demandée.
    let (status, body) = send(
        &ctx.app,
        post(
            &uri,
            &ctx.user,
            serde_json::json!({
                "plane": { "point": [0, 0, 0], "normal": [0, 0, 1] },
                "keep_lower": false
            }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    assert_eq!(body["parts"].as_array().unwrap().len(), 1);
    assert_eq!(body["parts"][0]["label"], "upper");
    assert!(approx(
        body["parts"][0]["volume_mm3"].as_f64().unwrap(),
        4000.0,
        0.5
    ));
}

#[tokio::test]
async fn tilted_cut_keeps_total_volume() {
    let Some(ctx) = setup(&cubes_stl(&[([0.0, 0.0, 0.0], 20.0)])).await else {
        return;
    };
    let (status, body) = send(
        &ctx.app,
        post(
            &format!("/api/models/{}/cut", ctx.model),
            &ctx.user,
            serde_json::json!({ "plane": { "point": [0, 0, 0], "normal": [1, 0, 1] } }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    let parts = body["parts"].as_array().unwrap();
    assert_eq!(parts.len(), 2);
    for part in parts {
        // Plan diagonal passant par le centre : deux moitiés symétriques fermées.
        assert_eq!(part["watertight"], true, "{part}");
        assert!(
            approx(part["volume_mm3"].as_f64().unwrap(), 4000.0, 1.0),
            "{part}"
        );
    }
}

#[tokio::test]
async fn invalid_cuts_are_refused() {
    let Some(ctx) = setup(&cubes_stl(&[([0.0, 0.0, 0.0], 20.0)])).await else {
        return;
    };
    for body in [
        serde_json::json!({ "plane": { "point": [0, 0, 50], "normal": [0, 0, 1] } }),
        serde_json::json!({ "plane": { "point": [0, 0, 0], "normal": [0, 0, 0] } }),
        serde_json::json!({
            "plane": { "point": [0, 0, 0], "normal": [0, 0, 1] },
            "keep_upper": false, "keep_lower": false
        }),
    ] {
        let (status, _) = send(
            &ctx.app,
            post(
                &format!("/api/models/{}/cut", ctx.model),
                &ctx.user,
                body.clone(),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    }
}

fn translation(x: f64) -> Vec<f64> {
    translation_xyz(x, 0.0, 0.0)
}

fn translation_xyz(x: f64, y: f64, z: f64) -> Vec<f64> {
    vec![1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., x, y, z, 1.]
}

#[tokio::test]
async fn boolean_operations_match_expected_volumes() {
    let stl = cubes_stl(&[([0.0, 0.0, 0.0], 20.0)]);
    let Some(ctx) = setup(&stl).await else {
        return;
    };
    let tool = upload_model(&ctx.app, &ctx.project, &ctx.user, "tool.stl", &stl).await;
    wait_for_mesh(&ctx.app, &tool, &ctx.user).await;
    // A centré en 0, B décalé de (10, 5, 5) : aucune face coplanaire (cas
    // dégénéré de mcut). Recouvrement 10×15×15 = 2250 mm³.
    for (op, volume, center) in [
        ("union", 13750.0, [5.0, 2.5, 2.5]),
        ("difference", 5750.0, [0.0, 0.0, 0.0]),
        ("intersection", 2250.0, [5.0, 2.5, 2.5]),
    ] {
        let (status, body) = send(
            &ctx.app,
            post(
                &format!("/api/projects/{}/boolean", ctx.project),
                &ctx.user,
                serde_json::json!({
                    "op": op,
                    "a": { "model_id": ctx.model, "matrix": translation(0.0) },
                    "b": { "model_id": tool, "matrix": translation_xyz(10.0, 5.0, 5.0) }
                }),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "{op}: {body}");
        assert_eq!(body["parts"].as_array().unwrap().len(), 1);
        let part = &body["parts"][0];
        assert_eq!(part["label"], "result");
        assert!(
            approx(part["volume_mm3"].as_f64().unwrap(), volume, 1.0),
            "{op}: {part}"
        );
        for axis in 0..3 {
            assert!(
                approx(part["center"][axis].as_f64().unwrap(), center[axis], 1e-2),
                "{op}: {part}"
            );
        }
        assert_eq!(part["watertight"], true, "{op}: {part}");
    }
}

#[tokio::test]
async fn mesh_ops_are_isolated_between_accounts() {
    let stl = cubes_stl(&[([0.0, 0.0, 0.0], 10.0), ([20.0, 0.0, 0.0], 10.0)]);
    let Some(ctx) = setup(&stl).await else {
        return;
    };
    let intruder = register(&ctx.app, "intruder@test.local").await;
    let (status, _) = send(
        &ctx.app,
        post(
            &format!("/api/models/{}/split", ctx.model),
            &intruder,
            serde_json::json!({}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = send(
        &ctx.app,
        post(
            &format!("/api/projects/{}/boolean", ctx.project),
            &intruder,
            serde_json::json!({
                "op": "union",
                "a": { "model_id": ctx.model, "matrix": translation(0.0) },
                "b": { "model_id": ctx.model, "matrix": translation(5.0) }
            }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

/// « Garder la peinture » (keep_painting d'Orca) : coupe avec KeepPaint —
/// peinture reportée sur chaque pièce, section neuve non peinte ; puis
/// remappage après réparation (POST …/remap-paint), 409 sans maillage
/// précédent, 422 canal inconnu.
#[tokio::test]
async fn keep_painting_follows_cut_and_repair() {
    let Some(ctx) = setup(&cubes_stl(&[([0.0, 0.0, 0.0], 20.0)])).await else {
        return;
    };
    let all: serde_json::Map<String, serde_json::Value> = (0..12)
        .map(|t| (t.to_string(), serde_json::json!("4")))
        .collect();
    let paint = serde_json::json!({ "supports": all });

    // Remappage sans maillage précédent : 409.
    let remap_uri = format!("/api/models/{}/remap-paint", ctx.model);
    let (status, body) = send(
        &ctx.app,
        post(
            &remap_uri,
            &ctx.user,
            serde_json::json!({ "channels": paint.clone() }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");

    // Coupe avec peinture : chaque pièce porte une peinture partielle.
    let uri = format!("/api/models/{}/cut", ctx.model);
    let (status, body) = send(
        &ctx.app,
        post(
            &uri,
            &ctx.user,
            serde_json::json!({
                "plane": { "point": [0, 0, 2.5], "normal": [0, 0, 1] },
                "keep_painting": paint.clone()
            }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    for part in body["parts"].as_array().unwrap() {
        let painted = part["paint"]["supports"]
            .as_object()
            .expect("peinture reportée");
        let triangles = part["triangles"].as_u64().unwrap() as usize;
        assert!(!painted.is_empty(), "{part}");
        assert!(
            painted.len() < triangles,
            "section de coupe non peinte : {part}"
        );
    }
    // Sans keep_painting : pas de champ paint.
    let (_, body) = send(
        &ctx.app,
        post(
            &uri,
            &ctx.user,
            serde_json::json!({ "plane": { "point": [0, 0, 2.5], "normal": [0, 0, 1] } }),
        ),
    )
    .await;
    assert!(body["parts"][0].get("paint").is_none(), "{body}");
    // Canal inconnu : 422.
    let (status, _) = send(
        &ctx.app,
        post(
            &uri,
            &ctx.user,
            serde_json::json!({ "plane": { "point": [0, 0, 2.5], "normal": [0, 0, 1] },
                                "keep_painting": { "colors": {} } }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    // Réparation (garde le maillage précédent) puis remappage : même cube,
    // peinture intégralement conservée.
    let (status, body) = send(
        &ctx.app,
        post(
            &format!("/api/models/{}/repair", ctx.model),
            &ctx.user,
            serde_json::json!({}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (status, body) = send(
        &ctx.app,
        post(
            &remap_uri,
            &ctx.user,
            serde_json::json!({ "channels": paint }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        body["channels"]["supports"].as_object().unwrap().len(),
        12,
        "{body}"
    );
}
