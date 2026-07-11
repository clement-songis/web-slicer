//! Bus d'événements et canal WebSocket `/api/ws` (T065, contrat http-api.md).
//!
//! Canal unique **authentifié** (session `tower-sessions`) : chaque client reçoit
//! uniquement les événements de son propre compte (isolation, SC-008). Les
//! événements sont diffusés via un `EventHub` cloisonné par compte (un canal
//! `tokio::broadcast` par utilisateur). La file de tranchage (T063) relaie sa
//! progression en implémentant le port `JobEventSink` ; les handlers HTTP (fin de
//! conversion STEP → mesh, T048) publient `model.converted` directement.

use std::collections::HashMap;
use std::sync::Mutex;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::Response;
use tokio::sync::broadcast;

use crate::domain::{JobStatus, UserId};
use crate::http::dto::ServerEvent;
use crate::http::extract::CurrentUser;
use crate::http::state::AppState;
use crate::queue::{JobEvent, JobEventSink};

/// Profondeur du canal par compte : au-delà, les événements les plus anciens
/// sont perdus (le client reçoit un `Lagged` puis reprend le flux courant).
const CHANNEL_CAPACITY: usize = 256;

/// Bus d'événements serveur→client, **cloisonné par compte** : un canal
/// `broadcast` par utilisateur. Un abonné ne reçoit que ses propres événements.
#[derive(Default)]
pub struct EventHub {
    channels: Mutex<HashMap<UserId, broadcast::Sender<ServerEvent>>>,
}

impl EventHub {
    pub fn new() -> Self {
        Self::default()
    }

    /// Émetteur du canal d'un compte (créé à la demande).
    fn sender(&self, user: UserId) -> broadcast::Sender<ServerEvent> {
        self.channels
            .lock()
            .unwrap()
            .entry(user)
            .or_insert_with(|| broadcast::channel(CHANNEL_CAPACITY).0)
            .clone()
    }

    /// Abonne un client : récepteur du canal de **son** compte.
    pub fn subscribe(&self, user: UserId) -> broadcast::Receiver<ServerEvent> {
        self.sender(user).subscribe()
    }

    /// Diffuse un événement au seul compte propriétaire. Sans abonné actif,
    /// l'événement est silencieusement ignoré (pas de canal créé → pas de fuite).
    pub fn publish(&self, user: UserId, event: ServerEvent) {
        let sender = self.channels.lock().unwrap().get(&user).cloned();
        if let Some(sender) = sender {
            let _ = sender.send(event);
        }
    }

    /// Publie `model.converted` (fin de conversion STEP → mesh, R7/T048).
    pub fn publish_model_converted(&self, user: UserId, model_id: &str, mesh_url: &str) {
        self.publish(
            user,
            ServerEvent::ModelConverted {
                model_id: model_id.to_string(),
                mesh_url: mesh_url.to_string(),
            },
        );
    }

    /// Publie `printer.status` (suivi Moonraker relayé, T076) au propriétaire.
    pub fn publish_printer_status(
        &self,
        user: UserId,
        printer_id: crate::domain::PrinterId,
        status: crate::adapters::moonraker::PrinterStatus,
    ) {
        self.publish(
            user,
            ServerEvent::PrinterStatus {
                printer_id: printer_id.to_string(),
                state: status.state,
                filename: status.filename,
                progress: status.progress,
                extruder_temp: status.extruder_temp,
                extruder_target: status.extruder_target,
                bed_temp: status.bed_temp,
                bed_target: status.bed_target,
            },
        );
    }
}

/// Adaptateur : la file de tranchage relaie ses événements via le bus. Traduit le
/// port domaine (`JobEvent`) vers le DTO de fil (`ServerEvent`).
impl JobEventSink for EventHub {
    fn publish(&self, user: UserId, event: JobEvent) {
        let wire = match event {
            JobEvent::Updated {
                id,
                status,
                progress,
                phase,
                error,
            } => ServerEvent::JobUpdated {
                id: id.to_string(),
                status: status_str(status),
                progress,
                phase,
                error,
            },
            JobEvent::Finished {
                id,
                gcode_id,
                stats,
            } => ServerEvent::JobFinished {
                id: id.to_string(),
                gcode_id: gcode_id.map(|g| g.to_string()),
                stats,
            },
        };
        // Désambiguïse la méthode inhérente de celle du trait (même nom).
        EventHub::publish(self, user, wire);
    }
}

/// Forme texte d'un statut de job (serde `rename_all = "lowercase"`).
fn status_str(status: JobStatus) -> String {
    serde_json::to_value(status)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default()
}

/// `GET /api/ws` — établit le canal temps réel. L'extraction de `CurrentUser`
/// (parts de requête) précède l'upgrade : une session absente/expirée répond
/// **401** avant toute négociation WebSocket.
pub async fn ws(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    upgrade: WebSocketUpgrade,
) -> Response {
    let rx = state.events.subscribe(user.id);
    upgrade.on_upgrade(move |socket| pump(socket, rx))
}

/// Boucle de diffusion : relaie chaque événement du canal vers la socket et
/// s'arrête à la fermeture côté client. Un retard (`Lagged`) est absorbé.
async fn pump(mut socket: WebSocket, mut rx: broadcast::Receiver<ServerEvent>) {
    loop {
        tokio::select! {
            event = rx.recv() => match event {
                Ok(event) => {
                    let Ok(text) = serde_json::to_string(&event) else { continue };
                    if socket.send(Message::Text(text.into())).await.is_err() {
                        return;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => return,
            },
            incoming = socket.recv() => match incoming {
                // Canal serveur→client : on ignore les messages entrants, on ne
                // s'arrête qu'à la fermeture (Close / flux terminé / erreur).
                Some(Ok(Message::Close(_))) | None | Some(Err(_)) => return,
                Some(Ok(_)) => {}
            },
        }
    }
}
