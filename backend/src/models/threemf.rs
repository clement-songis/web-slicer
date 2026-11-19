//! Import des projets 3MF (Orca, PrusaSlicer, 3MF générique) : lecture par
//! libslic3r (`engine-worker read-project-3mf`, `Model::read_from_file` /
//! `load_bbs_3mf` comme l'ouverture d'un projet dans Orca), puis conversion en
//! occurrences de scène (maillage pièce, volumes, peinture, réglages, plages de
//! couches, plateaux). L'ancien lecteur XML pur Rust a été retiré.

use engine::api::TriangleMesh;
use glam::{DMat4, DVec3};
use serde_json::{Map, Value};

#[derive(Debug, Clone)]
pub struct ImportedObject {
    pub name: String,
    pub mesh: TriangleMesh,
    /// Matrice de l'item `<build>`, dans le repère 3MF (mm).
    pub matrix: DMat4,
    /// `printable` de l'item `<build>` (absent = true).
    pub printable: bool,
    pub extruder: u64,
    pub settings: Map<String, Value>,
    pub layer_profile: Vec<f64>,
    pub source_id: String,
    pub instance_index: usize,
    /// Peinture du maillage pièce, au format scène (`canal → { triangle → hex }`).
    pub painting: Map<String, Value>,
    /// Relief ré-éditable du maillage pièce (scène : `emboss` de l'objet).
    pub emboss: Option<Value>,
    /// Volumes non pièce (négatif, modificateur, bloqueur/forceur) au format
    /// scène (`scene/volumes.ts`) : pavé englobant dans le repère recentré.
    pub volumes: Vec<Value>,
    /// Pièces supplémentaires (`normal_part` au-delà de la première) : chacune
    /// devient un modèle et un volume maillé de la scène (`shape: "mesh"`),
    /// comme les `ModelVolume` d'Orca, au lieu d'être fusionnée dans `mesh`.
    pub parts: Vec<ImportedPart>,
    /// Plages de hauteur (`layer_config_ranges`) au format scène
    /// (`{min, max, settings}`, éditeur `scene/layer-ranges.ts`).
    pub layer_ranges: Vec<Value>,
    /// Nom du premier volume pièce (`<part>` de `model_settings.config`) quand
    /// il diffère du nom de l'objet (Orca `<nom>_1` après un split) → `meshName`.
    pub mesh_name: Option<String>,
}

/// Pièce d'un objet multi-pièces 3MF (volume `normal_part` distinct).
#[derive(Debug, Clone)]
pub struct ImportedPart {
    pub name: String,
    /// Rôle de scène (`part`, `negative`, `modifier`, `blocker`, `enforcer`).
    pub role: String,
    pub mesh: TriangleMesh,
    /// Centre de la pièce moins centre du maillage principal (repère recentré).
    pub offset: [f64; 3],
    pub extruder: u64,
    pub settings: Map<String, Value>,
    /// Relief ré-éditable de la pièce (scène : `emboss` du volume maillé).
    pub emboss: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct ImportedPlate {
    pub name: String,
    /// Libellé du type de plaque côté WebSlicer (`Plate.plateType`).
    pub plate_type: String,
    /// Réglages par plateau Orca (`engine::threemf::PLATE_SETTING_KEYS`), au
    /// format texte de configuration Orca (ex. `print_sequence = by object`,
    /// `first_layer_print_sequence = 1,2`, `spiral_mode = 1`).
    pub settings: Map<String, Value>,
    pub instances: Vec<(String, usize)>,
}

/// Correspondance libellé WebSlicer (`PLATE_TYPES` du frontend) ↔ clé d'enum
/// Orca `BedType` (`s_keys_map_BedType`) sérialisée dans `bed_type`.
const PLATE_TYPE_BED_TYPES: &[(&str, &str)] = &[
    ("Cool Plate / PLA Plate", "Cool Plate"),
    ("Supertack Plate", "Supertack Plate"),
    ("Engineering Plate", "Engineering Plate"),
    ("Smooth High Temp Plate", "High Temp Plate"),
    ("Textured Cool Plate", "Textured Cool Plate"),
    ("Textured PEI Plate", "Textured PEI Plate"),
];

/// Type de plaque par défaut du frontend (`DEFAULT_PLATE_TYPE`).
pub const DEFAULT_PLATE_TYPE: &str = "Cool Plate / PLA Plate";

/// Libellé WebSlicer → valeur Orca de `curr_bed_type`. Une valeur déjà au
/// format Orca est acceptée telle quelle ; un libellé inconnu donne `None`.
pub fn plate_type_to_bed_type(plate_type: &str) -> Option<&'static str> {
    PLATE_TYPE_BED_TYPES
        .iter()
        .find(|(label, bed)| *label == plate_type || *bed == plate_type)
        .map(|(_, bed)| *bed)
}

