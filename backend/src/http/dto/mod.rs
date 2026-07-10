//! DTO de l'API HTTP — source unique des types partagés avec le frontend.
//!
//! Chaque type dérive `TS` avec `#[ts(export, export_to = …)]` : la suite
//! `cargo test -p backend export_bindings` régénère
//! `frontend/src/generated/api/*.ts`. La CI vérifie la fraîcheur
//! (diff vide après régénération) — ne jamais éditer ces .ts à la main.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::domain::{InstanceSettings, Invitation, Project, User};

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

/// Corps de `DELETE /api/auth/me` — confirmation par mot de passe.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
pub struct DeleteAccountRequest {
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

/// Réglages d'instance exposés à l'admin (`GET /api/admin/instance`).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct InstanceResponse {
    /// `open` | `closed` | `invite`.
    pub registration_policy: String,
    pub upload_limit_bytes: i64,
}

impl From<InstanceSettings> for InstanceResponse {
    fn from(s: InstanceSettings) -> Self {
        Self {
            registration_policy: json_lower(&s.registration_policy),
            upload_limit_bytes: s.upload_limit_bytes,
        }
    }
}

/// Corps de `PATCH /api/admin/instance` — champs optionnels (mise à jour partielle).
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
pub struct PatchInstanceRequest {
    /// `open` | `closed` | `invite`.
    #[ts(optional)]
    pub registration_policy: Option<String>,
    #[ts(optional)]
    pub upload_limit_bytes: Option<i64>,
}

/// Corps de `POST /api/admin/users` (compte géré par l'admin).
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
pub struct AdminCreateUserRequest {
    pub email: String,
    pub password: String,
    /// `admin` | `user` (défaut `user`).
    #[ts(optional)]
    pub role: Option<String>,
}

/// Corps de `POST /api/admin/users/{id}/reset-password`.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
pub struct ResetPasswordRequest {
    pub new_password: String,
}

/// Corps de `POST /api/admin/invitations`.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
pub struct CreateInvitationRequest {
    /// Durée de validité en jours (défaut 7).
    #[ts(optional)]
    pub valid_days: Option<i64>,
}

/// Invitation émise (`POST /api/admin/invitations`).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct InvitationResponse {
    pub token: String,
    /// Expiration au format RFC 3339.
    pub expires_at: String,
}

impl From<Invitation> for InvitationResponse {
    fn from(i: Invitation) -> Self {
        Self {
            token: i.token,
            expires_at: i
                .expires_at
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_default(),
        }
    }
}

/// Corps de `POST /api/projects` — crée un projet. `scene`/`active_presets`
/// sont optionnels (documents JSON par défaut si absents).
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
pub struct CreateProjectRequest {
    pub name: String,
    #[ts(optional, type = "unknown")]
    pub scene: Option<serde_json::Value>,
    #[ts(optional, type = "unknown")]
    pub active_presets: Option<serde_json::Value>,
}

/// Corps de `PUT /api/projects/{id}` — sauvegarde avec verrou optimiste.
/// `expected_version` doit égaler la version stockée, sinon 409.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
pub struct SaveProjectRequest {
    pub expected_version: i64,
    #[ts(type = "unknown")]
    pub scene: serde_json::Value,
    #[ts(type = "unknown")]
    pub active_presets: serde_json::Value,
}

/// Corps de `PATCH /api/projects/{id}/rename`.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
pub struct RenameProjectRequest {
    pub name: String,
}

/// Représentation d'un projet renvoyée par l'API.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ProjectResponse {
    pub id: String,
    pub name: String,
    /// Version courante (verrou optimiste).
    pub version: i64,
    #[ts(type = "unknown")]
    pub scene: serde_json::Value,
    #[ts(type = "unknown")]
    pub active_presets: serde_json::Value,
    /// Vrai si une vignette est associée (servie par `…/thumbnail`).
    pub has_thumbnail: bool,
    /// Dates au format RFC 3339.
    pub created_at: String,
    pub updated_at: String,
}

impl From<Project> for ProjectResponse {
    fn from(p: Project) -> Self {
        let rfc3339 = time::format_description::well_known::Rfc3339;
        Self {
            id: p.id.to_string(),
            name: p.name,
            version: p.version,
            scene: p.scene,
            active_presets: p.active_presets,
            has_thumbnail: p.thumbnail_path.is_some(),
            created_at: p.created_at.format(&rfc3339).unwrap_or_default(),
            updated_at: p.updated_at.format(&rfc3339).unwrap_or_default(),
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
