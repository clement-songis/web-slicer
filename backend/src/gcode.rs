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
        } else if let Some(v) = comment.strip_prefix("total layer number: ") {
            // OrcaSlicer 2.4.x : « total layer number: N » (variante à deux-points).
            stats.layer_count = v.trim().parse().ok();
        } else if let Some((_, rest)) = comment.split_once("total estimated time:") {
            // OrcaSlicer 2.4.x insère « … ; total estimated time: 8m 43s » sur la
            // ligne « model printing time ». On extrait la durée (jusqu'au `;`).
            let text = rest.trim().split(';').next().unwrap_or("").trim();
            if stats.estimated_time_seconds.is_none() && !text.is_empty() {
                stats.estimated_time_text = Some(text.to_string());
                stats.estimated_time_seconds = parse_dhms(text);
            }
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

// --- Prévisualisation par couches (T067, R6) ---------------------------------
//
// Modèle couches → segments typés reconstruit à partir des commentaires stables
// qu'OrcaSlicer émet (`;TYPE:`/`; FEATURE:`, `;LAYER_CHANGE`/`; CHANGE_LAYER`,
// `;HEIGHT:`/`; LAYER_HEIGHT:`, `;WIDTH:`/`; LINE_WIDTH:`) et des déplacements
// G0/G1 (E croissant = extrusion). Le backend l'expose en buffers binaires
// compacts par plage de couches (`WSPv`). La fidélité fine (arcs G2/G3, wipe,
// couture, débit exact) reste au parseur moteur (`engine/src/gcode`, FFI).

const KIND_UNKNOWN: u8 = 0;
const KIND_TRAVEL: u8 = 20;

/// Table rôle Orca (miroir `ExtrusionEntity::role_to_string`) → id u8 stable.
const SEGMENT_KINDS: &[(u8, &str)] = &[
    (1, "Inner wall"),
    (2, "Outer wall"),
    (3, "Overhang wall"),
    (4, "Sparse infill"),
    (5, "Internal solid infill"),
    (6, "Top surface"),
    (7, "Bottom surface"),
    (8, "Ironing"),
    (9, "Bridge"),
    (10, "Internal Bridge"),
    (11, "Gap infill"),
    (12, "Skirt"),
    (13, "Brim"),
    (14, "Support"),
    (15, "Support interface"),
    (16, "Support transition"),
    (17, "Prime tower"),
    (18, "Custom"),
    (19, "Multiple"),
];

/// Id u8 d'un rôle d'extrusion depuis son nom Orca (`Undefined`/inconnu → 0).
fn kind_id(name: &str) -> u8 {
    SEGMENT_KINDS
        .iter()
        .find(|(_, n)| n.eq_ignore_ascii_case(name))
        .map(|(id, _)| *id)
        .unwrap_or(KIND_UNKNOWN)
}

/// Nom d'un rôle d'extrusion depuis son id (pour la légende, FR-041).
pub fn kind_name(id: u8) -> &'static str {
    match id {
        KIND_TRAVEL => "Travel",
        _ => SEGMENT_KINDS
            .iter()
            .find(|(i, _)| *i == id)
            .map(|(_, n)| *n)
            .unwrap_or("Unknown"),
    }
}

/// Segment d'extrusion pour la préviz (positions mm).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PreviewSegment {
    pub start: [f32; 3],
    pub end: [f32; 3],
    /// Rôle d'extrusion (id `SEGMENT_KINDS`).
    pub kind: u8,
    pub extruder: u8,
    /// Vitesse commandée (mm/min, valeur `F`).
    pub feedrate: f32,
    pub width: f32,
    pub height: f32,
}

/// Couche de préviz : hauteur `z` + segments extrudés.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PreviewLayer {
    pub z: f32,
    pub segments: Vec<PreviewSegment>,
}

/// Modèle de préviz couches → segments (R6).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PreviewModel {
    pub layers: Vec<PreviewLayer>,
}

/// Résumé du modèle (méta-données `/preview/meta` : plages, types, échelles).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PreviewSummary {
    pub layer_count: u32,
    pub layer_z: Vec<f32>,
    pub layer_segment_counts: Vec<u32>,
    /// Ids de rôles présents (triés).
    pub kinds_present: Vec<u8>,
    /// Extrudeurs utilisés (triés).
    pub extruders_present: Vec<u8>,
    pub feedrate_range: (f32, f32),
    pub width_range: (f32, f32),
    pub height_range: (f32, f32),
}

const PREVIEW_MAGIC: &[u8; 4] = b"WSPv";
const PREVIEW_VERSION: u16 = 1;
/// Octets par enregistrement segment : 6×f32 (pos) + 3×f32 (F/w/h) + 2×u8 + u16.
pub const PREVIEW_RECORD_BYTES: u32 = 24 + 12 + 2 + 2;

