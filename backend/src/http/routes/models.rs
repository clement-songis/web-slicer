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

use axum::extract::{Multipart, Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;

use crate::domain::{Model, ModelFormat, ModelId, NewModel, ProjectId, UserId};
use crate::http::dto::ModelResponse;
use crate::http::error::{ApiError, ApiResult};
use crate::http::extract::CurrentUser;
use crate::http::state::AppState;

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

    let (filename, format, bytes) = read_model_upload(&state, user.id, multipart).await?;
    let model = write_model_record(&state, user.id, project_id, format, filename, &bytes).await?;
    // Décodage moteur asynchrone (R7/T123) : la réponse part immédiatement, le
    // maillage arrive via l'événement WS `model.converted`.
    spawn_conversion(&state, model.clone());
    Ok((StatusCode::CREATED, Json(model.into())))
}

/// Déclenche la conversion moteur du modèle en tâche détachée (bornée par le
/// sémaphore du convertisseur). L'upload ne bloque pas ; un échec est persisté
/// (`conversion_error`) sans affecter la réponse.
pub(crate) fn spawn_conversion(state: &AppState, model: Model) {
    let converter = state.converter.clone();
    tokio::spawn(async move {
        converter.convert(model).await;
    });
}

/// Lit un fichier modèle du multipart, applique la limite d'upload de l'instance
/// (FR-053) puis détecte et valide le format. Partagé entre l'upload dans un
/// projet et l'import de projet (T090).
pub(crate) async fn read_model_upload(
    state: &AppState,
    user: crate::domain::UserId,
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
    // Quota de stockage du compte (lot 32) : 413 `storage_quota_exceeded`.
    crate::http::quota::ensure_storage_room(state, user, bytes.len() as i64).await?;

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

/// Lot 46 (audit 44 OLD-a) : fichier d'un modèle absent du magasin (données
/// déplacées / perdues) → 409 lisible (`details.reason` = `model_file_missing`)
/// au lieu d'un 500 « Erreur interne ».
pub(crate) fn missing_model_file(
    e: crate::adapters::files::FilesError,
    model: &Model,
    context: &str,
) -> ApiError {
    let missing = matches!(&e, crate::adapters::files::FilesError::Io(io) if io.kind() == std::io::ErrorKind::NotFound);
    tracing::error!(error = %e, model = %model.id, "{context}");
    if !missing {
        return ApiError::internal();
    }
    ApiError::conflict_with(
        format!(
            "Fichier du modèle « {} » introuvable sur le serveur",
            model.filename
        ),
        serde_json::json!({
            "reason": "model_file_missing",
            "model": model.id.to_string(),
            "filename": model.filename,
        }),
    )
}

/// Copie un modèle et tous ses artefacts sous de nouvelles clés. Le maillage
/// prêt est copié directement ; l'état d'erreur est conservé. Un modèle encore
/// en attente sera converti par l'appelant après remappage de la scène.
pub(crate) async fn clone_model_record(
    state: &AppState,
    user_id: UserId,
    project_id: ProjectId,
    source: &Model,
) -> ApiResult<Model> {
    let bytes = state
        .files
        .read(FsPath::new(&source.file_path))
        .await
        .map_err(|e| missing_model_file(e, source, "lecture du modèle à dupliquer"))?;
    let mesh =
        match source.mesh_path.as_deref() {
            Some(path) => {
                Some(state.files.read(FsPath::new(path)).await.map_err(|e| {
                    missing_model_file(e, source, "lecture du maillage à dupliquer")
                })?)
            }
            None => None,
        };

    let storage_key = ModelId::new();
    let file_path = state
        .files
        .write_model(user_id, storage_key, format_ext(source.format), &bytes)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "écriture de la copie du modèle");
            ApiError::internal()
        })?;
    let file_path_text = file_path.to_string_lossy().into_owned();
    let mut copy = state
        .storage
        .models()
        .create(
            user_id,
            NewModel {
                project_id: Some(project_id),
                filename: source.filename.clone(),
                format: source.format,
                file_path: file_path_text.clone(),
                mesh_path: None,
                size_bytes: source.size_bytes,
                triangle_count: source.triangle_count,
                repair_report: source.repair_report.clone(),
            },
        )
        .await?;

    let result = async {
        if let Some(bytes) = mesh {
            let path = state
                .files
                .write_mesh(user_id, copy.id, &bytes)
                .await
                .map_err(|e| {
                    tracing::error!(error = %e, "écriture de la copie du maillage");
                    ApiError::internal()
                })?;
            let path = path.to_string_lossy().into_owned();
            state
                .storage
                .models()
                .set_mesh(user_id, copy.id, &path, source.triangle_count)
                .await?;
            copy.mesh_path = Some(path);
        }
        if let Some(error) = source.conversion_error.as_deref() {
            state
                .storage
                .models()
                .mark_conversion_failed(user_id, copy.id, error)
                .await?;
            copy.conversion_error = Some(error.to_string());
        }
        Ok(())
    }
    .await;
    if let Err(error) = result {
        let _ = state.storage.models().delete(user_id, copy.id).await;
        let _ = tokio::fs::remove_file(&file_path).await;
        if let Some(path) = copy.mesh_path.as_deref() {
            let _ = tokio::fs::remove_file(path).await;
        }
        return Err(error);
    }
    Ok(copy)
}

