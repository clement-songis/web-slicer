//! Intégration du moteur de slicing via son process isolé `engine-worker`.

pub mod drc;
pub mod export_meshes;
pub mod gcode_3mf;
pub mod gcode_process;
pub mod layer_profile;
pub mod mesh_ops;
pub mod project_3mf;
pub mod raw_model;
pub mod scene;
pub mod serve_pool;
pub mod worker;

pub use worker::{
    run_worker, run_worker_at, run_worker_warm_at, worker_binary, WorkerError, DEFAULT_TIMEOUT,
};
