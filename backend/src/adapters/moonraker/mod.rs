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
use serde::Serialize;
use serde_json::{json, Map, Value};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite::Message;
use ts_rs::TS;

/// Objets Moonraker suivis (R10). L'ordre est stable pour des requêtes
/// reproductibles.
pub const SUBSCRIBED_OBJECTS: [&str; 6] = [
    "print_stats",
    "display_status",
    "extruder",
    "heater_bed",
    "fan",
    "toolhead",
];

/// Bornes du relais `/printer/gcode/script` (onglet Appareil) : script court,
/// quelques lignes (déplacement G91/G1/G90, consignes), caractères imprimables.
pub const MAX_GCODE_SCRIPT_BYTES: usize = 2048;
pub const MAX_GCODE_SCRIPT_LINES: usize = 32;

/// Fichier G-code présent sur l'imprimante (`/server/files/list`).
#[derive(Debug, Clone, PartialEq)]
pub struct PrinterFile {
    pub path: String,
    pub size: u64,
    /// Horodatage Unix (s) de dernière modification.
    pub modified: f64,
}

/// Encodage d'un paramètre de requête (RFC 3986, caractères non réservés).
fn percent_encode(raw: &str) -> String {
    raw.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// Valide un script G-code relayé : vide autorisé (sonde de capacité de
/// l'UI), sinon longueur et nombre de lignes bornés, ASCII imprimable +
/// tabulation et sauts de ligne uniquement (pas de caractère de contrôle).
pub fn validate_gcode_script(script: &str) -> Result<(), String> {
    if script.len() > MAX_GCODE_SCRIPT_BYTES {
        return Err(format!(
            "script trop long (max {MAX_GCODE_SCRIPT_BYTES} octets)"
        ));
    }
    if script.lines().count() > MAX_GCODE_SCRIPT_LINES {
        return Err(format!("trop de lignes (max {MAX_GCODE_SCRIPT_LINES})"));
    }
    if script
        .chars()
        .any(|c| !(c == '\n' || c == '\r' || c == '\t' || (' '..='~').contains(&c)))
    {
        return Err("caractère non autorisé dans le script".into());
    }
    Ok(())
}

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
    /// `print_stats.info.current_layer` / `total_layer` (si le G-code émet
    /// SET_PRINT_STATS_INFO, comme celui d'Orca).
    pub current_layer: Option<i64>,
    pub total_layer: Option<i64>,
    /// `fan.speed` (0 → 1).
    pub fan_speed: Option<f64>,
    /// `toolhead.homed_axes` (ex. « xyz »).
    pub homed_axes: Option<String>,
}

/// Objet étiqueté du G-code en cours (Klipper `exclude_object.objects[]`,
/// défini par `EXCLUDE_OBJECT_DEFINE NAME=… CENTER=… POLYGON=…` qu'Orca émet
/// quand « Exclude objects » est actif, GCode.cpp:8104).
#[derive(Debug, Clone, Default, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct LabeledObject {
    pub name: String,
    #[ts(optional)]
    pub center: Option<[f64; 2]>,
    pub polygon: Vec<[f64; 2]>,
}

/// État Klipper `exclude_object` : objets du G-code, objets exclus, objet en
/// cours d'impression.
#[derive(Debug, Clone, Default, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct ExcludeObjectStatus {
    pub objects: Vec<LabeledObject>,
    pub excluded_objects: Vec<String>,
    #[ts(optional)]
    pub current_object: Option<String>,
}

