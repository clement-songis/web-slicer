// Wrappers arrangement automatique + orientation automatique (T016, FR-013).
// Arrange.hpp utilise BOOST_LOG_TRIVIAL : Boost.Log est lié dynamiquement.
#ifndef BOOST_LOG_DYN_LINK
#define BOOST_LOG_DYN_LINK 1
#endif
#include "engine/src/adapters/ffi/bridge/model.hpp"
#include "engine/src/adapters/ffi/bridge.rs.h"

#include "libslic3r/Model.hpp"
#include "libslic3r/PrintConfig.hpp"
#include "libslic3r/Arrange.hpp"
#include "libslic3r/ClipperUtils.hpp"
#include "libslic3r/ModelArrange.hpp"
#include "libslic3r/Orient.hpp"
#include "libslic3r/Print.hpp"
#include "libslic3r/GCode/WipeTower.hpp"

#include <algorithm>
#include <map>
#include <set>

#include <cmath>
#include <stdexcept>
#include <vector>

namespace webslicer {

// défini dans model.cpp / project.cpp
rust::Vec<RawObject> model_to_raw(const Slic3r::Model &model);
void raw_into_model(const rust::Vec<RawObject> &objects, Slic3r::Model &model);
void config_from_json_print(Slic3r::DynamicPrintConfig &config, const std::string &json_text);

static void config_from_json(Slic3r::DynamicPrintConfig &config, const std::string &json_text)
{
    config_from_json_print(config, json_text);
}

// Lot 49 (audit 47 ARR-a) : config d'arrangement complétée comme
// `PresetBundle::full_fff_config` (PresetBundle.cpp:3883-3891, ce que
// ArrangeJob lit via `preset_bundle->full_config()` et ce qu'a déjà reçu le
// Print du plateau dont init_arrange_params tire object_skirt_offset) :
// `filament_map` a exactement une entrée par filament (complétée par 1).
// Sans cela `Print::apply` (print_config_diffs → compute_filament_override_value,
// PrintConfig.cpp:10323) appelait ConfigOptionVector::apply_override avec une
// carte d'une entrée pour des surcharges filament de N entrées : assertion
// `default_index.size() == rhs_vec->size()` (Config.hpp:739), worker abattu,
// HTTP 500 sur « Disposer » dès 2 filaments (A1 mini, Voron). Nombre de
// filaments = taille de filament_diameter, comme le tranchage (slice.cpp).
static void resize_filament_map(Slic3r::DynamicPrintConfig &config)
{
    using namespace Slic3r;
    const DynamicPrintConfig &view = config;
    size_t filaments = 1;
    if (const auto *diameters = view.option<ConfigOptionFloats>("filament_diameter"))
        filaments = std::max<size_t>(1, diameters->values.size());
    std::vector<int> maps;
    if (const auto *current = view.option<ConfigOptionInts>("filament_map"))
        maps = current->values;
    if (maps.size() == filaments)
        return;
    maps.resize(filaments, 1);
    config.set_key_value("filament_map", new ConfigOptionInts(maps));
}

namespace {

// ArrangeJob::prepare_wipe_tower (lot 35) : réserve la place de la tour
// d'essuyage comme obstacle **fixe** sur chaque plateau potentiel (0..
// MAX_NUM_PLATES, verrouillés sautés, indexés sur la séquence non verrouillée
// comme les objets arrangés). Reproduit PartPlate::estimate_wipe_tower_size +
// estimate_wipe_tower_polygon sans la GUI : volume de purge par extrudeur,
// hauteur max des objets du plateau, marge WIPE_TOWER_MARGIN (1 mm) + brim,
// position clampée depuis wipe_tower_x/wipe_tower_y. Une tour déjà tranchée
// (GLVolume GUI) n'existe pas côté serveur : seule l'estimation est posée,
// exactement comme la branche `else if (need_wipe_tower)` d'Orca.
// `need_min_depth` = timelapse fluide ou wrapping (variable locale
// `need_wipe_tower` d'estimate_wipe_tower_size).
void prepare_wipe_towers(Slic3r::arrangement::ArrangePolygons &unselected,
                         const Slic3r::Model &model,
                         const Slic3r::DynamicPrintConfig &config,
                         const Slic3r::Points &bed,
                         const rust::Vec<uint32_t> &plate_of_instance,
                         const std::vector<Slic3r::ModelInstance *> &flat,
                         const std::set<uint32_t> &locked,
                         int plate_count,
                         bool need_min_depth)
{
    using namespace Slic3r;
    using namespace Slic3r::arrangement;
    constexpr int max_plates = 36; // MAX_NUM_PLATES = MAX_PLATES_COUNT

    const BoundingBox bed_bb = Polygon(bed).bounding_box();
    const double plate_width  = unscaled(bed_bb.size().x());
    const double plate_depth = unscaled(bed_bb.size().y());

    // PresetBundle::get_printer_extruder_count : nombre de buses.
    int extruder_count = 1;
    if (const auto *nozzle = config.option<ConfigOptionFloats>("nozzle_diameter"))
        extruder_count = std::max<int>(1, static_cast<int>(nozzle->values.size()));

    // PartPlateList::get_extruders(true) : extrude_ids des instances
    // imprimables + surcharges globales murs/infill/support
    // (PartPlate::get_extruders, replis objet → global omis : les extrudeurs
    // par objet sont déjà dans extrude_ids).
    std::set<int> extruder_ids;
    for (size_t k = 0; k < flat.size(); ++k) {
        if (!flat[k]->printable)
            continue;
        // Reconstruit les extrudeurs depuis le modèle (extrude_ids du polygone
        // d'arrangement n'est pas conservé pour les instances verrouillées).
        for (const ModelVolume *mv : flat[k]->get_object()->volumes)
            for (int id : mv->get_extruders())
                extruder_ids.insert(id);
    }
    for (const char *key : {"support_interface_filament", "support_filament",
                            "outer_wall_filament_id", "inner_wall_filament_id",
                            "sparse_infill_filament_id", "internal_solid_filament_id",
                            "top_surface_filament_id", "bottom_surface_filament_id"}) {
        const ConfigOption *o = config.option(key);
        if (o != nullptr && o->getInt() > 0)
            extruder_ids.insert(o->getInt());
    }
    const int extruder_size = static_cast<int>(extruder_ids.size());

    // Hauteur max par plateau : estimate_wipe_tower_size ne prend que les
    // objets dont l'instance 0 est sur le plateau (contain_instance_totally).
    std::map<int, double> height_by_plate;
    {
        size_t k = 0;
        for (const ModelObject *mo : model.objects) {
            if (!mo->instances.empty() && k < flat.size()) {
                const int plate = static_cast<int>(plate_of_instance[k]);
                const double h  = mo->bounding_box_exact().size().z();
                height_by_plate[plate] = std::max(height_by_plate[plate], h);
            }
            k += mo->instances.size();
        }
    }

    // estimate_wipe_tower_size.
    double layer_height = 0.08;
    if (const ConfigOption *o = config.option("layer_height"))
        layer_height = o->getFloat();
    double filament_change_volume = 0.;
    {
        std::vector<double> lengths, diameters;
        if (const auto *o = config.option<ConfigOptionFloats>("filament_change_length"))
            lengths = o->values;
        if (const auto *o = config.option<ConfigOptionFloats>("filament_diameter"))
            diameters = o->values;
        const double length   = lengths.empty() ? 0. : *std::max_element(lengths.begin(), lengths.end());
        const double diameter = diameters.empty() ? 1.75 : *std::max_element(diameters.begin(), diameters.end());
        filament_change_volume = length * PI * diameter * diameter / 4.;
    }
    const double wipe_volume = config.opt_float("prime_volume");
    const double width = config.opt_float("prime_tower_width");
    const double extra_spacing = config.opt_float("prime_tower_infill_gap") / 100.;
    bool use_rib_wall = false;
    if (const auto *o = config.option<ConfigOptionEnum<WipeTowerWallType>>("wipe_tower_wall_type"))
        use_rib_wall = o->value == WipeTowerWallType::wtwRib;
    double rib_width = config.opt_float("wipe_tower_rib_width");

    const auto tower_x = config.option<ConfigOptionFloats>("wipe_tower_x");
    const auto tower_y = config.option<ConfigOptionFloats>("wipe_tower_y");
    if (tower_x == nullptr || tower_y == nullptr)
        return;
    const float tower_brim = static_cast<float>(config.opt_float("prime_tower_brim_width"));
    const float margin = 1.f + tower_brim; // WIPE_TOWER_MARGIN = 1 mm

    int bedid_unlocked = 0;
    for (int bedid = 0; bedid < max_plates; ++bedid) {
        const int plate_index_valid = std::min(bedid, plate_count - 1);
        if (bedid < plate_count && locked.count(static_cast<uint32_t>(bedid)) > 0)
            continue; // plateau verrouillé : ni tour, ni incrémentation

        const double max_height = height_by_plate.count(plate_index_valid) != 0
                                      ? height_by_plate[plate_index_valid]
                                      : 0.;
        double volume = wipe_volume * (extruder_count == 2 ? extruder_size : (extruder_size - 1));
        if (extruder_count == 2)
            volume += filament_change_volume * static_cast<int>(extruder_size / 2);
        double depth;
        if (use_rib_wall) {
            depth = std::sqrt(volume / layer_height * extra_spacing);
            if (need_min_depth || extruder_size > 1) {
                const float min_depth = Slic3r::WipeTower::get_limit_depth_by_height(static_cast<float>(max_height));
                const double volume_depth = depth;
                depth = std::max<double>(min_depth, depth);
                rib_width = std::min(rib_width, depth / 2);
                depth = rib_width / std::sqrt(2.) + std::max(depth + config.opt_float("wipe_tower_extra_rib_length"), volume_depth);
            }
        } else {
            depth = volume / (layer_height * width) * extra_spacing;
            if (need_min_depth || depth > EPSILON) {
                const float min_depth = Slic3r::WipeTower::get_limit_depth_by_height(static_cast<float>(max_height));
                depth = std::max<double>(min_depth, depth);
            }
        }

        // estimate_wipe_tower_polygon : position clampée dans le plateau.
        float x = tower_x->get_at(bedid);
        float y = tower_y->get_at(bedid);
        float wp_brim_width = tower_brim;
        if (wp_brim_width < 0)
            wp_brim_width = Slic3r::WipeTower::get_auto_brim_by_height(static_cast<float>(max_height));
        x = std::clamp(x, margin, static_cast<float>(plate_width) - static_cast<float>(width) - margin - wp_brim_width);
        y = std::clamp(y, margin, static_cast<float>(plate_depth) - static_cast<float>(depth) - margin - wp_brim_width);

        ArrangePolygon wipe_tower_ap;
        wipe_tower_ap.poly.contour = Polygon({
            {scaled(x - wp_brim_width), scaled(y - wp_brim_width)},
            {scaled(x + width + wp_brim_width), scaled(y - wp_brim_width)},
            {scaled(x + width + wp_brim_width), scaled(y + depth + wp_brim_width)},
            {scaled(x - wp_brim_width), scaled(y + depth + wp_brim_width)},
        });
        wipe_tower_ap.bed_idx = bedid_unlocked;
        wipe_tower_ap.setter = nullptr; // ne pas déplacer la tour
        wipe_tower_ap.translation = Vec2crd(scaled(0.f), scaled(0.f));
        wipe_tower_ap.name = "WipeTower";
        wipe_tower_ap.is_virt_object = true;
        wipe_tower_ap.is_wipe_tower = true;
        unselected.emplace_back(std::move(wipe_tower_ap));
        ++bedid_unlocked;
    }
}

Slic3r::Points take_polygon(const rust::Vec<double> &xy, size_t &cursor)
{
    Slic3r::Points points;
    while (cursor + 1 < xy.size()) {
        const double x = xy[cursor];
        const double y = xy[cursor + 1];
        cursor += 2;
        // Séparateur du protocole interne Rust : les coordonnées réelles sont
        // validées finies avant l'appel, seul [NaN, NaN] termine un contour.
        if (std::isnan(x) && std::isnan(y))
            break;
        if (!std::isfinite(x) || !std::isfinite(y))
            throw std::runtime_error("coordonnée de plateau non finie");
        points.emplace_back(Slic3r::scaled(x), Slic3r::scaled(y));
    }
    return points;
}

} // namespace

rust::Vec<RawObject> arrange_raw(const rust::Vec<RawObject> &objects,
                                 const rust::Vec<double> &bed_xy, double clearance_mm,
                                 bool allow_rotations, bool orca_cli_mode)
{
    if (bed_xy.size() < 6 || bed_xy.size() % 2 != 0 ||
        !std::isfinite(clearance_mm) || clearance_mm < 0.)
        throw std::runtime_error("paramètres d'arrangement invalides");

    size_t cursor = 0;
    Slic3r::Points bed = take_polygon(bed_xy, cursor);
    if (bed.size() < 3)
        throw std::runtime_error("contour de plateau invalide");

    Slic3r::arrangement::ArrangeParams params(Slic3r::scaled(clearance_mm));
    // Le bridge n'a pas de progression à remonter : ne pas polluer stdout du
    // worker avec le callback par défaut d'Orca.
    params.progressind = [](unsigned, std::string) {};
    // Mode CLI (OrcaSlicer.cpp, branche `need_arrange`) : `arrange_cfg =
    // ArrangeParams()` garde `do_final_align = true` ; finalAlign (nfpplacer)
    // cherche alors, dans le NFP des zones exclues, la position de la pile la
    // plus proche du centre du plateau (ex. cube placé à gauche d'une zone
    // centrale au lieu d'être refusé).
    // GUI Orca (ArrangeJob.cpp:777, ArrangeParams() : do_final_align = true,
    // align_center = best_object_pos = 0,5×0,5) : la pile est recentrée sur le
    // plateau. Sans recentrage, le nester laisse les objets dans le coin où il
    // les a empilés (F9 : « coin arrière droit »). Avec zones exclues (Bambu :
    // bed_exclude_area), la GUI garde aussi l'alignement (seul FillBedJob le
    // coupe) : les zones y sont des objets virtuels gonflés, exactement comme
    // dans le CLI (PartPlateList::preprocess_exclude_areas, appelé par
    // ArrangeJob::prepare sur m_unselected puis sur params.excluded_regions
    // avec scale_(1)) — finalAlign cherche alors la position de la pile la
    // plus proche du centre hors des zones (lot 25 : la pile restait dans le
    // coin avant gauche d'une P1S).
    params.do_final_align = true;
    // Comme le CLI Orca (`arrange_cfg.allow_rotations`) : rotation de la boîte
    // englobante minimale de l'enveloppe convexe, puis essais à 45° près.
    params.allow_rotations = allow_rotations;
    Slic3r::arrangement::ArrangePolygons excluded_regions;
    while (cursor < bed_xy.size()) {
        Slic3r::Points points = take_polygon(bed_xy, cursor);
        if (points.size() < 3)
            throw std::runtime_error("zone d'exclusion invalide");
        Slic3r::arrangement::ArrangePolygon excluded;
        // PartPlate::m_exclude_bounding_box : Orca arrange contre la boîte
        // englobante de chaque zone (identique pour les zones rectangulaires
        // des Bambu, ex. P1S 0,0 18,0 18,28 0,28).
        excluded.poly = Slic3r::ExPolygon(Slic3r::BoundingBox(points).polygon());
        excluded.bed_idx = 0;
        // PartPlateList::preprocess_exclude_areas (CLI et GUI ArrangeJob) :
        // objet virtuel, hauteur 1 ; gonflé de 1 mm, aussi bien dans
        // `unselected` (update_unselected_items_inflation, exclusion_gap = 1
        // hors impression séquentielle) que dans `excluded_regions`
        // (preprocess_exclude_areas(…, scale_(1))).
        excluded.is_virt_object = true;
        excluded.height = 1;
        excluded.name = "ExcludedRegion" + std::to_string(excluded_regions.size());
        excluded.inflation = Slic3r::scaled(1.0);
        excluded_regions.emplace_back(std::move(excluded));
    }
    // `orca_cli_mode` ne change plus la prise en compte des zones exclues
    // (identique CLI / GUI) ; conservé dans le protocole.
    (void)orca_cli_mode;

    Slic3r::Model model;
    raw_into_model(objects, model);
    Slic3r::ModelInstancePtrs instances;
    Slic3r::arrangement::ArrangePolygons items =
        Slic3r::get_arrange_polys(model, instances);
    // Le second argument précharge les obstacles dans le nester ; la copie dans
    // `params.excluded_regions` les prend aussi en compte pendant l'alignement
    // final (deux mécanismes distincts dans Arrange.cpp).
    params.excluded_regions = excluded_regions;
    // Comme le CLI Orca (update_selected_items_inflation, OrcaSlicer.cpp
    // ~l.4783) : _arrange remet min_obj_distance à 0 (« items are already
    // inflated ») — l'écart passe par l'inflation de chaque item : demi-
    // distance si fournie, sinon brim_width par défaut de get_arrange_poly
    // (1 mm sans support). min_area_boundingbox_rotation travaille sur la
    // forme gonflée : sans cette inflation, l'angle diffère d'Orca au 1e-5°
    // près (cylindre golden −137,8124891° au lieu de −137,8124525°, mesuré
    // par engine/tests/replica et la matrice d'instance du 3MF du CLI).
    {
        const coord_t brim = Slic3r::scaled(1.0);
        const Slic3r::BoundingBox bed_bb = Slic3r::Polygon(bed).bounding_box();
        for (Slic3r::arrangement::ArrangePolygon &item : items) {
            item.inflation = params.min_obj_distance != 0 ? params.min_obj_distance / 2 : brim;
            const Slic3r::BoundingBox item_bb = item.poly.contour.bounding_box();
            const auto diffx = bed_bb.size().x() - item_bb.size().x() - 5;
            const auto diffy = bed_bb.size().y() - item_bb.size().y() - 5;
            if (diffx > 0 && diffy > 0)
                item.inflation = std::min<coord_t>(std::min(diffx, diffy) / 2, item.inflation);
        }
    }
    Slic3r::arrangement::arrange(items, excluded_regions, bed, params);
    if (!Slic3r::apply_arrange_polys(items, instances,
                                     [](Slic3r::arrangement::ArrangePolygon &) {
                                         throw std::runtime_error(
                                             "objet(s) hors du plateau après arrangement");
                                     }))
        throw std::runtime_error("échec de l'arrangement");

    return model_to_raw(model);
}

// Arrangement du CLI Orca avec la config d'impression (OrcaSlicer.cpp, branche
// need_arrange, l. 4950-5240) : items get_instance_arrange_poly(instance,
// config) (brim, extrudeurs, températures plateau), ArrangeParams() remis à
// zéro, is_seq_print = print_sequence « by object » (ou forcé), dégagements
// extrudeur, printable_height, bed_shrink = BED_SHRINK_SEQ_PRINT en
// séquentiel, puis update_arrange_params / update_selected_items_inflation /
// update_unselected_items_inflation / update_selected_items_axis_align et
// plateau get_shrink_bedpts. Les zones exclues (après le contour dans bed_xy)
// sont des objets virtuels gonflés comme preprocess_exclude_areas. Renvoie
// aussi ModelInstance::arrange_order (= itemid après placement), ordre
// d'impression séquentielle.
RawArrangeResult arrange_orca_raw(const rust::Vec<RawObject> &objects, const rust::Vec<double> &bed_xy,
                                  rust::Str config_json, bool force_seq_print, bool allow_rotations)
{
    using namespace Slic3r;
    using namespace Slic3r::arrangement;
    size_t cursor = 0;
    Points bed = take_polygon(bed_xy, cursor);
    if (bed.size() < 3)
        throw std::runtime_error("contour de plateau invalide");

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
    resize_filament_map(config);
    // Contour du plateau demandé = printable_area (lu par get_shrink_bedpts).
    {
        Pointfs area;
        for (const Point &p : bed)
            area.emplace_back(unscaled(p.x()), unscaled(p.y()));
        config.set_key_value("printable_area", new ConfigOptionPoints(area));
    }

    ArrangePolygons unselected;
    while (cursor < bed_xy.size()) {
        Points points = take_polygon(bed_xy, cursor);
        if (points.size() < 3)
            throw std::runtime_error("zone d'exclusion invalide");
        ArrangePolygon excluded;
        excluded.poly = ExPolygon(std::move(points));
        excluded.bed_idx = 0;
        excluded.is_virt_object = true;
        excluded.height = 1;
        excluded.name = "ExcludedRegion" + std::to_string(unselected.size());
        unselected.emplace_back(std::move(excluded));
    }

    Model model;
    raw_into_model(objects, model);
    ArrangePolygons selected;
    for (ModelObject *mo : model.objects)
        for (ModelInstance *minst : mo->instances) {
            ArrangePolygon ap = get_instance_arrange_poly(minst, config);
            ap.itemid = static_cast<int>(selected.size());
            selected.emplace_back(std::move(ap));
        }

    ArrangeParams params;
    params.is_seq_print = force_seq_print;
    if (config.has("print_sequence") &&
        config.option<ConfigOptionEnum<PrintSequence>>("print_sequence")->value == PrintSequence::ByObject)
        params.is_seq_print = true;
    params.allow_rotations = allow_rotations;
    params.clearance_height_to_rod = config.opt_float("extruder_clearance_height_to_rod");
    params.clearance_height_to_lid = config.opt_float("extruder_clearance_height_to_lid");
    params.clearance_radius = config.opt_float("extruder_clearance_radius");
    params.printable_height = config.opt_float("printable_height");
    params.min_obj_distance = 0;
    if (params.is_seq_print) {
        params.bed_shrink_x = BED_SHRINK_SEQ_PRINT;
        params.bed_shrink_y = BED_SHRINK_SEQ_PRINT;
    }
    if (auto *structure = config.option<ConfigOptionEnum<PrinterStructure>>("printer_structure"))
        params.align_to_y_axis = structure->value == PrinterStructure::psI3;
    params.progressind = [](unsigned, std::string) {};

    update_arrange_params(params, &config, selected);
    update_selected_items_inflation(selected, &config, params);
    update_unselected_items_inflation(unselected, &config, params);
    update_selected_items_axis_align(selected, &config, params);
    const Points beds = get_shrink_bedpts(&config, params);
    // preprocess_exclude_areas(arrange_cfg.excluded_regions, …, scale_(1)).
    for (const ArrangePolygon &u : unselected) {
        ArrangePolygon region = u;
        region.inflation = scaled(1.0);
        params.excluded_regions.emplace_back(std::move(region));
    }

    arrange(selected, unselected, beds, params);

    RawArrangeResult out;
    for (ArrangePolygon &ap : selected) {
        if (ap.bed_idx != 0)
            throw std::runtime_error("objet(s) hors du plateau après arrangement");
        ap.apply();
    }
    out.objects = model_to_raw(model);
    for (const ModelObject *mo : model.objects)
        for (const ModelInstance *minst : mo->instances)
            out.arrange_order.push_back(minst->arrange_order);
    out.is_seq_print = params.is_seq_print;
    return out;
}


// Arrangement de la GUI d'Orca (GUI/Jobs/ArrangeJob.cpp : init_arrange_params,
// prepare_all / prepare_partplate, check_unprintable, process, finalize) avec
// les ArrangeSettings du menu « Options d'arrangement »
// (GLCanvas3D::_render_arrange_menu) : espacement (0 = auto), rotation
// automatique, plusieurs matériaux par plateau, zone de calibration
// d'extrusion, alignement sur Y. Plateaux : `plate_of_instance` (ordre
// objets/instances) ; `settings.plate_scope` < 0 = « Tout arranger »
// (PREPARE_STATE_DEFAULT), sinon « Arranger ce plateau » (PREPARE_STATE_MENU).
// Les plateaux verrouillés (`locked_plates`, prepare_all : print_sequence du
// plateau ≠ global) gardent leurs instances. Sortie : plateau final et
// position locale de chaque instance (PartPlateList::postprocess_* sans la
// translation de grille), drapeau « non arrangé » (bed_idx = -1, posé hors
// plateau comme postprocess_arrange_polygon) et nombre de plateaux après
// création (create_plate jusqu'à MAX_PLATES_COUNT = 36).
// Écart documenté : la tour d'essuyage (prepare_wipe_tower,
// estimate_wipe_tower_polygon : code GUI de PartPlate) n'est pas réservée.
RawGuiArrangeResult arrange_gui_raw(const rust::Vec<RawObject> &objects, const rust::Vec<double> &bed_xy,
                                    rust::Str config_json, const RawGuiArrangeSettings &settings,
                                    const rust::Vec<uint32_t> &plate_of_instance,
                                    const rust::Vec<uint32_t> &locked_plates)
{
    using namespace Slic3r;
    using namespace Slic3r::arrangement;
    constexpr int max_plates = 36; // MAX_NUM_PLATES (libnest2d) = MAX_PLATES_COUNT
    size_t cursor = 0;
    Points bed = take_polygon(bed_xy, cursor);
    if (bed.size() < 3)
        throw std::runtime_error("contour de plateau invalide");
    if (!std::isfinite(settings.distance) || settings.distance < 0.)
        throw std::runtime_error("espacement d'arrangement invalide");

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
    resize_filament_map(config);
    {
        Pointfs area;
        for (const Point &p : bed)
            area.emplace_back(unscaled(p.x()), unscaled(p.y()));
        config.set_key_value("printable_area", new ConfigOptionPoints(area));
    }
    // Zones exclues : boîtes englobantes (PartPlate::m_exclude_bounding_box).
    std::vector<Polygon> exclude_boxes;
    while (cursor < bed_xy.size()) {
        Points points = take_polygon(bed_xy, cursor);
        if (points.size() < 3)
            throw std::runtime_error("zone d'exclusion invalide");
        exclude_boxes.push_back(BoundingBox(points).polygon());
    }
    Polygon wrapping_area;
    if (const auto *wrap = config.option<ConfigOptionPoints>("wrapping_exclude_area"))
        for (const Vec2d &p : wrap->values)
            wrapping_area.append({scaled(p.x()), scaled(p.y())});
    const bool enable_wrapping = config.has("enable_wrapping_detection") &&
                                 config.opt_bool("enable_wrapping_detection");
    // PartPlateList::preprocess_exclude_areas.
    auto add_exclude_areas = [&](ArrangePolygons &out, int num_plates, coord_t inflation) {
        if (enable_wrapping && !wrapping_area.empty())
            for (int j = 0; j < num_plates; ++j) {
                ArrangePolygon ret;
                ret.poly.contour = wrapping_area;
                ret.translation = Vec2crd(0, 0);
                ret.rotation = 0.0f;
                ret.is_virt_object = true;
                ret.bed_idx = j;
                ret.height = 1;
                ret.name = "WrappingRegion";
                ret.inflation = inflation;
                out.emplace_back(std::move(ret));
            }
        for (size_t index = 0; index < exclude_boxes.size(); ++index)
            for (int j = 0; j < num_plates; ++j) {
                ArrangePolygon ret;
                ret.poly.contour = exclude_boxes[index];
                ret.translation = Vec2crd(0, 0);
                ret.rotation = 0.0f;
                ret.is_virt_object = true;
                ret.bed_idx = j;
                ret.height = 1;
                ret.name = "ExcludedRegion" + std::to_string(index);
                ret.inflation = inflation;
                out.emplace_back(std::move(ret));
            }
    };

    Model model;
    raw_into_model(objects, model);
    std::vector<ModelInstance *> flat;
    for (ModelObject *mo : model.objects)
        for (ModelInstance *mi : mo->instances)
            flat.push_back(mi);
    if (plate_of_instance.size() != flat.size())
        throw std::runtime_error("plateaux d'instances incohérents");
    const std::set<uint32_t> locked(locked_plates.begin(), locked_plates.end());
    const bool only_on_partplate = settings.plate_scope >= 0;
    const int current_plate = only_on_partplate ? settings.plate_scope : static_cast<int>(settings.current_plate);
    int plate_count = std::max<int>(1, static_cast<int>(settings.plate_count));

    // ArrangeJob::prepare : paramètres extrudeur et table de vitesses.
    PrintConfig print_config;
    print_config.apply(config, true);
    {
        int filaments = 1;
        if (const auto *d = config.option<ConfigOptionFloats>("filament_diameter"))
            filaments = std::max<int>(1, static_cast<int>(d->values.size()));
        Model::setExtruderParams(config, filaments);
        Model::setPrintSpeedTable(config, print_config);
    }

    // init_arrange_params : Print::object_skirt_offset du plateau courant.
    float object_skirt_offset = 0.f;
    try {
        Model plate_model;
        for (size_t o = 0, k = 0; o < model.objects.size(); ++o) {
            ModelObject *copy = nullptr;
            for (size_t i = 0; i < model.objects[o]->instances.size(); ++i, ++k) {
                if (static_cast<int>(plate_of_instance[k]) != current_plate)
                    continue;
                if (copy == nullptr) {
                    copy = plate_model.add_object(*model.objects[o]);
                    copy->clear_instances();
                }
                copy->add_instance(*model.objects[o]->instances[i]);
            }
        }
        if (!plate_model.objects.empty()) {
            Print print;
            DynamicPrintConfig print_cfg = config;
            print_cfg.normalize_fdm();
            print.apply(plate_model, print_cfg);
            object_skirt_offset = std::get<0>(print.object_skirt_offset());
        }
    } catch (const std::exception &) {
        object_skirt_offset = 0.f;
    }

    ArrangeParams params;
    params.clearance_height_to_rod = config.opt_float("extruder_clearance_height_to_rod");
    params.clearance_height_to_lid = config.opt_float("extruder_clearance_height_to_lid");
    params.clearance_radius = config.opt_float("extruder_clearance_radius") + object_skirt_offset * 2;
    params.object_skirt_offset = object_skirt_offset;
    params.printable_height = config.opt_float("printable_height");
    params.allow_rotations = settings.enable_rotation;
    params.nozzle_height = config.opt_float("nozzle_height");
    if (const auto *pos = config.option<ConfigOptionPoint>("best_object_pos"))
        params.align_center = pos->value;
    params.allow_multi_materials_on_same_plate = settings.allow_multi_materials_on_same_plate;
    params.avoid_extrusion_cali_region = settings.avoid_extrusion_cali_region;
    params.is_seq_print = settings.is_seq_print;
    params.min_obj_distance = scaled(settings.distance);
    params.align_to_y_axis = settings.align_to_y_axis;
    if (only_on_partplate && settings.plate_seq_differs)
        params.min_obj_distance = 0;
    if (params.is_seq_print) {
        params.bed_shrink_x = BED_SHRINK_SEQ_PRINT;
        params.bed_shrink_y = BED_SHRINK_SEQ_PRINT;
    }
    params.progressind = [](unsigned, std::string) {};

    // prepare_all / prepare_partplate.
    // Plateau final et drapeau « non arrangé » par instance, relevés par le
    // setter (l'arrangeur réécrit itemid avec l'ordre de placement).
    std::vector<int32_t> bed_of(flat.size(), 0);
    std::vector<bool> unarranged(flat.size(), false);
    for (size_t k = 0; k < flat.size(); ++k)
        bed_of[k] = static_cast<int32_t>(plate_of_instance[k]);
    bool pending_unarranged = false;
    ArrangePolygons selected, unselected, unprintable;
    std::vector<int> locked_bed; // plateau des instances verrouillées
    for (size_t k = 0; k < flat.size(); ++k) {
        const int plate = static_cast<int>(plate_of_instance[k]);
        ArrangePolygon ap = get_instance_arrange_poly(flat[k], config);
        ap.name = flat[k]->get_object()->name;
        ap.setter = [setter = ap.setter, k, &bed_of, &unarranged, &pending_unarranged](const ArrangePolygon &placed) {
            if (setter)
                setter(placed);
            bed_of[k] = placed.bed_idx;
            if (pending_unarranged)
                unarranged[k] = true;
        };
        const bool is_locked = only_on_partplate ? plate != current_plate : locked.count(plate) > 0;
        if (is_locked) {
            locked_bed.push_back(plate);
            continue;
        }
        if (flat[k]->printable) {
            ap.itemid = static_cast<int>(selected.size());
            selected.emplace_back(std::move(ap));
        } else {
            ap.itemid = static_cast<int>(unprintable.size());
            unprintable.emplace_back(std::move(ap));
        }
    }

    // ArrangeJob::prepare_wipe_tower (prepare_selected/prepare_all ; la
    // portée « ce plateau » de prepare_partplate n'estime pas de tour) :
    // besoin = timelapse fluide, objet multi-extrudeurs (peinture/support),
    // ou plusieurs matériaux de même température de plateau autorisés
    // ensemble. La tour est posée comme obstacle fixe sur chaque plateau
    // potentiel (voir prepare_wipe_towers).
    if (!only_on_partplate) {
        const bool enable_prime_tower =
            config.has("enable_prime_tower") && config.opt_bool("enable_prime_tower");
        bool need_wipe_tower = false;
        bool smooth_timelapse = false;
        if (const auto *timelapse = config.option<ConfigOptionEnum<TimelapseType>>("timelapse_type"))
            smooth_timelapse = timelapse->value == TimelapseType::tlSmooth;
        if (smooth_timelapse)
            need_wipe_tower = true;
        if (!need_wipe_tower)
            for (const ArrangePolygon &item : selected) {
                std::set<int> obj_extruders(item.extrude_ids.begin(), item.extrude_ids.end());
                if (obj_extruders.size() > 1) {
                    need_wipe_tower = true;
                    break;
                }
            }
        if (!need_wipe_tower && params.allow_multi_materials_on_same_plate) {
            std::map<int, std::set<int>> bedTemp2extruderIds;
            for (const ArrangePolygon &item : selected)
                for (auto id : item.extrude_ids)
                    bedTemp2extruderIds[item.bed_temp].insert(id);
            for (const auto &be : bedTemp2extruderIds)
                if (be.second.size() > 1) {
                    need_wipe_tower = true;
                    break;
                }
        }
        if (enable_prime_tower && !params.is_seq_print && need_wipe_tower)
            prepare_wipe_towers(unselected, model, config, bed, plate_of_instance, flat, locked,
                                plate_count, smooth_timelapse || enable_wrapping);
    }
    add_exclude_areas(unselected, only_on_partplate ? current_plate + 1 : max_plates, 0);

    // check_unprintable : aire nulle ou plus haut que printable_height.
    for (auto it = selected.begin(); it != selected.end();) {
        if (it->poly.area() < 0.001 || it->height > params.printable_height) {
            unprintable.push_back(*it);
            it = selected.erase(it);
        } else
            ++it;
    }

    // process.
    const bool scan_first_layer = config.has("scan_first_layer") && config.opt_bool("scan_first_layer");
    if (settings.is_bbl_vendor && params.avoid_extrusion_cali_region && scan_first_layer) {
        // PartPlateList::preprocess_nonprefered_areas (inflation 0).
        const Polygon ap = scaled(BoundingBoxf(Vec2d{18, 0}, Vec2d{240, 15})).polygon();
        for (int j = 0; j < max_plates; ++j) {
            ArrangePolygon ret;
            ret.poly.contour = ap;
            ret.translation = Vec2crd(0, 0);
            ret.rotation = 0.0f;
            ret.is_virt_object = true;
            ret.is_extrusion_cali_object = true;
            ret.bed_idx = j;
            ret.height = 1;
            ret.name = "NonpreferedRegion0";
            ret.inflation = 0;
            unselected.emplace_back(std::move(ret));
        }
    }
    update_arrange_params(params, &config, selected);
    update_selected_items_inflation(selected, &config, params);
    update_unselected_items_inflation(unselected, &config, params);
    update_selected_items_axis_align(selected, &config, params);
    const Points bedpts = get_shrink_bedpts(&config, params);
    add_exclude_areas(params.excluded_regions, 1, scaled(1.));

    if (!selected.empty())
        arrange(selected, unselected, bedpts, params);
    std::sort(selected.begin(), selected.end(), [](const auto &a, const auto &b) { return a.itemid < b.itemid; });

    // finalize.
    RawGuiArrangeResult out;
    int beds = 0;
    for (ArrangePolygon &ap : selected) {
        if (only_on_partplate) {
            // postprocess_bed_index_for_current_plate.
            if (ap.bed_idx == 0)
                ap.bed_idx += current_plate;
            else if (ap.bed_idx != -1)
                ap.bed_idx = plate_count;
        } else if (ap.bed_idx != -1) {
            // postprocess_bed_index_for_selected.
            bool placed = false;
            for (int i = 0; i < plate_count; ++i) {
                if (locked.count(static_cast<uint32_t>(i)) > 0)
                    ap.bed_idx += 1;
                else if (ap.bed_idx <= i) {
                    placed = true;
                    break;
                }
            }
            if (!placed) {
                int plate_index = plate_count < max_plates ? plate_count++ : -1;
                while (plate_index != -1) {
                    if (ap.bed_idx <= plate_index)
                        break;
                    plate_index = plate_count < max_plates ? plate_count++ : -1;
                }
            }
        }
        beds = std::max(ap.bed_idx, beds);
    }
    for (int bed_of_locked : locked_bed)
        beds = std::max(bed_of_locked, beds);
    const BoundingBox bed_bb = Polygon(bed).bounding_box();
    for (ArrangePolygon &ap : selected) {
        pending_unarranged = ap.bed_idx == -1;
        if (ap.bed_idx == -1) {
            // postprocess_arrange_polygon : objet trop grand, posé sur un
            // plateau supplémentaire, coin arrière gauche.
            ap.bed_idx = plate_count;
            const BoundingBox apbox = get_extents(ap.transformed_poly());
            const auto size = apbox.size();
            ap.translation(X) = 0.5 * size[0];
            ap.translation(Y) = bed_bb.size().y() - 0.5 * size[1];
        }
        ap.apply();
    }
    pending_unarranged = false;
    for (ArrangePolygon &ap : unprintable) {
        ap.bed_idx = beds + 1;
        ap.apply();
    }
    int max_bed = plate_count - 1;
    for (int32_t b : bed_of)
        max_bed = std::max<int>(max_bed, b);
    for (size_t k = 0; k < flat.size(); ++k) {
        out.bed_idx.push_back(bed_of[k]);
        out.unarranged.push_back(unarranged[k]);
    }
    out.plate_count = static_cast<uint32_t>(std::min(max_bed + 1, max_plates));
    out.objects = model_to_raw(model);
    for (ModelInstance *mi : flat)
        out.arrange_order.push_back(mi->arrange_order);
    out.is_seq_print = params.is_seq_print;
    return out;
}

// « Remplir le plateau avec des instances » d'Orca (GUI/Jobs/FillBedJob.cpp,
// prepare/process/finalize) : instances imprimables de l'objet cible
// (priority + 1) + copies du gabarit (priority 0, autant que
// (aire plateau − aire fixe) / aire de l'item gonflé de min_obj_distance/2) ;
// autres instances posées sur le plateau = fixes ; zones exclues = objets
// virtuels ; update_arrange_params / get_shrink_bedpts / inflations ;
// arrêt dès qu'une copie déborde du plateau (on_packed). Les instances
// placées sur le plateau sont appliquées (originales déplacées, copies
// ajoutées à l'objet cible). Paramètres GUI repris d'init_arrange_params :
// dégagements extrudeur, printable_height, is_seq_print selon
// print_sequence, min_obj_distance = clearance_mm (0 = auto).
// Écart : au-delà de 100 items Orca pose les copies sur une grille de cases
// vides (Plater::get_empty_cells, code GUI) ; ici le nombre d'items est
// plafonné à 100.
rust::Vec<RawObject> fill_bed_raw(const rust::Vec<RawObject> &objects, const rust::Vec<double> &bed_xy,
                                  rust::Str config_json, uint32_t object_index, double clearance_mm)
{
    using namespace Slic3r;
    using namespace Slic3r::arrangement;
    size_t cursor = 0;
    Points bed = take_polygon(bed_xy, cursor);
    if (bed.size() < 3)
        throw std::runtime_error("contour de plateau invalide");

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
    resize_filament_map(config);
    {
        Pointfs area;
        for (const Point &p : bed)
            area.emplace_back(unscaled(p.x()), unscaled(p.y()));
        config.set_key_value("printable_area", new ConfigOptionPoints(area));
    }

    Model model;
    raw_into_model(objects, model);
    if (object_index >= model.objects.size() || model.objects[object_index]->instances.empty())
        throw std::runtime_error("objet à dupliquer introuvable");

    ArrangeParams params;
    params.clearance_height_to_rod = config.opt_float("extruder_clearance_height_to_rod");
    params.clearance_height_to_lid = config.opt_float("extruder_clearance_height_to_lid");
    params.clearance_radius = config.opt_float("extruder_clearance_radius");
    params.printable_height = config.opt_float("printable_height");
    params.nozzle_height = config.opt_float("nozzle_height");
    params.min_obj_distance = clearance_mm > 0 ? scaled(clearance_mm) : 0;
    params.is_seq_print = config.has("print_sequence") &&
                          config.option<ConfigOptionEnum<PrintSequence>>("print_sequence")->value ==
                              PrintSequence::ByObject;
    if (params.is_seq_print) {
        params.bed_shrink_x = BED_SHRINK_SEQ_PRINT;
        params.bed_shrink_y = BED_SHRINK_SEQ_PRINT;
    }
    params.progressind = [](unsigned, std::string) {};

    const BoundingBox plate_bb = get_extents(Polygon(bed));
    ArrangePolygons selected, unselected;
    std::vector<size_t> selected_instance; // indice d'instance de l'objet cible
    for (size_t oidx = 0; oidx < model.objects.size(); ++oidx) {
        ModelObject *mo = model.objects[oidx];
        for (size_t inst_idx = 0; inst_idx < mo->instances.size(); ++inst_idx) {
            ArrangePolygon ap = get_instance_arrange_poly(mo->instances[inst_idx], config);
            const BoundingBox ap_bb = ap.transformed_poly().contour.bounding_box();
            ap.name = mo->name;
            if (oidx == object_index && mo->instances[inst_idx]->printable) {
                ++ap.priority;
                ap.itemid = static_cast<int>(selected.size());
                selected.emplace_back(ap);
                selected_instance.push_back(inst_idx);
            } else if (plate_bb.contains(ap_bb)) {
                ap.bed_idx = 0;
                ap.itemid = static_cast<int>(unselected.size());
                unselected.emplace_back(ap);
            }
        }
    }
    if (selected.empty())
        throw std::runtime_error("aucune instance imprimable à dupliquer");

    // Zones exclues (après le contour dans bed_xy) : objets virtuels.
    while (cursor < bed_xy.size()) {
        Points points = take_polygon(bed_xy, cursor);
        if (points.size() < 3)
            throw std::runtime_error("zone d'exclusion invalide");
        ArrangePolygon excluded;
        excluded.poly = ExPolygon(std::move(points));
        excluded.bed_idx = 0;
        excluded.is_virt_object = true;
        excluded.height = 1;
        excluded.name = "ExcludedRegion" + std::to_string(unselected.size());
        ArrangePolygon region = excluded;
        region.inflation = scaled(1.0);
        params.excluded_regions.emplace_back(std::move(region));
        unselected.emplace_back(std::move(excluded));
    }

    const double sc = scaled<double>(1.) * scaled(1.);
    auto polys = offset_ex(selected.front().poly, params.min_obj_distance / 2);
    const ExPolygon poly = polys.empty() ? selected.front().poly : polys.front();
    const double poly_area = poly.area() / sc;
    double unsel_area = 0;
    for (const ArrangePolygon &ap : unselected)
        unsel_area += (ap.bed_idx == 0) * ap.poly.area();
    unsel_area /= sc;
    const double fixed_area = unsel_area + selected.size() * poly_area;
    const double bed_area = Polygon(bed).area() / sc;
    int needed_items = poly_area > 0 ? static_cast<int>((bed_area - fixed_area) / poly_area) : 0;
    needed_items = std::clamp(needed_items, 0, std::max(0, 100 - static_cast<int>(selected.size())));

    ModelObject *target = model.objects[object_index];
    const ArrangePolygon template_ap = get_instance_arrange_poly(target->instances[selected_instance.front()], config);
    for (int i = 0; i < needed_items; ++i) {
        ArrangePolygon ap = template_ap;
        ap.poly = selected.front().poly;
        ap.bed_idx = 36; // PartPlateList::MAX_PLATES_COUNT
        ap.itemid = -1;
        selected.emplace_back(ap);
    }

    update_arrange_params(params, &config, selected);
    const Points beds = get_shrink_bedpts(&config, params);
    update_selected_items_inflation(selected, &config, params);
    update_unselected_items_inflation(unselected, &config, params);
    bool do_stop = false;
    params.stopcondition = [&do_stop]() { return do_stop; };
    params.on_packed = [&do_stop](const ArrangePolygon &ap) { do_stop = ap.bed_idx > 0 && ap.priority == 0; };
    params.do_final_align = true; // !is_bbl (imprimantes non Bambu)

    arrange(selected, unselected, beds, params);

    const ModelInstance *tmpl = target->instances[selected_instance.front()];
    for (size_t k = 0; k < selected.size(); ++k) {
        const ArrangePolygon &ap = selected[k];
        if (ap.bed_idx != 0)
            continue;
        ModelInstance *inst = k < selected_instance.size() ? target->instances[selected_instance[k]]
                                                           : target->add_instance(*tmpl);
        inst->apply_arrange_result(ap.translation.cast<double>(), ap.rotation);
    }
    return model_to_raw(model);
}

RawObject orient_raw(const RawObject &object)
{
    Slic3r::Model model;
    rust::Vec<RawObject> wrapper;
    wrapper.push_back(object); // copie
    raw_into_model(wrapper, model);
    if (model.objects.empty())
        throw std::runtime_error("objet vide");
    Slic3r::orientation::orient(model.objects.front());
    rust::Vec<RawObject> out = model_to_raw(model);
    if (out.empty())
        throw std::runtime_error("orientation sans résultat");
    return std::move(out[0]);
}

} // namespace webslicer
