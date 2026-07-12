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

    /// Normales par sommet, lissées : moyenne des normales de face incidentes,
    /// normalisée (repli +Z si dégénérée). Les triangles aux indices hors bornes
    /// sont ignorés (robustesse). Alignées sur `vertices`.
    fn vertex_normals(&self) -> Vec<[f32; 3]> {
        let mut normals = vec![[0.0f32; 3]; self.vertices.len()];
        for tri in &self.indices {
            let (a, b, c) = (tri[0] as usize, tri[1] as usize, tri[2] as usize);
            if a >= self.vertices.len() || b >= self.vertices.len() || c >= self.vertices.len() {
                continue;
            }
            let (va, vb, vc) = (self.vertices[a], self.vertices[b], self.vertices[c]);
            let u = [vb[0] - va[0], vb[1] - va[1], vb[2] - va[2]];
            let w = [vc[0] - va[0], vc[1] - va[1], vc[2] - va[2]];
            // Normale de face (u × w), pondérée par l'aire (non normalisée ici).
            let n = [
                u[1] * w[2] - u[2] * w[1],
                u[2] * w[0] - u[0] * w[2],
                u[0] * w[1] - u[1] * w[0],
            ];
            for &vi in &[a, b, c] {
                for k in 0..3 {
                    normals[vi][k] += n[k];
                }
            }
        }
        for n in &mut normals {
            let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
            if len > f32::EPSILON {
                for c in n.iter_mut() {
                    *c /= len;
                }
            } else {
                *n = [0.0, 0.0, 1.0];
            }
        }
        normals
    }

    /// Encode le maillage au format d'affichage compact « WSMh » consommé par le
    /// client 3D (`frontend/src/lib/scene/mesh.ts`) et servi par le backend : les
    /// normales par sommet sont calculées ([`Self::vertex_normals`]). Source de
    /// vérité unique du maillage d'affichage produit par le moteur.
    pub fn encode_display(&self) -> Vec<u8> {
        let normals = self.vertex_normals();
        let vertex_count = self.vertices.len() as u32;
        let index_count = (self.indices.len() * 3) as u32;
        let mut out = Vec::with_capacity(16 + self.vertices.len() * 24 + self.indices.len() * 12);
        out.extend_from_slice(DISPLAY_MAGIC);
        out.extend_from_slice(&DISPLAY_VERSION.to_le_bytes());
        out.extend_from_slice(&vertex_count.to_le_bytes());
        out.extend_from_slice(&index_count.to_le_bytes());
        for v in &self.vertices {
            for &c in v {
                out.extend_from_slice(&c.to_le_bytes());
            }
        }
        for n in &normals {
            for &c in n {
                out.extend_from_slice(&c.to_le_bytes());
            }
        }
        for tri in &self.indices {
            for &i in tri {
                out.extend_from_slice(&i.to_le_bytes());
            }
        }
        out
    }
}

/// Magic + version du format d'affichage « WSMh » (little-endian) : miroir de
/// `backend::mesh` et du décodeur client `frontend/src/lib/scene/mesh.ts`.
const DISPLAY_MAGIC: &[u8; 4] = b"WSMh";
const DISPLAY_VERSION: u32 = 1;

/// Maillage d'affichage décodé (positions/normales/indices aplatis), tel que le
/// client le consomme — sert au round-trip de test et à la réutilisation backend.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DisplayMesh {
    pub positions: Vec<f32>,
    pub normals: Vec<f32>,
    pub indices: Vec<u32>,
}

/// Erreur de décodage du format d'affichage « WSMh ».
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum DisplayMeshError {
    #[error("en-tête WSMh invalide")]
    BadHeader,
    #[error("tampon WSMh tronqué")]
    Truncated,
}

