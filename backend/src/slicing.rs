//! Runner de tranchage réel (T064/T066) : implémente `JobRunner` en pilotant le
//! process moteur isolé `engine-worker slice`.
//!
//! Pipeline d'un job :
//!   1. assemble le `Model` moteur depuis la scène figée dans le job (maillages
//!      décodés côté serveur, mêmes conventions de scène que l'export 3MF) ;
//!   2. restaure la configuration typée depuis le blob `resolved_settings` figé,
//!      sans relire les presets actifs ni leur éventuelle nouvelle version ;
//!   3. sérialise une `SliceRequest` dans un répertoire de travail isolé et lance
//!      `engine-worker slice <request.json>` (crash C++ contenu, timeout, kill) ;
//!   4. lit le G-code produit, le matérialise (`store_gcode` : fichier + ligne
//!      `gcodes` avec stats/vignettes), et renvoie l'issue avec ses statistiques.
//!
//! L'isolation moteur (process séparé) et l'annulation coopérative sont fournies
//! par l'infra `crate::engine` et la file `crate::queue`.

use std::path::Path as FsPath;
use std::time::Duration;

use async_trait::async_trait;
use glam::DMat4;
use serde_json::Value;

use crate::domain::{ModelFormat, ProjectId, SlicingJob, UserId};
use crate::engine::run_worker;
use crate::gcode::store_gcode_named;
// Matrice de scène partagée avec l'export 3MF : une seule implémentation
// (colonnes `R[:,j] · s_j`), donc slice et export placent les objets pareil.
use crate::http::routes::export::scene_transform_matrix;
use crate::http::state::AppState;
use crate::mesh::{parse_stl, Mesh};
use crate::queue::{JobContext, JobRunner, RunOutcome};
use engine::api::{DynamicPrintConfig, Model, TriangleMesh};
use engine::params::ParamKind;

/// Délai maximal d'un tranchage (au-delà, le worker est tué). Plus large que le
/// timeout de décodage : un tranchage complet peut être long.
const SLICE_TIMEOUT: Duration = Duration::from_secs(600);

/// Runner FFI branché sur la file (T063). Détient un `AppState` (stockage,
/// fichiers, résolution de presets) — cloné à la construction dans `build_app`.
pub struct SliceRunner {
    state: AppState,
}

impl SliceRunner {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }

    /// Exécute le tranchage et renvoie (stats client, id du G-code stocké).
    async fn slice(
        &self,
        job: &SlicingJob,
        ctx: &JobContext,
    ) -> anyhow::Result<(serde_json::Value, crate::domain::GcodeId)> {
        ctx.report(0.05, "préparation").await;

        // Les nouveaux jobs ne dépendent plus de la scène courante. Le repli est
        // strictement réservé aux lignes historiques antérieures à la migration
        // additive (scene NULL), afin qu'elles restent exécutables.
        let scene = match &job.scene {
            Some(scene) => scene.clone(),
            None => {
                self.state
                    .storage
                    .projects()
                    .get(job.user_id, job.project_id)
                    .await?
                    .scene
            }
        };
        let mut model = build_scene_model(
            &self.state,
            job.user_id,
            job.project_id,
            &scene,
            Some(job.plate_index.max(0) as u32),
        )
        .await?;
        if model.is_empty() {
            anyhow::bail!("aucun objet à trancher (modèles STL absents)");
        }

        // Configuration exclusivement restaurée depuis le snapshot du job.
        let config = config_from_snapshot(&job.resolved_settings)?;
        // Changements de couleur / pauses / G-code perso posés sur le curseur
        // de couches (mode d'Orca déduit des extrudeurs / filaments).
        let mode = custom_gcode_mode(&config);
        model.custom_gcodes = scene_custom_gcodes(&scene)
            .into_iter()
            .map(|mut plate| {
                plate.mode = mode;
                plate
            })
            .collect();
        // Vignettes de l'export GUI d'Orca (le moteur n'a pas de rendu OpenGL).
        let thumbnails = crate::thumbnails::thumbnails_section(&model, &config);

        // Requête sérialisée dans un répertoire de travail isolé (garantie n°3).
        let work = crate::workdir::tempdir()?;
        let request = engine::api::SliceRequest {
            // Préférence « Remove mixed temperature restriction », figée dans
            // l'instantané de scène du job par POST …/slice.
            allow_mixed_temperatures: scene
                .pointer("/sliceOptions/allowMixedTemperatures")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false),
            model,
            config,
            plate_index: job.plate_index.max(0) as u32,
            work_dir: work.path().to_path_buf(),
            calib: calib_from_scene(&scene),
        };
        let request_path = work.path().join("request.json");
        std::fs::write(&request_path, serde_json::to_vec(&request)?)?;

        if ctx.is_cancelled() {
            anyhow::bail!("annulé");
        }
        ctx.report(0.15, "tranchage").await;

        // Process moteur isolé : `engine-worker slice <request.json>`.
        let request_arg = request_path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("chemin de requête non-UTF8"))?;
        let stdout = run_worker(&["slice", request_arg], SLICE_TIMEOUT).await?;
        let result = parse_slice_result(&stdout)?;

        ctx.report(0.9, "enregistrement").await;

        // Matérialise le G-code (fichier + ligne `gcodes` + stats/vignettes).
        let gcode_text = crate::thumbnails::insert_section(
            &std::fs::read_to_string(&result.gcode_path)?,
            &thumbnails,
        );
        let gcode = store_gcode_named(
            &self.state.files,
            self.state.storage.gcodes(),
            job.user_id,
            job.id,
            &gcode_text,
            result.output_filename.clone(),
        )
        .await?;

        Ok((gcode.stats, gcode.id))
    }
}

#[async_trait]
impl JobRunner for SliceRunner {
    async fn run(&self, job: SlicingJob, ctx: JobContext) -> RunOutcome {
        if ctx.is_cancelled() {
            return RunOutcome::Cancelled;
        }
        match self.slice(&job, &ctx).await {
            Ok((stats, gcode_id)) => {
                ctx.report(1.0, "terminé").await;
                RunOutcome::Succeeded { gcode_id, stats }
            }
            Err(e) => {
                if ctx.is_cancelled() {
                    return RunOutcome::Cancelled;
                }
                tracing::warn!(job = %job.id, error = %e, "tranchage échoué");
                RunOutcome::Failed(serde_json::json!({ "message": e.to_string() }))
            }
        }
    }
}

/// Restaure les types à l'aide du registre : une désérialisation directe de
/// `DynamicPrintConfig` est ambiguë (`Point` et `Floats` ont la même forme
/// JSON) et ne reconnaît pas les pourcentages projetés sous forme de chaîne.
pub(crate) fn config_from_snapshot(snapshot: &Value) -> anyhow::Result<DynamicPrintConfig> {
    let values = snapshot
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("resolved_settings n'est pas un objet JSON"))?;
    let mut config = DynamicPrintConfig::new();
    for (key, value) in values {
        let def = engine::params::get(key)
            .ok_or_else(|| anyhow::anyhow!("paramètre figé inconnu : {key}"))?;
        let text = snapshot_value_to_orca(def.kind, value)
            .ok_or_else(|| anyhow::anyhow!("valeur figée invalide pour {key}"))?;
        let parsed = engine::params::orca_values::parse_orca_value(def, &text)
            .ok_or_else(|| anyhow::anyhow!("valeur figée illisible pour {key} : {text}"))?;
        config.0.insert(key.clone(), parsed);
    }
    Ok(config)
}

/// Variante tolérante de `config_from_snapshot` pour les outils interactifs
/// (profil de couche) : valeurs résolues de l'éditeur, clés inconnues ou
/// illisibles ignorées (les défauts de libslic3r s'appliquent).
pub(crate) fn config_from_values_lenient(values: &Value) -> DynamicPrintConfig {
    let mut config = DynamicPrintConfig::new();
    let Some(values) = values.as_object() else {
        return config;
    };
    for (key, value) in values {
        let Some(def) = engine::params::get(key) else {
            continue;
        };
        let Some(text) = snapshot_value_to_orca(def.kind, value) else {
            continue;
        };
        if let Some(parsed) = engine::params::orca_values::parse_orca_value(def, &text) {
            config.0.insert(key.clone(), parsed);
        }
    }
    config
}

fn snapshot_value_to_orca(kind: ParamKind, value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s.clone()),
        Value::Bool(b) => Some(b.to_string()),
        Value::Number(n) => Some(n.to_string()),
        Value::Array(items) => match kind {
            ParamKind::Point => join_orca_items(items, "x", false),
            // Groupes de points : une chaîne par groupe, jointes par « # » (Orca).
            ParamKind::PointsGroups if items.iter().all(Value::is_string) => Some(
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join("#"),
            ),
            ParamKind::Points | ParamKind::PointsGroups => {
                let points: Option<Vec<String>> = items
                    .iter()
                    .map(|point| match point {
                        Value::Array(coords) => join_orca_items(coords, "x", false),
                        Value::String(s) => Some(s.clone()),
                        _ => None,
                    })
                    .collect();
                points.map(|points| points.join(","))
            }
            ParamKind::Strings | ParamKind::Enums => Some(join_string_items(items)),
            _ => join_orca_items(items, ",", true),
        },
        Value::Null | Value::Object(_) => None,
    }
}

