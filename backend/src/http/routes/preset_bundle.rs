//! Lot 46 (audit 44 PR1-a / PR1-b) — préréglages au format de fichier
//! d'OrcaSlicer 2.4.1 et lots de préréglages.
//!
//! - **Fichier de preset** (`Preset::save` → `ConfigBase::save_to_json`) : un
//!   JSON par preset ne contenant que les clés de **son** type
//!   (`Preset::print_options` / `filament_options` / `printer_options` +
//!   `<type>_settings_id`) ; un preset qui hérite n'écrit que ses
//!   **différences** avec son parent système (`config.diff(parent)`), plus
//!   `inherits`, `name`, `from: "User"`, `version`. Orca ne connaît que des
//!   presets utilisateur héritant directement d'un preset système : une chaîne
//!   utilisateur → utilisateur est donc exportée relativement à l'ancêtre
//!   système le plus proche (mêmes valeurs résolues). `type` est ajouté (clé
//!   méta ignorée par l'import d'Orca, exigée par `--load-settings` de la CLI).
//!   Lot 46c : un preset **système** demandé est exporté comme preset
//!   utilisateur racine autonome (`from: "User"`, `inherits: ""`, config
//!   résolue complète de son type), relu tel quel par `--load-settings` et par
//!   l'import d'Orca sur une installation qui n'a pas ce preset.
//! - **Lot** (`ExportConfigsDialog::archive_preset_bundle_to_file`,
//!   CreatePresetsDialog.cpp:3936) : ZIP `<imprimante>.orca_printer` avec
//!   `bundle_structure.json` et les dossiers `printer/`, `filament/`,
//!   `process/` ; lot de filaments `<nom>.orca_filament`
//!   (`archive_filament_bundle_to_file`) ; sinon ZIP plat de JSON
//!   (`archive_*_preset_to_file`).
//! - **Import** (`PresetBundle::import_json_presets`, PresetBundle.cpp:1439) :
//!   type déduit de `printer_settings_id` / `print_settings_id` /
//!   `filament_settings_id` (dans cet ordre), parent système + valeurs du
//!   fichier, clés étrangères au type retirées (`Preset::remove_invalid_keys`),
//!   puis stockage des seules différences (`preset.save(&parent)`). Un preset
//!   système homonyme bloque l'import ; un preset utilisateur homonyme est
//!   remplacé (réponse « Oui » au dialogue d'écrasement d'Orca).

use std::collections::HashSet;
use std::io::{Cursor, Read, Write};
use std::sync::LazyLock;

use axum::body::Bytes;
use axum::extract::{Query, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use engine::api::{DynamicPrintConfig, RawPreset};
use serde::Deserialize;
use serde_json::{Map, Value};

use crate::domain::{Preset, PresetId, PresetKind, PresetOrigin, UserId};
use crate::http::dto::PresetDetail;
use crate::http::error::{ApiError, ApiResult};
use crate::http::extract::CurrentUser;
use crate::http::state::AppState;

/// Version écrite dans les fichiers exportés (Semver lisible par Orca :
/// `import_json_presets` refuse un fichier sans version valide).
pub(crate) const ORCA_PRESET_VERSION: &str = "2.4.1.0";

/// Nom du fichier de structure d'un lot (BUNDLE_STRUCTURE_JSON_NAME).
const BUNDLE_STRUCTURE: &str = "bundle_structure.json";

static PRINT_KEYS: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| super::overrides::ORCA_PRINT_OPTIONS.into_iter().collect());
static PRINTER_KEYS: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| super::overrides::ORCA_PRINTER_OPTIONS.into_iter().collect());
static FILAMENT_KEYS: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| super::projects::ORCA_FILAMENT_OPTIONS.into_iter().collect());

/// Clé `<type>_settings_id` d'un type de preset.
pub(crate) fn settings_id_key(kind: PresetKind) -> Option<&'static str> {
    match kind {
        PresetKind::Machine => Some("printer_settings_id"),
        PresetKind::Process => Some("print_settings_id"),
        PresetKind::Filament => Some("filament_settings_id"),
        PresetKind::MachineModel => None,
    }
}

