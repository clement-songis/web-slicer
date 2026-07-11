//! Scène — miroir de `Model` / `ModelObject` / `ModelVolume` /
//! `ModelInstance` (Model.hpp).

use glam::DMat4;
use serde::{Deserialize, Serialize};

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
}

/// Occurrence d'un objet sur le plateau.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInstance {
    pub matrix: DMat4,
    /// Plateau d'appartenance (multi-plateaux, FR-014).
    pub plate_index: u32,
}

/// Objet imprimable : volumes + instances.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelObject {
    pub name: String,
    pub volumes: Vec<ModelVolume>,
    pub instances: Vec<ModelInstance>,
}

/// Scène complète (miroir `Model`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Model {
    pub objects: Vec<ModelObject>,
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
            }],
            instances: vec![ModelInstance {
                matrix: DMat4::IDENTITY,
                plate_index: 0,
            }],
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
