//! Imprimantes Moonraker (contrat http-api.md, T075/US8) :
//!   - CRUD `/api/printers` (clé API **chiffrée au repos**, jamais renvoyée) ;
//!   - `POST …/test` : relais `GET /server/info` (FR-060) ;
//!   - `POST …/upload` : envoi d'un G-code du compte (gcode_id, start_now, FR-061) ;
//!   - `GET …/status` : état instantané ;
//!   - `POST …/pause|resume|cancel` : contrôle d'impression.
//!
//! Isolation (SC-008) : une imprimante — ou un G-code — d'un autre compte est
//! traitée comme inexistante (404). Toute panne de l'hôte distant devient un 502
//! propre (`printer_unreachable`, FR-062), jamais une 500.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde_json::Value;
use std::path::Path as FsPath;
use std::sync::Arc;

use std::collections::BTreeMap;

use crate::adapters::moonraker::MoonrakerClient;
use crate::domain::repo::NewPrinter;
use crate::domain::{GcodeId, PresetKind, Printer, PrinterId};
use crate::http::dto::{
    PrinterCatalogModel, PrinterCatalogVariant, PrinterCatalogVendor, PrinterResponse,
    PrinterStatusResponse, PrinterUploadResponse, SavePrinterRequest, TestPrinterResponse,
    UploadToPrinterRequest,
};
use crate::http::error::{ApiError, ApiResult};
use crate::http::extract::CurrentUser;
use crate::http::state::AppState;

pub(crate) fn parse_printer_id(raw: &str) -> ApiResult<PrinterId> {
    uuid::Uuid::parse_str(raw)
        .map(PrinterId)
        .map_err(|_| ApiError::not_found("Imprimante"))
}

fn parse_preset_id(raw: &str) -> ApiResult<crate::domain::PresetId> {
    uuid::Uuid::parse_str(raw)
        .map(crate::domain::PresetId)
        .map_err(|_| ApiError::validation("machine_preset_id invalide", serde_json::json!({})))
}

/// Traduit un corps de requête en `NewPrinter`, en chiffrant la clé API.
fn to_new_printer(state: &AppState, body: SavePrinterRequest) -> ApiResult<NewPrinter> {
    let machine_preset_id = parse_preset_id(&body.machine_preset_id)?;
    let api_key = body
        .api_key
        .filter(|k| !k.is_empty())
        .map(|k| state.secrets.encrypt(&k));
    // URL vide (champ laissé blanc dans le formulaire) = imprimante non connectée.
    let moonraker_url = body.moonraker_url.filter(|u| !u.is_empty());
    Ok(NewPrinter {
        name: body.name,
        moonraker_url,
        api_key,
        machine_preset_id,
    })
}

/// Charge une imprimante du compte et construit son client Moonraker (clé API
/// déchiffrée à la volée, jamais persistée en clair).
pub(crate) async fn load_client(
    state: &AppState,
    user: crate::domain::UserId,
    id: PrinterId,
) -> ApiResult<(Printer, MoonrakerClient)> {
    let printer = state.storage.printers().get(user, id).await?; // 404, SC-008
                                                                 // Imprimante déclarée mais sans connexion réseau : aucun appel Moonraker
                                                                 // possible → 409 `printer_not_connected` (l'UI propose de la connecter).
    let Some(url) = printer.moonraker_url.clone() else {
        return Err(ApiError::printer_not_connected(
            "Imprimante non connectée (aucune URL Moonraker)",
        ));
    };
    let api_key = printer
        .api_key
        .as_deref()
        .and_then(|enc| state.secrets.decrypt(enc));
    let client = MoonrakerClient::new(url, api_key);
    Ok((printer, client))
}

