// Wrappers C++ minces au-dessus de libslic3r (déclarations pour cxx).
#pragma once

#include "rust/cxx.h"

#include <cstddef>

namespace webslicer {

size_t model_triangle_count(rust::Str path);

size_t print_config_option_count();

} // namespace webslicer
