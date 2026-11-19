//! Uniformisation des réponses d'erreur (lot 27) : toute réponse 4xx/5xx de
//! l'API sort dans l'enveloppe `{ code, message, details? }` (`ErrorBody`).
//!
//! Les handlers produisent déjà cette enveloppe via `ApiError` ; restaient hors
//! contrat les réponses générées par axum lui-même, en `text/plain` ou vides :
//! rejets d'extracteurs (`Json` : corps illisible 400 / champ manquant 422 /
//! Content-Type absent 415 ; `Path`, `Query`, `Multipart`), dépassement de
//! `DefaultBodyLimit` (413), route inconnue (404) et méthode non permise (405).
//! Cette couche les réécrit : code stable déduit du statut, message clair en
//! français, raison technique d'axum dans `details.reason` (jamais pour une 5xx,
//! dont le détail reste dans les logs). Les réponses déjà JSON passent telles
//! quelles.

use axum::body::{to_bytes, Body};
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::Response;

use super::error::ErrorBody;

/// Taille maximale lue d'un corps d'erreur non JSON (message axum).
const MAX_REASON_BYTES: usize = 16 * 1024;

/// Code stable et message par défaut d'un statut d'erreur.
pub fn status_code_slug(status: StatusCode) -> (&'static str, &'static str) {
    match status {
        StatusCode::BAD_REQUEST => ("bad_request", "Requête invalide"),
        StatusCode::UNAUTHORIZED => ("unauthorized", "Session requise"),
        StatusCode::FORBIDDEN => ("forbidden", "Action non autorisée"),
        StatusCode::NOT_FOUND => ("not_found", "Ressource introuvable"),
        StatusCode::METHOD_NOT_ALLOWED => (
            "method_not_allowed",
            "Méthode non autorisée sur cette route",
        ),
        StatusCode::CONFLICT => ("conflict", "Conflit"),
        StatusCode::PAYLOAD_TOO_LARGE => ("too_large", "Corps de requête trop volumineux"),
        StatusCode::UNSUPPORTED_MEDIA_TYPE => (
            "unsupported_media_type",
            "Type de contenu non pris en charge (application/json attendu)",
        ),
        StatusCode::UNPROCESSABLE_ENTITY => ("validation", "Corps de requête invalide"),
        StatusCode::TOO_MANY_REQUESTS => ("too_many_requests", "Trop de requêtes"),
        StatusCode::NOT_IMPLEMENTED => ("not_implemented", "Fonctionnalité indisponible"),
        StatusCode::BAD_GATEWAY => ("bad_gateway", "Service distant injoignable"),
        StatusCode::SERVICE_UNAVAILABLE => ("unavailable", "Service momentanément indisponible"),
        StatusCode::GATEWAY_TIMEOUT => ("timeout", "Délai dépassé"),
        s if s.is_server_error() => ("internal", "Erreur interne"),
        _ => ("error", "Requête refusée"),
    }
}

/// Couche `map_response` : réécrit toute erreur non JSON dans l'enveloppe.
pub async fn normalize_error_response(response: Response) -> Response {
    let status = response.status();
    if !(status.is_client_error() || status.is_server_error()) {
        return response;
    }
    let is_json = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|ct| ct.starts_with("application/json"));
    if is_json {
        return response;
    }
    let (mut parts, body) = response.into_parts();
    let reason = to_bytes(body, MAX_REASON_BYTES)
        .await
        .map(|b| String::from_utf8_lossy(&b).trim().to_string())
        .unwrap_or_default();
    let (code, message) = status_code_slug(status);
    let details = (!reason.is_empty() && status.is_client_error())
        .then(|| serde_json::json!({ "reason": reason }));
    let body = ErrorBody {
        code: code.into(),
        message: message.into(),
        details,
    };
    let bytes = serde_json::to_vec(&body).unwrap_or_default();
    parts.headers.remove(header::CONTENT_LENGTH);
    parts.headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    Response::from_parts(parts, Body::from(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::response::IntoResponse;

    async fn json(resp: Response) -> serde_json::Value {
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn plain_text_rejection_becomes_envelope_with_reason() {
        let resp = (
            StatusCode::UNPROCESSABLE_ENTITY,
            "Failed to deserialize the JSON body into the target type: missing field `name`",
        )
            .into_response();
        let resp = normalize_error_response(resp).await;
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(
            resp.headers()[header::CONTENT_TYPE],
            HeaderValue::from_static("application/json")
        );
        let body = json(resp).await;
        assert_eq!(body["code"], "validation");
        assert!(body["details"]["reason"]
            .as_str()
            .unwrap()
            .contains("missing field"));
    }

    #[tokio::test]
    async fn empty_404_and_405_get_codes_and_5xx_leaks_nothing() {
        let body =
            json(normalize_error_response(StatusCode::NOT_FOUND.into_response()).await).await;
        assert_eq!(body["code"], "not_found");
        assert!(body.get("details").is_none());
        let body =
            json(normalize_error_response(StatusCode::METHOD_NOT_ALLOWED.into_response()).await)
                .await;
        assert_eq!(body["code"], "method_not_allowed");
        let resp = (StatusCode::INTERNAL_SERVER_ERROR, "panic: secret").into_response();
        let body = json(normalize_error_response(resp).await).await;
        assert_eq!(body["code"], "internal");
        assert!(body.get("details").is_none());
    }

    #[tokio::test]
    async fn json_errors_and_successes_pass_through() {
        let ok = normalize_error_response((StatusCode::OK, "texte").into_response()).await;
        assert_eq!(ok.status(), StatusCode::OK);
        let api = super::super::error::ApiError::conflict("x").into_response();
        let body = json(normalize_error_response(api).await).await;
        assert_eq!(body["code"], "conflict");
        assert_eq!(body["message"], "x");
    }
}
