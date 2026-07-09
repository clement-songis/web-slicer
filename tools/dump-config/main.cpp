// Valide la chaîne de link vers libslic3r-headless et dump un résumé du
// registre de configuration, croisable avec audit/parameters.json.
#include "libslic3r/PrintConfig.hpp"

#include <cstdio>
#include <cstring>

int main(int argc, char **argv)
{
    using namespace Slic3r;

    const auto &options = print_config_def.options;

    if (argc > 1 && std::strcmp(argv[1], "--keys") == 0) {
        for (const auto &[key, def] : options)
            std::printf("%s\n", key.c_str());
        return 0;
    }

    std::size_t with_enum   = 0;
    std::size_t nullable    = 0;
    for (const auto &[key, def] : options) {
        if (!def.enum_values.empty())
            ++with_enum;
        if (def.nullable)
            ++nullable;
    }

    const bool has_layer_height   = options.count("layer_height") != 0;
    const bool has_infill_pattern = options.count("sparse_infill_pattern") != 0;

    std::printf("{\"options\":%zu,\"with_enum_values\":%zu,\"nullable\":%zu,"
                "\"has_layer_height\":%s,\"has_sparse_infill_pattern\":%s}\n",
                options.size(), with_enum, nullable,
                has_layer_height ? "true" : "false",
                has_infill_pattern ? "true" : "false");

    return (has_layer_height && has_infill_pattern) ? 0 : 1;
}
