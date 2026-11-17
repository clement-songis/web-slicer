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

use crate::domain::preset_compat::{self, ActivePrinter, RegexCache};
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
    // Process et filaments : compatibilité exacte Orca (héritage + condition,
    // `PresetCollection::update_compatible` filtre les deux collections avec
    // `is_compatible_with_printer`) quand l'imprimante est connue ; sinon
    // filtre de liste historique (FR-021).
    if matches!(kind, PresetKind::Process | PresetKind::Filament) {
        if let Some(name) = q.printer.as_deref() {
            if let Some(printer) = active_printer(&state, user.id, name).await? {
                let presets = list_compatible_exact(&state, user.id, kind, &printer).await?;
                return Ok(Json(presets.into_iter().map(Into::into).collect()));
            }
        }
    }
    let presets = state
        .storage
        .presets()
        .list_compatible(kind, q.printer.as_deref(), user.id)
        .await?;
    Ok(Json(presets.into_iter().map(Into::into).collect()))
}

/// Imprimante active par nom (système en priorité, sinon copie utilisateur),
/// avec sa config résolue — défauts du registre inclus, comme la config
/// complète d'Orca sur laquelle s'évaluent les conditions. `None` si inconnue.
async fn active_printer(
    state: &AppState,
    user: UserId,
    name: &str,
) -> ApiResult<Option<ActivePrinter>> {
    let named = state
        .storage
        .presets()
        .list_by_name(PresetKind::Machine, name, user)
        .await?;
    let Some(machine) = named
        .iter()
        .find(|p| p.origin == PresetOrigin::System)
        .or_else(|| named.iter().find(|p| p.user_id == Some(user)))
    else {
        return Ok(None);
    };
    let chain = load_chain(state, user, machine).await?;
    let mut config = engine::presets::resolve_preset_chain(&chain).map_err(|e| {
        tracing::error!(error = %e, "résolution de l'imprimante active");
        ApiError::internal()
    })?;
    fill_registry_defaults(&mut config);
    let user_parent = (machine.origin == PresetOrigin::User)
        .then(|| chain.get(1).map(|p| p.name.clone()))
        .flatten();
    let mut values = config_to_json(&config);
    // Clés hors registre (ex. `printer_notes`, base des conditions Prusa) :
    // valeur brute la plus proche de la feuille.
    for raw in &chain {
        for (key, value) in &raw.values {
            values.entry(key.clone()).or_insert_with(|| value.clone());
        }
    }
    Ok(Some(ActivePrinter::new(
        machine.name.clone(),
        user_parent,
        values,
    )))
}

/// Presets instanciables d'un type compatibles avec `printer` selon
/// `Preset::is_compatible_with_printer` d'Orca : `compatible_printers` et
/// `compatible_printers_condition` résolus sur la chaîne `inherits` (parents
/// abstraits inclus, chargés en une requête), puis évalués. Ordre de
/// `list_compatible` (origine, nom).
async fn list_compatible_exact(
    state: &AppState,
    user: UserId,
    kind: PresetKind,
    printer: &ActivePrinter,
) -> ApiResult<Vec<Preset>> {
    let all = state.storage.presets().list_by_kind(kind, user).await?;
    let mut by_name: HashMap<&str, Vec<&Preset>> = HashMap::new();
    let mut by_id: HashMap<String, &Preset> = HashMap::new();
    for p in &all {
        by_name.entry(p.name.as_str()).or_default().push(p);
        by_id.insert(p.id.to_string(), p);
    }
    let empty = Map::new();
    let mut cache = RegexCache::default();
    let mut out: Vec<Preset> = all
        .iter()
        .filter(|p| p.instantiation && (p.user_id.is_none() || p.user_id == Some(user)))
        .filter(|leaf| {
            let mut chain: Vec<&Map<String, Value>> = Vec::new();
            let mut seen: HashSet<String> = HashSet::new();
            let mut current: &Preset = leaf;
            loop {
                if !seen.insert(current.id.to_string()) {
                    break; // cycle
                }
                chain.push(match &current.values {
                    Value::Object(m) => m,
                    _ => &empty,
                });
                let Some(parent) = current.inherits.as_deref() else {
                    break;
                };
                let next = pick_parent(by_name.get(parent), current)
                    .or_else(|| by_id.get(parent).copied().filter(|p| p.kind == kind));
                match next {
                    Some(p) => current = p,
                    None => break,
                }
            }
            let spec = preset_compat::resolve_spec(chain);
            preset_compat::is_compatible(&spec, printer, &mut cache)
        })
        .cloned()
        .collect();
    out.sort_by(|a, b| {
        let origin = |p: &Preset| p.origin == PresetOrigin::User;
        origin(a).cmp(&origin(b)).then_with(|| a.name.cmp(&b.name))
    });
    Ok(out)
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
    let chain = load_chain(&state, user.id, &leaf).await?;
    let mut config = engine::presets::resolve_preset_chain(&chain).map_err(|e| {
        tracing::error!(error = %e, "résolution de preset");
        ApiError::internal()
    })?;
    fill_registry_defaults(&mut config);
    Ok(Json(ResolvedPreset {
        id: leaf.id.to_string(),
        values: Value::Object(config_to_json(&config)),
    }))
}

