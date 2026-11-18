// Wrappers projet 3MF : lecture (scène + config embarquée) et écriture
// compatible OrcaSlicer (T014).
// Les en-têtes GCodeProcessor utilisent BOOST_LOG_TRIVIAL : Boost.Log est lié
// dynamiquement (symboles v2_mt_posix), pas en variante statique.
#ifndef BOOST_LOG_DYN_LINK
#define BOOST_LOG_DYN_LINK 1
#endif
#include "engine/src/adapters/ffi/bridge/model.hpp"
#include "engine/src/adapters/ffi/bridge.rs.h"

#include "libslic3r/Model.hpp"
#include "libslic3r/Print.hpp"
#include "libslic3r/Slicing.hpp"
#include "libslic3r/PrintConfig.hpp"
#include "libslic3r/TriangleMesh.hpp"
// après Model.hpp (dépendances de déclaration)
#include "libslic3r/Format/bbs_3mf.hpp"
#include "libslic3r/GCode/GCodeProcessor.hpp"

#include <boost/filesystem.hpp>
#include <cstdio>

#include <nlohmann/json.hpp>

#include <algorithm>
#include <cctype>
#include <cmath>
#include <stdexcept>
#include <string>
#include <utility>
#include <vector>

namespace webslicer {

// helper partagé, défini dans model.cpp
rust::Vec<RawObject> model_to_raw(const Slic3r::Model &model);

namespace {

std::string config_to_json(const Slic3r::DynamicPrintConfig &config)
{
    nlohmann::json j = nlohmann::json::object();
    for (const std::string &key : config.keys())
        j[key] = config.opt_serialize(key);
    return j.dump();
}

template <typename TConfig>
void config_from_json(TConfig &config, const std::string &json_text)
{
    const auto j = nlohmann::json::parse(json_text);
    Slic3r::ConfigSubstitutionContext ctx(
        Slic3r::ForwardCompatibilitySubstitutionRule::Enable);
    for (auto it = j.begin(); it != j.end(); ++it) {
        const std::string value = it.value().get<std::string>();
        config.set_deserialize(it.key(), value, ctx);
    }
}

Slic3r::TriangleMesh raw_to_mesh(const RawMesh &raw)
{
    indexed_triangle_set its;
    its.vertices.reserve(raw.vertices.size() / 3);
    for (size_t i = 0; i + 2 < raw.vertices.size(); i += 3)
        its.vertices.emplace_back(raw.vertices[i], raw.vertices[i + 1], raw.vertices[i + 2]);
    its.indices.reserve(raw.indices.size() / 3);
    for (size_t i = 0; i + 2 < raw.indices.size(); i += 3)
        its.indices.emplace_back(static_cast<int>(raw.indices[i]),
                                 static_cast<int>(raw.indices[i + 1]),
                                 static_cast<int>(raw.indices[i + 2]));
    return Slic3r::TriangleMesh(std::move(its));
}

Slic3r::Transform3d from_col_major(const std::array<double, 16> &m)
{
    Slic3r::Transform3d t = Slic3r::Transform3d::Identity();
    auto &mat = t.matrix();
    for (int c = 0; c < 4; ++c)
        for (int r = 0; r < 4; ++r)
            mat(r, c) = m[static_cast<size_t>(c) * 4 + r];
    return t;
}

} // namespace

void raw_into_model(const rust::Vec<RawObject> &objects, Slic3r::Model &model)
{
    for (const RawObject &raw_object : objects) {
        Slic3r::ModelObject *object = model.add_object();
        object->name = std::string(raw_object.name);
        if (!raw_object.config_json.empty())
            config_from_json(object->config, std::string(raw_object.config_json));
        std::vector<coordf_t> layer_profile;
        layer_profile.reserve(raw_object.layer_height_profile.size());
        for (double value : raw_object.layer_height_profile)
            layer_profile.push_back(value);
        object->layer_height_profile.set(std::move(layer_profile));
        // Plages de hauteur à réglages propres (Orca `layer_config_ranges`).
        if (!raw_object.layer_ranges_json.empty()) {
            const auto ranges = nlohmann::json::parse(std::string(raw_object.layer_ranges_json));
            for (const auto &range : ranges) {
                Slic3r::DynamicPrintConfig range_config;
                config_from_json(range_config, range.at("config").dump());
                object->layer_config_ranges[{range.at("min").get<double>(),
                                             range.at("max").get<double>()}]
                    .assign_config(std::move(range_config));
            }
        }
        for (const RawVolume &raw_volume : raw_object.volumes) {
            Slic3r::ModelVolume *volume =
                object->add_volume(raw_to_mesh(raw_volume.mesh),
                                   Slic3r::ModelVolumeType::MODEL_PART,
                                   /*modify_to_center_geometry=*/false);
            volume->name = std::string(raw_volume.name);
            volume->set_type(static_cast<Slic3r::ModelVolumeType>(raw_volume.role));
            if (!raw_volume.config_json.empty())
                config_from_json(volume->config, std::string(raw_volume.config_json));
            if (raw_volume.extruder > 0)
                volume->config.set("extruder", static_cast<int>(raw_volume.extruder));
            volume->set_transformation(
                Slic3r::Geometry::Transformation(from_col_major(raw_volume.matrix)));
            // Peintures (supports / couture / MMU / fuzzy) comme le lecteur 3MF
            // d'Orca : une chaîne hex par triangle, indices croissants par canal.
            const size_t painted = std::min({raw_volume.paint_channel.size(),
                                             raw_volume.paint_triangle.size(),
                                             raw_volume.paint_state.size()});
            const int triangles = static_cast<int>(volume->mesh().its.indices.size());
            for (uint8_t channel = 0; channel < 4; ++channel) {
                Slic3r::FacetsAnnotation &facets = channel == 0 ? volume->supported_facets
                    : channel == 1                              ? volume->seam_facets
                    : channel == 2                              ? volume->mmu_segmentation_facets
                                                                : volume->fuzzy_skin_facets;
                std::vector<std::pair<int, std::string>> entries;
                for (size_t i = 0; i < painted; ++i) {
                    const int triangle = static_cast<int>(raw_volume.paint_triangle[i]);
                    std::string state(raw_volume.paint_state[i]);
                    if (raw_volume.paint_channel[i] != channel || triangle < 0 ||
                        triangle >= triangles || state.empty())
                        continue;
                    for (char &c : state)
                        c = static_cast<char>(std::toupper(static_cast<unsigned char>(c)));
                    if (state.find_first_not_of("0123456789ABCDEF") != std::string::npos)
                        continue;
                    entries.emplace_back(triangle, std::move(state));
                }
                std::sort(entries.begin(), entries.end(),
                          [](const auto &a, const auto &b) { return a.first < b.first; });
                int last = -1;
                for (const auto &[triangle, state] : entries) {
                    if (triangle == last)
                        continue;
                    facets.set_triangle_from_string(triangle, state);
                    last = triangle;
                }
                // Horodatage propre à cette peinture (lot 25, tour d'essuyage
                // coupée sur un objet peint seul) : set_triangle_from_string ne
                // touche pas le timestamp, resté à 1 comme pour TOUT volume non
                // peint. Or ModelVolume::get_extruders met en cache les
                // filaments peints sous `mmuseg_ts`, membre NON initialisé
                // par les constructeurs de ModelVolume (Orca 2.4.1) : la copie
                // faite par Print::apply héritait d'une valeur de tas égale à 1
                // (ancien volume) → cache vide, un seul filament compté,
                // normalize_fdm_2 coupait la tour. Dans la GUI d'Orca, peindre
                // passe par FacetsAnnotation::set() qui touche l'horodatage.
                if (!entries.empty())
                    facets.touch();
            }
        }
        for (const RawInstance &raw_instance : raw_object.instances) {
            Slic3r::ModelInstance *instance = object->add_instance();
            instance->printable = raw_instance.printable;
            instance->set_transformation(
                Slic3r::Geometry::Transformation(from_col_major(raw_instance.matrix)));
        }
        if (object->instances.empty())
            object->add_instance();
    }
}

RawProject read_project_3mf_raw(rust::Str path)
{
    const std::string file(path);
    Slic3r::DynamicPrintConfig config;
    Slic3r::ConfigSubstitutionContext ctx(
        Slic3r::ForwardCompatibilitySubstitutionRule::Enable);
    Slic3r::PlateDataPtrs plate_data;
    std::vector<Slic3r::Preset *> project_presets;
    bool is_bbl_3mf = false;
    Slic3r::Semver file_version;
    const auto strategy = Slic3r::LoadStrategy::LoadModel | Slic3r::LoadStrategy::LoadConfig
        | Slic3r::LoadStrategy::AddDefaultInstances;
    Slic3r::Model model = Slic3r::Model::read_from_file(
        file, &config, &ctx, strategy, &plate_data, &project_presets, &is_bbl_3mf,
        &file_version);
    // Plateaux (PartPlateList::load_from_3mf_structure lit les mêmes PlateData) :
    // objects_and_instances = (indice d'objet, indice d'instance) du modèle chargé.
    // obj_inst_map : identifiant d'objet 3MF → (instance, identify_id) ; le
    // chargeur pose identify_id dans ModelInstance::loaded_id, d'où la paire
    // (indice d'objet, indice d'instance) du modèle chargé.
    // Sans identify_id (projets non Bambu/Orca récents), repli : les objets du
    // modèle suivent l'ordre croissant des identifiants d'objet 3MF.
    std::vector<int> object_ids_3mf;
    for (const Slic3r::PlateData *plate : plate_data)
        for (const auto &entry : plate->obj_inst_map)
            object_ids_3mf.push_back(entry.first);
    std::sort(object_ids_3mf.begin(), object_ids_3mf.end());
    object_ids_3mf.erase(std::unique(object_ids_3mf.begin(), object_ids_3mf.end()), object_ids_3mf.end());
    rust::Vec<RawProjectPlate> plates;
    for (Slic3r::PlateData *plate : plate_data) {
        RawProjectPlate raw;
        raw.plate_index = static_cast<uint32_t>(std::max(0, plate->plate_index));
        raw.name = plate->plate_name;
        raw.config_json = config_to_json(plate->config);
        for (const auto &entry : plate->obj_inst_map) {
            const int instance_id = entry.second.first;
            const int identify_id = entry.second.second;
            bool found = false;
            for (size_t o = 0; o < model.objects.size() && identify_id > 0; ++o)
                for (size_t i = 0; i < model.objects[o]->instances.size(); ++i)
                    if (model.objects[o]->instances[i]->loaded_id == identify_id) {
                        raw.obj_inst.push_back(static_cast<uint32_t>(o));
                        raw.obj_inst.push_back(static_cast<uint32_t>(i));
                        found = true;
                    }
            if (!found) {
                const auto rank = std::lower_bound(object_ids_3mf.begin(), object_ids_3mf.end(), entry.first) -
                                  object_ids_3mf.begin();
                if (static_cast<size_t>(rank) < model.objects.size() && instance_id >= 0 &&
                    static_cast<size_t>(instance_id) < model.objects[rank]->instances.size()) {
                    raw.obj_inst.push_back(static_cast<uint32_t>(rank));
                    raw.obj_inst.push_back(static_cast<uint32_t>(instance_id));
                }
            }
        }
        plates.push_back(std::move(raw));
        delete plate;
    }
    for (Slic3r::Preset *preset : project_presets)
        delete preset;

    // Comme le CLI à la lecture d'un 3MF (OrcaSlicer.cpp:1769-1771 :
    // `config.option<ConfigOptionStrings>(…, true)`) : ces options sont créées
    // vides si le projet ne les porte pas, et apparaissent dans le G-code.
    for (const char *key : {"upward_compatible_machine", "print_compatible_printers",
                            "different_settings_to_system"})
        if (!config.has(key))
            config.set_key_value(key, new Slic3r::ConfigOptionStrings());

    RawProject project;
    project.objects = model_to_raw(model);
    project.config_json = config_to_json(config);
    project.custom_gcodes_json = custom_gcodes_to_json(model);
    project.plates = std::move(plates);
    // project_settings.config d'Orca (DynamicPrintConfig::save_to_json) :
    // vecteurs en tableaux de chaînes, scalaires en chaîne.
    {
        nlohmann::json j = nlohmann::json::object();
        for (const std::string &key : config.keys()) {
            const Slic3r::ConfigOption *opt = config.option(key);
            if (opt != nullptr && opt->is_vector())
                j[key] = static_cast<const Slic3r::ConfigOptionVectorBase *>(opt)->vserialize();
            else
                j[key] = config.opt_serialize(key);
        }
        project.project_config_json = j.dump();
    }
    return project;
}

namespace {
const char *custom_kind(Slic3r::CustomGCode::Type t)
{
    switch (t) {
    case Slic3r::CustomGCode::ColorChange: return "color_change";
    case Slic3r::CustomGCode::PausePrint: return "pause_print";
    case Slic3r::CustomGCode::ToolChange: return "tool_change";
    case Slic3r::CustomGCode::Template: return "template";
    default: return "custom";
    }
}
Slic3r::CustomGCode::Type custom_type(const std::string &k)
{
    if (k == "color_change") return Slic3r::CustomGCode::ColorChange;
    if (k == "pause_print") return Slic3r::CustomGCode::PausePrint;
    if (k == "tool_change") return Slic3r::CustomGCode::ToolChange;
    if (k == "template") return Slic3r::CustomGCode::Template;
    return Slic3r::CustomGCode::Custom;
}
const char *mode_name(Slic3r::CustomGCode::Mode m)
{
    switch (m) {
    case Slic3r::CustomGCode::MultiAsSingle: return "multi_as_single";
    case Slic3r::CustomGCode::MultiExtruder: return "multi_extruder";
    default: return "single_extruder";
    }
}
Slic3r::CustomGCode::Mode mode_value(const std::string &m)
{
    if (m == "multi_as_single") return Slic3r::CustomGCode::MultiAsSingle;
    if (m == "multi_extruder") return Slic3r::CustomGCode::MultiExtruder;
    return Slic3r::CustomGCode::SingleExtruder;
}
} // namespace

// Model::plates_custom_gcodes (lu par le lecteur 3MF d'Orca depuis
// Metadata/custom_gcode_per_layer.xml, écrit par bbs_3mf.cpp:8462).
std::string custom_gcodes_to_json(const Slic3r::Model &model)
{
    nlohmann::json out = nlohmann::json::array();
    for (const auto &[plate, info] : model.plates_custom_gcodes) {
        if (info.gcodes.empty())
            continue;
        nlohmann::json items = nlohmann::json::array();
        for (const auto &item : info.gcodes)
            items.push_back({{"print_z", item.print_z}, {"kind", custom_kind(item.type)},
                             {"extruder", item.extruder}, {"color", item.color},
                             {"extra", item.extra}});
        out.push_back({{"plate_index", plate}, {"mode", mode_name(info.mode)}, {"items", items}});
    }
    return out.dump();
}

// only_plate >= 0 : ne garde que ce plateau ; as_plate >= 0 : le range sous cet
// index (tranchage : plateau courant du modèle).
void custom_gcodes_from_json(Slic3r::Model &model, const std::string &json, int only_plate,
                             int as_plate)
{
    if (json.empty())
        return;
    const nlohmann::json plates = nlohmann::json::parse(json);
    for (const auto &plate : plates) {
        const int index = plate.value("plate_index", 0);
        if (only_plate >= 0 && index != only_plate)
            continue;
        Slic3r::CustomGCode::Info info;
        info.mode = mode_value(plate.value("mode", std::string("single_extruder")));
        for (const auto &it : plate.at("items")) {
            Slic3r::CustomGCode::Item item;
            item.print_z = it.at("print_z").get<double>();
            item.type = custom_type(it.value("kind", std::string("custom")));
            item.extruder = it.value("extruder", 1);
            item.color = it.value("color", std::string());
            item.extra = it.value("extra", std::string());
            info.gcodes.push_back(item);
        }
        std::sort(info.gcodes.begin(), info.gcodes.end());
        if (!info.gcodes.empty())
            model.plates_custom_gcodes[as_plate >= 0 ? as_plate : index] = info;
    }
}

// Point d'accès public à config_to_json (calib_project.cpp).
std::string config_to_json_public(const Slic3r::DynamicPrintConfig &config)
{
    return config_to_json(config);
}

// Point d'accès non template à config_from_json (arrange.cpp).
void config_from_json_print(Slic3r::DynamicPrintConfig &config, const std::string &json_text)
{
    config_from_json(config, json_text);
}

void write_project_3mf_raw(const rust::Vec<RawObject> &objects, rust::Str config_json,
                           rust::Str out_path, rust::Str custom_gcodes_json)
{
    Slic3r::Model model;
    raw_into_model(objects, model);
    custom_gcodes_from_json(model, std::string(custom_gcodes_json));

    Slic3r::DynamicPrintConfig config;
    if (!config_json.empty())
        config_from_json(config, std::string(config_json));

    const std::string out(out_path);
    Slic3r::StoreParams params;
    params.path = out.c_str();
    params.model = &model;
    params.config = &config;
    params.strategy = Slic3r::SaveStrategy::Silence;
    if (!Slic3r::store_bbs_3mf(params))
        throw std::runtime_error("échec d'écriture 3MF : " + out);
}

// « Save project » d'Orca (Plater::priv::save_project → export_3mf(SplitModel |
// ShareMesh)) : projet 3MF écrit par store_bbs_3mf avec ses plateaux
// (PartPlateList::store_to_3mf_structure sans tranchage : index, nom, réglages
// du plateau, instances posées), G-code personnalisés par couche et config.
namespace {
// Copie récursive (fichiers ordinaires) de `from` vers `to` ; chemins relatifs.
std::vector<std::string> copy_tree(const boost::filesystem::path &from, const boost::filesystem::path &to)
{
    namespace fs = boost::filesystem;
    std::vector<std::string> copied;
    if (!fs::is_directory(from))
        return copied;
    for (fs::recursive_directory_iterator it(from), end; it != end; ++it) {
        if (!fs::is_regular_file(it->path()))
            continue;
        const fs::path rel = fs::relative(it->path(), from);
        fs::create_directories((to / rel).parent_path());
        fs::copy_file(it->path(), to / rel, fs::copy_options::overwrite_existing);
        copied.push_back(rel.generic_string());
    }
    std::sort(copied.begin(), copied.end());
    return copied;
}
} // namespace

// Fichiers auxiliaires d'un projet 3MF (dossier Auxiliaries/ : images du
// modèle, nomenclature, guide d'assemblage, autres), extraits par load_bbs_3mf
// dans le dossier temporaire du modèle (Model::get_auxiliary_file_temp_path)
// comme à l'ouverture d'un projet dans Orca, puis copiés dans `out_dir`.
rust::Vec<rust::String> read_project_auxiliaries_raw(rust::Str path, rust::Str out_dir)
{
    using namespace Slic3r;
    DynamicPrintConfig config;
    ConfigSubstitutionContext ctx{ForwardCompatibilitySubstitutionRule::Enable};
    Model model = Model::read_from_file(std::string(path), &config, &ctx,
                                        LoadStrategy::LoadModel | LoadStrategy::LoadAuxiliary);
    rust::Vec<rust::String> out;
    for (const std::string &rel : copy_tree(model.get_auxiliary_file_temp_path(), std::string(out_dir)))
        out.push_back(rust::String(rel));
    return out;
}

void write_project_plates_raw(const rust::Vec<RawObject> &objects, rust::Str config_json,
                              rust::Str custom_gcodes_json,
                              const rust::Vec<RawProjectPlate> &plates, rust::Str aux_dir, rust::Str out_path)
{
    using namespace Slic3r;
    Model model;
    raw_into_model(objects, model);
    // Fichiers auxiliaires : déposés dans le dossier temporaire du modèle, que
    // store_bbs_3mf empaquette sous Auxiliaries/ (comme Orca).
    if (!aux_dir.empty())
        copy_tree(std::string(aux_dir), model.get_auxiliary_file_temp_path());
    custom_gcodes_from_json(model, std::string(custom_gcodes_json));
    // Extrudeur de l'objet (ModelObject::config « extruder », celui qu'Orca
    // affiche et relit) : la scène le porte sur les volumes ; on le remonte
    // sur l'objet s'il n'y est pas déjà, comme l'onglet Objets d'Orca.
    for (ModelObject *object : model.objects) {
        if (object->config.has("extruder"))
            continue;
        for (const ModelVolume *volume : object->volumes)
            if (volume->is_model_part() && volume->config.has("extruder")) {
                object->config.set("extruder", volume->config.extruder());
                break;
            }
    }
    DynamicPrintConfig config;
    if (!config_json.empty())
        config_from_json(config, std::string(config_json));

    PlateDataPtrs plate_data;
    struct Guard {
        PlateDataPtrs &p;
        ~Guard() { release_PlateData_list(p); }
    } guard{plate_data};
    for (const RawProjectPlate &raw : plates) {
        auto *plate = new PlateData();
        plate_data.push_back(plate);
        plate->plate_index = static_cast<int>(raw.plate_index);
        plate->plate_name = std::string(raw.name);
        // PartPlate::init : tout plateau porte filament_map_mode = Auto For Flush.
        plate->config.option<ConfigOptionEnum<FilamentMapMode>>("filament_map_mode", true)->value =
            FilamentMapMode::fmmAutoForFlush;
        if (!raw.config_json.empty())
            config_from_json(plate->config, std::string(raw.config_json));
        for (size_t i = 0; i + 1 < raw.obj_inst.size(); i += 2)
            plate->objects_and_instances.emplace_back(static_cast<int>(raw.obj_inst[i]),
                                                      static_cast<int>(raw.obj_inst[i + 1]));
    }

    const std::string out(out_path);
    StoreParams params;
    params.path = out.c_str();
    params.model = &model;
    params.plate_data_list = plate_data;
    params.config = &config;
    params.strategy = SaveStrategy::SplitModel | SaveStrategy::ShareMesh;
    if (!store_bbs_3mf(params))
        throw std::runtime_error("échec d'écriture 3MF : " + out);
}

// « Export plate sliced file » / « Export all plate sliced file » d'Orca
// (Plater::export_gcode_3mf → export_3mf(Silence | SplitModel | WithGcode |
// SkipModel, plate_idx)) : un .gcode.3mf avec Metadata/plate_<n>.gcode (+ md5)
// par plateau tranché et slice_info.config. Les PlateData sont remplies comme
// PartPlateList::store_to_3mf_structure : prédiction = temps du mode Normal,
// poids = somme des grammes, filaments via PlateData::parse_filament_info, le
// résultat venant de GCodeProcessor::process_file sur le G-code du plateau.
void write_gcode_3mf_raw(const rust::Vec<RawObject> &objects, rust::Str config_json,
                         const rust::Vec<RawPlateGcode> &plates, int32_t export_plate_idx,
                         rust::Str out_path)
{
    using namespace Slic3r;
    Model model;
    raw_into_model(objects, model);
    // Config complète comme celle que la GUI passe à export_3mf (full_config) :
    // défauts FullPrintConfig + valeurs résolues fournies (store_bbs_3mf lit
    // gcode_flavor, nozzle_diameter… sans tester leur présence).
    // Désérialisation dans une config vierge (options créées depuis leur
    // définition) puis complément des clés absentes par les défauts de
    // print_config_def : désérialiser dans full_print_config() plante sur les
    // vecteurs d'enums génériques (keys_map non relié).
    DynamicPrintConfig config;
    if (!config_json.empty())
        config_from_json(config, std::string(config_json));
    for (const std::string &key : FullPrintConfig::defaults().keys()) {
        if (config.has(key))
            continue;
        const ConfigOptionDef *def = print_config_def.get(key);
        if (def != nullptr && def->default_value)
            config.set_key_value(key, def->default_value->clone());
    }

    PlateDataPtrs plate_data;
    struct Guard {
        PlateDataPtrs &p;
        ~Guard() { release_PlateData_list(p); }
    } guard{plate_data};
    for (const RawPlateGcode &raw : plates) {
        auto *plate = new PlateData();
        plate_data.push_back(plate);
        plate->plate_index = static_cast<int>(raw.plate_index);
        for (size_t i = 0; i + 1 < raw.obj_inst.size(); i += 2)
            plate->objects_and_instances.emplace_back(static_cast<int>(raw.obj_inst[i]),
                                                      static_cast<int>(raw.obj_inst[i + 1]));
        const std::string gcode(raw.gcode_path);
        const bool exported = export_plate_idx < 0 || export_plate_idx == plate->plate_index;
        if (gcode.empty() || !exported)
            continue;
        GCodeProcessor::s_IsBBLPrinter = true;
        GCodeProcessor processor;
        processor.enable_stealth_time_estimator(true);
        processor.enable_machine_envelope_processing(true);
        processor.process_file(gcode);
        GCodeProcessorResult &result = processor.result();
        plate->gcode_file = gcode;
        plate->is_sliced_valid = true;
        plate->gcode_prediction = std::to_string(static_cast<int>(
            result.print_statistics.modes[static_cast<size_t>(PrintEstimatedStatistics::ETimeMode::Normal)].time));
        plate->toolpath_outside = result.toolpath_outside;
        plate->timelapse_warning_code = result.timelapse_warning_code;
        // GCode::_do_export : label_object_enabled = exclude_object (imprimantes
        // non BBL) ; GCodeProcessor::apply_config : limit_filament_maps =
        // zéros de la taille de filament_map. process_file ne les recalcule pas.
        if (const auto *exclude = config.option<ConfigOptionBool>("exclude_object"))
            plate->is_label_object_enabled = exclude->value;
        if (const auto *map = config.option<ConfigOptionInts>("filament_map"))
            plate->limit_filament_maps.assign(map->values.size(), 0);
        if (const auto *nozzle = config.option<ConfigOptionFloats>("nozzle_diameter"))
            plate->nozzle_diameters = nozzle->serialize();
        plate->parse_filament_info(&result);
        // Type / couleur / id de filament comme le CLI (OrcaSlicer.cpp:6410).
        const auto *types = config.option<ConfigOptionStrings>("filament_type");
        const auto *colours = config.option<ConfigOptionStrings>("filament_colour");
        const auto *ids = config.option<ConfigOptionStrings>("filament_ids");
        // get_at sur un vecteur vide est une assertion : valeur par défaut alors.
        auto at = [](const ConfigOptionStrings *opt, int id, const std::string &fallback) {
            return opt && !opt->values.empty() ? opt->get_at(id) : fallback;
        };
        for (FilamentInfo &f : plate->slice_filaments_info) {
            f.type = at(types, f.id, std::string());
            f.color = at(colours, f.id, "#FFFFFF");
            f.filament_id = at(ids, f.id, std::string());
        }
        double weight = 0.0;
        for (const FilamentInfo &f : plate->slice_filaments_info)
            weight += f.used_g;
        if (weight != 0.0) {
            char buf[64];
            std::snprintf(buf, sizeof(buf), "%.2f", weight);
            plate->gcode_weight = buf;
        }
    }

    const std::string out(out_path);
    StoreParams params;
    params.path = out.c_str();
    params.model = &model;
    params.plate_data_list = plate_data;
    params.export_plate_idx = export_plate_idx;
    params.config = &config;
    params.strategy = SaveStrategy::Silence | SaveStrategy::SplitModel | SaveStrategy::WithGcode |
                      SaveStrategy::SkipModel;
    if (!store_bbs_3mf(params))
        throw std::runtime_error("échec d'écriture du .gcode.3mf : " + out);
}

// Outil « Variable layer height » d'Orca (GLCanvas3D::LayersEditing) :
// - SlicingParameters = PrintObject::slicing_parameters(full_config, object,
//   object->max_z(), shrinkage) comme update_slicing_parameters (GLCanvas3D.cpp:719) ;
//   config complète = défauts FullPrintConfig + valeurs résolues fournies ;
// - op 0 : layer_height_profile_adaptive(params, object, quality) (Slicing.cpp:309) ;
// - op 1 : smooth_height_profile(profile, params, {radius, keep_min}) (Slicing.cpp:409).
rust::Vec<double> layer_profile_raw(const rust::Vec<RawObject> &objects, rust::Str config_json,
                                    uint8_t op, float quality, const rust::Vec<double> &profile,
                                    uint32_t radius, bool keep_min)
{
    if (objects.size() != 1)
        throw std::runtime_error("un seul objet attendu");
    Slic3r::Model model;
    raw_into_model(objects, model);
    Slic3r::ModelObject &object = *model.objects.front();
    if (object.instances.empty())
        throw std::runtime_error("objet sans instance");

    Slic3r::DynamicPrintConfig config = Slic3r::DynamicPrintConfig::full_print_config();
    if (!config_json.empty())
        config_from_json(config, std::string(config_json));

    const float max_z = static_cast<float>(object.max_z());
    if (!(max_z > 0.f))
        throw std::runtime_error("objet de hauteur nulle");
    const Slic3r::SlicingParameters params = Slic3r::PrintObject::slicing_parameters(
        config, object, max_z, Slic3r::Vec3d::Ones());

    std::vector<double> result;
    if (op == 0) {
        result = Slic3r::layer_height_profile_adaptive(params, object, quality);
    } else if (op == 1) {
        std::vector<double> in(profile.begin(), profile.end());
        result = Slic3r::smooth_height_profile(
            in, params, Slic3r::HeightProfileSmoothingParams(radius, keep_min));
    } else {
        throw std::runtime_error("opération de profil inconnue");
    }
    rust::Vec<double> out;
    out.reserve(result.size());
    for (double v : result)
        out.push_back(v);
    return out;
}

// Pinceau de l'outil « Variable layer height » (GLCanvas3D::LayersEditing::
// adjust_layer_height_profile, GLCanvas3D.cpp:643) : profil validé/initialisé
// par PrintObject::update_layer_height_profile (comme la GUI à l'ouverture),
// puis Slic3r::adjust_layer_height_profile(z, delta, band_width, action).
rust::Vec<double> adjust_layer_profile_raw(const rust::Vec<RawObject> &objects,
                                           rust::Str config_json, const rust::Vec<double> &profile,
                                           double z, double delta, double band_width,
                                           uint8_t action)
{
    if (objects.size() != 1)
        throw std::runtime_error("un seul objet attendu");
    if (action > 3)
        throw std::runtime_error("action de pinceau inconnue");
    if (!std::isfinite(z) || !std::isfinite(delta) || !(band_width > 0.) || !std::isfinite(band_width))
        throw std::runtime_error("paramètres de pinceau invalides");
    Slic3r::Model model;
    raw_into_model(objects, model);
    Slic3r::ModelObject &object = *model.objects.front();
    if (object.instances.empty())
        throw std::runtime_error("objet sans instance");
    Slic3r::DynamicPrintConfig config = Slic3r::DynamicPrintConfig::full_print_config();
    if (!config_json.empty())
        config_from_json(config, std::string(config_json));
    const float max_z = static_cast<float>(object.max_z());
    if (!(max_z > 0.f))
        throw std::runtime_error("objet de hauteur nulle");
    const Slic3r::SlicingParameters params = Slic3r::PrintObject::slicing_parameters(
        config, object, max_z, Slic3r::Vec3d::Ones());
    std::vector<double> lhp(profile.begin(), profile.end());
    Slic3r::PrintObject::update_layer_height_profile(object, params, lhp);
    Slic3r::adjust_layer_height_profile(object, params, lhp, z, delta, band_width,
                                        static_cast<Slic3r::LayerHeightEditActionType>(action));
    rust::Vec<double> out;
    out.reserve(lhp.size());
    for (double v : lhp)
        out.push_back(v);
    return out;
}

} // namespace webslicer
