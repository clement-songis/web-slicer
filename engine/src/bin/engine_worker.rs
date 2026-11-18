//! Process worker : exécute les opérations libslic3r isolées du backend
//! (crash C++ contenu, annulation par kill — R1/R9/T018).
//!
//! Protocole de sortie (voir `engine::adapters::ffi::worker`) : lignes
//! `P <ratio> <phase>`, `R <json>`, `E <code> <message>`. La sortie standard
//! de Rust est un `LineWriter` : chaque `println!` est vidé sur `\n`, donc la
//! progression parvient au parent en flux.
//!
//! Sous-commandes :
//! - `triangle-count <fichier>` / `config-count` : smokes FFI (T012).
//! - `load-model <fichier> <format>` : décode le modèle via le moteur, écrit le
//!   maillage d'affichage WSMh dans un fichier temporaire et émet la ligne
//!   `R <chemin>` (T121, pipeline de conversion serveur). Le WSMh est binaire et
//!   libslic3r pollue stdout de lignes Boost.Log — d'où le protocole `R`/`E`
//!   plutôt qu'un flux brut. Erreur → ligne `E <code> <message>` + sortie ≠ 0.
//! - `self-test [--crash|--hang|--stderr-flood] <work_dir>` : exerce le protocole de T018
//!   sans dépendre du pipeline de tranchage (branché en T019).
//! - `arrange <request.json>` : nesting libslic3r, matrices complètes en `R`.
//! - `repair <request.json>` : réparation admesh, WSMh dans le fichier demandé
//!   et rapport `RepairedMeshErrors` en `R`.
//! - `simplify <request.json>` : simplification quadrique libslic3r
//!   (`its_quadric_edge_collapse`), WSMh dans le fichier demandé et rapport
//!   `SimplifyReport` (comptes avant/après) en `R`.
//! - `serve` : worker persistant JSON-lines (stdin) → `R <json>` (stdout),
//!   sessions de peinture TriangleSelector en mémoire (`api::paint`).
//! - `remap-paint <request.json>` : « garder la peinture » (keep_painting),
//!   TriangleSelector::remap_painting par canal, `R` = canaux remappés.
//! - `layer-profile <request.json>` : profil de hauteur variable libslic3r
//!   (`layer_height_profile_adaptive` / `smooth_height_profile`), `R` = profil plat.
//! - `--args-from-stdin` : worker **préchauffé à usage unique**. Le process est
//!   lancé à l'avance par le backend ; l'initialisation statique de libslic3r
//!   (~0,1 s avant `main`) est ainsi payée hors requête. Il lit ensuite UNE
//!   ligne JSON (tableau de chaînes = arguments de la sous-commande), l'exécute
//!   comme si elle avait été passée en ligne de commande, puis se termine :
//!   un process neuf par opération (isolation et déterminisme inchangés).

use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Les requêtes/résultats sont bornés avant lecture/écriture : un worker isolé
/// ne doit pas pouvoir allouer sans limite sur une entrée corrompue.
const MAX_SCENE_REQUEST_BYTES: u64 = 512 * 1024 * 1024;
const MAX_SCENE_RESULT_BYTES: usize = 16 * 1024 * 1024;
const MAX_REPAIRED_MESH_BYTES: usize = 512 * 1024 * 1024;

