//! Maillages — miroir de `TriangleMesh` / `RepairedMeshErrors` (TriangleMesh.hpp).

use serde::{Deserialize, Serialize};

/// Maillage triangulaire indexé.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TriangleMesh {
    /// Sommets [x, y, z] en millimètres.
    pub vertices: Vec<[f32; 3]>,
    /// Triangles = triplets d'indices de sommets.
    pub indices: Vec<[u32; 3]>,
}

impl TriangleMesh {
    pub fn triangle_count(&self) -> usize {
        self.indices.len()
    }

    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }

    /// Boîte englobante (min, max) — None si vide.
    pub fn bounding_box(&self) -> Option<([f32; 3], [f32; 3])> {
        let mut it = self.vertices.iter();
        let first = *it.next()?;
        let (mut lo, mut hi) = (first, first);
        for v in it {
            for i in 0..3 {
                lo[i] = lo[i].min(v[i]);
                hi[i] = hi[i].max(v[i]);
            }
        }
        Some((lo, hi))
    }
}

/// Erreurs réparées sur un maillage (miroir `RepairedMeshErrors`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairedMeshErrors {
    pub edges_fixed: u32,
    pub degenerate_facets: u32,
    pub facets_removed: u32,
    pub facets_reversed: u32,
    pub backwards_edges: u32,
}

impl RepairedMeshErrors {
    pub fn repaired_anything(&self) -> bool {
        *self != Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tetra() -> TriangleMesh {
        TriangleMesh {
            vertices: vec![
                [0.0, 0.0, 0.0],
                [10.0, 0.0, 0.0],
                [0.0, 10.0, 0.0],
                [0.0, 0.0, 10.0],
            ],
            indices: vec![[0, 1, 2], [0, 1, 3], [0, 2, 3], [1, 2, 3]],
        }
    }

    #[test]
    fn bounding_box_et_comptage() {
        let m = tetra();
        assert_eq!(m.triangle_count(), 4);
        let (lo, hi) = m.bounding_box().unwrap();
        assert_eq!(lo, [0.0, 0.0, 0.0]);
        assert_eq!(hi, [10.0, 10.0, 10.0]);
        assert!(TriangleMesh::default().bounding_box().is_none());
    }

    #[test]
    fn erreurs_reparees() {
        let mut e = RepairedMeshErrors::default();
        assert!(!e.repaired_anything());
        e.edges_fixed = 2;
        assert!(e.repaired_anything());
    }
}
