//! Assemblage du routeur HTTP. Les handlers sont minces et délèguent au
//! domaine ; la session est posée par une couche `tower-sessions`.

pub mod admin;
pub mod auth;
pub mod models;
pub mod presets;
pub mod projects;
pub mod scene;
pub mod slice;

use axum::extract::DefaultBodyLimit;
use axum::routing::{get, patch, post};
use axum::{Json, Router};
use tower_sessions::{SessionManagerLayer, SessionStore};

use super::dto::HealthResponse;
use super::state::AppState;
use super::ws;

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
        .route(
            "/api/projects/{id}/models",
            post(models::upload).layer(DefaultBodyLimit::max(models::MAX_BODY_BYTES)),
        )
        .route("/api/models/{id}/mesh", get(models::mesh))
        .route("/api/models/{id}/repair", post(scene::repair))
        .route("/api/projects/{id}/arrange", post(scene::arrange))
        .route("/api/projects/{id}/orient", post(scene::orient))
        .route("/api/projects/{id}/slice", post(slice::slice))
        .route("/api/ws", get(ws::ws))
        .route("/api/presets", get(presets::list).post(presets::create))
        .route(
            "/api/presets/{id}",
            get(presets::get)
                .put(presets::update)
                .delete(presets::delete),
        )
        .route("/api/presets/{id}/resolved", get(presets::resolved))
        .route("/api/presets/{id}/export", get(presets::export))
        .route("/api/presets/import", post(presets::import))
        .layer(session_layer)
        .with_state(state)
}
