//! API de testabilité (`harness/CONTRACT.md` §1).
//!
//! Ces routes ne sont montées que lorsque `HARNESS_API=1` (voir `routes::mod`).
//! Le tranchage suit le même pipeline worker que l'UI : décodage libslic3r,
//! `DynamicPrintConfig`, puis `engine-worker slice`. Aucun moteur parallèle n'est
//! introduit ici.

use std::path::Path;
use std::time::Duration;

use axum::extract::{Multipart, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use engine::api::RawPreset;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use tower_sessions::Session;

use crate::domain::repo::NewPrinter;
use crate::domain::{NewProject, Preset, PresetKind, PresetOrigin, Role, UserId};
use crate::engine::{run_worker, WorkerError};
use crate::http::error::{ApiError, ApiResult};
use crate::http::extract::SESSION_USER_KEY;
use crate::http::state::AppState;
use crate::models::{MeshDecoder, WorkerMeshDecoder};

const SLICE_TIMEOUT: Duration = Duration::from_secs(600);

#[derive(Debug, Deserialize)]
pub(crate) struct BootstrapRequest {
    email: String,
    password: String,
    printer: String,
    filament: String,
    process: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct BootstrapResponse {
    project_id: String,
}

/// `POST /api/_harness/bootstrap` — compte, session, imprimante et projet prêts.
///
/// L'onboarding de l'UI est fondé sur la présence d'au moins une imprimante ;
/// la déclarer ici termine donc effectivement le wizard, sans drapeau parallèle.
pub(crate) async fn bootstrap(
    State(state): State<AppState>,
    session: Session,
    Json(body): Json<BootstrapRequest>,
) -> ApiResult<Json<BootstrapResponse>> {
    let user = match state.storage.users().find_by_email(&body.email).await? {
        Some(user) => user,
        None => {
            // Le harness doit fonctionner même si les inscriptions publiques sont
            // fermées : il utilise le service de création administrée, pas la
            // politique d'inscription de l'instance.
            let role = if state.storage.users().count().await? == 0 {
                Role::Admin
            } else {
                Role::User
            };
            crate::auth::create_managed_user(
                state.storage.as_ref(),
                &body.email,
                &body.password,
                role,
            )
            .await?
        }
    };

    let printer = system_preset(&state, user.id, PresetKind::Machine, &body.printer).await?;
    let filament = system_preset(&state, user.id, PresetKind::Filament, &body.filament).await?;
    let process = system_preset(&state, user.id, PresetKind::Process, &body.process).await?;

    // Ne duplique pas l'imprimante lorsque bootstrap retrouve un compte connu.
    let printers = state.storage.printers().list(user.id).await?;
    if !printers.iter().any(|p| p.machine_preset_id == printer.id) {
        state
            .storage
            .printers()
            .create(
                user.id,
                NewPrinter {
                    name: super::printers::split_machine_name(&printer.name).0,
                    moonraker_url: None,
                    api_key: None,
                    machine_preset_id: printer.id,
                },
            )
            .await?;
    }

    let project = state
        .storage
        .projects()
        .create(
            user.id,
            NewProject {
                // Le nom est unique par compte ; chaque appel crée bien un projet
                // vide indépendant, y compris lors d'un bootstrap répété.
                name: format!("Projet harness {}", uuid::Uuid::new_v4().simple()),
                scene: serde_json::json!({
                    "schema_version": 1,
                    "objects": [],
                    "plates": []
                }),
                active_presets: serde_json::json!({
                    "printer": printer.id.to_string(),
                    "filaments": [filament.id.to_string()],
                    "process": process.id.to_string(),
                }),
                thumbnail_path: None,
            },
        )
        .await?;

    session
        .insert(SESSION_USER_KEY, user.id.to_string())
        .await
        .map_err(|_| ApiError::internal())?;
    Ok(Json(BootstrapResponse {
        project_id: project.id.to_string(),
    }))
}

/// Retrouve un preset **système** instanciable par son nom Orca exact.
async fn system_preset(
    state: &AppState,
    user: UserId,
    kind: PresetKind,
    name: &str,
) -> ApiResult<Preset> {
    let presets = state.storage.presets().list_by_kind(kind, user).await?;
    presets
        .into_iter()
        .filter(|p| p.origin == PresetOrigin::System && p.instantiation && p.name == name)
        .min_by(|a, b| a.vendor.cmp(&b.vendor))
        .ok_or_else(|| {
            ApiError::validation(
                format!("Preset système « {name} » introuvable ({kind:?})"),
                serde_json::json!({ "name": name, "kind": kind }),
            )
        })
}

/// Erreur du slicing harness : le contrat expose `{ "error": "..." }`.
#[derive(Debug)]
pub(crate) struct HarnessError {
    status: StatusCode,
    message: String,
}

impl HarnessError {
    fn invalid(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            message: message.into(),
        }
    }

    fn internal(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: message.into(),
        }
    }
}

