//! Lancement du tranchage (`POST /api/projects/{id}/slice`, contrat http-api.md,
//! T064).
//!
//! Fige les presets actifs résolus (`resolved_settings`, reproductibilité),
//! restitue les avertissements moteur (FR-032, bornes/enums clampés), refuse de
//! trancher un plateau vide ou contenant un objet hors plateau (erreur avant
//! slice), puis crée un job `queued` par plateau ciblé. L'exécution est prise en
//! charge par le pool de workers (T063) via le runner FFI (T066).

use std::collections::HashSet;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use engine::api::{ConfigValue, DynamicPrintConfig, RawPreset};
use engine::params::ParamKind;
use serde_json::{json, Value};

use super::presets;
use crate::domain::{NewJob, PresetId, ProjectId};
use crate::http::dto::{
    JobResponse, PlateSliceSignature, SliceRequest, SliceResponse, SliceSignatureResponse,
    SliceWarning,
};
use crate::http::error::{ApiError, ApiResult};
use crate::http::extract::CurrentUser;
use crate::http::state::AppState;

fn parse_project_id(raw: &str) -> ApiResult<ProjectId> {
    uuid::Uuid::parse_str(raw)
        .map(ProjectId)
        .map_err(|_| ApiError::not_found("Projet"))
}

/// `POST /api/projects/{id}/slice` — crée un ou plusieurs jobs de tranchage.
pub async fn slice(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(project_raw): Path<String>,
    Json(body): Json<SliceRequest>,
) -> ApiResult<(StatusCode, Json<SliceResponse>)> {
    let project_id = parse_project_id(&project_raw)?;
    let project = state.storage.projects().get(user.id, project_id).await?; // 404, SC-008
                                                                            // Scène non enregistrée de l'éditeur, sinon scène enregistrée.
    let scene = request_scene(&body, &project.scene)?;

    let targets = match plate_targets(scene, body.plate_index)? {
        targets if targets.is_empty() => {
            return Err(ApiError::validation(
                "plateau vide (aucun objet à trancher)",
                json!({ "plate_index": 0 }),
            ))
        }
        targets => targets,
    };

    // Garde avant slice : chaque plateau ciblé doit contenir des objets (les
    // objets hors plateau sont refusés plus bas, une fois le plateau de la
    // config connu).
    for &t in &targets {
        if plate_scene_ids(scene, t).is_empty() {
            return Err(ApiError::validation(
                "plateau vide (aucun objet à trancher)",
                json!({ "plate_index": t }),
            ));
        }
    }

    // Scène non enregistrée : chaque modèle référencé doit appartenir au
    // projet, sinon le job échouerait plus tard (« modèle supprimé ») — refus
    // immédiat, sans révéler si l'id existe ailleurs.
    let models = state
        .storage
        .models()
        .list(user.id, Some(project_id))
        .await?;
    if body.scene.is_some() {
        let known: HashSet<String> = models.iter().map(|m| m.id.to_string()).collect();
        let unknown = unknown_scene_models(scene, &known);
        if !unknown.is_empty() {
            return Err(ApiError::validation(
                "la scène référence un modèle absent du projet",
                json!({ "field": "scene", "model_ids": unknown }),
            ));
        }
    }

    let SliceConfig {
        resolved,
        warnings,
        allow_mixed,
    } = resolve_slice_config(
        &state,
        user.id,
        scene,
        request_active_presets(&body, &project.active_presets)?,
        &body,
    )
    .await?;
    // Lot 43 (audit 42 I1) : objets à cheval sur le bord, trop hauts ou hors
    // du plateau, calculés par le serveur (BuildVolume / calc_print_volume_state
    // d'Orca) sur la scène et le plateau de la config tranchée : refus avant
    // tout job, comme Orca grise « Trancher ».
    let placements = super::placement::object_placements(
        &state,
        user.id,
        project_id,
        scene,
        &resolved,
        body.plate_index.filter(|_| targets.len() == 1),
    )
    .await?;
    // Drapeau off_bed du client : pris en compte seulement pour un objet dont
    // le serveur ne connaît pas la géométrie (modèle absent).
    let off_bed = off_bed_ids(scene);
    for &t in &targets {
        let blocking: Vec<&super::placement::ObjectPlacement> = placements
            .iter()
            .filter(|o| o.plate_index == t && o.blocks_slicing())
            .collect();
        if !blocking.is_empty() {
            return Err(super::placement::outside_error(t, &blocking));
        }
        let unknown_flagged = plate_scene_ids(scene, t).into_iter().any(|id| {
            off_bed.contains(&id)
                && !placements
                    .iter()
                    .any(|o| o.object_id == id && o.state.is_some())
        });
        if unknown_flagged {
            return Err(ApiError::validation(
                "objet hors plateau : repositionner avant de trancher",
                json!({ "plate_index": t }),
            ));
        }
    }
    // Instantané de scène du job : options de tranchage hors config Orca.
    let mut job_scene = scene.clone();
    if allow_mixed {
        if let Value::Object(map) = &mut job_scene {
            map.insert(
                "sliceOptions".into(),
                json!({ "allowMixedTemperatures": true }),
            );
        }
    }

    // Quotas du compte (lot 32) : tranchages simultanés (429) puis stockage
    // (413, le G-code produit ne pourrait pas être gardé) — réservation
    // estimée du G-code de chaque plateau ciblé avant création des jobs
    // (lot 35 : plus de placeholder d'1 octet).
    crate::http::quota::ensure_slice_slots(&state, user.id, targets.len() as i64).await?;
    let reserved_gcode = estimated_gcode_bytes(scene, &models, &targets);
    crate::http::quota::ensure_storage_room(&state, user.id, reserved_gcode).await?;

    let mut jobs = Vec::with_capacity(targets.len());
    for &t in &targets {
        // Réglages par plateau (Orca « Plate settings » : type de plaque,
        // séquence d'impression, ordre de la 1re couche…) figés dans le
        // snapshot de ce plateau, par-dessus les presets.
        let resolved = with_plate_settings(&resolved, scene, t);
        // Signature de tranchage du plateau (lot 42, V1) figée avec le job :
        // l'interface la compare à celle du projet rouvert
        // (`GET /api/projects/{id}/slice-signature`).
        let mut plate_scene = job_scene.clone();
        if let Value::Object(map) = &mut plate_scene {
            map.insert(
                crate::slice_signature::SCENE_KEY.into(),
                Value::String(crate::slice_signature::plate_signature(&resolved, scene, t)),
            );
        }
        let job = state
            .storage
            .jobs()
            .enqueue_with_scene(
                user.id,
                NewJob {
                    project_id,
                    plate_index: t,
                    resolved_settings: resolved.clone(),
                },
                plate_scene,
            )
            .await?;
        jobs.push(JobResponse::from(job));
    }

    Ok((StatusCode::CREATED, Json(SliceResponse { jobs, warnings })))
}

/// `GET /api/projects/{id}/slice-signature` — signature de tranchage de chaque
/// plateau non vide du projet **enregistré** (presets actifs + surcharges +
/// réglages projet / plateau + objets du plateau, calcul documenté dans
/// `slice_signature.rs`) et dernier job réussi du plateau avec `fresh` : à la
/// réouverture, l'interface ne restaure le dernier G-code que s'il est `fresh`
/// (Orca ne rouvre jamais un résultat périmé).
pub async fn slice_signature(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(project_raw): Path<String>,
) -> ApiResult<Json<SliceSignatureResponse>> {
    let project_id = parse_project_id(&project_raw)?;
    let project = state.storage.projects().get(user.id, project_id).await?; // 404, SC-008
    let scene = &project.scene;
    let targets = plate_targets(scene, None)?;
    let saved = SliceRequest {
        plate_index: None,
        all: None,
        draft: None,
        allow_mixed_temperatures: None,
        scene: None,
        active_presets: None,
    };
    let jobs = state.storage.jobs().list(user.id).await?;
    let mut plates = Vec::with_capacity(targets.len());
    if !targets.is_empty() {
        let SliceConfig { resolved, .. } =
            resolve_slice_config(&state, user.id, scene, &project.active_presets, &saved).await?;
        for t in targets {
            let settings = with_plate_settings(&resolved, scene, t);
            let signature = crate::slice_signature::plate_signature(&settings, scene, t);
            let last = jobs
                .iter()
                .filter(|j| {
                    j.project_id == project_id
                        && j.plate_index == t
                        && j.status == crate::domain::JobStatus::Succeeded
                        && j.gcode_id.is_some()
                })
                .max_by_key(|j| j.created_at);
            let last_signature =
                last.and_then(|j| crate::slice_signature::of_job_scene(j.scene.as_ref()));
            plates.push(PlateSliceSignature {
                plate_index: t,
                fresh: last_signature.as_deref() == Some(signature.as_str()),
                signature,
                last_job_id: last.map(|j| j.id.to_string()),
                last_gcode_id: last.and_then(|j| j.gcode_id).map(|g| g.to_string()),
                last_signature,
            });
        }
    }
    Ok(Json(SliceSignatureResponse {
        version: crate::slice_signature::SIGNATURE_VERSION.into(),
        plates,
    }))
}

/// Scène à valider / trancher : `scene` du corps (état non enregistré de
/// l'éditeur) ou, à défaut, la scène enregistrée du projet. Même format que
/// la scène du projet (objet JSON) ; autre chose → 422. Les modèles qu'elle
/// référence sont relus parmi ceux du projet de l'utilisateur
/// (`build_scene_model`) : une scène ne peut pas désigner le modèle d'autrui.
pub(crate) fn request_scene<'a>(body: &'a SliceRequest, saved: &'a Value) -> ApiResult<&'a Value> {
    match &body.scene {
        None => Ok(saved),
        Some(scene) if scene.is_object() => Ok(scene),
        Some(_) => Err(ApiError::validation(
            "scene doit être un objet (même format que la scène du projet)",
            json!({ "field": "scene" }),
        )),
    }
}

