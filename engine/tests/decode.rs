//! Décodage moteur → maillage d'affichage WSMh (T120) : `convert_to_mesh`
//! (FFI `Slic3r::Model::read_from_file`) sur STL/OBJ/3MF, puis `encode_display`
//! produit un tampon WSMh valide et round-trip. Source de vérité unique du
//! maillage d'affichage — remplace les parseurs regex client (retournement R7).
//!
//! Exécution : `cargo test -p engine --features ffi` (devShell requis).

#![cfg(feature = "ffi")]

mod common;

use engine::adapters::ffi::convert_to_mesh;
use engine::api::DisplayMesh;

/// Chaque format supporté se décode en un maillage non vide, encodable en WSMh
/// valide (en-tête, comptes cohérents) et décodable en retour.
#[test]
fn decodes_all_formats_to_valid_display_mesh() {
    for name in ["cube20.stl", "cube20.obj", "orca_project.3mf"] {
        let mesh = convert_to_mesh(&common::fixture(name))
            .unwrap_or_else(|e| panic!("{name} : conversion moteur — {e}"));
        assert!(mesh.triangle_count() > 0, "{name} : triangles > 0");

        let bytes = mesh.encode_display();
        assert_eq!(&bytes[0..4], b"WSMh", "{name} : magic WSMh");

        let decoded =
            DisplayMesh::decode(&bytes).unwrap_or_else(|e| panic!("{name} : WSMh invalide — {e}"));
        // Positions = 3 f32 par sommet ; indices = 3 par triangle.
        assert_eq!(
            decoded.positions.len(),
            mesh.vertices.len() * 3,
            "{name} : positions"
        );
        assert_eq!(
            decoded.normals.len(),
            decoded.positions.len(),
            "{name} : normales"
        );
        assert_eq!(
            decoded.indices.len(),
            mesh.triangle_count() * 3,
            "{name} : indices"
        );
    }
}

/// Le cube (STL comme OBJ) fait 12 triangles : la géométrie survit au décodage.
#[test]
fn cube_stl_and_obj_agree_on_triangle_count() {
    let stl = convert_to_mesh(&common::fixture("cube20.stl")).expect("cube STL");
    let obj = convert_to_mesh(&common::fixture("cube20.obj")).expect("cube OBJ");
    assert_eq!(stl.triangle_count(), 12);
    assert_eq!(obj.triangle_count(), 12);
}
