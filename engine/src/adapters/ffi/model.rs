//! Chargement de modèles via libslic3r (T013) : STL/OBJ/3MF géométrie,
//! STEP via OCCT — remappage des structs brutes du bridge vers `api`.

use std::path::Path;
use std::sync::{Mutex, MutexGuard, Once, PoisonError};

use glam::DMat4;

use crate::api::{
    EngineError, EngineErrorCode, EngineResult, Model, ModelInstance, ModelObject, ModelVolume,
    TriangleMesh, VolumeRole,
};

use super::bridge::ffi;

/// libslic3r a des états globaux non thread-safe (OCCT XCAFApp, backup des
/// 3MF, print_config_def…) : tout appel FFI est sérialisé, et le runtime est
/// initialisé une seule fois (répertoires temp/data). L'isolation forte
/// arrive avec le process engine-worker (T018).
static FFI_LOCK: Mutex<()> = Mutex::new(());
static INIT: Once = Once::new();

pub(super) fn ffi_guard() -> MutexGuard<'static, ()> {
    let guard = FFI_LOCK.lock().unwrap_or_else(PoisonError::into_inner);
    INIT.call_once(|| {
        let tmp = std::env::temp_dir().join("web-slicer-engine");
        let data = tmp.join("data");
        std::fs::create_dir_all(&data).ok();
        ffi::init_runtime(&tmp.to_string_lossy(), &data.to_string_lossy());
    });
    guard
}

pub fn load_model(path: &Path) -> EngineResult<Model> {
    let _guard = ffi_guard();
    let raw = ffi::load_model_raw(&path.to_string_lossy())
        .map_err(|e| EngineError::new(EngineErrorCode::InvalidModel, e.to_string()))?;
    Ok(Model {
        objects: raw.into_iter().map(to_object).collect(),
    })
}

/// Conversion vers maillage unique (STEP → mesh, R7) : fusion des volumes.
pub fn convert_to_mesh(path: &Path) -> EngineResult<TriangleMesh> {
    let model = load_model(path)?;
    let mut merged = TriangleMesh::default();
    for object in model.objects {
        for volume in object.volumes {
            let offset = merged.vertices.len() as u32;
            // applique la transformation locale du volume
            let m = volume.matrix;
            merged.vertices.extend(volume.mesh.vertices.iter().map(|v| {
                let p = m.transform_point3(glam::DVec3::new(v[0] as f64, v[1] as f64, v[2] as f64));
                [p.x as f32, p.y as f32, p.z as f32]
            }));
            merged.indices.extend(
                volume
                    .mesh
                    .indices
                    .iter()
                    .map(|t| [t[0] + offset, t[1] + offset, t[2] + offset]),
            );
        }
    }
    if merged.is_empty() {
        return Err(
            EngineError::new(EngineErrorCode::InvalidModel, "conversion sans triangles")
                .with_subject(path.to_string_lossy()),
        );
    }
    Ok(merged)
}

pub(super) fn to_object(raw: ffi::RawObject) -> ModelObject {
    ModelObject {
        name: raw.name,
        volumes: raw.volumes.into_iter().map(to_volume).collect(),
        instances: raw
            .instances
            .into_iter()
            .map(|i| ModelInstance {
                matrix: DMat4::from_cols_array(&i.matrix),
                plate_index: 0,
            })
            .collect(),
    }
}

fn to_volume(raw: ffi::RawVolume) -> ModelVolume {
    ModelVolume {
        name: raw.name,
        mesh: to_mesh(raw.mesh),
        matrix: DMat4::from_cols_array(&raw.matrix),
        role: match raw.role {
            1 => VolumeRole::NegativeVolume,
            2 => VolumeRole::ParameterModifier,
            3 => VolumeRole::SupportBlocker,
            4 => VolumeRole::SupportEnforcer,
            _ => VolumeRole::ModelPart,
        },
        extruder: (raw.extruder > 0).then_some(raw.extruder as u16),
    }
}

fn to_mesh(raw: ffi::RawMesh) -> TriangleMesh {
    TriangleMesh {
        vertices: raw
            .vertices
            .chunks_exact(3)
            .map(|c| [c[0], c[1], c[2]])
            .collect(),
        indices: raw
            .indices
            .chunks_exact(3)
            .map(|c| [c[0], c[1], c[2]])
            .collect(),
    }
}

/// Nombre de triangles d'un fichier modèle (smoke T012).
pub fn model_triangle_count(path: &Path) -> EngineResult<usize> {
    let _guard = ffi_guard();
    ffi::model_triangle_count(&path.to_string_lossy())
        .map_err(|e| EngineError::new(EngineErrorCode::InvalidModel, e.to_string()))
}

/// Comptage runtime du registre C++ (croisement avec `params::REGISTRY`).
pub fn print_config_option_count() -> usize {
    ffi::print_config_option_count()
}

/// Conversion inverse : scène `api` → structs brutes du bridge (écriture 3MF).
pub(super) fn to_raw_objects(model: &Model) -> Vec<ffi::RawObject> {
    model
        .objects
        .iter()
        .map(|o| ffi::RawObject {
            name: o.name.clone(),
            volumes: o
                .volumes
                .iter()
                .map(|v| ffi::RawVolume {
                    name: v.name.clone(),
                    matrix: v.matrix.to_cols_array(),
                    role: match v.role {
                        VolumeRole::ModelPart => 0,
                        VolumeRole::NegativeVolume => 1,
                        VolumeRole::ParameterModifier => 2,
                        VolumeRole::SupportBlocker => 3,
                        VolumeRole::SupportEnforcer => 4,
                    },
                    extruder: v.extruder.map_or(0, u32::from),
                    mesh: ffi::RawMesh {
                        vertices: v.mesh.vertices.iter().flatten().copied().collect(),
                        indices: v.mesh.indices.iter().flatten().copied().collect(),
                    },
                })
                .collect(),
            instances: o
                .instances
                .iter()
                .map(|i| ffi::RawInstance {
                    matrix: i.matrix.to_cols_array(),
                })
                .collect(),
        })
        .collect()
}
