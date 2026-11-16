//! Découpe de maillage côté API (`splitobjects` / `splitvolumes`, outil Cut,
//! outil Mesh Boolean) — exécutée par `engine-worker mesh-op`, jamais par FFI
//! dans ce process.
//!
//! Chaque pièce produite devient un **nouveau modèle** du projet : STL binaire
//! (vraies normales) comme fichier source, WSMh du worker comme maillage
//! d'affichage (immédiatement servi par `/api/models/{id}/mesh`). Le modèle
//! source et la scène ne sont pas modifiés : le client remplace l'objet.
//!
//! Repères (le client recentre tout maillage sur le centre de sa boîte) :
//! - split / cut : le plan et les `center` renvoyés sont dans le repère
//!   **recentré** du modèle source (celui de `activeObject.mesh`). Placement
//!   d'une pièce : `position_source + R·S·center`, rotation/échelle copiées ;
//! - boolean : chaque opérande est transformé par sa matrice monde 4×4
//!   colonne-major (`Matrix4.elements` de three.js, appliquée au maillage
//!   recentré) ; le résultat est en repère monde, `center` = position monde du
//!   nouvel objet (rotation nulle, échelle 1).

use std::path::Path as FsPath;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use engine::api::split::{
    bbox_center, encode_binary_stl, primitive_side_for_bed, signed_volume, BooleanOp, CutPlane,
    MeshOp, PrimitiveShape,
};
use engine::api::TriangleMesh;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::domain::{Model, ModelFormat, ModelId, NewModel, ProjectId, UserId};
use crate::engine::mesh_ops::{run_mesh_op, MeshOpOutputPart};
use crate::engine::scene::SceneWorkerError;
use crate::http::dto::ModelResponse;
use crate::http::error::{ApiError, ApiResult};
use crate::http::extract::CurrentUser;
use crate::http::state::AppState;
use crate::mesh::Mesh;
use crate::scene::analyze_mesh;

/// Corps de `POST /api/models/{id}/cut` : plan dans le repère recentré du
/// modèle (normale quelconque ; `[0,0,1]` = coupe horizontale), moitiés à
/// conserver (défaut : les deux). `upper` = côté de la normale.
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct CutRequest {
    pub plane: CutPlaneDto,
    #[serde(default)]
    #[ts(optional)]
    pub keep_upper: Option<bool>,
    #[serde(default)]
    #[ts(optional)]
    pub keep_lower: Option<bool>,
}

#[derive(Debug, Clone, Copy, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct CutPlaneDto {
    pub point: [f64; 3],
    pub normal: [f64; 3],
}

/// Opérande booléen : modèle du projet + matrice monde colonne-major.
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct BooleanOperand {
    pub model_id: String,
    pub matrix: Vec<f64>,
}

/// Corps de `POST /api/projects/{id}/boolean` : `a` = source, `b` = outil.
/// `op` : `union` | `difference` (A − B) | `intersection`.
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct BooleanRequest {
    pub op: BooleanOpDto,
    pub a: BooleanOperand,
    pub b: BooleanOperand,
}

#[derive(Debug, Clone, Copy, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum BooleanOpDto {
    Union,
    Difference,
    Intersection,
}

impl From<BooleanOpDto> for BooleanOp {
    fn from(op: BooleanOpDto) -> Self {
        match op {
            BooleanOpDto::Union => BooleanOp::Union,
            BooleanOpDto::Difference => BooleanOp::Difference,
            BooleanOpDto::Intersection => BooleanOp::Intersection,
        }
    }
}

/// Pièce créée : nouveau modèle (maillage déjà prêt) + placement.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct MeshOpPartResponse {
    pub model: ModelResponse,
    /// `part_<n>` (split), `upper` / `lower` (cut), `result` (booléen).
    pub label: String,
    /// Centre de la boîte de la pièce (repère : voir l'en-tête du module).
    pub center: [f64; 3],
    pub triangles: u32,
    pub volume_mm3: f64,
    pub watertight: bool,
}

/// Réponse commune de split / cut / boolean (201).
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct MeshOpResponse {
    pub parts: Vec<MeshOpPartResponse>,
}

