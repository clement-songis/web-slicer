//! Objets hors du volume d'impression (lot 43, audit 42 I1).
//!
//! Orca ne tranche que les instances entièrement dans le volume du plateau
//! (ModelInstance::is_printable, état calculé par
//! ModelInstance::calc_print_volume_state / BuildVolume) et grise
//! « Trancher » quand un objet est à cheval sur le bord ou dépasse la hauteur
//! imprimable (GLCanvas3D, EWarning::ObjectClashed : « Following objects are
//! laid over the boundary of plate or exceeds the height limit »).
//!
//! Le serveur calcule cet état lui-même à partir de la scène (maillages,
//! transformations, volumes « pièce ») et du plateau de la config tranchée
//! (printable_area, printable_height), au lieu de se fier au drapeau
//! off_bed du client (jamais posé par l'éditeur) : POST …/slice refuse
//! (422), POST …/validate rapporte une erreur par objet, et
//! POST …/object-states expose l'état de chaque objet à l'interface.
//!
//! Repère : positions de la scène relatives au plateau de l'objet (convention
//! de l'éditeur, plate-layout.ts) ; les plateaux multiples n'ont donc pas de
//! décalage ici. Les 3MF d'Orca (positions monde, plateau k décalé de
//! PartPlateList::compute_shape_position) sont ramenés dans ce repère à
//! l'ouverture et décalés à nouveau à l'export (projects.rs / export.rs).

use axum::extract::{Path, State};
use axum::Json;
use serde::Serialize;
use serde_json::{json, Value};
use ts_rs::TS;

use super::slice::{plate_targets, request_active_presets, request_scene, resolve_slice_config};
use crate::build_volume::{BuildVolume, PrintVolumeState};
use crate::domain::{ProjectId, UserId};
use crate::http::dto::SliceRequest;
use crate::http::error::{ApiError, ApiResult};
use crate::http::extract::CurrentUser;
use crate::http::state::AppState;
use crate::slicing::ScenePlacement;

/// Message d'Orca pour un objet à cheval / trop haut (GLCanvas3D.cpp,
/// construct_error_string), msgids traduits par le catalogue du client.
pub(crate) fn clashed_message(name: &str) -> String {
    format!(
        "Following objects are laid over the boundary of plate or exceeds the height limit:\n{name}\nPlease solve the problem by moving it totally on or off the plate, and confirming that the height is within the build volume."
    )
}

/// État d'un objet de scène vis-à-vis du volume d'impression de son plateau.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct ObjectPlacement {
    /// scene.objects[].id.
    pub object_id: String,
    /// Nom affiché de l'objet.
    pub object_name: String,
    /// Plateau (0-based) de l'objet.
    pub plate_index: i64,
    /// Objet imprimable (un objet non imprimable n'est jamais tranché ni bloquant).
    pub printable: bool,
    /// inside | partly_outside | outside ; absent si la géométrie n'est pas
    /// encore disponible (conversion en cours, modèle absent).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub state: Option<PrintVolumeState>,
}

impl From<ScenePlacement> for ObjectPlacement {
    fn from(p: ScenePlacement) -> Self {
        Self {
            object_id: p.id,
            object_name: p.name,
            plate_index: i64::from(p.plate_index),
            printable: p.printable,
            state: p.state,
        }
    }
}

impl ObjectPlacement {
    /// Objet qui empêche de trancher son plateau : imprimable et pas
    /// entièrement dans le volume.
    pub fn blocks_slicing(&self) -> bool {
        self.printable
            && matches!(
                self.state,
                Some(PrintVolumeState::PartlyOutside | PrintVolumeState::Outside)
            )
    }
}

/// Réponse de POST /api/projects/{id}/object-states.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct ObjectStatesResponse {
    /// Un état par objet de la scène (plateau ciblé ou tous les plateaux).
    pub objects: Vec<ObjectPlacement>,
    /// Plateau utilisé (mm) : largeur, profondeur, hauteur imprimable
    /// (0 = illimitée) ; absent si la config ne décrit aucun plateau.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub bed: Option<[f64; 3]>,
}

/// Volume d'impression de la config figée (snapshot resolved), défauts du
/// registre à défaut (plateau 200 × 200, comme FullPrintConfig d'Orca).
pub(crate) fn build_volume_of(resolved: &Value) -> Option<BuildVolume> {
    let mut config = crate::slicing::config_from_snapshot(resolved).ok()?;
    super::presets::fill_registry_defaults(&mut config);
    BuildVolume::from_config(&config)
}

