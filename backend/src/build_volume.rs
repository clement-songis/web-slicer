//! Volume d'impression d'un plateau : port pur de Slic3r::BuildVolume
//! (libslic3r/BuildVolume.cpp, Orca 2.4.1) et de
//! ModelInstance::calc_print_volume_state (Model.cpp:3423) — lot 43, I1.
//!
//! Orca classe chaque instance avant tranchage (Model::update_print_volume_state
//! puis ModelInstance::is_printable) : entièrement dans le volume (Inside), à
//! cheval sur le bord ou trop haute (Partly_Outside, « An object is laid
//! over the boundary of plate or exceeds the height limit » — « Trancher »
//! grisé), ou entièrement hors du plateau (Fully_Outside, jamais tranché).
//!
//! Repère : celui du plateau (les positions de la scène sont relatives au
//! plateau de l'objet, voir plate_origin). Plateau = printable_area
//! (polygone, mm) × printable_height (0 = hauteur illimitée).
//!
//! Écarts assumés : inside_convex_polygon sur la décomposition haut/bas
//! « expand(SceneEpsilon) » d'Orca est remplacé par un test demi-plans sur
//! l'enveloppe convexe avec la même tolérance ; le cercle est ajusté par
//! moyenne (Orca : RANSAC) avec les mêmes critères d'acceptation. Les deux
//! donnent le même résultat sur les plateaux circulaires / convexes générés
//! par Orca (BedShapePanel).

use glam::{DMat4, DVec3};
use serde::Serialize;
use ts_rs::TS;

/// BuildVolume::SceneEpsilon (= EPSILON, libslic3r.h).
pub const SCENE_EPSILON: f64 = 1e-4;
/// SCALING_FACTOR de libslic3r (coordonnées entières des polygones).
const SCALING_FACTOR: f64 = 1e-6;
/// sqr(SCALED_EPSILON) en unités mises à l'échelle (SCALED_EPSILON = 100).
const SQR_SCALED_EPSILON: i128 = 100 * 100;
/// LOGICAL_PART_PLATE_GAP (PartPlate.cpp).
pub const LOGICAL_PART_PLATE_GAP: f64 = 1.0 / 5.0;

/// État d'une instance vis-à-vis du volume d'impression
/// (ModelInstanceEPrintVolumeState d'Orca).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum PrintVolumeState {
    /// ModelInstancePVS_Inside : entièrement dans le volume, imprimable.
    Inside,
    /// ModelInstancePVS_Partly_Outside : à cheval sur le bord du plateau ou
    /// plus haut que printable_height — Orca refuse de trancher.
    PartlyOutside,
    /// ModelInstancePVS_Fully_Outside : entièrement hors du plateau.
    Outside,
}

/// BuildVolume::ObjectState (état d'un volume).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VolumeState {
    Inside,
    Colliding,
    Outside,
    Below,
}

/// BuildVolume_Type.
#[derive(Debug, Clone, PartialEq)]
enum BedKind {
    Invalid,
    Rectangle,
    Circle {
        center: [f64; 2],
        radius: f64,
    },
    /// Enveloppe convexe (CCW) ; convex = le polygone est lui-même convexe.
    Polygon {
        hull: Vec<[f64; 2]>,
        convex: bool,
    },
}

/// Volume d'impression d'un plateau (Slic3r::BuildVolume).
#[derive(Debug, Clone, PartialEq)]
pub struct BuildVolume {
    kind: BedKind,
    /// m_bboxf (2D) : boîte du polygone du plateau.
    min: [f64; 2],
    max: [f64; 2],
    /// printable_height (0 = illimitée).
    max_print_height: f64,
}

/// Volume « pièce » à classer : sommets du maillage, triangles et matrice
/// monde (instance × volume), comme un ModelVolume is_model_part() d'Orca.
pub struct PartVolume<'a> {
    pub vertices: &'a [[f32; 3]],
    pub indices: &'a [[u32; 3]],
    pub matrix: DMat4,
}

fn scaled(v: f64) -> i64 {
    (v / SCALING_FACTOR).round() as i64
}

/// Aire signée ×2 (unités mises à l'échelle²) d'un polygone entier.
fn area2(points: &[(i64, i64)]) -> i128 {
    let mut sum: i128 = 0;
    for i in 0..points.len() {
        let (x1, y1) = points[i];
        let (x2, y2) = points[(i + 1) % points.len()];
        sum += x1 as i128 * y2 as i128 - x2 as i128 * y1 as i128;
    }
    sum
}

