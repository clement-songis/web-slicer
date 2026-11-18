//! Peinture moteur (TriangleSelector d'Orca) et protocole du worker persistant
//! (`engine-worker serve`, JSON-lines sur stdin/stdout).
//!
//! Pur Rust : types partagés entre le backend (qui pilote le process) et le
//! worker (qui garde maillages + `TriangleSelector` en mémoire).
//!
//! Protocole : une requête JSON par ligne sur stdin ; une réponse par requête
//! sur stdout, ligne `R <json>` (les autres lignes — logs Boost de libslic3r —
//! sont ignorées par le parent). Chaque réponse reprend l'`id` de la requête.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::layer_profile::LayerProfileRequest;
use super::{PaintChannel, TriangleMesh};

/// Facettes peintes d'un canal : indice de triangle → chaîne hex Orca
/// (`FacetsAnnotation::get_triangle_as_string`). Ordonné (BTreeMap) comme
/// l'exige `set_triangle_from_string`.
pub type PaintFacets = BTreeMap<u32, String>;

/// Curseur du pinceau (`TriangleSelector::CursorType` CIRCLE / SPHERE).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CursorShape {
    Circle,
    Sphere,
}

/// Trait de pinceau, repère du maillage de la session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaintStroke {
    /// Points touchés successifs (1 = tampon, n = capsules entre points).
    pub points: Vec<[f32; 3]>,
    /// Facette du maillage d'origine touchée par chaque point.
    pub facets: Vec<u32>,
    /// Position de la caméra dans le repère du maillage (curseur cercle).
    pub camera_pos: [f32; 3],
    /// Rayon du curseur en mm monde (`m_cursor_radius`).
    pub radius: f32,
    pub cursor: CursorShape,
    /// État posé : 0 efface, 1 enforcer, 2 blocker ; MMU : n° d'extrudeur.
    pub state: u8,
    /// Matrice monde (instance × volume), colonne-major.
    pub trafo: [f64; 16],
    /// Subdivision des triangles (`m_triangle_splitting_enabled`, vrai par défaut).
    #[serde(default = "default_true")]
    pub splitting: bool,
    /// « Paint on overhangs only » : angle (°), 0 = désactivé.
    #[serde(default)]
    pub highlight_deg: f32,
    /// Outil : pinceau (défaut), smart fill ou pot de peinture.
    #[serde(default)]
    pub tool: PaintTool,
    /// Angle d'arrêt des remplissages (°) ; défaut Orca `m_smart_fill_angle` = 30.
    #[serde(default = "default_fill_angle")]
    pub fill_angle: f32,
}

/// `GLGizmoPainterBase::ToolType` (BRUSH, SMART_FILL, BUCKET_FILL).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaintTool {
    #[default]
    Brush,
    SmartFill,
    BucketFill,
}

impl PaintTool {
    pub fn code(self) -> u8 {
        match self {
            PaintTool::Brush => 0,
            PaintTool::SmartFill => 1,
            PaintTool::BucketFill => 2,
        }
    }
}

fn default_fill_angle() -> f32 {
    30.0
}

fn default_true() -> bool {
    true
}

/// Bornes : maillage de session et trait (le worker est partagé).
pub const MAX_SESSION_TRIANGLES: usize = 2_000_000;
pub const MAX_STROKE_POINTS: usize = 4_096;

impl PaintStroke {
    pub fn validate(&self) -> Result<(), String> {
        if self.points.is_empty() || self.points.len() > MAX_STROKE_POINTS {
            return Err(format!(
                "trait vide ou trop long ({MAX_STROKE_POINTS} points max)"
            ));
        }
        if self.facets.len() != self.points.len() {
            return Err("une facette par point attendue".into());
        }
        if !(self.radius.is_finite() && self.radius > 0.0 && self.radius <= 100.0) {
            return Err("rayon hors plage (0–100 mm)".into());
        }
        if !(0.0..=90.0).contains(&self.fill_angle) {
            return Err("angle de remplissage hors plage (0–90°)".into());
        }
        if self.state > 15 {
            return Err("état de peinture hors plage (0–15)".into());
        }
        let finite = self
            .points
            .iter()
            .flatten()
            .chain(self.camera_pos.iter())
            .all(|v| v.is_finite())
            && self.trafo.iter().all(|v| v.is_finite())
            && self.highlight_deg.is_finite();
        if !finite {
            return Err("coordonnées non finies".into());
        }
        Ok(())
    }
}

