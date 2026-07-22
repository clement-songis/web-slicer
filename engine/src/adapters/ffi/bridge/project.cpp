// Wrappers projet 3MF : lecture (scène + config embarquée) et écriture
// compatible OrcaSlicer (T014).
#include "engine/src/adapters/ffi/bridge/model.hpp"
#include "engine/src/adapters/ffi/bridge.rs.h"

#include "libslic3r/Model.hpp"
#include "libslic3r/PrintConfig.hpp"
#include "libslic3r/TriangleMesh.hpp"
// après Model.hpp (dépendances de déclaration)
#include "libslic3r/Format/bbs_3mf.hpp"

#include <nlohmann/json.hpp>

#include <stdexcept>
#include <string>

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

void config_from_json(Slic3r::DynamicPrintConfig &config, const std::string &json_text)
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

// Applique une peinture au triangle sur une annotation de facettes (supports,
// couture, MMU, fuzzy). Les ids arrivent triés croissants côté Rust
// (`PaintedFacets::normalized`), contrat de `set_triangle_from_string`.
void apply_painted_facets(Slic3r::FacetsAnnotation &annotation,
                          const rust::Vec<RawPaintedFacet> &facets)
{
    if (facets.empty())
        return;
    annotation.reserve(static_cast<int>(facets.size()));
    for (const RawPaintedFacet &facet : facets)
        annotation.set_triangle_from_string(
            static_cast<int>(facet.triangle),
            std::string(facet.encoding.data(), facet.encoding.size()));
}

} // namespace

void raw_into_model(const rust::Vec<RawObject> &objects, Slic3r::Model &model)
{
    for (const RawObject &raw_object : objects) {
        Slic3r::ModelObject *object = model.add_object();
        object->name = std::string(raw_object.name);
        for (const RawVolume &raw_volume : raw_object.volumes) {
            Slic3r::ModelVolume *volume =
                object->add_volume(raw_to_mesh(raw_volume.mesh));
            volume->name = std::string(raw_volume.name);
            volume->set_type(static_cast<Slic3r::ModelVolumeType>(raw_volume.role));
            volume->set_transformation(
                Slic3r::Geometry::Transformation(from_col_major(raw_volume.matrix)));
            // Peinture au triangle (T056+) : injectée dans libslic3r via l'API
            // `FacetsAnnotation` — même chemin que le lecteur 3MF d'OrcaSlicer.
            apply_painted_facets(volume->supported_facets, raw_volume.supported_facets);
            apply_painted_facets(volume->seam_facets, raw_volume.seam_facets);
            apply_painted_facets(volume->mmu_segmentation_facets, raw_volume.mmu_facets);
            apply_painted_facets(volume->fuzzy_skin_facets, raw_volume.fuzzy_facets);
        }
        for (const RawInstance &raw_instance : raw_object.instances) {
            Slic3r::ModelInstance *instance = object->add_instance();
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
    for (Slic3r::PlateData *plate : plate_data)
        delete plate;
    for (Slic3r::Preset *preset : project_presets)
        delete preset;

    RawProject project;
    project.objects = model_to_raw(model);
    project.config_json = config_to_json(config);
    return project;
}

void write_project_3mf_raw(const rust::Vec<RawObject> &objects, rust::Str config_json,
                           rust::Str out_path)
{
    Slic3r::Model model;
    raw_into_model(objects, model);

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

} // namespace webslicer