/// Enveloppe convexe (chaîne monotone d'Andrew), CCW, sans points colinéaires.
fn convex_hull(points: &[(i64, i64)]) -> Vec<(i64, i64)> {
    let mut pts = points.to_vec();
    pts.sort_unstable();
    pts.dedup();
    if pts.len() < 3 {
        return pts;
    }
    let cross = |o: (i64, i64), a: (i64, i64), b: (i64, i64)| -> i128 {
        (a.0 - o.0) as i128 * (b.1 - o.1) as i128 - (a.1 - o.1) as i128 * (b.0 - o.0) as i128
    };
    let mut lower: Vec<(i64, i64)> = Vec::new();
    for &p in &pts {
        while lower.len() >= 2 && cross(lower[lower.len() - 2], lower[lower.len() - 1], p) <= 0 {
            lower.pop();
        }
        lower.push(p);
    }
    let mut upper: Vec<(i64, i64)> = Vec::new();
    for &p in pts.iter().rev() {
        while upper.len() >= 2 && cross(upper[upper.len() - 2], upper[upper.len() - 1], p) <= 0 {
            upper.pop();
        }
        upper.push(p);
    }
    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
}

impl BuildVolume {
    /// BuildVolume::BuildVolume(printable_area, printable_height).
    pub fn new(printable_area: &[[f64; 2]], printable_height: f64) -> Self {
        let (mut min, mut max) = ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]);
        for p in printable_area {
            for axis in 0..2 {
                min[axis] = min[axis].min(p[axis]);
                max[axis] = max[axis].max(p[axis]);
            }
        }
        let max_print_height = printable_height.max(0.0);
        if printable_area.len() < 3 || !(max[0] > min[0] && max[1] > min[1]) {
            return Self {
                kind: BedKind::Invalid,
                min: [0.0; 2],
                max: [0.0; 2],
                max_print_height,
            };
        }
        let polygon: Vec<(i64, i64)> = printable_area
            .iter()
            .map(|p| (scaled(p[0]), scaled(p[1])))
            .collect();
        let hull = convex_hull(&polygon);
        let area = area2(&polygon).abs();
        let (bx0, by0, bx1, by1) = hull.iter().fold(
            (i64::MAX, i64::MAX, i64::MIN, i64::MIN),
            |(a, b, c, d), &(x, y)| (a.min(x), b.min(y), c.max(x), d.max(y)),
        );
        let bbox_area2 = 2 * (bx1 - bx0) as i128 * (by1 - by0) as i128;
        // Aires ×2 : comparées au double de sqr(SCALED_EPSILON).
        let rectangle =
            printable_area.len() >= 4 && (area - bbox_area2).abs() < 2 * SQR_SCALED_EPSILON;
        let kind = if rectangle {
            BedKind::Rectangle
        } else if let Some((center, radius)) = (printable_area.len() > 3)
            .then(|| fit_circle(printable_area))
            .flatten()
        {
            BedKind::Circle { center, radius }
        } else {
            let hull_area = area2(&hull).abs();
            BedKind::Polygon {
                hull: hull
                    .iter()
                    .map(|&(x, y)| [x as f64 * SCALING_FACTOR, y as f64 * SCALING_FACTOR])
                    .collect(),
                convex: hull_area - area < 2 * SQR_SCALED_EPSILON,
            }
        };
        Self {
            kind,
            min,
            max,
            max_print_height,
        }
    }

    /// Volume d'une config Orca (printable_area en points, printable_height) ;
    /// None si la config ne décrit aucun plateau.
    pub fn from_config(config: &engine::api::DynamicPrintConfig) -> Option<Self> {
        use engine::api::ConfigValue;
        let area: Vec<[f64; 2]> = match config.0.get("printable_area")? {
            ConfigValue::Points(points) => points
                .iter()
                .filter(|p| p.len() >= 2)
                .map(|p| [p[0], p[1]])
                .collect(),
            _ => return None,
        };
        let height = match config.0.get("printable_height") {
            Some(ConfigValue::Float(h)) => *h,
            Some(ConfigValue::Int(h)) => *h as f64,
            _ => 0.0,
        };
        let volume = Self::new(&area, height);
        (volume.kind != BedKind::Invalid).then_some(volume)
    }

    /// Largeur et profondeur (mm) du plateau (m_bboxf).
    pub fn size(&self) -> (f64, f64) {
        (self.max[0] - self.min[0], self.max[1] - self.min[1])
    }

    /// printable_height (0 = illimitée).
    pub fn max_print_height(&self) -> f64 {
        self.max_print_height
    }

    /// BuildVolume::volume_state_bbox(bbox, ignore_bottom = true).
    fn volume_state_bbox(&self, min: DVec3, max: DVec3) -> VolumeState {
        let top = if self.max_print_height == 0.0 {
            f64::MAX
        } else {
            self.max_print_height + SCENE_EPSILON
        };
        let lo = [
            self.min[0] - SCENE_EPSILON,
            self.min[1] - SCENE_EPSILON,
            f64::MIN,
        ];
        let hi = [
            self.max[0] + SCENE_EPSILON,
            self.max[1] + SCENE_EPSILON,
            top,
        ];
        let contains = |p: DVec3| {
            p.x >= lo[0]
                && p.x <= hi[0]
                && p.y >= lo[1]
                && p.y <= hi[1]
                && p.z >= lo[2]
                && p.z <= hi[2]
        };
        if contains(min) && contains(max) {
            return VolumeState::Inside;
        }
        // BoundingBox3Base::intersects : sans les bords.
        let intersects = lo[0] < max.x
            && hi[0] > min.x
            && lo[1] < max.y
            && hi[1] > min.y
            && lo[2] < max.z
            && hi[2] > min.z;
        if intersects {
            VolumeState::Colliding
        } else {
            VolumeState::Outside
        }
    }

    /// Point dans le volume (cercle / polygone), hauteur comprise
    /// (pt.z < max_print_height + SceneEpsilon si la hauteur est bornée).
    fn contains_point(&self, p: DVec3) -> bool {
        if self.max_print_height != 0.0 && p.z >= self.max_print_height + SCENE_EPSILON {
            return false;
        }
        match &self.kind {
            BedKind::Circle { center, radius } => {
                let (dx, dy) = (p.x - center[0], p.y - center[1]);
                let r = radius + SCENE_EPSILON;
                dx * dx + dy * dy < r * r
            }
            BedKind::Polygon { hull, .. } => {
                // Demi-plans de l'enveloppe CCW, élargie de SceneEpsilon.
                (0..hull.len()).all(|i| {
                    let a = hull[i];
                    let b = hull[(i + 1) % hull.len()];
                    let (ex, ey) = (b[0] - a[0], b[1] - a[1]);
                    let len = (ex * ex + ey * ey).sqrt();
                    len == 0.0 || (ex * (p.y - a[1]) - ey * (p.x - a[0])) / len >= -SCENE_EPSILON
                })
            }
            BedKind::Rectangle | BedKind::Invalid => true,
        }
    }

    /// object_state_templ(its, trafo, may_be_below_bed = true, convex, is_inside).
    fn object_state(&self, vertices: &[DVec3], indices: &[[u32; 3]], convex: bool) -> VolumeState {
        let world_min_z = -SCENE_EPSILON;
        let sign = |z: f64| -> i8 {
            if z > world_min_z {
                1
            } else if z < world_min_z {
                -1
            } else {
                0
            }
        };
        let mut sides = Vec::with_capacity(vertices.len());
        let (mut num_above, mut num_inside) = (0usize, 0usize);
        let mut below_outside = false;
        for &v in vertices {
            let s = sign(v.z);
            sides.push(s);
            if s >= 0 {
                num_above += 1;
                if self.contains_point(v) {
                    num_inside += 1;
                }
            } else if convex && !below_outside && !self.contains_point(DVec3::new(v.x, v.y, 0.0)) {
                below_outside = true;
            }
        }
        if num_above == 0 {
            return VolumeState::Below;
        }
        let mut inside = num_inside > 0;
        let mut outside = num_inside < num_above;
        // Orca : sur un volume convexe entièrement dedans, inutile de tester
        // les arêtes qui traversent le plateau.
        let settled = inside && outside || inside && !below_outside && convex;
        if num_above < vertices.len() && !settled {
            for tri in indices {
                let idx = tri.map(|i| i as usize);
                if idx.iter().any(|&i| i >= vertices.len()) {
                    continue;
                }
                let s = idx.map(|i| sides[i]);
                let lowest = s.iter().copied().min().unwrap_or(0);
                let highest = s.iter().copied().max().unwrap_or(0);
                if lowest < 0 && highest > 0 {
                    let mut prev = 2;
                    for edge in 0..3 {
                        if s[prev] * s[edge] == -1 {
                            let (p1, p2) = (vertices[idx[prev]], vertices[idx[edge]]);
                            let t = (world_min_z - p1.z) / (p2.z - p1.z);
                            let cut = DVec3::new(
                                p1.x + (p2.x - p1.x) * t,
                                p1.y + (p2.y - p1.y) * t,
                                world_min_z,
                            );
                            if self.contains_point(cut) {
                                inside = true;
                            } else {
                                outside = true;
                            }
                        }
                        prev = edge;
                    }
                    if inside && outside {
                        break;
                    }
                }
            }
        }
        match (inside, outside) {
            (true, true) => VolumeState::Colliding,
            (true, false) => VolumeState::Inside,
            _ => VolumeState::Outside,
        }
    }

    /// ModelInstance::calc_print_volume_state : état d'une instance à partir
    /// de ses volumes « pièce » (modificateurs, volumes négatifs, bloqueurs et
    /// forceurs exclus par l'appelant, comme is_model_part()).
    pub fn instance_state(&self, parts: &[PartVolume<'_>]) -> PrintVolumeState {
        const INSIDE: u8 = 1;
        const OUTSIDE: u8 = 2;
        if self.kind == BedKind::Invalid {
            // BuildVolume_Type::Invalid : aucun contrôle (Orca : Inside).
            return PrintVolumeState::Inside;
        }
        let mut inside_outside = 0u8;
        for part in parts {
            // Volume vide (boîte du maillage brut de taille nulle) : ignoré.
            let raw = bounds(
                part.vertices
                    .iter()
                    .map(|v| DVec3::new(v[0] as f64, v[1] as f64, v[2] as f64)),
            );
            let Some((raw_min, raw_max)) = raw else {
                continue;
            };
            let size = raw_max - raw_min;
            if size.x == 0.0 || size.y == 0.0 || size.z == 0.0 {
                continue;
            }
            let world: Vec<DVec3> = part
                .vertices
                .iter()
                .map(|v| {
                    part.matrix
                        .transform_point3(DVec3::new(v[0] as f64, v[1] as f64, v[2] as f64))
                })
                .collect();
            let Some((min, max)) = bounds(world.iter().copied()) else {
                continue;
            };
            let overlap2d = !(self.max[0] + SCENE_EPSILON < min.x
                || self.min[0] - SCENE_EPSILON > max.x
                || self.max[1] + SCENE_EPSILON < min.y
                || self.min[1] - SCENE_EPSILON > max.y);
            let state = if !overlap2d {
                VolumeState::Outside
            } else {
                match &self.kind {
                    BedKind::Rectangle => self.volume_state_bbox(min, max),
                    BedKind::Circle { .. } => self.object_state(&world, part.indices, true),
                    BedKind::Polygon { convex, .. } => {
                        self.object_state(&world, part.indices, *convex)
                    }
                    BedKind::Invalid => VolumeState::Inside,
                }
            };
            match state {
                VolumeState::Inside => inside_outside |= INSIDE,
                VolumeState::Outside => inside_outside |= OUTSIDE,
                // Volume sous le plateau : n'empêche pas l'impression des autres.
                VolumeState::Below => {}
                VolumeState::Colliding => inside_outside |= INSIDE | OUTSIDE,
            }
        }
        if inside_outside == INSIDE | OUTSIDE {
            PrintVolumeState::PartlyOutside
        } else if inside_outside == INSIDE {
            PrintVolumeState::Inside
        } else {
            PrintVolumeState::Outside
        }
    }
}