fn parse_model_id(raw: &str) -> ApiResult<ModelId> {
    uuid::Uuid::parse_str(raw)
        .map(ModelId)
        .map_err(|_| ApiError::not_found("Modèle"))
}

/// `GET /api/models/{id}/mesh` — maillage d'affichage au format binaire compact
/// « WSMh » (positions/normales/indices, little-endian) pour Threlte.
///
/// **Tous** les formats sont décodés par le moteur (source de vérité unique,
/// R7/T124) : la route sert le maillage **stocké** (`mesh_path`) une fois la
/// conversion terminée (200). Tant qu'elle est en cours → **409** ; si elle a
/// échoué (`conversion_error`) → **422**. Plus de parseur STL serveur ni
/// d'aperçu client.
/// Attente maximale par défaut de `GET /api/models/{id}/mesh` pendant la
/// conversion (s) ; `?wait=<s>` la règle (0 = réponse immédiate), bornée.
pub const MESH_WAIT_DEFAULT_SECS: u64 = 30;
const MESH_WAIT_MAX_SECS: u64 = 120;

#[derive(Debug, Default, serde::Deserialize)]
pub struct MeshQuery {
    /// Secondes d'attente de la fin de conversion avant de répondre 409.
    pub wait: Option<u64>,
}

/// `GET /api/models/{id}/mesh` — maillage converti (WSMh). Pendant la
/// conversion asynchrone (import : 1 à quelques secondes), la requête ATTEND
/// sa fin (au plus `wait` s, défaut 30) au lieu de répondre 409 tout de suite :
/// l'éditeur demande le maillage juste après l'import et chaque import
/// produisait un 409 en console (audit J1-e). Au-delà du délai : 409
/// « conversion en cours » (inchangé) ; échec de conversion : 422.
pub async fn mesh(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<MeshQuery>,
) -> ApiResult<Response> {
    let id = parse_model_id(&id)?;
    let wait = std::time::Duration::from_secs(
        query
            .wait
            .unwrap_or(MESH_WAIT_DEFAULT_SECS)
            .min(MESH_WAIT_MAX_SECS),
    );
    let model = wait_converted(&state, user.id, id, wait).await?;
    let mesh_path = model.mesh_path.as_deref().expect("maillage prêt");

    let bytes = state
        .files
        .read(FsPath::new(mesh_path))
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "lecture du maillage");
            ApiError::internal()
        })?;

    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/octet-stream")],
        bytes,
    )
        .into_response())
}

