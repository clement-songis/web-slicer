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
    bbox_center, encode_binary_stl, primitive_side_for_bed, signed_volume, BooleanOp, CutConnector,
    CutGroove, CutOptions, CutPlane, MeshOp, PrimitiveShape,
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
///
/// Lot 26 — découpe complète d'Orca (GLGizmoCut3D → `Slic3r::Cut`), champs
/// facultatifs (absents = coupe plane historique) : `rotation_deg` (plan
/// incliné, prioritaire sur `plane.normal`), `connectors` (tenon / goujon /
/// clip), `groove` (queue d'aronde, exclusive des connecteurs),
/// `keep_as_parts`, `place_on_cut_upper/lower`, `flip_upper/lower`,
/// `instance_matrix` (repère monde centré sur l'objet, pièces sans rotation).
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
    /// « Garder la peinture » (keep_painting d'Orca, ModelObjectCutAttribute::
    /// KeepPaint) : peinture actuelle du modèle par canal (`supports`, `seam`,
    /// `mmu`, `fuzzy` → triangle → chaîne 3MF), reportée sur chaque pièce.
    #[serde(default)]
    #[ts(optional, type = "Record<string, Record<number, string>>")]
    pub keep_painting: Option<std::collections::BTreeMap<String, engine::api::paint::PaintFacets>>,
    /// Rotation du plan en degrés (X, Y, Z appliquées dans cet ordre, comme
    /// `Geometry::rotation_transform` d'Orca) : normale = R·Z ; l'axe X du plan
    /// oriente la queue d'aronde.
    #[serde(default)]
    #[ts(optional)]
    pub rotation_deg: Option<[f64; 3]>,
    /// Matrice monde 4×4 colonne-major de l'objet (rotation/échelle ; la
    /// translation est ignorée) : plan et connecteurs sont alors exprimés dans
    /// le repère monde centré sur la position de l'objet et la rotation est
    /// cuite dans les pièces (rotation nulle, échelle 1 à la pose), comme Orca.
    #[serde(default)]
    #[ts(optional)]
    pub instance_matrix: Option<Vec<f64>>,
    /// Connecteurs (Orca force alors les deux moitiés, sans « pièces »).
    #[serde(default)]
    #[ts(optional)]
    pub connectors: Option<Vec<CutConnector>>,
    /// Queue d'aronde (`CutMode::cutTongueAndGroove`).
    #[serde(default)]
    #[ts(optional)]
    pub groove: Option<CutGroove>,
    /// « Garder comme pièces » : un seul objet, deux volumes (_A, _B).
    #[serde(default)]
    #[ts(optional)]
    pub keep_as_parts: Option<bool>,
    #[serde(default)]
    #[ts(optional)]
    pub place_on_cut_upper: Option<bool>,
    #[serde(default)]
    #[ts(optional)]
    pub place_on_cut_lower: Option<bool>,
    #[serde(default)]
    #[ts(optional)]
    pub flip_upper: Option<bool>,
    #[serde(default)]
    #[ts(optional)]
    pub flip_lower: Option<bool>,
}

#[derive(Debug, Clone, Copy, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct CutPlaneDto {
    pub point: [f64; 3],
    /// Facultative si `rotation_deg` est fourni (sinon obligatoire).
    #[serde(default)]
    #[ts(optional)]
    pub normal: Option<[f64; 3]>,
}

