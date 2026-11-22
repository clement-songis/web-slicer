//! Scène — miroir de `Model` / `ModelObject` / `ModelVolume` /
//! `ModelInstance` (Model.hpp).

use glam::DMat4;
use serde::{Deserialize, Serialize};

use super::config::DynamicPrintConfig;
use super::mesh::TriangleMesh;

/// Format d'un fichier modèle importable (FR-010).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelFormat {
    Stl,
    Obj,
    ThreeMf,
    Step,
    Amf,
    Svg,
    Drc,
}

impl ModelFormat {
    /// Détection par extension (insensible à la casse). Jeu cross-plateforme
    /// d'OrcaSlicer (`GUI_App.cpp` FT_MODEL non-Apple ∪ drag-drop `Plater.cpp`) :
    /// `.oltp` est un alias STL ; `.amf`/`.xml` (et `.zip.amf`) → AMF.
    pub fn from_path(path: &std::path::Path) -> Option<Self> {
        match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
            "stl" | "oltp" => Some(Self::Stl),
            "obj" => Some(Self::Obj),
            "3mf" => Some(Self::ThreeMf),
            "step" | "stp" => Some(Self::Step),
            "amf" | "xml" => Some(Self::Amf),
            "svg" => Some(Self::Svg),
            "drc" => Some(Self::Drc),
            _ => None,
        }
    }
}

/// Rôle d'un volume dans son objet (miroir `ModelVolumeType`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VolumeRole {
    ModelPart,
    NegativeVolume,
    ParameterModifier,
    SupportBlocker,
    SupportEnforcer,
}

/// Pièce d'un objet : maillage + transformation locale + rôle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelVolume {
    pub name: String,
    pub mesh: TriangleMesh,
    /// Transformation locale (colonne-major, mm).
    pub matrix: DMat4,
    pub role: VolumeRole,
    /// Extrudeur/filament attribué (None = héritage objet, FR-015).
    pub extruder: Option<u16>,
    /// Peintures au triangle (format 3MF Orca), transmises au slicing.
    #[serde(default, skip_serializing_if = "VolumePaint::is_empty")]
    pub paint: VolumePaint,
    /// Surcharges `ModelVolume.config` (réglages d'un modificateur, FR-015).
    #[serde(default)]
    pub settings: DynamicPrintConfig,
    /// Relief ré-éditable (`text_configuration` / `emboss_shape` d'Orca),
    /// JSON web-slicer `{ type: "text" | "svg", … }` ; `None` = volume ordinaire.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emboss: Option<serde_json::Value>,
    /// Connecteur de coupe (`ModelVolume::cut_info` d'Orca, connecteur déjà
    /// découpé : `is_connector && is_processed`) ; écrit dans
    /// `Metadata/cut_information.xml` du 3MF. `None` = volume ordinaire.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cut_info: Option<VolumeCutInfo>,
    /// Statistiques d'erreurs du maillage lu par libslic3r
    /// (`TriangleMesh::stats()` : arêtes ouvertes restantes + erreurs réparées à
    /// la lecture, ou `mesh_stat` du 3MF) ; `None` = non renseigné.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mesh_stats: Option<MeshErrorStats>,
}

/// Erreurs d'un maillage (`TriangleMeshStats` d'Orca, partie erreurs) :
/// `open_edges` = erreurs restantes (non-manifold), `repaired` = erreurs
/// corrigées automatiquement (`RepairedMeshErrors`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeshErrorStats {
    pub open_edges: u32,
    #[serde(default)]
    pub repaired: crate::api::RepairedMeshErrors,
}

impl MeshErrorStats {
    /// `TriangleMeshStats::manifold`.
    pub fn manifold(&self) -> bool {
        self.open_edges == 0
    }

    /// `TriangleMeshStats::repaired`.
    pub fn repaired(&self) -> bool {
        self.repaired.repaired_anything()
    }

    /// `ModelObject::get_repaired_errors_count` : somme des erreurs réparées.
    pub fn repaired_errors_count(&self) -> u32 {
        let r = &self.repaired;
        r.degenerate_facets
            + r.edges_fixed
            + r.facets_removed
            + r.facets_reversed
            + r.backwards_edges
    }

    /// `RepairedMeshErrors::merge` + cumul des arêtes ouvertes
    /// (`ModelObject::get_object_stl_stats`).
    pub fn merge(&mut self, other: &MeshErrorStats) {
        self.open_edges += other.open_edges;
        let (a, b) = (&mut self.repaired, &other.repaired);
        a.edges_fixed += b.edges_fixed;
        a.degenerate_facets += b.degenerate_facets;
        a.facets_removed += b.facets_removed;
        a.facets_reversed += b.facets_reversed;
        a.backwards_edges += b.backwards_edges;
    }
}

/// `ModelVolume::CutInfo` d'un connecteur découpé (Orca 2.4.1, Model.hpp).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct VolumeCutInfo {
    /// `CutConnectorType` : 0 = Plug, 1 = Dowel, 2 = Snap.
    pub connector_type: u8,
    /// Tolérance de rayon `[0, 1]` (`radius_tolerance`).
    pub radius_tolerance: f32,
    /// Tolérance de hauteur `[0, 1]` (`height_tolerance`).
    pub height_tolerance: f32,
}

