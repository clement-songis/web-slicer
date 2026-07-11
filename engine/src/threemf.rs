//! Écriture **pure Rust** d'un projet 3MF compatible OrcaSlicer (T072, FR-044).
//!
//! Produit l'archive OPC standard qu'OrcaSlicer/PrusaSlicer lisent (miroir de
//! `libslic3r/Format/3mf.cpp`) : `[Content_Types].xml`, `_rels/.rels`,
//! `3D/3dmodel.model` (géométrie 3MF core) et `Metadata/Slic3r_PE.config`
//! (configuration figée, lignes `; key = value`). Ce chemin couvre géométrie +
//! configuration ; la fidélité fine (config par objet `Slic3r_PE_model.config`,
//! peinture, plateaux, vignette) reste au writer libslic3r (`adapters::ffi`).

use std::io::{Cursor, Read, Write};

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::api::{DynamicPrintConfig, EngineError, EngineErrorCode, EngineResult, Model};
use crate::params::orca_values;

const CONTENT_TYPES_FILE: &str = "[Content_Types].xml";
const RELATIONSHIPS_FILE: &str = "_rels/.rels";
const MODEL_FILE: &str = "3D/3dmodel.model";
const PRINT_CONFIG_FILE: &str = "Metadata/Slic3r_PE.config";

const CONTENT_TYPES: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
 <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
 <Default Extension="model" ContentType="application/vnd.ms-package.3dmanufacturing-3dmodel+xml"/>
 <Default Extension="png" ContentType="image/png"/>
</Types>"#;

const RELATIONSHIPS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
 <Relationship Target="/3D/3dmodel.model" Id="rel-1" Type="http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel"/>
</Relationships>"#;

/// Sérialise un modèle + une configuration en octets d'un projet 3MF (FR-044).
pub fn write_project_bytes(model: &Model, config: &DynamicPrintConfig) -> EngineResult<Vec<u8>> {
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

    let io = |e: zip::result::ZipError| EngineError::new(EngineErrorCode::Io, e.to_string());
    let wr = |e: std::io::Error| EngineError::new(EngineErrorCode::Io, e.to_string());

    zip.start_file(CONTENT_TYPES_FILE, opts).map_err(io)?;
    zip.write_all(CONTENT_TYPES.as_bytes()).map_err(wr)?;

    zip.start_file(RELATIONSHIPS_FILE, opts).map_err(io)?;
    zip.write_all(RELATIONSHIPS.as_bytes()).map_err(wr)?;

    zip.start_file(MODEL_FILE, opts).map_err(io)?;
    zip.write_all(model_xml(model).as_bytes()).map_err(wr)?;

    zip.start_file(PRINT_CONFIG_FILE, opts).map_err(io)?;
    zip.write_all(config_text(config).as_bytes()).map_err(wr)?;

    let cursor = zip.finish().map_err(io)?;
    Ok(cursor.into_inner())
}

/// Construit le XML `3D/3dmodel.model` : un `<object>` par objet (volumes
/// fusionnés dans le repère monde), un `<item>` par instance.
fn model_xml(model: &Model) -> String {
    let mut s = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <model unit=\"millimeter\" xml:lang=\"en-US\" \
         xmlns=\"http://schemas.microsoft.com/3dmanufacturing/core/2015/02\" \
         xmlns:slic3rpe=\"http://schemas.slic3r.org/3mf/2017/06\">\n\
         <resources>\n",
    );

    for (i, object) in model.objects.iter().enumerate() {
        let id = i + 1;
        s.push_str(&format!(" <object id=\"{id}\" type=\"model\">\n  <mesh>\n"));
        let (vertices, triangles) = merge_object(object);
        s.push_str("   <vertices>\n");
        for v in &vertices {
            s.push_str(&format!(
                "    <vertex x=\"{}\" y=\"{}\" z=\"{}\"/>\n",
                v[0], v[1], v[2]
            ));
        }
        s.push_str("   </vertices>\n   <triangles>\n");
        for t in &triangles {
            s.push_str(&format!(
                "    <triangle v1=\"{}\" v2=\"{}\" v3=\"{}\"/>\n",
                t[0], t[1], t[2]
            ));
        }
        s.push_str("   </triangles>\n  </mesh>\n </object>\n");
    }

    s.push_str(" </resources>\n <build>\n");
    for (i, object) in model.objects.iter().enumerate() {
        let id = i + 1;
        let instances = object.instances.len().max(1);
        for _ in 0..instances {
            s.push_str(&format!("  <item objectid=\"{id}\"/>\n"));
        }
    }
    s.push_str(" </build>\n</model>\n");
    s
}

/// Fusionne les volumes d'un objet (chaque volume transformé par sa matrice)
/// en une paire (sommets, triangles) indexée globalement.
fn merge_object(object: &crate::api::ModelObject) -> (Vec<[f32; 3]>, Vec<[u32; 3]>) {
    let mut vertices: Vec<[f32; 3]> = Vec::new();
    let mut triangles: Vec<[u32; 3]> = Vec::new();
    for volume in &object.volumes {
        let base = vertices.len() as u32;
        for v in &volume.mesh.vertices {
            let p = volume.matrix.transform_point3(glam::DVec3::new(
                v[0] as f64,
                v[1] as f64,
                v[2] as f64,
            ));
            vertices.push([p.x as f32, p.y as f32, p.z as f32]);
        }
        for t in &volume.mesh.indices {
            triangles.push([t[0] + base, t[1] + base, t[2] + base]);
        }
    }
    (vertices, triangles)
}

