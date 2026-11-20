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

/// Curseur du pinceau (`TriangleSelector::CursorType`, mêmes valeurs) :
/// CIRCLE, SPHERE, POINTER (outil « Triangles » : un seul triangle, au niveau
/// de subdivision touché) et HEIGHT_RANGE (« Plage de hauteur » : tout ce qui
/// est entre la cote touchée et cote + `height`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CursorShape {
    Circle,
    Sphere,
    Pointer,
    HeightRange,
}

impl CursorShape {
    /// Valeur de `TriangleSelector::CursorType`.
    pub fn code(self) -> u8 {
        match self {
            CursorShape::Circle => 0,
            CursorShape::Sphere => 1,
            CursorShape::Pointer => 2,
            CursorShape::HeightRange => 3,
        }
    }
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
    /// Négatif (-1) : pot de peinture sans détection d'arêtes (case « Edge
    /// detection » décochée du gizmo MMU, GLGizmoMmuSegmentation.cpp:585).
    #[serde(default = "default_fill_angle")]
    pub fill_angle: f32,
    /// Hauteur de la plage (mm monde) du curseur `height_range`
    /// (`m_cursor_height`, défaut 0,2 ; curseur 0,1–8 dans Orca).
    #[serde(default = "default_cursor_height")]
    pub height: f32,
    /// Curseur `height_range` sur les AUTRES pièces de l'objet
    /// (`GLGizmoPainterBase::get_projected_height_range`) : cote monde du bas
    /// de la plage, sans point touché (`points`/`facets` vides) ; le serveur
    /// cherche la première facette dans la plage comme Orca.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub z_world: Option<f32>,
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

fn default_cursor_height() -> f32 {
    0.2
}

fn default_true() -> bool {
    true
}

/// Bornes : maillage de session et trait (le worker est partagé).
pub const MAX_SESSION_TRIANGLES: usize = 2_000_000;
pub const MAX_STROKE_POINTS: usize = 4_096;

impl PaintStroke {
    pub fn validate(&self) -> Result<(), String> {
        let range_only = self.cursor == CursorShape::HeightRange
            && self.tool == PaintTool::Brush
            && self.points.is_empty()
            && self.z_world.is_some();
        if (self.points.is_empty() && !range_only) || self.points.len() > MAX_STROKE_POINTS {
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
        if !(-1.0..=90.0).contains(&self.fill_angle) {
            return Err("angle de remplissage hors plage (-1 = sans arêtes, 0–90°)".into());
        }
        if !(self.height.is_finite() && self.height > 0.0 && self.height <= 1000.0) {
            return Err("hauteur de plage hors plage (0–1000 mm)".into());
        }
        if self.z_world.is_some_and(|z| !z.is_finite()) {
            return Err("cote de plage non finie".into());
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
    /// « Comblement des interstices » (outil Gap fill des gizmos supports et
    /// MMU) : les zones d'un seul état d'aire < `area` (mm², repère maillage)
    /// prennent l'état voisin (TriangleSelectorPatch::update_selector_triangles).
    PaintGapFill {
        session: String,
        channel: PaintChannel,
        #[serde(default)]
        facets: Option<PaintFacets>,
        area: f32,
    },
    /// Remappage des filaments MMU (GLGizmoMmuSegmentation::remap_filament_assignments
    /// → TriangleSelector::remap_triangle_state) : `map[i]` = extrudeur
    /// destination (1-based) de l'extrudeur `i + 1`.
    PaintRemap {
        session: String,
        channel: PaintChannel,
        #[serde(default)]
        facets: Option<PaintFacets>,
        map: Vec<u8>,
    },
    /// Aperçu au survol des outils à sélection par propagation (smart fill,
    /// pot, triangle) : triangles que le clic peindrait (`Moving` d'Orca,
    /// `is_selected_by_seed_fill`), sans modifier l'état.
    PaintPreview {
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
    /// Aperçu : sommets (x, y, z ×3 par triangle, repère maillage).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selection: Option<Vec<f32>>,
    /// Aperçu tronqué à `MAX_PREVIEW_TRIANGLES`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub truncated: Option<bool>,
}

/// Aire maximale du comblement (`TriangleSelectorPatch::GapAreaMax`).
pub const GAP_AREA_MAX: f32 = 5.0;
/// Triangles max d'un aperçu de remplissage.
pub const MAX_PREVIEW_TRIANGLES: usize = 300_000;

/// Valide l'aire de comblement (`GapAreaMin`..`GapAreaMax`).
pub fn validate_gap_area(area: f32) -> Result<(), String> {
    if !(0.0..=GAP_AREA_MAX).contains(&area) {
        return Err(format!(
            "aire de comblement hors plage (0–{GAP_AREA_MAX} mm²)"
        ));
    }
    Ok(())
}

/// Valide une table de remappage d'extrudeurs (16 max, destinations 1–16).
pub fn validate_extruder_map(map: &[u8]) -> Result<(), String> {
    if map.is_empty() || map.len() > 16 {
        return Err("table de remappage vide ou trop longue (16 extrudeurs max)".into());
    }
    if map.iter().any(|d| !(1..=16).contains(d)) {
        return Err("extrudeur de destination hors plage (1–16)".into());
    }
    Ok(())
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
            height: 0.2,
            z_world: None,
        }
    }

    #[test]
    fn new_cursors_and_tools_validate() {
        // Plage de hauteur sur une autre pièce : pas de point, cote imposée.
        let mut s = stroke();
        s.cursor = CursorShape::HeightRange;
        s.points.clear();
        s.facets.clear();
        assert!(s.validate().is_err());
        s.z_world = Some(4.0);
        assert!(s.validate().is_ok());
        s.height = 0.0;
        assert!(s.validate().is_err());
        // Pot sans détection d'arêtes (-1) accepté, au-delà refusé.
        let mut s = stroke();
        s.tool = PaintTool::BucketFill;
        s.fill_angle = -1.0;
        assert!(s.validate().is_ok());
        s.fill_angle = -2.0;
        assert!(s.validate().is_err());
        assert_eq!(CursorShape::Pointer.code(), 2);
        assert!(validate_gap_area(5.0).is_ok() && validate_gap_area(5.1).is_err());
        assert!(validate_extruder_map(&[2, 1]).is_ok());
        assert!(validate_extruder_map(&[0]).is_err() && validate_extruder_map(&[]).is_err());
        // Défauts serde : hauteur 0,2, pas de cote.
        let json = serde_json::json!({
            "points": [[0,0,0]], "facets": [0], "camera_pos": [0,0,1], "radius": 1,
            "cursor": "height_range", "state": 1, "trafo": glam::DMat4::IDENTITY.to_cols_array()
        });
        let s: PaintStroke = serde_json::from_value(json).unwrap();
        assert_eq!(
            (s.cursor, s.height, s.z_world),
            (CursorShape::HeightRange, 0.2, None)
        );
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
