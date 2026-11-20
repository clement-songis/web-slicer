// Déclarations cxx de la peinture moteur (paint.cpp) : session TriangleSelector
// d'Orca gardée en mémoire par le worker persistant (`engine-worker serve`).
#pragma once

#include "rust/cxx.h"

#include <cstdint>
#include <memory>

namespace webslicer {

struct RawMesh;
struct RawPaintFacets;
struct RawPaintStroke;
struct RawPaintPreview;
struct PaintSessionImpl;

// Type opaque côté Rust : complet ici (cxx instancie le destructeur de
// UniquePtr dans le code généré), implémentation cachée (pimpl, paint.cpp).
struct PaintSession {
    std::unique_ptr<PaintSessionImpl> impl;
    PaintSession();
    ~PaintSession();
};

std::unique_ptr<PaintSession> paint_session_new(const RawMesh &mesh);
size_t paint_session_triangles(const PaintSession &session);
void paint_session_load(PaintSession &session, uint8_t channel, const RawPaintFacets &facets);
void paint_session_stroke(PaintSession &session, uint8_t channel, const RawPaintStroke &stroke);
RawPaintFacets paint_session_export(const PaintSession &session, uint8_t channel);
void paint_session_gap_fill(PaintSession &session, uint8_t channel, float area);
void paint_session_remap_states(PaintSession &session, uint8_t channel, rust::Slice<const uint8_t> map);
RawPaintPreview paint_session_preview(PaintSession &session, uint8_t channel, const RawPaintStroke &stroke,
                                      size_t max_triangles);
RawPaintFacets remap_painting_raw(const RawMesh &source, const RawMesh &target, const RawPaintFacets &facets);

} // namespace webslicer
