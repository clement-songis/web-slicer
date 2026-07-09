//! Adaptateur principal : bridge cxx vers libslic3r-headless (R1).
//!
//! Chaque domaine a son module Rust (remappage) et son .cpp (wrapper).
//! Les opérations lourdes passeront par le process `engine-worker` (T018).

mod bridge;
mod mesh;
mod model;
mod threemf;

pub use mesh::repair_mesh;
pub use model::{convert_to_mesh, load_model, model_triangle_count, print_config_option_count};
pub use threemf::{read_project_3mf, write_project_3mf};