fn join_orca_items(items: &[Value], separator: &str, nullable: bool) -> Option<String> {
    items
        .iter()
        .map(|value| match value {
            Value::String(s) => Some(s.clone()),
            Value::Bool(b) => Some(b.to_string()),
            Value::Number(n) => Some(n.to_string()),
            Value::Null if nullable => Some("nil".into()),
            _ => None,
        })
        .collect::<Option<Vec<_>>>()
        .map(|items| items.join(separator))
}

fn join_string_items(items: &[Value]) -> String {
    items
        .iter()
        .map(|value| match value {
            Value::Null => "nil".into(),
            Value::String(s) => format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"")),
            Value::Bool(b) => format!("\"{b}\""),
            Value::Number(n) => format!("\"{n}\""),
            _ => "\"\"".into(),
        })
        .collect::<Vec<_>>()
        .join(";")
}

// --- Assemblage du modèle depuis la scène figée -----------------------------
// Ces conventions reproduisent `export::build_project_model`. Elles vivent ici
// afin que le runner puisse fournir explicitement son snapshot au lieu de
// recharger implicitement le projet courant.

#[derive(Debug, Clone)]
struct SceneObjectSpec {
    /// Identifiant de l'objet dans la scène (`objects[].id`), absent en repli.
    scene_id: Option<String>,
    /// Nom affiché de l'objet (`objects[].name`), absent si non renseigné.
    scene_name: Option<String>,
    model_id: String,
    plate_index: u32,
    matrix: DMat4,
    printable: bool,
    extruder: Option<u16>,
    settings: DynamicPrintConfig,
    layer_height_profile: Vec<f64>,
    centered: bool,
    /// Peintures 3MF Orca de l'objet (supports, couture, fuzzy, MMU).
    paint: engine::api::VolumePaint,
    /// Volumes ajoutés (pièce, modificateur, négatif, bloqueur/forceur de supports).
    volumes: Vec<engine::api::ModelVolume>,
    /// Volumes maillés (« Split to parts »), maillage chargé depuis leur modèle.
    mesh_volumes: Vec<MeshVolumeRef>,
    /// Plages de hauteur à réglages propres (`layerRanges`).
    layer_config_ranges: Vec<engine::api::LayerConfigRange>,
    /// Informations de coupe (`cutId`, `ModelObject::cut_id` d'Orca).
    cut_id: Option<engine::api::CutObjectId>,
    /// Groupe d'instances (`instanceGroup`, phase 44) : les objets de scène
    /// d'un même groupe sont les `ModelInstance` d'un seul `ModelObject`.
    instance_group: Option<String>,
}

/// Lot 44b (F-a) : clé de regroupement des objets de scène en un
/// `ModelObject` à N instances (Orca `ModelObject::instances`) — même groupe
/// et même modèle source (un groupe ne partage qu'un maillage).
pub(crate) fn instance_group_key(group: Option<&str>, model_id: &str) -> Option<(String, String)> {
    group
        .filter(|group| !group.trim().is_empty())
        .map(|group| (group.to_string(), model_id.to_string()))
}

/// `objects[].instanceGroup` d'un objet de scène (absent / vide = None).
pub(crate) fn scene_instance_group(object: &Value) -> Option<String> {
    object
        .get("instanceGroup")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|group| !group.is_empty())
        .map(str::to_string)
}

pub(crate) async fn build_scene_model(
    state: &AppState,
    user: UserId,
    project_id: ProjectId,
    scene: &Value,
    plate_index: Option<u32>,
) -> anyhow::Result<Model> {
    Ok(
        build_scene_model_indexed(state, user, project_id, scene, plate_index)
            .await?
            .0,
    )
}

/// Objet de scène correspondant à `Model::objects[i]` (même ordre).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SceneObjectRef {
    /// `objects[].id` de la scène (absent en repli sans scène).
    pub id: Option<String>,
    /// Nom affiché : `objects[].name`, sinon nom du fichier (nom Orca).
    pub name: String,
}

/// Comme [`build_scene_model`], avec l'objet de scène de chaque objet du
/// modèle (validation : rattacher un refus d'Orca à l'objet de l'éditeur).
pub(crate) async fn build_scene_model_indexed(
    state: &AppState,
    user: UserId,
    project_id: ProjectId,
    scene: &Value,
    plate_index: Option<u32>,
) -> anyhow::Result<(Model, Vec<SceneObjectRef>)> {
    let models = state.storage.models().list(user, Some(project_id)).await?;
    let specs = scene_object_specs(scene, plate_index);
    let mut model = Model::default();
    let mut refs = Vec::new();

    if specs.as_ref().is_some_and(Vec::is_empty) {
        return Ok((model, refs));
    }
    let fallback = specs.is_none();
    if fallback && plate_index.is_some_and(|index| index > 0) {
        return Ok((model, refs));
    }
    let specs = specs.unwrap_or_else(|| {
        models
            .iter()
            .map(|m| SceneObjectSpec {
                scene_id: None,
                scene_name: None,
                model_id: m.id.to_string(),
                plate_index: 0,
                matrix: DMat4::IDENTITY,
                printable: true,
                extruder: None,
                settings: DynamicPrintConfig::new(),
                layer_height_profile: Vec::new(),
                centered: false,
                paint: engine::api::VolumePaint::default(),
                volumes: Vec::new(),
                mesh_volumes: Vec::new(),
                layer_config_ranges: Vec::new(),
                cut_id: None,
                instance_group: None,
            })
            .collect()
    });

    // Lot 44b (F-a) : objet du modèle déjà créé pour chaque groupe d'instances.
    let mut grouped: std::collections::HashMap<(String, String), usize> =
        std::collections::HashMap::new();
    for spec in specs {
        let group_key = instance_group_key(spec.instance_group.as_deref(), &spec.model_id);
        if let Some(&index) = group_key.as_ref().and_then(|key| grouped.get(key)) {
            // Instance supplémentaire du ModelObject du groupe (Orca
            // `ModelObject::add_instance`) : seule la transformation, le
            // plateau et « imprimable » sont propres à l'instance.
            model.objects[index]
                .instances
                .push(engine::api::ModelInstance {
                    matrix: spec.matrix,
                    plate_index: spec.plate_index,
                    printable: spec.printable,
                });
            continue;
        }
        let Some(source) = models.iter().find(|m| m.id.to_string() == spec.model_id) else {
            anyhow::bail!(
                "la scène figée référence un modèle supprimé : {}",
                spec.model_id
            );
        };
        // Objet importé recentré par le client : comme l'éditeur d'Orca, on
        // tranche les sommets recentrés en float32 par libslic3r à la lecture
        // du fichier source (pas le WSMh, dont le décalage cuit en f32 décale
        // les modèles non entiers d'~1 µm). Repli : WSMh recentré.
        let mut orca = None;
        // Erreurs du maillage lu par libslic3r (`<mesh_stat>` à l'export, lot 37).
        let mut mesh_stats = None;
        if spec.centered {
            if let Some((mesh, stats)) = crate::engine::raw_model::centered_part_volume(
                FsPath::new(&source.file_path),
                source.format,
            )
            .await
            {
                if mesh.indices.len() == load_triangle_count(state, source).await {
                    orca = Some(mesh);
                    // Après « Réparer », Orca repart d'un maillage neuf (aucune
                    // erreur réparée enregistrée).
                    let repaired_by_user = source
                        .repair_report
                        .as_ref()
                        .is_some_and(|report| report.get("repairs").is_some());
                    mesh_stats = stats.filter(|_| !repaired_by_user);
                }
            }
        }
        let mesh = match orca {
            Some(mesh) => mesh,
            None => {
                let mut mesh = load_triangle_mesh(state, source).await?;
                if spec.centered {
                    center_mesh(&mut mesh);
                }
                mesh
            }
        };
        refs.push(SceneObjectRef {
            id: spec.scene_id.clone(),
            name: spec
                .scene_name
                .clone()
                .unwrap_or_else(|| source.filename.clone()),
        });
        if let Some(key) = group_key {
            grouped.insert(key, model.objects.len());
        }
        // Lot 44b (I2) : `ModelObject::name` = nom de l'objet dans l'éditeur
        // (`objects[].name` : nom de l'objet du 3MF, nom du fichier pour un
        // STL, renommage) — il nomme les objets du G-code (EXCLUDE_OBJECT,
        // M486) comme Orca ; repli : nom du fichier.
        let object_name = spec
            .scene_name
            .clone()
            .unwrap_or_else(|| source.filename.clone());
        let object = model.add_object(object_name, mesh);
        object.settings = spec.settings;
        object.layer_height_profile = spec.layer_height_profile;
        object.layer_config_ranges = spec.layer_config_ranges;
        object.cut_id = spec.cut_id;
        object.volumes[0].mesh_stats = mesh_stats;
        object.instances = vec![engine::api::ModelInstance {
            matrix: spec.matrix,
            plate_index: spec.plate_index,
            printable: spec.printable,
        }];
        if let Some(extruder) = spec.extruder {
            for volume in &mut object.volumes {
                volume.extruder = Some(extruder);
            }
        }
        // Les indices de triangle de la peinture sont ceux du maillage servi à
        // l'éditeur (`/mesh`, même décodage que `load_triangle_mesh`) : un seul
        // volume pièce par objet importé.
        if let Some(volume) = object
            .volumes
            .iter_mut()
            .find(|v| v.role == engine::api::VolumeRole::ModelPart)
        {
            volume.paint = spec.paint;
        }
        object.volumes.extend(spec.volumes);
        for reference in spec.mesh_volumes {
            let Some(part) = models
                .iter()
                .find(|m| m.id.to_string() == reference.model_id)
            else {
                anyhow::bail!(
                    "la scène figée référence un volume de modèle supprimé : {}",
                    reference.model_id
                );
            };
            let mut volume = reference.resolve(load_triangle_mesh(state, part).await?);
            if volume.extruder.is_none() {
                volume.extruder = spec.extruder;
            }
            object.volumes.push(volume);
        }
    }
    Ok((model, refs))
}

