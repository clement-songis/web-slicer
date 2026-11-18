//! Découpe complète d'Orca (lot 26) : GLGizmoCut3D::perform_cut → `Slic3r::Cut`
//! (CutUtils.cpp) via `run_mesh_op_pieces`. Exécution :
//! cargo test -p engine --features ffi --test ffi_cut (devShell requis).
//!
//! Parité géométrique : nombre d'objets et de volumes, rôles Orca (pièce /
//! négatif, connecteur), volumes analytiques des connecteurs (rayon/hauteur et
//! tolérances d'`apply_tolerance`), et identité avec l'ancien chemin
//! `cut_mesh` pour une coupe plane sans option.

#![cfg(feature = "ffi")]

use std::f64::consts::PI;

use engine::adapters::ffi::{run_mesh_op, run_mesh_op_pieces};
use engine::api::split::{
    signed_volume, CutConnector, CutConnectorShape, CutConnectorStyle, CutConnectorType, CutGroove,
    CutOptions, CutPlane, MeshOp, MeshOpPiece,
};
use engine::api::TriangleMesh;

/// Cube de côté `s` centré à l'origine (repère recentré du client).
fn cube(s: f32) -> TriangleMesh {
    let h = s / 2.0;
    let vertices = vec![
        [-h, -h, -h],
        [h, -h, -h],
        [h, h, -h],
        [-h, h, -h],
        [-h, -h, h],
        [h, -h, h],
        [h, h, h],
        [-h, h, h],
    ];
    let indices = vec![
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
    ];
    TriangleMesh { vertices, indices }
}

fn cut(options: CutOptions, keep: (bool, bool)) -> Vec<MeshOpPiece> {
    let op = MeshOp::Cut {
        plane: CutPlane::horizontal(0.0),
        keep_upper: keep.0,
        keep_lower: keep.1,
        options,
    };
    run_mesh_op_pieces(&op, &[cube(20.0)]).expect("coupe")
}

fn connector(kind: CutConnectorType, style: CutConnectorStyle) -> CutConnector {
    CutConnector {
        pos: [0.0, 0.0, 0.0],
        kind,
        style,
        shape: CutConnectorShape::Circle,
        size_mm: 5.0,
        depth_mm: 4.0,
        size_tolerance_mm: 0.0,
        depth_tolerance_mm: 0.1,
        rotation_deg: 0.0,
    }
}

fn z_range(mesh: &TriangleMesh) -> (f32, f32) {
    let (lo, hi) = mesh.bounding_box().unwrap();
    (lo[2], hi[2])
}

fn summary(pieces: &[MeshOpPiece]) -> Vec<(String, String, bool)> {
    pieces
        .iter()
        .map(|p| match &p.volume {
            None => (p.label.clone(), "body".into(), false),
            Some(v) => (p.label.clone(), v.role.clone(), v.connector),
        })
        .collect()
}

/// Cylindre à 360 secteurs (`its_make_cylinder(1, 1, 2π/360)` mis à l'échelle).
fn prism_volume(r: f64, h: f64) -> f64 {
    0.5 * 360.0 * r * r * (2.0 * PI / 360.0).sin() * h
}

#[test]
fn plain_rotation_cut_matches_legacy_cut_mesh() {
    let pieces = cut(
        CutOptions {
            rotation_deg: Some([0.0, 0.0, 0.0]),
            ..CutOptions::default()
        },
        (true, true),
    );
    assert_eq!(summary(&pieces).len(), 2);
    let legacy = run_mesh_op(
        &MeshOp::Cut {
            plane: CutPlane::horizontal(0.0),
            keep_upper: true,
            keep_lower: true,
            options: CutOptions::default(),
        },
        &[cube(20.0)],
    )
    .unwrap();
    for (piece, (label, mesh)) in pieces.iter().zip(&legacy) {
        assert_eq!(&piece.label, label);
        assert_eq!(piece.mesh.triangle_count(), mesh.triangle_count());
        assert!((signed_volume(&piece.mesh) - signed_volume(mesh)).abs() < 1e-3);
        assert!((signed_volume(&piece.mesh) - 4000.0).abs() < 1e-2);
    }
    assert_eq!(z_range(&pieces[0].mesh), (0.0, 10.0));
    assert_eq!(z_range(&pieces[1].mesh), (-10.0, 0.0));
}

