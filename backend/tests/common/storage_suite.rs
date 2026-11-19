//! Suite générique de contrat du trait `Storage` (T024, TDD — écrite AVANT les
//! implémentations). La même suite doit passer sur SQLite (T025) et Postgres
//! (T026) : chaque adaptateur l'invoque depuis son test d'intégration via
//! `storage_suite::run_all(&storage).await`.
//!
//! Garanties couvertes (storage-trait.md § Garanties) : isolation par
//! `user_id` (SC-008), unicité (email, nom projet, nom preset), transactions
//! (`claim_next` sans double délivrance), cascade de suppression de compte,
//! re-seed système préservant les presets utilisateur, et verrou optimiste
//! des projets.

#![allow(dead_code)]

use backend::domain::*;
use serde_json::json;
use uuid::Uuid;

/// Exécute toutes les garanties. Panique au premier écart de contrat.
pub async fn run_all(s: &dyn Storage) {
    isolation_between_users(s).await;
    unique_email(s).await;
    unique_project_name_per_user(s).await;
    unique_user_preset_name(s).await;
    optimistic_version_lock(s).await;
    claim_next_delivers_each_job_once(s).await;
    requeue_running_on_boot(s).await;
    system_reseed_preserves_user_presets(s).await;
    referenced_user_preset_deletion_is_rejected(s).await;
    printer_preset_references_are_isolated(s).await;
    account_deletion_cascades(s).await;
    user_preferences_are_per_account_and_cascade(s).await;
    instance_quotas_round_trip(s).await;
}

/// Quotas d'instance (lot 32) : 0 (illimité) par défaut, écriture indépendante
/// des autres réglages, conflit d'unicité sans message SQL brut.
async fn instance_quotas_round_trip(s: &dyn Storage) {
    let before = s.instance().settings().await.unwrap();
    assert_eq!(before.storage_quota_bytes, 0);
    assert_eq!(before.max_concurrent_slices, 0);
    s.instance().set_storage_quota(5_000_000_000).await.unwrap();
    s.instance().set_max_concurrent_slices(3).await.unwrap();
    let after = s.instance().settings().await.unwrap();
    assert_eq!(after.storage_quota_bytes, 5_000_000_000);
    assert_eq!(after.max_concurrent_slices, 3);
    assert_eq!(after.upload_limit_bytes, before.upload_limit_bytes);
    assert_eq!(after.registration_policy, before.registration_policy);
    s.instance().set_storage_quota(0).await.unwrap();
    s.instance().set_max_concurrent_slices(0).await.unwrap();

    let owner = make_user(s).await;
    make_project(s, owner.id, "Doublon").await;
    let dup = s
        .projects()
        .create(
            owner.id,
            backend::domain::repo::NewProject {
                name: "Doublon".into(),
                scene: json!({}),
                active_presets: json!({}),
                thumbnail_path: None,
            },
        )
        .await;
    match dup {
        Err(StorageError::Conflict(m)) => {
            assert!(!m.contains("UNIQUE") && !m.contains("duplicate key"), "{m}")
        }
        other => panic!("conflit attendu : {other:?}"),
    }
}

/// Préférences par compte : `{}` par défaut, upsert, isolation, cascade.
async fn user_preferences_are_per_account_and_cascade(s: &dyn Storage) {
    let a = make_user(s).await;
    let b = make_user(s).await;
    assert_eq!(
        s.users().get_preferences(a.id).await.unwrap(),
        serde_json::json!({})
    );
    let prefs = serde_json::json!({ "zoom_to_mouse": true, "camera": { "style": "free" } });
    s.users()
        .set_preferences(a.id, prefs.clone())
        .await
        .unwrap();
    s.users()
        .set_preferences(a.id, serde_json::json!({ "zoom_to_mouse": false }))
        .await
        .unwrap();
    assert_eq!(
        s.users().get_preferences(a.id).await.unwrap(),
        serde_json::json!({ "zoom_to_mouse": false }),
        "remplacement (upsert)"
    );
    assert_eq!(
        s.users().get_preferences(b.id).await.unwrap(),
        serde_json::json!({})
    );
    s.users().delete(a.id).await.unwrap();
    assert!(matches!(
        s.users().set_preferences(a.id, prefs).await,
        Err(backend::domain::StorageError::NotFound)
    ));
    assert_eq!(
        s.users().get_preferences(a.id).await.unwrap(),
        serde_json::json!({})
    );
}