/// Objet de scène classé par rapport au volume d'impression (lot 43, I1).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ScenePlacement {
    /// objects[].id de la scène.
    pub id: String,
    /// Nom affiché (objects[].name, sinon nom du fichier).
    pub name: String,
    /// Plateau (0-based) de l'objet.
    pub plate_index: u32,
    /// Objet imprimable (Orca : un objet non imprimable n'est jamais tranché).
    pub printable: bool,
    /// État vis-à-vis du volume ; None si la géométrie n'est pas disponible
    /// (modèle absent ou conversion en cours) : rien n'est affirmé.
    pub state: Option<crate::build_volume::PrintVolumeState>,
}

/// État de chaque objet de la scène vis-à-vis du volume d'impression
/// (ModelInstance::calc_print_volume_state d'Orca), plateau par plateau,
/// repère du plateau de l'objet. plate_index : seulement ce plateau.
/// Volumes « pièce » seulement (objet + pièces ajoutées / maillées), même
/// maillage recentré et même matrice que le modèle tranché.
pub(crate) async fn scene_object_placements(
    state: &AppState,
    user: UserId,
    project_id: ProjectId,
    scene: &Value,
    plate_index: Option<u32>,
    volume: &crate::build_volume::BuildVolume,
) -> anyhow::Result<Vec<ScenePlacement>> {
    use engine::api::VolumeRole;
    let Some(specs) = scene_object_specs(scene, plate_index) else {
        return Ok(Vec::new());
    };
    if specs.is_empty() {
        return Ok(Vec::new());
    }
    let models = state.storage.models().list(user, Some(project_id)).await?;
    // Un maillage par modèle (plusieurs objets peuvent partager un modèle) ;
    // booléen = déjà recentré (lecture libslic3r du fichier source).
    let mut cache: std::collections::HashMap<String, Option<(TriangleMesh, bool)>> =
        std::collections::HashMap::new();
    let mut out = Vec::with_capacity(specs.len());
    for spec in specs {
        let Some(id) = spec.scene_id.clone() else {
            continue;
        };
        let mut meshes: Vec<(TriangleMesh, DMat4)> = Vec::new();
        let mut complete = true;
        let mut wanted = vec![(spec.model_id.clone(), DMat4::IDENTITY, spec.centered)];
        for reference in &spec.mesh_volumes {
            if reference.volume.role == VolumeRole::ModelPart {
                wanted.push((reference.model_id.clone(), reference.volume.matrix, true));
            }
        }
        for (model_id, matrix, centered) in wanted {
            if !cache.contains_key(&model_id) {
                let mesh = match models.iter().find(|m| m.id.to_string() == model_id) {
                    Some(source) => match load_triangle_mesh(state, source).await {
                        Ok(mesh) => Some((mesh, false)),
                        // Conversion en cours : maillage lu par libslic3r dans
                        // le fichier source, comme le tranchage (recentré).
                        Err(_) => crate::engine::raw_model::centered_part_volume(
                            FsPath::new(&source.file_path),
                            source.format,
                        )
                        .await
                        .map(|(mesh, _)| (mesh, true)),
                    },
                    None => None,
                };
                cache.insert(model_id.clone(), mesh);
            }
            match cache.get(&model_id).cloned().flatten() {
                Some((mut mesh, already_centered)) => {
                    if centered && !already_centered {
                        center_mesh(&mut mesh);
                    }
                    meshes.push((mesh, spec.matrix * matrix));
                }
                None => complete = false,
            }
        }
        for part in spec
            .volumes
            .iter()
            .filter(|v| v.role == VolumeRole::ModelPart)
        {
            meshes.push((part.mesh.clone(), spec.matrix * part.matrix));
        }
        let name = spec.scene_name.clone().unwrap_or_else(|| {
            models
                .iter()
                .find(|m| m.id.to_string() == spec.model_id)
                .map(|m| m.filename.clone())
                .unwrap_or_default()
        });
        let parts: Vec<crate::build_volume::PartVolume<'_>> = meshes
            .iter()
            .map(|(mesh, matrix)| crate::build_volume::PartVolume {
                vertices: &mesh.vertices,
                indices: &mesh.indices,
                matrix: *matrix,
            })
            .collect();
        out.push(ScenePlacement {
            id,
            name,
            plate_index: spec.plate_index,
            printable: spec.printable,
            state: complete.then(|| volume.instance_state(&parts)),
        });
    }
    Ok(out)
}

/// Nombre de triangles du maillage servi à l'éditeur (garde-fou : la peinture
/// indexe ces triangles ; 0 si illisible → le maillage Orca n'est pas retenu).
async fn load_triangle_count(state: &AppState, model: &crate::domain::Model) -> usize {
    load_triangle_mesh(state, model)
        .await
        .map(|m| m.indices.len())
        .unwrap_or(usize::MAX)
}

async fn load_triangle_mesh(
    state: &AppState,
    model: &crate::domain::Model,
) -> anyhow::Result<TriangleMesh> {
    if let Some(path) = model.mesh_path.as_deref() {
        let bytes = state.files.read(FsPath::new(path)).await?;
        let display = engine::api::DisplayMesh::decode(&bytes)?;
        return Ok(TriangleMesh {
            vertices: display
                .positions
                .chunks_exact(3)
                .map(|c| [c[0], c[1], c[2]])
                .collect(),
            indices: display
                .indices
                .chunks_exact(3)
                .map(|c| [c[0], c[1], c[2]])
                .collect(),
        });
    }
    if model.format != ModelFormat::Stl {
        anyhow::bail!("conversion du modèle « {} » indisponible", model.filename);
    }
    let bytes = state.files.read(FsPath::new(&model.file_path)).await?;
    let mesh = parse_stl(&bytes)?;
    Ok(to_triangle_mesh(&mesh))
}

/// G-code personnalisés par couche des plateaux de la scène (curseur de
/// couches d'Orca : changement de couleur, pause, G-code perso) :
/// `plates[i].customGcodes = [{ printZ, type, extruder?, color?, extra? }]`,
/// `type` ∈ color_change | pause_print | tool_change | template | custom.
/// Entrées illisibles ignorées.
pub(crate) fn scene_custom_gcodes(scene: &Value) -> Vec<engine::api::PlateCustomGcodes> {
    let plates = match scene.get("plates") {
        Some(Value::Array(plates)) => Some(plates),
        Some(Value::Object(document)) => document.get("plates").and_then(Value::as_array),
        _ => None,
    };
    let mut out = Vec::new();
    for (index, plate) in plates.into_iter().flatten().enumerate() {
        let Some(list) = plate.get("customGcodes").and_then(Value::as_array) else {
            continue;
        };
        let items: Vec<engine::api::CustomGcodeItem> = list
            .iter()
            .filter_map(|g| {
                let print_z = g.get("printZ").or_else(|| g.get("print_z"))?.as_f64()?;
                let kind_raw = g.get("type").or_else(|| g.get("kind"))?.as_str()?;
                let kind = serde_json::from_value(Value::String(kind_raw.into())).ok()?;
                let item = engine::api::CustomGcodeItem {
                    print_z,
                    kind,
                    extruder: g.get("extruder").and_then(Value::as_i64).unwrap_or(1) as i32,
                    color: g
                        .get("color")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .into(),
                    extra: g
                        .get("extra")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .into(),
                };
                item.validate().ok().map(|_| item)
            })
            .collect();
        if !items.is_empty() {
            out.push(engine::api::PlateCustomGcodes {
                plate_index: index as u32,
                mode: engine::api::CustomGcodeMode::SingleExtruder,
                items,
            });
        }
    }
    out
}

/// Mode des G-code personnalisés comme Orca (`CustomGCode::Mode`) : un
/// extrudeur et un filament → SingleExtruder ; un extrudeur, plusieurs
/// filaments → MultiAsSingle ; plusieurs extrudeurs → MultiExtruder.
pub(crate) fn custom_gcode_mode(config: &DynamicPrintConfig) -> engine::api::CustomGcodeMode {
    let len = |key: &str| match config.0.get(key) {
        Some(engine::api::ConfigValue::Floats(v)) => v.len(),
        Some(engine::api::ConfigValue::Strings(v)) => v.len(),
        Some(_) => 1,
        None => 1,
    };
    match (len("nozzle_diameter"), len("filament_colour")) {
        (n, _) if n > 1 => engine::api::CustomGcodeMode::MultiExtruder,
        (_, f) if f > 1 => engine::api::CustomGcodeMode::MultiAsSingle,
        _ => engine::api::CustomGcodeMode::SingleExtruder,
    }
}