/// Complète les clés absentes de la chaîne avec les défauts du registre, comme
/// l'UI Orca (config complète `full_print_config`) : sans cela l'éditeur
/// affiche l'état neutre du contrôle (case décochée, champ vide) au lieu du
/// défaut Orca (ex. `support_chamber_temp_control = 1`). Seuls les défauts
/// exprimables en texte Orca valide sont posés (les formes C++ non résolues
/// par l'audit — `Vec2d(…)`, noms d'enum internes — sont ignorées).
pub(crate) fn fill_registry_defaults(config: &mut engine::api::DynamicPrintConfig) {
    for def in engine::params::REGISTRY.iter() {
        if config.0.contains_key(def.key) {
            continue;
        }
        let Some(raw) = def.default_json else {
            continue;
        };
        let Ok(json) = serde_json::from_str::<Value>(raw) else {
            continue;
        };
        let scalar = |v: &Value| match v {
            Value::String(s) => Some(s.clone()),
            Value::Bool(b) => Some(if *b { "1" } else { "0" }.to_string()),
            Value::Number(n) => Some(n.to_string()),
            _ => None,
        };
        let text = match &json {
            Value::Array(items) => {
                let parts: Option<Vec<String>> = items.iter().map(scalar).collect();
                let Some(parts) = parts else { continue };
                let sep = if def.kind == engine::params::ParamKind::Point {
                    "x"
                } else {
                    ","
                };
                parts.join(sep)
            }
            other => {
                let Some(t) = scalar(other) else { continue };
                t
            }
        };
        // Un enum n'accepte qu'une valeur de sa liste (pas un nom C++ interne).
        if matches!(
            def.kind,
            engine::params::ParamKind::Enum | engine::params::ParamKind::Enums
        ) && !text.split(',').all(|t| def.enum_values.contains(&t.trim()))
        {
            continue;
        }
        if let Some(value) = engine::params::orca_values::parse_orca_value(def, &text) {
            config.0.insert(def.key.to_string(), value);
        }
    }
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
///
/// Comme Orca (PresetCollection::delete_current_preset : les projets gardent
/// leur config et retombent sur le preset parent), un preset utilisateur
/// référencé par des projets ne bloque pas la suppression : ces projets sont
/// basculés sur le preset PARENT (`inherits`), puis le preset est supprimé.
/// Réponse : 204 si aucun projet n'était concerné, sinon 200
/// `{ parent: {id, name}, projects: [{id, name}] }`. Sans parent résoluble
/// (preset racine), 409 avec `details.projects` (rien n'est modifié).
pub async fn delete(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<axum::response::Response> {
    use axum::response::IntoResponse;
    let id = parse_preset_id(&id)?;
    let preset = state.storage.presets().get(id).await?;
    if preset.user_id != Some(user.id) {
        return Err(ApiError::not_found("Preset"));
    }
    let uuid: uuid::Uuid = id.into();
    let projects: Vec<crate::domain::Project> = state
        .storage
        .projects()
        .list(user.id)
        .await?
        .into_iter()
        .filter(|p| crate::adapters::storage::json_references_id(&p.active_presets, uuid))
        .collect();
    if projects.is_empty() {
        state
            .storage
            .presets()
            .delete_user_preset(user.id, id)
            .await?;
        return Ok(StatusCode::NO_CONTENT.into_response());
    }
    let listed = |projects: &[crate::domain::Project]| -> Vec<Value> {
        projects
            .iter()
            .map(|p| serde_json::json!({ "id": p.id.to_string(), "name": p.name }))
            .collect()
    };
    // Parent : preset de même type portant le nom `inherits` (utilisateur
    // d'abord, sinon système), visible par l'utilisateur.
    let parent = match preset.inherits.as_deref().filter(|n| !n.is_empty()) {
        None => None,
        Some(name) => {
            let mut candidates: Vec<_> = state
                .storage
                .presets()
                .list_by_kind(preset.kind, user.id)
                .await?
                .into_iter()
                .filter(|p| p.name == name && p.id != id)
                .collect();
            candidates.sort_by_key(|p| p.user_id.is_none());
            candidates.into_iter().next()
        }
    };
    let Some(parent) = parent else {
        return Err(ApiError::conflict_with(
            "preset référencé par des projets et sans parent",
            serde_json::json!({ "projects": listed(&projects) }),
        ));
    };
    let parent_id = parent.id.to_string();
    let target = id.to_string();
    fn replace(value: &mut Value, from: &str, to: &str) {
        match value {
            Value::String(s) if s == from => *s = to.to_string(),
            Value::Array(items) => items.iter_mut().for_each(|v| replace(v, from, to)),
            Value::Object(map) => map.values_mut().for_each(|v| replace(v, from, to)),
            _ => {}
        }
    }
    for project in &projects {
        let mut active = project.active_presets.clone();
        replace(&mut active, &target, &parent_id);
        state
            .storage
            .projects()
            .update(
                user.id,
                project.id,
                project.version,
                project.scene.clone(),
                active,
                project.thumbnail_path.clone(),
            )
            .await?;
    }
    state
        .storage
        .presets()
        .delete_user_preset(user.id, id)
        .await?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "parent": { "id": parent_id, "name": parent.name },
            "projects": listed(&projects),
        })),
    )
        .into_response())
}

