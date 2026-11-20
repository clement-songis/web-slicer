//! Peinture moteur (TriangleSelector d'Orca, paint.cpp) : un trait donné doit
//! produire EXACTEMENT les chaînes 3MF d'Orca. Référence : programme C++ isolé
//! lié à libslic3r sans le bridge (tests/fixtures/paint/paint_ref.cpp, sortie
//! circle.txt / sphere.txt, voir README du même dossier).
//! Exécution : cargo test -p engine --features ffi --test ffi_paint (devShell requis).

#![cfg(feature = "ffi")]

use std::path::Path;

use engine::adapters::ffi::serve::Server;
use engine::adapters::ffi::PaintSession;
use engine::api::paint::{CursorShape, PaintFacets, PaintStroke, PaintTool, ServeOp, ServeRequest};
use engine::api::{PaintChannel, TriangleMesh};

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

/// Trait de la référence : 3 points sur la face du dessus, rayon 3, enforcer.
fn stroke(cursor: CursorShape) -> PaintStroke {
    PaintStroke {
        points: vec![[5.0, 5.0, 20.0], [12.0, 8.0, 20.0], [8.0, 14.0, 20.0]],
        facets: vec![2, 2, 3],
        camera_pos: [10.0, 10.0, 100.0],
        radius: 3.0,
        cursor,
        state: 1,
        trafo: glam::DMat4::IDENTITY.to_cols_array(),
        splitting: true,
        highlight_deg: 0.0,
        tool: PaintTool::Brush,
        fill_angle: 30.0,
        height: 0.2,
        z_world: None,
    }
}

fn reference(name: &str) -> PaintFacets {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/paint")
        .join(name);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()))
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
        .map(|l| {
            let (t, hex) = l.split_once(' ').expect("<triangle> <hex>");
            (t.parse().unwrap(), hex.to_string())
        })
        .collect()
}

fn paint(cursor: CursorShape) -> PaintFacets {
    let mut session = PaintSession::new(&cube(20.0)).expect("session");
    session
        .stroke(PaintChannel::Supports, &stroke(cursor))
        .expect("stroke");
    session.export(PaintChannel::Supports).expect("export")
}

#[test]
fn circle_stroke_matches_isolated_orca_reference() {
    let got = paint(CursorShape::Circle);
    let want = reference("circle.txt");
    assert!(!want.is_empty());
    assert_eq!(got, want);
    // Subdivision réelle : plus long qu'une facette entière ("4").
    assert!(got.values().any(|h| h.len() > 1), "{got:?}");
    eprintln!(
        "circle: {} triangles, {} hex chars",
        got.len(),
        got.values().map(String::len).sum::<usize>()
    );
}

#[test]
fn sphere_stroke_matches_isolated_orca_reference() {
    assert_eq!(paint(CursorShape::Sphere), reference("sphere.txt"));
}

#[test]
fn load_export_round_trip_is_identity() {
    let painted = paint(CursorShape::Circle);
    let mut session = PaintSession::new(&cube(20.0)).unwrap();
    session.load(PaintChannel::Supports, &painted).unwrap();
    assert_eq!(session.export(PaintChannel::Supports).unwrap(), painted);
    // Canaux indépendants.
    assert!(session.export(PaintChannel::Seam).unwrap().is_empty());
}

#[test]
fn serve_session_paints_and_reloads_client_state() {
    let mut server = Server::new();
    let open = server.handle(ServeRequest {
        id: 1,
        op: ServeOp::PaintOpen {
            session: "p/o".into(),
            mesh: cube(20.0),
        },
    });
    assert!(open.ok, "{open:?}");
    assert_eq!(open.triangles, Some(12));
    let r = server.handle(ServeRequest {
        id: 2,
        op: ServeOp::PaintStroke {
            session: "p/o".into(),
            channel: PaintChannel::Supports,
            facets: Some(PaintFacets::new()),
            stroke: stroke(CursorShape::Circle),
        },
    });
    assert!(r.ok, "{r:?}");
    assert_eq!(r.facets.unwrap(), reference("circle.txt"));
    let missing = server.handle(ServeRequest {
        id: 3,
        op: ServeOp::PaintStroke {
            session: "absent".into(),
            channel: PaintChannel::Supports,
            facets: None,
            stroke: stroke(CursorShape::Circle),
        },
    });
    assert!(!missing.ok);
}

