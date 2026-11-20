// Référence ISOLÉE (hors bridge web-slicer) : trait de pinceau Orca sur un
// cube 20 mm, chaîne 3MF par triangle. Reproduit GLGizmoPainterBase.cpp:846-886
// (SinglePointCursor puis DoublePointCursor entre points consécutifs) et
// GLGizmoFdmSupports (TriangleSelectorPatch(mesh) → select_patch → supported_facets.set).
// Sortie : une ligne "<triangle> <hex>" par triangle peint.
#include "libslic3r/Model.hpp"
#include "libslic3r/TriangleMesh.hpp"
#include "libslic3r/TriangleSelector.hpp"
#include <cstdio>
#include <queue>
#include <set>
#include <vector>
using namespace Slic3r;

// ---------------------------------------------------------------------------
// Lot 38 : outils du gizmo absents jusque-là. Tout passe par libslic3r SAUF le
// comblement des interstices, qui vit dans TriangleSelectorPatch (GUI, non
// lié ici) : copie VERBATIM de GLGizmoPainterBase.cpp:1423-1530 (sans les
// sommets de rendu, wireframe désactivé → stride 3) dans GapRef.
// ---------------------------------------------------------------------------
struct RefPatch {
    std::vector<float> patch_vertices;
    std::vector<int> triangle_indices;
    std::vector<int> facet_indices;
    EnforcerBlockerType type = EnforcerBlockerType::NONE;
    std::set<EnforcerBlockerType> neighbor_types;
    float area = 0.f;
};

class GapRef : public TriangleSelector {
public:
    using TriangleSelector::TriangleSelector;
    static constexpr float GapAreaMax = 5.f;
    std::vector<RefPatch> m_triangle_patches;
    float gap_area = 0.f;

    void update_selector_triangles()
    {
        for (RefPatch& patch : m_triangle_patches) {
            if (!(patch.area < gap_area) || patch.neighbor_types.empty())
                continue;
            EnforcerBlockerType type = *patch.neighbor_types.begin();
            for (int facet_idx : patch.facet_indices)
                m_triangles[facet_idx].set_state(type);
        }
    }

    void update_triangles_per_patch()
    {
        auto [neighbors, neighbors_propagated] = this->precompute_all_neighbors();
        std::vector<bool>  visited(m_triangles.size(), false);
        auto get_all_touching_triangles = [this](int facet_idx, const Vec3i32& neighbors, const Vec3i32& neighbors_propagated) -> std::vector<int> {
            std::vector<int> touching_triangles;
            Vec3i32            vertices = { m_triangles[facet_idx].verts_idxs[0], m_triangles[facet_idx].verts_idxs[1], m_triangles[facet_idx].verts_idxs[2] };
            append_touching_subtriangles(neighbors(0), vertices(1), vertices(0), touching_triangles);
            append_touching_subtriangles(neighbors(1), vertices(2), vertices(1), touching_triangles);
            append_touching_subtriangles(neighbors(2), vertices(0), vertices(2), touching_triangles);
            for (int neighbor_idx : neighbors_propagated)
                if (neighbor_idx != -1 && !m_triangles[neighbor_idx].is_split())
                    touching_triangles.emplace_back(neighbor_idx);
            return touching_triangles;
        };
        auto calc_fragment_area = [](const RefPatch& patch, float max_limit_area, int stride) {
            double total_area = 0.f;
            const std::vector<int>& ti = patch.triangle_indices;
            const std::vector<float>& vertices = patch.patch_vertices;
            for (int i = 0; i < ti.size(); i+=3) {
                stl_vertex v0(vertices[ti[i] * stride], vertices[ti[i] * stride + 1], vertices[ti[i] * stride + 2]);
                stl_vertex v1(vertices[ti[i + 1] * stride], vertices[ti[i + 1] * stride + 1], vertices[ti[i + 1] * stride + 2]);
                stl_vertex v2(vertices[ti[i + 2] * stride], vertices[ti[i + 2] * stride + 1], vertices[ti[i + 2] * stride + 2]);
                total_area += std::abs((v0 - v1).cross(v0 - v2).norm()) / 2;
                if (total_area >= max_limit_area)
                    break;
            }
            return total_area;
        };
        int start_facet_idx = 0;
        while (1) {
            for (; start_facet_idx < visited.size(); start_facet_idx++) {
                if (!visited[start_facet_idx] && m_triangles[start_facet_idx].valid() && !m_triangles[start_facet_idx].is_split())
                    break;
            }
            if (start_facet_idx >= m_triangles.size())
                break;
            EnforcerBlockerType start_facet_state = m_triangles[start_facet_idx].get_state();
            RefPatch patch;
            std::queue<int> facet_queue;
            facet_queue.push(start_facet_idx);
            while (!facet_queue.empty()) {
                int current_facet = facet_queue.front();
                facet_queue.pop();
                if (!visited[current_facet]) {
                    Triangle& triangle = m_triangles[current_facet];
                    for (int i = 0; i < 3; ++i) {
                        int j = triangle.verts_idxs[i];
                        int index = int(patch.patch_vertices.size()/3);
                        patch.patch_vertices.emplace_back(m_vertices[j].v(0));
                        patch.patch_vertices.emplace_back(m_vertices[j].v(1));
                        patch.patch_vertices.emplace_back(m_vertices[j].v(2));
                        patch.triangle_indices.emplace_back( index);
                    }
                    patch.facet_indices.push_back(current_facet);
                    std::vector<int> touching_triangles = get_all_touching_triangles(current_facet, neighbors[current_facet], neighbors_propagated[current_facet]);
                    for (const int tr_idx : touching_triangles) {
                        if (tr_idx < 0)
                            continue;
                        if (m_triangles[tr_idx].get_state() != start_facet_state) {
                            patch.neighbor_types.insert(m_triangles[tr_idx].get_state());
                            continue;
                        }
                        if (visited[tr_idx])
                            continue;
                        facet_queue.push(tr_idx);
                    }
                }
                visited[current_facet] = true;
            }
            patch.area = calc_fragment_area(patch, GapAreaMax, 3);
            patch.type = start_facet_state;
            m_triangle_patches.emplace_back(std::move(patch));
        }
    }
};

