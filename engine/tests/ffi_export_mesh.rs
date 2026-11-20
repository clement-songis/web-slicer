//! Maillages d'export STL/DRC calculés par libslic3r (export_mesh.cpp), comme
//! `Plater::export_stl` d'Orca : pièces positives fusionnées, volumes négatifs
//! soustraits (booléens CGAL), toutes les instances en repère plateau.
//! Exécution : cargo test -p engine --features ffi --test ffi_export_mesh.

#![cfg(feature = "ffi")]

use engine::adapters::ffi::export_meshes;
use engine::api::{
    DynamicPrintConfig, Model, ModelInstance, ModelObject, ModelVolume, TriangleMesh, VolumePaint,
    VolumeRole,
};
use glam::{DMat4, DVec3};

fn cube(size: f32) -> TriangleMesh {
    let s = size;
    TriangleMesh {
        vertices: vec![
            [0.0, 0.0, 0.0],
            [s, 0.0, 0.0],
            [s, s, 0.0],
            [0.0, s, 0.0],
            [0.0, 0.0, s],
            [s, 0.0, s],
            [s, s, s],
            [0.0, s, s],
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

fn volume(mesh: TriangleMesh, matrix: DMat4, role: VolumeRole) -> ModelVolume {
    ModelVolume {
        name: "v".into(),
        mesh,
        matrix,
        role,
        extruder: None,
        paint: VolumePaint::default(),
        settings: DynamicPrintConfig::new(),
        emboss: None,
        cut_info: None,
        mesh_stats: None,
    }
}

fn instance(offset: DVec3) -> ModelInstance {
    ModelInstance {
        matrix: DMat4::from_translation(offset),
        plate_index: 0,
        printable: true,
    }
}

fn object(volumes: Vec<ModelVolume>, instances: Vec<ModelInstance>) -> ModelObject {
    ModelObject {
        name: "o".into(),
        volumes,
        instances,
        settings: DynamicPrintConfig::new(),
        layer_height_profile: Vec::new(),
        layer_config_ranges: Vec::new(),
        cut_id: None,
    }
}

fn signed_volume(m: &TriangleMesh) -> f64 {
    m.indices
        .iter()
        .map(|&[a, b, c]| {
            let p = |i: u32| DVec3::from(m.vertices[i as usize].map(f64::from));
            p(a).dot(p(b).cross(p(c))) / 6.0
        })
        .sum()
}

fn bbox(m: &TriangleMesh) -> ([f32; 3], [f32; 3]) {
    let mut lo = [f32::MAX; 3];
    let mut hi = [f32::MIN; 3];
    for v in &m.vertices {
        for k in 0..3 {
            lo[k] = lo[k].min(v[k]);
            hi[k] = hi[k].max(v[k]);
        }
    }
    (lo, hi)
}

#[test]
fn instances_are_merged_in_bed_frame() {
    let model = Model {
        objects: vec![object(
            vec![volume(cube(20.0), DMat4::IDENTITY, VolumeRole::ModelPart)],
            vec![
                instance(DVec3::new(100.0, 100.0, 0.0)),
                instance(DVec3::new(140.0, 100.0, 0.0)),
            ],
        )],
        custom_gcodes: Vec::new(),
    };
    let meshes = export_meshes(&model).expect("export");
    assert_eq!(meshes.len(), 1);
    assert_eq!(meshes[0].indices.len(), 24);
    assert!((signed_volume(&meshes[0]) - 16000.0).abs() < 1e-3);
    let (lo, hi) = bbox(&meshes[0]);
    assert_eq!(lo, [100.0, 100.0, 0.0]);
    assert_eq!(hi, [160.0, 120.0, 20.0]);
}

#[test]
fn negative_volume_is_subtracted_and_modifiers_ignored() {
    let model = Model {
        objects: vec![object(
            vec![
                volume(cube(20.0), DMat4::IDENTITY, VolumeRole::ModelPart),
                // Trou débouchant par le haut : 10 × 10 × 10 retirés.
                volume(
                    cube(10.0),
                    DMat4::from_translation(DVec3::new(5.0, 5.0, 10.0))
                        * DMat4::from_scale(DVec3::new(1.0, 1.0, 1.5)),
                    VolumeRole::NegativeVolume,
                ),
                volume(cube(50.0), DMat4::IDENTITY, VolumeRole::ParameterModifier),
            ],
            vec![instance(DVec3::ZERO)],
        )],
        custom_gcodes: Vec::new(),
    };
    let meshes = export_meshes(&model).expect("export");
    assert_eq!(meshes.len(), 1);
    let v = signed_volume(&meshes[0]);
    assert!((v - 7000.0).abs() < 1e-2, "volume {v}");
    assert_eq!(bbox(&meshes[0]), ([0.0, 0.0, 0.0], [20.0, 20.0, 20.0]));
}