/// Vrai si `key` appartient aux options du type (clés du preset par défaut de
/// la collection d'Orca : options du type + `<type>_settings_id`).
pub(crate) fn kind_allows_key(kind: PresetKind, key: &str) -> bool {
    match kind {
        PresetKind::Machine => key == "printer_settings_id" || PRINTER_KEYS.contains(key),
        PresetKind::Process => key == "print_settings_id" || PRINT_KEYS.contains(key),
        PresetKind::Filament => key == "filament_settings_id" || FILAMENT_KEYS.contains(key),
        PresetKind::MachineModel => true,
    }
}

/// Type d'un fichier de preset comme `import_json_presets` : présence de
/// `printer_settings_id`, sinon `print_settings_id`, sinon
/// `filament_settings_id` ; à défaut, le type déclaré (`type`).
pub(crate) fn detect_kind(declared: PresetKind, values: &Map<String, Value>) -> PresetKind {
    if values.contains_key("printer_settings_id") {
        PresetKind::Machine
    } else if values.contains_key("print_settings_id") {
        PresetKind::Process
    } else if values.contains_key("filament_settings_id") {
        PresetKind::Filament
    } else {
        declared
    }
}

/// Clés jamais stockées dans `values` (méta du fichier, identifiants recalculés).
fn is_file_meta(key: &str) -> bool {
    matches!(
        key,
        "type"
            | "name"
            | "inherits"
            | "from"
            | "version"
            | "setting_id"
            | "instantiation"
            | "url"
            | "description"
            | "filament_id"
            | "base_id"
            | "user_id"
            | "updated_time"
            | "printer_settings_id"
            | "print_settings_id"
            | "filament_settings_id"
    )
}

fn resolve(chain: &[RawPreset]) -> ApiResult<DynamicPrintConfig> {
    if chain.is_empty() {
        return Ok(DynamicPrintConfig::new());
    }
    let mut config = engine::presets::resolve_preset_chain(chain).map_err(|e| {
        tracing::error!(error = %e, "résolution de preset (lot de préréglages)");
        ApiError::internal()
    })?;
    // Le preset parent d'Orca est complet : une clé absente de la chaîne vaut
    // le défaut du registre des deux côtés de la comparaison.
    super::presets::fill_registry_defaults(&mut config);
    Ok(config)
}

fn raw_of(p: &Preset) -> RawPreset {
    RawPreset {
        name: p.name.clone(),
        inherits: p.inherits.clone(),
        values: match &p.values {
            Value::Object(m) if p.origin == PresetOrigin::User => m
                .iter()
                .filter(|(k, _)| kind_allows_key(p.kind, k))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            Value::Object(m) => m.clone(),
            _ => Map::new(),
        },
    }
}

/// Valeurs du type normalisées au format de fichier Orca (clés étrangères,
/// méta et clés inconnues retirées ; clés legacy converties).
pub(crate) fn own_values(kind: PresetKind, values: &Map<String, Value>) -> Map<String, Value> {
    let legacy = super::presets::convert_legacy_values(values);
    legacy
        .into_iter()
        .filter(|(k, _)| !is_file_meta(k) && kind_allows_key(kind, k))
        .filter_map(|(k, v)| engine::presets::canonical_orca_json(&k, &v).map(|v| (k, v)))
        .map(|(k, v)| {
            // Lot 46c : `[""]` d'un vecteur de chaînes (défaut d'Orca, écrit à
            // l'export au lieu de `[]`) = `""` des profils système : même
            // valeur résolue que le preset d'origine (libslic3r relit `""`
            // comme son défaut `[""]`).
            let empty_strings = matches!(&v, Value::Array(items)
                if items.len() == 1 && items[0].as_str() == Some(""));
            let strings = engine::params::get(&k)
                .is_some_and(|d| d.kind == engine::params::ParamKind::Strings);
            if empty_strings && strings {
                (k, Value::String(String::new()))
            } else {
                (k, v)
            }
        })
        .collect()
}