/// Presets actifs à trancher : `active_presets` du corps (presets affichés non
/// enregistrés) ou, à défaut, ceux enregistrés du projet. Objet JSON au format
/// du projet ; autre chose → 422 `field = active_presets`. Les ids sont
/// ensuite résolus parmi les presets visibles du compte
/// (`resolve_active_config`) : même garde que pour le projet enregistré.
pub(crate) fn request_active_presets<'a>(
    body: &'a SliceRequest,
    saved: &'a Value,
) -> ApiResult<&'a Value> {
    match &body.active_presets {
        None => Ok(saved),
        Some(active) if valid_active_presets(active) => Ok(active),
        Some(_) => Err(ApiError::validation(
            "active_presets doit être un objet { printer?, process?, filaments? } (ids de presets)",
            json!({ "field": "active_presets" }),
        )),
    }
}

fn valid_active_presets(active: &Value) -> bool {
    let Some(map) = active.as_object() else {
        return false;
    };
    let id_ok = |v: &Value| v.as_str().is_some_and(|s| uuid::Uuid::parse_str(s).is_ok());
    map.iter().all(|(k, v)| match k.as_str() {
        "printer" | "process" => v.is_null() || id_ok(v),
        "filaments" => v
            .as_array()
            .is_some_and(|a| a.iter().all(|f| f.is_null() || id_ok(f))),
        // Clés annexes tolérées (même tolérance que l'enregistrement du projet).
        _ => true,
    })
}

/// Plateaux ciblés : `plate_index` (borné, 422 sinon) ou, pour « tout
/// trancher », les plateaux non vides (« Slice all » d'Orca : un plateau vide
/// n'a rien à trancher et est sauté, Plater::priv::on_process_completed →
/// plateau suivant). Vide = aucun objet dans le projet.
pub(crate) fn plate_targets(scene: &Value, plate_index: Option<i64>) -> ApiResult<Vec<i64>> {
    let plate_count = plate_object_ids(scene).len().max(1);
    match plate_index {
        Some(n) => {
            if n < 0 || n as usize >= plate_count {
                return Err(ApiError::validation(
                    "plate_index hors limites",
                    json!({ "plate_index": n, "plate_count": plate_count }),
                ));
            }
            Ok(vec![n])
        }
        None => Ok((0..plate_count as i64)
            .filter(|&t| !plate_scene_ids(scene, t).is_empty())
            .collect()),
    }
}

/// Objets (ids de scène) du plateau `t` ; sans plateaux, tous les objets.
pub(crate) fn plate_scene_ids(scene: &Value, t: i64) -> Vec<String> {
    plate_object_ids(scene)
        .get(t.max(0) as usize)
        .cloned()
        .unwrap_or_else(|| all_object_ids(scene))
}

/// Réservation de quota G-code (lot 35) : somme sur les plateaux ciblés de
/// l'estimation par plateau ([`crate::http::quota::estimate_gcode_bytes`]) à
/// partir des triangles des modèles des objets du plateau.
fn estimated_gcode_bytes(scene: &Value, models: &[crate::domain::Model], targets: &[i64]) -> i64 {
    let triangles_of = |model_id: &str| -> i64 {
        models
            .iter()
            .find(|m| m.id.to_string() == model_id)
            .map(|m| m.triangle_count)
            .unwrap_or(0)
    };
    let mut total = 0i64;
    for &t in targets {
        let ids: HashSet<String> = plate_scene_ids(scene, t).into_iter().collect();
        let triangles: i64 = scene
            .get("objects")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter(|object| {
                object
                    .get("id")
                    .and_then(Value::as_str)
                    .is_some_and(|id| ids.contains(id))
            })
            .filter_map(|object| object.get("modelId").and_then(Value::as_str))
            .map(triangles_of)
            .sum();
        total = total.saturating_add(crate::http::quota::estimate_gcode_bytes(triangles));
    }
    total
}

/// Configuration réellement tranchée (presets actifs + brouillon + projet).
pub(crate) struct SliceConfig {
    /// Snapshot JSON figé dans le job (`resolved_settings`, hors plateau).
    pub resolved: Value,
    /// Avertissements de configuration (bornes clampées, post-traitement,
    /// compatibilité des filaments).
    pub warnings: Vec<SliceWarning>,
    pub allow_mixed: bool,
}

/// Presets actifs résolus (figés) + avertissements moteur, puis brouillon de
/// l'éditeur, puis réglages du projet (scene.projectConfig : couleurs,
/// matrice/multiplicateur de purge) par-dessus, comme Orca applique sa config
/// projet sur les presets. Partagé par POST …/slice et POST …/validate.
pub(crate) async fn resolve_slice_config(
    state: &AppState,
    user: crate::domain::UserId,
    scene: &Value,
    active_presets: &Value,
    body: &SliceRequest,
) -> ApiResult<SliceConfig> {
    // Brouillon de l'éditeur fourni : il remplace les surcharges enregistrées
    // des presets actifs (il porte l'état édité complet), sinon elles
    // s'appliquent (lot 40).
    let stripped;
    let active_presets = if body.draft.is_some() {
        stripped = super::overrides::without_overrides(active_presets);
        &stripped
    } else {
        active_presets
    };
    let (mut merged, mut warnings) = resolve_active_config(state, user, active_presets).await?;
    if let Some(draft) = &body.draft {
        apply_draft(&mut merged, draft)?;
        // Avertissements recalculés sur la config réellement tranchée.
        warnings = merged
            .validate()
            .into_iter()
            .map(|w| SliceWarning {
                key: w.key,
                message: w.message,
            })
            .collect();
    }
    apply_project_config(&mut merged, scene);
    if let Some(warning) = post_process_warning(&merged) {
        warnings.push(warning);
    }
    let allow_mixed = body.allow_mixed_temperatures.unwrap_or(false);
    if let Some(warning) = filament_compatibility_warning(&merged, allow_mixed) {
        warnings.push(warning);
    }
    let resolved = Value::Object(presets::config_to_json(&merged));
    Ok(SliceConfig {
        resolved,
        warnings,
        allow_mixed,
    })
}

/// Vérification haute/basse température d'Orca sur les filaments `indices`
/// (tous si vide) de la config tranchée (`check_multi_filament_valid`).
pub(crate) fn filament_compatibility(
    config: &DynamicPrintConfig,
    indices: &[usize],
) -> engine::api::filament_compat::FilamentCompatibility {
    let strings = |k: &str| match config.get(k) {
        Some(ConfigValue::Strings(v)) => v.clone(),
        Some(ConfigValue::String(v)) => vec![v.clone()],
        _ => Vec::new(),
    };
    let ints = |k: &str| match config.get(k) {
        Some(ConfigValue::Ints(v)) => v.clone(),
        Some(ConfigValue::Int(v)) => vec![*v],
        _ => Vec::new(),
    };
    let types = strings("filament_type");
    let temps = ints("nozzle_temperature");
    let lows = ints("nozzle_temperature_range_low");
    let highs = ints("nozzle_temperature_range_high");
    // get_at d'Orca : index hors vecteur → premier élément.
    let at = |v: &Vec<i64>, i: usize| v.get(i).or(v.first()).copied().unwrap_or(0);
    let picked: Vec<usize> = if indices.is_empty() {
        (0..types.len()).collect()
    } else {
        indices
            .iter()
            .copied()
            .filter(|i| *i < types.len().max(1))
            .collect()
    };
    let t: Vec<String> = picked
        .iter()
        .map(|&i| types.get(i).or(types.first()).cloned().unwrap_or_default())
        .collect();
    let n: Vec<i64> = picked.iter().map(|&i| at(&temps, i)).collect();
    let l: Vec<i64> = picked.iter().map(|&i| at(&lows, i)).collect();
    let h: Vec<i64> = picked.iter().map(|&i| at(&highs, i)).collect();
    engine::api::filament_compat::check_multi_filaments_compatibility(&t, &n, &l, &h)
}

/// Avertissement pré-tranchage (tous les filaments de la barre ; le moteur
/// tranche avec les filaments réellement utilisés et refuse si nécessaire).
fn filament_compatibility_warning(
    config: &DynamicPrintConfig,
    allow_mixed: bool,
) -> Option<SliceWarning> {
    let compat = filament_compatibility(config, &[]);
    engine::api::filament_compat::verdict(compat, allow_mixed).map(|(message, _)| SliceWarning {
        key: "filament_compatibility".into(),
        message,
    })
}

/// Corps de `POST /api/projects/{id}/filament-compatibility`.
#[derive(Debug, Clone, Default, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
pub struct FilamentCompatibilityRequest {
    /// Filaments utilisés (index 0-based de la barre) ; absent = tous.
    #[ts(optional)]
    pub filaments: Option<Vec<usize>>,
    /// Brouillon de l'éditeur, comme pour POST …/slice.
    #[ts(optional)]
    pub draft: Option<crate::http::dto::SliceDraft>,
    /// Préférence « Remove mixed temperature restriction ».
    #[ts(optional)]
    pub allow_mixed_temperatures: Option<bool>,
    /// Presets actifs affichés non enregistrés, comme pour POST …/slice.
    #[ts(
        optional,
        type = "{ printer?: string; process?: string; filaments?: Array<string> }"
    )]
    pub active_presets: Option<serde_json::Value>,
}

/// Réponse : verdict d'Orca.
#[derive(Debug, Clone, serde::Serialize, ts_rs::TS)]
#[ts(export)]
pub struct FilamentCompatibilityResponse {
    /// `compatible` | `high_low_mixed` | `invalid_temperature_range`.
    #[ts(type = "\"compatible\" | \"high_low_mixed\" | \"invalid_temperature_range\"")]
    pub compatibility: engine::api::filament_compat::FilamentCompatibility,
    /// Le tranchage sera refusé (sinon simple avertissement).
    pub blocking: bool,
    /// Message d'Orca (anglais, à traduire côté client) ; absent si compatible.
    #[ts(optional)]
    pub message: Option<String>,
}

