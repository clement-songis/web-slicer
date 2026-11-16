#!/usr/bin/env bash
# Compile la référence isolée (à lancer dans `nix develop` du dépôt).
set -euo pipefail
cd "$(dirname "$0")"
mkdir -p gen
sed -e 's/@SLIC3R_APP_NAME@/OrcaSlicer/' -e 's/@SLIC3R_APP_KEY@/OrcaSlicer/' -e 's/@SLIC3R_VERSION@/0.0.0-headless/' \
    -e 's/@SoftFever_VERSION@/0.0.0/' -e 's/@SLIC3R_BUILD_ID@/paint-ref/' -e 's/@BBL_INTERNAL_TESTING@/0/' \
    -e 's/@ORCA_CHECK_GCODE_PLACEHOLDERS@/0/' "$LIBSLIC3R_DIR/include/libslic3r/libslic3r_version.h.in" > gen/libslic3r_version.h
EIGEN=$(pkg-config --cflags eigen3 2>/dev/null || echo "-I/nix/store/612s2c89klcpbg5qd6n1mfqynaqfs7fl-eigen-5.0.1/include/eigen3")
OCCT=-I/nix/store/1h5vlvvbwzbr5kyhz6ip38z0ichlv9ai-opencascade-occt-7.6.2/include/opencascade
clang++ -std=c++17 -O1 -w $EIGEN $OCCT -I gen -I "$LIBSLIC3R_DIR/include" -I "$LIBSLIC3R_DIR/include/libslic3r" \
  -I "$ORCA_SRC/src" -I "$ORCA_SRC/deps_src" paint_ref.cpp nanosvg_impl.cpp -o paint_ref \
  -Wl,--start-group "$LIBSLIC3R_DIR"/lib/*.a -Wl,--end-group -lboost_filesystem -lboost_system -lboost_thread -lboost_log -lboost_log_setup -lboost_locale -lboost_regex -lboost_chrono -lboost_atomic -lboost_date_time -lboost_iostreams -lboost_nowide -ltbb -ltbbmalloc -lcrypto -lgmp -lmpfr -ljpeg -lpng -lz -lexpat -ldraco -lnlopt -lTKXDESTEP -lTKSTEP -lTKSTEPBase -lTKSTEPAttr -lTKSTEP209 -lTKXCAF -lTKXSBase -lTKLCAF -lTKCAF -lTKCDF -lTKVCAF -lTKV3d -lTKService -lTKMesh -lTKBO -lTKPrim -lTKShHealing -lTKTopAlgo -lTKGeomAlgo -lTKBRep -lTKGeomBase -lTKG3d -lTKG2d -lTKMath -lTKernel -lnoise-static -lpthread -ldl
./paint_ref circle > circle.txt
./paint_ref sphere > sphere.txt
./paint_ref smart > smart.txt
./paint_ref bucket > bucket.txt
./paint_ref smart1 > smart1.txt
./paint_ref bucket1 > bucket1.txt
wc -l circle.txt sphere.txt smart.txt bucket.txt smart1.txt bucket1.txt
