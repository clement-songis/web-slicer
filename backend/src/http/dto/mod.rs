//! DTO de l'API HTTP — source unique des types partagés avec le frontend.
//!
//! Chaque type dérive `TS` avec `#[ts(export, export_to = …)]` : la suite
//! `cargo test -p backend export_bindings` régénère
//! `frontend/src/generated/api/*.ts`. La CI vérifie la fraîcheur
//! (diff vide après régénération) — ne jamais éditer ces .ts à la main.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Réponse de `GET /api/health`.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct HealthResponse {
    /// Toujours "ok" si le service répond.
    pub status: String,
    /// Version du backend (Cargo.toml).
    pub version: String,
}

impl HealthResponse {
    pub fn ok() -> Self {
        Self {
            status: "ok".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_response_serialise_en_json_stable() {
        let json = serde_json::to_value(HealthResponse::ok()).unwrap();
        assert_eq!(json["status"], "ok");
        assert_eq!(json["version"], env!("CARGO_PKG_VERSION"));
    }
}
