//! Lot 37 — informations de coupe et erreurs de maillage à travers libslic3r :
//! `ModelObject::cut_id` / `ModelVolume::cut_info` écrits par store_bbs_3mf
//! dans `Metadata/cut_information.xml` (format exact de
//! `_BBS_3MF_Exporter::_add_cut_information_file_to_archive`) et relus par
//! load_bbs_3mf ; erreurs réparées (`<mesh_stat>`) conservées ; statistiques
//! d'erreurs d'un STL lu comme Orca (`TriangleMesh::stats()`).
//! Exécution : cargo test -p engine --features ffi --test ffi_cut_info.

#![cfg(feature = "ffi")]

use std::io::Read;

use engine::api::{
    CutObjectId, DynamicPrintConfig, MeshErrorStats, Model, RepairedMeshErrors, TriangleMesh,
    VolumeCutInfo, VolumeRole,
};
use glam::DMat4;

fn cube(size: f32, center: [f32; 3]) -> TriangleMesh {
    let h = size / 2.0;
    let [cx, cy, cz] = center;
    TriangleMesh {
        vertices: vec![
            [cx - h, cy - h, cz - h],
            [cx + h, cy - h, cz - h],
            [cx + h, cy + h, cz - h],
            [cx - h, cy + h, cz - h],
            [cx - h, cy - h, cz + h],
            [cx + h, cy - h, cz + h],
            [cx + h, cy + h, cz + h],
            [cx - h, cy + h, cz + h],
        ],
        indices: vec![
            [0, 2, 1],
            [0, 3, 2],
            [4, 5, 6],
            [4, 6, 7],
            [0, 1, 5],
            [0, 5, 4],
            [1, 2, 6],
            [1, 6, 5],
            [2, 3, 7],
            [2, 7, 6],
            [3, 0, 4],
            [3, 4, 7],
        ],
    }
}

fn zip_entry(path: &std::path::Path, name: &str) -> Option<String> {
    let mut archive = zip::ZipArchive::new(std::fs::File::open(path).unwrap()).unwrap();
    let mut file = archive.by_name(name).ok()?;
    let mut text = String::new();
    file.read_to_string(&mut text).unwrap();
    Some(text)
}

