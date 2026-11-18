//! Lot 26 : relief ré-éditable et vignettes dans le projet 3MF écrit par
//! libslic3r (store_bbs_3mf) puis relu (load_bbs_3mf), comme Orca.
//! Exécution : cargo test -p engine --features ffi --test ffi_emboss_3mf.

#![cfg(feature = "ffi")]

use std::collections::BTreeMap;
use std::io::Read;
use std::path::Path;

use engine::adapters::ffi::{
    read_project_3mf, read_project_3mf_import, write_project_3mf_with_plates,
};
use engine::api::{DynamicPrintConfig, Model, TriangleMesh};
use engine::threemf::ProjectPlate;
use serde_json::json;

fn cube(s: f32) -> TriangleMesh {
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [s, 0.0, 0.0],
        [s, s, 0.0],
        [0.0, s, 0.0],
        [0.0, 0.0, s],
        [s, 0.0, s],
        [s, s, s],
        [0.0, s, s],
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

fn entries(path: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut zip = zip::ZipArchive::new(std::fs::File::open(path).unwrap()).unwrap();
    (0..zip.len())
        .map(|i| {
            let mut f = zip.by_index(i).unwrap();
            let mut out = Vec::new();
            f.read_to_end(&mut out).unwrap();
            (f.name().to_string(), out)
        })
        .collect()
}

/// PNG RGBA minimal (filtre 0, zlib « stored ») : évite toute dépendance.
fn png(width: u32, height: u32, rgba: [u8; 4]) -> Vec<u8> {
    fn crc(data: &[u8]) -> u32 {
        let mut c = 0xFFFF_FFFFu32;
        for &b in data {
            c ^= u32::from(b);
            for _ in 0..8 {
                c = if c & 1 != 0 {
                    0xEDB8_8320 ^ (c >> 1)
                } else {
                    c >> 1
                };
            }
        }
        !c
    }
    fn chunk(out: &mut Vec<u8>, kind: &[u8], data: &[u8]) {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        let mut body = kind.to_vec();
        body.extend_from_slice(data);
        out.extend_from_slice(&body);
        out.extend_from_slice(&crc(&body).to_be_bytes());
    }
    // Rangée = octet de filtre 0 + pixels RGBA.
    let row: Vec<u8> = std::iter::once(0)
        .chain(rgba.iter().copied().cycle().take(width as usize * 4))
        .collect();
    let raw = row.repeat(height as usize);
    let mut z = vec![0x78, 0x01];
    for (i, block) in raw.chunks(65535).enumerate() {
        let last = (i + 1) * 65535 >= raw.len();
        z.push(u8::from(last));
        z.extend_from_slice(&(block.len() as u16).to_le_bytes());
        z.extend_from_slice(&(!(block.len() as u16)).to_le_bytes());
        z.extend_from_slice(block);
    }
    let (mut a, mut b) = (1u32, 0u32);
    for &x in &raw {
        a = (a + u32::from(x)) % 65521;
        b = (b + a) % 65521;
    }
    z.extend_from_slice(&((b << 16) | a).to_be_bytes());
    let mut out = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
    chunk(&mut out, b"IHDR", &ihdr);
    chunk(&mut out, b"IDAT", &z);
    chunk(&mut out, b"IEND", &[]);
    out
}

#[test]
fn text_and_svg_emboss_round_trip_like_orca() {
    let mut model = Model::default();
    let text = json!({
        "type": "text", "text": "Bonjour", "font_file": "NotoSansKR-Regular.ttf",
        "style_name": "Noto Sans", "size_mm": 12.5, "depth_mm": 2.0, "boldness_mm": 0.3
    });
    let svg_source =
        "<svg xmlns=\"http://www.w3.org/2000/svg\"><rect width=\"10\" height=\"5\"/></svg>";
    let svg = json!({ "type": "svg", "svg": svg_source, "file_name": "étoile.svg", "depth_mm": 1.5, "width_mm": 20 });
    model.add_object("texte", cube(10.0)).volumes[0].emboss = Some(text);
    model.add_object("forme", cube(5.0)).volumes[0].emboss = Some(svg);
    model.add_object("ordinaire", cube(3.0));

    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("emboss.3mf");
    let plates = [ProjectPlate {
        thumbnail_png: png(256, 256, [255, 0, 0, 255]),
        ..ProjectPlate::default()
    }];
    write_project_3mf_with_plates(&model, &DynamicPrintConfig::new(), &plates, &out).unwrap();
    // Copie facultative pour une relecture par le CLI orca-slicer.
    if let Ok(dir) = std::env::var("WEBSLICER_KEEP_3MF_DIR") {
        std::fs::copy(&out, Path::new(&dir).join("emboss-thumbnail.3mf")).unwrap();
    }

    let files = entries(&out);
    let settings = String::from_utf8(files["Metadata/model_settings.config"].clone()).unwrap();
    // <slic3rpe:text> (TextConfigurationSerialization::to_xml) et <slic3rpe:shape>.
    assert!(
        settings.contains("<slic3rpe:text text=\"Bonjour\""),
        "{settings}"
    );
    assert!(
        settings.contains("font_descriptor=\"NotoSansKR-Regular.ttf\""),
        "{settings}"
    );
    assert_eq!(
        settings.matches("<slic3rpe:shape ").count(),
        2,
        "{settings}"
    );
    assert!(
        settings.contains("filepath3mf=\"3D/étoile.svg\""),
        "{settings}"
    );
    assert_eq!(files["3D/étoile.svg"], svg_source.as_bytes());
    // Vignettes : plate_1.png (512 × 512) + plate_1_small.png (128 × 128).
    let thumb = &files["Metadata/plate_1.png"];
    assert_eq!(u32::from_be_bytes(thumb[16..20].try_into().unwrap()), 512);
    let small = &files["Metadata/plate_1_small.png"];
    assert_eq!(u32::from_be_bytes(small[16..20].try_into().unwrap()), 128);
    assert!(settings.contains("Metadata/plate_1.png"), "{settings}");

    // Relecture libslic3r (load_bbs_3mf) : paramètres retrouvés.
    let view = read_project_3mf_import(&out).unwrap();
    let emboss: Vec<_> = view["objects"]
        .as_array()
        .unwrap()
        .iter()
        .map(|o| o["volumes"][0]["emboss"].clone())
        .collect();
    assert_eq!(emboss[0]["type"], "text");
    assert_eq!(emboss[0]["text"], "Bonjour");
    assert_eq!(emboss[0]["font_file"], "NotoSansKR-Regular.ttf");
    assert_eq!(emboss[0]["style_name"], "Noto Sans");
    assert!((emboss[0]["size_mm"].as_f64().unwrap() - 12.5).abs() < 1e-6);
    assert!((emboss[0]["boldness_mm"].as_f64().unwrap() - 0.3).abs() < 1e-6);
    assert!((emboss[0]["depth_mm"].as_f64().unwrap() - 2.0).abs() < 1e-6);
    assert_eq!(emboss[1]["type"], "svg");
    assert_eq!(emboss[1]["svg"], svg_source);
    assert_eq!(emboss[1]["file_name"], "étoile.svg");
    assert!((emboss[1]["depth_mm"].as_f64().unwrap() - 1.5).abs() < 1e-6);
    assert!(emboss[2].is_null());
}

/// Variantes pour la relecture par le CLI orca-slicer (seulement si
/// `WEBSLICER_KEEP_3MF_DIR` est défini) : cube seul, + vignette, + texte, + SVG.
#[test]
fn writes_cli_variants_when_requested() {
    let Ok(dir) = std::env::var("WEBSLICER_KEEP_3MF_DIR") else {
        return;
    };
    // Configuration complète d'un projet exporté (le CLI exige une imprimante).
    let config = std::env::var("WEBSLICER_CLI_BASE_3MF")
        .ok()
        .map(|base| read_project_3mf(Path::new(&base)).unwrap().1)
        .unwrap_or_default();
    let text = json!({ "type": "text", "text": "Bonjour", "font_file": "NotoSansKR-Regular.ttf",
        "style_name": "Noto Sans", "size_mm": 12.5, "depth_mm": 2.0 });
    let svg = json!({ "type": "svg", "file_name": "r.svg", "depth_mm": 1.5,
        "svg": "<svg xmlns=\"http://www.w3.org/2000/svg\"><rect width=\"10\" height=\"5\"/></svg>" });
    for (name, thumb, emboss) in [
        ("v-plain", false, None),
        ("v-thumb", true, None),
        ("v-text", false, Some(text)),
        ("v-svg", false, Some(svg.clone())),
        ("v-all", true, Some(svg)),
    ] {
        let mut model = Model::default();
        model.add_object("cube", cube(10.0)).volumes[0].emboss = emboss;
        let plates = [ProjectPlate {
            thumbnail_png: if thumb {
                png(256, 256, [255, 0, 0, 255])
            } else {
                Vec::new()
            },
            ..ProjectPlate::default()
        }];
        let out = Path::new(&dir).join(format!("{name}.3mf"));
        write_project_3mf_with_plates(&model, &config, &plates, &out).unwrap();
    }
}

#[test]
fn invalid_thumbnail_is_skipped_without_failing_the_export() {
    let mut model = Model::default();
    model.add_object("cube", cube(10.0));
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("p.3mf");
    let plates = [ProjectPlate {
        thumbnail_png: b"pas un png".to_vec(),
        ..ProjectPlate::default()
    }];
    write_project_3mf_with_plates(&model, &DynamicPrintConfig::new(), &plates, &out).unwrap();
    assert!(!entries(&out).contains_key("Metadata/plate_1.png"));
}
