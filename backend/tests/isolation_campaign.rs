//! Campagne d'isolation automatisée (T083, SC-008 à 100 %) : pour **chaque**
//! ressource adressable par identifiant, un compte tiers (Bob) qui vise
//! directement la ressource d'un autre (Alice) reçoit **404** — jamais 403, ni
//! aucune fuite. Couvre projets, modèles, jobs, G-codes, presets et imprimantes,
//! ainsi que les identifiants inconnus/malformés (traités comme inexistants).

use std::sync::Arc;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use axum::response::Response;
use axum::Router;
use backend::adapters::files::FileStore;
use backend::adapters::storage::AnyStorage;
use backend::auth::SecretBox;
use backend::domain::repo::{NewGcode, NewJob, NewModel, NewPrinter, NewProject};
use backend::domain::{ModelFormat, ModelId, Preset, PresetKind, PresetOrigin, Storage, UserId};
use serde_json::{json, Value};
use tower::ServiceExt;
use tower_sessions::{MemoryStore, SessionManagerLayer};

struct Harness {
    _dir: tempfile::TempDir,
    storage: Arc<AnyStorage>,
    files: FileStore,
    app: Router,
}

async fn harness() -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}", dir.path().join("isolation.db").display());
    let storage = Arc::new(AnyStorage::connect_test(&url).await.unwrap());
    let files = FileStore::new(dir.path().join("data"));
    let state = backend::http::state::AppState::new(
        Arc::clone(&storage) as Arc<dyn Storage>,
        files.clone(),
    )
    .with_secrets(SecretBox::new([3u8; 32]));
    let session_layer = SessionManagerLayer::new(MemoryStore::default()).with_secure(false);
    Harness {
        _dir: dir,
        storage,
        files,
        app: backend::http::routes::router(state, session_layer),
    }
}

fn cookie(resp: &Response) -> String {
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

async fn register(app: &Router, email: &str) -> String {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/register")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({ "email": email, "password": "password123" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    cookie(&resp)
}

async fn user_id(storage: &AnyStorage, email: &str) -> UserId {
    storage
        .users()
        .find_by_email(email)
        .await
        .unwrap()
        .unwrap()
        .id
}

/// Ressources d'Alice, toutes adressables par identifiant.
struct AliceResources {
    project: String,
    model: String,
    job: String,
    gcode: String,
    preset: String,
    printer: String,
}

async fn seed_alice(h: &Harness, owner: UserId) -> AliceResources {
    let project = h
        .storage
        .projects()
        .create(
            owner,
            NewProject {
                name: "Alice project".into(),
                scene: json!({}),
                active_presets: json!({}),
                thumbnail_path: None,
            },
        )
        .await
        .unwrap();

    let model = h
        .storage
        .models()
        .create(
            owner,
            NewModel {
                project_id: Some(project.id),
                filename: "part.stl".into(),
                format: ModelFormat::Stl,
                file_path: "/dev/null".into(),
                mesh_path: None,
                size_bytes: 1,
                triangle_count: 1,
                repair_report: None,
            },
        )
        .await
        .unwrap();

    let job = h
        .storage
        .jobs()
        .enqueue(
            owner,
            NewJob {
                project_id: project.id,
                plate_index: 0,
                resolved_settings: json!({}),
            },
        )
        .await
        .unwrap();
    // G-code avec un fichier réel (les endpoints qui lisent le fichier doivent
    // tout de même renvoyer 404 avant toute lecture pour un autre compte).
    let path = h
        .files
        .write_model(owner, ModelId::new(), "gcode", b"; g\nG28\n")
        .await
        .unwrap();
    let gcode = h
        .storage
        .gcodes()
        .create(
            owner,
            NewGcode {
                job_id: job.id,
                file_path: path.to_string_lossy().into_owned(),
                preview_path: String::new(),
                stats: json!({}),
                thumbnails: json!([]),
            },
        )
        .await
        .unwrap();

    let preset = h
        .storage
        .presets()
        .create_user_preset(
            owner,
            Preset {
                id: backend::domain::PresetId::new(),
                kind: PresetKind::Machine,
                name: "Alice machine".into(),
                origin: PresetOrigin::User,
                user_id: Some(owner),
                vendor: None,
                inherits: None,
                instantiation: true,
                setting_id: None,
                filament_id: None,
                compatible_printers: None,
                renamed_from: Vec::new(),
                values: json!({}),
            },
        )
        .await
        .unwrap();

    let printer = h
        .storage
        .printers()
        .create(
            owner,
            NewPrinter {
                name: "Alice printer".into(),
                moonraker_url: Some("http://alice.local".into()),
                api_key: None,
                machine_preset_id: preset.id,
            },
        )
        .await
        .unwrap();

    AliceResources {
        project: project.id.to_string(),
        model: model.id.to_string(),
        job: job.id.to_string(),
        gcode: gcode.id.to_string(),
        preset: preset.id.to_string(),
        printer: printer.id.to_string(),
    }
}

async fn attempt(
    app: &Router,
    method: &str,
    uri: &str,
    session: &str,
    body: Option<Value>,
) -> Response {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::COOKIE, session);
    let body = match body {
        Some(v) => {
            builder = builder.header(header::CONTENT_TYPE, "application/json");
            Body::from(v.to_string())
        }
        None => Body::empty(),
    };
    app.clone()
        .oneshot(builder.body(body).unwrap())
        .await
        .unwrap()
}

