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

/// `Retry-After` (secondes) des 429 de quota de tranchage : délai indicatif
/// après lequel un client peut retenter (un slot se sera probablement libéré).
pub const SLICE_QUOTA_RETRY_AFTER_SECS: u64 = 30;

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
    /// `Retry-After` (secondes) pour les 429 — absent sinon (lot 35).
    pub retry_after_secs: Option<u64>,
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
            retry_after_secs: None,
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

    /// Identifiants de connexion refusés (e-mail inconnu ou mot de passe faux,
    /// indistinguables). 401 comme une session absente, mais code dédié (lot 32).
    pub fn invalid_credentials() -> Self {
        Self::new(
            StatusCode::UNAUTHORIZED,
            "invalid_credentials",
            "E-mail ou mot de passe incorrect",
        )
    }

    /// Mot de passe actuel faux sur une action confirmée par mot de passe
    /// (changement de mot de passe). 403 et non 401 : la session reste valide.
    pub fn wrong_password(field: &str) -> Self {
        let mut e = Self::new(
            StatusCode::FORBIDDEN,
            "invalid_credentials",
            "Mot de passe actuel incorrect",
        );
        e.body.details = Some(serde_json::json!({ "field": field }));
        e
    }

    /// Nom de projet déjà pris par un autre projet du compte (lot 32).
    pub fn name_taken(name: &str, suggestion: &str) -> Self {
        let mut e = Self::new(
            StatusCode::CONFLICT,
            "name_taken",
            format!("Un autre projet s'appelle déjà « {name} »"),
        );
        e.body.details = Some(serde_json::json!({
            "field": "name",
            "name": name,
            "suggestion": suggestion,
        }));
        e
    }

    /// Quota de stockage du compte dépassé (lot 32).
    pub fn storage_quota(used: i64, quota: i64, incoming: i64) -> Self {
        let mut e = Self::new(
            StatusCode::PAYLOAD_TOO_LARGE,
            "storage_quota_exceeded",
            format!(
                "Quota de stockage atteint : {} Mo utilisés sur {} Mo",
                (used.max(0) + 999_999) / 1_000_000,
                quota / 1_000_000
            ),
        );
        e.body.details = Some(serde_json::json!({
            "used_bytes": used,
            "quota_bytes": quota,
            "incoming_bytes": incoming,
        }));
        e
    }

    /// Trop de tranchages simultanés pour le compte (lot 32).
    /// Porte `Retry-After` (lot 35) : délai indicatif avant qu'un slot se
    /// libère — un tranchage se termine typiquement en quelques dizaines de
    /// secondes à quelques minutes.
    pub fn slice_quota(active: i64, max: i64, requested: i64) -> Self {
        let mut e = Self::new(
            StatusCode::TOO_MANY_REQUESTS,
            "slice_quota_exceeded",
            format!(
                "Trop de tranchages simultanés : {active} en cours ou en file, {max} autorisé(s)"
            ),
        );
        e.body.details = Some(serde_json::json!({
            "active": active,
            "max": max,
            "requested": requested,
            "retry_after": SLICE_QUOTA_RETRY_AFTER_SECS,
        }));
        e.retry_after_secs = Some(SLICE_QUOTA_RETRY_AFTER_SECS);
        e
    }

    /// Action refusée par la politique d'instance (ex. inscription fermée).
    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::new(StatusCode::FORBIDDEN, "forbidden", message)
    }

    /// Conflit d'édition (verrou optimiste des projets, T060).
    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new(StatusCode::CONFLICT, "conflict", message)
    }

    /// 409 avec détails (ex. projets concernés).
    pub fn conflict_with(message: impl Into<String>, details: serde_json::Value) -> Self {
        let mut e = Self::conflict(message);
        e.body.details = Some(details);
        e
    }

    /// Échec de conversion moteur d'un modèle (T125) : le maillage ne pourra pas
    /// être produit (fichier corrompu, format non décodable) — l'UI le distingue
    /// de « conversion en cours » (409) par ce 422.
    pub fn conversion_failed(message: impl Into<String>) -> Self {
        Self::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "conversion_failed",
            message,
        )
    }

    /// `conversion_failed` d'un modèle précis, avec la **cause** remontée par le
    /// moteur (message du worker) dans le message et `details.cause`.
    pub fn conversion_failed_for(model_id: impl ToString, stored_error: &str) -> Self {
        let cause = crate::models::convert::conversion_cause(stored_error);
        let mut e = Self::conversion_failed(format!(
            "échec de la conversion du modèle par le moteur : {cause}"
        ));
        e.body.details = Some(serde_json::json!({
            "model_id": model_id.to_string(),
            "cause": cause,
        }));
        e
    }

    /// Imprimante déclarée mais **sans connexion réseau** (`moonraker_url` absent,
    /// Phase 14) : les actions Moonraker (test, statut, envoi, contrôle) sont
    /// impossibles. 409 avec un code dédié pour que l'UI propose de connecter
    /// l'imprimante plutôt que d'afficher une erreur générique.
    pub fn printer_not_connected(message: impl Into<String>) -> Self {
        Self::new(StatusCode::CONFLICT, "printer_not_connected", message)
    }

    /// Le preset imprimante choisi ne correspond à **aucune imprimante possédée**
    /// (Phase 14) : on ne tranche/exporte que pour une imprimante déclarée. 422
    /// avec un code dédié pour que l'UI renvoie vers l'ajout d'imprimante.
    pub fn printer_not_owned(message: impl Into<String>) -> Self {
        Self::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "printer_not_owned",
            message,
        )
    }

    /// Fonctionnalité pas encore câblée (placeholder de tâche à venir).
    pub fn not_implemented(message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_IMPLEMENTED, "not_implemented", message)
    }

    /// Fichier trop volumineux (limite d'upload de l'instance, FR-053).
    pub fn too_large(message: impl Into<String>) -> Self {
        Self::new(StatusCode::PAYLOAD_TOO_LARGE, "too_large", message)
    }

    /// Erreur interne : le détail part dans les logs, jamais au client.
    pub fn internal() -> Self {
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal",
            "Erreur interne",
        )
    }

    /// Hôte d'impression injoignable ou en erreur (FR-062) : échec propre, pas
    /// une 500. Le client peut réessayer sans que l'instance soit compromise.
    pub fn bad_gateway(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_GATEWAY, "printer_unreachable", message)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let mut response = (self.status, Json(self.body)).into_response();
        if let Some(secs) = self.retry_after_secs {
            if let Ok(value) = axum::http::HeaderValue::from_str(&secs.to_string()) {
                response
                    .headers_mut()
                    .insert(axum::http::header::RETRY_AFTER, value);
            }
        }
        response
    }
}

