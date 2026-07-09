//! Modèle de prévisualisation G-code (enrichi par `gcode::` en T021, R6).

use serde::{Deserialize, Serialize};

use super::slice::SliceStats;

/// Type de ligne extrudée (miroir des `;TYPE:` émis par Orca, FR-040).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LineKind {
    OuterWall,
    InnerWall,
    Infill,
    SolidInfill,
    TopSurface,
    BottomSurface,
    Bridge,
    InternalBridge,
    Overhang,
    Support,
    SupportInterface,
    SupportTransition,
    Skirt,
    Brim,
    PrimeTower,
    GapFill,
    Ironing,
    Custom,
    Travel,
    Retraction,
    Unretraction,
    Wipe,
    Seam,
    Unknown,
}

/// Segment extrudé ou déplacé, dans une couche.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GcodeSegment {
    pub kind: LineKind,
    /// Points [x, y, z] en mm (polyline).
    pub points: Vec<[f32; 3]>,
    /// Vitesse programmée (mm/s).
    pub feedrate: f32,
    /// Largeur d'extrusion (mm), 0 pour les déplacements.
    pub width: f32,
    pub extruder: u8,
}

/// Couche de prévisualisation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GcodeLayer {
    pub z: f32,
    pub height: f32,
    pub segments: Vec<GcodeSegment>,
}

/// Prévisualisation complète d'un G-code (FR-040..043).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GcodePreview {
    pub layers: Vec<GcodeLayer>,
    pub stats: SliceStats,
    /// Types de lignes présents (pour la légende).
    pub kinds_present: Vec<LineKind>,
}
