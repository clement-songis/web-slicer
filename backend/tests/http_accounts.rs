//! Lot 32 — comptes et quotas : liste paginée des comptes (admin), changement
//! de mot de passe avec révocation des autres sessions, code
//! `invalid_credentials` au login, quotas de stockage (413) et de tranchages
//! simultanés (429 + Retry-After, lot 35), noms de projet uniques côté serveur,
//! désactivation/réactivation de compte (lot 35).

use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{header, Request, StatusCode};
use axum::response::Response;
use axum::Router;
use backend::adapters::files::FileStore;
use backend::adapters::storage::AnyStorage;
use backend::domain::{Storage, UserId};
use backend::http::routes::router;
use backend::http::state::AppState;
use serde_json::{json, Value};
use tower::ServiceExt;
use tower_sessions::{MemoryStore, SessionManagerLayer};

struct App {
    dir: tempfile::TempDir,
    storage: Arc<AnyStorage>,
    app: Router,
}

async fn app() -> App {
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}", dir.path().join("accounts.db").display());
    let storage = Arc::new(AnyStorage::connect_test(&url).await.unwrap());
    let files = FileStore::new(dir.path().join("data"));
    let state = AppState::new(Arc::clone(&storage) as Arc<dyn Storage>, files);
    let session_layer = SessionManagerLayer::new(MemoryStore::default()).with_secure(false);
    App {
        dir,
        storage,
        app: router(state, session_layer),
    }
}

fn req(method: &str, uri: &str, body: Option<Value>, cookie: Option<&str>) -> Request<Body> {
    let mut b = Request::builder().method(method).uri(uri);
    if let Some(c) = cookie {
        b = b.header(header::COOKIE, c);
    }
    match body {
        Some(v) => b
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(v.to_string()))
            .unwrap(),
        None => b.body(Body::empty()).unwrap(),
    }
}

async fn send(app: &Router, r: Request<Body>) -> (StatusCode, Option<String>, Value) {
    let resp: Response = app.clone().oneshot(r).await.unwrap();
    let status = resp.status();
    let cookie = resp
        .headers()
        .get(header::SET_COOKIE)
        .map(|v| v.to_str().unwrap().split(';').next().unwrap().to_string());
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let body = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(Value::Null)
    };
    (status, cookie, body)
}

async fn register(app: &Router, email: &str) -> String {
    let (s, c, _) = send(
        app,
        req(
            "POST",
            "/api/auth/register",
            Some(json!({ "email": email, "password": "password123" })),
            None,
        ),
    )
    .await;
    assert_eq!(s, StatusCode::CREATED);
    c.unwrap()
}

async fn login(app: &Router, email: &str, password: &str) -> (StatusCode, Option<String>, Value) {
    send(
        app,
        req(
            "POST",
            "/api/auth/login",
            Some(json!({ "email": email, "password": password })),
            None,
        ),
    )
    .await
}

async fn uid(storage: &AnyStorage, email: &str) -> UserId {
    storage
        .users()
        .find_by_email(email)
        .await
        .unwrap()
        .unwrap()
        .id
}

/// Écrit `n` octets dans l'espace fichiers d'un compte (hors API).
fn fill(dir: &std::path::Path, user: UserId, n: usize) {
    let d = dir.join("data/users").join(user.to_string()).join("models");
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("blob.bin"), vec![7u8; n]).unwrap();
}

async fn create_project(app: &Router, cookie: &str, name: &str) -> Value {
    let (s, _, body) = send(
        app,
        req(
            "POST",
            "/api/projects",
            Some(json!({ "name": name })),
            Some(cookie),
        ),
    )
    .await;
    assert_eq!(s, StatusCode::CREATED, "{body}");
    body
}

