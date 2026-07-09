// Implémentation des wrappers — un .cpp par domaine (R1) ; celui-ci : modèle.
#include "engine/src/adapters/ffi/bridge/model.hpp"
#include "engine/src/adapters/ffi/bridge.rs.h"

#include "libslic3r/Model.hpp"
#include "libslic3r/Utils.hpp"
#include "libslic3r/PrintConfig.hpp"
#include "libslic3r/TriangleMesh.hpp"
// après Model.hpp : STEP.hpp référence Model sans le déclarer
#include "libslic3r/Format/STEP.hpp"

#include <boost/algorithm/string/predicate.hpp>

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
        for (const Slic3r::ModelVolume *volume : object->volumes) {
            RawVolume raw_volume;
            raw_volume.name = volume->name;
            raw_volume.matrix = to_col_major(volume->get_matrix());
            raw_volume.role = static_cast<uint8_t>(volume->type());
            const int extruder = volume->extruder_id();
            raw_volume.extruder = extruder > 0 ? static_cast<uint32_t>(extruder) : 0;
            raw_volume.mesh = to_raw_mesh(volume->mesh());
            raw_object.volumes.push_back(std::move(raw_volume));
        }
        for (const Slic3r::ModelInstance *instance : object->instances) {
            RawInstance raw_instance;
            raw_instance.matrix = to_col_major(instance->get_matrix());
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

} // namespace webslicer