#[test]
fn serve_loop_speaks_json_lines() {
    let mesh = serde_json::to_string(&cube(20.0)).unwrap();
    let input = format!(
        "{{\"id\":1,\"op\":\"ping\"}}\npas du json\n{{\"id\":2,\"op\":\"paint_open\",\"session\":\"s\",\"mesh\":{mesh}}}\n"
    );
    let mut out = Vec::new();
    engine::adapters::ffi::serve::serve(input.as_bytes(), &mut out).unwrap();
    let lines: Vec<_> = String::from_utf8(out)
        .unwrap()
        .lines()
        .map(str::to_string)
        .collect();
    assert_eq!(lines.len(), 3);
    assert!(lines[0].starts_with("R {\"id\":1,\"ok\":true"));
    assert!(lines[1].contains("\"ok\":false"));
    assert!(lines[2].contains("\"triangles\":12"));
}

/// e2e moteur : peinture TriangleSelector → écriture 3MF Orca (store_bbs_3mf)
/// → l'attribut `paint_supports`/`slic3rpe:custom_supports` du triangle est la chaîne
/// subdivisée (bien plus longue qu'une facette entière « 4 »), et la relecture
/// du 3MF par le moteur rend exactement la même peinture.
#[test]
fn painted_facets_survive_3mf_export_subdivided() {
    use engine::api::{DynamicPrintConfig, Model};
    let painted = paint(CursorShape::Circle);
    let mut model = Model::default();
    let object = model.add_object("cube", cube(20.0));
    object.volumes[0].paint.facets = painted
        .iter()
        .map(|(t, h)| (PaintChannel::Supports, *t, h.clone()))
        .collect();
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("painted.3mf");
    engine::adapters::ffi::write_project_3mf(&model, &DynamicPrintConfig::new(), &out).unwrap();

    let file = std::fs::File::open(&out).unwrap();
    let mut zip = zip::ZipArchive::new(file).unwrap();
    let mut xml = String::new();
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).unwrap();
        if entry.name().ends_with(".model") {
            use std::io::Read;
            let mut s = String::new();
            entry.read_to_string(&mut s).unwrap();
            xml.push_str(&s);
        }
    }
    let longest = painted.values().map(String::len).max().unwrap();
    let needle = &painted[&3];
    assert!(
        xml.contains(needle.as_str()),
        "chaîne subdivisée absente du 3MF"
    );
    assert!(longest > 100, "{longest}");
    eprintln!("3MF: attribut de peinture le plus long = {longest} caractères");

    let (back, _) = engine::adapters::ffi::read_project_3mf(&out).unwrap();
    let facets: PaintFacets = back.objects[0].volumes[0]
        .paint
        .facets
        .iter()
        .filter(|f| f.0 == PaintChannel::Supports)
        .map(|f| (f.1, f.2.clone()))
        .collect();
    assert_eq!(facets, painted);
}

/// Remplissages Orca (smart fill / pot de peinture, angle d'arrêt 30°) sur le
/// cylindre its_make_cylinder(10, 20) : facette et point de la référence C++
/// (ligne « # facet N hit x y z » de smart.txt / bucket.txt).
fn fill(tool: PaintTool, angle: f32, name: &str) -> (PaintFacets, PaintFacets) {
    use engine::adapters::ffi::run_mesh_op;
    use engine::api::split::{MeshOp, PrimitiveShape};
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/paint")
        .join(name);
    let text = std::fs::read_to_string(&path).unwrap();
    let header: Vec<f32> = text
        .lines()
        .next()
        .unwrap()
        .split_whitespace()
        .filter_map(|t| t.parse().ok())
        .collect();
    let (facet, hit) = (header[0] as u32, [header[1], header[2], header[3]]);
    // Même maillage que la référence (its_make_cylinder(0.5 × 20, 20)).
    let mesh = run_mesh_op(
        &MeshOp::Primitive {
            shape: PrimitiveShape::Cylinder,
            side_mm: 20.0,
        },
        &[],
    )
    .unwrap()
    .remove(0)
    .1;
    let mut session = PaintSession::new(&mesh).unwrap();
    let stroke = PaintStroke {
        points: vec![hit],
        facets: vec![facet],
        tool,
        fill_angle: angle,
        ..stroke(CursorShape::Circle)
    };
    session.stroke(PaintChannel::Supports, &stroke).unwrap();
    (
        session.export(PaintChannel::Supports).unwrap(),
        reference(name),
    )
}

#[test]
fn smart_fill_matches_isolated_orca_reference() {
    let (got, want) = fill(PaintTool::SmartFill, 30.0, "smart.txt");
    eprintln!("smart fill: {} triangles", got.len());
    assert!(!want.is_empty());
    assert_eq!(got, want);
}