/// États des objets de la scène (plateau plate_index ou tous).
pub(crate) async fn object_placements(
    state: &AppState,
    user: UserId,
    project_id: ProjectId,
    scene: &Value,
    resolved: &Value,
    plate_index: Option<i64>,
) -> ApiResult<Vec<ObjectPlacement>> {
    let Some(volume) = build_volume_of(resolved) else {
        return Ok(Vec::new());
    };
    let placements = crate::slicing::scene_object_placements(
        state,
        user,
        project_id,
        scene,
        plate_index.map(|i| i.max(0) as u32),
        &volume,
    )
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "état des objets / volume d'impression");
        ApiError::internal()
    })?;
    Ok(placements.into_iter().map(ObjectPlacement::from).collect())
}

/// Refus 422 lisible d'un plateau dont des objets ne sont pas entièrement
/// dans le volume (ids, noms et états dans details.objects).
pub(crate) fn outside_error(plate_index: i64, blocking: &[&ObjectPlacement]) -> ApiError {
    let names: Vec<String> = blocking
        .iter()
        .map(|o| {
            let state = match o.state {
                Some(PrintVolumeState::PartlyOutside) => {
                    "à cheval sur le bord du plateau ou trop haut"
                }
                _ => "hors du plateau",
            };
            format!("« {} » ({state})", o.object_name)
        })
        .collect();
    ApiError::validation(
        format!(
            "objet hors du volume d'impression : {} — repositionnez-le entièrement sur le plateau avant de trancher",
            names.join(", ")
        ),
        json!({
            "plate_index": plate_index,
            "reason": "object_outside_plate",
            "objects": blocking
                .iter()
                .map(|o| json!({ "id": o.object_id, "name": o.object_name, "state": o.state }))
                .collect::<Vec<_>>(),
        }),
    )
}

/// POST /api/projects/{id}/object-states — état de chaque objet vis-à-vis
/// du volume d'impression (même corps que POST …/slice : plateau, scène et
/// presets non enregistrés), sans moteur : réponse rapide, pour colorer les
/// objets « hors plateau » et griser « Trancher » comme Orca.
pub async fn object_states(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(project_raw): Path<String>,
    Json(body): Json<SliceRequest>,
) -> ApiResult<Json<ObjectStatesResponse>> {
    let project_id = uuid::Uuid::parse_str(&project_raw)
        .map(ProjectId)
        .map_err(|_| ApiError::not_found("Projet"))?;
    let project = state.storage.projects().get(user.id, project_id).await?; // 404, SC-008
    let scene = request_scene(&body, &project.scene)?;
    // Plateau hors limites : 422 comme …/slice.
    plate_targets(scene, body.plate_index)?;
    let config = resolve_slice_config(
        &state,
        user.id,
        scene,
        request_active_presets(&body, &project.active_presets)?,
        &body,
    )
    .await?;
    let objects = object_placements(
        &state,
        user.id,
        project_id,
        scene,
        &config.resolved,
        body.plate_index,
    )
    .await?;
    let bed = build_volume_of(&config.resolved).map(|v| {
        let (w, d) = v.size();
        [w, d, v.max_print_height()]
    });
    Ok(Json(ObjectStatesResponse { objects, bed }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_printable_objects_not_inside_block_slicing() {
        let object = |printable: bool, state: Option<PrintVolumeState>| ObjectPlacement {
            object_id: "o".into(),
            object_name: "Cube".into(),
            plate_index: 0,
            printable,
            state,
        };
        assert!(object(true, Some(PrintVolumeState::PartlyOutside)).blocks_slicing());
        assert!(object(true, Some(PrintVolumeState::Outside)).blocks_slicing());
        assert!(!object(true, Some(PrintVolumeState::Inside)).blocks_slicing());
        assert!(!object(false, Some(PrintVolumeState::Outside)).blocks_slicing());
        assert!(!object(true, None).blocks_slicing());
    }

    #[test]
    fn bed_defaults_to_the_registry_plate() {
        let volume = build_volume_of(&json!({})).expect("plateau par défaut");
        assert_eq!(volume.size(), (200.0, 200.0));
        let volume = build_volume_of(&json!({
            "printable_area": ["0x0", "350x0", "350x350", "0x350"],
            "printable_height": 340
        }))
        .expect("plateau Voron");
        assert_eq!(volume.size(), (350.0, 350.0));
        assert_eq!(volume.max_print_height(), 340.0);
    }

    #[test]
    fn outside_error_lists_ids_and_states() {
        let o = ObjectPlacement {
            object_id: "o1".into(),
            object_name: "Cube".into(),
            plate_index: 0,
            printable: true,
            state: Some(PrintVolumeState::PartlyOutside),
        };
        let error = outside_error(0, &[&o]);
        let body = serde_json::to_value(&error.body).unwrap();
        assert_eq!(body["details"]["objects"][0]["id"], "o1");
        assert_eq!(body["details"]["objects"][0]["state"], "partly_outside");
        assert!(body["message"].as_str().unwrap().contains("Cube"), "{body}");
    }
}
