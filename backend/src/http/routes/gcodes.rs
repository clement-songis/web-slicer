//! Consultation d'un G-code produit (contrat http-api.md, T066) :
//!   - `GET /api/gcodes/{id}/download` : export du fichier (FR-044) ;
//!   - `GET /api/gcodes/{id}/stats` : statistiques d'impression (FR-043).
//!
//! Isolation (SC-008) : un G-code d'un autre compte est traité comme inexistant
//! (404), la propriété étant vérifiée par `GcodeRepo::get(owner, id)`.

use std::path::Path as FsPath;

use axum::extract::{Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;

use crate::domain::GcodeId;
use crate::http::error::{ApiError, ApiResult};
use crate::http::extract::CurrentUser;
use crate::http::state::AppState;

fn parse_gcode_id(raw: &str) -> ApiResult<GcodeId> {
    uuid::Uuid::parse_str(raw)
        .map(GcodeId)
        .map_err(|_| ApiError::not_found("G-code"))
}

/// Lot 47c (audit 45 H2D-a) : un G-code dont les trajets sortent de la zone
/// atteignable d'une buse (`stats.gcode_check`) n'est ni exporté ni envoyé,
/// comme dans la GUI d'Orca (erreur SLICING_LIMIT_ERROR : boutons Print /
/// Export désactivés par Plater::update_print_required, aperçu conservé).
/// 409 lisible, `details.reason = "gcode_check"`.
pub(crate) fn ensure_gcode_printable(gcode: &crate::domain::Gcode) -> ApiResult<()> {
    let Some(check) = crate::gcode::stored_gcode_check(&gcode.stats) else {
        return Ok(());
    };
    let orca = check
        .get("message")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let reason = crate::gcode::gcode_check_reason(check);
    Err(ApiError::conflict_with(
        format!("Export et impression impossibles : {reason} {orca}")
            .trim()
            .to_string(),
        serde_json::json!({
            "reason": "gcode_check",
            "gcode_id": gcode.id.0.to_string(),
            "gcode_check": check,
        }),
    ))
}

/// Paramètres de `GET /api/gcodes/{id}/download`.
#[derive(Debug, Default, serde::Deserialize)]
pub struct DownloadQuery {
    /// Lecture pour l'aperçu de l'éditeur (visualiseur de G-code), pas un
    /// export : autorisée même quand l'impression est bloquée (lot 47c).
    #[serde(default)]
    pub preview: Option<String>,
}

/// `GET /api/gcodes/{id}/download` — télécharge le fichier G-code (pièce jointe).
pub async fn download(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<DownloadQuery>,
) -> ApiResult<Response> {
    let id = parse_gcode_id(&id)?;
    let gcode = state.storage.gcodes().get(user.id, id).await?; // 404, SC-008
    let preview = query
        .preview
        .as_deref()
        .is_some_and(|v| matches!(v, "1" | "true" | ""));
    if !preview {
        ensure_gcode_printable(&gcode)?;
    }

    let bytes = state
        .files
        .read(FsPath::new(&gcode.file_path))
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "lecture du G-code");
            ApiError::internal()
        })?;

    // Nom Orca (`filename_format` du preset, base = nom du projet) ; job ou
    // projet supprimé depuis : base « Untitled », comme un projet Orca sans nom.
    let project_name = match state.storage.jobs().get(user.id, gcode.job_id).await {
        Ok(job) => state
            .storage
            .projects()
            .get(user.id, job.project_id)
            .await
            .map(|p| p.name)
            .unwrap_or_default(),
        Err(_) => String::new(),
    };
    // Nom calculé par libslic3r au tranchage ; repli (G-code ancien ou ouvert
    // depuis un fichier) sur la relecture du bloc de configuration.
    let filename = crate::gcode_filename::for_gcode(&gcode.stats, &bytes, &project_name);
    Ok((
        StatusCode::OK,
        [
            (
                header::CONTENT_TYPE,
                "text/plain; charset=utf-8".to_string(),
            ),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{filename}\""),
            ),
        ],
        bytes,
    )
        .into_response())
}

/// `GET /api/gcodes/{id}/stats` — statistiques d'impression figées (FR-043).
pub async fn stats(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Response> {
    let id = parse_gcode_id(&id)?;
    let gcode = state.storage.gcodes().get(user.id, id).await?; // 404, SC-008
    Ok((StatusCode::OK, Json(gcode.stats)).into_response())
}