/// `POST /api/projects/{id}/filament-compatibility` — vérification haute/basse
/// température des filaments (Print::check_multi_filament_valid d'Orca) sur les
/// presets actifs du projet (+ brouillon), sans trancher.
pub async fn filament_compatibility_check(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(project_raw): Path<String>,
    Json(body): Json<FilamentCompatibilityRequest>,
) -> ApiResult<Json<FilamentCompatibilityResponse>> {
    let project_id = parse_project_id(&project_raw)?;
    let project = state.storage.projects().get(user.id, project_id).await?;
    let active = match &body.active_presets {
        Some(active) if valid_active_presets(active) => active,
        Some(_) => return Err(ApiError::validation(
            "active_presets doit être un objet { printer?, process?, filaments? } (ids de presets)",
            json!({ "field": "active_presets" }),
        )),
        None => &project.active_presets,
    };
    let (mut merged, _) = resolve_active_config(&state, user.id, active).await?;
    if let Some(draft) = &body.draft {
        apply_draft(&mut merged, draft)?;
    }
    apply_project_config(&mut merged, &project.scene);
    let compat = filament_compatibility(&merged, body.filaments.as_deref().unwrap_or(&[]));
    let verdict = engine::api::filament_compat::verdict(
        compat,
        body.allow_mixed_temperatures.unwrap_or(false),
    );
    Ok(Json(FilamentCompatibilityResponse {
        compatibility: compat,
        blocking: verdict.as_ref().is_some_and(|v| v.1),
        message: verdict.map(|v| v.0),
    }))
}

/// Applique le brouillon de l'éditeur (presets modifiés non enregistrés) :
/// process/imprimante clé par clé, filaments sur l'élément `index` des
/// vecteurs par filament (une clé scalaire n'est prise que du filament 1,
/// comme la fusion d'Orca). Valeur illisible → 422.
///
/// `null` (seul ou dans un vecteur) sur une option **nullable** d'Orca
/// (`ConfigOptionFloatsNullable`, `…BoolsNullable`, enums nullables : surcharges
/// filament de la rétraction, etc.) vaut `nil` (« hériter de l'imprimante »),
/// comme `ConfigOption*Nullable::deserialize` ; sur une option non nullable
/// Orca refuse (« Deserializing nil into a non-nullable object ») → 422 ciblé.
pub(crate) fn apply_draft(
    merged: &mut DynamicPrintConfig,
    draft: &crate::http::dto::SliceDraft,
) -> ApiResult<()> {
    let parse = |values: &serde_json::Map<String, Value>| {
        let values = draft_nil_values(values)?;
        let config = engine::presets::resolve_preset_chain(&[engine::api::RawPreset {
            name: "draft".into(),
            inherits: None,
            values: values.clone(),
        }])
        .map_err(|e| ApiError::validation(format!("brouillon invalide : {e}"), json!({})))?;
        // Le registre ignore une valeur illisible : une clé Orca connue absente
        // du résultat est une saisie invalide (les clés inconnues sont ignorées
        // comme par Orca).
        if let Some(key) = values
            .keys()
            .find(|k| engine::params::get(k).is_some() && !config.0.contains_key(*k))
        {
            return Err(ApiError::validation(
                format!("brouillon invalide : valeur illisible pour {key}"),
                json!({ "key": key }),
            ));
        }
        Ok(config)
    };
    for values in [&draft.printer, &draft.process].into_iter().flatten() {
        merged.0.extend(parse(values)?.0);
    }
    for (index, values) in draft.filaments.iter().flatten().enumerate() {
        let Some(values) = values else { continue };
        for (key, incoming) in parse(values)?.0 {
            set_filament_value(merged, index, key, incoming);
        }
    }
    Ok(())
}

/// `null` JSON → `nil` d'Orca pour les options nullables (voir `apply_draft`).
fn draft_nil_values(
    values: &serde_json::Map<String, Value>,
) -> ApiResult<serde_json::Map<String, Value>> {
    let mut out = values.clone();
    for (key, value) in out.iter_mut() {
        let Some(def) = engine::params::get(key) else {
            continue; // clé inconnue : ignorée comme Orca
        };
        let has_null = match value {
            Value::Null => true,
            Value::Array(items) => items.iter().any(Value::is_null),
            _ => false,
        };
        if !has_null {
            continue;
        }
        if !def.nullable {
            return Err(ApiError::validation(
                format!("brouillon invalide : {key} n'accepte pas de valeur nulle (nil)"),
                json!({ "key": key }),
            ));
        }
        match value {
            Value::Array(items) => {
                for item in items.iter_mut().filter(|item| item.is_null()) {
                    *item = Value::String("nil".into());
                }
            }
            other => *other = Value::String("nil".into()),
        }
    }
    Ok(out)
}

/// Remplace la valeur du filament `index` d'une clé (vecteur par filament) ;
/// clé scalaire : seul le filament 1 la porte. Un `nil` dans un vecteur
/// jusqu'ici non nullable le convertit en vecteur nullable (même longueur),
/// comme `ConfigOptionFloatsNullable` d'Orca.
fn set_filament_value(
    merged: &mut DynamicPrintConfig,
    index: usize,
    key: String,
    incoming: ConfigValue,
) {
    let existing_len = match merged.0.get(&key) {
        Some(ConfigValue::Bools(v)) => v.len(),
        Some(ConfigValue::BoolsNullable(v)) => v.len(),
        Some(ConfigValue::Ints(v)) => v.len(),
        Some(ConfigValue::Floats(v)) => v.len(),
        Some(ConfigValue::FloatsNullable(v)) => v.len(),
        Some(ConfigValue::Strings(v)) => v.len(),
        Some(ConfigValue::Points(v)) => v.len(),
        _ => 0,
    };
    let slots = variant_slots(merged, &key, index, existing_len);
    // Lot 44 : option par variante (vecteur filament × variante) — les cases
    // du filament `index` reçoivent ses valeurs (une par variante, sinon la
    // première), au lieu de la case `index` qui appartient à un autre filament.
    macro_rules! put {
        ($out:expr, $vals:expr, $fill:expr) => {{
            if let Some(slots) = slots.clone() {
                for (n, position) in slots.into_iter().enumerate() {
                    if let Some(v) = $vals.get(n).or($vals.first()).cloned() {
                        $out[position] = v;
                    }
                }
                return;
            }
            if let Some(v) = $vals.first().cloned() {
                if $out.len() <= index {
                    let pad = $out.last().cloned().unwrap_or($fill);
                    $out.resize(index + 1, pad);
                }
                $out[index] = v;
            }
            return;
        }};
    }
    if let Some(existing) = merged.0.get_mut(&key) {
        // Promotion en vecteur nullable quand le brouillon apporte un `nil`.
        match (&*existing, &incoming) {
            (ConfigValue::Floats(v), ConfigValue::FloatsNullable(_)) => {
                *existing = ConfigValue::FloatsNullable(v.iter().map(|x| Some(*x)).collect());
            }
            (ConfigValue::Bools(v), ConfigValue::BoolsNullable(_)) => {
                *existing = ConfigValue::BoolsNullable(v.iter().map(|x| Some(*x)).collect());
            }
            _ => {}
        }
        match (existing, &incoming) {
            (ConfigValue::Bools(out), ConfigValue::Bools(v)) => put!(out, v, false),
            (ConfigValue::Ints(out), ConfigValue::Ints(v)) => put!(out, v, 0),
            (ConfigValue::Floats(out), ConfigValue::Floats(v)) => put!(out, v, 0.0),
            (ConfigValue::FloatsNullable(out), ConfigValue::FloatsNullable(v)) => {
                put!(out, v, None)
            }
            (ConfigValue::FloatsNullable(out), ConfigValue::Floats(v)) => {
                let v: Vec<Option<f64>> = v.iter().map(|x| Some(*x)).collect();
                put!(out, v, None)
            }
            (ConfigValue::BoolsNullable(out), ConfigValue::BoolsNullable(v)) => put!(out, v, None),
            (ConfigValue::BoolsNullable(out), ConfigValue::Bools(v)) => {
                let v: Vec<Option<bool>> = v.iter().map(|x| Some(*x)).collect();
                put!(out, v, None)
            }
            (ConfigValue::Strings(out), ConfigValue::Strings(v)) => put!(out, v, String::new()),
            (ConfigValue::Points(out), ConfigValue::Points(v)) => put!(out, v, Vec::new()),
            _ => {}
        }
    }
    if index == 0 || (is_extruder_vector(&incoming) && !merged.0.contains_key(&key)) {
        merged.0.insert(key, incoming);
    }
}

