//! « Add Handy models » d'Orca (`MenuFactory::append_submenu_add_handy_model`,
//! GUI_Factories.cpp:576) : modèles livrés dans `resources/handy_models`,
//! chargés comme géométrie (`LoadStrategy::LoadModel`), éventuellement suivis
//! d'un arrangement, et pour « Orca String Hell » d'une suggestion de réglage.
//!
//! - `GET /api/handy-models` : catalogue (même table et même ordre qu'Orca).
//! - `POST /api/projects/{id}/handy-models/{key}` : ajoute les fichiers du
//!   modèle au projet (un `Model` par fichier, conversion moteur asynchrone
//!   comme un upload) ; la réponse dit au client s'il doit arranger et quelle
//!   suggestion proposer.

use std::path::PathBuf;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Serialize;
use ts_rs::TS;

use super::models::{detect_format, spawn_conversion, validate_content, write_model_record};
use crate::domain::ProjectId;
use crate::http::dto::ModelResponse;
use crate::http::error::{ApiError, ApiResult};
use crate::http::extract::CurrentUser;
use crate::http::state::AppState;

/// Entrée de la table d'Orca (libellé, fichiers, comportement).
struct HandyModel {
    key: &'static str,
    label: &'static str,
    files: &'static [&'static str],
    arrange_after_import: bool,
    is_stringhell: bool,
}

/// Table d'Orca 2.4.1 (GUI_Factories.cpp:590), dans le même ordre.
const HANDY_MODELS: &[HandyModel] = &[
    HandyModel {
        key: "orca-cube",
        label: "Orca Cube",
        files: &["OrcaCube_v2.drc", "OrcaPlug_v2.drc"],
        arrange_after_import: true,
        is_stringhell: false,
    },
    HandyModel {
        key: "orcasliced-combo",
        label: "OrcaSliced Combo",
        files: &["OrcaSliced.3mf", "OrcaCube_v2.drc", "OrcaPlug_v2.drc"],
        arrange_after_import: true,
        is_stringhell: false,
    },
    HandyModel {
        key: "orca-tolerance-test",
        label: "Orca Tolerance Test",
        files: &["OrcaToleranceTest.drc"],
        arrange_after_import: false,
        is_stringhell: false,
    },
    HandyModel {
        key: "3dbenchy",
        label: "3DBenchy",
        files: &["3DBenchy.drc"],
        arrange_after_import: false,
        is_stringhell: false,
    },
    HandyModel {
        key: "cali-cat",
        label: "Cali Cat",
        files: &["calicat.drc"],
        arrange_after_import: false,
        is_stringhell: false,
    },
    HandyModel {
        key: "autodesk-fdm-test",
        label: "Autodesk FDM Test",
        files: &["ksr_fdmtest_v4.drc"],
        arrange_after_import: false,
        is_stringhell: false,
    },
    HandyModel {
        key: "voron-cube",
        label: "Voron Cube",
        files: &["Voron_Design_Cube_v7.drc"],
        arrange_after_import: false,
        is_stringhell: false,
    },
    HandyModel {
        key: "stanford-bunny",
        label: "Stanford Bunny",
        files: &["Stanford_Bunny.drc"],
        arrange_after_import: false,
        is_stringhell: false,
    },
    HandyModel {
        key: "orca-string-hell",
        label: "Orca String Hell",
        files: &["Orca_stringhell.drc"],
        arrange_after_import: false,
        is_stringhell: true,
    },
];

/// Suggestion de réglage proposée après l'ajout (Orca : « String Hell »).
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct HandyModelSuggestion {
    /// Message d'Orca (texte d'origine, à traduire côté client).
    pub message: String,
    /// Condition d'affichage sur le preset process édité : toutes les clés
    /// doivent valoir « vrai » (booléen `1`) ou être strictement positives.
    pub when_all: Vec<String>,
    /// Valeurs à appliquer au preset process si l'utilisateur accepte.
    #[ts(type = "Record<string, string>")]
    pub apply: serde_json::Map<String, serde_json::Value>,
}

/// Entrée du catalogue.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct HandyModelInfo {
    pub key: String,
    /// Libellé d'Orca (anglais, à traduire côté client).
    pub label: String,
    pub files: Vec<String>,
    /// Arranger le plateau après l'ajout (Orca : Orca Cube, OrcaSliced Combo).
    pub arrange_after_import: bool,
    #[ts(optional)]
    pub suggestion: Option<HandyModelSuggestion>,
}

/// Réponse de `POST /api/projects/{id}/handy-models/{key}`.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct HandyModelAdded {
    pub handy_model: HandyModelInfo,
    /// Un modèle par fichier, dans l'ordre d'Orca (conversion asynchrone,
    /// événement WS `model.converted` comme pour un upload).
    pub models: Vec<ModelResponse>,
}

