//! Assemblage du routeur HTTP. Les handlers sont minces et délèguent au
//! domaine ; la session est posée par une couche `tower-sessions`.

pub mod auth;

use axum::routing::{get, post};
use axum::{Json, Router};
use tower_sessions::{SessionManagerLayer, SessionStore};

use super::dto::HealthResponse;
use super::state::AppState;

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse::ok())
}

/// Construit le routeur complet à partir de l'état applicatif et de la couche
/// de session (générique sur le store : SQLite en prod, mémoire en test).
pub fn router<Store>(state: AppState, session_layer: SessionManagerLayer<Store>) -> Router
where
    Store: SessionStore + Clone + 'static,
{
    Router::new()
        .route("/api/health", get(health))
        .route("/api/auth/register", post(auth::register))
        .route("/api/auth/login", post(auth::login))
        .route("/api/auth/logout", post(auth::logout))
        .route("/api/auth/me", get(auth::me))
        .layer(session_layer)
        .with_state(state)
}
