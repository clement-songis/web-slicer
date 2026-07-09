//! Types miroirs de l'API publique de libslic3r consommée par la GUI
//! (`audit/engine_api.json` fait foi — contrat slicer-engine-trait.md).

pub mod config;
pub mod error;
pub mod gcode;
pub mod mesh;
pub mod model;
pub mod presets;
pub mod slice;

pub use config::{ConfigValue, ConfigWarning, DynamicPrintConfig};
pub use error::{EngineError, EngineErrorCode, EngineResult};
pub use gcode::{GcodeLayer, GcodePreview, GcodeSegment, LineKind};
pub use mesh::{RepairedMeshErrors, TriangleMesh};
pub use model::{Model, ModelFormat, ModelInstance, ModelObject, ModelVolume, VolumeRole};
pub use presets::{ArrangeParams, RawPreset};
pub use slice::{BuildVolume, CancelToken, ProgressSink, SliceRequest, SliceResult, SliceStats};
