//! Validation avant tranchage (`POST /api/projects/{id}/validate`), équivalent
//! de `Print::validate` d'Orca appelé par la GUI à chaque modification
//! (Plater::priv::update_background_process → BackgroundSlicingProcess::validate).
//!
//! Mêmes entrées que `POST …/slice` (plateau, brouillon, températures mixtes),
//! même config figée par plateau et même scène que le job : le moteur
//! (`engine-worker validate`) prépare le `Print` comme pour trancher
//! (`DynamicPrintConfig::validate`, `Print::apply`, `Print::validate(&warning)`,
//! cohérence multi-filament) puis s'arrête avant `Print::process`. Rien n'est
//! mis en file, aucun G-code n'est produit.
//!
//! Réponse par plateau : erreurs bloquantes et avertissements, rattachés à
//! l'objet de scène en cause quand Orca le désigne (collisions « par objet »,
//! hauteur d'extrudeur, objet trop haut, réglages d'objet…), et à l'option
//! (`opt_key`) en cause.

use std::time::Duration;

use axum::extract::{Path, State};
use axum::Json;
use engine::api::{ConfigValue, DynamicPrintConfig, ValidateIssueKind, ValidateResult};
use serde::Serialize;
use serde_json::{json, Value};
use ts_rs::TS;

use super::slice::{
    off_bed_ids, plate_scene_ids, plate_targets, request_active_presets, request_scene,
    resolve_slice_config, with_plate_settings, SliceConfig,
};
use crate::domain::{ProjectId, UserId};
use crate::http::dto::{SliceRequest, SliceWarning};
use crate::http::error::{ApiError, ApiResult};
use crate::http::extract::CurrentUser;
use crate::http::state::AppState;
use crate::slicing::{
    build_scene_model_indexed, calib_from_scene, config_from_snapshot, custom_gcode_mode,
    scene_custom_gcodes, SceneObjectRef,
};

/// Délai maximal d'une validation (apply + validate, sans tranchage).
const VALIDATE_TIMEOUT: Duration = Duration::from_secs(120);

/// Réponse de `POST /api/projects/{id}/validate`.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct ValidateResponse {
    /// Aucun plateau ciblé n'a d'erreur bloquante : le tranchage peut être lancé.
    pub valid: bool,
    /// Un résultat par plateau ciblé (`plate_index` ou tous les plateaux non vides).
    pub plates: Vec<PlateValidation>,
    /// Avertissements de configuration, identiques à ceux de `POST …/slice`
    /// (bornes clampées, post-traitement, compatibilité des filaments).
    pub warnings: Vec<SliceWarning>,
}

/// Validation d'un plateau.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct PlateValidation {
    /// Index 0-based du plateau.
    pub plate_index: i64,
    /// Aucune erreur bloquante sur ce plateau.
    pub valid: bool,
    /// Erreurs bloquantes (Orca refuse de trancher).
    pub errors: Vec<ValidationIssue>,
    /// Avertissements (le tranchage reste possible).
    pub warnings: Vec<ValidationIssue>,
}

/// Une erreur ou un avertissement de validation.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct ValidationIssue {
    /// Origine : `StringExceptionType` d'Orca, contrôle de config / de
    /// cohérence, ou garde de scène du serveur.
    #[ts(
        type = "\"general\" | \"filament_not_match_bed_type\" | \"filaments_different_temp\" | \"object_collision_in_seq_print\" | \"object_collision_in_layer_print\" | \"layer_height_exceeds_limit\" | \"config\" | \"consistency\" | \"empty_plate\" | \"object_outside_plate\""
    )]
    pub kind: String,
    /// Message d'Orca (anglais, msgid à traduire côté client) ou du serveur.
    pub message: String,
    /// Option Orca en cause (ex. `enable_prime_tower`, `layer_height`).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub opt_key: Option<String>,
    /// Objet de scène en cause (`scene.objects[].id`).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub object_id: Option<String>,
    /// Nom affiché de cet objet.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub object_name: Option<String>,
    /// Paramètres du message d'Orca (`StringObjectException::params`).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[ts(optional, as = "Option<Vec<String>>")]
    pub params: Vec<String>,
}

