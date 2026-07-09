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
//! - `self-test [--crash|--hang] <work_dir>` : exerce le protocole de T018
//!   sans dépendre du pipeline de tranchage (branché en T019).

use std::io::Write;

fn main() {
    let args: Vec<String> = std::env::args().collect();
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
        Some("self-test") => self_test(&args[2..]),
        _ => {
            eprintln!("usage: engine-worker <triangle-count|config-count|self-test> [args]");
            std::process::exit(2);
        }
    }
}

/// Émet une progression monotone déterministe puis un résultat, en simulant
/// les callbacks statusbar. `--crash` interrompt brutalement le process (test
/// « crash contenu ») ; `--hang` boucle sans fin (test « annulation = kill »).
fn self_test(args: &[String]) {
    let mut crash = false;
    let mut hang = false;
    let mut work_dir: Option<std::path::PathBuf> = None;
    for arg in args {
        match arg.as_str() {
            "--crash" => crash = true,
            "--hang" => hang = true,
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
    println!("E {code} {message}");
    let _ = std::io::stdout().flush();
}
