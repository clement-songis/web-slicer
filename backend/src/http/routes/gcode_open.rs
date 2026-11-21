//! Ouverture d'un G-code local (menu « Open G-code… ») : aperçu sans stockage.
//!
//! `POST /api/gcode/preview` (corps = texte G-code brut) renvoie, comme les
//! routes `/api/gcodes/{id}/preview/*` :
//! - `?meta=1` : la méta JSON (`PreviewMeta`) ;
//! - sinon `?from=&to=` : le buffer binaire `WSPv` de la plage de couches.
//!
//! Sans état : le client renvoie le fichier pour chaque fenêtre de couches
//! (chargement paresseux) ; rien n'est écrit côté serveur. Même parseur que
//! l'aperçu des tranchages (`crate::gcode::parse_preview`).
//!
//! Simulation moteur (lot 31, comme `GUI_App` en mode visionneuse :
//! `GCodeProcessor::process_file` sur le fichier ouvert) :
//! - `POST /api/gcode/process` (corps = G-code brut) : le fichier est gardé
//!   dans l'espace de l'utilisateur (`opened-gcodes/<empreinte>.gcode`, les
//!   `OPENED_KEEP` derniers), simulé par `engine-worker gcode-process` (cache
//!   `.wspg`) → `{ token, meta, stats, footer }` ;
//! - `GET /api/gcode/opened/{token}/meta|moves|seams` : mêmes réponses que
//!   `/api/gcodes/{id}/preview/processed/*`, sans renvoyer le fichier.
//!
//! Lot 44 (audit 43 X-f) : le corps peut aussi être un **.gcode.3mf** (fichier
//! tranché d'Orca / de web-slicer : archive ZIP avec `Metadata/plate_<n>.gcode`,
//! sans géométrie — `store_bbs_3mf` en `SkipModel`). Comme Orca qui le rouvre
//! en aperçu (`Plater::is_gcode_3mf`), le G-code du plateau `?plate=<n>` (à
//! défaut le premier plateau tranché) est extrait puis traité comme un G-code
//! ouvert ; la réponse de `process` indique `plate` et `plates` (plateaux
//! tranchés de l'archive).

use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;

use crate::gcode::parse_preview;
use crate::http::dto::PreviewMeta;
use crate::http::error::{ApiError, ApiResult};
use crate::http::extract::CurrentUser;
use crate::http::routes::preview::{engine_stats, seams_json, LayerRange};
use crate::http::state::AppState;

/// Nombre de G-code externes conservés par compte (avec leur cache).
pub const OPENED_KEEP: usize = 4;

/// Taille maximale d'un G-code extrait d'un .gcode.3mf (décompressé).
const MAX_GCODE_3MF_ENTRY: u64 = 2 * 1024 * 1024 * 1024;

/// G-code d'un plateau d'un .gcode.3mf.
#[derive(Debug)]
pub(crate) struct ArchiveGcode {
    /// Plateau extrait (1-based, comme `Metadata/plate_<n>.gcode`).
    pub plate: u32,
    /// Plateaux tranchés présents dans l'archive (triés).
    pub plates: Vec<u32>,
    pub gcode: Vec<u8>,
}

/// Le corps est-il une archive ZIP (3MF) plutôt qu'un G-code texte ?
pub(crate) fn is_zip(body: &[u8]) -> bool {
    body.starts_with(b"PK\x03\x04")
}

/// Plateaux tranchés d'une archive (`Metadata/plate_<n>.gcode`), triés.
pub(crate) fn archive_gcode_plates<R: std::io::Read + std::io::Seek>(
    zip: &zip::ZipArchive<R>,
) -> Vec<u32> {
    let mut plates: Vec<u32> = zip
        .file_names()
        .filter_map(|name| {
            name.strip_prefix("Metadata/plate_")?
                .strip_suffix(".gcode")?
                .parse()
                .ok()
        })
        .collect();
    plates.sort_unstable();
    plates.dedup();
    plates
}

