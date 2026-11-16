//! Session de peinture libslic3r (paint.cpp) : maillage + `TriangleSelector` par
//! canal, gardés en mémoire par le worker persistant.

use cxx::UniquePtr;

use crate::api::paint::{CursorShape, PaintFacets, PaintStroke, MAX_SESSION_TRIANGLES};
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

    /// Applique un trait (`select_patch`).
    pub fn stroke(&mut self, channel: PaintChannel, stroke: &PaintStroke) -> EngineResult<()> {
        stroke.validate().map_err(invalid)?;
        let raw = ffi::RawPaintStroke {
            points: stroke.points.iter().flatten().copied().collect(),
            facets: stroke.facets.clone(),
            camera_pos: stroke.camera_pos,
            radius: stroke.radius,
            sphere: stroke.cursor == CursorShape::Sphere,
            state: stroke.state,
            trafo: stroke.trafo,
            splitting: stroke.splitting,
            highlight_deg: stroke.highlight_deg,
            tool: stroke.tool.code(),
            fill_angle: stroke.fill_angle,
        };
        ffi::paint_session_stroke(self.inner.pin_mut(), channel_code(channel), &raw)
            .map_err(|e| invalid(e.to_string()))
    }

    /// État du canal en chaînes 3MF.
    pub fn export(&self, channel: PaintChannel) -> EngineResult<PaintFacets> {
        let raw = ffi::paint_session_export(&self.inner, channel_code(channel))
            .map_err(|e| invalid(e.to_string()))?;
        Ok(raw.triangles.into_iter().zip(raw.states).collect())
    }
}
