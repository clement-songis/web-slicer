//! Export de projet 3MF (`GET /api/projects/{id}/export/3mf`, T072, FR-044).
//!
//! Assemble les maillages du projet et la configuration figée (presets actifs
//! résolus) en un projet 3MF **compatible OrcaSlicer** via `engine::threemf`
//! (writer pur Rust). Géométrie STL décodée côté serveur ; les formats dont la
//! géométrie serveur dépend du moteur (OBJ/3MF/STEP) sont ignorés à l'export en
//! attendant le writer libslic3r (`adapters::ffi::write_project_3mf`, T066+).
//!
//! Isolation (SC-008) : un projet d'autrui répond 404.

use axum::extract::{Path, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use std::collections::HashMap;
use std::path::Path as FsPath;

use crate::domain::{ModelFormat, ProjectId};
use crate::http::error::{ApiError, ApiResult};
use crate::http::extract::CurrentUser;
use crate::http::state::AppState;
use crate::mesh::{parse_stl, Mesh};

fn parse_project_id(raw: &str) -> ApiResult<ProjectId> {
    uuid::Uuid::parse_str(raw)
        .map(ProjectId)
        .map_err(|_| ApiError::not_found("Projet"))
}

/// `GET /api/projects/{id}/export/3mf` — projet 3MF (pièce jointe).
pub async fn export_3mf(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(project_raw): Path<String>,
) -> ApiResult<Response> {
    let project_id = parse_project_id(&project_raw)?;
    let project = state.storage.projects().get(user.id, project_id).await?; // 404, SC-008

    // Géométrie : maillages STL décodés côté serveur (+ peinture au triangle
    // rattachée depuis la scène : supports/couture/MMU/fuzzy).
    let model = build_stl_model(&state, user.id, project_id, &project.scene).await?;

    // Configuration figée : presets actifs résolus.
    let (config, _warnings) =
        super::slice::resolve_active_config(&state, user.id, &project.active_presets).await?;

    let bytes = engine::threemf::write_project_bytes(&model, &config).map_err(|e| {
        tracing::error!(error = %e, "écriture du projet 3MF");
        ApiError::internal()
    })?;

    let filename = format!("{}.3mf", sanitize(&project.name));
    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "model/3mf".to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{filename}\""),
            ),
        ],
        bytes,
    )
        .into_response())
}

/// Assemble le `Model` moteur d'un projet à partir de ses maillages **STL**
/// décodés côté serveur. Source unique de l'export 3MF et du runner de tranchage
/// (les formats dont la géométrie dépend du moteur — OBJ/3MF/STEP — restent à
/// câbler via le writer libslic3r). Un projet d'autrui répond 404 (SC-008).
pub(crate) async fn build_stl_model(
    state: &AppState,
    user: crate::domain::UserId,
    project_id: ProjectId,
    scene: &serde_json::Value,
) -> ApiResult<engine::api::Model> {
    let models = state.storage.models().list(user, Some(project_id)).await?;
    // Peinture au triangle par modèle, extraite du document scène (les indices de
    // triangle référencent le maillage STL — même ordre de facettes des deux côtés).
    let painting = painting_by_model(scene);
    let mut model = engine::api::Model::default();
    for m in models.iter().filter(|m| m.format == ModelFormat::Stl) {
        let bytes = state
            .files
            .read(FsPath::new(&m.file_path))
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "lecture du modèle (assemblage moteur)");
                ApiError::internal()
            })?;
        let mesh = parse_stl(&bytes).map_err(|e| {
            tracing::error!(error = %e, "décodage STL (assemblage moteur)");
            ApiError::validation(
                "STL illisible",
                serde_json::json!({ "model": m.id.to_string() }),
            )
        })?;
        let object = model.add_object(m.filename.clone(), to_triangle_mesh(&mesh));
        if let Some(painted) = painting.get(&m.id.to_string()) {
            // add_object crée un unique volume `ModelPart` : on y pose la peinture.
            if let Some(volume) = object.volumes.first_mut() {
                volume.painted = painted.clone();
            }
        }
    }
    Ok(model)
}

/// Extrait la peinture au triangle du document scène, indexée par `modelId`.
/// Forme attendue : `scene.objects[] = { modelId, painting: { supports: {"12":"4"},
/// seam: {...}, fuzzy: {...}, mmu: {...} } }` (format 3MF Orca par facette). Les
/// canaux sont normalisés (tri par index croissant) pour `set_triangle_from_string`.
fn painting_by_model(scene: &serde_json::Value) -> HashMap<String, engine::api::PaintedFacets> {
    let mut out = HashMap::new();
    let Some(objects) = scene.get("objects").and_then(|v| v.as_array()) else {
        return out;
    };
    for object in objects {
        let Some(model_id) = object.get("modelId").and_then(|v| v.as_str()) else {
            continue;
        };
        let Some(painting) = object.get("painting") else {
            continue;
        };
        let painted = engine::api::PaintedFacets {
            supports: channel_facets(painting, "supports"),
            seam: channel_facets(painting, "seam"),
            mmu: channel_facets(painting, "mmu"),
            fuzzy: channel_facets(painting, "fuzzy"),
        }
        .normalized();
        if !painted.is_empty() {
            out.insert(model_id.to_string(), painted);
        }
    }
    out
}

/// Facettes d'un canal : `{ "triangleIndex": "hexOrca" }` → `(u32, String)`.
fn channel_facets(painting: &serde_json::Value, channel: &str) -> Vec<(u32, String)> {
    painting
        .get(channel)
        .and_then(|v| v.as_object())
        .map(|map| {
            map.iter()
                .filter_map(|(idx, hex)| {
                    Some((idx.parse::<u32>().ok()?, hex.as_str()?.to_string()))
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Maillage backend (aplati) → `TriangleMesh` moteur (sommets/triangles indexés).
fn to_triangle_mesh(mesh: &Mesh) -> engine::api::TriangleMesh {
    engine::api::TriangleMesh {
        vertices: mesh
            .positions
            .chunks_exact(3)
            .map(|c| [c[0], c[1], c[2]])
            .collect(),
        indices: mesh
            .indices
            .chunks_exact(3)
            .map(|c| [c[0], c[1], c[2]])
            .collect(),
    }
}

/// Nom de fichier sûr (alphanumérique + `-_`), repli `project`.
fn sanitize(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let trimmed = cleaned.trim_matches('_');
    if trimmed.is_empty() {
        "project".to_string()
    } else {
        trimmed.to_string()
    }
}
