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

        orca = pkgs.orca-slicer;

        selfPkgs = self.packages.${pkgs.system};

        libslic3r = orca.overrideAttrs (old: {
          pname = "libslic3r-headless";

          patches = builtins.filter (p: !(lib.hasInfix "webkit" (baseNameOf (toString p)))) old.patches;

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
        };

        pwBrowsers = pkgs.playwright-driver.browsers;
        pwVersion = pkgs.playwright-driver.version;
      in
      {
        packages = {
          inherit orca libslic3r dump-config;
          orca-slicer = orca;
          orca-src = orca.src;
          default = dump-config;
        };

        devShells.default = pkgs.mkShell.override { stdenv = pkgs.clangStdenv; } {
          inputsFrom = [ selfPkgs.libslic3r ];

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