/// Différences de `values` (déjà propres au type) avec le parent résolu
/// `parent` (chaîne racine → parent) : seules les clés dont la valeur
/// effective change sont gardées (`config.diff(parent)`).
fn diff_against(parent: &[RawPreset], values: Map<String, Value>) -> ApiResult<Map<String, Value>> {
    if parent.is_empty() {
        return Ok(values);
    }
    let base = resolve(parent)?;
    let mut chain = parent.to_vec();
    chain.push(RawPreset {
        name: "__lot46_candidate__".into(),
        inherits: None,
        values: values.clone(),
    });
    let candidate = resolve(&chain)?;
    Ok(values
        .into_iter()
        .filter(|(k, _)| base.get(k).is_none() || candidate.get(k) != base.get(k))
        .collect())
}

/// Chaîne parent (racine → parent) d'un preset à importer : son `inherits`
/// (nom Orca ou id) résolu parmi les presets visibles de l'utilisateur ; vide
/// si aucun parent n'est trouvé.
async fn parent_chain(
    state: &AppState,
    user: UserId,
    kind: PresetKind,
    inherits: Option<&str>,
) -> ApiResult<Vec<RawPreset>> {
    let Some(parent) = inherits.filter(|s| !s.trim().is_empty()) else {
        return Ok(Vec::new());
    };
    let probe = Preset {
        id: PresetId::new(),
        kind,
        name: "__lot46_probe__".into(),
        origin: PresetOrigin::User,
        user_id: Some(user),
        vendor: None,
        inherits: Some(parent.to_string()),
        instantiation: true,
        setting_id: None,
        filament_id: None,
        compatible_printers: None,
        renamed_from: Vec::new(),
        values: Value::Object(Map::new()),
    };
    let mut chain = super::presets::load_chain_presets(state, user, &probe).await?;
    chain.pop(); // la sonde (feuille)
    Ok(chain.iter().map(raw_of).collect())
}

/// Valeurs à stocker pour un preset utilisateur importé (`kind` déjà
/// déterminé) : clés du type, normalisées, différences avec le parent.
pub(crate) async fn import_values(
    state: &AppState,
    user: UserId,
    kind: PresetKind,
    inherits: Option<&str>,
    values: &Map<String, Value>,
) -> ApiResult<Map<String, Value>> {
    let own = own_values(kind, values);
    let parent = parent_chain(state, user, kind, inherits).await?;
    diff_against(&parent, own)
}