#[derive(Debug, Deserialize)]
struct ArrangeRequest {
    model: engine::api::Model,
    bed: engine::api::BuildVolume,
    params: engine::api::ArrangeParams,
    instance_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ArrangedInstance {
    id: String,
    plate_index: u32,
    matrix: [f64; 16],
    /// Ordre d'impression séquentielle (1-based par plateau), si séquentiel.
    #[serde(skip_serializing_if = "Option::is_none")]
    print_order: Option<u32>,
}

#[derive(Debug, Serialize)]
struct ArrangeResponse {
    placements: Vec<ArrangedInstance>,
}

#[derive(Debug, Deserialize)]
struct RepairRequest {
    mesh: engine::api::TriangleMesh,
    output_path: PathBuf,
}

#[derive(Debug, Serialize)]
struct RepairResponse {
    mesh_path: PathBuf,
    repairs: engine::api::RepairedMeshErrors,
}

#[derive(Debug, Deserialize)]
struct SimplifyRequest {
    mesh: engine::api::TriangleMesh,
    params: engine::api::mesh::SimplifyParams,
    output_path: PathBuf,
}

/// `flush-volumes` : volumes de purge depuis les couleurs (FlushVolCalculator).
#[derive(Debug, Deserialize)]
struct FlushVolumesRequest {
    colours: Vec<String>,
    #[serde(default)]
    is_support: Vec<bool>,
    #[serde(default)]
    min_flush: Vec<i32>,
    #[serde(default)]
    flush_dataset: i32,
    /// Config résolue du projet : minimum (get_min_flush_volumes), jeu de
    /// données (nozzle_flush_dataset[0]) et filaments support en dérivent
    /// quand ils ne sont pas fournis.
    #[serde(default)]
    config: Option<engine::api::DynamicPrintConfig>,
}

/// `fill-bed` : « Remplir le plateau avec des instances » d'Orca.
#[derive(Debug, Deserialize)]
struct FillBedRequest {
    model: engine::api::Model,
    bed: engine::api::BuildVolume,
    config: engine::api::DynamicPrintConfig,
    object_index: usize,
    #[serde(default)]
    clearance: f64,
}

#[derive(Debug, Serialize)]
struct FillBedResponse {
    /// Matrices (colonne-major) des instances de l'objet après remplissage.
    instances: Vec<[f64; 16]>,
}

/// `export-project-3mf` : projet 3MF avec ses plateaux (« Save project »
/// d'Orca, `store_bbs_3mf`).
#[derive(Debug, Deserialize)]
struct Project3mfRequest {
    model: engine::api::Model,
    config: engine::api::DynamicPrintConfig,
    #[serde(default)]
    plates: Vec<engine::threemf::ProjectPlate>,
    /// Fichiers auxiliaires à empaqueter (Auxiliaries/), optionnel.
    #[serde(default)]
    aux_dir: Option<PathBuf>,
    output_path: PathBuf,
}

/// `export-gcode-3mf` : .gcode.3mf d'un ou de tous les plateaux tranchés
/// (« Export (all) plate sliced file » d'Orca, `store_bbs_3mf`).
#[derive(Debug, Deserialize)]
struct Gcode3mfRequest {
    model: engine::api::Model,
    config: engine::api::DynamicPrintConfig,
    gcodes: Vec<engine::adapters::ffi::PlateGcode>,
    #[serde(default)]
    export_plate: Option<u32>,
    output_path: PathBuf,
}

#[derive(Debug, Serialize)]
struct Gcode3mfResponse {
    path: PathBuf,
}

/// `export-meshes` : maillage d'export par objet comme `Plater::export_stl`
/// d'Orca (négatifs soustraits, instances fusionnées), WSMh `part-<i>.wsm`.
#[derive(Debug, Deserialize)]
struct ExportMeshesRequest {
    model: engine::api::Model,
    output_dir: PathBuf,
}

/// `export-drc` : un fichier Draco `<output_dir>/part-<i>.drc` par maillage.
#[derive(Debug, Deserialize)]
struct DrcExportRequest {
    meshes: Vec<engine::api::TriangleMesh>,
    output_dir: PathBuf,
}

#[derive(Debug, Serialize)]
struct DrcExportResponse {
    files: Vec<PathBuf>,
}

#[derive(Debug, Serialize)]
struct SimplifyResponse {
    mesh_path: PathBuf,
    report: engine::api::mesh::SimplifyReport,
}

/// Arguments effectifs : ligne de commande, ou (`--args-from-stdin`) première
/// ligne JSON de stdin. Stdin fermé sans ligne = parent parti → sortie muette.
fn effective_args() -> Vec<String> {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) != Some("--args-from-stdin") {
        return args;
    }
    let mut line = String::new();
    match std::io::stdin().read_line(&mut line) {
        Ok(0) | Err(_) => std::process::exit(0),
        Ok(_) => {}
    }
    match serde_json::from_str::<Vec<String>>(line.trim_end()) {
        Ok(rest) => std::iter::once(args[0].clone()).chain(rest).collect(),
        Err(e) => {
            eprintln!("--args-from-stdin : ligne JSON invalide : {e}");
            std::process::exit(2);
        }
    }
}

/// Code de sortie d'un worker dont le parent a disparu (voir [`exit_with_parent`]).
const EXIT_ORPHANED: i32 = 70;

/// Termine le worker si son parent (backend) disparaît : un backend tué en
/// plein tranchage ne laisse pas de process `engine-worker` orphelin qui
/// continuerait à consommer CPU et mémoire. Sondage du PID parent (pas de
/// `PR_SET_PDEATHSIG`, lié au *thread* créateur — les threads bloquants de
/// tokio qui lancent les workers peuvent s'arrêter avant eux).
fn exit_with_parent() {
    #[cfg(unix)]
    {
        let parent = std::os::unix::process::parent_id();
        if parent <= 1 {
            return; // déjà détaché (lancement manuel via init) : rien à surveiller
        }
        std::thread::spawn(move || loop {
            std::thread::sleep(std::time::Duration::from_millis(200));
            if std::os::unix::process::parent_id() != parent {
                std::process::exit(EXIT_ORPHANED);
            }
        });
    }
}

/// Plafond mémoire du worker (lot 25) : variable `ENGINE_WORKER_MAX_MEMORY_MB`
/// (Mio ; 0 = aucun plafond), défaut = quart de la mémoire physique, borné à
/// [3 Gio, 8 Gio] (le worker réserve déjà ~530 Mio de projections inscriptibles
/// au démarrage, TBB/OpenVDB compris, pour ~50 Mio résidents).
const ENV_MAX_MEMORY_MB: &str = "ENGINE_WORKER_MAX_MEMORY_MB";

/// Plafond en octets retenu pour ce worker (None = illimité).
fn memory_limit_bytes() -> Option<u64> {
    const MIB: u64 = 1024 * 1024;
    if let Ok(raw) = std::env::var(ENV_MAX_MEMORY_MB) {
        if let Ok(mb) = raw.trim().parse::<u64>() {
            return (mb > 0).then_some(mb.saturating_mul(MIB));
        }
    }
    #[cfg(unix)]
    {
        // SAFETY : sysconf est sans effet de bord.
        let pages = unsafe { nix::libc::sysconf(nix::libc::_SC_PHYS_PAGES) };
        let page = unsafe { nix::libc::sysconf(nix::libc::_SC_PAGESIZE) };
        if pages > 0 && page > 0 {
            let quarter = (pages as u64).saturating_mul(page as u64) / 4;
            return Some(quarter.clamp(3 * 1024 * MIB, 8 * 1024 * MIB));
        }
    }
    Some(8 * 1024 * MIB)
}

