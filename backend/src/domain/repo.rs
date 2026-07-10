//! Traits de stockage (contrat storage-trait.md). Chaque repo expose des
//! opérations **métier** (pas du CRUD générique) et scope par `UserId` toute
//! ressource possédée : l'oubli du scope est impossible par signature.
//!
//! Async via `async_trait` pour rester compatible `dyn` (la suite de contrat
//! générique dispatche sur `&dyn Storage`).

use async_trait::async_trait;

use super::entities::{
    Gcode, InstanceSettings, Invitation, Model, Preset, PresetKind, Printer, Project,
    RegistrationPolicy, Role, SlicingJob, User, UserStatus,
};
use super::error::StorageResult;
use super::id::{GcodeId, JobId, ModelId, PresetId, PrinterId, ProjectId, UserId};

/// Point d'accès unique aux repos (constitution III : le métier ne connaît ni
/// SQL ni SGBD).
pub trait Storage: Send + Sync {
    fn users(&self) -> &dyn UserRepo;
    fn projects(&self) -> &dyn ProjectRepo;
    fn models(&self) -> &dyn ModelRepo;
    fn presets(&self) -> &dyn PresetRepo;
    fn printers(&self) -> &dyn PrinterRepo;
    fn jobs(&self) -> &dyn JobRepo;
    fn gcodes(&self) -> &dyn GcodeRepo;
    fn instance(&self) -> &dyn InstanceRepo;
}

/// Données d'un nouveau compte (le hash argon2 est calculé par `auth`).
#[derive(Debug, Clone)]
pub struct NewUser {
    pub email: String,
    pub password_hash: String,
    pub role: Role,
}

#[async_trait]
pub trait UserRepo: Send + Sync {
    /// Crée un compte. `Conflict` si l'email existe déjà.
    async fn create(&self, user: NewUser) -> StorageResult<User>;
    async fn get(&self, id: UserId) -> StorageResult<User>;
    async fn find_by_email(&self, email: &str) -> StorageResult<Option<User>>;
    async fn list(&self) -> StorageResult<Vec<User>>;
    async fn count(&self) -> StorageResult<i64>;
    async fn set_password_hash(&self, id: UserId, password_hash: &str) -> StorageResult<()>;
    async fn set_status(&self, id: UserId, status: UserStatus) -> StorageResult<()>;
    /// Supprime le compte **et cascade** (projets, modèles, presets user,
    /// imprimantes, jobs, gcodes). La purge filesystem est orchestrée à part.
    async fn delete(&self, id: UserId) -> StorageResult<()>;
}

#[derive(Debug, Clone)]
pub struct NewProject {
    pub name: String,
    pub scene: serde_json::Value,
    pub active_presets: serde_json::Value,
    pub thumbnail_path: Option<String>,
}

#[async_trait]
pub trait ProjectRepo: Send + Sync {
    /// `Conflict` si le nom existe déjà pour cet utilisateur.
    async fn create(&self, owner: UserId, project: NewProject) -> StorageResult<Project>;
    /// `NotFound` si le projet n'existe pas **ou** n'appartient pas à `owner`.
    async fn get(&self, owner: UserId, id: ProjectId) -> StorageResult<Project>;
    async fn list(&self, owner: UserId) -> StorageResult<Vec<Project>>;
    /// Sauvegarde avec verrou optimiste : `expected_version` doit égaler la
    /// version stockée, sinon `VersionConflict` (409, conflit multi-onglets).
    async fn update(
        &self,
        owner: UserId,
        id: ProjectId,
        expected_version: i64,
        scene: serde_json::Value,
        active_presets: serde_json::Value,
        thumbnail_path: Option<String>,
    ) -> StorageResult<Project>;
    async fn rename(&self, owner: UserId, id: ProjectId, name: &str) -> StorageResult<Project>;
    async fn delete(&self, owner: UserId, id: ProjectId) -> StorageResult<()>;
}

#[derive(Debug, Clone)]
pub struct NewModel {
    pub project_id: Option<ProjectId>,
    pub filename: String,
    pub format: super::entities::ModelFormat,
    pub file_path: String,
    pub mesh_path: Option<String>,
    pub size_bytes: i64,
    pub triangle_count: i64,
    pub repair_report: Option<serde_json::Value>,
}

#[async_trait]
pub trait ModelRepo: Send + Sync {
    async fn create(&self, owner: UserId, model: NewModel) -> StorageResult<Model>;
    async fn get(&self, owner: UserId, id: ModelId) -> StorageResult<Model>;
    async fn list(&self, owner: UserId, project: Option<ProjectId>) -> StorageResult<Vec<Model>>;
    async fn delete(&self, owner: UserId, id: ModelId) -> StorageResult<()>;
}

