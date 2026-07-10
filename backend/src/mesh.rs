//! Maillage affichable et son encodage binaire compact pour le client 3D
//! (Threlte/three.js). Format little-endian, lisible via `DataView` :
//!
//! ```text
//! magic         : 4 o  = "WSMh"
//! version       : u32  = 1
//! vertex_count  : u32           (positions/normales = vertex_count*3 f32)
//! index_count   : u32
//! positions     : f32 * vertex_count*3
//! normals       : f32 * vertex_count*3
//! indices       : u32 * index_count
//! ```
//!
//! Le parseur STL (binaire et ASCII) est pur Rust : l'aperçu d'un STL ne
//! dépend pas du moteur. Les autres formats (OBJ/3MF/STEP) sont extraits via le
//! moteur (FFI) ou les loaders client (T051) — hors de ce module.

const MAGIC: &[u8; 4] = b"WSMh";
const VERSION: u32 = 1;

/// Maillage trianglé non indexé ou indexé : positions/normales par sommet,
/// indices de triangles.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Mesh {
    /// Coordonnées `x,y,z` aplaties (longueur = 3 × nombre de sommets).
    pub positions: Vec<f32>,
    /// Normales `x,y,z` aplaties, alignées sur `positions`.
    pub normals: Vec<f32>,
    /// Indices de sommets, 3 par triangle.
    pub indices: Vec<u32>,
}

impl Mesh {
    /// Nombre de sommets (positions/3).
    pub fn vertex_count(&self) -> usize {
        self.positions.len() / 3
    }

    /// Encode le maillage au format binaire compact décrit en tête de module.
    pub fn encode(&self) -> Vec<u8> {
        let vertex_count = self.vertex_count() as u32;
        let index_count = self.indices.len() as u32;
        let mut out = Vec::with_capacity(
            16 + self.positions.len() * 4 + self.normals.len() * 4 + self.indices.len() * 4,
        );
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&VERSION.to_le_bytes());
        out.extend_from_slice(&vertex_count.to_le_bytes());
        out.extend_from_slice(&index_count.to_le_bytes());
        for &p in &self.positions {
            out.extend_from_slice(&p.to_le_bytes());
        }
        for &n in &self.normals {
            out.extend_from_slice(&n.to_le_bytes());
        }
        for &i in &self.indices {
            out.extend_from_slice(&i.to_le_bytes());
        }
        out
    }

    /// Décode un tampon produit par [`Mesh::encode`] (round-trip, tests/clients).
    pub fn decode(bytes: &[u8]) -> Result<Mesh, MeshError> {
        if bytes.len() < 16 || &bytes[0..4] != MAGIC {
            return Err(MeshError::BadHeader);
        }
        let version = read_u32(bytes, 4);
        if version != VERSION {
            return Err(MeshError::BadHeader);
        }
        let vertex_count = read_u32(bytes, 8) as usize;
        let index_count = read_u32(bytes, 12) as usize;
        let floats = vertex_count * 3;
        let expected = 16 + floats * 4 * 2 + index_count * 4;
        if bytes.len() != expected {
            return Err(MeshError::Truncated);
        }
        let mut off = 16;
        let mut positions = Vec::with_capacity(floats);
        for _ in 0..floats {
            positions.push(read_f32(bytes, off));
            off += 4;
        }
        let mut normals = Vec::with_capacity(floats);
        for _ in 0..floats {
            normals.push(read_f32(bytes, off));
            off += 4;
        }
        let mut indices = Vec::with_capacity(index_count);
        for _ in 0..index_count {
            indices.push(read_u32(bytes, off));
            off += 4;
        }
        Ok(Mesh {
            positions,
            normals,
            indices,
        })
    }
}

