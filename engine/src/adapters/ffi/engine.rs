//! Implémentation FFI du trait `SlicerEngine` (R1) : agrège les wrappers du
//! bridge cxx et les fonctions pures partagées (presets, parseur G-code)
//! derrière l'interface unique consommée par le backend.
//!
//! La substituabilité est prouvée par la suite générique `trait_suite`
//! (constitution II, garantie n°4), invoquée par `tests/ffi_trait_suite.rs`.

use std::path::Path;

use crate::api::{
    ArrangeParams, BuildVolume, ConfigWarning, DynamicPrintConfig, EngineResult, GcodePreview,
    Model, ModelFormat, ModelObject, ProgressSink, RawPreset, RepairedMeshErrors, SliceRequest,
    SliceResult, TriangleMesh,
};
use crate::{gcode, presets, SlicerEngine};

/// Moteur adossé à libslic3r-headless via le bridge cxx et le worker isolé.
#[derive(Debug, Default, Clone, Copy)]
pub struct FfiEngine;

impl SlicerEngine for FfiEngine {
    fn load_model(&self, path: &Path, _format: ModelFormat) -> EngineResult<Model> {
        // libslic3r détecte le format par l'extension ; `format` reste
        // informatif (l'appelant l'a déjà déduit).
        super::load_model(path)
    }

    fn read_project_3mf(&self, path: &Path) -> EngineResult<(Model, DynamicPrintConfig)> {
        super::read_project_3mf(path)
    }

    fn write_project_3mf(
        &self,
        model: &Model,
        config: &DynamicPrintConfig,
        out: &Path,
    ) -> EngineResult<()> {
        super::write_project_3mf(model, config, out)
    }

    fn repair_mesh(&self, mesh: &TriangleMesh) -> EngineResult<(TriangleMesh, RepairedMeshErrors)> {
        super::repair_mesh(mesh)
    }

    fn convert_to_mesh(&self, path: &Path) -> EngineResult<TriangleMesh> {
        super::convert_to_mesh(path)
    }

    fn arrange(
        &self,
        model: &mut Model,
        bed: &BuildVolume,
        params: &ArrangeParams,
    ) -> EngineResult<()> {
        super::arrange(model, bed, params)
    }

    fn orient(&self, object: &mut ModelObject) -> EngineResult<()> {
        super::orient(object)
    }

    fn resolve_preset_chain(&self, chain: &[RawPreset]) -> EngineResult<DynamicPrintConfig> {
        presets::resolve_preset_chain(chain)
    }

    fn validate_config(&self, config: &DynamicPrintConfig) -> EngineResult<Vec<ConfigWarning>> {
        presets::validate_config(config)
    }

    fn slice(
        &self,
        req: SliceRequest,
        progress: ProgressSink,
        cancel: crate::api::CancelToken,
    ) -> EngineResult<SliceResult> {
        super::slice(req, progress, cancel)
    }

    fn parse_gcode(&self, path: &Path) -> EngineResult<GcodePreview> {
        gcode::parse_gcode(path)
    }
}
