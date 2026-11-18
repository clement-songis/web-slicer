//! Calibrations côté serveur (lot 14) : historique des avances de pression
//! (CRUD, filtres, isolation, validation) et construction d'un projet de
//! calibration par libslic3r (portage des Plater::calib_* d'Orca).

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

fn entry(k: f64, filament: &str) -> serde_json::Value {
    json!({
        "machine_preset": "Voron 2.4 350 0.4 nozzle",
        "filament_preset": filament,
        "filament_id": "GFL99",
        "nozzle_diameter": 0.4,
        "k_value": k,
        "method": "pa_pattern"
    })
}

#[tokio::test]
async fn pa_history_crud_filters_and_isolation() {
    let (_d, app) = app().await;
    let user = register(&app, "pa@test.local").await;
    let a = json_body(
        send(
            &app,
            request(
                "POST",
                "/api/calibrations/pa",
                entry(0.04, "Generic PLA"),
                Some(&user),
            ),
        )
        .await,
    )
    .await;
    assert_eq!(a["k_value"], 0.04);
    assert_eq!(a["n_coef"], 1.0);
    assert_eq!(a["name"], "Generic PLA 0.4", "nom par défaut");
    assert!(a["created_at"].as_str().unwrap().contains('T'));
    let id = a["id"].as_str().unwrap().to_string();
    let resp = send(
        &app,
        request(
            "POST",
            "/api/calibrations/pa",
            entry(0.06, "Generic PETG"),
            Some(&user),
        ),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    // Liste (plus récent d'abord) + filtres.
    let all = json_body(
        send(
            &app,
            request("GET", "/api/calibrations/pa", json!({}), Some(&user)),
        )
        .await,
    )
    .await;
    assert_eq!(all.as_array().unwrap().len(), 2);
    assert_eq!(all[0]["filament_preset"], "Generic PETG");
    let pla = json_body(
        send(
            &app,
            request(
                "GET",
                "/api/calibrations/pa?filament_preset=Generic%20PLA&nozzle_diameter=0.4",
                json!({}),
                Some(&user),
            ),
        )
        .await,
    )
    .await;
    assert_eq!(pla.as_array().unwrap().len(), 1);
    let none = json_body(
        send(
            &app,
            request(
                "GET",
                "/api/calibrations/pa?nozzle_diameter=0.6",
                json!({}),
                Some(&user),
            ),
        )
        .await,
    )
    .await;
    assert!(none.as_array().unwrap().is_empty());

    // Mise à jour.
    let mut changed = entry(0.045, "Generic PLA");
    changed["note"] = "retenue après motif".into();
    let upd = json_body(
        send(
            &app,
            request(
                "PUT",
                &format!("/api/calibrations/pa/{id}"),
                changed,
                Some(&user),
            ),
        )
        .await,
    )
    .await;
    assert_eq!(upd["k_value"], 0.045);
    assert_eq!(upd["note"], "retenue après motif");

    // Validation.
    for (field, body) in [
        ("k_value", entry(3.0, "X")),
        ("method", {
            let mut b = entry(0.04, "X");
            b["method"] = "guess".into();
            b
        }),
        ("nozzle_diameter", {
            let mut b = entry(0.04, "X");
            b["nozzle_diameter"] = 0.0.into();
            b
        }),
        ("printer", {
            let mut b = entry(0.04, "X");
            b["printer_id"] = uuid::Uuid::new_v4().to_string().into();
            b
        }),
    ] {
        let resp = send(
            &app,
            request("POST", "/api/calibrations/pa", body, Some(&user)),
        )
        .await;
        assert!(
            matches!(
                resp.status(),
                StatusCode::UNPROCESSABLE_ENTITY | StatusCode::NOT_FOUND
            ),
            "{field} : {}",
            resp.status()
        );
    }

    // Isolation : un autre compte ne voit ni ne modifie rien.
    let other = register(&app, "other-pa@test.local").await;
    let list = json_body(
        send(
            &app,
            request("GET", "/api/calibrations/pa", json!({}), Some(&other)),
        )
        .await,
    )
    .await;
    assert!(list.as_array().unwrap().is_empty());
    let resp = send(
        &app,
        request(
            "DELETE",
            &format!("/api/calibrations/pa/{id}"),
            json!({}),
            Some(&other),
        ),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    // Suppression.
    let resp = send(
        &app,
        request(
            "DELETE",
            &format!("/api/calibrations/pa/{id}"),
            json!({}),
            Some(&user),
        ),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    let resp = send(
        &app,
        request(
            "GET",
            &format!("/api/calibrations/pa/{id}"),
            json!({}),
            Some(&user),
        ),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let anonymous = send(
        &app,
        request("GET", "/api/calibrations/pa", json!({}), None),
    )
    .await;
    assert_eq!(anonymous.status(), StatusCode::UNAUTHORIZED);
}

/// Projet de calibration construit par le serveur (tour de température) :
/// objet de resources/calib coupé à 9 blocs, réglages d'objet et presets
/// édités d'Orca conservés, paramètres du mode calibration dans la scène.
#[tokio::test]
async fn temperature_tower_project_is_built_like_orca() {
    if !backend::engine::worker_binary().exists() {
        eprintln!("engine-worker absent — ignoré");
        return;
    }
    let (_d, app) = app().await;
    let user = register(&app, "calib@test.local").await;
    let project = json_body(
        send(
            &app,
            request(
                "POST",
                "/api/projects",
                json!({ "name": "Calib" }),
                Some(&user),
            ),
        )
        .await,
    )
    .await;
    let id = project["id"].as_str().unwrap();
    let resp = send(
        &app,
        request(
            "POST",
            &format!("/api/projects/{id}/calibration"),
            json!({ "kind": "temp_tower", "start": 230, "end": 190, "step": 5 }),
            Some(&user),
        ),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = json_body(resp).await;
    assert_eq!(body["name"], "Nozzle temperature test");
    assert_eq!(body["calibration"]["mode"], "temp_tower");
    assert_eq!(body["overrides"]["nozzle_temperature"], json!([230]));
    let objects = body["scene"]["objects"].as_array().unwrap();
    assert_eq!(objects.len(), 1);
    // Réglages d'objet typés selon le schéma des options (nombre, pas « 0.2 »).
    assert_eq!(
        objects[0]["settings"]["layer_height"],
        json!(0.2),
        "{}",
        objects[0]["settings"]
    );
    let saved = json_body(
        send(
            &app,
            request(
                "GET",
                &format!("/api/projects/{id}"),
                json!({}),
                Some(&user),
            ),
        )
        .await,
    )
    .await;
    assert_eq!(saved["scene"]["calibration"]["start"], 230.0);
    assert!(saved["scene"]["calibrationOverrides"].is_object());

    // Calibrations portées au lot 15 : motif PA (G-code du motif dans la scène)
    // et input shaping (paramètres du mode calibration conservés).
    let resp = send(
        &app,
        request(
            "POST",
            &format!("/api/projects/{id}/calibration"),
            json!({ "kind": "pa_pattern", "start": 0, "end": 0.04, "step": 0.01 }),
            Some(&user),
        ),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = json_body(resp).await;
    assert_eq!(body["calibration"]["mode"], "pa_pattern");
    let saved = json_body(
        send(
            &app,
            request(
                "GET",
                &format!("/api/projects/{id}"),
                json!({}),
                Some(&user),
            ),
        )
        .await,
    )
    .await;
    assert!(
        saved["scene"]["objects"][0]["name"]
            .as_str()
            .unwrap_or("")
            .starts_with("pa_pattern_")
            || body["scene"]["objects"]
                .as_array()
                .is_some_and(|o| !o.is_empty())
    );
    let resp = send(
        &app,
        request(
            "POST",
            &format!("/api/projects/{id}/calibration"),
            json!({ "kind": "is_freq", "start": 0.15, "freq_start_x": 15, "freq_end_x": 110,
                    "freq_start_y": 15, "freq_end_y": 110, "shaper_type": "MZV" }),
            Some(&user),
        ),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = json_body(resp).await;
    assert_eq!(body["calibration"]["mode"], "input_shaping_freq");
    assert_eq!(body["calibration"]["shaper_type"], "MZV");
    assert_eq!(body["calibration"]["freq_end_x"], 110.0);

    // Inconnue → 422.
    let resp = send(
        &app,
        request(
            "POST",
            &format!("/api/projects/{id}/calibration"),
            json!({ "kind": "nope" }),
            Some(&user),
        ),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

/// Projet de calibration du débit (passe 1) : `print_flow_ratio` de chaque
/// objet (« flowrate_m5 » … lu dans les métadonnées du 3MF d'Orca) rendu en
/// nombre selon le schéma des options (BACKEND.md §6), pas en chaîne « 0.95 ».
#[tokio::test]
async fn flow_pass1_object_settings_are_typed() {
    if !backend::engine::worker_binary().exists() {
        eprintln!("engine-worker absent — ignoré");
        return;
    }
    let (_d, app) = app().await;
    let user = register(&app, "calib-flow@test.local").await;
    let project = json_body(
        send(
            &app,
            request(
                "POST",
                "/api/projects",
                json!({ "name": "Débit" }),
                Some(&user),
            ),
        )
        .await,
    )
    .await;
    let id = project["id"].as_str().unwrap();
    let resp = send(
        &app,
        request(
            "POST",
            &format!("/api/projects/{id}/calibration"),
            json!({ "kind": "flow_pass1" }),
            Some(&user),
        ),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = json_body(resp).await;
    let objects = body["scene"]["objects"].as_array().unwrap();
    assert!(objects.len() >= 9, "{}", objects.len());
    let mut ratios: Vec<f64> = objects
        .iter()
        .map(|o| {
            let v = &o["settings"]["print_flow_ratio"];
            v.as_f64()
                .unwrap_or_else(|| panic!("print_flow_ratio non numérique : {v}"))
        })
        .collect();
    ratios.sort_by(f64::total_cmp);
    ratios.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
    assert!(ratios.len() >= 9, "{ratios:?}");
    assert!(ratios.iter().all(|r| (0.7..=1.3).contains(r)), "{ratios:?}");
    for o in objects {
        for (key, value) in o["settings"].as_object().unwrap() {
            if let Some(def) = engine::params::get(key) {
                use engine::params::ParamKind as K;
                // Les pourcentages restent « 15% » (chaîne) pour un export 3MF identique à Orca.
                let percent_text = value.as_str().is_some_and(|s| s.trim_end().ends_with('%'));
                if matches!(def.kind, K::Float | K::Int | K::Percent) && !percent_text {
                    assert!(value.is_number(), "{key} = {value}");
                }
                if def.kind == K::Bool {
                    assert!(value.is_boolean(), "{key} = {value}");
                }
            }
        }
    }
}