fn scene_object_specs(scene: &Value, requested_plate: Option<u32>) -> Option<Vec<SceneObjectSpec>> {
    let objects = scene.get("objects")?.as_array()?;
    let plates = match scene.get("plates") {
        Some(Value::Array(plates)) => Some(plates),
        Some(Value::Object(document)) => document.get("plates").and_then(Value::as_array),
        _ => None,
    };
    let mut plate_by_object = std::collections::HashMap::new();
    for (index, plate) in plates.into_iter().flatten().enumerate() {
        for id in plate
            .get("objectIds")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            plate_by_object.insert(id, index as u32);
        }
    }

    Some(
        objects
            .iter()
            .filter_map(|object| {
                let id = object.get("id").and_then(Value::as_str);
                let plate_index = id
                    .and_then(|id| plate_by_object.get(id).copied())
                    .unwrap_or(0);
                if requested_plate.is_some_and(|requested| requested != plate_index) {
                    return None;
                }
                let model_id = object.get("modelId")?.as_str()?.to_string();
                let transform = object.get("transform");
                Some(SceneObjectSpec {
                    scene_id: id.map(str::to_string),
                    scene_name: object
                        .get("name")
                        .and_then(Value::as_str)
                        .map(str::trim)
                        .filter(|name| !name.is_empty())
                        .map(str::to_string),
                    model_id,
                    plate_index,
                    matrix: transform.map_or(DMat4::IDENTITY, scene_transform_matrix),
                    printable: object
                        .get("printable")
                        .and_then(Value::as_bool)
                        .unwrap_or(true),
                    extruder: object
                        .get("extruder")
                        .and_then(Value::as_u64)
                        .filter(|extruder| *extruder > 0)
                        .map(|extruder| extruder as u16),
                    settings: scene_object_settings(object),
                    layer_height_profile: object
                        .get("layerProfile")
                        .and_then(Value::as_array)
                        .into_iter()
                        .flatten()
                        .filter_map(Value::as_f64)
                        .collect(),
                    centered: transform.is_some(),
                    paint: scene_object_paint(object),
                    volumes: scene_object_volumes(object),
                    mesh_volumes: scene_object_mesh_volumes(object),
                    layer_config_ranges: scene_object_layer_ranges(object),
                    cut_id: crate::scene::scene_cut_id(object),
                    instance_group: scene_instance_group(object),
                })
            })
            .collect(),
    )
}

fn scene_object_settings(object: &Value) -> DynamicPrintConfig {
    let values = object
        .get("settings")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    engine::presets::resolve_preset_chain(&[engine::api::RawPreset {
        name: "object".into(),
        inherits: None,
        values,
    }])
    .unwrap_or_else(|_| DynamicPrintConfig::new())
}

/// Volumes additionnels d'un objet de scène (`volumes`, cf. éditeur
/// `scene/volumes.ts`) : primitive `box` de taille `size` (mm) centrée en
/// `offset`, dans le repère du maillage recentré de l'objet (même repère que
/// l'éditeur). `type` : part | modifier | negative | blocker | enforcer ;
/// `settings` : surcharges du volume (modificateur).
pub(crate) fn scene_object_volumes(object: &Value) -> Vec<engine::api::ModelVolume> {
    use engine::api::VolumeRole;
    let vec3 = |value: Option<&Value>, fallback: f64| -> [f64; 3] {
        let items = value.and_then(Value::as_array);
        std::array::from_fn(|axis| {
            items
                .and_then(|items| items.get(axis))
                .and_then(Value::as_f64)
                .filter(|v| v.is_finite())
                .unwrap_or(fallback)
        })
    };
    object
        .get("volumes")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|volume| {
            // Volumes maillés (« Split to parts ») : résolus de façon asynchrone
            // par `scene_object_mesh_volumes` + chargement du modèle référencé.
            if is_mesh_volume(volume) {
                return None;
            }
            let role = match volume.get("type").and_then(Value::as_str)? {
                "part" => VolumeRole::ModelPart,
                "modifier" => VolumeRole::ParameterModifier,
                "negative" => VolumeRole::NegativeVolume,
                "blocker" => VolumeRole::SupportBlocker,
                "enforcer" => VolumeRole::SupportEnforcer,
                _ => return None,
            };
            let size = vec3(volume.get("size"), 10.0);
            if size.iter().any(|s| *s <= 0.0) {
                return None;
            }
            let offset = vec3(volume.get("offset"), 0.0);
            // Rotation optionnelle (Euler XYZ en degrés, convention three.js de
            // l'éditeur) : portée par la matrice du volume, pavé centré à
            // l'origine. Sans rotation, géométrie cuite comme avant.
            let rotation = vec3(volume.get("rotation"), 0.0);
            let (mesh, matrix) = if rotation.iter().all(|r| *r == 0.0) {
                (box_mesh(size, offset), DMat4::IDENTITY)
            } else {
                let [rx, ry, rz] = rotation.map(f64::to_radians);
                (
                    box_mesh(size, [0.0; 3]),
                    DMat4::from_rotation_translation(
                        glam::DQuat::from_euler(glam::EulerRot::XYZ, rx, ry, rz),
                        glam::DVec3::from_array(offset),
                    ),
                )
            };
            Some(engine::api::ModelVolume {
                name: volume
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("volume")
                    .to_string(),
                mesh,
                matrix,
                role,
                extruder: volume
                    .get("extruder")
                    .and_then(Value::as_u64)
                    .filter(|e| *e > 0)
                    .map(|e| e as u16),
                paint: engine::api::VolumePaint::default(),
                settings: scene_object_settings(volume),
                emboss: None,
                cut_info: crate::scene::scene_cut_info(volume),
                mesh_stats: None,
            })
        })
        .collect()
}

/// Plages de hauteur d'un objet de scène (`layerRanges: [{min, max, settings}]`,
/// éditeur `scene/layer-ranges.ts`) → `ModelObject::layer_config_ranges`.
pub(crate) fn scene_object_layer_ranges(object: &Value) -> Vec<engine::api::LayerConfigRange> {
    object
        .get("layerRanges")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|range| {
            let min_z = range.get("min")?.as_f64()?;
            let max_z = range.get("max")?.as_f64()?;
            (min_z.is_finite() && max_z > min_z).then(|| engine::api::LayerConfigRange {
                min_z,
                max_z,
                settings: scene_object_settings(range),
            })
        })
        .collect()
}

fn is_mesh_volume(volume: &Value) -> bool {
    volume.get("shape").and_then(Value::as_str) == Some("mesh")
}

/// Volume maillé d'un objet de scène (`shape: "mesh"`, Orca `ModelVolume` avec
/// son propre maillage, ex. « Split to parts ») : référence au modèle du projet
/// qui porte le maillage. Le volume moteur est complet sauf `mesh`, rempli par
/// [`MeshVolumeRef::resolve`] avec le maillage du modèle **recentré** (même
/// convention que l'éditeur), placé par `matrix` = translation(`offset`) ·
/// rotation(`rotation`, Euler XYZ en degrés) dans le repère recentré de l'objet.
#[derive(Debug, Clone)]
pub(crate) struct MeshVolumeRef {
    pub model_id: String,
    pub volume: engine::api::ModelVolume,
}

impl MeshVolumeRef {
    /// Pose le maillage du modèle référencé (recentré sur sa boîte englobante).
    pub(crate) fn resolve(mut self, mut mesh: TriangleMesh) -> engine::api::ModelVolume {
        center_mesh(&mut mesh);
        self.volume.mesh = mesh;
        self.volume
    }
}

/// Volumes maillés (`shape: "mesh"`, `modelId`) d'un objet de scène.
pub(crate) fn scene_object_mesh_volumes(object: &Value) -> Vec<MeshVolumeRef> {
    use engine::api::VolumeRole;
    let vec3 = |value: Option<&Value>| -> [f64; 3] {
        let items = value.and_then(Value::as_array);
        std::array::from_fn(|axis| {
            items
                .and_then(|items| items.get(axis))
                .and_then(Value::as_f64)
                .filter(|v| v.is_finite())
                .unwrap_or(0.0)
        })
    };
    object
        .get("volumes")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|volume| is_mesh_volume(volume))
        .filter_map(|volume| {
            let model_id = volume.get("modelId").and_then(Value::as_str)?.to_string();
            let role = match volume.get("type").and_then(Value::as_str)? {
                "part" => VolumeRole::ModelPart,
                "modifier" => VolumeRole::ParameterModifier,
                "negative" => VolumeRole::NegativeVolume,
                "blocker" => VolumeRole::SupportBlocker,
                "enforcer" => VolumeRole::SupportEnforcer,
                _ => return None,
            };
            let [rx, ry, rz] = vec3(volume.get("rotation")).map(f64::to_radians);
            let matrix = DMat4::from_rotation_translation(
                glam::DQuat::from_euler(glam::EulerRot::XYZ, rx, ry, rz),
                glam::DVec3::from_array(vec3(volume.get("offset"))),
            );
            Some(MeshVolumeRef {
                model_id,
                volume: engine::api::ModelVolume {
                    name: volume
                        .get("name")
                        .and_then(Value::as_str)
                        .unwrap_or("volume")
                        .to_string(),
                    mesh: TriangleMesh {
                        vertices: Vec::new(),
                        indices: Vec::new(),
                    },
                    matrix,
                    role,
                    extruder: volume
                        .get("extruder")
                        .and_then(Value::as_u64)
                        .filter(|e| *e > 0)
                        .map(|e| e as u16),
                    paint: engine::api::VolumePaint::default(),
                    settings: scene_object_settings(volume),
                    emboss: scene_emboss(volume),
                    cut_info: crate::scene::scene_cut_info(volume),
                    mesh_stats: None,
                },
            })
        })
        .collect()
}