/// `GET /api/printers` — imprimantes déclarées du compte.
pub async fn list(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<PrinterResponse>>> {
    let printers = state.storage.printers().list(user.id).await?;
    Ok(Json(
        printers.into_iter().map(PrinterResponse::from).collect(),
    ))
}

/// `GET /api/printer-catalog` — catalogue des modèles d'imprimante
/// sélectionnables (presets machine instanciables, système + ceux de
/// l'utilisateur), groupés par marque puis modèle avec leurs variantes de buse.
/// Source unique du wizard d'onboarding et de la page de gestion (Phase 14).
pub async fn catalog(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<PrinterCatalogVendor>>> {
    let presets = state
        .storage
        .presets()
        .list_by_kind(PresetKind::Machine, user.id)
        .await?;

    // Vignette comme Orca (WebGuideDialog.cpp:1369, Plater.cpp:4094) :
    // `<marque>/<printer_model>_cover.png`, `printer_model` étant résolu sur la
    // chaîne `inherits` des presets machine (le nom du preset contient la buse).
    let values_by_name: std::collections::HashMap<String, (Option<String>, Value)> = presets
        .iter()
        .map(|p| (p.name.clone(), (p.inherits.clone(), p.values.clone())))
        .collect();
    let printer_model_of = |name: &str| -> Option<String> {
        let mut current = name.to_string();
        for _ in 0..16 {
            let (inherits, values) = values_by_name.get(&current)?;
            if let Some(m) = values.get("printer_model").and_then(Value::as_str) {
                if !m.is_empty() {
                    return Some(m.to_string());
                }
            }
            current = inherits.clone()?;
        }
        None
    };
    let mut cover_of_model: BTreeMap<(String, String), String> = BTreeMap::new();

    // marque → modèle → variantes ; BTreeMap pour un tri stable (marque, modèle).
    let mut by_vendor: BTreeMap<String, BTreeMap<String, Vec<PrinterCatalogVariant>>> =
        BTreeMap::new();
    for p in presets {
        // Les presets de base abstraits (`fdm_*_common`) ne sont pas
        // sélectionnables : on ne garde que les presets instanciables.
        if !p.instantiation {
            continue;
        }
        let vendor = p.vendor.clone().unwrap_or_else(|| "Autre".to_string());
        let (model, nozzle) = split_machine_name(&p.name);
        cover_of_model
            .entry((vendor.clone(), model.clone()))
            .or_insert_with(|| {
                printer_model_of(&p.name)
                    .into_iter()
                    .chain(std::iter::once(model.clone()))
                    .map(|m| format!("{vendor}/{m}_cover.png"))
                    .find(|c| state.profiles_dir.join(c).is_file())
                    .unwrap_or_default()
            });
        by_vendor
            .entry(vendor)
            .or_default()
            .entry(model)
            .or_default()
            .push(PrinterCatalogVariant {
                machine_preset_id: p.id.to_string(),
                nozzle,
            });
    }

    let mut vendors: Vec<PrinterCatalogVendor> = by_vendor
        .into_iter()
        .map(|(vendor, models)| {
            let models = models
                .into_iter()
                .map(|(model, mut variants)| {
                    // Buses triées par diamètre croissant.
                    variants.sort_by(|a, b| {
                        parse_nozzle(&a.nozzle)
                            .partial_cmp(&parse_nozzle(&b.nozzle))
                            .unwrap_or(std::cmp::Ordering::Equal)
                    });
                    // Défaut : 0.4 mm si présente, sinon la plus petite buse.
                    let default_machine_preset_id = variants
                        .iter()
                        .find(|v| v.nozzle == "0.4")
                        .or_else(|| variants.first())
                        .map(|v| v.machine_preset_id.clone())
                        .unwrap_or_default();
                    // Vide si Orca n'a pas de vignette (imprimantes « Custom ») :
                    // le client affiche le pictogramme sans requête 404.
                    let cover = cover_of_model
                        .get(&(vendor.clone(), model.clone()))
                        .cloned()
                        .unwrap_or_default();
                    PrinterCatalogModel {
                        vendor: vendor.clone(),
                        model,
                        cover,
                        variants,
                        default_machine_preset_id,
                    }
                })
                .collect();
            PrinterCatalogVendor { vendor, models }
        })
        .collect();

    // « Custom » (imprimantes génériques : Marlin, Klipper, RRF, Repetier…) en tête
    // du catalogue : accès direct quand l'imprimante réelle n'est pas reconnue. Tri
    // stable → les autres marques conservent leur ordre alphabétique.
    vendors.sort_by_key(|v| v.vendor != "Custom");

    Ok(Json(vendors))
}

/// Décompose « `<Modèle> <buse> nozzle` » → (modèle, buse). Si le nom ne suit
/// pas la convention OrcaSlicer, le modèle est le nom entier et la buse est vide.
/// `pub(crate)` : la garde de possession du tranchage (slice.rs) s'en sert pour
/// autoriser n'importe quelle buse d'un **modèle** possédé.
pub(crate) fn split_machine_name(name: &str) -> (String, String) {
    if let Some(stem) = name.strip_suffix(" nozzle") {
        if let Some((model, nozzle)) = stem.rsplit_once(' ') {
            let numeric = !nozzle.is_empty()
                && nozzle.chars().all(|c| c.is_ascii_digit() || c == '.')
                && nozzle.contains(|c: char| c.is_ascii_digit());
            if numeric {
                return (model.to_string(), nozzle.to_string());
            }
        }
    }
    (name.to_string(), String::new())
}

/// Diamètre de buse en nombre pour le tri (les non-numériques en dernier).
fn parse_nozzle(n: &str) -> f64 {
    n.parse().unwrap_or(f64::MAX)
}

/// `POST /api/printers` — déclare une imprimante (clé API chiffrée au repos).
pub async fn create(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Json(body): Json<SavePrinterRequest>,
) -> ApiResult<(StatusCode, Json<PrinterResponse>)> {
    let new_printer = to_new_printer(&state, body)?;
    // Écriture détachée de la requête : si le client se déconnecte (navigation
    // juste après « Enregistrer »), axum abandonne le handler ; la tâche, elle,
    // va au bout et l'imprimante déclarée n'est pas perdue.
    let storage = state.storage.clone();
    let user_id = user.id;
    let printer =
        tokio::spawn(async move { storage.printers().create(user_id, new_printer).await })
            .await
            .map_err(|_| ApiError::internal())??;
    Ok((StatusCode::CREATED, Json(PrinterResponse::from(printer))))
}

/// `GET /api/printers/{id}` — détail (404 pour un autre compte, SC-008).
pub async fn get(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<PrinterResponse>> {
    let id = parse_printer_id(&id)?;
    let printer = state.storage.printers().get(user.id, id).await?;
    Ok(Json(PrinterResponse::from(printer)))
}

/// `PUT /api/printers/{id}` — met à jour la déclaration (404, SC-008).
pub async fn update(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<SavePrinterRequest>,
) -> ApiResult<Json<PrinterResponse>> {
    let id = parse_printer_id(&id)?;
    let mut new_printer = to_new_printer(&state, body)?;
    // Mise à jour partielle : si la requête ne fournit pas de clé API, on préserve
    // celle déjà stockée (chiffrée) au lieu de l'effacer silencieusement. Permet de
    // repointer le preset machine d'une imprimante (édition des réglages sur la page
    // de gestion) sans reperdre le secret Moonraker.
    if new_printer.api_key.is_none() {
        let existing = state.storage.printers().get(user.id, id).await?; // 404, SC-008
        new_printer.api_key = existing.api_key;
    }
    let printer = state
        .storage
        .printers()
        .update(user.id, id, new_printer)
        .await?;
    Ok(Json(PrinterResponse::from(printer)))
}

/// `DELETE /api/printers/{id}` — retire la déclaration (404, SC-008).
pub async fn delete(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    let id = parse_printer_id(&id)?;
    state.storage.printers().delete(user.id, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /api/printers/{id}/test` — test de connexion (`GET /server/info`).
pub async fn test(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<TestPrinterResponse>> {
    let id = parse_printer_id(&id)?;
    let (_printer, client) = load_client(&state, user.id, id).await?;
    let info = client.server_info().await?; // 502 propre si injoignable (FR-062)
    Ok(Json(TestPrinterResponse {
        connected: info.klippy_connected,
        klippy_state: info.klippy_state,
        moonraker_version: info.moonraker_version,
    }))
}

/// `POST /api/printers/{id}/upload` — envoie un G-code du compte (FR-061).
pub async fn upload(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<UploadToPrinterRequest>,
) -> ApiResult<Json<PrinterUploadResponse>> {
    let id = parse_printer_id(&id)?;
    let gcode_id = uuid::Uuid::parse_str(&body.gcode_id)
        .map(GcodeId)
        .map_err(|_| ApiError::not_found("G-code"))?;

    // Le G-code doit appartenir au compte (404 sinon, SC-008).
    let gcode = state.storage.gcodes().get(user.id, gcode_id).await?;
    let bytes = state
        .files
        .read(FsPath::new(&gcode.file_path))
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "lecture du G-code (envoi imprimante)");
            ApiError::internal()
        })?;
    // Nom d'Orca (`Plater::send_gcode_legacy` : même nom que l'export,
    // `filename_format` évalué par libslic3r au tranchage), base = nom du
    // projet ; job ou projet supprimé depuis : base « Untitled » comme Orca.
    let project_name = match state.storage.jobs().get(user.id, gcode.job_id).await {
        Ok(job) => state
            .storage
            .projects()
            .get(user.id, job.project_id)
            .await
            .map(|p| p.name)
            .unwrap_or_default(),
        Err(_) => String::new(),
    };
    let filename = crate::gcode_filename::for_gcode(&gcode.stats, &bytes, &project_name);

    let (_printer, client) = load_client(&state, user.id, id).await?;
    let result = client.upload(&filename, bytes, body.start_now).await?;
    Ok(Json(PrinterUploadResponse::from(result)))
}

/// Slug ASCII sûr pour un nom de fichier Moonraker : minuscules, accents
/// latins repliés, tout autre caractère → `-` (séparateurs fusionnés), 60
/// caractères max. Vide → `web-slicer`.
pub(crate) fn safe_slug(raw: &str) -> String {
    let mut out = String::new();
    for c in raw.chars().flat_map(char::to_lowercase) {
        let mapped = match c {
            'a'..='z' | '0'..='9' => Some(c),
            'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' => Some('a'),
            'ç' => Some('c'),
            'è' | 'é' | 'ê' | 'ë' => Some('e'),
            'ì' | 'í' | 'î' | 'ï' => Some('i'),
            'ñ' => Some('n'),
            'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' => Some('o'),
            'ù' | 'ú' | 'û' | 'ü' => Some('u'),
            'ý' | 'ÿ' => Some('y'),
            'æ' => Some('a'),
            'œ' => Some('o'),
            _ => None,
        };
        match mapped {
            Some(ch) => out.push(ch),
            None if !out.is_empty() && !out.ends_with('-') => out.push('-'),
            None => {}
        }
        if out.len() >= 60 {
            break;
        }
    }
    let slug = out.trim_matches('-').to_string();
    if slug.is_empty() {
        "web-slicer".into()
    } else {
        slug
    }
}

/// `GET /api/printers/{id}/status` — état instantané (interrogation Moonraker).
/// Amorce aussi le relais WS `printer.status` : les mises à jour en direct
/// commencent à circuler sur le canal du compte (T076, idempotent).
pub async fn status(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<PrinterStatusResponse>> {
    let id = parse_printer_id(&id)?;
    let (_printer, client) = load_client(&state, user.id, id).await?;
    let status = client.query_status().await?;
    // Webcam déclarée côté Moonraker (facultatif : absence ou erreur → None).
    let webcam_url = client.webcam_url().await.ok().flatten();
    // Suivi en direct : abonnement WebSocket relayé au canal du propriétaire.
    state
        .relays
        .ensure(Arc::clone(&state.events), user.id, id, client);
    let mut response = PrinterStatusResponse::from(status);
    response.webcam_url = webcam_url;
    Ok(Json(response))
}

/// `GET /api/printers/{id}/files` — fichiers G-code de l'imprimante
/// (relais `/server/files/list?root=gcodes`), plus récents d'abord.
pub async fn files(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<Vec<crate::http::dto::PrinterFileResponse>>> {
    let id = parse_printer_id(&id)?;
    let (_printer, client) = load_client(&state, user.id, id).await?;
    let mut files = client.list_files().await?;
    files.sort_by(|a, b| b.modified.total_cmp(&a.modified));
    Ok(Json(
        files
            .into_iter()
            .map(|f| crate::http::dto::PrinterFileResponse {
                path: f.path,
                size: f.size,
                modified: f.modified,
            })
            .collect(),
    ))
}

/// `POST /api/printers/{id}/print` — lance un fichier présent sur
/// l'imprimante (relais `/printer/print/start`).
pub async fn print(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<crate::http::dto::StartPrintRequest>,
) -> ApiResult<StatusCode> {
    let filename = body.filename.trim().to_string();
    if filename.is_empty()
        || filename.len() > 255
        || filename.contains("..")
        || filename.chars().any(char::is_control)
    {
        return Err(ApiError::validation(
            "nom de fichier invalide",
            serde_json::json!({ "filename": body.filename }),
        ));
    }
    control(&state, user.id, &id, |c| async move {
        c.start_print(&filename).await
    })
    .await
}

/// `POST /api/printers/{id}/gcode` — exécute un script G-code (relais
/// `/printer/gcode/script`) : consignes, déplacements, ventilateur. Script
/// vide = sonde de capacité (204 sans appel). Borné (longueur, lignes,
/// caractères imprimables) : voir `validate_gcode_script`.
pub async fn gcode(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<crate::http::dto::GcodeScriptRequest>,
) -> ApiResult<StatusCode> {
    crate::adapters::moonraker::validate_gcode_script(&body.script)
        .map_err(|e| ApiError::validation(e, serde_json::json!({})))?;
    let printer_id = parse_printer_id(&id)?;
    let (_printer, client) = load_client(&state, user.id, printer_id).await?;
    let script = body.script.trim().to_string();
    if !script.is_empty() {
        client.gcode_script(&script).await?;
    }
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /api/printers/{id}/pause`.
pub async fn pause(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    control(&state, user.id, &id, |c| async move { c.pause().await }).await
}

/// `POST /api/printers/{id}/resume`.
pub async fn resume(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    control(&state, user.id, &id, |c| async move { c.resume().await }).await
}

/// `POST /api/printers/{id}/cancel`.
pub async fn cancel(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    control(&state, user.id, &id, |c| async move { c.cancel().await }).await
}

/// Fabrique commune des contrôles d'impression : résout l'imprimante (404),
/// exécute l'action et mappe toute panne distante en 502 (FR-062).
async fn control<F, Fut>(
    state: &AppState,
    user: crate::domain::UserId,
    raw_id: &str,
    action: F,
) -> ApiResult<StatusCode>
where
    F: FnOnce(MoonrakerClient) -> Fut,
    Fut: std::future::Future<Output = Result<(), crate::adapters::moonraker::MoonrakerError>>,
{
    let id = parse_printer_id(raw_id)?;
    let (_printer, client) = load_client(state, user, id).await?;
    action(client).await?;
    Ok(StatusCode::NO_CONTENT)
}
