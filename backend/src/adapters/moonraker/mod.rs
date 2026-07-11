//! Client Moonraker (T074, R10, FR-060/061) : hôte d'impression Klipper.
//!
//! Couvre le contrat R10 : test de connexion `GET /server/info`, envoi de
//! G-code par `POST /server/files/upload` (multipart, champ `print=true` pour
//! démarrage immédiat), suivi d'état par WebSocket JSON-RPC
//! (`printer.objects.subscribe` sur `print_stats`, `display_status`,
//! `extruder`, `heater_bed`) avec repli par interrogation
//! `GET /printer/objects/query`, et contrôles `printer.print.pause|resume|cancel`
//! exposés en REST. La clé API voyage dans l'en-tête `X-Api-Key`.
//!
//! L'adaptateur ne dépend d'aucun autre module du backend : il ne parle que le
//! protocole Moonraker et remonte des types simples au domaine.

use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Map, Value};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite::Message;

/// Objets Moonraker suivis (R10). L'ordre est stable pour des requêtes
/// reproductibles.
pub const SUBSCRIBED_OBJECTS: [&str; 4] =
    ["print_stats", "display_status", "extruder", "heater_bed"];

/// Erreurs de dialogue avec Moonraker (surfacées proprement, FR-062).
#[derive(Debug, thiserror::Error)]
pub enum MoonrakerError {
    /// Échec réseau (connexion refusée, timeout, DNS…).
    #[error("erreur réseau Moonraker : {0}")]
    Network(String),
    /// Réponse HTTP non 2xx.
    #[error("Moonraker a renvoyé le statut HTTP {0}")]
    Status(u16),
    /// Corps de réponse illisible ou inattendu.
    #[error("réponse Moonraker illisible : {0}")]
    Decode(String),
}

pub type MoonrakerResult<T> = Result<T, MoonrakerError>;

/// État de l'hôte relevé par `GET /server/info` (test de connexion, FR-060).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerInfo {
    pub klippy_connected: bool,
    pub klippy_state: String,
    pub moonraker_version: String,
}

/// Résultat d'un envoi de fichier (`POST /server/files/upload`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UploadResult {
    /// Chemin du fichier tel qu'enregistré côté Moonraker.
    pub path: String,
    /// Impression démarrée immédiatement (champ `print=true` honoré).
    pub print_started: bool,
}

/// Instantané d'état d'impression, fusionné à partir des objets Moonraker.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PrinterStatus {
    /// `print_stats.state` : `standby|printing|paused|complete|cancelled|error`.
    pub state: String,
    /// `print_stats.filename` en cours (le cas échéant).
    pub filename: Option<String>,
    /// `display_status.progress` (0.0 → 1.0).
    pub progress: f64,
    /// `print_stats.print_duration` en secondes.
    pub print_duration: f64,
    pub extruder_temp: f64,
    pub extruder_target: f64,
    pub bed_temp: f64,
    pub bed_target: f64,
}

/// Accumulateur d'état : Moonraker envoie des mises à jour **partielles**
/// (`notify_status_update`) qu'il faut fusionner sur l'état complet avant d'en
/// extraire un [`PrinterStatus`].
#[derive(Debug, Default)]
pub struct StatusAccumulator {
    objects: Map<String, Value>,
}

impl StatusAccumulator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Fusionne un bloc `status` (complet ou partiel) sur l'état courant.
    pub fn apply(&mut self, status: &Value) {
        if let Value::Object(patch) = status {
            for (name, value) in patch {
                let slot = self
                    .objects
                    .entry(name.clone())
                    .or_insert_with(|| Value::Object(Map::new()));
                merge_object(slot, value);
            }
        }
    }

    /// Projette l'état courant en [`PrinterStatus`] typé.
    pub fn snapshot(&self) -> PrinterStatus {
        let get = |obj: &str, field: &str| -> Option<Value> {
            self.objects.get(obj)?.get(field).cloned()
        };
        let num = |obj: &str, field: &str| -> f64 {
            get(obj, field).and_then(|v| v.as_f64()).unwrap_or(0.0)
        };
        PrinterStatus {
            state: get("print_stats", "state")
                .and_then(|v| v.as_str().map(str::to_owned))
                .unwrap_or_default(),
            filename: get("print_stats", "filename")
                .and_then(|v| v.as_str().filter(|s| !s.is_empty()).map(str::to_owned)),
            progress: num("display_status", "progress"),
            print_duration: num("print_stats", "print_duration"),
            extruder_temp: num("extruder", "temperature"),
            extruder_target: num("extruder", "target"),
            bed_temp: num("heater_bed", "temperature"),
            bed_target: num("heater_bed", "target"),
        }
    }
}