impl PreviewModel {
    /// Nombre de couches.
    pub fn layer_count(&self) -> usize {
        self.layers.len()
    }

    /// Sérialise les segments des couches `[from, to]` (indices inclusifs,
    /// bornés) en buffer binaire little-endian :
    ///
    /// ```text
    /// magic "WSPv" | version u16 | from u32 | to u32 | segment_count u32
    /// puis segment_count × { start 3×f32 | end 3×f32 | F f32 | w f32 | h f32
    ///                        | kind u8 | extruder u8 | layer u16 }
    /// ```
    pub fn encode_range(&self, from: usize, to: usize) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(PREVIEW_MAGIC);
        out.extend_from_slice(&PREVIEW_VERSION.to_le_bytes());

        if self.layers.is_empty() {
            out.extend_from_slice(&0u32.to_le_bytes()); // from
            out.extend_from_slice(&0u32.to_le_bytes()); // to
            out.extend_from_slice(&0u32.to_le_bytes()); // count
            return out;
        }

        let last = self.layers.len() - 1;
        let from = from.min(last);
        let to = to.min(last).max(from);

        let count: u32 = self.layers[from..=to]
            .iter()
            .map(|l| l.segments.len() as u32)
            .sum();

        out.extend_from_slice(&(from as u32).to_le_bytes());
        out.extend_from_slice(&(to as u32).to_le_bytes());
        out.extend_from_slice(&count.to_le_bytes());

        for (offset, layer) in self.layers[from..=to].iter().enumerate() {
            let layer_index = (from + offset) as u16;
            for s in &layer.segments {
                for v in s.start.iter().chain(s.end.iter()) {
                    out.extend_from_slice(&v.to_le_bytes());
                }
                out.extend_from_slice(&s.feedrate.to_le_bytes());
                out.extend_from_slice(&s.width.to_le_bytes());
                out.extend_from_slice(&s.height.to_le_bytes());
                out.push(s.kind);
                out.push(s.extruder);
                out.extend_from_slice(&layer_index.to_le_bytes());
            }
        }
        out
    }

    /// Calcule les méta-données de la préviz (plages, types, échelles).
    pub fn summary(&self) -> PreviewSummary {
        let mut kinds = std::collections::BTreeSet::new();
        let mut extruders = std::collections::BTreeSet::new();
        let (mut fmin, mut fmax) = (f32::INFINITY, f32::NEG_INFINITY);
        let (mut wmin, mut wmax) = (f32::INFINITY, f32::NEG_INFINITY);
        let (mut hmin, mut hmax) = (f32::INFINITY, f32::NEG_INFINITY);

        for layer in &self.layers {
            for s in &layer.segments {
                kinds.insert(s.kind);
                extruders.insert(s.extruder);
                if s.feedrate > 0.0 {
                    fmin = fmin.min(s.feedrate);
                    fmax = fmax.max(s.feedrate);
                }
                if s.width > 0.0 {
                    wmin = wmin.min(s.width);
                    wmax = wmax.max(s.width);
                }
                if s.height > 0.0 {
                    hmin = hmin.min(s.height);
                    hmax = hmax.max(s.height);
                }
            }
        }

        let clamp = |lo: f32, hi: f32| if lo.is_finite() { (lo, hi) } else { (0.0, 0.0) };
        PreviewSummary {
            layer_count: self.layers.len() as u32,
            layer_z: self.layers.iter().map(|l| l.z).collect(),
            layer_segment_counts: self
                .layers
                .iter()
                .map(|l| l.segments.len() as u32)
                .collect(),
            kinds_present: kinds.into_iter().collect(),
            extruders_present: extruders.into_iter().collect(),
            feedrate_range: clamp(fmin, fmax),
            width_range: clamp(wmin, wmax),
            height_range: clamp(hmin, hmax),
        }
    }
}

/// État courant de la tête pendant l'analyse d'un G-code.
#[derive(Default)]
struct PreviewState {
    pos: [f32; 3],
    e: f32,
    feedrate: f32,
    extruder: u8,
    kind: u8,
    width: f32,
    height: f32,
    relative_e: bool,
}

/// Reconstruit le modèle couches → segments d'un G-code (R6).
pub fn parse_preview(gcode: &str) -> PreviewModel {
    let mut st = PreviewState::default();
    // Couche implicite 0 : accueille les mouvements avant le premier changement.
    let mut layers = vec![PreviewLayer::default()];

    for line in gcode.lines() {
        let (code, comment) = match line.split_once(';') {
            Some((c, rest)) => (c.trim(), Some(rest.trim())),
            None => (line.trim(), None),
        };

        if let Some(comment) = comment {
            apply_comment(comment, &mut st, &mut layers);
        }
        if !code.is_empty() {
            apply_code(code, &mut st, layers.last_mut().unwrap());
        }
    }

    // On ne garde que les couches portant des extrusions (les couches de
    // pur déplacement — amorçage, sauts — ne sont pas rendues).
    layers.retain(|l| !l.segments.is_empty());
    PreviewModel { layers }
}

