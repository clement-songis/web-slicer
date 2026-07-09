//! Build du bridge cxx vers libslic3r-headless (feature `ffi` uniquement).
//!
//! Environnement requis (fourni par `nix develop`, cf. engine/README.md) :
//!   - `LIBSLIC3R_DIR` : statiques `lib/*.a` + headers `include/libslic3r/`
//!   - `ORCA_SRC`      : sources d'Orca (`src/`, `deps_src/`)
//!   - `NIX_LDFLAGS`   : chemins `-L` des bibliothèques externes (boost…)
//!
//! Référence de la chaîne de link : tools/dump-config/CMakeLists.txt.

use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    if env::var_os("CARGO_FEATURE_FFI").is_none() {
        return; // build pur Rust : aucun C++
    }

    let libslic3r_dir = PathBuf::from(
        env::var("LIBSLIC3R_DIR").expect("LIBSLIC3R_DIR requis (nix develop) pour --features ffi"),
    );
    let orca_src = PathBuf::from(
        env::var("ORCA_SRC").expect("ORCA_SRC requis (nix develop) pour --features ffi"),
    );
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    // libslic3r_version.h est généré par le build d'Orca, absent du paquet :
    // régénéré ici depuis le .h.in (mêmes valeurs que dump-config).
    let version_dir = out_dir.join("generated");
    fs::create_dir_all(&version_dir).unwrap();
    let template =
        fs::read_to_string(libslic3r_dir.join("include/libslic3r/libslic3r_version.h.in"))
            .expect("libslic3r_version.h.in présent dans le paquet");
    let header = template
        .replace("@SLIC3R_APP_NAME@", "OrcaSlicer")
        .replace("@SLIC3R_APP_KEY@", "OrcaSlicer")
        .replace("@SLIC3R_VERSION@", "0.0.0-headless")
        .replace("@SoftFever_VERSION@", "0.0.0")
        .replace("@SLIC3R_BUILD_ID@", "web-slicer-engine")
        .replace("@BBL_INTERNAL_TESTING@", "0")
        .replace("@ORCA_CHECK_GCODE_PLACEHOLDERS@", "0");
    fs::write(version_dir.join("libslic3r_version.h"), header).unwrap();

    let mut build = cxx_build::bridge("src/adapters/ffi/bridge.rs");
    build
        .file("src/adapters/ffi/bridge/model.cpp")
        .file("src/adapters/ffi/bridge/nanosvg_impl.cpp")
        .file("src/adapters/ffi/bridge/project.cpp")
        .file("src/adapters/ffi/bridge/mesh.cpp")
        .file("src/adapters/ffi/bridge/arrange.cpp")
        .file("src/adapters/ffi/bridge/slice.cpp")
        .std("c++17")
        .include(libslic3r_dir.join("include"))
        .include(libslic3r_dir.join("include/libslic3r"))
        .include(orca_src.join("src"))
        .include(orca_src.join("deps_src"))
        .include(&version_dir)
        .flag_if_supported("-Wno-unused-parameter")
        .flag_if_supported("-Wno-deprecated-declarations");

    // Sous-répertoires d'include non couverts par le wrapper nix
    // (Eigen : include/eigen3 → <Eigen/…> ; OCCT : include/opencascade →
    // <XCAFDoc_….hxx>, requis par Format/STEP.hpp).
    if let Ok(cflags) = env::var("NIX_CFLAGS_COMPILE") {
        let mut tokens = cflags.split_whitespace().peekable();
        while let Some(tok) = tokens.next() {
            if tok == "-isystem" || tok == "-I" {
                if let Some(path) = tokens.peek() {
                    for sub in ["eigen3", "opencascade"] {
                        let dir = PathBuf::from(path).join(sub);
                        if dir.is_dir() {
                            build.include(&dir);
                        }
                    }
                }
            }
        }
    }

    build.compile("webslicer-ffi");

    // Statiques libslic3r : dépendances circulaires → groupe de link.
    println!("cargo:rustc-link-arg=-Wl,--start-group");
    let mut statics: Vec<_> = fs::read_dir(libslic3r_dir.join("lib"))
        .expect("LIBSLIC3R_DIR/lib lisible")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "a"))
        .collect();
    statics.sort();
    for lib in &statics {
        println!("cargo:rustc-link-arg={}", lib.display());
    }
    println!("cargo:rustc-link-arg=-Wl,--end-group");

    // Chemins des bibliothèques externes : repris de NIX_LDFLAGS (devShell).
    if let Ok(ldflags) = env::var("NIX_LDFLAGS") {
        let mut tokens = ldflags.split_whitespace().peekable();
        while let Some(tok) = tokens.next() {
            if let Some(path) = tok.strip_prefix("-L") {
                let path = if path.is_empty() {
                    tokens.next().unwrap_or_default()
                } else {
                    path
                };
                if !path.is_empty() {
                    println!("cargo:rustc-link-search=native={path}");
                }
            }
        }
    }
    for lib in [
        "boost_filesystem",
        "boost_system",
        "boost_thread",
        "boost_log",
        "boost_log_setup",
        "boost_locale",
        "boost_regex",
        "boost_chrono",
        "boost_atomic",
        "boost_date_time",
        "boost_iostreams",
        "boost_nowide",
        "tbb",
        "tbbmalloc",
        "crypto",
        "gmp",
        "mpfr",
        "jpeg",
        "png",
        "z",
        "expat",
        "draco",
        "nlopt",
        // OCCT (lecture STEP, Format/STEP.cpp)
        "TKXDESTEP",
        "TKSTEP",
        "TKSTEPBase",
        "TKSTEPAttr",
        "TKSTEP209",
        "TKXCAF",
        "TKXSBase",
        "TKLCAF",
        "TKCAF",
        "TKCDF",
        "TKVCAF",
        "TKV3d",
        "TKService",
        "TKMesh",
        "TKBO",
        "TKPrim",
        "TKShHealing",
        "TKTopAlgo",
        "TKGeomAlgo",
        "TKBRep",
        "TKGeomBase",
        "TKG3d",
        "TKG2d",
        "TKMath",
        "TKernel",
    ] {
        println!("cargo:rustc-link-lib={lib}");
    }

    // libnoise ne fournit que des archives *-static.a
    println!("cargo:rustc-link-lib=static=noise-static");

    println!("cargo:rerun-if-changed=src/adapters/ffi/bridge.rs");
    println!("cargo:rerun-if-changed=src/adapters/ffi/bridge/model.cpp");
    println!("cargo:rerun-if-changed=src/adapters/ffi/bridge/nanosvg_impl.cpp");
    println!("cargo:rerun-if-changed=src/adapters/ffi/bridge/project.cpp");
    println!("cargo:rerun-if-changed=src/adapters/ffi/bridge/mesh.cpp");
    println!("cargo:rerun-if-changed=src/adapters/ffi/bridge/arrange.cpp");
    println!("cargo:rerun-if-changed=src/adapters/ffi/bridge/slice.cpp");
    println!("cargo:rerun-if-env-changed=LIBSLIC3R_DIR");
    println!("cargo:rerun-if-env-changed=ORCA_SRC");
}
