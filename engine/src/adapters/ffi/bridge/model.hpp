// Wrappers C++ minces au-dessus de libslic3r (déclarations pour cxx).
#pragma once

#include "rust/cxx.h"

#include <cstddef>

namespace webslicer {

struct RawObject;
struct RawProject;

void init_runtime(rust::Str temp_dir, rust::Str data_dir);

rust::Vec<RawObject> load_model_raw(rust::Str path);

RawProject read_project_3mf_raw(rust::Str path);

void write_project_3mf_raw(const rust::Vec<RawObject> &objects, rust::Str config_json,
                           rust::Str out_path);

size_t model_triangle_count(rust::Str path);

size_t print_config_option_count();

} // namespace webslicer
