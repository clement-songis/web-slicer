//! Entités du domaine (miroir de data-model.md). Structures pures, sans
//! dépendance au stockage : le trait `Storage` les fait transiter.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;

use super::id::{GcodeId, InvitationId, JobId, ModelId, PresetId, PrinterId, ProjectId, UserId};

// --- Utilisateurs & instance -------------------------------------------------

/// Rôle d'un compte. Le premier compte créé est `Admin`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Admin,
    User,
}

/// État d'un compte.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UserStatus {
    Active,
    Disabled,
}

/// Compte utilisateur. `password_hash` (argon2id) n'est jamais exposé par l'API.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct User {
    pub id: UserId,
    /// Normalisé en minuscules ; identité de connexion, unique.
    pub email: String,
    pub password_hash: String,
    pub role: Role,
    pub status: UserStatus,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

/// Politique d'inscription de l'instance (clarification).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RegistrationPolicy {
    Open,
    Closed,
    Invite,
}

/// Réglages d'instance (table singleton).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstanceSettings {
    pub registration_policy: RegistrationPolicy,
    /// Limite d'upload par fichier (octets ; défaut 500 Mo).
    pub upload_limit_bytes: i64,
}

/// Invitation à usage unique (politique `invite`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Invitation {
    pub id: InvitationId,
    pub token: String,
    pub issued_by: UserId,
    pub expires_at: OffsetDateTime,
    pub used: bool,
    pub created_at: OffsetDateTime,
}

// --- Projets & modèles -------------------------------------------------------

/// Projet (bibliothèque de l'utilisateur). `scene` et `active_presets` sont
/// des documents JSON versionnés (voir data-model.md).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Project {
    pub id: ProjectId,
    pub user_id: UserId,
    /// Unique par utilisateur.
    pub name: String,
    pub thumbnail_path: Option<String>,
    /// Verrou optimiste : incrémenté à chaque sauvegarde (409 si décalé).
    pub version: i64,
    pub scene: Value,
    pub active_presets: Value,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

/// Format d'un modèle importé.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelFormat {
    Stl,
    #[serde(rename = "3mf")]
    ThreeMf,
    Step,
    Obj,
}

/// Fichier 3D importé (source conservée + maillage converti éventuel).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Model {
    pub id: ModelId,
    pub user_id: UserId,
    /// Un modèle peut être partagé entre projets de l'utilisateur.
    pub project_id: Option<ProjectId>,
    pub filename: String,
    pub format: ModelFormat,
    pub file_path: String,
    /// Maillage converti (STEP → mesh, R7).
    pub mesh_path: Option<String>,
    pub size_bytes: i64,
    pub triangle_count: i64,
    pub repair_report: Option<Value>,
}

// --- Presets -----------------------------------------------------------------

/// Nature d'un preset (Annexe C).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PresetKind {
    MachineModel,
    Machine,
    Filament,
    Process,
}

/// Origine d'un preset : `System` (lecture seule, re-seedable) ou `User`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PresetOrigin {
    System,
    User,
}

/// Preset (profil OrcaSlicer). `values` ne contient **que** les clés
/// surchargées ; la résolution d'héritage est calculée par `engine::presets`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Preset {
    pub id: PresetId,
    pub kind: PresetKind,
    /// Unique par (kind, origin, user_id).
    pub name: String,
    pub origin: PresetOrigin,
    /// NULL pour les presets système.
    pub user_id: Option<UserId>,
    pub vendor: Option<String>,
    /// Nom du parent (chaîne Orca conservée telle quelle).
    pub inherits: Option<String>,
    /// Presets abstraits masqués si `false` (FR-020).
    pub instantiation: bool,
    pub setting_id: Option<String>,
    pub filament_id: Option<String>,
    pub compatible_printers: Option<Value>,
    pub values: Value,
}

// --- Imprimantes -------------------------------------------------------------

/// Imprimante déclarée (FR-060).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Printer {
    pub id: PrinterId,
    pub user_id: UserId,
    pub name: String,
    pub moonraker_url: String,
    /// Chiffrée au repos (clé d'instance).
    pub api_key: Option<String>,
    pub machine_preset_id: PresetId,
}

// --- File de tranchage & G-code ---------------------------------------------

/// État d'un job de tranchage (R9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

/// Job de tranchage : unité = un plateau (FR-014).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SlicingJob {
    pub id: JobId,
    pub user_id: UserId,
    pub project_id: ProjectId,
    pub plate_index: i64,
    pub status: JobStatus,
    /// Progression 0–1, relayée par WebSocket.
    pub progress: f64,
    pub phase: String,
    /// Presets résolus figés au lancement (reproductibilité).
    pub resolved_settings: Value,
    pub error: Option<Value>,
    pub gcode_id: Option<GcodeId>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

/// G-code produit + buffers de prévisualisation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Gcode {
    pub id: GcodeId,
    pub user_id: UserId,
    pub job_id: JobId,
    pub file_path: String,
    pub preview_path: String,
    pub stats: Value,
    pub thumbnails: Value,
}