impl IntoResponse for HarnessError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(serde_json::json!({ "error": self.message })),
        )
            .into_response()
    }
}

type HarnessResult<T> = Result<T, HarnessError>;

struct SliceUpload {
    filename: String,
    bytes: Vec<u8>,
    config: Map<String, Value>,
}

/// `POST /api/_harness/slice` — modèle + config Orca plate → G-code brut.
pub(crate) async fn slice(
    State(state): State<AppState>,
    multipart: Multipart,
) -> HarnessResult<Response> {
    let upload = read_slice_upload(&state, multipart).await?;
    let format = super::models::detect_format(&upload.filename)
        .filter(|format| {
            matches!(
                format,
                crate::domain::ModelFormat::Stl
                    | crate::domain::ModelFormat::ThreeMf
                    | crate::domain::ModelFormat::Obj
                    | crate::domain::ModelFormat::Step
            )
        })
        .ok_or_else(|| HarnessError::invalid("format non supporté (STL/3MF/OBJ/STEP)"))?;
    super::models::validate_content(format, &upload.bytes).map_err(HarnessError::invalid)?;

    // Conversion identique aux presets de l'UI : le registre parse les chaînes
    // Orca et ignore les clés vendeur inconnues. Les défauts absents sont ensuite
    // appliqués par `DynamicPrintConfig::full_print_config()` dans slice_raw.
    // Plateau d'arrangement du CLI, lu sur la config brute : les groupes de
    // `extruder_printable_area` (séparés par `#` / éléments du tableau JSON)
    // ne survivent pas au registre, qui les aplatit en une liste de points.
    let arrange_bed = harness_build_volume(&upload.config);
    let raw = RawPreset {
        name: "harness-flat-config".into(),
        inherits: None,
        values: upload.config,
    };
    let config = engine::presets::resolve_preset_chain(&[raw])
        .map_err(|e| HarnessError::invalid(format!("config Orca invalide : {e}")))?;

    // Le répertoire vit jusqu'à la lecture du G-code. Le nom d'origine fournit
    // l'extension nécessaire au dispatch du lecteur libslic3r.
    let work = crate::workdir::tempdir()
        .map_err(|e| HarnessError::internal(format!("préparation du slicing : {e}")))?;
    let source = work.path().join(safe_filename(&upload.filename));
    std::fs::write(&source, &upload.bytes)
        .map_err(|e| HarnessError::invalid(format!("écriture du modèle : {e}")))?;

    // Modèle tel que lu par Orca (read_from_file) : maillage recentré en float
    // par center_geometry_after_creation + décalage du volume, SANS cuisson.
    // Cuire le décalage dans les sommets (WSMh) arrondit autrement en f32 et
    // décale des points d'1 µm sur les maillages non entiers (trou lamé :
    // ordre des chemins différent, params-review cb2).
    let mut model = match load_raw_model(&source, format, &work).await {
        Some(model) => model,
        None => {
            let decoder = WorkerMeshDecoder::new();
            let wsmh = decoder.decode(&source, format).await.map_err(|e| {
                HarnessError::invalid(format!("décodage du modèle par le moteur : {e}"))
            })?;
            let mesh = display_to_triangle_mesh(&wsmh)?;
            let mut model = engine::api::Model::default();
            model.add_object(upload.filename.clone(), mesh);
            model
        }
    };

    // Équivalent de `orca-slicer --arrange 1 --orient 0` : aucune orientation
    // automatique, mais le CLI arrange avec `allow_rotations=true` (défaut du
    // CLI). Le nester libslic3r pivote alors l'empreinte vers sa boîte
    // englobante minimale (ex. demi-pas pour un 64-gone). Le bridge désactive
    // l'alignement final (`do_final_align=false`) : l'instance revient à la
    // position de départ du nester, et le centrage de la boîte APRÈS rotation
    // est donc fait ici, une seule fois — sur le centre du plateau d'arrangement
    // du CLI (zone commune des `extruder_printable_area` si présente).
    // Avec des zones exclues (`bed_exclude_area`), le CLI laisse finalAlign
    // chercher la position libre la plus proche du centre : le nester tourne
    // alors en mode CLI exact et sa translation est conservée.
    let arranged = arrange_single_occurrence(&mut model, &arrange_bed).await?;
    if arranged && !arrange_bed.excluded.is_empty() {
        drop_single_occurrence_on_bed(&mut model);
    } else {
        center_single_occurrence(&mut model, &arrange_bed);
    }

    let request = engine::api::SliceRequest {
        model,
        config,
        plate_index: 0,
        work_dir: work.path().to_path_buf(),
        calib: None,
    };
    let request_path = work.path().join("harness-slice-request.json");
    std::fs::write(
        &request_path,
        serde_json::to_vec(&request).map_err(|e| {
            HarnessError::internal(format!("sérialisation de la requête moteur : {e}"))
        })?,
    )
    .map_err(|e| HarnessError::internal(format!("écriture de la requête moteur : {e}")))?;

    let request_arg = request_arg(&request_path)?;
    // `engine-worker slice` ne lit que le chemin de requête : l'arrangement et
    // l'orientation sont déjà portés par la matrice d'instance ci-dessus.
    let stdout = run_worker(&["slice", &request_arg], SLICE_TIMEOUT)
        .await
        .map_err(slice_worker_error)?;
    let result = parse_slice_result(&stdout)?;
    let gcode = std::fs::read(&result.gcode_path)
        .map_err(|e| HarnessError::internal(format!("lecture du G-code moteur : {e}")))?;

    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
        gcode,
    )
        .into_response())
}

