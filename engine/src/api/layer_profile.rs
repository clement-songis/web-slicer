//! Profil de hauteur de couche variable calculé par libslic3r (Slicing.cpp) :
//! `layer_height_profile_adaptive` (bouton « Adaptive » de l'outil
//! « Variable layer height » d'Orca, GLCanvas3D.cpp:660) et
//! `smooth_height_profile` (bouton « Smooth », GLCanvas3D.cpp:670).
//!
//! Pur Rust : types partagés entre le backend et `engine-worker layer-profile`.
//! Le `SlicingParameters` est construit côté C++ par
//! `PrintObject::slicing_parameters(full_config, object, 0, 1)` comme la GUI.

use serde::{Deserialize, Serialize};

use super::{DynamicPrintConfig, Model};

/// Opération demandée (paramètres et défauts de la GUI d'Orca :
/// `m_adaptive_quality{0.5f}`, `HeightProfileSmoothingParams{radius 5, keep_min false}`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LayerProfileOp {
    /// `layer_height_profile_adaptive(slicing_params, object, quality)` ;
    /// `quality` ∈ [0, 1] (0 = couches fines, 1 = épaisses).
    Adaptive { quality: f32 },
    /// `smooth_height_profile(profile, slicing_params, {radius, keep_min})`.
    Smooth {
        profile: Vec<f64>,
        radius: u32,
        keep_min: bool,
    },
    /// Pinceau : `adjust_layer_height_profile(profile, z, delta, band_width,
    /// action)` (Slicing.cpp:518). Défauts GUI : `strength` 0,005, bande 2 mm
    /// (bornée 1,5–10 mm à la molette). Profil vide = initialisé comme Orca.
    Adjust {
        #[serde(default)]
        profile: Vec<f64>,
        z: f64,
        delta: f64,
        band_width: f64,
        action: LayerEditAction,
    },
}

/// `LayerHeightEditActionType` d'Orca.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LayerEditAction {
    Increase,
    Decrease,
    Reduce,
    Smooth,
}

impl LayerEditAction {
    pub fn code(self) -> u8 {
        match self {
            LayerEditAction::Increase => 0,
            LayerEditAction::Decrease => 1,
            LayerEditAction::Reduce => 2,
            LayerEditAction::Smooth => 3,
        }
    }
}

impl LayerProfileOp {
    pub fn validate(&self) -> Result<(), String> {
        match self {
            LayerProfileOp::Adaptive { quality } => {
                if !(quality.is_finite() && (0.0..=1.0).contains(quality)) {
                    return Err("qualité adaptative hors plage (0–1)".into());
                }
            }
            LayerProfileOp::Smooth {
                profile, radius, ..
            } => {
                // Bornes de l'ImGui d'Orca : rayon 1–10.
                if !(1..=10).contains(radius) {
                    return Err("rayon de lissage hors plage (1–10)".into());
                }
                if profile.len() < 4 || !profile.len().is_multiple_of(2) || profile.len() > 200_000
                {
                    return Err("profil invalide (paires <z, hauteur> attendues)".into());
                }
                if profile.iter().any(|v| !v.is_finite()) {
                    return Err("profil non fini".into());
                }
            }
            LayerProfileOp::Adjust {
                profile,
                z,
                delta,
                band_width,
                ..
            } => {
                if !profile.len().is_multiple_of(2) || profile.len() > 200_000 {
                    return Err("profil invalide (paires <z, hauteur> attendues)".into());
                }
                if !(z.is_finite() && delta.is_finite() && delta.abs() <= 1.0) {
                    return Err("cote ou intensité de pinceau invalide".into());
                }
                if !(band_width.is_finite() && (0.1..=50.0).contains(band_width)) {
                    return Err("largeur de bande hors plage (0,1–50 mm)".into());
                }
                if profile.iter().any(|v| !v.is_finite()) {
                    return Err("profil non fini".into());
                }
            }
        }
        Ok(())
    }
}

/// Requête worker : un objet (volumes + transformation d'instance appliquée
/// à la boîte, comme `ModelObject` de la GUI) + configuration résolue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerProfileRequest {
    pub model: Model,
    pub config: DynamicPrintConfig,
    pub op: LayerProfileOp,
}

/// Profil plat Orca `[z0, h0, z1, h1, …]`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayerProfileResponse {
    pub profile: Vec<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_follows_orca_ui_bounds() {
        assert!(LayerProfileOp::Adaptive { quality: 0.5 }.validate().is_ok());
        assert!(LayerProfileOp::Adaptive { quality: 1.5 }
            .validate()
            .is_err());
        let smooth = |radius, profile: Vec<f64>| LayerProfileOp::Smooth {
            profile,
            radius,
            keep_min: false,
        };
        assert!(smooth(5, vec![0.0, 0.2, 10.0, 0.1]).validate().is_ok());
        assert!(smooth(0, vec![0.0, 0.2, 10.0, 0.1]).validate().is_err());
        assert!(smooth(5, vec![0.0, 0.2, 10.0]).validate().is_err());
        let op: LayerProfileOp =
            serde_json::from_value(serde_json::json!({"kind": "adaptive", "quality": 0.3}))
                .unwrap();
        assert_eq!(op, LayerProfileOp::Adaptive { quality: 0.3 });
    }
}
