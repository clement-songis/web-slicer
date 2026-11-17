// Calcul des volumes de purge depuis les couleurs (Orca FlushVolCalculator).
#pragma once

#include "rust/cxx.h"

namespace webslicer {

rust::Vec<double> flush_volumes_raw(const rust::Vec<rust::String> &colours, const rust::Vec<bool> &is_support,
                                    const rust::Vec<int32_t> &min_flush, int32_t flush_dataset);

rust::Vec<int32_t> min_flush_volumes_raw(rust::Str config_json, uint32_t nozzle_id);

} // namespace webslicer
