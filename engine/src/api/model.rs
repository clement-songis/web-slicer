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

/// Peinture au triangle d'un volume (annotations 3MF Orca). Par canal, la liste
/// `(index de triangle, chaîne encodée)` façon `FacetsAnnotation` — miroir de
/// `ModelVolume::{supported,seam,mmu_segmentation,fuzzy_skin}_facets` (Model.hpp).
/// L'encodage par facette est celui de `TriangleSelector::serialize`
/// (`FacetsAnnotation::get_triangle_as_string`) : enforcer = "4", blocker = "8".
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PaintedFacets {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub supports: Vec<(u32, String)>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub seam: Vec<(u32, String)>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mmu: Vec<(u32, String)>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fuzzy: Vec<(u32, String)>,
}

impl PaintedFacets {
    pub fn is_empty(&self) -> bool {
        self.supports.is_empty()
            && self.seam.is_empty()
            && self.mmu.is_empty()
            && self.fuzzy.is_empty()
    }

    /// Trie chaque canal par index de triangle croissant et déduplique — contrat
    /// de `FacetsAnnotation::set_triangle_from_string` (ids strictement croissants).
    pub fn normalized(mut self) -> Self {
        for channel in [
            &mut self.supports,
            &mut self.seam,
            &mut self.mmu,
            &mut self.fuzzy,
        ] {
            channel.sort_by_key(|(triangle, _)| *triangle);
            channel.dedup_by_key(|(triangle, _)| *triangle);
        }
        self
    }
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
    /// Peinture au triangle (supports/couture/MMU/fuzzy, T056+) — vide par défaut.
    #[serde(default, skip_serializing_if = "PaintedFacets::is_empty")]
    pub painted: PaintedFacets,
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
    /// Profil de hauteur de couche variable (vecteur plat `[z0, h0, z1, h1, …]`,
    /// mm, trié par z) — miroir de `ModelObject::layer_height_profile`. Vide =
    /// hauteur uniforme du config (T058/T147).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub layer_height_profile: Vec<f64>,
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
                painted: PaintedFacets::default(),
            }],
            instances: vec![ModelInstance {
                matrix: DMat4::IDENTITY,
                plate_index: 0,
            }],
            layer_height_profile: Vec::new(),
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
    fn painted_facets_normalise_trie_et_dedoublonne() {
        let painted = PaintedFacets {
            supports: vec![(5, "4".into()), (1, "4".into()), (5, "8".into())],
            ..Default::default()
        }
        .normalized();
        // Trié par index croissant, dernier doublon (5) éliminé après tri.
        assert_eq!(painted.supports, vec![(1, "4".into()), (5, "4".into())]);
        assert!(painted.seam.is_empty());
    }

    #[test]
    fn painted_facets_serde_omet_les_canaux_vides() {
        // Un volume sans peinture ne doit pas alourdir la requête (skip serialize).
        let json = serde_json::to_string(&ModelVolume {
            name: String::new(),
            mesh: TriangleMesh::default(),
            matrix: DMat4::IDENTITY,
            role: VolumeRole::ModelPart,
            extruder: None,
            painted: PaintedFacets::default(),
        })
        .unwrap();
        assert!(!json.contains("painted"), "champ vide sérialisé : {json}");

        // Et un canal peint fait l'aller-retour.
        let painted = PaintedFacets {
            supports: vec![(0, "4".into())],
            ..Default::default()
        };
        let back: PaintedFacets =
            serde_json::from_str(&serde_json::to_string(&painted).unwrap()).unwrap();
        assert_eq!(back, painted);
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
