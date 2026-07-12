//! Modèles 3D importés : conversion vers maillage d'affichage (R7).

pub mod convert;

pub use convert::{ConvertError, MeshDecoder, ModelConverter, WorkerMeshDecoder};
