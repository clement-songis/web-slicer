# Research — Web-Slicer parité OrcaSlicer (Phase 0)

Toutes les inconnues du Technical Context sont résolues ci-dessous.
Décisions numérotées R1–R11, référencées par plan.md et les contrats.
Révision 2026-07-09 : R1 inversé (FFI principal, CLI fallback) sur directive
utilisateur ; R9 ajusté en conséquence ; R11 ajouté (phasage livrable).
Révision 2026-07-10 : **moteur v1 = FFI uniquement** (directive utilisateur) —
l'adaptateur CLI est retiré du périmètre v1 ; une implémentation CLI de
validation croisée devient un objectif backlog. Les mentions « fallback CLI »
ci-dessous sont conservées comme historique mais **ne s'appliquent plus à v1**.

## R1 — Moteur v1 : bridge cxx-FFI vers libslic3r, fallback CLI (révisé)

**Decision** : l'implémentation principale du trait `SlicerEngine` est un
**bridge cxx** (`engine/src/adapters/ffi/`) lié statiquement aux artefacts du
paquet Nix `libslic3r` (`libslic3r-headless` : `*.a` + headers, déjà défini
dans `flake.nix` ; chaîne de link validée par `tools/dump-config` avec
`$LIBSLIC3R_DIR`). Le bridge expose des wrappers C++ minces (un `.cpp` par
domaine : model, print, slice, mesh, presets) déclarés via `cxx::bridge`,
au-dessus des classes de `audit/engine_api.json` (Model, Print, PrintObject,
TriangleMesh, DynamicPrintConfig…). Le slicing s'exécute dans un process
worker dédié (`engine-worker`, spawné par la crate) pour isoler les crashs
C++ et permettre l'annulation par kill — le backend ne link pas le C++.
Un **fallback CLI** (`engine/src/adapters/cli/`) pilote `orca-slicer`
(`--slice`, `--arrange`, `--orient`, `--repair`, `--export-3mf`,
`--load-settings`, `--load-filaments`, vérifiés dans `CLIActionsConfigDef`,
PrintConfig.cpp:10588+) derrière le même trait ; sélection par config
`ENGINE_IMPL=ffi|cli` (défaut : ffi, bascule cli si lib indisponible).

**Rationale** : directive utilisateur (stack imposée) ; l'infrastructure de
build existe déjà dans le flake ; le FFI apporte les callbacks de progression
fins (statusbar libslic3r), la conversion STEP directe, l'accès aux
structures de préviz (GCodeProcessorResult) sans re-parsing, et évite le coût
fork+reload par job. Le CLI conservé sert de mode dégradé **et** d'oracle de
parité croisée : les tests P1 comparent FFI vs CLI vs orca desktop.

**Alternatives considered** : (a) CLI-first (plan initial) — remplacé sur
directive ; conservé en fallback ; (b) WASM libslic3r côté client — rejeté :
compilation non maintenue, mémoire navigateur, incompatible file serveur ;
(c) bindgen brut sans cxx — rejeté : pas de gestion sûre des exceptions/types
C++ riches.

## R2 — Registre des paramètres : codegen depuis `audit/parameters.json`

**Decision** : `engine/build.rs` (ou script `cargo xtask codegen`) génère depuis
`audit/parameters.json` : (1) `engine/src/params/registry.rs` — enum/const des
858 clés avec type, défaut, bornes, enum values, mode, groupe ; (2)
`frontend/src/generated/params.ts` — le même registre pour la validation et le
rendu côté client (via export JSON + script `bun run codegen`). La validation
d'une valeur (type, bornes, enum) vit dans l'engine (source de vérité) et est
répliquée mécaniquement côté TS.

**Rationale** : FR-001 exige les 858 paramètres sans dérive ; toute mise à jour
d'OrcaSlicer vendored se propage par régénération (`audit/run_all.py` →
codegen) ; le diff de code généré rend les changements de parité visibles en
revue.

