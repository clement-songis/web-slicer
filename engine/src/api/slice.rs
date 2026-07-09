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

/// Résultat d'un tranchage réussi.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SliceResult {
    pub gcode_path: PathBuf,
    pub stats: SliceStats,
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
    fn cancel_token_partage_entre_clones() {
        let t = CancelToken::new();
        let t2 = t.clone();
        assert!(!t2.is_cancelled());
        t.cancel();
        assert!(t2.is_cancelled(), "l'annulation se propage aux clones");
    }
}
