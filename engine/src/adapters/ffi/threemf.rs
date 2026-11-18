//! Projets 3MF via libslic3r (T014) : lecture scène + config embarquée,
//! écriture compatible OrcaSlicer (FR-044).

use std::collections::BTreeMap;
use std::path::Path;

use crate::api::{
    ConfigValue, DynamicPrintConfig, EngineError, EngineErrorCode, EngineResult, Model,
};
use crate::params::{self, orca_values};

use super::bridge::ffi;
use super::model::{ffi_guard, to_object, to_raw_objects};

pub fn read_project_3mf(path: &Path) -> EngineResult<(Model, DynamicPrintConfig)> {
    let _guard = ffi_guard();
    let raw = ffi::read_project_3mf_raw(&path.to_string_lossy())
        .map_err(|e| EngineError::new(EngineErrorCode::InvalidModel, e.to_string()))?;
    let model = Model {
        objects: raw.objects.into_iter().map(to_object).collect(),
        custom_gcodes: serde_json::from_str(&raw.custom_gcodes_json).unwrap_or_default(),
    };
    let config = config_from_orca_json(&raw.config_json)?;
    Ok((model, config))
}

/// Vue d'import d'un projet 3MF lue par libslic3r (`Model::read_from_file`,
/// `load_bbs_3mf` : Orca, PrusaSlicer, 3MF générique), réglages au format
/// texte Orca (`opt_serialize`) tels qu'écrits dans le fichier :
/// `{ objects: [{ name, settings, layer_profile, layer_ranges, instances:
/// [{matrix, printable}], volumes: [{ name, role, matrix, extruder, settings,
/// vertices, indices, paint: [[canal, triangle, hex]] }] }], config, plates:
/// [{ index, name, settings, obj_inst }] }` (rôles : part, negative, modifier,
/// blocker, enforcer ; canaux : supports, seam, mmu, fuzzy).
pub fn read_project_3mf_import(path: &Path) -> EngineResult<serde_json::Value> {
    use serde_json::{json, Value};
    let _guard = ffi_guard();
    let raw = ffi::read_project_3mf_raw(&path.to_string_lossy())
        .map_err(|e| EngineError::new(EngineErrorCode::InvalidModel, e.to_string()))?;
    let text_map = |s: &str| -> Value {
        if s.trim().is_empty() {
            json!({})
        } else {
            serde_json::from_str(s).unwrap_or_else(|_| json!({}))
        }
    };
    const ROLES: [&str; 5] = ["part", "negative", "modifier", "blocker", "enforcer"];
    const CHANNELS: [&str; 4] = ["supports", "seam", "mmu", "fuzzy"];
    let objects: Vec<Value> = raw
        .objects
        .iter()
        .map(|o| {
            let volumes: Vec<Value> = o
                .volumes
                .iter()
                .map(|v| {
                    let paint: Vec<Value> = v
                        .paint_channel
                        .iter()
                        .zip(&v.paint_triangle)
                        .zip(&v.paint_state)
                        .map(|((c, t), s)| json!([CHANNELS.get(*c as usize).copied().unwrap_or("supports"), t, s]))
                        .collect();
                    json!({
                        "name": v.name,
                        "role": ROLES.get(v.role as usize).copied().unwrap_or("part"),
                        "matrix": v.matrix.to_vec(),
                        "extruder": v.extruder,
                        "settings": text_map(&v.config_json),
                        "vertices": v.mesh.vertices,
                        "indices": v.mesh.indices,
                        "paint": paint,
                        "emboss": if v.emboss_json.trim().is_empty() { Value::Null } else { serde_json::from_str::<Value>(&v.emboss_json).unwrap_or(Value::Null) },
                    })
                })
                .collect();
            json!({
                "name": o.name,
                "settings": text_map(&o.config_json),
                "layer_profile": o.layer_height_profile,
                "layer_ranges": if o.layer_ranges_json.trim().is_empty() { json!([]) } else { serde_json::from_str::<Value>(&o.layer_ranges_json).unwrap_or(json!([])) },
                "instances": o.instances.iter().map(|i| json!({ "matrix": i.matrix.to_vec(), "printable": i.printable })).collect::<Vec<_>>(),
                "volumes": volumes,
            })
        })
        .collect();
    let plates: Vec<Value> = raw
        .plates
        .iter()
        .map(|p| json!({ "index": p.plate_index, "name": p.name, "settings": text_map(&p.config_json), "obj_inst": p.obj_inst }))
        .collect();
    Ok(
        json!({ "objects": objects, "config": text_map(&raw.project_config_json), "plates": plates }),
    )
}