/// Plafonne la mémoire du worker : une entrée hostile (3MF « zip bomb »
/// décompressé par libslic3r, maillage démesuré) fait échouer CE process
/// (allocation refusée → erreur ou abandon, contenus par le parent) au lieu
/// d'épuiser la RAM de l'hôte. RLIMIT_DATA plutôt que RLIMIT_AS : depuis
/// Linux 4.7 il compte les projections privées inscriptibles (tas, mmap
/// anonymes, piles des threads) mais pas les réservations d'espace d'adressage
/// sans accès (arènes glibc, réserves TBB), qui gonflent la mémoire virtuelle
/// sans consommer de RAM et feraient échouer un RLIMIT_AS raisonnable.
fn limit_memory() {
    #[cfg(unix)]
    if let Some(bytes) = memory_limit_bytes() {
        let mut current = nix::libc::rlimit {
            rlim_cur: nix::libc::RLIM_INFINITY,
            rlim_max: nix::libc::RLIM_INFINITY,
        };
        // SAFETY : appel système sur une structure locale valide.
        unsafe { nix::libc::getrlimit(nix::libc::RLIMIT_DATA, &mut current) };
        // Jamais au-dessus d'un plafond déjà imposé par le parent.
        let wanted = (bytes as nix::libc::rlim_t).min(current.rlim_max);
        let limit = nix::libc::rlimit {
            rlim_cur: wanted,
            rlim_max: current.rlim_max,
        };
        // SAFETY : appel système sur une structure locale valide.
        if unsafe { nix::libc::setrlimit(nix::libc::RLIMIT_DATA, &limit) } != 0 {
            eprintln!(
                "plafond mémoire du worker non appliqué : {}",
                std::io::Error::last_os_error()
            );
        }
    }
}

