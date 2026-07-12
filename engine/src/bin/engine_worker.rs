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
//! - `self-test [--crash|--hang] <work_dir>` : exerce le protocole de T018
//!   sans dépendre du pipeline de tranchage (branché en T019).

use std::io::Write;
use std::path::{Path, PathBuf};

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
        Some("self-test") => self_test(&args[2..]),
        _ => {
            eprintln!(
                "usage: engine-worker <triangle-count|config-count|load-model|slice|self-test> [args]"
            );
            std::process::exit(2);
        }
    }
}

/// Décode `path` (format `format`) via le moteur et renvoie le maillage
/// d'affichage encodé « WSMh » (T120). Toute erreur remonte en `EngineError`
/// (jamais de panique non gérée : un fichier corrompu doit produire un code de
/// sortie ≠ 0, pas abattre le worker).
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
