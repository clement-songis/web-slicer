//! Lecture des projets 3MF/Orca sans extraction de l'archive sur disque.
//!
//! Le 3MF « projet » ne tient pas dans le seul `3D/3dmodel.model` : Orca y
//! référence souvent des composants externes et conserve la configuration et
//! les plateaux dans `Metadata/*`. Ce module reconstruit les occurrences de la
//! scène, leur géométrie et les métadonnées utiles au backend. Les extensions
//! inconnues sont ignorées ; une géométrie ou une référence invalide est en
//! revanche une erreur d'import.

use std::collections::{HashMap, HashSet};
use std::io::{Cursor, Read};

use engine::api::TriangleMesh;
use glam::{DMat4, DVec3};
use quick_xml::events::Event;
use quick_xml::Reader;
use serde_json::{Map, Value};
use zip::ZipArchive;

const MAIN_MODEL: &str = "3D/3dmodel.model";
/// Garde-fou contre les archives très fortement compressées. La limite porte
/// sur les XML effectivement lus, pas sur les vignettes/G-code embarqués.
const MAX_XML_BYTES: usize = 512 * 1024 * 1024;

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
    pub mesh: TriangleMesh,
    /// Centre de la pièce moins centre du maillage principal (repère recentré).
    pub offset: [f64; 3],
    pub extruder: u64,
    pub settings: Map<String, Value>,
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
}

#[derive(Debug, thiserror::Error)]
#[error("3MF projet illisible : {0}")]
pub struct ThreeMfImportError(pub String);

type ImportResult<T> = Result<T, ThreeMfImportError>;

fn invalid(message: impl Into<String>) -> ThreeMfImportError {
    ThreeMfImportError(message.into())
}

