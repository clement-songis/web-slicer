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

/// Corps de `PUT /api/me/password` (lot 32).
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    /// 8 caractères minimum, différent de l'actuel.
    pub new_password: String,
}

/// Usage et quotas du compte connecté (`GET /api/me/usage`, lot 32).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct UsageResponse {
    /// Octets occupés par l'espace fichiers du compte (modèles, maillages,
    /// G-code, vignettes, fichiers auxiliaires, G-code ouverts).
    #[ts(type = "number")]
    pub storage_bytes: i64,
    /// Quota de stockage par compte (octets) ; 0 = illimité.
    #[ts(type = "number")]
    pub storage_quota_bytes: i64,
    /// Tranchages du compte en file ou en cours.
    #[ts(type = "number")]
    pub active_slices: i64,
    /// Tranchages simultanés autorisés ; 0 = illimité.
    #[ts(type = "number")]
    pub max_concurrent_slices: i64,
}

/// Ligne de la liste des comptes (`GET /api/admin/users`, lot 32).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AdminUserResponse {
    pub id: String,
    pub email: String,
    /// `admin` | `user`.
    pub role: String,
    /// `active` | `disabled`.
    pub status: String,
    /// Date de création (RFC 3339, UTC).
    pub created_at: String,
    /// Octets occupés par l'espace fichiers du compte.
    #[ts(type = "number")]
    pub storage_bytes: i64,
    /// Nombre de projets du compte.
    #[ts(type = "number")]
    pub project_count: i64,
}

/// Page de la liste des comptes (`GET /api/admin/users`, lot 32).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AdminUserListResponse {
    pub users: Vec<AdminUserResponse>,
    /// Nombre total de comptes correspondant au filtre `q`.
    #[ts(type = "number")]
    pub total: i64,
    /// Page courante (à partir de 1).
    #[ts(type = "number")]
    pub page: i64,
    /// Taille de page (1..=200, défaut 50).
    #[ts(type = "number")]
    pub per_page: i64,
}

/// Paramètres de `GET /api/admin/users` (query string).
#[derive(Debug, Clone, Default, Deserialize, TS)]
#[ts(export)]
pub struct AdminUsersQuery {
    /// Page (à partir de 1, défaut 1).
    #[ts(optional, type = "number")]
    pub page: Option<i64>,
    /// Taille de page (défaut 50, bornée à 200).
    #[ts(optional, type = "number")]
    pub per_page: Option<i64>,
    /// Filtre : sous-chaîne de l'e-mail, insensible à la casse.
    #[ts(optional)]
    pub q: Option<String>,
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
    /// Quota de stockage par compte (octets) ; 0 = illimité (lot 32).
    #[ts(type = "number")]
    pub storage_quota_bytes: i64,
    /// Tranchages simultanés (en file + en cours) par compte ; 0 = illimité.
    #[ts(type = "number")]
    pub max_concurrent_slices: i64,
}

