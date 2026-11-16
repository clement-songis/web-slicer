//! Smoke test du bridge cxx (T012) : chaîne include + link + exceptions.
//! Exécution : `cargo test -p engine --features ffi` (devShell requis).

#![cfg(feature = "ffi")]

mod common;

#[test]
fn loads_stl_and_counts_triangles() {
    let n = engine::adapters::ffi::model_triangle_count(&common::fixture("cube20.stl"))
        .expect("libslic3r charge le STL");
    assert_eq!(n, 12, "cube = 12 triangles");
}

#[test]
fn clean_error_on_missing_file() {
    let err = engine::adapters::ffi::model_triangle_count(std::path::Path::new(
        "/nulle/part/fantome.stl",
    ))
    .expect_err("doit échouer");
    assert_eq!(err.code, engine::api::EngineErrorCode::InvalidModel);
}

#[test]
fn registre_runtime_aligne_avec_le_registre_genere() {
    use engine::params::{ParamGroup, REGISTRY};
    let runtime = engine::adapters::ffi::print_config_option_count();
    let generated = REGISTRY
        .iter()
        .filter(|p| {
            matches!(
                p.group,
                ParamGroup::Fff | ParamGroup::Common | ParamGroup::Sla
            )
        })
        .count();
    assert_eq!(runtime, generated, "parité exacte C++ ↔ registre généré");
}

#[test]
fn loads_full_stl_scene() {
    let model = engine::adapters::ffi::load_model(&common::fixture("cube20.stl")).unwrap();
    assert_eq!(model.objects.len(), 1);
    let obj = &model.objects[0];
    assert_eq!(obj.volumes.len(), 1);
    assert_eq!(obj.volumes[0].mesh.triangle_count(), 12);
    assert!(!obj.instances.is_empty(), "AddDefaultInstances");
}

#[test]
fn loads_orca_3mf_project() {
    let model = engine::adapters::ffi::load_model(&common::fixture("orca_project.3mf")).unwrap();
    assert!(!model.is_empty());
    let n: usize = model
        .objects
        .iter()
        .flat_map(|o| &o.volumes)
        .map(|v| v.mesh.triangle_count())
        .sum();
    assert!(n > 0, "géométrie présente dans le 3MF");
}

#[test]
fn converts_step_to_mesh() {
    let mesh = engine::adapters::ffi::convert_to_mesh(&common::fixture("cube20.step"))
        .expect("libslic3r (OCCT) lit le STEP de fixture");
    assert!(!mesh.is_empty());
    let (lo, hi) = mesh.bounding_box().unwrap();
    for i in 0..3 {
        assert!(
            (hi[i] - lo[i] - 20.0).abs() < 0.5,
            "cube ~20 mm sur l'axe {i}: {lo:?} {hi:?}"
        );
    }
}

#[test]
fn roundtrip_3mf_project_with_config() {
    use engine::api::{ConfigValue, DynamicPrintConfig};
    let model = engine::adapters::ffi::load_model(&common::fixture("cube20.stl")).unwrap();
    let mut config = DynamicPrintConfig::new();
    config
        .set("layer_height", ConfigValue::Float(0.28))
        .unwrap();
    config
        .set(
            "sparse_infill_pattern",
            ConfigValue::String("gyroid".into()),
        )
        .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("roundtrip.3mf");
    engine::adapters::ffi::write_project_3mf(&model, &config, &out).expect("écrit");
    let (back_model, back_config) = engine::adapters::ffi::read_project_3mf(&out).expect("relit");
    let n: usize = back_model
        .objects
        .iter()
        .flat_map(|o| &o.volumes)
        .map(|v| v.mesh.triangle_count())
        .sum();
    assert_eq!(n, 12, "géométrie préservée");
    assert_eq!(
        back_config.get("layer_height"),
        Some(&ConfigValue::Float(0.28)),
        "config embarquée préservée"
    );
    assert_eq!(
        back_config.get("sparse_infill_pattern"),
        Some(&ConfigValue::String("gyroid".into()))
    );
}