// --- Helpers -----------------------------------------------------------------

async fn make_user(s: &dyn Storage) -> User {
    let email = format!("{}@test.local", Uuid::new_v4());
    s.users()
        .create(NewUser {
            email,
            password_hash: "argon2-placeholder".into(),
            role: Role::User,
        })
        .await
        .expect("création utilisateur")
}

async fn make_project(s: &dyn Storage, owner: UserId, name: &str) -> Project {
    s.projects()
        .create(
            owner,
            NewProject {
                name: name.into(),
                scene: json!({ "schema_version": 1, "objects": [] }),
                active_presets: json!({}),
                thumbnail_path: None,
            },
        )
        .await
        .expect("création projet")
}

async fn make_machine_preset(s: &dyn Storage, owner: UserId, name: &str) -> Preset {
    s.presets()
        .create_user_preset(
            owner,
            Preset {
                id: PresetId::new(),
                kind: PresetKind::Machine,
                name: name.into(),
                origin: PresetOrigin::User,
                user_id: Some(owner),
                vendor: None,
                inherits: None,
                instantiation: true,
                setting_id: None,
                filament_id: None,
                compatible_printers: None,
                values: json!({ "nozzle_diameter": "0.4" }),
            },
        )
        .await
        .expect("création preset machine")
}

// --- Garanties ---------------------------------------------------------------

async fn isolation_between_users(s: &dyn Storage) {
    let a = make_user(s).await;
    let b = make_user(s).await;
    let project = make_project(s, a.id, "isolé").await;

    // B ne voit ni n'atteint le projet de A.
    assert!(
        matches!(
            s.projects().get(b.id, project.id).await,
            Err(StorageError::NotFound)
        ),
        "get inter-comptes doit être NotFound (SC-008)"
    );
    assert!(
        s.projects().list(b.id).await.unwrap().is_empty(),
        "list de B ne contient pas le projet de A"
    );
    assert!(
        matches!(
            s.projects().delete(b.id, project.id).await,
            Err(StorageError::NotFound)
        ),
        "delete inter-comptes doit être NotFound"
    );
    // A y accède normalement.
    assert_eq!(
        s.projects().get(a.id, project.id).await.unwrap().id,
        project.id
    );
}

async fn unique_email(s: &dyn Storage) {
    let email = format!("{}@test.local", Uuid::new_v4());
    let mk = || NewUser {
        email: email.clone(),
        password_hash: "h".into(),
        role: Role::User,
    };
    s.users().create(mk()).await.expect("premier compte");
    assert!(
        matches!(s.users().create(mk()).await, Err(StorageError::Conflict(_))),
        "email dupliqué doit être Conflict"
    );
}

async fn unique_project_name_per_user(s: &dyn Storage) {
    let a = make_user(s).await;
    let b = make_user(s).await;
    make_project(s, a.id, "mon-projet").await;
    assert!(
        matches!(
            s.projects()
                .create(
                    a.id,
                    NewProject {
                        name: "mon-projet".into(),
                        scene: json!({}),
                        active_presets: json!({}),
                        thumbnail_path: None,
                    }
                )
                .await,
            Err(StorageError::Conflict(_))
        ),
        "nom de projet dupliqué pour le même utilisateur → Conflict"
    );
    // Un autre utilisateur peut réutiliser le même nom.
    make_project(s, b.id, "mon-projet").await;
}

async fn unique_user_preset_name(s: &dyn Storage) {
    let a = make_user(s).await;
    make_machine_preset(s, a.id, "ma-machine").await;
    let dup = s
        .presets()
        .create_user_preset(
            a.id,
            Preset {
                id: PresetId::new(),
                kind: PresetKind::Machine,
                name: "ma-machine".into(),
                origin: PresetOrigin::User,
                user_id: Some(a.id),
                vendor: None,
                inherits: None,
                instantiation: true,
                setting_id: None,
                filament_id: None,
                compatible_printers: None,
                values: json!({}),
            },
        )
        .await;
    assert!(
        matches!(dup, Err(StorageError::Conflict(_))),
        "preset (kind, name) dupliqué → Conflict"
    );
}

