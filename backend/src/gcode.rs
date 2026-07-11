//! Post-traitement d'un G-code produit par le moteur (T066, FR-043).
//!
//! Analyse **pure** des commentaires normalisés qu'OrcaSlicer écrit en tête de
//! fichier (miroir de `libslic3r/GCode.cpp` et `GCode/Thumbnails.hpp`) :
//!   - statistiques (`; estimated printing time (normal mode) = …`,
//!     `; total filament used [g] = …`, `; total layers count = …`,
//!     `; filament used [mm|cm3|g] = a, b`, `; filament cost = …`),
//!   - vignettes embarquées (blocs `; thumbnail begin WxH LEN` … `; thumbnail end`).
//!
//! L'orchestration (`store_gcode`) écrit le fichier via le magasin (`FileStore`)
//! puis crée la ligne `gcodes` avec `stats`/`thumbnails` figés. Le moteur FFI
//! (runner, T066+) appelle `store_gcode` pour matérialiser le résultat d'un job.

use serde::Serialize;

use crate::adapters::files::FileStore;
use crate::domain::repo::{GcodeRepo, NewGcode};
use crate::domain::{Gcode, GcodeId, JobId, UserId};

/// Statistiques d'impression extraites du G-code (FR-043).
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct GcodeStats {
    /// Temps d'impression estimé (mode « normal »), en secondes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_time_seconds: Option<u64>,
    /// Temps estimé sous sa forme texte d'origine (`1h 2m 3s`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_time_text: Option<String>,
    /// Masse totale de filament (g).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_filament_weight_g: Option<f64>,
    /// Coût total du filament (devise du profil).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_filament_cost: Option<f64>,
    /// Nombre de changements d'outil (multi-matériau).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_toolchanges: Option<u64>,
    /// Nombre total de couches.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layer_count: Option<u64>,
    /// Longueur de filament par extrudeur (mm).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub filament_used_mm: Vec<f64>,
    /// Volume de filament par extrudeur (cm³).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub filament_used_cm3: Vec<f64>,
    /// Masse de filament par extrudeur (g).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub filament_used_g: Vec<f64>,
    /// Coût de filament par extrudeur.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub filament_cost: Vec<f64>,
}

/// Vignette embarquée dans le G-code (PNG encodé base64).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GcodeThumbnail {
    pub width: u32,
    pub height: u32,
    /// Données PNG encodées base64 (concaténation des lignes du bloc).
    pub data: String,
}

/// Analyse les commentaires de statistiques en tête d'un G-code Orca.
pub fn parse_stats(gcode: &str) -> GcodeStats {
    let mut stats = GcodeStats::default();
    for line in gcode.lines() {
        // On ne lit que les commentaires (`;`), en tolérant l'indentation.
        let Some(comment) = line.trim_start().strip_prefix(';') else {
            continue;
        };
        let comment = comment.trim();
        if let Some(v) = comment.strip_prefix("estimated printing time (normal mode) = ") {
            stats.estimated_time_text = Some(v.trim().to_string());
            stats.estimated_time_seconds = parse_dhms(v.trim());
        } else if let Some(v) = comment.strip_prefix("total filament used [g] = ") {
            stats.total_filament_weight_g = v.trim().parse().ok();
        } else if let Some(v) = comment.strip_prefix("total filament cost = ") {
            stats.total_filament_cost = v.trim().parse().ok();
        } else if let Some(v) = comment.strip_prefix("total filament change = ") {
            stats.total_toolchanges = v.trim().parse().ok();
        } else if let Some(v) = comment.strip_prefix("total layers count = ") {
            stats.layer_count = v.trim().parse().ok();
        } else if let Some(v) = comment.strip_prefix("filament used [mm] = ") {
            stats.filament_used_mm = parse_number_list(v);
        } else if let Some(v) = comment.strip_prefix("filament used [cm3] = ") {
            stats.filament_used_cm3 = parse_number_list(v);
        } else if let Some(v) = comment.strip_prefix("filament used [g] = ") {
            stats.filament_used_g = parse_number_list(v);
        } else if let Some(v) = comment.strip_prefix("filament cost = ") {
            stats.filament_cost = parse_number_list(v);
        }
    }
    stats
}

/// Extrait les vignettes embarquées (blocs `; thumbnail begin … end`).
pub fn extract_thumbnails(gcode: &str) -> Vec<GcodeThumbnail> {
    let mut out = Vec::new();
    let mut current: Option<(u32, u32, String)> = None;
    for line in gcode.lines() {
        let Some(comment) = line.trim_start().strip_prefix(';') else {
            continue;
        };
        let comment = comment.trim();
        let tokens: Vec<&str> = comment.split_whitespace().collect();
        // Début : `<tag> begin WxH LEN` (tag = thumbnail, thumbnail_QOI, …).
        if tokens.len() >= 3 && tokens[1] == "begin" && tokens[0].starts_with("thumbnail") {
            if let Some((w, h)) = tokens[2].split_once('x') {
                if let (Ok(w), Ok(h)) = (w.parse(), h.parse()) {
                    current = Some((w, h, String::new()));
                }
            }
            continue;
        }
        // Fin : `<tag> end`.
        if tokens.len() >= 2 && tokens[1] == "end" && tokens[0].starts_with("thumbnail") {
            if let Some((width, height, data)) = current.take() {
                if !data.is_empty() {
                    out.push(GcodeThumbnail {
                        width,
                        height,
                        data,
                    });
                }
            }
            continue;
        }
        // Corps du bloc : lignes base64 (un seul jeton, sans espace).
        if let Some((_, _, data)) = current.as_mut() {
            if tokens.len() == 1 {
                data.push_str(tokens[0]);
            }
        }
    }
    out
}

