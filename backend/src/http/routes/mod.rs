//! Assemblage du routeur HTTP. Les handlers sont minces et délèguent au
//! domaine ; la session est posée par une couche `tower-sessions`.

pub mod admin;
pub mod auth;
pub mod projects;

use axum::routing::{get, patch, post};
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
        .route("/api/auth/me", get(auth::me).delete(auth::delete_me))
        .route(
            "/api/admin/instance",
            get(admin::get_instance).patch(admin::patch_instance),
        )
        .route("/api/admin/users", post(admin::create_user))
        .route(
            "/api/admin/users/{id}",
            axum::routing::delete(admin::delete_user),
        )
        .route(
            "/api/admin/users/{id}/reset-password",
            post(admin::reset_password),
        )
        .route("/api/admin/invitations", post(admin::create_invitation))
        .route("/api/admin/presets/reseed", post(admin::reseed_presets))
        .route("/api/projects", get(projects::list).post(projects::create))
        .route(
            "/api/projects/{id}",
            get(projects::get)
                .put(projects::save)
                .delete(projects::delete),
        )
        .route("/api/projects/{id}/duplicate", post(projects::duplicate))
        .route("/api/projects/{id}/rename", patch(projects::rename))
        .route("/api/projects/{id}/thumbnail", get(projects::thumbnail))
        .layer(session_layer)
        .with_state(state)
}