/// Fichier de preset Orca d'un preset (voir l'en-tête du module). `draft` :
/// valeurs éditées non enregistrées (config du type), appliquées par-dessus
/// le preset comme le preset « (modifié) » d'Orca.
pub(crate) async fn orca_preset_file(
    state: &AppState,
    user: UserId,
    leaf: &Preset,
    draft: Option<&Map<String, Value>>,
) -> ApiResult<Map<String, Value>> {
    let chain = super::presets::load_chain_presets(state, user, leaf).await?;
    let leaf_is_system = leaf.origin == PresetOrigin::System;
    // Ancêtre système le plus proche (hors feuille).
    let base = chain
        .iter()
        .enumerate()
        .rev()
        .skip(1)
        .find(|(_, p)| p.origin == PresetOrigin::System)
        .map(|(i, _)| i)
        .filter(|_| !leaf_is_system);
    let mut raws: Vec<RawPreset> = chain.iter().map(raw_of).collect();
    if let Some(draft) = draft {
        raws.push(RawPreset {
            name: format!("{} (modifié)", leaf.name),
            inherits: None,
            values: own_values(leaf.kind, draft),
        });
    }
    let leaf_config = resolve(&raws)?;
    // Texte brut le plus haut dans la chaîne (fidélité : « 0.20 » reste « 0.20 »).
    let mut raw_text: Map<String, Value> = Map::new();
    for raw in &raws {
        for (k, v) in &raw.values {
            raw_text.insert(k.clone(), v.clone());
        }
    }
    let value_of = |key: &str| -> Option<Value> {
        raw_text
            .get(key)
            .and_then(|v| engine::presets::canonical_orca_json(key, v))
            .or_else(|| {
                let def = engine::params::get(key)?;
                leaf_config
                    .get(key)
                    .map(|v| engine::presets::config_value_to_orca_json(def, v))
            })
    };
    let (inherits, keys): (Option<String>, Vec<String>) = match base {
        Some(i) => {
            let parent = resolve(&raws[..=i])?;
            let keys = leaf_config
                .0
                .iter()
                .filter(|(k, v)| kind_allows_key(leaf.kind, k) && parent.get(k) != Some(*v))
                .map(|(k, _)| k.clone())
                .collect();
            (Some(chain[i].name.clone()), keys)
        }
        // Preset racine (ou système exporté en preset autonome) : config
        // complète résolue du type.
        None => (
            None,
            leaf_config
                .0
                .keys()
                .filter(|k| kind_allows_key(leaf.kind, k))
                .cloned()
                .collect(),
        ),
    };
    let mut out = Map::new();
    for key in keys {
        if is_file_meta(&key) {
            continue;
        }
        if let Some(v) = value_of(&key) {
            let v = orca_non_empty_vector(&key, v);
            out.insert(key, v);
        }
    }
    // Identifiants d'extrudeur toujours écrits par Preset::save.
    for key in [
        "print_extruder_id",
        "print_extruder_variant",
        "printer_extruder_id",
        "printer_extruder_variant",
        "filament_extruder_variant",
    ] {
        if kind_allows_key(leaf.kind, key) && !out.contains_key(key) && raw_text.contains_key(key) {
            if let Some(v) = value_of(key) {
                out.insert(key.into(), v);
            }
        }
    }
    let name = leaf.name.clone();
    if let Some(key) = settings_id_key(leaf.kind) {
        let id = if leaf.kind == PresetKind::Filament {
            Value::Array(vec![Value::String(name.clone())])
        } else {
            Value::String(name.clone())
        };
        out.insert(key.into(), id);
    }
    if let Some(parent) = &inherits {
        out.insert("inherits".into(), Value::String(parent.clone()));
    } else if leaf_is_system {
        // Lot 46c : preset système exporté comme preset utilisateur racine
        // (Orca : `inherits` vide, config complète du type).
        out.insert("inherits".into(), Value::String(String::new()));
    }
    if inherits.is_none() && leaf.kind == PresetKind::Filament {
        if let Some(id) = leaf.filament_id.as_ref().filter(|s| !s.is_empty()) {
            out.insert("filament_id".into(), Value::String(id.clone()));
        }
    }
    out.insert("name".into(), Value::String(name));
    // Toujours « User » : un preset système exporté est relu par Orca comme un
    // preset utilisateur racine autonome (un fichier « system » serait refusé
    // par l'import et par `--load-settings`).
    out.insert("from".into(), Value::String("User".into()));
    out.insert("version".into(), Value::String(ORCA_PRESET_VERSION.into()));
    out.insert(
        "type".into(),
        Value::String(
            match leaf.kind {
                PresetKind::Machine => "machine",
                PresetKind::Process => "process",
                PresetKind::Filament => "filament",
                PresetKind::MachineModel => "machine_model",
            }
            .into(),
        ),
    );
    // Clés triées comme nlohmann::json (std::map) dans les fichiers d'Orca.
    let mut sorted: Vec<(String, Value)> = out.into_iter().collect();
    sorted.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(sorted.into_iter().collect())
}

/// Lot 46c : un vecteur vide (`[]`, ex. `filament_notes` résolu depuis `""`)
/// dont le défaut d'Orca a un élément (`[""]`) est écrit avec ce défaut : Orca
/// n'écrit jamais ce vecteur vide et la CLI (`--load-filaments`,
/// `ConfigOptionVector::set_at`) s'arrête sur « Assigning from an empty
/// vector ».
fn orca_non_empty_vector(key: &str, value: Value) -> Value {
    if !matches!(&value, Value::Array(items) if items.is_empty()) {
        return value;
    }
    engine::params::get(key)
        .and_then(|def| def.default_json)
        .and_then(|text| serde_json::from_str::<Value>(text).ok())
        .filter(|d| matches!(d, Value::Array(items) if !items.is_empty()))
        .unwrap_or(value)
}

// --- Export d'un lot -----------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct BundleEntry {
    pub id: String,
    /// Valeurs éditées non enregistrées (config du type), optionnelles.
    #[serde(default)]
    pub values: Option<Map<String, Value>>,
}