/// Valeur Orca `bed_type` → libellé WebSlicer (valeur brute si inconnue,
/// ex. `Default Plate`).
pub fn bed_type_to_plate_type(bed_type: &str) -> String {
    // Orca relit l'ancienne graphie « SuperTack Plate ».
    let bed_type = if bed_type == "SuperTack Plate" {
        "Supertack Plate"
    } else {
        bed_type
    };
    PLATE_TYPE_BED_TYPES
        .iter()
        .find(|(_, bed)| *bed == bed_type)
        .map_or_else(|| bed_type.to_string(), |(label, _)| (*label).to_string())
}

#[derive(Debug, Clone)]
pub struct ImportedProject {
    pub objects: Vec<ImportedObject>,
    pub plates: Vec<ImportedPlate>,
    /// Configuration Orca brute (chaînes ou tableaux de chaînes).
    pub config: Map<String, Value>,
    /// Métadonnées du projet (Title, Designer, License…), `None` si aucune.
    pub metadata: Option<engine::threemf::ProjectMetadata>,
}

#[derive(Debug, thiserror::Error)]
#[error("3MF projet illisible : {0}")]
pub struct ThreeMfImportError(pub String);

type ImportResult<T> = Result<T, ThreeMfImportError>;

fn invalid(message: impl Into<String>) -> ThreeMfImportError {
    ThreeMfImportError(message.into())
}

// Lecture par libslic3r (`engine-worker read-project-3mf` : Model::read_from_file /
// load_bbs_3mf, comme l'ouverture d'un projet dans Orca), puis conversion vers
// la vue d'import de la scène. Réglages au format texte Orca tels qu'écrits
// dans le fichier ; configuration au format project_settings.config.

#[derive(serde::Deserialize)]
struct ViewVolume {
    #[serde(default)]
    name: String,
    role: String,
    matrix: Vec<f64>,
    #[serde(default)]
    extruder: u64,
    #[serde(default)]
    settings: Map<String, Value>,
    vertices: Vec<f32>,
    indices: Vec<u32>,
    #[serde(default)]
    paint: Vec<(String, u32, String)>,
    /// Relief ré-éditable (`text_configuration` / `emboss_shape` d'Orca).
    #[serde(default)]
    emboss: Option<Value>,
}

#[derive(serde::Deserialize)]
struct ViewInstance {
    matrix: Vec<f64>,
    #[serde(default = "yes")]
    printable: bool,
}

fn yes() -> bool {
    true
}

#[derive(serde::Deserialize)]
struct ViewObject {
    #[serde(default)]
    name: String,
    #[serde(default)]
    settings: Map<String, Value>,
    #[serde(default)]
    layer_profile: Vec<f64>,
    #[serde(default)]
    layer_ranges: Vec<Value>,
    instances: Vec<ViewInstance>,
    volumes: Vec<ViewVolume>,
}

#[derive(serde::Deserialize)]
struct ViewPlate {
    index: usize,
    #[serde(default)]
    name: String,
    #[serde(default)]
    settings: Map<String, Value>,
    #[serde(default)]
    obj_inst: Vec<usize>,
}

#[derive(serde::Deserialize)]
struct ProjectView {
    objects: Vec<ViewObject>,
    #[serde(default)]
    config: Map<String, Value>,
    #[serde(default)]
    plates: Vec<ViewPlate>,
    #[serde(default)]
    metadata: Option<engine::threemf::ProjectMetadata>,
}

fn matrix4(values: &[f64]) -> ImportResult<DMat4> {
    let array: [f64; 16] = values
        .try_into()
        .map_err(|_| invalid("matrice de transformation invalide"))?;
    let m = DMat4::from_cols_array(&array);
    if !array.iter().all(|v| v.is_finite()) {
        return Err(invalid("matrice de transformation non finie"));
    }
    Ok(m)
}

