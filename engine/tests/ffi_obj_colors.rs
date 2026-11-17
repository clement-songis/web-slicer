//! Import OBJ coloré comme Orca (obj_colors.cpp) comparé au CLI Orca 2.4.1
//! (--load-assemble-list, tests/fixtures/obj_colors/make_ref.py) : même
//! peinture MMU par facette et même palette (filament_colour).
//! Exécution : cargo test -p engine --features ffi --test ffi_obj_colors

#![cfg(feature = "ffi")]

use std::path::{Path, PathBuf};

use engine::api::{ConfigValue, PaintChannel};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/obj_colors")
        .join(name)
}

fn mmu(model: &engine::api::Model) -> Vec<(u32, String)> {
    let mut v: Vec<(u32, String)> = model.objects[0].volumes[0]
        .paint
        .facets
        .iter()
        .filter(|(c, _, _)| *c == PaintChannel::Mmu)
        .map(|(_, t, s)| (*t, s.clone()))
        .collect();
    v.sort();
    v
}

fn check(stem: &str, expected_colours: usize) {
    let ours = engine::adapters::ffi::load_obj_colors(&fixture(&format!("{stem}.obj"))).unwrap();
    assert!(ours.colored);
    let (orca, config) =
        engine::adapters::ffi::read_project_3mf(&fixture(&format!("{stem}.orca.3mf"))).unwrap();
    let Some(ConfigValue::Strings(colours)) = config.get("filament_colour") else {
        panic!("filament_colour absent")
    };
    assert_eq!(&ours.palette, colours, "{stem} : palette");
    assert_eq!(ours.palette.len(), expected_colours);
    assert_eq!(
        ours.model.objects[0].volumes[0].mesh.indices.len(),
        orca.objects[0].volumes[0].mesh.indices.len()
    );
    let (a, b) = (mmu(&ours.model), mmu(&orca));
    assert!(!a.is_empty(), "{stem} : peinture MMU");
    assert_eq!(a, b, "{stem} : peinture par facette");
    assert_eq!(
        ours.model.objects[0].volumes[0].extruder,
        orca.objects[0].volumes[0].extruder
    );
}

#[test]
fn mtl_face_colours_like_orca() {
    check("cube_mtl", 3);
}

#[test]
fn vertex_colours_like_orca() {
    check("cube_vcolor", 2);
}

#[test]
fn uncoloured_obj_is_reported() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("plain.obj");
    std::fs::write(
        &p,
        "v 0 0 0\nv 10 0 0\nv 0 10 0\nv 0 0 10\nf 1 3 2\nf 1 2 4\nf 2 3 4\nf 3 1 4\n",
    )
    .unwrap();
    let r = engine::adapters::ffi::load_obj_colors(&p).unwrap();
    assert!(!r.colored && r.palette.is_empty());
}