impl From<InstanceSettings> for InstanceResponse {
    fn from(s: InstanceSettings) -> Self {
        Self {
            registration_policy: json_lower(&s.registration_policy),
            upload_limit_bytes: s.upload_limit_bytes,
            storage_quota_bytes: s.storage_quota_bytes,
            max_concurrent_slices: s.max_concurrent_slices,
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
    /// Quota de stockage par compte (octets, ≥ 0 ; 0 = illimité).
    #[ts(optional, type = "number")]
    pub storage_quota_bytes: Option<i64>,
    /// Tranchages simultanés par compte (≥ 0 ; 0 = illimité).
    #[ts(optional, type = "number")]
    pub max_concurrent_slices: Option<i64>,
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

/// Corps de `PATCH /api/projects/{id}` : mise à jour partielle (lot 26).
/// `name` renomme (comme `…/rename`) ; `scene` / `active_presets` remplacent
/// le champ correspondant et exigent `expected_version` (verrou optimiste,
/// 409 si décalé) ; un champ absent est conservé. Corps vide → 422.
#[derive(Debug, Clone, Default, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct PatchProjectRequest {
    #[serde(default)]
    #[ts(optional)]
    pub name: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub expected_version: Option<i64>,
    #[serde(default)]
    #[ts(optional, type = "unknown")]
    pub scene: Option<serde_json::Value>,
    #[serde(default)]
    #[ts(optional, type = "unknown")]
    pub active_presets: Option<serde_json::Value>,
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
    /// Rapport d'ouverture d'un 3MF (`POST /api/projects/import` seulement).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub import: Option<ImportReport>,
}

/// Mode d'ouverture d'un 3MF (Orca : `LoadType` de `determine_load_type`,
/// boîte « Ouvrir comme projet / Importer la géométrie seule »).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum ImportLoadMode {
    /// `LoadStrategy::LoadModel | LoadConfig` : géométrie, plateaux, config,
    /// presets du projet, métadonnées.
    #[default]
    Project,
    /// `LoadStrategy::LoadModel` seule : géométrie et réglages d'objet/volume
    /// (la remise à zéro d'Orca est inatteignable en 2.4.1), sans config ni
    /// presets, plateaux sans nom ni réglages, sans métadonnées ni auxiliaires.
    Geometry,
}

/// Paramètres de requête de `POST /api/projects/import`.
#[derive(Debug, Clone, Default, Deserialize, TS)]
#[ts(export)]
pub struct ImportQuery {
    #[serde(default)]
    #[ts(optional)]
    pub load: Option<ImportLoadMode>,
}

/// Résultat de `PresetBundle::validate_presets` à l'ouverture d'un projet
/// 3MF (Plater::priv::load_files) : `modified_gcodes` = G-codes modifiés dans
/// des presets filament/imprimante système (boîte « Modified G-code ») ;
/// `customized_presets` = presets filament/imprimante ne dérivant d'aucun
/// preset système (boîte « Customized Preset »).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct GcodeWarning {
    /// `modified_gcodes` | `customized_presets`.
    pub kind: String,
    /// Clés G-code modifiées (`machine_start_gcode`…) ou noms de presets.
    pub items: Vec<String>,
}

/// Rapport renvoyé par l'import d'un 3MF.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ImportReport {
    pub load: ImportLoadMode,
    /// Avertissement G-code (absent si aucun, en géométrie seule, sans config
    /// projet ou si la préférence `no_warn_when_modified_gcodes` est vraie).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub gcode_warning: Option<GcodeWarning>,
    /// Métadonnées (Title, Designer…) lues et conservées (`GET …/metadata`).
    pub has_metadata: bool,
}

/// Métadonnées 3MF d'un projet (Orca : onglet « Projet », ModelInfo /
/// ModelDesignInfo / ModelProfileInfo ; éditeur Auxiliary.cpp DesignerPanel).
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ProjectMetadataDto {
    /// « Title » (nom du modèle).
    #[serde(default)]
    pub title: String,
    /// « Designer » (auteur).
    #[serde(default)]
    pub designer: String,
    /// « License » : ``, CC0, BY, BY-SA, BY-ND, BY-NC, BY-NC-SA, BY-NC-ND.
    #[serde(default)]
    pub license: String,
    #[serde(default)]
    pub description: String,
    /// Couverture (« DesignerCover ») : nom d'un fichier de
    /// `Model Pictures` (auxiliaires), `` = aucune.
    #[serde(default)]
    pub cover: String,
    /// « Copyright » brut (JSON `[{"author": …}]` des projets MakerWorld).
    #[serde(default)]
    pub copyright: String,
    /// « Origin » (type de publication).
    #[serde(default)]
    pub origin: String,
    /// Auteur affiché par Orca (ProjectPanel::on_reload) : `author` du
    /// Copyright sinon Designer. Lecture seule.
    #[serde(default)]
    #[ts(optional)]
    pub display_author: Option<String>,
    /// Profil d'impression publié (ModelProfileInfo) : title, cover,
    /// description, user_id, user_name. Lecture seule.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional, type = "Record<string, string>")]
    pub profile: Option<std::collections::BTreeMap<String, String>>,
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
            import: None,
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
    /// Faux une fois la conversion terminée **ou échouée**.
    pub conversion_pending: bool,
    /// Cause de l'échec de conversion (message du moteur), absente sinon.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub conversion_error: Option<String>,
    /// Vrai si un maillage affichable est disponible (`…/mesh`).
    pub has_mesh: bool,
}

