//! Enveloppe d'erreur de l'API (contrat http-api.md).
//!
//! Toute erreur sort en JSON `{ code, message, details? }`. Règle SC-008 :
//! une ressource appartenant à un autre compte répond **404** (jamais 403),
//! pour ne pas révéler son existence — `ApiError::not_found` est donc la
//! seule réponse pour « absent » ET « pas à vous ».

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Corps JSON de toute réponse d'erreur.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ErrorBody {
    /// Code stable, machine-lisible (ex. `not_found`, `validation`).
    pub code: String,
    /// Message destiné à l'utilisateur, en langage clair (FR-032).
    pub message: String,
    /// Contexte optionnel (champ en faute, bornes, avertissements moteur…).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub details: Option<serde_json::Value>,
}

/// Erreur HTTP portée par les handlers — construite depuis le domaine.
#[derive(Debug)]
pub struct ApiError {
    pub status: StatusCode,
    pub body: ErrorBody,
}

impl ApiError {
    fn new(status: StatusCode, code: &str, message: impl Into<String>) -> Self {
        Self {
            status,
            body: ErrorBody {
                code: code.into(),
                message: message.into(),
                details: None,
            },
        }
    }

    /// Ressource absente **ou appartenant à un autre compte** (SC-008).
    pub fn not_found(resource: &str) -> Self {
        Self::new(
            StatusCode::NOT_FOUND,
            "not_found",
            format!("{resource} introuvable"),
        )
    }

    /// Entrée invalide (type, bornes, enum du registre…).
    pub fn validation(message: impl Into<String>, details: serde_json::Value) -> Self {
        let mut e = Self::new(StatusCode::UNPROCESSABLE_ENTITY, "validation", message);
        e.body.details = Some(details);
        e
    }

    /// Session absente ou expirée.
    pub fn unauthorized() -> Self {
        Self::new(StatusCode::UNAUTHORIZED, "unauthorized", "Session requise")
    }

    /// Action refusée par la politique d'instance (ex. inscription fermée).
    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::new(StatusCode::FORBIDDEN, "forbidden", message)
    }

    /// Conflit d'édition (verrou optimiste des projets, T060).
    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new(StatusCode::CONFLICT, "conflict", message)
    }

    /// Erreur interne : le détail part dans les logs, jamais au client.
    pub fn internal() -> Self {
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal",
            "Erreur interne",
        )
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(self.body)).into_response()
    }
}

/// Résultat standard des handlers.
pub type ApiResult<T> = Result<T, ApiError>;

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use axum::http::StatusCode;

    async fn body_json(err: ApiError) -> (StatusCode, serde_json::Value) {
        let resp = err.into_response();
        let status = resp.status();
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        (status, serde_json::from_slice(&bytes).unwrap())
    }

    #[tokio::test]
    async fn not_found_repond_404_avec_enveloppe() {
        let (status, json) = body_json(ApiError::not_found("Projet")).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(json["code"], "not_found");
        assert_eq!(json["message"], "Projet introuvable");
        assert!(json.get("details").is_none(), "details absent si None");
    }

    #[tokio::test]
    async fn validation_repond_422_avec_details() {
        let (status, json) = body_json(ApiError::validation(
            "layer_height hors bornes",
            serde_json::json!({"key": "layer_height", "min": 0}),
        ))
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(json["code"], "validation");
        assert_eq!(json["details"]["key"], "layer_height");
    }

    #[tokio::test]
    async fn conflit_et_session() {
        let (s1, j1) = body_json(ApiError::conflict("Version dépassée")).await;
        assert_eq!(s1, StatusCode::CONFLICT);
        assert_eq!(j1["code"], "conflict");
        let (s2, _) = body_json(ApiError::unauthorized()).await;
        assert_eq!(s2, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn interne_ne_fuit_aucun_detail() {
        let (status, json) = body_json(ApiError::internal()).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(json["message"], "Erreur interne");
        assert!(json.get("details").is_none());
    }
}
