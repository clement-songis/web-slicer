//! Résultat de `GCodeProcessor` d'Orca (simulation cinématique d'un G-code
//! tranché) : colonnes typées par mouvement + résumé, et formats binaires
//! compacts servis à l'aperçu.
//!
//! Pur Rust (le backend l'utilise sans FFI ; le worker le remplit).
//!
//! Fichier complet (cache backend, `WSPg`) :
//!   magic "WSPg" | version u16 | 0 u16 | json_len u32 | json (meta) | pad 4 |
//!   move_count u32 | bytes[4n] | floats[16n] f32 | ints[2n] u32   (little-endian)
//! Plage de couches (`WSPm`, réponse de `/preview/moves`) :
//!   magic "WSPm" | version u16 | 0 u16 | from u32 | to u32 | first_move u32 |
//!   count u32 | bytes[4c] (pad 4) | floats[16c] | ints[2c]

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Colonnes par mouvement (`MoveVertex`).
pub const BYTES_PER_MOVE: usize = 4;
pub const FLOATS_PER_MOVE: usize = 16;
pub const INTS_PER_MOVE: usize = 2;

/// Indices dans `floats` (ordre de `gcode_process.cpp`).
pub mod col {
    pub const X: usize = 0;
    pub const Y: usize = 1;
    pub const Z: usize = 2;
    pub const FEEDRATE: usize = 3;
    pub const ACTUAL_FEEDRATE: usize = 4;
    pub const MM3_PER_MM: usize = 5;
    pub const WIDTH: usize = 6;
    pub const HEIGHT: usize = 7;
    pub const FAN: usize = 8;
    pub const TEMPERATURE: usize = 9;
    pub const ACCELERATION: usize = 10;
    pub const JERK: usize = 11;
    pub const PRESSURE_ADVANCE: usize = 12;
    pub const TIME_NORMAL: usize = 13;
    pub const TIME_STEALTH: usize = 14;
    pub const DELTA_E: usize = 15;
}

/// `EMoveType` d'Orca.
pub mod move_type {
    pub const NOOP: u8 = 0;
    pub const RETRACT: u8 = 1;
    pub const UNRETRACT: u8 = 2;
    pub const SEAM: u8 = 3;
    pub const TOOL_CHANGE: u8 = 4;
    pub const COLOR_CHANGE: u8 = 5;
    pub const PAUSE_PRINT: u8 = 6;
    pub const CUSTOM_GCODE: u8 = 7;
    pub const TRAVEL: u8 = 8;
    pub const WIPE: u8 = 9;
    pub const EXTRUDE: u8 = 10;
}

/// Nombre de rôles d'extrusion (`ExtrusionRole::erCount`).
pub const ROLE_COUNT: usize = 22;

/// Couche : plage de mouvements [first, end) et temps simulés.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProcessedLayer {
    pub z: f32,
    pub first_move: u32,
    pub end_move: u32,
    /// Ligne G-code (1-based) du premier mouvement de la couche : corrélation
    /// avec les couches de l'aperçu texte/WSPv sans charger les mouvements.
    #[serde(default)]
    pub first_line: u32,
    /// Temps de couche (s) [Normal, Silencieux].
    pub time: [f32; 2],
}

/// Lecture d'une grandeur par segment (bornes globales du méta).
type ChannelGetter = fn(&ProcessedGcode, usize) -> f32;

/// Version du méta (le backend retraite un cache plus ancien).
pub const META_VERSION: u64 = 2;

#[derive(Debug, Clone, PartialEq)]
pub struct ProcessedGcode {
    pub bytes: Vec<u8>,
    pub floats: Vec<f32>,
    pub ints: Vec<u32>,
    /// Résumé + couches + temps par rôle (cf. `build_meta`).
    pub meta: Value,
    pub layers: Vec<ProcessedLayer>,
}

impl ProcessedGcode {
    pub fn move_count(&self) -> usize {
        self.bytes.len() / BYTES_PER_MOVE
    }
    pub fn move_type(&self, i: usize) -> u8 {
        self.bytes[i * BYTES_PER_MOVE]
    }
    pub fn role(&self, i: usize) -> u8 {
        self.bytes[i * BYTES_PER_MOVE + 1]
    }
    pub fn extruder(&self, i: usize) -> u8 {
        self.bytes[i * BYTES_PER_MOVE + 2]
    }
    pub fn f(&self, i: usize, c: usize) -> f32 {
        self.floats[i * FLOATS_PER_MOVE + c]
    }
    pub fn layer_id(&self, i: usize) -> u32 {
        self.ints[i * INTS_PER_MOVE]
    }
    pub fn gcode_id(&self, i: usize) -> u32 {
        self.ints[i * INTS_PER_MOVE + 1]
    }

