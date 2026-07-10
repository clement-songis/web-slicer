//! Seed des presets système (T038) au niveau stockage : idempotence du re-seed
//! et préservation des presets utilisateur. Utilise un petit lot de presets
//! importés synthétiques (pas les 11 895 profils réels — testé côté engine).

use backend::adapters::storage::sqlite::SqliteStorage;
use backend::domain::presets::reseed_system_presets;
use backend::domain::{NewUser, Preset, PresetId, PresetKind, PresetOrigin, Role, Storage};
use engine::presets::{ImportedPreset, PresetKind as EngineKind};

fn imported(name: &str, kind: EngineKind) -> ImportedPreset {
    ImportedPreset {
        vendor: "TestVendor".into(),
        kind,
        name: name.into(),
        sub_path: String::new(),
        inherits: None,
        from: None,
        setting_id: None,
        filament_id: None,
        instantiation: true,
        compatible_printers: vec![],
        values: serde_json::Map::new(),
    }
}

async fn storage() -> (tempfile::TempDir, SqliteStorage) {
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}", dir.path().join("seed.db").display());
    (dir, SqliteStorage::connect(&url).await.unwrap())
}

#[tokio::test]
async fn reseed_is_idempotent() {
    let (_d, storage) = storage().await;
    let batch = vec![
        imported("fdm_process_common", EngineKind::Process),
        imported("0.20 Standard", EngineKind::Process),
        imported("Generic PLA", EngineKind::Filament),
    ];

    let n1 = reseed_system_presets(&storage, &batch).await.unwrap();
    assert_eq!(n1, 3);
    assert_eq!(storage.presets().system_count().await.unwrap(), 3);

    // Re-seed : remplace, ne cumule pas.
    let n2 = reseed_system_presets(&storage, &batch).await.unwrap();
    assert_eq!(n2, 3);
    assert_eq!(
        storage.presets().system_count().await.unwrap(),
        3,
        "le re-seed remplace au lieu de cumuler"
    );
}

#[tokio::test]
async fn reseed_preserves_user_presets() {
    let (_d, storage) = storage().await;
    reseed_system_presets(&storage, &[imported("Generic PLA", EngineKind::Filament)])
        .await
        .unwrap();

    // Un utilisateur et son preset dérivé.
    let user = storage
        .users()
        .create(NewUser {
            email: "u@test.local".into(),
            password_hash: "x".into(),
            role: Role::User,
        })
        .await
        .unwrap();
    let user_preset = Preset {
        id: PresetId::new(),
        kind: PresetKind::Filament,
        name: "Mon PLA".into(),
        origin: PresetOrigin::User,
        user_id: Some(user.id),
        vendor: None,
        inherits: Some("Generic PLA".into()),
        instantiation: true,
        setting_id: None,
        filament_id: None,
        compatible_printers: None,
        values: serde_json::json!({ "filament_flow_ratio": "0.98" }),
    };
    let created = storage
        .presets()
        .create_user_preset(user.id, user_preset)
        .await
        .unwrap();

    // Re-seed système : le preset utilisateur survit.
    reseed_system_presets(&storage, &[imported("Generic PETG", EngineKind::Filament)])
        .await
        .unwrap();

    let fetched = storage.presets().get(created.id).await.unwrap();
    assert_eq!(fetched.name, "Mon PLA");
    assert_eq!(fetched.user_id, Some(user.id));
    assert_eq!(
        storage.presets().system_count().await.unwrap(),
        1,
        "un seul preset système après re-seed"
    );
}
