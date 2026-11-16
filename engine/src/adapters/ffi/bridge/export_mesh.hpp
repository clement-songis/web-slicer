// Déclarations cxx : maillages d'export STL comme Plater::export_stl d'Orca.
#pragma once

#include "rust/cxx.h"

namespace webslicer {

struct RawObject;
struct RawMesh;

rust::Vec<RawMesh> export_meshes_raw(const rust::Vec<RawObject> &objects);

} // namespace webslicer
