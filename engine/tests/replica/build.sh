#!/usr/bin/env bash
# Compile la réplique (à lancer dans nix develop, depuis la racine du dépôt).
set -euo pipefail
cd "$(dirname "$0")"
L="$LIBSLIC3R_DIR"; O="$ORCA_SRC"
V=$(mktemp -d)
sed -e 's/@SLIC3R_APP_NAME@/OrcaSlicer/;s/@SLIC3R_APP_KEY@/OrcaSlicer/;s/@SLIC3R_VERSION@/0.0.0-headless/;s/@SoftFever_VERSION@/0.0.0/;s/@SLIC3R_BUILD_ID@/replica/;s/@BBL_INTERNAL_TESTING@/0/;s/@ORCA_CHECK_GCODE_PLACEHOLDERS@/0/' \
  "$L/include/libslic3r/libslic3r_version.h.in" > "$V/libslic3r_version.h"
EXTRA=()
for tok in $NIX_CFLAGS_COMPILE; do
  for sub in eigen3 opencascade; do [ -d "$tok/$sub" ] && EXTRA+=(-I"$tok/$sub"); done
done
LIBS="boost_filesystem boost_system boost_thread boost_log boost_log_setup boost_locale boost_regex boost_chrono boost_atomic boost_date_time boost_iostreams boost_nowide tbb tbbmalloc crypto gmp mpfr jpeg png z expat draco nlopt TKXDESTEP TKSTEP TKSTEPBase TKSTEPAttr TKSTEP209 TKXCAF TKXSBase TKLCAF TKCAF TKCDF TKVCAF TKV3d TKService TKMesh TKBO TKPrim TKShHealing TKTopAlgo TKGeomAlgo TKBRep TKGeomBase TKG3d TKG2d TKMath TKernel"
clang++ -std=c++17 -O1 -w -DBOOST_LOG_DYN_LINK -I"$L/include" -I"$L/include/libslic3r" -I"$O/src" -I"$O/deps_src" -I"$V" "${EXTRA[@]}" \
  arrange_replica.cpp ../../src/adapters/ffi/bridge/nanosvg_impl.cpp -o ${OUT:-/var/tmp/arrange_replica} \
  -Wl,--start-group "$L"/lib/*.a -Wl,--end-group $(for l in $LIBS; do printf -- '-l%s ' "$l"; done) -lnoise-static -lpthread