    /// Construit depuis les colonnes du worker et le résumé JSON de libslic3r.
    pub fn from_columns(
        bytes: Vec<u8>,
        floats: Vec<f32>,
        ints: Vec<u32>,
        summary_json: &str,
    ) -> Result<Self, String> {
        let n = bytes.len() / BYTES_PER_MOVE;
        if !bytes.len().is_multiple_of(BYTES_PER_MOVE)
            || floats.len() != n * FLOATS_PER_MOVE
            || ints.len() != n * INTS_PER_MOVE
        {
            return Err("colonnes GCodeProcessor incohérentes".into());
        }
        let summary: Value =
            serde_json::from_str(summary_json).map_err(|e| format!("résumé invalide : {e}"))?;
        let mut p = Self {
            bytes,
            floats,
            ints,
            meta: Value::Null,
            layers: Vec::new(),
        };
        p.layers = p.compute_layers();
        p.meta = p.build_meta(summary);
        Ok(p)
    }

    /// Couches = plages contiguës de même `layer_id` (ordre G-code) ; la
    /// cote est la plus haute des extrusions de la couche (comme GCodeViewer).
    fn compute_layers(&self) -> Vec<ProcessedLayer> {
        let mut layers: Vec<ProcessedLayer> = Vec::new();
        let mut current: Option<u32> = None;
        for i in 0..self.move_count() {
            let id = self.layer_id(i);
            if current != Some(id) {
                layers.push(ProcessedLayer {
                    z: 0.0,
                    first_move: i as u32,
                    end_move: i as u32,
                    first_line: self.gcode_id(i),
                    time: [0.0; 2],
                });
                current = Some(id);
            }
            let l = layers.last_mut().expect("couche courante");
            l.end_move = i as u32 + 1;
            l.time[0] += self.f(i, col::TIME_NORMAL);
            l.time[1] += self.f(i, col::TIME_STEALTH);
            if self.move_type(i) == move_type::EXTRUDE {
                l.z = l.z.max(self.f(i, col::Z));
            }
        }
        layers
    }

    /// Temps (s) cumulé des mouvements dont la ligne G-code est < `line`.
    pub fn elapsed_before_line(&self, line: u32, mode: usize) -> f64 {
        let c = if mode == 0 {
            col::TIME_NORMAL
        } else {
            col::TIME_STEALTH
        };
        (0..self.move_count())
            .filter(|&i| self.gcode_id(i) < line)
            .map(|i| f64::from(self.f(i, c)))
            .sum()
    }

