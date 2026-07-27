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

/// `GET /api/projects/{id}/export/stl` — tous les objets du projet **fusionnés**
/// en un seul STL binaire (pièce jointe, T154). Réutilise l'assemblage moteur
/// (`build_stl_model`) puis l'écrivain STL du moteur (`TriangleMesh::to_binary_stl`).
/// Un projet d'autrui répond 404 (SC-008).
pub async fn export_stl(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(project_raw): Path<String>,
) -> ApiResult<Response> {
    let project_id = parse_project_id(&project_raw)?;
    let project = state.storage.projects().get(user.id, project_id).await?; // 404, SC-008
    let model = build_stl_model(&state, user.id, project_id, &project.scene).await?;
    let merged = merge_model_meshes(&model);
    if merged.is_empty() {
        return Err(ApiError::validation(
            "aucun objet STL à exporter",
            serde_json::json!({}),
        ));
    }
    let filename = format!("{}.stl", sanitize(&project.name));
    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "model/stl".to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{filename}\""),
            ),
        ],
        merged.to_binary_stl(),
    )
        .into_response())
}

/// Fusionne tous les volumes d'un `Model` en un seul maillage indexé (les indices
/// de chaque volume sont décalés du nombre de sommets déjà accumulés).
fn merge_model_meshes(model: &engine::api::Model) -> engine::api::TriangleMesh {
    let mut merged = engine::api::TriangleMesh::default();
    for object in &model.objects {
        for volume in &object.volumes {
            let offset = merged.vertices.len() as u32;
            merged.vertices.extend_from_slice(&volume.mesh.vertices);
            merged.indices.extend(
                volume
                    .mesh
                    .indices
                    .iter()
                    .map(|t| [t[0] + offset, t[1] + offset, t[2] + offset]),
            );
        }
    }
    merged
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
    // Profil de hauteur de couche variable par modèle (T147) : vecteur plat.
    let layer_profiles = layer_profiles_by_model(scene);
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
        if let Some(profile) = layer_profiles.get(&m.id.to_string()) {
            object.layer_height_profile = profile.clone();
        }
    }
    Ok(model)
}

/// Extrait le profil de hauteur variable par `modelId` du document scène.
/// Forme : `scene.objects[] = { modelId, layerProfile: [z0, h0, z1, h1, …] }`
/// (vecteur plat, miroir de `ModelObject::layer_height_profile`).
fn layer_profiles_by_model(scene: &serde_json::Value) -> HashMap<String, Vec<f64>> {
    let mut out = HashMap::new();
    let Some(objects) = scene.get("objects").and_then(|v| v.as_array()) else {
        return out;
    };
    for object in objects {
        let Some(model_id) = object.get("modelId").and_then(|v| v.as_str()) else {
            continue;
        };
        let profile: Vec<f64> = object
            .get("layerProfile")
            .and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|v| v.as_f64()).collect())
            .unwrap_or_default();
        if !profile.is_empty() {
            out.insert(model_id.to_string(), profile);
        }
    }
    out
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
