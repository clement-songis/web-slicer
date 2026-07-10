//! DTO de l'API HTTP — source unique des types partagés avec le frontend.
//!
//! Chaque type dérive `TS` avec `#[ts(export, export_to = …)]` : la suite
//! `cargo test -p backend export_bindings` régénère
//! `frontend/src/generated/api/*.ts`. La CI vérifie la fraîcheur
//! (diff vide après régénération) — ne jamais éditer ces .ts à la main.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::domain::User;

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

/// Corps de `POST /api/auth/register`.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    /// Jeton d'invitation (politique `invite`).
    #[ts(optional)]
    pub invite_token: Option<String>,
}

/// Corps de `POST /api/auth/login`.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

/// Représentation publique d'un compte (jamais le hash).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct UserResponse {
    pub id: String,
    pub email: String,
    /// `admin` | `user`.
    pub role: String,
    /// `active` | `disabled`.
    pub status: String,
}

impl From<User> for UserResponse {
    fn from(u: User) -> Self {
        Self {
            id: u.id.to_string(),
            email: u.email,
            role: json_lower(&u.role),
            status: json_lower(&u.status),
        }
    }
}

/// Sérialise un enum unité du domaine en sa forme texte (serde `rename_all`).
fn json_lower<T: Serialize>(v: &T) -> String {
    serde_json::to_value(v)
        .ok()
        .and_then(|x| x.as_str().map(str::to_string))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_response_serializes_to_stable_json() {
        let json = serde_json::to_value(HealthResponse::ok()).unwrap();
        assert_eq!(json["status"], "ok");
        assert_eq!(json["version"], env!("CARGO_PKG_VERSION"));
    }
}