/// Attend (au plus `wait`) la fin de la conversion asynchrone du modèle `id`
/// et le renvoie avec son `mesh_path`. Échec de conversion persisté → 422 ;
/// délai dépassé → 409 « conversion en cours » ; modèle d'autrui → 404.
///
/// Partagé par `GET /api/models/{id}/mesh` et l'assemblage moteur des exports
/// (`export::build_project_model`) : un projet tout juste importé (conversion
/// lancée en tâche de fond après la réponse) s'exporte comme dans Orca, où le
/// modèle chargé est immédiatement disponible.
pub(crate) async fn wait_converted(
    state: &AppState,
    user: UserId,
    id: ModelId,
    wait: std::time::Duration,
) -> ApiResult<Model> {
    let deadline = tokio::time::Instant::now() + wait;
    // Abonnement AVANT la première lecture : un `model.converted` publié entre
    // la lecture et l'attente n'est pas perdu.
    let mut events = state.events.subscribe(user);
    loop {
        let model = state.storage.models().get(user, id).await?; // 404 si autre compte
        if model.mesh_path.is_some() {
            return Ok(model);
        }
        if let Some(error) = model.conversion_error.as_deref() {
            return Err(ApiError::conversion_failed_for(model.id, error));
        }
        let now = tokio::time::Instant::now();
        if now >= deadline {
            return Err(ApiError::conflict(
                "conversion en cours (voir l'événement model.converted)",
            ));
        }
        // Réveil sur tout événement du compte (model.converted…) ou, au plus
        // tard, toutes les 250 ms (échec de conversion : pas d'événement dédié).
        let tick = (deadline - now).min(std::time::Duration::from_millis(250));
        if let Ok(Err(tokio::sync::broadcast::error::RecvError::Closed)) =
            tokio::time::timeout(tick, events.recv()).await
        {
            tokio::time::sleep(tick).await;
        }
    }
}

/// Attente maximale de la conversion d'un modèle par les exports (s) : même
/// borne que `?wait` de `/mesh`.
pub(crate) const EXPORT_CONVERSION_WAIT_SECS: u64 = MESH_WAIT_MAX_SECS;

/// `GET /api/projects/{id}/models` — modèles rattachés à un projet (T092), pour
/// repeupler la scène à l'ouverture. Isolation : projet d'autrui → 404 (SC-008).
///
/// Lot 43 (audit 42 C1) : chaque modèle porte `referenced` (scène
/// enregistrée, tranchage en cours ou dernier G-code d'un plateau). Un import
/// interrompu (connexion perdue après le 201) reste non référencé et
/// **inconnu** de la scène enregistrée (`knownModels`) : l'éditeur le
/// restaure à l'ouverture (reprise, `planSceneRestore`). Un modèle que la
/// scène enregistrée **connaît** sans le référencer a été retiré par
/// l'utilisateur puis le projet enregistré : orphelin définitif, supprimé ici
/// (fichier source, maillage, ligne) pour libérer le quota — à l'ouverture
/// seulement, quand plus aucune session d'édition ne peut l'annuler.
pub async fn list_for_project(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(project_raw): Path<String>,
) -> ApiResult<Json<Vec<ModelResponse>>> {
    let project_id = parse_project_id(&project_raw)?;
    let project = state.storage.projects().get(user.id, project_id).await?; // 404 si autre compte
    let models = state
        .storage
        .models()
        .list(user.id, Some(project_id))
        .await?;
    let jobs = state.storage.jobs().list(user.id).await?;
    let referenced = referenced_model_ids(&project.scene, project_id, &jobs);
    let known = known_model_ids(&project.scene, project_id);
    let mut out = Vec::with_capacity(models.len());
    for model in models {
        let id = model.id.to_string();
        if known.contains(&id) && !referenced.contains(&id) {
            delete_model_files(&state, user.id, &model).await;
            continue;
        }
        let mut response: ModelResponse = model.into();
        response.referenced = Some(referenced.contains(&id));
        out.push(response);
    }
    Ok(Json(out))
}

/// Modèles que l'éditeur connaissait à l'enregistrement de la scène
/// (`knownModels = { projectId, ids }`, frontend scene/document.ts).
pub(crate) fn known_model_ids(
    scene: &serde_json::Value,
    project_id: ProjectId,
) -> std::collections::HashSet<String> {
    let Some(known) = scene.get("knownModels") else {
        return Default::default();
    };
    if known.get("projectId").and_then(serde_json::Value::as_str)
        != Some(project_id.to_string().as_str())
    {
        return Default::default();
    }
    known
        .get("ids")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .map(str::to_string)
        .collect()
}

/// Toute chaîne d'un document de scène (hors `knownModels`) : les modèles y
/// apparaissent par `modelId` (objets, volumes maillés…) ; un balayage
/// complet couvre aussi les références futures.
fn collect_strings(value: &serde_json::Value, out: &mut std::collections::HashSet<String>) {
    match value {
        serde_json::Value::String(s) => {
            out.insert(s.clone());
        }
        serde_json::Value::Array(items) => items.iter().for_each(|v| collect_strings(v, out)),
        serde_json::Value::Object(map) => {
            for (key, v) in map {
                if key != "knownModels" {
                    collect_strings(v, out);
                }
            }
        }
        _ => {}
    }
}

