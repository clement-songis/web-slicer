// Implémentation des wrappers — un .cpp par domaine (R1) ; celui-ci : modèle.
#include "engine/src/adapters/ffi/bridge/model.hpp"

#include "libslic3r/Model.hpp"
#include "libslic3r/PrintConfig.hpp"
#include "libslic3r/TriangleMesh.hpp"

#include <stdexcept>
#include <string>

namespace webslicer {

size_t model_triangle_count(rust::Str path)
{
    const std::string file(path);
    Slic3r::Model model = Slic3r::Model::read_from_file(file);
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
