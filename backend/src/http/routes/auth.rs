//! Endpoints d'authentification : register, login, logout, me. Handlers minces
//! (constitution I) : ils délèguent à `auth::` et posent/effacent la session.

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use tower_sessions::Session;

use crate::auth;
use crate::http::dto::{LoginRequest, RegisterRequest, UserResponse};
use crate::http::error::{ApiError, ApiResult};
use crate::http::extract::{CurrentUser, SESSION_USER_KEY};
use crate::http::state::AppState;

async fn open_session(session: &Session, user_id: &str) -> ApiResult<()> {
    session
        .insert(SESSION_USER_KEY, user_id.to_string())
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
    open_session(&session, &user.id.to_string()).await?;
    Ok((StatusCode::CREATED, Json(user.into())))
}

/// `POST /api/auth/login` — vérifie les identifiants et ouvre la session.
pub async fn login(
    State(state): State<AppState>,
    session: Session,
    Json(body): Json<LoginRequest>,
) -> ApiResult<Json<UserResponse>> {
    let user = auth::authenticate(state.storage.as_ref(), &body.email, &body.password).await?;
    open_session(&session, &user.id.to_string()).await?;
    Ok(Json(user.into()))
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