/// Deux pièces liées (même cut_id) dont l'une porte un connecteur négatif, et un
/// objet ordinaire : cut_information.xml ne liste que les pièces, ids 1-based
/// dans l'ordre des objets, `volume_id` = indice du volume dans l'objet.
#[test]
fn cut_information_is_written_and_read_like_orca() {
    let cut = CutObjectId {
        id: 7,
        check_sum: 3,
        connectors_cnt: 1,
    };
    let mut model = Model::default();
    let upper = model.add_object("cube_A", cube(20.0, [0.0, 0.0, 5.0]));
    upper.cut_id = Some(cut);
    let mut hole = upper.volumes[0].clone();
    hole.name = "Connector-1".into();
    hole.mesh = cube(4.0, [0.0, 0.0, 0.0]);
    hole.role = VolumeRole::NegativeVolume;
    hole.matrix = DMat4::from_translation(glam::DVec3::new(0.0, 0.0, -3.0));
    hole.cut_info = Some(VolumeCutInfo {
        connector_type: 1,
        radius_tolerance: 0.25,
        height_tolerance: 0.1,
    });
    upper.volumes.push(hole);
    model.add_object("plain", cube(10.0, [40.0, 0.0, 5.0]));
    model
        .add_object("cube_B", cube(20.0, [0.0, 40.0, 5.0]))
        .cut_id = Some(cut);

    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("cut.3mf");
    engine::adapters::ffi::write_project_3mf_with_plates(
        &model,
        &DynamicPrintConfig::new(),
        &[],
        &out,
    )
    .unwrap();
    let xml = zip_entry(&out, "Metadata/cut_information.xml").expect("cut_information.xml");
    let compact: String = xml.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        compact.contains(r#"<object id="1"> <cut_id id="7" check_sum="3" connectors_cnt="1"/>"#),
        "{xml}"
    );
    assert!(
        compact.contains(
            r#"<connector volume_id="1" type="1" r_tolerance="0.25" h_tolerance="0.1"/>"#
        ) || compact.contains(
            r#"<connector volume_id="1" type="1" r_tolerance="0.25" h_tolerance="0.100000001"/>"#
        ),
        "{xml}"
    );
    assert!(compact.contains(r#"<object id="3">"#), "{xml}");
    assert!(
        !compact.contains(r#"<object id="2">"#),
        "objet non coupé listé : {xml}"
    );

    let (read, _) = engine::adapters::ffi::read_project_3mf(&out).unwrap();
    assert_eq!(read.objects.len(), 3);
    assert_eq!(read.objects[0].cut_id, Some(cut));
    assert_eq!(read.objects[1].cut_id, None);
    assert_eq!(read.objects[2].cut_id, Some(cut));
    let connector = read.objects[0]
        .volumes
        .iter()
        .find(|v| v.name == "Connector-1")
        .expect("volume connecteur");
    let info = connector.cut_info.expect("cut_info relu");
    assert_eq!(info.connector_type, 1);
    assert!((info.radius_tolerance - 0.25).abs() < 1e-6);
    assert!((info.height_tolerance - 0.1).abs() < 1e-6);
    assert!(read.objects[0].volumes[0].cut_info.is_none());
}

/// Erreurs réparées d'un volume → `<mesh_stat>` (model_settings.config) →
/// relues avec les arêtes ouvertes recalculées (`TriangleMesh(its, stats)`).
#[test]
fn repaired_errors_survive_the_3mf_round_trip() {
    let mut model = Model::default();
    let object = model.add_object("stats", cube(10.0, [0.0, 0.0, 5.0]));
    object.volumes[0].mesh_stats = Some(MeshErrorStats {
        open_edges: 0,
        repaired: RepairedMeshErrors {
            edges_fixed: 3,
            degenerate_facets: 1,
            facets_removed: 2,
            facets_reversed: 4,
            backwards_edges: 5,
        },
    });
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("stats.3mf");
    engine::adapters::ffi::write_project_3mf_with_plates(
        &model,
        &DynamicPrintConfig::new(),
        &[],
        &out,
    )
    .unwrap();
    let settings = zip_entry(&out, "Metadata/model_settings.config").unwrap();
    assert!(
        settings.contains(r#"edges_fixed="3" degenerate_facets="1" facets_removed="2" facets_reversed="4" backwards_edges="5""#),
        "{settings}"
    );
    let (read, _) = engine::adapters::ffi::read_project_3mf(&out).unwrap();
    let stats = read.objects[0].volumes[0].mesh_stats.clone().unwrap();
    assert_eq!(
        stats.repaired,
        model.objects[0].volumes[0]
            .mesh_stats
            .clone()
            .unwrap()
            .repaired
    );
    assert_eq!(stats.open_edges, 0);
    assert_eq!(stats.repaired_errors_count(), 15);
}

fn binary_stl(triangles: &[[[f32; 3]; 3]]) -> Vec<u8> {
    let mut v = vec![0u8; 80];
    v.extend_from_slice(&(triangles.len() as u32).to_le_bytes());
    for t in triangles {
        v.extend_from_slice(&[0u8; 12]);
        for p in t {
            for c in p {
                v.extend_from_slice(&c.to_le_bytes());
            }
        }
        v.extend_from_slice(&0u16.to_le_bytes());
    }
    v
}

fn cube_triangles() -> Vec<[[f32; 3]; 3]> {
    let mesh = cube(10.0, [0.0, 0.0, 5.0]);
    mesh.indices
        .iter()
        .map(|t| t.map(|i| mesh.vertices[i as usize]))
        .collect()
}

/// STL lu comme Orca (Model::read_from_file → ReadSTLFile(repair=true)) : la
/// réparation à l'import corrige la facette à l'envers et retire la facette
/// dégénérée, mais Orca 2.4.1 ne reporte PAS ces compteurs dans les stats
/// (`TriangleMesh::from_stl` : bloc `repaired_errors = …` sous `#if 0`) —
/// donc aucune alerte « erreurs réparées » pour un STL, seulement les arêtes
/// ouvertes restantes (`fill_initial_stats` → its_num_open_edges).
#[test]
fn stl_mesh_stats_come_from_libslic3r() {
    let dir = tempfile::tempdir().unwrap();
    let load = |name: &str, triangles: &[[[f32; 3]; 3]]| {
        let path = dir.path().join(name);
        std::fs::write(&path, binary_stl(triangles)).unwrap();
        let model = engine::adapters::ffi::load_model(&path).unwrap();
        let mut total = MeshErrorStats::default();
        for v in model.objects.iter().flat_map(|o| &o.volumes) {
            total.merge(v.mesh_stats.as_ref().expect("stats renseignées"));
        }
        total
    };

    let clean = load("clean.stl", &cube_triangles());
    assert!(clean.manifold() && !clean.repaired(), "{clean:?}");

    let mut flipped = cube_triangles();
    flipped[3].swap(1, 2);
    let p = flipped[0][0];
    flipped.push([p, p, flipped[0][1]]);
    let fixed = load("flipped.stl", &flipped);
    assert!(fixed.manifold(), "{fixed:?}");
    assert!(
        !fixed.repaired(),
        "Orca 2.4.1 n'expose pas les réparations d'un STL : {fixed:?}"
    );

    let mut open = cube_triangles();
    open.pop();
    let holed = load("open.stl", &open);
    assert!(!holed.manifold(), "{holed:?}");
    assert_eq!(holed.open_edges, 3, "{holed:?}");
}