#[derive(Debug, Deserialize)]
pub struct ExportBundleRequest {
    pub presets: Vec<BundleEntry>,
}

/// Nom de fichier sûr dans une archive (Orca : nom du preset + `.json`).
fn entry_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '\0' => '_',
            c => c,
        })
        .collect();
    let trimmed = cleaned.trim().trim_start_matches('.');
    if trimmed.is_empty() {
        "preset".into()
    } else {
        trimmed.to_string()
    }
}

fn zip_bytes(files: &[(String, Vec<u8>)]) -> ApiResult<Vec<u8>> {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    // Lot 46c : entrées non compressées (JSON lisibles tels quels dans
    // l'archive, comme l'ancien ZIP client ; relu à l'identique par Orca).
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    for (name, data) in files {
        zip.start_file(name.as_str(), options)
            .map_err(|_| ApiError::internal())?;
        zip.write_all(data).map_err(|_| ApiError::internal())?;
    }
    Ok(zip.finish().map_err(|_| ApiError::internal())?.into_inner())
}

/// `j.dump(1, '\t')` d'Orca : indentation par tabulation.
pub(crate) fn json_bytes(value: &Map<String, Value>) -> ApiResult<Vec<u8>> {
    let mut out = Vec::new();
    let formatter = serde_json::ser::PrettyFormatter::with_indent(b"\t");
    let mut ser = serde_json::Serializer::with_formatter(&mut out, formatter);
    serde::Serialize::serialize(value, &mut ser).map_err(|_| ApiError::internal())?;
    out.push(b'\n');
    Ok(out)
}

fn unique(used: &mut HashSet<String>, path: String) -> String {
    if used.insert(path.clone()) {
        return path;
    }
    let stem = path.trim_end_matches(".json").to_string();
    (2..)
        .map(|n| format!("{stem}_{n}.json"))
        .find(|p| used.insert(p.clone()))
        .unwrap_or(path)
}

type Exported = (Preset, Map<String, Value>);

/// Archive d'un lot à partir des fichiers de preset (voir `export_bundle`).
/// Renvoie (nom de l'archive, octets).
pub(crate) fn bundle_archive(
    printers: &[Exported],
    filaments: &[Exported],
    processes: &[Exported],
    vendor: Option<String>,
    stamp: i64,
) -> ApiResult<(String, Vec<u8>)> {
    let mut used = HashSet::new();
    let mut files: Vec<(String, Vec<u8>)> = Vec::new();
    let mut section = |files: &mut Vec<(String, Vec<u8>)>,
                       dir: &str,
                       list: &[Exported]|
     -> ApiResult<Vec<Value>> {
        let mut names = Vec::new();
        for (p, file) in list {
            let path = if dir.is_empty() {
                unique(&mut used, format!("{}.json", entry_name(&p.name)))
            } else {
                unique(&mut used, format!("{dir}/{}.json", entry_name(&p.name)))
            };
            files.push((path.clone(), json_bytes(file)?));
            names.push(Value::String(path));
        }
        Ok(names)
    };
    // Lots d'Orca réservés aux presets utilisateur : une imprimante système
    // (ou un filament système) exportée en preset autonome donne un ZIP plat.
    let user_printer = printers
        .first()
        .filter(|(p, _)| p.origin == PresetOrigin::User);
    let user_filaments = filaments
        .iter()
        .all(|(p, _)| p.origin == PresetOrigin::User);
    let (archive_name, structure) = if let Some((printer, _)) = user_printer {
        // archive_preset_bundle_to_file : une imprimante + filaments / process.
        let printer_config = section(&mut files, "printer", printers)?;
        let filament_config = section(&mut files, "filament", filaments)?;
        let process_config = section(&mut files, "process", processes)?;
        (
            format!("{}.orca_printer", entry_name(&printer.name)),
            Some(serde_json::json!({
                "version": "",
                "bundle_id": format!("offline_{}_{stamp}", printer.name),
                "bundle_type": "printer config bundle",
                "printer_preset_name": printer.name,
                "printer_config": printer_config,
                "filament_config": filament_config,
                "process_config": process_config,
            })),
        )
    } else if printers.is_empty() && processes.is_empty() && !filaments.is_empty() && user_filaments
    {
        // archive_filament_bundle_to_file : <fournisseur>/<filament>.json.
        let vendor = vendor.unwrap_or_else(|| "Custom".into());
        let paths = section(&mut files, &entry_name(&vendor), filaments)?;
        let first = filaments[0].0.name.clone();
        (
            format!("{}.orca_filament", entry_name(&first)),
            Some(serde_json::json!({
                "version": "",
                "bundle_id": format!("offline_{first}_{stamp}"),
                "bundle_type": "filament config bundle",
                "filament_name": first,
                "printer_vendor": [{ "vendor": vendor, "filament_path": paths }],
            })),
        )
    } else {
        // archive_*_preset_to_file : JSON à plat.
        section(&mut files, "", printers)?;
        section(&mut files, "", filaments)?;
        section(&mut files, "", processes)?;
        ("Presets.zip".to_string(), None)
    };
    if let Some(structure) = structure {
        let text = serde_json::to_vec(&structure).map_err(|_| ApiError::internal())?;
        files.push((BUNDLE_STRUCTURE.into(), text));
    }
    Ok((archive_name, zip_bytes(&files)?))
}

