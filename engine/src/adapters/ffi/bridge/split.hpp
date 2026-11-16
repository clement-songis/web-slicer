// Déclarations cxx des wrappers de découpe (split.cpp) : séparation en
// composantes connexes (TriangleMesh::split), coupe plane (cut_mesh) et
// booléen mcut (MeshBoolean::mcut::make_boolean). Domaine isolé de mesh.hpp.
#pragma once

#include "rust/cxx.h"

#include <cstdint>

namespace webslicer {

struct RawMesh;
struct RawCutPlane;
struct RawCutResult;

rust::Vec<RawMesh> split_mesh_raw(const RawMesh &mesh);
RawCutResult cut_mesh_raw(const RawMesh &mesh, const RawCutPlane &plane);
RawMesh boolean_mesh_raw(const RawMesh &a, const RawMesh &b, uint8_t op);
RawMesh smooth_mesh_raw(const RawMesh &mesh);
RawMesh emboss_text_raw(rust::Str font_path, rust::Str text, float size_mm, float depth_mm,
                        float boldness_mm);
RawMesh emboss_svg_raw(rust::Str svg, float width_mm, float depth_mm);
RawMesh make_primitive_raw(uint8_t shape, double side_mm);

} // namespace webslicer
