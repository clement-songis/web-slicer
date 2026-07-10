//! Endpoints presets (T039) de bout en bout — FR-020..023 :
//! liste filtrée par instanciation/compatibilité, valeurs brutes vs résolues,
//! CRUD utilisateur, import/export JSON Orca avec conversion des clés legacy,
//! et **isolation inter-comptes** (SC-008 : un preset d'autrui répond 404).

use axum::body::{to_bytes, Body};
use axum::http::{header, Request, StatusCode};
use axum::response::Response;
use axum::Router;
use backend::adapters::files::FileStore;
use backend::adapters::storage::sqlite::SqliteStorage;
use backend::domain::{Preset, PresetId, PresetKind, PresetOrigin, Storage};
use backend::http::routes::router;
use backend::http::state::AppState;
use std::sync::Arc;
use tower::ServiceExt;
use tower_sessions::{MemoryStore, SessionManagerLayer};

/// Preset système synthétique (contrôle total du vendeur/héritage/valeurs).
fn sys(
    kind: PresetKind,
    name: &str,
    inherits: Option<&str>,
    instantiation: bool,
    values: serde_json::Value,
) -> Preset {
    Preset {
        id: PresetId::new(),
        kind,
        name: name.into(),
        origin: PresetOrigin::System,
        user_id: None,
        vendor: Some("TestVendor".into()),
        inherits: inherits.map(String::from),
        instantiation,
        setting_id: None,
        filament_id: None,
        compatible_printers: None,
        values,
    }
}

/// App avec un jeu de presets système : une chaîne process (racine abstraite →
/// feuille concrète) et deux filaments dont un restreint à une imprimante.
async fn app() -> (tempfile::TempDir, Router) {
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}", dir.path().join("presets.db").display());
    let storage = Arc::new(SqliteStorage::connect(&url).await.unwrap());

    // Parent abstrait (masqué de la liste) porteur des défauts hérités.
    let process_common = sys(
        PresetKind::Process,
        "fdm_process_common",
        None,
        false,
        serde_json::json!({ "layer_height": "0.2", "wall_loops": "3" }),
    );
    let standard = sys(
        PresetKind::Process,
        "0.20mm Standard",
        Some("fdm_process_common"),
        true,
        serde_json::json!({ "layer_height": "0.20" }),
    );

    let generic_pla = sys(
        PresetKind::Filament,
        "Generic PLA",
        None,
        true,
        serde_json::json!({ "filament_flow_ratio": "0.98" }),
    );
    let mut restricted = sys(
        PresetKind::Filament,
        "Special PLA",
        None,
        true,
        serde_json::json!({}),
    );
    restricted.compatible_printers = Some(serde_json::json!(["Printer A"]));

    storage
        .presets()
        .reseed_system(vec![process_common, standard, generic_pla, restricted])
        .await
        .unwrap();

    let files = FileStore::new(dir.path().join("data"));
    let state = AppState::new(storage, files);
    let session_layer = SessionManagerLayer::new(MemoryStore::default()).with_secure(false);
    (dir, router(state, session_layer))
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

