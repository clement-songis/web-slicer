//! Découpe de maillage via libslic3r (split.cpp) : séparation en pièces,
//! coupe plane et booléen mcut. Appelé uniquement depuis `engine-worker`
//! (sous-commande `mesh-op`) : le backend ne lie jamais la FFI.

use crate::api::split::{CutOptions, CutPlane, CutVolumeMeta, MeshOp, MeshOpPiece};
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

/// Comme [`run_mesh_op`] avec les volumes additionnels de la découpe
/// complète (connecteurs, 2e pièce) : c'est ce qu'utilise `engine-worker`.
pub fn run_mesh_op_pieces(op: &MeshOp, meshes: &[TriangleMesh]) -> EngineResult<Vec<MeshOpPiece>> {
    if let MeshOp::Cut { options, .. } = op {
        if !options.is_plain() {
            op.validate(meshes).map_err(|e| invalid(e.to_string()))?;
            let _guard = ffi_guard();
            return run_full_cut(op, meshes);
        }
    }
    Ok(run_mesh_op(op, meshes)?
        .into_iter()
        .map(|(label, mesh)| MeshOpPiece {
            label,
            mesh,
            volume: None,
        })
        .collect())
}

fn role_name(role: u8) -> &'static str {
    match role {
        1 => "negative",
        2 => "modifier",
        3 => "blocker",
        4 => "enforcer",
        _ => "part",
    }
}

fn connector_request(
    plane: &CutPlane,
    options: &CutOptions,
    keep: (bool, bool),
) -> ffi::RawCutRequest {
    let groove = options.groove;
    let mut instance = [0.0; 16];
    for i in 0..4 {
        instance[i * 5] = 1.0;
    }
    ffi::RawCutRequest {
        point: plane.point,
        normal: plane.normal,
        has_rotation: options.rotation_deg.is_some(),
        rotation: options
            .rotation_deg
            .unwrap_or_default()
            .map(f64::to_radians),
        instance: options.instance_matrix.unwrap_or(instance),
        keep_upper: keep.0,
        keep_lower: keep.1,
        keep_as_parts: options.keep_as_parts,
        place_on_cut_upper: options.place_on_cut_upper,
        place_on_cut_lower: options.place_on_cut_lower,
        flip_upper: options.flip_upper,
        flip_lower: options.flip_lower,
        connectors: options
            .connectors
            .iter()
            .map(|c| ffi::RawCutConnector {
                pos: c.pos,
                // GLGizmoCut3D::add_connector : rayon = taille / 2, tolérance
                // de rayon = tolérance de taille / 2.
                radius: c.size_mm * 0.5,
                height: c.depth_mm,
                radius_tolerance: c.size_tolerance_mm * 0.5,
                height_tolerance: c.depth_tolerance_mm,
                z_angle: c.rotation_deg.to_radians(),
                kind: c.kind as u8,
                style: c.style as u8,
                shape: c.shape as u8,
            })
            .collect(),
        // GLGizmoCut3D : m_snap_space_proportion 0,3, m_snap_bulge_proportion 0,15.
        snap_space_proportion: 0.3,
        snap_bulge_proportion: 0.15,
        groove: groove.is_some(),
        groove_depth: groove.map_or(0.0, |g| g.depth_mm),
        groove_width: groove.map_or(0.0, |g| g.width()),
        groove_flaps_angle: groove.map_or(0.0, |g| g.flaps_angle_deg.to_radians()),
        groove_angle: groove.map_or(0.0, |g| g.angle_deg.to_radians()),
        groove_depth_tolerance: groove.map_or(0.0, |g| g.depth_tolerance_mm),
        groove_width_tolerance: groove.map_or(0.0, |g| g.width_tolerance_mm),
        groove_count: groove.map_or(1, |g| g.count),
        groove_gap: groove.map_or(0.0, |g| g.gap_mm),
    }
}

/// Découpe complète (`cut_object_raw` → `Slic3r::Cut`). Étiquettes des
/// objets dans l'ordre de `Cut` : `upper`, `lower` (ou `parts` si
/// « garder comme pièces »), puis `dowel_<n>` ; pour chaque objet, le premier
/// volume pièce non connecteur est le corps, les autres sont des volumes
/// additionnels (`MeshOpPiece::volume`).
fn run_full_cut(op: &MeshOp, meshes: &[TriangleMesh]) -> EngineResult<Vec<MeshOpPiece>> {
    let MeshOp::Cut {
        plane,
        keep_upper,
        keep_lower,
        options,
    } = op
    else {
        unreachable!("run_full_cut appelé hors coupe");
    };
    // GLGizmoCut3D::perform_cut : connecteurs ⇒ deux moitiés, pas de pièces.
    let has_connectors = !options.connectors.is_empty();
    let as_parts = options.keep_as_parts && !has_connectors && options.groove.is_none();
    // « Garder comme pièces » : le gizmo grise les cases haut/bas (Cut clone
    // alors seulement la moitié haute, qui reçoit les deux volumes).
    let keep = if has_connectors || as_parts {
        (true, true)
    } else {
        (*keep_upper, *keep_lower)
    };
    let request = connector_request(plane, options, keep);
    let objects =
        ffi::cut_object_raw(&to_raw(&meshes[0]), &request).map_err(|e| invalid(e.to_string()))?;
    let mut labels: Vec<String> = Vec::new();
    if as_parts {
        labels.push("parts".into());
    } else {
        if keep.0 {
            labels.push("upper".into());
        }
        if keep.1 {
            labels.push("lower".into());
        }
    }
    let mut pieces = Vec::new();
    let mut dowel = 0;
    for (index, object) in objects.iter().enumerate() {
        let label = labels.get(index).cloned().unwrap_or_else(|| {
            dowel += 1;
            format!("dowel_{dowel}")
        });
        let body = object
            .volumes
            .iter()
            .position(|v| v.role == 0 && !v.connector)
            .or_else(|| object.volumes.iter().position(|v| v.role == 0))
            .ok_or_else(|| invalid("objet coupé sans pièce solide"))?;
        pieces.push(MeshOpPiece {
            label: label.clone(),
            mesh: from_raw(&object.volumes[body].mesh),
            volume: None,
        });
        for (i, volume) in object.volumes.iter().enumerate() {
            if i == body {
                continue;
            }
            pieces.push(MeshOpPiece {
                label: label.clone(),
                mesh: from_raw(&volume.mesh),
                volume: Some(CutVolumeMeta {
                    name: volume.name.clone(),
                    role: role_name(volume.role).into(),
                    connector: volume.connector,
                }),
            });
        }
    }
    if pieces.is_empty() {
        return Err(invalid("la coupe n'a produit aucune pièce"));
    }
    Ok(pieces)
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
        MeshOp::Cut { options, .. } if !options.is_plain() => Ok(run_full_cut(op, meshes)?
            .into_iter()
            .filter(|piece| piece.volume.is_none())
            .map(|piece| (piece.label, piece.mesh))
            .collect()),
        &MeshOp::Cut {
            plane,
            keep_upper,
            keep_lower,
            ..
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
