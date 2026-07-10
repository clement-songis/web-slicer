# Quickstart — validation de bout en bout

Guide de validation des scénarios de la spec. Prérequis : **`nix develop`**
(fournit toolchain Rust, Bun, cmake/ninja/clang, et `$LIBSLIC3R_DIR` via le
paquet `libslic3r` du flake). Moteur v1 = **FFI uniquement** (bridge cxx).
Vérification du link C++ : `nix build .#dump-config`.

## 0. Démos par phase (gates de sortie, plan.md « Phasage livrable »)

| Phase | Démo | Commande indicative |
|---|---|---|
| P1 | Slice d'une fixture via le trait (FFI) | `cargo run -p engine --example engine-cli -- slice tests/fixtures/cube20.stl` |
| P2 | Comptes + isolation | `cargo test -p backend --test storage_contract auth_isolation` + UI login |
| P3 | Onglets complets + presets | seed puis comparaison avec `references/orca-prepare.png` ; `check_traceability.py` |
| P4 | Scène multi-plateaux sauvegardée/rouverte | scénario US4 (§4.5) |
| P5 | Tranchage UI + préviz + WS | scénarios US1/US5/US7 (§4.2, 4.6, 4.7) ; cible `references/orca-preview.png` |
| P6 | Envoi Moonraker | scénario US8 (§4.8) |

## 1. Contrôles de parité (avant/après toute session de travail)

```sh
python3 audit/run_all.py                 # régénère les inventaires + contrôles croisés
python3 audit/generate_parity_annexes.py # resynchronise les annexes de la spec
python3 audit/check_traceability.py      # matrice FR-001/002/003 (gate : exit != 0 si écart)
```

Attendu : `run_all.py` byte-identique sur deux exécutions ;
`check_traceability.py` sans écart non justifié par
[exclusions.md](exclusions.md) (SC-001).

## 2. Qualité (gates constitution IV)

```sh
cargo fmt --check && cargo clippy --workspace -- -D warnings && cargo test --workspace
cd frontend && bun run lint && bun run check && bun test
```

## 3. Lancement

```sh
cargo run -p backend        # http://localhost:3000 (SQLite data/ par défaut)
cd frontend && bun dev      # http://localhost:5173 (proxy /api)
```

Premier démarrage : seed automatique des 11 895 presets système (vérifier le
log `presets seeded: {machine_model: 384, machine: 1158, filament: 7012,
process: 3341}` — comptages exacts Annexe C, SC-002). Premier compte créé =
admin.

## 4. Scénarios de validation manuelle

| # | Scénario (US) | Étapes | Attendu |
|---|---|---|---|
| 1 | US6 — comptes | Créer 2 comptes A/B, projet chez A, tenter l'URL du projet A connecté en B | 404 pour B (SC-008) ; persistance après reconnexion |
| 2 | US1 — flux cœur | Importer `engine/tests/fixtures/benchy.stl`, choisir « Bambu Lab A1 0.4 nozzle », trancher, exporter | G-code téléchargé ; < 3 min au total (SC-005) |
| 3 | US2 — parité réglages | Ouvrir Process/Filament/Imprimante, comparer avec OrcaSlicer desktop côte à côte (mêmes onglets/groupes) ; basculer simple→advanced→expert | Organisation identique à l'Annexe B ; visibilité pilotée par `mode` |
| 4 | US3 — presets | Dériver un preset filament, surcharger 2 valeurs, sauvegarder ; vérifier le diff/verrous ; supprimer | Héritage visible, seules les surcharges stockées |
| 5 | US4 — scène | Move/rotate/scale/plat ; couper le benchy en 2 ; réparer un STL cassé ; arranger 10 objets sur 2 plateaux | Chaque outil opérant ; undo/redo ; pas de collision après arrange |
| 6 | US5 — prévisualisation | Après tranchage : parcourir couches, masquer un type de ligne, colorer par vitesse | Types/vitesses/temps conformes aux stats ; curseurs fluides |
| 7 | US7 — file | Lancer 3 tranchages, redémarrer le backend pendant le 2ᵉ | Jobs repris automatiquement, aucun perdu (clarification reprise auto) |
| 8 | US8 — Moonraker | `docker run mainsailcrew/virtual-klipper-printer` (ou instance réelle) ; déclarer, tester, envoyer, pause/annuler | Upload visible dans Mainsail ; états remontés en direct |

## 5. Parité G-code (SC-003, gate P1)

```sh
cargo test -p engine --test gcode_parity -- --nocapture
```

Comparaison sur le corpus `engine/tests/fixtures/` (10 modèles × 5 presets),
après normalisation des métadonnées : **trait FFI vs orca-slicer desktop
enregistré** (diff vide, temps estimés à < 1 %).

## 6. Charge minimale (SC-006)

```sh
cargo test -p backend --test queue_concurrency  # 10 jobs simultanés, 2 comptes
```

Attendu : 10 succès, résultats attribués aux bons comptes, API réactive
pendant l'exécution.
