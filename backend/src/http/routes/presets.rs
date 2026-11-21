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
    CreatePresetRequest, DetachPresetRequest, ImportPresetRequest, PresetDetail, PresetSummary,
    ResolvedPreset, UpdatePresetRequest,
};
use crate::http::error::{ApiError, ApiResult};
use crate::http::extract::CurrentUser;
use crate::http::preset_cache;
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
            let started = std::time::Instant::now();
            if let Some(printer) = active_printer(&state, user.id, name).await? {
                let printer_ms = started.elapsed().as_millis() as u64;
                let presets = list_compatible_cached(&state, user.id, kind, &printer).await?;
                tracing::debug!(
                    ?kind,
                    printer_ms,
                    total_ms = started.elapsed().as_millis() as u64,
                    count = presets.len(),
                    "liste de presets compatibles"
                );
                return Ok(Json(presets));
            }
        }
    }
    Ok(Json(
        list_visible_cached(&state, user.id, kind, q.printer.as_deref()).await?,
    ))
}

/// Filtre de liste historique (FR-021, `PresetRepo::list_compatible`) servi
/// par le cache des presets système (lot 41) : instanciables, système puis
/// utilisateur par nom (`ORDER BY origin, name`), `compatible_printers`
/// absent ou contenant `printer`. Liste SQL si le cache ne peut pas
/// reproduire l'ordre (homonymes instanciables).
async fn list_visible_cached(
    state: &AppState,
    user: UserId,
    kind: PresetKind,
    printer: Option<&str>,
) -> ApiResult<Vec<PresetSummary>> {
    let system = state
        .preset_cache
        .system(state.storage.as_ref(), kind)
        .await?;
    if system.instantiable_homonyms {
        let presets = state
            .storage
            .presets()
            .list_compatible(kind, printer, user)
            .await?;
        return Ok(presets.into_iter().map(Into::into).collect());
    }
    let own = state
        .storage
        .presets()
        .list_user_by_kind(kind, user)
        .await?;
    let listed = |p: &&Preset| {
        p.instantiation
            && match (printer, &p.compatible_printers) {
                (Some(name), Some(Value::Array(list))) => {
                    list.iter().any(|v| v.as_str() == Some(name))
                }
                _ => true,
            }
    };
    Ok(system
        .presets
        .iter()
        .filter(listed)
        .chain(own.iter().filter(listed))
        .map(|p| PresetSummary::from(p.clone()))
        .collect())
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
    // Parent direct (Orca : Preset::inherits() de l'imprimante utilisateur,
    // is_compatible_with_parent_printer). `load_chain` renvoie la chaîne de la
    // racine vers la feuille : le parent est l'avant-dernier élément (et non
    // chain[1], qui désignait un ancêtre abstrait, ex. fdm_klipper_common).
    let user_parent = (machine.origin == PresetOrigin::User)
        .then(|| chain.iter().rev().nth(1).map(|p| p.name.clone()))
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

/// Liste compatible servie par le cache des presets système (lot 41) :
/// presets système chargés une fois par génération de reseed (compatibilité
/// résolue d'avance), presets de l'utilisateur lus à chaque requête, verdict
/// de chaque condition distincte évalué une fois. Réponse identique à
/// `list_compatible_exact`, qui reste le chemin suivi quand un preset
/// utilisateur porte le nom (ou l'id) d'un preset système ou d'une cible
/// `inherits` système — il pourrait alors s'insérer dans une chaîne système
/// ou concurrencer un parent selon l'ordre SQL.
async fn list_compatible_cached(
    state: &AppState,
    user: UserId,
    kind: PresetKind,
    printer: &ActivePrinter,
) -> ApiResult<Vec<PresetSummary>> {
    let system = state
        .preset_cache
        .system(state.storage.as_ref(), kind)
        .await?;
    let own = state
        .storage
        .presets()
        .list_user_by_kind(kind, user)
        .await?;
    if own
        .iter()
        .any(|p| system.reserved.contains(&p.name) || system.reserved.contains(&p.id.to_string()))
    {
        let presets = list_compatible_exact(state, user, kind, printer).await?;
        return Ok(presets.into_iter().map(Into::into).collect());
    }
    let mut cache = RegexCache::default();
    // Une condition ne dépend que de la config de l'imprimante : même verdict
    // pour tous les presets qui la partagent.
    let mut verdicts: HashMap<String, bool> = HashMap::new();
    let mut compatible = |spec: &preset_compat::CompatSpec| -> bool {
        if !spec.compatible_printers.is_empty() || spec.condition.is_empty() {
            return preset_compat::is_compatible(spec, printer, &mut cache);
        }
        if let Some(&v) = verdicts.get(spec.condition.as_str()) {
            return v;
        }
        let v = preset_compat::is_compatible(spec, printer, &mut cache);
        verdicts.insert(spec.condition.clone(), v);
        v
    };
    let mut system_out: Vec<&Preset> = system
        .presets
        .iter()
        .zip(&system.specs)
        .filter_map(|(p, spec)| spec.as_ref().filter(|s| compatible(s)).map(|_| p))
        .collect();
    system_out.sort_by(|a, b| a.name.cmp(&b.name));
    // Aucun nom utilisateur ne recoupe un nom système : les homonymes d'un
    // maillon sont soit le preset utilisateur, soit les presets système.
    let user_by_name: HashMap<&str, &Preset> = own.iter().map(|p| (p.name.as_str(), p)).collect();
    let user_by_id: HashMap<String, &Preset> = own.iter().map(|p| (p.id.to_string(), p)).collect();
    let mut user_out: Vec<&Preset> = own
        .iter()
        .filter(|p| p.instantiation)
        .filter(|leaf| {
            let spec = preset_cache::compat_spec(
                leaf,
                |name| match user_by_name.get(name) {
                    Some(p) => Some(vec![*p]),
                    None => system.candidates(name),
                },
                |id| user_by_id.get(id).copied().or_else(|| system.get_id(id)),
            );
            compatible(&spec)
        })
        .collect();
    user_out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(system_out
        .into_iter()
        .chain(user_out)
        .map(|p| PresetSummary::from(p.clone()))
        .collect())
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
    let values = config_to_json(&config);
    let default_bed_type = (leaf.kind == crate::domain::PresetKind::Machine)
        .then(|| printer_default_bed_type(&values));
    Ok(Json(ResolvedPreset {
        id: leaf.id.to_string(),
        values: Value::Object(values),
        default_plate_type: default_bed_type.map(crate::models::threemf::bed_type_to_plate_type),
        default_bed_type: default_bed_type.map(str::to_string),
    }))
}

/// Modèles d'imprimante dont le `model_id` (fichiers `machine_model` du
/// vendeur BBL d'Orca 2.4.1 : X1 Carbon « BL-P001 », X1 « BL-P002 », X1E
/// « C13 ») donne une plaque froide par défaut. Le seed des presets ne garde
/// pas `model_id` (engine::presets::import) : table recopiée des profils.
const COOL_PLATE_PRINTER_MODELS: &[&str] =
    &["Bambu Lab X1 Carbon", "Bambu Lab X1", "Bambu Lab X1E"];

/// Clés Orca de `BedType` (enum : btDefault = 0, btPC, btEP, btPEI, btPTE,
/// btPCT, btSuperTack), indexées par valeur.
const BED_TYPE_KEYS: &[&str] = &[
    "Default Plate",
    "Cool Plate",
    "Engineering Plate",
    "High Temp Plate",
    "Textured PEI Plate",
    "Textured Cool Plate",
    "Supertack Plate",
];

/// `Preset::get_default_bed_type` (Orca 2.4.1, Preset.cpp:966) sur les valeurs
/// résolues d'un preset machine : `default_bed_type` non vide → `atoi` > 0 =
/// ce type, sinon (y compris un libellé texte comme « Textured PEI Plate »,
/// que `atoi` lit 0) High Temp ; absent → X1 / X1 Carbon / X1E = Cool Plate,
/// toute autre imprimante = High Temp (btPEI). Renvoie la clé Orca.
pub(crate) fn printer_default_bed_type(values: &serde_json::Map<String, Value>) -> &'static str {
    let text = |key: &str| match values.get(key) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Array(items)) => items
            .first()
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        Some(Value::Number(n)) => n.to_string(),
        _ => String::new(),
    };
    let configured = text("default_bed_type");
    if !configured.is_empty() {
        return match c_atoi(&configured) {
            value if value > 0 => BED_TYPE_KEYS
                .get(value as usize)
                .copied()
                // Hors de l'enum : Orca renverrait un BedType invalide ;
                // le sélecteur retombe sur le premier choix utile.
                .unwrap_or("High Temp Plate"),
            _ => "High Temp Plate",
        };
    }
    if COOL_PLATE_PRINTER_MODELS.contains(&text("printer_model").as_str()) {
        "Cool Plate"
    } else {
        "High Temp Plate"
    }
}

