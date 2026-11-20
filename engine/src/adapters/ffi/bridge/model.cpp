// Implémentation des wrappers — un .cpp par domaine (R1) ; celui-ci : modèle.
#include "engine/src/adapters/ffi/bridge/model.hpp"
#include "engine/src/adapters/ffi/bridge.rs.h"

#include "libslic3r/Model.hpp"
#include "libslic3r/EmbossShape.hpp"
#include "libslic3r/TextConfiguration.hpp"
#include "libslic3r/Utils.hpp"
#include "libslic3r/PrintConfig.hpp"
#include "libslic3r/TriangleMesh.hpp"
// après Model.hpp : STEP.hpp référence Model sans le déclarer
#include "libslic3r/Format/STEP.hpp"

#include <boost/algorithm/string/predicate.hpp>
#include <nlohmann/json.hpp>

#include <algorithm>
#include <stdexcept>
#include <string>

namespace Slic3r {
// Défini dans PrintConfig.cpp (macro PRINT_CONFIG_CACHE_INITIALIZE), déclaré
// seulement en friend dans le header : déclaration locale pour l'appel.
int print_config_static_initializer();
} // namespace Slic3r

namespace webslicer {

namespace {

RawMesh to_raw_mesh(const Slic3r::TriangleMesh &mesh)
{
    RawMesh raw;
    const auto &its = mesh.its;
    raw.vertices.reserve(its.vertices.size() * 3);
    for (const auto &v : its.vertices) {
        raw.vertices.push_back(v.x());
        raw.vertices.push_back(v.y());
        raw.vertices.push_back(v.z());
    }
    raw.indices.reserve(its.indices.size() * 3);
    for (const auto &f : its.indices) {
        raw.indices.push_back(static_cast<uint32_t>(f.x()));
        raw.indices.push_back(static_cast<uint32_t>(f.y()));
        raw.indices.push_back(static_cast<uint32_t>(f.z()));
    }
    return raw;
}

std::array<double, 16> to_col_major(const Slic3r::Transform3d &t)
{
    std::array<double, 16> m{};
    const auto &mat = t.matrix();
    for (int c = 0; c < 4; ++c)
        for (int r = 0; r < 4; ++r)
            m[static_cast<size_t>(c) * 4 + r] = mat(r, c);
    return m;
}

// Dispatch par format : read_from_file ne gère que STL/OBJ/AMF ;
// les 3MF passent par read_from_archive, les STEP par load_step (OCCT).
Slic3r::Model read_any(const std::string &file)
{
    if (boost::iends_with(file, ".3mf")) {
        // Même appel que le CLI d'Orca (OrcaSlicer.cpp:1640) : tous les
        // out-params fournis — l'importeur BBS les déréférence sans garde.
        Slic3r::DynamicPrintConfig config;
        Slic3r::ConfigSubstitutionContext ctx(Slic3r::ForwardCompatibilitySubstitutionRule::Enable);
        Slic3r::PlateDataPtrs plate_data;
        std::vector<Slic3r::Preset *> project_presets;
        bool is_bbl_3mf = false;
        Slic3r::Semver file_version;
        const auto strategy = Slic3r::LoadStrategy::LoadModel | Slic3r::LoadStrategy::LoadConfig
            | Slic3r::LoadStrategy::AddDefaultInstances;
        Slic3r::Model model = Slic3r::Model::read_from_file(
            file, &config, &ctx, strategy, &plate_data, &project_presets, &is_bbl_3mf,
            &file_version);
        for (Slic3r::PlateData *plate : plate_data)
            delete plate;
        for (Slic3r::Preset *preset : project_presets)
            delete preset;
        return model;
    }
    if (boost::iends_with(file, ".step") || boost::iends_with(file, ".stp")) {
        Slic3r::Step step(file);
        if (step.load() != Slic3r::Step::Step_Status::LOAD_SUCCESS)
            throw std::runtime_error("échec de lecture STEP : " + file);
        Slic3r::Model model;
        bool cancel = false;
        if (step.mesh(&model, cancel, /*isSplitCompound=*/false)
            != Slic3r::Step::Step_Status::MESH_SUCCESS)
            throw std::runtime_error("échec de tessellation STEP : " + file);
        for (Slic3r::ModelObject *object : model.objects)
            object->add_instance();
        return model;
    }
    return Slic3r::Model::read_from_file(file);
}

} // namespace

rust::Vec<RawObject> model_to_raw(const Slic3r::Model &model);

// Import STEP avec les réglages du dialogue StepMeshDialog d'Orca (Model::
// read_from_step : Step::load puis Step::mesh(linear, angle, split compound)).
// Renvoie aussi le nombre de triangles annoncé par le dialogue
// (Step::get_triangle_num) pour ces déflexions.
RawStepMesh load_step_raw(rust::Str path_str, double linear, double angle, bool split_compound)
{
    const std::string file(path_str);
    if (!(linear > 0.0) || !(angle > 0.0))
        throw std::runtime_error("déflexions STEP strictement positives attendues");
    Slic3r::Step step(file);
    if (step.load() != Slic3r::Step::Step_Status::LOAD_SUCCESS)
        throw std::runtime_error("échec de lecture STEP : " + file);
    RawStepMesh out;
    out.dialog_triangles = step.get_triangle_num(linear, angle);
    Slic3r::Model model;
    bool cancel = false;
    if (step.mesh(&model, cancel, split_compound, linear, angle) != Slic3r::Step::Step_Status::MESH_SUCCESS)
        throw std::runtime_error("échec de tessellation STEP : " + file);
    if (model.objects.empty())
        throw std::runtime_error("STEP vide : " + file);
    for (Slic3r::ModelObject *object : model.objects)
        object->add_instance();
    out.objects = model_to_raw(model);
    return out;
}

// Point d'accès non anonyme (calib_project.cpp : modèles de resources/calib).
Slic3r::Model load_any_model(const std::string &file)
{
    return read_any(file);
}

void init_runtime(rust::Str temp_dir, rust::Str data_dir)
{
    Slic3r::set_temporary_dir(std::string(temp_dir));
    Slic3r::set_data_dir(std::string(data_dir));
    // Les caches statiques des StaticPrintConfig sont normalement remplis par
    // un initialiseur statique que --gc-sections (rustc) peut éliminer :
    // appel explicite, sans quoi load_bbs_3mf déréférence un cache vide.
    Slic3r::print_config_static_initializer();
}

rust::Vec<RawObject> model_to_raw(const Slic3r::Model &model)
{
    rust::Vec<RawObject> objects;
    for (const Slic3r::ModelObject *object : model.objects) {
        RawObject raw_object;
        raw_object.name = object->name;
        nlohmann::json object_config = nlohmann::json::object();
        for (const std::string &key : object->config.keys())
            object_config[key] = object->config.opt_serialize(key);
        raw_object.config_json = object_config.dump();
        for (double value : object->layer_height_profile.get())
            raw_object.layer_height_profile.push_back(value);
        if (!object->layer_config_ranges.empty()) {
            nlohmann::json ranges = nlohmann::json::array();
            for (const auto &[range, range_config] : object->layer_config_ranges) {
                nlohmann::json config = nlohmann::json::object();
                for (const std::string &key : range_config.keys())
                    config[key] = range_config.opt_serialize(key);
                ranges.push_back({{"min", range.first}, {"max", range.second}, {"config", config}});
            }
            raw_object.layer_ranges_json = ranges.dump();
        }
        for (const Slic3r::ModelVolume *volume : object->volumes) {
            RawVolume raw_volume;
            raw_volume.name = volume->name;
            raw_volume.matrix = to_col_major(volume->get_matrix());
            raw_volume.role = static_cast<uint8_t>(volume->type());
            const int extruder = volume->extruder_id();
            raw_volume.extruder = extruder > 0 ? static_cast<uint32_t>(extruder) : 0;
            raw_volume.mesh = to_raw_mesh(volume->mesh());
            nlohmann::json volume_config = nlohmann::json::object();
            for (const std::string &key : volume->config.keys())
                if (key != "extruder")
                    volume_config[key] = volume->config.opt_serialize(key);
            raw_volume.config_json = volume_config.dump();
            raw_volume.emboss_json = emboss_to_json(*volume);
            // Erreurs du maillage (TriangleMeshStats : restantes + réparées à la
            // lecture, ou <mesh_stat> du 3MF), cf. ObjectList::get_mesh_errors_info.
            {
                const Slic3r::TriangleMeshStats &stats = volume->mesh().stats();
                const Slic3r::RepairedMeshErrors &r = stats.repaired_errors;
                raw_volume.stats_json =
                    nlohmann::json{{"open_edges", std::max(stats.open_edges, 0)},
                                   {"repaired",
                                    {{"edges_fixed", std::max(r.edges_fixed, 0)},
                                     {"degenerate_facets", std::max(r.degenerate_facets, 0)},
                                     {"facets_removed", std::max(r.facets_removed, 0)},
                                     {"facets_reversed", std::max(r.facets_reversed, 0)},
                                     {"backwards_edges", std::max(r.backwards_edges, 0)}}}}
                        .dump();
            }
            // Connecteur de coupe (cut_information.xml : is_cut_connector()).
            if (volume->is_cut_connector())
                raw_volume.cut_json = nlohmann::json{
                    {"connector_type", int(volume->cut_info.connector_type)},
                    {"radius_tolerance", volume->cut_info.radius_tolerance},
                    {"height_tolerance", volume->cut_info.height_tolerance}}.dump();
            // Peintures (relecture 3MF) : même format hex que le writer d'Orca.
            const Slic3r::FacetsAnnotation *channels[4] = {
                &volume->supported_facets, &volume->seam_facets,
                &volume->mmu_segmentation_facets, &volume->fuzzy_skin_facets};
            for (uint8_t channel = 0; channel < 4; ++channel) {
                const Slic3r::FacetsAnnotation &facets = *channels[channel];
                if (facets.empty())
                    continue;
                for (const auto &mapping : facets.get_data().triangles_to_split) {
                    std::string state = facets.get_triangle_as_string(mapping.triangle_idx);
                    if (state.empty())
                        continue;
                    raw_volume.paint_channel.push_back(channel);
                    raw_volume.paint_triangle.push_back(static_cast<uint32_t>(mapping.triangle_idx));
                    raw_volume.paint_state.push_back(rust::String(state));
                }
            }
            raw_object.volumes.push_back(std::move(raw_volume));
        }
        if (object->is_cut())
            raw_object.cut_json = nlohmann::json{{"id", object->cut_id.id().id},
                                                 {"check_sum", object->cut_id.check_sum()},
                                                 {"connectors_cnt", object->cut_id.connectors_cnt()}}
                                      .dump();
        for (const Slic3r::ModelInstance *instance : object->instances) {
            RawInstance raw_instance;
            raw_instance.matrix = to_col_major(instance->get_matrix());
            raw_instance.printable = instance->printable;
            raw_object.instances.push_back(std::move(raw_instance));
        }
        objects.push_back(std::move(raw_object));
    }
    return objects;
}

rust::Vec<RawObject> load_model_raw(rust::Str path)
{
    const std::string file(path);
    Slic3r::Model model = read_any(file);
    if (model.objects.empty())
        throw std::runtime_error("aucun objet dans " + file);
    return model_to_raw(model);
}

size_t model_triangle_count(rust::Str path)
{
    const std::string file(path);
    Slic3r::Model model = read_any(file);
    if (model.objects.empty())
        throw std::runtime_error("aucun objet dans " + file);
    size_t count = 0;
    for (const Slic3r::ModelObject *object : model.objects)
        for (const Slic3r::ModelVolume *volume : object->volumes)
            count += volume->mesh().facets_count();
    return count;
}

size_t print_config_option_count()
{
    return Slic3r::print_config_def.options.size();
}


// ---------------------------------------------------------------------------
// Relief ré-éditable (lot 26) : paramètres de l'outil Emboss / SVG d'Orca
// conservés sur le volume (TextConfiguration + EmbossShape), écrits et relus
// par store_bbs_3mf / load_bbs_3mf (<slic3rpe:text>, <slic3rpe:shape>, SVG
// sous 3D/<nom>.svg). JSON web-slicer :
//   { "type": "text", "text", "font_file", "style_name", "size_mm", "depth_mm",
//     "boldness_mm"?, "skew"?, "char_gap"?, "line_gap"?, "per_glyph"?,
//     "align"?: ["left|center|right", "top|center|bottom"], "use_surface"? }
//   { "type": "svg", "svg", "file_name", "depth_mm", "width_mm"?, "use_surface"? }
namespace {
const char *const H_ALIGN[] = {"left", "center", "right"};
const char *const V_ALIGN[] = {"top", "center", "bottom"};
template<class T> T align_from(const nlohmann::json &j, const char *const (&names)[3], T fallback)
{
    if (!j.is_string())
        return fallback;
    for (int i = 0; i < 3; ++i)
        if (j.get<std::string>() == names[i])
            return static_cast<T>(i);
    return fallback;
}
} // namespace

std::string emboss_to_json(const Slic3r::ModelVolume &volume)
{
    using namespace Slic3r;
    nlohmann::json j;
    if (volume.text_configuration.has_value()) {
        const TextConfiguration &tc = *volume.text_configuration;
        const FontProp &fp = tc.style.prop;
        j["type"] = "text";
        j["text"] = tc.text;
        // Orca ne stocke que le nom du fichier de police (confidentialité).
        std::string file = tc.style.path;
        const size_t slash = file.find_last_of("/\\");
        if (slash != std::string::npos)
            file = file.substr(slash + 1);
        j["font_file"] = file;
        j["style_name"] = tc.style.name;
        j["size_mm"] = fp.size_in_mm;
        if (fp.boldness.has_value()) j["boldness_mm"] = *fp.boldness;
        if (fp.skew.has_value()) j["skew"] = *fp.skew;
        if (fp.char_gap.has_value()) j["char_gap"] = *fp.char_gap;
        if (fp.line_gap.has_value()) j["line_gap"] = *fp.line_gap;
        if (fp.per_glyph) j["per_glyph"] = true;
        j["align"] = {H_ALIGN[static_cast<int>(fp.align.first) % 3], V_ALIGN[static_cast<int>(fp.align.second) % 3]};
        if (volume.emboss_shape.has_value()) {
            j["depth_mm"] = volume.emboss_shape->projection.depth;
            if (volume.emboss_shape->projection.use_surface) j["use_surface"] = true;
        }
        return j.dump();
    }
    if (volume.emboss_shape.has_value() && volume.emboss_shape->svg_file.has_value()) {
        const EmbossShape &es = *volume.emboss_shape;
        const EmbossShape::SvgFile &svg = *es.svg_file;
        j["type"] = "svg";
        j["svg"] = svg.file_data ? *svg.file_data : std::string();
        std::string name = !svg.path.empty() ? svg.path : svg.path_in_3mf;
        const size_t slash = name.find_last_of("/\\");
        if (slash != std::string::npos)
            name = name.substr(slash + 1);
        j["file_name"] = name;
        j["depth_mm"] = es.projection.depth;
        if (es.projection.use_surface) j["use_surface"] = true;
        return j.dump();
    }
    return std::string();
}

void emboss_from_json(Slic3r::ModelVolume &volume, const std::string &text)
{
    using namespace Slic3r;
    if (text.empty())
        return;
    const nlohmann::json j = nlohmann::json::parse(text, nullptr, /*allow_exceptions=*/false);
    if (!j.is_object())
        return;
    const std::string type = j.value("type", std::string());
    EmbossShape shape;
    shape.projection.depth = j.value("depth_mm", 1.0);
    shape.projection.use_surface = j.value("use_surface", false);
    if (type == "text") {
        TextConfiguration tc;
        tc.text = j.value("text", std::string());
        tc.style.name = j.value("style_name", std::string());
        tc.style.path = j.value("font_file", std::string());
        // EmbossStyle::Type::file_path : police TrueType désignée par fichier.
        tc.style.type = EmbossStyle::Type::file_path;
        FontProp &fp = tc.style.prop;
        fp.size_in_mm = j.value("size_mm", 10.f);
        if (j.contains("boldness_mm") && j["boldness_mm"].is_number()) fp.boldness = j["boldness_mm"].get<float>();
        if (j.contains("skew") && j["skew"].is_number()) fp.skew = j["skew"].get<float>();
        if (j.contains("char_gap") && j["char_gap"].is_number()) fp.char_gap = j["char_gap"].get<int>();
        if (j.contains("line_gap") && j["line_gap"].is_number()) fp.line_gap = j["line_gap"].get<int>();
        fp.per_glyph = j.value("per_glyph", false);
        if (j.contains("align") && j["align"].is_array() && j["align"].size() == 2) {
            fp.align.first = align_from(j["align"][0], H_ALIGN, FontProp::HorizontalAlign::center);
            fp.align.second = align_from(j["align"][1], V_ALIGN, FontProp::VerticalAlign::center);
        }
        volume.text_configuration = std::move(tc);
        volume.emboss_shape = std::move(shape);
    } else if (type == "svg") {
        EmbossShape::SvgFile svg;
        svg.path = j.value("file_name", std::string("shape.svg"));
        svg.file_data = std::make_shared<std::string>(j.value("svg", std::string()));
        if (svg.file_data->empty())
            return;
        // Plater::publish → create_unique_3mf_filepath : « 3D/<nom>.svg ».
        std::string stem = svg.path;
        const size_t dot = stem.find_last_of('.');
        if (dot != std::string::npos)
            stem = stem.substr(0, dot);
        for (char &c : stem)
            if (c == '/' || c == '\\' || c == '"' || c == '<' || c == '>' || c == '&')
                c = '_';
        if (stem.empty())
            stem = "shape";
        std::string path_in_3mf = "3D/" + stem + ".svg";
        size_t suffix = 1;
        const Model *model = volume.get_object() ? volume.get_object()->get_model() : nullptr;
        auto taken = [&](const std::string &candidate) {
            if (!model)
                return false;
            for (const ModelObject *o : model->objects)
                for (const ModelVolume *v : o->volumes)
                    if (v != &volume && v->emboss_shape.has_value() && v->emboss_shape->svg_file.has_value() &&
                        v->emboss_shape->svg_file->path_in_3mf == candidate)
                        return true;
            return false;
        };
        while (taken(path_in_3mf))
            path_in_3mf = "3D/" + stem + "_" + std::to_string(++suffix) + ".svg";
        svg.path_in_3mf = path_in_3mf;
        shape.svg_file = std::move(svg);
        volume.emboss_shape = std::move(shape);
    }
}

} // namespace webslicer