/// `POST /api/presets/export-bundle` — lot de préréglages au format d'Orca.
/// Corps : `{ presets: [{ id, values? }] }`. Réponse : archive
/// `<imprimante>.orca_printer` (une imprimante utilisateur dans le lot),
/// `<filament>.orca_filament` (filaments utilisateur seuls) ou `Presets.zip`.
/// Lot 46c : un preset système demandé est exporté en preset autonome (config
/// complète du type, `from: "User"`, `inherits: ""`), sauf s'il est l'ancêtre
/// d'un preset utilisateur du même lot (alors seulement référencé par
/// `inherits`, comme Orca).
pub async fn export_bundle(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Json(body): Json<ExportBundleRequest>,
) -> ApiResult<Response> {
    if body.presets.is_empty() || body.presets.len() > 256 {
        return Err(ApiError::validation(
            "1 à 256 préréglages attendus",
            serde_json::json!({ "field": "presets" }),
        ));
    }
    let mut printers = Vec::new();
    let mut filaments = Vec::new();
    let mut processes = Vec::new();
    let mut vendor: Option<String> = None;
    let mut seen: HashSet<String> = HashSet::new();
    let mut wanted: Vec<(Preset, Option<&Map<String, Value>>)> = Vec::new();
    for entry in &body.presets {
        let id = uuid::Uuid::parse_str(&entry.id)
            .map(PresetId::from)
            .map_err(|_| ApiError::not_found("Preset"))?;
        let preset = super::presets::load_visible(&state, user.id, id).await?;
        if !seen.insert(preset.id.to_string()) {
            continue;
        }
        if preset.kind == PresetKind::Machine && vendor.is_none() {
            vendor = preset.vendor.clone();
        }
        wanted.push((preset, entry.values.as_ref()));
    }
    // Ancêtres système des presets utilisateur du lot : déjà référencés par
    // `inherits`, pas réexportés.
    let mut ancestors: HashSet<PresetId> = HashSet::new();
    for (preset, _) in wanted
        .iter()
        .filter(|(p, _)| p.origin == PresetOrigin::User)
    {
        let chain = super::presets::load_chain_presets(&state, user.id, preset).await?;
        ancestors.extend(
            chain
                .iter()
                .filter(|p| p.origin == PresetOrigin::System)
                .map(|p| p.id),
        );
    }
    for (preset, draft) in wanted {
        if preset.origin != PresetOrigin::User && ancestors.contains(&preset.id) {
            continue;
        }
        let file = orca_preset_file(&state, user.id, &preset, draft).await?;
        match preset.kind {
            PresetKind::Machine => printers.push((preset, file)),
            PresetKind::Filament => filaments.push((preset, file)),
            PresetKind::Process => processes.push((preset, file)),
            PresetKind::MachineModel => {}
        }
    }
    if printers.is_empty() && filaments.is_empty() && processes.is_empty() {
        return Err(ApiError::validation(
            "aucun préréglage à exporter",
            serde_json::json!({ "field": "presets" }),
        ));
    }
    // L'imprimante utilisateur d'abord (lot `.orca_printer`).
    printers.sort_by_key(|(p, _): &Exported| p.origin != PresetOrigin::User);
    let stamp = time::OffsetDateTime::now_utc().unix_timestamp();
    let (archive_name, bytes) = bundle_archive(&printers, &filaments, &processes, vendor, stamp)?;
    let ascii: String = archive_name
        .chars()
        .map(|c| {
            if c.is_ascii_graphic() || c == ' ' {
                c
            } else {
                '_'
            }
        })
        .filter(|c| *c != '"')
        .collect();
    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/zip".to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!(
                    "attachment; filename=\"{ascii}\"; filename*=UTF-8''{}",
                    percent_encode(&archive_name)
                ),
            ),
        ],
        bytes,
    )
        .into_response())
}