fn main() {
    limit_memory();
    exit_with_parent();
    let args = effective_args();
    match args.get(1).map(String::as_str) {
        Some("triangle-count") => {
            let path = std::path::Path::new(args.get(2).expect("usage: triangle-count <fichier>"));
            match engine::adapters::ffi::model_triangle_count(path) {
                Ok(n) => println!("{n}"),
                Err(e) => {
                    eprintln!("{e}");
                    std::process::exit(1);
                }
            }
        }
        Some("config-count") => {
            println!("{}", engine::adapters::ffi::print_config_option_count());
        }
        Some("load-model") => {
            let path = args.get(2).map(PathBuf::from);
            let format = args.get(3);
            let (Some(path), Some(format)) = (path, format) else {
                eprintln!("usage: load-model <fichier> <format>");
                std::process::exit(2);
            };
            // Le WSMh est binaire et libslic3r pollue stdout de lignes Boost.Log :
            // on écrit les octets dans un fichier et on émet la ligne `R <chemin>`
            // (protocole `P`/`R`/`E` de `worker`, tolérant au bruit de log). Le
            // parent lit puis supprime le fichier.
            match load_model_to_wsmh(&path, format).and_then(write_temp_wsmh) {
                Ok(out_path) => {
                    println!("R {}", out_path.display());
                    let _ = std::io::stdout().flush();
                }
                Err(e) => {
                    emit_error(&code_slug(e.code), &e.message);
                    std::process::exit(1);
                }
            }
        }
        // Modèle tel que lu par libslic3r (Model::read_from_file), SANS cuisson de
        // la transformation des volumes : maillage recentré en float par Orca
        // (ModelVolume::center_geometry_after_creation) + matrice du volume.
        // Sert au harness pour reproduire exactement les sommets du CLI Orca.
        Some("read-project-3mf") => {
            let (Some(path), Some(out)) = (
                args.get(2).map(PathBuf::from),
                args.get(3).map(PathBuf::from),
            ) else {
                eprintln!("usage: read-project-3mf <fichier.3mf> <sortie.json>");
                std::process::exit(2);
            };
            let result = engine::adapters::ffi::read_project_3mf_import(&path).and_then(|view| {
                std::fs::write(&out, view.to_string()).map_err(|e| {
                    engine::api::EngineError::new(engine::api::EngineErrorCode::Io, e.to_string())
                })
            });
            match result {
                Ok(()) => emit_json_result(&serde_json::json!({ "path": out })),
                Err(e) => {
                    emit_error(&code_slug(e.code), &e.message);
                    std::process::exit(1);
                }
            }
        }
        Some("load-model-raw") => {
            let (Some(path), Some(format), Some(out)) = (
                args.get(2).map(PathBuf::from),
                args.get(3),
                args.get(4).map(PathBuf::from),
            ) else {
                eprintln!("usage: load-model-raw <fichier> <format> <sortie.json>");
                std::process::exit(2);
            };
            let result = load_model_raw_json(&path, format).and_then(|json| {
                std::fs::write(&out, json).map_err(|e| {
                    engine::api::EngineError::new(engine::api::EngineErrorCode::Io, e.to_string())
                })
            });
            match result {
                Ok(()) => emit_json_result(&serde_json::json!({ "path": out })),
                Err(e) => {
                    emit_error(&code_slug(e.code), &e.message);
                    std::process::exit(1);
                }
            }
        }
        Some("slice") => {
            let request = std::path::Path::new(args.get(2).expect("usage: slice <request.json>"));
            match engine::adapters::ffi::run_in_worker(request) {
                Ok(result) => {
                    let json = serde_json::to_string(&result).expect("SliceResult sérialisable");
                    println!("R {json}");
                    let _ = std::io::stdout().flush();
                }
                Err(e) => {
                    emit_error(&code_slug(e.code), &e.message);
                    std::process::exit(1);
                }
            }
        }
        Some("arrange") => {
            let Some(path) = args.get(2).map(PathBuf::from) else {
                eprintln!("usage: arrange <request.json>");
                std::process::exit(2);
            };
            match run_arrange(&path) {
                Ok(result) => emit_json_result(&result),
                Err(e) => {
                    emit_error(&code_slug(e.code), &e.message);
                    std::process::exit(1);
                }
            }
        }
        Some("repair") => {
            let Some(path) = args.get(2).map(PathBuf::from) else {
                eprintln!("usage: repair <request.json>");
                std::process::exit(2);
            };
            match run_repair(&path) {
                Ok(result) => emit_json_result(&result),
                Err(e) => {
                    emit_error(&code_slug(e.code), &e.message);
                    std::process::exit(1);
                }
            }
        }
        Some("remap-paint") => {
            let Some(path) = args.get(2).map(PathBuf::from) else {
                eprintln!("usage: remap-paint <request.json>");
                std::process::exit(2);
            };
            match run_remap_paint(&path) {
                Ok(result) => emit_json_result(&result),
                Err(e) => {
                    emit_error(&code_slug(e.code), &e.message);
                    std::process::exit(1);
                }
            }
        }
        Some("simplify") => {
            let Some(path) = args.get(2).map(PathBuf::from) else {
                eprintln!("usage: simplify <request.json>");
                std::process::exit(2);
            };
            match run_simplify(&path) {
                Ok(result) => emit_json_result(&result),
                Err(e) => {
                    emit_error(&code_slug(e.code), &e.message);
                    std::process::exit(1);
                }
            }
        }
        // Découpe de maillage (split/cut/boolean, split.cpp) : WSMh par pièce
        // dans `output_dir`, `MeshOpResponse` en ligne `R`.
        Some("mesh-op") => {
            let Some(path) = args.get(2).map(PathBuf::from) else {
                eprintln!("usage: mesh-op <request.json>");
                std::process::exit(2);
            };
            match run_mesh_op(&path) {
                Ok(result) => emit_json_result(&result),
                Err(e) => {
                    emit_error(&code_slug(e.code), &e.message);
                    std::process::exit(1);
                }
            }
        }
        // Worker persistant : JSON-lines sur stdin, `R <json>` par requête
        // (peinture TriangleSelector, profils de couche) — cf. api::paint.
        Some("serve") => {
            let stdin = std::io::stdin();
            if let Err(e) = engine::adapters::ffi::serve::serve(stdin.lock(), std::io::stdout()) {
                emit_error("io", &e.to_string());
                std::process::exit(1);
            }
        }
        // Simulation GCodeProcessor d'Orca : `gcode-process <gcode> <sortie.wspg>`,
        // `R {move_count, layer_count, time_normal, time_stealth}`.
        Some("gcode-process") => {
            let (Some(input), Some(output)) = (
                args.get(2).map(PathBuf::from),
                args.get(3).map(PathBuf::from),
            ) else {
                eprintln!("usage: gcode-process <gcode> <sortie.wspg>");
                std::process::exit(2);
            };
            let result = engine::adapters::ffi::process_gcode(&input).and_then(|p| {
                std::fs::write(&output, p.encode_file()).map_err(|e| {
                    engine::api::EngineError::new(engine::api::EngineErrorCode::Io, e.to_string())
                })?;
                Ok(serde_json::json!({
                    "move_count": p.move_count(),
                    "layer_count": p.layers.len(),
                    "time_normal": p.meta["summary"]["modes"][0]["time"],
                    "time_stealth": p.meta["summary"]["modes"][1]["time"],
                }))
            });
            match result {
                Ok(v) => emit_json_result(&v),
                Err(e) => {
                    emit_error(&code_slug(e.code), &e.message);
                    std::process::exit(1);
                }
            }
        }
        // Profil de hauteur variable (Slicing.cpp : adaptatif / lissage).
        Some("layer-profile") => {
            let Some(path) = args.get(2).map(PathBuf::from) else {
                eprintln!("usage: layer-profile <request.json>");
                std::process::exit(2);
            };
            let result =
                read_json_bounded::<engine::api::layer_profile::LayerProfileRequest>(&path)
                    .and_then(|req| engine::adapters::ffi::layer_profile(&req))
                    .map(|profile| engine::api::layer_profile::LayerProfileResponse { profile });
            match result {
                Ok(result) => emit_json_result(&result),
                Err(e) => {
                    emit_error(&code_slug(e.code), &e.message);
                    std::process::exit(1);
                }
            }
        }
        // Export Draco (menu Export › … DRC d'Orca, `store_drc`).
        Some("load-step") => {
            // load-step <fichier.step> <options.json> <sortie.json> : StepMeshDialog d'Orca.
            let (Some(path), Some(opts), Some(out)) = (
                args.get(2).map(PathBuf::from),
                args.get(3).map(PathBuf::from),
                args.get(4).map(PathBuf::from),
            ) else {
                eprintln!("usage: load-step <fichier.step> <options.json> <sortie.json>");
                std::process::exit(2);
            };
            let result = read_json_bounded::<engine::adapters::ffi::StepMeshOptions>(&opts)
                .and_then(|o| engine::adapters::ffi::load_step(&path, o))
                .and_then(|(model, dialog_triangles)| {
                    // Un maillage par pièce (volume ; découpe : un par solide).
                    let mut objects: Vec<serde_json::Value> = Vec::new();
                    for o in &model.objects {
                        for v in &o.volumes {
                            let mut single = o.clone();
                            single.volumes = vec![v.clone()];
                            let mesh = engine::adapters::ffi::convert_to_mesh_model(&engine::api::Model {
                                objects: vec![single],
                                custom_gcodes: Vec::new(),
                            });
                            objects.push(serde_json::json!({ "name": v.name, "mesh": mesh }));
                        }
                    }
                    let view = serde_json::json!({ "dialog_triangles": dialog_triangles, "objects": objects });
                    std::fs::write(&out, view.to_string())
                        .map_err(|e| engine::api::EngineError::new(engine::api::EngineErrorCode::Io, e.to_string()))
                });
            match result {
                Ok(()) => emit_json_result(&serde_json::json!({ "path": out })),
                Err(e) => {
                    emit_error(&code_slug(e.code), &e.message);
                    std::process::exit(1);
                }
            }
        }
        Some("load-obj-colors") => {
            // load-obj-colors <fichier.obj> <sortie.json> : peinture MMU + palette.
            let (Some(path), Some(out)) = (
                args.get(2).map(PathBuf::from),
                args.get(3).map(PathBuf::from),
            ) else {
                eprintln!("usage: load-obj-colors <fichier.obj> <sortie.json>");
                std::process::exit(2);
            };
            let result = engine::adapters::ffi::load_obj_colors(&path).and_then(|colors| {
                let volume = &colors.model.objects[0].volumes[0];
                let mut mmu = serde_json::Map::new();
                for (channel, triangle, state) in &volume.paint.facets {
                    if *channel == engine::api::PaintChannel::Mmu {
                        mmu.insert(triangle.to_string(), state.clone().into());
                    }
                }
                let view = serde_json::json!({
                    "colored": colors.colored,
                    "palette": colors.palette,
                    "extruder": volume.extruder,
                    "triangles": volume.mesh.indices.len(),
                    "painting": { "mmu": mmu },
                });
                std::fs::write(&out, view.to_string()).map_err(|e| {
                    engine::api::EngineError::new(engine::api::EngineErrorCode::Io, e.to_string())
                })
            });
            match result {
                Ok(()) => emit_json_result(&serde_json::json!({ "path": out })),
                Err(e) => {
                    emit_error(&code_slug(e.code), &e.message);
                    std::process::exit(1);
                }
            }
        }
        Some("read-project-aux") => {
            // read-project-aux <fichier.3mf> <dossier> : fichiers auxiliaires.
            let (Some(path), Some(out)) = (
                args.get(2).map(PathBuf::from),
                args.get(3).map(PathBuf::from),
            ) else {
                eprintln!("usage: read-project-aux <fichier.3mf> <dossier>");
                std::process::exit(2);
            };
            match engine::adapters::ffi::read_project_auxiliaries(&path, &out) {
                Ok(files) => emit_json_result(&serde_json::json!({ "files": files })),
                Err(e) => {
                    emit_error(&code_slug(e.code), &e.message);
                    std::process::exit(1);
                }
            }
        }
        Some("calib-project") => {
            // calib-project <request.json> : { request, config, resources } → CalibProject
            let Some(path) = args.get(2).map(PathBuf::from) else {
                eprintln!("usage: calib-project <request.json>");
                std::process::exit(2);
            };
            #[derive(Deserialize)]
            struct Req {
                request: engine::adapters::ffi::CalibRequest,
                config: engine::api::DynamicPrintConfig,
                resources: PathBuf,
            }
            let result = read_json_bounded::<Req>(&path).and_then(|r| {
                engine::adapters::ffi::calib_project(&r.request, &r.config, &r.resources)
            });
            match result {
                Ok(project) => emit_json_result(&project),
                Err(e) => {
                    emit_error(&code_slug(e.code), &e.message);
                    std::process::exit(1);
                }
            }
        }
        Some("flush-volumes") => {
            let Some(path) = args.get(2).map(PathBuf::from) else {
                eprintln!("usage: flush-volumes <request.json>");
                std::process::exit(2);
            };
            let result = read_json_bounded::<FlushVolumesRequest>(&path).and_then(|mut r| {
                if let Some(config) = &r.config {
                    use engine::api::ConfigValue;
                    if r.min_flush.is_empty() {
                        r.min_flush = engine::adapters::ffi::min_flush_volumes(config, 0)?;
                    }
                    if r.flush_dataset == 0 {
                        if let Some(ConfigValue::Ints(v)) = config.get("nozzle_flush_dataset") {
                            r.flush_dataset = v.first().copied().unwrap_or(0) as i32;
                        }
                    }
                    if r.is_support.is_empty() {
                        if let Some(ConfigValue::Bools(v)) = config.get("filament_is_support") {
                            r.is_support = v.clone();
                        }
                    }
                }
                engine::adapters::ffi::flush_volumes(
                    &r.colours,
                    &r.is_support,
                    &r.min_flush,
                    r.flush_dataset,
                )
            });
            match result {
                Ok(matrix) => emit_json_result(&serde_json::json!({ "matrix": matrix })),
                Err(e) => {
                    emit_error(&code_slug(e.code), &e.message);
                    std::process::exit(1);
                }
            }
        }
        Some("fill-bed") => {
            let Some(path) = args.get(2).map(PathBuf::from) else {
                eprintln!("usage: fill-bed <request.json>");
                std::process::exit(2);
            };
            let result = read_json_bounded::<FillBedRequest>(&path).and_then(|r| {
                engine::adapters::ffi::fill_bed(
                    &r.model,
                    &r.bed,
                    &r.config,
                    r.object_index,
                    r.clearance,
                )
            });
            match result {
                Ok(m) => emit_json_result(&FillBedResponse {
                    instances: m.iter().map(|m| m.to_cols_array()).collect(),
                }),
                Err(e) => {
                    emit_error(&code_slug(e.code), &e.message);
                    std::process::exit(1);
                }
            }
        }
        Some("export-project-3mf") => {
            let Some(path) = args.get(2).map(PathBuf::from) else {
                eprintln!("usage: export-project-3mf <request.json>");
                std::process::exit(2);
            };
            match run_export_project_3mf(&path) {
                Ok(result) => emit_json_result(&result),
                Err(e) => {
                    emit_error(&code_slug(e.code), &e.message);
                    std::process::exit(1);
                }
            }
        }
        Some("export-gcode-3mf") => {
            let Some(path) = args.get(2).map(PathBuf::from) else {
                eprintln!("usage: export-gcode-3mf <request.json>");
                std::process::exit(2);
            };
            match run_export_gcode_3mf(&path) {
                Ok(result) => emit_json_result(&result),
                Err(e) => {
                    emit_error(&code_slug(e.code), &e.message);
                    std::process::exit(1);
                }
            }
        }
        Some("export-meshes") => {
            let Some(path) = args.get(2).map(PathBuf::from) else {
                eprintln!("usage: export-meshes <request.json>");
                std::process::exit(2);
            };
            match run_export_meshes(&path) {
                Ok(result) => emit_json_result(&result),
                Err(e) => {
                    emit_error(&code_slug(e.code), &e.message);
                    std::process::exit(1);
                }
            }
        }
        Some("export-drc") => {
            let Some(path) = args.get(2).map(PathBuf::from) else {
                eprintln!("usage: export-drc <request.json>");
                std::process::exit(2);
            };
            match run_export_drc(&path) {
                Ok(result) => emit_json_result(&result),
                Err(e) => {
                    emit_error(&code_slug(e.code), &e.message);
                    std::process::exit(1);
                }
            }
        }
        Some("self-test") => self_test(&args[2..]),
        _ => {
            eprintln!(
                "usage: engine-worker <triangle-count|config-count|load-model|slice|arrange|repair|simplify|self-test> [args]"
            );
            std::process::exit(2);
        }
    }
}

