//! « Export all plate sliced file » d'Orca : .gcode.3mf écrit par
//! `store_bbs_3mf` (WithGcode | SkipModel) depuis les G-code de chaque plateau.
//! Référence : `orca-slicer --slice 0 --export-3mf` sur un projet deux plateaux
//! (tests/fixtures/gcode_3mf, make_ref.py). Les G-code d'entrée sont ceux
//! d'Orca : on compare l'empaquetage (G-code + md5 par plateau, plateaux de
//! model_settings.config, métadonnées de slice_info.config).
//! Exécution : cargo test -p engine --features ffi --test ffi_gcode_3mf.

#![cfg(feature = "ffi")]

use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};

use engine::adapters::ffi::{
    read_project_3mf, write_gcode_3mf, write_gcode_3mf_with_settings, Gcode3mfExtras, PlateGcode,
};
use regex::Regex;

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/gcode_3mf")
}

fn entries(path: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut zip = zip::ZipArchive::new(std::fs::File::open(path).unwrap()).unwrap();
    (0..zip.len())
        .map(|i| {
            let mut f = zip.by_index(i).unwrap();
            let mut data = Vec::new();
            f.read_to_end(&mut data).unwrap();
            (f.name().to_string(), data)
        })
        .collect()
}

/// Métadonnées comparables d'un slice_info.config : par plateau, les clés
/// stables (hors first_layer_time, non initialisé côté CLI) + lignes filament
/// et objet.
fn slice_info(xml: &[u8]) -> Vec<Vec<String>> {
    let text = String::from_utf8_lossy(xml);
    let meta = Regex::new(r#"<metadata key="([a-z_]+)" value="([^"]*)"/>"#).unwrap();
    let ident = Regex::new(r#"identify_id="\d+""#).unwrap();
    text.split("<plate>")
        .skip(1)
        .map(|plate| {
            let plate = plate.split("</plate>").next().unwrap();
            let mut lines: Vec<String> = meta
                .captures_iter(plate)
                .filter(|c| &c[1] != "first_layer_time")
                .map(|c| format!("{}={}", &c[1], &c[2]))
                .collect();
            lines.extend(
                plate
                    .lines()
                    .map(str::trim)
                    .filter(|l| l.starts_with("<filament ") || l.starts_with("<object "))
                    // identify_id = identifiant d'instance du process (loaded_id côté CLI).
                    .map(|l| ident.replace(l, "identify_id=\"_\"").into_owned()),
            );
            lines
        })
        .collect()
}

fn plate_gcode_refs(xml: &[u8]) -> Vec<String> {
    let text = String::from_utf8_lossy(xml);
    Regex::new(r#"<metadata key="gcode_file" value="([^"]*)"/>"#)
        .unwrap()
        .captures_iter(&text)
        .map(|c| c[1].to_string())
        .collect()
}

fn two_plate_model() -> (engine::api::Model, engine::api::DynamicPrintConfig) {
    let (mut model, config) = read_project_3mf(&fixtures().join("two_plates.3mf")).unwrap();
    assert_eq!(model.objects.len(), 2);
    // Plateaux du projet (model_settings.config) : cylindre sur le plateau 2.
    model.objects[1].instances[0].plate_index = 1;
    (model, config)
}

#[test]
fn all_plates_sliced_file_matches_orca() {
    let (model, config) = two_plate_model();
    let gcodes = [1u32, 2].map(|n| PlateGcode {
        plate_index: n - 1,
        gcode_path: fixtures().join(format!("plate_{n}.orca.gcode")),
    });
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("all.gcode.3mf");
    write_gcode_3mf(&model, &config, &gcodes, None, &out).unwrap();

    let ours = entries(&out);
    let orca = entries(&fixtures().join("two_plates.orca.gcode.3mf"));
    for n in [1, 2] {
        let name = format!("Metadata/plate_{n}.gcode");
        assert_eq!(ours.get(&name), orca.get(&name), "{name}");
        let md5 = format!("{name}.md5");
        assert_eq!(ours.get(&md5), orca.get(&md5), "{md5}");
    }
    assert_eq!(
        plate_gcode_refs(&ours["Metadata/model_settings.config"]),
        plate_gcode_refs(&orca["Metadata/model_settings.config"])
    );
    assert_eq!(
        slice_info(&ours["Metadata/slice_info.config"]),
        slice_info(&orca["Metadata/slice_info.config"])
    );
}

#[test]
fn single_plate_sliced_file_embeds_only_that_plate() {
    let (model, config) = two_plate_model();
    let gcodes = [1u32, 2].map(|n| PlateGcode {
        plate_index: n - 1,
        gcode_path: fixtures().join(format!("plate_{n}.orca.gcode")),
    });
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("p2.gcode.3mf");
    // Audit 49 X1C49-a : vignettes des deux plateaux en entrée — le fichier
    // exporté (plateau 2 seul) ne doit emporter que la vignette et la fiche
    // Metadata/plate_2.{png,json} de CE plateau (une par plateau dont le
    // G-code est inclus), pas celles du plateau 1 exclu.
    let thumbnails = [
        std::fs::read(fixtures().join("plate_1.png")).unwrap(),
        std::fs::read(fixtures().join("plate_2.png")).unwrap(),
    ];
    write_gcode_3mf_with_settings(
        &model,
        &config,
        &gcodes,
        &[],
        Some(1),
        Gcode3mfExtras {
            thumbnails: &thumbnails,
            ..Gcode3mfExtras::default()
        },
        &out,
    )
    .unwrap();
    let ours = entries(&out);
    let orca = entries(&fixtures().join("two_plates.orca.gcode.3mf"));
    assert!(!ours.contains_key("Metadata/plate_1.gcode"));
    assert_eq!(
        ours.get("Metadata/plate_2.gcode"),
        orca.get("Metadata/plate_2.gcode")
    );
    // Vignettes : uniquement celles du plateau exporté (index aligné).
    assert!(
        !ours.contains_key("Metadata/plate_1.png"),
        "vignette du plateau exclu : {:?}",
        ours.keys().collect::<Vec<_>>()
    );
    assert!(!ours.contains_key("Metadata/plate_1_small.png"));
    assert!(ours.contains_key("Metadata/plate_2.png"));
    assert!(ours.contains_key("Metadata/plate_2_small.png"));
    // Fiches plate_<n>.json : uniquement celle du plateau exporté, SOUS SON
    // PROPRE NUMÉRO (le défaut d'audit compactait id_bboxes → la fiche du
    // plateau 2 était écrite en Metadata/plate_1.json).
    assert!(
        !ours.contains_key("Metadata/plate_1.json"),
        "fiche du plateau exclu : {:?}",
        ours.keys().collect::<Vec<_>>()
    );
    let plate2 = ours["Metadata/plate_2.json"].clone();
    let text = String::from_utf8_lossy(&plate2).to_string();
    assert!(text.contains("cylinder"), "fiche du plateau 2 : {text}");
    assert!(!text.contains("cube20"), "fiche du plateau 2 : {text}");
    let info = slice_info(&ours["Metadata/slice_info.config"]);
    assert_eq!(info.len(), 1, "{info:?}");
    assert_eq!(info[0], slice_info(&orca["Metadata/slice_info.config"])[1]);
}

#[test]
fn all_plates_sliced_file_keeps_thumbnails_aligned_by_plate() {
    // Audit 49 X1C49-a (export de tous les plateaux) : vignettes denses, une
    // par plateau, plate_<n>.png alignée sur l'index (la vignette du plateau 1
    // reste plate_1.png même si un plateau non tranché la précède).
    let (model, config) = two_plate_model();
    let gcodes = [1u32, 2].map(|n| PlateGcode {
        plate_index: n - 1,
        gcode_path: fixtures().join(format!("plate_{n}.orca.gcode")),
    });
    let thumbnails = [
        std::fs::read(fixtures().join("plate_1.png")).unwrap(),
        std::fs::read(fixtures().join("plate_2.png")).unwrap(),
    ];
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("all_thumbs.gcode.3mf");
    write_gcode_3mf_with_settings(
        &model,
        &config,
        &gcodes,
        &[],
        None,
        Gcode3mfExtras {
            thumbnails: &thumbnails,
            ..Gcode3mfExtras::default()
        },
        &out,
    )
    .unwrap();
    let ours = entries(&out);
    assert_eq!(
        ours.get("Metadata/plate_1.png").map(Vec::as_slice),
        Some(thumbnails[0].as_slice()),
        "plate_1.png doit être la vignette du plateau 1"
    );
    assert_eq!(
        ours.get("Metadata/plate_2.png").map(Vec::as_slice),
        Some(thumbnails[1].as_slice()),
        "plate_2.png doit être la vignette du plateau 2"
    );
    assert!(ours.contains_key("Metadata/plate_1.json"));
    assert!(ours.contains_key("Metadata/plate_2.json"));
}
