//! DTO de l'API HTTP — source unique des types partagés avec le frontend.
//!
//! Chaque type dérive `TS` avec `#[ts(export, export_to = …)]` : la suite
//! `cargo test -p backend export_bindings` régénère
//! `frontend/src/generated/api/*.ts`. La CI vérifie la fraîcheur
//! (diff vide après régénération) — ne jamais éditer ces .ts à la main.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::domain::{InstanceSettings, Invitation, Model, Preset, Project, User};

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

/// Résultat d'un re-seed des presets système (`POST /api/admin/presets/reseed`).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ReseedResponse {
    /// Nombre de presets système écrits.
    pub reseeded: u32,
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

/// Modèle 3D importé, renvoyé après upload (`POST …/models`).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ModelResponse {
    pub id: String,
    #[ts(optional)]
    pub project_id: Option<String>,
    pub filename: String,
    /// `stl` | `3mf` | `step` | `obj`.
    pub format: String,
    pub size_bytes: i64,
    /// Nombre de triangles (0 si non déterminé à l'import — ex. STEP avant conversion).
    pub triangle_count: i64,
    /// STEP en attente de conversion mesh asynchrone (R7) — voir `model.converted`.
    pub conversion_pending: bool,
    /// Vrai si un maillage affichable est disponible (`…/mesh`).
    pub has_mesh: bool,
}

impl From<Model> for ModelResponse {
    fn from(m: Model) -> Self {
        use crate::domain::ModelFormat;
        let conversion_pending = m.format == ModelFormat::Step && m.mesh_path.is_none();
        Self {
            id: m.id.to_string(),
            project_id: m.project_id.map(|p| p.to_string()),
            filename: m.filename,
            format: json_lower(&m.format),
            size_bytes: m.size_bytes,
            triangle_count: m.triangle_count,
            conversion_pending,
            has_mesh: m.mesh_path.is_some(),
        }
    }
}

/// Ligne de liste de presets (léger, sans les valeurs).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PresetSummary {
    pub id: String,
    /// `machine_model` | `machine` | `filament` | `process`.
    pub kind: String,
    pub name: String,
    /// `system` | `user`.
    pub origin: String,
    #[ts(optional)]
    pub vendor: Option<String>,
    #[ts(optional)]
    pub inherits: Option<String>,
    pub instantiation: bool,
}

impl From<Preset> for PresetSummary {
    fn from(p: Preset) -> Self {
        Self {
            id: p.id.to_string(),
            kind: json_lower(&p.kind),
            name: p.name,
            origin: json_lower(&p.origin),
            vendor: p.vendor,
            inherits: p.inherits,
            instantiation: p.instantiation,
        }
    }
}

/// Preset complet avec ses valeurs brutes (format JSON Orca).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PresetDetail {
    pub id: String,
    pub kind: String,
    pub name: String,
    pub origin: String,
    #[ts(optional)]
    pub vendor: Option<String>,
    #[ts(optional)]
    pub inherits: Option<String>,
    pub instantiation: bool,
    #[ts(type = "unknown")]
    pub values: serde_json::Value,
    #[ts(optional, type = "unknown")]
    pub compatible_printers: Option<serde_json::Value>,
}

impl From<Preset> for PresetDetail {
    fn from(p: Preset) -> Self {
        Self {
            id: p.id.to_string(),
            kind: json_lower(&p.kind),
            name: p.name,
            origin: json_lower(&p.origin),
            vendor: p.vendor,
            inherits: p.inherits,
            instantiation: p.instantiation,
            values: p.values,
            compatible_printers: p.compatible_printers,
        }
    }
}

/// Valeurs effectives d'un preset après résolution d'héritage.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ResolvedPreset {
    pub id: String,
    /// Valeurs effectives aplaties (clé → valeur au format JSON simple).
    #[ts(type = "Record<string, unknown>")]
    pub values: serde_json::Value,
}

/// Corps de `POST /api/presets` — crée un preset utilisateur.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
pub struct CreatePresetRequest {
    /// `machine_model` | `machine` | `filament` | `process`.
    pub kind: String,
    pub name: String,
    #[ts(optional)]
    pub inherits: Option<String>,
    #[ts(type = "unknown")]
    pub values: serde_json::Value,
}

/// Corps de `PUT /api/presets/{id}` — renomme et/ou remplace les valeurs.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
pub struct UpdatePresetRequest {
    #[ts(optional)]
    pub name: Option<String>,
    #[ts(optional, type = "unknown")]
    pub values: Option<serde_json::Value>,
}

/// Corps de `POST /api/presets/import` — profil JSON Orca (clés legacy tolérées).
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
pub struct ImportPresetRequest {
    pub kind: String,
    pub name: String,
    #[ts(optional)]
    pub inherits: Option<String>,
    /// Valeurs Orca brutes (peuvent contenir des clés legacy, converties FR-023).
    #[ts(type = "unknown")]
    pub values: serde_json::Value,
}

// --- Outils de scène (T054) --------------------------------------------------

/// Empreinte au sol d'un objet à arranger (`POST …/arrange`).
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
pub struct ArrangeItem {
    pub id: String,
    pub width: f64,
    pub depth: f64,
}

/// Corps de `POST /api/projects/{id}/arrange`.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
pub struct ArrangeRequest {
    pub bed_width: f64,
    pub bed_depth: f64,
    /// Dégagement entre objets et vis-à-vis des bords (mm).
    pub spacing: f64,
    pub items: Vec<ArrangeItem>,
}

/// Position calculée du centre d'un objet (mm, repère plateau).
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct Placement {
    pub id: String,
    pub x: f64,
    pub y: f64,
}

/// Réponse de `POST /api/projects/{id}/arrange`.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct ArrangeResponse {
    pub placements: Vec<Placement>,
}

/// Corps de `POST /api/projects/{id}/orient`.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
pub struct OrientRequest {
    pub model_id: String,
}

/// Réponse de `POST /api/projects/{id}/orient` : rotation Euler XYZ (degrés).
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct OrientResponse {
    pub rotation: Vec<f64>,
}

/// Réponse de `POST /api/models/{id}/repair` : rapport d'analyse (FR-012).
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct RepairResponse {
    pub triangles: u32,
    pub degenerate: u32,
    pub open_edges: u32,
    pub watertight: bool,
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
