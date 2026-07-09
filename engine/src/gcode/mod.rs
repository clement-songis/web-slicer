//! Parseur de G-code OrcaSlicer → modèle de prévisualisation (T021, R6/FR-040).
//!
//! Pur Rust (aucune dépendance FFI) : partagé par toutes les implémentations
//! du trait. Il relit le G-code produit par le moteur et reconstruit les
//! couches et segments à partir des marqueurs OrcaSlicer :
//! - `; CHANGE_LAYER` / `; Z_HEIGHT: z` / `; LAYER_HEIGHT: h` : bornes de couche ;
//! - `; FEATURE: <rôle>` : type de ligne (miroir de `role_to_string`) ;
//! - `; LINE_WIDTH: w` : largeur d'extrusion courante ;
//! - `G0`/`G1` : déplacements ; extrusion si ΔE > 0 (E relatif, `M83`).

use std::io::{BufRead, BufReader};
use std::path::Path;

use crate::api::{
    EngineError, EngineErrorCode, EngineResult, GcodeLayer, GcodePreview, GcodeSegment, LineKind,
    SliceStats,
};

/// Rôle OrcaSlicer (`; FEATURE: …`) → `LineKind` (ExtrusionEntity.cpp).
fn feature_to_kind(label: &str) -> LineKind {
    match label.trim() {
        "Outer wall" => LineKind::OuterWall,
        "Inner wall" => LineKind::InnerWall,
        "Overhang wall" => LineKind::Overhang,
        "Sparse infill" => LineKind::Infill,
        "Internal solid infill" => LineKind::SolidInfill,
        "Top surface" => LineKind::TopSurface,
        "Bottom surface" => LineKind::BottomSurface,
        "Ironing" => LineKind::Ironing,
        "Bridge" => LineKind::Bridge,
        "Internal Bridge" => LineKind::InternalBridge,
        "Gap infill" => LineKind::GapFill,
        "Skirt" => LineKind::Skirt,
        "Brim" => LineKind::Brim,
        "Support" => LineKind::Support,
        "Support interface" => LineKind::SupportInterface,
        "Support transition" => LineKind::SupportTransition,
        "Prime tower" => LineKind::PrimeTower,
        "Custom" => LineKind::Custom,
        _ => LineKind::Unknown,
    }
}

/// État de balayage du parseur.
struct Parser {
    layers: Vec<GcodeLayer>,
    current: Option<GcodeSegment>,
    // position courante (mm)
    x: f32,
    y: f32,
    z: f32,
    feedrate_mm_s: f32,
    width: f32,
    extruder: u8,
    kind: LineKind,
    relative_e: bool,
    last_e: f32,
    stats: SliceStats,
}

impl Parser {
    fn new() -> Self {
        Self {
            layers: Vec::new(),
            current: None,
            x: 0.0,
            y: 0.0,
            z: 0.0,
            feedrate_mm_s: 0.0,
            width: 0.0,
            extruder: 0,
            kind: LineKind::Custom,
            relative_e: false,
            last_e: 0.0,
            stats: SliceStats::default(),
        }
    }

    fn flush_segment(&mut self) {
        if let Some(seg) = self.current.take() {
            if seg.points.len() >= 2 {
                self.ensure_layer();
                if let Some(layer) = self.layers.last_mut() {
                    layer.segments.push(seg);
                }
            }
        }
    }

    fn ensure_layer(&mut self) {
        if self.layers.is_empty() {
            self.layers.push(GcodeLayer {
                z: self.z,
                height: 0.0,
                segments: Vec::new(),
            });
        }
    }

    fn start_layer(&mut self) {
        self.flush_segment();
        self.layers.push(GcodeLayer {
            z: self.z,
            height: 0.0,
            segments: Vec::new(),
        });
    }

    /// Ajoute un point au segment courant (crée/rompt selon le type).
    fn push_point(&mut self, kind: LineKind, x: f32, y: f32, z: f32, prev: (f32, f32, f32)) {
        let continues = matches!(&self.current, Some(seg)
            if seg.kind == kind && seg.extruder == self.extruder);
        if !continues {
            self.flush_segment();
            self.current = Some(GcodeSegment {
                kind,
                points: vec![[prev.0, prev.1, prev.2]],
                feedrate: self.feedrate_mm_s,
                width: if kind == LineKind::Travel {
                    0.0
                } else {
                    self.width
                },
                extruder: self.extruder,
            });
        }
        if let Some(seg) = self.current.as_mut() {
            seg.points.push([x, y, z]);
            seg.feedrate = self.feedrate_mm_s;
        }
    }

    fn handle_comment(&mut self, body: &str) {
        let body = body.trim();
        if body == "CHANGE_LAYER" {
            self.start_layer();
        } else if let Some(rest) = body.strip_prefix("Z_HEIGHT:") {
            if let Ok(z) = rest.trim().parse::<f32>() {
                self.z = z;
                self.ensure_layer();
                if let Some(layer) = self.layers.last_mut() {
                    layer.z = z;
                }
            }
        } else if let Some(rest) = body.strip_prefix("LAYER_HEIGHT:") {
            if let Ok(h) = rest.trim().parse::<f32>() {
                self.ensure_layer();
                if let Some(layer) = self.layers.last_mut() {
                    layer.height = h;
                }
            }
        } else if let Some(rest) = body.strip_prefix("FEATURE:") {
            self.flush_segment();
            self.kind = feature_to_kind(rest);
        } else if let Some(rest) = body.strip_prefix("LINE_WIDTH:") {
            if let Ok(w) = rest.trim().parse::<f32>() {
                self.width = w;
            }
        } else {
            self.parse_stat_comment(body);
        }
    }

