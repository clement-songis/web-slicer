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
