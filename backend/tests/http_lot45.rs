//! Lot 45 : `default_bed_type` conservé au seed des presets machine.
//!
//! Orca `Preset::get_default_bed_type` (Preset.cpp:966) lit l'option
//! `default_bed_type` du preset imprimante : « 4 » = btPTE (Textured PEI) pour
//! les WonderMaker ZR (×3), Elegoo Centauri et Centauri Carbon. Le seed la
//! retirait (champ méta de l'audit) : ces imprimantes recevaient High Temp.
//! Rejoue le seed de production (profils réels), puis le re-seed admin d'une
//! base existante (seed antérieur au lot 45), qui doit être idempotent.

use axum::body::{to_bytes, Body};
use axum::http::{header, Request, StatusCode};
use axum::Router;
use backend::adapters::files::FileStore;
use backend::adapters::storage::AnyStorage;
use backend::domain::presets::reseed_system_presets;
use backend::http::routes::router;
use backend::http::state::AppState;
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt;
use tower_sessions::{MemoryStore, SessionManagerLayer};

const TEXTURED_PEI_PRINTERS: [&str; 5] = [
    "WonderMaker ZR 0.4 nozzle",
    "WonderMaker ZR Ultra 0.4 nozzle",
    "WonderMaker ZR Ultra S 0.4 nozzle",
    "Elegoo Centauri 0.4 nozzle",
    "Elegoo Centauri Carbon 0.4 nozzle",
];

/// Base seedée « comme avant le lot 45 » (`default_bed_type` retiré de toutes
/// les valeurs) : reproduit une instance existante.
async fn app_with_pre_lot45_seed() -> Option<(tempfile::TempDir, Router)> {
    let imported =
        engine::presets::import_profiles(&backend::http::state::default_profiles_dir()).ok()?;
    let mut legacy = imported.presets;
    for p in &mut legacy {
        p.values.remove("default_bed_type");
    }
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}", dir.path().join("lot45.db").display());
    let storage = AnyStorage::connect_test(&url).await.unwrap();
    reseed_system_presets(&storage, &legacy).await.unwrap();
    let state = AppState::new(Arc::new(storage), FileStore::new(dir.path().join("data")));
    let layer = SessionManagerLayer::new(MemoryStore::default()).with_secure(false);
    Some((dir, router(state, layer)))
}

async fn call(
    app: &Router,
    method: &str,
    uri: &str,
    body: Value,
    cookie: Option<&str>,
) -> (StatusCode, Option<String>, Value) {
    let mut b = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(c) = cookie {
        b = b.header(header::COOKIE, c);
    }
    let resp = app
        .clone()
        .oneshot(b.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap();
    let status = resp.status();
    let cookie = resp
        .headers()
        .get(header::SET_COOKIE)
        .map(|v| v.to_str().unwrap().split(';').next().unwrap().to_string());
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let json = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, cookie, json)
}

/// Id du preset machine système `name`.
async fn machine_id(app: &Router, cookie: &str, name: &str) -> String {
    let (status, _, list) = call(
        app,
        "GET",
        "/api/presets?kind=machine",
        json!({}),
        Some(cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    list.as_array()
        .unwrap()
        .iter()
        .find(|p| p["name"] == name)
        .unwrap_or_else(|| panic!("preset machine {name} absent"))["id"]
        .as_str()
        .unwrap()
        .to_string()
}

async fn resolved(app: &Router, cookie: &str, id: &str) -> Value {
    let (status, _, body) = call(
        app,
        "GET",
        &format!("/api/presets/{id}/resolved"),
        json!({}),
        Some(cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body
}

#[tokio::test]
async fn reseed_restores_numeric_default_bed_type() {
    let Some((_d, app)) = app_with_pre_lot45_seed().await else {
        return; // profils absents (build hors dépôt)
    };
    // 1er compte = admin (politique ouverte par défaut).
    let (status, cookie, _) = call(
        &app,
        "POST",
        "/api/auth/register",
        json!({ "email": "lot45@test.local", "password": "password123" }),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let cookie = cookie.unwrap();

    // Avant re-seed : base existante, la valeur manque → repli btPEI (High Temp).
    let mut ids = Vec::new();
    for name in TEXTURED_PEI_PRINTERS {
        let id = machine_id(&app, &cookie, name).await;
        let body = resolved(&app, &cookie, &id).await;
        assert_eq!(body["default_bed_type"], "High Temp Plate", "{name} avant");
        ids.push(id);
    }

    // Re-seed admin (deux fois : idempotent, UUID conservés).
    for round in 0..2 {
        let (status, _, body) = call(
            &app,
            "POST",
            "/api/admin/presets/reseed",
            json!({}),
            Some(&cookie),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "re-seed {round} : {body}");
        assert_eq!(body["reseeded"], 11_895);
        for (name, id) in TEXTURED_PEI_PRINTERS.iter().zip(&ids) {
            assert_eq!(
                &machine_id(&app, &cookie, name).await,
                id,
                "{name} : UUID conservé"
            );
            let body = resolved(&app, &cookie, id).await;
            assert_eq!(body["values"]["default_bed_type"], "4", "{name}");
            assert_eq!(body["default_bed_type"], "Textured PEI Plate", "{name}");
            assert_eq!(body["default_plate_type"], "Textured PEI Plate", "{name}");
        }
    }

    // Témoins inchangés : texte « Textured PEI Plate » (atoi = 0) → High
    // Temp ; X1 Carbon (default_bed_type seulement dans la fiche du modèle) →
    // Cool Plate ; Voron (aucune valeur) → High Temp.
    for (name, bed, plate) in [
        (
            "Snapmaker U1 (0.4 nozzle)",
            "High Temp Plate",
            "Smooth High Temp Plate",
        ),
        (
            "Bambu Lab X1 Carbon 0.4 nozzle",
            "Cool Plate",
            "Cool Plate / PLA Plate",
        ),
        (
            "Voron 2.4 350 0.4 nozzle",
            "High Temp Plate",
            "Smooth High Temp Plate",
        ),
    ] {
        let id = machine_id(&app, &cookie, name).await;
        let body = resolved(&app, &cookie, &id).await;
        assert_eq!(body["default_bed_type"], bed, "{name}");
        assert_eq!(body["default_plate_type"], plate, "{name}");
    }
}