async fn optimistic_version_lock(s: &dyn Storage) {
    let a = make_user(s).await;
    let p = make_project(s, a.id, "verrou").await;
    assert_eq!(p.version, 1, "version initiale = 1");

    let updated = s
        .projects()
        .update(a.id, p.id, 1, json!({ "v": 2 }), json!({}), None)
        .await
        .expect("update avec la bonne version");
    assert_eq!(updated.version, 2, "version incrémentée");

    assert!(
        matches!(
            s.projects()
                .update(a.id, p.id, 1, json!({ "v": 3 }), json!({}), None)
                .await,
            Err(StorageError::VersionConflict { .. })
        ),
        "version périmée → VersionConflict (409)"
    );
}

async fn claim_next_delivers_each_job_once(s: &dyn Storage) {
    let a = make_user(s).await;
    let project = make_project(s, a.id, "file").await;
    const N: usize = 5;
    for i in 0..N {
        s.jobs()
            .enqueue(
                a.id,
                NewJob {
                    project_id: project.id,
                    plate_index: i as i64,
                    resolved_settings: json!({}),
                },
            )
            .await
            .expect("enqueue");
    }

    // Réclamations concurrentes : plus de tentatives que de jobs.
    let claims = (0..N + 3).map(|_| s.jobs().claim_next());
    let results = futures::future::join_all(claims).await;

    let mut ids: Vec<_> = results
        .into_iter()
        .flat_map(|r| r.expect("claim_next ok"))
        .map(|j| j.id)
        .collect();
    ids.sort();
    ids.dedup();
    assert_eq!(
        ids.len(),
        N,
        "chaque job réclamé exactement une fois : {ids:?}"
    );
}

async fn requeue_running_on_boot(s: &dyn Storage) {
    let a = make_user(s).await;
    let project = make_project(s, a.id, "reprise").await;
    s.jobs()
        .enqueue(
            a.id,
            NewJob {
                project_id: project.id,
                plate_index: 0,
                resolved_settings: json!({}),
            },
        )
        .await
        .unwrap();

    let claimed = s
        .jobs()
        .claim_next()
        .await
        .unwrap()
        .expect("un job réclamé");
    assert_eq!(claimed.status, JobStatus::Running);

    let requeued = s.jobs().requeue_running().await.unwrap();
    assert!(requeued >= 1, "au moins un job running remis en file");

    let again = s.jobs().claim_next().await.unwrap();
    assert!(again.is_some(), "le job repasse réclamable après reprise");
}

