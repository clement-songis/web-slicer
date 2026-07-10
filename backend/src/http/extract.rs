//! Extracteurs d'authentification : lisent l'`user_id` de la session (cookie
//! `tower-sessions`) et chargent le compte. Une session absente/expirée → 401 ;
//! un compte non-admin sur une route admin → 403.

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use tower_sessions::Session;
use uuid::Uuid;

use crate::domain::{Role, User, UserId};

use super::error::ApiError;
use super::state::AppState;

/// Clé de session portant l'identifiant du compte connecté.
pub const SESSION_USER_KEY: &str = "user_id";

/// Compte connecté (extrait de la session).
pub struct CurrentUser(pub User);

impl FromRequestParts<AppState> for CurrentUser {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, ApiError> {
        let session = Session::from_request_parts(parts, state)
            .await
            .map_err(|_| ApiError::unauthorized())?;
        let raw: Option<String> = session
            .get(SESSION_USER_KEY)
            .await
            .map_err(|_| ApiError::internal())?;
        let raw = raw.ok_or_else(ApiError::unauthorized)?;
        let user_id = UserId(Uuid::parse_str(&raw).map_err(|_| ApiError::unauthorized())?);
        // Compte supprimé/désactivé après ouverture de session → 401.
        let user = state
            .storage
            .users()
            .get(user_id)
            .await
            .map_err(|_| ApiError::unauthorized())?;
        Ok(CurrentUser(user))
    }
}

/// Compte connecté **et** administrateur.
pub struct AdminUser(pub User);

impl FromRequestParts<AppState> for AdminUser {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, ApiError> {
        let CurrentUser(user) = CurrentUser::from_request_parts(parts, state).await?;
        if user.role != Role::Admin {
            return Err(ApiError::forbidden("Réservé aux administrateurs"));
        }
        Ok(AdminUser(user))
    }
}
