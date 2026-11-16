// Wrappers maillage : réparation admesh (T015), y compris rebouchage des
// trous pour l'opération utilisateur explicite « réparer », et simplification
// quadrique (`its_quadric_edge_collapse`, outil « Simplifier » d'Orca).
#include "engine/src/adapters/ffi/bridge/model.hpp"
#include "engine/src/adapters/ffi/bridge/mesh.hpp"
#include "engine/src/adapters/ffi/bridge.rs.h"

#include "libslic3r/Format/DRC.hpp"
#include "libslic3r/QuadricEdgeCollapse.hpp"
#include "libslic3r/TriangleMesh.hpp"

#include <admesh/stl.h>

#include <climits>
#include <cmath>
#include <cstdint>
#include <cstring>
#include <limits>
#include <stdexcept>

namespace webslicer {

namespace {

// Contrôles communs : entrée non vide, triplets complets, indices bornés,
// coordonnées finies (libslic3r n'en vérifie aucun).
void validate_raw_mesh(const RawMesh &raw)
{
    if (raw.indices.empty() || raw.indices.size() % 3 != 0 ||
        raw.vertices.empty() || raw.vertices.size() % 3 != 0)
        throw std::runtime_error("maillage vide ou indices incomplets");
    const size_t vertex_count = raw.vertices.size() / 3;
    for (size_t i = 0; i < raw.indices.size(); ++i) {
        if (raw.indices[i] >= vertex_count)
            throw std::runtime_error("indice de sommet hors limites");
    }
    for (size_t i = 0; i < raw.vertices.size(); ++i) {
        if (!std::isfinite(raw.vertices[i]))
            throw std::runtime_error("coordonnée de sommet non finie");
    }
}

void its_to_raw(const indexed_triangle_set &its, RawMesh &out)
{
    out.vertices.reserve(its.vertices.size() * 3);
    for (const auto &v : its.vertices) {
        out.vertices.push_back(v.x());
        out.vertices.push_back(v.y());
        out.vertices.push_back(v.z());
    }
    out.indices.reserve(its.indices.size() * 3);
    for (const auto &f : its.indices) {
        out.indices.push_back(static_cast<uint32_t>(f.x()));
        out.indices.push_back(static_cast<uint32_t>(f.y()));
        out.indices.push_back(static_cast<uint32_t>(f.z()));
    }
}

} // namespace

RawSimplifyResult simplify_mesh_raw(const RawMesh &raw, uint32_t triangle_count,
                                    float max_error)
{
    validate_raw_mesh(raw);
    if (std::isnan(max_error) || max_error <= 0.f)
        throw std::runtime_error("erreur quadrique maximale invalide");
    if (raw.vertices.size() / 3 > static_cast<size_t>(INT32_MAX))
        throw std::runtime_error("trop de sommets pour un indexed_triangle_set");

    indexed_triangle_set its;
    its.vertices.reserve(raw.vertices.size() / 3);
    for (size_t i = 0; i < raw.vertices.size(); i += 3)
        its.vertices.emplace_back(raw.vertices[i], raw.vertices[i + 1], raw.vertices[i + 2]);
    its.indices.reserve(raw.indices.size() / 3);
    for (size_t i = 0; i < raw.indices.size(); i += 3)
        its.indices.emplace_back(static_cast<int>(raw.indices[i]),
                                 static_cast<int>(raw.indices[i + 1]),
                                 static_cast<int>(raw.indices[i + 2]));

    // Même appel que GLGizmoSimplify::process : cible de triangles (mode
    // ratio, max_error illimité) ou erreur maximale (mode niveau, cible 0).
    // En sortie, max_error contient la dernière erreur de fusion acceptée.
    float error = max_error;
    Slic3r::its_quadric_edge_collapse(its, triangle_count, &error);
    // La compaction interne retire les triangles supprimés mais peut laisser
    // des sommets orphelins : on les élimine pour un WSMh minimal.
    Slic3r::its_compactify_vertices(its, /*shrink_to_fit=*/true);
    if (its.indices.empty())
        throw std::runtime_error("la simplification a supprimé toutes les facettes");

    RawSimplifyResult result;
    // Sans fusion, libslic3r conserve la valeur d'entrée (FLT_MAX en mode
    // ratio) : on la ramène à 0 pour un rapport lisible.
    result.max_error = (error == std::numeric_limits<float>::max()) ? 0.f : error;
    its_to_raw(its, result.mesh);
    return result;
}

void store_drc_raw(const RawMesh &raw, rust::Str path, int32_t bits)
{
    validate_raw_mesh(raw);
    if (raw.vertices.size() / 3 > static_cast<size_t>(INT32_MAX))
        throw std::runtime_error("trop de sommets pour un indexed_triangle_set");
    indexed_triangle_set its;
    its.vertices.reserve(raw.vertices.size() / 3);
    for (size_t i = 0; i < raw.vertices.size(); i += 3)
        its.vertices.emplace_back(raw.vertices[i], raw.vertices[i + 1], raw.vertices[i + 2]);
    its.indices.reserve(raw.indices.size() / 3);
    for (size_t i = 0; i < raw.indices.size(); i += 3)
        its.indices.emplace_back(static_cast<int>(raw.indices[i]),
                                 static_cast<int>(raw.indices[i + 1]),
                                 static_cast<int>(raw.indices[i + 2]));
    Slic3r::TriangleMesh mesh(std::move(its));
    const std::string out(path.data(), path.size());
    // Même appel que Plater::export_stl(..., FT_DRC) : bits = `drc_bits`
    // (défaut 0 = positions non quantifiées), vitesse par défaut.
    if (!Slic3r::store_drc(out.c_str(), &mesh, bits))
        throw std::runtime_error("écriture Draco impossible");
}

RawRepairResult repair_mesh_raw(const RawMesh &raw)
{
    validate_raw_mesh(raw);
    const size_t facet_count = raw.indices.size() / 3;

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

    // TriangleMesh::from_stl(..., repair=true) désactive volontairement
    // stl_fill_holes chez Orca (import robuste). Ici l'utilisateur demande une
    // réparation effective : appeler admesh directement avec fill_holes=true,
    // puis seulement convertir le résultat en TriangleMesh sans le réparer une
    // seconde fois. Les compteurs restent ceux de cette réparation admesh.
    stl_repair(&stl,
               /*fixall_flag=*/false,
               /*exact_flag=*/true,
               /*tolerance_flag=*/false, 0.f,
               /*increment_flag=*/false, 0.f,
               /*nearby_flag=*/true, /*iterations=*/2,
               /*remove_unconnected_flag=*/true,
               /*fill_holes_flag=*/true,
               /*normal_directions_flag=*/true,
               /*normal_values_flag=*/true,
               /*reverse_all_flag=*/false,
               /*verbose_flag=*/false);
    if (stl.stats.number_of_facets == 0)
        throw std::runtime_error("la réparation a supprimé toutes les facettes");

    Slic3r::TriangleMesh mesh;
    if (!mesh.from_stl(stl, /*repair=*/false))
        throw std::runtime_error("conversion du maillage réparé");

    RawRepairResult result;
    result.edges_fixed = stl.stats.edges_fixed;
    result.degenerate_facets = stl.stats.degenerate_facets;
    result.facets_removed = stl.stats.facets_removed;
    result.facets_reversed = stl.stats.facets_reversed;
    result.backwards_edges = stl.stats.backwards_edges;

    its_to_raw(mesh.its, result.mesh);
    return result;
}

} // namespace webslicer