async fn system_reseed_preserves_user_presets(s: &dyn Storage) {
    let a = make_user(s).await;
    let user_preset = make_machine_preset(s, a.id, "à-préserver").await;

    let system = |kind: PresetKind, name: &str| Preset {
        id: PresetId::new(),
        kind,
        name: name.into(),
        origin: PresetOrigin::System,
        user_id: None,
        vendor: Some("BBL".into()),
        inherits: None,
        instantiation: true,
        setting_id: None,
        filament_id: None,
        compatible_printers: None,
        values: json!({ "layer_height": "0.2" }),
    };
    let machine = system(PresetKind::Machine, "Machine stable");
    let process = system(PresetKind::Process, "0.20 Standard");
    let filament = system(PresetKind::Filament, "PLA stable");
    let obsolete = system(PresetKind::Process, "0.28 Draft");
    s.presets()
        .reseed_system(vec![
            machine.clone(),
            process.clone(),
            filament.clone(),
            obsolete.clone(),
        ])
        .await
        .expect("premier seed");

    let printer = s
        .printers()
        .create(
            a.id,
            NewPrinter {
                name: "Imprimante stable".into(),
                moonraker_url: None,
                api_key: None,
                machine_preset_id: machine.id,
            },
        )
        .await
        .unwrap();
    let active_presets = json!({
        "printer": machine.id.to_string(),
        "filaments": [filament.id.to_string()],
        "process": process.id.to_string(),
    });
    let project = s
        .projects()
        .create(
            a.id,
            NewProject {
                name: "Références stables".into(),
                scene: json!({}),
                active_presets: active_presets.clone(),
                thumbnail_path: None,
            },
        )
        .await
        .unwrap();

    // Les IDs fournis par l'import peuvent changer : l'identité persistée est
    // la clé naturelle (kind, name, vendor), et les références ne bougent pas.
    let mut reimported_machine = machine.clone();
    reimported_machine.id = PresetId::new();
    reimported_machine.values = json!({ "nozzle_diameter": "0.6" });
    let mut reimported_process = process.clone();
    reimported_process.id = PresetId::new();
    let mut reimported_filament = filament.clone();
    reimported_filament.id = PresetId::new();
    s.presets()
        .reseed_system(vec![
            reimported_machine,
            reimported_process,
            reimported_filament,
        ])
        .await
        .expect("re-seed");

    assert_eq!(
        s.presets().get(user_preset.id).await.unwrap().id,
        user_preset.id,
        "le preset utilisateur survit au re-seed"
    );
    assert_eq!(
        s.presets().get(machine.id).await.unwrap().values,
        json!({ "nozzle_diameter": "0.6" }),
        "le re-seed met à jour le preset sans changer son UUID"
    );
    assert!(matches!(
        s.presets().get(obsolete.id).await,
        Err(StorageError::NotFound)
    ));
    assert_eq!(s.presets().system_count().await.unwrap(), 3);
    assert_eq!(
        s.printers()
            .get(a.id, printer.id)
            .await
            .unwrap()
            .machine_preset_id,
        machine.id,
        "l'imprimante et sa FK survivent au re-seed du même lot"
    );
    assert_eq!(
        s.projects()
            .get(a.id, project.id)
            .await
            .unwrap()
            .active_presets,
        active_presets,
        "les références JSON du projet restent valides et inchangées"
    );

    // Retirer du lot un preset encore référencé serait une suppression métier :
    // elle est refusée et toute la transaction est annulée, plutôt que de
    // casser silencieusement une imprimante ou un projet.
    let mut process_only = process.clone();
    process_only.id = PresetId::new();
    assert!(matches!(
        s.presets().reseed_system(vec![process_only]).await,
        Err(StorageError::Conflict(_))
    ));
    assert_eq!(s.presets().get(machine.id).await.unwrap().id, machine.id);
    assert_eq!(
        s.printers().get(a.id, printer.id).await.unwrap().id,
        printer.id
    );
    assert_eq!(s.presets().system_count().await.unwrap(), 3);
}

async fn referenced_user_preset_deletion_is_rejected(s: &dyn Storage) {
    let a = make_user(s).await;
    let preset = make_machine_preset(s, a.id, "machine-référencée").await;
    let printer = s
        .printers()
        .create(
            a.id,
            NewPrinter {
                name: "Référence imprimante".into(),
                moonraker_url: None,
                api_key: None,
                machine_preset_id: preset.id,
            },
        )
        .await
        .unwrap();
    let project = s
        .projects()
        .create(
            a.id,
            NewProject {
                name: "Référence projet".into(),
                scene: json!({}),
                active_presets: json!({ "printer": preset.id.to_string() }),
                thumbnail_path: None,
            },
        )
        .await
        .unwrap();

    assert!(matches!(
        s.presets().delete_user_preset(a.id, preset.id).await,
        Err(StorageError::Conflict(_))
    ));
    assert_eq!(s.presets().get(preset.id).await.unwrap().id, preset.id);
    assert_eq!(
        s.printers().get(a.id, printer.id).await.unwrap().id,
        printer.id
    );

    // La référence JSON suffit à elle seule à interdire la suppression.
    s.printers().delete(a.id, printer.id).await.unwrap();
    assert!(matches!(
        s.presets().delete_user_preset(a.id, preset.id).await,
        Err(StorageError::Conflict(_))
    ));
    assert_eq!(
        s.projects().get(a.id, project.id).await.unwrap().id,
        project.id
    );

    s.projects()
        .update(a.id, project.id, 1, json!({}), json!({}), None)
        .await
        .unwrap();
    s.presets()
        .delete_user_preset(a.id, preset.id)
        .await
        .expect("suppression possible une fois toutes les références retirées");
}

