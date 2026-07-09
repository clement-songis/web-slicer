//! Adaptateur principal : bridge cxx vers libslic3r-headless (R1).
//!
//! Chaque domaine a son module Rust (remappage) et son .cpp (wrapper).
//! Les opérations lourdes passeront par le process `engine-worker` (T018).

mod bridge;
mod model;

pub use model::{convert_to_mesh, load_model, model_triangle_count, print_config_option_count};
