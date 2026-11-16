//! Boucle du worker persistant (`engine-worker serve`) : JSON-lines sur stdin,
//! réponses `R <json>` sur stdout. Sessions de peinture bornées (LRU) en
//! nombre et en triangles cumulés ; un trait = deserialize → select_patch →
//! serialize, comme le gizmo de peinture d'Orca.

use std::collections::HashMap;
use std::io::{BufRead, Write};

use crate::api::paint::{ServeOp, ServeRequest, ServeResponse};

use super::paint::PaintSession;

/// Limites mémoire du worker (le backend borne le nombre de workers).
pub const MAX_SESSIONS: usize = 8;
pub const MAX_TOTAL_TRIANGLES: usize = 4_000_000;
/// Ligne de requête maximale (maillage inclus).
pub const MAX_LINE_BYTES: usize = 256 * 1024 * 1024;

struct Slot {
    session: PaintSession,
    last_use: u64,
}

/// État du serveur (testable sans process).
#[derive(Default)]
pub struct Server {
    sessions: HashMap<String, Slot>,
    clock: u64,
}

impl Server {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    fn total_triangles(&self) -> usize {
        self.sessions.values().map(|s| s.session.triangles()).sum()
    }

    fn evict_for(&mut self, incoming: usize) {
        while !self.sessions.is_empty()
            && (self.sessions.len() >= MAX_SESSIONS
                || self.total_triangles() + incoming > MAX_TOTAL_TRIANGLES)
        {
            let oldest = self
                .sessions
                .iter()
                .min_by_key(|(_, s)| s.last_use)
                .map(|(k, _)| k.clone())
                .expect("non vide");
            self.sessions.remove(&oldest);
        }
    }

    pub fn handle(&mut self, req: ServeRequest) -> ServeResponse {
        self.clock += 1;
        let id = req.id;
        let now = self.clock;
        match req.op {
            ServeOp::Ping => ServeResponse::ok(id),
            ServeOp::PaintOpen { session, mesh } => {
                self.sessions.remove(&session);
                self.evict_for(mesh.indices.len());
                match PaintSession::new(&mesh) {
                    Ok(s) => {
                        let triangles = s.triangles();
                        self.sessions.insert(
                            session,
                            Slot {
                                session: s,
                                last_use: now,
                            },
                        );
                        ServeResponse {
                            triangles: Some(triangles),
                            ..ServeResponse::ok(id)
                        }
                    }
                    Err(e) => ServeResponse::err(id, e.message),
                }
            }
            ServeOp::PaintClose { session } => {
                self.sessions.remove(&session);
                ServeResponse::ok(id)
            }
            ServeOp::PaintStroke {
                session,
                channel,
                facets,
                stroke,
            } => {
                let Some(slot) = self.sessions.get_mut(&session) else {
                    return ServeResponse::err(id, "session de peinture inconnue (expirée ?)");
                };
                slot.last_use = now;
                let result = (|| {
                    if let Some(facets) = &facets {
                        slot.session.load(channel, facets)?;
                    }
                    slot.session.stroke(channel, &stroke)?;
                    slot.session.export(channel)
                })();
                match result {
                    Ok(facets) => ServeResponse {
                        facets: Some(facets),
                        ..ServeResponse::ok(id)
                    },
                    Err(e) => ServeResponse::err(id, e.message),
                }
            }
            ServeOp::LayerProfile { request } => match super::layer_profile(&request) {
                Ok(profile) => ServeResponse {
                    profile: Some(profile),
                    ..ServeResponse::ok(id)
                },
                Err(e) => ServeResponse::err(id, e.message),
            },
        }
    }
}

/// Boucle principale : s'arrête à la fin de stdin.
pub fn serve(input: impl BufRead, mut output: impl Write) -> std::io::Result<()> {
    let mut server = Server::new();
    for line in input.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let response = if line.len() > MAX_LINE_BYTES {
            ServeResponse::err(0, "requête trop volumineuse")
        } else {
            match serde_json::from_str::<ServeRequest>(&line) {
                Ok(req) => server.handle(req),
                Err(e) => {
                    let id = serde_json::from_str::<serde_json::Value>(&line)
                        .ok()
                        .and_then(|v| v.get("id").and_then(serde_json::Value::as_u64))
                        .unwrap_or(0);
                    ServeResponse::err(id, format!("requête invalide : {e}"))
                }
            }
        };
        let json = serde_json::to_string(&response).expect("réponse sérialisable");
        writeln!(output, "R {json}")?;
        output.flush()?;
    }
    Ok(())
}
