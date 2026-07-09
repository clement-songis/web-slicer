# Outil de validation de la chaîne de link C++ vers libslic3r-headless.
# Consomme le paquet Nix `libslic3r` via $LIBSLIC3R_DIR (statiques + headers)
# — même mécanisme que le futur bridge cxx de engine/ (décision R1).

cmake_minimum_required(VERSION 3.20)
project(orca-dump-config CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

if(NOT DEFINED ENV{LIBSLIC3R_DIR})
  message(FATAL_ERROR "LIBSLIC3R_DIR doit pointer vers le paquet Nix libslic3r")
endif()
set(LIBSLIC3R_DIR $ENV{LIBSLIC3R_DIR})

# libslic3r_version.h est généré par le build d'OrcaSlicer (configure_file) et
# n'est pas installé dans le paquet : on le régénère ici depuis le .h.in fourni.
set(SLIC3R_APP_NAME "OrcaSlicer")
set(SLIC3R_APP_KEY "OrcaSlicer")
set(SLIC3R_VERSION "0.0.0-headless")
set(SoftFever_VERSION "0.0.0")
set(SLIC3R_BUILD_ID "web-slicer-dump-config")
set(BBL_INTERNAL_TESTING "0")
set(ORCA_CHECK_GCODE_PLACEHOLDERS "0")
configure_file(
  ${LIBSLIC3R_DIR}/include/libslic3r/libslic3r_version.h.in
  ${CMAKE_CURRENT_BINARY_DIR}/generated/libslic3r_version.h
  @ONLY)

add_executable(dump-config main.cpp)

if(NOT DEFINED ENV{ORCA_SRC})
  message(FATAL_ERROR "ORCA_SRC doit pointer vers les sources d'OrcaSlicer")
endif()
set(ORCA_SRC $ENV{ORCA_SRC})

target_include_directories(dump-config PRIVATE
  ${LIBSLIC3R_DIR}/include
  ${LIBSLIC3R_DIR}/include/libslic3r   # includes internes relatifs
  ${ORCA_SRC}/src                      # headers des libs sœurs (slic3r/…)
  ${ORCA_SRC}/deps_src                 # deps vendorées header-only (semver/…)
  ${CMAKE_CURRENT_BINARY_DIR}/generated)

find_package(Eigen3 REQUIRED)
find_package(OpenSSL REQUIRED)
find_library(GMP_LIB gmp REQUIRED)
find_library(MPFR_LIB mpfr REQUIRED)
find_package(JPEG REQUIRED)
find_package(PNG REQUIRED)
find_package(ZLIB REQUIRED)
find_package(Boost REQUIRED COMPONENTS
  filesystem system thread log log_setup locale regex chrono atomic date_time iostreams nowide)
find_package(TBB REQUIRED)
find_package(Threads REQUIRED)

file(GLOB LIBSLIC3R_STATICS ${LIBSLIC3R_DIR}/lib/*.a)

# Les statiques ont des dépendances circulaires entre elles : groupe de link.
target_link_libraries(dump-config PRIVATE
  -Wl,--start-group ${LIBSLIC3R_STATICS} -Wl,--end-group
  Boost::filesystem Boost::system Boost::thread Boost::log Boost::log_setup
  Boost::locale Boost::regex Boost::chrono Boost::atomic Boost::date_time
  Boost::iostreams
  TBB::tbb TBB::tbbmalloc
  Eigen3::Eigen
  OpenSSL::Crypto JPEG::JPEG PNG::PNG ZLIB::ZLIB Boost::nowide
  ${GMP_LIB} ${MPFR_LIB}
  Threads::Threads
  ${CMAKE_DL_LIBS})

install(TARGETS dump-config RUNTIME DESTINATION bin)
