//! Adaptateur principal : bridge cxx vers libslic3r-headless (R1).
//!
//! Chaque domaine a son module Rust (remappage) et son .cpp (wrapper).
//! Les opérations lourdes passeront par le process `engine-worker` (T018).

mod arrange;
mod bridge;
mod engine;
mod mesh;
mod model;
mod slice;
mod threemf;
pub mod worker;

pub use arrange::{arrange, orient};
pub use engine::FfiEngine;
pub use mesh::repair_mesh;
pub use model::{convert_to_mesh, load_model, model_triangle_count, print_config_option_count};
pub use slice::{run_in_worker, slice};
pub use threemf::{read_project_3mf, write_project_3mf};
