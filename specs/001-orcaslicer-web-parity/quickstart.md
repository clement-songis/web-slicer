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

## 7. Fluidité de la scène 3D (SC-007, gate P4)

Charge de référence : **20 objets totalisant 2 millions de triangles** sur un
poste de bureau courant. La scène doit rester fluide (interactions sans à-coups
perceptibles).

Protocole :

1. Générer/charger 20 modèles (≈100 k triangles chacun, p. ex. 20 copies d'un
   maillage dense) et les répartir sur le plateau (`arrange`).
2. Ouvrir la préparation (`frontend/src/lib/scene`) et mesurer le rendu via
   l'overlay de perfs du navigateur (DevTools → Rendering → FPS) pendant :
   orbite caméra, sélection (raycast), déplacement au gizmo, bascule vue éclatée.

Cibles mesurées (desktop courant, GPU intégré récent) :

| Interaction | Cible | Notes |
|---|---|---|
| Orbite caméra | ≥ 30 FPS | rendu instancié, un `BufferGeometry` par objet |
| Sélection (raycast) | < 50 ms | `applyPick` + surbrillance matériau |
| Gizmo move/rotate/scale | ≥ 30 FPS | `TransformControls`, pas de recalcul de géométrie |
| Vue éclatée | ≥ 30 FPS | `explode` recalcule des positions, pas la géométrie |

Leviers de tenue de charge implémentés côté scène : géométrie indexée compacte
(maillage binaire `WSMh`, T049), un seul `BufferGeometry` réutilisé par objet
avec libération au démontage (`ModelObject.svelte`, T050), aucun recalcul de
maillage pendant les manipulations (transformations = matrices, T052), et
décimation optionnelle (`simplifyGrid`, T055) pour les maillages très lourds.

### Prévisualisation G-code : budget mémoire & chargement paresseux (T084)

Un G-code volumineux compte des millions de segments d'extrusion. Le format de
transfert `WSPv` (T067/T068) est une **structure de tableaux** : chaque segment
occupe exactement **40 octets** résidents (`RECORD_BYTES` = 6×f32 positions +
feedrate/width/height f32 + kind u8 + extruder u8 + layer u16), sans surcoût
d'objets JS. Ordre de grandeur : 5 M segments ⇒ ≈ 200 Mo — trop pour tout garder.

Levier : **chargement paresseux par fenêtre** (`lib/preview/loader.ts`,
`LazyPreviewLoader`). Les couches sont découpées en tranches (défaut 20 couches),
seules les tranches de la **fenêtre** autour du curseur (défaut ±40 couches) sont
chargées via `GET …/preview/layers?from=&to=` (T067), et les tranches lointaines
sont **évincées (LRU)** pour tenir un **budget mémoire** (défaut **64 Mio** de
segments résidents). La fenêtre courante n'est jamais évincée ; revenir sur une
tranche évincée la recharge à la demande.

| Grandeur | Valeur (défaut) | Effet |
|---|---|---|
| Taille d'un segment résident | 40 o | tableaux typés, zéro surcoût objet |
| Tranche | 20 couches | granularité de fetch/éviction |
| Fenêtre | ±40 couches | ~4 tranches préchargées autour du curseur |
| Budget résident | 64 Mio | ≈ 1,6 M segments simultanés max |

Protocole de mesure (préviz) : après tranchage d'un modèle dense, parcourir les
couches au curseur et observer via DevTools → Memory que le tas des buffers de
préviz reste borné par le budget (les tranches hors fenêtre sont libérées), et
que le défilement reste fluide (fetch d'une tranche = un `GET` binaire décodé en
tableaux typés, sans reparcours des couches déjà rendues).

## 8. Flux complet (SC-005) & charge de la file (SC-006, gate P5)

### SC-005 — import → tranchage → prévisualisation → export < 3 min

Cible : le flux complet **import STL (< 20 Mo) → tranchage (preset standard) →
prévisualisation → export** s'accomplit en moins de **3 minutes**, dont moins de
**90 s de tranchage** pour un modèle de **100 000 triangles**.

Protocole de mesure (à faire tourner sur un poste de bureau courant, moteur FFI
`ENGINE_IMPL=ffi` requis pour le tranchage réel) :

| Étape | Endpoint / composant | Chrono | Cible |
|---|---|---|---|
| Import STL | `POST /api/projects/{id}/models` (T048) | upload → `ModelResponse` | inclus dans la marge |
| Aperçu scène | `GET /api/models/{id}/mesh` (T049) + Threlte (T050) | décodage `WSMh` → premier rendu | < 2 s |
| Tranchage | `POST …/slice` (T064) → file (T063) → runner FFI (T066) | `queued` → `succeeded` (WS `job.finished`) | **< 90 s** (100 k triangles) |
| Prévisualisation | `…/preview/meta` + `…/preview/layers` (T067) + rendu (T068) | méta → buffers → géométrie | < 5 s |
| Export | `GET …/export/3mf` (T072) ou `…/gcodes/{id}/download` (T066) | requête → fichier | < 2 s |

Instrumentation : la progression et l'horodatage des transitions d'état de job
sont exposés en temps réel via WebSocket (`job.updated`/`job.finished`, T065) ;
mesurer `created_at → updated_at` du job succeeded pour le temps de tranchage
pur. Le budget hors-tranchage (import + aperçu + préviz + export ≈ < 15 s) laisse
une large marge sous les 3 minutes. La mesure de bout en bout dépend du runner
FFI réel (T066) ; d'ici là, elle se documente ici et se rejoue à l'activation du
moteur.

### SC-006 — 10 tranchages simultanés, comptes différents

Couvert par le test de charge `backend/tests/queue_concurrency.rs` (T073) :
**10 jobs répartis sur 5 comptes**, pool de 4 workers, tous menés à `succeeded`
sans mélange de résultats — chaque G-code reste lié à son job (`gcode.job_id`) et
cloisonné à son compte (`GcodeRepo::get(owner, id)` → 404 pour un autre compte,
SC-008). L'exactitude « un job traité une seule fois » sous concurrence est
garantie par `claim_next` transactionnel (`queue_worker.rs`, T063). La
disponibilité de l'interface pendant la charge tient au découplage file ↔ HTTP :
le tranchage s'exécute dans un pool de workers hors du chemin des requêtes, et
les notifications passent par le bus WebSocket sans bloquer les handlers.