/// Résout les presets actifs (`{printer, process, filaments[], overrides?}`) en
/// une config unique figée, surcharges `overrides` comprises (lot 40) ; les
/// valeurs hors bornes sont clampées et signalées (FR-032).
/// Résout les presets actifs en `DynamicPrintConfig` typée (tranchage, export
/// 3MF, T072).
pub(crate) async fn resolve_active_config(
    state: &AppState,
    user: crate::domain::UserId,
    active: &Value,
) -> ApiResult<(engine::api::DynamicPrintConfig, Vec<SliceWarning>)> {
    // Garde de possession (Phase 14) : le preset imprimante retenu doit
    // correspondre à un **modèle** d'imprimante **déclaré** par l'utilisateur — on
    // ne tranche/exporte pas pour une imprimante qu'il ne possède pas. La garde
    // raisonne au niveau du modèle (et non du preset exact) : posséder une
    // imprimante autorise **toutes ses buses** (variantes du même modèle), ce qui
    // permet à l'éditeur de changer de buse sans redéclarer l'imprimante. L'absence
    // de clé `printer` reste tolérée (config par défaut).
    if let Some(s) = active.get("printer").and_then(Value::as_str) {
        let printer_id = parse_preset_id(s)?;
        let owned = state.storage.printers().list(user).await?;
        let machines = state
            .storage
            .presets()
            .list_by_kind(crate::domain::PresetKind::Machine, user)
            .await?;
        // Modèle (au sens OrcaSlicer) d'un preset machine, par son id. Seuls les
        // presets visibles (système + ceux de `user`, cf. `list_by_kind`) sont
        // considérés : un preset d'un autre compte n'a pas de modèle → refus.
        let model_of = |id: &str| -> Option<String> {
            machines
                .iter()
                .find(|p| p.id.to_string() == id)
                .map(|p| machine_model(&machines, p))
        };
        let owned_models: HashSet<String> = owned
            .iter()
            .filter_map(|p| model_of(&p.machine_preset_id.to_string()))
            .collect();
        let owns_it =
            model_of(&printer_id.to_string()).is_some_and(|model| owned_models.contains(&model));
        if !owns_it {
            return Err(ApiError::printer_not_owned(
                "Imprimante non possédée : ajoutez-la à vos imprimantes avant de trancher",
            ));
        }
    }

    let mut merged = DynamicPrintConfig::new();
    for key in ["printer", "process"] {
        if let Some(s) = active.get(key).and_then(Value::as_str) {
            let (preset, cfg) = resolve_one_preset(state, user, parse_preset_id(s)?).await?;
            if key == "process" {
                // La liste compatible_printers du filament ne doit pas écraser
                // la compatibilité du process lors de la fusion des portées.
                if let Some(compatible) = cfg.get("compatible_printers") {
                    merged
                        .0
                        .insert("print_compatible_printers".into(), compatible.clone());
                }
            }
            // Les presets machine/process se surchargent normalement clé par
            // clé ; leurs vecteurs ne décrivent pas une liste de filaments.
            merged.0.extend(cfg.0);
            let metadata_key = if key == "printer" {
                "printer_settings_id"
            } else {
                "print_settings_id"
            };
            merged.0.insert(
                metadata_key.into(),
                ConfigValue::String(preset.name.clone()),
            );
            if key == "printer" {
                let (_, variant) = super::printers::split_machine_name(&preset.name);
                if !variant.is_empty() {
                    merged
                        .0
                        .insert("printer_variant".into(), ConfigValue::String(variant));
                }
            }
        }
    }

    let mut filament_ids = Vec::new();
    if let Some(arr) = active.get("filaments").and_then(Value::as_array) {
        for v in arr {
            if let Some(s) = v.as_str() {
                filament_ids.push(parse_preset_id(s)?);
            }
        }
    }
    let mut filament_names = Vec::new();
    let mut filament_configs = Vec::new();
    for id in filament_ids {
        let (preset, cfg) = resolve_one_preset(state, user, id).await?;
        filament_names.push(preset.name);
        filament_configs.push(cfg);
    }
    merge_filament_configs(&mut merged, filament_configs);
    if !filament_names.is_empty() {
        merged.0.insert(
            "filament_settings_id".into(),
            ConfigValue::Strings(filament_names),
        );
    }
    // Réglages modifiés non enregistrés du projet (lot 40) : preset
    // « (modifié) » d'Orca, par-dessus les presets résolus.
    if let Some(overrides) = super::overrides::overrides_of(active)? {
        apply_draft(&mut merged, &overrides)?;
    }

    let warnings = merged
        .validate()
        .into_iter()
        .map(|w| SliceWarning {
            key: w.key,
            message: w.message,
        })
        .collect();

    Ok((merged, warnings))
}

/// Modèle d'imprimante d'un preset machine pour la garde de possession.
///
/// Un preset utilisateur dérivé (« `Voron 2.4 300 0.4 nozzle (copie)` », ou une
/// copie de copie) ne suit plus la convention de nommage Orca : on remonte donc
/// sa chaîne `inherits` (mêmes règles que la résolution, `presets::build_chain`)
/// jusqu'au premier ancêtre **système instanciable** et l'on retient son modèle.
/// Sans tel ancêtre (chaîne purement utilisateur, ou ne menant qu'à un parent
/// abstrait `fdm_*_common`), on retombe sur le nom de la feuille (comportement
/// historique). `visible` ne doit contenir que des presets visibles par le
/// compte courant : l'isolation inter-comptes en découle.
pub(crate) fn machine_model(
    visible: &[crate::domain::Preset],
    leaf: &crate::domain::Preset,
) -> String {
    let system: HashSet<&str> = visible
        .iter()
        .filter(|p| p.origin == crate::domain::PresetOrigin::System && p.instantiation)
        .map(|p| p.name.as_str())
        .collect();
    let leaf_is_system = leaf.origin == crate::domain::PresetOrigin::System && leaf.instantiation;
    let name = if leaf_is_system {
        leaf.name.clone()
    } else {
        presets::build_chain(visible, leaf)
            .into_iter()
            .rev() // feuille → racine
            .find(|raw| system.contains(raw.name.as_str()))
            .map(|raw| raw.name)
            .unwrap_or_else(|| leaf.name.clone())
    };
    super::printers::split_machine_name(&name).0
}

async fn resolve_one_preset(
    state: &AppState,
    user: crate::domain::UserId,
    id: PresetId,
) -> ApiResult<(crate::domain::Preset, DynamicPrintConfig)> {
    let leaf = presets::load_visible(state, user, id).await?;
    let chain = presets::load_chain(state, user, &leaf).await?;
    let config = resolve_preset_chain_orca(&chain).map_err(|e| {
        tracing::error!(error = %e, "résolution de preset pour le slice");
        ApiError::internal()
    })?;
    Ok((leaf, config))
}

/// Les vecteurs de chaînes et enums utilisent la forme interne sérialisée par
/// le moteur ; `orca_values` accepte ensuite les séparateurs canoniques Orca.
fn resolve_preset_chain_orca(chain: &[RawPreset]) -> engine::api::EngineResult<DynamicPrintConfig> {
    let chain: Vec<RawPreset> = chain
        .iter()
        .map(|preset| {
            let mut values = preset.values.clone();
            for (key, value) in &mut values {
                let is_strings = engine::params::get(key)
                    .is_some_and(|def| matches!(def.kind, ParamKind::Strings | ParamKind::Enums));
                if is_strings {
                    if let Value::Array(items) = value {
                        *value = Value::String(join_string_items(items));
                    }
                }
            }
            RawPreset {
                name: preset.name.clone(),
                inherits: preset.inherits.clone(),
                values,
            }
        })
        .collect();
    engine::presets::resolve_preset_chain(&chain)
}

fn json_item_text(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Null => "nil".into(),
        other => other.to_string(),
    }
}

/// Sérialise chaque élément comme une chaîne Orca : le séparateur `;` peut
/// ainsi apparaître dans une valeur sans créer un élément supplémentaire.
fn join_string_items(items: &[Value]) -> String {
    items
        .iter()
        .map(|value| match value {
            Value::Null => "nil".into(),
            _ => format!(
                "\"{}\"",
                json_item_text(value)
                    .replace('\\', "\\\\")
                    .replace('"', "\\\"")
            ),
        })
        .collect::<Vec<_>>()
        .join(";")
}

/// Fusionne les presets filament dans l'ordre, comme
/// `PresetBundle::construct_full_config` sur des presets COMPLETS : une clé
/// par filament absente d'un preset y prend sa valeur par défaut
/// (`print_config_def`, registre) avant d'être concaténée. Sans cela, un
/// filament qui ne définit pas `filament_colour` (défaut #F2754E) décalait
/// les vecteurs : [#FF0000] au lieu de [#F2754E, #FF0000] (lot multi-filament).
fn merge_filament_configs(merged: &mut DynamicPrintConfig, filaments: Vec<DynamicPrintConfig>) {
    // Clés par filament lues sans borne par libslic3r (ordonnancement des outils) :
    // toujours une valeur par filament, même si aucun preset ne les définit
    // (presets Prusa sans filament_is_support : vecteur à 1 entrée pour 2
    // filaments → refus « projet incohérent »). Orca les complète par le défaut.
    const ALWAYS_PER_FILAMENT: [&str; 2] = ["filament_colour", "filament_is_support"];
    let mut vector_keys: std::collections::BTreeSet<String> = filaments
        .iter()
        .flat_map(|f| f.0.iter())
        .filter(|(_, v)| is_extruder_vector(v))
        .map(|(k, _)| k.clone())
        .collect();
    vector_keys.extend(ALWAYS_PER_FILAMENT.iter().map(|k| k.to_string()));
    let multi = filaments.len() > 1;
    let mut variant_counts = Vec::with_capacity(filaments.len());
    for (index, mut filament) in filaments.into_iter().enumerate() {
        if multi {
            for key in &vector_keys {
                if !filament.0.contains_key(key) {
                    if let Some(default) = registry_default(key) {
                        filament.0.insert(key.clone(), default);
                    }
                }
            }
        }
        // Après les défauts : toutes les options par variante du filament.
        extend_variant_values(&mut filament);
        variant_counts.push(match filament.0.get("filament_extruder_variant") {
            Some(ConfigValue::Strings(values)) if !values.is_empty() => values.len(),
            _ => 1,
        });
        merge_filament_config(merged, filament, index);
    }
    set_filament_self_index(merged, &variant_counts);
}

/// Options filament « par variante de buse » d'Orca 2.4.1
/// (`filament_options_with_variant`, PrintConfig.cpp:8375) : une valeur par
/// élément de `filament_extruder_variant`.
const FILAMENT_OPTIONS_WITH_VARIANT: [&str; 37] = [
    "filament_flow_ratio",
    "filament_max_volumetric_speed",
    "filament_extruder_variant",
    "filament_retraction_length",
    "filament_z_hop",
    "filament_z_hop_types",
    "filament_retract_lift_above",
    "filament_retract_lift_below",
    "filament_retract_lift_enforce",
    "filament_retract_restart_extra",
    "filament_retraction_speed",
    "filament_deretraction_speed",
    "filament_retraction_minimum_travel",
    "filament_retract_when_changing_layer",
    "filament_wipe",
    "filament_wipe_distance",
    "filament_retract_before_wipe",
    "filament_long_retractions_when_cut",
    "filament_retraction_distances_when_cut",
    "long_retractions_when_ec",
    "retraction_distances_when_ec",
    "nozzle_temperature_initial_layer",
    "nozzle_temperature",
    "filament_flush_volumetric_speed",
    "filament_flush_temp",
    "filament_cooling_before_tower",
    "volumetric_speed_coefficients",
    "filament_adaptive_volumetric_speed",
    "filament_ironing_flow",
    "filament_ironing_spacing",
    "filament_ironing_inset",
    "filament_ironing_speed",
    "activate_air_filtration",
    "activate_air_filtration_during_print",
    "activate_air_filtration_on_completion",
    "during_print_exhaust_fan_speed",
    "complete_print_exhaust_fan_speed",
];

