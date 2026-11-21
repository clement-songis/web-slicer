//! Couche HTTP : DTO typés (exportés vers le frontend via ts-rs), état
//! applicatif, extracteurs de session et routes/handlers minces.

pub mod dto;
pub mod envelope;
pub mod error;
pub mod extract;
pub mod preset_cache;
pub mod printer_relay;
pub mod quota;
pub mod routes;
pub mod state;
pub mod ws;
