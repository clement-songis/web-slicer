//! Relais de suivi d'impression (T076/US8) : l'abonnement WebSocket Moonraker
//! est republié sur le canal `/api/ws` du **propriétaire** uniquement — un autre
//! compte ne voit jamais l'état de l'imprimante d'autrui (isolation SC-008).

mod moonraker_mock;

use std::sync::Arc;
use std::time::Duration;

use backend::adapters::moonraker::MoonrakerClient;
use backend::domain::{PrinterId, UserId};
use backend::http::dto::ServerEvent;
use backend::http::printer_relay::PrinterRelays;
use backend::http::ws::EventHub;
use moonraker_mock::MockServer;

#[tokio::test]
async fn relay_publishes_status_only_to_the_owner() {
    let events = Arc::new(EventHub::new());
    let relays = Arc::new(PrinterRelays::new());
    let owner = UserId(uuid::Uuid::new_v4());
    let other = UserId(uuid::Uuid::new_v4());
    let printer = PrinterId(uuid::Uuid::new_v4());

    // Deux abonnés (crée leurs canaux) avant de démarrer le relais.
    let mut rx_owner = events.subscribe(owner);
    let mut rx_other = events.subscribe(other);

    let mock = MockServer::start().await;
    let client = MoonrakerClient::new(mock.base_url(), None);
    relays.ensure(Arc::clone(&events), owner, printer, client);

    // Le propriétaire reçoit un `printer.status` étiqueté de son imprimante.
    let event = tokio::time::timeout(Duration::from_secs(5), rx_owner.recv())
        .await
        .expect("le propriétaire reçoit l'état à temps")
        .expect("canal ouvert");
    match event {
        ServerEvent::PrinterStatus {
            printer_id, state, ..
        } => {
            assert_eq!(printer_id, printer.to_string());
            assert_eq!(state, "printing");
        }
        other => panic!("printer.status attendu, reçu {other:?}"),
    }

    // L'autre compte ne reçoit rien (cloisonnement SC-008).
    assert!(
        tokio::time::timeout(Duration::from_millis(300), rx_other.recv())
            .await
            .is_err(),
        "aucun événement ne doit fuir vers un autre compte"
    );
}

#[tokio::test]
async fn ensure_starts_at_most_one_relay_per_printer() {
    let events = Arc::new(EventHub::new());
    let relays = Arc::new(PrinterRelays::new());
    let owner = UserId(uuid::Uuid::new_v4());
    let printer = PrinterId(uuid::Uuid::new_v4());
    let _rx = events.subscribe(owner);

    let mock = MockServer::start().await;
    relays.ensure(
        Arc::clone(&events),
        owner,
        printer,
        MoonrakerClient::new(mock.base_url(), None),
    );
    // Deuxième appel : dédoublonné, aucun relais supplémentaire.
    relays.ensure(
        Arc::clone(&events),
        owner,
        printer,
        MoonrakerClient::new(mock.base_url(), None),
    );
    assert_eq!(relays.active_count(), 1);
}