fn run_arrange(path: &Path) -> engine::api::EngineResult<ArrangeResponse> {
    use engine::api::{EngineError, EngineErrorCode};
    let mut request: ArrangeRequest = read_json_bounded(path)?;
    let instance_count = request
        .model
        .objects
        .iter()
        .map(|object| object.instances.len())
        .sum::<usize>();
    if request.instance_ids.len() != instance_count {
        return Err(EngineError::new(
            EngineErrorCode::InvalidModel,
            "le nombre d'identifiants ne correspond pas aux instances",
        ));
    }
    if request
        .instance_ids
        .iter()
        .collect::<std::collections::HashSet<_>>()
        .len()
        != instance_count
    {
        return Err(EngineError::new(
            EngineErrorCode::InvalidModel,
            "identifiants d'instances dupliqués",
        ));
    }
    let orders =
        engine::adapters::ffi::arrange_ordered(&mut request.model, &request.bed, &request.params)?;
    let mut orders = orders.into_iter();
    let mut ids = request.instance_ids.into_iter();
    let mut placements = Vec::with_capacity(instance_count);
    for object in request.model.objects {
        for instance in object.instances {
            let matrix = instance.matrix.to_cols_array();
            if matrix.iter().any(|value| !value.is_finite()) {
                return Err(EngineError::new(
                    EngineErrorCode::Internal,
                    "matrice non finie produite par l'arrangeur",
                ));
            }
            placements.push(ArrangedInstance {
                id: ids.next().expect("nombre d'identifiants validé"),
                plate_index: instance.plate_index,
                matrix,
                print_order: orders.next().flatten(),
            });
        }
    }
    Ok(ArrangeResponse { placements })
}