fn mesh_open_edges(mesh: &engine::api::TriangleMesh) -> usize {
    use std::collections::HashMap;
    let mut edges = HashMap::new();
    for triangle in &mesh.indices {
        for (a, b) in [
            (triangle[0], triangle[1]),
            (triangle[1], triangle[2]),
            (triangle[2], triangle[0]),
        ] {
            *edges.entry((a.min(b), a.max(b))).or_insert(0usize) += 1;
        }
    }
    edges.into_values().filter(|&count| count == 1).count()
}

fn mesh_degenerate_facets(mesh: &engine::api::TriangleMesh) -> usize {
    mesh.indices
        .iter()
        .filter(|triangle| {
            let [a, b, c] = **triangle;
            let pa = mesh.vertices[a as usize];
            let pb = mesh.vertices[b as usize];
            let pc = mesh.vertices[c as usize];
            let u = [pb[0] - pa[0], pb[1] - pa[1], pb[2] - pa[2]];
            let v = [pc[0] - pa[0], pc[1] - pa[1], pc[2] - pa[2]];
            let cross = [
                u[1] * v[2] - u[2] * v[1],
                u[2] * v[0] - u[0] * v[2],
                u[0] * v[1] - u[1] * v[0],
            ];
            cross.iter().map(|x| x * x).sum::<f32>().sqrt() <= 1e-6
        })
        .count()
}

#[test]
fn repairs_an_open_mesh_and_removes_degenerate_facets() {
    // Cube ouvert (une facette manquante) + un triangle dégénéré. La
    // réparation explicite doit supprimer ce dernier ET reboucher le trou.
    let mut broken = engine::adapters::ffi::load_model(&common::fixture("cube20.stl"))
        .unwrap()
        .objects
        .remove(0)
        .volumes
        .remove(0)
        .mesh;
    broken.indices.pop();
    broken.indices.push([0, 0, 1]);
    assert!(mesh_open_edges(&broken) > 0);
    assert_eq!(mesh_degenerate_facets(&broken), 1);

    let (repaired, report) = engine::adapters::ffi::repair_mesh(&broken).expect("répare");
    assert_eq!(repaired.triangle_count(), 12, "cube de nouveau fermé");
    assert_eq!(mesh_open_edges(&repaired), 0, "trou rebouché par admesh");
    assert_eq!(mesh_degenerate_facets(&repaired), 0);
    assert!(report.degenerate_facets > 0 || report.facets_removed > 0);
    assert!(report.repaired_anything(), "rapport conservé : {report:?}");
}

#[test]
fn repair_rejects_an_out_of_bounds_index() {
    let broken = engine::api::TriangleMesh {
        vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        indices: vec![[0, 1, 3]],
    };
    let error = engine::adapters::ffi::repair_mesh(&broken).expect_err("indice invalide");
    assert_eq!(error.code, engine::api::EngineErrorCode::InvalidModel);
}