    /// Extrait les statistiques des commentaires d'en-tête/pied.
    fn parse_stat_comment(&mut self, body: &str) {
        if let Some(rest) = body.strip_prefix("total estimated time:") {
            self.stats.estimated_time_s = parse_duration_s(rest.trim());
        } else if let Some((_, v)) = body.split_once("total estimated time:") {
            self.stats.estimated_time_s = parse_duration_s(v.trim());
        } else if let Some(rest) = body.strip_prefix("total layer number:") {
            if let Ok(n) = rest.trim().parse::<u32>() {
                self.stats.layer_count = n;
            }
        } else if let Some(rest) = body.strip_prefix("filament used [mm] =") {
            if let Ok(mm) = rest.trim().parse::<f64>() {
                self.stats.filament_mm.push(mm);
            }
        } else if let Some(rest) = body.strip_prefix("filament used [g] =") {
            for part in rest.split(',') {
                if let Ok(g) = part.trim().parse::<f64>() {
                    self.stats.filament_g.push(g);
                }
            }
        }
    }

    fn handle_move(&mut self, line: &str) {
        let prev = (self.x, self.y, self.z);
        let (mut nx, mut ny, mut nz) = prev;
        let mut de: Option<f32> = None;
        let mut moved_xy = false;
        for tok in line.split_whitespace().skip(1) {
            let (letter, val) = tok.split_at(1);
            let num: f32 = match val.parse() {
                Ok(v) => v,
                Err(_) => continue,
            };
            match letter {
                "X" => {
                    nx = num;
                    moved_xy = true;
                }
                "Y" => {
                    ny = num;
                    moved_xy = true;
                }
                "Z" => nz = num,
                "E" => {
                    de = Some(if self.relative_e {
                        num
                    } else {
                        num - self.last_e
                    });
                    if !self.relative_e {
                        self.last_e = num;
                    }
                }
                "F" => self.feedrate_mm_s = num / 60.0,
                _ => {}
            }
        }

        let extruding = de.is_some_and(|d| d > 0.0) && moved_xy;
        if extruding {
            self.push_point(self.kind, nx, ny, nz, prev);
        } else if moved_xy {
            self.push_point(LineKind::Travel, nx, ny, nz, prev);
        }
        // (déplacements Z purs et rétractions : mise à jour d'état sans segment)
        self.x = nx;
        self.y = ny;
        self.z = nz;
    }

    fn finish(mut self) -> GcodePreview {
        self.flush_segment();
        let mut kinds: Vec<LineKind> = Vec::new();
        for layer in &self.layers {
            for seg in &layer.segments {
                if !kinds.contains(&seg.kind) {
                    kinds.push(seg.kind);
                }
            }
        }
        if self.stats.layer_count == 0 {
            self.stats.layer_count = self.layers.len() as u32;
        }
        GcodePreview {
            layers: self.layers,
            stats: self.stats,
            kinds_present: kinds,
        }
    }
}

fn parse_duration_s(text: &str) -> f64 {
    // formats OrcaSlicer : « 29m 56s », « 1h 2m 3s », « 45s »
    let mut total = 0.0;
    for part in text.split_whitespace() {
        let (num, unit) = part.split_at(part.len().saturating_sub(1));
        if let Ok(v) = num.parse::<f64>() {
            match unit {
                "h" => total += v * 3600.0,
                "m" => total += v * 60.0,
                "s" => total += v,
                _ => {}
            }
        }
    }
    total
}

/// Relit un G-code produit par le moteur en modèle de prévisualisation.
pub fn parse_gcode(path: &Path) -> EngineResult<GcodePreview> {
    let file = std::fs::File::open(path).map_err(|e| {
        EngineError::new(EngineErrorCode::Io, e.to_string()).with_subject(path.to_string_lossy())
    })?;
    let reader = BufReader::new(file);
    let mut parser = Parser::new();
    let mut tool_changes: u32 = 0;

    for line in reader.lines() {
        let line = line.map_err(EngineError::from)?;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(body) = line.strip_prefix(';') {
            parser.handle_comment(body);
            continue;
        }
        // découpe un éventuel commentaire en fin de ligne
        let code = line.split(';').next().unwrap_or(line).trim();
        if code.is_empty() {
            continue;
        }
        let cmd = code.split_whitespace().next().unwrap_or("");
        match cmd {
            "G0" | "G1" => parser.handle_move(code),
            "M82" => parser.relative_e = false,
            "M83" => parser.relative_e = true,
            "G92" => parser.last_e = 0.0,
            _ => {
                if let Some(rest) = cmd.strip_prefix('T') {
                    if rest.chars().all(|c| c.is_ascii_digit()) && !rest.is_empty() {
                        tool_changes += 1;
                    }
                }
            }
        }
    }

    let mut preview = parser.finish();
    // le premier T<n> sélectionne l'outil sans être un changement.
    preview.stats.tool_changes = tool_changes.saturating_sub(1);
    Ok(preview)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_duration_formats() {
        assert_eq!(parse_duration_s("45s"), 45.0);
        assert_eq!(parse_duration_s("29m 56s"), 29.0 * 60.0 + 56.0);
        assert_eq!(parse_duration_s("1h 2m 3s"), 3600.0 + 120.0 + 3.0);
    }

    #[test]
    fn feature_labels_mappes() {
        assert_eq!(feature_to_kind("Outer wall"), LineKind::OuterWall);
        assert_eq!(
            feature_to_kind("Internal solid infill"),
            LineKind::SolidInfill
        );
        assert_eq!(feature_to_kind("Inconnu"), LineKind::Unknown);
    }
}