impl CutRequest {
    /// Options moteur ; erreurs de forme → 422.
    fn options(&self) -> ApiResult<CutOptions> {
        let instance_matrix = match &self.instance_matrix {
            None => None,
            Some(values) => Some(<[f64; 16]>::try_from(values.as_slice()).map_err(|_| {
                ApiError::validation(
                    "instance_matrix : 16 valeurs attendues",
                    serde_json::json!({ "field": "instance_matrix" }),
                )
            })?),
        };
        Ok(CutOptions {
            rotation_deg: self.rotation_deg,
            instance_matrix,
            connectors: self.connectors.clone().unwrap_or_default(),
            groove: self.groove,
            keep_as_parts: self.keep_as_parts.unwrap_or(false),
            place_on_cut_upper: self.place_on_cut_upper.unwrap_or(false),
            place_on_cut_lower: self.place_on_cut_lower.unwrap_or(false),
            flip_upper: self.flip_upper.unwrap_or(false),
            flip_lower: self.flip_lower.unwrap_or(false),
        })
    }

    /// Normale effective : celle de la rotation si fournie (une normale
    /// explicite doit alors concorder, à 0,1° près), sinon `plane.normal`.
    fn normal(&self) -> ApiResult<[f64; 3]> {
        let invalid = |message: &str| {
            ApiError::validation(
                message.to_string(),
                serde_json::json!({ "field": "plane.normal" }),
            )
        };
        match (self.rotation_deg, self.plane.normal) {
            (None, None) => Err(invalid("plane.normal ou rotation_deg est requis")),
            (None, Some(normal)) => Ok(normal),
            (Some(rotation), given) => {
                let normal = CutOptions::rotation_normal(rotation);
                if let Some(given) = given {
                    let len = given.iter().map(|v| v * v).sum::<f64>().sqrt();
                    let dot = (0..3).map(|i| normal[i] * given[i]).sum::<f64>() / len;
                    if !(len > 1e-9 && dot >= 0.1f64.to_radians().cos()) {
                        return Err(invalid("plane.normal ne correspond pas à rotation_deg"));
                    }
                }
                Ok(normal)
            }
        }
    }
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
    /// Peinture reportée sur la pièce (coupe avec `keep_painting`).
    #[ts(optional, type = "Record<string, Record<number, string>>")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paint: Option<std::collections::BTreeMap<String, engine::api::paint::PaintFacets>>,
    /// Texte / SVG en relief (lot 26) : paramètres ré-éditables à recopier tels
    /// quels dans l'objet ou le volume de scène (`emboss`) ; écrits dans le 3MF
    /// comme `TextConfiguration` / `EmbossShape` d'Orca et relus à l'import.
    /// Texte : `{ type: "text", text, font, font_file, style_name, size_mm,
    /// depth_mm, boldness_mm? }` ; SVG : `{ type: "svg", svg, file_name,
    /// width_mm, depth_mm }`. Ré-éditer = rappeler `/emboss` (ou `/emboss-svg`)
    /// avec ces paramètres modifiés et remplacer le maillage.
    #[ts(optional, type = "Record<string, unknown>")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emboss: Option<serde_json::Value>,
    /// Découpe complète : volumes additionnels de l'objet (connecteurs
    /// négatifs, tenon en pièce, 2e pièce de « garder comme pièces »). À poser
    /// comme volumes maillés de la scène : `{ shape: "mesh", modelId, type,
    /// name, offset, rotation: [0,0,0] }`.
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volumes: Option<Vec<MeshOpVolumeResponse>>,
}

