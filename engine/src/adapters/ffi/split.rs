//! Découpe de maillage via libslic3r (split.cpp) : séparation en pièces,
//! coupe plane et booléen mcut. Appelé uniquement depuis `engine-worker`
//! (sous-commande `mesh-op`) : le backend ne lie jamais la FFI.

use crate::api::split::MeshOp;
use crate::api::{EngineError, EngineErrorCode, EngineResult, TriangleMesh};

use super::bridge::ffi;
use super::model::ffi_guard;

fn to_raw(mesh: &TriangleMesh) -> ffi::RawMesh {
    ffi::RawMesh {
        vertices: mesh.vertices.iter().flatten().copied().collect(),
        indices: mesh.indices.iter().flatten().copied().collect(),
    }
}

fn from_raw(raw: &ffi::RawMesh) -> TriangleMesh {
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

fn invalid(message: impl Into<String>) -> EngineError {
    EngineError::new(EngineErrorCode::InvalidModel, message)
}

/// Exécute `op` sur `meshes` et renvoie les pièces étiquetées
/// (`part_<n>`, `upper`/`lower`, `result`). Validation pure d'abord ;
/// un résultat sans objet (une seule pièce, plan hors objet, booléen vide)
/// remonte en `InvalidModel` (erreur utilisateur, 422 côté HTTP).
pub fn run_mesh_op(
    op: &MeshOp,
    meshes: &[TriangleMesh],
) -> EngineResult<Vec<(String, TriangleMesh)>> {
    op.validate(meshes).map_err(|e| invalid(e.to_string()))?;
    let _guard = ffi_guard();
    match op {
        &MeshOp::Primitive { shape, side_mm } => {
            let raw = ffi::make_primitive_raw(shape.code(), side_mm)
                .map_err(|e| invalid(e.to_string()))?;
            Ok(vec![(shape.label().to_string(), from_raw(&raw))])
        }
        MeshOp::EmbossSvg {
            svg,
            width_mm,
            depth_mm,
        } => {
            let raw = ffi::emboss_svg_raw(svg, *width_mm, *depth_mm)
                .map_err(|e| invalid(e.to_string()))?;
            Ok(vec![("svg".to_string(), from_raw(&raw))])
        }
        MeshOp::EmbossText {
            font_path,
            text,
            size_mm,
            depth_mm,
            boldness_mm,
        } => {
            let path = font_path
                .to_str()
                .ok_or_else(|| invalid("chemin de police non UTF-8"))?;
            let raw = ffi::emboss_text_raw(path, text, *size_mm, *depth_mm, *boldness_mm)
                .map_err(|e| invalid(e.to_string()))?;
            Ok(vec![("text".to_string(), from_raw(&raw))])
        }
        MeshOp::Split => {
            let parts =
                ffi::split_mesh_raw(&to_raw(&meshes[0])).map_err(|e| invalid(e.to_string()))?;
            // Message d'Orca (Plater::split_object) : objet à une seule pièce.
            if parts.len() < 2 {
                return Err(invalid(
                    "l'objet ne peut pas être divisé : il ne contient qu'une seule pièce",
                ));
            }
            Ok(parts
                .iter()
                .enumerate()
                .map(|(i, raw)| (format!("part_{}", i + 1), from_raw(raw)))
                .collect())
        }
        &MeshOp::Cut {
            plane,
            keep_upper,
            keep_lower,
        } => {
            let raw_plane = ffi::RawCutPlane {
                point: plane.point,
                normal: plane.normal,
                keep_upper,
                keep_lower,
            };
            let result = ffi::cut_mesh_raw(&to_raw(&meshes[0]), &raw_plane)
                .map_err(|e| invalid(e.to_string()))?;
            if result.upper_facets == 0 || result.lower_facets == 0 {
                return Err(invalid("le plan de coupe ne traverse pas l'objet"));
            }
            let mut out = Vec::with_capacity(2);
            if keep_upper {
                out.push(("upper".to_string(), from_raw(&result.upper)));
            }
            if keep_lower {
                out.push(("lower".to_string(), from_raw(&result.lower)));
            }
            Ok(out)
        }
        MeshOp::Smooth => {
            let raw =
                ffi::smooth_mesh_raw(&to_raw(&meshes[0])).map_err(|e| invalid(e.to_string()))?;
            Ok(vec![("result".to_string(), from_raw(&raw))])
        }
        &MeshOp::Boolean { op } => {
            let raw = ffi::boolean_mesh_raw(&to_raw(&meshes[0]), &to_raw(&meshes[1]), op.code())
                .map_err(|e| invalid(e.to_string()))?;
            Ok(vec![("result".to_string(), from_raw(&raw))])
        }
    }
}