async fn read_slice_upload(
    state: &AppState,
    mut multipart: Multipart,
) -> HarnessResult<SliceUpload> {
    let mut model: Option<(String, Vec<u8>)> = None;
    let mut config: Option<Map<String, Value>> = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| HarnessError::invalid(format!("multipart illisible : {e}")))?
    {
        match field.name() {
            Some("model") => {
                let filename = field
                    .file_name()
                    .map(str::to_string)
                    .ok_or_else(|| HarnessError::invalid("le champ model doit être un fichier"))?;
                let bytes = field
                    .bytes()
                    .await
                    .map_err(|e| HarnessError::invalid(format!("lecture du modèle : {e}")))?
                    .to_vec();
                model = Some((filename, bytes));
            }
            Some("config") => {
                let text = field
                    .text()
                    .await
                    .map_err(|e| HarnessError::invalid(format!("lecture de config : {e}")))?;
                let values = serde_json::from_str::<Map<String, Value>>(&text)
                    .map_err(|e| HarnessError::invalid(format!("config JSON invalide : {e}")))?;
                if let Some((key, _)) = values.iter().find(|(_, value)| {
                    !value.is_string()
                        && !value
                            .as_array()
                            .is_some_and(|items| items.iter().all(Value::is_string))
                }) {
                    return Err(HarnessError::invalid(format!(
                        "config Orca : « {key} » doit être une chaîne ou un tableau de chaînes"
                    )));
                }
                config = Some(values);
            }
            _ => {}
        }
    }
    let (filename, bytes) = model.ok_or_else(|| HarnessError::invalid("champ model absent"))?;
    let limit = state
        .storage
        .instance()
        .settings()
        .await
        .map_err(|e| HarnessError::internal(format!("lecture de la limite d'upload : {e}")))?
        .upload_limit_bytes;
    if bytes.len() as i64 > limit {
        return Err(HarnessError::invalid(format!(
            "fichier de {} octets au-delà de la limite ({limit})",
            bytes.len()
        )));
    }
    Ok(SliceUpload {
        filename,
        bytes,
        config: config.ok_or_else(|| HarnessError::invalid("champ config absent"))?,
    })
}

