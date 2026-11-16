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

namespace {

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
    // Arrangement UI : l'alignement final d'Orca ne déplace pas les obstacles
    // préchargés et peut ainsi réintroduire un chevauchement ; conserver les
    // positions effectivement validées par le nester.
    // Mode CLI (OrcaSlicer.cpp, branche `need_arrange`) : `arrange_cfg =
    // ArrangeParams()` garde `do_final_align = true` ; finalAlign (nfpplacer)
    // cherche alors, dans le NFP des zones exclues, la position de la pile la
    // plus proche du centre du plateau (ex. cube placé à gauche d'une zone
    // centrale au lieu d'être refusé).
    // GUI Orca (ArrangeJob.cpp:777, ArrangeParams() : do_final_align = true,
    // align_center = best_object_pos = 0,5×0,5) : la pile est recentrée sur le
    // plateau. Sans recentrage, le nester laisse les objets dans le coin où il
    // les a empilés (F9 : « coin arrière droit »). Exception conservée : en
    // mode UI avec zones exclues, l'alignement peut recouvrir un obstacle
    // (cf. ci-dessus) — ajusté après lecture des zones.
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
        excluded.poly = Slic3r::ExPolygon(std::move(points));
        excluded.bed_idx = 0;
        if (orca_cli_mode) {
            // PartPlateList::preprocess_exclude_areas : objet virtuel, hauteur
            // 1 ; gonflé de 1 mm, aussi bien dans `unselected`
            // (update_unselected_items_inflation, exclusion_gap = 1 hors
            // impression séquentielle) que dans `arrange_cfg.excluded_regions`
            // (preprocess_exclude_areas(…, scale_(1))).
            excluded.is_virt_object = true;
            excluded.height = 1;
            excluded.name = "ExcludedRegion" + std::to_string(excluded_regions.size());
            excluded.inflation = Slic3r::scaled(1.0);
        }
        excluded_regions.emplace_back(std::move(excluded));
    }

    if (!orca_cli_mode && !excluded_regions.empty())
        params.do_final_align = false;

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