impl ValidationIssue {
    fn new(kind: &str, message: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            message: message.into(),
            opt_key: None,
            object_id: None,
            object_name: None,
            params: Vec::new(),
        }
    }
}

/// `POST /api/projects/{id}/validate` — erreurs et avertissements d'Orca avant
/// tranchage, sans trancher (même corps que `POST …/slice`, scène non
/// enregistrée comprise).
pub async fn validate(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(project_raw): Path<String>,
    Json(body): Json<SliceRequest>,
) -> ApiResult<Json<ValidateResponse>> {
    let project_id = uuid::Uuid::parse_str(&project_raw)
        .map(ProjectId)
        .map_err(|_| ApiError::not_found("Projet"))?;
    let project = state.storage.projects().get(user.id, project_id).await?; // 404, SC-008
    let scene = request_scene(&body, &project.scene)?;

    // Projet sans aucun objet : le plateau 0 est rapporté vide.
    let targets = match plate_targets(scene, body.plate_index)? {
        targets if targets.is_empty() => vec![0],
        targets => targets,
    };
    let config = resolve_slice_config(
        &state,
        user.id,
        scene,
        request_active_presets(&body, &project.active_presets)?,
        &body,
    )
    .await?;

    let checks = targets
        .iter()
        .map(|&t| validate_plate(&state, user.id, project_id, scene, &config, t));
    let plates = futures_util::future::try_join_all(checks).await?;
    Ok(Json(ValidateResponse {
        valid: plates.iter().all(|p| p.valid),
        plates,
        warnings: config.warnings,
    }))
}

/// Valide un plateau : gardes de scène du serveur (plateau vide, objet hors
/// plateau), puis `engine-worker validate` sur le modèle et la config figés
/// exactement comme le job de tranchage de ce plateau.
async fn validate_plate(
    state: &AppState,
    user: UserId,
    project_id: ProjectId,
    scene: &Value,
    config: &SliceConfig,
    t: i64,
) -> ApiResult<PlateValidation> {
    let ids = plate_scene_ids(scene, t);
    if ids.is_empty() {
        return Ok(plate_result(
            t,
            vec![ValidationIssue::new(
                "empty_plate",
                "plateau vide (aucun objet à trancher)",
            )],
            Vec::new(),
        ));
    }
    let off_bed = off_bed_ids(scene);
    let outside: Vec<ValidationIssue> = ids
        .iter()
        .filter(|id| off_bed.contains(*id))
        .map(|id| ValidationIssue {
            object_id: Some(id.clone()),
            object_name: scene_object_name(scene, id),
            ..ValidationIssue::new(
                "object_outside_plate",
                "objet hors plateau : repositionner avant de trancher",
            )
        })
        .collect();
    if !outside.is_empty() {
        return Ok(plate_result(t, outside, Vec::new()));
    }

    // Config du job de ce plateau : snapshot + réglages du plateau, relue par
    // le registre comme le runner (`config_from_snapshot`).
    let snapshot = with_plate_settings(&config.resolved, scene, t);
    let plate_config = config_from_snapshot(&snapshot).map_err(|e| {
        tracing::error!(error = %e, "config de validation");
        ApiError::internal()
    })?;
    let (mut model, refs) =
        build_scene_model_indexed(state, user, project_id, scene, Some(t.max(0) as u32))
            .await
            .map_err(|e| {
                // Modèle supprimé / conversion indisponible : refus explicite.
                ApiError::validation(
                    format!("scène non validable : {e}"),
                    json!({ "plate_index": t }),
                )
            })?;
    if model.is_empty() {
        return Ok(plate_result(
            t,
            vec![ValidationIssue::new(
                "empty_plate",
                "aucun objet à trancher (modèles absents)",
            )],
            Vec::new(),
        ));
    }
    let mode = custom_gcode_mode(&plate_config);
    model.custom_gcodes = scene_custom_gcodes(scene)
        .into_iter()
        .map(|mut plate| {
            plate.mode = mode;
            plate
        })
        .collect();

    let work = crate::workdir::tempdir().map_err(|e| {
        tracing::error!(error = %e, "répertoire de validation");
        ApiError::internal()
    })?;
    let request = engine::api::SliceRequest {
        allow_mixed_temperatures: config.allow_mixed,
        model,
        config: plate_config.clone(),
        plate_index: t.max(0) as u32,
        work_dir: work.path().to_path_buf(),
        calib: calib_from_scene(scene),
    };
    let request_path = work.path().join("validate-request.json");
    let bytes = serde_json::to_vec(&request).map_err(|e| {
        tracing::error!(error = %e, "sérialisation de la requête de validation");
        ApiError::internal()
    })?;
    std::fs::write(&request_path, bytes).map_err(|e| {
        tracing::error!(error = %e, "écriture de la requête de validation");
        ApiError::internal()
    })?;
    let path_arg = request_path.to_string_lossy().to_string();
    let result = match crate::engine::run_worker(&["validate", &path_arg], VALIDATE_TIMEOUT).await {
        Ok(stdout) => parse_validate_result(&stdout).map_err(|e| {
            tracing::error!(error = %e, "résultat de validation illisible");
            ApiError::internal()
        })?,
        Err(error) => match worker_refusal(&error) {
            // Refus structuré du moteur avant Print::validate (ex. paramètres de
            // calibration ou G-code de couche invalides) : erreur du plateau.
            Some(message) => ValidateResult {
                issues: vec![engine::api::ValidateIssue {
                    message,
                    kind: ValidateIssueKind::Config,
                    is_warning: false,
                    opt_key: None,
                    object_index: None,
                    params: Vec::new(),
                }],
            },
            None => {
                tracing::error!(error = %error, "engine-worker validate");
                return Err(ApiError::internal());
            }
        },
    };

    let (mut errors, mut warnings) = (Vec::new(), Vec::new());
    for issue in result.issues {
        let warning = issue.is_warning;
        let issue = to_issue(issue, &refs, &plate_config, t);
        if warning {
            warnings.push(issue);
        } else {
            errors.push(issue);
        }
    }
    Ok(plate_result(t, errors, warnings))
}