/// Corps de `POST /api/projects/{id}/emboss` : texte en relief (outil Emboss
/// d'Orca) — police parmi `GET /api/emboss/fonts`, taille de police et
/// profondeur d'extrusion en mm.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
pub struct EmbossRequest {
    pub text: String,
    pub font: String,
    pub size_mm: f32,
    pub depth_mm: f32,
    /// Graisse en mm (`FontProp::boldness`, réglage « Boldness » d'Orca).
    #[serde(default)]
    #[ts(optional)]
    pub boldness_mm: Option<f32>,
}

/// Corps de `POST /api/projects/{id}/emboss-svg` : forme SVG en relief (outil
/// SVG d'Orca), mise à l'échelle à `width_mm` et extrudée de `depth_mm`.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
pub struct EmbossSvgRequest {
    pub svg: String,
    pub width_mm: f32,
    pub depth_mm: f32,
}

/// Corps de `POST /api/projects/{id}/primitive` : forme du menu « Add »
/// d'Orca et dimensions XY du plateau imprimable (mm). Le côté est calculé ici
/// par la règle de la GUI d'Orca (0,1 × plus grande dimension), la géométrie
/// par `its_make_*` de libslic3r dans engine-worker.
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct PrimitiveRequest {
    pub shape: PrimitiveShapeDto,
    pub bed_size_mm: [f64; 2],
}

#[derive(Debug, Clone, Copy, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum PrimitiveShapeDto {
    Cube,
    Cylinder,
    Sphere,
    Cone,
    Disc,
    Torus,
}

impl From<PrimitiveShapeDto> for PrimitiveShape {
    fn from(s: PrimitiveShapeDto) -> Self {
        match s {
            PrimitiveShapeDto::Cube => PrimitiveShape::Cube,
            PrimitiveShapeDto::Cylinder => PrimitiveShape::Cylinder,
            PrimitiveShapeDto::Sphere => PrimitiveShape::Sphere,
            PrimitiveShapeDto::Cone => PrimitiveShape::Cone,
            PrimitiveShapeDto::Disc => PrimitiveShape::Disc,
            PrimitiveShapeDto::Torus => PrimitiveShape::Torus,
        }
    }
}

/// Police disponible pour l'outil Emboss.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct EmbossFont {
    pub id: String,
    pub label: String,
}

/// Polices libres (SIL OFL, cf. OFL.txt) livrées avec OrcaSlicer
/// (`resources/fonts`) : identifiant stable → fichier.
const EMBOSS_FONTS: [(&str, &str, &str); 5] = [
    ("noto-sans", "Noto Sans", "NotoSansKR-Regular.ttf"),
    ("noto-sans-bold", "Noto Sans Bold", "NotoSansKR-Bold.ttf"),
    ("nanum-gothic", "Nanum Gothic", "NanumGothic-Regular.ttf"),
    (
        "nanum-gothic-bold",
        "Nanum Gothic Bold",
        "NanumGothic-Bold.ttf",
    ),
    ("sarabun", "Sarabun", "Sarabun-Medium.ttf"),
];

/// Répertoires de polices candidats : `WEBSLICER_FONTS_DIR`, puis les
/// ressources d'Orca (`$ORCA_SRC`, `vendor/OrcaSlicer`).
fn font_dirs() -> Vec<std::path::PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(d) = std::env::var("WEBSLICER_FONTS_DIR") {
        dirs.push(d.into());
    }
    if let Ok(src) = std::env::var("ORCA_SRC") {
        dirs.push(FsPath::new(&src).join("resources/fonts"));
    }
    for rel in [
        "vendor/OrcaSlicer/resources/fonts",
        "../vendor/OrcaSlicer/resources/fonts",
    ] {
        dirs.push(rel.into());
    }
    dirs
}

fn font_path(id: &str) -> Option<std::path::PathBuf> {
    let (_, _, file) = EMBOSS_FONTS.iter().find(|(fid, _, _)| *fid == id)?;
    font_dirs()
        .into_iter()
        .map(|d| d.join(file))
        .find(|p| p.is_file())
}