/// Convertit une durée Orca (`1d 2h 3m 4s`, `2h 30m`, `45s`) en secondes.
fn parse_dhms(text: &str) -> Option<u64> {
    let mut total: u64 = 0;
    let mut seen = false;
    for token in text.split_whitespace() {
        let (num, unit) = token.split_at(token.len().saturating_sub(1));
        let factor = match unit {
            "d" => 86_400,
            "h" => 3_600,
            "m" => 60,
            "s" => 1,
            _ => return None,
        };
        let n: u64 = num.parse().ok()?;
        total += n * factor;
        seen = true;
    }
    seen.then_some(total)
}

/// Analyse une liste `a, b, c` de nombres (filament par extrudeur).
fn parse_number_list(raw: &str) -> Vec<f64> {
    raw.split(',')
        .filter_map(|s| s.trim().parse::<f64>().ok())
        .collect()
}

/// Matérialise le résultat d'un job : écrit le G-code via le magasin puis crée la
/// ligne `gcodes` avec `stats`/`thumbnails` figés. Le fichier est nommé par une
/// clé de stockage propre — `file_path` fait foi (comme pour les modèles).
pub async fn store_gcode(
    files: &FileStore,
    gcodes: &dyn GcodeRepo,
    owner: UserId,
    job_id: JobId,
    gcode_text: &str,
) -> anyhow::Result<Gcode> {
    let stats = parse_stats(gcode_text);
    let thumbnails = extract_thumbnails(gcode_text);

    let storage_key = GcodeId::new();
    let path = files
        .write_gcode(owner, storage_key, gcode_text.as_bytes())
        .await?;

    let gcode = gcodes
        .create(
            owner,
            NewGcode {
                job_id,
                file_path: path.to_string_lossy().into_owned(),
                // Buffers de préviz binaires par couche : T067.
                preview_path: String::new(),
                stats: serde_json::to_value(&stats)?,
                thumbnails: serde_json::to_value(&thumbnails)?,
            },
        )
        .await?;
    Ok(gcode)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// En-tête G-code Orca réaliste (statistiques + une vignette 2x2).
    const SAMPLE: &str = "\
; generated by OrcaSlicer
;
; thumbnail begin 2x2 16
; iVBORw0KGgoAAAAN
; SUhEUgAAAAI=
; thumbnail end
;
; filament used [mm] = 1234.56, 0
; filament used [cm3] = 2.97, 0
; filament used [g] = 3.68, 0
; filament cost = 0.11, 0
G1 X0 Y0
G1 X10 Y10 E1.2
; total filament used [g] = 3.68
; total filament cost = 0.11
; total filament change = 2
; total layers count = 42
; estimated printing time (normal mode) = 1h 2m 3s
";

    #[test]
    fn parses_totals_and_time() {
        let s = parse_stats(SAMPLE);
        assert_eq!(s.estimated_time_text.as_deref(), Some("1h 2m 3s"));
        assert_eq!(s.estimated_time_seconds, Some(3723));
        assert_eq!(s.total_filament_weight_g, Some(3.68));
        assert_eq!(s.total_filament_cost, Some(0.11));
        assert_eq!(s.total_toolchanges, Some(2));
        assert_eq!(s.layer_count, Some(42));
    }

    #[test]
    fn parses_per_extruder_lists() {
        let s = parse_stats(SAMPLE);
        assert_eq!(s.filament_used_mm, vec![1234.56, 0.0]);
        assert_eq!(s.filament_used_cm3, vec![2.97, 0.0]);
        assert_eq!(s.filament_used_g, vec![3.68, 0.0]);
        assert_eq!(s.filament_cost, vec![0.11, 0.0]);
    }

    #[test]
    fn parse_dhms_covers_all_units() {
        assert_eq!(parse_dhms("45s"), Some(45));
        assert_eq!(parse_dhms("2h 30m"), Some(9000));
        assert_eq!(parse_dhms("1d 1h 1m 1s"), Some(90061));
        assert_eq!(parse_dhms(""), None);
        assert_eq!(parse_dhms("garbage"), None);
    }

    #[test]
    fn extracts_embedded_thumbnail() {
        let thumbs = extract_thumbnails(SAMPLE);
        assert_eq!(thumbs.len(), 1);
        assert_eq!(thumbs[0].width, 2);
        assert_eq!(thumbs[0].height, 2);
        // Les lignes base64 du bloc sont concaténées sans le préfixe « ; ».
        assert_eq!(thumbs[0].data, "iVBORw0KGgoAAAANSUhEUgAAAAI=");
    }

    #[test]
    fn tolerates_gcode_without_metadata() {
        let s = parse_stats("G1 X0 Y0\nG1 X1 Y1 E0.5\n");
        assert_eq!(s, GcodeStats::default());
        assert!(extract_thumbnails("G28\nG1 Z0.2\n").is_empty());
    }

    #[test]
    fn stats_json_skips_absent_fields() {
        let json = serde_json::to_value(parse_stats("G1 X0\n")).unwrap();
        // Aucun champ présent → objet vide (rien de bruité côté client).
        assert_eq!(json, serde_json::json!({}));
    }
}