impl DisplayMesh {
    /// Décode un tampon produit par [`TriangleMesh::encode_display`].
    pub fn decode(bytes: &[u8]) -> Result<DisplayMesh, DisplayMeshError> {
        if bytes.len() < 16 || &bytes[0..4] != DISPLAY_MAGIC {
            return Err(DisplayMeshError::BadHeader);
        }
        if read_u32(bytes, 4) != DISPLAY_VERSION {
            return Err(DisplayMeshError::BadHeader);
        }
        let vertex_count = read_u32(bytes, 8) as usize;
        let index_count = read_u32(bytes, 12) as usize;
        let floats = vertex_count * 3;
        if bytes.len() != 16 + floats * 4 * 2 + index_count * 4 {
            return Err(DisplayMeshError::Truncated);
        }
        let mut off = 16;
        let take_f32 = |n: usize, off: &mut usize| {
            let mut v = Vec::with_capacity(n);
            for _ in 0..n {
                v.push(f32::from_le_bytes(
                    bytes[*off..*off + 4].try_into().unwrap(),
                ));
                *off += 4;
            }
            v
        };
        let positions = take_f32(floats, &mut off);
        let normals = take_f32(floats, &mut off);
        let mut indices = Vec::with_capacity(index_count);
        for _ in 0..index_count {
            indices.push(read_u32(bytes, off));
            off += 4;
        }
        Ok(DisplayMesh {
            positions,
            normals,
            indices,
        })
    }
}

fn read_u32(bytes: &[u8], off: usize) -> u32 {
    u32::from_le_bytes(bytes[off..off + 4].try_into().unwrap())
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
    fn repaired_errors_counted() {
        let mut e = RepairedMeshErrors::default();
        assert!(!e.repaired_anything());
        e.edges_fixed = 2;
        assert!(e.repaired_anything());
    }

    #[test]
    fn encode_display_header_and_counts() {
        let m = tetra();
        let bytes = m.encode_display();
        assert_eq!(&bytes[0..4], b"WSMh");
        assert_eq!(u32::from_le_bytes(bytes[4..8].try_into().unwrap()), 1); // version
        assert_eq!(u32::from_le_bytes(bytes[8..12].try_into().unwrap()), 4); // 4 sommets
        assert_eq!(u32::from_le_bytes(bytes[12..16].try_into().unwrap()), 12); // 4 tris × 3
                                                                               // Taille = 16 + positions(4·3·4) + normales(4·3·4) + indices(12·4).
        assert_eq!(bytes.len(), 16 + 48 + 48 + 48);
    }

    #[test]
    fn encode_decode_round_trip_preserves_geometry() {
        let m = tetra();
        let d = DisplayMesh::decode(&m.encode_display()).expect("décodage");
        // Positions = sommets aplatis.
        let flat: Vec<f32> = m.vertices.iter().flatten().copied().collect();
        assert_eq!(d.positions, flat);
        // Indices = triangles aplatis.
        let idx: Vec<u32> = m.indices.iter().flatten().copied().collect();
        assert_eq!(d.indices, idx);
        // Une normale par sommet, unitaire.
        assert_eq!(d.normals.len(), flat.len());
        for n in d.normals.chunks(3) {
            let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
            assert!((len - 1.0).abs() < 1e-5, "normale unitaire");
        }
    }

    #[test]
    fn decode_rejects_bad_header_and_truncation() {
        assert_eq!(
            DisplayMesh::decode(b"XXXX").unwrap_err(),
            DisplayMeshError::BadHeader
        );
        let mut bytes = tetra().encode_display();
        bytes.truncate(bytes.len() - 4); // enlève un index
        assert_eq!(
            DisplayMesh::decode(&bytes).unwrap_err(),
            DisplayMeshError::Truncated
        );
    }

    #[test]
    fn vertex_normals_ignore_out_of_bounds_triangles() {
        // Un triangle référençant un sommet inexistant ne doit pas paniquer.
        let m = TriangleMesh {
            vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            indices: vec![[0, 1, 2], [0, 1, 9]],
        };
        let d = DisplayMesh::decode(&m.encode_display()).expect("décodage");
        assert_eq!(d.indices.len(), 6);
    }
}
