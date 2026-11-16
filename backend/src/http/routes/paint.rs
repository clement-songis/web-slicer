//! `GET /api/projects/{id}/paint` — canal WebSocket des interactions continues
//! calculées par le moteur (worker persistant `engine-worker serve`, pool borné
//! `engine::serve_pool`) :
//! - peinture : `TriangleSelector` d'Orca (deserialize → select_patch →
//!   serialize) ; le client garde un aperçu TS instantané et remplace l'état
//!   par le résultat moteur au relâchement — c'est lui qui est persisté ;
//! - pinceau de hauteur de couche : `adjust_layer_height_profile` (Slicing.cpp:518).
//!
//! Authentification : `CurrentUser` + projet du compte, extraits AVANT l'upgrade
//! (401/404 sans négociation WebSocket), comme `/api/ws`.
//!
//! Messages client → serveur (JSON texte, `seq` renvoyé tel quel) :
//! - `{type:"open", seq, object_id, model_id}` → `{type:"opened", seq, object_id, triangles}`
//! - `{type:"stroke", seq, object_id, channel, facets, stroke}` → `{type:"facets", seq, object_id, channel, facets}`
//! - `{type:"layer_adjust", seq, model_id, values, matrix?, profile, z, delta, band_width, action}`
//!   → `{type:"profile", seq, profile}`
//! - erreur : `{type:"error", seq, message}`.

use std::collections::HashMap;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::response::Response;
use engine::api::layer_profile::{LayerEditAction, LayerProfileOp, LayerProfileRequest};
use engine::api::paint::{PaintFacets, PaintStroke, ServeOp};
use engine::api::PaintChannel;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::domain::{ModelId, ProjectId, UserId};
use crate::engine::serve_pool::ServePool;
use crate::http::error::{ApiError, ApiResult};
use crate::http::extract::CurrentUser;
use crate::http::state::AppState;

/// Taille maximale d'un message client (facettes d'un canal incluses).
const MAX_MESSAGE_BYTES: usize = 32 * 1024 * 1024;

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClientMsg {
    Open {
        seq: u64,
        object_id: String,
        model_id: String,
    },
    Stroke {
        seq: u64,
        object_id: String,
        channel: PaintChannel,
        #[serde(default)]
        facets: PaintFacets,
        stroke: PaintStroke,
    },
    LayerAdjust {
        seq: u64,
        model_id: String,
        #[serde(default)]
        values: Value,
        #[serde(default)]
        matrix: Option<Vec<f64>>,
        #[serde(default)]
        profile: Vec<f64>,
        z: f64,
        delta: f64,
        band_width: f64,
        action: LayerEditAction,
    },
}

pub async fn paint_ws(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(project_raw): Path<String>,
    upgrade: WebSocketUpgrade,
) -> ApiResult<Response> {
    let project_id = uuid::Uuid::parse_str(&project_raw)
        .map(ProjectId)
        .map_err(|_| ApiError::not_found("Projet"))?;
    state.storage.projects().get(user.id, project_id).await?;
    Ok(upgrade
        .max_message_size(MAX_MESSAGE_BYTES)
        .on_upgrade(move |socket| run(socket, state, user.id, project_id)))
}

struct Ctx {
    state: AppState,
    user: UserId,
    project: ProjectId,
    /// object_id → model_id ouvert (réouverture transparente si le worker a
    /// été évincé / a expiré).
    opened: HashMap<String, String>,
}

impl Ctx {
    fn pool_key(&self) -> String {
        self.project.0.to_string()
    }
    fn session(&self, object_id: &str) -> String {
        format!("{}/{}", self.project.0, object_id)
    }

    async fn model_mesh(
        &self,
        model_raw: &str,
    ) -> Result<(String, engine::api::TriangleMesh), String> {
        let id = uuid::Uuid::parse_str(model_raw)
            .map(ModelId)
            .map_err(|_| "modèle inconnu".to_string())?;
        let model = self
            .state
            .storage
            .models()
            .get(self.user, id)
            .await
            .map_err(|_| "modèle inconnu".to_string())?;
        if model.project_id != Some(self.project) {
            return Err("modèle hors du projet".into());
        }
        let mesh = super::mesh_ops::load_mesh(&self.state, &model)
            .await
            .map_err(|e| e.body.message)?;
        Ok((model.filename.clone(), super::mesh_ops::centered(mesh)))
    }

    async fn open(&mut self, object_id: &str, model_id: &str) -> Result<usize, String> {
        let (_, mesh) = self.model_mesh(model_id).await?;
        let resp = ServePool::global()
            .call(
                &self.pool_key(),
                ServeOp::PaintOpen {
                    session: self.session(object_id),
                    mesh,
                },
            )
            .await
            .map_err(|e| e.to_string())?;
        if !resp.ok {
            return Err(resp.error.unwrap_or_default());
        }
        self.opened
            .insert(object_id.to_string(), model_id.to_string());
        Ok(resp.triangles.unwrap_or(0))
    }

    async fn stroke(
        &mut self,
        object_id: &str,
        channel: PaintChannel,
        facets: PaintFacets,
        stroke: PaintStroke,
    ) -> Result<PaintFacets, String> {
        stroke.validate()?;
        for attempt in 0..2 {
            let op = ServeOp::PaintStroke {
                session: self.session(object_id),
                channel,
                facets: Some(facets.clone()),
                stroke: stroke.clone(),
            };
            match ServePool::global().call(&self.pool_key(), op).await {
                Ok(resp) if resp.ok => return Ok(resp.facets.unwrap_or_default()),
                Ok(resp) => {
                    let message = resp.error.unwrap_or_default();
                    // Session évincée/expirée : on rouvre une fois (l'état
                    // client est renvoyé à chaque trait, rien n'est perdu).
                    if attempt == 0 && message.contains("session de peinture inconnue") {
                        if let Some(model) = self.opened.get(object_id).cloned() {
                            self.open(object_id, &model).await?;
                            continue;
                        }
                    }
                    return Err(message);
                }
                Err(e) if attempt == 0 => {
                    let Some(model) = self.opened.get(object_id).cloned() else {
                        return Err(e.to_string());
                    };
                    self.open(object_id, &model).await?;
                }
                Err(e) => return Err(e.to_string()),
            }
        }
        Err("session de peinture inconnue".into())
    }

