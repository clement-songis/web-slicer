//! Relais de suivi d'impression (T076/US8) : abonne le backend au WebSocket
//! JSON-RPC d'une imprimante Moonraker et republie chaque instantané d'état sur
//! le canal applicatif `/api/ws` du **propriétaire** (événements `printer.status`).
//!
//! Isolation (SC-008) : chaque relais est lié à un compte ; les événements
//! transitent par [`EventHub::publish_printer_status`], cloisonné par compte.
//! Un seul relais par imprimante (dédoublonnage) ; la tâche se retire d'elle-même
//! quand la connexion Moonraker se termine.

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use crate::adapters::moonraker::MoonrakerClient;
use crate::domain::{PrinterId, UserId};
use crate::http::ws::EventHub;

/// Registre des relais actifs (au plus un par imprimante).
#[derive(Default)]
pub struct PrinterRelays {
    active: Mutex<HashSet<PrinterId>>,
}

impl PrinterRelays {
    pub fn new() -> Self {
        Self::default()
    }

    /// Nombre de relais actuellement actifs (diagnostic/tests).
    pub fn active_count(&self) -> usize {
        self.active.lock().unwrap().len()
    }

    /// Démarre un relais pour cette imprimante s'il n'en existe pas déjà un
    /// (idempotent). La clé API est déjà déchiffrée dans `client`.
    pub fn ensure(
        self: &Arc<Self>,
        events: Arc<EventHub>,
        owner: UserId,
        printer_id: PrinterId,
        client: MoonrakerClient,
    ) {
        // Dédoublonnage : un seul relais par imprimante.
        if !self.active.lock().unwrap().insert(printer_id) {
            return;
        }
        let relays = Arc::clone(self);
        tokio::spawn(async move {
            relay_loop(&events, owner, printer_id, client).await;
            // Fin de connexion : on libère la place pour un futur relais.
            relays.active.lock().unwrap().remove(&printer_id);
        });
    }
}

/// Boucle de relais : chaque instantané reçu de Moonraker devient un
/// `printer.status` sur le canal du propriétaire.
async fn relay_loop(
    events: &EventHub,
    owner: UserId,
    printer_id: PrinterId,
    client: MoonrakerClient,
) {
    let Ok(mut subscription) = client.subscribe().await else {
        // Hôte injoignable : rien à relayer, on abandonne proprement (FR-062).
        return;
    };
    while let Some(status) = subscription.next().await {
        events.publish_printer_status(owner, printer_id, status);
    }
}