fn bounds(points: impl Iterator<Item = DVec3>) -> Option<(DVec3, DVec3)> {
    let mut out: Option<(DVec3, DVec3)> = None;
    for p in points {
        out = Some(match out {
            None => (p, p),
            Some((lo, hi)) => (lo.min(p), hi.max(p)),
        });
    }
    out
}

/// Ajustement de cercle d'un plateau discrétisé et critères d'Orca : sommets à
/// moins de 0,005 mm du cercle, milieux d'arêtes à moins de 3 mm en retrait.
fn fit_circle(points: &[[f64; 2]]) -> Option<([f64; 2], f64)> {
    let n = points.len() as f64;
    let center = [
        points.iter().map(|p| p[0]).sum::<f64>() / n,
        points.iter().map(|p| p[1]).sum::<f64>() / n,
    ];
    let dist = |p: [f64; 2]| ((p[0] - center[0]).powi(2) + (p[1] - center[1]).powi(2)).sqrt();
    let radius = points.iter().map(|&p| dist(p)).sum::<f64>() / n;
    let mut prev = *points.last()?;
    for &p in points {
        let mid = [(prev[0] + p[0]) / 2.0, (prev[1] + p[1]) / 2.0];
        if (dist(p) - radius).abs() > 0.005 || radius - dist(mid) > 3.0 {
            return None;
        }
        prev = p;
    }
    Some((center, radius))
}