#[derive(Debug, Clone, Default)]
struct RawMesh {
    vertices: Vec<[f32; 3]>,
    triangles: Vec<[u32; 3]>,
    /// Peinture par triangle : (indice local, canal scène, état hex Orca).
    paint: Vec<(u32, &'static str, String)>,
}

/// Attributs de peinture d'Orca (`bbs_3mf.cpp`) et anciens noms PrusaSlicer → canal scène.
const PAINT_ATTRS: &[(&[u8], &str)] = &[
    (b"paint_supports", "supports"),
    (b"slic3rpe:custom_supports", "supports"),
    (b"paint_seam", "seam"),
    (b"slic3rpe:custom_seam", "seam"),
    (b"paint_color", "mmu"),
    (b"slic3rpe:mmu_segmentation", "mmu"),
    (b"paint_fuzzy_skin", "fuzzy"),
];

/// Maillage aplati d'un sous-objet, avec son id de ressource (lien `<part id>`).
#[derive(Debug, Clone)]
struct FlatMesh {
    object_id: String,
    mesh: TriangleMesh,
    paint: Vec<(u32, &'static str, String)>,
}

#[derive(Debug, Clone)]
struct RawComponent {
    object_id: String,
    path: Option<String>,
    transform: DMat4,
}

#[derive(Debug, Clone)]
struct RawObject {
    name: Option<String>,
    mesh: RawMesh,
    components: Vec<RawComponent>,
}

#[derive(Debug, Clone)]
struct BuildItem {
    object_id: String,
    transform: DMat4,
    printable: bool,
}

#[derive(Debug, Default)]
struct ModelFile {
    objects: HashMap<String, RawObject>,
    build: Vec<BuildItem>,
}

#[derive(Debug, Default)]
struct ObjectMetadata {
    name: Option<String>,
    extruder: Option<u64>,
    layer_profile: Vec<f64>,
    values: Map<String, Value>,
    /// `<part id subtype>` : sous-type, nom et réglages du volume.
    parts: HashMap<String, PartMetadata>,
}

#[derive(Debug, Default, Clone)]
struct PartMetadata {
    subtype: String,
    name: Option<String>,
    extruder: Option<u64>,
    values: Map<String, Value>,
}

#[derive(Debug, Default)]
struct PlateMetadata {
    id: Option<usize>,
    name: Option<String>,
    plate_type: Option<String>,
    settings: Map<String, Value>,
    instances: Vec<(String, usize)>,
    pending_instance: Option<(String, usize)>,
}

#[derive(Debug, Default)]
struct ModelMetadata {
    objects: HashMap<String, ObjectMetadata>,
    plates: Vec<PlateMetadata>,
}

const LAYER_HEIGHTS_PROFILE: &str = "Metadata/layer_heights_profile.txt";
const LAYER_CONFIG_RANGES: &str = "Metadata/layer_config_ranges.xml";

/// Lit les entrées utiles d'une archive 3MF et reconstruit sa scène.
pub fn read_project(bytes: &[u8]) -> ImportResult<ImportedProject> {
    let mut archive = ZipArchive::new(Cursor::new(bytes))
        .map_err(|e| invalid(format!("archive ZIP invalide : {e}")))?;

    let mut entries = HashMap::<String, Vec<u8>>::new();
    let mut total = 0usize;
    for index in 0..archive.len() {
        let file = archive
            .by_index(index)
            .map_err(|e| invalid(format!("entrée ZIP illisible : {e}")))?;
        let name = normalize_path(file.name());
        let wanted = name == MAIN_MODEL
            || (name.starts_with("3D/") && name.ends_with(".model"))
            || matches!(
                name.as_str(),
                "Metadata/model_settings.config"
                    | "Metadata/Slic3r_PE_model.config"
                    | "Metadata/project_settings.config"
                    | "Metadata/Slic3r_PE.config"
                    | "Metadata/print_profile.config"
                    | "Metadata/layer_heights_profile.txt"
                    | "Metadata/layer_config_ranges.xml"
            );
        if !wanted {
            continue;
        }
        let size = usize::try_from(file.size()).unwrap_or(MAX_XML_BYTES + 1);
        if size > MAX_XML_BYTES || total.saturating_add(size) > MAX_XML_BYTES {
            return Err(invalid("métadonnées/géométrie XML trop volumineuses"));
        }
        let mut content = Vec::with_capacity(size.min(16 * 1024 * 1024));
        file.take((MAX_XML_BYTES + 1) as u64)
            .read_to_end(&mut content)
            .map_err(|e| invalid(format!("lecture de {name} : {e}")))?;
        if content.len() > MAX_XML_BYTES || total.saturating_add(content.len()) > MAX_XML_BYTES {
            return Err(invalid("métadonnées/géométrie XML trop volumineuses"));
        }
        total += content.len();
        entries.insert(name, content);
    }

    let main = entries
        .get(MAIN_MODEL)
        .ok_or_else(|| invalid("entrée 3D/3dmodel.model absente"))?;
    let main_file = parse_model_file(main)?;

    let mut files = HashMap::new();
    files.insert(MAIN_MODEL.to_string(), main_file);
    for (name, content) in &entries {
        if name != MAIN_MODEL && name.starts_with("3D/") && name.ends_with(".model") {
            files.insert(name.clone(), parse_model_file(content)?);
        }
    }

    let metadata = [
        "Metadata/model_settings.config",
        "Metadata/Slic3r_PE_model.config",
    ]
    .iter()
    .find_map(|name| entries.get(*name))
    .map(|bytes| parse_model_metadata(bytes))
    .transpose()?
    .unwrap_or_default();

    let config = [
        "Metadata/project_settings.config",
        "Metadata/Slic3r_PE.config",
        "Metadata/print_profile.config",
    ]
    .iter()
    .find_map(|name| entries.get(*name))
    .map(|bytes| parse_project_config(bytes))
    .transpose()?
    .unwrap_or_default();

    let main = files
        .get(MAIN_MODEL)
        .expect("le modèle principal vient d'être inséré");

    // `layer_heights_profile.txt` est indexé par la position 1-based de
    // l'objet Orca (`ModelObject`), soit l'ordre de première apparition des
    // ressources dans `<build>`.
    let profiles = entries
        .get(LAYER_HEIGHTS_PROFILE)
        .map(|bytes| parse_layer_heights_profiles(bytes))
        .unwrap_or_default();
    // `layer_config_ranges.xml` : même indexation 1-based que les profils.
    let ranges = entries
        .get(LAYER_CONFIG_RANGES)
        .map(|bytes| parse_layer_config_ranges(bytes))
        .unwrap_or_default();
    let mut ranges_by_resource = HashMap::<String, Vec<Value>>::new();
    let mut profile_by_resource = HashMap::<String, Vec<f64>>::new();
    {
        let mut order = Vec::<&str>::new();
        for item in &main.build {
            if !order.contains(&item.object_id.as_str()) {
                order.push(&item.object_id);
            }
        }
        for (index, resource) in order.iter().enumerate() {
            if let Some(profile) = profiles.get(&(index + 1)) {
                profile_by_resource.insert((*resource).to_string(), profile.clone());
            }
            if let Some(object_ranges) = ranges.get(&(index + 1)) {
                ranges_by_resource.insert((*resource).to_string(), object_ranges.clone());
            }
        }
    }

    let mut objects = Vec::new();
    let mut per_resource_instances = HashMap::<String, usize>::new();
    for item in &main.build {
        let instance_index = per_resource_instances
            .entry(item.object_id.clone())
            .and_modify(|index| *index += 1)
            .or_insert(0);
        let raw = main
            .objects
            .get(&item.object_id)
            .ok_or_else(|| invalid(format!("objet {} absent des ressources", item.object_id)))?;
        let mut visiting = HashSet::new();
        let meshes = flatten_object(
            &files,
            MAIN_MODEL,
            &item.object_id,
            DMat4::IDENTITY,
            &mut visiting,
            0,
        )?;
        let meta = metadata.objects.get(&item.object_id);
        // Volumes pièce → maillage de l'objet (peinture réindexée) ; autres
        // sous-types → volumes de scène.
        let is_part = |flat: &FlatMesh| {
            meta.and_then(|m| m.parts.get(&flat.object_id))
                .is_none_or(|p| p.subtype == "normal_part")
        };
        let parts: Vec<&FlatMesh> = meshes.iter().filter(|f| is_part(f)).collect();
        let others: Vec<&FlatMesh> = meshes.iter().filter(|f| !is_part(f)).collect();
        // Première pièce = maillage de l'objet ; les suivantes restent des
        // volumes distincts (Orca `ModelObject::volumes`).
        let (main_parts, extra_parts) = parts.split_at(parts.len().min(1));
        let mesh = merge_meshes(
            &main_parts
                .iter()
                .map(|f| f.mesh.clone())
                .collect::<Vec<_>>(),
        )?;
        let mut painting = Map::new();
        let mut offset = 0u32;
        for flat in main_parts {
            for (triangle, channel, state) in &flat.paint {
                painting
                    .entry(channel.to_string())
                    .or_insert_with(|| Value::Object(Map::new()))
                    .as_object_mut()
                    .expect("canal = objet")
                    .insert(
                        (offset + triangle).to_string(),
                        Value::String(state.clone()),
                    );
            }
            offset += flat.mesh.indices.len() as u32;
        }
        let center = mesh
            .bounding_box()
            .map(|(min, max)| std::array::from_fn::<f64, 3, _>(|a| (min[a] + max[a]) as f64 / 2.0))
            .unwrap_or([0.0; 3]);
        let volumes: Vec<Value> = others
            .iter()
            .enumerate()
            .filter_map(|(index, flat)| {
                let part = meta.and_then(|m| m.parts.get(&flat.object_id))?;
                let kind = match part.subtype.as_str() {
                    "negative_part" | "negative_volume" => "negative",
                    "modifier_part" | "modifier" => "modifier",
                    "support_blocker" => "blocker",
                    "support_enforcer" => "enforcer",
                    _ => return None,
                };
                let (min, max) = flat.mesh.bounding_box()?;
                let size: [f64; 3] = std::array::from_fn(|a| (max[a] - min[a]) as f64);
                let middle: [f64; 3] =
                    std::array::from_fn(|a| (min[a] + max[a]) as f64 / 2.0 - center[a]);
                let round = |v: f64| (v * 1e4).round() / 1e4;
                let mut volume = serde_json::json!({
                    "id": format!("{}-{}", item.object_id, flat.object_id),
                    "type": kind,
                    "name": part.name.clone().unwrap_or_else(|| format!("Volume {}", index + 1)),
                    "shape": "box",
                    "size": size.map(round),
                    "offset": middle.map(round),
                    "settings": part.values,
                });
                if let Some(extruder) = part.extruder.filter(|e| *e > 0) {
                    volume["extruder"] = extruder.into();
                }
                Some(volume)
            })
            .collect();
        let mut extra = Vec::with_capacity(extra_parts.len());
        for (index, flat) in extra_parts.iter().enumerate() {
            let Some((min, max)) = flat.mesh.bounding_box() else {
                continue;
            };
            let part = meta.and_then(|m| m.parts.get(&flat.object_id));
            let name = part
                .and_then(|p| p.name.clone())
                .unwrap_or_else(|| format!("Pièce {}", index + 2));
            let round = |v: f64| (v * 1e4).round() / 1e4;
            extra.push(ImportedPart {
                name,
                mesh: flat.mesh.clone(),
                offset: std::array::from_fn(|a| round((min[a] + max[a]) as f64 / 2.0 - center[a])),
                extruder: part.and_then(|p| p.extruder).unwrap_or(0),
                settings: part.map(|p| p.values.clone()).unwrap_or_default(),
            });
        }
        let name = meta
            .and_then(|m| m.name.clone())
            .or_else(|| raw.name.clone())
            .unwrap_or_else(|| format!("Objet {}", item.object_id));
        let mesh_name = main_parts
            .first()
            .and_then(|flat| meta.and_then(|m| m.parts.get(&flat.object_id)))
            .and_then(|part| part.name.clone())
            .filter(|part_name| !part_name.trim().is_empty() && *part_name != name);
        let settings = meta.map(|m| m.values.clone()).unwrap_or_default();
        objects.push(ImportedObject {
            name,
            mesh,
            matrix: item.transform,
            printable: item.printable,
            extruder: meta.and_then(|m| m.extruder).unwrap_or(0),
            settings,
            layer_profile: profile_by_resource
                .get(&item.object_id)
                .cloned()
                .or_else(|| meta.map(|m| m.layer_profile.clone()))
                .unwrap_or_default(),
            source_id: item.object_id.clone(),
            instance_index: *instance_index,
            painting,
            volumes,
            parts: extra,
            layer_ranges: ranges_by_resource
                .get(&item.object_id)
                .cloned()
                .unwrap_or_default(),
            mesh_name,
        });
    }

    if objects.is_empty() {
        return Err(invalid("aucun objet dans la section <build>"));
    }

    let mut plate_metadata = metadata.plates;
    // `plater_id` est 1-based dans le 3MF ; ne pas supposer que le XML est trié.
    plate_metadata.sort_by_key(|plate| plate.id.unwrap_or(usize::MAX));
    let mut plates: Vec<ImportedPlate> = plate_metadata
        .into_iter()
        .filter_map(|plate| {
            plate.id.map(|id| ImportedPlate {
                name: plate
                    .name
                    .filter(|name| !name.trim().is_empty())
                    .unwrap_or_else(|| format!("Plateau {id}")),
                plate_type: plate
                    .plate_type
                    .map(|bed| bed_type_to_plate_type(&bed))
                    .unwrap_or_else(|| DEFAULT_PLATE_TYPE.into()),
                settings: plate.settings,
                instances: plate.instances,
            })
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
        config,
    })
}

fn normalize_path(path: &str) -> String {
    path.trim_start_matches('/').replace('\\', "/")
}

fn component_path(current_file: &str, component: Option<&str>) -> String {
    match component {
        Some(path) if path.starts_with('/') => normalize_path(path),
        Some(path) => {
            let parent = current_file
                .rsplit_once('/')
                .map(|(parent, _)| parent)
                .unwrap_or("");
            normalize_path(&format!("{parent}/{path}"))
        }
        None => current_file.to_string(),
    }
}

fn parse_model_file(bytes: &[u8]) -> ImportResult<ModelFile> {
    let mut reader = Reader::from_reader(bytes);
    reader.config_mut().trim_text(true);
    let mut buffer = Vec::new();
    let mut file = ModelFile::default();
    let mut current_id: Option<String> = None;
    let mut current = RawObject {
        name: None,
        mesh: RawMesh::default(),
        components: Vec::new(),
    };

    loop {
        match reader
            .read_event_into(&mut buffer)
            .map_err(|e| invalid(format!("XML du modèle invalide : {e}")))?
        {
            Event::Start(event) if local(event.name().as_ref()) == b"object" => {
                current_id = Some(required_attr(&event, b"id")?);
                current = RawObject {
                    name: attr(&event, b"name")?,
                    mesh: RawMesh::default(),
                    components: Vec::new(),
                };
            }
            Event::Empty(event) if current_id.is_some() => match local(event.name().as_ref()) {
                b"vertex" => {
                    let [x, y, z] = [b"x", b"y", b"z"].map(|name| {
                        required_attr(&event, name)
                            .and_then(|v| parse_number::<f64>(&v))
                            .map(|v| v as f32)
                    });
                    current.mesh.vertices.push([x?, y?, z?]);
                }
                b"triangle" => {
                    let [v1, v2, v3] = [b"v1", b"v2", b"v3"]
                        .map(|name| required_attr(&event, name).and_then(|v| parse_index(&v)));
                    let index = current.mesh.triangles.len() as u32;
                    current.mesh.triangles.push([v1?, v2?, v3?]);
                    for (name, channel) in PAINT_ATTRS {
                        if let Some(state) = attr(&event, name)?.filter(|s| !s.is_empty()) {
                            current.mesh.paint.push((index, channel, state));
                        }
                    }
                }
                b"component" => current.components.push(RawComponent {
                    object_id: required_attr(&event, b"objectid")?,
                    path: attr(&event, b"path")?,
                    transform: transform_attr(&event)?,
                }),
                _ => {}
            },
            Event::Empty(event) if local(event.name().as_ref()) == b"item" => {
                file.build.push(BuildItem {
                    object_id: required_attr(&event, b"objectid")?,
                    transform: transform_attr(&event)?,
                    printable: attr(&event, b"printable")?
                        .is_none_or(|v| v != "0" && !v.eq_ignore_ascii_case("false")),
                });
            }
            Event::Start(event) if local(event.name().as_ref()) == b"item" => {
                file.build.push(BuildItem {
                    object_id: required_attr(&event, b"objectid")?,
                    transform: transform_attr(&event)?,
                    printable: attr(&event, b"printable")?
                        .is_none_or(|v| v != "0" && !v.eq_ignore_ascii_case("false")),
                });
            }
            Event::End(event) if local(event.name().as_ref()) == b"object" => {
                if let Some(id) = current_id.take() {
                    file.objects.insert(id, current.clone());
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buffer.clear();
    }
    Ok(file)
}

fn parse_model_metadata(bytes: &[u8]) -> ImportResult<ModelMetadata> {
    let mut reader = Reader::from_reader(bytes);
    reader.config_mut().trim_text(true);
    let mut buffer = Vec::new();
    let mut result = ModelMetadata::default();
    let mut current_object: Option<String> = None;
    let mut current_plate: Option<PlateMetadata> = None;
    let mut in_instance = false;
    // Métadonnées de `<part>` (nom, matrice, source…) : propres au volume,
    // elles ne doivent pas écraser celles de l'objet.
    let mut part_depth = 0usize;
    let mut current_part: Option<(String, String)> = None;

    loop {
        match reader
            .read_event_into(&mut buffer)
            .map_err(|e| invalid(format!("XML des métadonnées invalide : {e}")))?
        {
            Event::Start(event) => match local(event.name().as_ref()) {
                b"object" => current_object = Some(required_attr(&event, b"id")?),
                b"plate" => current_plate = Some(PlateMetadata::default()),
                b"model_instance" => in_instance = true,
                b"part" => {
                    part_depth += 1;
                    if let (Some(object), Some(id)) =
                        (current_object.as_ref(), attr(&event, b"id")?)
                    {
                        current_part = Some((object.clone(), id.clone()));
                        result
                            .objects
                            .entry(object.clone())
                            .or_default()
                            .parts
                            .insert(
                                id,
                                PartMetadata {
                                    subtype: attr(&event, b"subtype")?
                                        .unwrap_or_else(|| "normal_part".into()),
                                    ..Default::default()
                                },
                            );
                    }
                }
                _ => {}
            },
            Event::Empty(event)
                if local(event.name().as_ref()) == b"metadata" && part_depth > 0 =>
            {
                let key = required_attr(&event, b"key")?;
                let value = attr(&event, b"value")?.unwrap_or_default();
                if let Some((object, id)) = current_part.as_ref() {
                    if let Some(part) = result
                        .objects
                        .get_mut(object)
                        .and_then(|o| o.parts.get_mut(id))
                    {
                        match key.as_str() {
                            "name" => part.name = Some(value),
                            "extruder" => part.extruder = value.parse().ok(),
                            // Métadonnées de géométrie/source, pas des réglages.
                            "matrix" | "mesh_shared" => {}
                            k if k.starts_with("source_") => {}
                            _ => {
                                part.values.insert(key, Value::String(value));
                            }
                        }
                    }
                }
            }
            Event::Empty(event)
                if local(event.name().as_ref()) == b"metadata" && part_depth == 0 =>
            {
                let key = required_attr(&event, b"key")?;
                let value = attr(&event, b"value")?.unwrap_or_default();
                if in_instance {
                    if let Some(plate) = current_plate.as_mut() {
                        match key.as_str() {
                            "object_id" => {
                                let instance =
                                    plate.pending_instance.get_or_insert((String::new(), 0));
                                instance.0 = value;
                            }
                            "instance_id" => {
                                let instance =
                                    plate.pending_instance.get_or_insert((String::new(), 0));
                                instance.1 =
                                    value.parse().map_err(|_| invalid("instance_id invalide"))?;
                            }
                            _ => {}
                        }
                    }
                } else if let Some(plate) = current_plate.as_mut() {
                    match key.as_str() {
                        "plater_id" => {
                            plate.id =
                                Some(value.parse().map_err(|_| invalid("plater_id invalide"))?)
                        }
                        "plater_name" => plate.name = Some(value),
                        // `bed_type` (BED_TYPE_ATTR) est la clé Orca ;
                        // `curr_bed_type` reste accepté (anciens fichiers).
                        "bed_type" | "curr_bed_type" => {
                            let bed = if value == "SuperTack Plate" {
                                "Supertack Plate".to_string()
                            } else {
                                value
                            };
                            plate
                                .settings
                                .insert("curr_bed_type".into(), Value::String(bed.clone()));
                            plate.plate_type = Some(bed);
                        }
                        "print_sequence" | "other_layers_print_sequence_nums" => {
                            plate.settings.insert(key, Value::String(value));
                        }
                        // Vecteurs d'entiers séparés par des espaces dans
                        // `<plate>`, par des virgules en configuration Orca.
                        "first_layer_print_sequence" | "other_layers_print_sequence" => {
                            let joined = value.split_whitespace().collect::<Vec<_>>().join(",");
                            plate.settings.insert(key, Value::String(joined));
                        }
                        "spiral_mode" => {
                            let on = matches!(value.trim(), "1" | "true");
                            plate
                                .settings
                                .insert(key, Value::String(if on { "1" } else { "0" }.into()));
                        }
                        _ => {}
                    }
                } else if let Some(id) = current_object.as_ref() {
                    let object = result.objects.entry(id.clone()).or_default();
                    match key.as_str() {
                        "name" => object.name = Some(value),
                        // Nom de module Bambu : pas un réglage d'impression.
                        "module" => {}
                        "extruder" => object.extruder = value.parse().ok(),
                        "layer_height_profile" => {
                            object.layer_profile = value
                                .split(|c: char| c == ',' || c == ';' || c.is_whitespace())
                                .filter(|part| !part.is_empty())
                                .filter_map(|part| part.parse().ok())
                                .collect();
                        }
                        _ => {
                            object.values.insert(key, Value::String(value));
                        }
                    }
                }
            }
            Event::End(event) => match local(event.name().as_ref()) {
                b"object" => current_object = None,
                b"part" => {
                    part_depth = part_depth.saturating_sub(1);
                    current_part = None;
                }
                b"model_instance" => {
                    in_instance = false;
                    if let Some(plate) = current_plate.as_mut() {
                        if let Some((object, instance)) = plate.pending_instance.take() {
                            if !object.is_empty() {
                                plate.instances.push((object, instance));
                            }
                        }
                    }
                }
                b"plate" => {
                    if let Some(plate) = current_plate.take() {
                        result.plates.push(plate);
                    }
                }
                _ => {}
            },
            Event::Eof => break,
            _ => {}
        }
        buffer.clear();
    }
    Ok(result)
}

/// Lit `Metadata/layer_config_ranges.xml` (Orca `_extract_layer_config_ranges_from_archive`) :
/// `<object id="n"><range min_z max_z><option opt_key="k">v</option>…`. Les plages
/// invalides (bornes non numériques ou vides) sont ignorées, comme dans Orca.
fn parse_layer_config_ranges(bytes: &[u8]) -> HashMap<usize, Vec<Value>> {
    let mut result = HashMap::<usize, Vec<Value>>::new();
    let mut reader = Reader::from_reader(bytes);
    reader.config_mut().trim_text(true);
    let mut buffer = Vec::new();
    let mut object: Option<usize> = None;
    let mut range: Option<(f64, f64, Map<String, Value>)> = None;
    let mut option: Option<String> = None;
    let number = |event: &quick_xml::events::BytesStart<'_>, name: &[u8]| {
        attr(event, name)
            .ok()
            .flatten()
            .and_then(|v| v.trim().parse::<f64>().ok())
            .filter(|v| v.is_finite())
    };
    while let Ok(event) = reader.read_event_into(&mut buffer) {
        match event {
            Event::Start(event) => match local(event.name().as_ref()) {
                b"object" => {
                    object = attr(&event, b"id")
                        .ok()
                        .flatten()
                        .and_then(|id| id.trim().parse().ok())
                        .filter(|id| *id > 0);
                }
                b"range" => {
                    range = match (number(&event, b"min_z"), number(&event, b"max_z")) {
                        (Some(min), Some(max)) if max > min => Some((min, max, Map::new())),
                        _ => None,
                    };
                }
                b"option" => option = attr(&event, b"opt_key").ok().flatten(),
                _ => {}
            },
            Event::Text(text) => {
                if let (Some(key), Some((_, _, settings))) = (option.as_ref(), range.as_mut()) {
                    let value = text
                        .decode()
                        .ok()
                        .and_then(|v| quick_xml::escape::unescape(&v).ok().map(|v| v.into_owned()))
                        .unwrap_or_default();
                    settings.insert(key.clone(), Value::String(value));
                }
            }
            Event::End(event) => match local(event.name().as_ref()) {
                b"option" => option = None,
                b"range" => {
                    if let (Some(id), Some((min, max, settings))) = (object, range.take()) {
                        result.entry(id).or_default().push(serde_json::json!({
                            "min": min,
                            "max": max,
                            "settings": settings,
                        }));
                    }
                }
                b"object" => object = None,
                _ => {}
            },
            Event::Eof => break,
            _ => {}
        }
        buffer.clear();
    }
    result
}

/// Lit `Metadata/layer_heights_profile.txt` (`object_id=<n>|z;h;z;h…`),
/// avec les mêmes gardes que l'importeur Orca : une ligne invalide est
/// ignorée, pas fatale.
fn parse_layer_heights_profiles(bytes: &[u8]) -> HashMap<usize, Vec<f64>> {
    let mut profiles = HashMap::new();
    let text = String::from_utf8_lossy(bytes);
    for line in text.lines() {
        let Some((id, values)) = line.trim().split_once('|') else {
            continue;
        };
        let Some(id) = id
            .split_once('=')
            .and_then(|(_, id)| id.trim().parse::<usize>().ok())
            .filter(|id| *id > 0)
        else {
            continue;
        };
        let profile: Option<Vec<f64>> = values
            .split(';')
            .map(|v| v.trim().parse::<f64>().ok())
            .collect();
        match profile {
            Some(profile) if profile.len() > 4 && profile.len() % 2 == 0 => {
                profiles.entry(id).or_insert(profile);
            }
            _ => {}
        }
    }
    profiles
}

fn parse_project_config(bytes: &[u8]) -> ImportResult<Map<String, Value>> {
    let text = std::str::from_utf8(bytes).map_err(|_| invalid("configuration non UTF-8"))?;
    if text.trim_start().starts_with('{') {
        let value: Value = serde_json::from_str(text)
            .map_err(|e| invalid(format!("configuration JSON invalide : {e}")))?;
        return value
            .as_object()
            .cloned()
            .ok_or_else(|| invalid("configuration JSON : objet attendu"));
    }
    let mut values = Map::new();
    for line in text.lines() {
        let line = line.trim().trim_start_matches(';').trim();
        if let Some((key, value)) = line.split_once('=') {
            values.insert(
                key.trim().to_string(),
                Value::String(value.trim().to_string()),
            );
        }
    }
    Ok(values)
}

fn flatten_object(
    files: &HashMap<String, ModelFile>,
    file: &str,
    object_id: &str,
    transform: DMat4,
    visiting: &mut HashSet<(String, String)>,
    depth: usize,
) -> ImportResult<Vec<FlatMesh>> {
    if depth > 64 {
        return Err(invalid("imbrication de composants trop profonde"));
    }
    let key = (file.to_string(), object_id.to_string());
    if !visiting.insert(key.clone()) {
        return Err(invalid("référence circulaire entre composants"));
    }
    let model = files
        .get(file)
        .ok_or_else(|| invalid(format!("ressource externe absente : {file}")))?;
    let object = model
        .objects
        .get(object_id)
        .ok_or_else(|| invalid(format!("objet {object_id} absent de {file}")))?;

    let mut meshes = Vec::new();
    if !object.mesh.vertices.is_empty() || !object.mesh.triangles.is_empty() {
        meshes.push(FlatMesh {
            object_id: object_id.to_string(),
            mesh: transform_mesh(&object.mesh, transform)?,
            paint: object.mesh.paint.clone(),
        });
    }
    for component in &object.components {
        let path = component_path(file, component.path.as_deref());
        meshes.extend(flatten_object(
            files,
            &path,
            &component.object_id,
            transform * component.transform,
            visiting,
            depth + 1,
        )?);
    }
    visiting.remove(&key);
    Ok(meshes)
}

fn transform_mesh(mesh: &RawMesh, transform: DMat4) -> ImportResult<TriangleMesh> {
    let vertices = mesh
        .vertices
        .iter()
        .map(|vertex| {
            let point = transform.transform_point3(DVec3::new(
                vertex[0] as f64,
                vertex[1] as f64,
                vertex[2] as f64,
            ));
            [point.x as f32, point.y as f32, point.z as f32]
        })
        .collect();
    Ok(TriangleMesh {
        vertices,
        indices: mesh.triangles.clone(),
    })
}

fn merge_meshes(meshes: &[TriangleMesh]) -> ImportResult<TriangleMesh> {
    let mut merged = TriangleMesh::default();
    for mesh in meshes {
        let base = u32::try_from(merged.vertices.len())
            .map_err(|_| invalid("trop de sommets pour un maillage indexé 32 bits"))?;
        merged.vertices.extend_from_slice(&mesh.vertices);
        for triangle in &mesh.indices {
            merged.indices.push([
                triangle[0]
                    .checked_add(base)
                    .ok_or_else(|| invalid("indice trop grand"))?,
                triangle[1]
                    .checked_add(base)
                    .ok_or_else(|| invalid("indice trop grand"))?,
                triangle[2]
                    .checked_add(base)
                    .ok_or_else(|| invalid("indice trop grand"))?,
            ]);
        }
    }
    if merged.vertices.is_empty() || merged.indices.is_empty() {
        return Err(invalid("objet sans géométrie"));
    }
    if merged
        .indices
        .iter()
        .flatten()
        .any(|index| *index as usize >= merged.vertices.len())
    {
        return Err(invalid("triangle référençant un sommet absent"));
    }
    Ok(merged)
}

fn local(name: &[u8]) -> &[u8] {
    name.rsplit(|byte| *byte == b':').next().unwrap_or(name)
}

fn attr(event: &quick_xml::events::BytesStart<'_>, name: &[u8]) -> ImportResult<Option<String>> {
    for attribute in event.attributes().with_checks(false) {
        let attribute = attribute.map_err(|e| invalid(format!("attribut XML invalide : {e}")))?;
        if local(attribute.key.as_ref()) == name {
            return quick_xml::escape::unescape(&String::from_utf8_lossy(attribute.value.as_ref()))
                .map(|value| Some(value.into_owned()))
                .map_err(|e| invalid(format!("attribut XML invalide : {e}")));
        }
    }
    Ok(None)
}

fn required_attr(event: &quick_xml::events::BytesStart<'_>, name: &[u8]) -> ImportResult<String> {
    attr(event, name)?
        .ok_or_else(|| invalid(format!("attribut {} absent", String::from_utf8_lossy(name))))
}

fn parse_number<T: std::str::FromStr>(value: &str) -> ImportResult<T> {
    value
        .trim()
        .parse()
        .map_err(|_| invalid(format!("nombre invalide : {value}")))
}

fn parse_index(value: &str) -> ImportResult<u32> {
    parse_number(value)
}

fn transform_attr(event: &quick_xml::events::BytesStart<'_>) -> ImportResult<DMat4> {
    let Some(value) = attr(event, b"transform")? else {
        return Ok(DMat4::IDENTITY);
    };
    let values: Vec<f64> = value
        .split_whitespace()
        .map(parse_number)
        .collect::<ImportResult<_>>()?;
    if values.len() != 12 {
        return Err(invalid("une matrice 3MF doit contenir 12 nombres"));
    }
    // Le 3MF sérialise une matrice 4×3 en row-major ; glam attend des colonnes.
    Ok(DMat4::from_cols(
        DVec3::new(values[0], values[3], values[6]).extend(0.0),
        DVec3::new(values[1], values[4], values[7]).extend(0.0),
        DVec3::new(values[2], values[5], values[8]).extend(0.0),
        DVec3::new(values[9], values[10], values[11]).extend(1.0),
    ))
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
            },
            ProjectPlate {
                name: "Second".into(),
                settings: second,
            },
        ];
        (model, plates)
    }

