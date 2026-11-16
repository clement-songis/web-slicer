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

    // Presets actifs résolus (figés) + avertissements moteur.
    let (resolved, warnings) = resolve_active(&state, user.id, &project.active_presets).await?;

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
async fn resolve_active(
    state: &AppState,
    user: crate::domain::UserId,
    active: &Value,
) -> ApiResult<(Value, Vec<SliceWarning>)> {
    let (merged, warnings) = resolve_active_config(state, user, active).await?;
    Ok((Value::Object(presets::config_to_json(&merged)), warnings))
}

/// Comme [`resolve_active`], mais renvoie la `DynamicPrintConfig` typée (export
/// 3MF, T072) plutôt que sa projection JSON.
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
    for (index, id) in filament_ids.into_iter().enumerate() {
        let (preset, cfg) = resolve_one_preset(state, user, id).await?;
        filament_names.push(preset.name);
        merge_filament_config(&mut merged, cfg, index);
    }
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
