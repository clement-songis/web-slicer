//! Charge concurrente sur l'adaptateur SQLite : reproduit les « database is
//! locked » (500 au bootstrap harness) observés dès 2 workers Playwright.
//!
//! Cause : transactions d'écriture en `BEGIN` différé qui lisent puis écrivent
//! (ex. création d'imprimante : vérification du preset puis INSERT) — la
//! promotion du verrou échoue sans passer par le busy timeout. Le correctif
//! (`BEGIN IMMEDIATE` + journal WAL posé une fois au démarrage) doit laisser
//! passer toutes les opérations, et la base doit être en WAL.

use std::sync::Arc;

use backend::adapters::storage::sqlite::SqliteStorage;
use backend::domain::{
    NewPrinter, NewProject, NewUser, Preset, PresetId, PresetKind, PresetOrigin, Role, Storage,
};
use serde_json::json;
use sqlx::Row;

const TASKS: usize = 16;
const ROUNDS: usize = 12;

async fn user_with_machine(
    storage: &SqliteStorage,
    i: usize,
) -> (backend::domain::UserId, PresetId) {
    let user = storage
        .users()
        .create(NewUser {
            email: format!("load{i}@test.local"),
            password_hash: "x".into(),
            role: Role::User,
        })
        .await
        .expect("création d'utilisateur concurrente");
    let preset = storage
        .presets()
        .create_user_preset(
            user.id,
            Preset {
                id: PresetId::new(),
                kind: PresetKind::Machine,
                name: format!("machine-{i}"),
                origin: PresetOrigin::User,
                user_id: Some(user.id),
                vendor: None,
                inherits: None,
                instantiation: true,
                setting_id: None,
                filament_id: None,
                compatible_printers: None,
                renamed_from: Vec::new(),
                values: json!({ "nozzle_diameter": "0.4" }),
            },
        )
        .await
        .expect("création de preset concurrente");
    (user.id, preset.id)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_read_then_write_transactions_never_lock() {
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}", dir.path().join("load.db").display());
    let storage = Arc::new(SqliteStorage::connect(&url).await.expect("connexion"));

    let mode: String = sqlx::query("PRAGMA journal_mode")
        .fetch_one(&storage.pool())
        .await
        .unwrap()
        .get(0);
    assert_eq!(mode, "wal", "journal WAL attendu après connect()");

    let started = std::time::Instant::now();
    let mut handles = Vec::new();
    for i in 0..TASKS {
        let storage = storage.clone();
        handles.push(tokio::spawn(async move {
            // Même enchaînement que /api/_harness/bootstrap : compte, preset,
            // imprimante (transaction lecture → écriture), projet, sauvegardes.
            let (user, machine) = user_with_machine(&storage, i).await;
            for round in 0..ROUNDS {
                let printer = storage
                    .printers()
                    .create(
                        user,
                        NewPrinter {
                            name: format!("p{round}"),
                            moonraker_url: None,
                            api_key: None,
                            machine_preset_id: machine,
                        },
                    )
                    .await
                    .unwrap_or_else(|e| panic!("création d'imprimante {i}/{round} : {e}"));
                storage
                    .printers()
                    .update(
                        user,
                        printer.id,
                        NewPrinter {
                            name: format!("p{round}-bis"),
                            moonraker_url: None,
                            api_key: None,
                            machine_preset_id: machine,
                        },
                    )
                    .await
                    .unwrap_or_else(|e| panic!("mise à jour d'imprimante {i}/{round} : {e}"));
                storage
                    .projects()
                    .create(
                        user,
                        NewProject {
                            name: format!("projet {round}"),
                            scene: json!({ "objects": [] }),
                            active_presets: json!({}),
                            thumbnail_path: None,
                        },
                    )
                    .await
                    .unwrap_or_else(|e| panic!("création de projet {i}/{round} : {e}"));
                // Lectures concurrentes des écritures en cours.
                storage.projects().list(user).await.unwrap();
                storage.printers().list(user).await.unwrap();
            }
        }));
    }
    for handle in handles {
        handle.await.expect("tâche de charge");
    }
    let elapsed = started.elapsed();

    let printers: i64 = sqlx::query("SELECT COUNT(*) FROM printers")
        .fetch_one(&storage.pool())
        .await
        .unwrap()
        .get(0);
    assert_eq!(printers as usize, TASKS * ROUNDS);
    eprintln!(
        "{} opérations d'écriture concurrentes ({TASKS} tâches) en {elapsed:?}",
        TASKS * (2 + ROUNDS * 3)
    );
}

#[tokio::test]
async fn wal_is_enabled_even_if_another_connection_is_open() {
    // Base existante en journal « delete » tenue ouverte par une autre
    // connexion (cas de la base créée/lue par un autre process) : connect()
    // ne doit pas échouer, et le WAL est posé dès que le verrou se libère.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("legacy.db");
    let url = format!("sqlite://{}", path.display());
    let first = SqliteStorage::connect(&url).await.unwrap();
    first.pool().close().await;

    use sqlx::{ConnectOptions, Connection};
    // Repasse la base en journal « delete » sur une connexion seule (le même
    // PRAGMA via le pool échoue : changer de mode exige un verrou exclusif).
    let mut legacy = sqlx::sqlite::SqliteConnectOptions::new()
        .filename(&path)
        .connect()
        .await
        .unwrap();
    sqlx::query("PRAGMA journal_mode = DELETE")
        .execute(&mut legacy)
        .await
        .unwrap();
    legacy.close().await.unwrap();

    let mut other = sqlx::sqlite::SqliteConnectOptions::new()
        .filename(&path)
        .connect()
        .await
        .unwrap();
    // Transaction de lecture ouverte : empêche le passage en WAL.
    sqlx::query("BEGIN").execute(&mut other).await.unwrap();
    sqlx::query("SELECT COUNT(*) FROM users")
        .fetch_one(&mut other)
        .await
        .unwrap();
    let release = tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(600)).await;
        sqlx::query("ROLLBACK").execute(&mut other).await.unwrap();
        other.close().await.unwrap();
    });
    let storage = SqliteStorage::connect(&url)
        .await
        .expect("connect malgré le verrou");
    release.await.unwrap();
    let mode: String = sqlx::query("PRAGMA journal_mode")
        .fetch_one(&storage.pool())
        .await
        .unwrap()
        .get(0);
    assert_eq!(mode, "wal");
}
