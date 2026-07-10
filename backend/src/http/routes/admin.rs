//! Endpoints d'administration (`/api/admin/*`), tous réservés aux comptes
//! `admin` via l'extracteur `AdminUser`. Handlers minces : validation DTO →
//! service `auth::` ou repo → réponse.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::de::DeserializeOwned;
use serde_json::Value;
use uuid::Uuid;

use crate::auth;
use crate::domain::{self, RegistrationPolicy, Role, UserId};
use crate::http::dto::{
    AdminCreateUserRequest, CreateInvitationRequest, InstanceResponse, InvitationResponse,
    PatchInstanceRequest, ReseedResponse, ResetPasswordRequest, UserResponse,
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
    let settings = state.storage.instance().settings().await?;
    Ok(Json(settings.into()))
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