/// `CutObjectBase` d'un objet issu d'une coupe (`ModelObject::cut_id`) : les
/// pièces d'un même objet coupé partagent `id` ; `check_sum` = nombre de
/// pièces de l'objet initial, `connectors_cnt` = connecteurs créés. Un
/// `ModelObject` est « coupé » (`is_cut()`) si `id` est valide (≠ 0).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CutObjectId {
    pub id: u64,
    pub check_sum: u64,
    pub connectors_cnt: u64,
}

impl CutObjectId {
    /// `CutObjectBase::is_equal` : même id, même check_sum, mêmes connecteurs.
    pub fn is_equal(&self, other: &CutObjectId) -> bool {
        self == other
    }
}

/// Canal de peinture d'un volume (miroir des `FacetsAnnotation` de `ModelVolume`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaintChannel {
    /// `supported_facets` (3MF `slic3rpe:custom_supports`).
    Supports,
    /// `seam_facets` (`slic3rpe:custom_seam`).
    Seam,
    /// `mmu_segmentation_facets` (`slic3rpe:mmu_segmentation`).
    Mmu,
    /// `fuzzy_skin_facets` (`slic3rpe:fuzzy_skin`).
    Fuzzy,
}

/// Facettes peintes d'un volume : (canal, indice de triangle, état sérialisé Orca
/// en hexadécimal, cf. `FacetsAnnotation::set_triangle_from_string`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct VolumePaint {
    pub facets: Vec<(PaintChannel, u32, String)>,
}

impl VolumePaint {
    pub fn is_empty(&self) -> bool {
        self.facets.is_empty()
    }
}

/// Occurrence d'un objet sur le plateau.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInstance {
    pub matrix: DMat4,
    /// Plateau d'appartenance (multi-plateaux, FR-014).
    pub plate_index: u32,
    /// `ModelInstance::printable` d'Orca : false exclut l'occurrence du slicing.
    #[serde(default = "default_printable")]
    pub printable: bool,
}

const fn default_printable() -> bool {
    true
}

/// Objet imprimable : volumes + instances.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelObject {
    pub name: String,
    pub volumes: Vec<ModelVolume>,
    pub instances: Vec<ModelInstance>,
    /// Surcharges `ModelObject.config`, appliquées avant le slicing.
    pub settings: DynamicPrintConfig,
    /// Profil variable : vecteur plat `<z, hauteur>` d'Orca.
    pub layer_height_profile: Vec<f64>,
    /// Plages de hauteur à réglages propres (`ModelObject::layer_config_ranges`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub layer_config_ranges: Vec<LayerConfigRange>,
    /// Informations de coupe (`ModelObject::cut_id`), `None` = objet non coupé.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cut_id: Option<CutObjectId>,
    /// Oreilles de bordure peintes (`ModelObject::brim_points`, outil E d'Orca),
    /// utilisées par `Brim.cpp` quand `brim_type = painted` et écrites dans
    /// `Metadata/brim_ear_points.txt` du 3MF.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub brim_points: Vec<BrimPoint>,
}

/// `Slic3r::BrimPoint` : position dans le repère de l'objet (avant la
/// transformation d'instance) et rayon de la tête. `head_front_radius <= 0`
/// = rayon par défaut du gizmo d'Orca (`GLGizmoBrimEars::get_brim_default_radius`
/// : largeur de ligne de la 1re couche × 16 de diamètre), résolu par le moteur
/// avec la config du tranchage / de l'export.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BrimPoint {
    pub pos: [f32; 3],
    pub head_front_radius: f32,
}

impl BrimPoint {
    /// Point posé comme le gizmo d'Orca (`GLGizmoBrimEars::gizmo_event`,
    /// LeftDown) : point monde ramené sous le plateau (`z = -0.0001`) puis
    /// exprimé dans le repère de l'objet par l'inverse de la transformation
    /// d'instance (`trsf.inverse() * world_pos`, cast en float).
    pub fn from_world(instance: DMat4, x: f64, y: f64, head_front_radius: f32) -> Self {
        let local = instance
            .inverse()
            .transform_point3(glam::DVec3::new(x, y, -0.0001));
        BrimPoint {
            pos: [local.x as f32, local.y as f32, local.z as f32],
            head_front_radius,
        }
    }

    /// Position monde (x, y) du point pour une instance (`pt.transform(trsf)`).
    pub fn to_world(&self, instance: DMat4) -> [f64; 2] {
        let world = instance.transform_point3(glam::DVec3::new(
            self.pos[0] as f64,
            self.pos[1] as f64,
            self.pos[2] as f64,
        ));
        [world.x, world.y]
    }
}

/// Plage `[min_z, max_z]` (mm, objet posé) et ses surcharges de réglages,
/// miroir d'une entrée de `ModelObject::layer_config_ranges` d'Orca.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerConfigRange {
    pub min_z: f64,
    pub max_z: f64,
    #[serde(default)]
    pub settings: DynamicPrintConfig,
}

