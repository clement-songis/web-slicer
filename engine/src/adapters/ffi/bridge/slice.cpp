// Wrapper de tranchage (T019, FR-014/R1) : pilote Slic3r::Print de bout en
// bout (apply → validate → process → export_gcode) dans le process
// engine-worker. La progression du statusbar libslic3r est émise directement
// sur la sortie standard (`P <ratio> <phase>`), lue par le pilote parent
// (engine::adapters::ffi::worker).
#include "engine/src/adapters/ffi/bridge/model.hpp"
#include "engine/src/adapters/ffi/bridge.rs.h"

#include "libslic3r/Model.hpp"
#include "libslic3r/Print.hpp"
#include "libslic3r/PrintConfig.hpp"
#include "libslic3r/GCode/GCodeProcessor.hpp"

#include <nlohmann/json.hpp>

#include <algorithm>
#include <iostream>
#include <stdexcept>
#include <string>

namespace webslicer {

// défini dans project.cpp / model.cpp
void raw_into_model(const rust::Vec<RawObject> &objects, Slic3r::Model &model);

namespace {

// Déserialise la config Orca (clé → valeur sérialisée) dans un DynamicPrintConfig.
void load_config(Slic3r::DynamicPrintConfig &config, const std::string &json_text)
{
    const auto j = nlohmann::json::parse(json_text);
    Slic3r::ConfigSubstitutionContext ctx(
        Slic3r::ForwardCompatibilitySubstitutionRule::Enable);
    for (auto it = j.begin(); it != j.end(); ++it) {
        try {
            config.set_deserialize(it.key(), it.value().get<std::string>(), ctx);
        } catch (const std::exception &) {
            // clé hors registre runtime : ignorée (comme handle_legacy).
        }
    }
}

// Nombre de filaments = taille du vecteur filament_diameter (défaut 1).
int filament_count(const Slic3r::DynamicPrintConfig &config)
{
    if (auto *opt = config.option<Slic3r::ConfigOptionFloats>("filament_diameter"))
        return std::max<int>(1, static_cast<int>(opt->values.size()));
    return 1;
}

} // namespace

RawSliceResult slice_raw(const rust::Vec<RawObject> &objects, rust::Str config_json,
                         rust::Str work_dir)
{
    Slic3r::Model model;
    raw_into_model(objects, model);
    if (model.objects.empty())
        throw std::runtime_error("aucun objet à trancher");

    Slic3r::DynamicPrintConfig config;
    load_config(config, std::string(config_json));

    Slic3r::Print print;
    print.apply(model, config);

    // Paramètres par extrudeur/vitesse renseignés côté Model avant le slicing
    // (comme le CLI : sans quoi certains calculs de vitesse plantent).
    const int filaments = filament_count(config);
    Slic3r::Model::setExtruderParams(config, filaments);
    Slic3r::Model::setPrintSpeedTable(config, print.config());

    print.set_status_callback([](const Slic3r::PrintBase::SlicingStatus &s) {
        std::cout << "P " << (static_cast<double>(s.percent) / 100.0) << " "
                  << s.text << "\n";
        std::cout.flush();
    });

    if (print.empty())
        throw std::runtime_error(
            "rien à trancher : plateau vide ou objets hors du volume d'impression");

    Slic3r::StringObjectException err = print.validate();
    if (!err.string.empty() && !err.is_warning)
        throw std::runtime_error("configuration invalide : " + err.string);

    print.process();

    Slic3r::GCodeProcessorResult result;
    const std::string out = std::string(work_dir) + "/plate_1.gcode";
    const std::string exported = print.export_gcode(out, &result, nullptr);

    RawSliceResult raw;
    raw.gcode_path = exported.empty() ? out : exported;

    using Mode = Slic3r::PrintEstimatedStatistics::ETimeMode;
    raw.estimated_time_s =
        result.print_statistics.modes[static_cast<size_t>(Mode::Normal)].time;

    const Slic3r::PrintStatistics &stats = print.print_statistics();
    for (const auto &kv : stats.filament_stats)
        raw.filament_mm.push_back(kv.second);
    raw.filament_g = stats.total_weight;
    raw.tool_changes = static_cast<uint32_t>(std::max(0, stats.total_toolchanges));

    unsigned int max_layer = 0;
    for (const auto &mv : result.moves)
        max_layer = std::max(max_layer, mv.layer_id);
    raw.layer_count = max_layer; // couche 0 = première ; +0 (ids déjà 1-based sur extrusions)

    return raw;
}

} // namespace webslicer
