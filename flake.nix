{
  description = "Slicer web — front SvelteKit/Bun, back Rust, moteur OrcaSlicer";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      flake-utils,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };
        lib = pkgs.lib;

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [
            "rust-src"
            "rust-analyzer"
            "clippy"
            "rustfmt"
          ];
        };

        # Correctifs de déterminisme appliqués à libslic3r (moteur). Ex. :
        # FillConcentricInternal lisait des champs non initialisés de
        # WallToolPathsParams (démontré par Valgrind).
        orcaDeterminismPatches = [
          ./patches/orca-fill-concentric-internal-init-params.patch
        ];

        # Le CLI Orca (oracle) reste celui de nixpkgs : le patcher imposerait une
        # recompilation complète d'OrcaSlicer (GUI incluse) pour le devShell, alors
        # que les refs golden sont figées et capturent déjà la branche déterministe.
        orca = pkgs.orca-slicer;

        # Source Orca main (submodule vendor/OrcaSlicer) pour le portage
        # libslic3r-main. Les correctifs de déterminisme 2.4.1 ne s'appliquent
        # pas encore : premier objectif = compiler, portage ensuite.
        orcaMainSrc = pkgs.fetchFromGitHub {
          owner = "SoftFever";
          repo = "OrcaSlicer";
          rev = "aefb6b3d66d0766dab630af721513967e3fafa3f";
          hash = "sha256-n8oRdioTP7VvRqGT9oGUkA73UaCRH4rBDJJEN/bBVdU=";
        };

        # Expat 2.10.0 : Orca main exige exactement cette version
        # (find_package(EXPAT 2.10.0 REQUIRED), CMakeLists.txt:924) alors que
        # le nixpkgs épinglé par le flake fournit expat 2.8.1.
        expatMain = pkgs.expat.overrideAttrs (old: {
          version = "2.10.0";
          src = pkgs.fetchurl {
            url = "https://github.com/libexpat/libexpat/releases/download/R_2_10_0/expat-2.10.0.tar.gz";
            hash = "sha256-VGajQfnGku96lYpogTEb5Zf44c40WYTPeEFPVHk96kw=";
          };
        });

        # CGAL 6.2 : Orca main exige find_package(CGAL 6.2 REQUIRED)
        # (src/libslic3r/CMakeLists.txt:631) ; le nixpkgs épinglé fournit
        # cgal 6.1.1 et cgal_5 5.6.3 (celui d'orca-slicer 2.4.1). CGAL >= 6
        # calcule ses chemins depuis l'emplacement du CGALConfig.cmake →
        # le patch nixpkgs (layout pré-6.x) ne s'applique plus.
        cgal62 = pkgs.cgal.overrideAttrs (old: {
          version = "6.2.1";
          src = pkgs.fetchurl {
            url = "https://github.com/CGAL/cgal/releases/download/v6.2.1/CGAL-6.2.1.tar.xz";
            hash = "sha256-tr53xgdlqEVjNd6ZHur2/+xVJWmE5Knsxql8N7v+hb8=";
          };
          patches = [ ];
        });

        # OCCT 8.0.1 : Orca main épingle V8_0_1 (deps/OCCT/OCCT.cmake) et
        # son code CAD exige que Standard_Failure dérive de std::exception
        # (GeometryEngine.cpp:89 e.what(), « on OCCT >= 8 ») ; nixpkgs fournit
        # 7.9.3/7.6.2 → override vers 8.0.1. Le patch amont 0001-BRepMesh
        # (qualité de maillage, pas de build) n'est pas reporté ici.
        occt80 = pkgs.opencascade-occt.overrideAttrs (old: {
          version = "8.0.1";
          src = pkgs.fetchFromGitHub {
            owner = "Open-Cascade-SAS";
            repo = "OCCT";
            tag = "V8_0_1";
            hash = "sha256-QgSfrWc6qxbiHBapyGa9+ixRi/dOXqMox8U2f+tCH8o=";
          };
          # OCCT 8 TKService lie -lexpat/-lfreetype/-lfontconfig (pas besoin
          # en 7.9.3) : la recette nixpkgs n'inclut pas ces libs.
          buildInputs = old.buildInputs ++ [
            pkgs.expat
            pkgs.freetype
            pkgs.fontconfig
          ];
          # Comme deps/OCCT/OCCT.cmake d'Orca : tous les modules OFF, seuls
          # les toolkits qu'utilise libslic3r (OCCT_LIBS) sont construits —
          # la compilation complète (5964 cibles) est inutile ici.
          cmakeFlags = old.cmakeFlags ++ [
            (lib.cmakeBool "USE_TK" false)
            (lib.cmakeBool "USE_TBB" false)
            (lib.cmakeBool "USE_FFMPEG" false)
            (lib.cmakeBool "USE_VTK" false)
            (lib.cmakeBool "BUILD_DOC_Overview" false)
            (lib.cmakeBool "BUILD_MODULE_ApplicationFramework" false)
            (lib.cmakeBool "BUILD_MODULE_DataExchange" false)
            (lib.cmakeBool "BUILD_MODULE_Draw" false)
            (lib.cmakeBool "BUILD_MODULE_FoundationClasses" false)
            (lib.cmakeBool "BUILD_MODULE_ModelingAlgorithms" false)
            (lib.cmakeBool "BUILD_MODULE_ModelingData" false)
            (lib.cmakeBool "BUILD_MODULE_Visualization" false)
            # Séparé par des ';' (pas des espaces) : sans __structuredAttrs,
            # le stdenv découpe cmakeFlags au mot — une valeur à espaces ne
            # passerait que le 1er toolkit (les autres devenant des arguments
            # positionnels ignorés par cmake). Le ';' est le séparateur de
            # liste CMake, OCCT le lit comme la liste de toolkits attendue.
            (lib.cmakeFeature "BUILD_ADDITIONAL_TOOLKITS" "TKDESTEP;TKDE;TKXCAF;TKXSBase;TKVCAF;TKCAF;TKLCAF;TKCDF;TKV3d;TKService;TKMesh;TKBO;TKPrim;TKHLR;TKShHealing;TKTopAlgo;TKGeomAlgo;TKBRep;TKGeomBase;TKG3d;TKG2d;TKMath;TKernel;TKFillet;TKOffset;TKBool")
          ];
        });

        # libslvs — solveur de contraintes 2D du Design/CAD (SLIC3R_CAD=ON
        # par défaut dans Orca main, src/libslic3r/CMakeLists.txt:26).
        # Même source et même hash que le pin deps/SLVS/SLVS.cmake d'Orca
        # (zip GitHub, SHA256=1c4bdde9…) ; le CMakeLists.txt du dépôt amont
        # (démo, sans règles d'install) est remplacé par le
        # deps/SLVS/CMakeLists.txt.in d'Orca, comme le fait leur superbuild.
        slvsLib = pkgs.stdenv.mkDerivation {
          pname = "slvs";
          version = "3.0";
          src = pkgs.fetchurl {
            url = "https://github.com/JacobStoren/SolveSpaceLib/archive/4d8704523e4bf212fadf5189f92484244f670fea.zip";
            hash = "sha256-HEvd6cPG7yDqS1C3NgHeVnafLrExs2kn18ZInxAubDA=";
          };
          nativeBuildInputs = [
            pkgs.cmake
            pkgs.unzip
          ];
          postPatch = ''
            cp ${orcaMainSrc}/deps/SLVS/CMakeLists.txt.in CMakeLists.txt
          '';
        };

        selfPkgs = self.packages.${pkgs.system};

        # Basé sur pkgs.orca-slicer (non patché) : les correctifs de
        # déterminisme sont ajoutés explicitement ici, sans double application.
        libslic3r = pkgs.orca-slicer.overrideAttrs (old: {
          pname = "libslic3r-headless";

          patches =
            builtins.filter (p: !(lib.hasInfix "webkit" (baseNameOf (toString p)))) old.patches
            ++ orcaDeterminismPatches;

          cmakeFlags = old.cmakeFlags ++ [
            (lib.cmakeBool "SLIC3R_GUI" false)
            (lib.cmakeBool "SLIC3R_BUILD_TESTS" false)
          ];

          doCheck = false;
          dontWrapGApps = true;
          postBuild = "";
          preFixup = "";
          postInstall = "";
          separateDebugInfo = false;

          buildPhase = ''
            runHook preBuild
            cmake --build . --target libslic3r -j''${NIX_BUILD_CORES:-4}
            runHook postBuild
          '';

          installPhase = ''
            runHook preInstall
            mkdir -p $out/lib $out/include
            # Toutes les statiques produites par le build moteur
            # (libslic3r + deps internes : admesh, clipper, libslic3r_cgal…)
            find . -name '*.a' -exec cp -v {} $out/lib/ \;
            # Headers publics, gardés sous libslic3r/ pour que les
            # consommateurs fassent
            #   #include "libslic3r/Print.hpp"  avec  -I$out/include
            cp -r ../src/libslic3r $out/include/libslic3r
            runHook postInstall
          '';
        });

        dump-config = pkgs.clangStdenv.mkDerivation {
          pname = "orca-dump-config";
          version = orca.version;
          src = ./tools/dump-config;

          nativeBuildInputs = [
            pkgs.cmake
            pkgs.ninja
            pkgs.pkg-config
          ];

          buildInputs = libslic3r.buildInputs;

          LIBSLIC3R_DIR = libslic3r;
          ORCA_SRC = orca.src;
        };

        pwBrowsers = pkgs.playwright-driver.browsers;
        pwVersion = pkgs.playwright-driver.version;

        # Sous-ensemble des deps d'Orca nécessaires au link du bridge FFI
        # (référence : tools/dump-config/CMakeLists.txt).
        # NB : ne PAS utiliser `inputsFrom = [ libslic3r ]` dans le devShell —
        # les ~60 deps d'Orca gonflent NIX_CFLAGS_COMPILE au-delà de la limite
        # kernel par argument et tout exec de cc/gcc échoue en E2BIG
        # (« Argument list too long »).
        ffiDeps = builtins.filter (
          d:
          lib.any (p: lib.hasPrefix p (d.pname or "")) [
            "boost"
            "onetbb"
            "eigen"
            "gmp"
            "mpfr"
            "libpng"
            "cereal"
            "opencascade"
            "libnoise"
            "nlopt"
            "expat"
            "openvdb"
            "opencv"
            "curl"
            "draco"
          ]
        ) (libslic3r.buildInputs or [ ]);

        # Portage libslic3r depuis Orca main : même chaîne headless que
        # libslic3r, mais source = main et sans les patches 2.4.1 (à reporter
        # une fois la compilation validée).
        libslic3r-main = pkgs.orca-slicer.overrideAttrs (old: {
          pname = "libslic3r-headless-main";
          src = orcaMainSrc;
          # Seul patch de build : orcaslicer_copy_sos() (CMakeLists.txt:1194)
          # copie au configure les .so du ffmpeg groupé 7.1.100 sous
          # ${CMAKE_PREFIX_PATH}/lib — layout « bundle » d'Orca, inexistant
          # sous nix (libs du store, autres sonames). Le patch rend ce
          # staging optionnel (no-op si le layout n'existe pas).
          patches = [
            ./patches/orca-main-skip-bundled-ffmpeg-copy.patch
            # OCCT 8.0 (occt80) : Standard_Failure::what() retire,
            # GetMessageString() est l'API universelle.
            ./patches/orca-main-occt-standard-failure-compat.patch
          ];
          # Dépendances dont Orca main exige des versions ≠ nixpkgs :
          # - expat 2.10.0 (nixpkgs : 2.8.1) → expatMain ;
          # - CGAL 6.2 (nixpkgs : 6.1.1 ; cgal_5 5.6.3 dans les inputs
          #   d'orca-slicer) → cgal62, on retire cgal_5 (même pname) ;
          # - Python 3.12.13 EXACT (CMakeLists.txt:1048, nixpkgs python3 =
          #   3.13.x) → python312 (3.12.13 pile) ;
          # - assimp et SLVS : nouvelles deps de main (absentes du paquet
          #   2.4.1) → nixpkgs assimp 6.0.5 et slvsLib (pin Orca).
          # Boost 1.83.0+ et Eigen3 5.0.1 sont déjà couverts par boost186 (1.86)
          # et eigen_5 (5.0.1) des buildInputs d'orca-slicer ; NLopt 1.4+ par
          # nlopt 2.11.0 (config mode) ; OpenVDB est désactivé par défaut
          # (SLIC3R_OPENVDB=0) ; LibDataChannel est conditionné à SLIC3R_GUI.
          buildInputs =
            builtins.filter (
              p:
              (p.pname or "") != "expat"
              && (p.pname or "") != "cgal"
              && (p.pname or "") != "opencascade-occt"
            ) old.buildInputs
            ++ [
              expatMain
              pkgs.python312
              cgal62
              pkgs.assimp
              slvsLib
              occt80
            ];
          # gtk+3-dev propage expat 2.8.1 dans l'environnement de build
          # (nix-support/propagated-build-inputs) : le filtre ci-dessus ne
          # suffit pas, CMake trouverait la 2.8.1 propagée. On épingle donc
          # les variables cache de FindEXPAT sur expat 2.10.0, et on place son
          # include en tête de NIX_CFLAGS_COMPILE pour la compilation.
          env = old.env // {
            NIX_CFLAGS_COMPILE = "-isystem ${lib.getDev expatMain}/include ${old.env.NIX_CFLAGS_COMPILE}";
          };
          # prePatch hérité (2.4.1) : seds FindNLopt.cmake (no-op sur main),
          # PerimeterGenerator.cpp (no-op) et FuzzySkin.cpp
          # ("libnoise/noise.h" → "noise/noise.h", ligne 30). Main ajoute
          # <libnoise/module/modulebase.h> (FuzzySkin.cpp:1) : nixpkgs libnoise
          # installe les headers sous noise/ (pas libnoise/), on étend le sed.
          prePatch = (old.prePatch or "") + ''
            sed -i 's|libnoise/module/|noise/module/|g' src/libslic3r/Feature/FuzzySkin/FuzzySkin.cpp
          '';
          cmakeFlags = old.cmakeFlags ++ [
            (lib.cmakeBool "SLIC3R_GUI" false)
            (lib.cmakeBool "SLIC3R_BUILD_TESTS" false)
            # Orca main télécharge uv pendant configure (CMakeLists.txt:212) :
            # impossible dans le sandbox nix. Binaire nixpkgs fourni.
            "-DORCA_BUNDLED_UV_EXECUTABLE=${pkgs.uv}/bin/uv"
            # FindEXPAT : évite la 2.8.1 propagée par gtk+3-dev (voir env).
            "-DEXPAT_LIBRARY=${expatMain}/lib/libexpat.so"
            "-DEXPAT_INCLUDE_DIR=${lib.getDev expatMain}/include"
            # Le hook cmake de nixpkgs ne passe PAS CMAKE_PREFIX_PATH (pour
            # éviter que cmake trouve des programmes) : find_program ne voit
            # donc pas bin/python3.12 et FindPython3 échoue sur
            # l'interpréteur (Development seul est trouvé via
            # CMAKE_LIBRARY/INCLUDE_PATH). On épingle le préfixe python312.
            "-DCMAKE_PREFIX_PATH=${pkgs.python312}"
          ];
          doCheck = false;
          dontWrapGApps = true;
          # Économise plusieurs Go sur / (débug complet d'Orca) — le portage
          # n'a pas besoin des symboles de debug.
          separateDebugInfo = false;
          # Mêmes phases que libslic3r (2.4.1) : ne construire QUE la cible
          # libslic3r (pas l'app GUI qui exige wx/setup.h) et installer
          # statiques + headers.
          buildPhase = ''
            runHook preBuild
            cmake --build . --target libslic3r -j''${NIX_BUILD_CORES:-4}
            runHook postBuild
          '';
          installPhase = ''
            runHook preInstall
            mkdir -p $out/lib $out/include
            find . -name '*.a' -exec cp -v {} $out/lib/ \;
            cp -r ../src/libslic3r $out/include/libslic3r
            runHook postInstall
          '';
        });
      in
      {

        packages = {
          inherit orca libslic3r dump-config;
          orca-slicer = orca;
          orca-src = orca.src;
          inherit orcaMainSrc libslic3r-main;
          orca-main-src = orcaMainSrc;
          default = dump-config;
        };

        devShells.default = pkgs.mkShell.override { stdenv = pkgs.clangStdenv; } {
          # ffiDeps (issus d'Orca) + libs absentes de ses buildInputs directs
          buildInputs = ffiDeps ++ [
            pkgs.libjpeg
            pkgs.zlib
            pkgs.openssl
          ];

          packages = [
            # --- C++ / build ---
            pkgs.cmake
            pkgs.ninja
            pkgs.pkg-config

            # --- Back ---
            rustToolchain
            pkgs.sqlx-cli
            pkgs.cargo-watch
            pkgs.openssl
            pkgs.sqlite

            # --- Front ---
            pkgs.bun

            # --- Moteur ---
            selfPkgs.orca

            # --- Vérification / agent ---
            pkgs.playwright-driver
            # Harness oracle : générateurs de fixtures/références et Orca CLI
            # headless (le binaire GUI peut sinon seg-faulter sans DISPLAY).
            pkgs.python3
            pkgs.xvfb-run
            pkgs.jq
            pkgs.git
          ];

          env = {
            LIBSLIC3R_DIR = selfPkgs.libslic3r;

            ORCA_SRC = selfPkgs.orca-src;

            PLAYWRIGHT_BROWSERS_PATH = pwBrowsers;
            PLAYWRIGHT_SKIP_VALIDATE_HOST_REQUIREMENTS = "true";

            DATABASE_URL = "sqlite://./data/dev.db";

            RUST_BACKTRACE = "1";
          };

          shellHook = ''
            # Garde-fou 1 : submodule external/OrcaSlicer vs pin nixpkgs.
            # ("keep both in sync when bumping" → test, pas promesse)
            if [ -d external/OrcaSlicer/.git ] || [ -f external/OrcaSlicer/.git ]; then
              pin="${selfPkgs.orca.src.rev or selfPkgs.orca.version}"
              sub=$(git -C external/OrcaSlicer describe --tags --always 2>/dev/null || true)
              case "$sub" in
                *"$pin"*|"$pin"*) : ;;
                *)
                  echo "⚠️  submodule external/OrcaSlicer ($sub) ≠ pin nixpkgs ($pin)."
                  echo "   L'agent lirait un code différent du binaire exécuté. À resynchroniser."
                  ;;
              esac
            fi

            # Garde-fou 2 : version @playwright/test vs playwright-driver nixpkgs
            if [ -f frontend/package.json ]; then
              want="${pwVersion}"
              have=$(${pkgs.jq}/bin/jq -r '.devDependencies["@playwright/test"] // .dependencies["@playwright/test"] // empty' frontend/package.json | tr -d '^~')
              if [ -n "$have" ] && [ "$have" != "$want" ]; then
                echo "⚠️  @playwright/test=$have ≠ playwright-driver=$want (nixpkgs)."
                echo "   Aligne package.json sur $want sinon les navigateurs du store seront refusés."
              fi
            fi

            echo "── devShell slicer-web ───────────────────────────"
            echo " orca-slicer : ${selfPkgs.orca.version} (CLI dans le PATH)"
            echo " clang       : $(clang --version | head -1 | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -1)"
            echo " rust        : $(rustc --version | grep -oE '[0-9]+\.[0-9]+\.[0-9]+')"
            echo " bun         : $(bun --version)"
            echo " playwright  : ${pwVersion} (browsers via store)"
            echo " ORCA_SRC    : $ORCA_SRC"
            echo " LIBSLIC3R   : $LIBSLIC3R_DIR"
            echo "─────────────────────────────────────────────────"
          '';
        };

        checks = {
          inherit (self.packages.${pkgs.system}) libslic3r dump-config;
        };
      }
    );
}