/// Identifiants référencés par la scène enregistrée et par les scènes figées
/// des tranchages du projet encore utiles (en file / en cours, et dernier
/// tranchage réussi de chaque plateau : export du 3MF tranché).
pub(crate) fn referenced_model_ids(
    scene: &serde_json::Value,
    project_id: ProjectId,
    jobs: &[crate::domain::SlicingJob],
) -> std::collections::HashSet<String> {
    use crate::domain::JobStatus;
    let mut out = std::collections::HashSet::new();
    collect_strings(scene, &mut out);
    let mut latest: std::collections::HashMap<i64, &crate::domain::SlicingJob> =
        std::collections::HashMap::new();
    for job in jobs.iter().filter(|j| j.project_id == project_id) {
        match job.status {
            JobStatus::Queued | JobStatus::Running => {
                if let Some(scene) = &job.scene {
                    collect_strings(scene, &mut out);
                }
            }
            JobStatus::Succeeded => {
                let newer = latest
                    .get(&job.plate_index)
                    .is_none_or(|prev| job.created_at > prev.created_at);
                if newer {
                    latest.insert(job.plate_index, job);
                }
            }
            _ => {}
        }
    }
    for job in latest.values() {
        if let Some(scene) = &job.scene {
            collect_strings(scene, &mut out);
        }
    }
    out
}

/// Supprime un modèle orphelin : ligne puis fichiers (source, maillage).
async fn delete_model_files(state: &AppState, user: UserId, model: &Model) {
    if let Err(error) = state.storage.models().delete(user, model.id).await {
        tracing::warn!(%error, model = %model.id, "modèle orphelin non supprimé");
        return;
    }
    let _ = tokio::fs::remove_file(&model.file_path).await;
    if let Some(path) = model.mesh_path.as_deref() {
        let _ = tokio::fs::remove_file(path).await;
    }
    tracing::info!(model = %model.id, "modèle orphelin supprimé (retiré de la scène enregistrée)");
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

/// Réglages StepMeshDialog (même forme que engine::adapters::ffi::StepMeshOptions,
/// transmis tels quels au worker).
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct StepMeshOptions {
    #[serde(default = "default_linear")]
    pub linear_deflection: f64,
    #[serde(default = "default_angle")]
    pub angle_deflection: f64,
    #[serde(default)]
    pub split_compound: bool,
}

fn default_linear() -> f64 {
    0.003
}

fn default_angle() -> f64 {
    0.5
}

/// `POST /api/models/{id}/step-mesh` — remaillage d'un STEP avec les réglages du
/// dialogue StepMeshDialog d'Orca : `{ linear_deflection (0,003), angle_deflection
/// (0,5), split_compound (false) }`. Le maillage d'affichage du modèle est remplacé
/// (événement model.converted) ; réponse `{ triangles, dialog_triangles, objects:
/// [{ name, triangles }] }` : une entrée par pièce (Step::mesh crée un objet ;
/// découpe des compounds = une pièce « <nom>-SOLID-<n> » par solide).
pub async fn step_mesh(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(options): Json<StepMeshOptions>,
) -> ApiResult<Json<serde_json::Value>> {
    let id = parse_model_id(&id)?;
    let model = state.storage.models().get(user.id, id).await?; // 404 si autre compte
    if model.format != ModelFormat::Step {
        return Err(ApiError::validation(
            "modèle STEP attendu",
            serde_json::json!({ "format": model.format }),
        ));
    }
    let valid = |v: f64, max: f64| v.is_finite() && v > 0.0 && v <= max;
    if !valid(options.linear_deflection, 1.0) || !valid(options.angle_deflection, 1.0) {
        return Err(ApiError::validation(
            "déflexions hors bornes (dialogue d'Orca : linéaire 0,001–0,1 mm, angulaire 0,01–1 rad)",
            serde_json::json!({}),
        ));
    }
    let dir = crate::workdir::tempdir().map_err(|_| ApiError::internal())?;
    let opts_path = dir.path().join("opts.json");
    let out = dir.path().join("step.json");
    std::fs::write(
        &opts_path,
        serde_json::to_vec(&options).map_err(|_| ApiError::internal())?,
    )
    .map_err(|_| ApiError::internal())?;
    crate::engine::run_worker(
        &[
            "load-step",
            &model.file_path,
            &opts_path.to_string_lossy(),
            &out.to_string_lossy(),
        ],
        std::time::Duration::from_secs(170),
    )
    .await
    .map_err(|e| ApiError::validation(format!("maillage STEP : {e}"), serde_json::json!({})))?;
    let view: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&out).map_err(|_| ApiError::internal())?)
            .map_err(|_| ApiError::internal())?;
    let meshes: Vec<(String, engine::api::TriangleMesh)> = view["objects"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|o| {
            Some((
                o["name"].as_str().unwrap_or_default().to_string(),
                serde_json::from_value(o["mesh"].clone()).ok()?,
            ))
        })
        .collect();
    let mut merged = engine::api::TriangleMesh::default();
    for (_, mesh) in &meshes {
        let offset = merged.vertices.len() as u32;
        merged.vertices.extend(mesh.vertices.iter().copied());
        merged.indices.extend(
            mesh.indices
                .iter()
                .map(|t| [t[0] + offset, t[1] + offset, t[2] + offset]),
        );
    }
    let triangles = merged.indices.len();
    let converter = state.converter.clone();
    converter.convert_mesh(model, merged).await;
    Ok(Json(serde_json::json!({
        "triangles": triangles,
        "dialog_triangles": view["dialog_triangles"],
        "objects": meshes.iter().map(|(n, m)| serde_json::json!({ "name": n, "triangles": m.indices.len() })).collect::<Vec<_>>(),
    })))
}