static indexed_triangle_set cube_its(float s)
{
    indexed_triangle_set its;
    its.vertices = {{0,0,0},{s,0,0},{s,s,0},{0,s,0},{0,0,s},{s,0,s},{s,s,s},{0,s,s}};
    its.indices = {{0,2,1},{0,3,2},{4,5,6},{4,6,7},{0,1,5},{0,5,4},{1,2,6},{1,6,5},{2,3,7},{2,7,6},{3,0,4},{3,4,7}};
    return its;
}

static void dump(ModelVolume *mv, const TriangleSelector &selector)
{
    mv->supported_facets.set(selector);
    for (const auto &m : mv->supported_facets.get_data().triangles_to_split)
        std::printf("%d %s\n", m.triangle_idx, mv->supported_facets.get_triangle_as_string(m.triangle_idx).c_str());
}

// Trait de référence du cube (3 points, rayon 3, face du dessus) + un tampon
// de rayon r_dot en (12, 8) à l'état dot_state.
template<class Sel>
static void cube_strokes(Sel &selector, EnforcerBlockerType state, float r_dot, EnforcerBlockerType dot_state)
{
    const float s = 20.f;
    const std::vector<Vec3f> pts = {{5.f, 5.f, s}, {12.f, 8.f, s}, {8.f, 14.f, s}};
    const std::vector<int> facets = {2, 2, 3};
    const Vec3f camera(10.f, 10.f, 100.f);
    const Transform3d trafo = Transform3d::Identity();
    const TriangleSelector::ClippingPlane clp;
    for (size_t i = 0; i + 1 < pts.size(); ++i) {
        auto cursor = TriangleSelector::DoublePointCursor::cursor_factory(pts[i], pts[i + 1], camera, 3.f, TriangleSelector::CursorType::CIRCLE, trafo, clp);
        selector.select_patch(facets[i], std::move(cursor), state, trafo, true, 0.f);
    }
    if (r_dot > 0.f) {
        auto cursor = TriangleSelector::SinglePointCursor::cursor_factory(Vec3f(12.f, 8.f, s), camera, r_dot, TriangleSelector::CursorType::CIRCLE, trafo, clp);
        selector.select_patch(2, std::move(cursor), dot_state, trafo, true, 0.f);
    }
}

