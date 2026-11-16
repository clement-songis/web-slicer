// Simulation cinématique d'Orca (GCodeProcessor.cpp, libslic3r) sur un G-code
// déjà tranché : process_file() relit la configuration du bloc CONFIG du
// G-code (producteur OrcaSlicer) et simule chaque mouvement — temps réel
// (accélérations, jerk, limites machine), modes Normal/Silencieux, vitesse et
// débit réels — comme le fait l'aperçu de la GUI (GCodeViewer::load). finalize
// sans post-traitement : le fichier n'est jamais réécrit.
//
// Sortie en colonnes typées (cf. RawGcodeProcess dans bridge.rs) + résumé JSON
// (temps par mode, temps « prepare », temps par G-code personnalisé, volumes
// par extrudeur / rôle, changements de couleur / pause / G-code personnalisé).
#include "engine/src/adapters/ffi/bridge/gcode_process.hpp"
#include "engine/src/adapters/ffi/bridge.rs.h"

#include "libslic3r/CustomGCode.hpp"
#include "libslic3r/ExtrusionEntity.hpp"
#include "libslic3r/GCode/GCodeProcessor.hpp"

#include <nlohmann/json.hpp>

#include <stdexcept>
#include <string>

namespace webslicer {

namespace {

const char *custom_type_name(Slic3r::CustomGCode::Type t)
{
    using namespace Slic3r::CustomGCode;
    switch (t) {
    case ColorChange: return "color_change";
    case PausePrint: return "pause_print";
    case ToolChange: return "tool_change";
    case Template: return "template";
    case Custom: return "custom";
    default: return "unknown";
    }
}

} // namespace

RawGcodeProcess gcode_process_raw(rust::Str path)
{
    using namespace Slic3r;
    using Mode = PrintEstimatedStatistics::ETimeMode;
    // `s_IsBBLPrinter` est un statique global d'Orca (vrai au démarrage du
    // process, GCodeProcessor.cpp:116) que process_file ne recalcule que si le
    // G-code porte un `printer_model` non vide (:2560) — et que le tranchage
    // (WipeTower/WipeTower2) modifie. Sans remise à la valeur de démarrage, un
    // G-code BBL sans printer_model traité après un G-code Voron dans le même
    // process perdait ses balises BBL (prepare_time = 0 au lieu de 2,06 s).
    GCodeProcessor::s_IsBBLPrinter = true;
    GCodeProcessor processor;
    // Comme GUI_App/Plater : estimation silencieuse toujours calculée (le mode
    // n'est affiché que si la machine le déclare), enveloppe machine appliquée.
    processor.enable_stealth_time_estimator(true);
    processor.enable_machine_envelope_processing(true);
    processor.process_file(std::string(path));
    const GCodeProcessorResult &r = processor.get_result();

    RawGcodeProcess out;
    const size_t n = r.moves.size();
    out.bytes.reserve(n * 4);
    out.floats.reserve(n * 16);
    out.ints.reserve(n * 2);
    for (const auto &m : r.moves) {
        out.bytes.push_back(static_cast<uint8_t>(m.type));
        out.bytes.push_back(static_cast<uint8_t>(m.extrusion_role));
        out.bytes.push_back(m.extruder_id);
        out.bytes.push_back(m.cp_color_id);
        const float f[16] = {m.position.x(), m.position.y(), m.position.z(), m.feedrate, m.actual_feedrate,
                             m.mm3_per_mm, m.width, m.height, m.fan_speed, m.temperature, m.acceleration,
                             m.jerk, m.pressure_advance, m.time[size_t(Mode::Normal)],
                             m.time[size_t(Mode::Stealth)], m.delta_extruder};
        for (float v : f)
            out.floats.push_back(v);
        out.ints.push_back(m.layer_id);
        out.ints.push_back(m.gcode_id);
    }

    nlohmann::json j;
    const auto &ps = r.print_statistics;
    nlohmann::json modes = nlohmann::json::array();
    for (size_t i = 0; i < size_t(Mode::Count); ++i) {
        const auto &md = ps.modes[i];
        nlohmann::json cg = nlohmann::json::array();
        for (const auto &[type, times] : md.custom_gcode_times)
            cg.push_back({{"type", custom_type_name(type)}, {"time", times.first}, {"remaining", times.second}});
        modes.push_back({{"time", md.time}, {"prepare_time", md.prepare_time}, {"custom_gcode_times", cg}});
    }
    j["modes"] = modes;
    auto vol_map = [](const std::map<size_t, double> &m) {
        nlohmann::json o = nlohmann::json::object();
        for (const auto &[k, v] : m)
            o[std::to_string(k)] = v;
        return o;
    };
    j["model_volumes_per_extruder"] = vol_map(ps.model_volumes_per_extruder);
    j["support_volumes_per_extruder"] = vol_map(ps.support_volumes_per_extruder);
    j["wipe_tower_volumes_per_extruder"] = vol_map(ps.wipe_tower_volumes_per_extruder);
    j["total_volumes_per_extruder"] = vol_map(ps.total_volumes_per_extruder);
    j["flush_per_filament"] = vol_map(ps.flush_per_filament);
    nlohmann::json per_role = nlohmann::json::object();
    for (const auto &[role, used] : ps.used_filaments_per_role)
        per_role[std::to_string(int(role))] = {{"meters", used.first}, {"grams", used.second}};
    j["used_filaments_per_role"] = per_role;
    j["volumes_per_color_change"] = ps.volumes_per_color_change;
    j["total_filament_changes"] = ps.total_filament_changes;
    j["total_extruder_changes"] = ps.total_extruder_changes;
    j["total_tool_change_time"] = ps.total_tool_change_time;
    j["total_travel_distance"] = ps.total_travel_distance;
    j["total_travel_moves"] = ps.total_travel_moves;
    j["initial_layer_time"] = r.initial_layer_time;
    j["first_layer_time_normal"] = processor.get_first_layer_time(Mode::Normal);
    j["filament_diameters"] = r.filament_diameters;
    j["filament_densities"] = r.filament_densities;
    j["filament_costs"] = r.filament_costs;
    j["extruder_colors"] = r.extruder_colors;
    j["spiral_vase_mode"] = r.spiral_vase_mode;
    nlohmann::json custom = nlohmann::json::array();
    for (const auto &item : r.custom_gcode_per_print_z)
        custom.push_back({{"print_z", item.print_z}, {"type", custom_type_name(item.type)},
                          {"extruder", item.extruder}, {"color", item.color}, {"extra", item.extra}});
    j["custom_gcode_per_print_z"] = custom;
    out.summary_json = rust::String(j.dump());
    return out;
}

} // namespace webslicer