#[test]
fn bucket_fill_matches_isolated_orca_reference() {
    let (got, want) = fill(PaintTool::BucketFill, 30.0, "bucket.txt");
    eprintln!("bucket fill: {} triangles", got.len());
    assert!(!want.is_empty());
    assert_eq!(got, want);
}

/// Angle d'arrêt 1° < 2° entre secteurs voisins : la propagation s'arrête
/// (seules la facette touchée et sa jumelle coplanaire sont peintes).
#[test]
fn fill_stop_angle_matches_isolated_orca_reference() {
    for (tool, name) in [
        (PaintTool::SmartFill, "smart1.txt"),
        (PaintTool::BucketFill, "bucket1.txt"),
    ] {
        let (got, want) = fill(tool, 1.0, name);
        assert_eq!(got, want, "{name}");
        assert_eq!(got.len(), 2, "{name}");
    }
}

/// « Garder la peinture » (keep_painting d'Orca) : remap_painting reporte la
/// peinture sur un maillage aux triangles réordonnés (même géométrie) ; une
/// chaîne invalide est refusée ; aucune peinture → vide.
#[test]
fn remap_painting_follows_geometry_not_triangle_indices() {
    let source = cube(20.0);
    let mut target = source.clone();
    target.indices.reverse();
    let n = source.indices.len() as u32;
    let mut facets = engine::api::paint::PaintFacets::new();
    facets.insert(0, "4".into());
    facets.insert(1, "8".into());
    let out = engine::adapters::ffi::remap_painting(&source, &target, &facets).unwrap();
    // Seuls les triangles de même géométrie reçoivent la peinture ; le
    // remappage spatial d'Orca peut subdiviser un triangle à la frontière de
    // deux états (arête partagée), d'où une chaîne longue pour le n-1.
    assert_eq!(
        out.keys().copied().collect::<Vec<_>>(),
        vec![n - 2, n - 1],
        "{out:?}"
    );
    assert_eq!(out.get(&(n - 2)).map(String::as_str), Some("8"), "{out:?}");
    assert!(out[&(n - 1)].contains('4'), "enforcer conservé : {out:?}");
    let mut bad = engine::api::paint::PaintFacets::new();
    bad.insert(0, "zz".into());
    assert!(engine::adapters::ffi::remap_painting(&source, &target, &bad).is_err());
    assert!(
        engine::adapters::ffi::remap_painting(&source, &target, &Default::default())
            .unwrap()
            .is_empty()
    );
}

// ---------------------------------------------------------------------------
// Lot 38 : curseur triangle, plage de hauteur, comblement, pot sans arêtes,
// remappage MMU, aperçu au survol — références paint_ref.cpp (modes pointer,
// height, height_z, gap, gap0, remap, remap_input, bucket_noedge).
// ---------------------------------------------------------------------------

/// En-tête « # facet N hit x y z » / « # z_world z facet N » d'une référence.
fn header(name: &str) -> Vec<f32> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/paint")
        .join(name);
    std::fs::read_to_string(&path)
        .unwrap()
        .lines()
        .next()
        .unwrap()
        .split_whitespace()
        .filter_map(|t| t.parse().ok())
        .collect()
}

fn cylinder() -> TriangleMesh {
    use engine::adapters::ffi::run_mesh_op;
    use engine::api::split::{MeshOp, PrimitiveShape};
    run_mesh_op(
        &MeshOp::Primitive {
            shape: PrimitiveShape::Cylinder,
            side_mm: 20.0,
        },
        &[],
    )
    .unwrap()
    .remove(0)
    .1
}

/// Scène du cube de la référence : trait (3 points, rayon 3) à `state` puis,
/// si `dot > 0`, tampon de rayon `dot` en (12, 8) à `dot_state`.
fn cube_scene(
    session: &mut PaintSession,
    channel: PaintChannel,
    state: u8,
    dot: f32,
    dot_state: u8,
) {
    session
        .stroke(
            channel,
            &PaintStroke {
                state,
                ..stroke(CursorShape::Circle)
            },
        )
        .unwrap();
    if dot > 0.0 {
        session
            .stroke(
                channel,
                &PaintStroke {
                    points: vec![[12.0, 8.0, 20.0]],
                    facets: vec![2],
                    radius: dot,
                    state: dot_state,
                    ..stroke(CursorShape::Circle)
                },
            )
            .unwrap();
    }
}

