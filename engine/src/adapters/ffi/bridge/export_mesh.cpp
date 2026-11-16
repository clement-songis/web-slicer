// Maillages d'export STL/3D comme le menu « Export plate as STL » d'Orca
// (Plater::export_stl, Plater.cpp:15188, lambda mesh_to_export_fff_no_boolean) :
// par objet, pièces positives fusionnées, volumes négatifs soustraits par
// booléens CGAL quand c'est possible (csgmesh), sinon pièces positives seules ;
// puis chaque instance transformée et fusionnée (instance_id = -1).
// PerformCSGMeshBooleans.hpp utilise BOOST_LOG_TRIVIAL : Boost.Log est lié
// dynamiquement (symboles v2_mt_posix), pas en variante statique.
#ifndef BOOST_LOG_DYN_LINK
#define BOOST_LOG_DYN_LINK 1
#endif
#include "engine/src/adapters/ffi/bridge/export_mesh.hpp"
#include "engine/src/adapters/ffi/bridge.rs.h"

#include "libslic3r/Model.hpp"
#include "libslic3r/TriangleMesh.hpp"
#include "libslic3r/MeshBoolean.hpp"
// Les en-têtes CSG supposent Point.hpp / TriangleMesh.hpp déjà inclus.
#include "libslic3r/CSGMesh/CSGMesh.hpp"
#include "libslic3r/CSGMesh/ModelToCSGMesh.hpp"
#include "libslic3r/CSGMesh/PerformCSGMeshBooleans.hpp"

namespace webslicer {

void raw_into_model(const rust::Vec<RawObject> &objects, Slic3r::Model &model);
namespace {

RawMesh to_raw(const indexed_triangle_set &its)
{
    RawMesh out;
    out.vertices.reserve(its.vertices.size() * 3);
    for (const auto &v : its.vertices) {
        out.vertices.push_back(v.x());
        out.vertices.push_back(v.y());
        out.vertices.push_back(v.z());
    }
    out.indices.reserve(its.indices.size() * 3);
    for (const auto &f : its.indices)
        for (int k = 0; k < 3; ++k)
            out.indices.push_back(static_cast<uint32_t>(f[k]));
    return out;
}

Slic3r::TriangleMesh mesh_to_export(const Slic3r::ModelObject &mo)
{
    using namespace Slic3r;
    TriangleMesh mesh;
    std::vector<csg::CSGPart> csgmesh;
    csgmesh.reserve(2 * mo.volumes.size());
    csg::model_to_csgmesh(mo, Transform3d::Identity(), std::back_inserter(csgmesh),
                          csg::mpartsPositive | csg::mpartsNegative | csg::mpartsDoSplits);
    auto csgrange = range(csgmesh);
    if (csg::is_all_positive(csgrange)) {
        mesh = TriangleMesh{csg::csgmesh_merge_positive_parts(csgrange)};
    } else if (std::get<2>(csg::check_csgmesh_booleans(csgrange)) == csgrange.end()) {
        try {
            auto cgalm = csg::perform_csgmesh_booleans(csgrange);
            mesh = MeshBoolean::cgal::cgal_to_triangle_mesh(*cgalm);
        } catch (...) {
        }
    }
    if (mesh.empty()) {
        for (const ModelVolume *v : mo.volumes)
            if (v->is_model_part()) {
                TriangleMesh vol_mesh(v->mesh());
                vol_mesh.transform(v->get_matrix(), true);
                mesh.merge(vol_mesh);
            }
    }
    TriangleMesh vols_mesh(mesh);
    mesh = TriangleMesh();
    for (const ModelInstance *i : mo.instances) {
        TriangleMesh m = vols_mesh;
        m.transform(i->get_matrix(), true);
        mesh.merge(m);
    }
    return mesh;
}

} // namespace

rust::Vec<RawMesh> export_meshes_raw(const rust::Vec<RawObject> &objects)
{
    Slic3r::Model model;
    raw_into_model(objects, model);
    rust::Vec<RawMesh> out;
    for (const Slic3r::ModelObject *mo : model.objects)
        out.push_back(to_raw(mesh_to_export(*mo).its));
    return out;
}

} // namespace webslicer