fn plate_result(
    t: i64,
    errors: Vec<ValidationIssue>,
    warnings: Vec<ValidationIssue>,
) -> PlateValidation {
    PlateValidation {
        plate_index: t,
        valid: errors.is_empty(),
        errors,
        warnings,
    }
}

/// Ligne `R {"issues":[…]}` du worker (tolère le bruit de log libslic3r).
fn parse_validate_result(stdout: &[u8]) -> anyhow::Result<ValidateResult> {
    let text = String::from_utf8_lossy(stdout);
    let payload = text
        .lines()
        .rev()
        .find_map(|l| l.strip_prefix("R "))
        .ok_or_else(|| anyhow::anyhow!("sortie moteur sans résultat « R »"))?;
    Ok(serde_json::from_str(payload)?)
}

/// Message d'un refus structuré `E invalid_config|invalid_model <message>`.
fn worker_refusal(error: &crate::engine::WorkerError) -> Option<String> {
    let crate::engine::WorkerError::Failed { stdout, .. } = error else {
        return None;
    };
    let text = String::from_utf8_lossy(stdout);
    let line = text.lines().find_map(|line| line.strip_prefix("E "))?;
    let (code, message) = line.split_once(' ').unwrap_or((line, ""));
    matches!(code, "invalid_config" | "invalid_model").then(|| message.to_string())
}

/// Entrée moteur → entrée API : objet de scène, nom, et message du type
/// « plateau / filament » réécrit comme la GUI d'Orca.
fn to_issue(
    issue: engine::api::ValidateIssue,
    refs: &[SceneObjectRef],
    config: &DynamicPrintConfig,
    t: i64,
) -> ValidationIssue {
    let kind = serde_json::to_value(issue.kind)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_else(|| "general".into());
    let object = issue.object_index.and_then(|i| refs.get(i));
    let message = if issue.kind == ValidateIssueKind::FilamentNotMatchBedType {
        bed_type_message(&issue.params, config, t).unwrap_or(issue.message)
    } else {
        issue.message
    };
    ValidationIssue {
        kind,
        message,
        opt_key: issue.opt_key,
        object_id: object.and_then(|o| o.id.clone()),
        object_name: object.map(|o| o.name.clone()),
        params: issue.params,
    }
}

