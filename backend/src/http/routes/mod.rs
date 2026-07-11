//! Assemblage du routeur HTTP. Les handlers sont minces et délèguent au
//! domaine ; la session est posée par une couche `tower-sessions`.

pub mod admin;
pub mod auth;
pub mod export;
pub mod gcodes;
pub mod jobs;
pub mod models;
pub mod presets;
pub mod preview;
pub mod printers;
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
            "/api/projects/import",
            post(projects::import).layer(DefaultBodyLimit::max(models::MAX_BODY_BYTES)),
        )
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
        .route("/api/projects/{id}/export/3mf", get(export::export_3mf))
        .route("/api/jobs", get(jobs::list))
        .route("/api/jobs/{id}", get(jobs::get))
        .route("/api/jobs/{id}/cancel", post(jobs::cancel))
        .route("/api/gcodes/{id}/download", get(gcodes::download))
        .route("/api/gcodes/{id}/stats", get(gcodes::stats))
        .route("/api/gcodes/{id}/preview/meta", get(preview::meta))
        .route("/api/gcodes/{id}/preview/layers", get(preview::layers))
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
        .route("/api/printers", get(printers::list).post(printers::create))
        .route(
            "/api/printers/{id}",
            get(printers::get)
                .put(printers::update)
                .delete(printers::delete),
        )
        .route("/api/printers/{id}/test", post(printers::test))
        .route("/api/printers/{id}/upload", post(printers::upload))
        .route("/api/printers/{id}/status", get(printers::status))
        .route("/api/printers/{id}/pause", post(printers::pause))
        .route("/api/printers/{id}/resume", post(printers::resume))
        .route("/api/printers/{id}/cancel", post(printers::cancel))
        .layer(session_layer)
        .with_state(state)
}