/// Fusion récursive « patch » d'objets JSON (les scalaires écrasent).
fn merge_object(base: &mut Value, patch: &Value) {
    match (base, patch) {
        (Value::Object(b), Value::Object(p)) => {
            for (k, v) in p {
                merge_object(b.entry(k.clone()).or_insert(Value::Null), v);
            }
        }
        (slot, patch) => *slot = patch.clone(),
    }
}

/// Corps de requête d'abonnement `printer.objects.subscribe` (JSON-RPC 2.0).
pub fn subscribe_request(id: u64) -> Value {
    let objects: Map<String, Value> = SUBSCRIBED_OBJECTS
        .iter()
        .map(|o| ((*o).to_string(), Value::Null))
        .collect();
    json!({
        "jsonrpc": "2.0",
        "method": "printer.objects.subscribe",
        "params": { "objects": objects },
        "id": id,
    })
}

/// Extrait un bloc `status` d'un message JSON-RPC Moonraker (réponse d'abonnement
/// `result.status` ou notification `notify_status_update` → `params[0]`).
fn status_from_message(msg: &Value) -> Option<&Value> {
    if let Some(status) = msg.get("result").and_then(|r| r.get("status")) {
        return Some(status);
    }
    if msg.get("method").and_then(Value::as_str) == Some("notify_status_update") {
        return msg.get("params").and_then(|p| p.get(0));
    }
    None
}

/// Abonnement WebSocket actif : reçoit des [`PrinterStatus`] fusionnés et permet
/// de couper proprement le flux.
pub struct Subscription {
    rx: mpsc::Receiver<PrinterStatus>,
    task: JoinHandle<()>,
}

impl Subscription {
    /// Attend le prochain instantané d'état (None si le flux est terminé).
    pub async fn next(&mut self) -> Option<PrinterStatus> {
        self.rx.recv().await
    }

    /// Coupe l'abonnement et libère la tâche de lecture.
    pub fn close(self) {
        self.task.abort();
    }
}

/// Client REST + WebSocket d'une instance Moonraker (R10).
#[derive(Clone)]
pub struct MoonrakerClient {
    /// URL de base sans slash final (ex. `http://klipper.local:7125`).
    base_url: String,
    api_key: Option<String>,
    http: reqwest::Client,
}