async fn printer_preset_references_are_isolated(s: &dyn Storage) {
    let alice = make_user(s).await;
    let bob = make_user(s).await;
    let alice_preset = make_machine_preset(s, alice.id, "machine-privée").await;

    let foreign_reference = s
        .printers()
        .create(
            bob.id,
            NewPrinter {
                name: "Référence interdite".into(),
                moonraker_url: None,
                api_key: None,
                machine_preset_id: alice_preset.id,
            },
        )
        .await;
    assert!(matches!(foreign_reference, Err(StorageError::NotFound)));

    let bob_preset = make_machine_preset(s, bob.id, "machine-de-bob").await;
    let printer = s
        .printers()
        .create(
            bob.id,
            NewPrinter {
                name: "Imprimante de Bob".into(),
                moonraker_url: None,
                api_key: None,
                machine_preset_id: bob_preset.id,
            },
        )
        .await
        .unwrap();
    let foreign_update = s
        .printers()
        .update(
            bob.id,
            printer.id,
            NewPrinter {
                name: "Imprimante de Bob".into(),
                moonraker_url: None,
                api_key: None,
                machine_preset_id: alice_preset.id,
            },
        )
        .await;
    assert!(matches!(foreign_update, Err(StorageError::NotFound)));
    assert_eq!(
        s.printers()
            .get(bob.id, printer.id)
            .await
            .unwrap()
            .machine_preset_id,
        bob_preset.id,
        "une mise à jour inter-comptes rejetée ne modifie pas la référence"
    );
}

async fn account_deletion_cascades(s: &dyn Storage) {
    let a = make_user(s).await;
    let project = make_project(s, a.id, "à-supprimer").await;
    let preset = make_machine_preset(s, a.id, "machine-à-supprimer").await;
    let model = s
        .models()
        .create(
            a.id,
            NewModel {
                project_id: Some(project.id),
                filename: "cube.stl".into(),
                format: ModelFormat::Stl,
                file_path: "data/users/x/models/1.stl".into(),
                mesh_path: None,
                size_bytes: 100,
                triangle_count: 12,
                repair_report: None,
            },
        )
        .await
        .unwrap();
    let printer = s
        .printers()
        .create(
            a.id,
            NewPrinter {
                name: "A1".into(),
                moonraker_url: Some("http://printer.local".into()),
                api_key: None,
                machine_preset_id: preset.id,
            },
        )
        .await
        .unwrap();
    assert_eq!(
        printer.moonraker_url.as_deref(),
        Some("http://printer.local")
    );
    // Phase 14 : imprimante possédée **sans** connexion réseau — `moonraker_url`
    // à `None` doit persister et se relire tel quel (colonne rendue nullable).
    let offline = s
        .printers()
        .create(
            a.id,
            NewPrinter {
                name: "Offline".into(),
                moonraker_url: None,
                api_key: None,
                machine_preset_id: preset.id,
            },
        )
        .await
        .unwrap();
    assert_eq!(offline.moonraker_url, None);
    assert_eq!(
        s.printers()
            .get(a.id, offline.id)
            .await
            .unwrap()
            .moonraker_url,
        None
    );
    let job = s
        .jobs()
        .enqueue(
            a.id,
            NewJob {
                project_id: project.id,
                plate_index: 0,
                resolved_settings: json!({}),
            },
        )
        .await
        .unwrap();
    let gcode = s
        .gcodes()
        .create(
            a.id,
            NewGcode {
                job_id: job.id,
                file_path: "data/users/x/gcodes/1.gcode".into(),
                preview_path: "data/users/x/gcodes/1.preview".into(),
                stats: json!({}),
                thumbnails: json!([]),
            },
        )
        .await
        .unwrap();

    s.users().delete(a.id).await.expect("suppression compte");

    // Toutes les ressources possédées ont disparu (cascade BDD).
    assert!(matches!(
        s.projects().get(a.id, project.id).await,
        Err(StorageError::NotFound)
    ));
    assert!(matches!(
        s.models().get(a.id, model.id).await,
        Err(StorageError::NotFound)
    ));
    assert!(matches!(
        s.printers().get(a.id, printer.id).await,
        Err(StorageError::NotFound)
    ));
    assert!(matches!(
        s.jobs().get(a.id, job.id).await,
        Err(StorageError::NotFound)
    ));
    assert!(matches!(
        s.gcodes().get(a.id, gcode.id).await,
        Err(StorageError::NotFound)
    ));
    assert!(matches!(
        s.presets().get(preset.id).await,
        Err(StorageError::NotFound)
    ));
    assert!(matches!(
        s.users().get(a.id).await,
        Err(StorageError::NotFound)
    ));
}
