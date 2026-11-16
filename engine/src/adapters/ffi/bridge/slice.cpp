// Wrapper de tranchage (T019, FR-014/R1) : pilote Slic3r::Print de bout en
// bout (apply → validate → process → export_gcode) dans le process
// engine-worker. La progression du statusbar libslic3r est émise directement
// sur la sortie standard (`P <ratio> <phase>`), lue par le pilote parent
// (engine::adapters::ffi::worker).
#include "engine/src/adapters/ffi/bridge/model.hpp"
#include "engine/src/adapters/ffi/bridge.rs.h"

#include "libslic3r/Model.hpp"
#include "libslic3r/Print.hpp"
#include "libslic3r/calib.hpp"
#include "libslic3r/FlushVolCalc.hpp"
#include "libslic3r/PrintConfig.hpp"
#include "libslic3r/GCode/GCodeProcessor.hpp"

#include <nlohmann/json.hpp>

#include <algorithm>
#include <cmath>
#include <fstream>
#include <iostream>
#include <map>
#include <sstream>
#include <stdexcept>
#include <string>
#include <vector>

// slic3r/Utils/ColorSpaceConvert.cpp (Orca 2.4.1, à l'identique) : appelé par
// FlushVolCalculator::calc_flush_vol_rgb (libslic3r), mais ce fichier dépend
// de wxWidgets et n'est pas dans libslic3r-headless — seule cette fonction
// est requise par le calcul de purge du CLI.
void RGB2HSV(float r, float g, float b, float *h, float *s, float *v)
{
    float Cmax = std::max(std::max(r, g), b);
    float Cmin = std::min(std::min(r, g), b);
    float delta = Cmax - Cmin;

    if (std::abs(delta) < 0.001) {
        *h = 0.f;
    }
    else if (Cmax == r) {
        *h = 60.f * std::fmod((g - b) / delta, 6.f);
    }
    else if (Cmax == g) {
        *h = 60.f * ((b - r) / delta + 2);
    }
    else {
        *h = 60.f * ((r - g) / delta + 4);
    }

    if (std::abs(Cmax) < 0.001) {
        *s = 0.f;
    }
    else {
        *s = delta / Cmax;
    }

    *v = Cmax;
}

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

