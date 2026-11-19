//! Endpoints d'administration (`/api/admin/*`), tous réservés aux comptes
//! `admin` via l'extracteur `AdminUser`. Handlers minces : validation DTO →
//! service `auth::` ou repo → réponse.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::de::DeserializeOwned;
use serde_json::Value;
use uuid::Uuid;

use crate::auth;
use crate::domain::{self, RegistrationPolicy, Role, UserId};
use crate::http::dto::{
    AdminCreateUserRequest, AdminUserListResponse, AdminUserResponse, AdminUsersQuery,
    CreateInvitationRequest, InstanceResponse, InvitationResponse, PatchInstanceRequest,
    ReseedResponse, ResetPasswordRequest, UserResponse,
};
use crate::http::error::{ApiError, ApiResult};
use crate::http::extract::AdminUser;
use crate::http::state::AppState;

/// Parse un enum unité du domaine depuis sa forme texte, en 422 si invalide.
fn parse_enum<T: DeserializeOwned>(field: &str, value: &str) -> ApiResult<T> {
    serde_json::from_value(Value::String(value.to_string())).map_err(|_| {
        ApiError::validation(
            format!("valeur invalide pour « {field} »"),
            serde_json::json!({ "field": field, "value": value }),
        )
    })
}

fn parse_user_id(raw: &str) -> ApiResult<UserId> {
    Uuid::parse_str(raw)
        .map(UserId)
        .map_err(|_| ApiError::not_found("Compte"))
}

/// `GET /api/admin/instance` — réglages de l'instance.
pub async fn get_instance(
    _admin: AdminUser,
    State(state): State<AppState>,
) -> ApiResult<Json<InstanceResponse>> {
    let settings = state.storage.instance().settings().await?;
    Ok(Json(settings.into()))
}

/// `PATCH /api/admin/instance` — met à jour politique et/ou limite d'upload.
pub async fn patch_instance(
    _admin: AdminUser,
    State(state): State<AppState>,
    Json(body): Json<PatchInstanceRequest>,
) -> ApiResult<Json<InstanceResponse>> {
    if let Some(policy) = body.registration_policy.as_deref() {
        let policy: RegistrationPolicy = parse_enum("registration_policy", policy)?;
        state
            .storage
            .instance()
            .set_registration_policy(policy)
            .await?;
    }
    if let Some(limit) = body.upload_limit_bytes {
        if limit <= 0 {
            return Err(ApiError::validation(
                "la limite d'upload doit être positive",
                serde_json::json!({ "field": "upload_limit_bytes" }),
            ));
        }
        state.storage.instance().set_upload_limit(limit).await?;
    }
    if let Some(quota) = body.storage_quota_bytes {
        if quota < 0 {
            return Err(ApiError::validation(
                "le quota de stockage doit être positif ou nul (0 = illimité)",
                serde_json::json!({ "field": "storage_quota_bytes" }),
            ));
        }
        state.storage.instance().set_storage_quota(quota).await?;
    }
    if let Some(max) = body.max_concurrent_slices {
        if !(0..=1000).contains(&max) {
            return Err(ApiError::validation(
                "le nombre de tranchages simultanés doit être entre 0 (illimité) et 1000",
                serde_json::json!({ "field": "max_concurrent_slices", "min": 0, "max": 1000 }),
            ));
        }
        state
            .storage
            .instance()
            .set_max_concurrent_slices(max)
            .await?;
    }
    let settings = state.storage.instance().settings().await?;
    Ok(Json(settings.into()))
}

/// Taille de page par défaut / maximale de `GET /api/admin/users`.
const USERS_PER_PAGE: i64 = 50;
const USERS_PER_PAGE_MAX: i64 = 200;

