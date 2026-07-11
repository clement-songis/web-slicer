//! Upload de modèles 3D (`POST /api/projects/{id}/models`, contrat http-api.md).
//!
//! Multipart, limite d'upload de l'instance (500 Mo par défaut, FR-053), formats
//! STL/3MF/STEP/OBJ détectés par extension + contrôle de contenu (fichier
//! corrompu rejeté). Le fichier est stocké dans l'espace de l'utilisateur puis
//! un enregistrement `Model` est créé.
//!
//! **Périmètre v1 (séquencement)** : la conversion STEP → mesh asynchrone et
//! l'événement WebSocket `model.converted` (R7) sont livrés avec le bus WS
//! (T065) ; l'import de la scène+réglages d'un 3MF *projet* est livré avec le
//! wrapper 3MF côté API (dépend du handle moteur). Ici, un STEP est stocké en
//! l'état (`conversion_pending`), un 3MF est stocké comme modèle. La détection,
//! la validation et le stockage — testés — sont complets.

use std::path::Path as FsPath;

use axum::extract::{Multipart, Path, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;

use crate::domain::{Model, ModelFormat, ModelId, NewModel, ProjectId, UserId};
use crate::http::dto::ModelResponse;
use crate::http::error::{ApiError, ApiResult};
use crate::http::extract::CurrentUser;
use crate::http::state::AppState;
use crate::mesh::parse_stl;

/// Plafond dur de la requête multipart (garde-fou couche transport, 500 Mo).
pub const MAX_BODY_BYTES: usize = 500 * 1024 * 1024;

fn parse_project_id(raw: &str) -> ApiResult<ProjectId> {
    uuid::Uuid::parse_str(raw)
        .map(ProjectId)
        .map_err(|_| ApiError::not_found("Projet"))
}

/// `POST /api/projects/{id}/models` — importe un modèle dans un projet.
pub async fn upload(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(project_raw): Path<String>,
    multipart: Multipart,
) -> ApiResult<(StatusCode, Json<ModelResponse>)> {
    let project_id = parse_project_id(&project_raw)?;
    // Isolation : le projet doit appartenir au compte (sinon 404, SC-008).
    state.storage.projects().get(user.id, project_id).await?;

    let (filename, format, bytes) = read_model_upload(&state, multipart).await?;
    let model = write_model_record(&state, user.id, project_id, format, filename, &bytes).await?;
    Ok((StatusCode::CREATED, Json(model.into())))
}

/// Lit un fichier modèle du multipart, applique la limite d'upload de l'instance
/// (FR-053) puis détecte et valide le format. Partagé entre l'upload dans un
/// projet et l'import de projet (T090).
pub(crate) async fn read_model_upload(
    state: &AppState,
    multipart: Multipart,
) -> ApiResult<(String, ModelFormat, Vec<u8>)> {
    let (filename, bytes) = read_upload(multipart).await?;

    let limit = state
        .storage
        .instance()
        .settings()
        .await?
        .upload_limit_bytes;
    if bytes.len() as i64 > limit {
        return Err(ApiError::too_large(format!(
            "fichier de {} octets au-delà de la limite ({limit})",
            bytes.len()
        )));
    }

    let format = detect_format(&filename).ok_or_else(|| {
        ApiError::validation(
            "format non supporté (STL/OBJ/3MF/STEP/AMF/SVG/DRC)",
            details(&filename),
        )
    })?;
    validate_content(format, &bytes)
        .map_err(|reason| ApiError::validation(reason, details(&filename)))?;
    Ok((filename, format, bytes))
}

/// Écrit le fichier et crée l'enregistrement `Model` (format déjà validé).
pub(crate) async fn write_model_record(
    state: &AppState,
    user_id: UserId,
    project_id: ProjectId,
    format: ModelFormat,
    filename: String,
    bytes: &[u8],
) -> ApiResult<Model> {
    let triangle_count = i64::from(binary_stl_triangle_count(format, bytes).unwrap_or(0));

    // Le fichier est nommé par une clé de stockage propre ; `file_path` fait foi.
    let storage_key = ModelId::new();
    let path = state
        .files
        .write_model(user_id, storage_key, format_ext(format), bytes)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "écriture du modèle");
            ApiError::internal()
        })?;

    let model = state
        .storage
        .models()
        .create(
            user_id,
            NewModel {
                project_id: Some(project_id),
                filename,
                format,
                file_path: path.to_string_lossy().into_owned(),
                mesh_path: None,
                size_bytes: bytes.len() as i64,
                triangle_count,
                repair_report: None,
            },
        )
        .await?;
    Ok(model)
}

