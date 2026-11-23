# Guide de développement — web-slicer

> Vue d'ensemble : [ARCHITECTURE.md](./ARCHITECTURE.md) · schéma :
> [architecture.html](./architecture.html) · parité :
> [PARITE-ORCA.md](./PARITE-ORCA.md) · API : ../backend/API.md

## Environnement (Nix)

```bash
nix develop          # ou : source .work/devenv.sh && . .work/env.sh (cache)
bash harness/stack.sh up    # backend :18080 + frontend :15173
```

Le devShell fournit `LIBSLIC3R_DIR` (statiques libslic3r-headless, pin
nixpkgs **Orca 2.4.1** + patches de déterminisme) et `ORCA_SRC` (source
Orca). Le submodule `vendor/OrcaSlicer` (branche main) sert de
référence de lecture du code.

## Commandes utiles

| Tâche | Commande |
|---|---|
| Tests Rust (avec worker) | `cargo test --workspace --features engine/ffi` |
| Clippy strict | `cargo clippy --workspace --all-targets -- -D warnings` |
| Front : types | `cd frontend && bun run check` |
| Front : tests | `bunx vitest run` |
| Front : lint | `bun run lint` |
| Stack locale | `bash harness/stack.sh up` |

Le binaire `engine-worker` (feature ffi) est spawné par le backend via
la résolution de voisin (`target/debug/engine-worker`) ou
`ENGINE_WORKER_BIN` — d'où le drapeau `--features engine/ffi` pour les
tests.

## CI (GitHub Actions)

Trois jobs : **Parité (audit)**, **Backend + engine** (fmt, clippy,
tests dans le devShell — le job source `nix print-dev-env` car
`nix develop --command` perd des variables d'env et les flags du
wrapper clang sur les runners), **Frontend** (lint, check, tests).

## Production (adapter-node)

```sh
cargo build --release -p backend                  # écoute :8080 par défaut
cd frontend && bun install --frozen-lockfile && bun run build
PORT=3000 HOST=0.0.0.0 ORIGIN=https://slicer.example.com \
BACKEND_URL=http://127.0.0.1:8080 node server.js
```

- `ORIGIN` : URL publique vue par les navigateurs (contrôle CSRF SvelteKit).
- `BACKEND_URL` : base du backend (défaut `http://127.0.0.1:8080`).
- Utiliser `server.js` (et pas `node build`) : lui seul relaie le WebSocket
  `/api/ws` (progression des imports/tranchages).

## Conventions du dépôt

- Bindings ts-rs générés (`frontend/src/generated/api`) : jamais édités
  à la main ( gate CI `git diff --exit-code` ).
- Refs oracle (`tests/oracle/refs/`) hors git (354 Mo) sauf
  `base_config.json` ; régénérées via `tools/generate_oracle_refs.py`.
- Le gel du harness (`harness/loopctl.py freeze`) doit être refait
  après toute modification de chemins gelés (`HARNESS_UNLOCK=1`).
- Un seul `CARGO_TARGET_DIR` partagé (`web-slicer/target`) ; purge via
  `node .work/tmp/sweep.js go`. Les gates lourdes passent sur la CI.