/// `GET /api/emboss/fonts` — polices réellement présentes sur le serveur.
pub async fn emboss_fonts(CurrentUser(_user): CurrentUser) -> Json<Vec<EmbossFont>> {
    Json(
        EMBOSS_FONTS
            .iter()
            .filter(|(id, _, _)| font_path(id).is_some())
            .map(|(id, label, _)| EmbossFont {
                id: (*id).into(),
                label: (*label).into(),
            })
            .collect(),
    )
}

/// `POST /api/projects/{id}/emboss` — crée un modèle « texte en relief »
/// (Emboss::text2shapes + polygons2model dans engine-worker).
pub async fn emboss(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(project_raw): Path<String>,
    Json(req): Json<EmbossRequest>,
) -> ApiResult<(StatusCode, Json<MeshOpResponse>)> {
    let project_id = parse_project_id(&project_raw)?;
    state.storage.projects().get(user.id, project_id).await?;
    let font = font_path(&req.font).ok_or_else(|| {
        ApiError::validation(
            format!("police inconnue ou absente du serveur : {}", req.font),
            serde_json::json!({ "font": req.font }),
        )
    })?;
    let op = MeshOp::EmbossText {
        font_path: font,
        text: req.text.clone(),
        size_mm: req.size_mm,
        depth_mm: req.depth_mm,
        boldness_mm: req.boldness_mm.unwrap_or(0.0),
    };
    op.validate(&[])
        .map_err(|e| ApiError::validation(e.to_string(), serde_json::json!({})))?;
    let parts = run_mesh_op(&op, &[]).await.map_err(worker_api_error)?;
    let stem: String = req
        .text
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .take(32)
        .collect();
    persist_parts(
        &state,
        user.id,
        project_id,
        &format!("Texte {stem}.stl"),
        parts,
    )
    .await
}

/// `POST /api/projects/{id}/emboss-svg` — modèle « forme SVG » en relief
/// (nanosvg → ExPolygons → polygons2model dans engine-worker).
pub async fn emboss_svg(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(project_raw): Path<String>,
    Json(req): Json<EmbossSvgRequest>,
) -> ApiResult<(StatusCode, Json<MeshOpResponse>)> {
    let project_id = parse_project_id(&project_raw)?;
    state.storage.projects().get(user.id, project_id).await?;
    let op = MeshOp::EmbossSvg {
        svg: req.svg,
        width_mm: req.width_mm,
        depth_mm: req.depth_mm,
    };
    op.validate(&[])
        .map_err(|e| ApiError::validation(e.to_string(), serde_json::json!({})))?;
    let parts = run_mesh_op(&op, &[]).await.map_err(worker_api_error)?;
    persist_parts(&state, user.id, project_id, "Forme SVG.stl", parts).await
}

/// `POST /api/projects/{id}/primitive` — primitive d'Orca (`create_mesh`,
/// `its_make_*` de libslic3r) : nouveau modèle du projet, maillage prêt.
pub async fn primitive(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(project_raw): Path<String>,
    Json(req): Json<PrimitiveRequest>,
) -> ApiResult<(StatusCode, Json<MeshOpResponse>)> {
    let project_id = parse_project_id(&project_raw)?;
    state.storage.projects().get(user.id, project_id).await?;
    let shape = PrimitiveShape::from(req.shape);
    let [bx, by] = req.bed_size_mm;
    let op = MeshOp::Primitive {
        shape,
        side_mm: primitive_side_for_bed(bx, by),
    };
    op.validate(&[]).map_err(|e| {
        ApiError::validation(
            e.to_string(),
            serde_json::json!({ "bed_size_mm": req.bed_size_mm }),
        )
    })?;
    let parts = run_mesh_op(&op, &[]).await.map_err(worker_api_error)?;
    persist_parts(
        &state,
        user.id,
        project_id,
        &format!("{}.stl", shape.label()),
        parts,
    )
    .await
}