impl From<crate::domain::StorageError> for ApiError {
    fn from(e: crate::domain::StorageError) -> Self {
        use crate::domain::StorageError as E;
        match e {
            E::NotFound => ApiError::not_found("Ressource"),
            E::Conflict(m) => ApiError::conflict(m),
            E::VersionConflict { .. } => {
                ApiError::conflict("Version dépassée (rechargez le projet)")
            }
            E::Forbidden(m) => ApiError::forbidden(m),
            // Le détail SQL part dans les logs, jamais au client.
            E::Backend(m) => {
                tracing::error!(error = %m, "erreur de stockage");
                ApiError::internal()
            }
        }
    }
}

impl From<crate::adapters::moonraker::MoonrakerError> for ApiError {
    fn from(e: crate::adapters::moonraker::MoonrakerError) -> Self {
        // L'imprimante distante est hors du périmètre de confiance : toute panne
        // (réseau, statut, décodage) se traduit par un 502 propre (FR-062).
        tracing::warn!(error = %e, "dialogue Moonraker en échec");
        ApiError::bad_gateway(e.to_string())
    }
}

impl From<crate::auth::AuthError> for ApiError {
    fn from(e: crate::auth::AuthError) -> Self {
        use crate::auth::AuthError as E;
        match e {
            E::WeakPassword => {
                ApiError::validation(e.to_string(), serde_json::json!({ "field": "password" }))
            }
            E::EmailTaken => ApiError::conflict(e.to_string()),
            E::RegistrationClosed | E::InvalidInvitation | E::AccountDisabled => {
                ApiError::forbidden(e.to_string())
            }
            E::NotFound => ApiError::not_found("Compte"),
            E::LastAdmin => ApiError::forbidden(e.to_string()),
            E::InvalidCredentials => ApiError::invalid_credentials(),
            E::WrongPassword => ApiError::wrong_password("current_password"),
            E::SamePassword => ApiError::validation(
                e.to_string(),
                serde_json::json!({ "field": "new_password" }),
            ),
            E::Password(_) | E::Storage(_) => {
                tracing::error!(error = %e, "erreur d'authentification");
                ApiError::internal()
            }
        }
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
    async fn not_found_responds_404_with_envelope() {
        let (status, json) = body_json(ApiError::not_found("Projet")).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(json["code"], "not_found");
        assert_eq!(json["message"], "Projet introuvable");
        assert!(json.get("details").is_none(), "details absent si None");
    }

    #[tokio::test]
    async fn validation_responds_422_with_details() {
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
    async fn conflict_and_session() {
        let (s1, j1) = body_json(ApiError::conflict("Version dépassée")).await;
        assert_eq!(s1, StatusCode::CONFLICT);
        assert_eq!(j1["code"], "conflict");
        let (s2, _) = body_json(ApiError::unauthorized()).await;
        assert_eq!(s2, StatusCode::UNAUTHORIZED);
        let (s3, j3) = body_json(ApiError::invalid_credentials()).await;
        assert_eq!(s3, StatusCode::UNAUTHORIZED);
        assert_eq!(j3["code"], "invalid_credentials");
    }

    #[tokio::test]
    async fn quotas_and_name_taken() {
        let (s, j) = body_json(ApiError::storage_quota(2_500_000, 2_000_000, 10)).await;
        assert_eq!(s, StatusCode::PAYLOAD_TOO_LARGE);
        assert_eq!(j["code"], "storage_quota_exceeded");
        assert_eq!(j["details"]["quota_bytes"], 2_000_000);
        let (s, j) = body_json(ApiError::slice_quota(2, 2, 1)).await;
        assert_eq!(s, StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(j["code"], "slice_quota_exceeded");
        let (s, j) = body_json(ApiError::name_taken("A", "A (2)")).await;
        assert_eq!(s, StatusCode::CONFLICT);
        assert_eq!(j["details"]["suggestion"], "A (2)");
    }

    #[tokio::test]
    async fn internal_leaks_no_detail() {
        let (status, json) = body_json(ApiError::internal()).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(json["message"], "Erreur interne");
        assert!(json.get("details").is_none());
    }
}