#[tokio::test]
async fn admin_lists_accounts_paginated_with_usage() {
    let a = app().await;
    let admin = register(&a.app, "admin@test.local").await;
    let user = register(&a.app, "bob@test.local").await;
    register(&a.app, "carol@test.local").await;
    fill(
        a.dir.path(),
        uid(&a.storage, "bob@test.local").await,
        12_345,
    );
    create_project(&a.app, &user, "Bob 1").await;
    create_project(&a.app, &user, "Bob 2").await;

    let (s, _, body) = send(&a.app, req("GET", "/api/admin/users", None, Some(&admin))).await;
    assert_eq!(s, StatusCode::OK, "{body}");
    assert_eq!(body["total"], 3);
    assert_eq!(body["page"], 1);
    assert_eq!(body["per_page"], 50);
    let users = body["users"].as_array().unwrap();
    assert_eq!(users.len(), 3);
    // Ordre de création ; premier compte = admin.
    assert_eq!(users[0]["email"], "admin@test.local");
    assert_eq!(users[0]["role"], "admin");
    assert_eq!(users[1]["role"], "user");
    assert_eq!(users[1]["status"], "active");
    assert_eq!(users[1]["storage_bytes"], 12_345);
    assert_eq!(users[1]["project_count"], 2);
    assert_eq!(users[2]["storage_bytes"], 0);
    assert!(users[1]["created_at"].as_str().unwrap().contains('T'));
    assert!(users[1].get("password_hash").is_none());

    // Pagination.
    let (_, _, p2) = send(
        &a.app,
        req(
            "GET",
            "/api/admin/users?page=2&per_page=2",
            None,
            Some(&admin),
        ),
    )
    .await;
    assert_eq!(p2["total"], 3);
    assert_eq!(p2["users"].as_array().unwrap().len(), 1);
    assert_eq!(p2["users"][0]["email"], "carol@test.local");
    let (_, _, p9) = send(
        &a.app,
        req(
            "GET",
            "/api/admin/users?page=9&per_page=2",
            None,
            Some(&admin),
        ),
    )
    .await;
    assert_eq!(p9["users"].as_array().unwrap().len(), 0);

    // Filtre e-mail.
    let (_, _, q) = send(
        &a.app,
        req("GET", "/api/admin/users?q=BOB", None, Some(&admin)),
    )
    .await;
    assert_eq!(q["total"], 1);
    assert_eq!(q["users"][0]["email"], "bob@test.local");

    // Bornes.
    for bad in ["per_page=0", "per_page=201", "page=0", "page=abc"] {
        let (s, _, body) = send(
            &a.app,
            req(
                "GET",
                &format!("/api/admin/users?{bad}"),
                None,
                Some(&admin),
            ),
        )
        .await;
        assert!(
            s == StatusCode::UNPROCESSABLE_ENTITY || s == StatusCode::BAD_REQUEST,
            "{bad} → {s}"
        );
        assert!(body["code"].is_string(), "{bad} : enveloppe d'erreur");
    }

    // Réservé aux admins.
    let (s, _, body) = send(&a.app, req("GET", "/api/admin/users", None, Some(&user))).await;
    assert_eq!(s, StatusCode::FORBIDDEN);
    assert!(!body.to_string().contains("carol"));
    let (s, _, _) = send(&a.app, req("GET", "/api/admin/users", None, None)).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn login_failure_has_a_dedicated_code() {
    let a = app().await;
    register(&a.app, "admin@test.local").await;
    let (s, _, body) = login(&a.app, "admin@test.local", "wrong-password").await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
    assert_eq!(body["code"], "invalid_credentials");
    // E-mail inconnu : même réponse (aucun oracle d'existence de compte).
    let (s2, _, body2) = login(&a.app, "nobody@test.local", "wrong-password").await;
    assert_eq!(s2, StatusCode::UNAUTHORIZED);
    assert_eq!(body2, body);
    // Session absente : toujours `unauthorized`.
    let (s, _, body) = send(&a.app, req("GET", "/api/auth/me", None, None)).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
    assert_eq!(body["code"], "unauthorized");
}

#[tokio::test]
async fn change_password_revokes_other_sessions() {
    let a = app().await;
    register(&a.app, "admin@test.local").await;
    let first = register(&a.app, "bob@test.local").await;
    let (_, second, _) = login(&a.app, "bob@test.local", "password123").await;
    let second = second.unwrap();

    // Ancien mot de passe faux → 403 invalid_credentials, session conservée.
    let (s, _, body) = send(
        &a.app,
        req(
            "PUT",
            "/api/me/password",
            Some(json!({ "current_password": "nope-nope", "new_password": "brand-new-pass" })),
            Some(&first),
        ),
    )
    .await;
    assert_eq!(s, StatusCode::FORBIDDEN);
    assert_eq!(body["code"], "invalid_credentials");
    assert_eq!(body["details"]["field"], "current_password");
    // Nouveau trop court / identique → 422.
    for (new, field) in [("short", "password"), ("password123", "new_password")] {
        let (s, _, body) = send(
            &a.app,
            req(
                "PUT",
                "/api/me/password",
                Some(json!({ "current_password": "password123", "new_password": new })),
                Some(&first),
            ),
        )
        .await;
        assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY, "{new}");
        assert_eq!(body["details"]["field"], field);
    }
    let (s, _, _) = send(&a.app, req("GET", "/api/auth/me", None, Some(&second))).await;
    assert_eq!(s, StatusCode::OK, "les échecs ne révoquent rien");

    // Succès : 204, nouvel identifiant de session pour la session courante.
    let (s, renewed, _) = send(
        &a.app,
        req(
            "PUT",
            "/api/me/password",
            Some(json!({ "current_password": "password123", "new_password": "brand-new-pass" })),
            Some(&first),
        ),
    )
    .await;
    assert_eq!(s, StatusCode::NO_CONTENT);
    let renewed = renewed.expect("nouveau cookie de session");
    let (s, _, me) = send(&a.app, req("GET", "/api/auth/me", None, Some(&renewed))).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(me["email"], "bob@test.local");
    // L'autre session est révoquée.
    let (s, _, body) = send(&a.app, req("GET", "/api/auth/me", None, Some(&second))).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
    assert_eq!(body["code"], "unauthorized");
    // Ancien mot de passe refusé, nouveau accepté.
    assert_eq!(
        login(&a.app, "bob@test.local", "password123").await.0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        login(&a.app, "bob@test.local", "brand-new-pass").await.0,
        StatusCode::OK
    );
    // Sans session → 401.
    let (s, _, _) = send(
        &a.app,
        req(
            "PUT",
            "/api/me/password",
            Some(json!({ "current_password": "a", "new_password": "bbbbbbbbbb" })),
            None,
        ),
    )
    .await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn admin_reset_revokes_the_target_sessions() {
    let a = app().await;
    let admin = register(&a.app, "admin@test.local").await;
    let bob = register(&a.app, "bob@test.local").await;
    let bob_id = uid(&a.storage, "bob@test.local").await;
    let (s, _, _) = send(
        &a.app,
        req(
            "POST",
            &format!("/api/admin/users/{bob_id}/reset-password"),
            Some(json!({ "new_password": "reset-by-admin" })),
            Some(&admin),
        ),
    )
    .await;
    assert_eq!(s, StatusCode::NO_CONTENT);
    let (s, _, _) = send(&a.app, req("GET", "/api/auth/me", None, Some(&bob))).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
    let (s, _, _) = send(&a.app, req("GET", "/api/auth/me", None, Some(&admin))).await;
    assert_eq!(s, StatusCode::OK, "la session de l'admin reste valide");
}

#[tokio::test]
async fn quotas_are_configurable_and_enforced() {
    let a = app().await;
    let admin = register(&a.app, "admin@test.local").await;
    let bob = register(&a.app, "bob@test.local").await;
    let bob_id = uid(&a.storage, "bob@test.local").await;

    // Défauts : illimité.
    let (_, _, inst) = send(
        &a.app,
        req("GET", "/api/admin/instance", None, Some(&admin)),
    )
    .await;
    assert_eq!(inst["storage_quota_bytes"], 0);
    assert_eq!(inst["max_concurrent_slices"], 0);
    // Bornes.
    for bad in [
        json!({ "storage_quota_bytes": -1 }),
        json!({ "max_concurrent_slices": -1 }),
        json!({ "max_concurrent_slices": 1001 }),
    ] {
        let (s, _, body) = send(
            &a.app,
            req(
                "PATCH",
                "/api/admin/instance",
                Some(bad.clone()),
                Some(&admin),
            ),
        )
        .await;
        assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY, "{bad}");
        assert_eq!(body["code"], "validation");
    }
    let (s, _, _) = send(
        &a.app,
        req(
            "PATCH",
            "/api/admin/instance",
            Some(json!({ "max_concurrent_slices": 1 })),
            Some(&bob),
        ),
    )
    .await;
    assert_eq!(s, StatusCode::FORBIDDEN);
    let (s, _, inst) = send(
        &a.app,
        req(
            "PATCH",
            "/api/admin/instance",
            Some(json!({ "storage_quota_bytes": 10_000, "max_concurrent_slices": 1 })),
            Some(&admin),
        ),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(inst["storage_quota_bytes"], 10_000);
    assert_eq!(inst["max_concurrent_slices"], 1);
    assert_eq!(
        inst["registration_policy"], "open",
        "champs absents conservés"
    );

    // Usage du compte.
    fill(a.dir.path(), bob_id, 9_000);
    let (s, _, usage) = send(&a.app, req("GET", "/api/me/usage", None, Some(&bob))).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(usage["storage_bytes"], 9_000);
    assert_eq!(usage["storage_quota_bytes"], 10_000);
    assert_eq!(usage["active_slices"], 0);
    assert_eq!(usage["max_concurrent_slices"], 1);

    // Stockage : fichier auxiliaire de 2 000 octets → 413 explicite.
    let project = create_project(&a.app, &bob, "Quota").await;
    let pid = project["id"].as_str().unwrap();
    let put = |len: usize| {
        Request::builder()
            .method("PUT")
            .uri(format!("/api/projects/{pid}/auxiliaries/Others/a.bin"))
            .header(header::COOKIE, &bob)
            .body(Body::from(vec![1u8; len]))
            .unwrap()
    };
    let (s, _, body) = send(&a.app, put(2_000)).await;
    assert_eq!(s, StatusCode::PAYLOAD_TOO_LARGE, "{body}");
    assert_eq!(body["code"], "storage_quota_exceeded");
    assert_eq!(body["details"]["used_bytes"], 9_000);
    assert_eq!(body["details"]["quota_bytes"], 10_000);
    assert_eq!(body["details"]["incoming_bytes"], 2_000);
    let (s, _, _) = send(&a.app, put(500)).await;
    assert_eq!(s, StatusCode::CREATED, "sous le quota : accepté");

    // Upload de modèle au-delà du quota → 413 (avant toute écriture).
    let boundary = "XBOUNDARY";
    let mut mp = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"big.stl\"\r\nContent-Type: application/octet-stream\r\n\r\n"
    )
    .into_bytes();
    mp.extend(vec![0u8; 4_000]);
    mp.extend(format!("\r\n--{boundary}--\r\n").into_bytes());
    let up = Request::builder()
        .method("POST")
        .uri(format!("/api/projects/{pid}/models"))
        .header(header::COOKIE, &bob)
        .header(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(Body::from(mp))
        .unwrap();
    let (s, _, body) = send(&a.app, up).await;
    assert_eq!(s, StatusCode::PAYLOAD_TOO_LARGE, "{body}");
    assert_eq!(body["code"], "storage_quota_exceeded");

    // Tranchage, stockage plein → 413 (réservation estimée du G-code, lot 35 :
    // 128 Kio de socle par plateau, objet sans modèle → 0 triangle).
    let scene = json!({
        "schema_version": 1,
        "objects": [{ "id": "o1" }, { "id": "o2" }],
        "plates": [
            { "id": "p1", "objectIds": ["o1"] },
            { "id": "p2", "objectIds": ["o2"] }
        ]
    });
    fill(a.dir.path(), bob_id, 20_000);
    let slice = |body: Value| {
        req(
            "POST",
            &format!("/api/projects/{pid}/slice"),
            Some(body),
            Some(&bob),
        )
    };
    let (s, _, body) = send(&a.app, slice(json!({ "plate_index": 0, "scene": scene }))).await;
    assert_eq!(s, StatusCode::PAYLOAD_TOO_LARGE, "{body}");
    assert_eq!(body["code"], "storage_quota_exceeded");
    assert_eq!(
        body["details"]["incoming_bytes"],
        128 * 1024,
        "estimation G-code réservée avant création des jobs"
    );
    fill(a.dir.path(), bob_id, 10);

    // Tranchages simultanés (max 1) : 2 plateaux d'un coup → 429. Le quota de
    // stockage (10 000) couvrirait tout juste les fichiers mais pas la
    // réservation G-code (131 072) : repasse en illimité pour isoler le 429.
    let (_, _, _) = send(
        &a.app,
        req(
            "PATCH",
            "/api/admin/instance",
            Some(json!({ "storage_quota_bytes": 0 })),
            Some(&admin),
        ),
    )
    .await;
    let (s, _, body) = send(&a.app, slice(json!({ "all": true, "scene": scene }))).await;
    assert_eq!(s, StatusCode::TOO_MANY_REQUESTS, "{body}");
    assert_eq!(body["code"], "slice_quota_exceeded");
    assert_eq!(body["details"]["requested"], 2);
    assert_eq!(body["details"]["max"], 1);
    // Un plateau → 201, le suivant → 429 tant que le premier est en file.
    let (s, _, body) = send(&a.app, slice(json!({ "plate_index": 0, "scene": scene }))).await;
    assert_eq!(s, StatusCode::CREATED, "{body}");
    let job = body["jobs"][0]["id"].as_str().unwrap().to_string();
    let (s, _, body) = send(&a.app, slice(json!({ "plate_index": 1, "scene": scene }))).await;
    assert_eq!(s, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(body["details"]["active"], 1);
    let (_, _, usage) = send(&a.app, req("GET", "/api/me/usage", None, Some(&bob))).await;
    assert_eq!(usage["active_slices"], 1);
    // Job annulé → place libérée.
    let (s, _, _) = send(
        &a.app,
        req(
            "POST",
            &format!("/api/jobs/{job}/cancel"),
            Some(json!({})),
            Some(&bob),
        ),
    )
    .await;
    assert!(s.is_success());
    let (s, _, body) = send(&a.app, slice(json!({ "plate_index": 1, "scene": scene }))).await;
    assert_eq!(s, StatusCode::CREATED, "{body}");

    // Retour à l'illimité.
    let (_, _, inst) = send(
        &a.app,
        req(
            "PATCH",
            "/api/admin/instance",
            Some(json!({ "storage_quota_bytes": 0, "max_concurrent_slices": 0 })),
            Some(&admin),
        ),
    )
    .await;
    assert_eq!(inst["max_concurrent_slices"], 0);
    let (s, _, _) = send(&a.app, slice(json!({ "all": true, "scene": scene }))).await;
    assert_eq!(s, StatusCode::CREATED);
}

#[tokio::test]
async fn duplicate_project_names_are_resolved_by_the_server() {
    let a = app().await;
    let bob = register(&a.app, "bob@test.local").await;
    let p1 = create_project(&a.app, &bob, "Sans titre").await;
    assert_eq!(p1["name"], "Sans titre");
    let p2 = create_project(&a.app, &bob, "Sans titre").await;
    assert_eq!(p2["name"], "Sans titre (2)");
    let p3 = create_project(&a.app, &bob, "Sans titre").await;
    assert_eq!(p3["name"], "Sans titre (3)");
    // Nom déjà suffixé et pris → suffixe suivant, pas « (2) (2) ».
    let p4 = create_project(&a.app, &bob, "Sans titre (2)").await;
    assert_eq!(p4["name"], "Sans titre (4)");

    // Duplication répétée.
    let id1 = p1["id"].as_str().unwrap();
    let dup = |id: &str| {
        req(
            "POST",
            &format!("/api/projects/{id}/duplicate"),
            Some(json!({})),
            Some(&bob),
        )
    };
    let (s, _, d1) = send(&a.app, dup(id1)).await;
    assert_eq!(s, StatusCode::CREATED);
    assert_eq!(d1["name"], "Sans titre (copie)");
    let (_, _, d2) = send(&a.app, dup(id1)).await;
    assert_eq!(d2["name"], "Sans titre (copie) (2)");

    // Renommage vers un nom pris → 409 name_taken + suggestion, jamais de SQL.
    let id3 = p3["id"].as_str().unwrap();
    for (method, uri) in [
        ("PATCH", format!("/api/projects/{id3}/rename")),
        ("PATCH", format!("/api/projects/{id3}")),
    ] {
        let (s, _, body) = send(
            &a.app,
            req(
                method,
                &uri,
                Some(json!({ "name": "Sans titre" })),
                Some(&bob),
            ),
        )
        .await;
        assert_eq!(s, StatusCode::CONFLICT, "{uri}");
        assert_eq!(body["code"], "name_taken");
        assert_eq!(body["details"]["field"], "name");
        assert_eq!(body["details"]["suggestion"], "Sans titre (5)");
        let text = body.to_string();
        assert!(
            !text.contains("UNIQUE") && !text.contains("constraint"),
            "{text}"
        );
    }
    // Renommer vers son propre nom : accepté.
    let (s, _, body) = send(
        &a.app,
        req(
            "PATCH",
            &format!("/api/projects/{id3}/rename"),
            Some(json!({ "name": "Sans titre (3)" })),
            Some(&bob),
        ),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(body["name"], "Sans titre (3)");

    // Les noms sont propres à chaque compte.
    let carol = register(&a.app, "carol@test.local").await;
    let c = create_project(&a.app, &carol, "Sans titre").await;
    assert_eq!(c["name"], "Sans titre");
}

/// Lot 35 — `PUT /api/admin/users/{id}/status` : désactivation (sessions
/// ouvertes invalidées à la requête suivante, login refusé) puis réactivation ;
/// le dernier administrateur actif ne peut pas être désactivé.
#[tokio::test]
async fn admin_can_disable_and_reenable_accounts() {
    let a = app().await;
    let admin = register(&a.app, "admin@test.local").await;
    let bob = register(&a.app, "bob@test.local").await;
    let bob_id = uid(&a.storage, "bob@test.local").await;
    let admin_id = uid(&a.storage, "admin@test.local").await;
    let status = |id: &UserId, value: &str| {
        req(
            "PUT",
            &format!("/api/admin/users/{id}/status"),
            Some(json!({ "status": value })),
            Some(&admin),
        )
    };

    // Statut visible dans la liste admin.
    let (s, _, _) = send(&a.app, req("GET", "/api/me/usage", None, Some(&bob))).await;
    assert_eq!(s, StatusCode::OK);

    // Valeur invalide → 422.
    let (s, _, body) = send(&a.app, status(&bob_id, "paused")).await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(body["details"]["field"], "status");

    // Non-admin → 403.
    let (s, _, _) = send(
        &a.app,
        req(
            "PUT",
            &format!("/api/admin/users/{admin_id}/status"),
            Some(json!({ "status": "disabled" })),
            Some(&bob),
        ),
    )
    .await;
    assert_eq!(s, StatusCode::FORBIDDEN);

    // Désactivation : 204, session de bob → 401, login → 403.
    let (s, _, body) = send(&a.app, status(&bob_id, "disabled")).await;
    assert_eq!(s, StatusCode::NO_CONTENT, "{body}");
    let (s, _, _) = send(&a.app, req("GET", "/api/me/usage", None, Some(&bob))).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED, "session invalidée aussitôt");
    let (s, _, body) = login(&a.app, "bob@test.local", "password123").await;
    assert_eq!(s, StatusCode::FORBIDDEN, "{body}");
    assert_eq!(body["code"], "forbidden");
    assert_eq!(body["message"], "compte désactivé");

    // La liste admin reflète le statut.
    let (_, _, list) = send(
        &a.app,
        req("GET", "/api/admin/users?q=bob", None, Some(&admin)),
    )
    .await;
    assert_eq!(list["users"][0]["status"], "disabled");

    // Réactivation : login à nouveau possible.
    let (s, _, _) = send(&a.app, status(&bob_id, "active")).await;
    assert_eq!(s, StatusCode::NO_CONTENT);
    let (s, _, _) = login(&a.app, "bob@test.local", "password123").await;
    assert_eq!(s, StatusCode::OK);

    // Dernier administrateur actif → 403 (même garde que la suppression).
    let (s, _, body) = send(&a.app, status(&admin_id, "disabled")).await;
    assert_eq!(s, StatusCode::FORBIDDEN, "{body}");
    // Idempotent : désactiver un compte déjà désactivé reste sans effet.
    let (s, _, _) = send(&a.app, status(&bob_id, "disabled")).await;
    assert_eq!(s, StatusCode::NO_CONTENT);
}

/// Lot 35 — le 429 de quota de tranchage porte `Retry-After` (30 s) et
/// `details.retry_after`.
#[tokio::test]
async fn slice_quota_429_carries_retry_after() {
    let a = app().await;
    let admin = register(&a.app, "admin@test.local").await;
    let bob = register(&a.app, "bob@test.local").await;
    let (s, _, _) = send(
        &a.app,
        req(
            "PATCH",
            "/api/admin/instance",
            Some(json!({ "max_concurrent_slices": 1 })),
            Some(&admin),
        ),
    )
    .await;
    assert_eq!(s, StatusCode::OK);

    let project = create_project(&a.app, &bob, "Retry").await;
    let pid = project["id"].as_str().unwrap();
    let scene = json!({
        "schema_version": 1,
        "objects": [{ "id": "o1" }, { "id": "o2" }],
        "plates": [
            { "id": "p1", "objectIds": ["o1"] },
            { "id": "p2", "objectIds": ["o2"] }
        ]
    });
    let slice = |body: Value| {
        req(
            "POST",
            &format!("/api/projects/{pid}/slice"),
            Some(body),
            Some(&bob),
        )
    };
    let (s, _, _) = send(&a.app, slice(json!({ "plate_index": 0, "scene": scene }))).await;
    assert_eq!(s, StatusCode::CREATED);

    let resp: Response = a
        .app
        .clone()
        .oneshot(slice(json!({ "plate_index": 1, "scene": scene })))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(
        resp.headers()
            .get(header::RETRY_AFTER)
            .and_then(|v| v.to_str().ok()),
        Some("30"),
        "Retry-After en secondes"
    );
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["details"]["retry_after"], 30);
}