    fn build_meta(&self, summary: Value) -> Value {
        // Temps par rôle (extrusions), déplacements, essuyages, autres — panneau
        // « Résumé » d'Orca (GCodeViewer::render_legend, temps par rôle).
        let mut role_time = vec![[0f64; 2]; ROLE_COUNT];
        let mut travel = [0f64; 2];
        let mut wipe = [0f64; 2];
        let mut other = [0f64; 2];
        let mut extruder_time = std::collections::BTreeMap::<u8, [f64; 2]>::new();
        let mut max_actual = 0f32;
        let mut max_volumetric = 0f32;
        // Bornes globales des grandeurs par segment d'extrusion (coloration de
        // l'aperçu sans charger tous les mouvements).
        let channels: [(&str, ChannelGetter); 7] = [
            ("fan", |p, i| p.f(i, col::FAN)),
            ("temperature", |p, i| p.f(i, col::TEMPERATURE)),
            ("acceleration", |p, i| p.f(i, col::ACCELERATION)),
            ("jerk", |p, i| p.f(i, col::JERK)),
            ("pa", |p, i| p.f(i, col::PRESSURE_ADVANCE)),
            ("actualSpeed", |p, i| p.f(i, col::ACTUAL_FEEDRATE)),
            ("actualFlow", |p, i| {
                p.f(i, col::ACTUAL_FEEDRATE) * p.f(i, col::MM3_PER_MM)
            }),
        ];
        let mut bounds = [(f32::INFINITY, f32::NEG_INFINITY); 7];
        for i in 0..self.move_count() {
            let t = [
                f64::from(self.f(i, col::TIME_NORMAL)),
                f64::from(self.f(i, col::TIME_STEALTH)),
            ];
            let slot = match self.move_type(i) {
                move_type::EXTRUDE => {
                    let r = (self.role(i) as usize).min(ROLE_COUNT - 1);
                    max_actual = max_actual.max(self.f(i, col::ACTUAL_FEEDRATE));
                    max_volumetric = max_volumetric
                        .max(self.f(i, col::ACTUAL_FEEDRATE) * self.f(i, col::MM3_PER_MM));
                    for (b, (_, get)) in bounds.iter_mut().zip(channels.iter()) {
                        let v = get(self, i);
                        b.0 = b.0.min(v);
                        b.1 = b.1.max(v);
                    }
                    &mut role_time[r]
                }
                move_type::TRAVEL => &mut travel,
                move_type::WIPE => &mut wipe,
                _ => &mut other,
            };
            slot[0] += t[0];
            slot[1] += t[1];
            let e = extruder_time.entry(self.extruder(i)).or_default();
            e[0] += t[0];
            e[1] += t[1];
        }
        let ranges: serde_json::Map<String, Value> = channels
            .iter()
            .zip(bounds.iter())
            .map(|((name, _), (lo, hi))| {
                let r = if lo.is_finite() {
                    [*lo, *hi]
                } else {
                    [0.0, 0.0]
                };
                ((*name).to_string(), serde_json::json!(r))
            })
            .collect();
        serde_json::json!({
            "version": META_VERSION,
            "ranges": ranges,
            "move_count": self.move_count(),
            "layer_count": self.layers.len(),
            "layers": self.layers,
            "role_times": role_time,
            "travel_time": travel,
            "wipe_time": wipe,
            "other_time": other,
            "extruder_times": extruder_time,
            "max_actual_feedrate": max_actual,
            "max_actual_volumetric_rate": max_volumetric,
            "summary": summary,
        })
    }

