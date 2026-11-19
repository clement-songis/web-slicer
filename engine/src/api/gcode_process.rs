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
    /// Temps estimé (s) [Normal, Silencieux] écoulé avant le premier mouvement
    /// de la couche : somme des temps des mouvements 1..first_move, comme
    /// `libvgcode::ViewerImpl::get_estimated_time_at` (le mouvement 0 n'a pas
    /// de sommet). Temps au mouvement m de la couche =
    /// `elapsed + Σ time(k)` pour k ∈ [max(first_move, 1), m] (cf.
    /// [`ProcessedGcode::estimated_time_at`]).
    #[serde(default)]
    pub elapsed: [f64; 2],
}

/// Couture émise par GCodeProcessor (`EMoveType::Seam`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Seam {
    /// Index de couche (dans `layers`).
    pub layer: u32,
    /// Index du mouvement (même numérotation que `/preview/processed/moves`).
    pub r#move: u32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/// Longueur (m) et masse (g) d'un volume de filament (mm³), comme
/// `get_used_filament_from_volume` de GCodeViewer::render_legend (unités
/// métriques) : m = 0,001·V / (π·(d/2)²), g = V·ρ·0,001.
pub fn used_filament_from_volume(volume: f64, diameter: f64, density: f64) -> (f64, f64) {
    let section = std::f64::consts::PI * (0.5 * diameter).powi(2);
    let m = if section > 0.0 {
        0.001 * volume / section
    } else {
        0.0
    };
    (m, volume * density * 0.001)
}

/// `filament_colour` du bloc de configuration d'un G-code Orca
/// (`; filament_colour = #F2754E;#FF0000`) : couleurs des outils de l'éditeur
/// (`Plater::get_extruder_colors_from_plater_config`, config du projet
/// tranché). GCodeProcessor ne garde que `extruder_colors` (repli de la
/// visionneuse G-code d'Orca).
pub fn filament_colours_from_gcode(reader: impl std::io::BufRead) -> Option<Vec<String>> {
    reader.lines().map_while(Result::ok).find_map(|line| {
        line.strip_prefix("; filament_colour = ").map(|rest| {
            rest.trim()
                .split(';')
                .map(|s| s.trim().trim_matches('"').to_string())
                .collect()
        })
    })
}

/// Les quatre colonnes du tableau « Filament » d'Orca (render_legend /
/// render_all_plates_stats) et la clé du volume par extrudeur dans
/// `PrintEstimatedStatistics`.
pub const FILAMENT_COLUMNS: [(&str, &str); 4] = [
    ("model", "model_volumes_per_extruder"),
    ("support", "support_volumes_per_extruder"),
    ("flushed", "flush_per_filament"),
    ("tower", "wipe_tower_volumes_per_extruder"),
];

fn f64_at(v: &Value, i: usize) -> f64 {
    v.get(i).and_then(Value::as_f64).unwrap_or(0.0)
}

fn usage(volume: f64, diameter: f64, density: f64) -> Value {
    let (m, g) = used_filament_from_volume(volume, diameter, density);
    serde_json::json!({ "volume": volume, "m": m, "g": g })
}