pub fn write_project_3mf(
    model: &Model,
    config: &DynamicPrintConfig,
    out: &Path,
) -> EngineResult<()> {
    let _guard = ffi_guard();
    let raw_objects = to_raw_objects(model);
    let json = config_to_orca_json(config);
    let custom = serde_json::to_string(&model.custom_gcodes).expect("sérialisable");
    ffi::write_project_3mf_raw(&raw_objects, &json, &out.to_string_lossy(), &custom)
        .map_err(|e| EngineError::new(EngineErrorCode::Io, e.to_string()))
}

/// Projet 3MF avec ses plateaux écrit par libslic3r (`store_bbs_3mf`,
/// SplitModel | ShareMesh comme « Save project » d'Orca). Les instances sont
/// rattachées aux plateaux par `ModelInstance::plate_index`.
pub fn write_project_3mf_with_plates(
    model: &Model,
    config: &DynamicPrintConfig,
    plates: &[crate::threemf::ProjectPlate],
    out: &Path,
) -> EngineResult<()> {
    write_project_3mf_with_auxiliaries(model, config, plates, None, out)
}

/// Comme [`write_project_3mf_with_plates`] avec les fichiers auxiliaires du
/// dossier `aux_dir` (arborescence « Model Pictures/… », « Others/… »),
/// empaquetés sous Auxiliaries/ par store_bbs_3mf comme dans Orca.
pub fn write_project_3mf_with_auxiliaries(
    model: &Model,
    config: &DynamicPrintConfig,
    plates: &[crate::threemf::ProjectPlate],
    aux_dir: Option<&Path>,
    out: &Path,
) -> EngineResult<()> {
    let plate_count = model
        .objects
        .iter()
        .flat_map(|o| o.instances.iter().map(|i| i.plate_index as usize + 1))
        .chain(std::iter::once(plates.len()))
        .max()
        .unwrap_or(0)
        .max(1);
    let raw_plates: Vec<ffi::RawProjectPlate> = (0..plate_count)
        .map(|plate| {
            let mut obj_inst = Vec::new();
            for (o, object) in model.objects.iter().enumerate() {
                for (i, instance) in object.instances.iter().enumerate() {
                    if instance.plate_index as usize == plate {
                        obj_inst.extend([o as u32, i as u32]);
                    }
                }
            }
            let (name, config_json, thumbnail_png) = plates.get(plate).map_or_else(
                || (String::new(), String::new(), Vec::new()),
                |p| {
                    (
                        p.name.clone(),
                        config_to_orca_json(&p.settings),
                        p.thumbnail_png.clone(),
                    )
                },
            );
            ffi::RawProjectPlate {
                plate_index: plate as u32,
                name,
                config_json,
                obj_inst,
                thumbnail_png,
            }
        })
        .collect();
    let _guard = ffi_guard();
    let raw_objects = to_raw_objects(model);
    let json = config_to_orca_json(config);
    let custom = serde_json::to_string(&model.custom_gcodes).expect("sérialisable");
    ffi::write_project_plates_raw(
        &raw_objects,
        &json,
        &custom,
        &raw_plates,
        &aux_dir
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default(),
        &out.to_string_lossy(),
    )
    .map_err(|e| EngineError::new(EngineErrorCode::Io, e.to_string()))
}

/// Fichiers auxiliaires d'un projet 3MF (Orca : Auxiliaries/) copiés dans
/// `out_dir` ; renvoie leurs chemins relatifs triés.
pub fn read_project_auxiliaries(path: &Path, out_dir: &Path) -> EngineResult<Vec<String>> {
    let _guard = ffi_guard();
    ffi::read_project_auxiliaries_raw(&path.to_string_lossy(), &out_dir.to_string_lossy())
        .map(|v| v.into_iter().collect())
        .map_err(|e| EngineError::new(EngineErrorCode::InvalidModel, e.to_string()))
}

