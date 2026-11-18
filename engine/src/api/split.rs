//! Opérations de découpe de maillage — miroir de `ModelObject::split` /
//! `ModelVolume::split` (`TriangleMesh::split`), de la coupe plane `Cut`
//! (`cut_mesh`, CutUtils.cpp) et du booléen `MeshBoolean::mcut::make_boolean`
//! (outil « Mesh Boolean » d'Orca).
//!
//! Ce module est **pur Rust** : types de requête/réponse partagés entre le
//! process `engine-worker` (qui appelle la FFI) et le backend (qui ne la lie
//! jamais), validation des paramètres et encodage STL binaire des pièces.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::TriangleMesh;

/// Plan de coupe : point + normale, dans le repère du maillage fourni. La
/// moitié « upper » est du côté de la normale (comme `Cut` d'Orca, dont le plan
/// est ramené sur z = 0 avant `cut_mesh`).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CutPlane {
    pub point: [f64; 3],
    pub normal: [f64; 3],
}

impl CutPlane {
    /// Plan horizontal z = `z` (cas nominal de l'outil Cut).
    pub fn horizontal(z: f64) -> Self {
        Self {
            point: [0.0, 0.0, z],
            normal: [0.0, 0.0, 1.0],
        }
    }

    /// Refuse coordonnées non finies et normale nulle.
    pub fn validate(&self) -> Result<(), MeshOpError> {
        if self
            .point
            .iter()
            .chain(self.normal.iter())
            .any(|v| !v.is_finite())
        {
            return Err(MeshOpError::InvalidPlane("coordonnées non finies".into()));
        }
        if self.normal_length() < 1e-9 {
            return Err(MeshOpError::InvalidPlane("normale nulle".into()));
        }
        Ok(())
    }

    fn normal_length(&self) -> f64 {
        self.normal.iter().map(|v| v * v).sum::<f64>().sqrt()
    }

    /// Distance signée d'un point au plan (positive côté « upper »).
    pub fn signed_distance(&self, p: [f32; 3]) -> f64 {
        (0..3)
            .map(|i| (f64::from(p[i]) - self.point[i]) * self.normal[i])
            .sum::<f64>()
            / self.normal_length()
    }
}

/// Type de connecteur (`CutConnectorType` : tenon, goujon, clip).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum CutConnectorType {
    #[default]
    Plug,
    Dowel,
    Snap,
}

/// Style (`CutConnectorStyle` : prisme ou tronc de cône).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum CutConnectorStyle {
    #[default]
    Prism,
    Frustum,
}

/// Forme (`CutConnectorShape`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum CutConnectorShape {
    Triangle,
    Square,
    Hexagon,
    #[default]
    Circle,
}

fn default_connector_size() -> f32 {
    2.5
}
fn default_connector_depth() -> f32 {
    3.0
}
fn default_depth_tolerance() -> f32 {
    0.1
}

/// Connecteur du gizmo Cut (`GLGizmoCut3D::add_connector`) ; défauts = ceux
/// du gizmo (`m_connector_size` 2,5, `m_connector_depth_ratio` 3,
/// `m_connector_size_tolerance` 0, `m_connector_depth_ratio_tolerance` 0,1).
/// `pos` : point du plan de coupe, même repère que le plan.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CutConnector {
    pub pos: [f64; 3],
    #[serde(default, rename = "type")]
    #[ts(as = "Option<CutConnectorType>", optional)]
    pub kind: CutConnectorType,
    #[serde(default)]
    #[ts(as = "Option<CutConnectorStyle>", optional)]
    pub style: CutConnectorStyle,
    #[serde(default)]
    #[ts(as = "Option<CutConnectorShape>", optional)]
    pub shape: CutConnectorShape,
    /// Diamètre (mm) ; rayon Orca = size / 2.
    #[serde(default = "default_connector_size")]
    #[ts(as = "Option<f32>", optional)]
    pub size_mm: f32,
    /// Profondeur (mm, `CutConnector::height`).
    #[serde(default = "default_connector_depth")]
    #[ts(as = "Option<f32>", optional)]
    pub depth_mm: f32,
    /// Tolérance de diamètre (mm) ; Orca `radius_tolerance` = moitié.
    #[serde(default)]
    #[ts(as = "Option<f32>", optional)]
    pub size_tolerance_mm: f32,
    #[serde(default = "default_depth_tolerance")]
    #[ts(as = "Option<f32>", optional)]
    pub depth_tolerance_mm: f32,
    /// Rotation autour de l'axe du connecteur (degrés, 0–180 dans Orca).
    #[serde(default)]
    #[ts(as = "Option<f32>", optional)]
    pub rotation_deg: f32,
}