/// `GET /api/presets/create-options` — métadonnées des assistants « Créer un
/// filament / une imprimante » d'Orca (CreateFilamentPresetDialog /
/// CreatePrinterPresetDialog) :
/// - listes du dialogue Orca (fournisseurs, types, constructeurs, modèles,
///   diamètres de buse ; audit/create_preset_lists.json, extrait de
///   CreatePresetsDialog.cpp) complétées par les fournisseurs et types des
///   filaments système (comme m_system_filament_types_set) ;
/// - gabarits : presets système instanciables (filaments : nom, fournisseur,
///   type, filament_id, imprimantes compatibles ; imprimantes : nom,
///   constructeur, modèle, diamètre de buse), valeurs héritées résolues.
pub async fn create_options(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
) -> ApiResult<Json<Value>> {
    static LISTS: &str = include_str!("../../../../audit/create_preset_lists.json");
    let lists: Value = serde_json::from_str(LISTS).map_err(|_| ApiError::internal())?;
    let system = |presets: Vec<Preset>| -> Vec<Preset> {
        presets
            .into_iter()
            .filter(|p| p.user_id.is_none())
            .collect()
    };
    let filaments = system(
        state
            .storage
            .presets()
            .list_by_kind(PresetKind::Filament, user.id)
            .await?,
    );
    let machines = system(
        state
            .storage
            .presets()
            .list_by_kind(PresetKind::Machine, user.id)
            .await?,
    );

    // Première valeur d'une clé, en remontant la chaîne `inherits` (presets
    // Orca partiels : filament_type porté par le parent).
    fn inherited(presets: &[Preset], start: &Preset, key: &str) -> Option<String> {
        let by_name: HashMap<&str, &Preset> =
            presets.iter().map(|p| (p.name.as_str(), p)).collect();
        let mut current = Some(start);
        for _ in 0..16 {
            let preset = current?;
            if let Some(v) = preset.values.get(key) {
                let first = match v {
                    Value::Array(items) => {
                        items.first().and_then(Value::as_str).map(str::to_string)
                    }
                    Value::String(s) => Some(
                        s.split(';')
                            .next()
                            .unwrap_or("")
                            .trim_matches('"')
                            .to_string(),
                    ),
                    _ => None,
                };
                if let Some(first) = first.filter(|s| !s.is_empty()) {
                    return Some(first);
                }
            }
            current = preset
                .inherits
                .as_deref()
                .and_then(|n| by_name.get(n).copied());
        }
        None
    }
    let strings = |v: &Value| -> Vec<String> {
        v.as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|s| s.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default()
    };
    let mut vendors: std::collections::BTreeSet<String> =
        strings(&lists["filament_vendors"]).into_iter().collect();
    let mut types: Vec<String> = strings(&lists["filament_types"]);
    let mut filament_templates = Vec::new();
    for preset in filaments.iter().filter(|p| p.instantiation) {
        let vendor = inherited(&filaments, preset, "filament_vendor");
        let kind = inherited(&filaments, preset, "filament_type");
        if let Some(v) = &vendor {
            vendors.insert(v.clone());
        }
        if let Some(t) = &kind {
            if !types.contains(t) {
                types.push(t.clone());
            }
        }
        filament_templates.push(serde_json::json!({
            "id": preset.id.to_string(),
            "name": preset.name,
            "vendor": vendor,
            "type": kind,
            "filament_id": preset.filament_id,
            "compatible_printers": preset.compatible_printers,
        }));
    }
    let printer_templates: Vec<Value> = machines
        .iter()
        .filter(|p| p.instantiation)
        .map(|preset| {
            serde_json::json!({
                "id": preset.id.to_string(),
                "name": preset.name,
                "vendor": preset.vendor,
                "model": inherited(&machines, preset, "printer_model"),
                "nozzle_diameter": inherited(&machines, preset, "nozzle_diameter")
                    .or_else(|| inherited(&machines, preset, "printer_variant")),
            })
        })
        .collect();
    Ok(Json(serde_json::json!({
        "source": lists["source"],
        "filament": {
            "vendors": vendors.into_iter().collect::<Vec<_>>(),
            "types": types,
            "templates": filament_templates,
        },
        "printer": {
            "vendors": lists["printer_vendors"],
            "models": lists["printer_models"],
            "nozzle_diameters": lists["nozzle_diameters"],
            "templates": printer_templates,
        },
    })))
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
    // L'éditeur dérive un preset avec `inherits = <id du parent>` (et non son
    // nom Orca) : on résout aussi le parent par id, sinon la chaîne s'arrête à la
    // feuille (valeurs = seules les différences, modèle d'imprimante introuvable
    // → « Imprimante non possédée » au tranchage).
    let by_id: HashMap<String, Vec<&Preset>> = candidates
        .iter()
        .map(|p| (p.id.to_string(), vec![p]))
        .collect();
    let mut chain = Vec::new();
    // Cycle détecté par id : une copie peut porter le nom de son parent.
    let mut seen: HashSet<String> = HashSet::new();
    let mut current = Some(leaf);
    while let Some(p) = current {
        if !seen.insert(p.id.to_string()) {
            break; // cycle
        }
        chain.push(to_raw(p));
        current = p.inherits.as_deref().and_then(|parent| {
            pick_parent(by_name.get(parent), p).or_else(|| pick_parent(by_id.get(parent), p))
        });
    }
    chain.reverse();
    chain
}