impl From<Model> for ModelResponse {
    fn from(m: Model) -> Self {
        let conversion_error = m
            .conversion_error
            .as_deref()
            .map(crate::models::convert::conversion_cause);
        let conversion_pending = m.format.needs_engine_conversion()
            && m.mesh_path.is_none()
            && conversion_error.is_none();
        Self {
            id: m.id.to_string(),
            project_id: m.project_id.map(|p| p.to_string()),
            filename: m.filename,
            format: json_lower(&m.format),
            size_bytes: m.size_bytes,
            triangle_count: m.triangle_count,
            conversion_pending,
            conversion_error,
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

/// Corps de `POST /api/presets/{id}/detach` (Orca : bouton « Detach from system
/// preset », Tab.cpp:2163). `name` : nom de la copie, requis pour un preset
/// système (Orca crée une copie détachée) ; ignoré pour un preset utilisateur
/// (détaché sur place).
#[derive(Debug, Clone, Default, Deserialize, TS)]
#[ts(export)]
pub struct DetachPresetRequest {
    #[ts(optional)]
    pub name: Option<String>,
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
    /// Modifications non enregistrées des presets (brouillon de l'éditeur) :
    /// Orca tranche avec le preset **édité**, pas avec la version enregistrée.
    #[ts(optional)]
    pub draft: Option<SliceDraft>,
    /// Préférence Orca « Remove mixed temperature restriction »
    /// (enable_high_low_temp_mixed_printing, défaut false) : filaments haute /
    /// basse température tranchés avec avertissement au lieu d'être refusés.
    #[ts(optional)]
    pub allow_mixed_temperatures: Option<bool>,
    /// Scène non enregistrée de l'éditeur (même format que `scene` du projet) :
    /// valide / tranche l'état affiché sans forcer la sauvegarde, comme Orca
    /// tranche le plateau courant. Absente ⇒ scène enregistrée du projet. Les
    /// modèles référencés doivent appartenir au projet (sinon refus explicite).
    #[ts(optional, type = "unknown")]
    pub scene: Option<serde_json::Value>,
    /// Presets actifs affichés non enregistrés (même format que
    /// `active_presets` du projet : `{ printer, process, filaments: [...] }`,
    /// ids de presets) : filament ajouté / imprimante ou process changés sans
    /// sauvegarde. Orca tranche la configuration éditée, pas celle du projet
    /// enregistré. Absent ⇒ presets actifs enregistrés du projet. Mêmes gardes
    /// (visibilité des presets, possession de l'imprimante) ; le brouillon
    /// `draft` s'applique ensuite par-dessus.
    #[ts(
        optional,
        type = "{ printer?: string; process?: string; filaments?: Array<string> }"
    )]
    pub active_presets: Option<serde_json::Value>,
}

/// Brouillon de presets appliqué au tranchage, par-dessus les presets actifs
/// et avant la config projet / plateau (ordre d'Orca). Valeurs au format des
/// presets (chaînes Orca ou JSON typé de `/resolved`).
#[derive(Debug, Clone, Default, Deserialize, TS)]
#[ts(export)]
pub struct SliceDraft {
    #[ts(optional, type = "Record<string, unknown>")]
    pub process: Option<serde_json::Map<String, serde_json::Value>>,
    #[ts(optional, type = "Record<string, unknown>")]
    pub printer: Option<serde_json::Map<String, serde_json::Value>>,
    /// Par filament (index de la barre de filaments) ; `null` = inchangé.
    #[ts(optional, type = "Array<Record<string, unknown> | null>")]
    pub filaments: Option<Vec<Option<serde_json::Map<String, serde_json::Value>>>>,
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
    /// Longueur totale de ce rôle sur tout le G-code (mm).
    pub length_mm: f64,
    /// Temps nominal de ce rôle (s) : longueur / vitesse commandée, sans
    /// accélérations. Sert à répartir le temps total estimé par type.
    pub time_s: f64,
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
                .zip(s.kind_totals.iter())
                .map(|(id, (length_mm, time_s))| PreviewSegmentType {
                    id: *id as u32,
                    name: crate::gcode::kind_name(*id).to_string(),
                    length_mm: *length_mm,
                    time_s: *time_s,
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
    /// Échec de conversion (lot 27) : cause précise remontée par le moteur
    /// (même texte que `ModelResponse.conversion_error` et le 422 de `/mesh`).
    #[serde(rename = "model.conversion_failed")]
    ModelConversionFailed { model_id: String, error: String },
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
    /// Contour réel du plateau ; à défaut, `printable_area` du preset actif,
    /// puis le rectangle `bed_width × bed_depth`.
    #[serde(default)]
    pub bed_shape: Option<Vec<[f64; 2]>>,
    /// Zones exclues ; à défaut, `bed_exclude_area` du preset actif.
    #[serde(default)]
    pub excluded: Option<Vec<Vec<[f64; 2]>>>,
    #[serde(default)]
    pub max_height: Option<f64>,
    /// Arrangement séquentiel « objet par objet » (`ArrangeParams::is_seq_print`
    /// d'Orca) : forcé par le plateau ; sinon déduit de `print_sequence` du
    /// preset actif.
    #[serde(default)]
    #[ts(optional)]
    pub is_seq_print: Option<bool>,
    /// Dégagements extrudeur (surcharge des clés machine du preset actif).
    #[serde(default)]
    #[ts(optional)]
    pub extruder_clearance_radius: Option<f64>,
    #[serde(default)]
    #[ts(optional)]
    pub extruder_clearance_height_to_rod: Option<f64>,
    #[serde(default)]
    #[ts(optional)]
    pub extruder_clearance_height_to_lid: Option<f64>,
    #[serde(default)]
    #[ts(optional)]
    pub nozzle_height: Option<f64>,
    /// « Remplir le plateau avec des instances » (FillBedJob d'Orca) : objet de
    /// `items` à dupliquer ; les autres items sont fixes. La réponse contient
    /// les instances existantes repositionnées (même id) puis les copies
    /// ajoutées (id `<objet>#fill<n>`, matrice complète).
    #[serde(default)]
    #[ts(optional)]
    pub fill_object_id: Option<String>,
    /// Options d'arrangement de la GUI d'Orca (menu « Options d'arrangement »,
    /// ArrangeSettings). Présentes → arrangement d'ArrangeJob sur plusieurs
    /// plateaux (`items` = toutes les instances de la scène, plateau lu dans la
    /// scène ; plateaux créés au besoin) ; absentes → arrangement historique
    /// plateau par plateau.
    #[serde(default)]
    #[ts(optional)]
    pub options: Option<ArrangeOptions>,
    /// Avec `options` : `all` (« Tout arranger », défaut) ou `plate`
    /// (« Arranger ce plateau », plateau `plate_index`).
    #[serde(default)]
    #[ts(optional)]
    pub scope: Option<ArrangeScope>,
    /// Plateau courant (0-based) : cible de `scope = plate`, et plateau dont
    /// le Print fournit object_skirt_offset (init_arrange_params).
    #[serde(default)]
    #[ts(optional)]
    pub plate_index: Option<u32>,
}

/// Portée de l'arrangement GUI (Orca : PREPARE_STATE_DEFAULT / MENU).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum ArrangeScope {
    #[default]
    All,
    Plate,
}

/// `GLCanvas3D::ArrangeSettings` d'Orca 2.4.1.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ArrangeOptions {
    /// Espacement (mm), 0 = automatique (« 0 means auto spacing »), ≤ 100.
    #[serde(default)]
    pub spacing: f64,
    /// « Auto rotate for arrangement ».
    #[serde(default)]
    pub enable_rotation: bool,
    /// « Allow multiple materials on same plate » (défaut vrai).
    #[serde(default = "default_true")]
    pub allow_multi_materials_on_same_plate: bool,
    /// « Avoid extrusion calibration region » (défaut vrai ; effectif seulement
    /// sur imprimante Bambu avec `scan_first_layer`).
    #[serde(default = "default_true")]
    pub avoid_extrusion_cali_region: bool,
    /// « Align to Y axis » (ignoré si la rotation automatique est active).
    #[serde(default)]
    pub align_to_y_axis: bool,
}

fn default_true() -> bool {
    true
}

/// Réponse de `GET /api/projects/{id}/arrange/options` : valeurs d'Orca à la
/// réinitialisation (bouton « Reset ») et disponibilité des cases.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct ArrangeOptionsInfo {
    pub defaults: ArrangeOptions,
    /// Jeu de réglages séquentiel (`print_sequence` = by object) : l'espacement
    /// affiché est au moins `min_spacing` (GLCanvas3D::get_arrange_settings).
    pub is_seq_print: bool,
    pub min_spacing: f64,
    /// Case « Avoid extrusion calibration region » affichée (Bambu + scan_first_layer).
    pub show_avoid_extrusion_cali_region: bool,
}

