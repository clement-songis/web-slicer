//! Adaptateur principal : bridge cxx vers libslic3r-headless (R1).
//!
//! Chaque domaine a son module Rust (remappage) et son .cpp (wrapper).
//! Les opérations lourdes passeront par le process `engine-worker` (T018).

mod arrange;
mod bridge;
mod calib_project;
mod engine;
mod export_mesh;
mod flush;
mod gcode_process;
mod layer_profile;
mod mesh;
mod model;
mod obj_colors;
mod paint;
pub mod serve;
mod slice;
mod split;
mod threemf;
pub mod worker;

pub use arrange::{arrange, arrange_ordered, fill_bed, orient};
pub use calib_project::{calib_project, CalibProject, CalibRequest, CALIB_KINDS};
pub use engine::FfiEngine;
pub use export_mesh::export_meshes;
pub use flush::{flush_volumes, min_flush_volumes};
pub use gcode_process::process_gcode;
pub use layer_profile::layer_profile;
pub use mesh::{repair_mesh, simplify_mesh, store_drc};
pub use model::{
    convert_to_mesh, convert_to_mesh_model, load_model, load_step, model_triangle_count,
    print_config_option_count, StepMeshOptions,
};
pub use obj_colors::{load_obj_colors, ObjColors};
pub use paint::{remap_painting, PaintSession};
pub use slice::{run_in_worker, slice};
pub use split::run_mesh_op;
pub use threemf::{
    read_project_3mf, read_project_3mf_import, read_project_auxiliaries, write_gcode_3mf,
    write_project_3mf, write_project_3mf_with_auxiliaries, write_project_3mf_with_plates,
    PlateGcode,
};
