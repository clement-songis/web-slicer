//! Requête/résultat de tranchage + volume machine + progression/annulation.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::config::DynamicPrintConfig;
use super::model::Model;

/// Volume d'impression (miroir `BuildVolume`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildVolume {
    /// Contour du plateau [x, y] en mm (`printable_area`).
    pub bed_shape: Vec<[f64; 2]>,
    /// Hauteur maximale d'impression en mm.
    pub max_height: f64,
    /// Zones exclues (`bed_exclude_area`).
    pub excluded: Vec<Vec<[f64; 2]>>,
}

/// Requête de tranchage d'un plateau (unité de la file, FR-014/R9).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SliceRequest {
    pub model: Model,
    /// Configuration résolue (presets aplatis + surcharges projet).
    pub config: DynamicPrintConfig,
    pub plate_index: u32,
    /// Répertoire de travail imposé (garantie d'isolation n°3 du contrat).
    pub work_dir: PathBuf,
    /// Mode calibration d'Orca (`Print::set_calib_params`), absent hors calibration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calib: Option<CalibParams>,
}

/// `CalibMode` d'Orca (libslic3r/calib.hpp), dans le même ordre.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CalibMode {
    None,
    PaLine,
    PaPattern,
    PaTower,
    AutoPaLine,
    FlowRate,
    TempTower,
    VolSpeedTower,
    VfaTower,
    RetractionTower,
    InputShapingFreq,
    InputShapingDamp,
    Cornering,
}

impl CalibMode {
    /// Valeur entière de `Slic3r::CalibMode`.
    pub fn code(self) -> i32 {
        self as i32
    }
}

/// Sous-ensemble de `Calib_Params` posé par les calibrations du menu
/// (`Plater::calib_retraction` / `calib_VFA` / `calib_max_vol_speed`…).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CalibParams {
    pub mode: CalibMode,
    pub start: f64,
    pub end: f64,
    pub step: f64,
    #[serde(default)]
    pub print_numbers: bool,
    /// Input shaping (`Calib_Params::freqStartX/freqEndX/freqStartY/freqEndY`),
    /// en Hz : plage balayée (fréquence) ou fréquence fixe (amortissement).
    #[serde(default)]
    pub freq_start_x: f64,
    #[serde(default)]
    pub freq_end_x: f64,
    #[serde(default)]
    pub freq_start_y: f64,
    #[serde(default)]
    pub freq_end_y: f64,
}

impl CalibParams {
    /// Refuse valeurs non finies et pas nul (les tours en dépendent).
    pub fn validate(&self) -> Result<(), String> {
        let freqs = [
            self.freq_start_x,
            self.freq_end_x,
            self.freq_start_y,
            self.freq_end_y,
        ];
        if ![self.start, self.end, self.step]
            .iter()
            .chain(freqs.iter())
            .all(|v| v.is_finite())
        {
            return Err("paramètres de calibration non finis".into());
        }
        if freqs.iter().any(|f| *f < 0.0) {
            return Err("fréquence d'input shaping négative".into());
        }
        // GCodeWriter::set_input_shaping refuse un amortissement hors [0, 1].
        let damping = match self.mode {
            CalibMode::InputShapingFreq => vec![self.start],
            CalibMode::InputShapingDamp => vec![self.start, self.end],
            _ => vec![],
        };
        if damping.iter().any(|d| !(0.0..=1.0).contains(d)) {
            return Err("amortissement d'input shaping hors de [0, 1]".into());
        }
        if self.mode != CalibMode::None && self.step <= 0.0 {
            return Err("pas de calibration nul ou négatif".into());
        }
        Ok(())
    }
}

/// Statistiques minimales extraites du tranchage (enrichies par gcode::).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SliceStats {
    /// Temps total estimé, en secondes.
    pub estimated_time_s: f64,
    /// Filament par extrudeur, en millimètres.
    pub filament_mm: Vec<f64>,
    /// Filament par extrudeur, en grammes.
    pub filament_g: Vec<f64>,
    pub layer_count: u32,
    pub tool_changes: u32,
}

/// Jeton passé comme `input_filename_base` à `Print::output_filename`,
/// remplacé par le nom du projet au téléchargement.
pub const OUTPUT_FILENAME_BASE_TOKEN: &str = "__ws_filename_base__";

/// Résultat d'un tranchage réussi.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SliceResult {
    pub gcode_path: PathBuf,
    pub stats: SliceStats,
    /// Nom d'export d'Orca (`Print::output_filename`, gabarit `filename_format`
    /// évalué par libslic3r) dont la base (nom du projet) vaut
    /// [`OUTPUT_FILENAME_BASE_TOKEN`] ; `None` si non évaluable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_filename: Option<String>,
    /// Vignettes PNG embarquées (chemins dans work_dir).
    pub thumbnails: Vec<PathBuf>,
}

/// Récepteur de progression : `phase` lisible + ratio [0, 1].
pub type ProgressSink = Box<dyn Fn(&str, f32) + Send + Sync>;

/// Jeton d'annulation coopératif (kill du process moteur, R1/R9).
#[derive(Debug, Clone, Default)]
pub struct CancelToken(Arc<AtomicBool>);

impl CancelToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancel_token_shared_across_clones() {
        let t = CancelToken::new();
        let t2 = t.clone();
        assert!(!t2.is_cancelled());
        t.cancel();
        assert!(t2.is_cancelled(), "l'annulation se propage aux clones");
    }
}