/// Statistiques « tous les plateaux » d'Orca (GCodeViewer::render_all_plates_stats) :
/// volumes de chaque colonne cumulés par extrudeur sur les plateaux (0 si
/// absent), puis convertis en m/g avec les diamètres/densités ; une colonne est
/// affichée si une ligne a m ou g non nul. Temps (Normal/Silencieux) sommés.
/// Chaque entrée de `plates` est le méta d'un plateau (`build_meta`) ; les
/// extrudeurs d'un plateau sont les lignes de son `filament_table`.
pub fn all_plates_filament_table(plates: &[&Value]) -> Value {
    use std::collections::BTreeMap;
    let mut volumes: BTreeMap<u64, [f64; 4]> = BTreeMap::new();
    let mut diameters: BTreeMap<u64, f64> = BTreeMap::new();
    let mut densities: BTreeMap<u64, f64> = BTreeMap::new();
    let mut colors: BTreeMap<u64, Value> = BTreeMap::new();
    let mut time = [0f64; 2];
    for meta in plates {
        let summary = &meta["summary"];
        for (mode, t) in time.iter_mut().enumerate() {
            *t += summary["modes"][mode]["time"].as_f64().unwrap_or(0.0);
        }
        let rows = meta["filament_table"]["rows"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        for row in rows {
            let Some(e) = row["extruder"].as_u64() else {
                continue;
            };
            let slot = volumes.entry(e).or_default();
            for (k, (_, key)) in FILAMENT_COLUMNS.iter().enumerate() {
                slot[k] += summary[*key][e.to_string()].as_f64().unwrap_or(0.0);
            }
            diameters.insert(e, f64_at(&summary["filament_diameters"], e as usize));
            densities.insert(e, f64_at(&summary["filament_densities"], e as usize));
            colors.insert(e, row["color"].clone());
        }
    }
    let mut columns = [false; 4];
    let mut totals = [(0f64, 0f64); 5];
    let mut out_rows = Vec::new();
    for (e, vols) in &volumes {
        let (d, rho) = (diameters[e], densities[e]);
        let mut row = serde_json::Map::new();
        row.insert("extruder".into(), serde_json::json!(e));
        row.insert("color".into(), colors[e].clone());
        let (mut sum_m, mut sum_g) = (0.0, 0.0);
        for (k, (name, _)) in FILAMENT_COLUMNS.iter().enumerate() {
            let (m, g) = used_filament_from_volume(vols[k], d, rho);
            if m != 0.0 || g != 0.0 {
                columns[k] = true;
            }
            sum_m += m;
            sum_g += g;
            totals[k].0 += m;
            totals[k].1 += g;
            row.insert((*name).into(), usage(vols[k], d, rho));
        }
        totals[4].0 += sum_m;
        totals[4].1 += sum_g;
        row.insert(
            "total".into(),
            serde_json::json!({ "m": sum_m, "g": sum_g }),
        );
        out_rows.push(Value::Object(row));
    }
    serde_json::json!({
        "columns": columns_json(columns),
        "rows": out_rows,
        "total": totals_json(&totals),
        "time": time,
        "plate_count": plates.len(),
    })
}

/// Couleurs des outils de l'éditeur : `filament_colours` (config du G-code,
/// ajouté au résumé par l'adaptateur FFI) sinon `extruder_colors` de
/// GCodeProcessor.
fn tool_colors(summary: &Value) -> &Value {
    match &summary["filament_colours"] {
        Value::Array(a) if !a.is_empty() => &summary["filament_colours"],
        _ => &summary["extruder_colors"],
    }
}

fn columns_json(c: [bool; 4]) -> Value {
    serde_json::json!({ "model": c[0], "support": c[1], "flushed": c[2], "tower": c[3] })
}

fn totals_json(t: &[(f64, f64); 5]) -> Value {
    let mut o = serde_json::Map::new();
    for (k, (name, _)) in FILAMENT_COLUMNS.iter().enumerate() {
        o.insert(
            (*name).into(),
            serde_json::json!({ "m": t[k].0, "g": t[k].1 }),
        );
    }
    o.insert(
        "total".into(),
        serde_json::json!({ "m": t[4].0, "g": t[4].1 }),
    );
    Value::Object(o)
}

/// Lecture d'une grandeur par segment (bornes globales du méta).
type ChannelGetter = fn(&ProcessedGcode, usize) -> f32;

/// Version du méta (le backend retraite un cache plus ancien).
pub const META_VERSION: u64 = 2;

/// Révision additive du méta (champs ajoutés sans casser la version 2 :
/// `color_print`, `filament_table`, `layers[].elapsed`, `seam_count`). Un
/// backend récent retraite un cache de révision inférieure ; un backend plus
/// ancien (qui ne vérifie que `version`) lit toujours le nouveau format.
pub const META_REVISION: u64 = 3;

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

    /// Couches comme `libvgcode::Layers::update` : une nouvelle couche commence
    /// au premier mouvement dont `layer_id` atteint le nombre de couches déjà
    /// vues ; un mouvement d'id inférieur (p. ex. le déplacement qu'Orca insère
    /// après une pause, encore à l'id de la couche précédente) reste dans la
    /// couche courante au lieu d'en ouvrir une fausse. La cote est la plus
    /// haute des extrusions de la couche.
    fn compute_layers(&self) -> Vec<ProcessedLayer> {
        let mut layers: Vec<ProcessedLayer> = Vec::new();
        // Temps cumulé des mouvements 1..i (le mouvement 0 n'est pas un sommet
        // de libvgcode, LibVGCodeWrapper::convert commence à i = 1).
        let mut elapsed = [0f64; 2];
        for i in 0..self.move_count() {
            let id = self.layer_id(i) as usize;
            if layers.is_empty() || id >= layers.len() {
                layers.push(ProcessedLayer {
                    z: 0.0,
                    first_move: i as u32,
                    end_move: i as u32,
                    first_line: self.gcode_id(i),
                    time: [0.0; 2],
                    elapsed,
                });
            }
            if i > 0 {
                elapsed[0] += f64::from(self.f(i, col::TIME_NORMAL));
                elapsed[1] += f64::from(self.f(i, col::TIME_STEALTH));
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

    /// Temps estimé (s) au mouvement `m` (inclus), mode 0 = Normal, 1 =
    /// Silencieux : `libvgcode::ViewerImpl::get_estimated_time_at` (fenêtre
    /// « position de l'outil » d'Orca, GCodeViewer.cpp:452), soit la somme des
    /// temps des mouvements 1..=m.
    pub fn estimated_time_at(&self, m: usize, mode: usize) -> f64 {
        let c = if mode == 0 {
            col::TIME_NORMAL
        } else {
            col::TIME_STEALTH
        };
        if self.move_count() == 0 {
            return 0.0;
        }
        let m = m.min(self.move_count() - 1);
        let li = self.layer_index_of_move(m);
        let layer = &self.layers[li];
        let start = (layer.first_move as usize).max(1);
        layer.elapsed[mode.min(1)] + (start..=m).map(|k| f64::from(self.f(k, c))).sum::<f64>()
    }

    /// Index (dans `layers`) de la couche qui contient le mouvement `m`.
    pub fn layer_index_of_move(&self, m: usize) -> usize {
        self.layers
            .partition_point(|l| (l.end_move as usize) <= m)
            .min(self.layers.len().saturating_sub(1))
    }

    /// Coutures émises par GCodeProcessor (`EMoveType::Seam`, détecteur de
    /// boucles fermées `m_seams_detector`) dans les couches [from, to] :
    /// `(couche, mouvement, x, y, z)` — marqueurs « Seams » de l'aperçu
    /// d'Orca (position du sommet option, LibVGCodeWrapper::convert).
    pub fn seams(&self, from: usize, to: usize) -> Vec<Seam> {
        if self.layers.is_empty() {
            return Vec::new();
        }
        let last = self.layers.len() - 1;
        let from = from.min(last);
        let to = to.clamp(from, last);
        let mut out = Vec::new();
        for (li, l) in self.layers.iter().enumerate().take(to + 1).skip(from) {
            for i in l.first_move as usize..l.end_move as usize {
                if self.move_type(i) == move_type::SEAM {
                    out.push(Seam {
                        layer: li as u32,
                        r#move: i as u32,
                        x: self.f(i, col::X),
                        y: self.f(i, col::Y),
                        z: self.f(i, col::Z),
                    });
                }
            }
        }
        out
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
        let color_print = self.color_print(&summary);
        let filament_table = self.filament_table(&summary);
        let seam_count = (0..self.move_count())
            .filter(|&i| self.move_type(i) == move_type::SEAM)
            .count();
        serde_json::json!({
            "version": META_VERSION,
            "revision": META_REVISION,
            "color_print": color_print,
            "filament_table": filament_table,
            "seam_count": seam_count,
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

    /// Extrudeurs réellement utilisés (au moins une extrusion), triés —
    /// `libvgcode::Viewer::get_used_extruders_ids`.
    pub fn used_extruders(&self) -> Vec<u8> {
        let mut used = std::collections::BTreeSet::new();
        for i in 0..self.move_count() {
            if self.move_type(i) == move_type::EXTRUDE {
                used.insert(self.extruder(i));
            }
        }
        used.into_iter().collect()
    }

    /// Données de la vue « Couleur » (`EViewType::ColorPrint`) d'Orca :
    /// - `palette` : couleurs des outils (`extruder_colors` du G-code) puis
    ///   une couleur par changement de couleur (`Plater::get_colors_for_color_print`) ;
    ///   la couleur d'un segment est `palette[cp_color_id % len]` (octet 3 des
    ///   mouvements `WSPm`, `ViewerImpl::get_vertex_color`) ;
    /// - `option_layers` : couches contenant une pause ou un G-code
    ///   personnalisé, rendues en gris (`DUMMY_COLOR`, `Layers::update`) ;
    /// - `extruders` : séquence des couleurs par extrudeur utilisé
    ///   (`ViewerImpl::m_used_extruders` / `get_color_prints`), avec la couche et
    ///   le temps estimé cumulé au premier segment de chaque couleur ;
    /// - `markers` : changements de couleur / pauses / G-code personnalisés
    ///   simulés (couche, mouvement, position), dans l'ordre du fichier ;
    /// - `custom_gcodes` : `custom_gcode_per_print_z` + couche correspondante.
    fn color_print(&self, summary: &Value) -> Value {
        use std::collections::{BTreeMap, BTreeSet};
        let customs = summary["custom_gcode_per_print_z"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        let with_changes = |tools: &Value| -> Vec<Value> {
            let mut palette = tools.as_array().cloned().unwrap_or_default();
            for item in &customs {
                if item["type"] == "color_change" {
                    palette.push(item["color"].clone());
                }
            }
            palette
        };
        let palette = with_changes(tool_colors(summary));
        let viewer_palette = with_changes(&summary["extruder_colors"]);
        let mut total = [0f64; 2];
        let mut option_layers = BTreeSet::new();
        let mut per_extruder: BTreeMap<u8, Vec<Value>> = BTreeMap::new();
        let mut markers = Vec::new();
        for i in 1..self.move_count() {
            total[0] += f64::from(self.f(i, col::TIME_NORMAL));
            total[1] += f64::from(self.f(i, col::TIME_STEALTH));
            let t = self.move_type(i);
            let kind = match t {
                move_type::COLOR_CHANGE => Some("color_change"),
                move_type::PAUSE_PRINT => Some("pause_print"),
                move_type::CUSTOM_GCODE => Some("custom"),
                _ => None,
            };
            if let Some(kind) = kind {
                let layer = self.layer_index_of_move(i);
                if t != move_type::COLOR_CHANGE {
                    option_layers.insert(layer);
                }
                markers.push(serde_json::json!({
                    "type": kind,
                    "layer": layer,
                    "move": i,
                    "extruder": self.extruder(i),
                    "x": self.f(i, col::X),
                    "y": self.f(i, col::Y),
                    "z": self.f(i, col::Z),
                    "time": total,
                }));
            }
            if t == move_type::EXTRUDE {
                let color_id = self.bytes[i * BYTES_PER_MOVE + 3];
                let list = per_extruder.entry(self.extruder(i)).or_default();
                if list.last().and_then(|v| v["color_id"].as_u64()) != Some(u64::from(color_id)) {
                    list.push(serde_json::json!({
                        "color_id": color_id,
                        "layer": self.layer_index_of_move(i),
                        "time": total,
                    }));
                }
            }
        }
        let custom_gcodes: Vec<Value> = customs
            .into_iter()
            .map(|mut item| {
                let z = item["print_z"].as_f64().unwrap_or(0.0);
                let layer = self
                    .layers
                    .iter()
                    .position(|l| f64::from(l.z) >= z - 1e-4)
                    .map(Value::from)
                    .unwrap_or(Value::Null);
                item["layer"] = layer;
                item
            })
            .collect();
        let extruders: Vec<Value> = per_extruder
            .into_iter()
            .map(|(e, items)| serde_json::json!({ "extruder": e, "items": items }))
            .collect();
        serde_json::json!({
            "palette": palette,
            "viewer_palette": viewer_palette,
            "option_layers": option_layers,
            "extruders": extruders,
            "markers": markers,
            "custom_gcodes": custom_gcodes,
        })
    }

    /// Tableau « Filament » du plateau (GCodeViewer::render_legend, vue
    /// Couleur / Résumé) : une ligne par extrudeur utilisé, colonnes Modèle,
    /// Support, Purgé, Tour (volume mm³, longueur m, masse g) et Total. Une
    /// colonne est affichée dès qu'un extrudeur utilisé a une entrée dans le
    /// volume correspondant de `PrintEstimatedStatistics` (comme Orca).
    fn filament_table(&self, summary: &Value) -> Value {
        let used = self.used_extruders();
        let colors = tool_colors(summary).as_array();
        let mut columns = [false; 4];
        let mut totals = [(0f64, 0f64); 5];
        let mut rows = Vec::new();
        for e in used {
            let i = e as usize;
            let d = f64_at(&summary["filament_diameters"], i);
            let rho = f64_at(&summary["filament_densities"], i);
            let mut row = serde_json::Map::new();
            row.insert("extruder".into(), serde_json::json!(e));
            row.insert(
                "color".into(),
                colors
                    .and_then(|c| c.get(i))
                    .cloned()
                    .unwrap_or(Value::Null),
            );
            let (mut sum_m, mut sum_g) = (0.0, 0.0);
            for (k, (name, key)) in FILAMENT_COLUMNS.iter().enumerate() {
                let entry = summary[*key].get(e.to_string()).and_then(Value::as_f64);
                if entry.is_some() {
                    columns[k] = true;
                }
                let volume = entry.unwrap_or(0.0);
                let (m, g) = used_filament_from_volume(volume, d, rho);
                sum_m += m;
                sum_g += g;
                totals[k].0 += m;
                totals[k].1 += g;
                row.insert((*name).into(), usage(volume, d, rho));
            }
            totals[4].0 += sum_m;
            totals[4].1 += sum_g;
            row.insert(
                "total".into(),
                serde_json::json!({ "m": sum_m, "g": sum_g }),
            );
            rows.push(Value::Object(row));
        }
        serde_json::json!({
            "columns": columns_json(columns),
            "rows": rows,
            "total": totals_json(&totals),
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
    fn lower_layer_id_stays_in_the_current_layer_like_libvgcode() {
        // ids 0, 1, 0 (déplacement inséré après une pause), 1, 2.
        let n = 5;
        let bytes = [8u8, 10, 8, 10, 10]
            .iter()
            .flat_map(|t| [*t, 1, 0, 0])
            .collect();
        let floats = vec![0f32; n * FLOATS_PER_MOVE];
        let ints = [0u32, 1, 0, 1, 2]
            .iter()
            .enumerate()
            .flat_map(|(i, id)| [*id, i as u32 + 1])
            .collect();
        let p = ProcessedGcode::from_columns(bytes, floats, ints, "{}").unwrap();
        let ranges: Vec<(u32, u32)> = p
            .layers
            .iter()
            .map(|l| (l.first_move, l.end_move))
            .collect();
        assert_eq!(ranges, vec![(0, 1), (1, 4), (4, 5)]);
    }

    #[test]
    fn estimated_time_skips_move_zero_like_libvgcode() {
        let p = sample();
        // Mouvement 0 (1 s) sans sommet libvgcode : temps cumulé 1..=m.
        assert_eq!(p.layers[0].elapsed, [0.0, 0.0]);
        assert_eq!(p.layers[1].elapsed, [2.0, 4.0]);
        assert_eq!(p.estimated_time_at(0, 0), 0.0);
        assert_eq!(p.estimated_time_at(1, 0), 2.0);
        assert_eq!(p.estimated_time_at(2, 0), 6.0);
        assert_eq!(p.estimated_time_at(2, 1), 12.0);
        assert_eq!(p.estimated_time_at(99, 0), 6.0);
        assert_eq!(p.layer_index_of_move(1), 0);
        assert_eq!(p.layer_index_of_move(2), 1);
        assert_eq!(p.meta["revision"], META_REVISION);
    }

    /// Plateau à 2 extrudeurs : changement de couleur M600 sur l'outil 0,
    /// pause, couture ; volumes de `PrintEstimatedStatistics`.
    fn colored() -> ProcessedGcode {
        // type, rôle, extrudeur, cp_color_id
        let moves: [(u8, u8, u8, u8, u32, f32); 8] = [
            (move_type::TRAVEL, 0, 0, 0, 0, 0.5),
            (move_type::EXTRUDE, 1, 0, 0, 0, 1.0),
            (move_type::SEAM, 0, 0, 0, 0, 0.0),
            (move_type::COLOR_CHANGE, 0, 0, 2, 1, 0.0),
            (move_type::EXTRUDE, 1, 0, 2, 1, 2.0),
            (move_type::PAUSE_PRINT, 0, 0, 2, 2, 0.0),
            (move_type::EXTRUDE, 1, 1, 1, 2, 3.0),
            (move_type::EXTRUDE, 1, 0, 2, 2, 4.0),
        ];
        let mut bytes = Vec::new();
        let mut floats = vec![0f32; moves.len() * FLOATS_PER_MOVE];
        let mut ints = Vec::new();
        for (i, (t, r, e, c, layer, time)) in moves.iter().enumerate() {
            bytes.extend_from_slice(&[*t, *r, *e, *c]);
            floats[i * FLOATS_PER_MOVE + col::X] = i as f32;
            floats[i * FLOATS_PER_MOVE + col::Z] = 0.2 * (*layer as f32 + 1.0);
            floats[i * FLOATS_PER_MOVE + col::TIME_NORMAL] = *time;
            floats[i * FLOATS_PER_MOVE + col::TIME_STEALTH] = *time;
            ints.extend_from_slice(&[*layer, 10 + i as u32]);
        }
        let summary = serde_json::json!({
            "modes": [{"time": 10.5}, {"time": 11.0}],
            "extruder_colors": ["#F2754E", "#FF0000"],
            "filament_diameters": [1.75, 1.75],
            "filament_densities": [1.24, 1.26],
            "model_volumes_per_extruder": {"0": 1000.0, "1": 500.0},
            "flush_per_filament": {"0": 100.0},
            "wipe_tower_volumes_per_extruder": {"0": 50.0, "1": 25.0},
            "support_volumes_per_extruder": {},
            "custom_gcode_per_print_z": [
                {"print_z": 0.4, "type": "color_change", "extruder": 1, "color": "#00FF00", "extra": ""},
                {"print_z": 0.6, "type": "pause_print", "extruder": 1, "color": "", "extra": ""}
            ]
        });
        ProcessedGcode::from_columns(bytes, floats, ints, &summary.to_string()).unwrap()
    }

    #[test]
    fn filament_colours_are_read_from_the_config_block() {
        let text = "G1 X1\n; extruder_colour = #FF8000\n; filament_colour = #F2754E;#FF0000\n";
        assert_eq!(
            filament_colours_from_gcode(text.as_bytes()),
            Some(vec!["#F2754E".to_string(), "#FF0000".to_string()])
        );
        assert_eq!(filament_colours_from_gcode("G1 X1\n".as_bytes()), None);
    }

    #[test]
    fn color_print_follows_libvgcode() {
        let p = colored();
        let cp = &p.meta["color_print"];
        assert_eq!(
            cp["palette"],
            serde_json::json!(["#F2754E", "#FF0000", "#00FF00"])
        );
        // Pause (couche 2) grisée ; le changement de couleur ne l'est pas.
        assert_eq!(cp["option_layers"], serde_json::json!([2]));
        let e0 = &cp["extruders"][0];
        assert_eq!(e0["extruder"], 0);
        let ids: Vec<u64> = e0["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v["color_id"].as_u64().unwrap())
            .collect();
        assert_eq!(ids, vec![0, 2]);
        // Temps cumulé (sans le mouvement 0) au premier segment de la couleur 2.
        assert_eq!(e0["items"][1]["time"][0], 3.0);
        assert_eq!(e0["items"][1]["layer"], 1);
        assert_eq!(cp["extruders"][1]["items"][0]["color_id"], 1);
        assert_eq!(cp["markers"][0]["type"], "color_change");
        assert_eq!(cp["markers"][1]["type"], "pause_print");
        assert_eq!(cp["custom_gcodes"][0]["layer"], 1);
        assert_eq!(cp["custom_gcodes"][1]["layer"], 2);
        assert_eq!(p.meta["seam_count"], 1);
        let seams = p.seams(0, 9);
        assert_eq!(seams.len(), 1);
        assert_eq!((seams[0].layer, seams[0].r#move, seams[0].x), (0, 2, 2.0));
        assert!(p.seams(1, 2).is_empty());
    }

    #[test]
    fn filament_table_matches_orca_conversion() {
        let p = colored();
        let t = &p.meta["filament_table"];
        assert_eq!(
            t["columns"],
            serde_json::json!({"model": true, "support": false, "flushed": true, "tower": true})
        );
        let section = std::f64::consts::PI * 0.875f64.powi(2);
        let row0 = &t["rows"][0];
        let m = row0["model"]["m"].as_f64().unwrap();
        assert!((m - 1.0 / section).abs() < 1e-12);
        assert!((row0["model"]["g"].as_f64().unwrap() - 1.24).abs() < 1e-12);
        assert!((row0["flushed"]["g"].as_f64().unwrap() - 0.124).abs() < 1e-12);
        assert!((row0["total"]["g"].as_f64().unwrap() - 1150.0 * 1.24e-3).abs() < 1e-12);
        assert_eq!(t["rows"][1]["flushed"]["volume"], 0.0);
        let total_g = t["total"]["total"]["g"].as_f64().unwrap();
        assert!((total_g - (1150.0 * 1.24e-3 + 525.0 * 1.26e-3)).abs() < 1e-12);

        // Tous les plateaux : volumes cumulés puis convertis.
        let all = all_plates_filament_table(&[&p.meta, &p.meta]);
        assert_eq!(all["plate_count"], 2);
        assert_eq!(all["time"], serde_json::json!([21.0, 22.0]));
        assert_eq!(all["rows"][0]["model"]["volume"], 2000.0);
        assert_eq!(all["columns"]["support"], false);
        assert!((all["rows"][1]["tower"]["g"].as_f64().unwrap() - 50.0 * 1.26e-3).abs() < 1e-12);
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
