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
}