/// Lot 44 (audit 43 I3-bis) : `extend_default_config_length` d'Orca
/// (Preset.cpp:231, appliqué à chaque preset chargé) — chaque option par
/// variante d'un filament a exactement autant de valeurs que son
/// `filament_extruder_variant` (`ConfigOptionVector::resize` : complétée par
/// la **première** valeur, tronquée au-delà). Sans cela, un preset qui ne
/// donne qu'une valeur (ex. `during_print_exhaust_fan_speed` hérité de
/// `fdm_filament_common`) pour 2 variantes (Bambu PLA Basic : Standard / High
/// Flow) décale la lecture par `filament_self_index` de libslic3r : le 2ᵉ
/// filament lisait une case absente (« 70,0 » au lieu de « 70,70 »).
fn extend_variant_values(filament: &mut DynamicPrintConfig) {
    let length = match filament.0.get("filament_extruder_variant") {
        Some(ConfigValue::Strings(values)) if !values.is_empty() => values.len(),
        _ => return,
    };
    fn fit<T: Clone>(values: &mut Vec<T>, length: usize) {
        if let Some(first) = values.first().cloned() {
            values.resize(length, first);
        }
    }
    for key in FILAMENT_OPTIONS_WITH_VARIANT {
        match filament.0.get_mut(key) {
            Some(ConfigValue::Bools(v)) => fit(v, length),
            Some(ConfigValue::BoolsNullable(v)) => fit(v, length),
            Some(ConfigValue::Ints(v)) => fit(v, length),
            Some(ConfigValue::Floats(v)) => fit(v, length),
            Some(ConfigValue::FloatsNullable(v)) => fit(v, length),
            Some(ConfigValue::Strings(v)) => fit(v, length),
            Some(ConfigValue::Points(v)) => fit(v, length),
            _ => {}
        }
    }
}

/// Cases du filament `index` dans un vecteur par variante (positions où
/// `filament_self_index` vaut `index + 1`), si le vecteur a bien une valeur
/// par élément de `filament_self_index`.
fn variant_slots(
    merged: &DynamicPrintConfig,
    key: &str,
    index: usize,
    len: usize,
) -> Option<Vec<usize>> {
    if !FILAMENT_OPTIONS_WITH_VARIANT.contains(&key) {
        return None;
    }
    let Some(ConfigValue::Ints(self_index)) = merged.0.get("filament_self_index") else {
        return None;
    };
    if self_index.len() != len {
        return None;
    }
    let slots: Vec<usize> = self_index
        .iter()
        .enumerate()
        .filter(|(_, owner)| **owner == index as i64 + 1)
        .map(|(position, _)| position)
        .collect();
    (!slots.is_empty()).then_some(slots)
}

/// `filament_self_index` comme PresetBundle::construct_full_config d'Orca : une
/// entrée par élément de `filament_extruder_variant` (filament × variante de
/// buse), valant le n° du filament (1-based). Print::apply s'en sert
/// (update_values_to_printer_extruders_for_multiple_filaments) pour prendre,
/// pour CHAQUE filament, ses propres valeurs de la variante de sa buse
/// (Standard / High Flow). Sans lui, libslic3r prend la première entrée de la
/// variante, celle du filament 1, pour tous les filaments. Mono-filament :
/// tout à 1 (resize(index_size, 1)).
fn set_filament_self_index(merged: &mut DynamicPrintConfig, variant_counts: &[usize]) {
    let Some(ConfigValue::Strings(variants)) = merged.0.get("filament_extruder_variant") else {
        return;
    };
    let size = variants.len();
    let mut indices: Vec<i64> = if variant_counts.len() > 1 {
        variant_counts
            .iter()
            .enumerate()
            .flat_map(|(i, n)| std::iter::repeat_n(i as i64 + 1, *n))
            .collect()
    } else {
        Vec::new()
    };
    indices.resize(size, 1);
    merged
        .0
        .insert("filament_self_index".into(), ConfigValue::Ints(indices));
}

/// Valeur par défaut d'Orca (`print_config_def`, via le registre) d'une clé.
fn registry_default(key: &str) -> Option<ConfigValue> {
    let def = engine::params::get(key)?;
    let value: Value = serde_json::from_str(def.default_json?).ok()?;
    let mut values = serde_json::Map::new();
    values.insert(key.to_string(), value);
    let resolved = engine::presets::resolve_preset_chain(&[engine::api::RawPreset {
        name: "défauts".into(),
        inherits: None,
        values,
    }])
    .ok()?;
    resolved.0.get(key).cloned()
}

/// Ajoute les valeurs du filament `index` aux vecteurs par extrudeur. Chaque
/// preset filament apporte ses propres valeurs : elles ne doivent jamais
/// remplacer celles des filaments déjà fusionnés. Comme dans
/// `PresetBundle::construct_full_config`, les scalaires viennent du premier
/// filament en mode multi-filament ; les vecteurs sont assemblés dans l'ordre.
fn merge_filament_config(
    merged: &mut DynamicPrintConfig,
    filament: DynamicPrintConfig,
    index: usize,
) {
    for (key, incoming) in filament.0 {
        if index > 0 {
            if let Some(existing) = merged.0.get_mut(&key) {
                if append_extruder_values(existing, &incoming) {
                    continue;
                }
                // Orca conserve la valeur scalaire du premier filament. Les
                // variantes Floats/Bools et leurs formes nullable ci-dessus.
                if !is_extruder_vector(&incoming) {
                    continue;
                }
            }
        }
        merged.0.insert(key, incoming);
    }
}

fn is_extruder_vector(value: &ConfigValue) -> bool {
    matches!(
        value,
        ConfigValue::Bools(_)
            | ConfigValue::BoolsNullable(_)
            | ConfigValue::Ints(_)
            | ConfigValue::Floats(_)
            | ConfigValue::FloatsNullable(_)
            | ConfigValue::Strings(_)
    )
}

/// Renvoie `false` si les deux variantes diffèrent : l'appelant remplace alors
/// la valeur incohérente au lieu de mélanger deux types de paramètres.
fn append_extruder_values(existing: &mut ConfigValue, incoming: &ConfigValue) -> bool {
    match (existing, incoming) {
        (ConfigValue::Bools(out), ConfigValue::Bools(values)) => out.extend(values.iter().copied()),
        (existing @ ConfigValue::Bools(_), ConfigValue::BoolsNullable(values)) => {
            let mut converted = match existing {
                ConfigValue::Bools(out) => out.iter().map(|value| Some(*value)).collect::<Vec<_>>(),
                _ => unreachable!(),
            };
            converted.extend(values.iter().copied());
            *existing = ConfigValue::BoolsNullable(converted);
        }
        (ConfigValue::BoolsNullable(out), ConfigValue::Bools(values)) => {
            out.extend(values.iter().map(|value| Some(*value)))
        }
        (ConfigValue::BoolsNullable(out), ConfigValue::BoolsNullable(values)) => {
            out.extend(values.iter().copied())
        }
        (ConfigValue::Ints(out), ConfigValue::Ints(values)) => out.extend(values.iter().copied()),
        (ConfigValue::Floats(out), ConfigValue::Floats(values)) => {
            out.extend(values.iter().copied())
        }
        (existing @ ConfigValue::Floats(_), ConfigValue::FloatsNullable(values)) => {
            let mut converted = match existing {
                ConfigValue::Floats(out) => {
                    out.iter().map(|value| Some(*value)).collect::<Vec<_>>()
                }
                _ => unreachable!(),
            };
            converted.extend(values.iter().copied());
            *existing = ConfigValue::FloatsNullable(converted);
        }
        (ConfigValue::FloatsNullable(out), ConfigValue::Floats(values)) => {
            out.extend(values.iter().map(|value| Some(*value)))
        }
        (ConfigValue::FloatsNullable(out), ConfigValue::FloatsNullable(values)) => {
            out.extend(values.iter().copied())
        }
        (ConfigValue::Strings(out), ConfigValue::Strings(values)) => {
            out.extend(values.iter().cloned())
        }
        _ => return false,
    }
    true
}

fn parse_preset_id(raw: &str) -> ApiResult<PresetId> {
    uuid::Uuid::parse_str(raw)
        .map(PresetId)
        .map_err(|_| ApiError::validation("preset actif invalide", json!({ "id": raw })))
}

// --- Lecture tolérante du document scène -------------------------------------

/// Scripts de post-traitement (`post_process`) : jamais exécutés côté serveur
/// (exécution de commandes arbitraires sur un serveur partagé). Orca les lance
/// seulement depuis la GUI, après l'export (BackgroundSlicingProcess →
/// run_post_process_scripts) ; le CLI et libslic3r ne les exécutent pas. Le
/// réglage est conservé (G-code, projet, export 3MF) ; avertissement explicite.
pub(crate) fn post_process_warning(
    config: &engine::api::DynamicPrintConfig,
) -> Option<SliceWarning> {
    let scripts: Vec<String> = match config.get("post_process") {
        Some(ConfigValue::Strings(v)) => v
            .iter()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
        Some(ConfigValue::String(s)) if !s.trim().is_empty() => vec![s.trim().to_string()],
        _ => Vec::new(),
    };
    (!scripts.is_empty()).then(|| SliceWarning {
        key: "post_process".into(),
        message: format!(
            "Scripts de post-traitement non exécutés sur le serveur ({}) : ils sont conservés dans le              projet et le G-code ; exécutez-les localement sur le fichier téléchargé.",
            scripts.join(" ; ")
        ),
    })
}

