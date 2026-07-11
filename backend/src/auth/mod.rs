//! Authentification : hachage argon2id, inscription selon la politique
//! d'instance, vérification des identifiants. Les sessions (cookies) sont
//! gérées par `tower-sessions` au niveau HTTP.

pub mod password;
pub mod secrets;
pub mod service;

pub use password::{hash_password, verify_password, PasswordError};
pub use secrets::SecretBox;
pub use service::{
    authenticate, create_invitation, create_managed_user, delete_account, register, reset_password,
    AuthError,
};