// Le CLI enrichit les métadonnées de projet après avoir chargé les presets.
// Les reproduire ici évite que la normalisation interne de Print efface les
// vecteurs machine d'origine dans le bloc de configuration exporté.
void restore_export_metadata(const std::string &path,
                             const Slic3r::DynamicPrintConfig &source,
                             const std::string &raw_json)
{
    std::ifstream input(path);
    if (!input)
        throw std::runtime_error("lecture du G-code exporté impossible");
    std::vector<std::string> lines;
    std::string line;
    while (std::getline(input, line))
        lines.push_back(line);
    input.close();

    const auto begin = std::find(lines.begin(), lines.end(), "; CONFIG_BLOCK_START");
    const auto end = std::find(lines.begin(), lines.end(), "; CONFIG_BLOCK_END");
    if (begin == lines.end() || end == lines.end() || begin >= end)
        return;

    struct Entry { std::string key; std::string text; };
    std::vector<Entry> entries;
    std::vector<std::string> special;
    for (auto it = begin + 1; it != end; ++it) {
        if (it->rfind("; first_layer_bed_temperature", 0) == 0 ||
            it->rfind("; bed_shape", 0) == 0 ||
            it->rfind("; first_layer_temperature", 0) == 0 ||
            it->rfind("; first_layer_height", 0) == 0) {
            special.push_back(*it);
            continue;
        }
        const auto separator = it->find(" = ");
        if (it->rfind("; ", 0) == 0 && separator != std::string::npos)
            entries.push_back({it->substr(2, separator - 2), *it});
    }

    auto remove = [&entries](const std::string &key) {
        entries.erase(std::remove_if(entries.begin(), entries.end(),
                                     [&key](const Entry &entry) { return entry.key == key; }),
                      entries.end());
    };
    auto set = [&entries, &remove](const std::string &key, const std::string &value) {
        remove(key);
        entries.push_back({key, "; " + key + " = " + value});
    };
    auto has = [&source](const std::string &key) { return source.has(key); };
    auto value = [&source](const std::string &key) { return source.opt_serialize(key); };

    // Valeurs machine saisies par l'utilisateur, avant la normalisation interne.
    const auto raw = nlohmann::json::parse(raw_json);
    for (const char *key : {
             "machine_max_acceleration_e", "machine_max_acceleration_extruding",
             "machine_max_acceleration_retracting", "machine_max_acceleration_travel",
             "machine_max_acceleration_x", "machine_max_acceleration_y",
             "machine_max_acceleration_z", "machine_max_jerk_e", "machine_max_jerk_x",
             "machine_max_jerk_y", "machine_max_jerk_z", "machine_max_speed_e",
             "machine_max_speed_x", "machine_max_speed_y", "machine_max_speed_z",
             "machine_min_extruding_rate", "machine_min_travel_rate"})
        if (raw.contains(key) && raw[key].is_string())
            set(key, raw[key].get<std::string>());

    // Les enums de projet ne sont jamais entre guillemets dans le G-code.
    for (const auto &[key, fallback] : std::vector<std::pair<std::string, std::string>>{
             {"extruder_type", "Direct Drive"}, {"nozzle_type", "undefine"},
             {"nozzle_volume_type", "Standard"}, {"overhang_fan_threshold", "50%"},
             {"retract_lift_enforce", "All Surfaces"}, {"z_hop_types", "Normal Lift"}}) {
        const std::string current = has(key) ? value(key) : std::string();
        set(key, current.empty() ? fallback : current);
    }

    const int filaments = filament_count(source);
    // Orca (OrcaSlicer.cpp, chargement CLI) : les presets système chargés sans
    // clé modifiée ont des diffs vides. Un vecteur fourni garde sa taille mais
    // ses entrées sont vidées (référence : ["x"] → "") ; sinon process +
    // filaments + machine, tous vides.
    std::size_t different_count = static_cast<std::size_t>(filaments + 2);
    if (const auto *given = source.option<Slic3r::ConfigOptionStrings>("different_settings_to_system"))
        if (!given->values.empty())
            different_count = given->values.size();
    set("different_settings_to_system",
        Slic3r::ConfigOptionStrings(std::vector<std::string>(different_count, "")).serialize());
    // `filament_extruder_variant` est recalculé par Orca : seule une variante
    // réelle (« <type d'extrudeur> <volume de buse> ») est conservée, toute
    // autre valeur retombe sur Direct Drive Standard (défaut du CLI).
    if (const auto *variants = source.option<Slic3r::ConfigOptionStrings>("filament_extruder_variant")) {
        std::vector<std::string> known;
        for (int e = 0; e <= Slic3r::etMaxExtruderType; ++e)
            for (int n = 0; n <= Slic3r::nvtMaxNozzleVolumeType; ++n)
                known.push_back(Slic3r::get_extruder_variant_string(
                    static_cast<Slic3r::ExtruderType>(e), static_cast<Slic3r::NozzleVolumeType>(n)));
        const std::string fallback =
            Slic3r::get_extruder_variant_string(Slic3r::etDirectDrive, Slic3r::nvtStandard);
        std::vector<std::string> values = variants->values;
        if (values.empty())
            values.assign(filaments, fallback);
        for (auto &variant : values)
            if (std::find(known.begin(), known.end(), variant) == known.end())
                variant = fallback;
        set("filament_extruder_variant", Slic3r::ConfigOptionStrings(values).serialize());
    }
    set("filament_ids",
        Slic3r::ConfigOptionStrings(std::vector<std::string>(filaments, "")).serialize());
    std::vector<int> self_indices;
    for (int index = 0; index < filaments; ++index)
        self_indices.push_back(index + 1);
    set("filament_self_index", Slic3r::ConfigOptionInts(self_indices).serialize());

    // inherits_group[0] = `inherits` du preset d'impression, comme Orca
    // (CLI : OrcaSlicer.cpp:2985 `new_process_system_name` = inherits d'un preset
    // utilisateur ; GUI : PresetBundle.cpp:236 `preset.inherits()`) — pas son
    // nom (`print_settings_id`), qui n'est qu'un repli quand `inherits` manque.
    std::vector<std::string> inheritance;
    if (has("inherits"))
        inheritance.push_back(value("inherits"));
    else if (has("print_settings_id"))
        inheritance.push_back(value("print_settings_id"));
    if (const auto *filaments_value = source.option<Slic3r::ConfigOptionStrings>("filament_settings_id"))
        inheritance.insert(inheritance.end(), filaments_value->values.begin(), filaments_value->values.end());
    if (has("printer_settings_id"))
        inheritance.push_back(value("printer_settings_id"));
    // Un `inherits_group` fourni est conservé tel quel par Orca (CLI make_refs
    // brut : « ; inherits_group = harness_test », staging/params-review) ;
    // sinon il est reconstruit à partir des presets (process, filaments, machine).
    if (const auto *given = source.option<Slic3r::ConfigOptionStrings>("inherits_group");
        given && !given->values.empty())
        set("inherits_group", given->serialize());
    else if (!inheritance.empty())
        set("inherits_group", Slic3r::ConfigOptionStrings(inheritance).serialize());
    if (!has("print_compatible_printers"))
        if (const auto *compatible = source.option<Slic3r::ConfigOptionStrings>("compatible_printers"))
            set("print_compatible_printers", compatible->serialize());
    remove("inherits");

    std::stable_sort(entries.begin(), entries.end(), [](const Entry &a, const Entry &b) {
        return a.key < b.key;
    });
    std::vector<std::string> output;
    output.reserve(lines.size() + 12);
    output.insert(output.end(), lines.begin(), begin + 1);
    for (const auto &entry : entries)
        output.push_back(entry.text);
    output.insert(output.end(), special.begin(), special.end());
    output.insert(output.end(), end, lines.end());

    std::ofstream out(path, std::ios::trunc);
    if (!out)
        throw std::runtime_error("réécriture du G-code exporté impossible");
    for (const auto &text : output)
        out << text << "\n";
}