#[tokio::test]
async fn every_cross_account_resource_access_is_404_never_403() {
    let h = harness().await;
    register(&h.app, "alice@test.local").await;
    let alice = user_id(&h.storage, "alice@test.local").await;
    let r = seed_alice(&h, alice).await;

    let bob = register(&h.app, "bob@test.local").await;
    let random = uuid::Uuid::new_v4().to_string();

    // (méthode, URI, corps JSON valide éventuel) — chaque appel vise une
    // ressource d'Alice depuis le compte de Bob.
    let save_body = json!({ "expected_version": 0, "scene": {}, "active_presets": {} });
    let arrange_body =
        json!({ "bed_width": 200.0, "bed_depth": 200.0, "spacing": 2.0, "items": [] });
    let cases: Vec<(&str, String, Option<Value>)> = vec![
        // Projets et sous-ressources projet.
        ("GET", format!("/api/projects/{}", r.project), None),
        (
            "PUT",
            format!("/api/projects/{}", r.project),
            Some(save_body.clone()),
        ),
        ("DELETE", format!("/api/projects/{}", r.project), None),
        (
            "POST",
            format!("/api/projects/{}/duplicate", r.project),
            None,
        ),
        (
            "PATCH",
            format!("/api/projects/{}/rename", r.project),
            Some(json!({ "name": "x" })),
        ),
        (
            "GET",
            format!("/api/projects/{}/thumbnail", r.project),
            None,
        ),
        (
            "POST",
            format!("/api/projects/{}/arrange", r.project),
            Some(arrange_body),
        ),
        (
            "POST",
            format!("/api/projects/{}/orient", r.project),
            Some(json!({ "model_id": r.model })),
        ),
        (
            "POST",
            format!("/api/projects/{}/slice", r.project),
            Some(json!({ "all": true })),
        ),
        (
            "GET",
            format!("/api/projects/{}/export/3mf", r.project),
            None,
        ),
        // Modèles.
        ("GET", format!("/api/models/{}/mesh", r.model), None),
        ("POST", format!("/api/models/{}/repair", r.model), None),
        // Jobs.
        ("GET", format!("/api/jobs/{}", r.job), None),
        ("POST", format!("/api/jobs/{}/cancel", r.job), None),
        // G-codes.
        ("GET", format!("/api/gcodes/{}/download", r.gcode), None),
        ("GET", format!("/api/gcodes/{}/stats", r.gcode), None),
        ("GET", format!("/api/gcodes/{}/preview/meta", r.gcode), None),
        (
            "GET",
            format!("/api/gcodes/{}/preview/layers", r.gcode),
            None,
        ),
        // Presets.
        ("GET", format!("/api/presets/{}", r.preset), None),
        (
            "PUT",
            format!("/api/presets/{}", r.preset),
            Some(json!({ "name": "x" })),
        ),
        ("DELETE", format!("/api/presets/{}", r.preset), None),
        ("GET", format!("/api/presets/{}/resolved", r.preset), None),
        ("GET", format!("/api/presets/{}/export", r.preset), None),
        // Imprimantes.
        ("GET", format!("/api/printers/{}", r.printer), None),
        (
            "PUT",
            format!("/api/printers/{}", r.printer),
            Some(
                json!({ "name": "x", "moonraker_url": "http://h", "api_key": null, "machine_preset_id": random }),
            ),
        ),
        ("DELETE", format!("/api/printers/{}", r.printer), None),
        ("POST", format!("/api/printers/{}/test", r.printer), None),
        (
            "POST",
            format!("/api/printers/{}/upload", r.printer),
            Some(json!({ "gcode_id": r.gcode, "start_now": false })),
        ),
        ("GET", format!("/api/printers/{}/status", r.printer), None),
        ("POST", format!("/api/printers/{}/pause", r.printer), None),
        ("POST", format!("/api/printers/{}/resume", r.printer), None),
        ("POST", format!("/api/printers/{}/cancel", r.printer), None),
    ];

    for (method, uri, body) in &cases {
        let resp = attempt(&h.app, method, uri, &bob, body.clone()).await;
        assert_eq!(
            resp.status(),
            StatusCode::NOT_FOUND,
            "{method} {uri} devrait renvoyer 404 pour un autre compte (SC-008)"
        );
        assert_ne!(
            resp.status(),
            StatusCode::FORBIDDEN,
            "{method} {uri} ne doit jamais révéler l'existence via 403 (SC-008)"
        );
    }

    // Identifiants inconnus (bien formés mais inexistants) : mêmes 404.
    for (method, uri) in [
        ("GET", format!("/api/projects/{random}")),
        ("GET", format!("/api/models/{random}/mesh")),
        ("GET", format!("/api/jobs/{random}")),
        ("GET", format!("/api/gcodes/{random}/stats")),
        ("GET", format!("/api/presets/{random}")),
        ("GET", format!("/api/printers/{random}")),
    ] {
        let resp = attempt(&h.app, method, &uri, &bob, None).await;
        assert_eq!(
            resp.status(),
            StatusCode::NOT_FOUND,
            "{method} {uri} (inconnu)"
        );
    }

    // Identifiants malformés (pas des UUID) : 404 également, jamais 500.
    for (method, uri) in [
        ("GET", "/api/projects/not-a-uuid"),
        ("GET", "/api/gcodes/xyz/stats"),
        ("GET", "/api/printers/%20/status"),
    ] {
        let resp = attempt(&h.app, method, uri, &bob, None).await;
        assert_eq!(
            resp.status(),
            StatusCode::NOT_FOUND,
            "{method} {uri} (malformé)"
        );
    }

    // Contrôle positif : Alice, elle, accède bien à sa ressource (pas de faux 404).
    let alice_session = register_existing(&h.app, "alice@test.local").await;
    let ok = attempt(
        &h.app,
        "GET",
        &format!("/api/projects/{}", r.project),
        &alice_session,
        None,
    )
    .await;
    assert_eq!(ok.status(), StatusCode::OK, "le propriétaire garde l'accès");
}