/// Position calculée du centre d'un objet (mm, repère plateau).
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct Placement {
    pub id: String,
    pub x: f64,
    pub y: f64,
    /// Index conservé du plateau d'origine.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub plate_index: Option<u32>,
    /// Matrice monde complète (colonne-major), pour ne pas perdre rotation et
    /// scale lorsque le client applique le résultat du nesting.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub matrix: Option<Vec<f64>>,
    /// Ordre d'impression (1-based, `ModelInstance::arrange_order` d'Orca),
    /// présent seulement en arrangement séquentiel.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub print_order: Option<u32>,
    /// Arrangement GUI : objet trop grand pour un plateau, posé sur un plateau
    /// ajouté (« Arrangement ignored the following objects… »).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[ts(as = "Option<bool>", optional)]
    pub unarranged: bool,
}

/// Réponse de `POST /api/projects/{id}/arrange`.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct ArrangeResponse {
    pub placements: Vec<Placement>,
    /// Arrangement GUI : nombre de plateaux après arrangement (plateaux créés).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub plate_count: Option<u32>,
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
    /// État du maillage **après** réparation.
    pub triangles: u32,
    pub degenerate: u32,
    pub open_edges: u32,
    pub watertight: bool,
    /// Compteurs de corrections effectivement réalisées par admesh.
    pub repairs: RepairCounters,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct RepairCounters {
    pub edges_fixed: u32,
    pub degenerate_facets: u32,
    pub facets_removed: u32,
    pub facets_reversed: u32,
    pub backwards_edges: u32,
}