/// Réglages du PROJET (`scene.projectConfig`, pas des presets) appliqués sur la
/// config résolue, comme la config projet d'Orca par-dessus les presets
/// (`PresetBundle::full_fff_config` : `filament_colour` y est une option projet,
/// `flush_volumes_matrix` / `flush_multiplier` aussi) :
/// - `filament_colour` : entrées non vides seulement (`""` = couleur du preset) ;
/// - `flush_volumes_matrix` : matrice N×N aplatie (mm³) ;
/// - `flush_multiplier` : nombre (ou tableau par buse).
///
/// Sans matrice explicite, le moteur la recalcule depuis les couleurs
/// (FlushVolCalculator, comme le CLI quand les couleurs changent).
pub(crate) fn apply_project_config(config: &mut engine::api::DynamicPrintConfig, scene: &Value) {
    // Projet de calibration (POST …/calibration) : presets édités par Orca
    // (Plater::calib_*), posés par-dessus les presets résolus.
    if let Some(overrides) = scene.get("calibrationOverrides").filter(|v| v.is_object()) {
        match crate::slicing::config_from_snapshot(overrides) {
            Ok(values) => config.0.extend(values.0),
            Err(error) => tracing::warn!(%error, "surcharges de calibration ignorées"),
        }
    }
    let Some(project) = scene.get("projectConfig").and_then(Value::as_object) else {
        return;
    };
    if let Some(colours) = project.get("filament_colour").and_then(Value::as_array) {
        let mut current = match config.get("filament_colour") {
            Some(ConfigValue::Strings(v)) => v.clone(),
            _ => Vec::new(),
        };
        for (index, colour) in colours.iter().enumerate() {
            let Some(colour) = colour.as_str().map(str::trim).filter(|c| !c.is_empty()) else {
                continue;
            };
            if current.len() <= index {
                current.resize(index + 1, "#FFFFFF".into());
            }
            current[index] = colour.to_string();
        }
        if !current.is_empty() {
            config
                .0
                .insert("filament_colour".into(), ConfigValue::Strings(current));
        }
    }
    let floats = |value: &Value| -> Option<Vec<f64>> {
        match value {
            Value::Number(n) => n.as_f64().map(|f| vec![f]),
            Value::String(s) => s.trim().parse().ok().map(|f| vec![f]),
            Value::Array(items) => items
                .iter()
                .map(|v| {
                    v.as_f64()
                        .or_else(|| v.as_str().and_then(|s| s.trim().parse().ok()))
                })
                .collect(),
            _ => None,
        }
        .filter(|v: &Vec<f64>| !v.is_empty() && v.iter().all(|f| f.is_finite() && *f >= 0.0))
    };
    // Imprimantes à plusieurs buses : Orca stocke une matrice N×N et un
    // multiplicateur PAR BUSE (flush_volumes_matrix = N²×buses). Une matrice
    // N×N et un multiplicateur unique venant du projet valent pour toutes les
    // buses (répétés), au lieu de ne viser que la première.
    let nozzles = match config.get("nozzle_diameter") {
        Some(ConfigValue::Floats(v)) => v.len().max(1),
        _ => 1,
    };
    if let Some(matrix) = project.get("flush_volumes_matrix").and_then(floats) {
        let n = (matrix.len() as f64).sqrt().round() as usize;
        if n * n == matrix.len() {
            let full: Vec<f64> = matrix
                .iter()
                .copied()
                .cycle()
                .take(matrix.len() * nozzles)
                .collect();
            config
                .0
                .insert("flush_volumes_matrix".into(), ConfigValue::Floats(full));
        } else if nozzles > 1 && matrix.len() % nozzles == 0 {
            let per = matrix.len() / nozzles;
            let n = (per as f64).sqrt().round() as usize;
            if n * n == per {
                config
                    .0
                    .insert("flush_volumes_matrix".into(), ConfigValue::Floats(matrix));
            }
        }
    }
    if let Some(multiplier) = project.get("flush_multiplier").and_then(floats) {
        let full = if multiplier.len() == 1 {
            vec![multiplier[0]; nozzles]
        } else {
            multiplier
        };
        config
            .0
            .insert("flush_multiplier".into(), ConfigValue::Floats(full));
    }
}

/// Liste des `objectIds` par plateau (deux formes de scène tolérées : tableau de
/// plateaux, ou document `{ plates: [...], activeId }`). Vide si aucun plateau.
/// Snapshot `resolved` + réglages du plateau `index` (`plates[index].settings`,
/// clés Orca par plateau uniquement) + type de plaque du plateau.
///
/// Lot 44 (audit 43 B-1) : Orca tranche un plateau avec
/// `PartPlate::get_bed_type()` (réglage `curr_bed_type` du plateau, ou celui
/// du projet si le plateau est « par défaut ») ; `bed_temperature` en découle
/// (`*_plate_temp*` du type). Le type choisi dans la barre du plateau
/// (`plateType`) est donc appliqué comme `curr_bed_type` — exactement ce que
/// l'export 3MF écrit dans `bed_type` du plateau (`plate_settings_config`) ;
/// une surcharge explicite `settings.curr_bed_type` (dialogue « Réglages du
/// plateau ») prime ; libellé absent ou inconnu (`Default Plate`) → valeur du
/// projet (snapshot inchangé).
pub(crate) fn with_plate_settings(resolved: &Value, scene: &Value, index: i64) -> Value {
    let plates = match scene.get("plates") {
        Some(Value::Array(plates)) => Some(plates),
        Some(Value::Object(document)) => document.get("plates").and_then(Value::as_array),
        _ => None,
    };
    let Some(plate) = plates.and_then(|plates| plates.get(index.max(0) as usize)) else {
        return resolved.clone();
    };
    let overrides = super::export::plate_settings_config(plate);
    if overrides.0.is_empty() {
        return resolved.clone();
    }
    let mut out = resolved.clone();
    if let Value::Object(map) = &mut out {
        map.extend(presets::config_to_json(&overrides));
    }
    out
}

fn plate_object_ids(scene: &Value) -> Vec<Vec<String>> {
    let plates = match scene.get("plates") {
        Some(Value::Array(a)) => a.clone(),
        Some(Value::Object(o)) => o
            .get("plates")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default(),
        _ => Vec::new(),
    };
    plates
        .iter()
        .map(|p| {
            p.get("objectIds")
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default()
        })
        .collect()
}

/// Tous les identifiants d'objets de la scène (repli quand aucun plateau).
fn all_object_ids(scene: &Value) -> Vec<String> {
    scene
        .get("objects")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|o| o.get("id").and_then(Value::as_str).map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

/// `modelId` (objets et volumes maillés) de la scène absents de `known`,
/// triés et dédoublonnés.
pub(crate) fn unknown_scene_models(scene: &Value, known: &HashSet<String>) -> Vec<String> {
    let mut unknown = std::collections::BTreeSet::new();
    for object in scene
        .get("objects")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let volumes = object
            .get("volumes")
            .and_then(Value::as_array)
            .into_iter()
            .flatten();
        for id in std::iter::once(object)
            .chain(volumes)
            .filter_map(|node| node.get("modelId").and_then(Value::as_str))
        {
            if !known.contains(id) {
                unknown.insert(id.to_string());
            }
        }
    }
    unknown.into_iter().collect()
}