    #[test]
    fn object_settings_and_layer_profile_survive_export_import() {
        let project = read_project(&exported()).unwrap();
        assert_eq!(project.objects.len(), 2);
        let right = &project.objects[1];
        assert_eq!(right.name, "Droite");
        assert_eq!(
            right.settings.get("wall_loops"),
            Some(&Value::String("5".into()))
        );
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

    #[test]
    fn plate_settings_survive_export_import() {
        let project = read_project(&exported()).unwrap();
        assert_eq!(project.plates.len(), 2);
        let first = &project.plates[0];
        assert_eq!(first.name, "Premier");
        assert_eq!(first.plate_type, DEFAULT_PLATE_TYPE);
        // Identifiants d'objet du 3MF écrit par Orca/libslic3r : la ressource
        // objet suit celle de son maillage (2, 4…), comme les projets Orca.
        assert_eq!(first.instances, vec![("2".to_string(), 0)]);
        let second = &project.plates[1];
        assert_eq!(second.name, "Second");
        assert_eq!(second.plate_type, "Smooth High Temp Plate");
        assert_eq!(second.instances, vec![("4".to_string(), 0)]);
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

    #[test]
    fn invalid_layer_profile_lines_are_ignored() {
        let profiles = parse_layer_heights_profiles(
            b"object_id=1|0;0.2;1;0.2;2;0.2\nobject_id=0|0;0.2;1;0.2;2;0.2\nobject_id=2|0;0.2\nnonsense\n",
        );
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[&1].len(), 6);
    }
}