/// Applique un commentaire (tags de rôle, changement de couche, hauteur/largeur).
fn apply_comment(comment: &str, st: &mut PreviewState, layers: &mut Vec<PreviewLayer>) {
    // Familles « compatible » (`TYPE:`) et moderne (`FEATURE:`).
    if let Some(v) = comment
        .strip_prefix("TYPE:")
        .or_else(|| strip_tag(comment, "FEATURE:"))
    {
        st.kind = kind_id(v.trim());
    } else if comment == "LAYER_CHANGE" || comment == "CHANGE_LAYER" {
        layers.push(PreviewLayer::default());
    } else if let Some(v) = comment.strip_prefix("Z:") {
        if let Ok(z) = v.trim().parse::<f32>() {
            layers.last_mut().unwrap().z = z;
        }
    } else if let Some(v) = comment
        .strip_prefix("HEIGHT:")
        .or_else(|| strip_tag(comment, "LAYER_HEIGHT:"))
    {
        st.height = v.trim().parse().unwrap_or(st.height);
    } else if let Some(v) = comment
        .strip_prefix("WIDTH:")
        .or_else(|| strip_tag(comment, "LINE_WIDTH:"))
    {
        st.width = v.trim().parse().unwrap_or(st.width);
    }
}

/// `strip_prefix` tolérant à un espace de tête (`; FEATURE: x` → `x`).
fn strip_tag<'a>(comment: &'a str, tag: &str) -> Option<&'a str> {
    comment.trim_start().strip_prefix(tag)
}

