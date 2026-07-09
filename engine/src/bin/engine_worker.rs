//! Process worker : exécute les opérations libslic3r isolées du backend
//! (crash C++ contenu, annulation par kill — R1/T018).
//!
//! T012 : smoke uniquement. Protocole IPC complet en T018.

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
        _ => {
            eprintln!("usage: engine-worker <triangle-count|config-count> [args]");
            std::process::exit(2);
        }
    }
}