/// `POST /api/models/{id}/obj-colors` — couleurs d'un OBJ (sommets « v x y z r g b »
/// ou matériaux MTL) lues par libslic3r et regroupées comme Orca (ObjColorDialog,
/// réglage par défaut : un nouveau filament par couleur regroupée) :
/// `{ colored, palette: ["#RRGGBBAA"…] (filament n = palette[n-1]), extruder,
/// triangles, painting: { mmu: { "<triangle>": "<état TriangleSelector>" } } }`,
/// même forme que la peinture d'un 3MF importé. Corps optionnel : texte du MTL
/// (l'OBJ téléversé seul ne porte pas son .mtl), écrit sous le nom de son `mtllib`.
pub async fn obj_colors(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    mtl: axum::body::Bytes,
) -> ApiResult<Json<serde_json::Value>> {
    let id = parse_model_id(&id)?;
    let model = state.storage.models().get(user.id, id).await?; // 404 si autre compte
    if model.format != ModelFormat::Obj {
        return Err(ApiError::validation(
            "modèle OBJ attendu",
            serde_json::json!({ "format": model.format }),
        ));
    }
    let obj = state
        .files
        .read(FsPath::new(&model.file_path))
        .await
        .map_err(|_| ApiError::internal())?;
    let dir = crate::workdir::tempdir().map_err(|_| ApiError::internal())?;
    let obj_path = dir.path().join("model.obj");
    std::fs::write(&obj_path, &obj).map_err(|_| ApiError::internal())?;
    if !mtl.is_empty() {
        let text = String::from_utf8_lossy(&obj);
        let name = text
            .lines()
            .find_map(|l| l.trim().strip_prefix("mtllib "))
            .map(str::trim)
            .filter(|n| !n.is_empty() && !n.contains('/') && !n.contains('\\') && *n != "..")
            .unwrap_or("model.mtl");
        std::fs::write(dir.path().join(name), &mtl).map_err(|_| ApiError::internal())?;
    }
    let out = dir.path().join("colors.json");
    crate::engine::run_worker(
        &[
            "load-obj-colors",
            &obj_path.to_string_lossy(),
            &out.to_string_lossy(),
        ],
        std::time::Duration::from_secs(120),
    )
    .await
    .map_err(|e| ApiError::validation(format!("couleurs OBJ : {e}"), serde_json::json!({})))?;
    let view: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&out).map_err(|_| ApiError::internal())?)
            .map_err(|_| ApiError::internal())?;
    Ok(Json(view))
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
    fn conversion_flag_covers_all_formats() {
        // Source de vérité unique (T124) : tout format passe par le moteur.
        for format in [
            ModelFormat::Stl,
            ModelFormat::Obj,
            ModelFormat::ThreeMf,
            ModelFormat::Step,
            ModelFormat::Amf,
            ModelFormat::Svg,
            ModelFormat::Drc,
        ] {
            assert!(
                format.needs_engine_conversion(),
                "{format:?} doit passer par la conversion moteur"
            );
        }
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