**Alternatives considered** : saisie manuelle (ingérable) ; chargement runtime
du JSON sans types (perd la vérification statique et le tree-shaking des modes).

## R3 — Layout des onglets : descripteur généré depuis `audit/ui_inventory.json`

**Decision** : un codegen produit `frontend/src/generated/ui-layout.ts` :
arbre pages → sections → clés d'options (+ icônes, titres) copié de
`settings_tabs` de l'inventaire. Le composant `SettingsTabs.svelte` rend cet
arbre générique : chaque clé résout son widget par le type du registre R2
(bool → toggle, enum → select, float+sidetext → champ unité, etc.), le champ
`mode` pilote l'affichage simple/advanced/expert, `category`/tooltips repris
du registre. Les pages spéciales (Setting Overrides avec cases N/A, Machine
G-code multilignes, tables Multimaterial) sont des widgets dédiés référencés
par le descripteur.

**Rationale** : FR-002/FR-005 par construction — l'UI ne peut pas s'écarter de
l'Annexe B puisqu'elle est rendue depuis celle-ci.

**Alternatives considered** : pages Svelte écrites à la main (dérive garantie
sur 525 options) ; iframe/portage wxWidgets (absurde en web).

## R4 — Traçabilité : `audit/check_traceability.py`

**Decision** : nouveau script de contrôle qui vérifie mécaniquement :
(1) chaque clé de `parameters.json` (groupes `fff`/`common`/`sla`) existe dans
le registre généré R2 OU dans `specs/001-orcaslicer-web-parity/exclusions.md` ;
(2) chaque option/page/section de `ui_inventory.json` existe dans le layout R3
OU dans exclusions.md ; (3) menus, gizmos, raccourcis mappés vers un composant
via un fichier de correspondance maintenu (`frontend/src/generated/`
+ `traceability-map.json`) ; (4) comptages presets == Annexe C. Sortie non-zéro
si écart non justifié → gate CI et contrôle de jalon (FR-003, SC-001).

**Rationale** : la parité doit être un test qui échoue, pas une intention.

**Alternatives considered** : revue manuelle par checklist (invérifiable à
cette échelle).

## R5 — Presets : import au seed + résolution d'héritage dans l'engine

**Decision** : au premier démarrage (et sur commande admin `reseed`), le
backend importe `vendor/OrcaSlicer/resources/profiles` en base via
`engine::presets` : parsing des index vendeurs, presets bruts stockés avec
`inherits`, `instantiation`, `setting_id`. La **résolution** (chaîne d'héritage
aplatie + validation par le registre R2) est une fonction pure de l'engine,
utilisée aussi pour les presets utilisateur (parent système + surcharges). Les
287 clés legacy identifiées par l'audit sont converties à l'import par une
table de correspondance extraite de `PrintConfigDef::handle_legacy`
(nouvel extracteur `audit/extract_legacy_keys.py`, généré vers
`engine/src/params/legacy.rs`).

**Rationale** : FR-020/023 ; l'héritage résolu côté serveur garantit les mêmes
valeurs effectives que le desktop ; le stockage des presets bruts permet la
mise à jour des profils vendeurs sans casser les dérivés utilisateur (US3-AS4).

**Alternatives considered** : résolution à la volée côté client (divergence de
logique) ; aplatir à l'import (perd la propagation parent → enfant).

## R6 — Prévisualisation G-code : parseur serveur + buffers binaires

