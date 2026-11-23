# Architecture de web-slicer

> Schéma complet : [architecture.html](./architecture.html) (autonome, s'ouvre dans le navigateur).

web-slicer est un trancheur 3D web en parité fonctionnelle et G-code avec
OrcaSlicer (référence : release 2.4.1 via le pin nix ; le submodule
`vendor/OrcaSlicer` suit `main` comme référence de code).

## Vue d'ensemble

```
Navigateur (SvelteKit 3, Threlte/three)
  ⇅ HTTP /api + WebSocket /api/ws
Serveur frontal adapter-node (server.js, hooks CSRF, proxy /api)
  ⇅
Backend axum (Rust)
  ├─ AnyStorage (SQLite dev / Postgres prod) + FileStore (modèles, auxiliaires)
  ├─ crate engine (Rust pur) : registre de paramètres, presets, writer 3MF
  └─ sous-processus engine-worker (feature ffi) : bridge CXX → libslic3r
```

## Modules

### Frontend (`frontend/`)
- **SvelteKit 3** (adapter-node), **Threlte/three** pour la scène 3D
  (gizmos cut/simplify/paint, aperçu par couche), **Paraglide** pour l'i18n.
- `server.js` (prod) relaie `/api/*` vers le backend et le WebSocket `/api/ws` ;
  `hooks.server.ts` réapplique le contrôle CSRF aux routes SvelteKit.
- Bindings TypeScript générés par ts-rs (`src/generated/api`, jamais édités à la main).

### Backend (`backend/`, axum)
- Routes API : projets, scène, tranchage, export 3MF/G-code, presets,
  aperçu, calibration, préférences, imprimantes (Moonraker), file d'attente.
- Sessions cookies, isolation stricte par compte (SC-008 : 404 pour autrui).
- `AnyStorage` : SQLite (dev/tests) ou Postgres ; `FileStore` : modèles,
  vignettes, auxiliaires de projet.

### Crate engine (`engine/`)
- **Rust pur** : registre de paramètres (défauts Orca, plages, alias),
  résolution de chaînes de presets, writer 3MF compatible Orca
  (`Metadata/model_settings.config`, plateaux, purges, timelapse…).
- **Feature `ffi`** : bridge CXX vers libslic3r (statiques) et binaire
  `engine-worker` (`required-features = ["ffi"]`).

### Sous-processus engine-worker
- Le backend spawn `engine-worker` pour découpe, arrangement, réparation,
  split, peinture : un crash C++ de libslic3r est contenu (kill + erreur
  typée), l'annulation se fait par kill, chaque opération tourne dans un
  process neuf.
- Résolution du binaire : `ENGINE_WORKER_BIN` s'il est défini, sinon voisin
  de l'exécutable (`target/debug/engine-worker` en dev). C'est pourquoi la
  CI construit les tests avec `--features engine/ffi`.

### Toolchain Nix (`flake.nix`)
- devShell : `LIBSLIC3R_DIR` (libslic3r-headless, statiques + headers) et
  `ORCA_SRC` (source Orca **2.4.1** épinglé par nixpkgs + patches de
  déterminisme).
- Le submodule `vendor/OrcaSlicer` suit la branche `main` en amont :
  référence de **lecture** du code, pas la source du binaire.
- En CI, le job Rust source `nix print-dev-env` (l'invocation
  `nix develop --command` perd des variables et les flags du wrapper clang
  sur les runners GitHub).

### Harness & oracle de parité (`harness/`, `tests/oracle/`)
- `harness/stack.sh` démarre backend (18080) + frontend (15173).
- Corpus de parité : 634 cas (base + golden + un cas par paramètre), rejoués
  contre les G-code de référence d'Orca via l'API `/api/_harness`.
- `tests/oracle/refs/` (golden 8 Mo, param 354 Mo) est ignoré par git ;
  seul `base_config.json` est suivi.

### CI (`.github/workflows/ci.yml`)
Trois jobs : **Parité (audit)** (inventaires déterministes Python), **Backend +
engine** (fmt, clippy `-D warnings`, `cargo test --workspace --features
engine/ffi` dans le devShell), **Frontend** (lint, check, tests Bun/Vitest).

## Flux principaux

1. **Import** : STL/OBJ/3MF/STEP → décodage (STL côté backend, le reste via
   engine-worker) → scène persistée.
2. **Tranchage** : presets actifs résolus + surcharges projet → `SliceRequest`
   → engine-worker → G-code + aperçu par couche.
3. **Export 3MF** : même modèle de scène et la même configuration figée que le
   tranchage → writer 3MF compatible Orca (relisible par l'import).
4. **Envoi imprimante** : upload du G-code à un Moonraker (`print=true`),
   statut et contrôles ; toute panne devient 502.

## Ports et variables

| Élément | Valeur |
|---|---|
| Backend dev | `http://127.0.0.1:18080` |
| Frontend dev | `http://127.0.0.1:15173` |
| `ENGINE_WORKER_BIN` | override du chemin du worker |
| `BACKEND_URL` | cible du proxy frontal (prod) |
| `MOONRAKER_URL` | imprimante virtuelle des tests |
