# Implementation Plan: Web-Slicer — parité OrcaSlicer multi-utilisateurs

**Branch**: `001-orcaslicer-web-parity` | **Date**: 2026-07-09 (mis à jour) | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/001-orcaslicer-web-parity/spec.md`
+ directives utilisateur : stack imposée (SvelteKit/TS strict/Tailwind/Threlte,
axum/sqlx/tower, **engine cxx-FFI vers libslic3r avec fallback CLI**, WebSocket
progression, build Nix) et découpage en 6 phases livrables démontrables.

## Summary

Application web multi-utilisateurs répliquant l'intégralité d'OrcaSlicer :
import STL/3MF/STEP/OBJ, scène 3D complète, onglets de réglages avec les
846 paramètres du registre, 11 895 presets système avec héritage, tranchage
serveur en file persistante, prévisualisation G-code par couches, export et
envoi Moonraker. Le moteur est un **bridge cxx-FFI vers libslic3r** (statiques
`libslic3r-headless` déjà produites par `flake.nix`) avec **fallback CLI**
`orca-slicer` derrière le même trait `SlicerEngine`. La parité est garantie
par construction (codegen depuis `audit/*.json`) et vérifiée par une matrice
de traçabilité automatisée (FR-003). Livraison en 6 phases démontrables.

## Technical Context

**Language/Version**: Rust 2021 (workspace `backend` + `engine`), C++ (bridge cxx vers
libslic3r vendored), TypeScript strict (frontend)

**Primary Dependencies**: axum 0.8 + tower, sqlx 0.9 (sqlite + postgres), tower-sessions,
argon2, ts-rs 12 ; SvelteKit 2 + Svelte 5 + Tailwind + Threlte (Three.js) ; engine : cxx
(bridge FFI), zip + quick-xml (3MF), glam, tempfile/which (fallback CLI `orca-slicer`)

**Build**: Nix flake existant — paquet `libslic3r` (= `libslic3r-headless` : statiques
`*.a` + headers `libslic3r/`), devShell clangStdenv avec cmake/ninja ; l'outil
`tools/dump-config` valide déjà la chaîne de link C++

**Storage**: trait `Storage` (constitution III) — SQLite par défaut, Postgres par feature
flag/config ; fichiers utilisateur sur filesystem (`data/users/<id>/…`)

**Testing**: cargo test (TDD, constitution IV) dont **tests de parité contre l'API C++**
(phase 1) ; vitest + svelte testing library ; tests de contrat HTTP ; contrôle de parité
`audit/run_all.py` + `audit/check_traceability.py`

**Target Platform**: serveur Linux (NixOS dev, build reproductible Nix), navigateurs
desktop récents (WebGL2)

**Project Type**: web-service (backend + frontend) + crate moteur FFI

**Performance Goals**: SC-005 (flux complet < 3 min, tranchage < 90 s / 100 k triangles),
SC-006 (10 tranchages simultanés), SC-007 (scène fluide à 2 M triangles / 20 objets) ;
progression de slicing relayée en WebSocket

**Constraints**: parité normative Annexes A/B/C ; G-code fonctionnellement identique à
OrcaSlicer (SC-003) ; isolation stricte entre comptes (SC-008) ; `vendor/OrcaSlicer` en
lecture seule (le build Nix part de `pkgs.orca-slicer` patché headless)

**Scale/Scope**: mono-instance ~50 utilisateurs actifs ; uploads ≤ 500 Mo ; pas de quota v1

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principe | Conformité du plan |
|---|---|
| I. Monorepo trois couches, séparation stricte | Domaine backend dans `backend/src/domain`, adaptateurs isolés (`adapters/`), handlers HTTP minces (`http/`), composants Svelte sans logique métier. ✅ |
| II. Moteur derrière `SlicerEngine` | Trait miroir de `audit/engine_api.json` (contrat `contracts/slicer-engine-trait.md`) ; v1 = **bridge cxx-FFI** (`adapters/ffi`, décision R1 révisée) + **fallback CLI** (`adapters/cli`) sélectionné par config/détection ; le backend n'importe que le trait ; substituabilité testée par suite générique. ✅ |
| III. Persistance derrière `Storage` | Traits de repositories, impl sqlx SQLite défaut, Postgres feature (R8). ✅ |
| IV. TDD & qualité | Phase 1 = engine wrapper **avec tests de parité contre l'API C++** avant tout backend ; gates clippy `-D warnings`, rustfmt, eslint + prettier ; commits atomiques par tâche. ✅ |
| V. Parité totale | Registre/layout/presets générés depuis `audit/*.json` (R2, R3) ; matrice `audit/check_traceability.py` (R4) ; registre d'exclusions : [exclusions.md](exclusions.md). ✅ |

**Post-design re-check (Phase 1)** : contrats inchangés côté backend/frontend — le
passage FFI-first est interne à `engine/` (le trait est identique), aucun écart. ✅

## Phasage livrable (directive utilisateur)

Chaque phase est démontrable indépendamment ; sa démo et son gate de sortie
sont définis ici et repris par quickstart.md. Le découpage en tâches
(/speckit-tasks) doit s'aligner sur ces phases.

| Phase | Contenu | Démo (indépendante) | Gate de sortie |
|---|---|---|---|
| **P1 — Engine wrapper** | Trait `SlicerEngine`, bridge cxx-FFI (Model/Print/slice/arrange/orient/repair/3MF/STEP), fallback CLI, codegen registre params (R2), parseur G-code (R6) | Binaire de démo `engine-cli` : slice d'un STL de fixture via le trait (FFI puis CLI), G-code + stats en sortie | Tests de parité contre l'API C++ verts (corpus 10×5, SC-003) ; suite générique du trait passe sur FFI **et** CLI |
| **P2 — Auth + storage** | Trait `Storage`, impls sqlite/postgres, migrations, sessions, argon2, politique d'inscription, comptes admin, espaces fichiers | Inscription/connexion via l'UI minimale ; 2 comptes, isolation prouvée (tests SC-008) | Suite de contrat Storage verte sur les 2 backends ; tests d'isolation verts |
| **P3 — Presets + onglets paramètres** | Seed des 11 895 presets, héritage résolu (R5), clés legacy, API presets, codegen layout UI (R3), rendu des onglets (modes simple/advanced/expert), presets utilisateur | Ouvrir les onglets Process/Filament/Imprimante complets, comparer côte à côte avec `references/orca-prepare.png` ; dériver un preset | Comptages == Annexe C (SC-002) ; `check_traceability.py` vert sur params+UI réglages |
| **P4 — Scène 3D + plater** | Import STL/OBJ/3MF/STEP, Threlte : plateau/objets/gizmos, transformations + undo/redo, coupe/réparation/booléens, arrange/orient (via engine), multi-plateau, liste d'objets, projets (sauvegarde manuelle + brouillon) | Préparer une scène multi-objets multi-plateaux et la sauvegarder/rouvrir | Scénarios US4 verts ; SC-007 mesuré |
| **P5 — Slicing + préviz G-code** | File persistante + workers (R9), WebSocket progression, préviz par couches (buffers R6, 7 colorations, curseurs), stats, export G-code/3MF | Trancher depuis l'UI, suivre la progression WS, explorer la préviz (cible : `references/orca-preview.png`), télécharger | Scénarios US1/US5/US7 verts ; SC-005/SC-006 mesurés ; reprise au reboot testée |
| **P6 — Intégration Moonraker** | Adaptateur Moonraker (R10), déclaration d'imprimantes, upload/start, suivi état WS, pause/reprise/annulation | Envoi vers `virtual-klipper-printer` (ou imprimante réelle), contrôle depuis l'UI | Scénarios US8 verts contre l'instance simulée en CI |

Dépendances : P1 → P5 (moteur), P2 → P3/P4/P5/P6 (auth), P3 → P5 (presets résolus),
P4 → P5 (scène à trancher). P3 et P4 sont parallélisables après P2.

## Project Structure

### Documentation (this feature)

```text
specs/001-orcaslicer-web-parity/
├── plan.md              # Ce fichier
├── spec.md              # Spécification (+ annexes/, references/)
├── research.md          # Phase 0 : décisions R1–R11
├── data-model.md        # Entités et relations
├── quickstart.md        # Guide de validation (par phase)
├── exclusions.md        # Registre d'exclusions de parité
├── contracts/           # http-api, slicer-engine-trait, storage-trait
└── tasks.md             # Phase 2 (/speckit-tasks)
```

### Source Code (repository root)

```text
engine/                      # Crate moteur (constitution II) — Phase P1
├── src/
│   ├── lib.rs               # Trait SlicerEngine + sélection d'implémentation
│   ├── api/                 # Types miroirs de libslic3r (Model, Print, PrintObject…)
│   ├── params/              # Registre des 846 paramètres (généré, R2) + legacy
│   ├── presets/             # Héritage, profils vendor/user (P3)
│   ├── threemf/             # Lecture/écriture 3MF projets Orca
│   ├── gcode/               # Parseur G-code → couches/segments/stats (R6)
│   └── adapters/
│       ├── ffi/             # v1 PRINCIPAL : bridge cxx → libslic3r-headless (Nix)
│       │   └── bridge/      # code C++ (cxx::bridge, wrappers minces)
│       └── cli/             # FALLBACK : orca-slicer CLI (même trait)
├── build.rs                 # Codegen params + link libslic3r ($LIBSLIC3R_DIR Nix)
├── examples/engine-cli.rs   # Démo P1
└── tests/                   # Parité API C++ + G-code de référence

backend/                     # Phases P2, P3, P5, P6
├── src/
│   ├── main.rs              # Composition root (Storage + SlicerEngine ffi|cli)
│   ├── domain/              # users, projects, presets, jobs, printers
│   ├── adapters/storage/    # sqlite/ (défaut), postgres/ (feature)
│   ├── adapters/files/      # Espace fichiers par utilisateur
│   ├── adapters/moonraker/  # Client REST + WS (P6)
│   ├── queue/               # File persistante, workers, reprise (R9)
│   ├── auth/                # Sessions, argon2, politique d'inscription
│   └── http/                # Handlers minces + DTO ts-rs + WS progression
├── migrations/{sqlite,postgres}/
└── tests/

frontend/                    # Phases P2 (minimal), P3, P4, P5, P6
├── src/
│   ├── lib/
│   │   ├── api/             # Client typé (ts-rs) + client WS
│   │   ├── settings/        # Onglets rendus depuis le layout généré (P3)
│   │   ├── scene/           # Threlte : plateau, gizmos, outils (P4)
│   │   ├── preview/         # Préviz G-code par couches (P5)
│   │   ├── presets/         # Sélecteurs, héritage, diff (P3)
│   │   └── stores/          # projet, sélection, file
│   ├── routes/              # auth, bibliothèque, éditeur, file, imprimantes
│   └── generated/           # params.ts, ui-layout.ts, types API (générés)
└── tests/

audit/
└── check_traceability.py    # (nouveau, R4) gate FR-001/002/003

flake.nix                    # Existant : libslic3r-headless, devShell, dump-config
tools/dump-config/           # Existant : validation de la chaîne C++
```

**Structure Decision**: inchangée sur le fond (3 unités, artefacts de parité
générés) ; `engine/adapters/ffi` devient l'implémentation par défaut, liée aux
statiques du paquet Nix `libslic3r` via `$LIBSLIC3R_DIR` (mécanisme déjà
éprouvé par `tools/dump-config`) ; `adapters/cli` reste compilé et sélectionnable
par config (`ENGINE_IMPL=ffi|cli`) comme filet et outil de comparaison.

## Complexity Tracking

Aucune violation constitutionnelle. Choix tracés :

| Choix | Pourquoi | Alternative rejetée |
|---|---|---|
| FFI cxx **et** fallback CLI maintenus tous deux | Directive utilisateur + constitution II ; le CLI sert de filet (crash FFI, environnement sans lib) et d'oracle de parité croisée dans les tests | FFI seul : perd l'oracle de comparaison et le mode dégradé ; CLI seul : perd les callbacks de progression fins, la conversion STEP directe et les perfs (pas de re-fork par job) |
| Codegen depuis `audit/*.json` | Seule façon de tenir 846 paramètres + 525 options d'UI sans dérive | Écriture manuelle : ingérable, invérifiable |
| Phasage 6 livrables | Directive utilisateur ; chaque phase a une démo et un gate | Livraison big-bang : risque d'intégration massif |
