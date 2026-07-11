//! Backend Web-Slicer — exposé en lib pour les tests d'intégration.
//!
//! Architecture (constitution I) : `domain` ne dépend d'aucun adaptateur ;
//! `http` ne contient que des handlers minces et des DTO ; les adaptateurs
//! (storage, files, moonraker) implémentent les traits du domaine.

pub mod adapters;
pub mod auth;
pub mod domain;
pub mod http;
pub mod mesh;
pub mod queue;
pub mod scene;
pub mod server;
