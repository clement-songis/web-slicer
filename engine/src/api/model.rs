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
        if self.extra.len() > 4096 || self.color.len() > 32 || !(1..=64).contains(&self.extruder) {
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
            }],
            instances: vec![ModelInstance {
                matrix: DMat4::IDENTITY,
                plate_index: 0,
                printable: true,
            }],
            settings: DynamicPrintConfig::new(),
            layer_height_profile: Vec::new(),
            layer_config_ranges: Vec::new(),
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
