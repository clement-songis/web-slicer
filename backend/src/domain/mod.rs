//! Domaine métier (constitution I & III) : entités et traits de stockage,
//! sans aucune dépendance aux adaptateurs (SQL, HTTP, filesystem).

pub mod entities;
pub mod error;
pub mod id;
pub mod repo;

pub use entities::*;
pub use error::{StorageError, StorageResult};
pub use id::*;
pub use repo::*;