// Miroir de la déduction du CLI (OrcaSlicer.cpp, `is_bbl_vendor_preset`) :
// 1. `printer_model` non vide → préfixe « Bambu Lab » ;
// 2. sinon le nom du preset machine : celui chargé (`new_printer_name`, ici
//    `printer_settings_id` posé par la résolution backend/harness) ou, pour un
//    3MF, le nom système courant (`current_printer_system_name`) = dernier
//    élément non vide de `inherits_group`, à défaut `printer_settings_id` ;
// 3. aucun nom connu (3MF sans preset) → faux, défaut du CLI.
bool is_bbl_vendor_preset(const Slic3r::DynamicPrintConfig &config);

// Plater.cpp `has_junction_deviation` : Marlin 2 avec junction deviation max > 0.
bool has_junction_deviation(const Slic3r::DynamicPrintConfig &view)
{
    const auto *flavor = view.option<Slic3r::ConfigOptionEnum<Slic3r::GCodeFlavor>>("gcode_flavor");
    const auto *jd = view.option<Slic3r::ConfigOptionFloats>("machine_max_junction_deviation");
    return flavor && flavor->value == Slic3r::gcfMarlinFirmware && jd && !jd->values.empty() &&
           jd->values.front() > 0.0;
}

// Plater::_calib_pa_pattern : réglages imposés aux profils (sans listes de
// vitesses/accélérations : un seul motif, vitesse optimale d'Orca).
void apply_pa_pattern_config(Slic3r::DynamicPrintConfig &config)
{
    using namespace Slic3r;
    const DynamicPrintConfig &view = config;
    const auto read = [&](const char *key) { return view.option<ConfigOptionFloat>(key)->value; };
    const double nozzle = view.option<ConfigOptionFloats>("nozzle_diameter")->get_at(0);
    config.set_key_value("filament_retract_when_changing_layer", new ConfigOptionBoolsNullable{false});
    config.set_key_value("filament_wipe", new ConfigOptionBoolsNullable{false});
    config.set_key_value("wipe", new ConfigOptionBools{false});
    config.set_key_value("retract_when_changing_layer", new ConfigOptionBools{false});
    config.set_key_value("resonance_avoidance", new ConfigOptionBool(false));

    double accel = read("outer_wall_acceleration");
    if (accel == 0) accel = read("inner_wall_acceleration");
    if (accel == 0) accel = read("default_acceleration");
    config.set_key_value("outer_wall_acceleration", new ConfigOptionFloat(accel));
    config.set_key_value("print_sequence", new ConfigOptionEnum<PrintSequence>(PrintSequence::ByLayer));

    const bool jd = has_junction_deviation(view);
    if (!jd && read("default_jerk") > 0) {
        double jerk = read("outer_wall_jerk");
        if (jerk == 0) jerk = read("inner_wall_jerk");
        if (jerk == 0) jerk = read("default_jerk");
        for (const char *key : {"default_jerk", "outer_wall_jerk", "inner_wall_jerk",
                                "top_surface_jerk", "infill_jerk", "travel_jerk"})
            config.set_key_value(key, new ConfigOptionFloat(jerk));
    }
    if (jd)
        config.set_key_value("default_junction_deviation", new ConfigOptionFloat(0));

    const SuggestedConfigCalibPAPattern suggested;
    for (const auto &opt : suggested.float_pairs)
        config.set_key_value(opt.first, new ConfigOptionFloat(opt.second));
    for (const auto &opt : suggested.nozzle_ratio_pairs)
        config.set_key_value(opt.first, new ConfigOptionFloatOrPercent(nozzle * opt.second / 100, false));
    for (const auto &opt : suggested.int_pairs)
        config.set_key_value(opt.first, new ConfigOptionInt(opt.second));
    config.set_key_value(suggested.brim_pair.first, new ConfigOptionEnum<BrimType>(suggested.brim_pair.second));
    config.set_key_value("enable_wrapping_detection", new ConfigOptionBool(false));

    const double speed = CalibPressureAdvance::find_optimal_PA_speed(
        view, view.get_abs_value("line_width", nozzle), view.get_abs_value("layer_height"), 0, 0);
    config.set_key_value("outer_wall_speed", new ConfigOptionFloat(speed));
}