/// Extrait le G-code du plateau `plate` (1-based ; `None` = premier plateau
/// tranché) d'un .gcode.3mf. 422 si l'archive est illisible, sans G-code de
/// plateau, ou si le plateau demandé n'y est pas.
pub(crate) fn gcode_from_archive(body: &[u8], plate: Option<u32>) -> ApiResult<ArchiveGcode> {
    use std::io::Read;
    let invalid =
        |message: &str, details: serde_json::Value| ApiError::validation(message, details);
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(body))
        .map_err(|_| invalid("archive 3MF illisible", serde_json::json!({})))?;
    let plates = archive_gcode_plates(&zip);
    if plates.is_empty() {
        return Err(invalid(
            "ce 3MF ne contient aucun G-code de plateau (Metadata/plate_<n>.gcode) : \
             ouvrez-le comme projet (Fichier › Ouvrir un projet)",
            serde_json::json!({ "kind": "project_3mf" }),
        ));
    }
    let chosen = match plate {
        None => plates[0],
        Some(p) if plates.contains(&p) => p,
        Some(p) => {
            return Err(invalid(
                "plateau non tranché dans ce .gcode.3mf",
                serde_json::json!({ "plate": p, "plates": plates }),
            ))
        }
    };
    let mut entry = zip
        .by_name(&format!("Metadata/plate_{chosen}.gcode"))
        .map_err(|_| invalid("archive 3MF illisible", serde_json::json!({})))?;
    if entry.size() > MAX_GCODE_3MF_ENTRY {
        return Err(invalid(
            "G-code du .gcode.3mf trop volumineux",
            serde_json::json!({ "plate": chosen }),
        ));
    }
    let mut gcode = Vec::with_capacity(entry.size() as usize);
    entry
        .by_ref()
        .take(MAX_GCODE_3MF_ENTRY)
        .read_to_end(&mut gcode)
        .map_err(|_| invalid("archive 3MF illisible", serde_json::json!({})))?;
    Ok(ArchiveGcode {
        plate: chosen,
        plates,
        gcode,
    })
}

/// Plateau d'un .gcode.3mf à ouvrir (`?plate=<n>`, 1-based).
#[derive(Debug, Default, Deserialize)]
pub struct ProcessQuery {
    #[serde(default)]
    plate: Option<u32>,
}

/// `POST /api/gcode/process` — simulation GCodeProcessor d'un G-code ouvert.
pub async fn process(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Query(query): Query<ProcessQuery>,
    body: Bytes,
) -> ApiResult<Response> {
    use md5::{Digest, Md5};
    if body.is_empty() {
        return Err(ApiError::validation("G-code vide", serde_json::json!({})));
    }
    // .gcode.3mf : G-code du plateau extrait (X-f), puis même traitement.
    let archive = if is_zip(&body) {
        let body = body.clone();
        Some(
            tokio::task::spawn_blocking(move || gcode_from_archive(&body, query.plate))
                .await
                .map_err(|_| ApiError::internal())??,
        )
    } else {
        None
    };
    let body: Bytes = match &archive {
        Some(a) => Bytes::from(a.gcode.clone()),
        None => body,
    };
    let token: String = Md5::digest(&body)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    // Quota de stockage (lot 32) : seul un fichier pas encore gardé compte.
    let already_kept = state
        .files
        .opened_gcode_path(user.id, &token)
        .is_ok_and(|p| p.is_file());
    if !already_kept {
        crate::http::quota::ensure_storage_room(&state, user.id, body.len() as i64).await?;
    }
    let path = state
        .files
        .write_opened_gcode(user.id, &token, &body, OPENED_KEEP)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "écriture du G-code ouvert");
            ApiError::internal()
        })?;
    let p = simulate(&path).await?;
    if p.layers.is_empty() {
        return Err(ApiError::validation(
            "aucune couche simulée : ce fichier n'est pas un G-code imprimable",
            serde_json::json!({}),
        ));
    }
    let footer = crate::gcode::parse_stats(&String::from_utf8_lossy(&body));
    let mut response = serde_json::json!({
        "token": token,
        "stats": engine_stats(&p),
        "footer": footer,
        "meta": p.meta,
    });
    if let Some(a) = &archive {
        response["plate"] = serde_json::json!(a.plate);
        response["plates"] = serde_json::json!(a.plates);
    }
    Ok(Json(response).into_response())
}

async fn simulate(
    path: &std::path::Path,
) -> ApiResult<std::sync::Arc<engine::api::gcode_process::ProcessedGcode>> {
    crate::engine::gcode_process::processed(path)
        .await
        .map_err(|e| {
            tracing::warn!(error = %e, "GCodeProcessor (G-code ouvert)");
            ApiError::conversion_failed("simulation du G-code par le moteur impossible")
        })
}

/// G-code ouvert du compte (404 si jeton inconnu, invalide ou d'un autre compte).
async fn opened(
    state: &AppState,
    user: crate::domain::UserId,
    token: &str,
) -> ApiResult<std::sync::Arc<engine::api::gcode_process::ProcessedGcode>> {
    let path = state
        .files
        .opened_gcode_path(user, token)
        .map_err(|_| ApiError::not_found("G-code"))?;
    if tokio::fs::metadata(&path).await.is_err() {
        return Err(ApiError::not_found("G-code"));
    }
    simulate(&path).await
}