fn percent_encode(s: &str) -> String {
    s.bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}

// --- Import d'un lot -----------------------------------------------------------

/// Un fichier de preset lu dans un lot (ou JSON seul).
#[derive(Debug, Clone)]
pub(crate) struct PresetFile {
    pub file: String,
    pub json: Map<String, Value>,
}

/// Fichiers de preset d'un lot dans l'ordre d'import : ceux listés par
/// `bundle_structure.json` (imprimante, filaments, process ; ou fournisseurs
/// d'un lot de filaments), sinon tous les `.json` de l'archive ; imprimantes
/// d'abord. Un JSON seul donne un fichier.
pub(crate) fn read_bundle(bytes: &[u8], filename: &str) -> ApiResult<Vec<PresetFile>> {
    let invalid =
        |m: &str| ApiError::validation(m.to_string(), serde_json::json!({ "file": filename }));
    if !bytes.starts_with(b"PK") {
        let json: Value =
            serde_json::from_slice(bytes).map_err(|_| invalid("JSON de préréglage illisible"))?;
        let Value::Object(json) = json else {
            return Err(invalid("JSON de préréglage invalide"));
        };
        return Ok(vec![PresetFile {
            file: filename.to_string(),
            json,
        }]);
    }
    let mut zip =
        zip::ZipArchive::new(Cursor::new(bytes)).map_err(|_| invalid("archive illisible"))?;
    if zip.len() > 1000 {
        return Err(invalid("archive trop volumineuse"));
    }
    fn read_entry(zip: &mut zip::ZipArchive<Cursor<&[u8]>>, name: &str) -> Option<Vec<u8>> {
        let f = zip.by_name(name).ok()?;
        if f.size() > 64 * 1024 * 1024 {
            return None;
        }
        let mut buf = Vec::new();
        f.take(64 * 1024 * 1024).read_to_end(&mut buf).ok()?;
        Some(buf)
    }
    let mut order: Vec<String> = Vec::new();
    if let Some(structure) = read_entry(&mut zip, BUNDLE_STRUCTURE)
        .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
    {
        for key in ["printer_config", "filament_config", "process_config"] {
            if let Some(list) = structure[key].as_array() {
                order.extend(list.iter().filter_map(|v| v.as_str().map(str::to_string)));
            }
        }
        if let Some(vendors) = structure["printer_vendor"].as_array() {
            for v in vendors {
                if let Some(list) = v["filament_path"].as_array() {
                    order.extend(list.iter().filter_map(|p| p.as_str().map(str::to_string)));
                }
            }
        }
    }
    if order.is_empty() {
        let mut names: Vec<String> = zip
            .file_names()
            .filter(|n| n.to_ascii_lowercase().ends_with(".json") && *n != BUNDLE_STRUCTURE)
            .map(str::to_string)
            .collect();
        names.sort();
        order = names;
    }
    let mut out = Vec::new();
    for name in order {
        let Some(data) = read_entry(&mut zip, &name) else {
            continue;
        };
        if let Ok(Value::Object(json)) = serde_json::from_slice::<Value>(&data) {
            out.push(PresetFile { file: name, json });
        }
    }
    out.sort_by_key(
        |f| match detect_kind(parse_declared_kind(&f.json), &f.json) {
            PresetKind::Machine => 0,
            PresetKind::Filament => 1,
            _ => 2,
        },
    );
    Ok(out)
}

