//! Projet 3MF avec plateaux écrit par libslic3r (`store_bbs_3mf`, SplitModel |
//! ShareMesh comme « Save project » d'Orca) : plateaux, noms, réglages de plateau
//! et objets rattachés identiques au projet d'Orca (tests/fixtures/gcode_3mf/
//! two_plates.3mf), relecture libslic3r fidèle.
//! Exécution : cargo test -p engine --features ffi --test ffi_project_3mf.

#![cfg(feature = "ffi")]

use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};

use engine::adapters::ffi::{read_project_3mf, write_project_3mf_with_plates};
use engine::api::ConfigValue;
use engine::threemf::ProjectPlate;
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

/// Blocs `<plate>` de model_settings.config, identify_id neutralisé
/// (identifiant d'instance du process) ; la ligne gcode_file n'est écrite
/// qu'avec WithGcode (export CLI de la fixture), pas par « Save project ».
fn plates_xml(xml: &[u8]) -> Vec<String> {
    let text = String::from_utf8_lossy(xml);
    let ident = Regex::new(r#"key="identify_id" value="\d+""#).unwrap();
    text.split("<plate>")
        .skip(1)
        .map(|p| {
            ident
                .replace_all(p.split("</plate>").next().unwrap(), "identify_id")
                .lines()
                .filter(|l| !l.contains(r#"key="gcode_file""#))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .collect()
}

#[test]
fn project_plates_match_orca_and_read_back() {
    let (mut model, config) = read_project_3mf(&fixtures().join("two_plates.3mf")).unwrap();
    model.objects[1].instances[0].plate_index = 1;
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("p.3mf");
    write_project_3mf_with_plates(&model, &config, &[], &out).unwrap();
    // Copie optionnelle pour un contrôle par le CLI Orca (staging).
    if let Ok(keep) = std::env::var("WS_PROJECT_OUT") {
        std::fs::copy(&out, keep).unwrap();
    }

    let ours = entries(&out);
    let orca = entries(&fixtures().join("two_plates.3mf"));
    assert_eq!(
        plates_xml(&ours["Metadata/model_settings.config"]),
        plates_xml(&orca["Metadata/model_settings.config"])
    );
    // SplitModel comme Orca : une partie 3D/Objects/*.model par objet.
    let parts =
        |e: &BTreeMap<String, Vec<u8>>| e.keys().filter(|k| k.starts_with("3D/Objects/")).count();
    assert_eq!(parts(&ours), parts(&orca));

    let (back, back_config) = read_project_3mf(&out).unwrap();
    assert_eq!(back.objects.len(), 2);
    for (a, b) in back.objects.iter().zip(&model.objects) {
        assert_eq!(
            a.volumes[0].mesh.indices.len(),
            b.volumes[0].mesh.indices.len()
        );
        assert_eq!(a.instances[0].matrix, b.instances[0].matrix);
    }
    assert_eq!(
        back_config.0.get("layer_height"),
        config.0.get("layer_height")
    );
}

#[test]
fn plate_name_and_settings_are_written() {
    let (mut model, config) = read_project_3mf(&fixtures().join("two_plates.3mf")).unwrap();
    model.objects[1].instances[0].plate_index = 1;
    let mut settings = engine::api::DynamicPrintConfig::new();
    settings
        .set(
            "curr_bed_type",
            ConfigValue::String("Textured PEI Plate".into()),
        )
        .unwrap();
    let plates = [
        ProjectPlate {
            name: "Cubes".into(),
            settings,
        },
        ProjectPlate::default(),
    ];
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("p.3mf");
    write_project_3mf_with_plates(&model, &config, &plates, &out).unwrap();
    let xml = String::from_utf8(entries(&out)["Metadata/model_settings.config"].clone()).unwrap();
    assert!(xml.contains(r#"key="plater_name" value="Cubes""#), "{xml}");
    // Orca écrit curr_bed_type du plateau sous la clé « bed_type ».
    assert!(
        xml.contains(r#"key="bed_type" value="Textured PEI Plate""#),
        "{xml}"
    );
}
