//! Lot 37 — arrangement des objets tournés et mis à l'échelle (matrice
//! d'instance transmise à l'ArrangeJob d'Orca), informations de coupe
//! (cut_id, « Invalidate cut info », cut_information.xml) et erreurs de
//! maillage par objet (GUI_ObjectList::get_mesh_errors_info).

use axum::body::{to_bytes, Body};
use axum::http::{header, Request, StatusCode};
use axum::response::Response;
use axum::Router;
use backend::adapters::files::FileStore;
use backend::adapters::storage::AnyStorage;
use backend::http::routes::router;
use backend::http::state::AppState;
use glam::{DMat4, DQuat, DVec2, DVec3, EulerRot};
use serde_json::{json, Value};
use std::io::Read;
use std::sync::Arc;
use tower::ServiceExt;
use tower_sessions::{MemoryStore, SessionManagerLayer};

async fn app() -> (tempfile::TempDir, Router) {
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}", dir.path().join("l37.db").display());
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

fn worker_available() -> bool {
    backend::engine::worker_binary().exists()
}

fn request(method: &str, uri: &str, body: Option<Value>, cookie: &str) -> Request<Body> {
    let builder = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::COOKIE, cookie);
    match body {
        Some(body) => builder
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
        None => builder.body(Body::empty()).unwrap(),
    }
}

async fn send(app: &Router, req: Request<Body>) -> Response {
    app.clone().oneshot(req).await.unwrap()
}

async fn json_of(resp: Response) -> (StatusCode, Value) {
    let status = resp.status();
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

async fn call(
    app: &Router,
    method: &str,
    uri: &str,
    body: Option<Value>,
    c: &str,
) -> (StatusCode, Value) {
    json_of(send(app, request(method, uri, body, c)).await).await
}

async fn register(app: &Router, email: &str) -> String {
    let resp = send(
        app,
        Request::builder()
            .method("POST")
            .uri("/api/auth/register")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                json!({ "email": email, "password": "correct horse battery" }).to_string(),
            ))
            .unwrap(),
    )
    .await;
    assert!(resp.status().is_success(), "{}", resp.status());
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

async fn create_project(app: &Router, cookie: &str, name: &str) -> String {
    let (status, body) = call(
        app,
        "POST",
        "/api/projects",
        Some(json!({ "name": name })),
        cookie,
    )
    .await;
    assert!(status.is_success(), "{status} {body}");
    body["id"].as_str().unwrap().to_string()
}

