// Déclarations cxx des wrappers maillage (mesh.cpp) hors réparation : la
// simplification quadrique. Séparé de model.hpp pour isoler ce domaine.
#pragma once

#include "rust/cxx.h"

#include <cstdint>

namespace webslicer {

struct RawMesh;
struct RawSimplifyResult;

RawSimplifyResult simplify_mesh_raw(const RawMesh &mesh, uint32_t triangle_count,
                                    float max_error);

// Export Draco (`Slic3r::store_drc`, menu Export … DRC d'Orca).
void store_drc_raw(const RawMesh &mesh, rust::Str path, int32_t bits);

} // namespace webslicer