/// Maillage du volume dans le repère de l'objet (matrice du volume appliquée).
fn volume_mesh(volume: &ViewVolume) -> ImportResult<TriangleMesh> {
    let m = matrix4(&volume.matrix)?;
    let mirrored = m.determinant() < 0.0;
    let vertices = volume
        .vertices
        .chunks_exact(3)
        .map(|v| {
            let p = m.transform_point3(DVec3::new(v[0].into(), v[1].into(), v[2].into()));
            [p.x as f32, p.y as f32, p.z as f32]
        })
        .collect();
    let indices = volume
        .indices
        .chunks_exact(3)
        .map(|t| {
            if mirrored {
                [t[0], t[2], t[1]]
            } else {
                [t[0], t[1], t[2]]
            }
        })
        .collect();
    Ok(TriangleMesh { vertices, indices })
}

fn round4(v: f64) -> f64 {
    (v * 1e4).round() / 1e4
}

fn mesh_center(mesh: &TriangleMesh) -> [f64; 3] {
    mesh.bounding_box()
        .map(|(min, max)| std::array::from_fn(|a| (min[a] + max[a]) as f64 / 2.0))
        .unwrap_or([0.0; 3])
}

fn extruder_of(settings: &Map<String, Value>) -> Option<u64> {
    settings
        .get("extruder")
        .and_then(Value::as_str)
        .and_then(|s| s.trim().parse().ok())
        .filter(|e| *e > 0)
}

/// Réglages d'objet / volume / plage de couches pour la scène : sans
/// `extruder` (porté à part) et typés selon le registre des options.
fn without_extruder(settings: &Map<String, Value>) -> Map<String, Value> {
    typed_settings(settings)
        .into_iter()
        .filter(|(k, _)| k.as_str() != "extruder")
        .collect()
}

/// Valeurs texte Orca (`model_settings.config` : `print_flow_ratio = "0.8"`)
/// typées selon le schéma des options (`engine::params`), au format JSON des
/// presets et de l'éditeur : float/int/pourcentage → nombre, bool → booléen,
/// vecteurs → tableaux, float-ou-pourcentage avec `%` → chaîne « 50% ».
/// Clés inconnues du registre ou valeurs illisibles : conservées telles quelles
/// (le tranchage les relit par `resolve_preset_chain`, tolérant aux deux formes).
pub(crate) fn typed_settings(settings: &Map<String, Value>) -> Map<String, Value> {
    settings
        .iter()
        .map(|(key, value)| {
            let typed = match (value, engine::params::get(key)) {
                // Les pourcentages restent des chaînes (« 0% », « 105% ») : l'export
                // 3MF doit restituer exactement la valeur d'Orca.
                (Value::String(text), Some(_)) if text.trim_end().ends_with('%') => value.clone(),
                (Value::String(text), Some(def)) => {
                    engine::params::orca_values::parse_orca_value(def, text)
                        .map(|parsed| crate::http::routes::presets::config_value_to_json(&parsed))
                        .unwrap_or_else(|| value.clone())
                }
                _ => value.clone(),
            };
            (key.clone(), typed)
        })
        .collect()
}

/// Projet 3MF lu par libslic3r (worker) et converti pour la scène.
pub async fn read_project(bytes: &[u8]) -> ImportResult<ImportedProject> {
    let work =
        crate::workdir::tempdir().map_err(|e| invalid(format!("répertoire de travail : {e}")))?;
    let input = work.path().join("project.3mf");
    let output = work.path().join("project.json");
    tokio::fs::write(&input, bytes)
        .await
        .map_err(|e| invalid(format!("écriture temporaire : {e}")))?;
    crate::engine::run_worker(
        &[
            "read-project-3mf",
            &input.to_string_lossy(),
            &output.to_string_lossy(),
        ],
        std::time::Duration::from_secs(170),
    )
    .await
    .map_err(|e| invalid(format!("lecture libslic3r : {e}")))?;
    let json = tokio::fs::read(&output)
        .await
        .map_err(|e| invalid(format!("résultat de lecture : {e}")))?;
    let view: ProjectView =
        serde_json::from_slice(&json).map_err(|e| invalid(format!("vue d'import : {e}")))?;
    convert_view(view)
}

