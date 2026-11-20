// Peinture au triangle par le code d'Orca (TriangleSelector.cpp, libslic3r),
// à l'identique du gizmo de peinture (GLGizmoPainterBase.cpp:846-886) :
// - chargement : FacetsAnnotation::set_triangle_from_string (lecteur 3MF) puis
//   TriangleSelector::deserialize (GLGizmoFdmSupports::update_from_model) ;
// - trait : un point → SinglePointCursor::cursor_factory + select_patch ;
//   plusieurs points → DoublePointCursor (capsule) entre points consécutifs ;
// - export : FacetsAnnotation::set(selector) puis get_triangle_as_string
//   (chaînes hex écrites dans le 3MF, `slic3rpe:custom_supports` & co).
// Un TriangleSelector par canal (0 supports, 1 couture, 2 MMU, 3 fuzzy) ;
// edge_limit 0,2 pour le MMU (GLGizmoMmuSegmentation.cpp:740), défaut sinon.
// Outils du gizmo (GLGizmoPainterBase::gizmo_event, l. 780-886) : pinceau
// cercle/sphère, triangle (POINTER → bucket_fill sans propagation), plage de
// hauteur (HEIGHT_RANGE → SinglePointCursor::cursor_factory(z_world, …)),
// smart fill, pot ; comblement des interstices et remappage MMU (boutons du
// gizmo). Le comblement vit dans TriangleSelectorPatch (code GUI non lié au
// moteur) : porté ligne à ligne dans WebTriangleSelector ci-dessous.
#include "engine/src/adapters/ffi/bridge/paint.hpp"
#include "engine/src/adapters/ffi/bridge.rs.h"

#include "libslic3r/Model.hpp"
#include "libslic3r/TriangleMesh.hpp"
#include "libslic3r/TriangleSelector.hpp"

#include <array>
#include <cmath>
#include <queue>
#include <set>
#include <stdexcept>
#include <string>

namespace webslicer {

// TriangleSelector + les algorithmes de TriangleSelectorPatch dont le moteur a
// besoin (GLGizmoPainterBase.cpp/.hpp d'Orca, sans la géométrie de rendu).
class WebTriangleSelector : public Slic3r::TriangleSelector {
public:
    using Slic3r::TriangleSelector::TriangleSelector;

    // TriangleSelectorPatch::GapAreaMax (GLGizmoPainterBase.hpp:119).
    static constexpr float GapAreaMax = 5.f;

    // TrianglePatch (GLGizmoPainterBase.hpp:83) réduit à ce que lit le comblement.
    struct Patch {
        std::vector<int> facet_indices;
        std::set<Slic3r::EnforcerBlockerType> neighbor_types;
        float area = 0.f;
    };