fn parse_model_id(raw: &str) -> ApiResult<ModelId> {
    uuid::Uuid::parse_str(raw)
        .map(ModelId)
        .map_err(|_| ApiError::not_found("Modèle"))
}

/// `GET /api/models/{id}/mesh` — maillage affichable au format binaire compact
/// (positions/normales/indices, little-endian) pour Threlte.
///
/// STL (binaire/ASCII) est décodé en pur Rust. Les autres formats dépendent du
/// moteur : un STEP renvoie 409 tant que la conversion (T065) n'a pas produit de
/// mesh ; OBJ/3MF s'appuient sur l'aperçu client (T051) — 501 côté serveur.
pub async fn mesh(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Response> {
    let id = parse_model_id(&id)?;
    let model = state.storage.models().get(user.id, id).await?; // 404 si autre compte

    if model.format != ModelFormat::Stl {
        return Err(if model.format.needs_engine_conversion() {
            ApiError::conflict("conversion en cours (voir l'événement model.converted)")
        } else {
            // OBJ/3MF : aperçu produit côté client (T051).
            ApiError::not_implemented("maillage serveur indisponible (aperçu côté client)")
        });
    }

    let bytes = state
        .files
        .read(FsPath::new(&model.file_path))
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "lecture du modèle");
            ApiError::internal()
        })?;
    let mesh = parse_stl(&bytes).map_err(|e| {
        tracing::error!(error = %e, "décodage STL");
        ApiError::validation("STL illisible", serde_json::json!({}))
    })?;

    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/octet-stream")],
        mesh.encode(),
    )
        .into_response())
}

/// `GET /api/projects/{id}/models` — modèles rattachés à un projet (T092), pour
/// repeupler la scène à l'ouverture. Isolation : projet d'autrui → 404 (SC-008).
pub async fn list_for_project(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(project_raw): Path<String>,
) -> ApiResult<Json<Vec<ModelResponse>>> {
    let project_id = parse_project_id(&project_raw)?;
    state.storage.projects().get(user.id, project_id).await?; // 404 si autre compte
    let models = state
        .storage
        .models()
        .list(user.id, Some(project_id))
        .await?;
    Ok(Json(models.into_iter().map(Into::into).collect()))
}

/// `GET /api/models/{id}/file` — fichier source brut (T092) : permet à
/// l'aperçu client de parser les formats non décodés côté serveur (OBJ/3MF/…).
/// 404 inter-comptes (SC-008).
pub async fn download_file(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Response> {
    let id = parse_model_id(&id)?;
    let model = state.storage.models().get(user.id, id).await?; // 404 si autre compte
    let bytes = state
        .files
        .read(FsPath::new(&model.file_path))
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "lecture du modèle");
            ApiError::internal()
        })?;
    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/octet-stream")],
        bytes,
    )
        .into_response())
}

/// Lit le premier champ fichier du multipart → (nom, octets).
async fn read_upload(mut multipart: Multipart) -> ApiResult<(String, Vec<u8>)> {
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| ApiError::validation("multipart illisible", serde_json::json!({})))?
    {
        let Some(filename) = field.file_name().map(str::to_string) else {
            continue; // champ sans fichier : ignoré
        };
        let bytes = field.bytes().await.map_err(|_| {
            ApiError::too_large("lecture du fichier interrompue (trop volumineux ?)")
        })?;
        return Ok((filename, bytes.to_vec()));
    }
    Err(ApiError::validation(
        "aucun fichier dans la requête",
        serde_json::json!({ "field": "file" }),
    ))
}

fn details(filename: &str) -> serde_json::Value {
    serde_json::json!({ "filename": filename })
}

// --- Détection & validation de format (pur, testé) ---------------------------

