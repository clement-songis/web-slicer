# web-slicer

Trancheur 3D web multi-utilisateur en **parité complète avec
[OrcaSlicer](https://github.com/SoftFever/OrcaSlicer)** : mêmes 858 paramètres,
même organisation des réglages (21 onglets, 100 sections), mêmes 11 895 presets
système, préparation 3D complète, tranchage, aperçu G-code et envoi aux
imprimantes Klipper (Moonraker) — dans le navigateur, comptes isolés.
Le moteur enveloppe **libslic3r** (`vendor/OrcaSlicer`, en lecture seule) :
mêmes algorithmes, même G-code.

## Démarrage rapide

```sh
nix develop                  # tout l'outillage (Rust, Bun, clang, Orca, Playwright)
bash harness/stack.sh up     # backend :18080 + frontend :15173
```

Au premier démarrage, les presets système sont semés automatiquement ; **le
premier compte créé devient administrateur**, un assistant propose les
imprimantes (connexion Moonraker optionnelle, ajoutable plus tard).

## Documentation

| Sujet | Document |
|---|---|
| Index | [docs/README.md](docs/README.md) |
| Architecture + schéma | [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) · [schema](docs/architecture.html) |
| Guide utilisateur | [docs/USAGE.md](docs/USAGE.md) |
| Guide de développement | [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md) (setup, commandes, CI, prod) |
| Parité Orca | [docs/PARITE-ORCA.md](docs/PARITE-ORCA.md) · matrice : [specs/](specs/001-orcaslicer-web-parity/) |
| API HTTP | [backend/API.md](backend/API.md) |
| Moteur (FFI) | [engine/README.md](engine/README.md) |

## Principes

Monorepo 3 couches — `engine/` (Rust + bridge CXX vers libslic3r, sous-processus
`engine-worker`), `backend/` (axum, sqlx, sessions), `frontend/` (SvelteKit,
Threlte). Principes non négociables :
[constitution](.specify/memory/constitution.md). Identifiants en anglais,
commentaires en français. Chaque écart de parité passe par
[exclusions.md](specs/001-orcaslicer-web-parity/exclusions.md), jamais par
omission.

## Licence / amont

libslic3r et les profils viennent d'OrcaSlicer (AGPL-3.0). `vendor/OrcaSlicer`
est en lecture seule : la parité se mesure contre lui, jamais en le modifiant.