/// Contenu de `Metadata/Slic3r_PE.config` : lignes `; key = value` (Orca).
fn config_text(config: &DynamicPrintConfig) -> String {
    let mut out = String::from("; generated by web-slicer\n\n");
    for (key, value) in &config.0 {
        // `compatible_printers` est exclu par Orca de ce fichier.
        if key == "compatible_printers" {
            continue;
        }
        out.push_str(&format!(
            "; {key} = {}\n",
            orca_values::serialize_orca_value(value)
        ));
    }
    out
}

/// Configuration relue d'un projet 3MF (round-trip / validation croisée).
#[derive(Debug, Default, PartialEq, Eq)]
pub struct ProjectArchive {
    /// Noms des entrées présentes dans l'archive.
    pub entries: Vec<String>,
    /// Paires `key = value` du fichier de configuration.
    pub config: std::collections::BTreeMap<String, String>,
    /// Nombre de sommets et de triangles trouvés dans `3dmodel.model`.
    pub vertex_count: usize,
    pub triangle_count: usize,
}

/// Relit une archive produite par [`write_project_bytes`] (test de round-trip).
pub fn read_project_bytes(bytes: &[u8]) -> EngineResult<ProjectArchive> {
    let io =
        |e: zip::result::ZipError| EngineError::new(EngineErrorCode::InvalidModel, e.to_string());
    let mut archive = ZipArchive::new(Cursor::new(bytes)).map_err(io)?;

    let mut out = ProjectArchive::default();
    let mut model_xml = String::new();
    let mut config_text = String::new();

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(io)?;
        let name = file.name().to_string();
        out.entries.push(name.clone());
        let mut content = String::new();
        let _ = file.read_to_string(&mut content);
        if name == MODEL_FILE {
            model_xml = content;
        } else if name == PRINT_CONFIG_FILE {
            config_text = content;
        }
    }

    out.vertex_count = model_xml.matches("<vertex ").count();
    out.triangle_count = model_xml.matches("<triangle ").count();
    for line in config_text.lines() {
        let line = line.trim_start_matches(';').trim();
        if let Some((k, v)) = line.split_once('=') {
            out.config
                .insert(k.trim().to_string(), v.trim().to_string());
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{ConfigValue, ModelObject, ModelVolume, TriangleMesh, VolumeRole};
    use glam::DMat4;

    fn tetra() -> TriangleMesh {
        TriangleMesh {
            vertices: vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 1.0],
            ],
            indices: vec![[0, 1, 2], [0, 1, 3], [1, 2, 3], [0, 2, 3]],
        }
    }

    fn model() -> Model {
        Model {
            objects: vec![ModelObject {
                name: "cube".into(),
                volumes: vec![ModelVolume {
                    name: String::new(),
                    mesh: tetra(),
                    matrix: DMat4::IDENTITY,
                    role: VolumeRole::ModelPart,
                    extruder: None,
                }],
                instances: vec![],
            }],
        }
    }

    fn config() -> DynamicPrintConfig {
        let mut c = DynamicPrintConfig::new();
        c.0.insert("layer_height".into(), ConfigValue::Float(0.2));
        c.0.insert(
            "compatible_printers".into(),
            ConfigValue::String("X".into()),
        );
        c
    }

    #[test]
    fn writes_the_standard_opc_entries() {
        let bytes = write_project_bytes(&model(), &config()).unwrap();
        let archive = read_project_bytes(&bytes).unwrap();
        for expected in [
            CONTENT_TYPES_FILE,
            RELATIONSHIPS_FILE,
            MODEL_FILE,
            PRINT_CONFIG_FILE,
        ] {
            assert!(
                archive.entries.iter().any(|e| e == expected),
                "manque {expected}"
            );
        }
    }

    #[test]
    fn round_trips_geometry_and_config() {
        let bytes = write_project_bytes(&model(), &config()).unwrap();
        let archive = read_project_bytes(&bytes).unwrap();
        assert_eq!(archive.vertex_count, 4);
        assert_eq!(archive.triangle_count, 4);
        // La configuration figée est relue…
        assert_eq!(
            archive.config.get("layer_height").map(String::as_str),
            Some("0.2")
        );
        // …sans `compatible_printers` (exclu par Orca).
        assert!(!archive.config.contains_key("compatible_printers"));
    }

    #[test]
    fn applies_volume_matrix_to_vertices() {
        let mut m = model();
        m.objects[0].volumes[0].matrix = DMat4::from_translation(glam::DVec3::new(10.0, 0.0, 0.0));
        let bytes = write_project_bytes(&m, &config()).unwrap();
        let xml = String::from_utf8({
            let mut a = ZipArchive::new(Cursor::new(&bytes)).unwrap();
            let mut f = a.by_name(MODEL_FILE).unwrap();
            let mut s = Vec::new();
            std::io::copy(&mut f, &mut s).unwrap();
            s
        })
        .unwrap();
        // Le premier sommet (0,0,0) translaté de +10 en x.
        assert!(xml.contains("x=\"10\""), "sommet translaté attendu");
    }
}