fn stringhell_suggestion() -> HandyModelSuggestion {
    let mut apply = serde_json::Map::new();
    apply.insert("min_width_top_surface".into(), "0".into());
    HandyModelSuggestion {
        message:
            "This model features text embossment on the top surface. For optimal results, it is \
                  advisable to set the 'One Wall Threshold (min_width_top_surface)' to 0 for the \
                  'Only One Wall on Top Surfaces' to work best."
                .into(),
        when_all: vec!["only_one_wall_top".into(), "min_width_top_surface".into()],
        apply,
    }
}

fn info(m: &HandyModel) -> HandyModelInfo {
    HandyModelInfo {
        key: m.key.into(),
        label: m.label.into(),
        files: m.files.iter().map(|f| (*f).to_string()).collect(),
        arrange_after_import: m.arrange_after_import,
        suggestion: m.is_stringhell.then(stringhell_suggestion),
    }
}

/// `resources/handy_models` : `HANDY_MODELS_DIR`, sinon voisin du répertoire
/// des profils système (`resources/profiles`).
fn handy_models_dir(state: &AppState) -> PathBuf {
    if let Ok(dir) = std::env::var("HANDY_MODELS_DIR") {
        if !dir.is_empty() {
            return PathBuf::from(dir);
        }
    }
    state
        .profiles_dir
        .parent()
        .map(|p| p.join("handy_models"))
        .unwrap_or_else(|| PathBuf::from("handy_models"))
}

/// `GET /api/handy-models` — catalogue des modèles pratiques d'Orca.
pub async fn list(CurrentUser(_user): CurrentUser) -> Json<Vec<HandyModelInfo>> {
    Json(HANDY_MODELS.iter().map(info).collect())
}

/// `POST /api/projects/{id}/handy-models/{key}` — ajoute un modèle pratique.
pub async fn add(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path((project_raw, key)): Path<(String, String)>,
) -> ApiResult<(StatusCode, Json<HandyModelAdded>)> {
    let project_id = uuid::Uuid::parse_str(&project_raw)
        .map(ProjectId)
        .map_err(|_| ApiError::not_found("Projet"))?;
    state.storage.projects().get(user.id, project_id).await?;
    let model = HANDY_MODELS
        .iter()
        .find(|m| m.key == key)
        .ok_or_else(|| ApiError::not_found("Modèle pratique"))?;

    // Lecture de tous les fichiers avant toute écriture : un fichier manquant
    // n'ajoute rien au projet.
    let dir = handy_models_dir(&state);
    let mut files = Vec::with_capacity(model.files.len());
    for name in model.files {
        let path = dir.join(name);
        let bytes = tokio::fs::read(&path).await.map_err(|e| {
            tracing::error!(error = %e, path = %path.display(), "modèle pratique introuvable");
            ApiError::internal()
        })?;
        let format = detect_format(name).ok_or_else(ApiError::internal)?;
        validate_content(format, &bytes).map_err(|reason| {
            tracing::error!(%reason, file = name, "modèle pratique invalide");
            ApiError::internal()
        })?;
        files.push((name.to_string(), format, bytes));
    }

    let mut models = Vec::with_capacity(files.len());
    for (name, format, bytes) in files {
        let record = write_model_record(&state, user.id, project_id, format, name, &bytes).await?;
        spawn_conversion(&state, record.clone());
        models.push(record.into());
    }
    Ok((
        StatusCode::CREATED,
        Json(HandyModelAdded {
            handy_model: info(model),
            models,
        }),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_matches_orca_order_and_files_exist_in_vendored_resources() {
        let labels: Vec<_> = HANDY_MODELS.iter().map(|m| m.label).collect();
        assert_eq!(
            labels,
            [
                "Orca Cube",
                "OrcaSliced Combo",
                "Orca Tolerance Test",
                "3DBenchy",
                "Cali Cat",
                "Autodesk FDM Test",
                "Voron Cube",
                "Stanford Bunny",
                "Orca String Hell"
            ]
        );
        let dir = PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../vendor/OrcaSlicer/resources/handy_models"
        ));
        for m in HANDY_MODELS {
            for f in m.files {
                assert!(dir.join(f).is_file(), "{f} absent de {}", dir.display());
                assert!(detect_format(f).is_some(), "{f} : format détecté");
            }
        }
        let arranged: Vec<_> = HANDY_MODELS
            .iter()
            .filter(|m| m.arrange_after_import)
            .map(|m| m.key)
            .collect();
        assert_eq!(arranged, ["orca-cube", "orcasliced-combo"]);
    }
}
