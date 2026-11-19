//! Types miroirs de l'API publique de libslic3r consommée par la GUI
//! (`audit/engine_api.json` fait foi — contrat slicer-engine-trait.md).

pub mod calib;
pub mod config;
pub mod error;
pub mod filament_compat;
pub mod gcode;
pub mod gcode_process;
pub mod layer_profile;
pub mod mesh;
pub mod model;
pub mod paint;
pub mod presets;
pub mod slice;
pub mod split;

pub use config::{ConfigValue, ConfigWarning, DynamicPrintConfig};
pub use error::{EngineError, EngineErrorCode, EngineResult};
pub use gcode::{GcodeLayer, GcodePreview, GcodeSegment, LineKind};
pub use mesh::{DisplayMesh, DisplayMeshError, RepairedMeshErrors, TriangleMesh};
pub use model::{
    CustomGcodeItem, CustomGcodeKind, CustomGcodeMode, LayerConfigRange, Model, ModelFormat,
    ModelInstance, ModelObject, ModelVolume, PaintChannel, PlateCustomGcodes, VolumePaint,
    VolumeRole,
};
pub use presets::{ArrangeParams, GuiArrangeOutcome, GuiArrangeSettings, RawPreset};
pub use slice::{
    BuildVolume, CalibMode, CalibParams, CancelToken, ProgressSink, ShaperType, SliceRequest,
    SliceResult, SliceStats, ValidateIssue, ValidateIssueKind, ValidateResult,
    OUTPUT_FILENAME_BASE_TOKEN,
};