/// Corps de `POST /api/models/{id}/layer-profile` : outil « Variable layer
/// height » d'Orca. `op` = `{kind:"adaptive", quality}` ou
/// `{kind:"smooth", profile, radius, keep_min}` ; `values` = réglages résolus
/// (clé → valeur) ; `matrix` = matrice d'instance colonne-major (rotation /
/// échelle : la hauteur de l'objet en dépend), identité par défaut.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LayerProfileHttpRequest {
    pub op: engine::api::layer_profile::LayerProfileOp,
    #[serde(default)]
    pub values: serde_json::Value,
    #[serde(default)]
    pub matrix: Option<Vec<f64>>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct LayerProfileHttpResponse {
    /// Profil plat Orca `[z0, h0, z1, h1, …]` (repère objet posé, z = 0 en bas).
    pub profile: Vec<f64>,
}

/// `POST /api/models/{id}/layer-profile` — `layer_height_profile_adaptive` /
/// `smooth_height_profile` de libslic3r (Slicing.cpp) dans engine-worker.
pub async fn layer_profile(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<LayerProfileHttpRequest>,
) -> ApiResult<Json<LayerProfileHttpResponse>> {
    let source = state
        .storage
        .models()
        .get(user.id, parse_model_id(&id)?)
        .await?;
    let mesh = centered(load_mesh(&state, &source).await?);
    let matrix = match req.matrix.as_deref() {
        None => glam::DMat4::IDENTITY,
        Some(m) if m.len() == 16 && m.iter().all(|v| v.is_finite()) => {
            let mut cols = [0.0; 16];
            cols.copy_from_slice(m);
            // Translation ignorée : le profil est relatif au bas de l'objet.
            let mut mat = glam::DMat4::from_cols_array(&cols);
            mat.w_axis = glam::DVec4::new(0.0, 0.0, 0.0, 1.0);
            mat
        }
        Some(_) => {
            return Err(ApiError::validation(
                "matrice d'instance invalide (16 valeurs finies)",
                serde_json::json!({}),
            ))
        }
    };
    let mut model = engine::api::Model::default();
    model.add_object(source.filename.clone(), mesh).instances[0].matrix = matrix;
    let request = engine::api::layer_profile::LayerProfileRequest {
        model,
        config: crate::slicing::config_from_values_lenient(&req.values),
        op: req.op,
    };
    let profile = crate::engine::layer_profile::run_layer_profile(&request)
        .await
        .map_err(worker_api_error)?;
    Ok(Json(LayerProfileHttpResponse { profile }))
}

fn parse_model_id(raw: &str) -> ApiResult<ModelId> {
    uuid::Uuid::parse_str(raw)
        .map(ModelId)
        .map_err(|_| ApiError::not_found("Modèle"))
}

fn parse_project_id(raw: &str) -> ApiResult<ProjectId> {
    uuid::Uuid::parse_str(raw)
        .map(ProjectId)
        .map_err(|_| ApiError::not_found("Projet"))
}

