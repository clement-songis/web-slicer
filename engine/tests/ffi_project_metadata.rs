//! Métadonnées d'un projet 3MF (Orca : Model::model_info / design_info) :
//! écriture par store_bbs_3mf (_add_model_file_to_archive : Title, Designer,
//! DesignerCover, Description, Copyright, License, entrées conservées,
//! DesignerUserId vidé « PRIVACY ») et relecture par load_bbs_3mf
//! (_handle_end_metadata).
//! Exécution : cargo test -p engine --features ffi --test ffi_project_metadata

#![cfg(feature = "ffi")]

use std::collections::BTreeMap;
use std::io::Read;
use std::path::Path;

use engine::threemf::ProjectMetadata;

fn model_metadata(path: &Path) -> BTreeMap<String, String> {
    let mut zip = zip::ZipArchive::new(std::fs::File::open(path).unwrap()).unwrap();
    let mut xml = String::new();
    zip.by_name("3D/3dmodel.model")
        .unwrap()
        .read_to_string(&mut xml)
        .unwrap();
    let mut out = BTreeMap::new();
    let mut rest = xml.as_str();
    while let Some(start) = rest.find("<metadata name=\"") {
        rest = &rest[start + 16..];
        let name_end = rest.find('"').unwrap();
        let name = rest[..name_end].to_string();
        let value_start = rest.find('>').unwrap() + 1;
        let value_end = rest.find("</metadata>").unwrap();
        out.insert(name, rest[value_start..value_end].to_string());
        rest = &rest[value_end..];
    }
    out
}

fn sample() -> ProjectMetadata {
    let mut items = BTreeMap::new();
    items.insert(
        "Thumbnail_Middle".to_string(),
        "/Metadata/plate_1.png".to_string(),
    );
    items.insert("Title".to_string(), "ancien titre".to_string());
    ProjectMetadata {
        model_name: "Support de casque & co".into(),
        origin: "original".into(),
        cover_file: "vue.png".into(),
        description: "<p>Imprimé en PETG</p>".into(),
        copyright: r#"[{"author":"Zed"}]"#.into(),
        license: "BY-NC-SA".into(),
        designer: Some("Alice".into()),
        designer_user_id: Some("123456".into()),
        metadata_items: items,
        profile: None,
    }
}

#[test]
fn metadata_written_like_orca_and_read_back() {
    let (model, config) = engine::adapters::ffi::read_project_3mf(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/multi_filament/two_objects.3mf"),
    )
    .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("meta.3mf");
    let metadata = sample();
    engine::adapters::ffi::write_project_3mf_full(
        &model,
        &config,
        &[],
        None,
        Some(&metadata),
        &out,
    )
    .unwrap();

    let entries = model_metadata(&out);
    let get = |k: &str| entries.get(k).cloned().unwrap_or_default();
    // xml_escape appliqué deux fois par Orca (metadata_item_map puis flux).
    assert!(get("Title").contains("Support de casque"), "{entries:?}");
    assert_eq!(get("Designer"), "Alice");
    assert_eq!(get("DesignerUserId"), "", "PRIVACY : identifiant non écrit");
    assert_eq!(get("DesignerCover"), "vue.png");
    assert_eq!(get("License"), "BY-NC-SA");
    assert_eq!(get("Origin"), "original");
    assert!(get("Copyright").contains("Zed"));
    assert_eq!(
        get("Thumbnail_Middle"),
        "/Metadata/plate_1.png",
        "entrée conservée"
    );
    assert!(entries.contains_key("Application"));

    let view = engine::adapters::ffi::read_project_3mf_import(&out).unwrap();
    let back: ProjectMetadata = serde_json::from_value(view["metadata"].clone()).unwrap();
    assert_eq!(back.model_name, metadata.model_name);
    assert_eq!(back.description, metadata.description);
    assert_eq!(back.license, "BY-NC-SA");
    assert_eq!(back.cover_file, "vue.png");
    assert_eq!(back.designer.as_deref(), Some("Alice"));
    // Comme Orca : double xml_escape à l'écriture, xml_unescape ne connaît pas
    // &quot; → les guillemets du Copyright reviennent échappés une fois.
    assert_eq!(back.copyright, metadata.copyright.replace('"', "&quot;"));
    assert_eq!(
        back.metadata_items.get("Title").map(String::as_str),
        Some(metadata.model_name.as_str())
    );
}

#[test]
fn without_metadata_designer_is_empty() {
    let (model, config) = engine::adapters::ffi::read_project_3mf(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/multi_filament/two_objects.3mf"),
    )
    .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("plain.3mf");
    engine::adapters::ffi::write_project_3mf_full(&model, &config, &[], None, None, &out).unwrap();
    let entries = model_metadata(&out);
    assert_eq!(entries.get("Designer").map(String::as_str), Some(""));
    assert_eq!(entries.get("Title").map(String::as_str), Some(""));
}