/// `Plater::post_process_string_object_exception` (Plater.cpp:18290) :
/// message « Plate %d: %s is not suggested to be used to print filament %s
/// (%s)… » avec le nom du preset filament (suffixe « @imprimante » retiré).
/// Le numéro de plateau est celui du plateau validé (le worker n'en a qu'un).
fn bed_type_message(params: &[String], config: &DynamicPrintConfig, t: i64) -> Option<String> {
    let bed_type = params.get(1)?;
    let filament = params.get(2)?;
    let extruder_id = filament.parse::<usize>().ok()?.checked_sub(1)?;
    let names = match config.get("filament_settings_id") {
        Some(ConfigValue::Strings(v)) => v.clone(),
        _ => Vec::new(),
    };
    let mut name = names.get(extruder_id)?.clone();
    if let Some(pos) = name.find('@') {
        // `substr(0, pos - 1)` d'Orca : l'espace qui précède « @ » est retiré.
        if pos > 0 {
            name.truncate(pos - 1);
        }
    }
    Some(format!(
        "Plate {}: {} is not suggested to be used to print filament {} ({}). If you still want to do this print job, please set this filament's bed temperature to non-zero.",
        t + 1,
        bed_type,
        filament,
        name
    ))
}

/// Nom affiché d'un objet de scène (`objects[].name`).
fn scene_object_name(scene: &Value, id: &str) -> Option<String> {
    scene
        .get("objects")?
        .as_array()?
        .iter()
        .find(|o| o.get("id").and_then(Value::as_str) == Some(id))?
        .get("name")?
        .as_str()
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_worker_result_after_log_noise() {
        let stdout = b"[info] bruit\nR {\"issues\":[{\"message\":\"m\",\"kind\":\"object_collision_in_seq_print\",\"is_warning\":false,\"object_index\":1}]}\n";
        let result = parse_validate_result(stdout).unwrap();
        assert_eq!(result.issues.len(), 1);
        assert_eq!(
            result.issues[0].kind,
            ValidateIssueKind::ObjectCollisionInSeqPrint
        );
        assert_eq!(result.issues[0].object_index, Some(1));
    }

    #[test]
    fn issue_is_attached_to_its_scene_object() {
        let refs = vec![
            SceneObjectRef {
                id: Some("a".into()),
                name: "A".into(),
            },
            SceneObjectRef {
                id: Some("b".into()),
                name: "B".into(),
            },
        ];
        let issue = to_issue(
            engine::api::ValidateIssue {
                message: "B is too tall, and collisions will be caused.".into(),
                kind: ValidateIssueKind::ObjectCollisionInSeqPrint,
                is_warning: false,
                opt_key: None,
                object_index: Some(1),
                params: Vec::new(),
            },
            &refs,
            &DynamicPrintConfig::new(),
            0,
        );
        assert_eq!(issue.kind, "object_collision_in_seq_print");
        assert_eq!(issue.object_id.as_deref(), Some("b"));
        assert_eq!(issue.object_name.as_deref(), Some("B"));
    }

    #[test]
    fn bed_type_message_uses_plate_and_filament_preset_like_the_gui() {
        let mut config = DynamicPrintConfig::new();
        config.0.insert(
            "filament_settings_id".into(),
            ConfigValue::Strings(vec!["Generic PLA @BBL X1C".into(), "Generic PETG".into()]),
        );
        let message =
            bed_type_message(&["1".into(), "Cool Plate".into(), "1".into()], &config, 2).unwrap();
        assert_eq!(
            message,
            "Plate 3: Cool Plate is not suggested to be used to print filament 1 (Generic PLA). If you still want to do this print job, please set this filament's bed temperature to non-zero."
        );
    }

    #[test]
    fn structured_worker_refusal_becomes_a_plate_error() {
        let error = crate::engine::WorkerError::Failed {
            code: 1,
            stdout: b"E invalid_config pas de calibration nul\n".to_vec(),
            stderr: String::new(),
        };
        assert_eq!(
            worker_refusal(&error).as_deref(),
            Some("pas de calibration nul")
        );
        let crash = crate::engine::WorkerError::Crashed {
            stderr: String::new(),
        };
        assert!(worker_refusal(&crash).is_none());
    }
}
