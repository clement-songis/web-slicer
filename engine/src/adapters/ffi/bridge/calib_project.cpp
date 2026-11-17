// Projets de calibration d'OrcaSlicer 2.4.1, portage littéral des fonctions de
// l'interface (GUI/Plater.cpp : calib_temp, calib_pa / _calib_pa_tower,
// calib_flowrate + adjust_settings_for_flowrate_calib, calib_max_vol_speed,
// calib_retraction, calib_VFA). Ces fonctions vivent dans la GUI (absentes de
// libslic3r et du CLI) : elles chargent le modèle de resources/calib, le
// coupent/mettent à l'échelle, modifient les presets édités (impression,
// filament, imprimante) et la config de l'objet, puis posent les paramètres
// de calibration (Print::set_calib_params) que GCode.cpp exploite.
//
// Sortie : objets (config d'objet comprise), surcharges de presets (JSON clé →
// valeur Orca, à appliquer par-dessus la config résolue comme les presets
// édités d'Orca) et paramètres de calibration.
#ifndef BOOST_LOG_DYN_LINK
#define BOOST_LOG_DYN_LINK 1
#endif
#include "engine/src/adapters/ffi/bridge/model.hpp"
#include "engine/src/adapters/ffi/bridge.rs.h"

#include "libslic3r/Arrange.hpp"
#include "libslic3r/CutUtils.hpp"
#include "libslic3r/TriangleMesh.hpp"

#include <nlohmann/json.hpp>
#include "libslic3r/Flow.hpp"
#include "libslic3r/Geometry.hpp"
#include "libslic3r/Model.hpp"
#include "libslic3r/PrintConfig.hpp"
#include "libslic3r/calib.hpp"

#include <cmath>
#include <stdexcept>

