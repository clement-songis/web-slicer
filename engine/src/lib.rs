//! Moteur de slicing — trait `SlicerEngine`, registre de paramètres, types
//! miroirs de libslic3r.
//!
//! Constitution II : ce crate encapsule libslic3r d'OrcaSlicer ; le backend
//! ne dépend que du trait défini ici et jamais des adaptateurs
//! (`adapters::ffi` — bridge cxx principal ; `adapters::cli` — fallback
//! `orca-slicer`, sélection par `ENGINE_IMPL=ffi|cli`).

pub mod adapters;
pub mod api;
pub mod params;
pub mod presets;

use std::path::Path;

use api::{
    ArrangeParams, BuildVolume, CancelToken, ConfigWarning, DynamicPrintConfig, EngineResult,
    GcodePreview, Model, ModelFormat, ModelObject, ProgressSink, RawPreset, RepairedMeshErrors,
    SliceRequest, SliceResult, TriangleMesh,
};

/// Contrat du moteur (miroir 1:1 de l'API libslic3r consommée par la GUI —
/// `audit/engine_api.json` fait foi ; contrat détaillé :
/// specs/001-orcaslicer-web-parity/contracts/slicer-engine-trait.md).
///
/// Toute implémentation doit passer la suite générique
/// `engine/tests/common/trait_suite.rs` (garantie de substituabilité).
pub trait SlicerEngine: Send + Sync {
    // --- Modèle / scène -----------------------------------------------------

    /// Charge un fichier modèle (STL/OBJ/3MF géométrie seule).
    fn load_model(&self, path: &Path, format: ModelFormat) -> EngineResult<Model>;

    /// Lit un projet 3MF OrcaSlicer : scène + configuration embarquée.
    fn read_project_3mf(&self, path: &Path) -> EngineResult<(Model, DynamicPrintConfig)>;

    /// Écrit un projet 3MF compatible OrcaSlicer (FR-044).
    fn write_project_3mf(
        &self,
        model: &Model,
        config: &DynamicPrintConfig,
        out: &Path,
    ) -> EngineResult<()>;

    /// Répare un maillage et rapporte les corrections (FR-012).
    fn repair_mesh(&self, mesh: &TriangleMesh) -> EngineResult<(TriangleMesh, RepairedMeshErrors)>;

    /// Convertit un fichier (notamment STEP via OCCT, R7) en maillage.
    fn convert_to_mesh(&self, path: &Path) -> EngineResult<TriangleMesh>;

    // --- Opérations de scène déléguées au moteur ----------------------------

    /// Arrangement automatique sans collision dans le volume (FR-013).
    fn arrange(
        &self,
        model: &mut Model,
        bed: &BuildVolume,
        params: &ArrangeParams,
    ) -> EngineResult<()>;

    /// Orientation automatique d'un objet (FR-013).
    fn orient(&self, object: &mut ModelObject) -> EngineResult<()>;

    // --- Presets / configuration --------------------------------------------

    /// Aplati une chaîne d'héritage (racine → feuille) en valeurs effectives,
    /// clés legacy converties (R5, FR-023).
    fn resolve_preset_chain(&self, chain: &[RawPreset]) -> EngineResult<DynamicPrintConfig>;

    /// Contrôles de cohérence identiques à OrcaSlicer (FR-032).
    fn validate_config(&self, config: &DynamicPrintConfig) -> EngineResult<Vec<ConfigWarning>>;

    // --- Tranchage -----------------------------------------------------------

    /// Tranche un plateau. Progression via `progress`, annulation coopérative
    /// via `cancel` (kill du process moteur). N'écrit que dans
    /// `req.work_dir` (garantie d'isolation n°3).
    fn slice(
        &self,
        req: SliceRequest,
        progress: ProgressSink,
        cancel: CancelToken,
    ) -> EngineResult<SliceResult>;

    // --- Prévisualisation -----------------------------------------------------

    /// Parse un G-code produit en modèle de prévisualisation (R6).
    fn parse_gcode(&self, path: &Path) -> EngineResult<GcodePreview>;
}
