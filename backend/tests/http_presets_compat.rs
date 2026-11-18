//! Compatibilité exacte des process avec l'imprimante active, comme Orca
//! (`Preset::is_compatible_with_printer`) : `compatible_printers` et
//! `compatible_printers_condition` résolus sur la chaîne `inherits`, puis
//! évalués sur la config résolue de l'imprimante. Rejoue le seed de production
//! (profils réels). Les nombres attendus viennent d'un calcul indépendant
//! effectué directement sur les fichiers `resources/profiles` d'OrcaSlicer
//! (script `.ux/preset-search/orca-truth.mjs` du rapport).

use axum::body::{to_bytes, Body};
use axum::http::{header, Request, StatusCode};
use axum::Router;
use backend::adapters::files::FileStore;
use backend::adapters::storage::AnyStorage;
use backend::domain::presets::reseed_system_presets;
use backend::http::routes::router;
use backend::http::state::AppState;
use std::sync::Arc;
use tower::ServiceExt;
use tower_sessions::{MemoryStore, SessionManagerLayer};

async fn app_with_real_profiles() -> Option<(tempfile::TempDir, Router)> {
    let imported =
        engine::presets::import_profiles(&backend::http::state::default_profiles_dir()).ok()?;
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}", dir.path().join("compat.db").display());
    let storage = AnyStorage::connect_test(&url).await.unwrap();
    reseed_system_presets(&storage, &imported.presets)
        .await
        .unwrap();
    let state = AppState::new(Arc::new(storage), FileStore::new(dir.path().join("data")));
    let layer = SessionManagerLayer::new(MemoryStore::default()).with_secure(false);
    Some((dir, router(state, layer)))
}

async fn call(
    app: &Router,
    method: &str,
    uri: &str,
    body: serde_json::Value,
    cookie: Option<&str>,
) -> (StatusCode, Option<String>, serde_json::Value) {
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
    let json = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    (status, cookie, json)
}

async fn process_names(app: &Router, cookie: &str, printer: &str) -> Vec<String> {
    let uri = format!(
        "/api/presets?kind=process&printer={}",
        printer.replace(' ', "%20")
    );
    let (status, _, list) = call(app, "GET", &uri, serde_json::json!({}), Some(cookie)).await;
    assert_eq!(status, StatusCode::OK);
    list.as_array()
        .unwrap()
        .iter()
        .map(|p| p["name"].as_str().unwrap().to_string())
        .collect()
}

#[tokio::test]
async fn process_compatibility_matches_orca() {
    let Some((_d, app)) = app_with_real_profiles().await else {
        return; // profils absents (build hors dépôt)
    };
    let (status, cookie, _) = call(
        &app,
        "POST",
        "/api/auth/register",
        serde_json::json!({ "email": "compat@test.local", "password": "password123" }),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let cookie = cookie.unwrap();

    // Voron : compatibilité par liste héritée (les feuilles « @Voron » n'ont
    // pas de liste propre — avant ce correctif, 594 process « universels »).
    let voron = process_names(&app, &cookie, "Voron 2.4 350 0.4 nozzle").await;
    assert_eq!(voron.len(), 6, "{voron:?}");
    assert!(voron.iter().all(|n| n.ends_with("@Voron")), "{voron:?}");
    assert!(voron.iter().any(|n| n == "0.20mm Standard @Voron"));

    // Prusa MK4S : compatibilité par condition (printer_notes / nozzle).
    let mk4s = process_names(&app, &cookie, "Prusa MK4S 0.4 nozzle").await;
    assert_eq!(mk4s.len(), 5, "{mk4s:?}");
    assert!(mk4s.iter().all(|n| n.contains("@MK4S 0.4")), "{mk4s:?}");
    // « SOLUBLE » exige single_extruder_multi_material (faux sur la MK4S).
    assert!(!mk4s.iter().any(|n| n.contains("SOLUBLE")));

    // Variante HF : la négation printer_notes!~/HF_NOZZLE/ bascule les profils.
    let hf = process_names(&app, &cookie, "Prusa MK4S HF0.4 nozzle").await;
    assert_eq!(hf.len(), 8, "{hf:?}");

    let x1c = process_names(&app, &cookie, "Bambu Lab X1 Carbon 0.4 nozzle").await;
    assert_eq!(x1c.len(), 10, "{x1c:?}");

    // Imprimante inconnue : filtre de liste historique (aucune régression).
    let unknown = process_names(&app, &cookie, "Imprimante inconnue").await;
    assert!(unknown.len() > 500, "{}", unknown.len());
}

/// Imprimante personnalisée créée depuis un modèle du catalogue (preset machine
/// utilisateur héritant d'un preset système) : comme Orca
/// (`is_compatible_with_parent_printer`), elle reçoit les process et filaments
/// compatibles avec son parent.
#[tokio::test]
async fn user_printer_inherits_parent_compatibility() {
    let Some((_d, app)) = app_with_real_profiles().await else {
        return;
    };
    let (_, cookie, _) = call(
        &app,
        "POST",
        "/api/auth/register",
        serde_json::json!({ "email": "atelier@test.local", "password": "motdepasse-solide-42" }),
        None,
    )
    .await;
    let cookie = cookie.expect("session");
    let parent = "Voron 2.4 350 0.4 nozzle";
    let (status, _, created) = call(
        &app,
        "POST",
        "/api/presets",
        serde_json::json!({ "kind": "machine", "name": "Atelier Compat 0.4 nozzle", "inherits": parent,
                            "values": { "printable_height": 200 } }),
        Some(&cookie),
    )
    .await;
    assert!(status.is_success(), "{status} {created}");
    for kind in ["process", "filament"] {
        let names = |printer: &str| {
            let uri = format!(
                "/api/presets?kind={kind}&printer={}",
                printer.replace(' ', "%20")
            );
            let app = app.clone();
            let cookie = cookie.clone();
            async move {
                let (status, _, list) =
                    call(&app, "GET", &uri, serde_json::json!({}), Some(&cookie)).await;
                assert_eq!(status, StatusCode::OK);
                let mut v: Vec<String> = list
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|p| p["name"].as_str().unwrap().to_string())
                    .collect();
                v.sort();
                v
            }
        };
        let custom = names("Atelier Compat 0.4 nozzle").await;
        let base = names(parent).await;
        eprintln!(
            "{kind} : personnalisée {} / parent {}",
            custom.len(),
            base.len()
        );
        assert!(!custom.is_empty(), "{kind} : aucun preset compatible");
        assert_eq!(custom, base, "{kind} : même compatibilité que le parent");
    }
}