namespace webslicer {

rust::Vec<RawObject> model_to_raw(const Slic3r::Model &model);
void config_from_json_print(Slic3r::DynamicPrintConfig &config, const std::string &json_text);
std::string config_to_json_public(const Slic3r::DynamicPrintConfig &config);
Slic3r::Model load_any_model(const std::string &file);
std::string custom_gcodes_to_json(const Slic3r::Model &model);
bool calib_is_bbl(const Slic3r::DynamicPrintConfig &config);
bool calib_has_junction_deviation(const Slic3r::DynamicPrintConfig &config);

namespace {

using namespace Slic3r;

// Presets « édités » : la config de travail suit chaque modification (les
// calculs suivants d'Orca lisent les presets déjà modifiés) et les surcharges
// sont collectées pour le projet.
struct Edited {
    DynamicPrintConfig cfg;
    DynamicPrintConfig overrides;
    void set(const std::string &key, ConfigOption *value)
    {
        overrides.set_key_value(key, value->clone());
        cfg.set_key_value(key, value);
    }
};

double nozzle0(const DynamicPrintConfig &cfg)
{
    const auto *opt = cfg.option<ConfigOptionFloats>("nozzle_diameter");
    return opt && !opt->values.empty() ? opt->values[0] : 0.4;
}

// Plater::cut_horizontal (plan horizontal à la hauteur monde z).
void cut_horizontal(Model &model, size_t idx, double z, ModelObjectCutAttribute keep)
{
    ModelObject *object = model.objects[idx];
    const Vec3d offset = object->instances[0]->get_offset();
    Cut cut(object, 0, Geometry::translation_transform(z * Vec3d::UnitZ() - offset), keep);
    const ModelObjectPtrs &parts = cut.perform_with_plane();
    std::vector<ModelObject *> added;
    for (const ModelObject *part : parts)
        added.push_back(model.add_object(*part));
    model.delete_object(idx);
    for (ModelObject *part : added)
        part->ensure_on_bed();
}

// Place l'ensemble des objets au centre du plateau (ajout dans un projet vide).
void center_on_bed(Model &model, const DynamicPrintConfig &cfg)
{
    BoundingBoxf3 all;
    for (const ModelObject *o : model.objects)
        all.merge(o->instance_bounding_box(0, false));
    if (!all.defined)
        return;
    Vec2d bed_center(0, 0);
    if (const auto *area = cfg.option<ConfigOptionPoints>("printable_area"); area && !area->values.empty())
        bed_center = get_extents(area->values).center();
    const Vec3d shift(bed_center.x() - all.center().x(), bed_center.y() - all.center().y(), 0);
    for (ModelObject *o : model.objects)
        for (ModelInstance *i : o->instances)
            i->set_offset(i->get_offset() + shift);
}

void common_pa(Edited &e)
{
    e.set("overhang_reverse", new ConfigOptionBool(false));
    e.set("precise_z_height", new ConfigOptionBool(false));
    e.set("resonance_avoidance", new ConfigOptionBool(false));
}

} // namespace

// Réglages communs des tours d'input shaping (calib_input_shaping_freq / _damp)
// et du cornering (Calib_Cornering), dans l'ordre du code Orca.
void input_shaping_common(Edited &e, bool cornering, double cornering_end)
{
    const bool jd = calib_has_junction_deviation(e.cfg);
    if (cornering) {
        if (jd) {
            e.set("machine_max_junction_deviation", new ConfigOptionFloats{cornering_end});
            e.set("default_junction_deviation", new ConfigOptionFloat(0.0));
        } else {
            e.set("machine_max_jerk_x", new ConfigOptionFloats{cornering_end});
            e.set("machine_max_jerk_y", new ConfigOptionFloats{cornering_end});
            e.set("default_jerk", new ConfigOptionFloat(0));
        }
    } else if (jd) {
        const double front = e.cfg.option<ConfigOptionFloats>("machine_max_junction_deviation")->values.front();
        e.set("machine_max_junction_deviation", new ConfigOptionFloats{std::max(front, 0.25)});
        e.set("default_junction_deviation", new ConfigOptionFloat(0));
    } else {
        const auto *flavor = e.cfg.option<ConfigOptionEnum<GCodeFlavor>>("gcode_flavor");
        const double jerk_value = (flavor && flavor->value == GCodeFlavor::gcfKlipper) ? 5.0 : 10.0;
        const double jx = e.cfg.option<ConfigOptionFloats>("machine_max_jerk_x")->values.front();
        const double jy = e.cfg.option<ConfigOptionFloats>("machine_max_jerk_y")->values.front();
        e.set("machine_max_jerk_x", new ConfigOptionFloats{std::max(jx, jerk_value)});
        e.set("machine_max_jerk_y", new ConfigOptionFloats{std::max(jy, jerk_value)});
        e.set("default_jerk", new ConfigOptionFloat(0));
    }
    if (!e.cfg.option<ConfigOptionBools>("enable_pressure_advance")->get_at(0)) {
        e.set("enable_pressure_advance", new ConfigOptionBools{true});
        e.set("pressure_advance", new ConfigOptionFloats{0.0});
        e.set("adaptive_pressure_advance", new ConfigOptionBools{false});
    }
    e.set("resonance_avoidance", new ConfigOptionBool{false});
    e.set("input_shaping_emit", new ConfigOptionBool{cornering});
    if (cornering)
        e.set("input_shaping_type", new ConfigOptionEnum<InputShaperType>(InputShaperType::Disable));
    e.set("slow_down_layer_time", new ConfigOptionFloats{0.0});
    e.set("slow_down_min_speed", new ConfigOptionFloats{0.0});
    e.set("slow_down_for_layer_cooling", new ConfigOptionBools{false});
    if (cornering)
        e.set("filament_max_volumetric_speed", new ConfigOptionFloats{200});
}

void input_shaping_print(Edited &e, bool freq, double accel)
{
    if (freq)
        e.set("layer_height", new ConfigOptionFloat(0.2));
    e.set("enable_overhang_speed", new ConfigOptionBool{false});
    e.set("timelapse_type", new ConfigOptionEnum<TimelapseType>(tlTraditional));
    e.set("wall_loops", new ConfigOptionInt(1));
    e.set("top_shell_layers", new ConfigOptionInt(0));
    e.set("bottom_shell_layers", new ConfigOptionInt(1));
    e.set("sparse_infill_density", new ConfigOptionPercent(0));
    e.set("detect_thin_wall", new ConfigOptionBool(false));
    e.set("spiral_mode", new ConfigOptionBool(true));
    e.set("spiral_mode_smooth", new ConfigOptionBool(false));
    e.set("bottom_surface_pattern", new ConfigOptionEnum<InfillPattern>(ipRectilinear));
    e.set("outer_wall_speed", new ConfigOptionFloat(200));
    e.set("default_acceleration", new ConfigOptionFloat(accel));
    e.set("outer_wall_acceleration", new ConfigOptionFloat(accel));
    e.set("precise_z_height", new ConfigOptionBool(false));
}

void outer_brim3(ModelObject *o)
{
    o->config.set_key_value("brim_type", new ConfigOptionEnum<BrimType>(btOuterOnly));
    o->config.set_key_value("brim_width", new ConfigOptionFloat(3.0));
    o->config.set_key_value("brim_object_gap", new ConfigOptionFloat(0.0));
}

RawCalibProject calib_project_raw(rust::Str kind_str, double start, double end, double step,
                                  rust::Str config_json, rust::Str resources, rust::Str extra_json)
{
    const std::string kind(kind_str), res(resources);
    // extra : { option, test_model, print_numbers, speeds[], accelerations[] } (Calib_Params).
    const nlohmann::json extra =
        extra_json.empty() ? nlohmann::json::object() : nlohmann::json::parse(std::string(extra_json));
    const std::string option = extra.value("option", std::string());
    const int test_model = extra.value("test_model", 0);
    Edited e;
    config_from_json_print(e.cfg, std::string(config_json));
    // Presets complets (comme ceux de la GUI) : défauts d'Orca pour les clés absentes.
    for (const std::string &key : FullPrintConfig::defaults().keys()) {
        if (e.cfg.has(key))
            continue;
        if (const ConfigOptionDef *def = print_config_def.get(key); def != nullptr && def->default_value)
            e.cfg.set_key_value(key, def->default_value->clone());
    }
    Model model;
    CalibMode mode = CalibMode::Calib_None;
    double c_start = start, c_end = end, c_step = step;
    std::string name;
    auto load = [&](const std::string &rel) {
        model = load_any_model(res + "/calib/" + rel);
        if (model.objects.empty())
            throw std::runtime_error("modèle de calibration introuvable : " + rel);
        for (ModelObject *o : model.objects) {
            if (o->instances.empty())
                o->add_instance();
            o->ensure_on_bed();
        }
    };

    if (kind == "temp_tower") {
        // Plater::calib_temp
        name = "Nozzle temperature test";
        mode = CalibMode::Calib_Temp_Tower;
        load("temperature_tower/temperature_tower.drc");
        constexpr double base_nozzle = 0.4, block_height = 10.0;
        constexpr int temp_step = 5;
        const long start_temp = lround(start);
        double nozzle = nozzle0(e.cfg);
        if (nozzle <= 0.0)
            nozzle = base_nozzle;
        const double nozzle_scale = nozzle / base_nozzle;
        auto bb = model.objects[0]->bounding_box_exact();
        long blocks = lround((500 - end) / temp_step + 1);
        if (blocks > 0) {
            const double h = blocks * block_height - EPSILON;
            if (h < bb.size().z())
                cut_horizontal(model, 0, h, ModelObjectCutAttribute::KeepLower);
        }
        bb = model.objects[0]->bounding_box_exact();
        blocks = lround((500 - start) / temp_step);
        if (blocks > 0) {
            const double h = blocks * block_height + EPSILON;
            if (h < bb.size().z())
                cut_horizontal(model, 0, h, ModelObjectCutAttribute::KeepUpper);
        }
        if (std::abs(nozzle_scale - 1.0) > EPSILON)
            model.objects[0]->scale(nozzle_scale, nozzle_scale, nozzle_scale);
        model.objects[0]->ensure_on_bed();
        e.set("resonance_avoidance", new ConfigOptionBool(false));
        e.set("nozzle_temperature_initial_layer", new ConfigOptionInts(1, int(start_temp)));
        e.set("nozzle_temperature", new ConfigOptionInts(1, int(start_temp)));
        auto &oc = model.objects[0]->config;
        oc.set_key_value("layer_height", new ConfigOptionFloat(nozzle / 2));
        oc.set_key_value("brim_type", new ConfigOptionEnum<BrimType>(btOuterOnly));
        oc.set_key_value("brim_width", new ConfigOptionFloat(5.0));
        oc.set_key_value("brim_object_gap", new ConfigOptionFloat(0.0));
        oc.set_key_value("alternate_extra_wall", new ConfigOptionBool(false));
        oc.set_key_value("seam_slope_type", new ConfigOptionEnum<SeamScarfType>(SeamScarfType::None));
        oc.set_key_value("overhang_reverse", new ConfigOptionBool(false));
        oc.set_key_value("precise_z_height", new ConfigOptionBool(false));
        e.set("enable_wrapping_detection", new ConfigOptionBool(false));
        e.set("initial_layer_print_height", new ConfigOptionFloat(nozzle / 2));
    } else if (kind == "pa_line") {
        // Plater::calib_pa (Calib_PA_Line)
        name = "Pressure Advance Test";
        mode = CalibMode::Calib_PA_Line;
        common_pa(e);
        load("pressure_advance/pressure_advance_test.drc");
    } else if (kind == "pa_tower") {
        // Plater::calib_pa + _calib_pa_tower
        name = "Pressure Advance Test";
        mode = CalibMode::Calib_PA_Tower;
        common_pa(e);
        load("pressure_advance/tower_with_seam.drc");
        const double nozzle = nozzle0(e.cfg);
        e.set("enable_wrapping_detection", new ConfigOptionBool(false));
        e.set("slow_down_layer_time", new ConfigOptionFloats{1.0f});
        auto &oc = model.objects[0]->config;
        oc.set_key_value("alternate_extra_wall", new ConfigOptionBool(false));
        const double wall_speed = CalibPressureAdvance::find_optimal_PA_speed(
            e.cfg, e.cfg.get_abs_value("line_width", nozzle), e.cfg.get_abs_value("layer_height"), 0, 0);
        oc.set_key_value("outer_wall_speed", new ConfigOptionFloat(wall_speed));
        oc.set_key_value("inner_wall_speed", new ConfigOptionFloat(wall_speed));
        oc.set_key_value("seam_position", new ConfigOptionEnum<SeamPosition>(spRear));
        oc.set_key_value("wall_loops", new ConfigOptionInt(2));
        oc.set_key_value("top_shell_layers", new ConfigOptionInt(0));
        oc.set_key_value("bottom_shell_layers", new ConfigOptionInt(0));
        oc.set_key_value("sparse_infill_density", new ConfigOptionPercent(0));
        oc.set_key_value("brim_type", new ConfigOptionEnum<BrimType>(btEar));
        oc.set_key_value("brim_object_gap", new ConfigOptionFloat(.0f));
        oc.set_key_value("brim_ears_max_angle", new ConfigOptionFloat(135.f));
        oc.set_key_value("brim_width", new ConfigOptionFloat(6.f));
        oc.set_key_value("seam_slope_type", new ConfigOptionEnum<SeamScarfType>(SeamScarfType::None));
        e.set("max_volumetric_extrusion_rate_slope", new ConfigOptionFloat(0));
        const double new_height = std::ceil((end - start) / step) + 1;
        const auto bb = model.objects[0]->bounding_box_exact();
        if (new_height < bb.size().z())
            cut_horizontal(model, 0, new_height, ModelObjectCutAttribute::KeepLower);
    } else if (kind == "flow_pass1" || kind == "flow_pass2" || kind == "flow_yolo" || kind == "flow_yolo_fine") {
        // Plater::calib_flowrate + adjust_settings_for_flowrate_calib (pas de mode calibration)
        const bool linear = kind == "flow_yolo" || kind == "flow_yolo_fine";
        const int pass = (kind == "flow_pass2" || kind == "flow_yolo_fine") ? 2 : 1;
        name = linear ? (pass == 2 ? "Orca YOLO Flow Calibration - Perfectionist version" : "Orca YOLO Flow Calibration")
                      : (pass == 2 ? "Flowrate Test - Pass2" : "Flowrate Test - Pass1");
        load(linear ? (pass == 1 ? "filament_flow/Orca-LinearFlow.3mf" : "filament_flow/Orca-LinearFlow_fine.3mf")
                    : (pass == 1 ? "filament_flow/flowrate-test-pass1.3mf" : "filament_flow/flowrate-test-pass2.3mf"));
        const InfillPattern pattern = option == "monotonic" ? ipMonotonic : ipArchimedeanChords;
        const float nozzle = float(nozzle0(e.cfg));
        const float xy_scale = nozzle / 0.6f;
        double first_layer_height = e.cfg.opt_float("initial_layer_print_height");
        const double layer_height = nozzle / 2.0;
        first_layer_height = std::max(first_layer_height, layer_height);
        const float zscale = float((first_layer_height + 9 * layer_height) / 2);
        // selection.scale(relatif) sur toute la sélection, autour de son centre.
        const Vec3d s = xy_scale > 1.2f ? Vec3d(xy_scale, xy_scale, zscale) : Vec3d(1, 1, zscale);
        BoundingBoxf3 all;
        for (const ModelObject *o : model.objects)
            all.merge(o->instance_bounding_box(0, false));
        const Vec3d c = all.center();
        for (ModelObject *o : model.objects)
            for (ModelInstance *i : o->instances) {
                const Vec3d off = i->get_offset();
                i->set_offset(c + (off - c).cwiseProduct(s));
                i->set_scaling_factor(i->get_scaling_factor().cwiseProduct(s));
            }
        const double cur_flowrate = e.cfg.option<ConfigOptionFloats>("filament_flow_ratio")->get_at(0);
        const Flow infill_flow = Flow(nozzle * 1.2f, layer_height, nozzle);
        const double max_vol = e.cfg.option<ConfigOptionFloats>("filament_max_volumetric_speed")->get_at(0);
        const double max_infill_speed =
            linear ? max_vol / (infill_flow.mm3_per_mm() * (cur_flowrate + (pass == 2 ? 0.035 : 0.05)) / cur_flowrate)
                   : max_vol / (infill_flow.mm3_per_mm() * (pass == 1 ? 1.2 : 1));
        const double internal_solid_speed = std::floor(std::min(e.cfg.opt_float("internal_solid_infill_speed"), max_infill_speed));
        const double top_surface_speed = std::floor(std::min(e.cfg.opt_float("top_surface_speed"), max_infill_speed));
        for (ModelObject *o : model.objects) {
            o->ensure_on_bed();
            auto &oc = o->config;
            oc.set_key_value("wall_loops", new ConfigOptionInt(1));
            oc.set_key_value("only_one_wall_top", new ConfigOptionBool(true));
            oc.set_key_value("thick_internal_bridges", new ConfigOptionBool(false));
            oc.set_key_value("enable_extra_bridge_layer", new ConfigOptionEnum<EnableExtraBridgeLayer>(eblDisabled));
            oc.set_key_value("internal_bridge_density", new ConfigOptionPercent(100));
            oc.set_key_value("sparse_infill_density", new ConfigOptionPercent(35));
            oc.set_key_value("min_width_top_surface", new ConfigOptionFloatOrPercent(100, true));
            oc.set_key_value("bottom_shell_layers", new ConfigOptionInt(2));
            oc.set_key_value("top_shell_layers", new ConfigOptionInt(5));
            oc.set_key_value("top_shell_thickness", new ConfigOptionFloat(0));
            oc.set_key_value("bottom_shell_thickness", new ConfigOptionFloat(0));
            oc.set_key_value("detect_thin_wall", new ConfigOptionBool(true));
            oc.set_key_value("filter_out_gap_fill", new ConfigOptionFloat(0));
            oc.set_key_value("sparse_infill_pattern", new ConfigOptionEnum<InfillPattern>(ipRectilinear));
            oc.set_key_value("top_surface_line_width", new ConfigOptionFloatOrPercent(nozzle * 1.2f, false));
            oc.set_key_value("internal_solid_infill_line_width", new ConfigOptionFloatOrPercent(nozzle * 1.2f, false));
            oc.set_key_value("top_surface_pattern", new ConfigOptionEnum<InfillPattern>(pattern));
            oc.set_key_value("top_solid_infill_flow_ratio", new ConfigOptionFloat(1.0f));
            oc.set_key_value("infill_direction", new ConfigOptionFloat(45));
            oc.set_key_value("solid_infill_direction", new ConfigOptionFloat(135));
            oc.set_key_value("align_infill_direction_to_model", new ConfigOptionBool(true));
            oc.set_key_value("ironing_type", new ConfigOptionEnum<IroningType>(IroningType::NoIroning));
            oc.set_key_value("internal_solid_infill_speed", new ConfigOptionFloat(internal_solid_speed));
            oc.set_key_value("top_surface_speed", new ConfigOptionFloat(top_surface_speed));
            oc.set_key_value("seam_slope_type", new ConfigOptionEnum<SeamScarfType>(SeamScarfType::None));
            oc.set_key_value("gap_fill_target", new ConfigOptionEnum<GapFillTarget>(GapFillTarget::gftNowhere));
            e.set("max_volumetric_extrusion_rate_slope", new ConfigOptionFloat(0));
            oc.set_key_value("calib_flowrate_topinfill_special_order", new ConfigOptionBool(true));
            // Débit lu dans le nom « flowrate_xxx » (m = signe moins).
            std::string modifier_text = o->name.size() > 9 ? o->name.substr(9) : std::string();
            if (!modifier_text.empty() && modifier_text[0] == 'm')
                modifier_text[0] = '-';
            float modifier = 1.0f;
            try {
                modifier = std::stof(modifier_text);
            } catch (...) {
            }
            oc.set_key_value("print_flow_ratio",
                             new ConfigOptionFloat(linear ? (cur_flowrate + modifier) / cur_flowrate : 1.0f + modifier / 100.f));
        }
        e.set("layer_height", new ConfigOptionFloat(layer_height));
        e.set("alternate_extra_wall", new ConfigOptionBool(false));
        e.set("initial_layer_print_height", new ConfigOptionFloat(first_layer_height));
        e.set("reduce_crossing_wall", new ConfigOptionBool(true));
        e.set("enable_wrapping_detection", new ConfigOptionBool(false));
        e.set("resonance_avoidance", new ConfigOptionBool(false));
    } else if (kind == "max_vol_speed") {
        // Plater::calib_max_vol_speed
        name = "Max volumetric speed test";
        mode = CalibMode::Calib_Vol_speed_Tower;
        load("volumetric_speed/SpeedTestStructure.drc");
        ModelObject *obj = model.objects[0];
        const auto bed = e.cfg.option<ConfigOptionPoints>("printable_area")->values;
        const BoundingBoxf bed_ext = get_extents(bed);
        const double scale_obj = (bed_ext.size().x() - 10) / obj->bounding_box_exact().size().x();
        if (scale_obj < 1.0)
            obj->scale(scale_obj, 1, 1);
        const double nozzle = nozzle0(e.cfg);
        const double line_width = nozzle * 1.75;
        const double layer_height = nozzle * 0.8;
        if (auto *max_lh = e.cfg.option<ConfigOptionFloats>("max_layer_height"); max_lh && !max_lh->values.empty()
                                                                                  && max_lh->values[0] < layer_height) {
            ConfigOptionFloats updated = *max_lh;
            updated.values[0] = layer_height;
            e.set("max_layer_height", new ConfigOptionFloats(updated));
        }
        e.set("filament_max_volumetric_speed", new ConfigOptionFloats{200});
        e.set("slow_down_layer_time", new ConfigOptionFloats{0.0});
        e.set("resonance_avoidance", new ConfigOptionBool(false));
        auto &oc = obj->config;
        oc.set_key_value("enable_overhang_speed", new ConfigOptionBool{false});
        oc.set_key_value("wall_loops", new ConfigOptionInt(1));
        oc.set_key_value("alternate_extra_wall", new ConfigOptionBool(false));
        oc.set_key_value("top_shell_layers", new ConfigOptionInt(0));
        oc.set_key_value("bottom_shell_layers", new ConfigOptionInt(0));
        oc.set_key_value("sparse_infill_density", new ConfigOptionPercent(0));
        oc.set_key_value("outer_wall_line_width", new ConfigOptionFloatOrPercent(line_width, false));
        oc.set_key_value("layer_height", new ConfigOptionFloat(layer_height));
        oc.set_key_value("brim_type", new ConfigOptionEnum<BrimType>(btOuterAndInner));
        oc.set_key_value("brim_width", new ConfigOptionFloat(5.0));
        oc.set_key_value("brim_object_gap", new ConfigOptionFloat(0.0));
        oc.set_key_value("precise_z_height", new ConfigOptionBool(false));
        e.set("timelapse_type", new ConfigOptionEnum<TimelapseType>(tlTraditional));
        e.set("spiral_mode", new ConfigOptionBool(true));
        e.set("max_volumetric_extrusion_rate_slope", new ConfigOptionFloat(0));
        e.set("enable_wrapping_detection", new ConfigOptionBool(false));
        const double height = (end - start + 1) / step;
        if (height < obj->bounding_box_exact().size().z())
            cut_horizontal(model, 0, height, ModelObjectCutAttribute::KeepLower);
        const double flow_ratio = e.cfg.option<ConfigOptionFloats>("filament_flow_ratio")->get_at(0);
        const double mm3_per_mm = Flow(line_width, layer_height, nozzle).mm3_per_mm() * flow_ratio;
        c_end = end / mm3_per_mm;
        c_start = start / mm3_per_mm;
        c_step = step / mm3_per_mm;
    } else if (kind == "retraction") {
        // Plater::calib_retraction
        name = "Retraction";
        mode = CalibMode::Calib_Retraction_tower;
        load("retraction/retraction_tower.drc");
        ModelObject *obj = model.objects[0];
        e.set("enable_wrapping_detection", new ConfigOptionBool(false));
        const float nozzle = float(nozzle0(e.cfg));
        const float layer_height = nozzle <= 0.1f ? 0.05f : (nozzle <= 0.2f ? 0.1f : 0.2f);
        if (auto *max_lh = e.cfg.option<ConfigOptionFloats>("max_layer_height"); max_lh && !max_lh->values.empty()
                                                                                  && max_lh->values[0] < layer_height) {
            ConfigOptionFloats updated = *max_lh;
            updated.values[0] = layer_height;
            e.set("max_layer_height", new ConfigOptionFloats(updated));
        }
        e.set("resonance_avoidance", new ConfigOptionBool(false));
        e.set("use_firmware_retraction", new ConfigOptionBool(false));
        auto &oc = obj->config;
        oc.set_key_value("wall_loops", new ConfigOptionInt(2));
        oc.set_key_value("top_shell_layers", new ConfigOptionInt(0));
        oc.set_key_value("bottom_shell_layers", new ConfigOptionInt(3));
        oc.set_key_value("sparse_infill_density", new ConfigOptionPercent(0));
        e.set("initial_layer_print_height", new ConfigOptionFloat(layer_height));
        oc.set_key_value("layer_height", new ConfigOptionFloat(layer_height));
        oc.set_key_value("alternate_extra_wall", new ConfigOptionBool(false));
        oc.set_key_value("seam_position", new ConfigOptionEnum<SeamPosition>(spAligned));
        oc.set_key_value("wall_sequence", new ConfigOptionEnum<WallSequence>(WallSequence::InnerOuter));
        oc.set_key_value("overhang_reverse", new ConfigOptionBool(false));
        oc.set_key_value("precise_z_height", new ConfigOptionBool(false));
        const double height = 1.0 + 0.4 + (end - start) / step - EPSILON;
        if (height < obj->bounding_box_exact().size().z())
            cut_horizontal(model, 0, height, ModelObjectCutAttribute::KeepLower);
    } else if (kind == "vfa") {
        // Plater::calib_VFA
        name = "VFA test";
        mode = CalibMode::Calib_VFA_Tower;
        load("vfa/vfa.drc");
        e.set("resonance_avoidance", new ConfigOptionBool(false));
        e.set("slow_down_layer_time", new ConfigOptionFloats{0.0});
        e.set("enable_overhang_speed", new ConfigOptionBool{false});
        e.set("timelapse_type", new ConfigOptionEnum<TimelapseType>(tlTraditional));
        e.set("wall_loops", new ConfigOptionInt(1));
        e.set("alternate_extra_wall", new ConfigOptionBool(false));
        e.set("top_shell_layers", new ConfigOptionInt(0));
        e.set("bottom_shell_layers", new ConfigOptionInt(1));
        e.set("sparse_infill_density", new ConfigOptionPercent(0));
        e.set("detect_thin_wall", new ConfigOptionBool(false));
        e.set("spiral_mode", new ConfigOptionBool(true));
        e.set("enable_wrapping_detection", new ConfigOptionBool(false));
        e.set("precise_z_height", new ConfigOptionBool(false));
        auto &oc = model.objects[0]->config;
        oc.set_key_value("brim_type", new ConfigOptionEnum<BrimType>(btOuterOnly));
        oc.set_key_value("brim_width", new ConfigOptionFloat(3.0));
        oc.set_key_value("brim_object_gap", new ConfigOptionFloat(0.0));
        const double height = 5 * ((end - start) / step + 1);
        if (height < model.objects[0]->bounding_box_exact().size().z())
            cut_horizontal(model, 0, height, ModelObjectCutAttribute::KeepLower);
    } else if (kind == "is_freq" || kind == "is_damp" || kind == "cornering") {
        // Plater::calib_input_shaping_freq / calib_input_shaping_damp / Calib_Cornering
        const bool cornering = kind == "cornering";
        name = kind == "is_freq" ? "Input shaping Frequency test"
                                 : (kind == "is_damp" ? "Input shaping Damping test" : "Cornering test");
        mode = kind == "is_freq" ? CalibMode::Calib_Input_shaping_freq
                                 : (kind == "is_damp" ? CalibMode::Calib_Input_shaping_damp : CalibMode::Calib_Cornering);
        load(test_model < 1 ? "input_shaping/ringing_tower.drc"
                            : (!cornering || test_model == 1 ? "input_shaping/fast_tower_test.drc" : "cornering/SCV-V2.drc"));
        input_shaping_common(e, cornering, end);
        input_shaping_print(e, kind == "is_freq", cornering ? 2000 : 20000);
        outer_brim3(model.objects[0]);
    } else if (kind == "pa_pattern") {
        // Plater::calib_pa (commun) + _calib_pa_pattern + _calib_pa_pattern_gen_gcode
        name = "Pressure Advance Test";
        mode = CalibMode::Calib_PA_Pattern;
        common_pa(e);
        std::vector<double> speeds = extra.value("speeds", std::vector<double>{});
        std::vector<double> accels = extra.value("accelerations", std::vector<double>{});
        const double nozzle = nozzle0(e.cfg);
        e.set("filament_retract_when_changing_layer", new ConfigOptionBoolsNullable{false});
        e.set("filament_wipe", new ConfigOptionBoolsNullable{false});
        e.set("wipe", new ConfigOptionBools{false});
        e.set("retract_when_changing_layer", new ConfigOptionBools{false});
        e.set("resonance_avoidance", new ConfigOptionBool{false});
        auto fval = [&](const char *key) { return e.cfg.option<ConfigOptionFloat>(key)->value; };
        double accel = fval("outer_wall_acceleration");
        if (accel == 0)
            accel = fval("inner_wall_acceleration");
        if (accel == 0)
            accel = fval("default_acceleration");
        if (accels.empty())
            accels.assign({accel});
        else
            accel = *std::max_element(accels.begin(), accels.end());
        e.set("outer_wall_acceleration", new ConfigOptionFloat(accel));
        e.set("print_sequence", new ConfigOptionEnum<PrintSequence>(PrintSequence::ByLayer));
        const bool jd = calib_has_junction_deviation(e.cfg);
        if (!jd && fval("default_jerk") > 0) {
            double jerk = fval("outer_wall_jerk");
            if (jerk == 0)
                jerk = fval("inner_wall_jerk");
            if (jerk == 0)
                jerk = fval("default_jerk");
            for (const char *key : {"default_jerk", "outer_wall_jerk", "inner_wall_jerk", "top_surface_jerk", "infill_jerk", "travel_jerk"})
                e.set(key, new ConfigOptionFloat(jerk));
        }
        if (jd)
            e.set("default_junction_deviation", new ConfigOptionFloat(0));
        const SuggestedConfigCalibPAPattern suggested;
        for (const auto &opt : suggested.float_pairs)
            e.set(opt.first, new ConfigOptionFloat(opt.second));
        for (const auto &opt : suggested.nozzle_ratio_pairs)
            e.set(opt.first, new ConfigOptionFloatOrPercent(nozzle * opt.second / 100, false));
        for (const auto &opt : suggested.int_pairs)
            e.set(opt.first, new ConfigOptionInt(opt.second));
        e.set(suggested.brim_pair.first, new ConfigOptionEnum<BrimType>(suggested.brim_pair.second));
        e.set("enable_wrapping_detection", new ConfigOptionBool(false));
        if (speeds.empty()) {
            const double speed = CalibPressureAdvance::find_optimal_PA_speed(
                e.cfg, e.cfg.get_abs_value("line_width", nozzle), e.cfg.get_abs_value("layer_height"), 0, 0);
            e.set("outer_wall_speed", new ConfigOptionFloat(speed));
            speeds.assign({speed});
        } else if (speeds.size() == 1) {
            e.set("outer_wall_speed", new ConfigOptionFloat(speeds.front()));
        }
        const bool is_bbl = calib_is_bbl(e.cfg);
        Calib_Params params;
        params.mode = mode;
        params.start = start;
        params.end = end;
        params.step = step;
        params.print_numbers = extra.value("print_numbers", false);
        params.speeds = speeds;
        params.accelerations = accels;
        // Cube « poignée » (load_generic_subobject("Cube")).
        ModelObject *cube = model.add_object();
        cube->name = "Cube";
        cube->add_volume(TriangleMesh(its_make_cube(20, 20, 20)));
        cube->add_instance();
        cube->center_around_origin();
        cube->ensure_on_bed();
        CalibPressureAdvancePattern pa_pattern(params, e.cfg, is_bbl, *cube, Vec3d::Zero());
        auto cube_bb = cube->raw_bounding_box();
        cube->scale((pa_pattern.print_size_x() + 4) / cube_bb.size().x(), (pa_pattern.print_size_y() + 4) / cube_bb.size().y(),
                    pa_pattern.max_layer_z() / cube_bb.size().z());
        arrangement::ArrangePolygons arranged;
        {
            arrangement::ArrangeParams ap;
            Points bedpts = arrangement::get_shrink_bedpts(&e.cfg, ap);
            for (size_t i = 0; i < speeds.size() * accels.size(); i++) {
                arrangement::ArrangePolygon p;
                cube->instances[0]->get_arrange_polygon(&p);
                p.bed_idx = 0;
                arranged.emplace_back(p);
            }
            arrangement::arrange(arranged, bedpts, ap);
        }
        cube_bb = cube->raw_bounding_box();
        cube->scale(pa_pattern.handle_xy_size() / cube_bb.size().x(), pa_pattern.handle_xy_size() / cube_bb.size().y(),
                    pa_pattern.max_layer_z() / cube_bb.size().z());
        std::vector<ModelObject *> tests;
        for (size_t idx = 0; idx < arranged.size(); idx++) {
            const auto &ai = arranged[idx];
            if (ai.bed_idx != 0)
                throw std::runtime_error("motif PA : tests hors du plateau (trop de vitesses × accélérations)");
            const double tspd = speeds[idx % speeds.size()];
            const double tacc = accels[idx / speeds.size()];
            ModelObject *obj = idx == 0 ? cube : model.add_object(*cube);
            obj->name = "pa_pattern_" + std::to_string(int(tspd)) + "_" + std::to_string(int(tacc));
            if (speeds.size() > 1)
                obj->config.set_key_value("outer_wall_speed", new ConfigOptionFloat(tspd));
            if (accels.size() > 1)
                obj->config.set_key_value("outer_wall_acceleration", new ConfigOptionFloat(tacc));
            obj->instances[0]->set_offset(Vec3d(unscale<double>(ai.translation(X)), unscale<double>(ai.translation(Y)), 0) +
                                          pa_pattern.handle_pos_offset());
            obj->ensure_on_bed();
            tests.push_back(obj);
        }
        // _calib_pa_pattern_gen_gcode : un G-code par test, concaténés par couche.
        std::vector<CustomGCode::Info> mgc;
        for (ModelObject *obj : tests)
            mgc.emplace_back(pa_pattern.generate_custom_gcodes(e.cfg, is_bbl, *obj, Vec3d::Zero()));
        CustomGCode::Info &pcgc = model.plates_custom_gcodes[0];
        pcgc = std::move(mgc[0]);
        for (size_t i = 0; i < pcgc.gcodes.size(); i++)
            for (size_t k = 1; k < mgc.size(); k++)
                pcgc.gcodes[i].extra += mgc[k].gcodes[i].extra;
    } else {
        throw std::runtime_error("calibration inconnue : " + kind);
    }
    if (mode != CalibMode::Calib_PA_Pattern) // le motif est déjà disposé (arrangement Orca)
        center_on_bed(model, e.cfg);

    RawCalibProject out;
    out.name = name;
    out.objects = model_to_raw(model);
    out.overrides_json = config_to_json_public(e.overrides);
    out.custom_gcodes_json = custom_gcodes_to_json(model);
    out.calib_mode = static_cast<int32_t>(mode);
    out.calib_start = c_start;
    out.calib_end = c_end;
    out.calib_step = c_step;
    return out;
}

} // namespace webslicer
