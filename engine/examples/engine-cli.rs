//! Démo P1 (T023) : exerce le trait `SlicerEngine` (implémentation `FfiEngine`)
//! depuis le shell — chargement, réparation, arrangement, tranchage, préviz
//! G-code. Prouve que le moteur est démontrable indépendamment du backend.
//!
//! Exemples :
//! ```sh
//! cargo run -p engine --features ffi --example engine-cli -- slice tests/fixtures/cube20.stl
//! cargo run -p engine --features ffi --example engine-cli -- repair tests/fixtures/cube20.stl
//! cargo run -p engine --features ffi --example engine-cli -- arrange tests/fixtures/cube20.stl
//! cargo run -p engine --features ffi --example engine-cli -- parse plate.gcode
//! ```

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use engine::adapters::ffi::FfiEngine;
use engine::api::{
    ArrangeParams, BuildVolume, CancelToken, ConfigValue, DynamicPrintConfig, ModelFormat,
    ProgressSink, SliceRequest,
};
use engine::SlicerEngine;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let (command, path) = match (args.get(1), args.get(2)) {
        (Some(cmd), Some(path)) => (cmd.as_str(), PathBuf::from(path)),
        _ => {
            eprintln!("usage: engine-cli <slice|repair|arrange|parse> <fichier>");
            return ExitCode::from(2);
        }
    };

    let engine = FfiEngine;
    let result = match command {
        "slice" => run_slice(&engine, &path),
        "repair" => run_repair(&engine, &path),
        "arrange" => run_arrange(&engine, &path),
        "parse" => run_parse(&engine, &path),
        other => {
            eprintln!("commande inconnue : {other}");
            return ExitCode::from(2);
        }
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("erreur : {e}");
            ExitCode::FAILURE
        }
    }
}

fn load(engine: &FfiEngine, path: &Path) -> engine::api::EngineResult<engine::api::Model> {
    let format = ModelFormat::from_path(path).unwrap_or(ModelFormat::Stl);
    engine.load_model(path, format)
}

fn run_slice(engine: &FfiEngine, path: &Path) -> engine::api::EngineResult<()> {
    let model = load(engine, path)?;

    let mut config = DynamicPrintConfig::new();
    config.set("layer_height", ConfigValue::Float(0.2))?;

    let work_dir = std::env::temp_dir().join("engine-cli-slice");
    std::fs::create_dir_all(&work_dir)?;

    // Progression du statusbar libslic3r, remontée par le pipe du worker.
    let progress: ProgressSink = Box::new(|phase, ratio| {
        print!("\r  {:>3.0}%  {phase:<40}", ratio * 100.0);
        let _ = std::io::Write::flush(&mut std::io::stdout());
    });

    let out = engine.slice(
        SliceRequest {
            model,
            config,
            plate_index: 0,
            work_dir,
        },
        progress,
        CancelToken::new(),
    )?;

    println!("\rG-code : {}", out.gcode_path.display());
    println!(
        "  temps estimé : {:.0} s | couches : {} | filament : {:?} mm | outils : {}",
        out.stats.estimated_time_s,
        out.stats.layer_count,
        out.stats.filament_mm,
        out.stats.tool_changes
    );
    Ok(())
}

fn run_repair(engine: &FfiEngine, path: &Path) -> engine::api::EngineResult<()> {
    let model = load(engine, path)?;
    let mesh = &model.objects[0].volumes[0].mesh;
    let (repaired, report) = engine.repair_mesh(mesh)?;
    println!(
        "réparé : {} triangles ; corrections {report:?}",
        repaired.triangle_count()
    );
    Ok(())
}

fn run_arrange(engine: &FfiEngine, path: &Path) -> engine::api::EngineResult<()> {
    let mut model = load(engine, path)?;
    let bed = BuildVolume {
        bed_shape: vec![[0.0, 0.0], [200.0, 0.0], [200.0, 200.0], [0.0, 200.0]],
        max_height: 200.0,
        excluded: vec![],
    };
    engine.arrange(&mut model, &bed, &ArrangeParams::default())?;
    for (i, obj) in model.objects.iter().enumerate() {
        for inst in &obj.instances {
            let t = inst.matrix.w_axis;
            println!("objet {i} : instance en ({:.1}, {:.1})", t.x, t.y);
        }
    }
    Ok(())
}

fn run_parse(engine: &FfiEngine, path: &Path) -> engine::api::EngineResult<()> {
    let preview = engine.parse_gcode(path)?;
    println!(
        "préviz : {} couches | types {:?}",
        preview.layers.len(),
        preview.kinds_present
    );
    println!(
        "  temps estimé : {:.0} s | filament : {:?} mm",
        preview.stats.estimated_time_s, preview.stats.filament_mm
    );
    Ok(())
}