    #[allow(clippy::too_many_arguments)]
    async fn layer_adjust(
        &self,
        model_id: &str,
        values: &Value,
        matrix: Option<&[f64]>,
        profile: Vec<f64>,
        z: f64,
        delta: f64,
        band_width: f64,
        action: LayerEditAction,
    ) -> Result<Vec<f64>, String> {
        let (name, mesh) = self.model_mesh(model_id).await?;
        let mut mat = glam::DMat4::IDENTITY;
        if let Some(m) = matrix {
            if m.len() != 16 || m.iter().any(|v| !v.is_finite()) {
                return Err("matrice d'instance invalide".into());
            }
            let mut cols = [0.0; 16];
            cols.copy_from_slice(m);
            mat = glam::DMat4::from_cols_array(&cols);
            mat.w_axis = glam::DVec4::new(0.0, 0.0, 0.0, 1.0);
        }
        let mut model = engine::api::Model::default();
        model.add_object(name, mesh).instances[0].matrix = mat;
        let request = LayerProfileRequest {
            model,
            config: crate::slicing::config_from_values_lenient(values),
            op: LayerProfileOp::Adjust {
                profile,
                z,
                delta,
                band_width,
                action,
            },
        };
        request.op.validate()?;
        let resp = ServePool::global()
            .call(&self.pool_key(), ServeOp::LayerProfile { request })
            .await
            .map_err(|e| e.to_string())?;
        if !resp.ok {
            return Err(resp.error.unwrap_or_default());
        }
        Ok(resp.profile.unwrap_or_default())
    }

    async fn handle(&mut self, text: &str) -> Value {
        let msg: ClientMsg = match serde_json::from_str(text) {
            Ok(m) => m,
            Err(e) => {
                let seq = serde_json::from_str::<Value>(text)
                    .ok()
                    .and_then(|v| v.get("seq").and_then(Value::as_u64))
                    .unwrap_or(0);
                return json!({ "type": "error", "seq": seq, "message": format!("message invalide : {e}") });
            }
        };
        match msg {
            ClientMsg::Open {
                seq,
                object_id,
                model_id,
            } => match self.open(&object_id, &model_id).await {
                Ok(triangles) => {
                    json!({ "type": "opened", "seq": seq, "object_id": object_id, "triangles": triangles })
                }
                Err(message) => json!({ "type": "error", "seq": seq, "message": message }),
            },
            ClientMsg::Stroke {
                seq,
                object_id,
                channel,
                facets,
                stroke,
            } => match self.stroke(&object_id, channel, facets, stroke).await {
                Ok(facets) => {
                    json!({ "type": "facets", "seq": seq, "object_id": object_id, "channel": channel, "facets": facets })
                }
                Err(message) => json!({ "type": "error", "seq": seq, "message": message }),
            },
            ClientMsg::LayerAdjust {
                seq,
                model_id,
                values,
                matrix,
                profile,
                z,
                delta,
                band_width,
                action,
            } => {
                match self
                    .layer_adjust(
                        &model_id,
                        &values,
                        matrix.as_deref(),
                        profile,
                        z,
                        delta,
                        band_width,
                        action,
                    )
                    .await
                {
                    Ok(profile) => json!({ "type": "profile", "seq": seq, "profile": profile }),
                    Err(message) => json!({ "type": "error", "seq": seq, "message": message }),
                }
            }
        }
    }
}

async fn run(mut socket: WebSocket, state: AppState, user: UserId, project: ProjectId) {
    let mut ctx = Ctx {
        state,
        user,
        project,
        opened: HashMap::new(),
    };
    while let Some(Ok(message)) = socket.recv().await {
        let reply = match message {
            Message::Text(text) => ctx.handle(&text).await,
            Message::Close(_) => break,
            _ => continue,
        };
        if socket
            .send(Message::Text(reply.to_string().into()))
            .await
            .is_err()
        {
            break;
        }
    }
    // Libère les sessions de cette connexion (le worker reste pour le projet).
    for object_id in ctx.opened.keys().cloned().collect::<Vec<_>>() {
        let _ = ServePool::global()
            .call(
                &ctx.pool_key(),
                ServeOp::PaintClose {
                    session: ctx.session(&object_id),
                },
            )
            .await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_messages_parse() {
        let m: ClientMsg = serde_json::from_value(json!({
            "type": "layer_adjust", "seq": 3, "model_id": "x", "z": 5.0, "delta": 0.005,
            "band_width": 2.0, "action": "increase"
        }))
        .unwrap();
        assert!(matches!(
            m,
            ClientMsg::LayerAdjust {
                seq: 3,
                action: LayerEditAction::Increase,
                ..
            }
        ));
        let m: ClientMsg = serde_json::from_value(json!({
            "type": "stroke", "seq": 1, "object_id": "o", "channel": "seam",
            "stroke": { "points": [[0,0,0]], "facets": [0], "camera_pos": [0,0,10], "radius": 2,
                        "cursor": "circle", "state": 1, "trafo": [1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1] }
        }))
        .unwrap();
        assert!(matches!(
            m,
            ClientMsg::Stroke {
                channel: PaintChannel::Seam,
                ..
            }
        ));
    }
}