fn safe_filename(filename: &str) -> String {
    Path::new(filename)
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("model.stl")
        .to_string()
}

/// `engine-worker load-model-raw` : objet unique, une instance, volumes pièces
/// avec leur matrice (repli WSMh si la forme ne correspond pas).
async fn load_raw_model(
    source: &Path,
    format: crate::domain::ModelFormat,
    work: &tempfile::TempDir,
) -> Option<engine::api::Model> {
    let _ = work;
    let mut model = crate::engine::raw_model::load(source, format).await?;
    if model.objects.len() != 1
        || model.objects[0].volumes.is_empty()
        || model.objects[0]
            .volumes
            .iter()
            .any(|v| v.role != engine::api::VolumeRole::ModelPart)
    {
        return None;
    }
    let object = &mut model.objects[0];
    object.instances.truncate(1);
    if object.instances.is_empty() {
        object.instances.push(engine::api::ModelInstance {
            matrix: glam::DMat4::IDENTITY,
            plate_index: 0,
            printable: true,
        });
    }
    Some(model)
}

fn display_to_triangle_mesh(bytes: &[u8]) -> HarnessResult<engine::api::TriangleMesh> {
    let display = engine::api::DisplayMesh::decode(bytes)
        .map_err(|e| HarnessError::internal(format!("maillage moteur illisible : {e}")))?;
    Ok(engine::api::TriangleMesh {
        vertices: display
            .positions
            .chunks_exact(3)
            .map(|c| [c[0], c[1], c[2]])
            .collect(),
        indices: display
            .indices
            .chunks_exact(3)
            .map(|c| [c[0], c[1], c[2]])
            .collect(),
    })
}

/// Arrangement réel par le nester libslic3r (`engine-worker arrange`), avec
/// rotations autorisées comme le CLI Orca, sur le seul contour printable_area
/// et sans dégagement (`min_obj_distance = 0` côté CLI). Seule la partie
/// linéaire de la matrice retournée est utile ici : la translation est
/// recalculée par [`center_single_occurrence`].
///
/// Un refus du moteur (objet plus grand que le plateau…) laisse l'occurrence
/// non pivotée : la validation du tranchage reste juge, comme avant. Renvoie
/// `true` si les matrices du nester ont été appliquées.
async fn arrange_single_occurrence(
    model: &mut engine::api::Model,
    bed: &engine::api::BuildVolume,
) -> HarnessResult<bool> {
    let instance_ids: Vec<String> = model
        .objects
        .iter()
        .enumerate()
        .flat_map(|(o, object)| (0..object.instances.len()).map(move |i| format!("{o}:{i}")))
        .collect();
    if instance_ids.is_empty() {
        return Ok(false);
    }
    let request = crate::engine::scene::ArrangeWorkerRequest {
        model: model.clone(),
        bed: bed.clone(),
        params: harness_arrange_params(!bed.excluded.is_empty()),
        instance_ids,
    };
    let response = match crate::engine::scene::arrange(&request).await {
        Ok(response) => response,
        Err(error) if error.is_invalid_input() => {
            tracing::warn!(%error, "harness : arrangement refusé, occurrence non pivotée");
            return Ok(false);
        }
        Err(error) => {
            return Err(HarnessError::internal(format!(
                "échec de l'arrangement moteur : {error}"
            )))
        }
    };
    apply_arranged_matrices(model, &response)?;
    Ok(true)
}

/// Paramètres d'arrangement du CLI Orca pour une occurrence unique.
///
/// `orca_cli_mode` (alignement final CENTER + exclusions gonflées de 1 mm)
/// n'est activé qu'en présence de zones exclues : sans obstacle, la
/// translation finale est recalculée par [`center_single_occurrence`] et le
/// nester garde la configuration validée par les références existantes.
fn harness_arrange_params(with_exclusions: bool) -> engine::api::ArrangeParams {
    engine::api::ArrangeParams {
        clearance: 0.0,
        use_machine_clearance: false,
        allow_rotations: true,
        orca_cli_mode: with_exclusions,
        print_config: None,
        is_seq_print: false,
    }
}