impl ExcludeObjectStatus {
    /// Décode `status.exclude_object` (champs absents tolérés).
    pub fn from_value(v: &Value) -> Self {
        let pair = |v: &Value| -> Option<[f64; 2]> {
            let a = v.as_array()?;
            Some([a.first()?.as_f64()?, a.get(1)?.as_f64()?])
        };
        let objects = v
            .get("objects")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(|o| {
                        Some(LabeledObject {
                            name: o.get("name")?.as_str()?.to_string(),
                            center: o.get("center").and_then(pair),
                            polygon: o
                                .get("polygon")
                                .and_then(Value::as_array)
                                .map(|pts| pts.iter().filter_map(pair).collect())
                                .unwrap_or_default(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        let excluded_objects = v
            .get("excluded_objects")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(|n| n.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default();
        let current_object = v
            .get("current_object")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .map(str::to_owned);
        Self {
            objects,
            excluded_objects,
            current_object,
        }
    }
}

/// Nom d'objet acceptable dans `EXCLUDE_OBJECT NAME=…` : caractères des noms
/// qu'Orca produit (`sanitize_instance_name` : alphanumériques, `_`, `-`, `.`),
/// sans espace ni retour ligne (pas d'injection de commande dans le script).
pub fn valid_object_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 255
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
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
            current_layer: get("print_stats", "info")
                .and_then(|i| i.get("current_layer").and_then(Value::as_i64)),
            total_layer: get("print_stats", "info")
                .and_then(|i| i.get("total_layer").and_then(Value::as_i64)),
            fan_speed: get("fan", "speed").and_then(|v| v.as_f64()),
            homed_axes: get("toolhead", "homed_axes").and_then(|v| v.as_str().map(str::to_owned)),
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

    async fn get_result(&self, path: &str) -> MoonrakerResult<Value> {
        let resp = self
            .with_key(self.http.get(self.url(path)))
            .send()
            .await
            .map_err(|e| MoonrakerError::Network(e.to_string()))?;
        let body: Value = Self::ensure_ok(resp)
            .await?
            .json()
            .await
            .map_err(|e| MoonrakerError::Decode(e.to_string()))?;
        body.get("result")
            .cloned()
            .ok_or_else(|| MoonrakerError::Decode("champ `result` absent".into()))
    }

    /// `GET /server/files/list?root=gcodes` — fichiers G-code de l'imprimante.
    pub async fn list_files(&self) -> MoonrakerResult<Vec<PrinterFile>> {
        let result = self.get_result("/server/files/list?root=gcodes").await?;
        let items = result
            .as_array()
            .ok_or_else(|| MoonrakerError::Decode("liste de fichiers attendue".into()))?;
        Ok(items
            .iter()
            .filter_map(|f| {
                Some(PrinterFile {
                    path: f.get("path")?.as_str()?.to_string(),
                    size: f.get("size").and_then(Value::as_u64).unwrap_or(0),
                    modified: f.get("modified").and_then(Value::as_f64).unwrap_or(0.0),
                })
            })
            .collect())
    }

    /// `POST /printer/print/start?filename=` — lance un fichier déjà présent.
    pub async fn start_print(&self, filename: &str) -> MoonrakerResult<()> {
        let resp = self
            .with_key(self.http.post(self.url(&format!(
                "/printer/print/start?filename={}",
                percent_encode(filename)
            ))))
            .send()
            .await
            .map_err(|e| MoonrakerError::Network(e.to_string()))?;
        Self::ensure_ok(resp).await.map(|_| ())
    }

    /// `POST /printer/gcode/script` — exécute un script G-code (corps JSON).
    pub async fn gcode_script(&self, script: &str) -> MoonrakerResult<()> {
        let resp = self
            .with_key(
                self.http
                    .post(self.url("/printer/gcode/script"))
                    .json(&json!({ "script": script })),
            )
            .send()
            .await
            .map_err(|e| MoonrakerError::Network(e.to_string()))?;
        Self::ensure_ok(resp).await.map(|_| ())
    }

    /// `GET /server/webcams/list` — URL absolue de la première webcam active
    /// (instantané de préférence, sinon flux), `None` sans webcam.
    pub async fn webcam_url(&self) -> MoonrakerResult<Option<String>> {
        let result = self.get_result("/server/webcams/list").await?;
        let cams = result
            .get("webcams")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let url = cams
            .iter()
            .filter(|c| c.get("enabled").and_then(Value::as_bool).unwrap_or(true))
            .find_map(|c| {
                ["snapshot_url", "stream_url"].iter().find_map(|k| {
                    c.get(*k)
                        .and_then(Value::as_str)
                        .filter(|s| !s.is_empty())
                        .map(str::to_owned)
                })
            });
        Ok(url.map(|u| self.absolute(&u)))
    }

    /// URL Moonraker relative (`/webcam/…`) → absolue sur l'hôte déclaré.
    fn absolute(&self, url: &str) -> String {
        if url.starts_with("http://") || url.starts_with("https://") {
            return url.to_string();
        }
        let host_end = self
            .base_url
            .find("://")
            .map(|i| i + 3)
            .and_then(|start| self.base_url[start..].find('/').map(|j| start + j))
            .unwrap_or(self.base_url.len());
        let origin = &self.base_url[..host_end];
        if url.starts_with('/') {
            format!("{origin}{url}")
        } else {
            format!("{origin}/{url}")
        }
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

    /// `GET /printer/objects/query?exclude_object` — objets étiquetés du
    /// G-code en cours, exclus et courant (Klipper `[exclude_object]`).
    pub async fn exclude_object_status(&self) -> MoonrakerResult<ExcludeObjectStatus> {
        let resp = self
            .with_key(
                self.http
                    .get(self.url("/printer/objects/query?exclude_object")),
            )
            .send()
            .await
            .map_err(|e| MoonrakerError::Network(e.to_string()))?;
        let body: Value = Self::ensure_ok(resp)
            .await?
            .json()
            .await
            .map_err(|e| MoonrakerError::Decode(e.to_string()))?;
        let status = body
            .get("result")
            .and_then(|r| r.get("status"))
            .and_then(|s| s.get("exclude_object"))
            .cloned()
            .unwrap_or(Value::Null);
        Ok(ExcludeObjectStatus::from_value(&status))
    }

    /// Exclut un objet de l'impression en cours : `EXCLUDE_OBJECT NAME=<nom>`
    /// (commande Klipper ; le nom doit passer [`valid_object_name`]).
    pub async fn exclude_object(&self, name: &str) -> MoonrakerResult<()> {
        if !valid_object_name(name) {
            return Err(MoonrakerError::Decode(format!(
                "nom d'objet invalide : {name:?}"
            )));
        }
        self.gcode_script(&format!("EXCLUDE_OBJECT NAME={name}"))
            .await
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
    #[test]
    fn gcode_script_bounds() {
        assert!(validate_gcode_script("").is_ok());
        assert!(validate_gcode_script("G91\nG1 X10 F6000\nG90").is_ok());
        assert!(validate_gcode_script(&"G28\n".repeat(40)).is_err());
        assert!(validate_gcode_script(&"M117 ".repeat(500)).is_err());
        assert!(validate_gcode_script("M117 \u{7}").is_err());
        assert!(validate_gcode_script("M117 é").is_err());
    }

    #[test]
    fn status_reads_layers_fan_and_homing() {
        let mut acc = StatusAccumulator::new();
        acc.apply(&json!({
            "print_stats": { "state": "printing", "info": { "current_layer": 12, "total_layer": 50 } },
            "fan": { "speed": 0.5 },
            "toolhead": { "homed_axes": "xyz" }
        }));
        let s = acc.snapshot();
        assert_eq!((s.current_layer, s.total_layer), (Some(12), Some(50)));
        assert_eq!(s.fan_speed, Some(0.5));
        assert_eq!(s.homed_axes.as_deref(), Some("xyz"));
        assert_eq!(StatusAccumulator::new().snapshot().current_layer, None);
    }

    #[test]
    fn webcam_urls_are_made_absolute() {
        let c = MoonrakerClient::new("http://klipper.local:7125/", None);
        assert_eq!(
            c.absolute("/webcam/?action=snapshot"),
            "http://klipper.local:7125/webcam/?action=snapshot"
        );
        assert_eq!(c.absolute("http://cam/x.jpg"), "http://cam/x.jpg");
        let c = MoonrakerClient::new("http://h:7125/moonraker", None);
        assert_eq!(c.absolute("/webcam/a"), "http://h:7125/webcam/a");
    }

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
