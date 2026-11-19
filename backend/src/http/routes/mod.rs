//! Assemblage du routeur HTTP. Les handlers sont minces et délèguent au
//! domaine ; la session est posée par une couche `tower-sessions`.

pub mod admin;
pub mod auth;
pub mod auxiliaries;
pub mod calibration;
pub mod export;
pub mod flush;
pub mod gcode_open;
pub mod gcodes;
pub mod handy_models;
pub mod harness;
pub mod jobs;
pub mod mesh_ops;
pub mod metadata;
pub mod models;
pub mod paint;
pub mod preferences;
pub mod presets;
pub mod preview;
pub mod printers;
pub mod projects;
pub mod scene;
pub mod slice;
pub mod sliced;
pub mod validate;

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
    let app = Router::new()
        .route("/api/health", get(health))
        .route("/api/auth/register", post(auth::register))
        .route("/api/auth/login", post(auth::login))
        .route("/api/auth/logout", post(auth::logout))
        .route("/api/auth/me", get(auth::me).delete(auth::delete_me))
        .route(
            "/api/me/password",
            axum::routing::put(auth::change_password),
        )
        .route("/api/me/usage", get(auth::usage))
        .route(
            "/api/me/preferences",
            get(preferences::get).put(preferences::put),
        )
        .route(
            "/api/admin/instance",
            get(admin::get_instance).patch(admin::patch_instance),
        )
        .route(
            "/api/admin/users",
            get(admin::list_users).post(admin::create_user),
        )
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
            "/api/projects/{id}/import-3mf",
            post(projects::import_into).layer(DefaultBodyLimit::max(models::MAX_BODY_BYTES)),
        )
        .route(
            "/api/projects/{id}",
            get(projects::get)
                .put(projects::save)
                .patch(projects::patch)
                .delete(projects::delete),
        )
        .route("/api/projects/{id}/duplicate", post(projects::duplicate))
        .route("/api/projects/{id}/rename", patch(projects::rename))
        .route("/api/projects/{id}/thumbnail", get(projects::thumbnail))
        .route(
            "/api/projects/{id}/models",
            get(models::list_for_project)
                .post(models::upload)
                .layer(DefaultBodyLimit::max(models::MAX_BODY_BYTES)),
        )
        .route("/api/models/{id}/mesh", get(models::mesh))
        .route("/api/models/{id}/file", get(models::download_file))
        .route(
            "/api/models/{id}/obj-colors",
            post(models::obj_colors).layer(DefaultBodyLimit::max(models::MAX_BODY_BYTES)),
        )
        .route("/api/models/{id}/step-mesh", post(models::step_mesh))
        .route("/api/models/{id}/repair", post(scene::repair))
        .route("/api/models/{id}/simplify", post(scene::simplify))
        .route("/api/models/{id}/remap-paint", post(scene::remap_paint))
        .route("/api/models/{id}/split", post(mesh_ops::split))
        .route("/api/models/{id}/smooth", post(mesh_ops::smooth))
        .route("/api/models/{id}/cut", post(mesh_ops::cut))
        .route("/api/projects/{id}/boolean", post(mesh_ops::boolean))
        .route("/api/projects/{id}/emboss", post(mesh_ops::emboss))
        .route("/api/projects/{id}/emboss-svg", post(mesh_ops::emboss_svg))
        .route("/api/projects/{id}/primitive", post(mesh_ops::primitive))
        .route(
            "/api/projects/{id}/filament-compatibility",
            post(slice::filament_compatibility_check),
        )
        .route("/api/handy-models", get(handy_models::list))
        .route(
            "/api/projects/{id}/handy-models/{key}",
            post(handy_models::add),
        )
        .route("/api/projects/{id}/paint", get(paint::paint_ws))
        .route(
            "/api/models/{id}/layer-profile",
            post(mesh_ops::layer_profile),
        )
        .route("/api/emboss/fonts", get(mesh_ops::emboss_fonts))
        .route("/api/projects/{id}/arrange", post(scene::arrange))
        .route(
            "/api/projects/{id}/arrange/options",
            get(scene::arrange_options),
        )
        .route(
            "/api/projects/{id}/metadata",
            get(metadata::get).put(metadata::put),
        )
        .route("/api/projects/{id}/orient", post(scene::orient))
        .route("/api/projects/{id}/slice", post(slice::slice))
        .route("/api/projects/{id}/validate", post(validate::validate))
        .route("/api/projects/{id}/export/3mf", get(export::export_3mf))
        .route(
            "/api/projects/{id}/plates/{index}/thumbnail",
            axum::routing::put(export::put_plate_thumbnail)
                .delete(export::delete_plate_thumbnail)
                .layer(DefaultBodyLimit::max(8 * 1024 * 1024)),
        )
        .route("/api/projects/{id}/export/drc", post(export::export_drc))
        .route("/api/projects/{id}/export/stl", post(export::export_stl))
        .route("/api/flush-volumes", post(flush::flush_volumes))
        .route(
            "/api/projects/{id}/calibration",
            post(calibration::create_project_calibration),
        )
        .route("/api/projects/{id}/auxiliaries", get(auxiliaries::list))
        .route(
            "/api/projects/{id}/auxiliaries/{folder}/{name}",
            get(auxiliaries::get)
                .put(auxiliaries::put)
                .delete(auxiliaries::delete)
                .layer(DefaultBodyLimit::max(auxiliaries::MAX_AUX_BYTES + 1024)),
        )
        .route(
            "/api/calibrations/pa",
            get(calibration::list_pa).post(calibration::create_pa),
        )
        .route(
            "/api/calibrations/pa/{id}",
            get(calibration::get_pa)
                .put(calibration::update_pa)
                .delete(calibration::delete_pa),
        )
        .route(
            "/api/projects/{id}/export/sliced",
            post(sliced::export_sliced),
        )
        .route("/api/jobs", get(jobs::list))
        .route("/api/jobs/{id}", get(jobs::get))
        .route("/api/jobs/{id}/cancel", post(jobs::cancel))
        .route("/api/gcodes/{id}/download", get(gcodes::download))
        .route("/api/gcodes/{id}/stats", get(gcodes::stats))
        .route("/api/gcodes/{id}/preview/meta", get(preview::meta))
        .route("/api/gcodes/{id}/preview/layers", get(preview::layers))
        .route(
            "/api/gcodes/{id}/preview/processed/meta",
            get(preview::processed_meta),
        )
        .route(
            "/api/gcodes/{id}/preview/processed/moves",
            get(preview::processed_moves),
        )
        .route(
            "/api/gcodes/{id}/preview/processed/seams",
            get(preview::processed_seams),
        )
        .route("/api/gcodes/plates-stats", post(preview::plates_stats))
        .route(
            "/api/gcode/preview",
            post(gcode_open::preview).layer(DefaultBodyLimit::max(models::MAX_BODY_BYTES)),
        )
        .route(
            "/api/gcode/process",
            post(gcode_open::process).layer(DefaultBodyLimit::max(models::MAX_BODY_BYTES)),
        )
        .route(
            "/api/gcode/opened/{token}/meta",
            get(gcode_open::opened_meta),
        )
        .route(
            "/api/gcode/opened/{token}/moves",
            get(gcode_open::opened_moves),
        )
        .route(
            "/api/gcode/opened/{token}/seams",
            get(gcode_open::opened_seams),
        )
        .route("/api/ws", get(ws::ws))
        .route("/api/presets", get(presets::list).post(presets::create))
        .route(
            "/api/presets/{id}",
            get(presets::get)
                .put(presets::update)
                .delete(presets::delete),
        )
        .route("/api/presets/{id}/resolved", get(presets::resolved))
        .route("/api/presets/{id}/detach", post(presets::detach))
        .route("/api/presets/{id}/export", get(presets::export))
        .route("/api/presets/import", post(presets::import))
        .route("/api/presets/create-options", get(presets::create_options))
        .route("/api/printer-catalog", get(printers::catalog))
        .route("/api/printers", get(printers::list).post(printers::create))
        .route("/api/printers/discover", get(printers::discover))
        .route(
            "/api/printers/{id}",
            get(printers::get)
                .put(printers::update)
                .delete(printers::delete),
        )
        .route("/api/printers/{id}/test", post(printers::test))
        .route("/api/printers/{id}/storages", get(printers::storages))
        .route(
            "/api/printers/{id}/host-printers",
            get(printers::host_printers),
        )
        .route("/api/printers/{id}/upload", post(printers::upload))
        .route(
            "/api/printers/{id}/upload-sliced",
            post(sliced::upload_sliced),
        )
        .route("/api/printers/{id}/status", get(printers::status))
        .route("/api/printers/{id}/pause", post(printers::pause))
        .route("/api/printers/{id}/resume", post(printers::resume))
        .route("/api/printers/{id}/cancel", post(printers::cancel))
        .route("/api/printers/{id}/files", get(printers::files))
        .route("/api/printers/{id}/print", post(printers::print))
        .route("/api/printers/{id}/gcode", post(printers::gcode))
        .route("/api/printers/{id}/objects", get(printers::objects))
        .route(
            "/api/printers/{id}/objects/exclude",
            post(printers::exclude_object),
        );

    // API de testabilité strictement absente de la configuration normale :
    // aucune réponse d'erreur spécifique, les URI restent de simples 404.
    let app = if std::env::var("HARNESS_API").as_deref() == Ok("1") {
        app.route("/api/_harness/bootstrap", post(harness::bootstrap))
            .route(
                "/api/_harness/slice",
                post(harness::slice).layer(DefaultBodyLimit::max(models::MAX_BODY_BYTES)),
            )
    } else {
        app
    };

    // Enveloppe d'erreur uniforme { code, message, details? } sur toutes les
    // routes, y compris les rejets d'axum (extracteurs, limite de corps,
    // route inconnue, méthode non permise) — lot 27, voir backend/API.md.
    app.layer(session_layer)
        .layer(axum::middleware::map_response(
            super::envelope::normalize_error_response,
        ))
        .with_state(state)
}
