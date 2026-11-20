//! Entités du domaine (miroir de data-model.md). Structures pures, sans
//! dépendance au stockage : le trait `Storage` les fait transiter.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;

use super::id::{
    GcodeId, InvitationId, JobId, ModelId, PaCalibrationId, PresetId, PrinterId, ProjectId, UserId,
};

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
    /// Quota de stockage par compte (octets, tout l'espace fichiers du
    /// compte) ; 0 = illimité (lot 32).
    pub storage_quota_bytes: i64,
    /// Nombre maximal de tranchages simultanés par compte (jobs `queued` +
    /// `running`) ; 0 = illimité (lot 32).
    pub max_concurrent_slices: i64,
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
    Amf,
    Svg,
    Drc,
}

impl ModelFormat {
    /// **Tous** les formats sont désormais décodés par le moteur (source de
    /// vérité unique, retournement R7/T124) : plus de parseur STL serveur ni
    /// d'aperçu JS client. Chaque import déclenche une conversion asynchrone et
    /// reste `conversion_pending` jusqu'à l'event `model.converted`.
    pub fn needs_engine_conversion(self) -> bool {
        match self {
            Self::Stl
            | Self::Obj
            | Self::ThreeMf
            | Self::Step
            | Self::Amf
            | Self::Svg
            | Self::Drc => true,
        }
    }
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
    /// Maillage converti (décodé par le moteur, R7).
    pub mesh_path: Option<String>,
    pub size_bytes: i64,
    pub triangle_count: i64,
    pub repair_report: Option<Value>,
    /// Message d'échec de conversion moteur (None = pas d'échec). Persistée pour
    /// distinguer « en cours » (409) de « échouée » (422) au service `/mesh`.
    pub conversion_error: Option<String>,
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
    /// Anciens noms du preset système (`renamed_from` Orca, lot 35) — vide
    /// pour les presets utilisateur.
    pub renamed_from: Vec<String>,
    pub values: Value,
}

// --- Imprimantes -------------------------------------------------------------

/// Imprimante déclarée (FR-060).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Printer {
    pub id: PrinterId,
    pub user_id: UserId,
    pub name: String,
    /// URL Moonraker — `None` si l'imprimante n'est pas connectée en réseau
    /// (impression carte SD / USB ; connexion ajoutée plus tard).
    pub moonraker_url: Option<String>,
    /// Chiffrée au repos (clé d'instance).
    pub api_key: Option<String>,
    pub machine_preset_id: PresetId,
    /// Type d'hôte d'impression (host_type d'Orca : moonraker, octoprint,
    /// prusalink, prusaconnect, duet, repetier, mks, flashair, astrobox, esp3d).
    pub host_type: String,
    /// Options de l'hôte : { auth_user, user, password (chiffré), port }.
    pub host_options: Value,
}

/// Résultat de calibration d'avance de pression retenu (historique, comme
/// PACalibResult / CaliHistoryDialog d'Orca) : par imprimante, filament et buse.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaCalibration {
    pub id: PaCalibrationId,
    pub user_id: UserId,
    #[serde(flatten)]
    pub data: NewPaCalibration,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

/// Champs saisissables d'un résultat PA (création / mise à jour).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewPaCalibration {
    /// Imprimante déclarée (optionnelle) et preset machine utilisé.
    pub printer_id: Option<PrinterId>,
    pub machine_preset: String,
    /// Preset filament et son identifiant Orca (`filament_id`, ex. GFL99).
    pub filament_preset: String,
    pub filament_id: Option<String>,
    pub nozzle_diameter: f64,
    /// Extrudeur (0-based) et type de volume de buse (Standard / High Flow).
    pub extruder_id: i64,
    pub nozzle_volume_type: Option<String>,
    /// Valeur d'avance de pression (K) et coefficient n d'Orca.
    pub k_value: f64,
    pub n_coef: f64,
    /// Méthode : pa_line | pa_pattern | pa_tower | manual.
    pub method: String,
    /// Nom affiché (Orca : nom du résultat) et note libre.
    pub name: String,
    pub note: Option<String>,
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
    /// Scène figée au lancement. `None` uniquement pour les jobs créés avant
    /// l'introduction du snapshot (migration additive).
    pub scene: Option<Value>,
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
