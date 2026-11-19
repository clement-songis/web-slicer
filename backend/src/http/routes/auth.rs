//! Endpoints d'authentification : register, login, logout, me. Handlers minces
//! (constitution I) : ils délèguent à `auth::` et posent/effacent la session.

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use tower_sessions::Session;

use crate::auth;
use crate::domain::User;
use crate::http::dto::{
    ChangePasswordRequest, DeleteAccountRequest, LoginRequest, RegisterRequest, UsageResponse,
    UserResponse,
};
use crate::http::error::{ApiError, ApiResult};
use crate::http::extract::{CurrentUser, SESSION_STAMP_KEY, SESSION_USER_KEY};
use crate::http::state::AppState;

/// Ouvre (ou rafraîchit) la session du compte : identifiant + empreinte du mot
/// de passe courant (`auth::session_stamp`).
pub(crate) async fn open_session(session: &Session, user: &User) -> ApiResult<()> {
    session
        .insert(SESSION_USER_KEY, user.id.to_string())
        .await
        .map_err(|_| ApiError::internal())?;
    session
        .insert(SESSION_STAMP_KEY, auth::session_stamp(&user.password_hash))
        .await
        .map_err(|_| ApiError::internal())
}

/// `POST /api/auth/register` — crée un compte (premier = admin) et ouvre la session.
pub async fn register(
    State(state): State<AppState>,
    session: Session,
    Json(body): Json<RegisterRequest>,
) -> ApiResult<(StatusCode, Json<UserResponse>)> {
    let user = auth::register(
        state.storage.as_ref(),
        &body.email,
        &body.password,
        body.invite_token.as_deref(),
    )
    .await?;
    open_session(&session, &user).await?;
    Ok((StatusCode::CREATED, Json(user.into())))
}

/// `POST /api/auth/login` — vérifie les identifiants et ouvre la session.
/// Identifiants refusés → 401 `invalid_credentials` (lot 32 ; e-mail inconnu
/// et mot de passe faux indistinguables).
pub async fn login(
    State(state): State<AppState>,
    session: Session,
    Json(body): Json<LoginRequest>,
) -> ApiResult<Json<UserResponse>> {
    let user = auth::authenticate(state.storage.as_ref(), &body.email, &body.password).await?;
    // Nouvel identifiant de session à la connexion (fixation de session).
    session.cycle_id().await.map_err(|_| ApiError::internal())?;
    open_session(&session, &user).await?;
    Ok(Json(user.into()))
}

/// `GET /api/me/usage` — espace occupé, tranchages actifs et quotas
/// d'instance du compte connecté (lot 32).
pub async fn usage(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
) -> ApiResult<Json<UsageResponse>> {
    Ok(Json(crate::http::quota::usage(&state, user.id).await?))
}

/// `PUT /api/me/password` — change son propre mot de passe (lot 32).
/// Ancien mot de passe faux → 403 `invalid_credentials` (la session reste
/// valide) ; nouveau trop court ou identique → 422 `validation`. En cas de
/// succès (204) toutes les **autres** sessions du compte sont révoquées
/// (empreinte de mot de passe changée) ; la session courante reçoit un nouvel
/// identifiant et reste ouverte.
pub async fn change_password(
    CurrentUser(user): CurrentUser,
    session: Session,
    State(state): State<AppState>,
    Json(body): Json<ChangePasswordRequest>,
) -> ApiResult<StatusCode> {
    let updated = auth::change_password(
        state.storage.as_ref(),
        &user,
        &body.current_password,
        &body.new_password,
    )
    .await?;
    session.cycle_id().await.map_err(|_| ApiError::internal())?;
    open_session(&session, &updated).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /api/auth/logout` — détruit la session.
pub async fn logout(session: Session) -> ApiResult<StatusCode> {
    session.flush().await.map_err(|_| ApiError::internal())?;
    Ok(StatusCode::NO_CONTENT)
}

/// `GET /api/auth/me` — compte connecté.
pub async fn me(CurrentUser(user): CurrentUser) -> Json<UserResponse> {
    Json(user.into())
}

/// `DELETE /api/auth/me` — suppression de son propre compte, confirmée par mot
/// de passe. Cascade BDD + purge fichiers, puis destruction de la session.
pub async fn delete_me(
    CurrentUser(user): CurrentUser,
    session: Session,
    State(state): State<AppState>,
    Json(body): Json<DeleteAccountRequest>,
) -> ApiResult<StatusCode> {
    if !auth::verify_password(&body.password, &user.password_hash) {
        return Err(ApiError::forbidden("Mot de passe incorrect"));
    }
    auth::delete_account(state.storage.as_ref(), user.id).await?;
    state
        .files
        .purge_user(user.id)
        .await
        .map_err(|_| ApiError::internal())?;
    session.flush().await.map_err(|_| ApiError::internal())?;
    Ok(StatusCode::NO_CONTENT)
}