/// Détermine le format à partir de l'extension du nom de fichier.
pub fn detect_format(filename: &str) -> Option<ModelFormat> {
    let ext = filename.rsplit('.').next()?.to_ascii_lowercase();
    // Jeu cross-plateforme d'OrcaSlicer (T091) : `.oltp` = alias STL ; `.amf`,
    // `.xml` (et `.zip.amf`, dont l'ext finale est `.amf`) → AMF.
    match ext.as_str() {
        "stl" | "oltp" => Some(ModelFormat::Stl),
        "obj" => Some(ModelFormat::Obj),
        "3mf" => Some(ModelFormat::ThreeMf),
        "step" | "stp" => Some(ModelFormat::Step),
        "amf" | "xml" => Some(ModelFormat::Amf),
        "svg" => Some(ModelFormat::Svg),
        "drc" => Some(ModelFormat::Drc),
        _ => None,
    }
}

/// Extension canonique de stockage d'un format.
fn format_ext(format: ModelFormat) -> &'static str {
    match format {
        ModelFormat::Stl => "stl",
        ModelFormat::Obj => "obj",
        ModelFormat::ThreeMf => "3mf",
        ModelFormat::Step => "step",
        ModelFormat::Amf => "amf",
        ModelFormat::Svg => "svg",
        ModelFormat::Drc => "drc",
    }
}

/// Contrôle de contenu minimal par format : rejette un fichier vide ou dont la
/// signature ne correspond pas (fichier corrompu / mal étiqueté).
pub fn validate_content(format: ModelFormat, bytes: &[u8]) -> Result<(), &'static str> {
    if bytes.is_empty() {
        return Err("fichier vide");
    }
    match format {
        // 3MF est un conteneur ZIP : signature « PK\x03\x04 ».
        ModelFormat::ThreeMf => {
            if bytes.starts_with(b"PK\x03\x04") {
                Ok(())
            } else {
                Err("3MF invalide (archive ZIP attendue)")
            }
        }
        // STEP est un texte ISO-10303 (« ISO-10303 » dans l'en-tête).
        ModelFormat::Step => {
            let head = &bytes[..bytes.len().min(256)];
            if contains(head, b"ISO-10303") {
                Ok(())
            } else {
                Err("STEP invalide (en-tête ISO-10303 absent)")
            }
        }
        // STL : soit ASCII (« solid »), soit binaire (taille cohérente).
        ModelFormat::Stl => {
            let ascii = bytes.len() >= 5 && bytes[..5].eq_ignore_ascii_case(b"solid");
            if ascii || binary_stl_triangle_count(format, bytes).is_some() {
                Ok(())
            } else {
                Err("STL invalide (ni ASCII ni binaire cohérent)")
            }
        }
        // OBJ : texte UTF-8 (validation légère, laisse passer).
        ModelFormat::Obj => {
            if std::str::from_utf8(bytes).is_ok() {
                Ok(())
            } else {
                Err("OBJ invalide (texte UTF-8 attendu)")
            }
        }
        // AMF : XML texte (`<amf`/`<?xml`) ou conteneur `.zip.amf` (« PK »).
        ModelFormat::Amf => {
            let head = &bytes[..bytes.len().min(256)];
            if bytes.starts_with(b"PK\x03\x04")
                || contains(head, b"<amf")
                || contains(head, b"<?xml")
            {
                Ok(())
            } else {
                Err("AMF invalide (XML ou archive .zip.amf attendus)")
            }
        }
        // SVG : XML texte contenant une balise `<svg`.
        ModelFormat::Svg => {
            if std::str::from_utf8(bytes).is_ok()
                && contains(&bytes[..bytes.len().min(512)], b"<svg")
            {
                Ok(())
            } else {
                Err("SVG invalide (balise <svg absente)")
            }
        }
        // DRC : maillage Draco, en-tête magique « DRACO ».
        ModelFormat::Drc => {
            if bytes.starts_with(b"DRACO") {
                Ok(())
            } else {
                Err("Draco invalide (en-tête DRACO absent)")
            }
        }
    }
}

