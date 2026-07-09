# Tasks: Web-Slicer — parité OrcaSlicer multi-utilisateurs

**Input**: Design documents from `/specs/001-orcaslicer-web-parity/`

**Prerequisites**: plan.md (phasage livrable P1–P6), spec.md (US1–US8), research.md (R1–R11), data-model.md, contracts/

**Tests**: OBLIGATOIRES pour toute tâche `backend/` ou `engine/` (constitution IV — TDD, tests écrits d'abord). Frontend : tests sur la logique (`lib/`), pas sur le rendu pur.

**Organization**: phases = livraisons démontrables du plan (P1–P6), chaque phase mappée sur ses user stories. Gates de sortie : plan.md « Phasage livrable » + quickstart.md §0.

**Révision post-analyze 2026-07-09** : +6 tâches (gizmos de peinture/emboss/measure/brim ears, éditeur de hauteur de couche variable + vue assemblage, dialogs spéciaux de paramètres, suppression de compte, campagne SC-004) ; T038 ré-étiqueté [US3] ; T060 inclut le verrou optimiste (conflit multi-onglets).

## Format: `[ID] [P?] [Story] Description`

- **[P]** : parallélisable (fichiers distincts, pas de dépendance à une tâche inachevée)
- **[Story]** : US1–US8 (phases user story uniquement)

---

## Phase 1: Setup (infrastructure partagée)

**Purpose**: chaîne de build et gates prêts avant toute implémentation

- [X] T001 Vérifier la chaîne Nix : `nix build .#libslic3r` et `nix build .#dump-config` passent ; documenter `$LIBSLIC3R_DIR` dans engine/README.md
- [X] T002 [P] Script de codegen racine `scripts/codegen.sh` : `audit/run_all.py` → `audit/generate_parity_annexes.py` → génération `frontend/src/generated/` (params.ts, ui-layout.ts vides pour l'instant) ; idempotent
- [X] T003 [P] CI (`.github/workflows/ci.yml` ou équivalent) : gates constitution — `cargo fmt --check`, `cargo clippy --workspace -- -D warnings`, `cargo test --workspace`, `bun run lint`, `bun run check`, `bun test`, `python3 audit/run_all.py`
- [ ] T004 [P] Fixtures de test : `engine/tests/fixtures/` — 10 modèles de référence (benchy, tour, pièce à supports, multi-corps, STEP, OBJ, 3MF projet Orca…) + manifest des 5 combinaisons de presets (SC-003)

---

## Phase 2: Foundational (prérequis bloquants)

**Purpose**: artefacts générés et contrôles dont dépendent toutes les phases

- [ ] T005 Extracteur `audit/extract_legacy_keys.py` : parse `PrintConfigDef::handle_legacy` + `handle_legacy_composite` de vendor/OrcaSlicer/src/libslic3r/PrintConfig.cpp → `audit/legacy_keys.json` (ré-exécutable, intégré à run_all.py) ; valider contre les 287 clés du contrôle croisé
- [ ] T006 Squelette `audit/check_traceability.py` (R4) : vérifie params registre ↔ parameters.json ↔ exclusions.md, comptages presets ; sort non-zéro sur écart ; branché en CI (les périmètres UI s'ajoutent en P3/P4)
- [ ] T007 [P] Pipeline ts-rs : `backend/src/http/dto/mod.rs` exporte vers `frontend/src/generated/api/` via `cargo test export_bindings` ; test de fraîcheur en CI (diff vide après régénération)
- [ ] T008 [P] Module d'erreurs backend `backend/src/http/error.rs` : enveloppe `{code, message, details}`, 404 pour ressources d'autrui (SC-008), conversions domaine→HTTP ; tests unitaires

**Checkpoint**: codegen + gates opérationnels — les livraisons peuvent démarrer

---

## Phase 3: Livraison P1 — Engine wrapper + parité API C++ (US1 fondation)

**Goal**: trait `SlicerEngine` complet, bridge cxx-FFI + fallback CLI, parité prouvée contre l'oracle C++ (gate : suite générique verte sur FFI **et** CLI, corpus 10×5 identique à orca desktop)

**Independent Test**: `ENGINE_IMPL=ffi cargo run -p engine --example engine-cli -- slice engine/tests/fixtures/benchy.stl` produit un G-code identique à orca-slicer desktop ; idem `ENGINE_IMPL=cli`

- [ ] T009 [US1] Codegen registre : `engine/build.rs` génère `engine/src/params/registry.rs` depuis audit/parameters.json (858 clés : type, défaut, bornes, enums, mode, groupe) ; tests : comptage exact, spot-checks (layer_height, sparse_infill_pattern, host_type)
- [ ] T010 [P] [US1] Types miroirs `engine/src/api/` : Model/ModelObject/ModelVolume/ModelInstance, DynamicPrintConfig, TriangleMesh, BuildVolume, SliceRequest/SliceResult, EngineError (contrat slicer-engine-trait.md) ; tests de construction/validation
- [ ] T011 [US1] Trait `SlicerEngine` dans engine/src/lib.rs + suite de tests générique `engine/tests/common/trait_suite.rs` (`fn test_engine(e: &dyn SlicerEngine)`) — écrite AVANT les implémentations (TDD)
- [ ] T012 [US1] Bridge cxx : `engine/src/adapters/ffi/bridge.rs` (cxx::bridge) + `engine/src/adapters/ffi/bridge/model.cpp` ; build.rs linke `$LIBSLIC3R_DIR` (motif tools/dump-config) ; smoke test : charger un STL, compter les triangles
- [ ] T013 [US1] FFI load_model (STL/OBJ/3MF) + convert_to_mesh (STEP via OCCT) dans engine/src/adapters/ffi/model.rs ; tests sur fixtures (dont STEP)
- [ ] T014 [P] [US1] FFI read/write_project_3mf (scène + config embarquée) dans engine/src/adapters/ffi/threemf.rs + module commun engine/src/threemf/ ; tests aller-retour sur 3MF Orca de fixture
- [ ] T015 [P] [US1] FFI repair_mesh + RepairedMeshErrors dans engine/src/adapters/ffi/mesh.rs ; tests sur maillage non-manifold de fixture
- [ ] T016 [P] [US1] FFI arrange + orient (paramètres de dégagement machine) dans engine/src/adapters/ffi/arrange.rs ; tests : pas de collision, objets dans le volume
- [ ] T017 [US1] FFI resolve_preset_chain + validate_config dans engine/src/adapters/ffi/config.rs ; tests : chaîne BBL connue → valeurs effectives attendues, valeur hors bornes → ConfigWarning
- [ ] T018 [US1] Process worker `engine/src/bin/engine-worker.rs` : slice isolé (crash C++ contenu), progression par pipe (callbacks statusbar libslic3r), annulation par kill (R1/R9) ; tests : progression monotone, kill → cancelled, crash simulé → EngineError
- [ ] T019 [US1] FFI slice via engine-worker dans engine/src/adapters/ffi/slice.rs : SliceRequest → G-code + GCodeProcessorResult + thumbnails ; tests sur benchy
- [ ] T020 [US1] Adaptateur CLI complet `engine/src/adapters/cli/` : mêmes opérations via orca-slicer (--slice/--arrange/--orient/--repair/--export-3mf/--load-settings/--load-filaments, R1) ; la suite générique T011 passe telle quelle
- [ ] T021 [US1] Parseur G-code `engine/src/gcode/` : couches/segments (;TYPE:, ;LAYER_CHANGE, F, rétractions), stats (temps, filament) → GcodePreview (R6) ; tests sur G-codes de fixture (tous types de lignes présents)
- [ ] T022 [US1] Test de parité `engine/tests/gcode_parity.rs` : triple diff normalisé FFI vs CLI vs sortie desktop enregistrée, corpus 10×5 ; temps estimés < 1 % (SC-003)
- [ ] T023 [P] [US1] Démo `engine/examples/engine-cli.rs` : slice/arrange/repair/parse depuis le shell, sélection ENGINE_IMPL (démo gate P1)

**Checkpoint**: gate P1 — moteur démontrable et prouvé, indépendant du backend

---

## Phase 4: Livraison P2 — Auth + storage (US6)

**Goal**: comptes, sessions, isolation totale, bibliothèque de projets persistée (gate : suite de contrat Storage verte sur SQLite ET Postgres, tests d'isolation verts)

**Independent Test**: 2 comptes via l'UI minimale ; le projet de A est en 404 pour B (accès direct URL) ; persistance après reconnexion

- [ ] T024 [US6] Entités domaine + traits `backend/src/domain/` : User, Project, Model, Preset, Printer, SlicingJob, Gcode, InstanceSettings + traits UserRepo/ProjectRepo/…/Storage (contrat storage-trait.md, signatures scopées par UserId) — AVANT les impls, avec suite de contrat générique `backend/tests/storage_contract.rs` (TDD)
- [ ] T025 [US6] Migrations SQLite `backend/migrations/sqlite/0001_init.sql` (schéma data-model.md complet, y c. slicing_jobs/gcodes pour P5) + impl `backend/src/adapters/storage/sqlite/` ; suite T024 verte
- [ ] T026 [US6] Migrations + impl Postgres `backend/src/adapters/storage/postgres/` (feature `postgres`) ; suite T024 verte sur Postgres (job CI optionnel)
- [ ] T027 [P] [US6] Adaptateur fichiers `backend/src/adapters/files/` : espaces `data/users/<id>/{models,gcodes,thumbnails}`, écriture atomique, purge par cascade ; tests (traversée de chemin impossible)
- [ ] T028 [US6] Auth `backend/src/auth/` : argon2id, tower-sessions (store sqlx), register selon `registration_policy` (open/closed/invite), login/logout/me, premier compte = admin ; tests : hash jamais en clair, expiration session, politiques
- [ ] T029 [US6] Endpoints admin `backend/src/http/routes/admin.rs` : instance (GET/PATCH), users (POST, reset-password), invitations (POST), reseed placeholder ; tests : réservé admin
- [ ] T030 [US6] Endpoints projets `backend/src/http/routes/projects.rs` : CRUD + duplicate/rename, document scène versionné (schema_version), vignette ; tests de contrat + isolation (404 inter-comptes, SC-008)
- [ ] T031 [US6] Suppression de compte : `DELETE /api/auth/me` (avec confirmation par mot de passe) + `DELETE /api/admin/users/{id}` dans backend/src/http/routes/{auth,admin}.rs — cascade BDD (T024) + purge fichiers (T027), edge case spec ; tests : purge complète, admin ne peut pas se supprimer s'il est le dernier
- [ ] T032 [P] [US6] Frontend auth : `frontend/src/routes/(auth)/login/+page.svelte`, register, garde de session dans `frontend/src/lib/api/session.ts` ; test du client API
- [ ] T033 [P] [US6] Frontend bibliothèque : `frontend/src/routes/library/+page.svelte` (vignettes, dates, ouvrir/dupliquer/renommer/supprimer) sur le client typé ts-rs
- [ ] T034 [P] [US6] Brouillon de session client (clarification) : `frontend/src/lib/stores/draft.ts` sur IndexedDB, restauration après fermeture accidentelle ; tests vitest

**Checkpoint**: gate P2 — multi-comptes démontrable, socle pour P3–P6

---

## Phase 5: Livraison P3 — Presets + onglets paramètres (US2, US3)

**Goal**: 11 895 presets seedés avec héritage, onglets Process/Filament/Imprimante générés complets, modes simple/advanced/expert (gate : comptages == Annexe C, check_traceability vert sur params + UI réglages)

**Independent Test**: comparaison côte à côte avec references/orca-prepare.png ; dériver un preset filament et vérifier le diff

- [ ] T035 [US3] Codegen legacy : `engine/src/params/legacy.rs` généré depuis audit/legacy_keys.json (T005) ; tests : les 287 clés converties (layer_heigth→layer_height, etc.)
- [ ] T036 [US3] Import des profils système `engine/src/presets/import.rs` : parsing des 65 index vendeurs + presets bruts (inherits/instantiation/compatible_printers) ; tests : comptages exacts {384, 1158, 7012, 3341} (SC-002)
- [ ] T037 [US3] Résolution d'héritage `engine/src/presets/resolve.rs` (fonction pure, R5) ; tests : chaîne « Bambu Lab A1 0.4 nozzle » → valeurs effectives connues ; preset user dérivé → surcharges seules stockées, parent mis à jour → propagation (US3-AS4)
- [ ] T038 [US3] Seed au démarrage + commande admin reseed dans backend/src/domain/presets.rs + backend/src/http/routes/admin.rs ; tests : idempotence, presets user intacts après reseed
- [ ] T039 [US3] Endpoints presets `backend/src/http/routes/presets.rs` : liste filtrée compatibilité (kind, printer, instantiation), GET brut/résolu, CRUD user, import/export JSON Orca avec conversion legacy ; tests de contrat (FR-020..023)
- [ ] T040 [US2] Codegen UI : `scripts/codegen.sh` étendu → `frontend/src/generated/ui-layout.ts` (21 pages/100 sections/525 options depuis ui_inventory.json) + `frontend/src/generated/params.ts` (registre complet) ; test de fraîcheur CI
- [ ] T041 [US2] Bibliothèque de widgets `frontend/src/lib/settings/widgets/` : Bool, Int, Float+unité, Percent, FloatOrPercent, Enum(select), String, Strings, Multiline (G-code), Point(s), Color — résolution par type du registre ; tests vitest de mapping type→widget
- [ ] T042 [US2] Rendu générique `frontend/src/lib/settings/SettingsTabs.svelte` + `OptionLine.svelte` : arbre ui-layout.ts, modes simple/advanced/expert (champ mode), tooltips, sidetext, catégories, recherche additive ; tests du filtre de mode
- [ ] T043 [US2] État de réglages `frontend/src/lib/settings/store.ts` : valeurs effectives (preset résolu) + surcharges projet, marqueurs modifié/verrou, reset par option (US2-AS6), validation bornes/enums via params.ts ; tests vitest
- [ ] T044 [US2] Pages spéciales : `frontend/src/lib/settings/special/OverridesPage.svelte` (cases N/A filament_*), MachineGcode.svelte (éditeurs multilignes), MultimaterialTables.svelte ; conformes Annexe B
- [ ] T045 [US2] Dialogs spéciaux de paramètres `frontend/src/lib/settings/special/` : éditeur de forme de plateau (BedShape.svelte : `printable_area`, `bed_exclude_area`, `bed_custom_model`, `bed_custom_texture`), volumes de purge (FlushVolumes.svelte : `flush_volumes_matrix/vector`, `flush_multiplier`), table des températures par type de plaque (PlateTemps.svelte : `cool/eng/hot/textured*_plate_temp*`) — couvre les clés hors lignes d'option (analyse G6) ; tests de sérialisation des valeurs
- [ ] T046 [P] [US3] Sélecteurs de presets `frontend/src/lib/presets/` : imprimante (vendeurs/modèles/buses, cover images), filament/process filtrés, dériver/sauvegarder/supprimer, badge héritage ; tests du filtre de compatibilité
- [ ] T047 [US2] Étendre audit/check_traceability.py au périmètre UI réglages : chaque option de ui_inventory ↔ ui-layout.ts ∪ exclusions.md, y compris les clés portées par les dialogs T045 ; gate CI vert (SC-001 partiel)

**Checkpoint**: gate P3 — onglets complets démontrables contre orca-prepare.png

---

## Phase 6: Livraison P4 — Scène 3D + plater (US4 + import US1)

**Goal**: préparation de scène complète : import, transformations, outils (16 gizmos), multi-plateaux, sauvegarde (gate : scénarios US4 verts, SC-007 mesuré)

**Independent Test**: scène multi-objets multi-plateaux préparée (avec peintures et hauteurs de couche variables), sauvegardée, rouverte à l'identique

- [ ] T048 [US1] Upload modèles `backend/src/http/routes/models.rs` : multipart 500 Mo, formats STL/3MF/STEP/OBJ, STEP → conversion asynchrone via SlicerEngine (WS model.converted), 3MF projet → import scène+réglages ; tests (formats, corrompu, limite taille)
- [ ] T049 [US1] Endpoint maillage `GET /api/models/{id}/mesh` : format binaire compact (positions/normales/index) pour Threlte ; tests de sérialisation
- [ ] T050 [P] [US4] Scène Threlte `frontend/src/lib/scene/` : Canvas, plateau (bed_model/bed_texture du preset machine, grille, origine), caméra orbit, éclairage, sélection (raycast) ; conforme references/orca-prepare.png
- [ ] T051 [P] [US4] Loaders client STL/OBJ/3MF `frontend/src/lib/scene/loaders.ts` (aperçu immédiat, R7) + upload en tâche de fond ; tests vitest sur petits fichiers
- [ ] T052 [US4] Gizmos move/rotate/scale + pose à plat `frontend/src/lib/scene/gizmos/` + panneau numérique de transformation ; undo/redo `frontend/src/lib/stores/history.ts` ; tests du store history
- [ ] T053 [US4] Liste d'objets `frontend/src/lib/scene/ObjectList.svelte` : sélection, groupes, duplication, verrouillage/masquage, extrudeur par objet/pièce, réglages par objet (surcharges FR-015)
- [ ] T054 [US4] Outils moteur côté API `backend/src/http/routes/scene.rs` : repair (rapport), arrange, orient, opérations booléennes via SlicerEngine ; tests de contrat
- [ ] T055 [US4] Outils UI : coupe (plan positionnable, moitiés/connecteurs), réparation avec rapport, booléens, simplification — `frontend/src/lib/scene/tools/` ; branchés sur T054
- [ ] T056 [US4] Gizmos de peinture `frontend/src/lib/scene/gizmos/painting/` : supports (FdmSupports), couture (Seam), fuzzy skin (FuzzySkin), segmentation multi-matériaux (MmSegmentation) — pinceau/sphère/remplissage, rayon ajustable (raccourcis groupe Gizmo : Ctrl/Alt+molette), stockage au format 3MF Orca dans le document scène (data-model.md) ; tests de sérialisation des peintures (analyse G1)
- [ ] T057 [US4] Gizmos texte/SVG en relief (Emboss, SVG — création de volumes via engine), mesure (Measure : distances/angles sur le maillage), oreilles de bord (BrimEars : points + `brim_ears` params) dans `frontend/src/lib/scene/gizmos/` + endpoints engine nécessaires dans backend/src/http/routes/scene.rs ; tests (analyse G1)
- [ ] T058 [US4] Éditeur de hauteur de couche variable (toolbar `layersediting` : profil par objet via LayerHeightProfile du trait engine, courbe éditable + lissage adaptatif) `frontend/src/lib/scene/tools/LayerHeight.svelte` + vue assemblage (`assembly_view` + gizmo Assembly : éclaté des pièces) ; profil persisté dans le document scène ; tests engine du profil (analyse G2)
- [ ] T059 [US4] Multi-plateaux `frontend/src/lib/scene/plates.ts` + réglages par plateau (type de plaque) ; ajout/suppression/répartition ; tranchage par plateau préparé pour P5
- [ ] T060 [US4] Sauvegarde projet complète : document scène (data-model.md, y c. peintures T056 et profils T058) sérialisé/restauré, vignette générée (canvas), Ctrl+S ; **verrou optimiste** : champ `version` sur PUT, 409 + avertissement en cas de conflit multi-onglets (edge case spec, analyse G5) ; tests aller-retour + conflit
- [ ] T061 [P] [US4] Menus contextuels + barre d'outils + raccourcis plateau/liste (Annexe B §B.3–B.6 groupes Plater/Objects List) dans `frontend/src/lib/scene/menus.ts` ; entrées ajoutées à traceability-map.json
- [ ] T062 [US4] Étendre check_traceability.py : gizmos (16/16)/toolbars/menus/raccourcis ↔ traceability-map.json ∪ exclusions.md ; benchmark SC-007 (2 M triangles / 20 objets) documenté dans quickstart

**Checkpoint**: gate P4 — préparation de scène démontrable de bout en bout (16 gizmos couverts)

---

## Phase 7: Livraison P5 — Slicing serveur + préviz G-code (US1, US5, US7)

**Goal**: file persistante avec reprise, progression WS, prévisualisation par couches complète, exports (gate : US1/US5/US7 verts, SC-005/006 mesurés, reprise au reboot testée)

**Independent Test**: trancher depuis l'UI, suivre la progression, explorer la préviz (cible orca-preview.png), télécharger le G-code ; kill du backend pendant un job → reprise

- [ ] T063 [US7] File `backend/src/queue/` : JobRepo.claim_next transactionnel, pool de workers (config), transitions d'états (data-model.md), requeue_running au boot ; tests : concurrence (jamais 2× le même job), reprise, annulation kill
- [ ] T064 [US1] Endpoint slice `POST /api/projects/{id}/slice` (plate_index|all) : presets résolus figés (resolved_settings), création jobs ; tests de contrat + avertissements moteur restitués (FR-032, hors plateau → erreur avant slice)
- [ ] T065 [US7] WebSocket `/api/ws` `backend/src/http/ws.rs` : canal authentifié, events job.updated/job.finished/model.converted (DTO ts-rs) ; tests : isolation par compte, progression relayée depuis le pipe engine-worker
- [ ] T066 [US1] Post-traitement job : G-code stocké (files adapter), stats extraites (GcodePreview), vignettes ; `GET /api/gcodes/{id}/download` + `/stats` ; tests
- [ ] T067 [US5] Buffers préviz `backend/src/http/routes/preview.rs` : `/preview/meta` (couches, types présents, échelles) + `/preview/layers?from&to` (binaire R6) ; tests : format, plages, tailles
- [ ] T068 [US5] Rendu préviz `frontend/src/lib/preview/` : BufferGeometry par plage, 7 colorations (type/vitesse/hauteur/largeur/débit/température/filament) avec légende/échelle, visibilité par type ; conforme references/orca-preview.png
- [ ] T069 [US5] Curseurs couches (min–max) + progression intra-couche + raccourcis groupe Preview `frontend/src/lib/preview/sliders.ts` ; tests vitest des plages d'index
- [ ] T070 [P] [US5] Panneau stats `frontend/src/lib/preview/StatsPanel.svelte` : durées par type (%, utilisation), estimation totale, filament (longueur/volume/masse/coût), changements d'outil — parité orca-preview.png
- [ ] T071 [P] [US7] Page file d'attente `frontend/src/routes/queue/+page.svelte` : états, progression, annulation, historique, notifications job.finished
- [ ] T072 [US1] Export projet 3MF `GET /api/projects/{id}/export/3mf` (via engine::threemf, compatible Orca) + bouton UI ; test aller-retour desktop
- [ ] T073 [US7] Tests de charge `backend/tests/queue_concurrency.rs` : 10 jobs / 2 comptes simultanés (SC-006) ; mesure du flux complet SC-005 documentée dans quickstart

**Checkpoint**: gate P5 — flux cœur complet démontrable (US1 de bout en bout)

---

## Phase 8: Livraison P6 — Intégration Moonraker (US8)

**Goal**: déclaration d'imprimantes Klipper, envoi, suivi, contrôle (gate : US8 vert contre instance simulée en CI)

**Independent Test**: contre `virtual-klipper-printer` : déclarer, tester, envoyer avec démarrage, pause/reprise/annulation, états en direct

- [ ] T074 [US8] Client Moonraker `backend/src/adapters/moonraker/` : /server/info, upload multipart (print=true), WS JSON-RPC subscribe (print_stats, heaters), pause/resume/cancel, X-Api-Key (R10) ; tests contre serveur mock in-process
- [ ] T075 [US8] Endpoints imprimantes `backend/src/http/routes/printers.rs` : CRUD (api_key chiffrée au repos), test, upload (gcode_id, start_now), status, contrôles ; tests de contrat + échec réseau propre (FR-062)
- [ ] T076 [US8] Relay WS printer.status : subscription Moonraker → événements clients ; tests d'isolation (imprimante de A invisible pour B)
- [ ] T077 [P] [US8] Frontend imprimantes `frontend/src/routes/printers/+page.svelte` : déclaration/test, envoi depuis la préviz/file (start_now), panneau état (progression, températures), pause/reprise/annulation
- [ ] T078 [US8] Test d'intégration CI `backend/tests/moonraker_integration.rs` contre le mock complet (upload 50 Mo simulé, suivi, SC-009)

**Checkpoint**: gate P6 — chaîne complète modèle → impression démontrable

---

## Phase 9: Polish & transverse

**Purpose**: parité finale vérifiée, i18n, aide, performance, validation utilisateur

- [ ] T079 [P] Menus principaux restants (Fichier/Édition/Vue/Aide, Annexe B §B.2) + dialogue d'aide raccourcis (92 entrées, adaptations navigateur consignées dans exclusions.md) dans frontend/src/lib/menus/
- [ ] T080 [P] i18n additive fr/en `frontend/src/lib/i18n/` (libellés de parité anglais = clés, FR-072)
- [ ] T081 Résolution des entrées « à trancher à l'implémentation » d'exclusions.md (classement final de chaque clé restante) — check_traceability sans catégorie provisoire
- [ ] T082 Passe finale de traçabilité : `python3 audit/run_all.py && python3 audit/check_traceability.py` vert complet (SC-001) ; annexes resynchronisées ; registre d'exclusions revu
- [ ] T083 [P] Campagne d'isolation automatisée `backend/tests/isolation_campaign.rs` : accès direct à toutes les ressources d'autrui (SC-008 à 100 %)
- [ ] T084 [P] Performance : profil préviz/scène (SC-007), budget mémoire buffers, lazy-loading des couches ; résultats documentés dans quickstart.md
- [ ] T085 [P] Campagne SC-004 : protocole de test utilisateur (40 paramètres tirés au sort, cible 95 % trouvés au même endroit qu'OrcaSlicer) documenté et exécuté ; résultats dans specs/001-orcaslicer-web-parity/validation-sc004.md (analyse G4)
- [ ] T086 Documentation finale : README racine (installation nix, démarrage), engine/README.md (FFI/CLI), parcours quickstart.md rejoué intégralement

---

## Dependencies & Execution Order

```text
Phase 1 (Setup) → Phase 2 (Foundational)
Phase 2 → Phase 3 (P1 engine) → Phase 7 (P5 slicing)
Phase 2 → Phase 4 (P2 auth/storage) → Phases 5, 6, 7, 8
Phase 5 (P3 presets/onglets) → Phase 7 (presets résolus requis)
Phase 6 (P4 scène) → Phase 7 (scène à trancher)
Phase 7 → Phase 8 (P6 Moonraker : envoie les G-codes produits)
Phases 5 ∥ 6 (parallélisables après Phase 4 ; Phase 3 indépendante d'elles)
Phase 9 après Phase 8
```

Dépendances intra-phase notables : T011 (suite générique) avant T012–T020 ;
T024 (traits + suite de contrat) avant T025–T031 ; T040 (codegen UI) avant
T041–T047 ; T052/T054 avant T055–T058 ; T063 (file) avant T064–T066.

## Parallel Execution Examples

- **Phase 3** : T013 ∥ T014 ∥ T015 ∥ T016 (wrappers FFI distincts) après T012 ; T021 ∥ T020.
- **Après Phase 4** : une piste « P3 » (T035–T047) et une piste « P4 » (T048–T062) en parallèle.
- **Phase 6** : T056 ∥ T057 ∥ T058 (gizmos indépendants) après T052/T054.
- **Phase 7** : T068 ∥ T070 ∥ T071 côté frontend pendant T063–T067 côté backend.
- **Phase 9** : T079 ∥ T080 ∥ T083 ∥ T084 ∥ T085.

## Implementation Strategy

- **MVP** : Phases 1–4 + chemin minimal de la Phase 7 (T063–T066 + bouton
  slice basique) = US1 démontrable (import → slice presets par défaut →
  export). Les onglets complets (Phase 5) et la scène complète (Phase 6)
  enrichissent ensuite sans bloquer la démo.
- **Livraison incrémentale** : chaque phase se termine par son gate
  (quickstart §0) et une démo ; commit atomique par tâche, tests inclus
  (constitution IV) ; check_traceability s'étend phase par phase (T006 →
  T047 → T062 → T082) pour que la parité reste mesurée en continu.
- **Risque en tête** : la Phase 3 (bridge FFI) est le chemin critique — le
  fallback CLI (T020) débloque les phases aval si le FFI prend du retard,
  sans changer aucun contrat.