/// Comme [`build_chain`] mais par requêtes ciblées (un `list_by_name` par
/// maillon, repli par id) au lieu de charger tous les presets du type : la
/// résolution d'un filament lisait ~7000 presets système (~7 Mo de JSON).
pub(crate) async fn load_chain(
    state: &AppState,
    user: UserId,
    leaf: &Preset,
) -> ApiResult<Vec<RawPreset>> {
    let repo = state.storage.presets();
    let mut chain = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut current = leaf.clone();
    loop {
        if !seen.insert(current.id.to_string()) {
            break; // cycle
        }
        chain.push(to_raw(&current));
        let Some(parent) = current.inherits.clone() else {
            break;
        };
        let named = repo.list_by_name(current.kind, &parent, user).await?;
        let refs: Vec<&Preset> = named.iter().collect();
        let mut next = pick_parent(Some(&refs), &current).cloned();
        if next.is_none() {
            // L'éditeur a pu dériver avec `inherits = <id du parent>`.
            if let Ok(pid) = parse_preset_id(&parent) {
                if let Ok(p) = repo.get(pid).await {
                    let visible = p.user_id.is_none() || p.user_id == Some(user);
                    if visible && p.kind == current.kind && p.id != current.id {
                        next = Some(p);
                    }
                }
            }
        }
        match next {
            Some(p) => current = p,
            None => break,
        }
    }
    chain.reverse();
    Ok(chain)
}

/// Choisit le parent : même vendeur que l'enfant en priorité, sinon l'unique.
fn pick_parent<'a>(candidates: Option<&Vec<&'a Preset>>, child: &Preset) -> Option<&'a Preset> {
    // Un preset utilisateur peut porter le nom de son parent (copie d'imprimante
    // « même nom → même modèle ») : il ne peut pas être son propre parent.
    let candidates: Vec<&'a Preset> = candidates?
        .iter()
        .copied()
        .filter(|c| c.id != child.id)
        .collect();
    if let Some(v) = &child.vendor {
        if let Some(p) = candidates.iter().find(|c| c.vendor.as_ref() == Some(v)) {
            return Some(p);
        }
    }
    // Sinon un preset système en priorité (parent Orca), puis le premier.
    candidates
        .iter()
        .find(|c| c.origin == PresetOrigin::System)
        .or_else(|| candidates.first())
        .copied()
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
        ConfigValue::BoolsNullable(v) => Value::Array(
            v.iter()
                .map(|o| o.map(Value::Bool).unwrap_or(Value::Null))
                .collect(),
        ),
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