/// Requête du worker persistant.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum ServeOp {
    /// Ouvre (ou remplace) une session : maillage dans le repère du client.
    PaintOpen { session: String, mesh: TriangleMesh },
    /// Applique un trait. `facets` (optionnel) remplace d'abord l'état du
    /// canal — le client renvoie l'état persisté pour rester la source unique.
    PaintStroke {
        session: String,
        channel: PaintChannel,
        #[serde(default)]
        facets: Option<PaintFacets>,
        stroke: PaintStroke,
    },
    /// Ferme une session (libère maillage + sélecteurs).
    PaintClose { session: String },
    /// Profil de couche (adaptatif / lissage / pinceau) — même canal.
    LayerProfile { request: LayerProfileRequest },
    /// Test de vie.
    Ping,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServeRequest {
    pub id: u64,
    #[serde(flatten)]
    pub op: ServeOp,
}

/// Réponse : `ok` + charge utile, ou `error` (message utilisateur).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ServeResponse {
    pub id: u64,
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub triangles: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub facets: Option<PaintFacets>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<Vec<f64>>,
}

impl ServeResponse {
    pub fn ok(id: u64) -> Self {
        Self {
            id,
            ok: true,
            ..Self::default()
        }
    }
    pub fn err(id: u64, message: impl Into<String>) -> Self {
        Self {
            id,
            ok: false,
            error: Some(message.into()),
            ..Self::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stroke() -> PaintStroke {
        PaintStroke {
            points: vec![[0.0, 0.0, 10.0]],
            facets: vec![2],
            camera_pos: [0.0, 0.0, 100.0],
            radius: 2.0,
            cursor: CursorShape::Circle,
            state: 1,
            trafo: glam::DMat4::IDENTITY.to_cols_array(),
            splitting: true,
            highlight_deg: 0.0,
            tool: PaintTool::Brush,
            fill_angle: 30.0,
        }
    }

    #[test]
    fn stroke_validation() {
        assert!(stroke().validate().is_ok());
        let mut s = stroke();
        s.facets.clear();
        assert!(s.validate().is_err());
        let mut s = stroke();
        s.radius = 0.0;
        assert!(s.validate().is_err());
        let mut s = stroke();
        s.points[0][0] = f32::NAN;
        assert!(s.validate().is_err());
    }

    #[test]
    fn serve_protocol_json_shape() {
        let req: ServeRequest = serde_json::from_value(serde_json::json!({
            "id": 7, "op": "paint_stroke", "session": "p/o", "channel": "supports",
            "stroke": serde_json::to_value(stroke()).unwrap()
        }))
        .unwrap();
        assert_eq!(req.id, 7);
        assert!(matches!(req.op, ServeOp::PaintStroke { facets: None, .. }));
        let resp = ServeResponse {
            facets: Some(PaintFacets::from([(3, "4".to_string())])),
            ..ServeResponse::ok(7)
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["facets"]["3"], "4");
        assert!(json.get("error").is_none());
    }
}

/// Requête worker `remap-paint` : « garder la peinture » d'Orca (préférence
/// keep_painting) — peinture des canaux de `source` reportée sur `target`.
/// Clés de canal : `supports`, `seam`, `mmu`, `fuzzy`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemapPaintRequest {
    pub source: crate::api::TriangleMesh,
    pub target: crate::api::TriangleMesh,
    pub channels: BTreeMap<String, PaintFacets>,
}

/// Réponse : mêmes canaux, sur les triangles de `target`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RemapPaintResponse {
    pub channels: BTreeMap<String, PaintFacets>,
}

/// Canaux reconnus (ordre des TriangleSelector de la session de peinture).
pub const PAINT_CHANNEL_KEYS: [&str; 4] = ["supports", "seam", "mmu", "fuzzy"];