// Plater::_calib_pa_pattern + _calib_pa_pattern_gen_gcode : le premier objet
// devient la « poignée » (cube redimensionné, placé à gauche du motif autour
// de sa position XY actuelle) et le motif est injecté en G-code personnalisé
// par couche (plates_custom_gcodes du plateau courant).
void prepare_pa_pattern(Slic3r::Model &model, const Slic3r::DynamicPrintConfig &config,
                        const Slic3r::Calib_Params &params, bool is_bbl)
{
    using namespace Slic3r;
    while (model.objects.size() > 1)
        model.delete_object(model.objects.size() - 1);
    ModelObject *handle = model.objects.front();
    if (handle->instances.empty())
        throw std::runtime_error("motif PA : objet poignée sans instance");
    const Vec3d anchor = handle->instance_bounding_box(0).center();
    handle->center_around_origin();

    CalibPressureAdvancePattern pattern(params, config, is_bbl, *handle, Vec3d::Zero());
    const Vec3d size = handle->raw_bounding_box().size();
    if (size.x() <= 0 || size.y() <= 0 || size.z() <= 0)
        throw std::runtime_error("motif PA : objet poignée vide");
    handle->scale(Vec3d(pattern.handle_xy_size() / size.x(), pattern.handle_xy_size() / size.y(),
                        pattern.max_layer_z() / size.z()));
    handle->instances[0]->set_offset(Vec3d(anchor.x(), anchor.y(), 0) + pattern.handle_pos_offset());
    handle->ensure_on_bed();
    handle->name = "pa_pattern";

    model.plates_custom_gcodes[model.curr_plate_index] =
        pattern.generate_custom_gcodes(config, is_bbl, *handle, Vec3d::Zero());
}