#[test]
fn pointer_cursor_matches_isolated_orca_reference() {
    let mut session = PaintSession::new(&cube(20.0)).unwrap();
    cube_scene(&mut session, PaintChannel::Supports, 1, 0.0, 0);
    let before = session.export(PaintChannel::Supports).unwrap();
    session
        .stroke(
            PaintChannel::Supports,
            &PaintStroke {
                points: vec![[6.0, 4.5, 20.0]],
                facets: vec![2],
                cursor: CursorShape::Pointer,
                state: 2,
                ..stroke(CursorShape::Circle)
            },
        )
        .unwrap();
    let got = session.export(PaintChannel::Supports).unwrap();
    assert_eq!(got, reference("pointer.txt"));
    // Un seul sous-triangle change : seule la facette 2 diffère.
    let changed: Vec<_> = got.keys().filter(|k| before.get(k) != got.get(k)).collect();
    assert_eq!(changed, vec![&2], "{changed:?}");
}

#[test]
fn height_range_matches_isolated_orca_reference() {
    let h = header("height.txt");
    let mut session = PaintSession::new(&cylinder()).unwrap();
    session
        .stroke(
            PaintChannel::Mmu,
            &PaintStroke {
                points: vec![[h[1], h[2], h[3]]],
                facets: vec![h[0] as u32],
                camera_pos: [100.0, 0.0, 10.0],
                cursor: CursorShape::HeightRange,
                height: 2.0,
                state: 2,
                ..stroke(CursorShape::Circle)
            },
        )
        .unwrap();
    let got = session.export(PaintChannel::Mmu).unwrap();
    eprintln!("height range: {} triangles", got.len());
    assert!(
        got.len() > 100,
        "toute la ceinture du cylindre : {}",
        got.len()
    );
    assert_eq!(got, reference("height.txt"));
}

#[test]
fn height_range_on_other_part_matches_isolated_orca_reference() {
    let h = header("height_z.txt");
    assert_eq!(h[0], 8.0);
    let trafo = glam::DMat4::from_translation(glam::DVec3::new(0.0, 0.0, 3.0)).to_cols_array();
    let mut session = PaintSession::new(&cylinder()).unwrap();
    session
        .stroke(
            PaintChannel::Mmu,
            &PaintStroke {
                points: vec![],
                facets: vec![],
                camera_pos: [100.0, 0.0, 10.0],
                cursor: CursorShape::HeightRange,
                height: 2.0,
                z_world: Some(8.0),
                state: 2,
                trafo,
                ..stroke(CursorShape::Circle)
            },
        )
        .unwrap();
    assert_eq!(
        session.export(PaintChannel::Mmu).unwrap(),
        reference("height_z.txt")
    );
    // Plage hors de l'objet : rien.
    let mut empty = PaintSession::new(&cylinder()).unwrap();
    empty
        .stroke(
            PaintChannel::Mmu,
            &PaintStroke {
                points: vec![],
                facets: vec![],
                cursor: CursorShape::HeightRange,
                z_world: Some(500.0),
                state: 2,
                ..stroke(CursorShape::Circle)
            },
        )
        .unwrap();
    assert!(empty.export(PaintChannel::Mmu).unwrap().is_empty());
}

#[test]
fn gap_fill_matches_orca_triangle_selector_patch() {
    for (area, name) in [(4.0, "gap.txt"), (0.0, "gap0.txt")] {
        let mut session = PaintSession::new(&cube(20.0)).unwrap();
        cube_scene(&mut session, PaintChannel::Mmu, 1, 0.8, 2);
        session.gap_fill(PaintChannel::Mmu, area).unwrap();
        let got = session.export(PaintChannel::Mmu).unwrap();
        assert_eq!(got, reference(name), "{name}");
    }
    // Le tampon (≈ 2 mm²) disparaît à 4 mm² : l'état 2 n'est plus encodé
    // (chiffre hex « 8 » des feuilles d'état 2 absent), il reste à 0 mm².
    let filled = reference("gap.txt");
    let kept = reference("gap0.txt");
    assert_ne!(filled, kept);
    let mut session = PaintSession::new(&cube(20.0)).unwrap();
    assert!(session.gap_fill(PaintChannel::Mmu, 5.5).is_err());
}