/// Plateau d'arrangement du CLI Orca (OrcaSlicer.cpp, branche `need_arrange`),
/// lu sur la config Orca brute :
/// - contour = `get_bed_shape(config, use_share = true)` : zone commune des
///   `extruder_printable_area` (`get_shared_poly`) si la clé est fournie,
///   sinon `printable_area` (défaut Orca 200 × 200 mm) ;
/// - zones exclues = `PartPlate::m_exclude_bounding_box` : boîte englobante
///   de chaque groupe de 4 points de `bed_exclude_area` (`0x0` → aucune).
fn harness_build_volume(raw: &Map<String, Value>) -> engine::api::BuildVolume {
    let mut bed_shape = raw
        .get("printable_area")
        .map(orca_points)
        .unwrap_or_default();
    if bed_shape.len() < 3 {
        bed_shape = vec![[0.0, 0.0], [200.0, 0.0], [200.0, 200.0], [0.0, 200.0]];
    }
    if let Some(shared) = raw
        .get("extruder_printable_area")
        .and_then(|value| shared_extruder_area(&orca_point_groups(value)))
    {
        bed_shape = shared;
    }
    let max_height = raw
        .get("printable_height")
        .and_then(orca_scalar)
        .and_then(|text| text.trim().parse::<f64>().ok())
        .filter(|h| h.is_finite() && *h > 0.0)
        .unwrap_or(256.0);
    let excluded = raw
        .get("bed_exclude_area")
        .map(|value| exclude_bounding_boxes(&orca_points(value)))
        .unwrap_or_default();
    engine::api::BuildVolume {
        bed_shape,
        max_height,
        excluded,
    }
}

fn orca_scalar(value: &Value) -> Option<&str> {
    match value {
        Value::String(text) => Some(text),
        Value::Array(items) => items.first().and_then(Value::as_str),
        _ => None,
    }
}

/// `"AxB"` → point fini ; tout autre texte est ignoré.
fn orca_point(text: &str) -> Option<[f64; 2]> {
    let (x, y) = text.trim().split_once('x')?;
    let point = [x.trim().parse::<f64>().ok()?, y.trim().parse::<f64>().ok()?];
    point.iter().all(|c| c.is_finite()).then_some(point)
}

/// `ConfigOptionPoints` : tableau JSON de `"AxB"` ou chaîne `"AxB,CxD"`.
fn orca_points(value: &Value) -> Vec<[f64; 2]> {
    let texts: Vec<&str> = match value {
        Value::String(text) => text.split(',').collect(),
        Value::Array(items) => items
            .iter()
            .filter_map(Value::as_str)
            .flat_map(|text| text.split(','))
            .collect(),
        _ => Vec::new(),
    };
    texts.into_iter().filter_map(orca_point).collect()
}

/// `ConfigOptionPointsGroups` : un groupe par élément du tableau JSON, ou
/// groupes séparés par `#` dans la forme sérialisée.
fn orca_point_groups(value: &Value) -> Vec<Vec<[f64; 2]>> {
    let groups: Vec<&str> = match value {
        Value::String(text) => text.split('#').collect(),
        Value::Array(items) => items
            .iter()
            .filter_map(Value::as_str)
            .flat_map(|text| text.split('#'))
            .collect(),
        _ => Vec::new(),
    };
    groups
        .into_iter()
        .map(|group| group.split(',').filter_map(orca_point).collect::<Vec<_>>())
        .filter(|group| !group.is_empty())
        .collect()
}

/// `get_shared_poly` : intersection des zones des extrudeurs. Les zones Orca
/// sont des rectangles alignés : l'intersection est celle de leurs boîtes
/// (un seul groupe → le contour tel quel).
fn shared_extruder_area(groups: &[Vec<[f64; 2]>]) -> Option<Vec<[f64; 2]>> {
    match groups {
        [] => None,
        [single] => (single.len() >= 3).then(|| single.clone()),
        _ => {
            let (mut min, mut max) = ([f64::MIN; 2], [f64::MAX; 2]);
            for group in groups {
                let (gmin, gmax) = bounding_box_2d(group)?;
                min = [min[0].max(gmin[0]), min[1].max(gmin[1])];
                max = [max[0].min(gmax[0]), max[1].min(gmax[1])];
            }
            (min[0] < max[0] && min[1] < max[1])
                .then(|| vec![min, [max[0], min[1]], max, [min[0], max[1]]])
        }
    }
}

