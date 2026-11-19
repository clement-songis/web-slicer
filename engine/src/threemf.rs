//! Types de plateau d'un projet 3MF OrcaSlicer.
//!
//! Le projet 3MF est écrit par libslic3r (`store_bbs_3mf`, SplitModel |
//! ShareMesh comme « Save project » d'Orca) via
//! `adapters::ffi::write_project_3mf_with_plates` et relu par
//! `read_project_3mf` ; l'ancien writer/lecteur pur Rust (T072) a été retiré.

use crate::api::DynamicPrintConfig;

/// Réglages qu'OrcaSlicer persiste **par plateau** dans `<plate>` (clé de
/// configuration Orca, `PartPlate::config`). Toute autre clé n'a pas de
/// représentation par plateau dans le format et n'est donc pas écrite.
pub const PLATE_SETTING_KEYS: &[&str] = &[
    "curr_bed_type",
    "print_sequence",
    "first_layer_print_sequence",
    "first_layer_sequence_choice",
    "other_layers_print_sequence",
    "other_layers_sequence_choice",
    "other_layers_print_sequence_nums",
    "spiral_mode",
    // Affectation filament → buse (imprimantes multi-buses, H2D) : PartPlate
    // persiste filament_map_mode et filament_map (bbs_3mf.cpp:7968, attributs
    // « filament_map_mode » / « filament_maps ») ; le CLI les applique au
    // tranchage du plateau (OrcaSlicer.cpp ~5980).
    "filament_map_mode",
    "filament_map",
];

/// Plateau d'un projet exporté : nom et réglages par plateau
/// (sous-ensemble [`PLATE_SETTING_KEYS`]). Les instances y sont rattachées par
/// `ModelInstance::plate_index` (index dans la liste des plateaux).
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ProjectPlate {
    pub name: String,
    pub settings: DynamicPrintConfig,
    /// Vignette PNG du plateau (Orca : `PartPlate::thumbnail_data`, rendue à
    /// 512 × 512) ; vide = aucune. Transmise au worker avec le plateau.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub thumbnail_png: Vec<u8>,
}

/// Métadonnées d'un projet 3MF (Orca : `Model::model_info`, `design_info`,
/// `profile_info`, lues par `_BBS_3MF_Importer::_handle_end_metadata` et
/// écrites par `_BBS_3MF_Exporter::_add_model_file_to_archive` : Title,
/// Origin, Designer, DesignerCover, Description, Copyright, License + toutes
/// les autres entrées `<metadata>` dans `metadata_items`).
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ProjectMetadata {
    /// Titre (« Title », ModelInfo::model_name).
    #[serde(default)]
    pub model_name: String,
    /// « Origin » (type de publication).
    #[serde(default)]
    pub origin: String,
    /// Couverture (« DesignerCover ») : nom d'un fichier de « Model Pictures ».
    #[serde(default)]
    pub cover_file: String,
    #[serde(default)]
    pub description: String,
    /// « Copyright » (JSON `[{"author": …}]` dans les projets MakerWorld).
    #[serde(default)]
    pub copyright: String,
    /// Licence (Auxiliary.cpp : « », CC0, BY, BY-SA, BY-ND, BY-NC, BY-NC-SA, BY-NC-ND).
    #[serde(default)]
    pub license: String,
    /// Auteur (« Designer », ModelDesignInfo::Designer).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub designer: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub designer_user_id: Option<String>,
    /// Toutes les entrées `<metadata>` lues (réécrites à l'export comme Orca).
    #[serde(default)]
    pub metadata_items: std::collections::BTreeMap<String, String>,
    /// ModelProfileInfo (lecture seule) : title, cover, description, user_id, user_name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<std::collections::BTreeMap<String, String>>,
}