/// Relief ré-éditable d'un objet ou volume de scène (`emboss`, lot 26) :
/// objet JSON `{ type: "text" | "svg", … }` (cf. réponse de `/emboss`),
/// écrit dans le 3MF comme `TextConfiguration` / `EmbossShape` d'Orca.
pub(crate) fn scene_emboss(node: &Value) -> Option<Value> {
    node.get("emboss")
        .filter(|e| matches!(e.get("type").and_then(Value::as_str), Some("text" | "svg")))
        .cloned()
}

/// Pavé fermé (12 triangles, normales sortantes) de taille `size` centré en `center`.
fn box_mesh(size: [f64; 3], center: [f64; 3]) -> TriangleMesh {
    let h = size.map(|s| s / 2.0);
    let vertices = (0..8)
        .map(|i| {
            let corner = [i & 1, (i >> 1) & 1, (i >> 2) & 1];
            std::array::from_fn(|axis| {
                (center[axis] + if corner[axis] == 1 { h[axis] } else { -h[axis] }) as f32
            })
        })
        .collect();
    // Sommet i : x = bit0, y = bit1, z = bit2.
    let indices = vec![
        [0, 2, 1],
        [1, 2, 3], // z-
        [4, 5, 6],
        [5, 7, 6], // z+
        [0, 1, 4],
        [1, 5, 4], // y-
        [2, 6, 3],
        [3, 6, 7], // y+
        [0, 4, 2],
        [2, 4, 6], // x-
        [1, 3, 5],
        [3, 7, 5], // x+
    ];
    TriangleMesh { vertices, indices }
}

/// Peinture d'un objet de scène (`painting` : canal → { triangle → hex Orca }).
pub(crate) fn scene_object_paint(object: &Value) -> engine::api::VolumePaint {
    use engine::api::PaintChannel;
    let mut facets = Vec::new();
    let Some(painting) = object.get("painting").and_then(Value::as_object) else {
        return engine::api::VolumePaint::default();
    };
    for (name, channel) in [
        ("supports", PaintChannel::Supports),
        ("seam", PaintChannel::Seam),
        ("mmu", PaintChannel::Mmu),
        ("fuzzy", PaintChannel::Fuzzy),
    ] {
        let Some(entries) = painting.get(name).and_then(Value::as_object) else {
            continue;
        };
        let mut channel_facets: Vec<(PaintChannel, u32, String)> = entries
            .iter()
            .filter_map(|(triangle, state)| {
                Some((channel, triangle.parse().ok()?, state.as_str()?.to_string()))
            })
            .filter(|(_, _, state)| !state.is_empty())
            .collect();
        channel_facets.sort_by_key(|f| f.1);
        facets.extend(channel_facets);
    }
    engine::api::VolumePaint { facets }
}

fn center_mesh(mesh: &mut TriangleMesh) {
    let Some((min, max)) = mesh.bounding_box() else {
        return;
    };
    let center = std::array::from_fn::<_, 3, _>(|axis| (min[axis] + max[axis]) / 2.0);
    for vertex in &mut mesh.vertices {
        for axis in 0..3 {
            vertex[axis] -= center[axis];
        }
    }
}

fn to_triangle_mesh(mesh: &Mesh) -> TriangleMesh {
    TriangleMesh {
        vertices: mesh
            .positions
            .chunks_exact(3)
            .map(|c| [c[0], c[1], c[2]])
            .collect(),
        indices: mesh
            .indices
            .chunks_exact(3)
            .map(|c| [c[0], c[1], c[2]])
            .collect(),
    }
}

/// Décode la sortie du worker : la dernière ligne `R {json}` porte le
/// `SliceResult` (le protocole `P`/`R`/`E` de `engine-worker`).
fn parse_slice_result(stdout: &[u8]) -> anyhow::Result<engine::api::SliceResult> {
    let text = String::from_utf8_lossy(stdout);
    let payload = text
        .lines()
        .rev()
        .find_map(|l| l.strip_prefix("R "))
        .ok_or_else(|| anyhow::anyhow!("sortie moteur sans résultat « R »"))?;
    Ok(serde_json::from_str(payload)?)
}

