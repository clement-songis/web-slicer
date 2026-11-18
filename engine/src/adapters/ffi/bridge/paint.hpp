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
RawPaintFacets remap_painting_raw(const RawMesh &source, const RawMesh &target, const RawPaintFacets &facets);

} // namespace webslicer