/// G-code tranché d'un plateau pour un .gcode.3mf.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlateGcode {
    pub plate_index: u32,
    pub gcode_path: std::path::PathBuf,
}

/// « Export plate sliced file » (`export_plate = Some(n)`) ou « Export all
/// plate sliced file » (`None`) d'Orca : .gcode.3mf écrit par `store_bbs_3mf`
/// (WithGcode | SkipModel). Tous les plateaux de la scène sont décrits
/// (objets posés), seuls ceux de `gcodes` embarquent leur G-code.
pub fn write_gcode_3mf(
    model: &Model,
    config: &DynamicPrintConfig,
    gcodes: &[PlateGcode],
    export_plate: Option<u32>,
    out: &Path,
) -> EngineResult<()> {
    if gcodes.is_empty() {
        return Err(EngineError::new(
            EngineErrorCode::InvalidModel,
            "aucun plateau tranché à exporter",
        ));
    }
    let plate_count = model
        .objects
        .iter()
        .flat_map(|o| o.instances.iter().map(|i| i.plate_index + 1))
        .chain(gcodes.iter().map(|g| g.plate_index + 1))
        .max()
        .unwrap_or(1);
    let plates: Vec<ffi::RawPlateGcode> = (0..plate_count)
        .map(|plate| {
            let mut obj_inst = Vec::new();
            for (o, object) in model.objects.iter().enumerate() {
                for (i, instance) in object.instances.iter().enumerate() {
                    if instance.plate_index == plate {
                        obj_inst.extend([o as u32, i as u32]);
                    }
                }
            }
            let gcode_path = gcodes
                .iter()
                .find(|g| g.plate_index == plate)
                .map(|g| g.gcode_path.to_string_lossy().into_owned())
                .unwrap_or_default();
            ffi::RawPlateGcode {
                plate_index: plate,
                gcode_path,
                obj_inst,
            }
        })
        .collect();
    let _guard = ffi_guard();
    let raw_objects = to_raw_objects(model);
    let json = config_to_orca_json(config);
    let export_idx = export_plate.map_or(-1, |p| p as i32);
    ffi::write_gcode_3mf_raw(
        &raw_objects,
        &json,
        &plates,
        export_idx,
        &out.to_string_lossy(),
    )
    .map_err(|e| EngineError::new(EngineErrorCode::Io, e.to_string()))
}

/// JSON {clé → valeur sérialisée Orca} → config typée par le registre.
/// Les clés inconnues sont ignorées (même comportement que handle_legacy).
fn config_from_orca_json(json: &str) -> EngineResult<DynamicPrintConfig> {
    let map: BTreeMap<String, String> = serde_json::from_str(json)
        .map_err(|e| EngineError::new(EngineErrorCode::Internal, e.to_string()))?;
    let mut config = DynamicPrintConfig::new();
    for (key, text) in map {
        let Some(def) = params::get(&key) else {
            continue;
        };
        if let Some(value) = orca_values::parse_orca_value(def, &text) {
            config.0.insert(key, value);
        }
    }
    Ok(config)
}

fn config_to_orca_json(config: &DynamicPrintConfig) -> String {
    let map: BTreeMap<&str, String> = config
        .0
        .iter()
        .map(|(k, v)| (k.as_str(), serialize_for_key(k, v)))
        .collect();
    serde_json::to_string(&map).expect("map de chaînes toujours sérialisable")
}

/// Sérialise une valeur selon le type du registre (réimpose `%` aux pourcentages) ;
/// repli sur la sérialisation nue si la clé est hors registre.
fn serialize_for_key(key: &str, value: &ConfigValue) -> String {
    match params::get(key) {
        Some(def) => orca_values::serialize_orca_value_for(def, value),
        None => orca_values::serialize_orca_value(value),
    }
}

#[allow(dead_code)]
fn as_float(v: &ConfigValue) -> Option<f64> {
    match v {
        ConfigValue::Float(x) => Some(*x),
        ConfigValue::Int(i) => Some(*i as f64),
        _ => None,
    }
}