/// Writer 3MF Rust → lecteur Orca (`Model::read_from_file`) : volumes (rôles,
/// réglages) et peinture restitués.
#[test]
fn project_3mf_export_round_trips_volumes_and_paint_through_orca_reader() {
    use engine::api::{
        ConfigValue, DynamicPrintConfig, ModelVolume, PaintChannel, VolumePaint, VolumeRole,
    };
    let mut model = engine::adapters::ffi::load_model(&common::fixture("cube20.stl")).unwrap();
    let object = &mut model.objects[0];
    object.volumes[0].paint = VolumePaint {
        facets: vec![
            (PaintChannel::Supports, 0, "4".into()),
            (PaintChannel::Seam, 10, "8".into()),
        ],
    };
    let base = object.volumes[0].clone();
    let mut modifier_settings = DynamicPrintConfig::new();
    modifier_settings
        .set("sparse_infill_density", ConfigValue::Float(0.0))
        .unwrap();
    for (role, settings, name) in [
        (
            VolumeRole::NegativeVolume,
            DynamicPrintConfig::new(),
            "trou",
        ),
        (VolumeRole::ParameterModifier, modifier_settings, "modif"),
        (
            VolumeRole::SupportBlocker,
            DynamicPrintConfig::new(),
            "bloqueur",
        ),
    ] {
        object.volumes.push(ModelVolume {
            name: name.into(),
            role,
            settings,
            paint: VolumePaint::default(),
            ..base.clone()
        });
    }
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("volumes.3mf");
    engine::adapters::ffi::write_project_3mf_with_plates(
        &model,
        &DynamicPrintConfig::new(),
        &[],
        &out,
    )
    .unwrap();
    // Contenu du fichier (e2e editor.export-3mf-paint-volumes) : sous-types,
    // réglage du modificateur, peinture par triangle, <component> par volume.
    {
        use std::io::Read;
        let mut zip = zip::ZipArchive::new(std::fs::File::open(&out).unwrap()).unwrap();
        let mut text = |name: &str| {
            let mut s = String::new();
            zip.by_name(name).unwrap().read_to_string(&mut s).unwrap();
            s
        };
        let settings = text("Metadata/model_settings.config");
        for subtype in [
            "normal_part",
            "negative_part",
            "modifier_part",
            "support_blocker",
        ] {
            assert!(
                settings.contains(&format!("subtype=\"{subtype}\"")),
                "{subtype} : {settings}"
            );
        }
        assert!(
            settings.contains(r#"key="sparse_infill_density" value="0%""#),
            "{settings}"
        );
        let main = text("3D/3dmodel.model");
        assert!(
            main.contains("<components>") && main.contains("<component "),
            "{main}"
        );
        let names: Vec<String> = zip
            .file_names()
            .filter(|n| n.starts_with("3D/Objects/"))
            .map(String::from)
            .collect();
        let part = names.first().expect("partie 3D/Objects");
        let mut part_text = String::new();
        zip.by_name(part)
            .unwrap()
            .read_to_string(&mut part_text)
            .unwrap();
        assert!(
            part_text.contains(r#"paint_supports="4""#) && part_text.contains(r#"paint_seam="8""#)
        );
    }
    let (back, _) = engine::adapters::ffi::read_project_3mf(&out).expect("relu par Orca");
    assert_eq!(back.objects.len(), 1);
    let volumes = &back.objects[0].volumes;
    let roles: Vec<_> = volumes.iter().map(|v| v.role).collect();
    assert_eq!(
        roles,
        vec![
            VolumeRole::ModelPart,
            VolumeRole::NegativeVolume,
            VolumeRole::ParameterModifier,
            VolumeRole::SupportBlocker
        ]
    );
    assert_eq!(volumes[2].name, "modif");
    assert!(
        volumes[2].settings.get("sparse_infill_density").is_some(),
        "réglage du modificateur restitué : {:?}",
        volumes[2].settings
    );
    let paint = &volumes[0].paint.facets;
    assert!(
        paint.contains(&(PaintChannel::Supports, 0, "4".into())),
        "{paint:?}"
    );
    assert!(
        paint.contains(&(PaintChannel::Seam, 10, "8".into())),
        "{paint:?}"
    );
}

fn cube_scene(count: usize) -> engine::api::Model {
    let cube = engine::adapters::ffi::load_model(&common::fixture("cube20.stl")).unwrap();
    let mut model = engine::api::Model::default();
    for _ in 0..count {
        model.objects.extend(cube.objects.clone());
    }
    model
}

fn bed_100() -> engine::api::BuildVolume {
    engine::api::BuildVolume {
        bed_shape: vec![[0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [0.0, 100.0]],
        max_height: 200.0,
        excluded: vec![],
    }
}

/// Boîte XY monde (min, max).
type XyBox = ([f64; 2], [f64; 2]);

/// Boîtes XY monde des instances, regroupées par plateau.
fn world_boxes(model: &engine::api::Model) -> std::collections::BTreeMap<u32, Vec<XyBox>> {
    let mut plates = std::collections::BTreeMap::new();
    for object in &model.objects {
        for instance in &object.instances {
            for volume in &object.volumes {
                let transform = instance.matrix * volume.matrix;
                let mut lo = [f64::INFINITY; 2];
                let mut hi = [f64::NEG_INFINITY; 2];
                for vertex in &volume.mesh.vertices {
                    let point = transform.transform_point3(glam::DVec3::new(
                        vertex[0] as f64,
                        vertex[1] as f64,
                        vertex[2] as f64,
                    ));
                    for axis in 0..2 {
                        lo[axis] = lo[axis].min(point[axis]);
                        hi[axis] = hi[axis].max(point[axis]);
                    }
                }
                plates
                    .entry(instance.plate_index)
                    .or_insert_with(Vec::new)
                    .push((lo, hi));
            }
        }
    }
    plates
}

fn assert_no_box_collisions(boxes: &[([f64; 2], [f64; 2])]) {
    for (index, (lo_a, hi_a)) in boxes.iter().enumerate() {
        for (lo_b, hi_b) in &boxes[index + 1..] {
            let overlap_x = hi_a[0].min(hi_b[0]) - lo_a[0].max(lo_b[0]);
            let overlap_y = hi_a[1].min(hi_b[1]) - lo_a[1].max(lo_b[1]);
            assert!(
                overlap_x <= 1e-4 || overlap_y <= 1e-4,
                "collision entre {lo_a:?}/{hi_a:?} et {lo_b:?}/{hi_b:?}"
            );
        }
    }
}

/// F9 : comme la GUI d'Orca (ArrangeJob, align_center = best_object_pos 0,5),
/// la pile arrangée est centrée sur le plateau, pas laissée dans un coin.
#[test]
fn arrange_centre_la_pile_sur_le_plateau() {
    use engine::api::ArrangeParams;
    let mut model = cube_scene(4);
    let bed = engine::api::BuildVolume {
        bed_shape: vec![[0.0, 0.0], [200.0, 0.0], [200.0, 200.0], [0.0, 200.0]],
        max_height: 200.0,
        excluded: vec![],
    };
    engine::adapters::ffi::arrange(&mut model, &bed, &ArrangeParams::default()).expect("arrange");
    let boxes = world_boxes(&model);
    let boxes = boxes.get(&0).unwrap();
    let (mut lo, mut hi) = ([f64::MAX; 2], [f64::MIN; 2]);
    for (l, h) in boxes {
        for a in 0..2 {
            lo[a] = lo[a].min(l[a]);
            hi[a] = hi[a].max(h[a]);
        }
    }
    let center = [(lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0];
    eprintln!("centre de la pile : {center:?}");
    assert!(
        (center[0] - 100.0).abs() < 5.0 && (center[1] - 100.0).abs() < 5.0,
        "{center:?}"
    );
}

#[test]
fn arrange_quatre_cubes_sans_collision() {
    use engine::api::ArrangeParams;
    let mut model = cube_scene(4);
    let bed = engine::api::BuildVolume {
        bed_shape: vec![[0.0, 0.0], [200.0, 0.0], [200.0, 200.0], [0.0, 200.0]],
        max_height: 200.0,
        excluded: vec![],
    };
    engine::adapters::ffi::arrange(&mut model, &bed, &ArrangeParams::default()).expect("arrange");
    let boxes = world_boxes(&model);
    let boxes = boxes.get(&0).unwrap();
    assert_eq!(boxes.len(), 4);
    assert_no_box_collisions(boxes);
    for (lo, hi) in boxes {
        assert!(lo[0] >= -1e-4 && lo[1] >= -1e-4);
        assert!(hi[0] <= 200.0001 && hi[1] <= 200.0001);
    }
}

#[test]
fn arrange_respects_excluded_regions() {
    use engine::api::ArrangeParams;
    let mut model = cube_scene(4);
    let mut bed = bed_100();
    bed.excluded
        .push(vec![[40.0, 40.0], [60.0, 40.0], [60.0, 60.0], [40.0, 60.0]]);
    engine::adapters::ffi::arrange(&mut model, &bed, &ArrangeParams::default()).expect("arrange");

    let boxes = world_boxes(&model);
    let boxes = boxes.get(&0).unwrap();
    assert_no_box_collisions(boxes);
    for (lo, hi) in boxes {
        let overlap_x = hi[0].min(60.0) - lo[0].max(40.0);
        let overlap_y = hi[1].min(60.0) - lo[1].max(40.0);
        assert!(
            overlap_x <= 1e-3 || overlap_y <= 1e-3,
            "objet dans la zone exclue : {lo:?} {hi:?}"
        );
    }
}

#[test]
fn arrange_is_independent_per_plate_and_preserves_indices() {
    use engine::api::ArrangeParams;
    let mut model = cube_scene(4);
    for (index, object) in model.objects.iter_mut().enumerate() {
        object.instances[0].plate_index = (index / 2) as u32;
    }
    engine::adapters::ffi::arrange(&mut model, &bed_100(), &ArrangeParams::default())
        .expect("arrange");

    let boxes = world_boxes(&model);
    assert_eq!(boxes.len(), 2, "les indices de plateau sont conservés");
    for (plate, plate_boxes) in boxes {
        assert_eq!(plate_boxes.len(), 2, "plateau {plate}");
        assert_no_box_collisions(&plate_boxes);
    }
}

#[test]
fn orients_an_object() {
    let mut object = engine::adapters::ffi::load_model(&common::fixture("overhang.stl"))
        .unwrap()
        .objects
        .remove(0);
    engine::adapters::ffi::orient(&mut object).expect("orient");
}

#[test]
fn slice_normalizes_spiral_mode_like_the_cli() {
    // Parité CLI : `normalize_fdm` après overrides/défauts. En vase spiral,
    // la config effective (bloc exporté) force 1 paroi, 0 couche du dessus,
    // 0 % de remplissage et aucune rétraction au changement de couche, même si
    // la requête demande autre chose.
    use engine::api::{CancelToken, ConfigValue, SliceRequest};
    let (model, mut config) =
        engine::adapters::ffi::read_project_3mf(&common::fixture("orca_project.3mf"))
            .expect("lecture du projet 3MF");
    config.set("spiral_mode", ConfigValue::Bool(true)).unwrap();
    config.set("wall_loops", ConfigValue::Int(3)).unwrap();
    config.set("top_shell_layers", ConfigValue::Int(4)).unwrap();
    // On isole la normalisation de la validation « E relative », qui exige
    // G92 E0 avec Marlin ; cette combinaison est couverte par le test 3MF réel.
    config
        .set("use_relative_e_distances", ConfigValue::Bool(false))
        .unwrap();

    let work = tempfile::tempdir().unwrap();
    let req = SliceRequest {
        model,
        config,
        plate_index: 0,
        work_dir: work.path().to_path_buf(),
        calib: None,
    };
    let result = engine::adapters::ffi::slice(req, Box::new(|_, _| {}), CancelToken::new())
        .expect("le tranchage spiral aboutit");
    let gcode = std::fs::read_to_string(&result.gcode_path).unwrap();
    let setting = |key: &str| {
        gcode
            .lines()
            .find_map(|line| line.strip_prefix(&format!("; {key} = ")))
            .map(str::to_owned)
            .unwrap_or_else(|| panic!("{key} absent du bloc de configuration"))
    };
    assert_eq!(setting("spiral_mode"), "1");
    assert_eq!(setting("wall_loops"), "1");
    assert_eq!(setting("top_shell_layers"), "0");
    assert_eq!(setting("sparse_infill_density"), "0%");
    assert!(
        setting("retract_when_changing_layer")
            .split(',')
            .all(|v| v == "0"),
        "pas de rétraction au changement de couche"
    );
}

/// Prisme en L centré sur l'origine (40 × 40 mm, bras de 10 mm, hauteur `h`) :
/// bras long en X le long de Y bas, bras vertical à X bas → asymétrique en X.
fn l_prism(h: f32) -> engine::api::TriangleMesh {
    let outline: [[f32; 2]; 6] = [
        [0.0, 0.0],
        [40.0, 0.0],
        [40.0, 10.0],
        [10.0, 10.0],
        [10.0, 40.0],
        [0.0, 40.0],
    ];
    let mut vertices = Vec::new();
    for z in [0.0, h] {
        for [x, y] in outline {
            vertices.push([x - 20.0, y - 20.0, z - h / 2.0]);
        }
    }
    let n = outline.len() as u32;
    let mut indices = Vec::new();
    // Contour CCW étoilé depuis le sommet 0 : éventail valide pour les deux faces.
    for i in 1..n - 1 {
        indices.push([n, n + i, n + i + 1]); // dessus (+Z)
        indices.push([0, i + 1, i]); // dessous (−Z)
    }
    for i in 0..n {
        let j = (i + 1) % n;
        indices.push([i, j, n + j]);
        indices.push([i, n + j, n + i]);
    }
    engine::api::TriangleMesh { vertices, indices }
}

/// Points d'extrusion de paroi (`;TYPE:Outer wall`) par couche, et nombre de couches.
fn outer_wall_points(gcode: &str) -> (usize, Vec<[f64; 2]>) {
    let (mut layers, mut in_wall, mut points) = (0, false, Vec::new());
    let (mut x, mut y) = (f64::NAN, f64::NAN);
    for line in gcode.lines() {
        if line.starts_with(";LAYER_CHANGE") {
            layers += 1;
        } else if let Some(kind) = line.strip_prefix(";TYPE:") {
            in_wall = kind == "Outer wall";
        } else if line.starts_with("G1 ") || line.starts_with("G0 ") {
            let mut extrudes = false;
            for word in line.split(';').next().unwrap_or("").split_whitespace() {
                let (k, v) = word.split_at(1);
                match k {
                    "X" => x = v.parse().unwrap_or(x),
                    "Y" => y = v.parse().unwrap_or(y),
                    "E" => extrudes = v.parse::<f64>().is_ok_and(|e| e > 0.0),
                    _ => {}
                }
            }
            if in_wall && extrudes && layers > 0 {
                points.push([x, y]);
            }
        }
    }
    (layers, points)
}

#[test]
fn mirrored_instance_slices_as_the_mirror_image() {
    // Miroir de l'éditeur = échelle négative de l'instance (det < 0). Le G-code
    // doit être l'image miroir de l'original autour du centre de l'objet, avec
    // le même nombre de couches (faces non retournées → tranches pleines).
    use engine::api::{CancelToken, ConfigValue, ModelInstance, SliceRequest};
    let (reference, mut config) =
        engine::adapters::ffi::read_project_3mf(&common::fixture("orca_project.3mf"))
            .expect("lecture du projet 3MF");
    config
        .set("use_relative_e_distances", ConfigValue::Bool(false))
        .unwrap();
    // Centre de l'objet de référence : forcément sur le plateau du profil.
    let center = reference.objects[0].instances[0].matrix.w_axis;
    let (cx, cy) = (center.x, center.y);
    let slice = |scale_x: f64| {
        let mut model = engine::api::Model::default();
        let object = model.add_object("L", l_prism(6.0));
        object.instances = vec![ModelInstance {
            matrix: glam::DMat4::from_translation(glam::DVec3::new(cx, cy, 3.0))
                * glam::DMat4::from_scale(glam::DVec3::new(scale_x, 1.0, 1.0)),
            plate_index: 0,
            printable: true,
        }];
        let work = tempfile::tempdir().unwrap();
        let req = SliceRequest {
            model,
            config: config.clone(),
            plate_index: 0,
            work_dir: work.path().to_path_buf(),
            calib: None,
        };
        let result = engine::adapters::ffi::slice(req, Box::new(|_, _| {}), CancelToken::new())
            .expect("le tranchage aboutit");
        outer_wall_points(&std::fs::read_to_string(&result.gcode_path).unwrap())
    };
    let (layers, original) = slice(1.0);
    let (mirrored_layers, mirrored) = slice(-1.0);
    assert!(layers >= 20, "{layers} couches pour 6 mm");
    assert_eq!(mirrored_layers, layers, "même nombre de couches");
    assert!(!original.is_empty() && !mirrored.is_empty());

    let bounds = |pts: &[[f64; 2]]| {
        pts.iter().fold([f64::MAX, f64::MIN], |[lo, hi], p| {
            [lo.min(p[0]), hi.max(p[0])]
        })
    };
    let [lo, hi] = bounds(&original);
    let [mlo, mhi] = bounds(&mirrored);
    // Bornes X inversées autour du centre (tolérance : demi-largeur de ligne).
    assert!(
        (mlo - (2.0 * cx - hi)).abs() < 0.3,
        "min miroir {mlo} ≠ {}",
        2.0 * cx - hi
    );
    assert!(
        (mhi - (2.0 * cx - lo)).abs() < 0.3,
        "max miroir {mhi} ≠ {}",
        2.0 * cx - lo
    );
    // Bras vertical (Y haut) : côté X bas à l'original, côté X haut en miroir.
    let arm = |pts: &[[f64; 2]]| {
        bounds(
            &pts.iter()
                .copied()
                .filter(|p| p[1] > cy + 5.0)
                .collect::<Vec<_>>(),
        )
    };
    let [alo, ahi] = arm(&original);
    let [malo, mahi] = arm(&mirrored);
    assert!(ahi < cx - 5.0, "bras original à gauche : [{alo}, {ahi}]");
    assert!(malo > cx + 5.0, "bras miroir à droite : [{malo}, {mahi}]");
    // Même quantité de paroi extérieure (aucune perte de contour).
    let ratio = mirrored.len() as f64 / original.len() as f64;
    assert!(
        (0.9..=1.1).contains(&ratio),
        "points de paroi {} vs {}",
        mirrored.len(),
        original.len()
    );
}

#[test]
fn non_printable_instance_is_excluded_from_gcode() {
    use engine::api::{CancelToken, ConfigValue, ModelInstance, SliceRequest};
    let (reference, mut config) =
        engine::adapters::ffi::read_project_3mf(&common::fixture("orca_project.3mf"))
            .expect("lecture du projet 3MF");
    config
        .set("use_relative_e_distances", ConfigValue::Bool(false))
        .unwrap();
    let center = reference.objects[0].instances[0].matrix.w_axis;

    let slice = |second_printable: bool| {
        let mut model = reference.clone();
        model.objects[0].name = "base".into();
        for instance in &mut model.objects[0].instances {
            instance.printable = true;
        }
        let second = model.add_object("second", l_prism(6.0));
        second.instances = vec![ModelInstance {
            matrix: glam::DMat4::from_translation(glam::DVec3::new(center.x, center.y, 3.0)),
            plate_index: 0,
            printable: second_printable,
        }];
        let work = tempfile::tempdir().unwrap();
        let result = engine::adapters::ffi::slice(
            SliceRequest {
                model,
                config: config.clone(),
                plate_index: 0,
                work_dir: work.path().to_path_buf(),
                calib: None,
            },
            Box::new(|_, _| {}),
            CancelToken::new(),
        )
        .expect("le tranchage aboutit");
        std::fs::read_to_string(&result.gcode_path).unwrap()
    };

    let with_second = slice(true);
    let without_second = slice(false);
    for gcode in [&with_second, &without_second] {
        assert!(
            gcode.contains("; printing object base"),
            "objet base conservé"
        );
    }
    assert!(with_second.contains("; printing object second"));
    assert!(!without_second.contains("; printing object second"));
    let extrusions = |gcode: &str| {
        gcode
            .lines()
            .filter(|line| line.starts_with("G1 ") && line.contains(" E"))
            .count()
    };
    assert!(
        extrusions(&with_second) > extrusions(&without_second),
        "l'objet non imprimable doit réduire les extrusions : {} vs {}",
        extrusions(&with_second),
        extrusions(&without_second)
    );
}
