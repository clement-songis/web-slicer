//! Session de peinture libslic3r (paint.cpp) : maillage + `TriangleSelector` par
//! canal, gardés en mémoire par le worker persistant.

use cxx::UniquePtr;

use crate::api::paint::{
    validate_extruder_map, validate_gap_area, PaintFacets, PaintStroke, PaintTool,
    MAX_PREVIEW_TRIANGLES, MAX_SESSION_TRIANGLES,
};
use crate::api::{EngineError, EngineErrorCode, EngineResult, PaintChannel, TriangleMesh};

use super::bridge::ffi;

fn invalid(m: impl Into<String>) -> EngineError {
    EngineError::new(EngineErrorCode::InvalidModel, m)
}

pub(super) fn channel_code(channel: PaintChannel) -> u8 {
    match channel {
        PaintChannel::Supports => 0,
        PaintChannel::Seam => 1,
        PaintChannel::Mmu => 2,
        PaintChannel::Fuzzy => 3,
    }
}

/// Session de peinture (non `Send` : propre au thread du worker).
pub struct PaintSession {
    inner: UniquePtr<ffi::PaintSession>,
    triangles: usize,
}

impl PaintSession {
    pub fn new(mesh: &TriangleMesh) -> EngineResult<Self> {
        if mesh.indices.is_empty() || mesh.indices.len() > MAX_SESSION_TRIANGLES {
            return Err(invalid(format!(
                "maillage vide ou trop grand ({MAX_SESSION_TRIANGLES} triangles max)"
            )));
        }
        let raw = ffi::RawMesh {
            vertices: mesh.vertices.iter().flatten().copied().collect(),
            indices: mesh.indices.iter().flatten().copied().collect(),
        };
        let inner = ffi::paint_session_new(&raw).map_err(|e| invalid(e.to_string()))?;
        let triangles = ffi::paint_session_triangles(&inner);
        Ok(Self { inner, triangles })
    }

    pub fn triangles(&self) -> usize {
        self.triangles
    }

    /// Remplace l'état d'un canal par des chaînes 3MF.
    pub fn load(&mut self, channel: PaintChannel, facets: &PaintFacets) -> EngineResult<()> {
        let raw = ffi::RawPaintFacets {
            triangles: facets.keys().copied().collect(),
            states: facets.values().cloned().collect(),
        };
        ffi::paint_session_load(self.inner.pin_mut(), channel_code(channel), &raw)
            .map_err(|e| invalid(e.to_string()))
    }

    /// Applique un trait (`select_patch`, remplissages, triangle, plage de hauteur).
    pub fn stroke(&mut self, channel: PaintChannel, stroke: &PaintStroke) -> EngineResult<()> {
        stroke.validate().map_err(invalid)?;
        ffi::paint_session_stroke(
            self.inner.pin_mut(),
            channel_code(channel),
            &raw_stroke(stroke),
        )
        .map_err(|e| invalid(e.to_string()))
    }

    /// Comblement des interstices (outil Gap fill) : aire seuil en mm².
    pub fn gap_fill(&mut self, channel: PaintChannel, area: f32) -> EngineResult<()> {
        validate_gap_area(area).map_err(invalid)?;
        ffi::paint_session_gap_fill(self.inner.pin_mut(), channel_code(channel), area)
            .map_err(|e| invalid(e.to_string()))
    }

    /// Remappage des filaments MMU : `map[i]` = destination de l'extrudeur `i + 1`.
    pub fn remap_states(&mut self, channel: PaintChannel, map: &[u8]) -> EngineResult<()> {
        validate_extruder_map(map).map_err(invalid)?;
        ffi::paint_session_remap_states(self.inner.pin_mut(), channel_code(channel), map)
            .map_err(|e| invalid(e.to_string()))
    }

    /// Aperçu au survol d'un outil à propagation (premier point du trait) :
    /// sommets des triangles sélectionnés + drapeau de troncature.
    pub fn preview(
        &mut self,
        channel: PaintChannel,
        stroke: &PaintStroke,
    ) -> EngineResult<(Vec<f32>, bool)> {
        stroke.validate().map_err(invalid)?;
        let propagating = stroke.tool != PaintTool::Brush
            || stroke.cursor == crate::api::paint::CursorShape::Pointer;
        if !propagating {
            return Err(invalid(
                "aperçu réservé au smart fill, au pot de peinture et au curseur triangle",
            ));
        }
        let raw = ffi::paint_session_preview(
            self.inner.pin_mut(),
            channel_code(channel),
            &raw_stroke(stroke),
            MAX_PREVIEW_TRIANGLES,
        )
        .map_err(|e| invalid(e.to_string()))?;
        Ok((raw.vertices, raw.truncated))
    }

    /// État du canal en chaînes 3MF.
    pub fn export(&self, channel: PaintChannel) -> EngineResult<PaintFacets> {
        let raw = ffi::paint_session_export(&self.inner, channel_code(channel))
            .map_err(|e| invalid(e.to_string()))?;
        Ok(raw.triangles.into_iter().zip(raw.states).collect())
    }
}

fn raw_stroke(stroke: &PaintStroke) -> ffi::RawPaintStroke {
    ffi::RawPaintStroke {
        points: stroke.points.iter().flatten().copied().collect(),
        facets: stroke.facets.clone(),
        camera_pos: stroke.camera_pos,
        radius: stroke.radius,
        cursor: stroke.cursor.code(),
        state: stroke.state,
        trafo: stroke.trafo,
        splitting: stroke.splitting,
        highlight_deg: stroke.highlight_deg,
        tool: stroke.tool.code(),
        fill_angle: stroke.fill_angle,
        height: stroke.height,
        has_z_world: stroke.z_world.is_some(),
        z_world: stroke.z_world.unwrap_or(0.0),
    }
}

/// « Garder la peinture » (préférence keep_painting d'Orca) : peinture d'un
/// canal de `source` reportée sur `target` après un changement de maillage
/// (ModelVolume::save_painting / restore_painting → remap_painting).
pub fn remap_painting(
    source: &TriangleMesh,
    target: &TriangleMesh,
    facets: &PaintFacets,
) -> EngineResult<PaintFacets> {
    if facets.is_empty() {
        return Ok(PaintFacets::new());
    }
    let raw_mesh = |m: &TriangleMesh| ffi::RawMesh {
        vertices: m.vertices.iter().flatten().copied().collect(),
        indices: m.indices.iter().flatten().copied().collect(),
    };
    let raw = ffi::RawPaintFacets {
        triangles: facets.keys().copied().collect(),
        states: facets.values().cloned().collect(),
    };
    let out = ffi::remap_painting_raw(&raw_mesh(source), &raw_mesh(target), &raw)
        .map_err(|e| invalid(e.to_string()))?;
    Ok(out.triangles.into_iter().zip(out.states).collect())
}
