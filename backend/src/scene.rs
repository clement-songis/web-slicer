//! Outils de scène purs (T054) : arrangement en grille (FR-013), analyse de
//! maillage pour le rapport de réparation (FR-012) et suggestion d'orientation
//! (poser la plus grande facette vers le bas, FR-013).
//!
//! **Périmètre.** Ces implémentations pures fournissent des outils moteur
//! exploitables et testables sans dépendre de libslic3r. Les versions
//! haute-fidélité — nesting sans collision, réparation par recousu de trous,
//! auto-orient tenant compte des supports — passeront par le worker FFI
//! (`SlicerEngine`) une fois celui-ci câblé côté backend (phase tranchage P5).

use std::collections::HashMap;

use crate::mesh::Mesh;

// --- Arrangement -------------------------------------------------------------

/// Empreinte au sol d'un objet à arranger (mm).
#[derive(Debug, Clone)]
pub struct Footprint {
    pub id: String,
    pub width: f64,
    pub depth: f64,
}

/// Position calculée du centre d'un objet dans le repère plateau (mm).
#[derive(Debug, Clone, PartialEq)]
pub struct Placement {
    pub id: String,
    pub x: f64,
    pub y: f64,
}

/// Arrangement en étagères (shelf packing) dans un plateau `bed_w × bed_d`, avec
/// `spacing` mm entre objets et vis-à-vis des bords. Les empreintes sont placées
/// dans l'ordre fourni (déterministe) ; garanti sans collision car les cellules
/// ne se chevauchent pas. Renvoie les centres.
pub fn arrange_grid(items: &[Footprint], bed_w: f64, bed_d: f64, spacing: f64) -> Vec<Placement> {
    let mut placements = Vec::with_capacity(items.len());
    let mut cursor_x = spacing;
    let mut cursor_y = spacing;
    let mut shelf_depth = 0.0_f64;

    for it in items {
        // Passe à l'étagère suivante si l'objet ne tient plus en largeur.
        if cursor_x + it.width + spacing > bed_w && cursor_x > spacing {
            cursor_x = spacing;
            cursor_y += shelf_depth + spacing;
            shelf_depth = 0.0;
        }
        placements.push(Placement {
            id: it.id.clone(),
            x: cursor_x + it.width / 2.0,
            y: cursor_y + it.depth / 2.0,
        });
        cursor_x += it.width + spacing;
        if it.depth > shelf_depth {
            shelf_depth = it.depth;
        }
    }
    // `bed_d` sert de garde-fou de documentation ; le débordement vertical est
    // laissé au rapport client (l'arrangement fidèle multi-plateaux est FFI).
    let _ = bed_d;
    placements
}

// --- Analyse de maillage (rapport de réparation) -----------------------------

/// Rapport d'analyse d'un maillage (FR-012).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeshReport {
    /// Nombre de triangles.
    pub triangles: usize,
    /// Triangles d'aire quasi nulle (dégénérés).
    pub degenerate: usize,
    /// Arêtes de bord (présentes une seule fois → trou / maillage ouvert).
    pub open_edges: usize,
}

impl MeshReport {
    /// Un maillage est étanche s'il n'a aucune arête de bord.
    pub fn watertight(&self) -> bool {
        self.open_edges == 0
    }
}

fn triangle_area(mesh: &Mesh, a: usize, b: usize, c: usize) -> f64 {
    let p = |i: usize| {
        [
            mesh.positions[i * 3] as f64,
            mesh.positions[i * 3 + 1] as f64,
            mesh.positions[i * 3 + 2] as f64,
        ]
    };
    let (pa, pb, pc) = (p(a), p(b), p(c));
    let u = [pb[0] - pa[0], pb[1] - pa[1], pb[2] - pa[2]];
    let v = [pc[0] - pa[0], pc[1] - pa[1], pc[2] - pa[2]];
    let cx = u[1] * v[2] - u[2] * v[1];
    let cy = u[2] * v[0] - u[0] * v[2];
    let cz = u[0] * v[1] - u[1] * v[0];
    0.5 * (cx * cx + cy * cy + cz * cz).sqrt()
}