    /// Fichier complet `WSPg` (cache).
    pub fn encode_file(&self) -> Vec<u8> {
        let json = serde_json::to_vec(&self.meta).expect("meta sérialisable");
        let mut out =
            Vec::with_capacity(16 + json.len() + self.floats.len() * 4 + self.bytes.len() * 3);
        out.extend_from_slice(b"WSPg");
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&(json.len() as u32).to_le_bytes());
        out.extend_from_slice(&json);
        while out.len() % 4 != 0 {
            out.push(0);
        }
        out.extend_from_slice(&(self.move_count() as u32).to_le_bytes());
        out.extend_from_slice(&self.bytes);
        for v in &self.floats {
            out.extend_from_slice(&v.to_le_bytes());
        }
        for v in &self.ints {
            out.extend_from_slice(&v.to_le_bytes());
        }
        out
    }

    pub fn decode_file(data: &[u8]) -> Result<Self, String> {
        let err = || "fichier WSPg invalide".to_string();
        let u32_at = |o: usize| -> Result<u32, String> {
            data.get(o..o + 4)
                .map(|b| u32::from_le_bytes(b.try_into().expect("4 octets")))
                .ok_or_else(err)
        };
        if data.get(0..4) != Some(b"WSPg") {
            return Err(err());
        }
        let json_len = u32_at(8)? as usize;
        let json = data.get(12..12 + json_len).ok_or_else(err)?;
        let meta: Value = serde_json::from_slice(json).map_err(|_| err())?;
        let mut o = (12 + json_len).div_ceil(4) * 4;
        let n = u32_at(o)? as usize;
        o += 4;
        let bytes = data
            .get(o..o + n * BYTES_PER_MOVE)
            .ok_or_else(err)?
            .to_vec();
        o += n * BYTES_PER_MOVE;
        let fl = data.get(o..o + n * FLOATS_PER_MOVE * 4).ok_or_else(err)?;
        let floats = fl
            .chunks_exact(4)
            .map(|b| f32::from_le_bytes(b.try_into().expect("4")))
            .collect();
        o += n * FLOATS_PER_MOVE * 4;
        let it = data.get(o..o + n * INTS_PER_MOVE * 4).ok_or_else(err)?;
        let ints = it
            .chunks_exact(4)
            .map(|b| u32::from_le_bytes(b.try_into().expect("4")))
            .collect();
        let layers: Vec<ProcessedLayer> =
            serde_json::from_value(meta.get("layers").cloned().unwrap_or(Value::Null))
                .map_err(|_| err())?;
        Ok(Self {
            bytes,
            floats,
            ints,
            meta,
            layers,
        })
    }

    /// Mouvements des couches [from, to] (bornes ramenées dans l'intervalle).
    pub fn encode_range(&self, from: usize, to: usize) -> Vec<u8> {
        let (from, to, first, end) = if self.layers.is_empty() {
            (0, 0, 0usize, 0usize)
        } else {
            let last = self.layers.len() - 1;
            let from = from.min(last);
            let to = to.clamp(from, last);
            (
                from,
                to,
                self.layers[from].first_move as usize,
                self.layers[to].end_move as usize,
            )
        };
        let c = end - first;
        let mut out = Vec::with_capacity(24 + c * 76);
        out.extend_from_slice(b"WSPm");
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        for v in [from as u32, to as u32, first as u32, c as u32] {
            out.extend_from_slice(&v.to_le_bytes());
        }
        out.extend_from_slice(&self.bytes[first * BYTES_PER_MOVE..end * BYTES_PER_MOVE]);
        while out.len() % 4 != 0 {
            out.push(0);
        }
        for v in &self.floats[first * FLOATS_PER_MOVE..end * FLOATS_PER_MOVE] {
            out.extend_from_slice(&v.to_le_bytes());
        }
        for v in &self.ints[first * INTS_PER_MOVE..end * INTS_PER_MOVE] {
            out.extend_from_slice(&v.to_le_bytes());
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> ProcessedGcode {
        // 3 mouvements : déplacement (couche 0), extrusion (couche 0), extrusion (couche 1).
        let bytes = vec![8, 0, 0, 0, 10, 1, 0, 0, 10, 5, 1, 0];
        let mut floats = vec![0f32; 3 * FLOATS_PER_MOVE];
        for (i, (z, t)) in [(0.2f32, 1.0f32), (0.2, 2.0), (0.4, 4.0)]
            .iter()
            .enumerate()
        {
            floats[i * FLOATS_PER_MOVE + col::Z] = *z;
            floats[i * FLOATS_PER_MOVE + col::TIME_NORMAL] = *t;
            floats[i * FLOATS_PER_MOVE + col::TIME_STEALTH] = *t * 2.0;
            floats[i * FLOATS_PER_MOVE + col::ACTUAL_FEEDRATE] = 10.0;
            floats[i * FLOATS_PER_MOVE + col::MM3_PER_MM] = 0.1;
        }
        let ints = vec![0, 10, 0, 11, 1, 20];
        ProcessedGcode::from_columns(bytes, floats, ints, r#"{"modes":[]}"#).unwrap()
    }

    #[test]
    fn layers_times_and_roles() {
        let p = sample();
        assert_eq!(p.layers.len(), 2);
        assert_eq!(p.layers[0].time, [3.0, 6.0]);
        assert_eq!((p.layers[1].first_move, p.layers[1].end_move), (2, 3));
        assert!((p.layers[1].z - 0.4).abs() < 1e-6);
        assert_eq!(p.meta["travel_time"][0], 1.0);
        assert_eq!(p.meta["role_times"][1][0], 2.0);
        assert_eq!(p.meta["role_times"][5][1], 8.0);
        assert!((p.meta["max_actual_volumetric_rate"].as_f64().unwrap() - 1.0).abs() < 1e-6);
        assert_eq!(p.elapsed_before_line(20, 0), 3.0);
        assert_eq!(p.layers[1].first_line, 20);
        assert_eq!(p.meta["version"], META_VERSION);
        assert_eq!(
            p.meta["ranges"]["actualSpeed"],
            serde_json::json!([10.0, 10.0])
        );
    }

    #[test]
    fn file_round_trip_and_range() {
        let p = sample();
        let back = ProcessedGcode::decode_file(&p.encode_file()).unwrap();
        assert_eq!(back, p);
        let r = p.encode_range(1, 9);
        assert_eq!(&r[0..4], b"WSPm");
        let u = |o: usize| u32::from_le_bytes(r[o..o + 4].try_into().unwrap());
        assert_eq!((u(8), u(12), u(16), u(20)), (1, 1, 2, 1));
        assert_eq!(r.len(), 24 + 4 + FLOATS_PER_MOVE * 4 + INTS_PER_MOVE * 4);
    }
}
