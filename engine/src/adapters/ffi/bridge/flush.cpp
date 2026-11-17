// Volumes de purge calculés depuis les couleurs des filaments, comme
// Sidebar::auto_calc_flushing_volumes d'Orca (Plater.cpp ~4117) et le CLI
// (OrcaSlicer.cpp ~3400) : FlushVolCalculator(min_flush[from],
// g_max_flush_volume, nozzle_flush_dataset).calc_flush_vol(ARGB from, ARGB to),
// vers un filament support → g_flush_volume_to_support, depuis un support →
// au moins g_min_flush_volume_from_support, diagonale nulle. Matrice N×N
// aplatie (ligne = depuis), SANS le multiplicateur (flush_multiplier est
// appliqué au tranchage, comme la matrice stockée par Orca).
// PrintConfig.hpp tire BOOST_LOG_TRIVIAL : Boost.Log est lié dynamiquement.
#ifndef BOOST_LOG_DYN_LINK
#define BOOST_LOG_DYN_LINK 1
#endif
#include "engine/src/adapters/ffi/bridge/flush.hpp"

#include "libslic3r/FlushVolCalc.hpp"
#include "libslic3r/PrintConfig.hpp"

#include <cmath>

#include <stdexcept>
#include <string>

namespace webslicer {
namespace {

// wxColour(« #RRGGBB » | « #RRGGBBAA ») : alpha 255 par défaut.
void parse_colour(const std::string &text, unsigned char argb[4])
{
    std::string hex = text;
    if (!hex.empty() && hex[0] == '#')
        hex.erase(0, 1);
    if (hex.size() != 6 && hex.size() != 8)
        throw std::invalid_argument("couleur invalide : « " + text + " » (#RRGGBB attendu)");
    auto byte = [&](size_t at) { return static_cast<unsigned char>(std::stoi(hex.substr(at, 2), nullptr, 16)); };
    argb[1] = byte(0);
    argb[2] = byte(2);
    argb[3] = byte(4);
    argb[0] = hex.size() == 8 ? byte(6) : 255;
}

} // namespace

void config_from_json_print(Slic3r::DynamicPrintConfig &config, const std::string &json_text);

// Minimum de purge par filament source : portage de get_min_flush_volumes
// (GUI, Plater.cpp ~764 — absent de libslic3r) : nozzle_volume[buse] moins le
// volume de filament rétracté à la coupe (π·1,75²/4 × longueur) quand la
// rétraction longue est activée (machine + filament), troncature entière
// comme Orca. Mêmes types d'options que la GUI (option<ConfigOptionBools>
// sur des options nullables → absentes, comme dans Orca).
rust::Vec<int32_t> min_flush_volumes_raw(rust::Str config_json, uint32_t nozzle_id)
{
    using namespace Slic3r;
    DynamicPrintConfig full_config;
    config_from_json_print(full_config, std::string(config_json));
    const auto *nozzle_volume_opt = full_config.option<ConfigOptionFloatsNullable>("nozzle_volume");
    int nozzle_volume_val = nozzle_volume_opt && nozzle_id < nozzle_volume_opt->values.size()
                                ? (int)nozzle_volume_opt->get_at(nozzle_id)
                                : 0;
    int machine_enabled_level = 0;
    if (const auto *opt = full_config.option<ConfigOptionInt>("enable_long_retraction_when_cut"))
        machine_enabled_level = opt->value;
    bool machine_activated = false;
    if (const auto *opt = full_config.option<ConfigOptionBools>("long_retractions_when_cut"))
        machine_activated = nozzle_id < opt->values.size() && opt->values[nozzle_id] == 1;
    const auto *diameters = full_config.option<ConfigOptionFloats>("filament_diameter");
    const size_t filament_size = diameters ? std::max<size_t>(1, diameters->values.size()) : 1;
    std::vector<double> filament_distance(filament_size, 18.0), printer_distance(filament_size, 18.0);
    std::vector<unsigned char> filament_long(filament_size, 0);
    if (const auto *opt = full_config.option<ConfigOptionFloats>("filament_retraction_distances_when_cut"))
        filament_distance = opt->values;
    if (const auto *opt = full_config.option<ConfigOptionFloats>("retraction_distances_when_cut"))
        printer_distance = opt->values;
    if (const auto *opt = full_config.option<ConfigOptionBools>("filament_long_retractions_when_cut"))
        filament_long = opt->values;
    auto at = [](const auto &v, size_t i, auto fallback) { return i < v.size() ? v[i] : fallback; };
    rust::Vec<int32_t> out;
    for (size_t idx = 0; idx < filament_size; ++idx) {
        int extra = nozzle_volume_val;
        int retract_length = machine_enabled_level && machine_activated ? (int)at(printer_distance, nozzle_id, 18.0) : 0;
        const unsigned char activated = at(filament_long, idx, (unsigned char)0);
        const double filament_retract = at(filament_distance, idx, 18.0);
        if (activated == 0)
            retract_length = 0;
        else if (activated == 1 && machine_enabled_level == LongRectrationLevel::EnableFilament) {
            if (!std::isnan(filament_retract))
                retract_length = (int)filament_retract;
            else
                retract_length = (int)at(printer_distance, nozzle_id, 18.0);
        }
        extra -= PI * 1.75 * 1.75 / 4 * retract_length;
        out.push_back(extra);
    }
    return out;
}

rust::Vec<double> flush_volumes_raw(const rust::Vec<rust::String> &colours, const rust::Vec<bool> &is_support,
                                    const rust::Vec<int32_t> &min_flush, int32_t flush_dataset)
{
    using namespace Slic3r;
    const size_t n = colours.size();
    std::vector<std::array<unsigned char, 4>> argb(n);
    for (size_t i = 0; i < n; ++i)
        parse_colour(std::string(colours[i]), argb[i].data());
    auto support = [&](size_t i) { return i < is_support.size() && is_support[i]; };
    auto minimum = [&](size_t i) { return min_flush.empty() ? 0 : min_flush[std::min(i, min_flush.size() - 1)]; };
    rust::Vec<double> matrix;
    for (size_t from = 0; from < n; ++from)
        for (size_t to = 0; to < n; ++to) {
            int volume = 0;
            if (from != to) {
                if (support(to)) {
                    volume = g_flush_volume_to_support;
                } else {
                    FlushVolCalculator calculator(minimum(from), g_max_flush_volume, flush_dataset);
                    const auto &a = argb[from];
                    const auto &b = argb[to];
                    volume = calculator.calc_flush_vol(a[0], a[1], a[2], a[3], b[0], b[1], b[2], b[3]);
                    if (support(from))
                        volume = std::max(volume, g_min_flush_volume_from_support);
                }
            }
            matrix.push_back(volume);
        }
    return matrix;
}

} // namespace webslicer