fn default_flaps_angle() -> f32 {
    60.0
}
fn default_groove_count() -> i32 {
    1
}
fn default_groove_gap() -> f32 {
    10.0
}

/// Queue d'aronde (`CutMode::cutTongueAndGroove`, `Cut::Groove`) ; défauts du
/// gizmo : largeur 4 × profondeur, angle des flancs 60°, angle 0°, tolérances
/// 0,1 mm, 1 queue, écart 10 mm.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CutGroove {
    pub depth_mm: f32,
    #[serde(default)]
    #[ts(optional)]
    pub width_mm: Option<f32>,
    #[serde(default = "default_flaps_angle")]
    #[ts(as = "Option<f32>", optional)]
    pub flaps_angle_deg: f32,
    #[serde(default)]
    #[ts(as = "Option<f32>", optional)]
    pub angle_deg: f32,
    #[serde(default = "default_depth_tolerance")]
    #[ts(as = "Option<f32>", optional)]
    pub depth_tolerance_mm: f32,
    #[serde(default = "default_depth_tolerance")]
    #[ts(as = "Option<f32>", optional)]
    pub width_tolerance_mm: f32,
    #[serde(default = "default_groove_count")]
    #[ts(as = "Option<i32>", optional)]
    pub count: i32,
    #[serde(default = "default_groove_gap")]
    #[ts(as = "Option<f32>", optional)]
    pub gap_mm: f32,
}

impl CutGroove {
    pub fn width(&self) -> f32 {
        self.width_mm.unwrap_or(4.0 * self.depth_mm)
    }
}

/// Options de la découpe complète d'Orca (toutes facultatives : absentes =
/// coupe plane simple, comportement historique).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CutOptions {
    /// Rotation du plan (degrés, X puis Y puis Z : `Geometry::rotation_transform`) ;
    /// prioritaire sur la normale (normale = R·Z). Oriente aussi les connecteurs
    /// et la queue d'aronde (axe X du plan).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rotation_deg: Option<[f64; 3]>,
    /// Matrice d'instance 4×4 colonne-major (rotation/échelle de l'objet,
    /// translation ignorée) : le plan est alors dans le repère monde centré sur
    /// l'objet, comme le gizmo d'Orca, et les pièces sortent sans rotation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance_matrix: Option<[f64; 16]>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub connectors: Vec<CutConnector>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub groove: Option<CutGroove>,
    #[serde(default)]
    pub keep_as_parts: bool,
    #[serde(default)]
    pub place_on_cut_upper: bool,
    #[serde(default)]
    pub place_on_cut_lower: bool,
    #[serde(default)]
    pub flip_upper: bool,
    #[serde(default)]
    pub flip_lower: bool,
}

impl CutOptions {
    /// Coupe historique (`cut_mesh` seul) : aucune option Orca demandée.
    pub fn is_plain(&self) -> bool {
        *self == CutOptions::default()
    }

    /// Normale R·Z de la rotation (degrés, convention Orca Rz·Ry·Rx).
    pub fn rotation_normal(rotation_deg: [f64; 3]) -> [f64; 3] {
        let [x, y, z] = rotation_deg.map(f64::to_radians);
        let r = glam::DQuat::from_rotation_z(z)
            * glam::DQuat::from_rotation_y(y)
            * glam::DQuat::from_rotation_x(x);
        (r * glam::DVec3::Z).to_array()
    }