#[async_trait]
pub trait PresetRepo: Send + Sync {
    /// Remplace **tous** les presets système par ceux fournis (re-seed), sans
    /// toucher aux presets utilisateur.
    async fn reseed_system(&self, presets: Vec<Preset>) -> StorageResult<u64>;
    /// Nombre de presets système en base (pour le seed conditionnel au boot).
    async fn system_count(&self) -> StorageResult<i64>;
    /// Presets système + presets de l'utilisateur, filtrés `instantiation` et
    /// compatibilité imprimante (FR-021).
    async fn list_compatible(
        &self,
        kind: PresetKind,
        printer_name: Option<&str>,
        user: UserId,
    ) -> StorageResult<Vec<Preset>>;
    async fn get(&self, id: PresetId) -> StorageResult<Preset>;
    /// Tous les presets d'un type (système + ceux de l'utilisateur), **sans**
    /// filtre d'instanciation ni de compatibilité — pour reconstruire une
    /// chaîne d'héritage (parents abstraits inclus).
    async fn list_by_kind(&self, kind: PresetKind, user: UserId) -> StorageResult<Vec<Preset>>;
    /// `Conflict` si (kind, name) existe déjà pour cet utilisateur.
    async fn create_user_preset(&self, owner: UserId, preset: Preset) -> StorageResult<Preset>;
    /// Met à jour le nom et les valeurs d'un preset **utilisateur** (rename /
    /// reset). `NotFound` s'il n'existe pas ou n'appartient pas à `owner`.
    async fn update_user_preset(
        &self,
        owner: UserId,
        id: PresetId,
        name: &str,
        values: serde_json::Value,
    ) -> StorageResult<Preset>;
    async fn delete_user_preset(&self, owner: UserId, id: PresetId) -> StorageResult<()>;
}

#[derive(Debug, Clone)]
pub struct NewPrinter {
    pub name: String,
    pub moonraker_url: String,
    pub api_key: Option<String>,
    pub machine_preset_id: PresetId,
}

#[async_trait]
pub trait PrinterRepo: Send + Sync {
    async fn create(&self, owner: UserId, printer: NewPrinter) -> StorageResult<Printer>;
    async fn get(&self, owner: UserId, id: PrinterId) -> StorageResult<Printer>;
    async fn list(&self, owner: UserId) -> StorageResult<Vec<Printer>>;
    async fn delete(&self, owner: UserId, id: PrinterId) -> StorageResult<()>;
}

#[derive(Debug, Clone)]
pub struct NewJob {
    pub project_id: ProjectId,
    pub plate_index: i64,
    pub resolved_settings: serde_json::Value,
}

/// Résolution d'un job terminé.
#[derive(Debug, Clone)]
pub enum JobOutcome {
    Succeeded { gcode_id: GcodeId },
    Failed { error: serde_json::Value },
}

#[async_trait]
pub trait JobRepo: Send + Sync {
    async fn enqueue(&self, owner: UserId, job: NewJob) -> StorageResult<SlicingJob>;
    async fn get(&self, owner: UserId, id: JobId) -> StorageResult<SlicingJob>;
    async fn list(&self, owner: UserId) -> StorageResult<Vec<SlicingJob>>;
    /// Réclamation transactionnelle du prochain job `queued` → `running`
    /// (un seul worker gagne sous concurrence). `None` si la file est vide.
    async fn claim_next(&self) -> StorageResult<Option<SlicingJob>>;
    /// Reprise au boot : tous les `running` repassent `queued` (R9).
    async fn requeue_running(&self) -> StorageResult<u64>;
    async fn update_progress(&self, id: JobId, progress: f64, phase: &str) -> StorageResult<()>;
    async fn finish(&self, id: JobId, outcome: JobOutcome) -> StorageResult<()>;
    /// Annulation par le propriétaire (`queued|running → cancelled`).
    async fn cancel(&self, owner: UserId, id: JobId) -> StorageResult<()>;
}

#[derive(Debug, Clone)]
pub struct NewGcode {
    pub job_id: JobId,
    pub file_path: String,
    pub preview_path: String,
    pub stats: serde_json::Value,
    pub thumbnails: serde_json::Value,
}

#[async_trait]
pub trait GcodeRepo: Send + Sync {
    async fn create(&self, owner: UserId, gcode: NewGcode) -> StorageResult<Gcode>;
    async fn get(&self, owner: UserId, id: GcodeId) -> StorageResult<Gcode>;
    async fn delete(&self, owner: UserId, id: GcodeId) -> StorageResult<()>;
}

#[derive(Debug, Clone)]
pub struct NewInvitation {
    pub token: String,
    pub issued_by: UserId,
    pub expires_at: time::OffsetDateTime,
}

#[async_trait]
pub trait InstanceRepo: Send + Sync {
    async fn settings(&self) -> StorageResult<InstanceSettings>;
    async fn set_registration_policy(&self, policy: RegistrationPolicy) -> StorageResult<()>;
    async fn set_upload_limit(&self, bytes: i64) -> StorageResult<()>;
    async fn create_invitation(&self, invitation: NewInvitation) -> StorageResult<Invitation>;
    /// Consomme une invitation valide (non utilisée, non expirée) → l'émetteur.
    /// `NotFound` si le token est invalide/expiré/déjà utilisé.
    async fn consume_invitation(&self, token: &str) -> StorageResult<Invitation>;
    async fn list_invitations(&self) -> StorageResult<Vec<Invitation>>;
}
