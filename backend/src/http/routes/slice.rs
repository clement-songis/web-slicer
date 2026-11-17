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
use crate::http::dto::{JobResponse, SliceRequest, SliceResponse, SliceWarning};
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

    let plates = plate_object_ids(&project.scene);
    let plate_count = plates.len().max(1);

    // Cible : un plateau précis (borné) ou tous les plateaux préparés.
    let plate_ids = |t: i64| {
        plates
            .get(t as usize)
            .cloned()
            .unwrap_or_else(|| all_object_ids(&project.scene))
    };
    let targets: Vec<i64> = match body.plate_index {
        Some(n) => {
            if n < 0 || n as usize >= plate_count {
                return Err(ApiError::validation(
                    "plate_index hors limites",
                    json!({ "plate_index": n, "plate_count": plate_count }),
                ));
            }
            vec![n]
        }
        // « Slice all » d'Orca : un plateau vide n'a rien à trancher et est
        // sauté (Plater::priv::on_process_completed → plateau suivant), seul
        // un projet sans aucun objet est refusé.
        None => {
            let non_empty: Vec<i64> = (0..plate_count as i64)
                .filter(|&t| !plate_ids(t).is_empty())
                .collect();
            if non_empty.is_empty() {
                return Err(ApiError::validation(
                    "plateau vide (aucun objet à trancher)",
                    json!({ "plate_index": 0 }),
                ));
            }
            non_empty
        }
    };

    // Garde avant slice : chaque plateau ciblé doit contenir des objets, tous
    // sur le plateau (FR-032, « hors plateau → erreur avant slice »).
    let off_bed = off_bed_ids(&project.scene);
    for &t in &targets {
        let ids = plate_ids(t);
        if ids.is_empty() {
            return Err(ApiError::validation(
                "plateau vide (aucun objet à trancher)",
                json!({ "plate_index": t }),
            ));
        }
        if ids.iter().any(|id| off_bed.contains(id)) {
            return Err(ApiError::validation(
                "objet hors plateau : repositionner avant de trancher",
                json!({ "plate_index": t }),
            ));
        }
    }

    // Presets actifs résolus (figés) + avertissements moteur, puis réglages du
    // projet (scene.projectConfig : couleurs, matrice/multiplicateur de purge)
    // par-dessus, comme Orca applique sa config projet sur les presets.
    let (mut merged, mut warnings) =
        resolve_active_config(&state, user.id, &project.active_presets).await?;
    apply_project_config(&mut merged, &project.scene);
    if let Some(warning) = post_process_warning(&merged) {
        warnings.push(warning);
    }
    let resolved = Value::Object(presets::config_to_json(&merged));

    let mut jobs = Vec::with_capacity(targets.len());
    for &t in &targets {
        // Réglages par plateau (Orca « Plate settings » : type de plaque,
        // séquence d'impression, ordre de la 1re couche…) figés dans le
        // snapshot de ce plateau, par-dessus les presets.
        let resolved = with_plate_settings(&resolved, &project.scene, t);
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
                project.scene.clone(),
            )
            .await?;
        jobs.push(JobResponse::from(job));
    }

    Ok((StatusCode::CREATED, Json(SliceResponse { jobs, warnings })))
}

/// Résout les presets actifs (`{printer, process, filaments[]}`) en une config
/// unique figée ; les valeurs hors bornes sont clampées et signalées (FR-032).
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
        merge_filament_config(merged, filament, index);
    }
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
/// Snapshot `resolved` + surcharges explicites du plateau `index`
/// (`plates[index].settings`, clés Orca par plateau uniquement).
fn with_plate_settings(resolved: &Value, scene: &Value, index: i64) -> Value {
    let plates = match scene.get("plates") {
        Some(Value::Array(plates)) => Some(plates),
        Some(Value::Object(document)) => document.get("plates").and_then(Value::as_array),
        _ => None,
    };
    let Some(plate) = plates.and_then(|plates| plates.get(index.max(0) as usize)) else {
        return resolved.clone();
    };
    let overrides = super::export::plate_explicit_settings(plate);
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

/// Identifiants d'objets marqués hors plateau (`off_bed: true`) par le client.
fn off_bed_ids(scene: &Value) -> HashSet<String> {
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
        // Clé hors PLATE_SETTING_KEYS ignorée ; plateType seul ne fige rien.
        assert_eq!(b["layer_height"], json!(0.2));
        assert!(b.get("curr_bed_type").is_none());
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