**Decision** : `engine/src/gcode` parse le G-code produit (commentaires
`;TYPE:`, `;LAYER_CHANGE`, `; layer num`, vitesses F, largeur/hauteur emitted
par Orca, retractions, temps `; estimated printing time`) en un modèle
couches → segments typés. Le backend expose ce modèle en buffers binaires
compacts (positions Float32, attributs par segment : type u8, feedrate f32,
extrudeur u8, plage par couche) téléchargés par plage de couches. Le frontend
rend en Three.js (BufferGeometry indexée par couche, shader color-by-attribute
pour les 7 colorations FR-041, curseurs = plages d'index).

**Rationale** : FR-040/041/042/043 et SC-007 — le parsing serveur évite de
transférer le G-code texte et rend les statistiques (FR-043) cohérentes avec
l'affichage ; Orca émet des commentaires stables (même famille que le
GCodeProcessor de l'Annexe engine_api).

**Alternatives considered** : parser en TS dans le navigateur (double
implémentation des règles, coût mémoire) ; `--export-slicedata` seul
(format interne moins documenté ; conservé comme source des statistiques si
plus fiable au moment de l'implémentation).

## R7 — Import de modèles : STL/OBJ/3MF natifs client, STEP via serveur

**Decision** : STL/OBJ/3MF sont chargés et affichés immédiatement côté client
(loaders Three.js) puis uploadés bruts au serveur (fichier source conservé,
FR-053). STEP est uploadé puis converti par l'engine — en FFI, directement via
le lecteur STEP de libslic3r (OCCT) ; en fallback CLI, via `orca-slicer
--export-3mf` d'un projet contenant le STEP (tessellation via le pipeline
CLI) ; le maillage résultant redescend au client. Les 3MF de projets OrcaSlicer
passent par `engine::threemf` pour extraire scène + réglages embarqués
(edge case spec).

**Rationale** : préserve la réactivité (pas d'aller-retour pour 90 % des
imports) tout en couvrant STEP sans embarquer OCCT dans le navigateur.

**Alternatives considered** : occt-import-js WASM (~15 Mo, divergence de
tessellation avec Orca) ; conversion de tout côté serveur (latence inutile).

**Retournement (Phase 13, T120–T128)** : la décision initiale « STL/OBJ/3MF
natifs client » est **abandonnée**. *Tous* les formats (STL, OBJ, 3MF, STEP,
AMF, …) sont désormais décodés **côté serveur** par le process `engine-worker`
(commande `load-model <fichier> <format>`), qui tessellise via libslic3r et
renvoie un maillage d'affichage binaire **WSMh** (`engine::api::TriangleMesh::
encode_display`). Le frontend ne contient plus aucun parseur de modèle : il
uploade le fichier source, suit l'état de conversion (`uploading` → `converting`
→ `ready`/`failed`) et télécharge le WSMh via `GET /api/models/{id}/mesh`
(200 prêt, 409 en cours, 422 décodage échoué).

*Rationale du retournement* : (1) **parité de tessellation** — un seul chemin
moteur garantit que l'aperçu correspond exactement à ce que découpe Orca, la
divergence redoutée pour occt-import-js s'appliquait en réalité aussi aux
loaders Three.js STL/OBJ/3MF ; (2) **isolation** (constitution) — un fichier
malveillant ou corrompu fait planter un process worker jetable, pas le
navigateur ni le backend (couvert par `backend/tests/model_decode.rs`) ;
(3) **surface unique** — supprime la double implémentation client/serveur des
règles de lecture 3MF et le poids des parseurs côté navigateur. Le compromis de
réactivité (aller-retour serveur pour 100 % des imports au lieu de 10 %) est
accepté : la conversion est asynchrone et l'UI affiche un état de progression.

## R8 — Storage : traits de repositories + deux backends sqlx

**Decision** : le domaine définit des traits fins (`UserRepo`, `ProjectRepo`,
`PresetRepo`, `JobRepo`, `PrinterRepo`, unités de travail par transaction)
regroupés dans un trait-façade `Storage`. Implémentations : `storage/sqlite`
(défaut) et `storage/postgres` (feature `postgres` + config runtime
`DATABASE_URL`). Migrations sqlx par backend (`migrations/sqlite`,
`migrations/postgres`), mêmes schémas logiques. Les tests de contrat du trait
tournent sur SQLite en CI (et Postgres en job optionnel).

**Rationale** : constitution III ; sqlx 0.9 sans driver `Any` (déprécié) —
deux impls explicites évitent le plus petit dénominateur SQL.

**Alternatives considered** : `sqlx::Any` (déconseillé, perd les types) ;
ORM (SeaORM) — couche de plus sans bénéfice ici.

## R9 — File de tranchage : table `slicing_jobs` + workers tokio

**Decision** : file persistée en base (états `queued/running/succeeded/failed/
cancelled`, progression, `user_id`, plateau, presets résolus figés en JSON).
Pool de N workers tokio (N = config, défaut = cœurs/2) qui réclament les jobs
par transaction (`SELECT … WHERE status='queued' ORDER BY created_at LIMIT 1
FOR UPDATE`/équivalent SQLite). Au boot : jobs `running` → re-`queued`
(reprise auto, clarification session). Progression relayée par WebSocket
(canal `jobs`) : en FFI, callbacks du statusbar libslic3r remontés par le
process `engine-worker` (pipe, R1) ; en fallback CLI, parsing du stdout
d'`orca-slicer`. Annulation = kill du process (worker FFI ou CLI) + statut
`cancelled` — identique dans les deux modes.

**Rationale** : FR-031 + clarification « reprise auto » ; pas de broker externe
à opérer pour ~50 utilisateurs ; l'isolation par `user_id` est triviale à
tester (SC-008).

**Alternatives considered** : Redis/NATS (infrastructure en plus, contraire au
dimensionnement v1) ; exécution synchrone dans le handler (bloque, perd la
reprise).

## R10 — Moonraker : client REST + WebSocket dédié dans un adaptateur

**Decision** : `backend/src/adapters/moonraker` — upload par
`POST /server/files/upload` (multipart, champ `print=true` optionnel pour
démarrage immédiat), état par WebSocket JSON-RPC (`printer.objects.subscribe`
sur `print_stats`, `heater_bed`, `extruder`, `display_status`) avec fallback
polling `GET /printer/objects/query`. Pause/reprise/annulation via
`printer.print.pause|resume|cancel`. Clé API en header `X-Api-Key`. Test de
connexion = `GET /server/info`. Les états remontent au client par le même
canal WS applicatif que la file (FR-061). Une instance Moonraker simulée
(serveur de test in-process) couvre l'intégration en CI (dépendance spec).

**Rationale** : API Moonraker stable et documentée, identique à ce que fait
Mainsail ; le trait domaine `PrintHost` laisse la place aux 15 autres hôtes du
backlog v2 sans refonte.

**Alternatives considered** : passer par l'implémentation print-host de
libslic3r via CLI (non exposée par le CLI) ; support multi-hôtes v1 (exclu par
clarification).

## R11 — Phasage livrable en 6 phases démontrables

**Decision** : livraison découpée selon la directive utilisateur — P1 engine
wrapper + tests de parité contre l'API C++ ; P2 auth + storage ; P3 presets +
onglets paramètres générés ; P4 scène 3D + plater ; P5 slicing + préviz
G-code ; P6 intégration Moonraker. Chaque phase a une démo autonome et un
gate de sortie (tableau « Phasage livrable » de plan.md) ; P3 ∥ P4 après P2.
Les tâches (/speckit-tasks) sont organisées par phase, chaque tâche livrant
ses tests (constitution IV).

**Rationale** : réduit le risque d'intégration (le moteur, zone la plus
incertaine, est validé en premier contre l'oracle C++) ; permet des démos
intermédiaires alignées sur les user stories (P2→US6, P3→US2/US3, P4→US4,
P5→US1/US5/US7, P6→US8).

**Alternatives considered** : découpage par user story pur (mélange les
chantiers techniques transverses comme le codegen) ; big-bang (rejeté,
risque d'intégration).
