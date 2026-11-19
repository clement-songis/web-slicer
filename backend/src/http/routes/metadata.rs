//! Métadonnées 3MF d'un projet (Orca 2.4.1 : `Model::model_info`,
//! `design_info`, `profile_info` ; onglet « Projet » — Project.cpp
//! `ProjectPanel::on_reload` — et éditeur `DesignerPanel` d'Auxiliary.cpp).
//!
//! - lues à l'ouverture d'un 3MF « comme projet » (`_BBS_3MF_Importer`,
//!   `_handle_end_metadata`) et conservées dans
//!   `<uid>/project-metadata/<projet>.json` ;
//! - `GET /api/projects/{id}/metadata` → [`ProjectMetadataDto`] (champs
//!   vides si aucune) ;
//! - `PUT /api/projects/{id}/metadata` → remplace les champs éditables d'Orca
//!   (Title, Designer, License, Description, couverture) ; Copyright, Origin,
//!   profil et autres entrées `<metadata>` restent ceux du fichier ;
//! - réécrites à l'export du projet (`store_bbs_3mf`,
//!   `_add_model_file_to_archive`).

use axum::extract::{Path, State};
use axum::Json;
use engine::threemf::ProjectMetadata;
use serde_json::json;

use crate::domain::{ProjectId, UserId};
use crate::http::dto::ProjectMetadataDto;
use crate::http::error::{ApiError, ApiResult};
use crate::http::extract::CurrentUser;
use crate::http::state::AppState;

/// Licences proposées par Orca (Auxiliary.cpp, `license_list`).
pub const LICENSES: &[&str] = &[
    "", "CC0", "BY", "BY-SA", "BY-ND", "BY-NC", "BY-NC-SA", "BY-NC-ND",
];

/// Taille maximale d'un champ texte (titre, auteur, couverture).
pub const MAX_FIELD_CHARS: usize = 512;
/// Taille maximale de la description.
pub const MAX_DESCRIPTION_CHARS: usize = 64 * 1024;

/// Dossier auxiliaire des images du modèle (couverture « DesignerCover »).
pub const PICTURES_FOLDER: &str = "Model Pictures";

fn parse_project_id(raw: &str) -> ApiResult<ProjectId> {
    uuid::Uuid::parse_str(raw)
        .map(ProjectId)
        .map_err(|_| ApiError::not_found("Projet"))
}

/// Métadonnées enregistrées d'un projet (`None` si aucune ou illisibles).
pub async fn load(state: &AppState, user: UserId, project: ProjectId) -> Option<ProjectMetadata> {
    let path = state.files.project_metadata_path(user, project);
    let bytes = tokio::fs::read(&path).await.ok()?;
    match serde_json::from_slice(&bytes) {
        Ok(metadata) => Some(metadata),
        Err(error) => {
            tracing::warn!(%error, "métadonnées de projet illisibles");
            None
        }
    }
}

/// Enregistre (écriture atomique) les métadonnées d'un projet.
pub async fn store(
    state: &AppState,
    user: UserId,
    project: ProjectId,
    metadata: &ProjectMetadata,
) -> ApiResult<()> {
    let path = state.files.project_metadata_path(user, project);
    let write = async {
        if let Some(dir) = path.parent() {
            tokio::fs::create_dir_all(dir).await?;
        }
        let bytes = serde_json::to_vec(metadata).map_err(std::io::Error::other)?;
        let tmp = path.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
        tokio::fs::write(&tmp, bytes).await?;
        tokio::fs::rename(&tmp, &path).await
    };
    write.await.map_err(|error| {
        tracing::error!(%error, "écriture des métadonnées de projet");
        ApiError::internal()
    })
}

/// Supprime les métadonnées d'un projet (suppression du projet).
pub async fn remove(state: &AppState, user: UserId, project: ProjectId) {
    let _ = tokio::fs::remove_file(state.files.project_metadata_path(user, project)).await;
}

/// Auteur affiché par Orca (`ProjectPanel::on_reload`) : dernier `author` du
/// tableau JSON « Copyright », sinon Designer.
pub fn display_author(metadata: &ProjectMetadata) -> Option<String> {
    let from_copyright = serde_json::from_str::<serde_json::Value>(&metadata.copyright)
        .ok()
        .and_then(|value| {
            value.as_array().and_then(|items| {
                items
                    .iter()
                    .filter_map(|item| item.get("author").and_then(|a| a.as_str()))
                    .next_back()
                    .map(str::to_string)
            })
        })
        .filter(|a| !a.is_empty());
    from_copyright
        .or_else(|| metadata.designer.clone())
        .filter(|a| !a.is_empty())
}

/// Au moins un champ affiché par l'onglet « Projet » d'Orca.
pub fn is_meaningful(metadata: &ProjectMetadata) -> bool {
    [
        &metadata.model_name,
        &metadata.description,
        &metadata.license,
        &metadata.cover_file,
        &metadata.copyright,
    ]
    .iter()
    .any(|s| !s.is_empty())
        || metadata.designer.as_deref().is_some_and(|d| !d.is_empty())
        || metadata.profile.is_some()
}

/// Vue API des métadonnées.
pub fn to_dto(metadata: &ProjectMetadata) -> ProjectMetadataDto {
    ProjectMetadataDto {
        title: metadata.model_name.clone(),
        designer: metadata.designer.clone().unwrap_or_default(),
        license: metadata.license.clone(),
        description: metadata.description.clone(),
        cover: metadata.cover_file.clone(),
        copyright: metadata.copyright.clone(),
        origin: metadata.origin.clone(),
        display_author: display_author(metadata),
        profile: metadata.profile.clone(),
    }
}

