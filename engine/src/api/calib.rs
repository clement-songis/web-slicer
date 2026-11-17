//! Projets de calibration d'Orca (types partagés ; construction :
//! `adapters::ffi::calib_project`, portage des Plater::calib_*).

use serde::{Deserialize, Serialize};

use super::{CalibParams, DynamicPrintConfig, Model};

/// Calibrations portées (noms stables de l'API).
pub const CALIB_KINDS: [&str; 14] = [
    "pa_pattern",
    "is_freq",
    "is_damp",
    "cornering",
    "temp_tower",
    "pa_line",
    "pa_tower",
    "flow_pass1",
    "flow_pass2",
    "flow_yolo",
    "flow_yolo_fine",
    "max_vol_speed",
    "retraction",
    "vfa",
];

/// Paramètres du dialogue Orca (Calib_Params) + option (motif du test de débit :
/// « archimedean » (défaut d'Orca) | « monotonic »).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CalibRequest {
    pub kind: String,
    #[serde(default)]
    pub start: f64,
    #[serde(default)]
    pub end: f64,
    #[serde(default)]
    pub step: f64,
    #[serde(default)]
    pub option: String,
    /// Motif PA : numéros imprimés, vitesses et accélérations (lot de tests).
    #[serde(default)]
    pub print_numbers: bool,
    #[serde(default)]
    pub speeds: Vec<f64>,
    #[serde(default)]
    pub accelerations: Vec<f64>,
    /// Input shaping / cornering : modèle de tour (0 ringing_tower,
    /// 1 fast_tower_test, 2 SCV-V2 pour le cornering) et type de shaper.
    #[serde(default)]
    pub test_model: i32,
    #[serde(default)]
    pub shaper_type: super::ShaperType,
    /// Fréquences (Hz) : plage (fréquence) ou valeur fixe X/Y (amortissement).
    #[serde(default)]
    pub freq_start_x: f64,
    #[serde(default)]
    pub freq_end_x: f64,
    #[serde(default)]
    pub freq_start_y: f64,
    #[serde(default)]
    pub freq_end_y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibProject {
    pub name: String,
    pub model: Model,
    /// Surcharges de presets (impression / filament / imprimante) posées par Orca.
    pub overrides: DynamicPrintConfig,
    /// Paramètres du mode calibration (absent pour les tests de débit).
    pub calib: Option<CalibParams>,
}