/// Applique une commande G-code (déplacements, modes E, changement d'outil).
fn apply_code(code: &str, st: &mut PreviewState, layer: &mut PreviewLayer) {
    let mut tokens = code.split_whitespace();
    let Some(cmd) = tokens.next() else { return };
    match cmd {
        "G0" | "G1" => {
            let mut new_pos = st.pos;
            let mut e_param: Option<f32> = None;
            for tok in tokens {
                let (axis, num) = tok.split_at(1);
                let Ok(val) = num.parse::<f32>() else {
                    continue;
                };
                match axis {
                    "X" => new_pos[0] = val,
                    "Y" => new_pos[1] = val,
                    "Z" => new_pos[2] = val,
                    "E" => e_param = Some(val),
                    "F" => st.feedrate = val,
                    _ => {}
                }
            }

            let extruding = match e_param {
                Some(e) if st.relative_e => e > 0.0,
                Some(e) => e > st.e,
                None => false,
            };

            if extruding {
                if layer.z == 0.0 {
                    layer.z = new_pos[2];
                }
                layer.segments.push(PreviewSegment {
                    start: st.pos,
                    end: new_pos,
                    kind: st.kind,
                    extruder: st.extruder,
                    feedrate: st.feedrate,
                    width: st.width,
                    height: st.height,
                });
            }

            st.pos = new_pos;
            if let Some(e) = e_param {
                if !st.relative_e {
                    st.e = e;
                }
            }
        }
        "G92" => {
            // Réinitialisation de l'origine d'extrusion (`G92 E0`).
            for tok in tokens {
                if let Some(v) = tok.strip_prefix('E') {
                    st.e = v.parse().unwrap_or(st.e);
                }
            }
        }
        "M82" => st.relative_e = false,
        "M83" => st.relative_e = true,
        _ => {
            // Changement d'outil `T0`..`T9`.
            if let Some(n) = cmd.strip_prefix('T').and_then(|n| n.parse::<u8>().ok()) {
                st.extruder = n;
            }
        }
    }
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

    // En-tête réel produit par OrcaSlicer 2.4.1 (variantes à deux-points pour la
    // durée totale et le nombre de couches, absentes du footer historique).
    const ORCA_2_4: &str = "\
; model printing time: 8m 42s; total estimated time: 8m 43s
; estimated first layer printing time (normal mode) = 1s
; total layer number: 50
; filament used [mm] = 793.51
; filament used [cm3] = 1.91
; filament used [g] = 2.37
G1 X0 Y0
";

    #[test]
    fn parses_orcaslicer_2_4_colon_variants() {
        let s = parse_stats(ORCA_2_4);
        assert_eq!(s.layer_count, Some(50));
        assert_eq!(s.estimated_time_text.as_deref(), Some("8m 43s"));
        assert_eq!(s.estimated_time_seconds, Some(523));
        assert_eq!(s.filament_used_g, vec![2.37]);
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

    // --- Préviz par couches (T067) -------------------------------------------

    /// Deux couches, rôles et extrudeurs typés (mode E absolu par défaut).
    const PREVIEW_GCODE: &str = "\
G1 X0 Y0 Z0.2 F1200
;LAYER_CHANGE
;Z:0.2
;HEIGHT:0.2
;WIDTH:0.45
;TYPE:Outer wall
G1 X10 Y0 E0.5 F1800
G1 X10 Y10 E1.0
;TYPE:Sparse infill
G1 X0 Y10 E1.5 F3000
;LAYER_CHANGE
;Z:0.4
;TYPE:Outer wall
T1
G1 X0 Y0 E2.0 F1800
";

    #[test]
    fn parses_layers_and_typed_segments() {
        let model = parse_preview(PREVIEW_GCODE);
        assert_eq!(model.layer_count(), 2);

        // Couche 0 : 3 extrusions (2 outer wall + 1 sparse infill).
        let l0 = &model.layers[0];
        assert_eq!(l0.z, 0.2);
        assert_eq!(l0.segments.len(), 3);
        assert_eq!(l0.segments[0].kind, kind_id("Outer wall"));
        assert_eq!(l0.segments[0].start, [0.0, 0.0, 0.2]);
        assert_eq!(l0.segments[0].end, [10.0, 0.0, 0.2]);
        assert_eq!(l0.segments[0].feedrate, 1800.0);
        assert_eq!(l0.segments[0].width, 0.45);
        assert_eq!(l0.segments[0].height, 0.2);
        assert_eq!(l0.segments[2].kind, kind_id("Sparse infill"));

        // Couche 1 : 1 extrusion sur l'extrudeur 1.
        let l1 = &model.layers[1];
        assert_eq!(l1.z, 0.4);
        assert_eq!(l1.segments.len(), 1);
        assert_eq!(l1.segments[0].extruder, 1);
    }

    #[test]
    fn travel_only_layers_are_dropped() {
        // Aucune extrusion → aucune couche rendue.
        let model = parse_preview("G1 X0 Y0 Z0.2\nG1 X5 Y5\n");
        assert_eq!(model.layer_count(), 0);
    }

    #[test]
    fn relative_extrusion_is_detected() {
        let model = parse_preview("M83\n;TYPE:Outer wall\nG1 X1 Y0 E0.1\nG1 X2 Y0 E0.1\n");
        assert_eq!(model.layers[0].segments.len(), 2);
    }

    #[test]
    fn summary_reports_ranges_and_types() {
        let s = parse_preview(PREVIEW_GCODE).summary();
        assert_eq!(s.layer_count, 2);
        assert_eq!(s.layer_z, vec![0.2, 0.4]);
        assert_eq!(s.layer_segment_counts, vec![3, 1]);
        assert_eq!(
            s.kinds_present,
            vec![kind_id("Outer wall"), kind_id("Sparse infill")]
        );
        assert_eq!(s.extruders_present, vec![0, 1]);
        assert_eq!(s.feedrate_range, (1800.0, 3000.0));
        assert_eq!(s.width_range, (0.45, 0.45));
    }

    #[test]
    fn encode_range_header_and_record_sizes() {
        let model = parse_preview(PREVIEW_GCODE);
        let buf = model.encode_range(0, 0);

        assert_eq!(&buf[0..4], PREVIEW_MAGIC);
        assert_eq!(u16::from_le_bytes([buf[4], buf[5]]), PREVIEW_VERSION);
        let from = u32::from_le_bytes([buf[6], buf[7], buf[8], buf[9]]);
        let to = u32::from_le_bytes([buf[10], buf[11], buf[12], buf[13]]);
        let count = u32::from_le_bytes([buf[14], buf[15], buf[16], buf[17]]);
        assert_eq!((from, to, count), (0, 0, 3));

        // En-tête (18 o) + 3 enregistrements.
        assert_eq!(buf.len(), 18 + 3 * PREVIEW_RECORD_BYTES as usize);
    }

    #[test]
    fn encode_range_is_clamped_to_available_layers() {
        let model = parse_preview(PREVIEW_GCODE);
        // Bornes hors limites → repli sur la dernière couche disponible.
        let buf = model.encode_range(5, 9);
        let to = u32::from_le_bytes([buf[10], buf[11], buf[12], buf[13]]);
        assert_eq!(to, 1, "borné à la dernière couche");

        // Modèle vide : en-tête seul, zéro segment.
        let empty = PreviewModel::default().encode_range(0, 3);
        assert_eq!(empty.len(), 18);
        assert_eq!(
            u32::from_le_bytes([empty[14], empty[15], empty[16], empty[17]]),
            0
        );
    }

    #[test]
    fn kind_name_round_trips_ids() {
        assert_eq!(kind_name(kind_id("Outer wall")), "Outer wall");
        assert_eq!(kind_name(KIND_TRAVEL), "Travel");
        assert_eq!(kind_name(200), "Unknown");
    }
}