impl MoonrakerClient {
    /// Construit un client pour l'URL déclarée et sa clé API éventuelle.
    pub fn new(base_url: impl Into<String>, api_key: Option<String>) -> Self {
        let base_url = base_url.into().trim_end_matches('/').to_string();
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("client HTTP Moonraker");
        Self {
            base_url,
            api_key,
            http,
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    /// Applique l'en-tête `X-Api-Key` si une clé est déclarée.
    fn with_key(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match &self.api_key {
            Some(key) => req.header("X-Api-Key", key),
            None => req,
        }
    }

    async fn ensure_ok(resp: reqwest::Response) -> MoonrakerResult<reqwest::Response> {
        let status = resp.status();
        if status.is_success() {
            Ok(resp)
        } else {
            Err(MoonrakerError::Status(status.as_u16()))
        }
    }

    /// `GET /server/info` — test de connexion (FR-060).
    pub async fn server_info(&self) -> MoonrakerResult<ServerInfo> {
        let resp = self
            .with_key(self.http.get(self.url("/server/info")))
            .send()
            .await
            .map_err(|e| MoonrakerError::Network(e.to_string()))?;
        let body: Value = Self::ensure_ok(resp)
            .await?
            .json()
            .await
            .map_err(|e| MoonrakerError::Decode(e.to_string()))?;
        let result = body
            .get("result")
            .ok_or_else(|| MoonrakerError::Decode("champ `result` absent".into()))?;
        Ok(ServerInfo {
            klippy_connected: result
                .get("klippy_connected")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            klippy_state: result
                .get("klippy_state")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            moonraker_version: result
                .get("moonraker_version")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
        })
    }

    /// `POST /server/files/upload` — envoi multipart d'un G-code (FR-061).
    /// `start_now` pose le champ `print=true` pour un démarrage immédiat.
    pub async fn upload(
        &self,
        filename: &str,
        data: Vec<u8>,
        start_now: bool,
    ) -> MoonrakerResult<UploadResult> {
        let part = reqwest::multipart::Part::bytes(data)
            .file_name(filename.to_string())
            .mime_str("application/octet-stream")
            .map_err(|e| MoonrakerError::Decode(e.to_string()))?;
        let form = reqwest::multipart::Form::new()
            .text("root", "gcodes")
            .text("print", if start_now { "true" } else { "false" })
            .part("file", part);

        let resp = self
            .with_key(self.http.post(self.url("/server/files/upload")))
            .multipart(form)
            .send()
            .await
            .map_err(|e| MoonrakerError::Network(e.to_string()))?;
        let body: Value = Self::ensure_ok(resp)
            .await?
            .json()
            .await
            .map_err(|e| MoonrakerError::Decode(e.to_string()))?;

        let path = body
            .get("item")
            .and_then(|i| i.get("path"))
            .and_then(Value::as_str)
            .unwrap_or(filename)
            .to_string();
        let print_started = body
            .get("print_started")
            .and_then(Value::as_bool)
            .unwrap_or(start_now);
        Ok(UploadResult {
            path,
            print_started,
        })
    }

    async fn print_action(&self, action: &str) -> MoonrakerResult<()> {
        let resp = self
            .with_key(
                self.http
                    .post(self.url(&format!("/printer/print/{action}"))),
            )
            .send()
            .await
            .map_err(|e| MoonrakerError::Network(e.to_string()))?;
        Self::ensure_ok(resp).await.map(|_| ())
    }

    /// `printer.print.pause` (REST : `POST /printer/print/pause`).
    pub async fn pause(&self) -> MoonrakerResult<()> {
        self.print_action("pause").await
    }

    /// `printer.print.resume`.
    pub async fn resume(&self) -> MoonrakerResult<()> {
        self.print_action("resume").await
    }

    /// `printer.print.cancel`.
    pub async fn cancel(&self) -> MoonrakerResult<()> {
        self.print_action("cancel").await
    }

    /// Repli d'interrogation `GET /printer/objects/query` (si le WS est coupé).
    pub async fn query_status(&self) -> MoonrakerResult<PrinterStatus> {
        let query = SUBSCRIBED_OBJECTS.join("&");
        let resp = self
            .with_key(
                self.http
                    .get(self.url(&format!("/printer/objects/query?{query}"))),
            )
            .send()
            .await
            .map_err(|e| MoonrakerError::Network(e.to_string()))?;
        let body: Value = Self::ensure_ok(resp)
            .await?
            .json()
            .await
            .map_err(|e| MoonrakerError::Decode(e.to_string()))?;
        let mut acc = StatusAccumulator::new();
        if let Some(status) = body.get("result").and_then(|r| r.get("status")) {
            acc.apply(status);
        }
        Ok(acc.snapshot())
    }

    /// URL WebSocket dérivée de l'URL de base (`http`→`ws`, `https`→`wss`).
    fn websocket_url(&self) -> String {
        let ws = if let Some(rest) = self.base_url.strip_prefix("https://") {
            format!("wss://{rest}")
        } else if let Some(rest) = self.base_url.strip_prefix("http://") {
            format!("ws://{rest}")
        } else {
            self.base_url.clone()
        };
        format!("{ws}/websocket")
    }

    /// Ouvre un abonnement WebSocket JSON-RPC et pousse les instantanés d'état
    /// fusionnés au fil des `notify_status_update` (R10, FR-061).
    pub async fn subscribe(&self) -> MoonrakerResult<Subscription> {
        let url = self.websocket_url();
        let (mut socket, _resp) = tokio_tungstenite::connect_async(&url)
            .await
            .map_err(|e| MoonrakerError::Network(e.to_string()))?;

        socket
            .send(Message::Text(subscribe_request(1).to_string().into()))
            .await
            .map_err(|e| MoonrakerError::Network(e.to_string()))?;

        let (tx, rx) = mpsc::channel(64);
        let task = tokio::spawn(async move {
            let mut acc = StatusAccumulator::new();
            while let Some(Ok(message)) = socket.next().await {
                let text = match message {
                    Message::Text(t) => t.to_string(),
                    Message::Ping(_) | Message::Pong(_) => continue,
                    Message::Close(_) => break,
                    _ => continue,
                };
                let Ok(value) = serde_json::from_str::<Value>(&text) else {
                    continue;
                };
                if let Some(status) = status_from_message(&value) {
                    acc.apply(status);
                    if tx.send(acc.snapshot()).await.is_err() {
                        break;
                    }
                }
            }
        });

        Ok(Subscription { rx, task })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accumulator_merges_partial_updates() {
        let mut acc = StatusAccumulator::new();
        acc.apply(&json!({
            "print_stats": { "state": "printing", "filename": "benchy.gcode", "print_duration": 12.0 },
            "display_status": { "progress": 0.25 },
            "extruder": { "temperature": 210.0, "target": 220.0 },
            "heater_bed": { "temperature": 60.0, "target": 60.0 },
        }));
        // Mise à jour partielle : seule la progression change.
        acc.apply(&json!({ "display_status": { "progress": 0.5 } }));

        let s = acc.snapshot();
        assert_eq!(s.state, "printing");
        assert_eq!(s.filename.as_deref(), Some("benchy.gcode"));
        assert!((s.progress - 0.5).abs() < 1e-9);
        assert!((s.print_duration - 12.0).abs() < 1e-9);
        assert!((s.extruder_temp - 210.0).abs() < 1e-9);
        assert!((s.bed_target - 60.0).abs() < 1e-9);
    }

    #[test]
    fn subscribe_request_lists_all_objects() {
        let req = subscribe_request(7);
        assert_eq!(req["method"], "printer.objects.subscribe");
        assert_eq!(req["id"], 7);
        let objects = &req["params"]["objects"];
        for name in SUBSCRIBED_OBJECTS {
            assert!(objects.get(name).is_some(), "objet {name} manquant");
        }
    }

    #[test]
    fn status_extracted_from_subscribe_reply_and_notification() {
        let reply = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": { "eventtime": 1.0, "status": { "print_stats": { "state": "paused" } } },
        });
        assert_eq!(
            status_from_message(&reply).unwrap()["print_stats"]["state"],
            "paused"
        );

        let notify = json!({
            "jsonrpc": "2.0",
            "method": "notify_status_update",
            "params": [{ "display_status": { "progress": 0.9 } }, 2.0],
        });
        assert_eq!(
            status_from_message(&notify).unwrap()["display_status"]["progress"],
            0.9
        );

        let unrelated = json!({ "jsonrpc": "2.0", "method": "notify_klippy_ready" });
        assert!(status_from_message(&unrelated).is_none());
    }

    #[test]
    fn websocket_url_switches_scheme() {
        let c = MoonrakerClient::new("http://host:7125", None);
        assert_eq!(c.websocket_url(), "ws://host:7125/websocket");
        let c = MoonrakerClient::new("https://host/", None);
        assert_eq!(c.websocket_url(), "wss://host/websocket");
    }
}
