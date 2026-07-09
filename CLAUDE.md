# Web-Slicer — contexte agent

Slicer 3D web multi-utilisateurs, parité totale avec OrcaSlicer
(`vendor/OrcaSlicer`, **lecture seule**).

## Gouvernance

- Constitution : `.specify/memory/constitution.md` (v1.0.0) — 5 principes non
  négociables : monorepo 3 couches, trait `SlicerEngine`, trait `Storage`,
  TDD backend/engine, parité totale traçable.
- Feature active : `specs/001-orcaslicer-web-parity/` (spec + plan + research
  + data-model + contracts/ + exclusions.md). `.specify/feature.json` pointe dessus.

## Sources de vérité de parité

- `audit/parameters.json` (858 paramètres), `audit/ui_inventory.json`,
  `audit/presets_inventory.json` (11 895 presets), `audit/engine_api.json`.
- Régénération : `python3 audit/run_all.py` puis
  `python3 audit/generate_parity_annexes.py`. Gate : `audit/check_traceability.py`.
- Registre params, layout UI et types API sont **générés** (engine/build.rs,
  `frontend/src/generated/`) — ne jamais les éditer à la main.

## Stack (imposée)

- `engine/` : crate Rust, trait `SlicerEngine` miroir de libslic3r ; v1 =
  **bridge cxx-FFI** vers les statiques Nix `libslic3r-headless` (slicing dans
  un process `engine-worker` isolé) + fallback CLI `orca-slicer`
  (`ENGINE_IMPL=ffi|cli`) — décision R1 révisée. Livraison en 6 phases
  démontrables (plan.md « Phasage livrable »).
- `backend/` : axum 0.8, sqlx 0.9 (SQLite défaut, Postgres feature),
  tower-sessions, argon2 ; domaine pur dans `src/domain`, adaptateurs isolés.
- `frontend/` : SvelteKit 2 + TS strict + Tailwind, Three.js/Threlte ;
  types API générés par ts-rs ; aucune logique métier dans les composants.

## Commandes

```sh
cargo fmt --check && cargo clippy --workspace -- -D warnings && cargo test --workspace
cd frontend && bun run lint && bun run check && bun test
python3 audit/run_all.py   # contrôle de parité
```

Commits atomiques par tâche ; chaque tâche backend/engine livre ses tests
(TDD). Les écarts de parité passent par `specs/001-orcaslicer-web-parity/exclusions.md`,
jamais par omission.