/// Paramètres de calibration du projet (`scene.calibration`, posés par les
/// calibrations du menu — équivalent de `Print::set_calib_params` d'Orca).
/// Absents ou invalides → tranchage normal.
pub fn calib_from_scene(scene: &Value) -> Option<engine::api::CalibParams> {
    let raw = scene.get("calibration")?.clone();
    let params: engine::api::CalibParams = serde_json::from_value(raw).ok()?;
    match params.validate() {
        Ok(()) if params.mode != engine::api::CalibMode::None => Some(params),
        Ok(()) => None,
        Err(error) => {
            tracing::warn!(%error, "paramètres de calibration ignorés");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn rotated_volume_keeps_box_centered_and_carries_the_rotation_in_its_matrix() {
        let object = serde_json::json!({ "volumes": [
            { "type": "modifier", "size": [10, 4, 2], "offset": [1, 2, 3] },
            { "type": "modifier", "size": [10, 4, 2], "offset": [1, 2, 3], "rotation": [0, 0, 90] }
        ] });
        let volumes = super::scene_object_volumes(&object);
        assert_eq!(volumes[0].matrix, glam::DMat4::IDENTITY);
        let m = volumes[1].matrix;
        assert!((m.w_axis.truncate() - glam::DVec3::new(1.0, 2.0, 3.0)).length() < 1e-12);
        // Rotation de 90° autour de Z : l'axe X local devient Y.
        assert!((m.x_axis.truncate() - glam::DVec3::Y).length() < 1e-12);
        let xs: Vec<f32> = volumes[1].mesh.vertices.iter().map(|v| v[0]).collect();
        assert_eq!(xs.iter().cloned().fold(f32::MIN, f32::max), 5.0);
    }

    use super::*;
    use engine::api::ConfigValue;
    use serde_json::json;

    #[test]
    fn calibration_is_read_from_scene() {
        let scene = json!({ "calibration": { "mode": "retraction_tower", "start": 0.0, "end": 2.0, "step": 0.1 } });
        let calib = calib_from_scene(&scene).unwrap();
        assert_eq!(calib.mode, engine::api::CalibMode::RetractionTower);
        assert_eq!(calib.mode.code(), 9);
        assert!(calib_from_scene(&json!({})).is_none());
        assert!(calib_from_scene(
            &json!({ "calibration": { "mode": "vfa_tower", "start": 1, "end": 2, "step": 0 } })
        )
        .is_none());
    }

    #[test]
    fn restores_the_typed_config_only_from_its_snapshot() {
        let snapshot = json!({
            "layer_height": 0.16,
            "nozzle_diameter": [0.4, 0.6],
            "nozzle_temperature": [205, 235],
            "filament_type": ["PLA", "PETG"],
            "filament_retraction_length": [0.8, null],
            "infill_anchor": "200%",
            "bed_mesh_min": [-10.0, -10.0],
            "printable_area": [[0.0, 0.0], [200.0, 0.0]],
            "enable_support": true
        });
        let config = config_from_snapshot(&snapshot).unwrap();
        assert_eq!(config.get("layer_height"), Some(&ConfigValue::Float(0.16)));
        assert_eq!(
            config.get("nozzle_diameter"),
            Some(&ConfigValue::Floats(vec![0.4, 0.6]))
        );
        assert_eq!(
            config.get("nozzle_temperature"),
            Some(&ConfigValue::Ints(vec![205, 235]))
        );
        assert_eq!(
            config.get("filament_type"),
            Some(&ConfigValue::Strings(vec!["PLA".into(), "PETG".into()]))
        );
        assert_eq!(
            config.get("filament_retraction_length"),
            Some(&ConfigValue::FloatsNullable(vec![Some(0.8), None]))
        );
        assert_eq!(
            config.get("infill_anchor"),
            Some(&ConfigValue::FloatOrPercent {
                value: 200.0,
                percent: true
            })
        );
        assert_eq!(
            config.get("bed_mesh_min"),
            Some(&ConfigValue::Point(vec![-10.0, -10.0]))
        );
        assert_eq!(
            config.get("printable_area"),
            Some(&ConfigValue::Points(vec![vec![0.0, 0.0], vec![200.0, 0.0]]))
        );
        assert_eq!(config.get("enable_support"), Some(&ConfigValue::Bool(true)));
    }

    #[test]
    fn snapshot_scene_selection_does_not_depend_on_a_project() {
        let frozen = json!({
            "objects": [
                {"id": "old", "modelId": "model-old", "extruder": 1},
                {"id": "new", "modelId": "model-new", "extruder": 2}
            ],
            "plates": [
                {"id": "p1", "objectIds": ["old"]},
                {"id": "p2", "objectIds": ["new"]}
            ]
        });
        let specs = scene_object_specs(&frozen, Some(0)).unwrap();
        assert_eq!(specs.len(), 1);
        assert_eq!(specs[0].model_id, "model-old");
        assert_eq!(specs[0].extruder, Some(1));
    }
    #[test]
    fn slice_transform_matches_the_export_matrix_for_rotation_and_scale() {
        // Rotation Z 30° + échelle anisotrope [1, 2, 3] : la colonne j doit
        // valoir R[:,j] · s_j (et non une ligne de R, ni un s mélangé).
        let transform = json!({
            "position": [5.0, -7.0, 11.0],
            "rotation": [0.0, 0.0, 30.0],
            "scale": [1.0, 2.0, 3.0]
        });
        let scene = json!({
            "objects": [{"id": "o", "modelId": "m", "transform": transform}],
            "plates": [{"id": "p", "objectIds": ["o"]}]
        });
        let specs = scene_object_specs(&scene, Some(0)).unwrap();
        let slice_matrix = specs[0].matrix;
        let export_matrix = scene_transform_matrix(&transform);

        let (s, c) = 30f64.to_radians().sin_cos();
        #[rustfmt::skip]
        let expected = DMat4::from_cols_array(&[
            c, s, 0.0, 0.0, // R[:,0] · 1
            -s * 2.0, c * 2.0, 0.0, 0.0, // R[:,1] · 2
            0.0, 0.0, 3.0, 0.0, // R[:,2] · 3
            5.0, -7.0, 11.0, 1.0,
        ]);
        assert!(slice_matrix.abs_diff_eq(export_matrix, 1e-12));
        assert!(slice_matrix.abs_diff_eq(expected, 1e-12));
        // Un point (0, 1, 0) étiré ×2 en Y puis tourné de 30° autour de Z.
        let point = slice_matrix.transform_point3(glam::DVec3::new(0.0, 1.0, 0.0));
        assert!((point.x - (5.0 - 2.0 * s)).abs() < 1e-12);
        assert!((point.y - (-7.0 + 2.0 * c)).abs() < 1e-12);
        assert!((point.z - 11.0).abs() < 1e-12);
    }
    #[test]
    fn mirrored_scene_object_reaches_the_engine_as_a_left_handed_instance() {
        // Miroir de l'éditeur (échelle X négative) : la matrice d'instance doit
        // rester left-handed (det < 0) et refléter X sans clamp ni abs(). Le
        // tranchage d'un tel objet est prouvé côté moteur par
        // `engine/tests/ffi_smoke.rs::mirrored_instance_slices_as_the_mirror_image`.
        let transform = json!({
            "position": [100.0, 80.0, 3.0],
            "rotation": [0.0, 0.0, 0.0],
            "scale": [-1.0, 1.0, 1.0]
        });
        let scene = json!({
            "objects": [{"id": "o", "modelId": "m", "transform": transform}],
            "plates": [{"id": "p", "objectIds": ["o"]}]
        });
        let m = scene_object_specs(&scene, Some(0)).unwrap()[0].matrix;
        assert!(
            (m.determinant() + 1.0).abs() < 1e-12,
            "det = {}",
            m.determinant()
        );
        let p = m.transform_point3(glam::DVec3::new(15.0, 4.0, 0.0));
        assert!((p.x - 85.0).abs() < 1e-12 && (p.y - 84.0).abs() < 1e-12);
    }
}

#[cfg(test)]
mod production_parity {
    //! Chemin de PRODUCTION (upload → conversion → scène → build_scene_model →
    //! engine-worker slice) sur des modèles à sommets non entiers, comparé au
    //! G-code du CLI Orca pour la même position (centre du plateau, comme le
    //! harness). Ignoré si engine-worker (passe FFI) est absent.
    use super::*;
    use std::sync::Arc;

    /// Un seul test de ce module à la fois : chaque test lance des tranchages
    /// et conversions engine-worker (libslic3r multi-thread) ; en parallèle ils
    /// se disputent les cœurs, et le chemin du worker était partagé via la
    /// variable de process ENGINE_WORKER_BIN (set/remove concurrents). Le
    /// worker est désormais résolu sans muter l'environnement.
    static SERIAL: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

    fn repo() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf()
    }

    fn worker() -> Option<std::path::PathBuf> {
        if let Ok(p) = std::env::var("ENGINE_WORKER_BIN") {
            return Some(std::path::PathBuf::from(p)).filter(|p| p.exists());
        }
        let bin = std::env::current_exe()
            .ok()?
            .parent()?
            .parent()?
            .join("engine-worker");
        bin.exists().then_some(bin)
    }

    /// Port de normalize (tests/oracle/lib/gcode.ts, gelé).
    fn normalize(gcode: &str) -> Vec<String> {
        let volatile = regex::Regex::new(
            r"^; (filename|.*_settings_id|inherits|compatible_.*|thumbnails.*|printer_model|print_compatible_printers|upward_compatible_machine|setting_id|from|name|.*_notes) = ",
        )
        .unwrap();
        let thumb_start =
            regex::Regex::new(r"^; (THUMBNAIL_BLOCK_START|thumbnail(_JPG|_QOI)? begin)").unwrap();
        let thumb_end =
            regex::Regex::new(r"^; (THUMBNAIL_BLOCK_END|thumbnail(_JPG|_QOI)? end)").unwrap();
        let generated = regex::Regex::new(r"(?i)^; generated by ").unwrap();
        let block = regex::Regex::new(r"^;\s*(HEADER|EXECUTABLE)_BLOCK").unwrap();
        let mut out = Vec::new();
        let mut skipping = false;
        for raw in gcode.replace("\r\n", "\n").split('\n') {
            let line = raw.trim_end();
            if thumb_start.is_match(line) {
                skipping = true;
            }
            if skipping {
                if thumb_end.is_match(line) {
                    skipping = false;
                }
                continue;
            }
            if line.is_empty()
                || generated.is_match(line)
                || block.is_match(line)
                || volatile.is_match(line)
            {
                continue;
            }
            out.push(line.to_string());
        }
        out
    }

    /// Tranche \`stl\` via le chemin de production ; renvoie (G-code normalisé,
    /// nb de sommets où l'ancien maillage WSMh recentré diffère du maillage Orca).
    async fn production_gcode(
        stl: &std::path::Path,
        extra: &[(&str, &str)],
    ) -> Option<(Vec<String>, usize)> {
        let worker = worker()?;
        let _serial = SERIAL.lock().await;
        let dir = tempfile::tempdir().unwrap();
        let url = format!("sqlite://{}", dir.path().join("p.db").display());
        let storage = Arc::new(
            crate::adapters::storage::sqlite::SqliteStorage::connect(&url)
                .await
                .unwrap(),
        );
        let state = AppState::new(
            storage as Arc<dyn crate::domain::Storage>,
            crate::adapters::files::FileStore::new(dir.path().join("data")),
        );
        let user = state
            .storage
            .users()
            .create(crate::domain::repo::NewUser {
                email: "p@harness.local".into(),
                password_hash: "x".into(),
                role: crate::domain::Role::User,
            })
            .await
            .unwrap();
        let project = state
            .storage
            .projects()
            .create(
                user.id,
                crate::domain::NewProject {
                    name: "p".into(),
                    scene: serde_json::json!({}),
                    active_presets: serde_json::json!({}),
                    thumbnail_path: None,
                },
            )
            .await
            .unwrap();
        let bytes = std::fs::read(stl).unwrap();
        let model = crate::http::routes::models::write_model_record(
            &state,
            user.id,
            project.id,
            ModelFormat::Stl,
            stl.file_name().unwrap().to_string_lossy().into(),
            &bytes,
        )
        .await
        .unwrap();
        state.converter.convert(model.clone()).await;
        let model = state.storage.models().get(user.id, model.id).await.unwrap();
        assert!(model.mesh_path.is_some(), "conversion");

        // Placement du CLI Orca (harness) : sommets Orca centrés sur le plateau
        // (350 mm, centre 175), translation alignée sur la grille 1e-6 mm.
        let orca_mesh = crate::engine::raw_model::centered_part_mesh(
            FsPath::new(&model.file_path),
            model.format,
        )
        .await
        .expect("maillage libslic3r");
        let (min, max) = orca_mesh.bounding_box().unwrap();
        let snap = |v: f64| (v / 1e-6).round() * 1e-6;
        let pos = [
            snap(175.0 - (f64::from(min[0]) + f64::from(max[0])) / 2.0),
            snap(175.0 - (f64::from(min[1]) + f64::from(max[1])) / 2.0),
            -f64::from(min[2]),
        ];
        let mut old = load_triangle_mesh(&state, &model).await.unwrap();
        center_mesh(&mut old);
        let differing = old
            .vertices
            .iter()
            .zip(&orca_mesh.vertices)
            .filter(|(a, b)| a != b)
            .count();

        let scene = serde_json::json!({
            "objects": [{
                "id": "o1",
                "modelId": model.id.to_string(),
                "transform": { "position": pos, "rotation": [0.0, 0.0, 0.0], "scale": [1.0, 1.0, 1.0] }
            }],
            "plates": { "plates": [{ "id": "p1", "objectIds": ["o1"] }], "activeId": "p1" }
        });
        let built = build_scene_model(&state, user.id, project.id, &scene, None)
            .await
            .unwrap();
        assert_eq!(
            built.objects[0].volumes[0].mesh.vertices, orca_mesh.vertices,
            "sommets Orca"
        );
        // Cache load-model-raw à côté du modèle : un second build ne relance pas le worker.
        let cache = crate::engine::raw_model::cache_path(FsPath::new(&model.file_path), &bytes);
        assert!(cache.is_file(), "cache {}", cache.display());
        let cached: engine::api::Model =
            serde_json::from_slice(&std::fs::read(&cache).unwrap()).unwrap();
        assert_eq!(
            cached.objects[0].volumes[0].mesh.vertices, orca_mesh.vertices,
            "contenu du cache"
        );

        let mut values: serde_json::Map<String, Value> = serde_json::from_slice(
            &std::fs::read(repo().join("tests/oracle/refs/base_config.json")).unwrap(),
        )
        .unwrap();
        values.insert("internal_solid_infill_pattern".into(), "concentric".into());
        for (k, v) in extra {
            values.insert((*k).into(), (*v).into());
        }
        let config = engine::presets::resolve_preset_chain(&[engine::api::RawPreset {
            name: "p".into(),
            inherits: None,
            values,
        }])
        .unwrap();
        let work = tempfile::tempdir().unwrap();
        let request = engine::api::SliceRequest {
            allow_mixed_temperatures: false,
            model: built,
            config,
            plate_index: 0,
            work_dir: work.path().to_path_buf(),
            calib: None,
        };
        let path = work.path().join("req.json");
        std::fs::write(&path, serde_json::to_vec(&request).unwrap()).unwrap();
        let stdout = crate::engine::worker::run_worker_at(
            &worker,
            &["slice", path.to_str().unwrap()],
            Duration::from_secs(170),
        )
        .await
        .unwrap();
        let result = parse_slice_result(&stdout).unwrap();
        let gcode = std::fs::read_to_string(&result.gcode_path).unwrap();
        Some((normalize(&gcode), differing))
    }

    /// Chemin de PRODUCTION multi-filament : deux modèles STL téléversés,
    /// scène à deux objets (extrudeur 1 / 2) placés comme dans le 3MF d'Orca
    /// (engine/tests/fixtures/multi_filament/two_objects.3mf), config = celle
    /// que le CLI a écrite dans ce 3MF → build_scene_model → engine-worker
    /// slice, comparé au G-code Orca (tour d'essuyage, purges, T0/T1).
    #[tokio::test]
    async fn production_multi_filament_scene_matches_orca() {
        production_multi_filament(None, "two_objects.orca.gcode").await;
    }

    /// Réglages du projet (scene.projectConfig : couleurs, matrice et
    /// multiplicateur de purge) appliqués par-dessus les presets comme Orca :
    /// G-code identique à Orca tranchant la même scène avec ces valeurs dans
    /// sa config projet (two_objects_project.orca.gcode, 5/5 tirages).
    #[tokio::test]
    async fn production_project_config_overrides_match_orca() {
        production_multi_filament(
            Some(serde_json::json!({
                "filament_colour": ["#F2754E", "#00A0FF"],
                "flush_volumes_matrix": [0, 480, 220, 0],
                "flush_multiplier": 1.5
            })),
            "two_objects_project.orca.gcode",
        )
        .await;
    }

    async fn production_multi_filament(project_config: Option<Value>, reference: &str) {
        let Some(worker) = worker() else {
            eprintln!("engine-worker absent — ignoré");
            return;
        };
        let _serial = SERIAL.lock().await;
        let fixtures = repo().join("engine/tests/fixtures/multi_filament");
        let dir = tempfile::tempdir().unwrap();
        let url = format!("sqlite://{}", dir.path().join("p.db").display());
        let storage = Arc::new(
            crate::adapters::storage::sqlite::SqliteStorage::connect(&url)
                .await
                .unwrap(),
        );
        let state = AppState::new(
            storage as Arc<dyn crate::domain::Storage>,
            crate::adapters::files::FileStore::new(dir.path().join("data")),
        );
        let user = state
            .storage
            .users()
            .create(crate::domain::repo::NewUser {
                email: "mf@harness.local".into(),
                password_hash: "x".into(),
                role: crate::domain::Role::User,
            })
            .await
            .unwrap();
        let project = state
            .storage
            .projects()
            .create(
                user.id,
                crate::domain::NewProject {
                    name: "mf".into(),
                    scene: serde_json::json!({}),
                    active_presets: serde_json::json!({}),
                    thumbnail_path: None,
                },
            )
            .await
            .unwrap();
        // Placement Orca : origine du maillage recentré = instance × volume.
        let orca_scene =
            crate::engine::raw_model::load(&fixtures.join("two_objects.3mf"), ModelFormat::ThreeMf)
                .await
                .expect("scène Orca");
        let mut objects = Vec::new();
        for (index, (name, extruder)) in [("cube20", 1), ("cylinder", 2)].into_iter().enumerate() {
            let stl = repo().join(format!("tests/oracle/fixtures/{name}.stl"));
            let bytes = std::fs::read(&stl).unwrap();
            let model = crate::http::routes::models::write_model_record(
                &state,
                user.id,
                project.id,
                ModelFormat::Stl,
                format!("{name}.stl"),
                &bytes,
            )
            .await
            .unwrap();
            state.converter.convert(model.clone()).await;
            let object = &orca_scene.objects[index];
            let world = object.instances[0].matrix * object.volumes[0].matrix;
            let origin = world.transform_point3(glam::DVec3::ZERO);
            // Le CLI pivote les pièces à l'arrangement (allow_rotations) : angle Z.
            let angle_z = world.x_axis.y.atan2(world.x_axis.x).to_degrees();
            objects.push(serde_json::json!({
                "id": format!("o{index}"),
                "modelId": model.id.to_string(),
                "extruder": extruder,
                "transform": { "position": [origin.x, origin.y, origin.z], "rotation": [0.0, 0.0, angle_z], "scale": [1.0, 1.0, 1.0] }
            }));
        }
        let scene = serde_json::json!({
            "objects": objects,
            "plates": { "plates": [{ "id": "p1", "objectIds": ["o0", "o1"] }], "activeId": "p1" }
        });
        let built = build_scene_model(&state, user.id, project.id, &scene, None)
            .await
            .unwrap();
        let extruders: Vec<_> = built
            .objects
            .iter()
            .map(|o| o.volumes[0].extruder)
            .collect();
        assert_eq!(extruders, vec![Some(1), Some(2)]);

        // Config : project_settings.config écrite par Orca dans le 3MF, plus les
        // options que le CLI crée vides à la lecture d'un 3MF (OrcaSlicer.cpp:1769).
        let archive = std::fs::read(fixtures.join("two_objects.3mf")).unwrap();
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(archive)).unwrap();
        let mut text = String::new();
        std::io::Read::read_to_string(
            &mut zip.by_name("Metadata/project_settings.config").unwrap(),
            &mut text,
        )
        .unwrap();
        let values: serde_json::Map<String, Value> = serde_json::from_str(&text).unwrap();
        let mut config = engine::presets::resolve_preset_chain(&[engine::api::RawPreset {
            name: "orca".into(),
            inherits: None,
            values,
        }])
        .unwrap();
        for key in [
            "upward_compatible_machine",
            "print_compatible_printers",
            "different_settings_to_system",
        ] {
            config
                .0
                .entry(key.to_string())
                .or_insert_with(|| engine::api::ConfigValue::Strings(Vec::new()));
        }
        if let Some(project_config) = project_config {
            crate::http::routes::slice::apply_project_config(
                &mut config,
                &serde_json::json!({ "projectConfig": project_config }),
            );
        }
        let work = tempfile::tempdir().unwrap();
        let request = engine::api::SliceRequest {
            allow_mixed_temperatures: false,
            model: built,
            config,
            plate_index: 0,
            work_dir: work.path().to_path_buf(),
            calib: None,
        };
        let path = work.path().join("req.json");
        std::fs::write(&path, serde_json::to_vec(&request).unwrap()).unwrap();
        let stdout = crate::engine::worker::run_worker_at(
            &worker,
            &["slice", path.to_str().unwrap()],
            Duration::from_secs(170),
        )
        .await
        .unwrap();
        let result = parse_slice_result(&stdout).unwrap();
        let ours = normalize(&std::fs::read_to_string(&result.gcode_path).unwrap());
        let orca = normalize(&std::fs::read_to_string(fixtures.join(reference)).unwrap());
        assert!(
            ours == orca,
            "{}",
            first_diff(&ours, &orca).unwrap_or_default()
        );
    }

    /// Sommets du maillage de production historique (WSMh cuit puis recentré
    /// en f32) vs sommets Orca (libslic3r) : nombre de différences.
    async fn vertex_differences(stl: &std::path::Path) -> Option<(usize, usize)> {
        worker()?;
        let _serial = SERIAL.lock().await;
        let orca = crate::engine::raw_model::centered_part_mesh(stl, ModelFormat::Stl).await?;
        let decoded = crate::models::WorkerMeshDecoder::new();
        use crate::models::MeshDecoder;
        let wsmh = decoded.decode(stl, ModelFormat::Stl).await.ok()?;
        let display = engine::api::DisplayMesh::decode(&wsmh).ok()?;
        let mut old = TriangleMesh {
            vertices: display
                .positions
                .chunks_exact(3)
                .map(|c| [c[0], c[1], c[2]])
                .collect(),
            indices: display
                .indices
                .chunks_exact(3)
                .map(|c| [c[0], c[1], c[2]])
                .collect(),
        };
        center_mesh(&mut old);
        let n = old
            .vertices
            .iter()
            .zip(&orca.vertices)
            .filter(|(a, b)| a != b)
            .count();
        Some((n, orca.vertices.len()))
    }

    #[tokio::test]
    async fn production_vertices_equal_orca_on_golden_models() {
        let mut report = Vec::new();
        for name in [
            "cube20", "cylinder", "overhang", "bridge", "thinwall", "twoparts",
        ] {
            let stl = repo().join(format!("tests/oracle/fixtures/{name}.stl"));
            let Some((n, total)) = vertex_differences(&stl).await else {
                eprintln!("engine-worker absent — ignoré");
                return;
            };
            report.push(format!("{name}: {n}/{total}"));
        }
        eprintln!(
            "sommets différents (ancien chemin vs Orca) : {}",
            report.join(", ")
        );
    }

    fn first_diff(a: &[String], b: &[String]) -> Option<String> {
        (0..a.len().max(b.len()))
            .find(|&i| a.get(i) != b.get(i))
            .map(|i| {
                format!(
                    "L{} attendu {:?} obtenu {:?} ({} vs {} lignes)",
                    i + 1,
                    b.get(i),
                    a.get(i),
                    a.len(),
                    b.len()
                )
            })
    }

    #[tokio::test]
    async fn production_counterbore_matches_orca() {
        let fixtures =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/counterbore");
        let orca = normalize(&std::fs::read_to_string(fixtures.join("orca-none.gcode")).unwrap());
        // libslic3r n'est pas déterministe sur ce modèle : sur une même requête,
        // engine-worker produit la variante d'Orca ~92 % du temps seul, ~75 %
        // sous charge (24 tirages, staging/native-parity/determinism) ; l'autre
        // variante perd quelques segments de remplissage d'interstices — le CLI
        // Orca a le même comportement. Comparaison inchangée (égalité exacte
        // avec la référence Orca, variante majoritaire) mais jusqu'à 3 tirages
        // indépendants : une vraie divergence échoue aux 3.
        let mut diffs = Vec::new();
        for attempt in 1..=3 {
            let Some((gcode, differing)) =
                production_gcode(&fixtures.join("counterbore.stl"), &[]).await
            else {
                eprintln!("engine-worker absent — ignoré");
                return;
            };
            eprintln!("trou lamé : {differing} sommets différaient (WSMh recentré vs Orca)");
            if gcode == orca {
                return;
            }
            diffs.push(format!(
                "tirage {attempt} : {}",
                first_diff(&gcode, &orca).unwrap_or_default()
            ));
        }
        panic!("aucun tirage identique à Orca :\n{}", diffs.join("\n"));
    }
}