/// Identifiants d'objets marqués hors plateau (`off_bed: true`) par le client.
/// Lot 43 : seulement un repli quand le serveur ne peut pas classer l'objet
/// (géométrie inconnue) ; sinon l'état calculé (placement.rs) fait foi.
pub(crate) fn off_bed_ids(scene: &Value) -> HashSet<String> {
    scene
        .get("objects")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter(|o| o.get("off_bed").and_then(Value::as_bool).unwrap_or(false))
                .filter_map(|o| o.get("id").and_then(Value::as_str).map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn filament(material: &str, temperature: i64, retraction: Option<f64>) -> DynamicPrintConfig {
        let mut config = DynamicPrintConfig::new();
        config.0.insert(
            "filament_type".into(),
            ConfigValue::Strings(vec![material.into()]),
        );
        config.0.insert(
            "nozzle_temperature".into(),
            ConfigValue::Ints(vec![temperature]),
        );
        config.0.insert(
            "filament_retraction_length".into(),
            ConfigValue::FloatsNullable(vec![retraction]),
        );
        config
    }

    #[test]
    fn plate_settings_override_the_snapshot_for_their_plate_only() {
        let resolved = json!({ "print_sequence": "by layer", "layer_height": 0.2 });
        let scene = json!({ "plates": { "plates": [
            { "id": "a", "objectIds": [] },
            { "id": "b", "objectIds": [], "plateType": "Engineering Plate",
              "settings": { "print_sequence": "by object", "layer_height": "0.3",
                            "first_layer_sequence_choice": "Customize",
                            "other_layers_sequence_choice": "Customize" } }
        ] } });
        assert_eq!(super::with_plate_settings(&resolved, &scene, 0), resolved);
        let b = super::with_plate_settings(&resolved, &scene, 1);
        assert_eq!(b["print_sequence"], json!("by object"));
        assert_eq!(b["first_layer_sequence_choice"], json!("Customize"));
        assert_eq!(b["other_layers_sequence_choice"], json!("Customize"));
        // Clé hors PLATE_SETTING_KEYS ignorée.
        assert_eq!(b["layer_height"], json!(0.2));
        // Lot 44 (B-1) : le type de plaque du plateau est tranché (Orca
        // PartPlate::get_bed_type), comme l'export 3MF l'écrit.
        assert_eq!(b["curr_bed_type"], json!("Engineering Plate"));
    }

    #[test]
    fn plate_type_sets_the_sliced_bed_type_unless_overridden_or_default() {
        let resolved = json!({ "curr_bed_type": "Cool Plate", "layer_height": 0.2 });
        let scene = json!({ "plates": { "plates": [
            { "id": "a", "objectIds": [], "plateType": "Textured PEI Plate" },
            { "id": "b", "objectIds": [], "plateType": "Smooth High Temp Plate",
              "settings": { "curr_bed_type": "Engineering Plate" } },
            { "id": "c", "objectIds": [], "plateType": "Default Plate" },
            { "id": "d", "objectIds": [] }
        ] } });
        let bed = |i| super::with_plate_settings(&resolved, &scene, i)["curr_bed_type"].clone();
        assert_eq!(bed(0), json!("Textured PEI Plate"));
        // Surcharge explicite du dialogue « Réglages du plateau ».
        assert_eq!(bed(1), json!("Engineering Plate"));
        // Plateau « par défaut » : type du projet.
        assert_eq!(bed(2), json!("Cool Plate"));
        assert_eq!(bed(3), json!("Cool Plate"));
    }

    #[test]
    fn variant_options_follow_the_filament_variants_and_overrides_hit_their_filament() {
        // Lot 44 (audit 43 I3-bis) : Bambu PLA Basic (Standard / High Flow).
        let filament = || {
            let mut config = DynamicPrintConfig::new();
            config.0.insert(
                "filament_extruder_variant".into(),
                ConfigValue::Strings(vec![
                    "Direct Drive Standard".into(),
                    "Direct Drive High Flow".into(),
                ]),
            );
            config.0.insert(
                "during_print_exhaust_fan_speed".into(),
                ConfigValue::Ints(vec![70]),
            );
            config.0.insert(
                "nozzle_temperature".into(),
                ConfigValue::Ints(vec![220, 230]),
            );
            config
        };
        let mut merged = DynamicPrintConfig::new();
        merge_filament_configs(&mut merged, vec![filament(), filament()]);
        assert_eq!(
            merged.get("during_print_exhaust_fan_speed"),
            Some(&ConfigValue::Ints(vec![70, 70, 70, 70]))
        );
        assert_eq!(
            merged.get("filament_self_index"),
            Some(&ConfigValue::Ints(vec![1, 1, 2, 2]))
        );
        // Surcharge du filament 2 : ses deux cases, pas la case 1 du filament 1.
        set_filament_value(
            &mut merged,
            1,
            "nozzle_temperature".into(),
            ConfigValue::Ints(vec![240, 250]),
        );
        assert_eq!(
            merged.get("nozzle_temperature"),
            Some(&ConfigValue::Ints(vec![220, 230, 240, 250]))
        );
        set_filament_value(
            &mut merged,
            0,
            "during_print_exhaust_fan_speed".into(),
            ConfigValue::Ints(vec![50]),
        );
        assert_eq!(
            merged.get("during_print_exhaust_fan_speed"),
            Some(&ConfigValue::Ints(vec![50, 50, 70, 70]))
        );
    }

    #[test]
    fn draft_overrides_process_printer_and_one_filament() {
        let mut merged = DynamicPrintConfig::new();
        merge_filament_config(&mut merged, filament("PLA", 205, Some(0.8)), 0);
        merge_filament_config(&mut merged, filament("PETG", 235, None), 1);
        merged
            .0
            .insert("layer_height".into(), ConfigValue::Float(0.2));
        let draft: crate::http::dto::SliceDraft = serde_json::from_value(json!({
            "process": { "layer_height": "0.28" },
            "printer": { "nozzle_diameter": ["0.6"] },
            "filaments": [null, { "nozzle_temperature": ["250"], "filament_type": ["ASA"] }]
        }))
        .unwrap();
        super::apply_draft(&mut merged, &draft).unwrap();
        assert_eq!(merged.get("layer_height"), Some(&ConfigValue::Float(0.28)));
        assert_eq!(
            merged.get("nozzle_diameter"),
            Some(&ConfigValue::Floats(vec![0.6]))
        );
        assert_eq!(
            merged.get("nozzle_temperature"),
            Some(&ConfigValue::Ints(vec![205, 250]))
        );
        assert_eq!(
            merged.get("filament_type"),
            Some(&ConfigValue::Strings(vec!["PLA".into(), "ASA".into()]))
        );
        let bad: crate::http::dto::SliceDraft =
            serde_json::from_value(json!({ "process": { "layer_height": "abc" } })).unwrap();
        assert!(super::apply_draft(&mut merged, &bad).is_err());
    }

    /// filament_self_index comme PresetBundle::construct_full_config (lot 39) :
    /// une entrée par (filament × variante), n° du filament ; mono-filament → 1.
    #[test]
    fn filament_self_index_follows_each_filament_variants() {
        let variants = |names: &[&str]| {
            let mut c = DynamicPrintConfig::new();
            c.0.insert(
                "filament_extruder_variant".into(),
                ConfigValue::Strings(names.iter().map(|s| s.to_string()).collect()),
            );
            c.0.insert(
                "nozzle_temperature".into(),
                ConfigValue::Ints(vec![220; names.len()]),
            );
            c
        };
        let both = ["Direct Drive Standard", "Direct Drive High Flow"];
        let mut merged = DynamicPrintConfig::new();
        merge_filament_configs(&mut merged, vec![variants(&both), variants(&both)]);
        assert_eq!(
            merged.get("filament_self_index"),
            Some(&ConfigValue::Ints(vec![1, 1, 2, 2]))
        );
        // Filament sans la clé : défaut d'Orca (une variante Standard).
        let mut merged = DynamicPrintConfig::new();
        merge_filament_configs(
            &mut merged,
            vec![variants(&both), DynamicPrintConfig::new(), variants(&both)],
        );
        assert_eq!(
            merged.get("filament_self_index"),
            Some(&ConfigValue::Ints(vec![1, 1, 2, 3, 3]))
        );
        let mut merged = DynamicPrintConfig::new();
        merge_filament_configs(&mut merged, vec![variants(&both)]);
        assert_eq!(
            merged.get("filament_self_index"),
            Some(&ConfigValue::Ints(vec![1, 1]))
        );
        // Aucune variante déclarée : clé non posée (défaut du moteur).
        let mut merged = DynamicPrintConfig::new();
        merge_filament_configs(&mut merged, vec![DynamicPrintConfig::new()]);
        assert_eq!(merged.get("filament_self_index"), None);
    }

    #[test]
    fn filament_compatibility_uses_orca_ranges_and_selected_filaments() {
        use engine::api::filament_compat::FilamentCompatibility as C;
        let mut merged = DynamicPrintConfig::new();
        merge_filament_config(&mut merged, filament("PLA", 220, None), 0);
        merge_filament_config(&mut merged, filament("PC", 270, None), 1);
        merge_filament_config(&mut merged, filament("PETG", 240, None), 2);
        assert_eq!(super::filament_compatibility(&merged, &[]), C::HighLowMixed);
        assert_eq!(
            super::filament_compatibility(&merged, &[0, 2]),
            C::Compatible
        );
        let w = super::filament_compatibility_warning(&merged, false).unwrap();
        assert_eq!(w.key, "filament_compatibility");
        assert!(w.message.contains("Remove mixed temperature restriction"));
        assert!(!super::filament_compatibility_warning(&merged, true)
            .unwrap()
            .message
            .contains("Preferences"));
    }

    #[test]
    fn plate_filament_map_reaches_the_slice_snapshot() {
        let resolved = json!({ "filament_map_mode": "Auto For Flush", "filament_map": [1, 1] });
        let scene = json!({ "plates": { "plates": [
            { "id": "a", "objectIds": [], "settings": { "filament_map_mode": "Manual", "filament_map": ["2", "1"] } }
        ] } });
        let a = super::with_plate_settings(&resolved, &scene, 0);
        assert_eq!(a["filament_map_mode"], json!("Manual"));
        assert_eq!(a["filament_map"], json!([2, 1]));
    }

    #[test]
    fn merges_each_filament_vector_by_extruder() {
        let mut merged = DynamicPrintConfig::new();
        // Une valeur machine éventuelle est remplacée par la liste des
        // filaments, pas conservée comme un extrudeur supplémentaire.
        merged.0.insert(
            "nozzle_temperature".into(),
            ConfigValue::Ints(vec![180, 180]),
        );
        merge_filament_config(&mut merged, filament("PLA", 205, Some(0.8)), 0);
        merge_filament_config(&mut merged, filament("PETG", 235, None), 1);

        assert_eq!(
            merged.get("filament_type"),
            Some(&ConfigValue::Strings(vec!["PLA".into(), "PETG".into()]))
        );
        assert_eq!(
            merged.get("nozzle_temperature"),
            Some(&ConfigValue::Ints(vec![205, 235]))
        );
        assert_eq!(
            merged.get("filament_retraction_length"),
            Some(&ConfigValue::FloatsNullable(vec![Some(0.8), None]))
        );
    }

    #[test]
    fn orca_string_arrays_use_semicolons_not_commas() {
        let chain = [RawPreset {
            name: "deux filaments".into(),
            inherits: None,
            values: json!({"filament_type": ["PLA", "PETG"]})
                .as_object()
                .unwrap()
                .clone(),
        }];
        let config = resolve_preset_chain_orca(&chain).unwrap();
        let value = config.get("filament_type").unwrap();
        assert_eq!(
            value,
            &ConfigValue::Strings(vec!["PLA".into(), "PETG".into()])
        );
        assert_eq!(
            engine::params::orca_values::serialize_orca_value(value),
            "\"PLA\";\"PETG\""
        );
    }

    /// Fusion multi-filament comme Orca : presets aplatis de la scène
    /// engine/tests/fixtures/multi_filament (Voron 2.4 350 + 0.20 Standard +
    /// 2 × Generic PLA) fusionnés comme `resolve_active_config`, comparés à la
    /// config que le CLI Orca a écrite dans two_objects.3mf
    /// (project_settings.config) pour toutes les clés par filament et
    /// multi-matériau (+ toutes les clés que les deux définissent).
    #[test]
    fn multi_filament_merge_matches_orca_project_settings() {
        use std::io::Read;
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../engine/tests/fixtures/multi_filament");
        let resolve = |name: &str| {
            let values: serde_json::Map<String, Value> = serde_json::from_slice(
                &std::fs::read(dir.join(format!("presets/{name}.json"))).unwrap(),
            )
            .unwrap();
            engine::presets::resolve_preset_chain(&[engine::api::RawPreset {
                name: name.into(),
                inherits: None,
                values,
            }])
            .unwrap()
        };
        let mut merged = DynamicPrintConfig::new();
        for name in ["printer", "process"] {
            merged.0.extend(resolve(name).0);
        }
        merge_filament_configs(&mut merged, vec![resolve("filament"), resolve("filament2")]);

        let mut zip =
            zip::ZipArchive::new(std::fs::File::open(dir.join("two_objects.3mf")).unwrap())
                .unwrap();
        let mut text = String::new();
        zip.by_name("Metadata/project_settings.config")
            .unwrap()
            .read_to_string(&mut text)
            .unwrap();
        let orca_values: serde_json::Map<String, Value> = serde_json::from_str(&text).unwrap();
        let orca = engine::presets::resolve_preset_chain(&[engine::api::RawPreset {
            name: "orca".into(),
            inherits: None,
            values: orca_values,
        }])
        .unwrap();
        let mut diffs = Vec::new();
        for (key, ours) in &merged.0 {
            if matches!(
                key.as_str(),
                "inherits" | "name" | "from" | "compatible_printers" | "compatible_prints"
            ) || key.ends_with("_settings_id")
            {
                continue;
            }
            if let Some(theirs) = orca.0.get(key) {
                if theirs != ours {
                    diffs.push(format!("{key} : moteur {ours:?} / Orca {theirs:?}"));
                }
            }
        }
        assert!(
            diffs.is_empty(),
            "{} écarts :\n{}",
            diffs.len(),
            diffs.join("\n")
        );
    }

    /// Outil (ignoré) : fusionne les presets aplatis d'un dossier
    /// (MF_PRESETS_DIR : printer/process/filament/filament2.json) comme
    /// `resolve_active_config` et écrit la config (DynamicPrintConfig JSON)
    /// dans MF_MERGED_OUT — reproduction des tranchages de production.
    #[test]
    #[ignore = "outil de reproduction (MF_PRESETS_DIR / MF_MERGED_OUT)"]
    fn dump_merged_presets() {
        // MF_SYSTEM = "vendor|machine|process|filament" : presets SYSTÈME résolus
        // comme le backend (chaîne d'héritage des profils Orca importés), deux
        // filaments identiques, couleur du 2e via scene.projectConfig.
        let spec = std::env::var("MF_SYSTEM").unwrap();
        let parts: Vec<&str> = spec.split('|').collect();
        let profiles = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../vendor/OrcaSlicer/resources/profiles");
        let outcome = engine::presets::import_profiles(&profiles).unwrap();
        let index = engine::presets::PresetIndex::new(&outcome.presets);
        use engine::presets::PresetKind;
        // Vendeur donné, sinon celui du preset portant ce nom (bibliothèque de
        // filaments Orca : « Generic PLA @System »).
        let get = |kind, name: &str| {
            index.resolve(parts[0], kind, name).unwrap_or_else(|_| {
                let vendor = outcome
                    .presets
                    .iter()
                    .find(|p| p.kind == kind && p.name == name)
                    .map(|p| p.vendor.clone())
                    .expect("preset introuvable");
                index.resolve(&vendor, kind, name).unwrap()
            })
        };
        let mut merged = DynamicPrintConfig::new();
        merged.0.extend(get(PresetKind::Machine, parts[1]).0);
        merged.0.extend(get(PresetKind::Process, parts[2]).0);
        let filament = get(PresetKind::Filament, parts[3]);
        merge_filament_configs(&mut merged, vec![filament.clone(), filament]);
        let mut extra = std::env::var("MF_PROCESS_EXTRA").unwrap_or_default();
        if extra.is_empty() {
            extra = "{}".into();
        }
        let extra: serde_json::Map<String, Value> = serde_json::from_str(&extra).unwrap();
        let extra = engine::presets::resolve_preset_chain(&[engine::api::RawPreset {
            name: "extra".into(),
            inherits: None,
            values: extra,
        }])
        .unwrap();
        merged.0.extend(extra.0);
        apply_project_config(
            &mut merged,
            &serde_json::json!({ "projectConfig": std::env::var("MF_PROJECT_CONFIG")
                .ok()
                .and_then(|s| serde_json::from_str::<Value>(&s).ok())
                .unwrap_or_else(|| serde_json::json!({ "filament_colour": ["", "#FF0000"] })) }),
        );
        std::fs::write(
            std::env::var("MF_MERGED_OUT").unwrap(),
            serde_json::to_vec(&merged).unwrap(),
        )
        .unwrap();
    }

    /// scene.projectConfig appliqué par-dessus les presets (couleurs, matrice,
    /// multiplicateur) ; entrées vides et matrices non carrées ignorées.
    /// Plusieurs buses (Prusa XL 5T) : matrice N×N et multiplicateur unique du
    /// projet répétés pour chaque buse.
    #[test]
    fn project_flush_settings_apply_to_every_nozzle() {
        let mut config = DynamicPrintConfig::new();
        config
            .0
            .insert("nozzle_diameter".into(), ConfigValue::Floats(vec![0.4; 5]));
        let scene = serde_json::json!({ "projectConfig": {
            "flush_volumes_matrix": [0, 80, 560, 0],
            "flush_multiplier": 0.3
        }});
        apply_project_config(&mut config, &scene);
        assert_eq!(
            config.get("flush_multiplier"),
            Some(&ConfigValue::Floats(vec![0.3; 5]))
        );
        let Some(ConfigValue::Floats(m)) = config.get("flush_volumes_matrix") else {
            panic!()
        };
        assert_eq!(m.len(), 20);
        assert_eq!(&m[16..], &[0.0, 80.0, 560.0, 0.0]);
    }

    /// Chaîne du dialogue Ramming d'Orca : aller-retour exact preset JSON →
    /// config moteur → JSON (sauvegarde de preset, snapshot du job).
    #[test]
    fn ramming_parameters_round_trip_exactly() {
        let curve = "120 100 6.6 6.8 7.2 7.6 7.9 8.2 8.7 9.4 9.9 10.0| 0.05 6.6 0.45 6.8 0.95 7.8 1.45 8.3 1.95 9.7 2.45 10 2.95 7.6 3.45 7.6 3.95 7.6 4.45 7.6 4.95 7.6";
        let mut values = serde_json::Map::new();
        values.insert(
            "filament_ramming_parameters".into(),
            serde_json::json!([curve, curve]),
        );
        let config = engine::presets::resolve_preset_chain(&[engine::api::RawPreset {
            name: "f".into(),
            inherits: None,
            values,
        }])
        .unwrap();
        assert_eq!(
            config.get("filament_ramming_parameters"),
            Some(&ConfigValue::Strings(vec![curve.into(), curve.into()]))
        );
        let json = presets::config_to_json(&config);
        let back = crate::slicing::config_from_snapshot(&Value::Object(json)).unwrap();
        assert_eq!(
            back.get("filament_ramming_parameters"),
            config.get("filament_ramming_parameters")
        );
    }

    #[test]
    fn post_process_scripts_are_reported_not_run() {
        let mut config = DynamicPrintConfig::new();
        assert!(post_process_warning(&config).is_none());
        config
            .0
            .insert("post_process".into(), ConfigValue::Strings(vec!["".into()]));
        assert!(post_process_warning(&config).is_none());
        config.0.insert(
            "post_process".into(),
            ConfigValue::Strings(vec!["/usr/bin/python3 fix.py".into()]),
        );
        let w = post_process_warning(&config).expect("avertissement");
        assert_eq!(w.key, "post_process");
        assert!(w.message.contains("fix.py") && w.message.contains("non exécutés"));
    }

    #[test]
    fn project_config_overrides_presets() {
        let mut config = DynamicPrintConfig::new();
        config.0.insert(
            "filament_colour".into(),
            ConfigValue::Strings(vec!["#F2754E".into(), "#F2754E".into()]),
        );
        config
            .0
            .insert("flush_multiplier".into(), ConfigValue::Floats(vec![1.0]));
        let scene = serde_json::json!({ "projectConfig": {
            "filament_colour": ["", "#00A0FF"],
            "flush_volumes_matrix": [0, 480, 220, 0],
            "flush_multiplier": 1.5
        }});
        apply_project_config(&mut config, &scene);
        assert_eq!(
            config.get("filament_colour"),
            Some(&ConfigValue::Strings(vec![
                "#F2754E".into(),
                "#00A0FF".into()
            ]))
        );
        assert_eq!(
            config.get("flush_volumes_matrix"),
            Some(&ConfigValue::Floats(vec![0.0, 480.0, 220.0, 0.0]))
        );
        assert_eq!(
            config.get("flush_multiplier"),
            Some(&ConfigValue::Floats(vec![1.5]))
        );
        // Matrice non carrée : ignorée ; pas de projectConfig : inchangé.
        apply_project_config(
            &mut config,
            &serde_json::json!({ "projectConfig": { "flush_volumes_matrix": [1, 2, 3] } }),
        );
        assert_eq!(
            config.get("flush_volumes_matrix"),
            Some(&ConfigValue::Floats(vec![0.0, 480.0, 220.0, 0.0]))
        );
        let before = config.0.clone();
        apply_project_config(&mut config, &serde_json::json!({}));
        assert_eq!(config.0, before);
    }

    #[test]
    fn multi_filament_scalars_come_from_the_first_filament() {
        let mut merged = DynamicPrintConfig::new();
        let mut first = DynamicPrintConfig::new();
        first
            .0
            .insert("filament_extruder_id".into(), ConfigValue::Int(1));
        let mut second = DynamicPrintConfig::new();
        second
            .0
            .insert("filament_extruder_id".into(), ConfigValue::Int(2));
        merge_filament_config(&mut merged, first, 0);
        merge_filament_config(&mut merged, second, 1);
        assert_eq!(
            merged.get("filament_extruder_id"),
            Some(&ConfigValue::Int(1))
        );
    }

    #[test]
    fn merges_nullable_bools_without_shifting_filament_indices() {
        let mut merged = DynamicPrintConfig::new();
        let mut first = DynamicPrintConfig::new();
        first
            .0
            .insert("filament_wipe".into(), ConfigValue::Bools(vec![true]));
        let mut second = DynamicPrintConfig::new();
        second.0.insert(
            "filament_wipe".into(),
            ConfigValue::BoolsNullable(vec![None]),
        );
        let mut third = DynamicPrintConfig::new();
        third
            .0
            .insert("filament_wipe".into(), ConfigValue::Bools(vec![false]));

        merge_filament_config(&mut merged, first, 0);
        merge_filament_config(&mut merged, second, 1);
        merge_filament_config(&mut merged, third, 2);

        assert_eq!(
            merged.get("filament_wipe"),
            Some(&ConfigValue::BoolsNullable(vec![
                Some(true),
                None,
                Some(false)
            ]))
        );
    }
}
