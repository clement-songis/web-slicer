//! Tests du client Moonraker (T074, R10) contre l'instance simulée in-process :
//! test de connexion, envoi multipart avec démarrage immédiat, contrôles
//! pause/reprise/annulation, en-tête `X-Api-Key`, et suivi d'état par WebSocket.

mod moonraker_mock;

use backend::adapters::moonraker::{MoonrakerClient, MoonrakerError};
use moonraker_mock::MockServer;

#[tokio::test]
async fn server_info_reports_connection() {
    let mock = MockServer::start().await;
    let client = MoonrakerClient::new(mock.base_url(), None);

    let info = client.server_info().await.unwrap();
    assert!(info.klippy_connected);
    assert_eq!(info.klippy_state, "ready");
    assert_eq!(info.moonraker_version, "v0.9.3-mock");
}

#[tokio::test]
async fn upload_sends_multipart_with_immediate_start() {
    let mock = MockServer::start().await;
    let client = MoonrakerClient::new(mock.base_url(), None);

    let data = b"; G-code\nG28\n".to_vec();
    let result = client
        .upload("part.gcode", data.clone(), true)
        .await
        .unwrap();
    assert_eq!(result.path, "part.gcode");
    assert!(result.print_started);

    let uploads = mock.uploads();
    assert_eq!(uploads.len(), 1);
    assert_eq!(uploads[0].filename, "part.gcode");
    assert_eq!(uploads[0].size, data.len());
    assert!(uploads[0].start_now, "champ print=true attendu");

    // Un envoi avec démarrage lance aussi l'impression côté hôte.
    assert_eq!(mock.actions(), vec!["print".to_string()]);
}

#[tokio::test]
async fn upload_without_start_does_not_print() {
    let mock = MockServer::start().await;
    let client = MoonrakerClient::new(mock.base_url(), None);

    let result = client
        .upload("part.gcode", b"G28\n".to_vec(), false)
        .await
        .unwrap();
    assert!(!result.print_started);
    assert!(!mock.uploads()[0].start_now);
    assert!(mock.actions().is_empty());
}

#[tokio::test]
async fn print_controls_reach_the_host() {
    let mock = MockServer::start().await;
    let client = MoonrakerClient::new(mock.base_url(), None);

    client.pause().await.unwrap();
    client.resume().await.unwrap();
    client.cancel().await.unwrap();

    assert_eq!(
        mock.actions(),
        vec![
            "pause".to_string(),
            "resume".to_string(),
            "cancel".to_string()
        ]
    );
}

#[tokio::test]
async fn api_key_is_sent_and_enforced() {
    let mock = MockServer::start_with_key("secret-key").await;

    // Sans clé → 401 propre.
    let anon = MoonrakerClient::new(mock.base_url(), None);
    match anon.server_info().await {
        Err(MoonrakerError::Status(401)) => {}
        other => panic!("401 attendu sans clé, obtenu {other:?}"),
    }

    // Avec la bonne clé → succès, et la clé est transmise à l'envoi.
    let client = MoonrakerClient::new(mock.base_url(), Some("secret-key".into()));
    assert!(client.server_info().await.unwrap().klippy_connected);
    client
        .upload("x.gcode", b"G28\n".to_vec(), false)
        .await
        .unwrap();
    assert_eq!(
        mock.uploads()[0].api_key.as_deref(),
        Some("secret-key"),
        "l'en-tête X-Api-Key doit accompagner l'envoi"
    );
}

#[tokio::test]
async fn network_failure_surfaces_cleanly() {
    // Port fermé : aucune écoute → erreur réseau, pas de panique (FR-062).
    let client = MoonrakerClient::new("http://127.0.0.1:1", None);
    assert!(matches!(
        client.server_info().await,
        Err(MoonrakerError::Network(_))
    ));
}

#[tokio::test]
async fn query_status_polls_current_state() {
    let mock = MockServer::start().await;
    let client = MoonrakerClient::new(mock.base_url(), None);

    let status = client.query_status().await.unwrap();
    assert_eq!(status.state, "printing");
    assert_eq!(status.filename.as_deref(), Some("benchy.gcode"));
    assert!((status.progress - 0.25).abs() < 1e-9);
    assert!((status.extruder_temp - 210.0).abs() < 1e-9);
    assert!((status.bed_target - 60.0).abs() < 1e-9);
}

#[tokio::test]
async fn websocket_subscription_streams_live_updates() {
    let mock = MockServer::start().await;
    let client = MoonrakerClient::new(mock.base_url(), None);

    let mut sub = client.subscribe().await.unwrap();

    // Premier instantané : état complet renvoyé à l'abonnement.
    let first = sub.next().await.unwrap();
    assert_eq!(first.state, "printing");
    assert!((first.progress - 0.25).abs() < 1e-9);

    // Deuxième instantané : mise à jour partielle fusionnée (progression 0.5).
    let second = sub.next().await.unwrap();
    assert!((second.progress - 0.5).abs() < 1e-9);
    // Les champs non mis à jour sont conservés.
    assert_eq!(second.state, "printing");
    assert_eq!(second.filename.as_deref(), Some("benchy.gcode"));

    sub.close();
}
