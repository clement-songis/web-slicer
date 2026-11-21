//! Lot 42 (audit 41) : validation serveur des adresses d'hôte d'impression
//! (M1), nom de fichier choisi à l'envoi et historique des envois par
//! imprimante (M2/M3, Orca PrintHostSendDialog / PrintHostQueueDialog),
//! signature de tranchage pour la réouverture d'un projet (V1).

mod moonraker_mock;

use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{header, Request, StatusCode};
use axum::response::Response;
use axum::Router;
use backend::adapters::files::FileStore;
use backend::adapters::storage::AnyStorage;
use backend::auth::SecretBox;
use backend::domain::repo::{JobOutcome, NewGcode, NewJob, NewPrinter, NewProject};
use backend::domain::{PresetId, PresetKind, PresetOrigin, PrinterId, ProjectId, Storage, UserId};
use moonraker_mock::MockServer;
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
    let url = format!("sqlite://{}", dir.path().join("lot42.db").display());
    let storage = Arc::new(AnyStorage::connect_test(&url).await.unwrap());
    let files = FileStore::new(dir.path().join("data"));
    let state = backend::http::state::AppState::new(
        Arc::clone(&storage) as Arc<dyn Storage>,
        files.clone(),
    )
    .with_secrets(SecretBox::new([42u8; 32]));
    let session_layer = SessionManagerLayer::new(MemoryStore::default()).with_secure(false);
    Harness {
        _dir: dir,
        storage,
        files,
        app: backend::http::routes::router(state, session_layer),
    }
}

async fn call(
    app: &Router,
    method: &str,
    uri: &str,
    session: &str,
    body: Option<Value>,
) -> Response {
    let builder = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::COOKIE, session);
    let req = match body {
        Some(body) => builder
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
        None => builder.body(Body::empty()).unwrap(),
    };
    app.clone().oneshot(req).await.unwrap()
}

async fn json_body(resp: Response) -> Value {
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    }
}

async fn register(h: &Harness, email: &str) -> (String, UserId) {
    let resp = h
        .app
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
    let session = resp.headers()[header::SET_COOKIE]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_string();
    let id = h
        .storage
        .users()
        .find_by_email(email)
        .await
        .unwrap()
        .unwrap()
        .id;
    (session, id)
}

async fn user_preset(
    h: &Harness,
    owner: UserId,
    kind: PresetKind,
    name: &str,
    values: Value,
) -> String {
    h.storage
        .presets()
        .create_user_preset(
            owner,
            backend::domain::Preset {
                id: PresetId::new(),
                kind,
                name: name.into(),
                origin: PresetOrigin::User,
                user_id: Some(owner),
                vendor: None,
                inherits: None,
                instantiation: true,
                setting_id: None,
                filament_id: None,
                compatible_printers: None,
                renamed_from: Vec::new(),
                values,
            },
        )
        .await
        .unwrap()
        .id
        .to_string()
}

async fn declare(h: &Harness, session: &str, preset: &str, name: &str, url: &str) -> String {
    let resp = call(
        &h.app,
        "POST",
        "/api/printers",
        session,
        Some(json!({ "name": name, "moonraker_url": url, "machine_preset_id": preset })),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::CREATED, "{url}");
    json_body(resp).await["id"].as_str().unwrap().to_string()
}

async fn seed_gcode(h: &Harness, owner: UserId, project: ProjectId) -> String {
    let job = h
        .storage
        .jobs()
        .enqueue(
            owner,
            NewJob {
                project_id: project,
                plate_index: 0,
                resolved_settings: json!({}),
            },
        )
        .await
        .unwrap();
    let path = h
        .files
        .write_model(
            owner,
            backend::domain::ModelId::new(),
            "gcode",
            b"; G-code\nG28\n",
        )
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
                stats: json!({ "output_filename": format!("{}_PLA_8m52s.gcode", engine::api::OUTPUT_FILENAME_BASE_TOKEN) }),
                thumbnails: json!([]),
            },
        )
        .await
        .unwrap();
    gcode.id.to_string()
}