/// Analyse un maillage : compte triangles, facettes dégénérées et arêtes de
/// bord. Les sommets sont soudés par position (quantifiée) afin de détecter les
/// arêtes réellement partagées même si le maillage est une « soupe » (STL).
pub fn analyze_mesh(mesh: &Mesh) -> MeshReport {
    let triangles = mesh.indices.len() / 3;

    // Soudure des sommets par position quantifiée (0,1 µm).
    let mut weld: HashMap<(i64, i64, i64), usize> = HashMap::new();
    let mut welded_id = |i: u32| -> usize {
        let b = i as usize * 3;
        let key = (
            (mesh.positions[b] as f64 * 10_000.0).round() as i64,
            (mesh.positions[b + 1] as f64 * 10_000.0).round() as i64,
            (mesh.positions[b + 2] as f64 * 10_000.0).round() as i64,
        );
        let next = weld.len();
        *weld.entry(key).or_insert(next)
    };

    let mut degenerate = 0;
    let mut edges: HashMap<(usize, usize), u32> = HashMap::new();
    for t in 0..triangles {
        let ia = mesh.indices[t * 3];
        let ib = mesh.indices[t * 3 + 1];
        let ic = mesh.indices[t * 3 + 2];
        if triangle_area(mesh, ia as usize, ib as usize, ic as usize) < 1e-9 {
            degenerate += 1;
        }
        let (wa, wb, wc) = (welded_id(ia), welded_id(ib), welded_id(ic));
        for (u, v) in [(wa, wb), (wb, wc), (wc, wa)] {
            let key = if u <= v { (u, v) } else { (v, u) };
            *edges.entry(key).or_insert(0) += 1;
        }
    }
    let open_edges = edges.values().filter(|&&c| c == 1).count();

    MeshReport {
        triangles,
        degenerate,
        open_edges,
    }
}

// --- Suggestion d'orientation ------------------------------------------------

type Vec3 = [f64; 3];
type Quat = [f64; 4]; // x, y, z, w

fn normalize(v: Vec3) -> Vec3 {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if len == 0.0 {
        return v;
    }
    [v[0] / len, v[1] / len, v[2] / len]
}

/// Quaternion faisant tourner le vecteur unitaire `from` vers `to`
/// (algorithme `setFromUnitVectors` de Three.js), pour cohérence avec le client.
fn quaternion_from_unit_vectors(from: Vec3, to: Vec3) -> Quat {
    let f = normalize(from);
    let t = normalize(to);
    let mut r = f[0] * t[0] + f[1] * t[1] + f[2] * t[2] + 1.0;
    let (x, y, z);
    if r < 1e-8 {
        r = 0.0;
        if f[0].abs() > f[2].abs() {
            x = -f[1];
            y = f[0];
            z = 0.0;
        } else {
            x = 0.0;
            y = -f[2];
            z = f[1];
        }
    } else {
        x = f[1] * t[2] - f[2] * t[1];
        y = f[2] * t[0] - f[0] * t[2];
        z = f[0] * t[1] - f[1] * t[0];
    }
    let len = (x * x + y * y + z * z + r * r).sqrt();
    let len = if len == 0.0 { 1.0 } else { len };
    [x / len, y / len, z / len, r / len]
}

/// Convertit un quaternion en angles Euler XYZ (degrés), formules Three.js.
fn euler_deg_from_quaternion(q: Quat) -> Vec3 {
    let [x, y, z, w] = q;
    let m11 = 1.0 - 2.0 * (y * y + z * z);
    let m12 = 2.0 * (x * y - w * z);
    let m13 = 2.0 * (x * z + w * y);
    let m22 = 1.0 - 2.0 * (x * x + z * z);
    let m23 = 2.0 * (y * z - w * x);
    let m32 = 2.0 * (y * z + w * x);
    let m33 = 1.0 - 2.0 * (x * x + y * y);

    let ey = m13.clamp(-1.0, 1.0).asin();
    let (ex, ez) = if m13.abs() < 0.999_999_9 {
        ((-m23).atan2(m33), (-m12).atan2(m11))
    } else {
        (m32.atan2(m22), 0.0)
    };
    let deg = 180.0 / std::f64::consts::PI;
    [ex * deg, ey * deg, ez * deg]
}

/// Applique une rotation Euler XYZ (degrés) à un vecteur (matrice Three.js,
/// ordre XYZ). Exposé pour les tests d'orientation.
pub fn rotate_vector_by_euler_deg(euler: Vec3, v: Vec3) -> Vec3 {
    let rad = std::f64::consts::PI / 180.0;
    let (rx, ry, rz) = (euler[0] * rad, euler[1] * rad, euler[2] * rad);
    let (c1, s1) = (rx.cos(), rx.sin());
    let (c2, s2) = (ry.cos(), ry.sin());
    let (c3, s3) = (rz.cos(), rz.sin());
    let m = [
        [c2 * c3, -c2 * s3, s2],
        [c1 * s3 + c3 * s1 * s2, c1 * c3 - s1 * s2 * s3, -c2 * s1],
        [s1 * s3 - c1 * c3 * s2, c3 * s1 + c1 * s2 * s3, c1 * c2],
    ];
    [
        m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2],
        m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2],
        m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2],
    ]
}