    // TriangleSelectorPatch::update_triangles_per_patch (GLGizmoPainterBase.cpp:1435) :
    // composantes connexes d'un même état (parcours en largeur, même ordre),
    // états voisins, aire cumulée arrêtée à GapAreaMax (calc_fragment_area).
    std::vector<Patch> patches() const
    {
        std::vector<Patch> out;
        auto [neighbors, neighbors_propagated] = this->precompute_all_neighbors();
        std::vector<bool> visited(m_triangles.size(), false);

        auto get_all_touching_triangles = [this](int facet_idx, const Slic3r::Vec3i32 &neighbors,
                                                 const Slic3r::Vec3i32 &neighbors_propagated) -> std::vector<int> {
            std::vector<int> touching_triangles;
            Slic3r::Vec3i32 vertices = {m_triangles[facet_idx].verts_idxs[0], m_triangles[facet_idx].verts_idxs[1],
                                        m_triangles[facet_idx].verts_idxs[2]};
            append_touching_subtriangles(neighbors(0), vertices(1), vertices(0), touching_triangles);
            append_touching_subtriangles(neighbors(1), vertices(2), vertices(1), touching_triangles);
            append_touching_subtriangles(neighbors(2), vertices(0), vertices(2), touching_triangles);
            for (int neighbor_idx : neighbors_propagated)
                if (neighbor_idx != -1 && !m_triangles[neighbor_idx].is_split())
                    touching_triangles.emplace_back(neighbor_idx);
            return touching_triangles;
        };

        int start_facet_idx = 0;
        while (true) {
            for (; start_facet_idx < int(visited.size()); start_facet_idx++)
                if (!visited[start_facet_idx] && m_triangles[start_facet_idx].valid() &&
                    !m_triangles[start_facet_idx].is_split())
                    break;
            if (start_facet_idx >= int(m_triangles.size()))
                break;

            const Slic3r::EnforcerBlockerType start_facet_state = m_triangles[start_facet_idx].get_state();
            Patch patch;
            // calc_fragment_area : double cumulé, terme flottant par triangle,
            // dans l'ordre de visite, arrêt dès GapAreaMax.
            double total_area = 0.;
            bool area_done = false;
            std::queue<int> facet_queue;
            facet_queue.push(start_facet_idx);
            while (!facet_queue.empty()) {
                const int current_facet = facet_queue.front();
                facet_queue.pop();
                if (!visited[current_facet]) {
                    const Triangle &triangle = m_triangles[current_facet];
                    if (!area_done) {
                        const stl_vertex v0 = m_vertices[triangle.verts_idxs[0]].v;
                        const stl_vertex v1 = m_vertices[triangle.verts_idxs[1]].v;
                        const stl_vertex v2 = m_vertices[triangle.verts_idxs[2]].v;
                        total_area += std::abs((v0 - v1).cross(v0 - v2).norm()) / 2;
                        if (total_area >= GapAreaMax)
                            area_done = true;
                    }
                    patch.facet_indices.push_back(current_facet);
                    for (const int tr_idx : get_all_touching_triangles(current_facet, neighbors[current_facet],
                                                                       neighbors_propagated[current_facet])) {
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
            patch.area = float(total_area);
            out.emplace_back(std::move(patch));
        }
        return out;
    }

    // TriangleSelectorPatch::update_selector_triangles (GLGizmoPainterBase.cpp:1423) :
    // un fragment (aire < gap_area, TrianglePatch::is_fragment) prend le plus
    // petit état voisin (std::set::begin).
    void gap_fill(float gap_area)
    {
        for (const Patch &patch : patches()) {
            if (!(patch.area < gap_area) || patch.neighbor_types.empty())
                continue;
            const Slic3r::EnforcerBlockerType type = *patch.neighbor_types.begin();
            for (int facet_idx : patch.facet_indices)
                m_triangles[facet_idx].set_state(type);
        }
    }

    // Triangles marqués par la sélection par propagation (rendu m_iva_seed_fills).
    void seed_fill_selection(size_t max_triangles, rust::Vec<float> &out, bool &truncated) const
    {
        size_t n = 0;
        for (const Triangle &tr : m_triangles) {
            if (!tr.valid() || tr.is_split() || !tr.is_selected_by_seed_fill())
                continue;
            if (n == max_triangles) {
                truncated = true;
                return;
            }
            for (int k : tr.verts_idxs) {
                const stl_vertex &v = m_vertices[k].v;
                out.push_back(v.x());
                out.push_back(v.y());
                out.push_back(v.z());
            }
            ++n;
        }
    }
};

struct PaintSessionImpl {
    Slic3r::Model model;
    Slic3r::ModelVolume *volume = nullptr;
    std::array<std::unique_ptr<WebTriangleSelector>, 4> selectors;
};

namespace {

WebTriangleSelector &selector_for(PaintSession &session, uint8_t channel)
{
    PaintSessionImpl &s = *session.impl;
    if (channel > 3)
        throw std::runtime_error("canal de peinture inconnu");
    auto &slot = s.selectors[channel];
    if (!slot) {
        if (channel == 2)
            slot = std::make_unique<WebTriangleSelector>(s.volume->mesh(), 0.2f);
        else
            slot = std::make_unique<WebTriangleSelector>(s.volume->mesh());
    }
    return *slot;
}

Slic3r::Transform3d from_col_major16(const std::array<double, 16> &m)
{
    Slic3r::Transform3d t;
    for (int c = 0; c < 4; ++c)
        for (int r = 0; r < 4; ++r)
            t.matrix()(r, c) = m[c * 4 + r];
    return t;
}

} // namespace

PaintSession::PaintSession() : impl(std::make_unique<PaintSessionImpl>()) {}
PaintSession::~PaintSession() = default;

namespace {
// RawMesh → its avec contrôles (indices de triangle conservés, pas de réparation).
indexed_triangle_set paint_its(const RawMesh &raw)
{
    if (raw.indices.empty() || raw.indices.size() % 3 != 0 || raw.vertices.size() % 3 != 0)
        throw std::runtime_error("maillage vide ou indices incomplets");
    const size_t nv = raw.vertices.size() / 3;
    indexed_triangle_set its;
    its.vertices.reserve(nv);
    for (size_t i = 0; i < nv; ++i) {
        const float x = raw.vertices[3 * i], y = raw.vertices[3 * i + 1], z = raw.vertices[3 * i + 2];
        if (!std::isfinite(x) || !std::isfinite(y) || !std::isfinite(z))
            throw std::runtime_error("coordonnée non finie");
        its.vertices.emplace_back(x, y, z);
    }
    for (size_t i = 0; i < raw.indices.size(); i += 3) {
        if (raw.indices[i] >= nv || raw.indices[i + 1] >= nv || raw.indices[i + 2] >= nv)
            throw std::runtime_error("indice de sommet hors limites");
        its.indices.emplace_back(int(raw.indices[i]), int(raw.indices[i + 1]), int(raw.indices[i + 2]));
    }
    return its;
}
} // namespace

RawPaintFacets remap_painting_raw(const RawMesh &source, const RawMesh &target, const RawPaintFacets &facets)
{
    const indexed_triangle_set src = paint_its(source);
    const indexed_triangle_set dst = paint_its(target);
    // Tampon de (dé)sérialisation : FacetsAnnotation n'est constructible que
    // par un ModelVolume (comme la session de peinture).
    Slic3r::Model model;
    Slic3r::ModelObject *object = model.add_object();
    Slic3r::ModelVolume *volume = object->add_volume(Slic3r::TriangleMesh(indexed_triangle_set(src)),
                                                     Slic3r::ModelVolumeType::MODEL_PART,
                                                     /*modify_to_center_geometry=*/false);
    Slic3r::FacetsAnnotation &ann = volume->supported_facets;
    ann.reset();
    const size_t n = std::min(facets.triangles.size(), facets.states.size());
    int last = -1;
    for (size_t i = 0; i < n; ++i) {
        const int t = int(facets.triangles[i]);
        std::string state(facets.states[i]);
        if (t <= last || t >= int(src.indices.size()) || state.empty())
            throw std::runtime_error("peinture : indices non croissants ou hors maillage");
        for (char &c : state)
            c = char(std::toupper(static_cast<unsigned char>(c)));
        if (state.find_first_not_of("0123456789ABCDEF") != std::string::npos)
            throw std::runtime_error("peinture : chaîne hexadécimale invalide");
        ann.set_triangle_from_string(t, state);
        last = t;
    }
    RawPaintFacets out;
    if (ann.get_data().bitstream.empty())
        return out;
    // ModelVolume::restore_painting : maillage cible dans le repère source
    // (maillages d'un même volume, sans décalage d'origine ici → identité).
    Slic3r::TriangleSelector::TriangleSplittingData remapped = Slic3r::TriangleSelector::remap_painting(
        src, ann.get_data(), dst, Slic3r::Transform3d::Identity(), std::nullopt);
    Slic3r::FacetsAnnotation &result = volume->seam_facets;
    result.set_data(std::move(remapped));
    for (const auto &m : result.get_data().triangles_to_split) {
        std::string str = result.get_triangle_as_string(m.triangle_idx);
        if (str.empty())
            continue;
        out.triangles.push_back(uint32_t(m.triangle_idx));
        out.states.push_back(rust::String(str));
    }
    return out;
}

std::unique_ptr<PaintSession> paint_session_new(const RawMesh &raw)
{
    if (raw.indices.empty() || raw.indices.size() % 3 != 0 || raw.vertices.size() % 3 != 0)
        throw std::runtime_error("maillage vide ou indices incomplets");
    const size_t nv = raw.vertices.size() / 3;
    indexed_triangle_set its;
    its.vertices.reserve(nv);
    for (size_t i = 0; i < nv; ++i) {
        const float x = raw.vertices[3 * i], y = raw.vertices[3 * i + 1], z = raw.vertices[3 * i + 2];
        if (!std::isfinite(x) || !std::isfinite(y) || !std::isfinite(z))
            throw std::runtime_error("coordonnée non finie");
        its.vertices.emplace_back(x, y, z);
    }
    for (size_t i = 0; i < raw.indices.size(); i += 3) {
        if (raw.indices[i] >= nv || raw.indices[i + 1] >= nv || raw.indices[i + 2] >= nv)
            throw std::runtime_error("indice de sommet hors limites");
        its.indices.emplace_back(int(raw.indices[i]), int(raw.indices[i + 1]), int(raw.indices[i + 2]));
    }
    auto s = std::make_unique<PaintSession>();
    Slic3r::ModelObject *object = s->impl->model.add_object();
    // Pas de réparation : les indices de triangle doivent rester ceux du client.
    s->impl->volume = object->add_volume(Slic3r::TriangleMesh(std::move(its)),
                                   Slic3r::ModelVolumeType::MODEL_PART,
                                   /*modify_to_center_geometry=*/false);
    return s;
}

size_t paint_session_triangles(const PaintSession &s)
{
    return s.impl->volume->mesh().its.indices.size();
}

void paint_session_load(PaintSession &s, uint8_t channel, const RawPaintFacets &facets)
{
    Slic3r::TriangleSelector &selector = selector_for(s, channel);
    Slic3r::FacetsAnnotation &ann = s.impl->volume->supported_facets; // tampon de (dé)sérialisation
    ann.reset();
    const int triangles = int(paint_session_triangles(s));
    const size_t n = std::min(facets.triangles.size(), facets.states.size());
    int last = -1;
    for (size_t i = 0; i < n; ++i) {
        const int t = int(facets.triangles[i]);
        std::string state(facets.states[i]);
        if (t <= last || t >= triangles || state.empty())
            throw std::runtime_error("peinture : indices non croissants ou hors maillage");
        for (char &c : state)
            c = char(std::toupper(static_cast<unsigned char>(c)));
        if (state.find_first_not_of("0123456789ABCDEF") != std::string::npos)
            throw std::runtime_error("peinture : chaîne hexadécimale invalide");
        ann.set_triangle_from_string(t, state);
        last = t;
    }
    selector.deserialize(ann.get_data(), true);
    ann.reset();
}

namespace {

// Contrôles communs d'un trait ; renvoie le nombre de points.
size_t check_stroke(PaintSession &s, const RawPaintStroke &st)
{
    const size_t npts = st.points.size() / 3;
    const bool range_only = st.cursor == 3 && st.tool == 0 && npts == 0 && st.has_z_world;
    if ((npts == 0 && !range_only) || st.points.size() % 3 != 0 || st.facets.size() != npts)
        throw std::runtime_error("trait vide ou incohérent");
    if (!(st.radius > 0.f) || !std::isfinite(st.radius))
        throw std::runtime_error("rayon de pinceau invalide");
    if (st.state > 15)
        throw std::runtime_error("état de peinture invalide");
    if (st.cursor > 3)
        throw std::runtime_error("curseur de peinture inconnu");
    if (st.tool > 2)
        throw std::runtime_error("outil de peinture inconnu");
    if (st.tool != 0 && !(st.fill_angle >= -1.f && st.fill_angle <= 90.f))
        throw std::runtime_error("angle de remplissage hors plage (-1 = sans arêtes, 0–90°)");
    if (st.cursor == 3 && !(st.height > 0.f && std::isfinite(st.height)))
        throw std::runtime_error("hauteur de plage invalide");
    if (st.has_z_world && !std::isfinite(st.z_world))
        throw std::runtime_error("cote de plage non finie");
    const int triangles = int(paint_session_triangles(s));
    for (uint32_t f : st.facets)
        if (int(f) >= triangles)
            throw std::runtime_error("facette du trait hors maillage");
    return npts;
}

Slic3r::Vec3f stroke_point(const RawPaintStroke &st, size_t i)
{
    return Slic3r::Vec3f(st.points[3 * i], st.points[3 * i + 1], st.points[3 * i + 2]);
}

// Outils à sélection par propagation (GLGizmoPainterBase.cpp:850-866) :
// smart fill, pot de peinture, curseur triangle (POINTER : bucket fill sans
// propagation, angle -1).
bool is_seed_tool(const RawPaintStroke &st)
{
    return st.tool == 1 || st.tool == 2 || (st.tool == 0 && st.cursor == 2);
}

void seed_select(WebTriangleSelector &selector, const RawPaintStroke &st, size_t i,
                 const Slic3r::Transform3d &trafo_no_translate, const Slic3r::TriangleSelector::ClippingPlane &clp)
{
    const Slic3r::Vec3f hit = stroke_point(st, i);
    const int facet = int(st.facets[i]);
    if (st.tool == 1)
        selector.seed_fill_select_triangles(hit, facet, trafo_no_translate, clp, st.fill_angle, st.highlight_deg, true);
    else if (st.tool == 2)
        selector.bucket_fill_select_triangles(hit, facet, clp, st.fill_angle, true, true);
    else
        selector.bucket_fill_select_triangles(hit, facet, clp, -1.f, false, true);
}

} // namespace

void paint_session_stroke(PaintSession &s, uint8_t channel, const RawPaintStroke &st)
{
    WebTriangleSelector &selector = selector_for(s, channel);
    const size_t npts = check_stroke(s, st);

    const Slic3r::Transform3d trafo = from_col_major16(st.trafo);
    Slic3r::Transform3d trafo_no_translate = trafo;
    trafo_no_translate.translation() = Slic3r::Vec3d::Zero();
    const Slic3r::Vec3f camera(st.camera_pos[0], st.camera_pos[1], st.camera_pos[2]);
    const Slic3r::TriangleSelector::ClippingPlane clp; // inactif
    const auto state = static_cast<Slic3r::EnforcerBlockerType>(st.state);
    if (is_seed_tool(st)) {
        // Clic : la sélection du survol (même point) est appliquée, puis
        // effacée — une application par point du trait.
        for (size_t i = 0; i < npts; ++i) {
            seed_select(selector, st, i, trafo_no_translate, clp);
            selector.seed_fill_apply_on_triangles(state);
            selector.seed_fill_unselect_all_triangles();
        }
        return;
    }
    if (st.tool != 0)
        throw std::runtime_error("outil de peinture inconnu");
    if (st.cursor == 3) {
        // Plage de hauteur (GLGizmoPainterBase.cpp:780-806, get_projected_height_range) :
        // bas de plage = cote monde du point touché ; sur les autres pièces,
        // cote imposée et première facette dont un sommet est dans la plage.
        auto apply = [&](float z_world, int facet) {
            auto cursor = Slic3r::TriangleSelector::SinglePointCursor::cursor_factory(z_world, camera, st.height, trafo, clp);
            selector.select_patch(facet, std::move(cursor), state, trafo_no_translate, st.splitting, st.highlight_deg);
        };
        if (npts == 0) {
            const indexed_triangle_set &its = s.impl->volume->mesh().its;
            const float z_bot_world = st.z_world;
            const float z_top_world = z_bot_world + st.height;
            int first_hit_facet_idx = -1;
            for (int facet_idx = 0; facet_idx < int(its.indices.size()); facet_idx++) {
                const stl_vertex v0 = its.vertices[its.indices[facet_idx].x()];
                const stl_vertex v1 = its.vertices[its.indices[facet_idx].y()];
                const stl_vertex v2 = its.vertices[its.indices[facet_idx].z()];
                const float v0_z = (trafo * Slic3r::Vec3d(v0(0), v0(1), v0(2))).z();
                const float v1_z = (trafo * Slic3r::Vec3d(v1(0), v1(1), v1(2))).z();
                const float v2_z = (trafo * Slic3r::Vec3d(v2(0), v2(1), v2(2))).z();
                const bool outside_range = (v0_z < z_bot_world && v1_z < z_bot_world && v2_z < z_bot_world) ||
                                           (v0_z > z_top_world && v1_z > z_top_world && v2_z > z_top_world);
                if (!outside_range) {
                    first_hit_facet_idx = facet_idx;
                    break;
                }
            }
            if (first_hit_facet_idx != -1)
                apply(z_bot_world, first_hit_facet_idx);
            return;
        }
        for (size_t i = 0; i < npts; ++i) {
            const Slic3r::Vec3f hit = stroke_point(st, i);
            const float z_bot_world = (trafo * Slic3r::Vec3d(hit(0), hit(1), hit(2))).z();
            apply(z_bot_world, int(st.facets[i]));
        }
        return;
    }
    const auto type = st.cursor == 1 ? Slic3r::TriangleSelector::CursorType::SPHERE
                                     : Slic3r::TriangleSelector::CursorType::CIRCLE;
    if (npts == 1) {
        auto cursor = Slic3r::TriangleSelector::SinglePointCursor::cursor_factory(
            stroke_point(st, 0), camera, st.radius, type, trafo, clp);
        selector.select_patch(int(st.facets[0]), std::move(cursor), state, trafo_no_translate,
                              st.splitting, st.highlight_deg);
    } else {
        for (size_t i = 0; i + 1 < npts; ++i) {
            auto cursor = Slic3r::TriangleSelector::DoublePointCursor::cursor_factory(
                stroke_point(st, i), stroke_point(st, i + 1), camera, st.radius, type, trafo, clp);
            selector.select_patch(int(st.facets[i]), std::move(cursor), state, trafo_no_translate,
                                  st.splitting, st.highlight_deg);
        }
    }
}

RawPaintPreview paint_session_preview(PaintSession &s, uint8_t channel, const RawPaintStroke &st, size_t max_triangles)
{
    WebTriangleSelector &selector = selector_for(s, channel);
    const size_t npts = check_stroke(s, st);
    if (!is_seed_tool(st) || npts == 0)
        throw std::runtime_error("aperçu réservé au smart fill, au pot de peinture et au curseur triangle");
    Slic3r::Transform3d trafo_no_translate = from_col_major16(st.trafo);
    trafo_no_translate.translation() = Slic3r::Vec3d::Zero();
    const Slic3r::TriangleSelector::ClippingPlane clp;
    // Survol (SLAGizmoEventType::Moving) : sélection au premier point.
    seed_select(selector, st, 0, trafo_no_translate, clp);
    RawPaintPreview out;
    out.truncated = false;
    selector.seed_fill_selection(max_triangles, out.vertices, out.truncated);
    selector.seed_fill_unselect_all_triangles();
    return out;
}

void paint_session_gap_fill(PaintSession &s, uint8_t channel, float area)
{
    if (!(area >= 0.f && area <= WebTriangleSelector::GapAreaMax))
        throw std::runtime_error("aire de comblement hors plage (0–5 mm²)");
    selector_for(s, channel).gap_fill(area);
}

void paint_session_remap_states(PaintSession &s, uint8_t channel, rust::Slice<const uint8_t> map)
{
    // GLGizmoMmuSegmentation::remap_filament_assignments (l. 1082-1125).
    constexpr size_t MAX_EBT = (size_t) Slic3r::EnforcerBlockerType::ExtruderMax;
    if (map.empty() || map.size() > MAX_EBT)
        throw std::runtime_error("table de remappage vide ou trop longue");
    Slic3r::EnforcerBlockerStateMap state_map;
    for (size_t i = 0; i <= MAX_EBT; ++i)
        state_map[i] = static_cast<Slic3r::EnforcerBlockerType>(i);
    const int start_extruder = (int) Slic3r::EnforcerBlockerType::Extruder1;
    bool any_change = false;
    for (size_t src = 0; src < map.size(); ++src) {
        if (map[src] < 1 || map[src] > MAX_EBT)
            throw std::runtime_error("extrudeur de destination hors plage (1–16)");
        const size_t dst = size_t(map[src]) - 1;
        if (dst != src) {
            state_map[src + start_extruder] = static_cast<Slic3r::EnforcerBlockerType>(dst + start_extruder);
            any_change = true;
        }
    }
    if (!any_change)
        return;
    selector_for(s, channel).remap_triangle_state(state_map);
}

RawPaintFacets paint_session_export(const PaintSession &cs, uint8_t channel)
{
    PaintSession &s = const_cast<PaintSession &>(cs);
    Slic3r::TriangleSelector &selector = selector_for(s, channel);
    Slic3r::FacetsAnnotation &ann = s.impl->volume->supported_facets;
    ann.set(selector);
    RawPaintFacets out;
    for (const auto &m : ann.get_data().triangles_to_split) {
        std::string str = ann.get_triangle_as_string(m.triangle_idx);
        if (str.empty())
            continue;
        out.triangles.push_back(uint32_t(m.triangle_idx));
        out.states.push_back(rust::String(str));
    }
    ann.reset();
    return out;
}

} // namespace webslicer
