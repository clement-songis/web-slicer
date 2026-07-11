//! Endpoints presets (`/api/presets/*`) — FR-020..023. Liste filtrée par
//! compatibilité, valeurs brutes/résolues, CRUD utilisateur, import/export JSON
//! Orca avec conversion des clés legacy. Toute ressource utilisateur est scopée
//! par le compte connecté (un preset d'autrui → 404, SC-008).

use std::collections::{HashMap, HashSet};

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use engine::api::{ConfigValue, RawPreset};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::{Map, Value};
use uuid::Uuid;

use crate::domain::{Preset, PresetId, PresetKind, PresetOrigin, UserId};
use crate::http::dto::{
    CreatePresetRequest, ImportPresetRequest, PresetDetail, PresetSummary, ResolvedPreset,
    UpdatePresetRequest,
};
use crate::http::error::{ApiError, ApiResult};
use crate::http::extract::CurrentUser;
use crate::http::state::AppState;

/// Champs méta d'un profil Orca, exclus des valeurs de config à l'import.
const META_FIELDS: &[&str] = &[
    "type",
    "name",
    "inherits",
    "from",
    "setting_id",
    "instantiation",
    "version",
    "url",
    "description",
    "filament_id",
    "compatible_printers",
    "compatible_printers_condition",
];

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    /// Type de preset (obligatoire).
    kind: String,
    /// Imprimante active pour le filtre de compatibilité (optionnel).
    printer: Option<String>,
}

fn parse_kind(s: &str) -> ApiResult<PresetKind> {
    parse_enum("kind", s)
}

fn parse_enum<T: DeserializeOwned>(field: &str, value: &str) -> ApiResult<T> {
    serde_json::from_value(Value::String(value.to_string())).map_err(|_| {
        ApiError::validation(
            format!("valeur invalide pour « {field} »"),
            serde_json::json!({ "field": field, "value": value }),
        )
    })
}

fn parse_preset_id(raw: &str) -> ApiResult<PresetId> {
    Uuid::parse_str(raw)
        .map(PresetId)
        .map_err(|_| ApiError::not_found("Preset"))
}

/// Charge un preset et applique l'isolation : un preset **utilisateur** d'un
/// autre compte est traité comme inexistant (404, SC-008).
pub(crate) async fn load_visible(
    state: &AppState,
    user: UserId,
    id: PresetId,
) -> ApiResult<Preset> {
    let preset = state.storage.presets().get(id).await?;
    if preset.origin == PresetOrigin::User && preset.user_id != Some(user) {
        return Err(ApiError::not_found("Preset"));
    }
    Ok(preset)
}

/// `GET /api/presets?kind=&printer=` — système + utilisateur, filtre compat.
pub async fn list(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
) -> ApiResult<Json<Vec<PresetSummary>>> {
    let kind = parse_kind(&q.kind)?;
    let presets = state
        .storage
        .presets()
        .list_compatible(kind, q.printer.as_deref(), user.id)
        .await?;
    Ok(Json(presets.into_iter().map(Into::into).collect()))
}