/// Applique les champs éditables d'Orca (DesignerPanel / AuFile::on_set_cover)
/// sur les métadonnées existantes. `cover_exists` : la couverture est un
/// fichier de « Model Pictures ».
pub fn apply_edit(
    current: Option<ProjectMetadata>,
    body: &ProjectMetadataDto,
    cover_exists: bool,
) -> ApiResult<ProjectMetadata> {
    for (field, value, max) in [
        ("title", &body.title, MAX_FIELD_CHARS),
        ("designer", &body.designer, MAX_FIELD_CHARS),
        ("cover", &body.cover, MAX_FIELD_CHARS),
        ("description", &body.description, MAX_DESCRIPTION_CHARS),
    ] {
        if value.chars().count() > max {
            return Err(ApiError::validation(
                format!("champ « {field} » trop long"),
                json!({ "field": field, "max_chars": max }),
            ));
        }
    }
    if !LICENSES.contains(&body.license.as_str()) {
        return Err(ApiError::validation(
            "licence inconnue",
            json!({ "field": "license", "allowed": LICENSES }),
        ));
    }
    if !body.cover.is_empty() && !cover_exists {
        return Err(ApiError::validation(
            "couverture absente des images du modèle",
            json!({ "field": "cover", "folder": PICTURES_FOLDER }),
        ));
    }
    let mut metadata = current.unwrap_or_default();
    metadata.model_name = body.title.clone();
    metadata.description = body.description.clone();
    metadata.license = body.license.clone();
    metadata.cover_file = body.cover.clone();
    // Model::SetDesigner(name, "") : identifiant MakerWorld effacé au changement.
    let designer = body.designer.trim();
    if designer.is_empty() {
        metadata.designer = None;
        metadata.designer_user_id = None;
    } else if metadata.designer.as_deref() != Some(designer) {
        metadata.designer = Some(designer.to_string());
        metadata.designer_user_id = None;
    }
    Ok(metadata)
}

/// `GET /api/projects/{id}/metadata`.
pub async fn get(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(raw): Path<String>,
) -> ApiResult<Json<ProjectMetadataDto>> {
    let id = parse_project_id(&raw)?;
    state.storage.projects().get(user.id, id).await?;
    let metadata = load(&state, user.id, id).await.unwrap_or_default();
    Ok(Json(to_dto(&metadata)))
}

/// `PUT /api/projects/{id}/metadata`.
pub async fn put(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    Path(raw): Path<String>,
    Json(body): Json<ProjectMetadataDto>,
) -> ApiResult<Json<ProjectMetadataDto>> {
    let id = parse_project_id(&raw)?;
    state.storage.projects().get(user.id, id).await?;
    let cover_exists = if body.cover.is_empty() {
        false
    } else {
        match state
            .files
            .auxiliary_path(user.id, id, &format!("{PICTURES_FOLDER}/{}", body.cover))
        {
            Ok(path) => tokio::fs::metadata(&path).await.is_ok_and(|m| m.is_file()),
            Err(_) => false,
        }
    };
    let metadata = apply_edit(load(&state, user.id, id).await, &body, cover_exists)?;
    store(&state, user.id, id, &metadata).await?;
    Ok(Json(to_dto(&metadata)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dto(title: &str, license: &str) -> ProjectMetadataDto {
        ProjectMetadataDto {
            title: title.into(),
            license: license.into(),
            ..Default::default()
        }
    }

    #[test]
    fn auteur_affiche_comme_orca() {
        let mut m = ProjectMetadata {
            designer: Some("Bob".into()),
            ..Default::default()
        };
        assert_eq!(display_author(&m).as_deref(), Some("Bob"));
        m.copyright = r#"[{"author":"A"},{"author":"Zed"}]"#.into();
        assert_eq!(display_author(&m).as_deref(), Some("Zed"));
        m.copyright = "pas du json".into();
        assert_eq!(display_author(&m).as_deref(), Some("Bob"));
    }

    #[test]
    fn edition_conserve_les_champs_du_fichier() {
        let mut items = std::collections::BTreeMap::new();
        items.insert(
            "Thumbnail_Middle".to_string(),
            "/Metadata/plate_1.png".to_string(),
        );
        let current = ProjectMetadata {
            model_name: "Ancien".into(),
            origin: "original".into(),
            copyright: "[]".into(),
            designer: Some("Bob".into()),
            designer_user_id: Some("42".into()),
            metadata_items: items.clone(),
            ..Default::default()
        };
        let mut body = dto("Nouveau", "BY-SA");
        body.designer = "Bob".into();
        body.origin = "ignoré".into();
        let out = apply_edit(Some(current), &body, false).unwrap();
        assert_eq!(out.model_name, "Nouveau");
        assert_eq!(out.license, "BY-SA");
        assert_eq!(out.origin, "original");
        assert_eq!(out.copyright, "[]");
        assert_eq!(out.designer_user_id.as_deref(), Some("42"));
        assert_eq!(out.metadata_items, items);
        body.designer = "Alice".into();
        let out = apply_edit(Some(out), &body, false).unwrap();
        assert_eq!(out.designer.as_deref(), Some("Alice"));
        assert_eq!(out.designer_user_id, None);
    }

    #[test]
    fn edition_valide_licence_couverture_et_tailles() {
        assert!(apply_edit(None, &dto("t", "MIT"), false).is_err());
        for license in LICENSES {
            assert!(apply_edit(None, &dto("t", license), false).is_ok());
        }
        let mut body = dto("t", "");
        body.cover = "cover.png".into();
        assert!(apply_edit(None, &body, false).is_err());
        assert_eq!(
            apply_edit(None, &body, true).unwrap().cover_file,
            "cover.png"
        );
        assert!(apply_edit(None, &dto(&"x".repeat(MAX_FIELD_CHARS + 1), ""), false).is_err());
    }
}