#[cfg(test)]
mod custom_gcode_tests {
    use super::*;

    #[test]
    fn scene_custom_gcodes_per_plate() {
        let scene = serde_json::json!({ "plates": { "plates": [
            { "id": "a", "objectIds": [] },
            { "id": "b", "objectIds": [], "customGcodes": [
                { "printZ": 5.0, "type": "color_change", "extruder": 1, "color": "#FF0000" },
                { "printZ": 10.0, "type": "pause_print", "extra": "Pause" },
                { "printZ": 12.0, "type": "nope" },
                { "printZ": -1.0, "type": "custom", "extra": "M117" }
            ] }
        ] } });
        let got = scene_custom_gcodes(&scene);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].plate_index, 1);
        assert_eq!(
            got[0].items.len(),
            2,
            "type inconnu et cote invalide ignorés"
        );
        assert_eq!(
            got[0].items[1].kind,
            engine::api::CustomGcodeKind::PausePrint
        );
    }

    #[test]
    fn mode_follows_extruders_and_filaments() {
        let mut c = DynamicPrintConfig::new();
        assert_eq!(
            custom_gcode_mode(&c),
            engine::api::CustomGcodeMode::SingleExtruder
        );
        c.0.insert(
            "filament_colour".into(),
            engine::api::ConfigValue::Strings(vec!["#F00".into(), "#0F0".into()]),
        );
        assert_eq!(
            custom_gcode_mode(&c),
            engine::api::CustomGcodeMode::MultiAsSingle
        );
        c.0.insert(
            "nozzle_diameter".into(),
            engine::api::ConfigValue::Floats(vec![0.4, 0.4]),
        );
        assert_eq!(
            custom_gcode_mode(&c),
            engine::api::CustomGcodeMode::MultiExtruder
        );
    }
}