    pub fn validate(&self) -> Result<(), MeshOpError> {
        let bad = |m: &str| Err(MeshOpError::InvalidCut(m.into()));
        let finite = |v: f32| v.is_finite();
        if self
            .rotation_deg
            .is_some_and(|r| r.iter().any(|v| !v.is_finite()))
        {
            return bad("rotation non finie");
        }
        if let Some(m) = self.instance_matrix {
            if m.iter().any(|v| !v.is_finite())
                || glam::DMat4::from_cols_array(&m).determinant().abs() < 1e-12
            {
                return bad("matrice d'instance invalide");
            }
        }
        if self.groove.is_some() && !self.connectors.is_empty() {
            return bad("connecteurs et queue d'aronde sont exclusifs (Orca)");
        }
        if self.connectors.len() > 200 {
            return bad("trop de connecteurs (200 max)");
        }
        for c in &self.connectors {
            if c.pos.iter().any(|v| !v.is_finite())
                || ![
                    c.size_mm,
                    c.depth_mm,
                    c.size_tolerance_mm,
                    c.depth_tolerance_mm,
                    c.rotation_deg,
                ]
                .into_iter()
                .all(finite)
            {
                return bad("connecteur : valeur non finie");
            }
            if !(c.size_mm > 0.0 && c.size_mm <= 200.0 && c.depth_mm > 0.0 && c.depth_mm <= 200.0) {
                return bad("connecteur : taille ou profondeur hors plage (0–200 mm)");
            }
            if !(0.0..=10.0).contains(&c.size_tolerance_mm)
                || !(0.0..=10.0).contains(&c.depth_tolerance_mm)
            {
                return bad("connecteur : tolérance hors plage (0–10 mm)");
            }
        }
        if let Some(g) = &self.groove {
            let values = [
                g.depth_mm,
                g.width(),
                g.flaps_angle_deg,
                g.angle_deg,
                g.depth_tolerance_mm,
                g.width_tolerance_mm,
                g.gap_mm,
            ];
            if !values.into_iter().all(finite) {
                return bad("queue d'aronde : valeur non finie");
            }
            if !(g.depth_mm > 0.0 && g.depth_mm <= 200.0 && g.width() > 0.0 && g.width() <= 500.0) {
                return bad("queue d'aronde : profondeur ou largeur hors plage");
            }
            if !(g.flaps_angle_deg > 0.0 && g.flaps_angle_deg < 90.0) || g.angle_deg.abs() >= 90.0 {
                return bad("queue d'aronde : angles hors plage");
            }
            if !(0.0..=10.0).contains(&g.depth_tolerance_mm)
                || !(0.0..=10.0).contains(&g.width_tolerance_mm)
            {
                return bad("queue d'aronde : tolérance hors plage (0–10 mm)");
            }
            if !(1..=20).contains(&g.count) || !(0.0..=500.0).contains(&g.gap_mm) {
                return bad("queue d'aronde : nombre (1–20) ou écart hors plage");
            }
        }
        Ok(())
    }
}

/// Opération booléenne : chaînes `boolean_opts` de mcut utilisées par
/// `GLGizmoMeshBoolean` (UNION, A_NOT_B, INTERSECTION).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BooleanOp {
    Union,
    Difference,
    Intersection,
}

impl BooleanOp {
    /// Code transmis au bridge C++ (0 = UNION, 1 = A_NOT_B, 2 = INTERSECTION).
    pub fn code(self) -> u8 {
        match self {
            BooleanOp::Union => 0,
            BooleanOp::Difference => 1,
            BooleanOp::Intersection => 2,
        }
    }
}

/// Primitive de l'ajout de forme d'Orca (`ObjectList::load_shape_object` →
/// `create_mesh`, GUI_ObjectList.cpp) : chaque forme est un `its_make_*` de
/// libslic3r (TriangleMesh.cpp) avec les paramètres exacts de la GUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrimitiveShape {
    Cube,
    Cylinder,
    Sphere,
    Cone,
    Disc,
    Torus,
}

impl PrimitiveShape {
    /// Code transmis au bridge C++ (ordre ci-dessus).
    pub fn code(self) -> u8 {
        match self {
            PrimitiveShape::Cube => 0,
            PrimitiveShape::Cylinder => 1,
            PrimitiveShape::Sphere => 2,
            PrimitiveShape::Cone => 3,
            PrimitiveShape::Disc => 4,
            PrimitiveShape::Torus => 5,
        }
    }