#[test]
fn mmu_remap_matches_isolated_orca_reference() {
    let mut session = PaintSession::new(&cube(20.0)).unwrap();
    cube_scene(&mut session, PaintChannel::Mmu, 1, 2.0, 2);
    assert_eq!(
        session.export(PaintChannel::Mmu).unwrap(),
        reference("remap_input.txt")
    );
    session.remap_states(PaintChannel::Mmu, &[3, 1]).unwrap();
    assert_eq!(
        session.export(PaintChannel::Mmu).unwrap(),
        reference("remap.txt")
    );
    // Identité : inchangé ; table invalide refusée.
    let before = session.export(PaintChannel::Mmu).unwrap();
    session.remap_states(PaintChannel::Mmu, &[1, 2, 3]).unwrap();
    assert_eq!(session.export(PaintChannel::Mmu).unwrap(), before);
    assert!(session.remap_states(PaintChannel::Mmu, &[0]).is_err());
    assert!(session.remap_states(PaintChannel::Mmu, &[17]).is_err());
}

#[test]
fn bucket_without_edge_detection_matches_isolated_orca_reference() {
    let h = header("bucket_noedge.txt");
    let mut session = PaintSession::new(&cylinder()).unwrap();
    session
        .stroke(
            PaintChannel::Mmu,
            &PaintStroke {
                points: vec![[h[1], h[2], h[3]]],
                facets: vec![h[0] as u32],
                tool: PaintTool::BucketFill,
                fill_angle: -1.0,
                state: 3,
                ..stroke(CursorShape::Circle)
            },
        )
        .unwrap();
    let got = session.export(PaintChannel::Mmu).unwrap();
    let want = reference("bucket_noedge.txt");
    assert_eq!(got, want);
    // Sans arêtes : tout le cylindre (parois + capots), pas seulement la paroi.
    assert!(got.len() > 360, "{}", got.len());
}

/// Aperçu au survol : smart fill à 30° sur la paroi = les 360 triangles de
/// smart.txt (9 flottants chacun), état inchangé ; pinceau refusé.
#[test]
fn fill_preview_selects_what_the_click_paints() {
    let h = header("smart.txt");
    let mut session = PaintSession::new(&cylinder()).unwrap();
    let s = PaintStroke {
        points: vec![[h[1], h[2], h[3]]],
        facets: vec![h[0] as u32],
        tool: PaintTool::SmartFill,
        fill_angle: 30.0,
        ..stroke(CursorShape::Circle)
    };
    let (vertices, truncated) = session.preview(PaintChannel::Supports, &s).unwrap();
    assert!(!truncated);
    assert_eq!(vertices.len(), reference("smart.txt").len() * 9);
    assert!(session.export(PaintChannel::Supports).unwrap().is_empty());
    // Triangle : un seul.
    let (one, _) = session
        .preview(
            PaintChannel::Supports,
            &PaintStroke {
                tool: PaintTool::Brush,
                cursor: CursorShape::Pointer,
                ..s.clone()
            },
        )
        .unwrap();
    assert_eq!(one.len(), 9);
    assert!(session
        .preview(PaintChannel::Supports, &stroke(CursorShape::Circle))
        .is_err());
}

#[test]
fn serve_session_gap_fill_remap_and_preview() {
    let mut server = Server::new();
    let open = server.handle(ServeRequest {
        id: 1,
        op: ServeOp::PaintOpen {
            session: "p/o".into(),
            mesh: cube(20.0),
        },
    });
    assert!(open.ok);
    let input = reference("remap_input.txt");
    let r = server.handle(ServeRequest {
        id: 2,
        op: ServeOp::PaintRemap {
            session: "p/o".into(),
            channel: PaintChannel::Mmu,
            facets: Some(input.clone()),
            map: vec![3, 1],
        },
    });
    assert!(r.ok, "{r:?}");
    assert_eq!(r.facets.unwrap(), reference("remap.txt"));
    let r = server.handle(ServeRequest {
        id: 3,
        op: ServeOp::PaintGapFill {
            session: "p/o".into(),
            channel: PaintChannel::Mmu,
            facets: Some(input),
            area: 9.0,
        },
    });
    assert!(!r.ok, "aire hors plage refusée");
    let r = server.handle(ServeRequest {
        id: 4,
        op: ServeOp::PaintPreview {
            session: "p/o".into(),
            channel: PaintChannel::Supports,
            facets: Some(PaintFacets::new()),
            stroke: PaintStroke {
                points: vec![[5.0, 5.0, 20.0]],
                facets: vec![2],
                tool: PaintTool::BucketFill,
                ..stroke(CursorShape::Circle)
            },
        },
    });
    assert!(r.ok, "{r:?}");
    // Pot à 30° sur le cube : la face du dessus (2 triangles).
    assert_eq!(r.selection.unwrap().len(), 18);
    assert_eq!(r.truncated, Some(false));
}
