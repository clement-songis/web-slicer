//! DTO de l'API HTTP — source unique des types partagés avec le frontend.
//!
//! Chaque type dérive `TS` avec `#[ts(export, export_to = …)]` : la suite
//! `cargo test -p backend export_bindings` régénère
//! `frontend/src/generated/api/*.ts`. La CI vérifie la fraîcheur
//! (diff vide après régénération) — ne jamais éditer ces .ts à la main.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::domain::{InstanceSettings, Invitation, Model, Preset, Printer, Project, User};

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
        let conversion_pending = m.format.needs_engine_conversion() && m.mesh_path.is_none();
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

// --- Tranchage (T064) --------------------------------------------------------

/// Job de tranchage renvoyé par l'API (état, progression, résultat).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct JobResponse {
    pub id: String,
    pub project_id: String,
    pub plate_index: i64,
    /// `queued` | `running` | `succeeded` | `failed` | `cancelled`.
    pub status: String,
    pub progress: f64,
    pub phase: String,
    /// G-code produit (jobs `succeeded`).
    #[ts(optional)]
    pub gcode_id: Option<String>,
    /// Détail d'échec (jobs `failed`), langage clair (FR-032).
    #[ts(optional, type = "unknown")]
    pub error: Option<serde_json::Value>,
    /// Dates au format RFC 3339 (tri file/historique).
    pub created_at: String,
    pub updated_at: String,
}

impl From<crate::domain::SlicingJob> for JobResponse {
    fn from(j: crate::domain::SlicingJob) -> Self {
        let rfc3339 = time::format_description::well_known::Rfc3339;
        Self {
            id: j.id.to_string(),
            project_id: j.project_id.to_string(),
            plate_index: j.plate_index,
            status: json_lower(&j.status),
            progress: j.progress,
            phase: j.phase,
            gcode_id: j.gcode_id.map(|g| g.to_string()),
            error: j.error,
            created_at: j.created_at.format(&rfc3339).unwrap_or_default(),
            updated_at: j.updated_at.format(&rfc3339).unwrap_or_default(),
        }
    }
}

/// Corps de `POST /api/projects/{id}/slice` : cible un plateau ou tous.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
pub struct SliceRequest {
    /// Plateau ciblé (0-based). Absent ⇒ tous les plateaux si `all`.
    #[ts(optional)]
    pub plate_index: Option<i64>,
    /// Trancher tous les plateaux préparés.
    #[ts(optional)]
    pub all: Option<bool>,
}

/// Avertissement de configuration moteur (FR-032).
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct SliceWarning {
    pub key: String,
    pub message: String,
}

/// Réponse de `POST /api/projects/{id}/slice` : jobs créés + avertissements.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct SliceResponse {
    pub jobs: Vec<JobResponse>,
    pub warnings: Vec<SliceWarning>,
}

// --- Prévisualisation G-code (T067) ------------------------------------------

/// Rôle d'extrusion présent dans la préviz (id stable ↔ nom pour la légende).
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct PreviewSegmentType {
    pub id: u32,
    pub name: String,
}

/// Méta d'une couche : hauteur `z` et nombre de segments (dimensionnement client).
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct PreviewLayerMeta {
    pub z: f64,
    pub segment_count: u32,
}

/// Méta-données de prévisualisation (`GET /api/gcodes/{id}/preview/meta`, R6) :
/// couches, types présents et échelles pour les 7 colorations (FR-041).
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct PreviewMeta {
    pub layer_count: u32,
    pub layers: Vec<PreviewLayerMeta>,
    pub types_present: Vec<PreviewSegmentType>,
    pub extruders_present: Vec<u32>,
    /// Bornes (min, max) des attributs continus (mm/min, mm).
    pub feedrate_min: f64,
    pub feedrate_max: f64,
    pub width_min: f64,
    pub width_max: f64,
    pub height_min: f64,
    pub height_max: f64,
    /// Taille d'un enregistrement segment dans les buffers `/preview/layers`.
    pub segment_record_bytes: u32,
}

impl From<crate::gcode::PreviewSummary> for PreviewMeta {
    fn from(s: crate::gcode::PreviewSummary) -> Self {
        Self {
            layer_count: s.layer_count,
            layers: s
                .layer_z
                .iter()
                .zip(s.layer_segment_counts.iter())
                .map(|(z, c)| PreviewLayerMeta {
                    z: *z as f64,
                    segment_count: *c,
                })
                .collect(),
            types_present: s
                .kinds_present
                .iter()
                .map(|id| PreviewSegmentType {
                    id: *id as u32,
                    name: crate::gcode::kind_name(*id).to_string(),
                })
                .collect(),
            extruders_present: s.extruders_present.iter().map(|e| *e as u32).collect(),
            feedrate_min: s.feedrate_range.0 as f64,
            feedrate_max: s.feedrate_range.1 as f64,
            width_min: s.width_range.0 as f64,
            width_max: s.width_range.1 as f64,
            height_min: s.height_range.0 as f64,
            height_max: s.height_range.1 as f64,
            segment_record_bytes: crate::gcode::PREVIEW_RECORD_BYTES,
        }
    }
}