fn run_repair(path: &Path) -> engine::api::EngineResult<RepairResponse> {
    let request: RepairRequest = read_json_bounded(path)?;
    let (repaired, repairs) = engine::adapters::ffi::repair_mesh(&request.mesh)?;
    write_display_mesh(&request.output_path, &repaired.encode_display(), "réparé")?;
    Ok(RepairResponse {
        mesh_path: request.output_path,
        repairs,
    })
}

/// « Garder la peinture » (keep_painting) : remap_painting par canal.
fn run_remap_paint(
    path: &Path,
) -> engine::api::EngineResult<engine::api::paint::RemapPaintResponse> {
    use engine::api::paint::{RemapPaintRequest, RemapPaintResponse, PAINT_CHANNEL_KEYS};
    let request: RemapPaintRequest = read_json_bounded(path)?;
    let mut out = RemapPaintResponse::default();
    for (channel, facets) in &request.channels {
        if !PAINT_CHANNEL_KEYS.contains(&channel.as_str()) {
            return Err(engine::api::EngineError::new(
                engine::api::EngineErrorCode::InvalidConfig,
                format!("canal de peinture inconnu : {channel}"),
            ));
        }
        let remapped =
            engine::adapters::ffi::remap_painting(&request.source, &request.target, facets)?;
        out.channels.insert(channel.clone(), remapped);
    }
    Ok(out)
}

fn run_simplify(path: &Path) -> engine::api::EngineResult<SimplifyResponse> {
    let request: SimplifyRequest = read_json_bounded(path)?;
    // Le paramètre est revalidé côté moteur (`SimplifyParams::target`) : le
    // worker ne fait pas confiance à la validation de l'appelant.
    let (simplified, report) =
        engine::adapters::ffi::simplify_mesh(&request.mesh, &request.params)?;
    write_display_mesh(
        &request.output_path,
        &simplified.encode_display(),
        "simplifié",
    )?;
    Ok(SimplifyResponse {
        mesh_path: request.output_path,
        report,
    })
}

fn run_export_project_3mf(path: &Path) -> engine::api::EngineResult<Gcode3mfResponse> {
    let request: Project3mfRequest = read_json_bounded(path)?;
    engine::adapters::ffi::write_project_3mf_with_auxiliaries(
        &request.model,
        &request.config,
        &request.plates,
        request.aux_dir.as_deref().filter(|p| p.is_dir()),
        &request.output_path,
    )?;
    Ok(Gcode3mfResponse {
        path: request.output_path,
    })
}

fn run_export_gcode_3mf(path: &Path) -> engine::api::EngineResult<Gcode3mfResponse> {
    let request: Gcode3mfRequest = read_json_bounded(path)?;
    engine::adapters::ffi::write_gcode_3mf(
        &request.model,
        &request.config,
        &request.gcodes,
        request.export_plate,
        &request.output_path,
    )?;
    Ok(Gcode3mfResponse {
        path: request.output_path,
    })
}