// pointer : cube peint (cercle, enforcer), puis curseur « Triangles » en
// (6, 4.5, 20) facette 2 → bucket_fill_select_triangles(-1, propagate=false)
// sur le sous-triangle touché, état BLOCKER (GLGizmoPainterBase.cpp:859).
static int pointer_mode()
{
    Model model;
    ModelVolume *mv = model.add_object()->add_volume(TriangleMesh(cube_its(20.f)), ModelVolumeType::MODEL_PART, false);
    TriangleSelector selector(mv->mesh());
    cube_strokes(selector, EnforcerBlockerType::ENFORCER, 0.f, EnforcerBlockerType::NONE);
    const TriangleSelector::ClippingPlane clp;
    selector.bucket_fill_select_triangles(Vec3f(6.f, 4.5f, 20.f), 2, clp, -1.f, false, true);
    selector.seed_fill_apply_on_triangles(EnforcerBlockerType::BLOCKER);
    selector.seed_fill_unselect_all_triangles();
    dump(mv, selector);
    return 0;
}

// height / height_z : cylindre its_make_cylinder(10, 20), MMU (edge_limit 0,2),
// plage de hauteur 2 mm état Extruder2 (GLGizmoPainterBase.cpp:780-806).
// height : clic paroi (10, 0.3, 10) facette la plus proche, trafo identité.
// height_z : autre pièce, trafo translation z +3, cote monde imposée 8 →
// première facette dont un sommet est dans [8, 10] (get_projected_height_range).
static int height_mode(bool other)
{
    Model model;
    ModelVolume *mv = model.add_object()->add_volume(TriangleMesh(its_make_cylinder(10., 20.)), ModelVolumeType::MODEL_PART, false);
    TriangleSelector selector(mv->mesh(), 0.2f);
    const auto &its = mv->mesh().its;
    Transform3d trafo = Transform3d::Identity();
    if (other)
        trafo.translation() = Vec3d(0., 0., 3.);
    Transform3d trafo_nt = trafo; trafo_nt.translation() = Vec3d::Zero();
    const TriangleSelector::ClippingPlane clp;
    const Vec3f camera(100.f, 0.f, 10.f);
    const float height = 2.f;
    int facet = -1; float z_world = 0.f; Vec3f hit;
    if (!other) {
        float best = 1e9f;
        for (int i = 0; i < int(its.indices.size()); ++i) {
            Vec3f c = (its.vertices[its.indices[i][0]] + its.vertices[its.indices[i][1]] + its.vertices[its.indices[i][2]]) / 3.f;
            float d = (c - Vec3f(10.f, 0.3f, 10.f)).norm();
            if (d < best) { best = d; facet = i; }
        }
        hit = (its.vertices[its.indices[facet][0]] + its.vertices[its.indices[facet][1]] + its.vertices[its.indices[facet][2]]) / 3.f;
        z_world = (trafo * Vec3d(hit(0), hit(1), hit(2))).z();
        std::printf("# facet %d hit %.6f %.6f %.6f\n", facet, hit.x(), hit.y(), hit.z());
    } else {
        z_world = 8.f;
        const float z_top = z_world + height;
        for (int facet_idx = 0; facet_idx < int(its.indices.size()); facet_idx++) {
            stl_vertex v0 = its.vertices[its.indices[facet_idx].x()];
            stl_vertex v1 = its.vertices[its.indices[facet_idx].y()];
            stl_vertex v2 = its.vertices[its.indices[facet_idx].z()];
            float v0_z = (trafo * Vec3d(v0(0), v0(1), v0(2))).z();
            float v1_z = (trafo * Vec3d(v1(0), v1(1), v1(2))).z();
            float v2_z = (trafo * Vec3d(v2(0), v2(1), v2(2))).z();
            bool outside_range = (v0_z < z_world && v1_z < z_world && v2_z < z_world) || (v0_z > z_top && v1_z > z_top && v2_z > z_top);
            if (!outside_range) { facet = facet_idx; break; }
        }
        std::printf("# z_world %.6f facet %d\n", z_world, facet);
    }
    auto cursor = TriangleSelector::SinglePointCursor::cursor_factory(z_world, camera, height, trafo, clp);
    selector.select_patch(facet, std::move(cursor), EnforcerBlockerType::Extruder2, trafo_nt, true, 0.f);
    dump(mv, selector);
    return 0;
}