fn bounding_box_2d(points: &[[f64; 2]]) -> Option<([f64; 2], [f64; 2])> {
    let first = *points.first()?;
    Some(points.iter().fold((first, first), |(min, max), p| {
        (
            [min[0].min(p[0]), min[1].min(p[1])],
            [max[0].max(p[0]), max[1].max(p[1])],
        )
    }))
}

/// `PartPlate::calc_bounding_boxes` : une boîte par groupe complet de 4
/// points (les points restants sont ignorés) ; une boîte d'aire nulle
/// n'exclut rien.
fn exclude_bounding_boxes(points: &[[f64; 2]]) -> Vec<Vec<[f64; 2]>> {
    points
        .chunks_exact(4)
        .filter_map(bounding_box_2d)
        .filter(|(min, max)| min[0] < max[0] && min[1] < max[1])
        .map(|(min, max)| vec![min, [max[0], min[1]], max, [min[0], max[1]]])
        .collect()
}

/// Réapplique les matrices du worker dans l'ordre objets puis instances.
fn apply_arranged_matrices(
    model: &mut engine::api::Model,
    response: &crate::engine::scene::ArrangeWorkerResponse,
) -> HarnessResult<()> {
    let mut placements = response.placements.iter();
    for (o, object) in model.objects.iter_mut().enumerate() {
        for (i, instance) in object.instances.iter_mut().enumerate() {
            let placement = placements
                .next()
                .filter(|p| p.id == format!("{o}:{i}"))
                .filter(|p| p.matrix.iter().all(|v| v.is_finite()))
                .ok_or_else(|| HarnessError::internal("réponse d'arrangement incohérente"))?;
            instance.matrix = glam::DMat4::from_cols_array(&placement.matrix);
        }
    }
    if placements.next().is_some() {
        return Err(HarnessError::internal("réponse d'arrangement incohérente"));
    }
    Ok(())
}

/// Pose sur le plateau (z = 0) l'occurrence placée par le nester, sans toucher
/// à sa position XY (`ModelInstance::apply_arrange_result` puis
/// `ensure_on_bed` du CLI).
fn drop_single_occurrence_on_bed(model: &mut engine::api::Model) {
    let Some(object) = model.objects.first_mut() else {
        return;
    };
    let volumes = object.volumes.clone();
    for instance in &mut object.instances {
        if let Some((min, _)) = object_bounding_box(&volumes, &instance.matrix) {
            instance.matrix.w_axis.z = -min.z;
        }
    }
}

/// Centre l'unique occurrence produite par `load-model` sur le contour du lit.
/// La boîte est celle du maillage transformé par la partie linéaire de la
/// matrice (rotation de l'arrangeur incluse) : seul le décalage est remplacé.
fn center_single_occurrence(model: &mut engine::api::Model, bed: &engine::api::BuildVolume) {
    let Some(object) = model.objects.first_mut() else {
        return;
    };
    let volumes = object.volumes.clone();
    let (bed_center_x, bed_center_y) = bed_center(bed);
    for instance in &mut object.instances {
        let Some((min, max)) = object_bounding_box(&volumes, &instance.matrix) else {
            continue;
        };
        // L'arrangeur Orca travaille en coordonnées entières de 1e-6 mm.
        // Aligner le centre évite qu'une dimension f32 comme 1.2 produise une
        // translation juste sous la frontière de conversion (ex. thinwall).
        instance.matrix.w_axis.x = snap_to_orca_grid(bed_center_x - (min.x + max.x) / 2.0);
        instance.matrix.w_axis.y = snap_to_orca_grid(bed_center_y - (min.y + max.y) / 2.0);
        instance.matrix.w_axis.z = -min.z;
    }
}

