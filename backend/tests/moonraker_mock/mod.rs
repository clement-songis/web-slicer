//! Instance Moonraker simulée in-process (R10, dépendance spec) : un vrai
//! serveur axum sur un port éphémère `127.0.0.1:0` implémentant les routes REST
//! (`/server/info`, `/server/files/upload`, `/printer/print/*`,
//! `/printer/objects/query`) et le canal WebSocket JSON-RPC
//! (`printer.objects.subscribe` + `notify_status_update`). Partagé par les
//! tests du client (T074) et l'intégration CI (T078).

#![allow(dead_code)]

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Multipart, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{json, Value};
use tokio::task::JoinHandle;

/// Trace d'un envoi reçu par le mock.
#[derive(Debug, Clone)]
pub struct UploadRecord {
    pub filename: String,
    pub size: usize,
    pub start_now: bool,
    /// Clé API présentée dans l'en-tête `X-Api-Key` (le cas échéant).
    pub api_key: Option<String>,
}

#[derive(Clone)]
pub struct MockState {
    /// Clé API attendue : si posée, les routes REST exigent l'en-tête exact.
    pub expected_key: Option<String>,
    pub uploads: Arc<Mutex<Vec<UploadRecord>>>,
    pub actions: Arc<Mutex<Vec<String>>>,
    /// Objets d'état renvoyés à l'abonnement et à l'interrogation.
    pub status: Arc<Mutex<Value>>,
    /// Mise à jour partielle poussée après l'abonnement (notification WS).
    pub push_update: Arc<Mutex<Option<Value>>>,
}

pub struct MockServer {
    pub addr: SocketAddr,
    pub state: MockState,
    handle: JoinHandle<()>,
}

impl Drop for MockServer {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

impl MockServer {
    /// Démarre le mock sans clé API exigée.
    pub async fn start() -> Self {
        Self::start_inner(None).await
    }

    /// Démarre le mock en exigeant la clé API donnée sur les routes REST.
    pub async fn start_with_key(key: &str) -> Self {
        Self::start_inner(Some(key.to_string())).await
    }

    async fn start_inner(expected_key: Option<String>) -> Self {
        let state = MockState {
            expected_key,
            uploads: Arc::new(Mutex::new(Vec::new())),
            actions: Arc::new(Mutex::new(Vec::new())),
            status: Arc::new(Mutex::new(json!({
                "print_stats": {
                    "state": "printing",
                    "filename": "benchy.gcode",
                    "print_duration": 42.0
                },
                "display_status": { "progress": 0.25 },
                "extruder": { "temperature": 210.0, "target": 220.0 },
                "heater_bed": { "temperature": 60.0, "target": 60.0 }
            }))),
            push_update: Arc::new(Mutex::new(Some(json!({
                "display_status": { "progress": 0.5 }
            })))),
        };

        let app = Router::new()
            .route("/server/info", get(server_info))
            .route("/server/files/upload", post(upload))
            .route("/printer/print/pause", post(pause))
            .route("/printer/print/resume", post(resume))
            .route("/printer/print/cancel", post(cancel))
            .route("/printer/objects/query", get(query))
            .route("/websocket", get(websocket))
            .with_state(state.clone());

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        MockServer {
            addr,
            state,
            handle,
        }
    }

    pub fn base_url(&self) -> String {
        format!("http://{}", self.addr)
    }

    pub fn uploads(&self) -> Vec<UploadRecord> {
        self.state.uploads.lock().unwrap().clone()
    }