impl From<engine::api::RepairedMeshErrors> for RepairCounters {
    fn from(report: engine::api::RepairedMeshErrors) -> Self {
        Self {
            edges_fixed: report.edges_fixed,
            degenerate_facets: report.degenerate_facets,
            facets_removed: report.facets_removed,
            facets_reversed: report.facets_reversed,
            backwards_edges: report.backwards_edges,
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

// --- Imprimantes (Moonraker, T075) -------------------------------------------

/// Imprimante déclarée renvoyée au client. La clé API n'est **jamais** exposée :
/// seul `has_api_key` indique sa présence (FR-060).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PrinterResponse {
    pub id: String,
    pub name: String,
    #[ts(optional)]
    pub moonraker_url: Option<String>,
    pub has_api_key: bool,
    pub machine_preset_id: String,
    /// Type d'hôte d'impression (moonraker, octoprint, prusalink, …).
    pub host_type: String,
    #[ts(optional)]
    pub host_auth: Option<String>,
    #[ts(optional)]
    pub host_user: Option<String>,
    pub has_host_password: bool,
    #[ts(optional)]
    pub host_port: Option<String>,
    /// Flashforge : numéro de série (host_options.serial_number).
    #[ts(optional)]
    pub host_serial_number: Option<String>,
}

impl From<Printer> for PrinterResponse {
    fn from(p: Printer) -> Self {
        let opt = |k: &str| {
            p.host_options
                .get(k)
                .and_then(|v| v.as_str())
                .map(str::to_string)
        };
        Self {
            id: p.id.to_string(),
            name: p.name.clone(),
            moonraker_url: p.moonraker_url.clone(),
            has_api_key: p.api_key.is_some(),
            machine_preset_id: p.machine_preset_id.to_string(),
            host_type: p.host_type.clone(),
            host_auth: opt("auth"),
            host_user: opt("user"),
            has_host_password: opt("password").is_some(),
            host_port: opt("port"),
            host_serial_number: opt("serial_number"),
        }
    }
}

/// Corps de `POST`/`PUT /api/printers` : déclaration ou mise à jour. `api_key`
/// est chiffrée au repos avant stockage (T075).
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
pub struct SavePrinterRequest {
    pub name: String,
    #[ts(optional)]
    pub moonraker_url: Option<String>,
    #[ts(optional)]
    pub api_key: Option<String>,
    pub machine_preset_id: String,
    /// Type d'hôte d'impression (host_type d'Orca) ; absent = moonraker.
    /// `moonraker_url` porte l'adresse de l'hôte (print_host) quel que soit le type.
    #[serde(default)]
    #[ts(optional)]
    pub host_type: Option<String>,
    /// PrusaLink : « user » = utilisateur/mot de passe (digest), « key » = clé API.
    #[serde(default)]
    #[ts(optional)]
    pub host_auth: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub host_user: Option<String>,
    /// Mot de passe (chiffré au repos, jamais renvoyé) ; absent = conservé.
    #[serde(default)]
    #[ts(optional)]
    pub host_password: Option<String>,
    /// Repetier : slug de l'imprimante sur le serveur (printhost_port).
    #[serde(default)]
    #[ts(optional)]
    pub host_port: Option<String>,
    /// Flashforge : numéro de série (flashforge_serial_number d'Orca), stocké
    /// dans host_options.serial_number ; absent = conservé, vide = retiré.
    #[serde(default)]
    #[ts(optional)]
    pub host_serial_number: Option<String>,
}

// --- Catalogue de sélection d'imprimante (wizard/onboarding, Phase 14) --------

/// Une variante de buse d'un modèle (un preset machine instanciable).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PrinterCatalogVariant {
    /// Preset machine à retenir comme `machine_preset_id` de l'imprimante.
    pub machine_preset_id: String,
    /// Diamètre de buse (ex. « 0.4 ») ; vide si le nom ne le précise pas.
    pub nozzle: String,
}

/// Un modèle d'imprimante (regroupe ses variantes de buse), tel qu'affiché dans
/// la grille de sélection façon OrcaSlicer.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PrinterCatalogModel {
    pub vendor: String,
    pub model: String,
    /// Chemin relatif de la vignette (`<Marque>/<Modèle>_cover.png`) sous
    /// `static/printers/` ; le client retombe sur un placeholder si absente.
    pub cover: String,
    pub variants: Vec<PrinterCatalogVariant>,
    /// Variante par défaut (0.4 mm si présente, sinon la plus petite buse).
    pub default_machine_preset_id: String,
}