    /// Libellé d'objet d'Orca (`_(type_name)`, préfixe « Shape » retiré).
    pub fn label(self) -> &'static str {
        match self {
            PrimitiveShape::Cube => "Cube",
            PrimitiveShape::Cylinder => "Cylinder",
            PrimitiveShape::Sphere => "Sphere",
            PrimitiveShape::Cone => "Cone",
            PrimitiveShape::Disc => "Disc",
            PrimitiveShape::Torus => "Torus",
        }
    }
}

/// Côté de référence d'Orca : `GLCanvas3D::get_size_proportional_to_max_bed_size(0.1)`
/// = 0,1 × la plus grande dimension XY de la boîte du plateau imprimable.
pub fn primitive_side_for_bed(bed_size_x: f64, bed_size_y: f64) -> f64 {
    0.1 * bed_size_x.max(bed_size_y)
}

/// Opération demandée au worker (`engine-worker mesh-op <request.json>`).
// Requête ponctuelle sérialisée en JSON pour le worker : la taille de la
// variante `Cut` (options de découpe complète) est sans effet mesurable.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MeshOp {
    /// `TriangleMesh::split` : une pièce par composante connexe (≥ 3 facettes,
    /// orientation corrigée si volume négatif). Sert à « split to objects » et
    /// « split to parts » (seul le regroupement côté scène diffère).
    Split,
    /// Coupe plane (`cut_mesh` avec bouchage des sections) ; avec `options`
    /// non vides : découpe complète d'Orca (`Slic3r::Cut`).
    Cut {
        plane: CutPlane,
        keep_upper: bool,
        keep_lower: bool,
        #[serde(default, skip_serializing_if = "CutOptions::is_plain")]
        options: CutOptions,
    },
    /// Booléen mcut entre `meshes[0]` (source A) et `meshes[1]` (outil B),
    /// exprimés dans un repère commun.
    Boolean { op: BooleanOp },
    /// Subdivision lissante de Loop (`TriangleMeshDeal::smooth_triangle_mesh`,
    /// « Subdivision mesh (Lost color) » d'Orca) : ×4 triangles, une pièce
    /// `result`. Maillage ouvert refusé comme `Plater::can_smooth_mesh`.
    Smooth,
    /// Texte en relief (outil Emboss d'Orca) : aucun maillage d'entrée ;
    /// `font_path` = police TTF/OTF lisible par le worker.
    EmbossText {
        font_path: PathBuf,
        text: String,
        size_mm: f32,
        depth_mm: f32,
        /// Graisse (mm, `FontProp::boldness`) ; 0 = police telle quelle.
        #[serde(default)]
        boldness_mm: f32,
    },
    /// Forme SVG en relief (outil SVG d'Orca) : `svg` = contenu du fichier,
    /// mis à l'échelle à `width_mm` de large, extrudé de `depth_mm`.
    EmbossSvg {
        svg: String,
        width_mm: f32,
        depth_mm: f32,
    },
    /// Primitive d'Orca (`create_mesh`) : aucun maillage d'entrée ;
    /// `side_mm` = côté de référence (`primitive_side_for_bed`).
    Primitive { shape: PrimitiveShape, side_mm: f64 },
}

/// Requête worker : maillages d'entrée + répertoire où écrire les WSMh.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshOpRequest {
    pub op: MeshOp,
    pub meshes: Vec<TriangleMesh>,
    pub output_dir: PathBuf,
}

/// Pièce produite : WSMh écrit dans `output_dir` + comptes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshOpPart {
    /// `part_<n>` (split), `upper` / `lower` (cut), `result` (booléen).
    pub label: String,
    pub mesh_path: PathBuf,
    pub triangles: u32,
    pub vertices: u32,
    /// Découpe complète : volume additionnel (connecteur, 2e pièce…) de
    /// l'objet `label` ; absent pour le corps principal de chaque objet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volume: Option<CutVolumeMeta>,
}

/// Volume additionnel d'un objet issu de la découpe complète.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CutVolumeMeta {
    /// Nom Orca du volume (`Connector-1`, `object_B`…).
    pub name: String,
    /// `part` | `negative` | `modifier` | `blocker` | `enforcer`.
    pub role: String,
    /// `cut_info.is_connector`.
    pub connector: bool,
}