#[test]
fn inclined_plane_splits_volume_through_center() {
    let pieces = cut(
        CutOptions {
            rotation_deg: Some([30.0, 0.0, 0.0]),
            ..CutOptions::default()
        },
        (true, true),
    );
    assert_eq!(pieces.len(), 2);
    for p in &pieces {
        assert!(
            (signed_volume(&p.mesh) - 4000.0).abs() < 0.05,
            "{}",
            signed_volume(&p.mesh)
        );
    }
    // Plan incliné : la moitié haute descend sous z = 0 (côté −Y).
    assert!(z_range(&pieces[0].mesh).0 < -1.0);
}

#[test]
fn plug_connector_makes_hole_and_plug_like_orca() {
    let pieces = cut(
        CutOptions {
            connectors: vec![connector(CutConnectorType::Plug, CutConnectorStyle::Prism)],
            ..CutOptions::default()
        },
        // Orca force les deux moitiés avec connecteurs.
        (true, false),
    );
    assert_eq!(
        summary(&pieces),
        vec![
            ("upper".into(), "body".into(), false),
            ("upper".into(), "negative".into(), true),
            ("lower".into(), "body".into(), false),
            ("lower".into(), "part".into(), true),
        ]
    );
    let name = &pieces[1].volume.as_ref().unwrap().name;
    assert_eq!(name, "Connector-1");
    // Trou : hauteur + tolérance de profondeur (apply_tolerance), tenon : nominal.
    let hole = signed_volume(&pieces[1].mesh);
    let plug = signed_volume(&pieces[3].mesh);
    assert!((hole - prism_volume(2.5, 4.1)).abs() < 0.05, "trou {hole}");
    assert!((plug - prism_volume(2.5, 4.0)).abs() < 0.05, "tenon {plug}");
    // Tenon posé au-dessus du plan (pos + normale × h/2) : z ∈ [0, 4].
    let (lo, hi) = z_range(&pieces[3].mesh);
    assert!(lo.abs() < 1e-4 && (hi - 4.0).abs() < 1e-4, "{lo} {hi}");
}

#[test]
fn dowel_creates_separate_object_and_two_holes() {
    let pieces = cut(
        CutOptions {
            connectors: vec![connector(CutConnectorType::Dowel, CutConnectorStyle::Prism)],
            ..CutOptions::default()
        },
        (true, true),
    );
    let s = summary(&pieces);
    assert_eq!(
        s,
        vec![
            ("upper".into(), "body".into(), false),
            ("upper".into(), "negative".into(), true),
            ("lower".into(), "body".into(), false),
            ("lower".into(), "negative".into(), true),
            ("dowel_1".into(), "body".into(), false),
        ]
    );
    // Goujon prisme : hauteur doublée (apply_connectors_in_model), sans tolérance.
    let dowel = signed_volume(&pieces[4].mesh);
    assert!(
        (dowel - prism_volume(2.5, 8.0)).abs() < 0.05,
        "goujon {dowel}"
    );
    assert_eq!(pieces[1].volume.as_ref().unwrap().name, "Connector-1_A");
    assert_eq!(pieces[3].volume.as_ref().unwrap().name, "Connector-1_B");
}

#[test]
fn snap_connector_uses_cylinder_hole_and_snap_part() {
    let pieces = cut(
        CutOptions {
            connectors: vec![connector(CutConnectorType::Snap, CutConnectorStyle::Prism)],
            ..CutOptions::default()
        },
        (true, true),
    );
    let s = summary(&pieces);
    assert_eq!(s[1], ("upper".into(), "negative".into(), true));
    assert_eq!(s[3], ("lower".into(), "part".into(), true));
    // Trou du clip = cylindre (its_make_cylinder(1, 1, π/180)) + tolérance.
    let hole = signed_volume(&pieces[1].mesh);
    assert!((hole - prism_volume(2.5, 4.1)).abs() < 0.05, "trou {hole}");
    // Clip fendu : moins de matière que le cylindre plein.
    assert!(signed_volume(&pieces[3].mesh) < prism_volume(2.5, 4.0));
}