// BitmapCache::parse_color4 (GUI) : "#RRGGBB" ou "#RRGGBBAA", alpha 255 par
// défaut ; toute autre forme donne noir opaque.
void parse_color4(const std::string &text, unsigned char *rgba)
{
    rgba[0] = rgba[1] = rgba[2] = 0;
    rgba[3] = 255;
    if ((text.size() != 7 && text.size() != 9) || text.front() != '#')
        return;
    const auto digit = [](char c) -> int {
        if (c >= '0' && c <= '9') return c - '0';
        if (c >= 'a' && c <= 'f') return c - 'a' + 10;
        if (c >= 'A' && c <= 'F') return c - 'A' + 10;
        return -1;
    };
    for (size_t i = 0; i < text.size() / 2; ++i) {
        const int high = digit(text[1 + 2 * i]);
        const int low = digit(text[2 + 2 * i]);
        if (high == -1 || low == -1)
            return;
        rgba[i] = static_cast<unsigned char>(high * 16 + low);
    }
}

// OrcaSlicer.cpp (CLI, « compute the flush volume », ~l.3340-3500) : quand les
// presets chargés fournissent `filament_colour` sans `flush_volumes_matrix`,
// le CLI recalcule la matrice de purge à partir des couleurs (n × n par buse,
// diagonale nulle). `loaded` = clés réellement fournies (presets), `config`
// = config complète. Les conditions « nombre d'extrudeurs / type de volume
// de buse différents de ceux du 3MF » ne s'appliquent pas : la requête
// n'a pas de 3MF d'origine (et le projet UI conserve sa matrice).
void cli_update_flush_volumes(Slic3r::DynamicPrintConfig &config,
                              const Slic3r::DynamicPrintConfig &loaded)
{
    using namespace Slic3r;
    const DynamicPrintConfig &view = config;
    const auto *colours = loaded.option<ConfigOptionStrings>("filament_colour");
    if (!colours || colours->values.empty() || loaded.has("flush_volumes_matrix"))
        return;
    const std::vector<std::string> project_colours = colours->values;
    const size_t filaments = project_colours.size();
    size_t extruders = 1;
    if (const auto *nozzles = view.option<ConfigOptionFloats>("nozzle_diameter"))
        extruders = std::max<size_t>(1, nozzles->values.size());

    // get_min_flush_volumes(m_print_config, 0) : le CLI lit
    // `filament_long_retractions_when_cut` comme ConfigOptionBools alors que
    // l'option est nullable → jamais trouvée, longueur de rétraction 0 : le
    // minimum vaut `nozzle_volume[0]` des presets (0 à défaut).
    int min_flush = 0;
    if (const auto *volume = loaded.option<ConfigOptionFloatsNullable>("nozzle_volume");
        volume && !volume->values.empty())
        min_flush = static_cast<int>(volume->get_at(0));

    std::vector<int> dataset(extruders, 0);
    {
        std::vector<int> full;
        if (const auto *opt = view.option<ConfigOptionIntsNullable>("nozzle_flush_dataset"))
            full = opt->values;
        size_t variants = 1;
        if (const auto *opt = loaded.option<ConfigOptionStrings>("printer_extruder_variant"))
            variants = opt->values.size();
        full.resize(loaded.has("printer_extruder_variant") ? variants : 1, 0);
        std::vector<int> types(1, int(etDirectDrive)), volumes(1, int(nvtStandard));
        if (const auto *opt = dynamic_cast<const ConfigOptionEnumsGeneric *>(loaded.option("extruder_type")))
            types = opt->values;
        if (const auto *opt = dynamic_cast<const ConfigOptionEnumsGeneric *>(loaded.option("nozzle_volume_type")))
            volumes = opt->values;
        for (size_t e = 0; e < extruders; ++e) {
            int index = 0;
            if (loaded.has("printer_extruder_id") && loaded.has("printer_extruder_variant"))
                index = loaded.get_index_for_extruder(
                    int(e) + 1, "printer_extruder_id",
                    ExtruderType(types.empty() ? 0 : types[std::min(e, types.size() - 1)]),
                    NozzleVolumeType(volumes.empty() ? 0 : volumes[std::min(e, volumes.size() - 1)]),
                    "printer_extruder_variant");
            if (index >= 0 && size_t(index) < full.size())
                dataset[e] = full[index];
        }
    }

    std::vector<double> matrix;
    if (const auto *opt = view.option<ConfigOptionFloats>("flush_volumes_matrix"))
        matrix = opt->values;
    matrix.resize(filaments * filaments * extruders, 0.);
    std::vector<double> multipliers;
    if (const auto *opt = view.option<ConfigOptionFloats>("flush_multiplier"))
        multipliers = opt->values;
    multipliers.resize(extruders, 1.);

    std::vector<unsigned char> support(filaments, 0);
    if (const auto *opt = view.option<ConfigOptionBools>("filament_is_support"))
        for (size_t i = 0; i < filaments && !opt->values.empty(); ++i)
            support[i] = opt->get_at(i);

    for (size_t nozzle = 0; nozzle < extruders; ++nozzle) {
        std::vector<double> sub = get_flush_volumes_matrix(matrix, nozzle, extruders);
        sub.resize(filaments * filaments, 0.);
        for (size_t from = 0; from < filaments; ++from) {
            unsigned char from_rgb[4];
            parse_color4(project_colours[from], from_rgb);
            for (size_t to = 0; to < filaments; ++to) {
                if (from == to) {
                    sub[filaments * from + to] = 0.;
                    continue;
                }
                int volume = 0;
                if (support[to]) {
                    volume = g_flush_volume_to_support;
                } else {
                    unsigned char to_rgb[4];
                    parse_color4(project_colours[to], to_rgb);
                    FlushVolCalculator calculator(min_flush, g_max_flush_volume, dataset[nozzle]);
                    volume = calculator.calc_flush_vol(from_rgb[3], from_rgb[0], from_rgb[1], from_rgb[2],
                                                       to_rgb[3], to_rgb[0], to_rgb[1], to_rgb[2]);
                    if (support[from])
                        volume = std::max(g_min_flush_volume_from_support, volume);
                }
                sub[filaments * from + to] = volume;
            }
        }
        set_flush_volumes_matrix(matrix, sub, nozzle, extruders);
    }
    config.set_key_value("flush_volumes_matrix", new ConfigOptionFloats(matrix));
    config.set_key_value("flush_multiplier", new ConfigOptionFloats(multipliers));
}

