//! Export de projet 3MF (`GET /api/projects/{id}/export/3mf`, T072, FR-044).
//!
//! Assemble les maillages du projet et la configuration figée (presets actifs
//! résolus) en un projet 3MF **compatible OrcaSlicer** via `engine::threemf`
//! (writer pur Rust). Géométrie STL décodée côté serveur ; les formats dont la
//! géométrie serveur dépend du moteur (OBJ/3MF/STEP) sont ignorés à l'export en
//! attendant le writer libslic3r (`adapters::ffi::write_project_3mf`, T066+).
//!
//! Isolation (SC-008) : un projet d'autrui répond 404.

use axum::extract::{Path, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use std::path::Path as FsPath;

use crate::domain::{ModelFormat, ProjectId};
use crate::http::error::{ApiError, ApiResult};
use crate::http::extract::CurrentUser;
use crate::http::state::AppState;
use crate::mesh::{parse_stl, Mesh};

fn parse_project_id(raw: &str) -> ApiResult<ProjectId> {
    uuid::Uuid::parse_str(raw)
        .map(ProjectId)
        .map_err(|_| ApiError::not_found("Projet"))
}

/// `GET /api/projects/{id}/export/3mf` — projet 3MF (pièce jointe).
pub async fn export_3mf(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(project_raw): Path<String>,
) -> ApiResult<Response> {
    let project_id = parse_project_id(&project_raw)?;
    let project = state.storage.projects().get(user.id, project_id).await?; // 404, SC-008

    // Géométrie : maillages STL décodés côté serveur.
    let models = state
        .storage
        .models()
        .list(user.id, Some(project_id))
        .await?;
    let mut model = engine::api::Model::default();
    for m in models.iter().filter(|m| m.format == ModelFormat::Stl) {
        let bytes = state
            .files
            .read(FsPath::new(&m.file_path))
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "lecture du modèle (export 3MF)");
                ApiError::internal()
            })?;
        let mesh = parse_stl(&bytes).map_err(|e| {
            tracing::error!(error = %e, "décodage STL (export 3MF)");
            ApiError::validation(
                "STL illisible",
                serde_json::json!({ "model": m.id.to_string() }),
            )
        })?;
        model.add_object(m.filename.clone(), to_triangle_mesh(&mesh));
    }

    // Configuration figée : presets actifs résolus.
    let (config, _warnings) =
        super::slice::resolve_active_config(&state, user.id, &project.active_presets).await?;

    let bytes = engine::threemf::write_project_bytes(&model, &config).map_err(|e| {
        tracing::error!(error = %e, "écriture du projet 3MF");
        ApiError::internal()
    })?;

    let filename = format!("{}.3mf", sanitize(&project.name));
    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "model/3mf".to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{filename}\""),
            ),
        ],
        bytes,
    )
        .into_response())
}

/// Maillage backend (aplati) → `TriangleMesh` moteur (sommets/triangles indexés).
fn to_triangle_mesh(mesh: &Mesh) -> engine::api::TriangleMesh {
    engine::api::TriangleMesh {
        vertices: mesh
            .positions
            .chunks_exact(3)
            .map(|c| [c[0], c[1], c[2]])
            .collect(),
        indices: mesh
            .indices
            .chunks_exact(3)
            .map(|c| [c[0], c[1], c[2]])
            .collect(),
    }
}

/// Nom de fichier sûr (alphanumérique + `-_`), repli `project`.
fn sanitize(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let trimmed = cleaned.trim_matches('_');
    if trimmed.is_empty() {
        "project".to_string()
    } else {
        trimmed.to_string()
    }
}