fn run_export_meshes(path: &Path) -> engine::api::EngineResult<DrcExportResponse> {
    let request: ExportMeshesRequest = read_json_bounded(path)?;
    let meshes = engine::adapters::ffi::export_meshes(&request.model)?;
    let mut files = Vec::with_capacity(meshes.len());
    for (index, mesh) in meshes.iter().enumerate() {
        let file = request.output_dir.join(format!("part-{index}.wsm"));
        write_display_mesh(&file, &mesh.encode_display(), "export")?;
        files.push(file);
    }
    Ok(DrcExportResponse { files })
}

fn run_export_drc(path: &Path) -> engine::api::EngineResult<DrcExportResponse> {
    let request: DrcExportRequest = read_json_bounded(path)?;
    let mut files = Vec::with_capacity(request.meshes.len());
    for (index, mesh) in request.meshes.iter().enumerate() {
        let file = request.output_dir.join(format!("part-{index}.drc"));
        engine::adapters::ffi::store_drc(mesh, &file)?;
        files.push(file);
    }
    Ok(DrcExportResponse { files })
}

/// `mesh-op` : exécute l'opération (revalidée côté moteur) et écrit chaque
/// pièce en WSMh `<output_dir>/part-<i>.wsm`.
fn run_mesh_op(path: &Path) -> engine::api::EngineResult<engine::api::split::MeshOpResponse> {
    use engine::api::split::{MeshOpPart, MeshOpRequest, MeshOpResponse};
    let request: MeshOpRequest = read_json_bounded(path)?;
    let pieces = engine::adapters::ffi::run_mesh_op_pieces(&request.op, &request.meshes)?;
    let mut out = Vec::with_capacity(pieces.len());
    for (index, piece) in pieces.into_iter().enumerate() {
        let mesh_path = request.output_dir.join(format!("part-{index}.wsm"));
        write_display_mesh(&mesh_path, &piece.mesh.encode_display(), "découpé")?;
        out.push(MeshOpPart {
            label: piece.label,
            mesh_path,
            triangles: piece.mesh.triangle_count() as u32,
            vertices: piece.mesh.vertices.len() as u32,
            volume: piece.volume,
        });
    }
    Ok(MeshOpResponse { parts: out })
}

/// Écrit un WSMh produit par une opération de maillage, borné à 512 Mio.
fn write_display_mesh(path: &Path, bytes: &[u8], what: &str) -> engine::api::EngineResult<()> {
    use engine::api::{EngineError, EngineErrorCode};
    if bytes.len() > MAX_REPAIRED_MESH_BYTES {
        return Err(EngineError::new(
            EngineErrorCode::InvalidModel,
            format!("maillage {what} au-delà de la limite de 512 Mio"),
        ));
    }
    std::fs::write(path, bytes).map_err(|e| {
        EngineError::new(
            EngineErrorCode::Io,
            format!("écriture du maillage {what} : {e}"),
        )
    })
}

fn read_json_bounded<T: serde::de::DeserializeOwned>(path: &Path) -> engine::api::EngineResult<T> {
    use engine::api::{EngineError, EngineErrorCode};
    let invalid = |message: String| EngineError::new(EngineErrorCode::InvalidModel, message);
    let metadata = std::fs::metadata(path)
        .map_err(|e| invalid(format!("lecture des métadonnées de la requête : {e}")))?;
    if metadata.len() > MAX_SCENE_REQUEST_BYTES {
        return Err(invalid(
            "requête JSON au-delà de la limite de 512 Mio".into(),
        ));
    }
    let bytes = std::fs::read(path).map_err(|e| invalid(format!("lecture de la requête : {e}")))?;
    if bytes.len() as u64 > MAX_SCENE_REQUEST_BYTES {
        return Err(invalid(
            "requête JSON au-delà de la limite de 512 Mio".into(),
        ));
    }
    serde_json::from_slice(&bytes).map_err(|e| invalid(format!("requête JSON invalide : {e}")))
}

fn emit_json_result<T: Serialize>(result: &T) {
    match serde_json::to_string(result) {
        Ok(json) if json.len() <= MAX_SCENE_RESULT_BYTES => {
            println!("R {json}");
            let _ = std::io::stdout().flush();
        }
        Ok(_) => {
            emit_error("internal", "résultat JSON au-delà de la limite de 16 Mio");
            std::process::exit(1);
        }
        Err(e) => {
            emit_error("internal", &format!("sérialisation du résultat : {e}"));
            std::process::exit(1);
        }
    }
}

/// Décode `path` (format `format`) via le moteur et renvoie le maillage
/// d'affichage encodé « WSMh » (T120). Toute erreur remonte en `EngineError`
/// (jamais de panique non gérée : un fichier corrompu doit produire un code de
/// sortie ≠ 0, pas abattre le worker).
fn load_model_raw_json(path: &Path, format: &str) -> engine::api::EngineResult<Vec<u8>> {
    use engine::api::{EngineError, EngineErrorCode};
    let ext = format_extension(format).ok_or_else(|| {
        EngineError::new(
            EngineErrorCode::InvalidModel,
            format!("format non supporté : {format}"),
        )
    })?;
    let (readable, temp) = readable_path(path, ext).map_err(|e| {
        EngineError::new(
            EngineErrorCode::InvalidModel,
            format!("préparation du fichier : {e}"),
        )
    })?;
    let result = engine::adapters::ffi::load_model(&readable)
        .map(|model| serde_json::to_vec(&model).expect("Model sérialisable"));
    if let Some(tmp) = temp {
        let _ = std::fs::remove_file(tmp);
    }
    result
}