/// Boîte des volumes (matrice du volume puis partie linéaire de l'instance),
/// comme `ModelObject::instance_bounding_box` sans le décalage d'instance.
fn object_bounding_box(
    volumes: &[engine::api::ModelVolume],
    instance: &glam::DMat4,
) -> Option<(glam::DVec3, glam::DVec3)> {
    let linear = glam::DMat4::from_mat3(glam::DMat3::from_mat4(*instance));
    volumes
        .iter()
        .filter_map(|v| transformed_bounding_box(&v.mesh, &(linear * v.matrix)))
        .reduce(|(a0, a1), (b0, b1)| (a0.min(b0), a1.max(b1)))
}

/// Boîte englobante du maillage sous `matrix` (translation comprise).
fn transformed_bounding_box(
    mesh: &engine::api::TriangleMesh,
    matrix: &glam::DMat4,
) -> Option<(glam::DVec3, glam::DVec3)> {
    let mut points = mesh
        .vertices
        .iter()
        .map(|v| matrix.transform_point3(glam::DVec3::new(v[0] as f64, v[1] as f64, v[2] as f64)));
    let first = points.next()?;
    Some(points.fold((first, first), |(min, max), p| (min.min(p), max.max(p))))
}

fn snap_to_orca_grid(value: f64) -> f64 {
    const SCALING_FACTOR: f64 = 1e-6;
    (value / SCALING_FACTOR).round() * SCALING_FACTOR
}

/// Centre du rectangle englobant le plateau d'arrangement (centre du bac du
/// nester, cible de l'alignement final CENTER du CLI).
fn bed_center(bed: &engine::api::BuildVolume) -> (f64, f64) {
    match bounding_box_2d(&bed.bed_shape) {
        Some((min, max)) => ((min[0] + max[0]) / 2.0, (min[1] + max[1]) / 2.0),
        None => (100.0, 100.0),
    }
}

fn request_arg(path: &Path) -> HarnessResult<String> {
    path.to_str()
        .map(str::to_string)
        .ok_or_else(|| HarnessError::internal("chemin de requête non-UTF8"))
}

fn parse_slice_result(stdout: &[u8]) -> HarnessResult<engine::api::SliceResult> {
    let text = String::from_utf8_lossy(stdout);
    if let Some(error) = worker_protocol_error(&text) {
        return Err(HarnessError::invalid(error));
    }
    let payload = text
        .lines()
        .rev()
        .find_map(|line| line.strip_prefix("R "))
        .ok_or_else(|| HarnessError::internal("sortie moteur sans résultat « R »"))?;
    serde_json::from_str(payload)
        .map_err(|e| HarnessError::internal(format!("résultat moteur illisible : {e}")))
}

fn slice_worker_error(error: WorkerError) -> HarnessError {
    // Sur sortie non nulle, engine-worker émet `E <code> <message>` sur stdout.
    // `Print::validate` emprunte ce chemin : le contrat le traduit en 422.
    if let WorkerError::Failed { stdout, .. } = &error {
        if let Some(message) = worker_protocol_error(&String::from_utf8_lossy(stdout)) {
            return HarnessError::invalid(message);
        }
    }
    HarnessError::internal(format!("échec du moteur de slicing : {error}"))
}

