//! Backend Web-Slicer — exposé en lib pour les tests d'intégration.
//!
//! Architecture (constitution I) : `domain` ne dépend d'aucun adaptateur ;
//! `http` ne contient que des handlers minces et des DTO ; les adaptateurs
//! (storage, files, moonraker) implémentent les traits du domaine.

pub mod adapters;
pub mod auth;
pub mod domain;
pub mod engine;
pub mod gcode;
pub mod gcode_filename;
pub mod http;
pub mod mesh;
pub mod models;
pub mod print_host_url;
pub mod queue;
pub mod scene;
pub mod server;
pub mod slice_signature;
pub mod slicing;
pub mod thumbnail;
pub mod thumbnails;
pub mod workdir;
