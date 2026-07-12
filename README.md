# web-slicer

Slicer 3D **web multi-utilisateurs** visant la **parité totale** avec
[OrcaSlicer](https://github.com/SoftFever/OrcaSlicer) : mêmes 858 paramètres,
même organisation des réglages (21 onglets, 100 sections), mêmes 11 895 presets
système, préparation 3D complète, tranchage, prévisualisation G-code et envoi
vers imprimantes Klipper (Moonraker) — dans le navigateur, avec comptes isolés.

> Le moteur de tranchage encapsule **libslic3r** d'OrcaSlicer (`vendor/OrcaSlicer`,
> lecture seule) : mêmes algorithmes, même G-code.

## Architecture (monorepo 3 couches)

| Couche | Dossier | Stack | Rôle |
|---|---|---|---|
| Moteur | [`engine/`](engine/README.md) | Rust + bridge cxx-FFI | trait `SlicerEngine`, miroir de libslic3r (registre de paramètres généré, tranchage, parsing G-code) |
| Backend | `backend/` | Rust · axum 0.8 · sqlx (SQLite/Postgres) · tower-sessions · argon2 | domaine pur + adaptateurs (stockage, fichiers, Moonraker) ; API HTTP/WebSocket ; file de tranchage |
| Frontend | `frontend/` | SvelteKit 2 · TS strict · Tailwind · Threlte/Three.js | UI de parité ; scène 3D ; prévisualisation ; types API générés par ts-rs |

Principes non négociables : [`.specify/memory/constitution.md`](.specify/memory/constitution.md)
(monorepo 3 couches, trait `SlicerEngine`, trait `Storage`, TDD backend/moteur,
parité totale traçable). Les identifiants de code sont en **anglais**, les
commentaires en **français**.

## Prérequis — environnement Nix

Tout l'outillage (Rust, Bun, clang/cmake/ninja, `orca-slicer`, `$LIBSLIC3R_DIR`,
navigateurs Playwright) est fourni par le flake. **Aucune installation manuelle.**

```sh
nix develop            # entre dans la devShell (bannière d'outils au démarrage)
```

Le moteur v1 se lie aux statiques Nix `libslic3r-headless` (bridge FFI). Vérifier
la chaîne de link C++ :

```sh
nix build .#libslic3r      # → result/lib/*.a + include/
nix build .#dump-config    # valide le link complet
./result/bin/dump-config   # dump du print_config_def runtime (parité, SC-003)
```

Détails FFI/CLI : [`engine/README.md`](engine/README.md).

## Démarrage

```sh
# Backend — http://localhost:3000 (SQLite dans ./data par défaut)
cargo run -p backend

# Frontend — http://localhost:5173 (proxy /api → backend)
cd frontend && bun install && bun dev
```

Premier démarrage : seed automatique des **11 895 presets système** (65 vendeurs).
Le **premier compte créé est administrateur**.

## Qualité (portes de la constitution)

```sh
# Rust (moteur + backend)
cargo fmt --check && cargo clippy --workspace -- -D warnings && cargo test --workspace

# Frontend
cd frontend && bun run lint && bun run check && bun test

# Parité (régénère les inventaires, resynchronise les annexes, vérifie la matrice)
python3 audit/run_all.py
python3 audit/generate_parity_annexes.py
python3 audit/check_traceability.py
```

Commits atomiques par tâche ; TDD pour le backend et le moteur ; tout écart de
parité passe par [`specs/001-orcaslicer-web-parity/exclusions.md`](specs/001-orcaslicer-web-parity/exclusions.md),
jamais par omission (le contrôle échoue sinon).

## Sources de vérité de parité

- `audit/parameters.json` (858 paramètres) · `audit/ui_inventory.json`
  (onglets, menus, gizmos, 92 raccourcis) · `audit/presets_inventory.json`
  (11 895 presets) · `audit/engine_api.json`.
- **Générés, jamais édités à la main** : registre de paramètres Rust
  (`engine/build.rs`), layout UI et types API (`frontend/src/generated/`).
- Matrice et annexes : `specs/001-orcaslicer-web-parity/` (spec, plan, data-model,
  contracts/, exclusions.md, traceability-map.json).

## Fonctionnalités (parité par domaine)

- **Comptes & isolation** — inscription/politique d'instance, sessions, rôles ;
  toute ressource d'autrui est **404** (jamais 403), campagne d'isolation
  automatisée (SC-008 à 100 %).
- **Préparation 3D** — import STL/3MF/STEP/OBJ **décodés côté serveur** (process
  `engine-worker`, maillage d'affichage binaire WSMh ; le frontend n'embarque
  aucun parseur — voir retournement R7), scène Threlte, 16 gizmos,
  move/rotate/scale/cut/arrange/orient, multi-plateaux, réparation de maillage.
- **Réglages** — 21 onglets / 100 sections / 525 lignes d'options fidèles
  (modes simple/advanced/expert), presets dérivables avec héritage et surcharges.
- **Tranchage & file** — file multi-workers, reprise automatique, progression
  temps réel par WebSocket ; export G-code / 3MF.
- **Prévisualisation G-code** — couches typées, 7 colorations, curseurs, stats ;
  chargement paresseux par fenêtre sous budget mémoire (SC-007).
- **Impression (Moonraker)** — déclaration d'imprimante (clé API chiffrée au
  repos), test, envoi (démarrage immédiat), suivi d'état en direct,
  pause/reprise/annulation.

## Validation

- Parcours de bout en bout et protocoles de charge/perf :
  [`specs/001-orcaslicer-web-parity/quickstart.md`](specs/001-orcaslicer-web-parity/quickstart.md).
- Retrouvabilité des paramètres (SC-004, 40 tirés au sort → 100 %) :
  [`specs/001-orcaslicer-web-parity/validation-sc004.md`](specs/001-orcaslicer-web-parity/validation-sc004.md).

## Licence & amont

libslic3r et les profils proviennent d'OrcaSlicer (AGPL-3.0). `vendor/OrcaSlicer`
est **lecture seule** : la parité se mesure contre lui, jamais en le modifiant.