/// Volume additionnel d'un objet coupé (nouveau modèle du projet).
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct MeshOpVolumeResponse {
    pub model: ModelResponse,
    /// Nom Orca (`Connector-1`, `cube_B`…).
    pub name: String,
    /// `part` | `negative` | `modifier` | `blocker` | `enforcer` (scène).
    #[serde(rename = "type")]
    #[ts(rename = "type")]
    pub role: String,
    /// Volume issu d'un connecteur (`cut_info.is_connector`).
    pub connector: bool,
    /// Centre de la boîte du volume moins celui du corps (repère recentré de
    /// l'objet, = `offset` du volume de scène).
    pub offset: [f64; 3],
    pub triangles: u32,
    pub volume_mm3: f64,
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
    /// Nom du fichier SVG d'origine (Orca : `SvgFile::path`, seul le nom est
    /// écrit dans le 3MF ; le SVG est empaqueté sous `3D/<nom>.svg`).
    #[serde(default)]
    #[ts(optional)]
    pub file_name: Option<String>,
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
    let (status, Json(mut response)) = persist_parts(
        &state,
        user.id,
        project_id,
        &format!("Texte {stem}.stl"),
        parts,
    )
    .await?;
    let (label, file) = EMBOSS_FONTS
        .iter()
        .find(|(id, _, _)| *id == req.font)
        .map(|(_, label, file)| (*label, *file))
        .unwrap_or_default();
    let config = serde_json::json!({
        "type": "text",
        "text": req.text,
        "font": req.font,
        "font_file": file,
        "style_name": label,
        "size_mm": req.size_mm,
        "depth_mm": req.depth_mm,
        "boldness_mm": req.boldness_mm,
    });
    for part in &mut response.parts {
        part.emboss = Some(config.clone());
    }
    Ok((status, Json(response)))
}