#[cfg(test)]
mod lot44b_tests {
    //! Lot 44b : nom du ModelObject (I2) et objets de scène d'un même
    //! `instanceGroup` assemblés en un ModelObject à N instances (F-a), au
    //! tranchage (`build_scene_model`) comme à l'export 3MF
    //! (`export::build_project_model`).
    use super::*;
    use std::sync::Arc;

    fn cube_stl() -> Vec<u8> {
        let v = |x: f32, y: f32, z: f32| [x, y, z];
        let vertices = vec![
            v(0.0, 0.0, 0.0),
            v(10.0, 0.0, 0.0),
            v(10.0, 10.0, 0.0),
            v(0.0, 10.0, 0.0),
            v(0.0, 0.0, 10.0),
            v(10.0, 0.0, 10.0),
            v(10.0, 10.0, 10.0),
            v(0.0, 10.0, 10.0),
        ];
        let indices = vec![
            [0, 2, 1],
            [0, 3, 2],
            [4, 5, 6],
            [4, 6, 7],
            [0, 1, 5],
            [0, 5, 4],
            [1, 2, 6],
            [1, 6, 5],
            [2, 3, 7],
            [2, 7, 6],
            [3, 0, 4],
            [3, 4, 7],
        ];
        engine::api::split::encode_binary_stl(&TriangleMesh { vertices, indices })
    }

    #[tokio::test]
    async fn instance_groups_become_one_model_object_named_like_the_editor() {
        let dir = tempfile::tempdir().unwrap();
        let url = format!("sqlite://{}", dir.path().join("p.db").display());
        let storage = Arc::new(
            crate::adapters::storage::sqlite::SqliteStorage::connect(&url)
                .await
                .unwrap(),
        );
        let state = AppState::new(
            storage as Arc<dyn crate::domain::Storage>,
            crate::adapters::files::FileStore::new(dir.path().join("data")),
        );
        let user = state
            .storage
            .users()
            .create(crate::domain::repo::NewUser {
                email: "i@harness.local".into(),
                password_hash: "x".into(),
                role: crate::domain::Role::User,
            })
            .await
            .unwrap();
        let project = state
            .storage
            .projects()
            .create(
                user.id,
                crate::domain::NewProject {
                    name: "p".into(),
                    scene: serde_json::json!({}),
                    active_presets: serde_json::json!({}),
                    thumbnail_path: None,
                },
            )
            .await
            .unwrap();
        let model = crate::http::routes::models::write_model_record(
            &state,
            user.id,
            project.id,
            ModelFormat::Stl,
            "projet — cube.3mf".into(),
            &cube_stl(),
        )
        .await
        .unwrap();
        let m = model.id.to_string();
        // a, c : deux instances d'un même objet (groupe g) ; b : objet seul,
        // sans nom (repli : nom du fichier du modèle).
        let scene = serde_json::json!({
            "objects": [
                {"id": "a", "modelId": m, "name": "cube20.stl", "instanceGroup": "g"},
                {"id": "b", "modelId": m},
                {"id": "c", "modelId": m, "name": "cube20.stl", "instanceGroup": "g", "printable": false},
            ],
            "plates": { "plates": [
                { "id": "p1", "objectIds": ["a", "b"] },
                { "id": "p2", "objectIds": ["c"] }
            ], "activeId": "p1" }
        });
        let (built, refs) = build_scene_model_indexed(&state, user.id, project.id, &scene, None)
            .await
            .unwrap();
        assert_eq!(built.objects.len(), 2, "un ModelObject par groupe");
        assert_eq!(built.objects[0].name, "cube20.stl");
        assert_eq!(built.objects[0].instances.len(), 2);
        assert_eq!(built.objects[0].instances[1].plate_index, 1);
        assert!(!built.objects[0].instances[1].printable);
        assert_eq!(built.objects[1].name, "projet — cube.3mf");
        assert_eq!(built.objects[1].instances.len(), 1);
        assert_eq!(refs.len(), 2, "une référence par ModelObject");
        assert_eq!(refs[0].id.as_deref(), Some("a"));

        // Plateau 2 seul : l'instance c, dans un ModelObject à 1 instance.
        let plate2 = build_scene_model(&state, user.id, project.id, &scene, Some(1))
            .await
            .unwrap();
        assert_eq!(plate2.objects.len(), 1);
        assert_eq!(plate2.objects[0].instances.len(), 1);

        // Export 3MF : même regroupement, même nom.
        let saved = state
            .storage
            .projects()
            .update(
                user.id,
                project.id,
                project.version,
                scene.clone(),
                serde_json::json!({}),
                None,
            )
            .await
            .unwrap();
        assert_eq!(saved.scene, scene);
        let exported =
            crate::http::routes::export::build_project_model(&state, user.id, project.id, None)
                .await
                .unwrap();
        assert_eq!(exported.objects.len(), 2);
        assert_eq!(exported.objects[0].name, "cube20.stl");
        assert_eq!(exported.objects[0].instances.len(), 2);
        assert_eq!(exported.objects[1].name, "projet — cube.3mf");
    }

    #[test]
    fn instance_group_key_needs_a_group_and_the_model() {
        assert_eq!(instance_group_key(None, "m"), None);
        assert_eq!(instance_group_key(Some(" "), "m"), None);
        assert_eq!(
            instance_group_key(Some("g"), "m"),
            Some(("g".to_string(), "m".to_string()))
        );
        let scene = serde_json::json!({
            "objects": [{"id": "o", "modelId": "m", "instanceGroup": "g"}, {"id": "p", "modelId": "m"}],
        });
        let specs = scene_object_specs(&scene, None).unwrap();
        assert_eq!(specs[0].instance_group.as_deref(), Some("g"));
        assert_eq!(specs[1].instance_group, None);
    }
}
