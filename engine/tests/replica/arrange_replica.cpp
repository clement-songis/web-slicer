// Réplique du bloc d'arrangement du CLI Orca (OrcaSlicer.cpp ~4680-4830,
// arrangement global plate_to_slice == 0) vs l'appel de bridge/arrange.cpp,
// sur le cylindre golden. Bissection par drapeaux en argv.
// Build : engine/tests/replica/build.sh (dans nix develop).
#include "libslic3r/Model.hpp"
#include "libslic3r/ModelArrange.hpp"
#include "libslic3r/Arrange.hpp"
#include "libslic3r/PrintConfig.hpp"
#include "libslic3r/Utils.hpp"
#include <cmath>
#include <cstdio>
#include <cstring>
#include <string>

namespace Slic3r { int print_config_static_initializer(); }
using namespace Slic3r;
using namespace Slic3r::arrangement;

static bool has(int argc, char **argv, const char *f) {
    for (int i = 3; i < argc; ++i) if (!std::strcmp(argv[i], f)) return true;
    return false;
}

int main(int argc, char **argv) {
    if (argc < 3) { std::fprintf(stderr, "usage: replica model.stl config.json [flags]\n"); return 2; }
    set_temporary_dir("/tmp"); set_data_dir("/tmp");
    print_config_static_initializer();

    DynamicPrintConfig cfg = DynamicPrintConfig::full_print_config();
    {
        DynamicPrintConfig loaded; std::map<std::string, std::string> kv; std::string reason;
        loaded.load_from_json(argv[2], ForwardCompatibilitySubstitutionRule::Enable, kv, reason);
        cfg.apply(loaded, true);
    }
    Model model = Model::read_from_file(argv[1]);
    for (ModelObject *o : model.objects) o->ensure_on_bed();   // CLI l.1849
    model.add_default_instances();                             // CLI l.1908

    const bool bridge = has(argc, argv, "--bridge");
    ArrangePolygons selected, unselected;
    ModelInstancePtrs instances;
    ArrangeParams arrange_cfg;
    Points beds;
    if (!bridge) {
        // ---- réplique CLI ----
        arrange_cfg = ArrangeParams();
        for (ModelObject *mo : model.objects)
            for (ModelInstance *minst : mo->instances) {
                ArrangePolygon ap = get_instance_arrange_poly(minst, cfg);
                ap.itemid = selected.size();
                selected.emplace_back(ap);
                instances.push_back(minst);
            }
        if (cfg.has("print_sequence"))
            arrange_cfg.is_seq_print = cfg.option<ConfigOptionEnum<PrintSequence>>("print_sequence")->value == PrintSequence::ByObject;
        arrange_cfg.allow_rotations = true;
        arrange_cfg.min_obj_distance = 0;
        if (auto *ps = cfg.option<ConfigOptionEnum<PrinterStructure>>("printer_structure"))
            arrange_cfg.align_to_y_axis = (ps->value == PrinterStructure::psI3);
        if (has(argc, argv, "--no-shrink")) { arrange_cfg.bed_shrink_x = 0; arrange_cfg.bed_shrink_y = 0; }
        if (!has(argc, argv, "--no-update-params")) update_arrange_params(arrange_cfg, &cfg, selected);
        if (!has(argc, argv, "--no-inflation")) update_selected_items_inflation(selected, &cfg, arrange_cfg);
        update_unselected_items_inflation(unselected, &cfg, arrange_cfg);
        update_selected_items_axis_align(selected, &cfg, arrange_cfg);
        beds = get_shrink_bedpts(&cfg, arrange_cfg);
        if (has(argc, argv, "--no-final-align")) arrange_cfg.do_final_align = false;
        if (has(argc, argv, "--bridge-polys")) {
            selected = get_arrange_polys(model, instances);
        }
    } else {
        // ---- appel du bridge (arrange.cpp) ----
        for (const Vec2d &p : cfg.option<ConfigOptionPoints>("printable_area")->values)
            beds.emplace_back(scaled(p.x()), scaled(p.y()));
        arrange_cfg = ArrangeParams(scaled(0.));
        arrange_cfg.do_final_align = false;
        arrange_cfg.allow_rotations = true;
        selected = get_arrange_polys(model, instances);
    }
    arrange_cfg.progressind = [](unsigned, std::string) {};
    std::printf("bed_shrink=%g,%g final_align=%d bed0=(%lld,%lld) bed2=(%lld,%lld)\n",
                arrange_cfg.bed_shrink_x, arrange_cfg.bed_shrink_y, int(arrange_cfg.do_final_align),
                (long long)beds[0].x(), (long long)beds[0].y(), (long long)beds[2].x(), (long long)beds[2].y());
    for (auto &ap : selected)
        std::printf("item inflation=%lld trans=(%lld,%lld) rot=%g pts=%zu\n", (long long)ap.inflation,
                    (long long)ap.translation.x(), (long long)ap.translation.y(), ap.rotation, ap.poly.contour.size());
    arrange(selected, unselected, beds, arrange_cfg);
    for (size_t i = 0; i < selected.size(); ++i) {
        const ArrangePolygon &ap = selected[i];
        std::printf("result bed=%d trans=(%lld,%lld) rot_deg=%.10f\n", ap.bed_idx,
                    (long long)ap.translation.x(), (long long)ap.translation.y(), ap.rotation * 180. / M_PI);
    }
    return 0;
}