fn convert_view(view: ProjectView) -> ImportResult<ImportedProject> {
    let mut objects = Vec::new();
    for (object_index, object) in view.objects.iter().enumerate() {
        let source_id = (object_index + 1).to_string();
        let parts: Vec<&ViewVolume> = object.volumes.iter().filter(|v| v.role == "part").collect();
        let Some(first) = parts.first() else {
            continue;
        };
        let mesh = volume_mesh(first)?;
        if mesh.indices.is_empty() {
            continue;
        }
        let center = mesh_center(&mesh);
        let mut painting = Map::new();
        for (channel, triangle, state) in &first.paint {
            painting
                .entry(channel.clone())
                .or_insert_with(|| Value::Object(Map::new()))
                .as_object_mut()
                .expect("canal = objet")
                .insert(triangle.to_string(), Value::String(state.clone()));
        }
        let mut volumes = Vec::new();
        let mut parts_out = Vec::new();
        let mut other_index = 0usize;
        for (volume_index, volume) in object.volumes.iter().enumerate() {
            if std::ptr::eq(volume, *first) {
                continue;
            }
            let vmesh = volume_mesh(volume)?;
            let Some((min, max)) = vmesh.bounding_box() else {
                continue;
            };
            let middle: [f64; 3] =
                std::array::from_fn(|a| round4((min[a] + max[a]) as f64 / 2.0 - center[a]));
            // Volume non pièce au maillage quelconque (ex. connecteur de coupe
            // négatif, Orca CutUtils) : conservé maillé comme Orca ; seuls les
            // pavés (8 sommets, 12 facettes, volumes « box » de la scène)
            // restent des volumes paramétriques.
            let is_box = vmesh.indices.len() == 12 && vmesh.vertices.len() == 8;
            if volume.role == "part" || !is_box {
                parts_out.push(ImportedPart {
                    role: volume.role.clone(),
                    name: if volume.name.trim().is_empty() {
                        format!("Pièce {}", parts_out.len() + 2)
                    } else {
                        volume.name.clone()
                    },
                    mesh: vmesh,
                    offset: middle,
                    extruder: volume.extruder,
                    settings: without_extruder(&volume.settings),
                    emboss: volume.emboss.clone().filter(|e| !e.is_null()),
                });
                continue;
            }
            other_index += 1;
            let size: [f64; 3] = std::array::from_fn(|a| round4((max[a] - min[a]) as f64));
            let mut doc = serde_json::json!({
                "id": format!("{source_id}-{volume_index}"),
                "type": volume.role,
                "name": if volume.name.trim().is_empty() { format!("Volume {other_index}") } else { volume.name.clone() },
                "shape": "box",
                "size": size,
                "offset": middle,
                "settings": without_extruder(&volume.settings),
            });
            if volume.extruder > 0 {
                doc["extruder"] = volume.extruder.into();
            }
            volumes.push(doc);
        }
        let extruder = extruder_of(&object.settings).unwrap_or(first.extruder);
        let mesh_name =
            Some(first.name.clone()).filter(|n| !n.trim().is_empty() && *n != object.name);
        let layer_ranges: Vec<Value> = object
            .layer_ranges
            .iter()
            .filter_map(|r| {
                Some(serde_json::json!({
                    "min": r.get("min")?.as_f64()?,
                    "max": r.get("max")?.as_f64()?,
                    "settings": Value::Object(
                        r.get("config").and_then(Value::as_object).map(typed_settings).unwrap_or_default(),
                    ),
                }))
            })
            .collect();
        for (instance_index, instance) in object.instances.iter().enumerate() {
            objects.push(ImportedObject {
                name: object.name.clone(),
                mesh: mesh.clone(),
                matrix: matrix4(&instance.matrix)?,
                printable: instance.printable,
                extruder,
                settings: without_extruder(&object.settings),
                layer_profile: object.layer_profile.clone(),
                source_id: source_id.clone(),
                instance_index,
                painting: painting.clone(),
                emboss: first.emboss.clone().filter(|e| !e.is_null()),
                volumes: volumes.clone(),
                parts: parts_out.clone(),
                layer_ranges: layer_ranges.clone(),
                mesh_name: mesh_name.clone(),
            });
        }
    }
    if objects.is_empty() {
        return Err(invalid("aucun objet dans le projet"));
    }

    let mut plates_view = view.plates;
    plates_view.sort_by_key(|p| p.index);
    let mut plates: Vec<ImportedPlate> = plates_view
        .into_iter()
        .map(|plate| {
            let plate_type = plate
                .settings
                .get("curr_bed_type")
                .and_then(Value::as_str)
                .map(bed_type_to_plate_type)
                .unwrap_or_else(|| DEFAULT_PLATE_TYPE.into());
            let settings = plate
                .settings
                .iter()
                .filter(|(k, _)| engine::threemf::PLATE_SETTING_KEYS.contains(&k.as_str()))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            ImportedPlate {
                name: if plate.name.trim().is_empty() {
                    format!("Plateau {}", plate.index + 1)
                } else {
                    plate.name
                },
                plate_type,
                settings,
                instances: plate
                    .obj_inst
                    .chunks_exact(2)
                    .map(|p| ((p[0] + 1).to_string(), p[1]))
                    .collect(),
            }
        })
        .collect();
    if plates.is_empty() {
        plates.push(ImportedPlate {
            name: "Plateau 1".into(),
            plate_type: DEFAULT_PLATE_TYPE.into(),
            settings: Map::new(),
            instances: objects
                .iter()
                .map(|object| (object.source_id.clone(), object.instance_index))
                .collect(),
        });
    }
    Ok(ImportedProject {
        objects,
        plates,
        config: view.config,
        metadata: view.metadata,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine::api::{ConfigValue, DynamicPrintConfig, ModelInstance};
    use engine::threemf::ProjectPlate;

    /// Projet de test écrit par libslic3r (`store_bbs_3mf`, comme l'export
    /// projet) : tests/fixtures/threemf/plates_settings.3mf. Régénération :
    /// `cargo test -p backend --lib regenerate_plates_settings_fixture -- --ignored`.
    const FIXTURE: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/threemf/plates_settings.3mf"
    );

    fn tetra() -> TriangleMesh {
        TriangleMesh {
            vertices: vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 1.0],
            ],
            indices: vec![[0, 1, 2], [0, 1, 3], [1, 2, 3], [0, 2, 3]],
        }
    }

    /// Deux objets sur deux plateaux : surcharge `wall_loops` + profil sur
    /// le second, réglages de plateau sur le second plateau.
    fn exported() -> Vec<u8> {
        std::fs::read(FIXTURE).expect("fixture 3MF (voir FIXTURE)")
    }

    #[tokio::test]
    #[ignore = "régénère la fixture via engine-worker (libslic3r)"]
    async fn regenerate_plates_settings_fixture() {
        let (model, plates) = fixture_model();
        let bytes = crate::engine::project_3mf::write_project_3mf(
            &model,
            &DynamicPrintConfig::new(),
            &plates,
        )
        .await
        .unwrap();
        std::fs::create_dir_all(std::path::Path::new(FIXTURE).parent().unwrap()).unwrap();
        std::fs::write(FIXTURE, bytes).unwrap();
    }

    fn fixture_model() -> (engine::api::Model, [ProjectPlate; 2]) {
        let mut model = engine::api::Model::default();
        for (name, plate_index) in [("Gauche", 0), ("Droite", 1)] {
            let object = model.add_object(String::from(name), tetra());
            object.instances = vec![ModelInstance {
                matrix: DMat4::from_translation(DVec3::new(10.0 * plate_index as f64, 0.0, 0.0)),
                plate_index,
                printable: true,
            }];
        }
        model.objects[1]
            .settings
            .0
            .insert("wall_loops".into(), ConfigValue::Int(5));
        model.objects[1].volumes[0].extruder = Some(2);
        model.objects[1].layer_height_profile = vec![0.0, 0.2, 0.5, 0.12, 1.0, 0.28];

        let mut second = DynamicPrintConfig::new();
        second.0.insert(
            "curr_bed_type".into(),
            ConfigValue::String("High Temp Plate".into()),
        );
        second.0.insert(
            "print_sequence".into(),
            ConfigValue::String("by object".into()),
        );
        second.0.insert(
            "other_layers_print_sequence".into(),
            ConfigValue::Ints(vec![2, 1]),
        );
        second
            .0
            .insert("spiral_mode".into(), ConfigValue::Bool(true));
        let plates = [
            ProjectPlate {
                name: "Premier".into(),
                settings: DynamicPrintConfig::new(),
                ..ProjectPlate::default()
            },
            ProjectPlate {
                name: "Second".into(),
                settings: second,
                ..ProjectPlate::default()
            },
        ];
        (model, plates)
    }

    fn worker_available() -> bool {
        crate::engine::worker::worker_binary().exists()
    }

    #[tokio::test]
    async fn object_settings_and_layer_profile_survive_export_import() {
        if !worker_available() {
            return;
        }
        let project = read_project(&exported()).await.unwrap();
        assert_eq!(project.objects.len(), 2);
        let right = &project.objects[1];
        assert_eq!(right.name, "Droite");
        // Typé selon le registre (int → nombre), comme les presets.
        assert_eq!(right.settings.get("wall_loops"), Some(&Value::from(5)));
        assert_eq!(right.extruder, 2);
        assert_eq!(right.layer_profile, vec![0.0, 0.2, 0.5, 0.12, 1.0, 0.28]);
        // La métadonnée `name` de la `<part>` n'est pas un réglage d'objet.
        assert!(
            project.objects[0].settings.is_empty(),
            "{:?}",
            project.objects[0].settings
        );
        assert!(project.objects[0].layer_profile.is_empty());
    }

    #[tokio::test]
    async fn plate_settings_survive_export_import() {
        if !worker_available() {
            return;
        }
        let project = read_project(&exported()).await.unwrap();
        assert_eq!(project.plates.len(), 2);
        let first = &project.plates[0];
        assert_eq!(first.name, "Premier");
        assert_eq!(first.plate_type, DEFAULT_PLATE_TYPE);
        // Identifiants = rang 1-based de l'objet dans le modèle lu par libslic3r.
        assert_eq!(first.instances, vec![("1".to_string(), 0)]);
        let second = &project.plates[1];
        assert_eq!(second.name, "Second");
        assert_eq!(second.plate_type, "Smooth High Temp Plate");
        assert_eq!(second.instances, vec![("2".to_string(), 0)]);
        let text = |s: &str| Some(Value::String(s.into()));
        assert_eq!(
            second.settings.get("curr_bed_type").cloned(),
            text("High Temp Plate")
        );
        assert_eq!(
            second.settings.get("print_sequence").cloned(),
            text("by object")
        );
        assert_eq!(
            second.settings.get("other_layers_print_sequence").cloned(),
            text("2,1")
        );
        assert_eq!(second.settings.get("spiral_mode").cloned(), text("1"));
    }

    /// Réglages texte d'Orca typés selon le schéma des options (BACKEND.md §6 :
    /// `print_flow_ratio = "0.8"` du projet de calibration du débit → 0.8).
    #[test]
    fn object_settings_are_typed_from_the_registry() {
        let raw: Map<String, Value> = serde_json::from_value(serde_json::json!({
            "print_flow_ratio": "0.8",
            "wall_loops": "5",
            "enable_support": "1",
            "sparse_infill_density": "15%",
            "outer_wall_line_width": "0.45",
            "inner_wall_line_width": "105%",
            "sparse_infill_pattern": "gyroid",
            "layer_height": 0.2,
            "cle_inconnue": "0.5",
            "wall_loops_illisible": "x",
            "extruder": "2",
        }))
        .unwrap();
        let typed = without_extruder(&raw);
        assert_eq!(typed["print_flow_ratio"], serde_json::json!(0.8));
        assert_eq!(typed["wall_loops"], serde_json::json!(5));
        assert_eq!(typed["enable_support"], serde_json::json!(true));
        assert_eq!(typed["sparse_infill_density"], serde_json::json!("15%"));
        assert_eq!(typed["outer_wall_line_width"], serde_json::json!(0.45));
        assert_eq!(typed["inner_wall_line_width"], serde_json::json!("105%"));
        assert_eq!(typed["sparse_infill_pattern"], serde_json::json!("gyroid"));
        assert_eq!(typed["layer_height"], serde_json::json!(0.2));
        assert_eq!(typed["cle_inconnue"], serde_json::json!("0.5"));
        assert_eq!(typed["wall_loops_illisible"], serde_json::json!("x"));
        assert!(!typed.contains_key("extruder"));
        // Valeur illisible pour une clé connue : gardée telle quelle.
        let bad: Map<String, Value> =
            serde_json::from_value(serde_json::json!({ "wall_loops": "beaucoup" })).unwrap();
        assert_eq!(
            typed_settings(&bad)["wall_loops"],
            serde_json::json!("beaucoup")
        );
    }

    #[test]
    fn plate_type_labels_map_to_orca_bed_types() {
        assert_eq!(
            plate_type_to_bed_type("Cool Plate / PLA Plate"),
            Some("Cool Plate")
        );
        assert_eq!(
            plate_type_to_bed_type("High Temp Plate"),
            Some("High Temp Plate")
        );
        assert_eq!(plate_type_to_bed_type("Plaque inconnue"), None);
        assert_eq!(bed_type_to_plate_type("SuperTack Plate"), "Supertack Plate");
        assert_eq!(bed_type_to_plate_type("Default Plate"), "Default Plate");
        for (label, bed) in PLATE_TYPE_BED_TYPES {
            assert_eq!(
                bed_type_to_plate_type(plate_type_to_bed_type(label).unwrap()),
                *label
            );
            assert_eq!(
                plate_type_to_bed_type(&bed_type_to_plate_type(bed)),
                Some(*bed)
            );
        }
    }
}