/// Erreur de décodage/parse de maillage.
#[derive(Debug, thiserror::Error, PartialEq)]
pub enum MeshError {
    #[error("en-tête de maillage invalide")]
    BadHeader,
    #[error("tampon de maillage tronqué")]
    Truncated,
    #[error("STL invalide : {0}")]
    InvalidStl(&'static str),
}

/// Parse un STL (binaire ou ASCII) en maillage non indexé (3 sommets/triangle,
/// normale répétée par sommet ; indices séquentiels).
pub fn parse_stl(bytes: &[u8]) -> Result<Mesh, MeshError> {
    if is_binary_stl(bytes) {
        parse_binary_stl(bytes)
    } else {
        parse_ascii_stl(bytes)
    }
}

/// Un STL binaire cohérent : en-tête 80 o + u32 + n×50 o. On évite le faux
/// positif d'un ASCII commençant par « solid » en vérifiant la taille exacte.
fn is_binary_stl(bytes: &[u8]) -> bool {
    if bytes.len() < 84 {
        return false;
    }
    let n = read_u32(bytes, 80) as usize;
    84 + n * 50 == bytes.len()
}

fn parse_binary_stl(bytes: &[u8]) -> Result<Mesh, MeshError> {
    let n = read_u32(bytes, 80) as usize;
    let mut mesh = Mesh::default();
    mesh.positions.reserve(n * 9);
    mesh.normals.reserve(n * 9);
    mesh.indices.reserve(n * 3);
    let mut off = 84;
    for _ in 0..n {
        let nx = read_f32(bytes, off);
        let ny = read_f32(bytes, off + 4);
        let nz = read_f32(bytes, off + 8);
        off += 12;
        for _ in 0..3 {
            mesh.positions.push(read_f32(bytes, off));
            mesh.positions.push(read_f32(bytes, off + 4));
            mesh.positions.push(read_f32(bytes, off + 8));
            mesh.normals.extend_from_slice(&[nx, ny, nz]);
            off += 12;
        }
        off += 2; // attribute byte count
        let base = (mesh.indices.len()) as u32;
        mesh.indices.extend_from_slice(&[base, base + 1, base + 2]);
    }
    Ok(mesh)
}

fn parse_ascii_stl(bytes: &[u8]) -> Result<Mesh, MeshError> {
    let text = std::str::from_utf8(bytes).map_err(|_| MeshError::InvalidStl("non UTF-8"))?;
    let mut mesh = Mesh::default();
    let mut normal = [0.0f32; 3];
    let mut verts_in_facet = 0u32;
    for line in text.lines() {
        let mut it = line.split_whitespace();
        match it.next() {
            Some("facet") => {
                // « facet normal nx ny nz »
                let _ = it.next(); // "normal"
                normal = [
                    it.next().and_then(|s| s.parse().ok()).unwrap_or(0.0),
                    it.next().and_then(|s| s.parse().ok()).unwrap_or(0.0),
                    it.next().and_then(|s| s.parse().ok()).unwrap_or(0.0),
                ];
                verts_in_facet = 0;
            }
            Some("vertex") => {
                let x = it.next().and_then(|s| s.parse().ok());
                let y = it.next().and_then(|s| s.parse().ok());
                let z = it.next().and_then(|s| s.parse().ok());
                match (x, y, z) {
                    (Some(x), Some(y), Some(z)) => {
                        mesh.positions.extend_from_slice(&[x, y, z]);
                        mesh.normals.extend_from_slice(&normal);
                        let idx = (mesh.positions.len() / 3 - 1) as u32;
                        mesh.indices.push(idx);
                        verts_in_facet += 1;
                    }
                    _ => return Err(MeshError::InvalidStl("sommet illisible")),
                }
            }
            Some("endfacet") if verts_in_facet != 3 => {
                return Err(MeshError::InvalidStl("facette non triangulaire"));
            }
            _ => {}
        }
    }
    if mesh.indices.is_empty() {
        return Err(MeshError::InvalidStl("aucune facette"));
    }
    Ok(mesh)
}

fn read_u32(bytes: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([bytes[off], bytes[off + 1], bytes[off + 2], bytes[off + 3]])
}

fn read_f32(bytes: &[u8], off: usize) -> f32 {
    f32::from_le_bytes([bytes[off], bytes[off + 1], bytes[off + 2], bytes[off + 3]])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tri_mesh() -> Mesh {
        Mesh {
            positions: vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0],
            normals: vec![0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0],
            indices: vec![0, 1, 2],
        }
    }

    #[test]
    fn encode_decode_round_trip() {
        let mesh = tri_mesh();
        let bytes = mesh.encode();
        assert_eq!(&bytes[0..4], MAGIC);
        assert_eq!(bytes.len(), 16 + 9 * 4 + 9 * 4 + 3 * 4);
        assert_eq!(Mesh::decode(&bytes).unwrap(), mesh);
    }

    #[test]
    fn decode_rejects_bad_header_and_truncation() {
        assert_eq!(Mesh::decode(b"xxxx").unwrap_err(), MeshError::BadHeader);
        let mut bytes = tri_mesh().encode();
        bytes.pop();
        assert_eq!(Mesh::decode(&bytes).unwrap_err(), MeshError::Truncated);
    }

    fn binary_stl_one_triangle() -> Vec<u8> {
        let mut v = vec![0u8; 80];
        v.extend_from_slice(&1u32.to_le_bytes());
        // normale
        for f in [0.0f32, 0.0, 1.0] {
            v.extend_from_slice(&f.to_le_bytes());
        }
        // 3 sommets
        for f in [0.0f32, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0] {
            v.extend_from_slice(&f.to_le_bytes());
        }
        v.extend_from_slice(&0u16.to_le_bytes()); // attribute byte count
        v
    }

    #[test]
    fn parses_binary_stl() {
        let mesh = parse_stl(&binary_stl_one_triangle()).unwrap();
        assert_eq!(mesh.vertex_count(), 3);
        assert_eq!(mesh.indices, vec![0, 1, 2]);
        assert_eq!(&mesh.positions[3..6], &[1.0, 0.0, 0.0]);
        assert_eq!(&mesh.normals[0..3], &[0.0, 0.0, 1.0]);
    }

    #[test]
    fn parses_ascii_stl() {
        let ascii = "solid s\n\
            facet normal 0 0 1\n outer loop\n\
            vertex 0 0 0\n vertex 1 0 0\n vertex 0 1 0\n\
            endloop\n endfacet\nendsolid s\n";
        let mesh = parse_stl(ascii.as_bytes()).unwrap();
        assert_eq!(mesh.vertex_count(), 3);
        assert_eq!(mesh.indices, vec![0, 1, 2]);
        assert_eq!(&mesh.normals[6..9], &[0.0, 0.0, 1.0]);
    }

    #[test]
    fn ascii_stl_requires_facets() {
        assert!(parse_stl(b"solid empty\nendsolid empty\n").is_err());
    }
}