/// Scène complète (miroir `Model`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Model {
    pub objects: Vec<ModelObject>,
    /// G-code personnalisés par couche et par plateau (`Model::plates_custom_gcodes`
    /// d'Orca : changement de couleur, pause, G-code perso — curseur de couches).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub custom_gcodes: Vec<PlateCustomGcodes>,
}

/// `CustomGCode::Type` d'Orca (même ordre).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CustomGcodeKind {
    ColorChange,
    PausePrint,
    ToolChange,
    Template,
    Custom,
}

/// `CustomGCode::Item`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CustomGcodeItem {
    /// Cote (mm) de la couche après laquelle le code est inséré.
    pub print_z: f64,
    pub kind: CustomGcodeKind,
    /// Extrudeur (1-based) pour changement de couleur / d'outil.
    #[serde(default = "one")]
    pub extruder: i32,
    #[serde(default)]
    pub color: String,
    /// Texte de la pause ou G-code personnalisé.
    #[serde(default)]
    pub extra: String,
}

fn one() -> i32 {
    1
}

/// `CustomGCode::Mode` (choisi par Orca selon le nombre d'extrudeurs).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CustomGcodeMode {
    #[default]
    SingleExtruder,
    MultiAsSingle,
    MultiExtruder,
}

/// `CustomGCode::Info` d'un plateau.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlateCustomGcodes {
    pub plate_index: u32,
    #[serde(default)]
    pub mode: CustomGcodeMode,
    pub items: Vec<CustomGcodeItem>,
}

impl CustomGcodeItem {
    pub fn validate(&self) -> Result<(), String> {
        if !(self.print_z.is_finite() && self.print_z > 0.0 && self.print_z < 10_000.0) {
            return Err("cote de G-code personnalisé invalide".into());
        }
        // extra : motifs PA générés (Plater::_calib_pa_pattern_gen_gcode) — plusieurs
        // dizaines de Ko par couche ; extrudeur 0 = extrudeur courant (Orca).
        if self.extra.len() > 1 << 20 || self.color.len() > 32 || !(0..=64).contains(&self.extruder)
        {
            return Err("G-code personnalisé hors bornes".into());
        }
        Ok(())
    }
}

impl Model {
    pub fn is_empty(&self) -> bool {
        self.objects.is_empty()
    }

    pub fn add_object(&mut self, name: impl Into<String>, mesh: TriangleMesh) -> &mut ModelObject {
        self.objects.push(ModelObject {
            name: name.into(),
            volumes: vec![ModelVolume {
                name: String::new(),
                mesh,
                matrix: DMat4::IDENTITY,
                role: VolumeRole::ModelPart,
                extruder: None,
                paint: VolumePaint::default(),
                settings: DynamicPrintConfig::new(),
                emboss: None,
                cut_info: None,
                mesh_stats: None,
            }],
            instances: vec![ModelInstance {
                matrix: DMat4::IDENTITY,
                plate_index: 0,
                printable: true,
            }],
            settings: DynamicPrintConfig::new(),
            layer_height_profile: Vec::new(),
            layer_config_ranges: Vec::new(),
            cut_id: None,
            brim_points: Vec::new(),
        });
        self.objects.last_mut().expect("objet tout juste ajouté")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn format_depuis_extension() {
        assert_eq!(
            ModelFormat::from_path(Path::new("a/benchy.STL")),
            Some(ModelFormat::Stl)
        );
        assert_eq!(
            ModelFormat::from_path(Path::new("piece.stp")),
            Some(ModelFormat::Step)
        );
        assert_eq!(
            ModelFormat::from_path(Path::new("projet.3mf")),
            Some(ModelFormat::ThreeMf)
        );
        // Jeu OrcaSlicer étendu (T091).
        assert_eq!(
            ModelFormat::from_path(Path::new("piece.oltp")),
            Some(ModelFormat::Stl)
        );
        assert_eq!(
            ModelFormat::from_path(Path::new("piece.amf")),
            Some(ModelFormat::Amf)
        );
        assert_eq!(
            ModelFormat::from_path(Path::new("logo.svg")),
            Some(ModelFormat::Svg)
        );
        assert_eq!(
            ModelFormat::from_path(Path::new("mesh.drc")),
            Some(ModelFormat::Drc)
        );
        // `.zip` conteneur = hors jeu v1 (exclusions.md).
        assert_eq!(ModelFormat::from_path(Path::new("archive.zip")), None);
        assert_eq!(ModelFormat::from_path(Path::new("sans_extension")), None);
    }

    #[test]
    fn add_object_construit_volume_et_instance() {
        let mut model = Model::default();
        assert!(model.is_empty());
        model.add_object("cube", TriangleMesh::default());
        assert_eq!(model.objects.len(), 1);
        let obj = &model.objects[0];
        assert_eq!(obj.volumes.len(), 1);
        assert_eq!(obj.volumes[0].role, VolumeRole::ModelPart);
        assert_eq!(obj.instances[0].plate_index, 0);
    }
}
