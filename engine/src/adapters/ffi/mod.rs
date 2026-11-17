//! Adaptateur principal : bridge cxx vers libslic3r-headless (R1).
//!
//! Chaque domaine a son module Rust (remappage) et son .cpp (wrapper).
//! Les opérations lourdes passeront par le process `engine-worker` (T018).

mod arrange;
mod bridge;
mod engine;
mod export_mesh;
mod flush;
mod gcode_process;
mod layer_profile;
mod mesh;
mod model;
mod paint;
pub mod serve;
mod slice;
mod split;
mod threemf;
pub mod worker;

pub use arrange::{arrange, arrange_ordered, fill_bed, orient};
pub use engine::FfiEngine;
pub use export_mesh::export_meshes;
pub use flush::{flush_volumes, min_flush_volumes};
pub use gcode_process::process_gcode;
pub use layer_profile::layer_profile;
pub use mesh::{repair_mesh, simplify_mesh, store_drc};
pub use model::{convert_to_mesh, load_model, model_triangle_count, print_config_option_count};
pub use paint::PaintSession;
pub use slice::{run_in_worker, slice};
pub use split::run_mesh_op;
pub use threemf::{
    read_project_3mf, read_project_3mf_import, write_gcode_3mf, write_project_3mf,
    write_project_3mf_with_plates, PlateGcode,
};
