// Déclarations cxx : GCodeProcessor d'Orca sur un G-code tranché (gcode_process.cpp).
#pragma once

#include "rust/cxx.h"

namespace webslicer {

struct RawGcodeProcess;

RawGcodeProcess gcode_process_raw(rust::Str path);

} // namespace webslicer
