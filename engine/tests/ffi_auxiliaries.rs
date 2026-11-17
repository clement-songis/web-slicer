//! Fichiers auxiliaires d'un projet (Orca : Auxiliaries/ du 3MF) : écriture
//! par store_bbs_3mf, relecture par load_bbs_3mf, et conservation à
//! l'identique par le CLI Orca qui rouvre et réexporte le projet.
//! Exécution : cargo test -p engine --features ffi --test ffi_auxiliaries

#![cfg(feature = "ffi")]

use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};

fn dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/auxiliaries")
}

const FILES: [(&str, &[u8]); 3] = [
    ("Model Pictures/vue.png", b"\x89PNG\r\n\x1a\nfaux png"),
    (
        "Others/notes.txt",
        "Notes d'assemblage : vis M3 × 8.\n".as_bytes(),
    ),
    ("Bill of Materials/bom.csv", b"piece;qte\ncube;1\n"),
];

fn aux_entries(path: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut zip = zip::ZipArchive::new(std::fs::File::open(path).unwrap()).unwrap();
    let mut out = BTreeMap::new();
    for i in 0..zip.len() {
        let mut f = zip.by_index(i).unwrap();
        if let Some(rel) = f.name().strip_prefix("Auxiliaries/") {
            let rel = rel.to_string();
            let mut data = Vec::new();
            f.read_to_end(&mut data).unwrap();
            out.insert(rel, data);
        }
    }
    out
}

fn expected() -> BTreeMap<String, Vec<u8>> {
    FILES
        .iter()
        .map(|(p, d)| (p.to_string(), d.to_vec()))
        .collect()
}

fn write_project(out: &Path) {
    let src = tempfile::tempdir().unwrap();
    for (rel, data) in FILES {
        let p = src.path().join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, data).unwrap();
    }
    let (model, config) = engine::adapters::ffi::read_project_3mf(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/multi_filament/two_objects.3mf"),
    )
    .unwrap();
    engine::adapters::ffi::write_project_3mf_with_auxiliaries(
        &model,
        &config,
        &[],
        Some(src.path()),
        out,
    )
    .unwrap();
}

#[test]
#[ignore = "régénère tests/fixtures/auxiliaries/project_aux.3mf"]
fn export_aux_project() {
    write_project(&dir().join("project_aux.3mf"));
}

#[test]
fn auxiliaries_are_written_and_read_back() {
    let tmp = tempfile::tempdir().unwrap();
    let out = tmp.path().join("p.3mf");
    write_project(&out);
    assert_eq!(aux_entries(&out), expected(), "entrées Auxiliaries/ du 3MF");
    let back = tmp.path().join("aux");
    let files = engine::adapters::ffi::read_project_auxiliaries(&out, &back).unwrap();
    assert_eq!(files, expected().into_keys().collect::<Vec<_>>());
    for (rel, data) in FILES {
        assert_eq!(std::fs::read(back.join(rel)).unwrap(), data);
    }
    // Sans dossier : aucun fichier auxiliaire.
    let plain = tmp.path().join("plain.3mf");
    let (model, config) = engine::adapters::ffi::read_project_3mf(&out).unwrap();
    engine::adapters::ffi::write_project_3mf_with_plates(&model, &config, &[], &plain).unwrap();
    assert!(aux_entries(&plain).is_empty());
}

/// Le CLI Orca rouvre le projet et le réexporte avec les mêmes fichiers
/// auxiliaires (project_aux.orca.3mf, make_ref.py) ; le moteur relit aussi
/// les auxiliaires du 3MF écrit par Orca.
#[test]
fn orca_keeps_the_same_auxiliaries() {
    let orca = dir().join("project_aux.orca.3mf");
    assert_eq!(
        aux_entries(&orca),
        expected(),
        "Auxiliaries/ après passage par Orca"
    );
    let tmp = tempfile::tempdir().unwrap();
    let files = engine::adapters::ffi::read_project_auxiliaries(&orca, tmp.path()).unwrap();
    assert_eq!(files.len(), FILES.len());
}