/// compute_colum_count (PartPlate.hpp) : colonnes de la grille de plateaux.
pub fn plate_columns(count: usize) -> usize {
    let value = (count.max(1) as f32).sqrt();
    let rounded = value.round();
    if value > rounded {
        rounded as usize + 1
    } else {
        rounded as usize
    }
}

/// PartPlateList::compute_shape_position : origine (mm) du plateau index
/// parmi count dans le repère monde d'Orca (celui des positions écrites dans
/// un 3MF). Pas de grille = largeur entière (m_plate_width, int) × 1,2.
pub fn plate_origin(index: usize, count: usize, width: f64, depth: f64) -> [f64; 2] {
    let cols = plate_columns(count).max(1);
    let (row, col) = (index / cols, index % cols);
    let stride_x = width.trunc() * (1.0 + LOGICAL_PART_PLATE_GAP);
    let stride_y = depth.trunc() * (1.0 + LOGICAL_PART_PLATE_GAP);
    [col as f64 * stride_x, -(row as f64) * stride_y]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Cube de côté 20 centré sur l'origine.
    fn cube() -> (Vec<[f32; 3]>, Vec<[u32; 3]>) {
        let mut v = Vec::new();
        for x in [-10.0f32, 10.0] {
            for y in [-10.0f32, 10.0] {
                for z in [-10.0f32, 10.0] {
                    v.push([x, y, z]);
                }
            }
        }
        let f = vec![
            [0, 1, 3],
            [0, 3, 2],
            [4, 6, 7],
            [4, 7, 5],
            [0, 4, 5],
            [0, 5, 1],
            [2, 3, 7],
            [2, 7, 6],
            [0, 2, 6],
            [0, 6, 4],
            [1, 5, 7],
            [1, 7, 3],
        ];
        (v, f)
    }

    fn rect(w: f64, d: f64) -> Vec<[f64; 2]> {
        vec![[0.0, 0.0], [w, 0.0], [w, d], [0.0, d]]
    }

    fn state_at(bed: &BuildVolume, x: f64, y: f64, z: f64) -> PrintVolumeState {
        let (v, f) = cube();
        bed.instance_state(&[PartVolume {
            vertices: &v,
            indices: &f,
            matrix: DMat4::from_translation(DVec3::new(x, y, z)),
        }])
    }

    #[test]
    fn rectangle_bed_classifies_inside_straddling_outside_and_too_high() {
        let bed = BuildVolume::new(&rect(350.0, 350.0), 340.0);
        assert_eq!(bed.kind, BedKind::Rectangle);
        assert_eq!(state_at(&bed, 175.0, 175.0, 10.0), PrintVolumeState::Inside);
        // Bord exact (+ SceneEpsilon) : encore dedans, comme Orca.
        assert_eq!(state_at(&bed, 340.0, 175.0, 10.0), PrintVolumeState::Inside);
        // Audit 42 o2 : X = 345 → à cheval ; X = 500 → dehors.
        assert_eq!(
            state_at(&bed, 345.0, 175.0, 10.0),
            PrintVolumeState::PartlyOutside
        );
        assert_eq!(
            state_at(&bed, 500.0, 175.0, 10.0),
            PrintVolumeState::Outside
        );
        assert_eq!(
            state_at(&bed, 175.0, -30.0, 10.0),
            PrintVolumeState::Outside
        );
        // Plus haut que printable_height : à cheval (« exceeds the height limit »).
        assert_eq!(
            state_at(&bed, 175.0, 175.0, 335.0),
            PrintVolumeState::PartlyOutside
        );
        // Objet enfoncé sous le plateau : le bas est ignoré (ignore_bottom).
        assert_eq!(state_at(&bed, 175.0, 175.0, 0.0), PrintVolumeState::Inside);
        // Hauteur 0 : illimitée.
        let unlimited = BuildVolume::new(&rect(350.0, 350.0), 0.0);
        assert_eq!(
            state_at(&unlimited, 175.0, 175.0, 1000.0),
            PrintVolumeState::Inside
        );
    }

    #[test]
    fn volumes_combine_like_calc_print_volume_state() {
        let bed = BuildVolume::new(&rect(200.0, 200.0), 100.0);
        let (v, f) = cube();
        let part = |x: f64, z: f64| PartVolume {
            vertices: &v,
            indices: &f,
            matrix: DMat4::from_translation(DVec3::new(x, 100.0, z)),
        };
        // Une pièce dedans, une dehors : partiellement hors.
        assert_eq!(
            bed.instance_state(&[part(100.0, 10.0), part(400.0, 10.0)]),
            PrintVolumeState::PartlyOutside
        );
        // Pièce entièrement sous le plateau (rectangle : le bas est ignoré,
        // elle reste « dedans » comme volume_state_bbox d'Orca).
        assert_eq!(
            bed.instance_state(&[part(100.0, 10.0), part(100.0, -50.0)]),
            PrintVolumeState::Inside
        );
        // Volume plat (taille nulle) : ignoré ; aucun volume → hors plateau.
        let flat: Vec<[f32; 3]> = v.iter().map(|p| [p[0], p[1], 0.0]).collect();
        let flat_part = PartVolume {
            vertices: &flat,
            indices: &f,
            matrix: DMat4::from_translation(DVec3::new(400.0, 0.0, 0.0)),
        };
        assert_eq!(
            bed.instance_state(&[part(100.0, 10.0), flat_part]),
            PrintVolumeState::Inside
        );
        assert_eq!(bed.instance_state(&[]), PrintVolumeState::Outside);
    }

    #[test]
    fn circular_and_polygonal_beds() {
        let circle: Vec<[f64; 2]> = (0..72)
            .map(|i| {
                let a = i as f64 * std::f64::consts::TAU / 72.0;
                [100.0 * a.cos(), 100.0 * a.sin()]
            })
            .collect();
        let bed = BuildVolume::new(&circle, 200.0);
        assert!(matches!(bed.kind, BedKind::Circle { .. }), "{:?}", bed.kind);
        assert_eq!(state_at(&bed, 0.0, 0.0, 10.0), PrintVolumeState::Inside);
        // Coin du carré englobant : dans la boîte, hors du cercle.
        assert_eq!(state_at(&bed, 85.0, 85.0, 10.0), PrintVolumeState::Outside);
        assert_eq!(
            state_at(&bed, 90.0, 0.0, 10.0),
            PrintVolumeState::PartlyOutside
        );
        // Sous le plateau (cercle : object_state, may_be_below_bed) : ignoré.
        assert_eq!(state_at(&bed, 0.0, 0.0, -50.0), PrintVolumeState::Outside);

        // Hexagone (convexe ; 6 sommets : ni rectangle ni cercle).
        let hex: Vec<[f64; 2]> = (0..6)
            .map(|i| {
                let a = i as f64 * std::f64::consts::TAU / 6.0;
                [100.0 * a.cos(), 100.0 * a.sin()]
            })
            .collect();
        let bed = BuildVolume::new(&hex, 0.0);
        assert!(
            matches!(bed.kind, BedKind::Polygon { convex: true, .. }),
            "{:?}",
            bed.kind
        );
        assert_eq!(state_at(&bed, 0.0, 0.0, 10.0), PrintVolumeState::Inside);
        assert_eq!(
            state_at(&bed, 0.0, 85.0, 10.0),
            PrintVolumeState::PartlyOutside
        );
        assert_eq!(state_at(&bed, 300.0, 0.0, 10.0), PrintVolumeState::Outside);
    }

    #[test]
    fn config_without_bed_is_not_a_build_volume() {
        let config = engine::api::DynamicPrintConfig::new();
        assert!(BuildVolume::from_config(&config).is_none());
    }

    #[test]
    fn plate_origins_follow_partplatelist_grid() {
        assert_eq!(plate_columns(1), 1);
        assert_eq!(plate_columns(2), 2);
        assert_eq!(plate_columns(3), 2);
        assert_eq!(plate_columns(5), 3);
        assert_eq!(plate_origin(0, 2, 350.0, 350.0), [0.0, 0.0]);
        assert_eq!(plate_origin(1, 2, 350.0, 350.0), [420.0, 0.0]);
        assert_eq!(plate_origin(2, 3, 200.0, 240.0), [0.0, -288.0]);
        // Largeur non entière : tronquée comme m_plate_width (int).
        assert_eq!(plate_origin(1, 2, 220.5, 220.0), [264.0, 0.0]);
    }
}
