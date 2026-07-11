# engine — moteur de slicing (trait `SlicerEngine`)

Crate Rust encapsulant libslic3r d'OrcaSlicer derrière le trait `SlicerEngine`
(constitution II).

**Moteur v1 = FFI uniquement.** L'unique implémentation livrée est
`adapters/ffi` (bridge cxx vers `libslic3r-headless`, compilée derrière la
feature `ffi`). Le trait laisse la place à une seconde implémentation, mais
l'adaptateur **CLI `orca-slicer` (`ENGINE_IMPL=cli`) est un objectif backlog
de validation croisée** — non présent en v1 (CLAUDE.md, décision R1 révisée).

## Chaîne de build C++ (`$LIBSLIC3R_DIR`)

Le bridge FFI se lie aux statiques du paquet Nix `libslic3r`
(`libslic3r-headless` : build d'OrcaSlicer sans GUI) :

```sh
nix build .#libslic3r     # → result/lib/*.a + result/include/libslic3r/
nix build .#dump-config   # valide la chaîne de link complète
./result/bin/dump-config  # {"options":751,...} — croisé avec audit/parameters.json
```

Variables fournies par la devShell (`nix develop`) et consommées par
`engine/build.rs` et `tools/dump-config` :

- `LIBSLIC3R_DIR` — racine du paquet : `lib/*.a` (link en `--start-group` à
  cause des dépendances circulaires) + `include/` (headers `libslic3r/…`).
- `ORCA_SRC` — sources d'OrcaSlicer : `src/` (headers des libs sœurs) et
  `deps_src/` (deps vendorées header-only comme `semver/semver.h`).

Bibliothèques externes à lier en plus des statiques (voir
`tools/dump-config/CMakeLists.txt`, référence de la liste exacte) : Boost
(filesystem, system, thread, log, log_setup, locale, regex, chrono, atomic,
date_time, iostreams, nowide), TBB (`tbb`, `tbbmalloc`), OpenSSL (crypto),
GMP, MPFR, JPEG, PNG, ZLIB.

`libslic3r_version.h` est généré par le build d'Orca et absent du paquet :
le consommateur le régénère depuis `include/libslic3r/libslic3r_version.h.in`
(voir le `configure_file` de dump-config).

## Contrôle de parité

`dump-config` expose `--keys` : la liste des clés du `print_config_def`
runtime. Elle doit être **exactement égale** aux groupes `fff`+`common`+`sla`
de `audit/parameters.json` (751 clés — vérifié, y compris les 12 clés
`machine_max_*` générées par la boucle AxisDefault). Ce diff est intégré au
contrôle `audit/check_traceability.py`.

## Tests

```sh
cargo test -p engine                      # unitaires + suite générique du trait
cargo test -p engine --test gcode_parity  # corpus 10×5 vs orca desktop (SC-003)
```

> Backlog : la suite générique du trait est conçue pour rejouer à l'identique
> sur une future implémentation CLI (`ENGINE_IMPL=cli`) à des fins de validation
> croisée ; cet adaptateur n'existe pas en v1.