bool is_bbl_vendor_preset(const Slic3r::DynamicPrintConfig &config)
{
    const auto starts_with_bambu = [](const std::string &name) {
        return name.compare(0, 9, "Bambu Lab") == 0;
    };
    if (const auto *model = config.option<Slic3r::ConfigOptionString>("printer_model");
        model && !model->value.empty())
        return starts_with_bambu(model->value);
    std::string name;
    if (const auto *group = config.option<Slic3r::ConfigOptionStrings>("inherits_group");
        group && !group->values.empty() && !group->values.back().empty())
        name = group->values.back();
    else if (const auto *id = config.option<Slic3r::ConfigOptionString>("printer_settings_id"))
        name = id->value;
    return !name.empty() && starts_with_bambu(name);
}

} // namespace

RawSliceResult slice_raw(const rust::Vec<RawObject> &objects, rust::Str config_json,
                         rust::Str work_dir, const RawCalibParams &calib,
                         rust::Str custom_gcodes_json)
{
    Slic3r::Model model;
    raw_into_model(objects, model);
    // Changements de couleur / pauses / G-code perso du plateau tranché
    // (Model::get_curr_plate_custom_gcodes, lu par Print::apply).
    custom_gcodes_from_json(model, std::string(custom_gcodes_json), -1, model.curr_plate_index);
    if (model.objects.empty())
        throw std::runtime_error("aucun objet à trancher");

    // Config = tous les défauts du registre + surcharges de la requête
    // (contrat trait_suite : « complétée par les défauts côté implémentation »),
    // puis normalisation FDM (vecteurs par extrudeur, cohérence).
    // Surcharges de la requête désérialisées sur une config VIDE (voie sûre :
    // `set_deserialize` sur une option existante peut planter dans libslic3r),
    // puis appliquées par-dessus tous les défauts du registre (comme le
    // superpose OrcaSlicer pour ses presets). Résultat : config complète et
    // cohérente, que la requête soit minimale (cube) ou déjà complète (projet).
    Slic3r::DynamicPrintConfig overrides;
    load_config(overrides, std::string(config_json));
    // Contrairement à `full_print_config()` (qui ne contient que la région),
    // la base doit inclure machine, filament et process, comme dans le CLI.
    Slic3r::DynamicPrintConfig config;
    config.apply(Slic3r::FullPrintConfig::defaults(), /*ignore_nonexistent=*/true);
    config.apply(overrides, /*ignore_nonexistent=*/true);
    // Comme le CLI (OrcaSlicer.cpp : `m_print_config.normalize_fdm()` après
    // application des options) : spiral_mode → wall_loops=1, top_shell_layers=0,
    // sparse_infill_density=0, pas de rétraction au changement de couche ;
    // propagation `extruder` → *_filament_id ; plancher de `resolution`.
    // Argument par défaut (0) comme le CLI : la logique prime tower dépendant
    // des filaments réellement utilisés est faite par Print::apply
    // (normalize_fdm_2).
    config.normalize_fdm();
    cli_update_flush_volumes(config, overrides);
    // OrcaSlicer.cpp ~l.3619 : le CLI impose `printer_technology` déduit des
    // fichiers chargés, or `get_printer_technology` lit l'option comme
    // ConfigOptionEnum<PrinterTechnology> alors que load_from_json crée un
    // ConfigOptionEnumGeneric → toujours ptUnknown → ptFFF (seule technologie
    // tranchée par le CLI). Une valeur « SLA » fournie n'atteint donc jamais
    // le G-code. Option générique comme celles créées par le registre :
    // DynamicPrintConfig::validate la relit en ConfigOptionEnumGeneric.
    config.set_key_value("printer_technology",
                         new Slic3r::ConfigOptionEnumGeneric(
                             &Slic3r::ConfigOptionEnum<Slic3r::PrinterTechnology>::get_enum_values(),
                             Slic3r::ptFFF));
    if (calib.mode == static_cast<int>(Slic3r::CalibMode::Calib_PA_Pattern))
        apply_pa_pattern_config(config);

    // Comme le CLI juste avant `print->apply` : `filament_map` couvre tous les
    // filaments (complété par 1) et, en mono-extrudeur, tout pointe sur 1.
    // (Lectures via la vue const : l'accesseur non-const template de
    // DynamicConfig journalise via Boost.Log, non lié dans ce binaire.)
    {
        const Slic3r::DynamicPrintConfig &view = config;
        const int filaments = filament_count(view);
        int extruders = 1;
        if (const auto *nozzles = view.option<Slic3r::ConfigOptionFloats>("nozzle_diameter"))
            extruders = std::max<int>(1, static_cast<int>(nozzles->values.size()));
        std::vector<int> maps;
        if (const auto *current = view.option<Slic3r::ConfigOptionInts>("filament_map"))
            maps = current->values;
        if (static_cast<int>(maps.size()) < filaments)
            maps.resize(filaments, 1);
        if (extruders == 1)
            for (int index = 0; index < filaments; ++index)
                maps[index] = 1;
        config.set_key_value("filament_map", new Slic3r::ConfigOptionInts(maps));
    }

    // Comme le CLI Orca (OrcaSlicer.cpp ~l.3645, CLI_INVALID_VALUES_IN_3MF) :
    // bornes min/max du registre et incompatibilités (firmware retraction
    // + wipe…) via DynamicPrintConfig::validate, avant Print::validate qui ne
    // les couvre pas (ex. default_junction_deviation=0.5 > max 0.3).
    {
        const std::map<std::string, std::string> validity = config.validate(/*under_cli=*/true);
        if (!validity.empty()) {
            std::string message = "configuration invalide : valeur(s) de paramètre invalide(s) — ";
            bool first = true;
            for (const auto &[key, reason] : validity) {
                message += (first ? "" : " ; ") + key + " : " + reason;
                first = false;
            }
            throw std::runtime_error(message);
        }
    }

    Slic3r::Print print;
    // Orca 2.4.1 laisse `m_isBBLPrinter` non initialisé : on reproduit
    // exactement `is_bbl_vendor_preset` du CLI (OrcaSlicer.cpp, juste avant
    // `Print::validate`), sans quoi un profil Voron pouvait prendre la branche
    // Bambu (ou l'inverse) selon la mémoire.
    print.is_BBL_printer() = is_bbl_vendor_preset(config);
    // Mode calibration (Plater::calib_retraction / calib_VFA / calib_max_vol_speed…) :
    // la GUI pose `set_calib_params` sur le Print avant le tranchage ; GCode.cpp
    // l'exploite à chaque changement de couche (« Calibration Layer-specific GCode »).
    if (calib.mode != 0) {
        if (calib.mode < static_cast<int>(Slic3r::CalibMode::Calib_None) ||
            calib.mode > static_cast<int>(Slic3r::CalibMode::Calib_Cornering))
            throw std::runtime_error("mode de calibration inconnu");
        Slic3r::Calib_Params params;
        params.mode = static_cast<Slic3r::CalibMode>(calib.mode);
        params.start = calib.start;
        params.end = calib.end;
        params.step = calib.step;
        params.print_numbers = calib.print_numbers;
        params.freqStartX = calib.freq_start_x;
        params.freqEndX = calib.freq_end_x;
        params.freqStartY = calib.freq_start_y;
        params.freqEndY = calib.freq_end_y;
        params.test_model = 0;
        params.shaper_type = "";
        if (params.mode == Slic3r::CalibMode::Calib_PA_Pattern)
            prepare_pa_pattern(model, config, params, print.is_BBL_printer());
        print.set_calib_params(params);
    }
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
    restore_export_metadata(exported.empty() ? out : exported, overrides, std::string(config_json));

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

    // Nom d'export d'Orca (Plater::export_gcode → output_filepath_for_project →
    // Print::output_filename) : filename_format évalué par le PlaceholderParser
    // avec les statistiques du tranchage (print_time, used_filament…). La base
    // (nom du projet) est un jeton remplacé côté serveur.
    try {
        raw.output_filename = print.output_filename("__ws_filename_base__");
    } catch (const std::exception &) {
        raw.output_filename = rust::String();
    }

    return raw;
}

} // namespace webslicer
