//! Préférences de l'interface par compte (équivalent web de l'AppConfig
//! d'Orca, fenêtre Préférences) : document JSON libre, propre à chaque compte,
//! remplaçant le localStorage du navigateur.
//!
//! - `GET /api/me/preferences` → objet JSON (`{}` si rien d'enregistré) ;
//! - `PUT /api/me/preferences` (objet JSON) → remplace et renvoie le document.
//!   Objet requis (422 sinon), 64 Kio sérialisés au plus (413), profondeur 16.

use axum::extract::State;
use axum::Json;
use serde_json::Value;

use crate::http::error::{ApiError, ApiResult};
use crate::http::extract::CurrentUser;
use crate::http::state::AppState;

/// Taille maximale du document sérialisé.
pub const MAX_PREFERENCES_BYTES: usize = 64 * 1024;
/// Profondeur d'imbrication maximale.
pub const MAX_PREFERENCES_DEPTH: usize = 16;

fn depth(v: &Value) -> usize {
    match v {
        Value::Object(m) => 1 + m.values().map(depth).max().unwrap_or(0),
        Value::Array(a) => 1 + a.iter().map(depth).max().unwrap_or(0),
        _ => 0,
    }
}

/// Contrôle d'un document de préférences (objet, taille, profondeur).
pub fn validate_preferences(v: &Value) -> ApiResult<()> {
    if !v.is_object() {
        return Err(ApiError::validation(
            "les préférences doivent être un objet JSON",
            serde_json::json!({}),
        ));
    }
    let size = serde_json::to_vec(v).map(|b| b.len()).unwrap_or(usize::MAX);
    if size > MAX_PREFERENCES_BYTES {
        return Err(ApiError::too_large(format!(
            "préférences de {size} octets au-delà de la limite ({MAX_PREFERENCES_BYTES})"
        )));
    }
    if depth(v) > MAX_PREFERENCES_DEPTH {
        return Err(ApiError::validation(
            "préférences trop imbriquées",
            serde_json::json!({ "max_depth": MAX_PREFERENCES_DEPTH }),
        ));
    }
    Ok(())
}

/// `GET /api/me/preferences`.
pub async fn get(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
) -> ApiResult<Json<Value>> {
    Ok(Json(state.storage.users().get_preferences(user.id).await?))
}

/// `PUT /api/me/preferences`.
pub async fn put(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    validate_preferences(&body)?;
    state
        .storage
        .users()
        .set_preferences(user.id, body.clone())
        .await?;
    Ok(Json(body))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_bounds() {
        assert!(validate_preferences(&serde_json::json!({ "a": 1 })).is_ok());
        assert!(validate_preferences(&serde_json::json!([1])).is_err());
        let big = "x".repeat(MAX_PREFERENCES_BYTES);
        assert!(validate_preferences(&serde_json::json!({ "a": big })).is_err());
        let mut deep = serde_json::json!(1);
        for _ in 0..20 {
            deep = serde_json::json!({ "d": deep });
        }
        assert!(validate_preferences(&deep).is_err());
    }
}