// gap : cube MMU (edge_limit 0,2), trait Extruder1 + tampon Extruder2 de rayon
// 0,8 → comblement à 4 mm² (le tampon ≈ 2 mm² devient Extruder1) ; gap0 :
// même chose à 0 mm² (inchangé).
static int gap_mode(float area)
{
    Model model;
    ModelVolume *mv = model.add_object()->add_volume(TriangleMesh(cube_its(20.f)), ModelVolumeType::MODEL_PART, false);
    GapRef selector(mv->mesh(), 0.2f);
    cube_strokes(selector, EnforcerBlockerType::Extruder1, 0.8f, EnforcerBlockerType::Extruder2);
    selector.update_triangles_per_patch();
    selector.gap_area = area;
    selector.update_selector_triangles();
    dump(mv, selector);
    return 0;
}

// remap : cube MMU, trait Extruder1 + tampon Extruder2 (rayon 2), table
// [3, 1] (1 → 3, 2 → 1) → remap_triangle_state (GLGizmoMmuSegmentation.cpp:1082).
static int remap_mode()
{
    Model model;
    ModelVolume *mv = model.add_object()->add_volume(TriangleMesh(cube_its(20.f)), ModelVolumeType::MODEL_PART, false);
    TriangleSelector selector(mv->mesh(), 0.2f);
    cube_strokes(selector, EnforcerBlockerType::Extruder1, 2.f, EnforcerBlockerType::Extruder2);
    EnforcerBlockerStateMap state_map;
    for (size_t i = 0; i <= size_t(EnforcerBlockerType::ExtruderMax); ++i)
        state_map[i] = static_cast<EnforcerBlockerType>(i);
    state_map[1] = EnforcerBlockerType::Extruder3;
    state_map[2] = EnforcerBlockerType::Extruder1;
    selector.remap_triangle_state(state_map);
    dump(mv, selector);
    return 0;
}

// Avant remappage (même scène) : sert à vérifier que la peinture d'entrée
// du test Rust est bien celle-ci.
static int remap_input_mode()
{
    Model model;
    ModelVolume *mv = model.add_object()->add_volume(TriangleMesh(cube_its(20.f)), ModelVolumeType::MODEL_PART, false);
    TriangleSelector selector(mv->mesh(), 0.2f);
    cube_strokes(selector, EnforcerBlockerType::Extruder1, 2.f, EnforcerBlockerType::Extruder2);
    dump(mv, selector);
    return 0;
}

// bucket_noedge : pot sans détection d'arêtes (angle -1, case décochée du
// gizmo MMU) depuis la paroi du cylindre → tout le maillage connexe.
static int bucket_noedge_mode()
{
    Model model;
    ModelVolume *mv = model.add_object()->add_volume(TriangleMesh(its_make_cylinder(10., 20.)), ModelVolumeType::MODEL_PART, false);
    TriangleSelector selector(mv->mesh(), 0.2f);
    const auto &its = mv->mesh().its;
    int facet = -1; float best = 1e9f;
    for (int i = 0; i < int(its.indices.size()); ++i) {
        Vec3f c = (its.vertices[its.indices[i][0]] + its.vertices[its.indices[i][1]] + its.vertices[its.indices[i][2]]) / 3.f;
        float d = (c - Vec3f(10.f, 0.3f, 10.f)).norm();
        if (d < best) { best = d; facet = i; }
    }
    const Vec3f hit = (its.vertices[its.indices[facet][0]] + its.vertices[its.indices[facet][1]] + its.vertices[its.indices[facet][2]]) / 3.f;
    const TriangleSelector::ClippingPlane clp;
    selector.bucket_fill_select_triangles(hit, facet, clp, -1.f, true, true);
    selector.seed_fill_apply_on_triangles(EnforcerBlockerType::Extruder3);
    selector.seed_fill_unselect_all_triangles();
    std::printf("# facet %d hit %.6f %.6f %.6f\n", facet, hit.x(), hit.y(), hit.z());
    dump(mv, selector);
    return 0;
}

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
    if (mode == "pointer")
        return pointer_mode();
    if (mode == "height" || mode == "height_z")
        return height_mode(mode == "height_z");
    if (mode == "gap")
        return gap_mode(4.f);
    if (mode == "gap0")
        return gap_mode(0.f);
    if (mode == "remap")
        return remap_mode();
    if (mode == "remap_input")
        return remap_input_mode();
    if (mode == "bucket_noedge")
        return bucket_noedge_mode();
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