/// Relief relu d'un 3MF : rattache l'identifiant de police web-slicer
/// (`font`) quand le fichier de police d'Orca (`font_file`) est connu.
pub(crate) fn emboss_with_font_id(mut emboss: serde_json::Value) -> serde_json::Value {
    let file = emboss
        .get("font_file")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    if let (Some(file), Some(object)) = (file, emboss.as_object_mut()) {
        if let Some((id, _, _)) = EMBOSS_FONTS.iter().find(|(_, _, f)| *f == file) {
            object.insert("font".into(), (*id).into());
        }
    }
    emboss
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
        svg: req.svg.clone(),
        width_mm: req.width_mm,
        depth_mm: req.depth_mm,
    };
    op.validate(&[])
        .map_err(|e| ApiError::validation(e.to_string(), serde_json::json!({})))?;
    let parts = run_mesh_op(&op, &[]).await.map_err(worker_api_error)?;
    let (status, Json(mut response)) =
        persist_parts(&state, user.id, project_id, "Forme SVG.stl", parts).await?;
    let file_name = req
        .file_name
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .unwrap_or("shape.svg")
        .to_string();
    let config = serde_json::json!({
        "type": "svg",
        "svg": req.svg,
        "file_name": file_name,
        "width_mm": req.width_mm,
        "depth_mm": req.depth_mm,
    });
    for part in &mut response.parts {
        part.emboss = Some(config.clone());
    }
    Ok((status, Json(response)))
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
            normal: req.normal()?,
        },
        keep_upper: req.keep_upper.unwrap_or(true),
        keep_lower: req.keep_lower.unwrap_or(true),
        options: req.options()?,
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
    if let Some(bad) = req
        .keep_painting
        .iter()
        .flat_map(|c| c.keys())
        .find(|k| !engine::api::paint::PAINT_CHANNEL_KEYS.contains(&k.as_str()))
    {
        return Err(ApiError::validation(
            "canal de peinture inconnu (supports, seam, mmu, fuzzy)",
            serde_json::json!({ "channel": bad }),
        ));
    }
    let mesh = centered(load_mesh(&state, &source).await?);
    let parts = run_mesh_op(&op, std::slice::from_ref(&mesh))
        .await
        .map_err(worker_api_error)?;
    // Orca (CutUtils.cpp:336) : peinture sauvegardée dans le repère de coupe
    // puis remappée sur chaque pièce ; les pièces sortent du worker dans le
    // repère du maillage centré.
    let mut paints = Vec::with_capacity(parts.len());
    if let Some(channels) = req
        .keep_painting
        .filter(|c| c.values().any(|f| !f.is_empty()))
    {
        for part in parts.iter().filter(|p| p.volume.is_none()) {
            paints.push(Some(
                super::scene::remap_paint_via_worker(
                    mesh.clone(),
                    part.mesh.clone(),
                    channels.clone(),
                )
                .await?,
            ));
        }
    }
    let (status, Json(mut response)) =
        persist_parts(&state, user.id, project_id, &source.filename, parts).await?;
    for (part, paint) in response.parts.iter_mut().zip(paints) {
        part.paint = paint;
    }
    Ok((status, Json(response)))
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
        return Err(if let Some(error) = model.conversion_error.as_deref() {
            ApiError::conversion_failed_for(model.id, error)
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

/// Fichier d'un volume additionnel : `<pièce>_<nom Orca>.stl` (caractères
/// hors [A-Za-z0-9-_] remplacés par « _ »).
fn volume_filename(source: &str, label: &str, name: &str) -> String {
    let base = part_filename(source, label);
    let stem = base.trim_end_matches(".stl");
    let clean: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .take(64)
        .collect();
    format!("{stem}_{clean}.stl")
}

/// Crée un modèle par pièce ; en cas d'échec, tout ce qui a été créé par
/// cet appel est supprimé (pas de modèle orphelin à moitié écrit). Les
/// volumes additionnels (découpe complète) suivent leur corps dans `parts` et
/// sont rattachés à sa réponse.
async fn persist_parts(
    state: &AppState,
    user: UserId,
    project: ProjectId,
    source_filename: &str,
    parts: Vec<MeshOpOutputPart>,
) -> ApiResult<(StatusCode, Json<MeshOpResponse>)> {
    let mut created: Vec<(ModelId, Vec<std::path::PathBuf>)> = Vec::new();
    let mut responses: Vec<MeshOpPartResponse> = Vec::with_capacity(parts.len());
    let mut bodies: Vec<TriangleMesh> = Vec::new();
    let mut models: Vec<String> = Vec::new();
    for part in parts {
        let result = persist_part(state, user, project, source_filename, &part, &mut created).await;
        let outcome = match (result, &part.volume) {
            (Ok(response), None) => {
                bodies.push(part.mesh.clone());
                models.push(response.model.id.clone());
                responses.push(response);
                Ok(())
            }
            (Ok(response), Some(meta)) => match (responses.last_mut(), bodies.last()) {
                (Some(owner), Some(body)) if owner.label == part.label => {
                    let (c, b) = (bbox_center(&part.mesh), bbox_center(body));
                    models.push(response.model.id.clone());
                    owner
                        .volumes
                        .get_or_insert_with(Vec::new)
                        .push(MeshOpVolumeResponse {
                            model: response.model,
                            name: meta.name.clone(),
                            role: meta.role.clone(),
                            connector: meta.connector,
                            offset: std::array::from_fn(|i| c[i] - b[i]),
                            triangles: response.triangles,
                            volume_mm3: response.volume_mm3,
                        });
                    Ok(())
                }
                _ => {
                    tracing::error!(label = %part.label, "volume de découpe sans corps");
                    Err(ApiError::internal())
                }
            },
            (Err(error), _) => Err(error),
        };
        if let Err(error) = outcome {
            for (id, paths) in created {
                let _ = state.storage.models().delete(user, id).await;
                for path in paths {
                    let _ = tokio::fs::remove_file(path).await;
                }
            }
            return Err(error);
        }
    }
    for id in &models {
        state
            .events
            .publish_model_converted(user, id, &format!("/api/models/{id}/mesh"));
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
                filename: match &part.volume {
                    Some(meta) => volume_filename(source_filename, &part.label, &meta.name),
                    None => part_filename(source_filename, &part.label),
                },
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
        paint: None,
        emboss: None,
        volumes: None,
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
        assert_eq!(
            volume_filename("bracket.stl", "upper", "Connector-1"),
            "bracket_A_Connector-1.stl"
        );
        assert_eq!(
            part_filename("bracket.stl", "dowel_1"),
            "bracket_dowel_1.stl"
        );
        assert_eq!(volume_filename("b.stl", "parts", "b é"), "b_parts_b__.stl");
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