/// `GET /api/admin/users?page=&per_page=&q=` — liste paginée des comptes
/// (lot 32), par date de création croissante : rôle, statut, date, espace
/// disque occupé et nombre de projets. `q` filtre sur une sous-chaîne de
/// l'e-mail (insensible à la casse). Page au-delà de la fin → liste vide.
pub async fn list_users(
    _admin: AdminUser,
    State(state): State<AppState>,
    Query(query): Query<AdminUsersQuery>,
) -> ApiResult<Json<AdminUserListResponse>> {
    let page = query.page.unwrap_or(1);
    let per_page = query.per_page.unwrap_or(USERS_PER_PAGE);
    if page < 1 {
        return Err(ApiError::validation(
            "page doit être ≥ 1",
            serde_json::json!({ "field": "page", "min": 1 }),
        ));
    }
    if !(1..=USERS_PER_PAGE_MAX).contains(&per_page) {
        return Err(ApiError::validation(
            format!("per_page doit être entre 1 et {USERS_PER_PAGE_MAX}"),
            serde_json::json!({ "field": "per_page", "min": 1, "max": USERS_PER_PAGE_MAX }),
        ));
    }
    let needle = query
        .q
        .as_deref()
        .map(|q| q.trim().to_lowercase())
        .filter(|q| !q.is_empty());
    let users: Vec<_> = state
        .storage
        .users()
        .list()
        .await?
        .into_iter()
        .filter(|u| needle.as_deref().is_none_or(|n| u.email.contains(n)))
        .collect();
    let total = users.len() as i64;
    let start = usize::try_from((page - 1).saturating_mul(per_page)).unwrap_or(usize::MAX);
    let mut out = Vec::new();
    for user in users.into_iter().skip(start).take(per_page as usize) {
        let storage_bytes = crate::http::quota::storage_used(&state, user.id).await?;
        let project_count = state.storage.projects().list(user.id).await?.len() as i64;
        let created_at = user
            .created_at
            .format(&time::format_description::well_known::Rfc3339)
            .map_err(|_| ApiError::internal())?;
        let public = UserResponse::from(user);
        out.push(AdminUserResponse {
            id: public.id,
            email: public.email,
            role: public.role,
            status: public.status,
            created_at,
            storage_bytes,
            project_count,
        });
    }
    Ok(Json(AdminUserListResponse {
        users: out,
        total,
        page,
        per_page,
    }))
}

/// `POST /api/admin/users` — crée un compte géré par l'admin.
pub async fn create_user(
    _admin: AdminUser,
    State(state): State<AppState>,
    Json(body): Json<AdminCreateUserRequest>,
) -> ApiResult<(StatusCode, Json<UserResponse>)> {
    let role: Role = match body.role.as_deref() {
        Some(r) => parse_enum("role", r)?,
        None => Role::User,
    };
    let user = auth::create_managed_user(state.storage.as_ref(), &body.email, &body.password, role)
        .await?;
    Ok((StatusCode::CREATED, Json(user.into())))
}

/// `POST /api/admin/users/{id}/reset-password` — réinitialise un mot de passe.
pub async fn reset_password(
    _admin: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<ResetPasswordRequest>,
) -> ApiResult<StatusCode> {
    let user_id = parse_user_id(&id)?;
    auth::reset_password(state.storage.as_ref(), user_id, &body.new_password).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `DELETE /api/admin/users/{id}` — suppression d'un compte par l'admin.
/// Refuse de retirer le dernier administrateur (garde côté service). Cascade
/// BDD + purge fichiers.
pub async fn delete_user(
    _admin: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    let user_id = parse_user_id(&id)?;
    auth::delete_account(state.storage.as_ref(), user_id).await?;
    state
        .files
        .purge_user(user_id)
        .await
        .map_err(|_| ApiError::internal())?;
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /api/admin/invitations` — émet une invitation à usage unique.
pub async fn create_invitation(
    AdminUser(admin): AdminUser,
    State(state): State<AppState>,
    Json(body): Json<CreateInvitationRequest>,
) -> ApiResult<(StatusCode, Json<InvitationResponse>)> {
    let valid_days = body.valid_days.unwrap_or(7);
    let invitation = auth::create_invitation(state.storage.as_ref(), admin.id, valid_days).await?;
    Ok((StatusCode::CREATED, Json(invitation.into())))
}

/// `POST /api/admin/presets/reseed` — ré-importe les profils système depuis
/// `profiles_dir` et remplace les presets système (presets utilisateur
/// intacts). Idempotent.
pub async fn reseed_presets(
    _admin: AdminUser,
    State(state): State<AppState>,
) -> ApiResult<Json<ReseedResponse>> {
    let imported = engine::presets::import_profiles(&state.profiles_dir).map_err(|e| {
        tracing::error!(error = %e, "import des profils système");
        ApiError::internal()
    })?;
    let reseeded =
        domain::presets::reseed_system_presets(state.storage.as_ref(), &imported.presets).await?;
    Ok(Json(ReseedResponse {
        reseeded: reseeded as u32,
    }))
}