    pub fn actions(&self) -> Vec<String> {
        self.state.actions.lock().unwrap().clone()
    }
}

/// Vérifie l'en-tête `X-Api-Key` face à la clé attendue.
fn key_ok(state: &MockState, headers: &HeaderMap) -> bool {
    match &state.expected_key {
        None => true,
        Some(expected) => headers
            .get("X-Api-Key")
            .and_then(|v| v.to_str().ok())
            .map(|k| k == expected)
            .unwrap_or(false),
    }
}

async fn server_info(State(state): State<MockState>, headers: HeaderMap) -> impl IntoResponse {
    if !key_ok(&state, &headers) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "unauthorized" })),
        );
    }
    (
        StatusCode::OK,
        Json(json!({
            "result": {
                "klippy_connected": true,
                "klippy_state": "ready",
                "moonraker_version": "v0.9.3-mock"
            }
        })),
    )
}

async fn upload(
    State(state): State<MockState>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> impl IntoResponse {
    if !key_ok(&state, &headers) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "unauthorized" })),
        );
    }
    let mut filename = String::new();
    let mut size = 0usize;
    let mut start_now = false;
    while let Ok(Some(field)) = multipart.next_field().await {
        match field.name() {
            Some("file") => {
                filename = field.file_name().unwrap_or("upload.gcode").to_string();
                size = field.bytes().await.map(|b| b.len()).unwrap_or(0);
            }
            Some("print") => {
                let v = field.text().await.unwrap_or_default();
                start_now = v == "true";
            }
            _ => {
                let _ = field.bytes().await;
            }
        }
    }
    state.uploads.lock().unwrap().push(UploadRecord {
        filename: filename.clone(),
        size,
        start_now,
        api_key: headers
            .get("X-Api-Key")
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned),
    });
    if start_now {
        state.actions.lock().unwrap().push("print".to_string());
    }
    (
        StatusCode::CREATED,
        Json(json!({
            "item": { "path": filename, "root": "gcodes" },
            "print_started": start_now
        })),
    )
}

async fn print_action(state: &MockState, headers: &HeaderMap, action: &str) -> impl IntoResponse {
    if !key_ok(state, headers) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "unauthorized" })),
        );
    }
    state.actions.lock().unwrap().push(action.to_string());
    (StatusCode::OK, Json(json!({ "result": "ok" })))
}

async fn pause(State(state): State<MockState>, headers: HeaderMap) -> impl IntoResponse {
    print_action(&state, &headers, "pause").await
}

async fn resume(State(state): State<MockState>, headers: HeaderMap) -> impl IntoResponse {
    print_action(&state, &headers, "resume").await
}

async fn cancel(State(state): State<MockState>, headers: HeaderMap) -> impl IntoResponse {
    print_action(&state, &headers, "cancel").await
}

async fn query(State(state): State<MockState>, headers: HeaderMap) -> impl IntoResponse {
    if !key_ok(&state, &headers) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "unauthorized" })),
        );
    }
    let status = state.status.lock().unwrap().clone();
    (
        StatusCode::OK,
        Json(json!({ "result": { "eventtime": 1.0, "status": status } })),
    )
}

async fn websocket(State(state): State<MockState>, ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ws(socket, state))
}

async fn handle_ws(mut socket: WebSocket, state: MockState) {
    while let Some(Ok(msg)) = socket.recv().await {
        let Message::Text(text) = msg else {
            continue;
        };
        let Ok(value) = serde_json::from_str::<Value>(&text) else {
            continue;
        };
        if value.get("method").and_then(Value::as_str) == Some("printer.objects.subscribe") {
            let id = value.get("id").cloned().unwrap_or(json!(1));
            let status = state.status.lock().unwrap().clone();
            let reply = json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": { "eventtime": 1.0, "status": status }
            });
            if socket
                .send(Message::Text(reply.to_string().into()))
                .await
                .is_err()
            {
                return;
            }
            // Notification de mise à jour partielle (suivi en direct).
            let pending = state.push_update.lock().unwrap().clone();
            if let Some(update) = pending {
                let notify = json!({
                    "jsonrpc": "2.0",
                    "method": "notify_status_update",
                    "params": [update, 2.0]
                });
                let _ = socket.send(Message::Text(notify.to_string().into())).await;
            }
        }
    }
}
