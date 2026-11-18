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
#include "engine/src/adapters/ffi/bridge/paint.hpp"
#include "engine/src/adapters/ffi/bridge.rs.h"

#include "libslic3r/Model.hpp"
#include "libslic3r/TriangleMesh.hpp"
#include "libslic3r/TriangleSelector.hpp"

#include <array>
#include <cmath>
#include <stdexcept>
#include <string>

namespace webslicer {

struct PaintSessionImpl {
    Slic3r::Model model;
    Slic3r::ModelVolume *volume = nullptr;
    std::array<std::unique_ptr<Slic3r::TriangleSelector>, 4> selectors;
};

namespace {

Slic3r::TriangleSelector &selector_for(PaintSession &session, uint8_t channel)
{
    PaintSessionImpl &s = *session.impl;
    if (channel > 3)
        throw std::runtime_error("canal de peinture inconnu");
    auto &slot = s.selectors[channel];
    if (!slot) {
        if (channel == 2)
            slot = std::make_unique<Slic3r::TriangleSelector>(s.volume->mesh(), 0.2f);
        else
            slot = std::make_unique<Slic3r::TriangleSelector>(s.volume->mesh());
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

void paint_session_stroke(PaintSession &s, uint8_t channel, const RawPaintStroke &st)
{
    Slic3r::TriangleSelector &selector = selector_for(s, channel);
    const size_t npts = st.points.size() / 3;
    if (npts == 0 || st.points.size() % 3 != 0 || st.facets.size() != npts)
        throw std::runtime_error("trait vide ou incohérent");
    if (!(st.radius > 0.f) || !std::isfinite(st.radius))
        throw std::runtime_error("rayon de pinceau invalide");
    if (st.state > 15)
        throw std::runtime_error("état de peinture invalide");
    const int triangles = int(paint_session_triangles(s));
    for (uint32_t f : st.facets)
        if (int(f) >= triangles)
            throw std::runtime_error("facette du trait hors maillage");

    const Slic3r::Transform3d trafo = from_col_major16(st.trafo);
    Slic3r::Transform3d trafo_no_translate = trafo;
    trafo_no_translate.translation() = Slic3r::Vec3d::Zero();
    const Slic3r::Vec3f camera(st.camera_pos[0], st.camera_pos[1], st.camera_pos[2]);
    const auto type = st.sphere ? Slic3r::TriangleSelector::CursorType::SPHERE
                                : Slic3r::TriangleSelector::CursorType::CIRCLE;
    const Slic3r::TriangleSelector::ClippingPlane clp; // inactif
    const auto state = static_cast<Slic3r::EnforcerBlockerType>(st.state);
    auto point = [&](size_t i) {
        return Slic3r::Vec3f(st.points[3 * i], st.points[3 * i + 1], st.points[3 * i + 2]);
    };
    if (st.tool == 1 || st.tool == 2) {
        // Remplissages (GLGizmoPainterBase.cpp:851-866) : sélection par
        // propagation depuis le point, application de l'état, désélection.
        if (!(st.fill_angle >= 0.f && st.fill_angle <= 90.f))
            throw std::runtime_error("angle de remplissage hors plage (0–90°)");
        for (size_t i = 0; i < npts; ++i) {
            if (st.tool == 1)
                selector.seed_fill_select_triangles(point(i), int(st.facets[i]), trafo_no_translate,
                                                    clp, st.fill_angle, st.highlight_deg, true);
            else
                selector.bucket_fill_select_triangles(point(i), int(st.facets[i]), clp,
                                                      st.fill_angle, true, true);
            selector.seed_fill_apply_on_triangles(state);
            selector.seed_fill_unselect_all_triangles();
        }
        return;
    }
    if (st.tool != 0)
        throw std::runtime_error("outil de peinture inconnu");
    if (npts == 1) {
        auto cursor = Slic3r::TriangleSelector::SinglePointCursor::cursor_factory(
            point(0), camera, st.radius, type, trafo, clp);
        selector.select_patch(int(st.facets[0]), std::move(cursor), state, trafo_no_translate,
                              st.splitting, st.highlight_deg);
    } else {
        for (size_t i = 0; i + 1 < npts; ++i) {
            auto cursor = Slic3r::TriangleSelector::DoublePointCursor::cursor_factory(
                point(i), point(i + 1), camera, st.radius, type, trafo, clp);
            selector.select_patch(int(st.facets[i]), std::move(cursor), state, trafo_no_translate,
                                  st.splitting, st.highlight_deg);
        }
    }
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
