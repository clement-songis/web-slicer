//! Déclaration cxx du bridge vers libslic3r (T012 — périmètre smoke test,
//! étendu par domaine en T013+).
//!
//! Convention : les fonctions C++ jettent des exceptions (std::exception) ;
//! cxx les convertit en `Result` côté Rust.

#[cxx::bridge(namespace = "webslicer")]
pub(crate) mod ffi {
    unsafe extern "C++" {
        include!("engine/src/adapters/ffi/bridge/model.hpp");

        /// Charge un fichier modèle via `Slic3r::Model::read_from_file` et
        /// retourne le nombre total de triangles (smoke test de la chaîne).
        fn model_triangle_count(path: &str) -> Result<usize>;

        /// Nombre d'options du `print_config_def` runtime (croisement avec
        /// le registre généré — même contrôle que dump-config).
        fn print_config_option_count() -> usize;
    }
}