/// Rotation Euler (degrés) posant la plus grande facette du maillage à plat sur
/// le plateau : sa normale est amenée vers le bas (-Z, repère Z-up d'Orca).
/// Heuristique d'auto-orientation (l'orient support-aware de libslic3r est FFI).
pub fn suggest_orientation(mesh: &Mesh) -> Vec3 {
    let triangles = mesh.indices.len() / 3;
    let mut best_area = -1.0_f64;
    let mut best_normal: Vec3 = [0.0, 0.0, -1.0];
    for t in 0..triangles {
        let ia = mesh.indices[t * 3] as usize;
        let ib = mesh.indices[t * 3 + 1] as usize;
        let ic = mesh.indices[t * 3 + 2] as usize;
        let area = triangle_area(mesh, ia, ib, ic);
        if area <= best_area {
            continue;
        }
        let p = |i: usize| {
            [
                mesh.positions[i * 3] as f64,
                mesh.positions[i * 3 + 1] as f64,
                mesh.positions[i * 3 + 2] as f64,
            ]
        };
        let (pa, pb, pc) = (p(ia), p(ib), p(ic));
        let u = [pb[0] - pa[0], pb[1] - pa[1], pb[2] - pa[2]];
        let v = [pc[0] - pa[0], pc[1] - pa[1], pc[2] - pa[2]];
        best_area = area;
        best_normal = normalize([
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ]);
    }
    euler_deg_from_quaternion(quaternion_from_unit_vectors(best_normal, [0.0, 0.0, -1.0]))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn footprint(id: &str, w: f64, d: f64) -> Footprint {
        Footprint {
            id: id.into(),
            width: w,
            depth: d,
        }
    }

    #[test]
    fn arrange_places_row_without_overlap() {
        let items = [footprint("a", 20.0, 20.0), footprint("b", 20.0, 20.0)];
        let out = arrange_grid(&items, 100.0, 100.0, 5.0);
        assert_eq!(
            out[0],
            Placement {
                id: "a".into(),
                x: 15.0,
                y: 15.0
            }
        );
        assert_eq!(
            out[1],
            Placement {
                id: "b".into(),
                x: 40.0,
                y: 15.0
            }
        );
        // Bords droits de a (25) et gauche de b (30) séparés → pas de collision.
        assert!(out[1].x - 20.0 / 2.0 >= out[0].x + 20.0 / 2.0);
    }

    #[test]
    fn arrange_wraps_to_next_shelf() {
        let items = [footprint("a", 20.0, 20.0), footprint("b", 20.0, 20.0)];
        let out = arrange_grid(&items, 30.0, 100.0, 5.0);
        assert_eq!(out[0].y, 15.0);
        assert_eq!(out[1].x, 15.0); // repart à gauche
        assert_eq!(out[1].y, 40.0); // étagère suivante : 5 (marge) + 20 (a) + 5 (marge) + 10 (demi-b)
    }

    fn tetrahedron() -> Mesh {
        // Tétraèdre fermé : 4 sommets, 4 facettes (soupe indexée séquentiellement).
        let verts = [
            [0.0f32, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
        ];
        let faces = [[0, 1, 2], [0, 1, 3], [1, 2, 3], [0, 2, 3]];
        let mut positions = Vec::new();
        for f in faces {
            for &vi in &f {
                positions.extend_from_slice(&verts[vi]);
            }
        }
        let indices: Vec<u32> = (0..positions.len() as u32 / 3).collect();
        Mesh {
            positions,
            normals: Vec::new(),
            indices,
        }
    }

    #[test]
    fn analyze_closed_mesh_is_watertight() {
        let report = analyze_mesh(&tetrahedron());
        assert_eq!(report.triangles, 4);
        assert_eq!(report.degenerate, 0);
        assert_eq!(report.open_edges, 0);
        assert!(report.watertight());
    }

    #[test]
    fn analyze_open_triangle_has_border_edges() {
        let mesh = Mesh {
            positions: vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0],
            normals: Vec::new(),
            indices: vec![0, 1, 2],
        };
        let report = analyze_mesh(&mesh);
        assert_eq!(report.triangles, 1);
        assert_eq!(report.open_edges, 3);
        assert!(!report.watertight());
    }

    #[test]
    fn analyze_counts_degenerate_facet() {
        // Triangle aplati (trois points colinéaires) → aire nulle.
        let mesh = Mesh {
            positions: vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 2.0, 0.0, 0.0],
            normals: Vec::new(),
            indices: vec![0, 1, 2],
        };
        assert_eq!(analyze_mesh(&mesh).degenerate, 1);
    }

    #[test]
    fn orientation_lays_largest_facet_down() {
        // Grande facette dans le plan XY, normale +Z → doit finir vers le bas.
        let mesh = Mesh {
            positions: vec![0.0, 0.0, 5.0, 10.0, 0.0, 5.0, 0.0, 10.0, 5.0],
            normals: Vec::new(),
            indices: vec![0, 1, 2],
        };
        let euler = suggest_orientation(&mesh);
        let down = rotate_vector_by_euler_deg(euler, [0.0, 0.0, 1.0]);
        assert!(
            (down[2] + 1.0).abs() < 1e-6,
            "normale non ramenée vers -Z: {down:?}"
        );
    }
}