/// `POST /api/models/{id}/split` — `TriangleMesh::split` (Orca
/// « Split to objects » / « Split to parts ») : une pièce par composante
/// connexe. Objet à une seule pièce → 422.
pub async fn split(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<(StatusCode, Json<MeshOpResponse>)> {
    let source = state
        .storage
        .models()
        .get(user.id, parse_model_id(&id)?)
        .await?;
    let project_id = source_project(&source)?;
    let mesh = centered(load_mesh(&state, &source).await?);
    let parts = run_mesh_op(&MeshOp::Split, &[mesh])
        .await
        .map_err(worker_api_error)?;
    persist_parts(&state, user.id, project_id, &source.filename, parts).await
}

/// `POST /api/models/{id}/smooth` — subdivision de Loop
/// (`TriangleMeshDeal::smooth_triangle_mesh`, « Subdivision mesh (Lost
/// color) » d'Orca) : un nouveau modèle `result` (×4 triangles), `center`
/// dans le repère recentré de la source. Maillage ouvert → 422 (« réparez-le
/// d'abord », comme `Plater::can_smooth_mesh`).
pub async fn smooth(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<(StatusCode, Json<MeshOpResponse>)> {
    let source = state
        .storage
        .models()
        .get(user.id, parse_model_id(&id)?)
        .await?;
    let project_id = source_project(&source)?;
    let mesh = centered(load_mesh(&state, &source).await?);
    let parts = run_mesh_op(&MeshOp::Smooth, &[mesh])
        .await
        .map_err(worker_api_error)?;
    persist_parts(&state, user.id, project_id, &source.filename, parts).await
}

/// `POST /api/models/{id}/cut` — coupe plane (`cut_mesh`, sections
/// bouchées). Plan invalide, aucune moitié gardée ou plan ne traversant pas
/// l'objet → 422.
pub async fn cut(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<CutRequest>,
) -> ApiResult<(StatusCode, Json<MeshOpResponse>)> {
    let op = MeshOp::Cut {
        plane: CutPlane {
            point: req.plane.point,
            normal: req.plane.normal,
        },
        keep_upper: req.keep_upper.unwrap_or(true),
        keep_lower: req.keep_lower.unwrap_or(true),
    };
    let source = state
        .storage
        .models()
        .get(user.id, parse_model_id(&id)?)
        .await?;
    let project_id = source_project(&source)?;
    // Paramètres refusés avant toute lecture (422 même pendant la conversion).
    op.validate(&[TriangleMesh {
        vertices: vec![[0.0; 3]],
        indices: vec![[0, 0, 0]],
    }])
    .map_err(|e| ApiError::validation(e.to_string(), serde_json::json!({})))?;
    let mesh = centered(load_mesh(&state, &source).await?);
    let parts = run_mesh_op(&op, &[mesh]).await.map_err(worker_api_error)?;
    persist_parts(&state, user.id, project_id, &source.filename, parts).await
}

/// `POST /api/projects/{id}/boolean` — booléen mcut (outil Mesh Boolean)
/// entre deux modèles du projet placés par leurs matrices monde.
pub async fn boolean(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(project_raw): Path<String>,
    Json(req): Json<BooleanRequest>,
) -> ApiResult<(StatusCode, Json<MeshOpResponse>)> {
    let project_id = parse_project_id(&project_raw)?;
    state.storage.projects().get(user.id, project_id).await?;
    let matrix_a = parse_matrix(&req.a.matrix, "a")?;
    let matrix_b = parse_matrix(&req.b.matrix, "b")?;
    let model_a = project_model(&state, user.id, project_id, &req.a.model_id).await?;
    let model_b = project_model(&state, user.id, project_id, &req.b.model_id).await?;
    let mesh_a = transformed(centered(load_mesh(&state, &model_a).await?), &matrix_a);
    let mesh_b = transformed(centered(load_mesh(&state, &model_b).await?), &matrix_b);
    let op = MeshOp::Boolean { op: req.op.into() };
    let parts = run_mesh_op(&op, &[mesh_a, mesh_b])
        .await
        .map_err(worker_api_error)?;
    persist_parts(&state, user.id, project_id, &model_a.filename, parts).await
}

fn source_project(model: &Model) -> ApiResult<ProjectId> {
    model.project_id.ok_or_else(|| {
        ApiError::validation(
            "modèle hors projet : découpe impossible",
            serde_json::json!({ "model_id": model.id.to_string() }),
        )
    })
}

async fn project_model(
    state: &AppState,
    user: UserId,
    project: ProjectId,
    raw: &str,
) -> ApiResult<Model> {
    let model = state
        .storage
        .models()
        .get(user, parse_model_id(raw)?)
        .await?;
    if model.project_id != Some(project) {
        return Err(ApiError::not_found("Modèle"));
    }
    Ok(model)
}

fn parse_matrix(values: &[f64], operand: &str) -> ApiResult<glam::DMat4> {
    let invalid = |reason: &str| {
        ApiError::validation(
            format!("matrice de l'opérande {operand} invalide : {reason}"),
            serde_json::json!({ "operand": operand }),
        )
    };
    let array: [f64; 16] = values
        .try_into()
        .map_err(|_| invalid("16 valeurs attendues"))?;
    if array.iter().any(|v| !v.is_finite()) {
        return Err(invalid("valeur non finie"));
    }
    let matrix = glam::DMat4::from_cols_array(&array);
    if matrix.determinant().abs() < 1e-12 {
        return Err(invalid("matrice dégénérée"));
    }
    Ok(matrix)
}

/// Lit le WSMh converti du modèle (409 pendant la conversion, 422 en échec).
pub(crate) async fn load_mesh(state: &AppState, model: &Model) -> ApiResult<TriangleMesh> {
    let Some(path) = model.mesh_path.as_deref() else {
        return Err(if model.conversion_error.is_some() {
            ApiError::conversion_failed("échec de la conversion du modèle par le moteur")
        } else {
            ApiError::conflict("conversion en cours (voir l'événement model.converted)")
        });
    };
    let bytes = state.files.read(FsPath::new(path)).await.map_err(|e| {
        tracing::error!(error = %e, "lecture du maillage converti");
        ApiError::internal()
    })?;
    let display = engine::api::DisplayMesh::decode(&bytes).map_err(|e| {
        tracing::error!(error = %e, "décodage du maillage converti");
        ApiError::internal()
    })?;
    let mesh = TriangleMesh {
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
    };
    if mesh
        .indices
        .iter()
        .flatten()
        .any(|&index| index as usize >= mesh.vertices.len())
    {
        return Err(ApiError::validation(
            "maillage converti invalide",
            serde_json::json!({ "model_id": model.id.to_string() }),
        ));
    }
    Ok(mesh)
}

/// Repère du client : maillage recentré sur le centre de sa boîte.
pub(crate) fn centered(mut mesh: TriangleMesh) -> TriangleMesh {
    let center = bbox_center(&mesh);
    for vertex in &mut mesh.vertices {
        for axis in 0..3 {
            vertex[axis] = (f64::from(vertex[axis]) - center[axis]) as f32;
        }
    }
    mesh
}

/// Applique une matrice monde ; un miroir (déterminant < 0) inverse
/// l'ordre des sommets pour garder des normales sortantes.
fn transformed(mut mesh: TriangleMesh, matrix: &glam::DMat4) -> TriangleMesh {
    for vertex in &mut mesh.vertices {
        let p = matrix.transform_point3(glam::DVec3::new(
            f64::from(vertex[0]),
            f64::from(vertex[1]),
            f64::from(vertex[2]),
        ));
        *vertex = [p.x as f32, p.y as f32, p.z as f32];
    }
    if matrix.determinant() < 0.0 {
        for tri in &mut mesh.indices {
            tri.swap(1, 2);
        }
    }
    mesh
}

fn worker_api_error(error: SceneWorkerError) -> ApiError {
    if error.is_invalid_input() {
        return ApiError::validation(error.to_string(), serde_json::json!({}));
    }
    tracing::error!(error = %error, "découpe de maillage dans engine-worker");
    ApiError::internal()
}

/// Suffixe de nom de fichier, à la manière d'Orca (`_A`/`_B` pour la coupe).
fn part_suffix(label: &str) -> String {
    match label {
        "upper" => "_A".into(),
        "lower" => "_B".into(),
        "result" => "_boolean".into(),
        "text" | "svg" => String::new(),
        // Primitives (`PrimitiveShape::label`) : le nom de fichier est déjà la forme.
        "Cube" | "Cylinder" | "Sphere" | "Cone" | "Disc" | "Torus" => String::new(),
        other => format!("_{}", other.trim_start_matches("part_")),
    }
}

fn part_filename(source: &str, label: &str) -> String {
    let stem = FsPath::new(source)
        .file_stem()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("model");
    format!("{stem}{}.stl", part_suffix(label))
}

/// Crée un modèle par pièce ; en cas d'échec, tout ce qui a été créé par
/// cet appel est supprimé (pas de modèle orphelin à moitié écrit).
async fn persist_parts(
    state: &AppState,
    user: UserId,
    project: ProjectId,
    source_filename: &str,
    parts: Vec<MeshOpOutputPart>,
) -> ApiResult<(StatusCode, Json<MeshOpResponse>)> {
    let mut created: Vec<(ModelId, Vec<std::path::PathBuf>)> = Vec::new();
    let mut responses = Vec::with_capacity(parts.len());
    for part in parts {
        match persist_part(state, user, project, source_filename, &part, &mut created).await {
            Ok(response) => responses.push(response),
            Err(error) => {
                for (id, paths) in created {
                    let _ = state.storage.models().delete(user, id).await;
                    for path in paths {
                        let _ = tokio::fs::remove_file(path).await;
                    }
                }
                return Err(error);
            }
        }
    }
    for response in &responses {
        state.events.publish_model_converted(
            user,
            &response.model.id,
            &format!("/api/models/{}/mesh", response.model.id),
        );
    }
    Ok((
        StatusCode::CREATED,
        Json(MeshOpResponse { parts: responses }),
    ))
}

async fn persist_part(
    state: &AppState,
    user: UserId,
    project: ProjectId,
    source_filename: &str,
    part: &MeshOpOutputPart,
    created: &mut Vec<(ModelId, Vec<std::path::PathBuf>)>,
) -> ApiResult<MeshOpPartResponse> {
    let internal = |what: &'static str| {
        move |e: crate::adapters::files::FilesError| {
            tracing::error!(error = %e, "{what}");
            ApiError::internal()
        }
    };
    let stl = encode_binary_stl(&part.mesh);
    let file_path = state
        .files
        .write_model(user, ModelId::new(), "stl", &stl)
        .await
        .map_err(internal("écriture du STL de la pièce"))?;
    let triangles = part.mesh.triangle_count() as u32;
    let mut model = match state
        .storage
        .models()
        .create(
            user,
            NewModel {
                project_id: Some(project),
                filename: part_filename(source_filename, &part.label),
                format: ModelFormat::Stl,
                file_path: file_path.to_string_lossy().into_owned(),
                mesh_path: None,
                size_bytes: stl.len() as i64,
                triangle_count: i64::from(triangles),
                repair_report: None,
            },
        )
        .await
    {
        Ok(model) => model,
        Err(error) => {
            let _ = tokio::fs::remove_file(&file_path).await;
            return Err(error.into());
        }
    };
    created.push((model.id, vec![file_path]));
    let mesh_path = state
        .files
        .write_mesh(user, model.id, &part.display_bytes)
        .await
        .map_err(internal("écriture du maillage de la pièce"))?;
    if let Some((_, paths)) = created.last_mut() {
        paths.push(mesh_path.clone());
    }
    let mesh_path = mesh_path.to_string_lossy().into_owned();
    state
        .storage
        .models()
        .set_mesh(user, model.id, &mesh_path, i64::from(triangles))
        .await?;
    model.mesh_path = Some(mesh_path);

    let analysis = analyze_mesh(&Mesh {
        positions: part.mesh.vertices.iter().flatten().copied().collect(),
        normals: Vec::new(),
        indices: part.mesh.indices.iter().flatten().copied().collect(),
    });
    Ok(MeshOpPartResponse {
        model: model.into(),
        label: part.label.clone(),
        center: bbox_center(&part.mesh),
        triangles,
        volume_mm3: signed_volume(&part.mesh),
        watertight: analysis.watertight(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn part_names_follow_orca_suffixes() {
        assert_eq!(part_filename("bracket.stl", "upper"), "bracket_A.stl");
        assert_eq!(part_filename("bracket.stl", "lower"), "bracket_B.stl");
        assert_eq!(part_filename("two.3mf", "part_2"), "two_2.stl");
        assert_eq!(part_filename("", "result"), "model_boolean.stl");
    }

    #[test]
    fn matrix_validation_and_mirror_winding() {
        assert!(parse_matrix(&[0.0; 15], "a").is_err());
        assert!(parse_matrix(&[0.0; 16], "a").is_err());
        let mut mirror = glam::DMat4::IDENTITY.to_cols_array();
        mirror[0] = -1.0;
        let m = parse_matrix(&mirror, "a").unwrap();
        let mesh = TriangleMesh {
            vertices: vec![[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            indices: vec![[0, 1, 2]],
        };
        let out = transformed(mesh, &m);
        assert_eq!(out.indices[0], [0, 2, 1]);
        assert_eq!(out.vertices[1], [-1.0, 0.0, 0.0]);
    }
}