/// Pièce produite par le moteur avant écriture (corps ou volume additionnel).
#[derive(Debug, Clone)]
pub struct MeshOpPiece {
    pub label: String,
    pub mesh: TriangleMesh,
    pub volume: Option<CutVolumeMeta>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshOpResponse {
    pub parts: Vec<MeshOpPart>,
}

/// Paramètre refusé (erreur utilisateur, jamais une faute moteur).
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum MeshOpError {
    #[error("plan de coupe invalide : {0}")]
    InvalidPlane(String),
    #[error("aucune moitié conservée (keep_upper et keep_lower faux)")]
    NothingKept,
    #[error("l'opération attend {expected} maillage(s), {got} fourni(s)")]
    MeshCount { expected: usize, got: usize },
    #[error("maillage d'entrée vide")]
    EmptyMesh,
    #[error("texte en relief invalide : {0}")]
    InvalidText(String),
    #[error("primitive invalide : {0}")]
    InvalidPrimitive(String),
    #[error("découpe invalide : {0}")]
    InvalidCut(String),
}

impl MeshOp {
    /// Validation pure (avant tout lancement de worker / appel C++).
    pub fn validate(&self, meshes: &[TriangleMesh]) -> Result<(), MeshOpError> {
        if let MeshOp::Primitive { side_mm, .. } = self {
            if !meshes.is_empty() {
                return Err(MeshOpError::MeshCount {
                    expected: 0,
                    got: meshes.len(),
                });
            }
            // Plateaux réels : 50 mm à 2 m → côté 5 à 200 mm ; marge large.
            if !(side_mm.is_finite() && *side_mm >= 1.0 && *side_mm <= 1000.0) {
                return Err(MeshOpError::InvalidPrimitive(
                    "côté hors plage (1–1000 mm)".into(),
                ));
            }
            return Ok(());
        }
        if let MeshOp::EmbossSvg {
            svg,
            width_mm,
            depth_mm,
        } = self
        {
            if !meshes.is_empty() {
                return Err(MeshOpError::MeshCount {
                    expected: 0,
                    got: meshes.len(),
                });
            }
            if !svg.contains("<svg") || svg.len() > 2_000_000 {
                return Err(MeshOpError::InvalidText(
                    "SVG absent ou trop volumineux (2 Mo max)".into(),
                ));
            }
            if !(*width_mm > 0.5 && *width_mm <= 500.0 && *depth_mm > 0.05 && *depth_mm <= 100.0) {
                return Err(MeshOpError::InvalidText(
                    "largeur (0,5–500 mm) ou profondeur (0,05–100 mm) hors plage".into(),
                ));
            }
            return Ok(());
        }
        if let MeshOp::EmbossText {
            text,
            size_mm,
            depth_mm,
            boldness_mm,
            ..
        } = self
        {
            if !(*boldness_mm >= -2.0 && *boldness_mm <= 5.0) {
                return Err(MeshOpError::InvalidText(
                    "graisse hors plage (−2 à 5 mm)".into(),
                ));
            }
            if !meshes.is_empty() {
                return Err(MeshOpError::MeshCount {
                    expected: 0,
                    got: meshes.len(),
                });
            }
            if text.trim().is_empty() {
                return Err(MeshOpError::InvalidText("texte vide".into()));
            }
            if text.chars().count() > 200 {
                return Err(MeshOpError::InvalidText(
                    "texte trop long (200 caractères max)".into(),
                ));
            }
            if !(*size_mm > 0.5 && *size_mm <= 500.0 && *depth_mm > 0.05 && *depth_mm <= 100.0) {
                return Err(MeshOpError::InvalidText(
                    "taille (0,5–500 mm) ou profondeur (0,05–100 mm) hors plage".into(),
                ));
            }
            return Ok(());
        }
        let expected = match self {
            MeshOp::Boolean { .. } => 2,
            _ => 1,
        };
        if meshes.len() != expected {
            return Err(MeshOpError::MeshCount {
                expected,
                got: meshes.len(),
            });
        }
        if meshes.iter().any(TriangleMesh::is_empty) {
            return Err(MeshOpError::EmptyMesh);
        }
        if let MeshOp::Cut {
            plane,
            keep_upper,
            keep_lower,
            options,
        } = self
        {
            options.validate()?;
            if options.rotation_deg.is_none() {
                plane.validate()?;
            } else if plane.point.iter().any(|v| !v.is_finite()) {
                return Err(MeshOpError::InvalidPlane("coordonnées non finies".into()));
            }
            // Orca force les deux moitiés quand il y a des connecteurs.
            if !keep_upper && !keep_lower && options.connectors.is_empty() {
                return Err(MeshOpError::NothingKept);
            }
        }
        Ok(())
    }
}

/// Volume signé (somme des tétraèdres origine-facette), en mm³.
pub fn signed_volume(mesh: &TriangleMesh) -> f64 {
    let v = |i: u32| mesh.vertices[i as usize].map(f64::from);
    mesh.indices
        .iter()
        .map(|t| {
            let (a, b, c) = (v(t[0]), v(t[1]), v(t[2]));
            (a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0])
                + a[2] * (b[0] * c[1] - b[1] * c[0]))
                / 6.0
        })
        .sum()
}