/// FR-020/FR-021 : la liste masque les presets abstraits (`instantiation=false`)
/// et applique le filtre de compatibilité imprimante.
#[tokio::test]
async fn list_hides_abstract_and_filters_by_printer() {
    let (_d, app) = app().await;
    let user = register(&app, "boss@test.local").await;

    // Process : seule la feuille concrète apparaît, pas le parent abstrait.
    let resp = send(
        &app,
        request(
            "GET",
            "/api/presets?kind=process",
            serde_json::json!({}),
            Some(&user),
        ),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let list = json_body(resp).await;
    let names: Vec<&str> = list
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["0.20mm Standard"], "le parent abstrait est masqué");

    // Filaments sans imprimante : les deux visibles.
    let resp = send(
        &app,
        request(
            "GET",
            "/api/presets?kind=filament",
            serde_json::json!({}),
            Some(&user),
        ),
    )
    .await;
    assert_eq!(json_body(resp).await.as_array().unwrap().len(), 2);

    // Avec « Printer B », le filament restreint à « Printer A » disparaît.
    let resp = send(
        &app,
        request(
            "GET",
            "/api/presets?kind=filament&printer=Printer%20B",
            serde_json::json!({}),
            Some(&user),
        ),
    )
    .await;
    let names: Vec<String> = json_body(resp)
        .await
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["name"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(
        names,
        ["Generic PLA"],
        "compat imprimante appliquée (FR-021)"
    );
}

/// FR-020 : `GET /{id}` renvoie les surcharges brutes ; `…/resolved` la chaîne
/// d'héritage aplatie (valeur héritée du parent incluse).
#[tokio::test]
async fn raw_versus_resolved_values() {
    let (_d, app) = app().await;
    let user = register(&app, "boss@test.local").await;

    let list = json_body(
        send(
            &app,
            request(
                "GET",
                "/api/presets?kind=process",
                serde_json::json!({}),
                Some(&user),
            ),
        )
        .await,
    )
    .await;
    let id = list[0]["id"].as_str().unwrap().to_string();

    // Brut : seule la surcharge de la feuille.
    let raw = json_body(
        send(
            &app,
            request(
                "GET",
                &format!("/api/presets/{id}"),
                serde_json::json!({}),
                Some(&user),
            ),
        )
        .await,
    )
    .await;
    assert_eq!(raw["values"]["layer_height"], "0.20");
    assert!(
        raw["values"].get("wall_loops").is_none(),
        "wall_loops vient du parent"
    );

    // Résolu : valeurs effectives typées, wall_loops hérité de la racine.
    let resolved = json_body(
        send(
            &app,
            request(
                "GET",
                &format!("/api/presets/{id}/resolved"),
                serde_json::json!({}),
                Some(&user),
            ),
        )
        .await,
    )
    .await;
    assert_eq!(resolved["values"]["layer_height"], 0.2);
    assert_eq!(resolved["values"]["wall_loops"], 3);
}

/// FR-022 : cycle de vie d'un preset utilisateur (créer / renommer+valeurs /
/// supprimer).
#[tokio::test]
async fn user_preset_crud() {
    let (_d, app) = app().await;
    let user = register(&app, "boss@test.local").await;

    let created = json_body(
        send(
            &app,
            request(
                "POST",
                "/api/presets",
                serde_json::json!({
                    "kind": "filament",
                    "name": "Mon PLA",
                    "inherits": "Generic PLA",
                    "values": { "filament_flow_ratio": "0.95" }
                }),
                Some(&user),
            ),
        )
        .await,
    )
    .await;
    assert_eq!(created["origin"], "user");
    let id = created["id"].as_str().unwrap().to_string();

    // (kind, name) dupliqué → 409.
    let dup = send(
        &app,
        request(
            "POST",
            "/api/presets",
            serde_json::json!({ "kind": "filament", "name": "Mon PLA", "values": {} }),
            Some(&user),
        ),
    )
    .await;
    assert_eq!(dup.status(), StatusCode::CONFLICT);

    // Renommage + nouvelles valeurs.
    let updated = json_body(send(
        &app,
        request(
            "PUT",
            &format!("/api/presets/{id}"),
            serde_json::json!({ "name": "Mon PLA v2", "values": { "filament_flow_ratio": "0.97" } }),
            Some(&user),
        ),
    )
    .await)
    .await;
    assert_eq!(updated["name"], "Mon PLA v2");
    assert_eq!(updated["values"]["filament_flow_ratio"], "0.97");

    // Suppression.
    let del = send(
        &app,
        request(
            "DELETE",
            &format!("/api/presets/{id}"),
            serde_json::json!({}),
            Some(&user),
        ),
    )
    .await;
    assert_eq!(del.status(), StatusCode::NO_CONTENT);
    let gone = send(
        &app,
        request(
            "GET",
            &format!("/api/presets/{id}"),
            serde_json::json!({}),
            Some(&user),
        ),
    )
    .await;
    assert_eq!(gone.status(), StatusCode::NOT_FOUND);
}

/// FR-023 : import d'un profil Orca — clés legacy renommées, clés inconnues
/// abandonnées, méta ignorés.
#[tokio::test]
async fn import_converts_legacy_keys() {
    let (_d, app) = app().await;
    let user = register(&app, "boss@test.local").await;

    let created = json_body(
        send(
            &app,
            request(
                "POST",
                "/api/presets/import",
                serde_json::json!({
                    "kind": "process",
                    "name": "Importé",
                    "values": {
                        "support_material_angle": "55", // legacy → support_angle
                        "layer_height": "0.15",         // conservé tel quel
                        "foobar_unknown": "x"           // hors registre → abandonné
                    }
                }),
                Some(&user),
            ),
        )
        .await,
    )
    .await;
    let values = &created["values"];
    assert_eq!(
        values["support_angle"], "55",
        "clé legacy renommée (FR-023)"
    );
    assert!(
        values.get("support_material_angle").is_none(),
        "ancienne clé retirée"
    );
    assert_eq!(values["layer_height"], "0.15");
    assert!(
        values.get("foobar_unknown").is_none(),
        "clé inconnue abandonnée"
    );
}

/// FR-023 : export au format JSON Orca (`type`/`name`/`inherits`/valeurs).
#[tokio::test]
async fn export_produces_orca_json() {
    let (_d, app) = app().await;
    let user = register(&app, "boss@test.local").await;

    let created = json_body(
        send(
            &app,
            request(
                "POST",
                "/api/presets",
                serde_json::json!({
                    "kind": "filament",
                    "name": "Export PLA",
                    "inherits": "Generic PLA",
                    "values": { "filament_flow_ratio": "0.99" }
                }),
                Some(&user),
            ),
        )
        .await,
    )
    .await;
    let id = created["id"].as_str().unwrap().to_string();

    let exported = json_body(
        send(
            &app,
            request(
                "GET",
                &format!("/api/presets/{id}/export"),
                serde_json::json!({}),
                Some(&user),
            ),
        )
        .await,
    )
    .await;
    assert_eq!(exported["type"], "filament");
    assert_eq!(exported["name"], "Export PLA");
    assert_eq!(exported["inherits"], "Generic PLA");
    assert_eq!(exported["filament_flow_ratio"], "0.99");
}

/// SC-008 : un preset **utilisateur** d'un autre compte est invisible (404 sur
/// GET/PUT/DELETE), jamais 403. Les presets système restent partagés.
#[tokio::test]
async fn user_presets_isolated_between_accounts() {
    let (_d, app) = app().await;
    let alice = register(&app, "alice@test.local").await;
    let bob = register(&app, "bob@test.local").await;

    let created = json_body(
        send(
            &app,
            request(
                "POST",
                "/api/presets",
                serde_json::json!({ "kind": "filament", "name": "PLA secret", "values": {} }),
                Some(&alice),
            ),
        )
        .await,
    )
    .await;
    let id = created["id"].as_str().unwrap().to_string();

    let get = request(
        "GET",
        &format!("/api/presets/{id}"),
        serde_json::json!({}),
        Some(&bob),
    );
    let put = request(
        "PUT",
        &format!("/api/presets/{id}"),
        serde_json::json!({ "name": "vol" }),
        Some(&bob),
    );
    let del = request(
        "DELETE",
        &format!("/api/presets/{id}"),
        serde_json::json!({}),
        Some(&bob),
    );
    for req in [get, put, del] {
        let resp = send(&app, req).await;
        assert_eq!(
            resp.status(),
            StatusCode::NOT_FOUND,
            "preset d'autrui doit répondre 404 (SC-008)"
        );
    }

    // Bob voit quand même les presets système partagés.
    let resp = send(
        &app,
        request(
            "GET",
            "/api/presets?kind=filament",
            serde_json::json!({}),
            Some(&bob),
        ),
    )
    .await;
    let list = json_body(resp).await;
    assert!(
        list.as_array()
            .unwrap()
            .iter()
            .any(|p| p["name"] == "Generic PLA"),
        "presets système visibles par tous"
    );
    assert!(
        !list
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["name"] == "PLA secret"),
        "preset utilisateur d'Alice invisible pour Bob"
    );
}

#[tokio::test]
async fn presets_require_a_session() {
    let (_d, app) = app().await;
    let resp = send(
        &app,
        request(
            "GET",
            "/api/presets?kind=process",
            serde_json::json!({}),
            None,
        ),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