/// `GET /api/gcode/opened/{token}/meta`.
pub async fn opened_meta(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let p = opened(&state, user.id, &token).await?;
    Ok(Json(p.meta.clone()))
}

/// `GET /api/gcode/opened/{token}/moves?from=&to=` (format `WSPm`).
pub async fn opened_moves(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(token): Path<String>,
    Query(range): Query<LayerRange>,
) -> ApiResult<Response> {
    let p = opened(&state, user.id, &token).await?;
    let buffer = p.encode_range(range.from.unwrap_or(0), range.to.unwrap_or(usize::MAX));
    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/octet-stream")],
        buffer,
    )
        .into_response())
}

/// `GET /api/gcode/opened/{token}/seams?from=&to=`.
pub async fn opened_seams(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(token): Path<String>,
    Query(range): Query<LayerRange>,
) -> ApiResult<Json<serde_json::Value>> {
    let p = opened(&state, user.id, &token).await?;
    Ok(Json(seams_json(&p, &range)))
}

#[derive(Debug, Deserialize)]
pub struct OpenQuery {
    #[serde(default)]
    meta: Option<u8>,
    /// Plateau d'un .gcode.3mf (1-based ; défaut : premier plateau tranché).
    #[serde(default)]
    plate: Option<u32>,
    from: Option<usize>,
    to: Option<usize>,
}

/// `POST /api/gcode/preview` — aperçu d'un G-code fourni par le client.
pub async fn preview(
    CurrentUser(_user): CurrentUser,
    Query(query): Query<OpenQuery>,
    body: Bytes,
) -> ApiResult<Response> {
    if body.is_empty() {
        return Err(ApiError::validation("G-code vide", serde_json::json!({})));
    }
    let body: Bytes = if is_zip(&body) {
        let plate = query.plate;
        let archive = tokio::task::spawn_blocking(move || gcode_from_archive(&body, plate))
            .await
            .map_err(|_| ApiError::internal())??;
        Bytes::from(archive.gcode)
    } else {
        body
    };
    let text = String::from_utf8_lossy(&body);
    let model = parse_preview(&text);
    if model.layer_count() == 0 {
        return Err(ApiError::validation(
            "aucune couche reconnue : ce fichier n'est pas un G-code OrcaSlicer",
            serde_json::json!({}),
        ));
    }
    if query.meta.unwrap_or(0) != 0 {
        return Ok(Json(PreviewMeta::from(model.summary())).into_response());
    }
    let from = query.from.unwrap_or(0);
    let to = query
        .to
        .unwrap_or_else(|| model.layer_count().saturating_sub(1));
    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/octet-stream")],
        model.encode_range(from, to),
    )
        .into_response())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn archive(entries: &[(&str, &str)]) -> Vec<u8> {
        let mut out = std::io::Cursor::new(Vec::new());
        {
            let mut writer = zip::ZipWriter::new(&mut out);
            for (name, text) in entries {
                writer
                    .start_file(*name, zip::write::SimpleFileOptions::default())
                    .unwrap();
                std::io::Write::write_all(&mut writer, text.as_bytes()).unwrap();
            }
            writer.finish().unwrap();
        }
        out.into_inner()
    }

    #[test]
    fn gcode_3mf_yields_the_first_or_the_requested_plate() {
        let bytes = archive(&[
            ("3D/3dmodel.model", "<model/>"),
            ("Metadata/plate_2.gcode", "; plate 2\n"),
            ("Metadata/plate_2.gcode.md5", "x"),
            ("Metadata/plate_1.gcode", "; plate 1\n"),
        ]);
        assert!(is_zip(&bytes));
        let first = gcode_from_archive(&bytes, None).unwrap();
        assert_eq!((first.plate, first.plates.clone()), (1, vec![1, 2]));
        assert_eq!(first.gcode, b"; plate 1\n");
        let second = gcode_from_archive(&bytes, Some(2)).unwrap();
        assert_eq!(second.gcode, b"; plate 2\n");
        assert!(gcode_from_archive(&bytes, Some(3)).is_err());
    }

    #[test]
    fn project_3mf_without_gcode_is_refused_with_a_hint() {
        let bytes = archive(&[("3D/3dmodel.model", "<model/>")]);
        let error = gcode_from_archive(&bytes, None).unwrap_err();
        assert!(format!("{error:?}").contains("project_3mf"));
        assert!(!is_zip(b"; G-code\nG1 X0\n"));
    }
}