/// Centre de la boîte englobante (repère du maillage), `[0; 3]` si vide.
pub fn bbox_center(mesh: &TriangleMesh) -> [f64; 3] {
    mesh.bounding_box().map_or([0.0; 3], |(lo, hi)| {
        std::array::from_fn(|i| (f64::from(lo[i]) + f64::from(hi[i])) / 2.0)
    })
}

/// Encode un STL **binaire** avec les vraies normales de facette (admesh lit
/// un en-tête commençant par « solid » comme de l'ASCII : on l'évite).
pub fn encode_binary_stl(mesh: &TriangleMesh) -> Vec<u8> {
    let mut out = Vec::with_capacity(84 + mesh.indices.len() * 50);
    let mut header = [b' '; 80];
    let tag = b"web-slicer binary STL";
    header[..tag.len()].copy_from_slice(tag);
    out.extend_from_slice(&header);
    out.extend_from_slice(&(mesh.indices.len() as u32).to_le_bytes());
    for tri in &mesh.indices {
        let [a, b, c] = tri.map(|i| mesh.vertices[i as usize]);
        let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let w = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let mut n = [
            u[1] * w[2] - u[2] * w[1],
            u[2] * w[0] - u[0] * w[2],
            u[0] * w[1] - u[1] * w[0],
        ];
        let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
        if len > f32::EPSILON {
            n.iter_mut().for_each(|x| *x /= len);
        }
        for value in n.iter().chain(a.iter()).chain(b.iter()).chain(c.iter()) {
            out.extend_from_slice(&value.to_le_bytes());
        }
        out.extend_from_slice(&0u16.to_le_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cube(min: [f32; 3], size: f32) -> TriangleMesh {
        let [x, y, z] = min;
        let s = size;
        let vertices = vec![
            [x, y, z],
            [x + s, y, z],
            [x + s, y + s, z],
            [x, y + s, z],
            [x, y, z + s],
            [x + s, y, z + s],
            [x + s, y + s, z + s],
            [x, y + s, z + s],
        ];
        let indices = vec![
            [0, 2, 1],
            [0, 3, 2],
            [4, 5, 6],
            [4, 6, 7],
            [0, 1, 5],
            [0, 5, 4],
            [1, 2, 6],
            [1, 6, 5],
            [2, 3, 7],
            [2, 7, 6],
            [3, 0, 4],
            [3, 4, 7],
        ];
        TriangleMesh { vertices, indices }
    }

    #[test]
    fn cube_volume_is_positive_and_exact() {
        assert!((signed_volume(&cube([0.0; 3], 10.0)) - 1000.0).abs() < 1e-6);
        assert_eq!(bbox_center(&cube([0.0; 3], 10.0)), [5.0; 3]);
    }

    #[test]
    fn binary_stl_has_real_normals_and_no_solid_header() {
        let stl = encode_binary_stl(&cube([0.0; 3], 10.0));
        assert_eq!(stl.len(), 84 + 12 * 50);
        assert!(!stl.starts_with(b"solid"));
        assert_eq!(u32::from_le_bytes(stl[80..84].try_into().unwrap()), 12);
        // Première facette (fond, z = 0) : normale -Z.
        let nz = f32::from_le_bytes(stl[92..96].try_into().unwrap());
        assert!((nz + 1.0).abs() < 1e-6);
    }

    #[test]
    fn emboss_text_validation() {
        let op = |text: &str, size: f32, depth: f32| MeshOp::EmbossText {
            font_path: "f.ttf".into(),
            text: text.into(),
            size_mm: size,
            depth_mm: depth,
            boldness_mm: 0.0,
        };
        assert!(op("Bonjour", 10.0, 2.0).validate(&[]).is_ok());
        let svg = |s: &str| MeshOp::EmbossSvg {
            svg: s.into(),
            width_mm: 20.0,
            depth_mm: 2.0,
        };
        assert!(svg("<svg><rect/></svg>").validate(&[]).is_ok());
        assert!(matches!(
            svg("pas un svg").validate(&[]),
            Err(MeshOpError::InvalidText(_))
        ));
        assert!(matches!(
            op("  ", 10.0, 2.0).validate(&[]),
            Err(MeshOpError::InvalidText(_))
        ));
        assert!(matches!(
            op("a", 0.1, 2.0).validate(&[]),
            Err(MeshOpError::InvalidText(_))
        ));
        assert!(matches!(
            op("a", 10.0, 2.0).validate(&[TriangleMesh::default()]),
            Err(MeshOpError::MeshCount {
                expected: 0,
                got: 1
            })
        ));
    }

    #[test]
    fn validation_rejects_bad_requests() {
        let source = cube([0.0; 3], 10.0);
        let m = std::slice::from_ref(&source);
        let cut = |plane, keep_upper, keep_lower| MeshOp::Cut {
            plane,
            keep_upper,
            keep_lower,
            options: CutOptions::default(),
        };
        assert!(cut(CutPlane::horizontal(5.0), true, true)
            .validate(m)
            .is_ok());
        assert_eq!(
            cut(CutPlane::horizontal(5.0), false, false).validate(m),
            Err(MeshOpError::NothingKept)
        );
        let zero = CutPlane {
            point: [0.0; 3],
            normal: [0.0; 3],
        };
        assert!(matches!(
            cut(zero, true, true).validate(m),
            Err(MeshOpError::InvalidPlane(_))
        ));
        assert_eq!(
            MeshOp::Boolean {
                op: BooleanOp::Union
            }
            .validate(m),
            Err(MeshOpError::MeshCount {
                expected: 2,
                got: 1
            })
        );
        assert_eq!(
            MeshOp::Split.validate(&[TriangleMesh::default()]),
            Err(MeshOpError::EmptyMesh)
        );
    }

    #[test]
    fn primitive_validation_and_orca_side() {
        // Plateau 256 × 256 (X1C) : côté 25,6 mm comme la GUI d'Orca.
        assert!((primitive_side_for_bed(256.0, 256.0) - 25.6).abs() < 1e-12);
        assert!((primitive_side_for_bed(220.0, 250.0) - 25.0).abs() < 1e-12);
        let op = |side| MeshOp::Primitive {
            shape: PrimitiveShape::Sphere,
            side_mm: side,
        };
        assert!(op(25.6).validate(&[]).is_ok());
        assert!(matches!(
            op(0.0).validate(&[]),
            Err(MeshOpError::InvalidPrimitive(_))
        ));
        assert!(matches!(
            op(f64::NAN).validate(&[]),
            Err(MeshOpError::InvalidPrimitive(_))
        ));
        assert!(matches!(
            op(20.0).validate(&[TriangleMesh::default()]),
            Err(MeshOpError::MeshCount {
                expected: 0,
                got: 1
            })
        ));
        let parsed: MeshOp = serde_json::from_value(serde_json::json!({
            "kind": "primitive", "shape": "torus", "side_mm": 25.6
        }))
        .unwrap();
        assert_eq!(
            parsed,
            MeshOp::Primitive {
                shape: PrimitiveShape::Torus,
                side_mm: 25.6
            }
        );
    }

    #[test]
    fn plane_distance_follows_normal() {
        let p = CutPlane {
            point: [0.0, 0.0, 5.0],
            normal: [0.0, 0.0, 2.0],
        };
        assert!((p.signed_distance([1.0, 1.0, 7.0]) - 2.0).abs() < 1e-9);
    }

    #[test]
    fn cut_options_parse_with_orca_defaults_and_validate() {
        let op: MeshOp = serde_json::from_value(serde_json::json!({
            "kind": "cut",
            "plane": { "point": [0, 0, 1], "normal": [0, 0, 1] },
            "keep_upper": true, "keep_lower": true,
            "options": { "connectors": [{ "pos": [1, 2, 1], "type": "dowel", "shape": "hexagon" }] }
        }))
        .unwrap();
        let MeshOp::Cut { options, .. } = &op else {
            panic!("cut attendu")
        };
        let c = options.connectors[0];
        assert_eq!(c.kind, CutConnectorType::Dowel);
        assert_eq!(c.style, CutConnectorStyle::Prism);
        assert_eq!(c.shape, CutConnectorShape::Hexagon);
        assert_eq!((c.size_mm, c.depth_mm), (2.5, 3.0));
        assert_eq!((c.size_tolerance_mm, c.depth_tolerance_mm), (0.0, 0.1));
        let source = cube([0.0; 3], 10.0);
        assert!(op.validate(std::slice::from_ref(&source)).is_ok());
        // Sans option : forme sérialisée historique (pas de champ options).
        let plain = MeshOp::Cut {
            plane: CutPlane::horizontal(1.0),
            keep_upper: true,
            keep_lower: true,
            options: CutOptions::default(),
        };
        assert!(serde_json::to_value(&plain)
            .unwrap()
            .get("options")
            .is_none());

        let groove: CutGroove =
            serde_json::from_value(serde_json::json!({ "depth_mm": 2 })).unwrap();
        assert_eq!(groove.width(), 8.0);
        assert_eq!(
            (groove.flaps_angle_deg, groove.count, groove.gap_mm),
            (60.0, 1, 10.0)
        );
        let both = CutOptions {
            connectors: vec![c],
            groove: Some(groove),
            ..CutOptions::default()
        };
        assert!(matches!(both.validate(), Err(MeshOpError::InvalidCut(_))));
        let mut big = c;
        big.size_mm = 0.0;
        let bad = CutOptions {
            connectors: vec![big],
            ..CutOptions::default()
        };
        assert!(matches!(bad.validate(), Err(MeshOpError::InvalidCut(_))));
        let mut steep = groove;
        steep.flaps_angle_deg = 90.0;
        let bad = CutOptions {
            groove: Some(steep),
            ..CutOptions::default()
        };
        assert!(matches!(bad.validate(), Err(MeshOpError::InvalidCut(_))));
    }

    #[test]
    fn rotation_normal_follows_orca_convention() {
        let n = CutOptions::rotation_normal([0.0, 0.0, 0.0]);
        assert!((n[2] - 1.0).abs() < 1e-12);
        // Rotation de 90° autour de X : Z → −Y.
        let n = CutOptions::rotation_normal([90.0, 0.0, 0.0]);
        assert!((n[1] + 1.0).abs() < 1e-12 && n[2].abs() < 1e-12);
        // Rz·Ry·Rx : 90° autour de Y puis 90° autour de Z → Z → X → Y.
        let n = CutOptions::rotation_normal([0.0, 90.0, 90.0]);
        assert!((n[1] - 1.0).abs() < 1e-12, "{n:?}");
    }

    #[test]
    fn request_json_shape_is_stable() {
        let op: MeshOp = serde_json::from_value(serde_json::json!({
            "kind": "cut",
            "plane": { "point": [0, 0, 1], "normal": [0, 0, 1] },
            "keep_upper": true, "keep_lower": false
        }))
        .unwrap();
        assert!(matches!(
            op,
            MeshOp::Cut {
                keep_upper: true,
                ..
            }
        ));
        let op: MeshOp =
            serde_json::from_value(serde_json::json!({ "kind": "boolean", "op": "difference" }))
                .unwrap();
        assert_eq!(
            op,
            MeshOp::Boolean {
                op: BooleanOp::Difference
            }
        );
    }
}
