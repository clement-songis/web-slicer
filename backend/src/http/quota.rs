//! Quotas par compte (lot 32), configurés par l'admin dans les réglages
//! d'instance (`PATCH /api/admin/instance`) :
//! - **stockage** : `storage_quota_bytes` borne tout l'espace fichiers du
//!   compte (`FileStore::usage_bytes`) ; un upload qui le dépasserait, une
//!   duplication de projet ou un tranchage lancé quota atteint → **413**
//!   `storage_quota_exceeded` ;
//! - **tranchages simultanés** : `max_concurrent_slices` borne les jobs du
//!   compte en file ou en cours ; une demande qui le dépasserait (tous les
//!   plateaux demandés comptent) → **429** `slice_quota_exceeded`.
//!
//! 0 = illimité (valeur par défaut des deux réglages).

use crate::domain::{JobStatus, UserId};
use crate::http::dto::UsageResponse;
use crate::http::error::{ApiError, ApiResult};
use crate::http::state::AppState;

/// Octets occupés par l'espace fichiers du compte.
pub async fn storage_used(state: &AppState, user: UserId) -> ApiResult<i64> {
    let used = state.files.usage_bytes(user).await.map_err(|e| {
        tracing::error!(error = %e, "calcul de l'espace occupé");
        ApiError::internal()
    })?;
    Ok(i64::try_from(used).unwrap_or(i64::MAX))
}

/// Refuse (413) si `incoming` octets de plus dépasseraient le quota de
/// stockage du compte. Sans quota (0) : aucune lecture du disque.
pub async fn ensure_storage_room(state: &AppState, user: UserId, incoming: i64) -> ApiResult<()> {
    let quota = state
        .storage
        .instance()
        .settings()
        .await?
        .storage_quota_bytes;
    if quota <= 0 {
        return Ok(());
    }
    let used = storage_used(state, user).await?;
    if used.saturating_add(incoming.max(0)) > quota {
        return Err(ApiError::storage_quota(used, quota, incoming.max(0)));
    }
    Ok(())
}

/// Jobs du compte en file ou en cours.
pub async fn active_slices(state: &AppState, user: UserId) -> ApiResult<i64> {
    let jobs = state.storage.jobs().list(user).await?;
    Ok(jobs
        .iter()
        .filter(|j| matches!(j.status, JobStatus::Queued | JobStatus::Running))
        .count() as i64)
}

/// Refuse (429) si `requested` jobs de plus dépasseraient la limite de
/// tranchages simultanés du compte.
pub async fn ensure_slice_slots(state: &AppState, user: UserId, requested: i64) -> ApiResult<()> {
    let max = state
        .storage
        .instance()
        .settings()
        .await?
        .max_concurrent_slices;
    if max <= 0 {
        return Ok(());
    }
    let active = active_slices(state, user).await?;
    if active + requested > max {
        return Err(ApiError::slice_quota(active, max, requested));
    }
    Ok(())
}

/// Usage et quotas du compte (`GET /api/me/usage`).
pub async fn usage(state: &AppState, user: UserId) -> ApiResult<UsageResponse> {
    let settings = state.storage.instance().settings().await?;
    Ok(UsageResponse {
        storage_bytes: storage_used(state, user).await?,
        storage_quota_bytes: settings.storage_quota_bytes,
        active_slices: active_slices(state, user).await?,
        max_concurrent_slices: settings.max_concurrent_slices,
    })
}
