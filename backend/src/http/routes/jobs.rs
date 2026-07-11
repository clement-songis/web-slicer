//! File de tranchage de l'utilisateur (contrat http-api.md, T071/US7) :
//!   - `GET /api/jobs` : file + historique (FR-031) ;
//!   - `GET /api/jobs/{id}` : détail d'un job ;
//!   - `POST /api/jobs/{id}/cancel` : annulation (propriétaire).
//!
//! Isolation (SC-008) : un job d'un autre compte est traité comme inexistant
//! (404). L'annulation est idempotente sur un job déjà annulé et refuse (409) un
//! job déjà terminé (succeeded/failed).

use axum::extract::{Path, State};
use axum::Json;

use crate::domain::{JobId, JobStatus};
use crate::http::dto::JobResponse;
use crate::http::error::{ApiError, ApiResult};
use crate::http::extract::CurrentUser;
use crate::http::state::AppState;

fn parse_job_id(raw: &str) -> ApiResult<JobId> {
    uuid::Uuid::parse_str(raw)
        .map(JobId)
        .map_err(|_| ApiError::not_found("Job"))
}

/// `GET /api/jobs` — file + historique du compte (plus récent d'abord).
pub async fn list(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<JobResponse>>> {
    let mut jobs = state.storage.jobs().list(user.id).await?;
    jobs.sort_by_key(|j| std::cmp::Reverse(j.created_at));
    Ok(Json(jobs.into_iter().map(JobResponse::from).collect()))
}

/// `GET /api/jobs/{id}` — détail d'un job (404, SC-008).
pub async fn get(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<JobResponse>> {
    let id = parse_job_id(&id)?;
    let job = state.storage.jobs().get(user.id, id).await?; // 404, SC-008
    Ok(Json(JobResponse::from(job)))
}

/// `POST /api/jobs/{id}/cancel` — annule un job actif (`queued|running`).
/// Idempotent si déjà `cancelled` ; 409 si déjà terminé (succeeded/failed).
pub async fn cancel(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<JobResponse>> {
    let id = parse_job_id(&id)?;
    let job = state.storage.jobs().get(user.id, id).await?; // 404, SC-008

    match job.status {
        JobStatus::Queued | JobStatus::Running => {
            state.storage.jobs().cancel(user.id, id).await?;
            let updated = state.storage.jobs().get(user.id, id).await?;
            Ok(Json(JobResponse::from(updated)))
        }
        // Déjà annulé : réponse idempotente.
        JobStatus::Cancelled => Ok(Json(JobResponse::from(job))),
        // Terminé : rien à annuler.
        JobStatus::Succeeded | JobStatus::Failed => {
            Err(ApiError::conflict("job déjà terminé (rien à annuler)"))
        }
    }
}