fn load_model_to_wsmh(path: &Path, format: &str) -> engine::api::EngineResult<Vec<u8>> {
    use engine::api::{EngineError, EngineErrorCode};
    let ext = format_extension(format).ok_or_else(|| {
        EngineError::new(
            EngineErrorCode::InvalidModel,
            format!("format non supporté : {format}"),
        )
    })?;
    // libslic3r détecte le format par l'extension du fichier : si le chemin de
    // stockage n'en porte pas (upload nommé par UUID), on lit une copie
    // temporaire correctement suffixée.
    let (readable, temp) = readable_path(path, ext).map_err(|e| {
        EngineError::new(
            EngineErrorCode::InvalidModel,
            format!("préparation du fichier : {e}"),
        )
    })?;
    let result =
        engine::adapters::ffi::convert_to_mesh(&readable).map(|mesh| mesh.encode_display());
    if let Some(tmp) = temp {
        let _ = std::fs::remove_file(tmp);
    }
    result
}

/// Écrit le WSMh dans un fichier temporaire propre au process et renvoie son
/// chemin (émis en ligne `R`). Le parent le lit puis le supprime.
fn write_temp_wsmh(bytes: Vec<u8>) -> engine::api::EngineResult<PathBuf> {
    use engine::api::{EngineError, EngineErrorCode};
    let path = std::env::temp_dir().join(format!("wsm-load-{}.bin", std::process::id()));
    std::fs::write(&path, &bytes).map_err(|e| {
        EngineError::new(
            EngineErrorCode::EngineCrashed,
            format!("écriture du maillage : {e}"),
        )
    })?;
    Ok(path)
}

/// Extension canonique attendue par libslic3r pour un nom de format (T120 :
/// STL/OBJ/3MF/STEP/AMF). `None` = format non pris en charge en conversion.
fn format_extension(format: &str) -> Option<&'static str> {
    match format.to_ascii_lowercase().as_str() {
        "stl" | "oltp" => Some("stl"),
        "obj" => Some("obj"),
        "3mf" | "threemf" => Some("3mf"),
        "step" | "stp" => Some("step"),
        "amf" => Some("amf"),
        // Draco (modèles de calibration Orca) : Model::read_from_file → load_drc.
        "drc" => Some("drc"),
        // SVG : Model::read_from_file → load_svg (profils extrudés via OCCT, Orca).
        "svg" => Some("svg"),
        _ => None,
    }
}

/// Chemin lisible par libslic3r pour l'extension `ext` : le chemin d'origine si
/// son extension correspond déjà, sinon une copie temporaire suffixée (second
/// membre `Some` = fichier à supprimer après lecture).
fn readable_path(path: &Path, ext: &str) -> std::io::Result<(PathBuf, Option<PathBuf>)> {
    let already = path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case(ext));
    if already {
        return Ok((path.to_path_buf(), None));
    }
    let stem = path.file_name().and_then(|n| n.to_str()).unwrap_or("model");
    let tmp = std::env::temp_dir().join(format!("wsm-load-{}-{stem}.{ext}", std::process::id()));
    std::fs::copy(path, &tmp)?;
    Ok((tmp.clone(), Some(tmp)))
}

/// Émet une progression monotone déterministe puis un résultat, en simulant
/// les callbacks statusbar. `--crash` interrompt brutalement le process (test
/// « crash contenu ») ; `--hang` boucle sans fin (test « annulation = kill »).
fn self_test(args: &[String]) {
    let mut crash = false;
    let mut hang = false;
    let mut stderr_flood = false;
    let mut work_dir: Option<std::path::PathBuf> = None;
    for arg in args {
        match arg.as_str() {
            "--crash" => crash = true,
            "--hang" => hang = true,
            // Journal abondant sur stderr (Boost.Log) : le parent doit le
            // vider, sinon le pipe plein bloque le worker indéfiniment.
            "--stderr-flood" => stderr_flood = true,
            other => work_dir = Some(std::path::PathBuf::from(other)),
        }
    }
    let work_dir = work_dir.unwrap_or_else(std::env::temp_dir);

    let phases = [
        (0.0_f32, "démarrage"),
        (0.25, "tranchage des périmètres"),
        (0.50, "remplissage"),
        (0.75, "génération des parcours"),
        (1.0, "export G-code"),
    ];
    for (i, (ratio, phase)) in phases.iter().enumerate() {
        progress(*ratio, phase);
        if stderr_flood && i == 1 {
            let line = "x".repeat(1023);
            let mut stderr = std::io::stderr().lock();
            for _ in 0..1024 {
                let _ = writeln!(stderr, "{line}");
            }
        }
        if crash && i == 1 {
            // crash C++ typique : abandon du process (pas de ligne R/E).
            std::process::abort();
        }
        if hang && i == 1 {
            // le parent doit annuler en tuant ce process.
            loop {
                std::thread::sleep(std::time::Duration::from_secs(3600));
            }
        }
    }

    let gcode = work_dir.join("self-test.gcode");
    if let Err(e) = std::fs::write(&gcode, b"; self-test gcode\n") {
        emit_error("io", &format!("écriture du G-code de test : {e}"));
        std::process::exit(1);
    }
    let result = serde_json::json!({
        "gcode_path": gcode,
        "stats": {
            "estimated_time_s": 1.0,
            "filament_mm": [1.0],
            "filament_g": [0.1],
            "layer_count": 1,
            "tool_changes": 0
        },
        "thumbnails": []
    });
    println!("R {result}");
    let _ = std::io::stdout().flush();
}

fn progress(ratio: f32, phase: &str) {
    println!("P {ratio} {phase}");
    let _ = std::io::stdout().flush();
}

fn emit_error(code: &str, message: &str) {
    // le message peut contenir des sauts de ligne : une seule ligne attendue.
    let message = message.replace('\n', " ");
    println!("E {code} {message}");
    let _ = std::io::stdout().flush();
}

/// Slug snake_case du code d'erreur (symétrique de `worker::parse_code`).
fn code_slug(code: engine::api::EngineErrorCode) -> String {
    serde_json::to_value(code)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_else(|| "engine_crashed".to_string())
}
