//! Chargement de modèles via libslic3r (T013) : STL/OBJ/3MF géométrie,
//! STEP via OCCT — remappage des structs brutes du bridge vers `api`.

use std::path::Path;
use std::sync::{Mutex, MutexGuard, Once, PoisonError};

use glam::DMat4;

use crate::api::{
    DynamicPrintConfig, EngineError, EngineErrorCode, EngineResult, Model, ModelInstance,
    ModelObject, ModelVolume, TriangleMesh, VolumeRole,
};
use crate::params::{self, orca_values};

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
        custom_gcodes: Vec::new(),
    })
}

/// Réglages de maillage STEP du dialogue d'Orca (StepMeshDialog) ; défauts =
/// Step::mesh (0,003 mm ; 0,5 rad ; pas de découpe) = import du CLI.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct StepMeshOptions {
    #[serde(default = "default_linear")]
    pub linear_deflection: f64,
    #[serde(default = "default_angle")]
    pub angle_deflection: f64,
    #[serde(default)]
    pub split_compound: bool,
}

fn default_linear() -> f64 {
    0.003
}
fn default_angle() -> f64 {
    0.5
}

impl Default for StepMeshOptions {
    fn default() -> Self {
        Self {
            linear_deflection: default_linear(),
            angle_deflection: default_angle(),
            split_compound: false,
        }
    }
}

/// STEP maillé avec `options` ; renvoie le modèle et le nombre de triangles
/// annoncé par le dialogue d'Orca (Step::get_triangle_num).
pub fn load_step(path: &Path, options: StepMeshOptions) -> EngineResult<(Model, u32)> {
    let _guard = ffi_guard();
    let raw = ffi::load_step_raw(
        &path.to_string_lossy(),
        options.linear_deflection,
        options.angle_deflection,
        options.split_compound,
    )
    .map_err(|e| EngineError::new(EngineErrorCode::InvalidModel, e.to_string()))?;
    Ok((
        Model {
            objects: raw.objects.into_iter().map(to_object).collect(),
            custom_gcodes: Vec::new(),
        },
        raw.dialog_triangles,
    ))
}

/// Conversion vers maillage unique (STEP → mesh, R7) : fusion des volumes.
/// Fusion des volumes d'un modèle en un maillage (repère objet, matrices des
/// volumes appliquées), comme [`convert_to_mesh`].
pub fn convert_to_mesh_model(model: &Model) -> TriangleMesh {
    let mut merged = TriangleMesh::default();
    for object in &model.objects {
        for volume in &object.volumes {
            let offset = merged.vertices.len() as u32;
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
    merged
}

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
                printable: i.printable,
            })
            .collect(),
        settings: config_from_orca_json(&raw.config_json),
        layer_height_profile: raw.layer_height_profile,
        layer_config_ranges: ranges_from_json(&raw.layer_ranges_json),
        cut_id: cut_from_json(&raw.cut_json),
        brim_points: raw
            .brim_points
            .chunks_exact(4)
            .map(|p| crate::api::BrimPoint {
                pos: [p[0], p[1], p[2]],
                head_front_radius: p[3],
            })
            .collect(),
    }
}

/// JSON optionnel du bridge (`cut_json`, `stats_json`) : vide ou illisible =
/// `None`.
fn cut_from_json<T: serde::de::DeserializeOwned>(json: &str) -> Option<T> {
    if json.trim().is_empty() {
        return None;
    }
    serde_json::from_str(json).ok()
}

fn cut_to_json<T: serde::Serialize>(value: Option<&T>) -> String {
    value
        .and_then(|v| serde_json::to_string(v).ok())
        .unwrap_or_default()
}

/// `layer_config_ranges` ↔ JSON du bridge (`[{min, max, config}]`).
fn ranges_from_json(json: &str) -> Vec<crate::api::LayerConfigRange> {
    let Ok(serde_json::Value::Array(items)) = serde_json::from_str::<serde_json::Value>(json)
    else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(|item| {
            Some(crate::api::LayerConfigRange {
                min_z: item.get("min")?.as_f64()?,
                max_z: item.get("max")?.as_f64()?,
                settings: config_from_orca_json(&item.get("config")?.to_string()),
            })
        })
        .collect()
}