/// Modèles d'une marque (section de la grille).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PrinterCatalogVendor {
    pub vendor: String,
    pub models: Vec<PrinterCatalogModel>,
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
    /// Stockage PrusaLink (« /usb »…) ou groupe Repetier (GET …/storages).
    #[serde(default)]
    #[ts(optional)]
    pub storage: Option<String>,
    /// Démarrer l'impression immédiatement (`print=true`, FR-061).
    pub start_now: bool,
    /// « Téléverser et mettre en file » (Orca PrintHostPostUploadAction::QueuePrint,
    /// PrusaConnect : champ `to_queue=True`) ; exclusif de `start_now`, refusé
    /// (422) pour les hôtes qui ne le proposent pas.
    #[serde(default)]
    #[ts(optional)]
    pub queue: Option<bool>,
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
    /// Couche en cours / total (`print_stats.info`), si l'imprimante les publie.
    #[ts(optional)]
    pub current_layer: Option<i64>,
    #[ts(optional)]
    pub total_layer: Option<i64>,
    /// Ventilateur de pièce (`fan.speed`, 0 → 1).
    #[ts(optional)]
    pub fan_speed: Option<f64>,
    /// Axes référencés (`toolhead.homed_axes`).
    #[ts(optional)]
    pub homed_axes: Option<String>,
    /// URL absolue de la webcam (relais `/server/webcams/list`).
    #[ts(optional)]
    pub webcam_url: Option<String>,
}

/// Fichier présent sur l'imprimante (`GET /api/printers/{id}/files`).
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct PrinterFileResponse {
    pub path: String,
    pub size: u64,
    pub modified: f64,
}

/// Corps de `POST /api/printers/{id}/print`.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
pub struct StartPrintRequest {
    pub filename: String,
}

/// Corps de `POST /api/printers/{id}/gcode`.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
pub struct GcodeScriptRequest {
    pub script: String,
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
            current_layer: s.current_layer,
            total_layer: s.total_layer,
            fan_speed: s.fan_speed,
            homed_axes: s.homed_axes,
            webcam_url: None,
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