/// `atoi` du C : espaces initiaux, signe facultatif, chiffres ; 0 sinon.
fn c_atoi(text: &str) -> i64 {
    let trimmed = text.trim_start();
    let (sign, digits) = match trimmed.as_bytes().first() {
        Some(b'-') => (-1, &trimmed[1..]),
        Some(b'+') => (1, &trimmed[1..]),
        _ => (1, trimmed),
    };
    let number: String = digits.chars().take_while(char::is_ascii_digit).collect();
    number.parse::<i64>().map_or(0, |n| sign * n)
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
        // Lot 44 (audit 43 I3) : les défauts FloatOrPercent du registre sont
        // des objets `{ "value": 100.0, "percent": true }` (36 options :
        // initial_layer_travel_acceleration, skin_infill_line_width…) — ils
        // étaient ignorés, si bien qu'une valeur de projet 3MF différente du
        // défaut d'Orca n'était pas reconnue comme modifiée (imported_overrides).
        let scalar = |v: &Value| match v {
            Value::String(s) => Some(s.clone()),
            Value::Bool(b) => Some(if *b { "1" } else { "0" }.to_string()),
            Value::Number(n) => Some(n.to_string()),
            Value::Object(map) => {
                let number = map.get("value").and_then(Value::as_f64)?;
                let percent = map.get("percent").and_then(Value::as_bool).unwrap_or(false);
                let text = if number.fract() == 0.0 && number.abs() < 1e15 {
                    format!("{}", number as i64)
                } else {
                    number.to_string()
                };
                Some(if percent { format!("{text}%") } else { text })
            }
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

/// Valeurs brutes de la chaîne aplaties (racine → feuille, la feuille gagne),
/// au format JSON Orca stocké ; identifiants de réglage renommés comme Orca
/// (`save_current_preset` avec `detach` : `*_settings_id` = nouveau nom).
pub(crate) fn flatten_chain(chain: &[RawPreset], name: &str) -> Value {
    let mut values = Map::new();
    for raw in chain {
        for (k, v) in &raw.values {
            if k != "inherits" {
                values.insert(k.clone(), v.clone());
            }
        }
    }
    for key in ["print_settings_id", "printer_settings_id"] {
        if values.contains_key(key) {
            values.insert(key.into(), Value::String(name.into()));
        }
    }
    if let Some(v) = values.get_mut("filament_settings_id") {
        *v = match v {
            Value::Array(_) => Value::Array(vec![Value::String(name.into())]),
            _ => Value::String(name.into()),
        };
    }
    Value::Object(values)
}

/// `POST /api/presets/{id}/detach` — « Detach from system preset » d'Orca
/// (Tab.cpp:2163, `PresetCollection::save_current_preset(…, detach=true)`) :
/// - preset utilisateur : détaché sur place (`inherits` vidé, valeurs =
///   chaîne aplatie) → 200 ; déjà sans parent → 200 inchangé ;
/// - preset système : copie utilisateur détachée nommée `name` (requis) → 201.
pub async fn detach(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    body: Option<Json<DetachPresetRequest>>,
) -> ApiResult<(StatusCode, Json<PresetDetail>)> {
    let id = parse_preset_id(&id)?;
    let body = body.map(|Json(b)| b).unwrap_or_default();
    let leaf = load_visible(&state, user.id, id).await?;
    if leaf.origin == PresetOrigin::User && leaf.inherits.is_none() {
        return Ok((StatusCode::OK, Json(leaf.into())));
    }
    let chain = load_chain(&state, user.id, &leaf).await?;
    if leaf.origin == PresetOrigin::User {
        let values = flatten_chain(&chain, &leaf.name);
        let updated = state
            .storage
            .presets()
            .detach_user_preset(user.id, leaf.id, values)
            .await?;
        return Ok((StatusCode::OK, Json(updated.into())));
    }
    let name = body.name.unwrap_or_default();
    let preset = new_user_preset(
        user.id,
        leaf.kind,
        &name,
        None,
        flatten_chain(&chain, name.trim()),
    )?;
    let created = state
        .storage
        .presets()
        .create_user_preset(user.id, preset)
        .await?;
    Ok((StatusCode::CREATED, Json(created.into())))
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
        renamed_from: Vec::new(),
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
pub(crate) fn pick_parent<'a>(
    candidates: Option<&Vec<&'a Preset>>,
    child: &Preset,
) -> Option<&'a Preset> {
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

pub(crate) fn config_value_to_json(value: &ConfigValue) -> Value {
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

#[cfg(test)]
mod cache_tests {
    //! Lot 41 : la liste servie par le cache (`list_compatible_cached`) est
    //! identique à celle du chemin complet (`list_compatible_exact`) sur les
    //! profils réels d'OrcaSlicer — sans presets utilisateur, avec héritage
    //! d'un système, liste propre, condition propre, imprimante utilisateur,
    //! homonyme d'un parent système (repli), puis après modification,
    //! suppression et reseed (invalidation par génération).
    use super::*;
    use crate::adapters::files::FileStore;
    use crate::adapters::storage::AnyStorage;
    use crate::domain::{NewUser, Role};
    use std::sync::Arc;

    async fn assert_same(state: &AppState, user: UserId, printers: &[String]) -> usize {
        let mut checked = 0;
        for name in printers {
            let Some(printer) = active_printer(state, user, name).await.unwrap() else {
                panic!("imprimante inconnue : {name}");
            };
            for kind in [PresetKind::Filament, PresetKind::Process] {
                let exact: Vec<PresetSummary> = list_compatible_exact(state, user, kind, &printer)
                    .await
                    .unwrap()
                    .into_iter()
                    .map(Into::into)
                    .collect();
                let cached = list_compatible_cached(state, user, kind, &printer)
                    .await
                    .unwrap();
                assert_eq!(
                    serde_json::to_value(&exact).unwrap(),
                    serde_json::to_value(&cached).unwrap(),
                    "{kind:?} / {name}"
                );
                checked += 1;
            }
        }
        checked
    }

    /// Filtre de liste historique : cache identique à `list_compatible` (SQL).
    async fn assert_plain_same(state: &AppState, user: UserId, printers: &[Option<&str>]) {
        for kind in [
            PresetKind::MachineModel,
            PresetKind::Machine,
            PresetKind::Filament,
            PresetKind::Process,
        ] {
            for printer in printers {
                let sql: Vec<PresetSummary> = state
                    .storage
                    .presets()
                    .list_compatible(kind, *printer, user)
                    .await
                    .unwrap()
                    .into_iter()
                    .map(Into::into)
                    .collect();
                let cached = list_visible_cached(state, user, kind, *printer)
                    .await
                    .unwrap();
                assert_eq!(
                    serde_json::to_value(&sql).unwrap(),
                    serde_json::to_value(&cached).unwrap(),
                    "{kind:?} / {printer:?}"
                );
            }
        }
    }

    async fn names(state: &AppState, user: UserId, kind: PresetKind, printer: &str) -> Vec<String> {
        let printer = active_printer(state, user, printer).await.unwrap().unwrap();
        list_compatible_cached(state, user, kind, &printer)
            .await
            .unwrap()
            .into_iter()
            .map(|p| p.name)
            .collect()
    }

    async fn add(
        state: &AppState,
        user: UserId,
        kind: PresetKind,
        name: &str,
        inherits: Option<&str>,
        values: Value,
    ) -> Preset {
        let preset = new_user_preset(user, kind, name, inherits.map(String::from), values).unwrap();
        state
            .storage
            .presets()
            .create_user_preset(user, preset)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn cached_list_matches_full_path() {
        let Ok(imported) =
            engine::presets::import_profiles(&crate::http::state::default_profiles_dir())
        else {
            return; // profils absents (build hors dépôt)
        };
        let dir = tempfile::tempdir().unwrap();
        let url = format!("sqlite://{}", dir.path().join("cache.db").display());
        let storage = AnyStorage::connect_test(&url).await.unwrap();
        crate::domain::presets::reseed_system_presets(&storage, &imported.presets)
            .await
            .unwrap();
        let state = AppState::new(Arc::new(storage), FileStore::new(dir.path().join("data")));
        let user = state
            .storage
            .users()
            .create(NewUser {
                email: "cache@test.local".into(),
                password_hash: "x".into(),
                role: Role::User,
            })
            .await
            .unwrap()
            .id;

        // Échantillon d'imprimantes système (tous vendeurs) + références.
        let machines = state
            .storage
            .presets()
            .list_compatible(PresetKind::Machine, None, user)
            .await
            .unwrap();
        let mut printers: Vec<String> = machines
            .iter()
            .step_by(120)
            .map(|m| m.name.clone())
            .collect();
        let reference = [
            "Voron 2.4 350 0.4 nozzle",
            "Bambu Lab X1 Carbon 0.4 nozzle",
            "Prusa MK4S 0.4 nozzle",
            "Prusa MK4S HF0.4 nozzle",
        ];
        printers.extend(reference.iter().map(|s| s.to_string()));
        assert!(assert_same(&state, user, &printers).await >= 20);
        let plain = [None, Some("Voron 2.4 350 0.4 nozzle")];
        assert_plain_same(&state, user, &plain).await;

        // Presets utilisateur : héritage système, liste propre, condition
        // propre, imprimante utilisateur copiée d'un modèle système.
        add(
            &state,
            user,
            PresetKind::Filament,
            "Mon PLA",
            Some("Generic PLA @System"),
            serde_json::json!({}),
        )
        .await;
        let own_list = add(
            &state,
            user,
            PresetKind::Process,
            "Mon process",
            Some("0.20mm Standard @Voron"),
            serde_json::json!({ "compatible_printers": ["Prusa MK4S 0.4 nozzle"] }),
        )
        .await;
        add(
            &state,
            user,
            PresetKind::Process,
            "Process 0.6",
            None,
            serde_json::json!({ "compatible_printers_condition": "nozzle_diameter[0]==0.6" }),
        )
        .await;
        add(
            &state,
            user,
            PresetKind::Machine,
            "Mon Voron",
            Some("Voron 2.4 350 0.4 nozzle"),
            serde_json::json!({}),
        )
        .await;
        // Rondes suivantes : imprimantes concernées par les presets utilisateur.
        let small: Vec<String> = [
            "Voron 2.4 350 0.4 nozzle",
            "Prusa MK4S 0.4 nozzle",
            "Bambu Lab X1 Carbon 0.4 nozzle",
            "Mon Voron",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        assert_same(&state, user, &small).await;
        assert_plain_same(&state, user, &plain).await;
        assert!(
            names(&state, user, PresetKind::Process, "Prusa MK4S 0.4 nozzle")
                .await
                .contains(&"Mon process".to_string())
        );
        assert!(names(&state, user, PresetKind::Filament, "Mon Voron")
            .await
            .contains(&"Mon PLA".to_string()));

        // Homonyme d'un parent système : chemin complet (repli), même réponse.
        let clash = add(
            &state,
            user,
            PresetKind::Filament,
            "fdm_filament_pla",
            None,
            serde_json::json!({ "compatible_printers": ["Bambu Lab X1 Carbon 0.4 nozzle"] }),
        )
        .await;
        assert_same(&state, user, &small).await;
        state
            .storage
            .presets()
            .delete_user_preset(user, clash.id)
            .await
            .unwrap();
        assert_same(&state, user, &small).await;

        // Modification : la liste propre vidée rend le process universel.
        state
            .storage
            .presets()
            .update_user_preset(
                user,
                own_list.id,
                "Mon process",
                serde_json::json!({ "compatible_printers": [] }),
            )
            .await
            .unwrap();
        assert_same(&state, user, &small).await;
        assert!(names(
            &state,
            user,
            PresetKind::Process,
            "Bambu Lab X1 Carbon 0.4 nozzle"
        )
        .await
        .contains(&"Mon process".to_string()));
        state
            .storage
            .presets()
            .delete_user_preset(user, own_list.id)
            .await
            .unwrap();
        assert!(
            !names(&state, user, PresetKind::Process, "Prusa MK4S 0.4 nozzle")
                .await
                .contains(&"Mon process".to_string())
        );

        // Reseed sans les process « @Voron » : le cache est reconstruit.
        let before = names(
            &state,
            user,
            PresetKind::Process,
            "Voron 2.4 350 0.4 nozzle",
        )
        .await;
        assert!(before.iter().any(|n| n.ends_with("@Voron")));
        let kept: Vec<_> = imported
            .presets
            .iter()
            .filter(|p| {
                !(p.kind == engine::presets::PresetKind::Process && p.name.ends_with("@Voron"))
            })
            .cloned()
            .collect();
        crate::domain::presets::reseed_system_presets(state.storage.as_ref(), &kept)
            .await
            .unwrap();
        let after = names(
            &state,
            user,
            PresetKind::Process,
            "Voron 2.4 350 0.4 nozzle",
        )
        .await;
        assert!(!after.iter().any(|n| n.ends_with("@Voron")), "{after:?}");
        assert_same(&state, user, &small).await;
        assert_plain_same(&state, user, &plain).await;
    }
}

#[cfg(test)]
mod lot44b_tests {
    use super::*;
    use serde_json::json;

    fn values(v: Value) -> serde_json::Map<String, Value> {
        v.as_object().unwrap().clone()
    }

    /// Preset::get_default_bed_type (Orca 2.4.1, Preset.cpp:966).
    #[test]
    fn printer_default_bed_type_follows_orca() {
        // Sans default_bed_type : X1 / X1 Carbon / X1E → Cool Plate, sinon High Temp.
        for model in ["Bambu Lab X1 Carbon", "Bambu Lab X1", "Bambu Lab X1E"] {
            assert_eq!(
                printer_default_bed_type(&values(json!({ "printer_model": model }))),
                "Cool Plate"
            );
        }
        for model in [
            "Bambu Lab P1S",
            "Bambu Lab P1P",
            "Bambu Lab A1 mini",
            "Voron 2.4 350",
        ] {
            assert_eq!(
                printer_default_bed_type(&values(
                    json!({ "printer_model": model, "default_bed_type": "" })
                )),
                "High Temp Plate"
            );
        }
        // Valeur numérique (atoi > 0) : ce BedType ; libellé texte (atoi = 0) : btPEI.
        assert_eq!(
            printer_default_bed_type(&values(
                json!({ "default_bed_type": "4", "printer_model": "Bambu Lab X1" })
            )),
            "Textured PEI Plate"
        );
        assert_eq!(
            printer_default_bed_type(&values(json!({ "default_bed_type": "Textured PEI Plate" }))),
            "High Temp Plate"
        );
        assert_eq!(c_atoi("  12abc"), 12);
        assert_eq!(c_atoi("-3"), -3);
        assert_eq!(c_atoi("x1"), 0);
        assert_eq!(
            crate::models::threemf::bed_type_to_plate_type("High Temp Plate"),
            "Smooth High Temp Plate"
        );
    }
}
