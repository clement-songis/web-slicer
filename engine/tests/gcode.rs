//! Parseur G-code (T021) : sur un G-code OrcaSlicer réel (cube BBL), vérifie
//! la reconstruction des couches, la variété des types de lignes et les
//! statistiques d'en-tête. Pur Rust — s'exécute sans la feature `ffi`.

mod common;

use engine::api::LineKind;
use engine::gcode::parse_gcode;

#[test]
fn reconstruit_couches_types_et_stats() {
    let preview = parse_gcode(&common::fixture("cube_bbl.gcode")).expect("parse du G-code");

    // Couches : le cube fait 100 couches (en-tête « total layer number: 100 »).
    assert_eq!(preview.stats.layer_count, 100, "nombre de couches");
    assert!(
        preview.layers.len() >= 100,
        "au moins 100 couches reconstruites : {}",
        preview.layers.len()
    );

    // Chaque couche a un z croissant et des segments.
    let with_segments = preview
        .layers
        .iter()
        .filter(|l| !l.segments.is_empty())
        .count();
    assert!(with_segments >= 100, "couches peuplées : {with_segments}");
    let zs: Vec<f32> = preview.layers.iter().map(|l| l.z).collect();
    assert!(
        zs.windows(2).filter(|w| w[1] > w[0]).count() >= 90,
        "z globalement croissant"
    );

    // Tous les types présents dans le fichier sont reconnus (aucun Unknown).
    for kind in [
        LineKind::OuterWall,
        LineKind::InnerWall,
        LineKind::Infill,
        LineKind::SolidInfill,
        LineKind::TopSurface,
        LineKind::BottomSurface,
        LineKind::InternalBridge,
        LineKind::Skirt,
    ] {
        assert!(
            preview.kinds_present.contains(&kind),
            "type présent dans la légende : {kind:?}"
        );
    }
    assert!(
        !preview.kinds_present.contains(&LineKind::Unknown),
        "aucun rôle non reconnu : {:?}",
        preview.kinds_present
    );

    // Segments : polylignes d'au moins deux points, largeur > 0 pour l'extrusion.
    let extrusion = preview
        .layers
        .iter()
        .flat_map(|l| &l.segments)
        .find(|s| s.kind == LineKind::OuterWall)
        .expect("au moins un segment de paroi externe");
    assert!(extrusion.points.len() >= 2, "polyligne");
    assert!(extrusion.width > 0.0, "largeur d'extrusion renseignée");
    assert!(extrusion.feedrate > 0.0, "vitesse renseignée");

    // Stats d'en-tête.
    assert!(
        (preview.stats.estimated_time_s - (29.0 * 60.0 + 56.0)).abs() < 1.0,
        "temps estimé = 29m 56s : {}",
        preview.stats.estimated_time_s
    );
    assert_eq!(
        preview.stats.filament_mm,
        vec![1321.85],
        "filament utilisé (mm)"
    );
}