/// Reconnecte un compte existant (login) et renvoie son cookie de session.
async fn register_existing(app: &Router, email: &str) -> String {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({ "email": email, "password": "password123" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    cookie(&resp)
}

// --- Parcours automatique de toutes les routes par identifiant ---------------

/// Routes déclarées dans le routeur, lues depuis sa source : (méthode, chemin).
/// Lire la source plutôt qu'une liste figée garantit qu'une route ajoutée plus
/// tard est parcourue (ou fait échouer le test si son identifiant est inconnu).
fn declared_routes() -> Vec<(String, String)> {
    let source = include_str!("../src/http/routes/mod.rs");
    let method_re = regex::Regex::new(r"(?:^|[^A-Za-z0-9_])(get|post|put|delete|patch)\(").unwrap();
    let path_re = regex::Regex::new(r#""(/api/[^"]*)""#).unwrap();
    let mut routes = Vec::new();
    let mut rest = source;
    while let Some(start) = rest.find(".route(") {
        let after = &rest[start + ".route(".len()..];
        // Fin de l'appel : parenthèse fermante au niveau 0 (hors chaînes).
        let (mut depth, mut in_str, mut end) = (1usize, false, after.len());
        for (i, c) in after.char_indices() {
            match c {
                '"' => in_str = !in_str,
                '(' if !in_str => depth += 1,
                ')' if !in_str => {
                    depth -= 1;
                    if depth == 0 {
                        end = i;
                        break;
                    }
                }
                _ => {}
            }
        }
        let call = &after[..end];
        if let Some(path) = path_re.captures(call).map(|c| c[1].to_string()) {
            let handlers = &call[call.find(&path).unwrap() + path.len()..];
            for m in method_re.captures_iter(handlers) {
                routes.push((m[1].to_uppercase(), path.clone()));
            }
        }
        rest = &after[end..];
    }
    routes
}

#[test]
fn router_source_parsing_finds_the_known_routes() {
    let routes = declared_routes();
    for expected in [
        ("GET", "/api/projects/{id}"),
        ("DELETE", "/api/admin/users/{id}"),
        ("PUT", "/api/calibrations/pa/{id}"),
        ("DELETE", "/api/projects/{id}/auxiliaries/{folder}/{name}"),
        ("PATCH", "/api/projects/{id}/rename"),
        ("POST", "/api/printers/{id}/objects/exclude"),
        ("PATCH", "/api/projects/{id}"),
        ("POST", "/api/projects/{id}/validate"),
        ("GET", "/api/projects/{id}/thumbnail"),
        ("POST", "/api/models/{id}/cut"),
        ("PUT", "/api/projects/{id}/plates/{index}/thumbnail"),
        ("DELETE", "/api/projects/{id}/plates/{index}/thumbnail"),
    ] {
        assert!(
            routes
                .iter()
                .any(|(m, p)| m == expected.0 && p == expected.1),
            "route {expected:?} absente de l'analyse : {routes:?}"
        );
    }
    assert!(routes.len() > 100, "{} routes seulement", routes.len());
}

/// Isolation sur **toutes** les routes à identifiant du routeur : Bob visant
/// une ressource d'Alice obtient exactement la même réponse que pour un
/// identifiant inexistant (aucun oracle d'existence), jamais un succès ni un
/// 403 (hors routes d'administration, réservées au rôle admin), aucune donnée
/// d'Alice dans le corps ; les ressources d'Alice sont intactes à la fin.
#[tokio::test]
async fn every_id_route_of_the_router_is_isolated_between_accounts() {
    let h = harness().await;
    let alice_session = register(&h.app, "alice@test.local").await;
    let alice = user_id(&h.storage, "alice@test.local").await;
    let r = seed_alice(&h, alice).await;
    let bob = register(&h.app, "bob@test.local").await;

    // Ressources créées par l'API : calibration PA et fichier auxiliaire.
    let pa = attempt(
        &h.app,
        "POST",
        "/api/calibrations/pa",
        &alice_session,
        Some(json!({
            "machine_preset": "Voron 2.4 350 0.4 nozzle",
            "filament_preset": "Alice PLA",
            "filament_id": "GFL99",
            "nozzle_diameter": 0.4,
            "k_value": 0.04,
            "method": "pa_pattern"
        })),
    )
    .await;
    assert_eq!(pa.status(), StatusCode::CREATED);
    let pa: Value = serde_json::from_slice(
        &axum::body::to_bytes(pa.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    let pa = pa["id"].as_str().unwrap().to_string();
    let aux = h
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!(
                    "/api/projects/{}/auxiliaries/Others/mien.txt",
                    r.project
                ))
                .header(header::COOKIE, &alice_session)
                .body(Body::from("secret d'Alice"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(aux.status(), StatusCode::CREATED);

    // Lot 31 : G-code externe ouvert par Alice (jeton = empreinte du contenu).
    let opened = "0123456789abcdef0123456789abcdef".to_string();
    h.files
        .write_opened_gcode(alice, &opened, b"; secret d'Alice\nG1 X1\n", 4)
        .await
        .unwrap();

    let alice_id = alice.to_string();
    let substitute = |path: &str, id: &str| -> String {
        path.replace("{id}", id)
            .replace("{token}", id)
            .replace("{folder}", "Others")
            .replace("{name}", "mien.txt")
            .replace("{key}", "orca-cube")
            .replace("{index}", "0")
    };
    let owner_id = |path: &str| -> &str {
        let prefixes: [(&str, &str); 9] = [
            ("/api/gcode/opened/{token}", opened.as_str()),
            ("/api/projects/{id}", r.project.as_str()),
            ("/api/models/{id}", r.model.as_str()),
            ("/api/jobs/{id}", r.job.as_str()),
            ("/api/gcodes/{id}", r.gcode.as_str()),
            ("/api/presets/{id}", r.preset.as_str()),
            ("/api/printers/{id}", r.printer.as_str()),
            ("/api/calibrations/pa/{id}", pa.as_str()),
            ("/api/admin/users/{id}", alice_id.as_str()),
        ];
        prefixes
            .iter()
            .find(|(prefix, _)| path.starts_with(prefix))
            .map(|(_, id)| *id)
            .unwrap_or_else(|| {
                panic!("route {path} : identifiant non couvert par la campagne, l'ajouter ici")
            })
    };

    let routes: Vec<(String, String)> = declared_routes()
        .into_iter()
        .filter(|(_, path)| path.contains('{'))
        .collect();
    assert!(routes.len() >= 70, "{} routes à identifiant", routes.len());
    // Corps valides pour les routes qui valident la forme du corps avant le
    // propriétaire : avec eux, la réponse attendue est strictement 404.
    let valid_body = |method: &str, path: &str| -> Option<Value> {
        Some(match (method, path) {
            ("PUT", "/api/projects/{id}") => {
                json!({ "expected_version": 0, "scene": {}, "active_presets": {} })
            }
            ("PATCH", "/api/projects/{id}/rename") => json!({ "name": "pwned" }),
            // Lots 25-27 : mise à jour partielle, validation, découpe, vignette
            // de plateau (le propriétaire est vérifié avant le contenu PNG).
            ("PATCH", "/api/projects/{id}") => json!({ "name": "pwned" }),
            ("POST", "/api/projects/{id}/validate") => json!({}),
            ("POST", "/api/models/{id}/cut") => {
                json!({ "plane": { "point": [0.0, 0.0, 2.5], "normal": [0.0, 0.0, 1.0] } })
            }
            ("PUT", "/api/projects/{id}/plates/{index}/thumbnail") => json!({}),
            ("POST", "/api/models/{id}/remap-paint") => json!({ "channels": {} }),
            ("POST", "/api/projects/{id}/emboss-svg") => {
                json!({ "svg": "<svg xmlns='http://www.w3.org/2000/svg'/>", "width_mm": 10.0, "depth_mm": 1.0 })
            }
            ("POST", "/api/projects/{id}/export/sliced") => json!({ "job_ids": [r.job] }),
            ("POST", "/api/printers/{id}/upload-sliced") => {
                json!({ "project_id": r.project, "job_ids": [r.job], "start_now": false })
            }
            ("POST", "/api/printers/{id}/objects/exclude") => json!({ "name": "part" }),
            ("POST", "/api/printers/{id}/upload") => {
                json!({ "gcode_id": r.gcode, "start_now": false })
            }
            ("PUT", "/api/calibrations/pa/{id}") => json!({
                "machine_preset": "Voron 2.4 350 0.4 nozzle",
                "filament_preset": "Bob PLA",
                "nozzle_diameter": 0.4,
                "k_value": 0.09,
                "method": "manual"
            }),
            // Lot 34 : options d'arrangement d'Orca (ArrangeJob multi-plateaux).
            ("POST", "/api/projects/{id}/arrange") => json!({
                "bed_width": 200.0, "bed_depth": 200.0, "spacing": 2.0, "items": [],
                "options": { "spacing": 0.0, "enable_rotation": true }, "scope": "all"
            }),
            // Lot 34 : métadonnées 3MF (corps valide → 404 strict).
            ("PUT", "/api/projects/{id}/metadata") => json!({ "title": "pwned", "license": "CC0" }),
            _ => return None,
        })
    };
    let mut report = Vec::new();
    let mut strict = 0;
    for (method, path) in &routes {
        let target = substitute(path, owner_id(path));
        let unknown = substitute(path, &uuid::Uuid::new_v4().to_string());
        let valid = valid_body(method, path);
        if valid.is_some() {
            strict += 1;
        }
        let body = valid
            .clone()
            .or_else(|| matches!(method.as_str(), "POST" | "PUT" | "PATCH").then(|| json!({})));
        let on_alice = attempt(&h.app, method, &target, &bob, body.clone()).await;
        let status = on_alice.status();
        let text = String::from_utf8_lossy(
            &axum::body::to_bytes(on_alice.into_body(), usize::MAX)
                .await
                .unwrap(),
        )
        .into_owned();
        let on_unknown = attempt(&h.app, method, &unknown, &bob, body).await.status();
        report.push(format!("{method} {path} → {status}"));
        if valid.is_some() {
            assert_eq!(
                status,
                StatusCode::NOT_FOUND,
                "{method} {path} : corps valide, 404 attendu pour la ressource d'un autre compte"
            );
        }
        assert_eq!(
            status, on_unknown,
            "{method} {path} : réponse différente pour une ressource d'Alice ({status}) et un identifiant inexistant ({on_unknown}) — oracle d'existence"
        );
        assert!(
            !status.is_success(),
            "{method} {path} : Bob obtient {status} sur une ressource d'Alice"
        );
        if !path.starts_with("/api/admin/") {
            assert_ne!(
                status,
                StatusCode::FORBIDDEN,
                "{method} {path} : 403 (SC-008)"
            );
        }
        for secret in [
            "Alice project",
            "Alice machine",
            "Alice printer",
            "alice@test.local",
            "secret d'Alice",
            "Alice PLA",
        ] {
            assert!(
                !text.contains(secret),
                "{method} {path} : le corps révèle « {secret} » : {text}"
            );
        }
    }
    eprintln!(
        "{} routes à identifiant parcourues ({strict} avec corps valide) :\n{}",
        routes.len(),
        report.join("\n")
    );

    // Rien n'a été modifié ni supprimé chez Alice.
    for uri in [
        format!("/api/projects/{}", r.project),
        format!("/api/presets/{}", r.preset),
        format!("/api/printers/{}", r.printer),
        format!("/api/calibrations/pa/{pa}"),
        format!("/api/gcodes/{}/stats", r.gcode),
        format!("/api/jobs/{}", r.job),
        format!("/api/projects/{}/auxiliaries/Others/mien.txt", r.project),
        format!("/api/projects/{}/metadata", r.project),
    ] {
        let resp = attempt(&h.app, "GET", &uri, &alice_session, None).await;
        assert_eq!(resp.status(), StatusCode::OK, "Alice garde {uri}");
    }
    let project = h
        .storage
        .projects()
        .get(alice, uuid::Uuid::parse_str(&r.project).unwrap().into())
        .await
        .unwrap();
    assert_eq!(project.name, "Alice project", "projet ni renommé ni écrasé");
    assert!(h
        .storage
        .models()
        .get(alice, uuid::Uuid::parse_str(&r.model).unwrap().into())
        .await
        .is_ok());
    assert_eq!(
        h.storage.users().count().await.unwrap(),
        2,
        "aucun compte supprimé"
    );
}

/// Lot 32 — routes de compte et quotas, vues d'un compte tiers : la liste des
/// comptes et les réglages de quotas sont réservés à l'admin (403 sans aucune
/// donnée), l'usage et le changement de mot de passe ne portent que sur le
/// compte connecté, les quotas se comptent **par compte** (l'espace ou les
/// tranchages d'Alice ne bloquent jamais Bob) et l'unicité des noms de projet
/// ne révèle pas les noms d'un autre compte.
#[tokio::test]
async fn lot32_account_routes_and_quotas_are_scoped_per_account() {
    let h = harness().await;
    let alice_session = register(&h.app, "alice@test.local").await; // 1er = admin
    let alice = user_id(&h.storage, "alice@test.local").await;
    let _r = seed_alice(&h, alice).await; // projet « Alice project » + 1 job en file
    let bob = register(&h.app, "bob@test.local").await;
    let bob_id = user_id(&h.storage, "bob@test.local").await;

    // Espace d'Alice : 50 000 octets.
    h.files
        .write_opened_gcode(
            alice,
            "0123456789abcdef0123456789abcdef",
            &[b'G'; 50_000],
            4,
        )
        .await
        .unwrap();

    let read = |resp: Response| async move {
        let status = resp.status();
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        (status, String::from_utf8_lossy(&bytes).into_owned())
    };

    // Routes admin : 403 pour Bob, sans aucune donnée d'Alice.
    for (method, uri, body) in [
        ("GET", "/api/admin/users", None),
        ("GET", "/api/admin/users?q=alice", None),
        ("GET", "/api/admin/instance", None),
        (
            "PATCH",
            "/api/admin/instance",
            Some(json!({ "storage_quota_bytes": 1, "max_concurrent_slices": 1 })),
        ),
    ] {
        let (status, text) = read(attempt(&h.app, method, uri, &bob, body).await).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{method} {uri}");
        assert!(
            !text.contains("alice@test.local"),
            "{method} {uri} : {text}"
        );
    }

    // Usage : Bob ne voit que le sien.
    let (status, text) = read(attempt(&h.app, "GET", "/api/me/usage", &bob, None).await).await;
    assert_eq!(status, StatusCode::OK);
    let usage: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(
        usage["storage_bytes"], 0,
        "espace d'Alice non compté chez Bob"
    );
    assert_eq!(usage["active_slices"], 0, "job d'Alice non compté chez Bob");

    // Quotas posés par l'admin (Alice) : 10 000 octets, 1 tranchage.
    let resp = attempt(
        &h.app,
        "PATCH",
        "/api/admin/instance",
        &alice_session,
        Some(json!({ "storage_quota_bytes": 10_000, "max_concurrent_slices": 1 })),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Nom de projet identique à celui d'Alice : accepté tel quel chez Bob
    // (aucun suffixe = aucun oracle sur les noms d'autrui).
    let (status, text) = read(
        attempt(
            &h.app,
            "POST",
            "/api/projects",
            &bob,
            Some(json!({ "name": "Alice project" })),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let bob_project: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(bob_project["name"], "Alice project");
    let bob_pid = bob_project["id"].as_str().unwrap().to_string();

    // Stockage : Alice (50 000 > 10 000) est bloquée, Bob (vide) ne l'est pas.
    let put = |session: &str, project: &str| {
        Request::builder()
            .method("PUT")
            .uri(format!("/api/projects/{project}/auxiliaries/Others/f.txt"))
            .header(header::COOKIE, session)
            .body(Body::from(vec![b'x'; 100]))
            .unwrap()
    };
    let alice_pid = h
        .storage
        .projects()
        .list(alice)
        .await
        .unwrap()
        .remove(0)
        .id
        .to_string();
    let resp = h
        .app
        .clone()
        .oneshot(put(&alice_session, &alice_pid))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::PAYLOAD_TOO_LARGE);
    let resp = h.app.clone().oneshot(put(&bob, &bob_pid)).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED, "quota de Bob intact");

    // Tranchages : le job en file d'Alice ne consomme pas la place de Bob.
    // (Quota de stockage relâché : la réservation G-code estimée — 128 Kio
    // par plateau, lot 35 — n'est pas l'objet de cette vérification.)
    let resp = attempt(
        &h.app,
        "PATCH",
        "/api/admin/instance",
        &alice_session,
        Some(json!({ "storage_quota_bytes": 0, "max_concurrent_slices": 1 })),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let scene = json!({
        "schema_version": 1,
        "objects": [{ "id": "o1" }],
        "plates": [{ "id": "p1", "objectIds": ["o1"] }]
    });
    let (status, text) = read(
        attempt(
            &h.app,
            "POST",
            &format!("/api/projects/{bob_pid}/slice"),
            &bob,
            Some(json!({ "plate_index": 0, "scene": scene })),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{text}");
    let (status, text) = read(
        attempt(
            &h.app,
            "POST",
            &format!("/api/projects/{bob_pid}/slice"),
            &bob,
            Some(json!({ "plate_index": 0, "scene": scene })),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert!(!text.contains("Alice"), "{text}");
    let (_, text) = read(attempt(&h.app, "GET", "/api/me/usage", &bob, None).await).await;
    let usage: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(usage["active_slices"], 1, "seul le job de Bob compte");

    // Mot de passe : Bob ne change que le sien ; la session d'Alice survit.
    let resp = attempt(
        &h.app,
        "PUT",
        "/api/me/password",
        &bob,
        Some(json!({ "current_password": "password123", "new_password": "bob-new-pass" })),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    let resp = attempt(&h.app, "GET", "/api/auth/me", &alice_session, None).await;
    assert_eq!(resp.status(), StatusCode::OK, "session d'Alice intacte");
    register_existing(&h.app, "alice@test.local").await; // mot de passe d'Alice inchangé
    let _ = bob_id;
}