// --- Événements WebSocket (T065) ---------------------------------------------

/// Événement serveur→client du canal `/api/ws` (contrat http-api.md). Le champ
/// discriminant `event` vaut `job.updated` | `job.finished` | `model.converted`.
/// Chaque événement n'est diffusé qu'au compte propriétaire (isolation).
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(tag = "event")]
pub enum ServerEvent {
    /// File en direct : progression / changement d'état d'un job.
    #[serde(rename = "job.updated")]
    JobUpdated {
        id: String,
        /// `queued` | `running` | `succeeded` | `failed` | `cancelled`.
        status: String,
        progress: f64,
        phase: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional, type = "unknown")]
        error: Option<serde_json::Value>,
    },
    /// Job terminé avec succès (notification US7-AS1).
    #[serde(rename = "job.finished")]
    JobFinished {
        id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        gcode_id: Option<String>,
        #[ts(type = "unknown")]
        stats: serde_json::Value,
    },
    /// Fin de conversion STEP → mesh (R7) : maillage désormais servi par `mesh_url`.
    #[serde(rename = "model.converted")]
    ModelConverted { model_id: String, mesh_url: String },
    /// Suivi d'impression en direct (relais Moonraker, T076). Diffusé au seul
    /// propriétaire de l'imprimante (isolation SC-008).
    #[serde(rename = "printer.status")]
    PrinterStatus {
        printer_id: String,
        state: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        filename: Option<String>,
        progress: f64,
        extruder_temp: f64,
        extruder_target: f64,
        bed_temp: f64,
        bed_target: f64,
    },
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

// --- Imprimantes (Moonraker, T075) -------------------------------------------

/// Imprimante déclarée renvoyée au client. La clé API n'est **jamais** exposée :
/// seul `has_api_key` indique sa présence (FR-060).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PrinterResponse {
    pub id: String,
    pub name: String,
    pub moonraker_url: String,
    pub has_api_key: bool,
    pub machine_preset_id: String,
}

impl From<Printer> for PrinterResponse {
    fn from(p: Printer) -> Self {
        Self {
            id: p.id.to_string(),
            name: p.name,
            moonraker_url: p.moonraker_url,
            has_api_key: p.api_key.is_some(),
            machine_preset_id: p.machine_preset_id.to_string(),
        }
    }
}

/// Corps de `POST`/`PUT /api/printers` : déclaration ou mise à jour. `api_key`
/// est chiffrée au repos avant stockage (T075).
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
pub struct SavePrinterRequest {
    pub name: String,
    pub moonraker_url: String,
    #[ts(optional)]
    pub api_key: Option<String>,
    pub machine_preset_id: String,
}

/// Résultat de `POST /api/printers/{id}/test` : relais de `GET /server/info`.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TestPrinterResponse {
    pub connected: bool,
    pub klippy_state: String,
    pub moonraker_version: String,
}

/// Corps de `POST /api/printers/{id}/upload` : envoi d'un G-code du compte.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
pub struct UploadToPrinterRequest {
    pub gcode_id: String,
    /// Démarrer l'impression immédiatement (`print=true`, FR-061).
    pub start_now: bool,
}

/// Résultat d'un envoi vers l'imprimante (`POST /api/printers/{id}/upload`).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PrinterUploadResponse {
    /// Chemin du fichier tel qu'enregistré côté Moonraker.
    pub path: String,
    /// Impression démarrée immédiatement.
    pub print_started: bool,
}

impl From<crate::adapters::moonraker::UploadResult> for PrinterUploadResponse {
    fn from(r: crate::adapters::moonraker::UploadResult) -> Self {
        Self {
            path: r.path,
            print_started: r.print_started,
        }
    }
}

/// État d'impression instantané (`GET /api/printers/{id}/status`).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PrinterStatusResponse {
    pub state: String,
    #[ts(optional)]
    pub filename: Option<String>,
    pub progress: f64,
    pub print_duration: f64,
    pub extruder_temp: f64,
    pub extruder_target: f64,
    pub bed_temp: f64,
    pub bed_target: f64,
}

impl From<crate::adapters::moonraker::PrinterStatus> for PrinterStatusResponse {
    fn from(s: crate::adapters::moonraker::PrinterStatus) -> Self {
        Self {
            state: s.state,
            filename: s.filename,
            progress: s.progress,
            print_duration: s.print_duration,
            extruder_temp: s.extruder_temp,
            extruder_target: s.extruder_target,
            bed_temp: s.bed_temp,
            bed_target: s.bed_target,
        }
    }
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