fn worker_protocol_error(text: &str) -> Option<String> {
    text.lines().rev().find_map(|line| {
        line.strip_prefix("E ")
            .map(|rest| rest.split_once(' ').map_or(rest, |(_, message)| message))
            .map(str::trim)
            .filter(|message| !message.is_empty())
            .map(str::to_string)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square_model() -> engine::api::Model {
        let mut model = engine::api::Model::default();
        model.add_object(
            "square",
            engine::api::TriangleMesh {
                vertices: vec![[0.0, 0.0, 2.0], [20.0, 0.0, 2.0], [20.0, 10.0, 7.0]],
                indices: vec![[0, 1, 2]],
            },
        );
        model
    }

    #[test]
    fn harness_arrange_params_match_the_orca_cli() {
        let params = harness_arrange_params(false);
        assert!(params.allow_rotations);
        assert_eq!(params.clearance, 0.0);
        assert!(!params.orca_cli_mode);
        assert!(harness_arrange_params(true).orca_cli_mode);
    }

    #[test]
    fn centering_uses_the_bounding_box_after_the_arranged_rotation() {
        let mut model = square_model();
        let response = crate::engine::scene::ArrangeWorkerResponse {
            placements: vec![crate::engine::scene::ArrangedInstance {
                print_order: None,
                id: "0:0".into(),
                plate_index: 0,
                matrix: (glam::DMat4::from_translation(glam::DVec3::new(3.0, 4.0, 5.0))
                    * glam::DMat4::from_rotation_z(std::f64::consts::FRAC_PI_2))
                .to_cols_array(),
            }],
        };
        apply_arranged_matrices(&mut model, &response).unwrap();
        center_single_occurrence(&mut model, &harness_build_volume(&Map::new()));
        let matrix = model.objects[0].instances[0].matrix;
        // Rz(90°) : x' = -y ∈ [-10, 0], y' = x ∈ [0, 20] → centre (-5, 10).
        assert!((matrix.x_axis.y - 1.0).abs() < 1e-12);
        assert!((matrix.w_axis.x - 105.0).abs() < 1e-9);
        assert!((matrix.w_axis.y - 90.0).abs() < 1e-9);
        assert!((matrix.w_axis.z + 2.0).abs() < 1e-12);
    }

    #[test]
    fn arranged_matrices_are_rejected_when_ids_do_not_match() {
        let mut model = square_model();
        let response = crate::engine::scene::ArrangeWorkerResponse {
            placements: vec![crate::engine::scene::ArrangedInstance {
                print_order: None,
                id: "1:0".into(),
                plate_index: 0,
                matrix: glam::DMat4::IDENTITY.to_cols_array(),
            }],
        };
        assert!(apply_arranged_matrices(&mut model, &response).is_err());
    }

    #[test]
    fn build_volume_uses_printable_area_without_exclusions() {
        let config = serde_json::json!({
            "printable_area": ["0x0", "300x0", "300x250", "0x250"],
            "bed_exclude_area": ["0x0"]
        });
        let bed = harness_build_volume(config.as_object().unwrap());
        assert_eq!(bed.bed_shape[2], [300.0, 250.0]);
        assert!(bed.excluded.is_empty());
        assert_eq!(harness_build_volume(&Map::new()).bed_shape.len(), 4);
    }

    #[test]
    fn build_volume_follows_the_orca_cli_arrange_bed() {
        // Valeurs de l'audit params-review (refs param/bed_exclude_area et
        // param/extruder_printable_area) sur le plateau Voron 350 × 350.
        let config = serde_json::json!({
            "printable_area": ["0x0", "350x0", "350x350", "0x350"],
            "bed_exclude_area": ["150x150", "200x150", "200x200", "150x200"],
            "extruder_printable_area": ["0x0,100x0,100x100,0x100"]
        });
        let bed = harness_build_volume(config.as_object().unwrap());
        // get_bed_shape(use_share = true) : zone commune des extrudeurs.
        assert_eq!(
            bed.bed_shape,
            vec![[0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [0.0, 100.0]]
        );
        assert_eq!(bed_center(&bed), (50.0, 50.0));
        // PartPlate::m_exclude_bounding_box : une boîte par groupe de 4 points.
        assert_eq!(
            bed.excluded,
            vec![vec![
                [150.0, 150.0],
                [200.0, 150.0],
                [200.0, 200.0],
                [150.0, 200.0]
            ]]
        );

        // Plusieurs extrudeurs (`#` ou éléments du tableau) : intersection.
        let shared = shared_extruder_area(&orca_point_groups(&serde_json::json!(
            "0x0,300x0,300x300,0x300#50x0,350x0,350x300,50x300"
        )))
        .unwrap();
        assert_eq!(
            shared,
            vec![[50.0, 0.0], [300.0, 0.0], [300.0, 300.0], [50.0, 300.0]]
        );
    }

    #[test]
    fn center_translation_uses_the_orca_integer_grid() {
        assert_eq!(snap_to_orca_grid(174.39999997615814), 174.4);
        assert_eq!(snap_to_orca_grid(155.0), 155.0);
    }
}