pub(super) fn ranges_to_json(ranges: &[crate::api::LayerConfigRange]) -> String {
    if ranges.is_empty() {
        return String::new();
    }
    let items: Vec<String> = ranges
        .iter()
        .map(|r| {
            format!(
                "{{\"min\":{},\"max\":{},\"config\":{}}}",
                r.min_z,
                r.max_z,
                config_to_orca_json(&r.settings)
            )
        })
        .collect();
    format!("[{}]", items.join(","))
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
        paint: crate::api::VolumePaint {
            facets: raw
                .paint_channel
                .iter()
                .zip(&raw.paint_triangle)
                .zip(&raw.paint_state)
                .map(|((&channel, &triangle), state)| {
                    (paint_channel_from_code(channel), triangle, state.clone())
                })
                .collect(),
        },
        settings: config_from_orca_json(&raw.config_json),
        emboss: (!raw.emboss_json.trim().is_empty())
            .then(|| serde_json::from_str(&raw.emboss_json).ok())
            .flatten(),
        cut_info: cut_from_json(&raw.cut_json),
        mesh_stats: cut_from_json(&raw.stats_json),
    }
}

fn paint_channel_code(channel: crate::api::PaintChannel) -> u8 {
    use crate::api::PaintChannel::*;
    match channel {
        Supports => 0,
        Seam => 1,
        Mmu => 2,
        Fuzzy => 3,
    }
}

fn paint_channel_from_code(code: u8) -> crate::api::PaintChannel {
    use crate::api::PaintChannel::*;
    match code {
        1 => Seam,
        2 => Mmu,
        3 => Fuzzy,
        _ => Supports,
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
                    paint_channel: v
                        .paint
                        .facets
                        .iter()
                        .map(|f| paint_channel_code(f.0))
                        .collect(),
                    paint_triangle: v.paint.facets.iter().map(|f| f.1).collect(),
                    paint_state: v.paint.facets.iter().map(|f| f.2.clone()).collect(),
                    config_json: config_to_orca_json(&v.settings),
                    emboss_json: v.emboss.as_ref().map(|e| e.to_string()).unwrap_or_default(),
                    cut_json: cut_to_json(v.cut_info.as_ref()),
                    stats_json: cut_to_json(v.mesh_stats.as_ref()),
                })
                .collect(),
            instances: o
                .instances
                .iter()
                .map(|i| ffi::RawInstance {
                    matrix: i.matrix.to_cols_array(),
                    printable: i.printable,
                })
                .collect(),
            config_json: config_to_orca_json(&o.settings),
            layer_height_profile: o.layer_height_profile.clone(),
            layer_ranges_json: ranges_to_json(&o.layer_config_ranges),
            cut_json: cut_to_json(o.cut_id.as_ref()),
            brim_points: o
                .brim_points
                .iter()
                .flat_map(|p| [p.pos[0], p.pos[1], p.pos[2], p.head_front_radius])
                .collect(),
        })
        .collect()
}

/// Configuration typée → JSON {clé → valeur Orca}, partagé par slicing et 3MF.
pub(super) fn config_to_orca_json(config: &DynamicPrintConfig) -> String {
    let map: std::collections::BTreeMap<&str, String> = config
        .0
        .iter()
        .map(|(key, value)| {
            let serialized = match params::get(key) {
                Some(def) => orca_values::serialize_orca_value_for(def, value),
                None => orca_values::serialize_orca_value(value),
            };
            (key.as_str(), serialized)
        })
        .collect();
    serde_json::to_string(&map).expect("map de chaînes toujours sérialisable")
}

/// JSON {clé → valeur Orca} → configuration typée, selon le registre généré.
pub(super) fn config_from_orca_json(json: &str) -> DynamicPrintConfig {
    let mut config = DynamicPrintConfig::new();
    if json.is_empty() {
        return config;
    }
    let Ok(values) = serde_json::from_str::<std::collections::BTreeMap<String, String>>(json)
    else {
        return config;
    };
    for (key, raw) in values {
        if let Some(def) = params::get(&key) {
            if let Some(value) = orca_values::parse_orca_value(def, &raw) {
                config.0.insert(key, value);
            }
        }
    }
    config
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn float_vectors_use_commas_in_worker_json() {
        let mut config = DynamicPrintConfig::new();
        config.0.insert(
            "machine_max_acceleration_e".into(),
            crate::api::ConfigValue::Floats(vec![5000.0, 5000.0]),
        );
        let json = config_to_orca_json(&config);
        assert!(json.contains("\"5000,5000\""), "json={json}");
    }
}
