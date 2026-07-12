//! Intégration du moteur de slicing via son process isolé `engine-worker`.

pub mod worker;

pub use worker::{run_worker, run_worker_at, worker_binary, WorkerError, DEFAULT_TIMEOUT};