/// Nombre de triangles d'un STL **binaire** (en-tête 80 o + u32 + n×50 o).
/// `None` si ce n'est pas un STL binaire cohérent (ASCII, autre format…).
pub fn binary_stl_triangle_count(format: ModelFormat, bytes: &[u8]) -> Option<u32> {
    if format != ModelFormat::Stl || bytes.len() < 84 {
        return None;
    }
    let count = u32::from_le_bytes([bytes[80], bytes[81], bytes[82], bytes[83]]);
    let expected = 84usize.checked_add((count as usize).checked_mul(50)?)?;
    (expected == bytes.len()).then_some(count)
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn binary_stl(n: u32) -> Vec<u8> {
        let mut v = vec![0u8; 80];
        v.extend_from_slice(&n.to_le_bytes());
        v.extend(std::iter::repeat_n(0u8, n as usize * 50));
        v
    }

    #[test]
    fn detects_formats_by_extension() {
        assert_eq!(detect_format("a.stl"), Some(ModelFormat::Stl));
        assert_eq!(detect_format("A.STL"), Some(ModelFormat::Stl));
        assert_eq!(detect_format("b.obj"), Some(ModelFormat::Obj));
        assert_eq!(detect_format("c.3mf"), Some(ModelFormat::ThreeMf));
        assert_eq!(detect_format("d.step"), Some(ModelFormat::Step));
        assert_eq!(detect_format("d.stp"), Some(ModelFormat::Step));
        // Jeu OrcaSlicer étendu (T091) : oltp→STL, amf/xml→AMF, svg, drc.
        assert_eq!(detect_format("f.oltp"), Some(ModelFormat::Stl));
        assert_eq!(detect_format("g.amf"), Some(ModelFormat::Amf));
        assert_eq!(detect_format("g.zip.amf"), Some(ModelFormat::Amf));
        assert_eq!(detect_format("h.xml"), Some(ModelFormat::Amf));
        assert_eq!(detect_format("i.svg"), Some(ModelFormat::Svg));
        assert_eq!(detect_format("j.drc"), Some(ModelFormat::Drc));
        // `.zip` conteneur et formats Apple-only = hors jeu v1 (exclusions.md).
        assert_eq!(detect_format("k.zip"), None);
        assert_eq!(detect_format("l.ply"), None);
        assert_eq!(detect_format("e.gif"), None);
        assert_eq!(detect_format("noext"), None);
    }

    #[test]
    fn conversion_flag_covers_engine_formats() {
        assert!(ModelFormat::Step.needs_engine_conversion());
        assert!(ModelFormat::Amf.needs_engine_conversion());
        assert!(ModelFormat::Svg.needs_engine_conversion());
        assert!(ModelFormat::Drc.needs_engine_conversion());
        assert!(!ModelFormat::Stl.needs_engine_conversion());
        assert!(!ModelFormat::Obj.needs_engine_conversion());
        assert!(!ModelFormat::ThreeMf.needs_engine_conversion());
    }

    #[test]
    fn counts_binary_stl_triangles() {
        assert_eq!(
            binary_stl_triangle_count(ModelFormat::Stl, &binary_stl(3)),
            Some(3)
        );
        // Taille incohérente → None.
        let mut bad = binary_stl(3);
        bad.push(0);
        assert_eq!(binary_stl_triangle_count(ModelFormat::Stl, &bad), None);
    }

    #[test]
    fn validates_content_signatures() {
        assert!(validate_content(ModelFormat::Stl, &binary_stl(2)).is_ok());
        assert!(validate_content(ModelFormat::Stl, b"solid cube\n").is_ok());
        assert!(validate_content(ModelFormat::Stl, b"garbage").is_err());
        assert!(validate_content(ModelFormat::ThreeMf, b"PK\x03\x04rest").is_ok());
        assert!(validate_content(ModelFormat::ThreeMf, b"not a zip").is_err());
        assert!(validate_content(ModelFormat::Step, b"ISO-10303-21;\nHEADER;").is_ok());
        assert!(validate_content(ModelFormat::Step, b"nope").is_err());
        assert!(validate_content(ModelFormat::Obj, b"v 0 0 0\n").is_ok());
        assert!(validate_content(ModelFormat::Stl, b"").is_err());
        // Formats étendus (T091).
        assert!(validate_content(ModelFormat::Amf, b"<?xml version=\"1.0\"?><amf>").is_ok());
        assert!(validate_content(ModelFormat::Amf, b"PK\x03\x04rest").is_ok());
        assert!(validate_content(ModelFormat::Amf, b"garbage").is_err());
        assert!(validate_content(ModelFormat::Svg, b"<svg xmlns=\"...\">").is_ok());
        assert!(validate_content(ModelFormat::Svg, b"not svg").is_err());
        assert!(validate_content(ModelFormat::Drc, b"DRACO\x00\x01").is_ok());
        assert!(validate_content(ModelFormat::Drc, b"nope").is_err());
    }
}
