// Wrappers arrangement automatique + orientation automatique (T016, FR-013).
#include "engine/src/adapters/ffi/bridge/model.hpp"
#include "engine/src/adapters/ffi/bridge.rs.h"

#include "libslic3r/Model.hpp"
#include "libslic3r/ModelArrange.hpp"
#include "libslic3r/Orient.hpp"

#include <stdexcept>

namespace webslicer {

// défini dans model.cpp / project.cpp
rust::Vec<RawObject> model_to_raw(const Slic3r::Model &model);
void raw_into_model(const rust::Vec<RawObject> &objects, Slic3r::Model &model);

rust::Vec<RawObject> arrange_raw(const rust::Vec<RawObject> &objects,
                                 const rust::Vec<double> &bed_xy, double clearance_mm)
{
    if (bed_xy.size() < 6 || bed_xy.size() % 2 != 0)
        throw std::runtime_error("contour de plateau invalide");

    Slic3r::Model model;
    raw_into_model(objects, model);

    Slic3r::Points bed;
    bed.reserve(bed_xy.size() / 2);
    for (size_t i = 0; i + 1 < bed_xy.size(); i += 2)
        bed.emplace_back(Slic3r::scaled(bed_xy[i]), Slic3r::scaled(bed_xy[i + 1]));

    Slic3r::arrangement::ArrangeParams params(
        Slic3r::scaled(clearance_mm));

    Slic3r::ModelInstancePtrs instances;
    Slic3r::arrangement::ArrangePolygons items =
        Slic3r::get_arrange_polys(model, instances);
    Slic3r::arrangement::arrange(items, {}, bed, params);
    if (!Slic3r::apply_arrange_polys(items, instances,
                                              [](Slic3r::arrangement::ArrangePolygon &) {
                                                  throw std::runtime_error(
                                                      "objet(s) hors du plateau après arrangement");
                                              }))
        throw std::runtime_error("échec de l'arrangement");

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
