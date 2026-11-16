// Référence ISOLÉE (hors bridge web-slicer) : trait de pinceau Orca sur un
// cube 20 mm, chaîne 3MF par triangle. Reproduit GLGizmoPainterBase.cpp:846-886
// (SinglePointCursor puis DoublePointCursor entre points consécutifs) et
// GLGizmoFdmSupports (TriangleSelectorPatch(mesh) → select_patch → supported_facets.set).
// Sortie : une ligne "<triangle> <hex>" par triangle peint.
#include "libslic3r/Model.hpp"
#include "libslic3r/TriangleMesh.hpp"
#include "libslic3r/TriangleSelector.hpp"
#include <cstdio>
#include <vector>
using namespace Slic3r;

// Remplissages (GLGizmoPainterBase.cpp:851-866) : cylindre its_make_cylinder(10, 20)
// (180 secteurs), clic sur la paroi (10, 0, 10) facette 0 ; smart fill =
// seed_fill_select_triangles(angle 30°, défaut m_smart_fill_angle), bucket fill =
// bucket_fill_select_triangles(angle 30°, propagate) ; puis
// seed_fill_apply_on_triangles(ENFORCER) et seed_fill_unselect_all_triangles.
static int fill(bool smart, float angle)
{
    Model model;
    ModelObject *obj = model.add_object();
    ModelVolume *mv = obj->add_volume(TriangleMesh(its_make_cylinder(10., 20.)), ModelVolumeType::MODEL_PART, false);
    TriangleSelector selector(mv->mesh());
    const auto &its = mv->mesh().its;
    // Facette de paroi la plus proche de (10, 0, 10) : centroïde.
    int facet = -1; float best = 1e9f;
    for (int i = 0; i < int(its.indices.size()); ++i) {
        Vec3f c = (its.vertices[its.indices[i][0]] + its.vertices[its.indices[i][1]] + its.vertices[its.indices[i][2]]) / 3.f;
        float d = (c - Vec3f(10.f, 0.3f, 10.f)).norm();
        if (d < best) { best = d; facet = i; }
    }
    const Vec3f &a = its.vertices[its.indices[facet][0]], &b = its.vertices[its.indices[facet][1]], &c3 = its.vertices[its.indices[facet][2]];
    const Vec3f hit = (a + b + c3) / 3.f;
    const TriangleSelector::ClippingPlane clp;
    if (smart)
        selector.seed_fill_select_triangles(hit, facet, Transform3d::Identity(), clp, angle, 0.f, true);
    else
        selector.bucket_fill_select_triangles(hit, facet, clp, angle, true, true);
    selector.seed_fill_apply_on_triangles(EnforcerBlockerType::ENFORCER);
    selector.seed_fill_unselect_all_triangles();
    mv->supported_facets.set(selector);
    std::printf("# facet %d hit %.6f %.6f %.6f\n", facet, hit.x(), hit.y(), hit.z());
    for (const auto &m : mv->supported_facets.get_data().triangles_to_split)
        std::printf("%d %s\n", m.triangle_idx, mv->supported_facets.get_triangle_as_string(m.triangle_idx).c_str());
    return 0;
}

int main(int argc, char **argv)
{
    const std::string mode = argc > 1 ? argv[1] : "circle";
    if (mode == "smart" || mode == "bucket")
        return fill(mode == "smart", 30.f);
    // Angle d'arrêt 1° < 2° entre secteurs voisins : la propagation s'arrête.
    if (mode == "smart1" || mode == "bucket1")
        return fill(mode == "smart1", 1.f);
    const bool sphere = mode == "sphere";
    indexed_triangle_set its;
    const float s = 20.f;
    its.vertices = {{0,0,0},{s,0,0},{s,s,0},{0,s,0},{0,0,s},{s,0,s},{s,s,s},{0,s,s}};
    its.indices = {{0,2,1},{0,3,2},{4,5,6},{4,6,7},{0,1,5},{0,5,4},{1,2,6},{1,6,5},{2,3,7},{2,7,6},{3,0,4},{3,4,7}};
    Model model;
    ModelObject *obj = model.add_object();
    ModelVolume *mv = obj->add_volume(TriangleMesh(std::move(its)), ModelVolumeType::MODEL_PART, false);

    TriangleSelector selector(mv->mesh());
    selector.deserialize(mv->supported_facets.get_data(), false);

    const std::vector<Vec3f> pts = {{5.f, 5.f, s}, {12.f, 8.f, s}, {8.f, 14.f, s}};
    const std::vector<int> facets = {2, 2, 3};
    const Vec3f camera(10.f, 10.f, 100.f);
    const float radius = 3.f;
    const Transform3d trafo = Transform3d::Identity();
    Transform3d trafo_nt = trafo; trafo_nt.translation() = Vec3d::Zero();
    const TriangleSelector::ClippingPlane clp;
    const auto type = sphere ? TriangleSelector::CursorType::SPHERE : TriangleSelector::CursorType::CIRCLE;
    for (size_t i = 0; i + 1 < pts.size(); ++i) {
        auto cursor = TriangleSelector::DoublePointCursor::cursor_factory(pts[i], pts[i + 1], camera, radius, type, trafo, clp);
        selector.select_patch(facets[i], std::move(cursor), EnforcerBlockerType::ENFORCER, trafo_nt, true, 0.f);
    }
    mv->supported_facets.set(selector);
    for (const auto &m : mv->supported_facets.get_data().triangles_to_split)
        std::printf("%d %s\n", m.triangle_idx, mv->supported_facets.get_triangle_as_string(m.triangle_idx).c_str());
    return 0;
}