fn multipart(uri: &str, filename: &str, bytes: &[u8], cookie: &str) -> Request<Body> {
    let boundary = "----webslicer-lot37";
    let mut body = Vec::new();
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(
        format!("Content-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\n")
            .as_bytes(),
    );
    body.extend_from_slice(b"Content-Type: application/octet-stream\r\n\r\n");
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

async fn upload(app: &Router, project: &str, cookie: &str, name: &str, stl: &[u8]) -> String {
    let (status, body) = json_of(
        send(
            app,
            multipart(
                &format!("/api/projects/{project}/models"),
                name,
                stl,
                cookie,
            ),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    let id = body["id"].as_str().unwrap().to_string();
    wait_mesh(app, &id, cookie).await;
    id
}

async fn wait_mesh(app: &Router, model: &str, cookie: &str) {
    for _ in 0..600 {
        let status = send(
            app,
            request("GET", &format!("/api/models/{model}/mesh"), None, cookie),
        )
        .await
        .status();
        if status != StatusCode::CONFLICT {
            assert_eq!(status, StatusCode::OK, "conversion de {model}");
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    panic!("conversion de {model} non terminée");
}

async fn save_scene(app: &Router, project: &str, cookie: &str, scene: Value) {
    let (_, current) = call(
        app,
        "GET",
        &format!("/api/projects/{project}"),
        None,
        cookie,
    )
    .await;
    let (status, body) = call(
        app,
        "PUT",
        &format!("/api/projects/{project}"),
        Some(json!({
            "expected_version": current["version"], "scene": scene, "active_presets": {}
        })),
        cookie,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
}

/// Pavé `size` centré à l'origine, en triangles (faces extérieures).
fn box_triangles(size: [f32; 3]) -> Vec<[[f32; 3]; 3]> {
    let [x, y, z] = size.map(|s| s / 2.0);
    let v = [
        [-x, -y, -z],
        [x, -y, -z],
        [x, y, -z],
        [-x, y, -z],
        [-x, -y, z],
        [x, -y, z],
        [x, y, z],
        [-x, y, z],
    ];
    [
        [0, 2, 1],
        [0, 3, 2],
        [4, 5, 6],
        [4, 6, 7],
        [0, 1, 5],
        [0, 5, 4],
        [1, 2, 6],
        [1, 6, 5],
        [2, 3, 7],
        [2, 7, 6],
        [3, 0, 4],
        [3, 4, 7],
    ]
    .iter()
    .map(|t: &[usize; 3]| t.map(|i| v[i]))
    .collect()
}

fn binary_stl(triangles: &[[[f32; 3]; 3]]) -> Vec<u8> {
    let mut out = vec![0u8; 80];
    out.extend_from_slice(&(triangles.len() as u32).to_le_bytes());
    for t in triangles {
        out.extend_from_slice(&[0u8; 12]);
        for p in t {
            for c in p {
                out.extend_from_slice(&c.to_le_bytes());
            }
        }
        out.extend_from_slice(&0u16.to_le_bytes());
    }
    out
}

// --- §1 Arrangement des objets tournés / mis à l'échelle ---------------------

const BAR: [f32; 3] = [40.0, 10.0, 6.0];

/// Matrice monde attendue (Three.js : T·R·S, quaternion Euler XYZ).
fn expected_matrix(transform: &Value) -> DMat4 {
    let v =
        |key: &str| -> [f64; 3] { std::array::from_fn(|i| transform[key][i].as_f64().unwrap()) };
    let [rx, ry, rz] = v("rotation").map(f64::to_radians);
    DMat4::from_scale_rotation_translation(
        DVec3::from_array(v("scale")),
        DQuat::from_euler(EulerRot::XYZ, rx, ry, rz),
        DVec3::from_array(v("position")),
    )
}

fn matrix_of(placement: &Value) -> DMat4 {
    let values: Vec<f64> = placement["matrix"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap())
        .collect();
    DMat4::from_cols_slice(&values)
}

/// Enveloppe convexe XY (chaîne monotone) des 8 coins du pavé transformés.
fn world_hull(m: &DMat4) -> Vec<DVec2> {
    let [x, y, z] = BAR.map(|s| f64::from(s) / 2.0);
    let mut pts: Vec<DVec2> = (0..8)
        .map(|i| {
            let p = DVec3::new(
                if i & 1 == 0 { -x } else { x },
                if i & 2 == 0 { -y } else { y },
                if i & 4 == 0 { -z } else { z },
            );
            m.transform_point3(p).truncate()
        })
        .collect();
    pts.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
    let cross = |o: DVec2, a: DVec2, b: DVec2| (a - o).perp_dot(b - o);
    let mut hull: Vec<DVec2> = Vec::new();
    for pass in 0..2 {
        let start = hull.len();
        let iter: Box<dyn Iterator<Item = &DVec2>> = if pass == 0 {
            Box::new(pts.iter())
        } else {
            Box::new(pts.iter().rev())
        };
        for &p in iter {
            while hull.len() >= start + 2
                && cross(hull[hull.len() - 2], hull[hull.len() - 1], p) <= 1e-9
            {
                hull.pop();
            }
            hull.push(p);
        }
        hull.pop();
    }
    hull
}

/// Distance entre deux polygones convexes (0 s'ils se chevauchent, test des
/// axes séparateurs).
fn hull_gap(a: &[DVec2], b: &[DVec2]) -> f64 {
    let edges = |p: &[DVec2]| -> Vec<(DVec2, DVec2)> {
        (0..p.len()).map(|i| (p[i], p[(i + 1) % p.len()])).collect()
    };
    let separated = [a, b].iter().any(|poly| {
        edges(poly).iter().any(|&(s, e)| {
            let n = (e - s).perp();
            let (amin, amax) = a
                .iter()
                .map(|p| p.dot(n))
                .fold((f64::MAX, f64::MIN), |(lo, hi), v| (lo.min(v), hi.max(v)));
            let (bmin, bmax) = b
                .iter()
                .map(|p| p.dot(n))
                .fold((f64::MAX, f64::MIN), |(lo, hi), v| (lo.min(v), hi.max(v)));
            amax <= bmin + 1e-9 || bmax <= amin + 1e-9
        })
    });
    if !separated {
        return 0.0;
    }
    let point_segment = |p: DVec2, s: DVec2, e: DVec2| {
        let d = e - s;
        let t = ((p - s).dot(d) / d.length_squared()).clamp(0.0, 1.0);
        (s + d * t - p).length()
    };
    let mut best = f64::MAX;
    for &(s, e) in &edges(b) {
        for &p in a {
            best = best.min(point_segment(p, s, e));
        }
    }
    for &(s, e) in &edges(a) {
        for &p in b {
            best = best.min(point_segment(p, s, e));
        }
    }
    best
}

/// Rotation pure autour de Z (ce qu'ajoute l'arrangement d'Orca :
/// ModelInstance::apply_arrange_result → rotation Z de l'instance).
fn is_z_rotation(r: &DMat4) -> bool {
    let c = |col: usize, row: usize| r.col(col)[row];
    (c(2, 2) - 1.0).abs() < 1e-6
        && c(2, 0).abs() < 1e-6
        && c(2, 1).abs() < 1e-6
        && c(0, 2).abs() < 1e-6
        && c(1, 2).abs() < 1e-6
        && (c(0, 0) - c(1, 1)).abs() < 1e-6
        && (c(0, 1) + c(1, 0)).abs() < 1e-6
        && (c(0, 0).powi(2) + c(0, 1).powi(2) - 1.0).abs() < 1e-6
}

async fn arrange_bars(options: Value) -> Option<(Vec<(Value, DMat4)>, f64)> {
    if !worker_available() {
        eprintln!("engine-worker absent — arrangement réel ignoré");
        return None;
    }
    let (_d, app) = app().await;
    let user = register(&app, "arrange37@test.local").await;
    let project = create_project(&app, &user, "arrangement lot 37").await;
    let model = upload(
        &app,
        &project,
        &user,
        "bar.stl",
        &binary_stl(&box_triangles(BAR)),
    )
    .await;
    let transforms = [
        json!({ "position": [60.0, 60.0, 5.0], "rotation": [0.0, 0.0, 30.0], "scale": [1.5, 1.0, 1.0] }),
        json!({ "position": [60.0, 60.0, 3.0], "rotation": [90.0, 0.0, -70.0], "scale": [1.0, 1.0, 2.0] }),
        json!({ "position": [60.0, 60.0, 3.0], "rotation": [0.0, 0.0, 115.0], "scale": [1.0, 2.0, 1.0] }),
    ];
    let objects: Vec<Value> = transforms
        .iter()
        .enumerate()
        .map(|(i, t)| json!({ "id": format!("o{i}"), "modelId": model, "transform": t }))
        .collect();
    save_scene(
        &app,
        &project,
        &user,
        json!({
            "schemaVersion": 1,
            "objects": objects,
            "plates": { "plates": [{ "id": "p", "objectIds": ["o0", "o1", "o2"] }], "activeId": "p" }
        }),
    )
    .await;
    let spacing = options["spacing"].as_f64().unwrap();
    let (status, body) = call(
        &app,
        "POST",
        &format!("/api/projects/{project}/arrange"),
        Some(json!({
            "bed_width": 180.0, "bed_depth": 180.0, "spacing": spacing, "items": [],
            "bed_shape": [[0, 0], [180, 0], [180, 180], [0, 180]], "excluded": [],
            "max_height": 200.0, "options": options, "scope": "all", "plate_index": 0
        })),
        &user,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let placements = body["placements"].as_array().unwrap();
    assert_eq!(placements.len(), 3, "{body}");
    let mut out = Vec::new();
    for (i, t) in transforms.iter().enumerate() {
        let placement = placements
            .iter()
            .find(|p| p["id"] == format!("o{i}"))
            .unwrap();
        assert_eq!(placement["plate_index"], 0, "{body}");
        out.push((t.clone(), matrix_of(placement)));
    }
    // Emprises réelles (géométrie affichée par l'éditeur) : aucune
    // superposition, espacement demandé respecté et objets dans le plateau.
    let hulls: Vec<Vec<DVec2>> = out.iter().map(|(_, m)| world_hull(m)).collect();
    for i in 0..hulls.len() {
        for p in &hulls[i] {
            assert!(
                (-0.01..=180.01).contains(&p.x) && (-0.01..=180.01).contains(&p.y),
                "o{i} hors plateau : {p:?} ({body})"
            );
        }
        for j in i + 1..hulls.len() {
            let gap = hull_gap(&hulls[i], &hulls[j]);
            assert!(
                gap >= spacing - 0.05,
                "o{i}/o{j} : écart {gap} < {spacing} — emprise fausse ({body})"
            );
        }
    }
    Some((out, spacing))
}

/// Rotation auto et alignement Y inactifs : Orca ne fait que translater
/// (apply_arrange_result garde la rotation) → matrice renvoyée = matrice de la
/// scène, translation XY près. Avant le correctif, la rotation revenait
/// transposée (angle inversé) et les échelles croisées.
#[tokio::test]
async fn arrange_keeps_rotation_and_scale_of_transformed_objects() {
    let Some((placed, _)) = arrange_bars(json!({
        "spacing": 4.0, "enable_rotation": false, "allow_multi_materials_on_same_plate": true,
        "avoid_extrusion_cali_region": false, "align_to_y_axis": false
    }))
    .await
    else {
        return;
    };
    for (transform, matrix) in placed {
        let expected = expected_matrix(&transform);
        for col in 0..3 {
            assert!(
                matrix.col(col).abs_diff_eq(expected.col(col), 1e-6),
                "colonne {col} : {matrix:?} ≠ {expected:?} ({transform})"
            );
        }
    }
}

/// Rotation auto : la matrice renvoyée est R_z(θ)·(T·R·S de la scène), comme
/// l'instance d'Orca après ArrangeJob (rotation Z ajoutée, X/Y et échelles
/// conservées).
#[tokio::test]
async fn arrange_auto_rotation_only_adds_a_z_rotation() {
    let Some((placed, _)) = arrange_bars(json!({
        "spacing": 3.0, "enable_rotation": true, "allow_multi_materials_on_same_plate": true,
        "avoid_extrusion_cali_region": false, "align_to_y_axis": false
    }))
    .await
    else {
        return;
    };
    for (transform, matrix) in placed {
        let linear = |m: DMat4| DMat4::from_mat3(glam::DMat3::from_mat4(m));
        let delta = linear(matrix) * linear(expected_matrix(&transform)).inverse();
        assert!(is_z_rotation(&delta), "{transform} : {delta:?}");
    }
}

/// Alignement Y (rotation auto inactive) : idem, rotation Z seule.
#[tokio::test]
async fn arrange_align_to_y_only_adds_a_z_rotation() {
    let Some((placed, _)) = arrange_bars(json!({
        "spacing": 2.0, "enable_rotation": false, "allow_multi_materials_on_same_plate": true,
        "avoid_extrusion_cali_region": false, "align_to_y_axis": true
    }))
    .await
    else {
        return;
    };
    for (transform, matrix) in placed {
        let linear = |m: DMat4| DMat4::from_mat3(glam::DMat3::from_mat4(m));
        let delta = linear(matrix) * linear(expected_matrix(&transform)).inverse();
        assert!(is_z_rotation(&delta), "{transform} : {delta:?}");
    }
}

// --- §2 Informations de coupe -------------------------------------------------

fn zip_text(bytes: &[u8], name: &str) -> Option<String> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let mut file = archive.by_name(name).ok()?;
    let mut text = String::new();
    file.read_to_string(&mut text).unwrap();
    Some(text)
}

/// Coupe avec connecteurs → cut_id commun + cut_info des connecteurs (Orca :
/// update_object_cut_id, apply_cut_connectors) ; export 3MF → cut_information.xml ;
/// réouverture → cutId / cutInfo relus ; « Invalidate cut info » → liens retirés.
#[tokio::test]
async fn cut_info_flows_through_cut_export_import_and_invalidate() {
    if !worker_available() {
        eprintln!("engine-worker absent — ignoré");
        return;
    }
    let (_d, app) = app().await;
    let user = register(&app, "cut37@test.local").await;
    let project = create_project(&app, &user, "coupe lot 37").await;
    let model = upload(
        &app,
        &project,
        &user,
        "cube.stl",
        &binary_stl(&box_triangles([20.0; 3])),
    )
    .await;
    save_scene(
        &app,
        &project,
        &user,
        json!({ "schemaVersion": 1, "objects": [{ "id": "src", "modelId": model,
                "transform": { "position": [50.0, 50.0, 10.0], "rotation": [0, 0, 0], "scale": [1, 1, 1] } }],
                "plates": { "plates": [{ "id": "p", "objectIds": ["src"] }], "activeId": "p" } }),
    )
    .await;

    // Coupe simple d'un objet non coupé : Orca n'enregistre rien.
    let (status, simple) = call(
        &app,
        "POST",
        &format!("/api/models/{model}/cut"),
        Some(json!({ "plane": { "point": [0, 0, 0] }, "rotation_deg": [0, 0, 0], "object_id": "src" })),
        &user,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{simple}");
    assert!(
        simple["parts"]
            .as_array()
            .unwrap()
            .iter()
            .all(|p| p.get("cut_id").is_none()),
        "{simple}"
    );

    let (status, cut) = call(
        &app,
        "POST",
        &format!("/api/models/{model}/cut"),
        Some(json!({
            "plane": { "point": [0, 0, 0] }, "rotation_deg": [0, 0, 0], "object_id": "src",
            "connectors": [
                { "pos": [-5, 0, 0], "type": "plug", "size_mm": 5, "depth_mm": 4, "size_tolerance_mm": 0.4 },
                { "pos": [5, 0, 0], "type": "dowel", "shape": "square" }
            ]
        })),
        &user,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{cut}");
    let parts = cut["parts"].as_array().unwrap();
    // check_sum = 1 + (−1 + haut + bas + 1 goujon) = 3 ; 2 connecteurs ; id libre = 1.
    let expected_cut = json!({ "id": 1, "checkSum": 3, "connectorsCnt": 2 });
    for part in parts {
        assert_eq!(part["cut_id"], expected_cut, "{part}");
    }
    let upper = &parts[0];
    let plug = upper["volumes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["name"] == "Connector-1")
        .unwrap();
    assert_eq!(plug["cut_info"]["connectorType"], "plug", "{plug}");
    assert!((plug["cut_info"]["radiusTolerance"].as_f64().unwrap() - 0.2).abs() < 1e-6);
    assert!((plug["cut_info"]["heightTolerance"].as_f64().unwrap() - 0.1).abs() < 1e-6);
    let dowel_hole = upper["volumes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["name"].as_str().unwrap().starts_with("Connector-2"))
        .unwrap_or_else(|| panic!("{upper}"));
    assert_eq!(dowel_hole["cut_info"]["connectorType"], "dowel");

    // Scène : une pièce par objet (comme l'éditeur), volumes maillés recopiés.
    let mut objects = Vec::new();
    for (i, part) in parts.iter().enumerate() {
        let volumes: Vec<Value> = part["volumes"]
            .as_array()
            .into_iter()
            .flatten()
            .enumerate()
            .map(|(k, v)| {
                let mut volume = json!({
                    "id": format!("v{i}-{k}"), "shape": "mesh", "modelId": v["model"]["id"],
                    "type": v["type"], "name": v["name"], "offset": v["offset"], "rotation": [0, 0, 0]
                });
                if let Some(info) = v.get("cut_info") {
                    volume["cutInfo"] = info.clone();
                }
                volume
            })
            .collect();
        objects.push(json!({
            "id": format!("part{i}"), "modelId": part["model"]["id"], "cutId": part["cut_id"],
            "volumes": volumes,
            "transform": { "position": [30.0 + 40.0 * i as f64, 50.0, 5.0], "rotation": [0, 0, 0], "scale": [1, 1, 1] }
        }));
    }
    let ids: Vec<String> = (0..parts.len()).map(|i| format!("part{i}")).collect();
    save_scene(
        &app,
        &project,
        &user,
        json!({ "schemaVersion": 1, "objects": objects,
                "plates": { "plates": [{ "id": "p", "objectIds": ids }], "activeId": "p" } }),
    )
    .await;

    // Re-couper une pièce déjà coupée (sans connecteur) : même id, +1 pièce.
    let (status, recut) = call(
        &app,
        "POST",
        &format!("/api/models/{}/cut", parts[0]["model"]["id"].as_str().unwrap()),
        Some(json!({ "plane": { "point": [0, 0, 0] }, "rotation_deg": [0, 0, 0], "object_id": "part0" })),
        &user,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{recut}");
    assert_eq!(
        recut["parts"][0]["cut_id"],
        json!({ "id": 1, "checkSum": 4, "connectorsCnt": 2 })
    );

    // Export store_bbs_3mf : cut_information.xml comme Orca.
    let resp = send(
        &app,
        request(
            "GET",
            &format!("/api/projects/{project}/export/3mf"),
            None,
            &user,
        ),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let exported = to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap()
        .to_vec();
    let xml = zip_text(&exported, "Metadata/cut_information.xml").expect("cut_information.xml");
    assert_eq!(
        xml.matches(r#"<cut_id id="1" check_sum="3" connectors_cnt="2"/>"#)
            .count(),
        3,
        "{xml}"
    );
    assert!(
        xml.contains(r#"type="0""#) && xml.contains(r#"type="1""#),
        "{xml}"
    );

    // Réouverture : liens et connecteurs relus.
    let (status, reopened) = json_of(
        send(
            &app,
            multipart("/api/projects/import", "cut.3mf", &exported, &user),
        )
        .await,
    )
    .await;
    assert!(status.is_success(), "{reopened}");
    let reopened_id = reopened["id"].as_str().unwrap().to_string();
    let (_, project_doc) = call(
        &app,
        "GET",
        &format!("/api/projects/{reopened_id}"),
        None,
        &user,
    )
    .await;
    let objects = project_doc["scene"]["objects"].as_array().unwrap().clone();
    assert_eq!(objects.len(), 3, "{project_doc}");
    for object in &objects {
        assert_eq!(object["cutId"], expected_cut, "{object}");
    }
    let connectors: Vec<&Value> = objects
        .iter()
        .flat_map(|o| o["volumes"].as_array().into_iter().flatten())
        .filter(|v| v.get("cutInfo").is_some())
        .collect();
    assert!(connectors.len() >= 2, "{project_doc}");
    assert!(connectors
        .iter()
        .any(|v| v["cutInfo"]["connectorType"] == "plug"));

    // « Invalidate cut info » : toutes les pièces liées, connecteurs compris.
    let first = objects[0]["id"].as_str().unwrap();
    let (status, body) = call(
        &app,
        "POST",
        &format!("/api/projects/{reopened_id}/objects/{first}/invalidate-cut"),
        Some(json!({ "expected_version": project_doc["version"] })),
        &user,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["object_ids"].as_array().unwrap().len(), 3, "{body}");
    let scene = &body["project"]["scene"];
    assert!(scene["objects"]
        .as_array()
        .unwrap()
        .iter()
        .all(|o| o.get("cutId").is_none()));
    assert!(scene["objects"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|o| o["volumes"].as_array().into_iter().flatten())
        .all(|v| v.get("cutInfo").is_none()));
    // Plus de cut_id : 422 ; objet inconnu : 404 ; version périmée : 409.
    let uri = format!("/api/projects/{reopened_id}/objects/{first}/invalidate-cut");
    assert_eq!(
        call(&app, "POST", &uri, None, &user).await.0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let unknown = format!("/api/projects/{reopened_id}/objects/nope/invalidate-cut");
    assert_eq!(
        call(&app, "POST", &unknown, None, &user).await.0,
        StatusCode::NOT_FOUND
    );
    // Isolation : un autre compte reçoit 404.
    let bob = register(&app, "bob37@test.local").await;
    assert_eq!(
        call(&app, "POST", &uri, None, &bob).await.0,
        StatusCode::NOT_FOUND
    );
}

// --- §3 Erreurs de maillage ---------------------------------------------------

/// Icône d'alerte et infobulle de la liste d'objets : statistiques libslic3r
/// du fichier, par modèle et par objet de scène. STL : arêtes ouvertes
/// restantes seulement (Orca 2.4.1 ne reporte pas les réparations d'import,
/// `TriangleMesh::from_stl` sous `#if 0`) ; 3MF : `<mesh_stat>` relu
/// (« N errors repaired ») ; après « Réparer », maillage neuf.
#[tokio::test]
async fn mesh_errors_per_model_and_object() {
    if !worker_available() {
        eprintln!("engine-worker absent — ignoré");
        return;
    }
    let (_d, app) = app().await;
    let user = register(&app, "mesh37@test.local").await;
    let project = create_project(&app, &user, "maillage lot 37").await;

    let clean = upload(
        &app,
        &project,
        &user,
        "clean.stl",
        &binary_stl(&box_triangles([10.0; 3])),
    )
    .await;
    let mut flipped = box_triangles([10.0; 3]);
    flipped[3].swap(1, 2);
    let p = flipped[0][0];
    flipped.push([p, p, flipped[0][1]]);
    let fixed = upload(&app, &project, &user, "flipped.stl", &binary_stl(&flipped)).await;
    let mut open = box_triangles([10.0; 3]);
    open.pop();
    let holed = upload(&app, &project, &user, "open.stl", &binary_stl(&open)).await;

    let errors = |id: String| {
        let app = app.clone();
        let user = user.clone();
        async move {
            call(
                &app,
                "GET",
                &format!("/api/models/{id}/mesh-errors"),
                None,
                &user,
            )
            .await
        }
    };
    let (status, info) = errors(clean.clone()).await;
    assert_eq!(status, StatusCode::OK, "{info}");
    assert_eq!(info["warning"], false, "{info}");
    assert_eq!(info["tooltip"], "");

    // Facette à l'envers + facette dégénérée : réparées en silence à l'import.
    let (_, info) = errors(fixed.clone()).await;
    assert_eq!(info["warning"], false, "{info}");
    assert_eq!(info["manifold"], true, "{info}");
    assert_eq!(info["repaired_errors"], 0, "{info}");

    let (_, info) = errors(holed.clone()).await;
    assert_eq!(info["manifold"], false, "{info}");
    assert_eq!(info["open_edges"], 3, "{info}");
    assert!(
        info["tooltip"]
            .as_str()
            .unwrap()
            .contains("3 non-manifold edges"),
        "{info}"
    );

    // Par objet : modèle + volumes maillés cumulés.
    save_scene(
        &app,
        &project,
        &user,
        json!({ "schemaVersion": 1, "objects": [
            { "id": "a", "modelId": clean },
            { "id": "b", "modelId": clean, "volumes": [
                { "id": "vb", "shape": "mesh", "modelId": holed, "type": "part", "name": "trou", "offset": [0, 0, 0] }
            ] }
        ], "plates": { "plates": [{ "id": "p", "objectIds": ["a", "b"] }], "activeId": "p" } }),
    )
    .await;
    let (status, list) = call(
        &app,
        "GET",
        &format!("/api/projects/{project}/mesh-errors"),
        None,
        &user,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{list}");
    let list = list.as_array().unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!(list[0]["object_id"], "a");
    assert_eq!(list[0]["warning"], false);
    assert_eq!(list[1]["object_id"], "b");
    assert_eq!(list[1]["open_edges"], 3, "{list:?}");

    // Après « Réparer » (admesh, trous bouchés) : maillage stocké neuf, fermé.
    let (status, _) = call(
        &app,
        "POST",
        &format!("/api/models/{holed}/repair"),
        Some(json!({})),
        &user,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (_, info) = errors(holed.clone()).await;
    assert_eq!(info["repaired"], false, "{info}");
    assert_eq!(info["open_edges"], 0, "{info}");
    assert_eq!(info["warning"], false, "{info}");

    // 3MF portant des erreurs réparées (<mesh_stat>, écrit par Orca) : relues à
    // l'ouverture du projet et affichées sur l'objet.
    let mut model = engine::api::Model::default();
    // Maillage indexé (sommets partagés), comme un 3MF d'Orca.
    let triangles = box_triangles([10.0; 3]);
    let mut vertices: Vec<[f32; 3]> = Vec::new();
    let indices = triangles
        .iter()
        .map(|t| {
            t.map(|p| {
                let index = vertices.iter().position(|v| *v == p).unwrap_or_else(|| {
                    vertices.push(p);
                    vertices.len() - 1
                });
                index as u32
            })
        })
        .collect();
    let mesh = engine::api::TriangleMesh { vertices, indices };
    model.add_object("repaired", mesh).volumes[0].mesh_stats = Some(engine::api::MeshErrorStats {
        open_edges: 0,
        repaired: engine::api::RepairedMeshErrors {
            edges_fixed: 2,
            degenerate_facets: 1,
            facets_removed: 0,
            facets_reversed: 4,
            backwards_edges: 0,
        },
    });
    let bytes = backend::engine::project_3mf::write_project_3mf(
        &model,
        &engine::api::DynamicPrintConfig::new(),
        &[],
    )
    .await
    .unwrap();
    let (status, imported) = json_of(
        send(
            &app,
            multipart("/api/projects/import", "repaired.3mf", &bytes, &user),
        )
        .await,
    )
    .await;
    assert!(status.is_success(), "{imported}");
    let imported_id = imported["id"].as_str().unwrap().to_string();
    let (_, list) = call(
        &app,
        "GET",
        &format!("/api/projects/{imported_id}/mesh-errors"),
        None,
        &user,
    )
    .await;
    let entry = &list.as_array().unwrap()[0];
    assert_eq!(entry["warning"], true, "{list}");
    assert_eq!(entry["manifold"], true, "{list}");
    assert_eq!(entry["repaired_errors"], 7, "{list}");
    assert_eq!(entry["facets_reversed"], 4, "{list}");
    assert_eq!(
        entry["tooltip"], "7 errors repaired\n\nClick the icon to repair model object",
        "{list}"
    );
    // … et réécrites à l'export du projet (<mesh_stat> comme Orca).
    let resp = send(
        &app,
        request(
            "GET",
            &format!("/api/projects/{imported_id}/export/3mf"),
            None,
            &user,
        ),
    )
    .await;
    let exported = to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap()
        .to_vec();
    let settings = zip_text(&exported, "Metadata/model_settings.config").unwrap();
    assert!(
        settings.contains(r#"edges_fixed="2" degenerate_facets="1" facets_removed="0" facets_reversed="4" backwards_edges="0""#),
        "{settings}"
    );

    // Isolation.
    let bob = register(&app, "bob-mesh37@test.local").await;
    assert_eq!(
        call(
            &app,
            "GET",
            &format!("/api/models/{fixed}/mesh-errors"),
            None,
            &bob
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        call(
            &app,
            "GET",
            &format!("/api/projects/{project}/mesh-errors"),
            None,
            &bob
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
}
