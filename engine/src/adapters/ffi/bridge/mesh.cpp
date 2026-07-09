// Wrapper réparation de maillage (T015) : admesh via TriangleMesh::from_stl.
#include "engine/src/adapters/ffi/bridge/model.hpp"
#include "engine/src/adapters/ffi/bridge.rs.h"

#include "libslic3r/TriangleMesh.hpp"

#include <admesh/stl.h>

#include <cstring>
#include <stdexcept>

namespace webslicer {

RawRepairResult repair_mesh_raw(const RawMesh &raw)
{
    const size_t facet_count = raw.indices.size() / 3;
    if (facet_count == 0)
        throw std::runtime_error("maillage vide");

    stl_file stl;
    stl.stats.type = inmemory;
    stl.stats.number_of_facets = static_cast<uint32_t>(facet_count);
    stl.stats.original_num_facets = static_cast<int>(facet_count);
    stl_allocate(&stl);

    auto vertex = [&raw](uint32_t index) {
        return stl_vertex(raw.vertices[index * 3], raw.vertices[index * 3 + 1],
                          raw.vertices[index * 3 + 2]);
    };
    for (size_t i = 0; i < facet_count; ++i) {
        stl_facet &facet = stl.facet_start[i];
        facet.vertex[0] = vertex(raw.indices[i * 3]);
        facet.vertex[1] = vertex(raw.indices[i * 3 + 1]);
        facet.vertex[2] = vertex(raw.indices[i * 3 + 2]);
        facet.normal = stl_normal(0.f, 0.f, 0.f);
        std::memset(&facet.extra, 0, sizeof(facet.extra));
    }

    Slic3r::TriangleMesh mesh;
    if (!mesh.from_stl(stl, /*repair=*/true))
        throw std::runtime_error("échec de la réparation admesh");

    // BBS a désactivé (#if 0) la copie des compteurs vers TriangleMesh :
    // ils se lisent dans stl.stats après réparation.
    RawRepairResult result;
    result.edges_fixed = stl.stats.edges_fixed;
    result.degenerate_facets = stl.stats.degenerate_facets;
    result.facets_removed = stl.stats.facets_removed;
    result.facets_reversed = stl.stats.facets_reversed;
    result.backwards_edges = stl.stats.backwards_edges;

    const auto &its = mesh.its;
    result.mesh.vertices.reserve(its.vertices.size() * 3);
    for (const auto &v : its.vertices) {
        result.mesh.vertices.push_back(v.x());
        result.mesh.vertices.push_back(v.y());
        result.mesh.vertices.push_back(v.z());
    }
    result.mesh.indices.reserve(its.indices.size() * 3);
    for (const auto &f : its.indices) {
        result.mesh.indices.push_back(static_cast<uint32_t>(f.x()));
        result.mesh.indices.push_back(static_cast<uint32_t>(f.y()));
        result.mesh.indices.push_back(static_cast<uint32_t>(f.z()));
    }
    return result;
}

} // namespace webslicer