pub(crate) fn parse_declared_kind(json: &Map<String, Value>) -> PresetKind {
    match json.get("type").and_then(Value::as_str) {
        Some("machine") | Some("printer") => PresetKind::Machine,
        Some("filament") => PresetKind::Filament,
        _ => PresetKind::Process,
    }
}

#[derive(Debug, Deserialize)]
pub struct ImportBundleQuery {
    #[serde(default)]
    pub filename: Option<String>,
}

/// `POST /api/presets/import-bundle?filename=…` — corps : octets du fichier
/// (`.orca_printer`, `.orca_filament`, `.orca_bundle`, `.zip` ou `.json`).
/// Réponse 200 `{ imported: [PresetDetail], skipped: [{ file, name, reason }] }`.
pub async fn import_bundle(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Query(query): Query<ImportBundleQuery>,
    body: Bytes,
) -> ApiResult<Json<Value>> {
    let filename = query.filename.unwrap_or_else(|| "presets".into());
    let files = read_bundle(&body, &filename)?;
    let mut imported: Vec<PresetDetail> = Vec::new();
    let mut skipped: Vec<Value> = Vec::new();
    for f in files {
        match import_file(&state, user.id, &f).await {
            Ok(preset) => imported.push(preset.into()),
            Err(reason) => skipped.push(serde_json::json!({
                "file": f.file,
                "name": f.json.get("name"),
                "reason": reason,
            })),
        }
    }
    Ok(Json(
        serde_json::json!({ "imported": imported, "skipped": skipped }),
    ))
}

/// Importe un fichier de preset ; `Err(raison)` si Orca l'ignorerait.
async fn import_file(state: &AppState, user: UserId, f: &PresetFile) -> Result<Preset, String> {
    let kind = detect_kind(parse_declared_kind(&f.json), &f.json);
    let stem = f
        .file
        .rsplit('/')
        .next()
        .unwrap_or(&f.file)
        .trim_end_matches(".json")
        .to_string();
    let name = f
        .json
        .get("name")
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .map(str::to_string)
        .unwrap_or(stem)
        .trim()
        .to_string();
    let inherits = f
        .json
        .get("inherits")
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .map(str::to_string);
    let fail = |e: ApiError| e.body.message.clone();
    let repo = state.storage.presets();
    let same_name = repo
        .list_by_name(kind, &name, user)
        .await
        .map_err(|e| e.to_string())?;
    if same_name.iter().any(|p| p.origin == PresetOrigin::System) {
        return Err("un préréglage système porte déjà ce nom".into());
    }
    if let Some(parent) = &inherits {
        let chain = parent_chain(state, user, kind, Some(parent))
            .await
            .map_err(fail)?;
        if chain.is_empty() {
            // import_json_presets : « can not find inherit preset … just skip ».
            return Err(format!("préréglage parent « {parent} » introuvable"));
        }
    }
    let values = import_values(state, user, kind, inherits.as_deref(), &f.json)
        .await
        .map_err(fail)?;
    if let Some(existing) = same_name
        .iter()
        .find(|p| p.origin == PresetOrigin::User && p.user_id == Some(user))
    {
        if existing.inherits == inherits {
            return repo
                .update_user_preset(user, existing.id, &name, Value::Object(values))
                .await
                .map_err(|e| e.to_string());
        }
        // Parent différent : recréé (le dépôt ne met à jour que nom / valeurs).
        repo.delete_user_preset(user, existing.id)
            .await
            .map_err(|e| e.to_string())?;
    }
    let preset = Preset {
        id: PresetId::new(),
        kind,
        name,
        origin: PresetOrigin::User,
        user_id: Some(user),
        vendor: None,
        inherits,
        instantiation: true,
        setting_id: None,
        filament_id: f
            .json
            .get("filament_id")
            .and_then(Value::as_str)
            .map(str::to_string),
        compatible_printers: None,
        renamed_from: Vec::new(),
        values: Value::Object(values),
    };
    repo.create_user_preset(user, preset)
        .await
        .map_err(|e| e.to_string())
}