/// M1 : « pas une url » et consorts refusés à l'enregistrement (422 lisible,
/// \`details.reason\`), adresses d'Orca acceptées et nettoyées (trim) ; une
/// adresse sans schéma est jointe en http:// (make_url d'Orca) ; une adresse
/// inutilisable déjà en base donne 409 \`printer_not_connected\`, pas 502.
#[tokio::test]
async fn printer_host_addresses_are_validated_like_orca() {
    let mock = MockServer::start().await;
    let h = harness().await;
    let (session, owner) = register(&h, "m1@test.local").await;
    let preset = user_preset(&h, owner, PresetKind::Machine, "M", json!({})).await;

    for (raw, reason) in [
        ("pas une url", "whitespace"),
        ("http://pas une url", "whitespace"),
        ("ftp://voron.local", "scheme"),
        ("192.168.1.300", "host"),
        ("voron.local:99999", "port"),
        ("http://voron.local/?x=1", "query"),
    ] {
        let resp = call(
            &h.app,
            "POST",
            "/api/printers",
            &session,
            Some(json!({ "name": "V", "moonraker_url": raw, "machine_preset_id": preset })),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY, "{raw}");
        let body = json_body(resp).await;
        assert_eq!(body["code"], "validation");
        assert_eq!(body["details"]["field"], "moonraker_url");
        assert_eq!(body["details"]["reason"], reason, "{raw}");
        let message = body["message"].as_str().unwrap();
        assert!(message.contains(&format!("« {raw} »")), "{message}");
        assert!(message.contains("192.168.1.20:7125"), "exemple : {message}");
    }
    assert!(h.storage.printers().list(owner).await.unwrap().is_empty());

    // Adresses valides : saisie nettoyée conservée telle quelle (print_host).
    let id = declare(&h, &session, &preset, "V", "  192.168.1.20:7125 ").await;
    let printer = json_body(
        call(
            &h.app,
            "GET",
            &format!("/api/printers/{id}"),
            &session,
            None,
        )
        .await,
    )
    .await;
    assert_eq!(printer["moonraker_url"], "192.168.1.20:7125");
    // PUT : même validation.
    let resp = call(
        &h.app,
        "PUT",
        &format!("/api/printers/{id}"),
        &session,
        Some(json!({ "name": "V", "moonraker_url": "pas une url", "machine_preset_id": preset })),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    // Champ vide : imprimante non connectée.
    let offline = declare(&h, &session, &preset, "Off", "   ").await;
    let printer = json_body(
        call(
            &h.app,
            "GET",
            &format!("/api/printers/{offline}"),
            &session,
            None,
        )
        .await,
    )
    .await;
    assert!(printer["moonraker_url"].is_null());

    // Sans schéma : jointe en http:// comme Orca.
    let bare = mock.base_url().trim_start_matches("http://").to_string();
    let reach = declare(&h, &session, &preset, "Bare", &bare).await;
    let resp = call(
        &h.app,
        "POST",
        &format!("/api/printers/{reach}/test"),
        &session,
        Some(json!({})),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Adresse inutilisable enregistrée avant le lot 42 : 409 lisible.
    let pid = PrinterId(uuid::Uuid::parse_str(&id).unwrap());
    h.storage
        .printers()
        .update(
            owner,
            pid,
            NewPrinter {
                name: "V".into(),
                moonraker_url: Some("http://pas une url".into()),
                api_key: None,
                machine_preset_id: PresetId(uuid::Uuid::parse_str(&preset).unwrap()),
            },
        )
        .await
        .unwrap();
    let resp = call(
        &h.app,
        "POST",
        &format!("/api/printers/{id}/test"),
        &session,
        Some(json!({})),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::CONFLICT);
    let body = json_body(resp).await;
    assert_eq!(body["code"], "printer_not_connected");
    assert_eq!(body["details"]["reason"], "whitespace");
}

/// M2/M3 : nom choisi à l'envoi (validé, extension imposée), nom proposé,
/// historique par imprimante (succès et échec, toutes imprimantes, purge) et
/// isolation.
#[tokio::test]
async fn upload_filename_and_upload_history() {
    let mock = MockServer::start().await;
    let h = harness().await;
    let (session, owner) = register(&h, "m2@test.local").await;
    let preset = user_preset(&h, owner, PresetKind::Machine, "M", json!({})).await;
    let id = declare(&h, &session, &preset, "Voron", &mock.base_url()).await;
    let dead = declare(&h, &session, &preset, "Dead", "http://127.0.0.1:1").await;
    let project = h
        .storage
        .projects()
        .create(
            owner,
            NewProject {
                name: "Benchy".into(),
                scene: json!({}),
                active_presets: json!({}),
                thumbnail_path: None,
            },
        )
        .await
        .unwrap();
    let gcode = seed_gcode(&h, owner, project.id).await;

    // Nom proposé (pré-rempli, modifiable) et validation sans envoi.
    let proposed = json_body(
        call(
            &h.app,
            "POST",
            &format!("/api/printers/{id}/upload-name"),
            &session,
            Some(json!({ "gcode_id": gcode })),
        )
        .await,
    )
    .await;
    assert_eq!(proposed["filename"], "Benchy_PLA_8m52s.gcode");
    assert_eq!(proposed["extension"], ".gcode");
    assert_eq!(proposed["files"], json!(["Benchy_PLA_8m52s.gcode"]));
    let checked = json_body(
        call(
            &h.app,
            "POST",
            &format!("/api/printers/{id}/upload-name"),
            &session,
            Some(json!({ "gcode_id": gcode, "filename": "Tour d'essai" })),
        )
        .await,
    )
    .await;
    assert_eq!(checked["filename"], "Tour d'essai.gcode");
    let resp = call(
        &h.app,
        "POST",
        &format!("/api/printers/{id}/upload-name"),
        &session,
        Some(json!({ "gcode_id": gcode, "filename": "a:b" })),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);

    // Nom refusé : 422, rien d'envoyé ni d'inscrit.
    let resp = call(
        &h.app,
        "POST",
        &format!("/api/printers/{id}/upload"),
        &session,
        Some(json!({ "gcode_id": gcode, "start_now": false, "filename": "dossier/pièce" })),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body = json_body(resp).await;
    assert_eq!(body["details"]["field"], "filename");
    assert_eq!(body["details"]["reason"], "chars");
    assert!(mock.uploads().is_empty());

    // Nom choisi : extension ajoutée, accents gardés.
    let resp = call(
        &h.app,
        "POST",
        &format!("/api/printers/{id}/upload"),
        &session,
        Some(json!({ "gcode_id": gcode, "start_now": true, "filename": " Pièce finale " })),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let sent = json_body(resp).await;
    assert_eq!(sent["filename"], "Pièce finale.gcode");
    assert_eq!(mock.uploads()[0].filename, "Pièce finale.gcode");
    let upload_id = sent["upload_id"].as_str().unwrap().to_string();

    // Sans nom : nom proposé (inchangé).
    let resp = call(
        &h.app,
        "POST",
        &format!("/api/printers/{id}/upload"),
        &session,
        Some(json!({ "gcode_id": gcode, "start_now": false })),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(mock.uploads()[1].filename, "Benchy_PLA_8m52s.gcode");

    // Échec réseau : 502 et ligne « error » avec le message.
    let resp = call(
        &h.app,
        "POST",
        &format!("/api/printers/{dead}/upload"),
        &session,
        Some(json!({ "gcode_id": gcode, "start_now": false })),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::BAD_GATEWAY);

    let history = json_body(
        call(
            &h.app,
            "GET",
            &format!("/api/printers/{id}/uploads"),
            &session,
            None,
        )
        .await,
    )
    .await;
    let rows = history.as_array().unwrap();
    assert_eq!(rows.len(), 2, "{history}");
    assert_eq!(
        rows[0]["filename"], "Benchy_PLA_8m52s.gcode",
        "plus récent d'abord"
    );
    assert_eq!(rows[0]["action"], "upload");
    let first = &rows[1];
    assert_eq!(first["id"], upload_id);
    assert_eq!(first["status"], "completed");
    assert_eq!(first["action"], "print");
    assert_eq!(first["print_started"], true);
    assert_eq!(first["filename"], "Pièce finale.gcode");
    assert_eq!(first["path"], "Pièce finale.gcode");
    assert_eq!(first["host"], mock.base_url());
    assert_eq!(first["host_type"], "moonraker");
    assert_eq!(first["size_bytes"], 13);
    assert_eq!(first["gcode_id"], gcode);
    assert_eq!(first["project_id"], project.id.to_string());
    assert_eq!(first["printer_name"], "Voron");
    assert!(first["finished_at"].is_string());

    let failed = json_body(
        call(
            &h.app,
            "GET",
            &format!("/api/printers/{dead}/uploads"),
            &session,
            None,
        )
        .await,
    )
    .await;
    assert_eq!(failed[0]["status"], "error");
    assert!(!failed[0]["error"].as_str().unwrap().is_empty());
    assert!(failed[0]["path"].is_null());

    let all = json_body(
        call(
            &h.app,
            "GET",
            "/api/printer-uploads?limit=2",
            &session,
            None,
        )
        .await,
    )
    .await;
    assert_eq!(all.as_array().unwrap().len(), 2);
    assert_eq!(all[0]["printer_name"], "Dead");

    // Isolation : l'historique d'un autre compte est inexistant (404).
    let (bob, _) = register(&h, "bob@test.local").await;
    for method in ["GET", "DELETE"] {
        let resp = call(
            &h.app,
            method,
            &format!("/api/printers/{id}/uploads"),
            &bob,
            None,
        )
        .await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND, "{method}");
    }
    let resp = call(
        &h.app,
        "POST",
        &format!("/api/printers/{id}/upload-name"),
        &bob,
        Some(json!({ "gcode_id": gcode })),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let bob_all = json_body(call(&h.app, "GET", "/api/printer-uploads", &bob, None).await).await;
    assert_eq!(bob_all, json!([]));

    // Vider l'historique d'une imprimante.
    let resp = call(
        &h.app,
        "DELETE",
        &format!("/api/printers/{id}/uploads"),
        &session,
        None,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    let history = json_body(
        call(
            &h.app,
            "GET",
            &format!("/api/printers/{id}/uploads"),
            &session,
            None,
        )
        .await,
    )
    .await;
    assert_eq!(history, json!([]));
    let all = json_body(call(&h.app, "GET", "/api/printer-uploads", &session, None).await).await;
    assert_eq!(
        all.as_array().unwrap().len(),
        1,
        "l'historique de « Dead » reste"
    );
}

/// Compte avec imprimante possédée et presets utilisateur (process 3 parois).
async fn slicing_account(h: &Harness, email: &str) -> (String, UserId, Value) {
    let (session, owner) = register(h, email).await;
    let machine = user_preset(h, owner, PresetKind::Machine, "Bench 0.4 nozzle", json!({})).await;
    h.storage
        .printers()
        .create(
            owner,
            NewPrinter {
                name: "Bench".into(),
                moonraker_url: None,
                api_key: None,
                machine_preset_id: PresetId(uuid::Uuid::parse_str(&machine).unwrap()),
            },
        )
        .await
        .unwrap();
    let process = user_preset(
        h,
        owner,
        PresetKind::Process,
        "Mon process",
        json!({ "layer_height": "0.2", "wall_loops": "3" }),
    )
    .await;
    let filament = user_preset(
        h,
        owner,
        PresetKind::Filament,
        "Mon PLA",
        json!({ "filament_type": ["PLA"], "nozzle_temperature": ["210"] }),
    )
    .await;
    (
        session,
        owner,
        json!({ "printer": machine, "process": process, "filaments": [filament] }),
    )
}

fn two_plates() -> Value {
    json!({
        "schema_version": 1,
        "objects": [
            { "id": "o1", "modelId": "m1", "transform": { "position": [10.0, 10.0, 0.0] } },
            { "id": "o2", "modelId": "m2", "transform": { "position": [10.0, 10.0, 0.0] } }
        ],
        "plates": [
            { "id": "p1", "objectIds": ["o1"] },
            { "id": "p2", "objectIds": ["o2"] }
        ]
    })
}

/// V1 : signature figée avec chaque job, recalculée pour le projet enregistré
/// (\`fresh\` seulement si elle est égale), brouillon == surcharges enregistrées
/// ⇒ même signature, réglage ou objet modifié ⇒ signature différente, et par
/// plateau.
#[tokio::test]
async fn slice_signature_tells_whether_the_last_gcode_still_matches() {
    let h = harness().await;
    let (session, owner, mut active) = slicing_account(&h, "v1@test.local").await;
    active["overrides"] = json!({ "process": { "wall_loops": "5" } });
    let project = h
        .storage
        .projects()
        .create(
            owner,
            NewProject {
                name: "P".into(),
                scene: two_plates(),
                active_presets: active.clone(),
                thumbnail_path: None,
            },
        )
        .await
        .unwrap();
    let pid = project.id;

    // Tranchage du plateau 1 avec les surcharges enregistrées.
    let resp = call(
        &h.app,
        "POST",
        &format!("/api/projects/{pid}/slice"),
        &session,
        Some(json!({ "plate_index": 0 })),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let sliced = json_body(resp).await;
    let job0 = sliced["jobs"][0].clone();
    let sig0 = job0["slice_signature"].as_str().unwrap().to_string();
    assert!(sig0.starts_with("v1:"), "{sig0}");

    // Brouillon identique aux surcharges : même config effective, même signature.
    let resp = call(
        &h.app,
        "POST",
        &format!("/api/projects/{pid}/slice"),
        &session,
        Some(json!({ "plate_index": 0, "draft": { "process": { "wall_loops": "5" } } })),
    )
    .await;
    let draft_sig = json_body(resp).await["jobs"][0]["slice_signature"].clone();
    assert_eq!(draft_sig, json!(sig0));
    // Brouillon différent : autre signature.
    let resp = call(
        &h.app,
        "POST",
        &format!("/api/projects/{pid}/slice"),
        &session,
        Some(json!({ "plate_index": 0, "draft": { "process": { "wall_loops": "3" } } })),
    )
    .await;
    assert_ne!(
        json_body(resp).await["jobs"][0]["slice_signature"],
        json!(sig0)
    );

    // Signature du projet enregistré : égale, mais aucun job réussi → pas fresh.
    let state = json_body(
        call(
            &h.app,
            "GET",
            &format!("/api/projects/{pid}/slice-signature"),
            &session,
            None,
        )
        .await,
    )
    .await;
    assert_eq!(state["version"], "v1");
    assert_eq!(state["plates"].as_array().unwrap().len(), 2);
    assert_eq!(state["plates"][0]["signature"], json!(sig0));
    assert_eq!(state["plates"][0]["fresh"], false);
    assert_ne!(state["plates"][1]["signature"], json!(sig0), "par plateau");

    // Le premier job réussit (G-code produit) : plateau 1 à jour.
    let job_id =
        backend::domain::JobId(uuid::Uuid::parse_str(job0["id"].as_str().unwrap()).unwrap());
    let gcode = h
        .storage
        .gcodes()
        .create(
            owner,
            NewGcode {
                job_id,
                file_path: "x.gcode".into(),
                preview_path: String::new(),
                stats: json!({}),
                thumbnails: json!([]),
            },
        )
        .await
        .unwrap();
    h.storage.jobs().claim_next().await.unwrap();
    h.storage
        .jobs()
        .finish(job_id, JobOutcome::Succeeded { gcode_id: gcode.id })
        .await
        .unwrap();
    let job = json_body(
        call(
            &h.app,
            "GET",
            &format!("/api/jobs/{job_id}"),
            &session,
            None,
        )
        .await,
    )
    .await;
    assert_eq!(job["status"], "succeeded");
    assert_eq!(job["slice_signature"], json!(sig0));
    let state = json_body(
        call(
            &h.app,
            "GET",
            &format!("/api/projects/{pid}/slice-signature"),
            &session,
            None,
        )
        .await,
    )
    .await;
    let plate0 = &state["plates"][0];
    assert_eq!(plate0["fresh"], true, "{state}");
    assert_eq!(plate0["last_job_id"], job0["id"]);
    assert_eq!(plate0["last_gcode_id"], json!(gcode.id.to_string()));
    assert_eq!(state["plates"][1]["fresh"], false);

    // Objet du plateau 2 déplacé et enregistré : plateau 1 toujours à jour.
    let mut moved = two_plates();
    moved["objects"][1]["transform"]["position"][0] = json!(60.0);
    h.storage
        .projects()
        .update(owner, pid, 1, moved.clone(), active.clone(), None)
        .await
        .unwrap();
    let state = json_body(
        call(
            &h.app,
            "GET",
            &format!("/api/projects/{pid}/slice-signature"),
            &session,
            None,
        )
        .await,
    )
    .await;
    assert_eq!(state["plates"][0]["fresh"], true);

    // Réglage modifié puis enregistré (wall_loops 5 → 4) : périmé (audit V1).
    let mut changed = active.clone();
    changed["overrides"] = json!({ "process": { "wall_loops": "4" } });
    h.storage
        .projects()
        .update(owner, pid, 2, moved.clone(), changed, None)
        .await
        .unwrap();
    let state = json_body(
        call(
            &h.app,
            "GET",
            &format!("/api/projects/{pid}/slice-signature"),
            &session,
            None,
        )
        .await,
    )
    .await;
    assert_eq!(state["plates"][0]["fresh"], false);
    assert_ne!(state["plates"][0]["signature"], json!(sig0));

    // Objet du plateau 1 déplacé : périmé aussi.
    let mut moved1 = moved.clone();
    moved1["objects"][0]["transform"]["position"][1] = json!(30.0);
    h.storage
        .projects()
        .update(owner, pid, 3, moved1, active.clone(), None)
        .await
        .unwrap();
    let state = json_body(
        call(
            &h.app,
            "GET",
            &format!("/api/projects/{pid}/slice-signature"),
            &session,
            None,
        )
        .await,
    )
    .await;
    assert_eq!(state["plates"][0]["fresh"], false);

    // Isolation.
    let (bob, _) = register(&h, "bob@test.local").await;
    let resp = call(
        &h.app,
        "GET",
        &format!("/api/projects/{pid}/slice-signature"),
        &bob,
        None,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
