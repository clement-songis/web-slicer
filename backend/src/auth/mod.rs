//! Authentification : hachage argon2id, inscription selon la politique
//! d'instance, vérification des identifiants. Les sessions (cookies) sont
//! gérées par `tower-sessions` au niveau HTTP.

pub mod password;
pub mod service;

pub use password::{hash_password, verify_password, PasswordError};
pub use service::{authenticate, register, AuthError};