#[test]
fn square_frustum_plug_has_four_sides() {
    let mut c = connector(CutConnectorType::Plug, CutConnectorStyle::Frustum);
    c.shape = CutConnectorShape::Square;
    let pieces = cut(
        CutOptions {
            connectors: vec![c],
            ..CutOptions::default()
        },
        (true, true),
    );
    let plug = &pieces[3].mesh;
    // its_make_frustum(1, 1, 2π/4) : 4 secteurs, triangles < cylindre circulaire.
    assert!(plug.triangle_count() <= 16, "{}", plug.triangle_count());
    assert!(signed_volume(plug) > 0.0);
}

#[test]
fn groove_cut_produces_two_interlocking_objects() {
    let pieces = cut(
        CutOptions {
            groove: Some(CutGroove {
                depth_mm: 2.0,
                width_mm: None,
                flaps_angle_deg: 60.0,
                angle_deg: 0.0,
                depth_tolerance_mm: 0.1,
                width_tolerance_mm: 0.1,
                count: 1,
                gap_mm: 10.0,
            }),
            ..CutOptions::default()
        },
        (true, true),
    );
    assert_eq!(
        summary(&pieces),
        vec![
            ("upper".into(), "body".into(), false),
            ("lower".into(), "body".into(), false),
        ]
    );
    let (u, l) = (
        signed_volume(&pieces[0].mesh),
        signed_volume(&pieces[1].mesh),
    );
    // Tolérances retirées de la queue : un peu moins que le cube entier.
    assert!(u + l < 8000.0 && u + l > 7900.0, "{u} + {l}");
    // Queue d'aronde : les pièces débordent de part et d'autre du plan médian.
    assert!(z_range(&pieces[0].mesh).0 < -0.5 && z_range(&pieces[1].mesh).1 > 0.5);
}

#[test]
fn keep_as_parts_gives_one_object_with_two_parts() {
    let pieces = cut(
        CutOptions {
            keep_as_parts: true,
            ..CutOptions::default()
        },
        (true, false),
    );
    assert_eq!(
        summary(&pieces),
        vec![
            ("parts".into(), "body".into(), false),
            ("parts".into(), "part".into(), false),
        ]
    );
    assert_eq!(pieces[1].volume.as_ref().unwrap().name, "object_B");
}

#[test]
fn flip_and_place_on_cut_rotate_pieces_like_post_process() {
    let pieces = cut(
        CutOptions {
            flip_upper: true,
            place_on_cut_lower: true,
            ..CutOptions::default()
        },
        (true, true),
    );
    assert_eq!(pieces.len(), 2);
    // Retourner : rotation de π autour de X ; placer sur la coupe (bas) :
    // retourné aussi (Cut::post_process), face coupée vers le bas.
    let close =
        |(lo, hi): (f32, f32), (a, b): (f32, f32)| (lo - a).abs() < 1e-4 && (hi - b).abs() < 1e-4;
    assert!(
        close(z_range(&pieces[0].mesh), (-10.0, 0.0)),
        "{:?}",
        z_range(&pieces[0].mesh)
    );
    assert!(
        close(z_range(&pieces[1].mesh), (0.0, 10.0)),
        "{:?}",
        z_range(&pieces[1].mesh)
    );

    // Plan incliné à 45° : la face coupée devient horizontale (en bas).
    let pieces = cut(
        CutOptions {
            rotation_deg: Some([45.0, 0.0, 0.0]),
            place_on_cut_upper: true,
            ..CutOptions::default()
        },
        (true, false),
    );
    let mesh = &pieces[0].mesh;
    let (lo, _) = z_range(mesh);
    let on_floor = mesh
        .vertices
        .iter()
        .filter(|v| (v[2] - lo).abs() < 1e-4)
        .count();
    assert!(
        on_floor >= 4,
        "face coupée non posée : {on_floor} sommets à z min"
    );
}

#[test]
fn plane_outside_object_is_refused() {
    let op = MeshOp::Cut {
        plane: CutPlane::horizontal(50.0),
        keep_upper: true,
        keep_lower: true,
        options: CutOptions {
            rotation_deg: Some([0.0; 3]),
            ..CutOptions::default()
        },
    };
    let err = run_mesh_op_pieces(&op, &[cube(20.0)]).unwrap_err();
    assert!(err.to_string().contains("ne traverse pas"), "{err}");
}
