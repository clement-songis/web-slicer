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
    /// Préférence Orca « Remove mixed temperature restriction »
    /// (`enable_high_low_temp_mixed_printing`) : filaments haute/basse
    /// température acceptés (avertissement) au lieu d'être refusés.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub allow_mixed_temperatures: bool,
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

/// `InputShaperType` d'Orca (PrintConfig.cpp, `input_shaping_type`), valeur
/// transmise à `GCodeWriter::set_input_shaping` (`Calib_Params::shaper_type`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ShaperType {
    #[default]
    Default,
    #[serde(rename = "MZV")]
    Mzv,
    #[serde(rename = "ZV")]
    Zv,
    #[serde(rename = "ZVD")]
    Zvd,
    #[serde(rename = "ZVDD")]
    Zvdd,
    #[serde(rename = "ZVDDD")]
    Zvddd,
    #[serde(rename = "EI")]
    Ei,
    #[serde(rename = "EI2")]
    Ei2,
    #[serde(rename = "2HUMP_EI")]
    TwoHumpEi,
    #[serde(rename = "EI3")]
    Ei3,
    #[serde(rename = "3HUMP_EI")]
    ThreeHumpEi,
    #[serde(rename = "DAA")]
    Daa,
    Disable,
}

impl ShaperType {
    /// Chaîne d'Orca (`enum_values` de `input_shaping_type`).
    pub fn as_orca(self) -> &'static str {
        [
            "Default", "MZV", "ZV", "ZVD", "ZVDD", "ZVDDD", "EI", "EI2", "2HUMP_EI", "EI3",
            "3HUMP_EI", "DAA", "Disable",
        ][self as usize]
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
    /// `Calib_Params::test_model` : modèle de tour (input shaping / cornering :
    /// 0 = ringing_tower, 1 = fast_tower_test, 2 = SCV-V2 (cornering)).
    #[serde(default)]
    pub test_model: i32,
    /// `Calib_Params::shaper_type` (input shaping).
    #[serde(default)]
    pub shaper_type: ShaperType,
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
        // Input shaping et cornering n'utilisent pas de pas (dialogues d'Orca :
        // plage de fréquences / d'amortissement / de jerk interpolée par couche).
        let uses_step = !matches!(
            self.mode,
            CalibMode::None
                | CalibMode::InputShapingFreq
                | CalibMode::InputShapingDamp
                | CalibMode::Cornering
        );
        if uses_step && self.step <= 0.0 {
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
    /// Lot 46 (audit 44 CONFL-a) : conflit de trajets détecté par
    /// `ConflictChecker` à la fin de `Print::process` (`Print::get_conflict_result`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conflict: Option<GcodeConflict>,
    /// Lot 47c (audit 45 H2D-a) : trajets hors de la zone atteignable d'une
    /// buse (`GCodeProcessorResult::gcode_check_result`, bits 0 / 1). Comme la
    /// GUI d'Orca : tranchage réussi et aperçu disponible, impression / export
    /// bloqués (le CLI, lui, refuse : −102).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gcode_check: Option<GcodeCheck>,
}

/// Résultat du contrôle des trajets multi-buses (`GCodeCheckResult` d'Orca),
/// présent seulement en cas d'erreur : notification SLICING_LIMIT_ERROR de la
/// GUI (« Filament %s is placed in the %s, but the generated G-code path
/// exceeds the printable range of the %s. », GLCanvas3D.cpp:10118-10185).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GcodeCheck {
    /// `error_code` limité aux bits 0 (zone) / 1 (hauteur).
    pub error_code: i32,
    /// Texte de la GUI (msgid anglais, une ligne par buse).
    pub message: String,
    #[serde(default)]
    pub issues: Vec<GcodeCheckIssue>,
}

/// Une buse en cause.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GcodeCheckIssue {
    /// `range` (bit 0) ou `height` (bit 1).
    pub kind: String,
    /// Extrudeur 1-based.
    pub extruder: u32,
    /// Nom affiché par Orca (« left nozzle », « right nozzle », « Tool 2 »).
    pub nozzle: String,
    /// Filaments 1-based en cause.
    pub filaments: Vec<u32>,
}

/// Conflit de trajets G-code entre deux objets (`ConflictResult` d'Orca) :
/// avertissement grave de la GUI (« Conflicts of G-code paths… »,
/// GLCanvas3D.cpp:10078), refus de la CLI (code −101).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GcodeConflict {
    pub object1: String,
    pub object2: String,
    /// Hauteur Z du conflit (mm).
    pub height: f64,
    /// Couche affichée par Orca (`Layers::get_layer_id_at(height)` du visualiseur).
    pub layer: i64,
}

impl GcodeConflict {
    /// Message d'Orca 2.4.1 (GLCanvas3D.cpp:10089, msgid anglais).
    pub fn message(&self) -> String {
        format!(
            "Conflicts of G-code paths have been found at layer {}, Z = {:.2}mm. Please separate the conflicted objects farther ({} <-> {}).",
            self.layer, self.height, self.object1, self.object2
        )
    }
}

/// Origine d'une entrée de validation : `StringExceptionType` d'Orca
/// (PrintBase.hpp) ou contrôle hors `Print::validate`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidateIssueKind {
    /// `STRING_EXCEPT_NOT_DEFINED`.
    General,
    /// `STRING_EXCEPT_FILAMENT_NOT_MATCH_BED_TYPE`.
    FilamentNotMatchBedType,
    /// `STRING_EXCEPT_FILAMENTS_DIFFERENT_TEMP`.
    FilamentsDifferentTemp,
    /// `STRING_EXCEPT_OBJECT_COLLISION_IN_SEQ_PRINT` (« by object » : dégagement extrudeur).
    ObjectCollisionInSeqPrint,
    /// `STRING_EXCEPT_OBJECT_COLLISION_IN_LAYER_PRINT` (zones exclues, tour de purge).
    ObjectCollisionInLayerPrint,
    /// `STRING_EXCEPT_LAYER_HEIGHT_EXCEEDS_LIMIT`.
    LayerHeightExceedsLimit,
    /// `DynamicPrintConfig::validate` (bornes / incompatibilités, CLI Orca).
    Config,
    /// Cohérence de la scène vérifiée par le pont (filaments manquants,
    /// matrice de purge, plateau vide).
    Consistency,
}

impl ValidateIssueKind {
    /// Depuis le code du pont (`RawValidateIssue::kind`).
    pub fn from_code(code: i32) -> Self {
        match code {
            1 => Self::FilamentNotMatchBedType,
            2 => Self::FilamentsDifferentTemp,
            3 => Self::ObjectCollisionInSeqPrint,
            4 => Self::ObjectCollisionInLayerPrint,
            5 => Self::LayerHeightExceedsLimit,
            -1 => Self::Config,
            -2 => Self::Consistency,
            _ => Self::General,
        }
    }
}

/// Erreur ou avertissement de `Print::validate` (miroir `StringObjectException`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValidateIssue {
    /// Message d'Orca (anglais) ou du pont (français).
    pub message: String,
    pub kind: ValidateIssueKind,
    /// Avertissement (le tranchage reste possible) ou erreur bloquante.
    pub is_warning: bool,
    /// Option en cause (`opt_key`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opt_key: Option<String>,
    /// Index de l'objet en cause dans `Model::objects` de la requête.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub object_index: Option<usize>,
    /// Paramètres du message (`StringObjectException::params`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub params: Vec<String>,
}

/// Résultat de la validation avant tranchage d'un plateau.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ValidateResult {
    pub issues: Vec<ValidateIssue>,
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