/// `GET /api/presets/{id}` — valeurs brutes.
pub async fn get(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<PresetDetail>> {
    let id = parse_preset_id(&id)?;
    let preset = load_visible(&state, user.id, id).await?;
    Ok(Json(preset.into()))
}

/// `GET /api/presets/{id}/resolved` — chaîne d'héritage aplatie en valeurs
/// effectives (FR-020).
pub async fn resolved(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ResolvedPreset>> {
    let id = parse_preset_id(&id)?;
    let leaf = load_visible(&state, user.id, id).await?;
    let candidates = state
        .storage
        .presets()
        .list_by_kind(leaf.kind, user.id)
        .await?;
    let chain = build_chain(&candidates, &leaf);
    let config = engine::presets::resolve_preset_chain(&chain).map_err(|e| {
        tracing::error!(error = %e, "résolution de preset");
        ApiError::internal()
    })?;
    Ok(Json(ResolvedPreset {
        id: leaf.id.to_string(),
        values: Value::Object(config_to_json(&config)),
    }))
}

/// `POST /api/presets` — crée un preset utilisateur.
pub async fn create(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Json(body): Json<CreatePresetRequest>,
) -> ApiResult<(StatusCode, Json<PresetDetail>)> {
    let kind = parse_kind(&body.kind)?;
    let preset = new_user_preset(user.id, kind, &body.name, body.inherits, body.values)?;
    let created = state
        .storage
        .presets()
        .create_user_preset(user.id, preset)
        .await?;
    Ok((StatusCode::CREATED, Json(created.into())))
}

/// `PUT /api/presets/{id}` — renomme et/ou remplace les valeurs d'un preset user.
pub async fn update(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<UpdatePresetRequest>,
) -> ApiResult<Json<PresetDetail>> {
    let id = parse_preset_id(&id)?;
    let current = load_visible(&state, user.id, id).await?;
    if current.origin != PresetOrigin::User {
        return Err(ApiError::forbidden("Preset système en lecture seule"));
    }
    let name = body.name.unwrap_or(current.name);
    let values = body.values.unwrap_or(current.values);
    let updated = state
        .storage
        .presets()
        .update_user_preset(user.id, id, &name, values)
        .await?;
    Ok(Json(updated.into()))
}

/// `DELETE /api/presets/{id}` — supprime un preset utilisateur.
pub async fn delete(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    let id = parse_preset_id(&id)?;
    state
        .storage
        .presets()
        .delete_user_preset(user.id, id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /api/presets/import` — importe un profil JSON Orca, clés legacy
/// converties (FR-023), stocké comme preset utilisateur.
pub async fn import(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Json(body): Json<ImportPresetRequest>,
) -> ApiResult<(StatusCode, Json<PresetDetail>)> {
    let kind = parse_kind(&body.kind)?;
    let values = convert_legacy(&body.values);
    let preset = new_user_preset(
        user.id,
        kind,
        &body.name,
        body.inherits,
        Value::Object(values),
    )?;
    let created = state
        .storage
        .presets()
        .create_user_preset(user.id, preset)
        .await?;
    Ok((StatusCode::CREATED, Json(created.into())))
}

/// `GET /api/presets/{id}/export` — profil au format JSON Orca.
pub async fn export(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<Value>> {
    let id = parse_preset_id(&id)?;
    let preset = load_visible(&state, user.id, id).await?;
    let mut out = Map::new();
    out.insert("type".into(), Value::String(kind_str(preset.kind).into()));
    out.insert("name".into(), Value::String(preset.name));
    if let Some(parent) = preset.inherits {
        out.insert("inherits".into(), Value::String(parent));
    }
    if let Value::Object(values) = preset.values {
        for (k, v) in values {
            out.insert(k, v);
        }
    }
    Ok(Json(Value::Object(out)))
}

// --- Aides -------------------------------------------------------------------

fn kind_str(kind: PresetKind) -> &'static str {
    match kind {
        PresetKind::MachineModel => "machine_model",
        PresetKind::Machine => "machine",
        PresetKind::Filament => "filament",
        PresetKind::Process => "process",
    }
}

fn new_user_preset(
    user: UserId,
    kind: PresetKind,
    name: &str,
    inherits: Option<String>,
    values: Value,
) -> ApiResult<Preset> {
    if name.trim().is_empty() {
        return Err(ApiError::validation(
            "le nom du preset est requis",
            serde_json::json!({ "field": "name" }),
        ));
    }
    Ok(Preset {
        id: PresetId::new(),
        kind,
        name: name.trim().to_string(),
        origin: PresetOrigin::User,
        user_id: Some(user),
        vendor: None,
        inherits,
        instantiation: true,
        setting_id: None,
        filament_id: None,
        compatible_printers: None,
        values: match values {
            Value::Object(_) => values,
            _ => Value::Object(Map::new()),
        },
    })
}

/// Convertit les clés/valeurs legacy d'un profil Orca (FR-023). Les clés méta
/// sont ignorées ; les clés inconnues du registre sont abandonnées comme le
/// fait `handle_legacy`.
fn convert_legacy(values: &Value) -> Map<String, Value> {
    let Value::Object(input) = values else {
        return Map::new();
    };
    let mut out = Map::new();
    for (key, value) in input {
        if META_FIELDS.contains(&key.as_str()) {
            continue;
        }
        let converted = match value {
            Value::String(s) => engine::params::handle_legacy(key, s),
            _ => engine::params::handle_legacy(key, ""),
        };
        if let Some((new_key, new_value)) = converted {
            let stored = match value {
                Value::String(_) => Value::String(new_value),
                other => other.clone(),
            };
            out.insert(new_key, stored);
        }
    }
    out
}

/// Reconstruit la chaîne racine→feuille depuis les presets stockés en suivant
/// `inherits` (même vendeur préféré en cas d'homonymie ; parent manquant =
/// racine ; cycle stoppé).
pub(crate) fn build_chain(candidates: &[Preset], leaf: &Preset) -> Vec<RawPreset> {
    let mut by_name: HashMap<&str, Vec<&Preset>> = HashMap::new();
    for p in candidates {
        by_name.entry(p.name.as_str()).or_default().push(p);
    }
    let mut chain = Vec::new();
    let mut seen: HashSet<&str> = HashSet::new();
    let mut current = Some(leaf);
    while let Some(p) = current {
        if !seen.insert(p.name.as_str()) {
            break; // cycle
        }
        chain.push(to_raw(p));
        current = p
            .inherits
            .as_deref()
            .and_then(|parent| pick_parent(by_name.get(parent), p));
    }
    chain.reverse();
    chain
}

/// Choisit le parent : même vendeur que l'enfant en priorité, sinon l'unique.
fn pick_parent<'a>(candidates: Option<&Vec<&'a Preset>>, child: &Preset) -> Option<&'a Preset> {
    let candidates = candidates?;
    if let Some(v) = &child.vendor {
        if let Some(p) = candidates.iter().find(|c| c.vendor.as_ref() == Some(v)) {
            return Some(p);
        }
    }
    match candidates.as_slice() {
        [only] => Some(only),
        _ => candidates.first().copied(),
    }
}

fn to_raw(p: &Preset) -> RawPreset {
    RawPreset {
        name: p.name.clone(),
        inherits: p.inherits.clone(),
        values: match &p.values {
            Value::Object(m) => m.clone(),
            _ => Map::new(),
        },
    }
}

/// Aplati un `DynamicPrintConfig` en objet JSON simple (valeurs lisibles).
pub(crate) fn config_to_json(config: &engine::api::DynamicPrintConfig) -> Map<String, Value> {
    config
        .0
        .iter()
        .map(|(k, v)| (k.clone(), config_value_to_json(v)))
        .collect()
}

fn config_value_to_json(value: &ConfigValue) -> Value {
    match value {
        ConfigValue::Bool(b) => Value::Bool(*b),
        ConfigValue::Int(i) => Value::from(*i),
        ConfigValue::Float(f) => Value::from(*f),
        ConfigValue::FloatOrPercent { value, percent } => {
            if *percent {
                Value::String(format!("{value}%"))
            } else {
                Value::from(*value)
            }
        }
        ConfigValue::String(s) => Value::String(s.clone()),
        ConfigValue::Point(v) | ConfigValue::Floats(v) => Value::from(v.clone()),
        ConfigValue::Bools(v) => Value::from(v.clone()),
        ConfigValue::Ints(v) => Value::from(v.clone()),
        ConfigValue::FloatsNullable(v) => Value::Array(
            v.iter()
                .map(|o| o.map(Value::from).unwrap_or(Value::Null))
                .collect(),
        ),
        ConfigValue::Strings(v) => Value::from(v.clone()),
        ConfigValue::Points(v) => Value::Array(v.iter().map(|p| Value::from(p.clone())).collect()),
    }
}
